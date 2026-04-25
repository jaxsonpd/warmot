use serde::Serialize;
use tauri::State;
use crate::state::AppState;

#[derive(Serialize)]
pub struct AisVessel {
    pub mmsi: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub speed: f64,
    pub heading: f64,
    pub timestamp: String,
}

#[tauri::command]
pub async fn fetch_ais(
    state: State<'_, AppState>,
) -> Result<Vec<AisVessel>, String> {
    let _query = state.query.lock().unwrap().clone();
    // TODO: call AISHub or BarentsWatch API within bounding box + time range
    Err("AIS not yet implemented".into())
}