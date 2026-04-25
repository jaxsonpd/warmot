use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryParams {
    pub lon: f64,
    pub lat: f64,
    pub radius_deg: f64,   // half-width of bounding box in degrees
    pub date_from: String, // "YYYY-MM-DD"
    pub date_to: String,   // "YYYY-MM-DD"
}

/// Tauri managed state — wrapped in Mutex for interior mutability
pub struct AppState {
    pub query: Mutex<QueryParams>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            query: Mutex::new(QueryParams {
                lon: 174.77,
                lat: -41.29,
                radius_deg: 0.05,
                date_from: "2025-01-01".into(),
                date_to: "2025-04-01".into(),
            }),
        }
    }
}