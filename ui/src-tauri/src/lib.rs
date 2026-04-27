mod commands;
mod state;

use commands::{ais::fetch_ais, sentinel1::fetch_sentinel1, 
               sentinel2::fetch_sentinel2, weather::fetch_weather};
use state::AppState;
use tauri::State;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct QueryUpdate {
    lon: f64,
    lat: f64,
    radius_deg: f64,
    date_from: String,
    date_to: String,
}

/// Called by the frontend whenever the location/time form changes.
#[tauri::command]
fn update_query(state: State<'_, AppState>, params: QueryUpdate) -> Result<(), String> {
    log::info!("Updating query with {:?}", params);
    let mut q = state.query.lock().unwrap();
    q.lon = params.lon;
    q.lat = params.lat;
    q.radius_deg = params.radius_deg;
    q.date_from = params.date_from;
    q.date_to = params.date_to;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();    
    env_logger::init();

    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            update_query,
            fetch_sentinel2,
            fetch_sentinel1,
            fetch_weather,
            fetch_ais,
        ])
        .run(tauri::generate_context!())
        .expect("error running warmot");
}