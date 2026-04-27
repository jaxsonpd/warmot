use serde::{Deserialize, Serialize};
use tauri::State;
use warmot::copernicus::{
    BoundingBox, CollectionType, CopernicusClient, SearchParams, SortBy,
};
use warmot::jp2_convert::{convert_bytes, convert_bytes_with_speed, PngSpeed};
use std::env;

use crate::state::AppState;

#[derive(Serialize)]
pub struct S2Scene {
    pub id: String,
    pub datetime: String,
    pub cloud_cover: f64,
    /// Base64-encoded PNG bytes
    pub png_b64: String,
}

#[tauri::command]
pub async fn fetch_sentinel2(
    state: State<'_, AppState>,
    username: String,
    password: String,
    s3_access: String,
    s3_secret: String,
) -> Result<Vec<S2Scene>, String> {
    let query = state.query.lock().unwrap().clone();

    log::info!("Searching sentinel2 using {:?}", query);

    // ── 1. init ───────────────────────────────────────────────────────────────
    let client = CopernicusClient::init(
        &env::var("CDSE_USERNAME").unwrap(),
        &env::var("CDSE_PASSWORD").unwrap(),
        &env::var("CDSE_S3_ACCESS").unwrap(),
        &env::var("CDSE_S3_SECRET").unwrap(),
    )
    .await
    .map_err(|e| e.to_string())?;

    let bbox = BoundingBox::around(query.lon, query.lat, query.radius_deg);

    log::debug!("Searching for BoundingBox {:?}", bbox);

    let scenes = client
        .search(SearchParams {
            collection: CollectionType::Sentinel2L2A,
            bbox,
            limit: 5,
            max_cloud_cover: Some(30.0),
            sort_by: SortBy::DateDescending,
        })
        .await
        .map_err(|e| e.to_string())?;

    log::debug!("Found scenes {:?}", scenes);

    let mut results = Vec::new();

    for scene in &scenes {
        log::debug!("Fetching {:?}", scene);
        let asset = client
            .get_image_fallback(scene, &["TCI_10m", "TCI", "visual"])
            .await
            .map_err(|e| e.to_string())?;

        log::debug!("Converting to png");
        let png = convert_bytes_with_speed(&asset.bytes, PngSpeed::Fast).map_err(|e| e.to_string())?;
        let png_b64 = base64_encode(&png);

        results.push(S2Scene {
            id: scene.id.clone(),
            datetime: scene.datetime.clone(),
            cloud_cover: scene.cloud_cover.unwrap_or(-1.0),
            png_b64,
        });
    }

    Ok(results)
}

fn base64_encode(data: &[u8]) -> String {
    use std::io::Write;
    // Use the `base64` crate or roll a simple encode.
    // Add `base64 = "0.22"` to Cargo.toml.
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}