use serde::Serialize;
use tauri::State;
use crate::state::AppState;

#[derive(Serialize)]
pub struct WeatherPoint {
    pub time: String,
    pub temperature_2m: f64,
    pub precipitation: f64,
    pub windspeed_10m: f64,
}

#[tauri::command]
pub async fn fetch_weather(
    state: State<'_, AppState>,
) -> Result<Vec<WeatherPoint>, String> {
    let _query = state.query.lock().unwrap().clone();
    // TODO: call Open-Meteo archive API with query.lat/lon and date range
    Err("Weather not yet implemented".into())
}