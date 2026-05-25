use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Serialize)]
struct ProgressPayload {
    step: &'static str,
    current: usize,
    total: usize,
    scene_id: Option<String>,
}

#[tauri::command]
pub async fn fetch_sentinel2(
    app: AppHandle,
    state: State<'_, AppState>,
    username: String,
    password: String,
    s3_access: String,
    s3_secret: String,
) -> Result<Vec<S2Scene>, String> {
    let query = state.query.lock().unwrap().clone();

    let emit = |step: &'static str, current: usize, total: usize, scene_id: Option<String>| {
        let _ = app.emit("sentinel2-progress", ProgressPayload { step, current, total, scene_id });
    };

    emit("init", 0, 1, None);

    // ── 1. init ───────────────────────────────────────────────────────────────
    let client = CopernicusClient::init(&username, &password, &s3_access, &s3_secret)
        .await
        .map_err(|e| e.to_string())?;

    let bbox = BoundingBox::around(query.lat, query.lon, query.radius_deg);

    emit("searching", 0, 1, None);

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

    let total = scenes.len();
    let mut results = Vec::new();

    for (i, scene) in scenes.iter().enumerate() {
        emit("downloading", i, total, Some(scene.id.clone()));

        let asset = client
            .get_image_fallback(scene, &["TCI_10m", "TCI", "visual"])
            .await
            .map_err(|e| e.to_string())?;

        emit("converting", i, total, Some(scene.id.clone()));

        let png_b64 = convert_bytes(&asset.bytes).map_err(|e| e.to_string())?;

        results.push(S2Scene {
            id: scene.id.clone(),
            datetime: scene.datetime.clone(),
            cloud_cover: scene.cloud_cover.unwrap_or(-1.0),
            png_b64,
        });

        emit("done_scene", i + 1, total, Some(scene.id.clone()));
    }

    emit("complete", total, total, None);
    Ok(results)
}