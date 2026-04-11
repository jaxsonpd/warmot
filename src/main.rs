// src/main.rs
use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client as S3Client;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::env;

const TOKEN_URL: &str = "https://identity.dataspace.copernicus.eu/auth/realms/CDSE/protocol/openid-connect/token";
const STAC_BASE: &str = "https://stac.dataspace.copernicus.eu/v1";

// ── Auth ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn get_token(client: &Client, username: &str, password: &str) -> String {
    let res = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "password"),
            ("username", username),
            ("password", password),
            ("client_id", "cdse-public"),
        ])
        .send()
        .await
        .unwrap();

    println!("Token response status: {}", res.status());

    let body: TokenResponse = res.json().await.unwrap();

    match body.access_token {
        Some(token) => token,
        None => {
            eprintln!("Auth failed!");
            eprintln!("Error: {:?}", body.error);
            eprintln!("Description: {:?}", body.error_description);
            panic!("Could not obtain access token");
        }
    }
}

// ── S3 client ─────────────────────────────────────────────────────────────────

use aws_sdk_s3::config::{Builder, BehaviorVersion};

fn build_s3_client(access_key: &str, secret_key: &str) -> S3Client {
    let creds = Credentials::new(access_key, secret_key, None, None, "cdse");
    let config = Builder::new()
        .behavior_version(BehaviorVersion::latest())
        .endpoint_url("https://eodata.dataspace.copernicus.eu")
        .credentials_provider(creds)
        .region(Region::new("default"))
        .force_path_style(true)
        .build();
    S3Client::from_conf(config)
}

// ── STAC search ───────────────────────────────────────────────────────────────

async fn search_scenes(client: &Client, lon: f64, lat: f64) -> Vec<Value> {
    let delta = 0.01;
    let bbox = format!(
        "{},{},{},{}",
        lon - delta,
        lat - delta,
        lon + delta,
        lat + delta
    );
    let url = format!("{}/collections/sentinel-2-l2a/items", STAC_BASE);

    let raw = client
        .get(&url)
        .query(&[
            ("bbox", bbox.as_str()),
            ("limit", "5"),
            ("sortby", "-properties.datetime"),
            ("filter-lang", "cql2-text"),
            ("filter", "eo:cloud_cover < 80"),
        ])
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let json: Value = serde_json::from_str(&raw).unwrap();

    let features = match json["features"].as_array() {
        None => {
            eprintln!(
                "Unexpected response: {}",
                &raw[..raw.len().min(500)]
            );
            return vec![];
        }
        Some(f) if f.is_empty() => {
            println!("No scenes found for this location.");
            return vec![];
        }
        Some(f) => f,
    };

    println!("Found {} scenes:", features.len());
    for scene in features {
        let id = scene["id"].as_str().unwrap_or("?");
        let date = scene["properties"]["datetime"].as_str().unwrap_or("?");
        let cloud = scene["properties"]["eo:cloud_cover"]
            .as_f64()
            .unwrap_or(-1.0);
        println!("  {} | cloud: {:.1}% | {}", date, cloud, id);
    }

    features.clone()
}

// ── S3 download ───────────────────────────────────────────────────────────────

fn parse_s3_href(href: &str) -> Option<(String, String)> {
    let path = href.strip_prefix("s3://")?;
    let (bucket, key) = path.split_once('/')?;
    Some((bucket.to_string(), key.to_string()))
}

fn find_tci_href<'a>(scene: &'a Value) -> Option<&'a str> {
    let assets = scene["assets"].as_object()?;

    for key in &["TCI_10m", "TCI", "visual"] {
        if let Some(href) = assets.get(*key).and_then(|a| a["href"].as_str()) {
            return Some(href);
        }
    }

    eprintln!(
        "No TCI asset found. Available assets: {:?}",
        assets.keys().collect::<Vec<_>>()
    );
    None
}

async fn download_tci(s3: &S3Client, scene: &Value, output_dir: &str) {
    let id = scene["id"].as_str().unwrap_or("unknown");
    let date = scene["properties"]["datetime"]
        .as_str()
        .unwrap_or("unknown");

    println!("\nProcessing: {}", id);

    let href = match find_tci_href(scene) {
        Some(h) => h,
        None => return,
    };

    let (bucket, key) = match parse_s3_href(href) {
        Some(p) => p,
        None => {
            eprintln!("Could not parse S3 path: {}", href);
            return;
        }
    };

    println!("Fetching s3://{}/{}", bucket, key);

    match s3.get_object().bucket(&bucket).key(&key).send().await {
        Err(e) => eprintln!("S3 error: {}", e),
        Ok(resp) => {
            let bytes = resp.body.collect().await.unwrap().into_bytes();
            let safe_date = date.replace(":", "-");
            let filename = format!("{}/{}_{}.jp2", output_dir, &safe_date[..10], &id[..10]);
            std::fs::write(&filename, &bytes).unwrap();
            println!("Saved {} MB → {}", bytes.len() / 1_000_000, filename);
        }
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect(".env file not found");

    let username  = env::var("CDSE_USERNAME").expect("CDSE_USERNAME not set");
    let password  = env::var("CDSE_PASSWORD").expect("CDSE_PASSWORD not set");
    let s3_access = env::var("CDSE_S3_ACCESS").expect("CDSE_S3_ACCESS not set");
    let s3_secret = env::var("CDSE_S3_SECRET").expect("CDSE_S3_SECRET not set");

    // Location to search — Wellington, NZ
    let lon = 23.284044;
    let lat = 54.227850;

    // Output directory for downloaded TCI files
    let output_dir = "output";
    std::fs::create_dir_all(output_dir).unwrap();

    let http = Client::new();
    let s3   = build_s3_client(&s3_access, &s3_secret);

    println!("Authenticating...");
    let _token = get_token(&http, &username, &password).await;

    println!("\nSearching for last 5 scenes over ({}, {})...", lon, lat);
    let scenes = search_scenes(&http, lon, lat).await;

    for scene in &scenes {
        download_tci(&s3, scene, output_dir).await;
    }

    println!("\nDone. JP2 files saved to ./{}/", output_dir);
    println!("Open them in QGIS, ArcGIS, or any GIS viewer for full quality.");
}