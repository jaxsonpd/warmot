//! # jp2_convert
//!
//! Converts JPEG 2000 (`.jp2`) satellite imagery to PNG using **pure Rust** —
//! no system libraries, no OpenJPEG, no C dependencies.
//!
//! Backed by [`hayro-jpeg2000`](https://crates.io/crates/hayro-jpeg2000), a
//! memory-safe pure-Rust JPEG 2000 decoder.
//!
//! ## Cargo.toml
//!
//! ```toml
//! [dependencies]
//! hayro-jpeg2000 = { version = "0.3", features = ["image"] }
//! image          = "0.25"
//! thiserror      = "1"
//! ```
//!
//! No system packages required — `cargo build` is all you need.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use jp2_convert::{convert_file, convert_bytes};
//!
//! // File → file
//! convert_file("scene.jp2", "scene.png")?;
//!
//! // In-memory bytes → PNG bytes (pipe straight from S3 download)
//! let png_bytes = convert_bytes(&jp2_bytes)?;
//! std::fs::write("scene.png", png_bytes)?;
//! ```
//!
//! ## Note on bit depth
//!
//! `hayro-jpeg2000` decodes JP2 files to **8-bit** output (with ICC/colour-space
//! correction applied).  The resulting PNG is therefore always 8-bit per channel.
//! If you need to preserve the raw 16-bit values from Sentinel-2 bands for
//! scientific analysis, you will need a different tool (e.g. GDAL).  For visual
//! use — previewing scenes, web display, quick inspection — 8-bit is fine.

use hayro_jpeg2000::{DecodeSettings, Image};
use image::{DynamicImage, ImageDecoder, ImageFormat};
use std::io::Cursor;
use std::path::Path;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JP2 decode error: {0}")]
    Decode(String),

    #[error("Image encode error: {0}")]
    Encode(#[from] image::ImageError),
}

pub type Result<T> = std::result::Result<T, Error>;

// ── Public API ────────────────────────────────────────────────────────────────

/// Convert a JP2 file on disk to a PNG file on disk.
///
/// Any missing parent directories for `output_path` are created automatically.
pub fn convert_file(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<()> {
    let bytes = std::fs::read(input_path)?;
    let png = convert_bytes(&bytes)?;

    let output_path = output_path.as_ref();
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(output_path, png)?;
    Ok(())
}

/// Convert JP2 bytes to PNG bytes entirely in memory.
///
/// Plug this directly into the `copernicus` module:
/// ```rust,no_run
/// let asset = client.get_image(&scene, "TCI_10m").await?;
/// let png = jp2_convert::convert_bytes(&asset.bytes)?;
/// std::fs::write("scene.png", png)?;
/// ```
pub fn convert_bytes(jp2_bytes: &[u8]) -> Result<Vec<u8>> {
    let image = decode_jp2(jp2_bytes)?;
    encode_png(image)
}

/// Decode JP2 bytes to a [`DynamicImage`] without further processing.
///
/// Use this if you want to crop, resize, or otherwise manipulate the image
/// using the `image` crate before saving.
pub fn decode_to_image(jp2_bytes: &[u8]) -> Result<DynamicImage> {
    decode_jp2(jp2_bytes)
}

// ── Internal: decode ──────────────────────────────────────────────────────────

fn decode_jp2(bytes: &[u8]) -> Result<DynamicImage> {
    // hayro's Image implements ImageDecoder; from_decoder drives it into a DynamicImage.
    let decoder = Image::new(bytes, &DecodeSettings::default())
            .map_err(|e: hayro_jpeg2000::DecodeError| Error::Decode(e.to_string()))?;

    let color_type = decoder.color_type();
    let (width, height) = decoder.dimensions();
    let mut buf = vec![0u8; (width * height * color_type.channel_count() as u32) as usize];
    decoder
        .read_image(&mut buf)
        .map_err(|e: image::ImageError| Error::Decode(e.to_string()))?;


    // Reconstruct a DynamicImage from the raw 8-bit buffer.
    // hayro always outputs 8-bit (L8, La8, Rgb8, or Rgba8).
    use image::ColorType::*;
    let img = match color_type {
        L8 => DynamicImage::ImageLuma8(
            image::ImageBuffer::from_raw(width, height, buf)
                .ok_or_else(|| Error::Decode("buffer size mismatch (L8)".into()))?,
        ),
        La8 => DynamicImage::ImageLumaA8(
            image::ImageBuffer::from_raw(width, height, buf)
                .ok_or_else(|| Error::Decode("buffer size mismatch (La8)".into()))?,
        ),
        Rgb8 => DynamicImage::ImageRgb8(
            image::ImageBuffer::from_raw(width, height, buf)
                .ok_or_else(|| Error::Decode("buffer size mismatch (Rgb8)".into()))?,
        ),
        Rgba8 => DynamicImage::ImageRgba8(
            image::ImageBuffer::from_raw(width, height, buf)
                .ok_or_else(|| Error::Decode("buffer size mismatch (Rgba8)".into()))?,
        ),
        other => {
            return Err(Error::Decode(format!("unexpected color type from hayro: {:?}", other)));
        }
    };

    Ok(img)
}

// ── Internal: encode ──────────────────────────────────────────────────────────

fn encode_png(image: DynamicImage) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    image.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)?;
    Ok(buf)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_on_invalid_bytes() {
        let result = convert_bytes(b"not a jp2 file");
        assert!(result.is_err());
    }

    #[test]
    fn error_message_is_readable() {
        let err = convert_bytes(b"garbage").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("JP2 decode error"));
    }
}