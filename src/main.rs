use warmot::copernicus::{BoundingBox, CollectionType, CopernicusClient, SearchParams, SortBy};
use warmot::jp2_convert::{convert_file, convert_bytes};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    // ── 1. init ───────────────────────────────────────────────────────────────
    let client = CopernicusClient::init(
        &env::var("CDSE_USERNAME")?,
        &env::var("CDSE_PASSWORD")?,
        &env::var("CDSE_S3_ACCESS")?,
        &env::var("CDSE_S3_SECRET")?,
    )
    .await?;
    println!("Authenticated ✓");

    // ── 2. search – optical (S2C MSIL2A) ─────────────────────────────────────
    let wellington = BoundingBox::around(174.77, -41.29, 0.05);

    let optical_scenes = client
        .search(SearchParams {
            collection: CollectionType::Sentinel2L2A,
            bbox: wellington,
            limit: 5,
            max_cloud_cover: Some(30.0),
            sort_by: SortBy::DateDescending,
        })
        .await?;

    println!("\nOptical scenes (S2 L2A, ≤30% cloud):");
    for s in &optical_scenes {
        println!(
            "  {} | cloud: {:.1}% | {}",
            s.datetime,
            s.cloud_cover.unwrap_or(-1.0),
            s.id
        );
    }

    if optical_scenes.len() != 0 {
        for scene in optical_scenes.iter() {
            println!("\nDownloading TCI from: {}", scene.id);

            // Try keys in order of preference
            let asset = client
                .get_image_fallback(scene, &["TCI_10m", "TCI", "visual"])
                .await?;

            println!("Downloaded {} MB", asset.bytes.len() / 1_000_000);

            // ── 4. save_jp2 ───────────────────────────────────────────────────────
            let saved_path = client.save_jp2(&asset, "output/").await?;
            convert_file(&saved_path, format!("{}.png", saved_path))?;
            println!("Saved → {}", saved_path);


            // Or with an explicit path:
            // let saved_path = client.save_jp2(&asset, "output/wellington_tci.jp2").await?;
        }
    }

    Ok(())
}