//! # copernicus
//!
//! A client module for the Copernicus Data Space Ecosystem (CDSE).
//!
//! Supports STAC search over the following collection types (see [`CollectionType`])
//! and S3-based asset download.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use copernicus::{CopernicusClient, CollectionType, BoundingBox, SearchParams};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), copernicus::Error> {
//!     let client = CopernicusClient::init(
//!         "user@example.com",
//!         "password",
//!         "s3-access-key",
//!         "s3-secret-key",
//!     ).await?;
//!
//!     let bbox = BoundingBox::new(174.7, -41.3, 174.8, -41.2);
//!
//!     let scenes = client.search(SearchParams {
//!         collection: CollectionType::Sentinel2L2A,
//!         bbox,
//!         limit: 5,
//!         max_cloud_cover: Some(30.0),
//!         ..Default::default()
//!     }).await?;
//!
//!     if let Some(scene) = scenes.first() {
//!         let asset = client.get_image(scene, "TCI_10m").await?;
//!         client.save_jp2(&asset, "output/scene.jp2").await?;
//!     }
//!
//!     Ok(())
//! }
//! ```

use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Builder};
use aws_sdk_s3::Client as S3Client;
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

// ── Constants ─────────────────────────────────────────────────────────────────

const TOKEN_URL: &str =
    "https://identity.dataspace.copernicus.eu/auth/realms/CDSE/protocol/openid-connect/token";
const STAC_BASE: &str = "https://stac.dataspace.copernicus.eu/v1";
const S3_ENDPOINT: &str = "https://eodata.dataspace.copernicus.eu";

// ── Error type ────────────────────────────────────────────────────────────────

/// All errors that can be produced by this module.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("S3 error: {0}")]
    S3(String),

    #[error("Authentication failed: {error} – {description}")]
    Auth { error: String, description: String },

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Asset not found: tried keys {tried:?}")]
    AssetNotFound { tried: Vec<String> },

    #[error("Invalid S3 path: {0}")]
    InvalidS3Path(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("No scenes returned from STAC")]
    NoScenes,
}

pub type Result<T> = std::result::Result<T, Error>;

// ── Collection types ──────────────────────────────────────────────────────────

/// Satellite data collections available on CDSE.
///
/// Pass one of these variants to [`SearchParams::collection`] to choose
/// what kind of imagery you want to search for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CollectionType {
    /// Sentinel-2 Level-2A multispectral (optical, 10–60 m). **S2C MSIL2A.**
    Sentinel2L2A,
    /// Sentinel-1 GRD SAR (C-band radar, ~10 m).
    Sentinel1Grd,
    /// Sentinel-1 SLC SAR (interferometric coherence).
    Sentinel1Slc,
    /// Sentinel-3 OLCI ocean/land colour (300 m).
    Sentinel3Olci,
    /// Sentinel-3 SLSTR sea/land surface temperature.
    Sentinel3Slstr,
    /// Sentinel-5P tropospheric trace gas products.
    Sentinel5pL2,
    /// Any other STAC collection – supply the raw collection ID string.
    Custom(String),
}

impl CollectionType {
    /// Returns the STAC collection ID used in API URLs.
    pub fn stac_id(&self) -> &str {
        match self {
            Self::Sentinel2L2A => "sentinel-2-l2a",
            Self::Sentinel1Grd => "sentinel-1-grd",
            Self::Sentinel1Slc => "sentinel-1-slc",
            Self::Sentinel3Olci => "sentinel-3-olci",
            Self::Sentinel3Slstr => "sentinel-3-slstr",
            Self::Sentinel5pL2 => "sentinel-5p-l2",
            Self::Custom(id) => id.as_str(),
        }
    }
}

// ── BoundingBox ───────────────────────────────────────────────────────────────

/// A geographic bounding box in WGS-84 decimal degrees.
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

impl BoundingBox {
    /// Create a bounding box from (west, south, east, north).
    pub fn new(west: f64, south: f64, east: f64, north: f64) -> Self {
        Self { west, south, east, north }
    }

    /// Create a square bounding box centred on a point with a given half-width
    /// in degrees (e.g. `0.05` ≈ ~5 km at mid-latitudes).
    pub fn around(lon: f64, lat: f64, delta: f64) -> Self {
        Self::new(lon - delta, lat - delta, lon + delta, lat + delta)
    }

    fn as_query_string(&self) -> String {
        format!("{},{},{},{}", self.west, self.south, self.east, self.north)
    }
}

// ── SearchParams ──────────────────────────────────────────────────────────────

/// Parameters for a STAC scene search.
#[derive(Debug, Clone)]
pub struct SearchParams {
    /// Which satellite collection to query.
    pub collection: CollectionType,
    /// Area of interest.
    pub bbox: BoundingBox,
    /// Maximum number of results to return (default: 10).
    pub limit: usize,
    /// Reject scenes with cloud cover above this percentage (0–100).
    /// Only meaningful for optical sensors; pass `None` for radar/trace-gas.
    pub max_cloud_cover: Option<f64>,
    /// Sort order for results. Defaults to most-recent first.
    pub sort_by: SortBy,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            collection: CollectionType::Sentinel2L2A,
            bbox: BoundingBox::around(0.0, 0.0, 0.1),
            limit: 10,
            max_cloud_cover: Some(80.0),
            sort_by: SortBy::DateDescending,
        }
    }
}

/// Sort order for STAC results.
#[derive(Debug, Clone, Copy, Default)]
pub enum SortBy {
    #[default]
    DateDescending,
    DateAscending,
}

impl SortBy {
    fn as_str(&self) -> &'static str {
        match self {
            Self::DateDescending => "-properties.datetime",
            Self::DateAscending => "+properties.datetime",
        }
    }
}

// ── Scene / Asset types ───────────────────────────────────────────────────────

/// A STAC scene item returned from a search.
#[derive(Clone)]
pub struct Scene {
    /// STAC item ID (e.g. `S2B_MSIL2A_20250401T...`).
    pub id: String,
    /// Acquisition datetime (ISO 8601).
    pub datetime: String,
    /// Cloud cover percentage, if available.
    pub cloud_cover: Option<f64>,
    /// Raw STAC item JSON – use this to inspect available assets.
    pub raw: Value,
}

impl Scene {
    fn from_value(v: Value) -> Option<Self> {
        let id = v["id"].as_str()?.to_string();
        let datetime = v["properties"]["datetime"].as_str()?.to_string();
        let cloud_cover = v["properties"]["eo:cloud_cover"].as_f64();
        Some(Self { id, datetime, cloud_cover, raw: v })
    }

    /// List the asset keys available in this scene.
    pub fn asset_keys(&self) -> Vec<String> {
        self.raw["assets"]
            .as_object()
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl std::fmt::Debug for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Scene {{ id: {}, datetime: {}, cloud_cover: {} }}",
            self.id,
            self.datetime,
            self.cloud_cover.map_or("N/A".to_string(), |c| format!("{:.1}%", c))
        )
    }
}

/// Raw bytes of a downloaded satellite image asset.
pub struct ImageAsset {
    /// The S3 object key (useful for constructing filenames).
    pub key: String,
    /// The scene this asset belongs to.
    pub scene_id: String,
    /// Raw bytes (typically a JP2 file).
    pub bytes: bytes::Bytes,
}

// ── Auth response ─────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

// ── CopernicusClient ──────────────────────────────────────────────────────────

/// The main entry point for this module.
///
/// Obtain one via [`CopernicusClient::init`], then call [`search`](Self::search),
/// [`get_image`](Self::get_image), and [`save_jp2`](Self::save_jp2).
pub struct CopernicusClient {
    http: HttpClient,
    s3: S3Client,
    /// The current bearer token (refreshed on [`init`](Self::init)).
    pub token: String,
}

impl CopernicusClient {
    // ── init ──────────────────────────────────────────────────────────────────

    /// Initialise the client: authenticate with CDSE and build an S3 client.
    ///
    /// Credentials are for the [Copernicus Data Space](https://dataspace.copernicus.eu)
    /// account; S3 keys are from the same portal under *User Settings → S3 Access*.
    pub async fn init(
        username: &str,
        password: &str,
        s3_access_key: &str,
        s3_secret_key: &str,
    ) -> Result<Self> {
        let http = HttpClient::new();
        let token = Self::fetch_token(&http, username, password).await?;
        let s3 = Self::build_s3_client(s3_access_key, s3_secret_key);

        Ok(Self { http, s3, token })
    }

    // ── search ────────────────────────────────────────────────────────────────

    /// Search for scenes matching the given parameters.
    ///
    /// Returns a `Vec<Scene>` sorted according to [`SearchParams::sort_by`].
    ///
    /// # Example
    /// ```rust,no_run
    /// # use copernicus::*;
    /// # async fn run(client: CopernicusClient) -> Result<()> {
    /// let scenes = client.search(SearchParams {
    ///     collection: CollectionType::Sentinel1Grd,   // SAR radar
    ///     bbox: BoundingBox::around(174.77, -41.29, 0.05),
    ///     max_cloud_cover: None,                       // not relevant for radar
    ///     ..Default::default()
    /// }).await?;
    /// # Ok(()) }
    /// ```
    pub async fn search(&self, params: SearchParams) -> Result<Vec<Scene>> {
        let url = format!(
            "{}/collections/{}/items",
            STAC_BASE,
            params.collection.stac_id()
        );

        let mut query: Vec<(&str, String)> = vec![
            ("bbox", params.bbox.as_query_string()),
            ("limit", params.limit.to_string()),
            ("sortby", params.sort_by.as_str().to_string()),
            ("filter-lang", "cql2-text".to_string()),
        ];

        if let Some(max_cc) = params.max_cloud_cover {
            query.push(("filter", format!("eo:cloud_cover < {}", max_cc)));
        }

        let raw = self
            .http
            .get(&url)
            .query(&query)
            .send()
            .await?
            .text()
            .await?;

        let json: Value = serde_json::from_str(&raw)?;

        let features = match json["features"].as_array() {
            None => {
                return Err(Error::Json(serde_json::from_str::<Value>("null").unwrap_err()));
            }
            Some(f) => f,
        };

        let scenes: Vec<Scene> = features
            .iter()
            .cloned()
            .filter_map(Scene::from_value)
            .collect();

        Ok(scenes)
    }

    // ── get_image ─────────────────────────────────────────────────────────────

    /// Download a specific asset from a scene via S3.
    ///
    /// `asset_key` is the STAC asset key, e.g.:
    /// - `"TCI_10m"` – true-colour image, 10 m (Sentinel-2)
    /// - `"TCI"` – true-colour fallback
    /// - `"visual"` – another alias
    /// - `"B04_10m"`, `"B08_10m"` – individual bands
    /// - `"measurement/vv"` – Sentinel-1 polarisation
    ///
    /// If you pass multiple fallback keys (comma-separated), the first
    /// one present in the scene's assets will be used.
    ///
    /// Use [`Scene::asset_keys`] to inspect what is available.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use copernicus::*;
    /// # async fn run(client: CopernicusClient, scene: Scene) -> Result<()> {
    /// // Try TCI_10m first, then TCI, then visual
    /// let asset = client.get_image_fallback(&scene, &["TCI_10m", "TCI", "visual"]).await?;
    /// # Ok(()) }
    /// ```
    pub async fn get_image(&self, scene: &Scene, asset_key: &str) -> Result<ImageAsset> {
        self.get_image_fallback(scene, &[asset_key]).await
    }

    /// Like [`get_image`](Self::get_image) but tries each key in order,
    /// returning the first one that exists in the scene's assets.
    pub async fn get_image_fallback(
        &self,
        scene: &Scene,
        asset_keys: &[&str],
    ) -> Result<ImageAsset> {
        let assets = scene.raw["assets"]
            .as_object()
            .ok_or_else(|| Error::AssetNotFound {
                tried: asset_keys.iter().map(|s| s.to_string()).collect(),
            })?;

        let href = asset_keys
            .iter()
            .find_map(|k| assets.get(*k).and_then(|a| a["href"].as_str()))
            .ok_or_else(|| Error::AssetNotFound {
                tried: asset_keys.iter().map(|s| s.to_string()).collect(),
            })?;

        let (bucket, key) = Self::parse_s3_href(href)?;

        let resp = self
            .s3
            .get_object()
            .bucket(&bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| Error::S3(e.to_string()))?;

        let bytes = resp
            .body
            .collect()
            .await
            .map_err(|e| Error::S3(e.to_string()))?
            .into_bytes();

        Ok(ImageAsset {
            key,
            scene_id: scene.id.clone(),
            bytes,
        })
    }

    // ── save_jp2 ──────────────────────────────────────────────────────────────

    /// Write an [`ImageAsset`] to disk as a JP2 file.
    ///
    /// Any missing parent directories are created automatically.
    ///
    /// If `path` ends in `/` or is a directory, a filename is derived from
    /// the scene ID and acquisition date.  Otherwise the path is used as-is.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use copernicus::*;
    /// # async fn run(client: CopernicusClient, asset: ImageAsset) -> Result<()> {
    /// client.save_jp2(&asset, "output/scene.jp2").await?;
    /// // or let the module name the file:
    /// client.save_jp2(&asset, "output/").await?;
    /// # Ok(()) }
    /// ```
    pub async fn save_jp2(&self, asset: &ImageAsset, path: impl AsRef<Path>) -> Result<String> {
        let path = path.as_ref();

        // If path is a directory (or ends with /), auto-generate filename
        let final_path = if path.is_dir() || path.to_string_lossy().ends_with('/') {
            // Derive a safe filename from the S3 key (last component) and scene ID
            let filename = Path::new(&asset.key)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or(&asset.scene_id)
                .to_string();
            path.join(filename)
        } else {
            path.to_path_buf()
        };

        // Ensure the extension is .jp2
        let final_path = if final_path.extension().is_none() {
            final_path.with_extension("jp2")
        } else {
            final_path
        };

        // Create parent dirs
        if let Some(parent) = final_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&final_path, &asset.bytes).await?;

        Ok(final_path.to_string_lossy().into_owned())
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    async fn fetch_token(http: &HttpClient, username: &str, password: &str) -> Result<String> {
        let resp: TokenResponse = http
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "password"),
                ("username", username),
                ("password", password),
                ("client_id", "cdse-public"),
            ])
            .send()
            .await?
            .json()
            .await?;

        resp.access_token.ok_or_else(|| Error::Auth {
            error: resp.error.unwrap_or_default(),
            description: resp.error_description.unwrap_or_default(),
        })
    }

    fn build_s3_client(access_key: &str, secret_key: &str) -> S3Client {
        let creds = Credentials::new(access_key, secret_key, None, None, "cdse");
        let config = Builder::new()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url(S3_ENDPOINT)
            .credentials_provider(creds)
            .region(Region::new("default"))
            .force_path_style(true)
            .build();
        S3Client::from_conf(config)
    }

    fn parse_s3_href(href: &str) -> Result<(String, String)> {
        let path = href
            .strip_prefix("s3://")
            .ok_or_else(|| Error::InvalidS3Path(href.to_string()))?;
        let (bucket, key) = path
            .split_once('/')
            .ok_or_else(|| Error::InvalidS3Path(href.to_string()))?;
        Ok((bucket.to_string(), key.to_string()))
    }
}
