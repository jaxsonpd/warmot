use serde::Serialize;
use tauri::State;
use crate::state::AppState;

#[derive(Serialize)]
pub struct S1Scene {
    pub id: String,
    pub datetime: String,
    pub orbit_direction: String,
    pub png_b64: String, // false-colour composite
}

#[tauri::command]
pub async fn fetch_sentinel1(
    state: State<'_, AppState>,
    username: String,
    password: String,
    s3_access: String,
    s3_secret: String,
) -> Result<Vec<S1Scene>, String> {
    let _query = state.query.lock().unwrap().clone();
    // TODO: wire up Sentinel-1 GRD collection via CopernicusClient
    Err("Sentinel-1 not yet implemented".into())
}