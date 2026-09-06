//! Module for converting JP2 (JPEG 2000) files to other formats.
//!
//! Provides both file-based and in-memory conversion to PNG, using the
//! `jpeg2k` crate for decoding and the `png` crate for encoding directly.
//! The `png` crate is used in preference to `image`'s built-in PNG support
//! because it exposes compression and filter controls, allowing the encoder
//! to be tuned for speed rather than file size.
//!
//! Both paths use [`Compression::Fast`] (fdeflate-backed in `png` 0.18+) and
//! [`Filter::NoFilter`], prioritising encode speed over output size.

use std::path::Path;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::{ImageBuffer, Rgb};
use jpeg2k::Image;
use png::{BitDepth, ColorType, Compression, Encoder, Filter};

/// Decode a JPEG 2000 byte slice and return a PNG data URL.
///
/// The returned string is a base64-encoded `data:image/png;base64,...` URL
/// suitable for use directly as the `src` attribute of an `<img>` element in
/// the Tauri frontend.
///
/// # Errors
///
/// Returns an error if the input bytes cannot be decoded as a JPEG 2000 image,
/// or if PNG encoding fails.
pub fn convert_bytes(jp2_bytes: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let jp2 = Image::from_bytes(jp2_bytes)?;
    let img = decode_to_rgb(&jp2);

    let png_bytes = encode_png(&img)?;
    let b64 = BASE64.encode(&png_bytes);
    Ok(format!("data:image/png;base64,{}", b64))
}

/// Decode a JPEG 2000 file and save a PNG alongside the source.
///
/// The output file is written to the same directory as `jp2_file`, with the
/// same stem and a `.png` extension (e.g. `image.jp2` → `image.png`).
///
/// # Errors
///
/// Returns an error if the file cannot be read or decoded as a JPEG 2000
/// image, or if the PNG output file cannot be written.
pub fn convert_file(jp2_file: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Converting: {}", jp2_file.display());

    let jp2 = Image::from_file(jp2_file)?;
    let img = decode_to_rgb(&jp2);

    let png_path = jp2_file.with_extension("png");
    let file = std::fs::File::create(&png_path)?;
    let mut writer = std::io::BufWriter::new(file);
    write_png(&img, &mut writer)?;

    println!("Saved: {}", png_path.display());
    Ok(())
}

/// Encode an RGB image buffer as PNG bytes.
///
/// Uses [`Compression::Fast`] and [`Filter::NoFilter`] to minimise encode
/// time. The resulting bytes are larger than a fully compressed PNG but encode
/// significantly faster, appropriate for both transient display and file
/// output where speed is the priority.
fn encode_png(img: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let capacity = (img.width() * img.height() * 3) as usize;
    let mut png_bytes = Vec::with_capacity(capacity);
    write_png(img, &mut png_bytes)?;
    Ok(png_bytes)
}

/// Write PNG-encoded image data to any [`std::io::Write`] destination.
///
/// Shared by both [`encode_png`] (in-memory) and [`convert_file`] (disk),
/// keeping encoder settings consistent across both paths.
fn write_png<W: std::io::Write>(
    img: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    writer: &mut W,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut encoder = Encoder::new(writer, img.width(), img.height());
    encoder.set_color(ColorType::Rgb);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_compression(Compression::Fast);
    encoder.set_filter(Filter::NoFilter);

    let mut png_writer = encoder.write_header()?;
    png_writer.write_image_data(img.as_raw())?;
    drop(png_writer);

    Ok(())
}

/// Decode a [`jpeg2k::Image`] into an 8-bit-per-channel RGB [`ImageBuffer`].
///
/// This is the shared decode path used by both [`convert_bytes`] and
/// [`convert_file`]. It handles two cases:
///
/// - **1 component** — grayscale; the single channel is replicated into R, G,
///   and B.
/// - **2+ components** — the first three components are mapped to R, G, and B.
///   Any additional components (e.g. alpha) are ignored.
///
/// Samples with a precision greater than 8 bits are scaled down by
/// right-shifting so that the most-significant 8 bits are retained.
fn decode_to_rgb(jp2: &Image) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let width = jp2.width();
    let height = jp2.height();
    let components = jp2.components();

    match components.len() {
        1 => {
            // Grayscale — replicate the single channel into R, G, B.
            let comp = &components[0];
            let shift = (comp.precision() as i32 - 8).max(0) as u32;
            ImageBuffer::from_fn(width, height, |x, y| {
                let idx = (y * width + x) as usize;
                let v = (comp.data()[idx] >> shift) as u8;
                Rgb([v, v, v])
            })
        }
        _ => {
            // Multi-component — use first three channels as R, G, B.
            let shift = (components[0].precision() as i32 - 8).max(0) as u32;
            ImageBuffer::from_fn(width, height, |x, y| {
                let idx = (y * width + x) as usize;
                let r = (components[0].data()[idx] >> shift) as u8;
                let g = (components[1].data()[idx] >> shift) as u8;
                let b = (components[2].data()[idx] >> shift) as u8;
                Rgb([r, g, b])
            })
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("test.jp2")
    }

    /// Copy the fixture JP2 to a temp file and return its path. The caller
    /// owns the temp file and is responsible for cleaning it up. Each test
    /// gets its own copy so parallel runs don't interfere with each other.
    fn temp_jp2(test_name: &str) -> PathBuf {
        let dir = std::env::temp_dir();
        let dest = dir.join(format!("warmot_test_{}.jp2", test_name));
        std::fs::copy(fixture_path(), &dest).expect("failed to copy fixture to temp dir");
        dest
    }

    // -------------------------------------------------------------------------
    // convert_bytes
    // -------------------------------------------------------------------------

    #[test]
    fn convert_bytes_returns_data_url_prefix() {
        let jp2 = std::fs::read(fixture_path()).expect("fixture file missing");
        let result = convert_bytes(&jp2).expect("convert_bytes failed");
        assert!(
            result.starts_with("data:image/png;base64,"),
            "expected data URL prefix, got: {}",
            &result[..result.len().min(64)]
        );
    }

    #[test]
    fn convert_bytes_base64_payload_is_valid_png() {
        let jp2 = std::fs::read(fixture_path()).expect("fixture file missing");
        let data_url = convert_bytes(&jp2).expect("convert_bytes failed");

        let b64 = data_url
            .strip_prefix("data:image/png;base64,")
            .expect("missing prefix");

        let png_bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .expect("base64 decode failed");

        assert_eq!(
            &png_bytes[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "decoded bytes do not have a PNG signature"
        );
    }

    #[test]
    fn convert_bytes_output_dimensions_match_input() {
        let jp2_bytes = std::fs::read(fixture_path()).expect("fixture file missing");
        let jp2 = Image::from_bytes(&jp2_bytes).expect("failed to decode JP2");
        let expected_width = jp2.width();
        let expected_height = jp2.height();

        let data_url = convert_bytes(&jp2_bytes).expect("convert_bytes failed");
        let b64 = data_url.strip_prefix("data:image/png;base64,").unwrap();
        let png_bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .expect("base64 decode failed");

        let decoded = image::load_from_memory(&png_bytes).expect("failed to load PNG");
        assert_eq!(decoded.width(), expected_width);
        assert_eq!(decoded.height(), expected_height);
    }

    #[test]
    fn convert_bytes_rejects_invalid_input() {
        assert!(convert_bytes(b"not a jp2").is_err());
    }

    #[test]
    fn convert_bytes_empty_input_returns_error() {
        assert!(convert_bytes(&[]).is_err());
    }

    // -------------------------------------------------------------------------
    // convert_file
    // -------------------------------------------------------------------------

    #[test]
    fn convert_file_creates_png_alongside_source() {
        let jp2_path = temp_jp2("creates_png");
        let png_path = jp2_path.with_extension("png");

        convert_file(&jp2_path).expect("convert_file failed");

        assert!(png_path.exists(), "expected PNG file to be created");

        std::fs::remove_file(&jp2_path).ok();
        std::fs::remove_file(&png_path).ok();
    }

    #[test]
    fn convert_file_output_is_valid_png() {
        let jp2_path = temp_jp2("valid_png");
        let png_path = jp2_path.with_extension("png");

        convert_file(&jp2_path).expect("convert_file failed");

        let png_bytes = std::fs::read(&png_path).expect("could not read output PNG");
        assert_eq!(
            &png_bytes[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "output file does not have a PNG signature"
        );

        std::fs::remove_file(&jp2_path).ok();
        std::fs::remove_file(&png_path).ok();
    }

    #[test]
    fn convert_file_output_dimensions_match_input() {
        let jp2_path = temp_jp2("dimensions");
        let png_path = jp2_path.with_extension("png");

        let jp2_bytes = std::fs::read(&jp2_path).expect("fixture file missing");
        let jp2 = Image::from_bytes(&jp2_bytes).expect("failed to decode JP2");
        let expected_width = jp2.width();
        let expected_height = jp2.height();

        convert_file(&jp2_path).expect("convert_file failed");

        let decoded = image::open(&png_path).expect("failed to open output PNG");
        assert_eq!(decoded.width(), expected_width);
        assert_eq!(decoded.height(), expected_height);

        std::fs::remove_file(&jp2_path).ok();
        std::fs::remove_file(&png_path).ok();
    }

    #[test]
    fn convert_file_extension_is_always_png() {
        let jp2_path = temp_jp2("extension");
        let png_path = jp2_path.with_extension("png");

        convert_file(&jp2_path).expect("convert_file failed");

        assert_eq!(png_path.extension().and_then(|e| e.to_str()), Some("png"));

        std::fs::remove_file(&jp2_path).ok();
        std::fs::remove_file(&png_path).ok();
    }

    #[test]
    fn convert_file_nonexistent_input_returns_error() {
        let bad_path = PathBuf::from("/nonexistent/path/to/file.jp2");
        assert!(convert_file(&bad_path).is_err());
    }

    // -------------------------------------------------------------------------
    // Consistency
    // -------------------------------------------------------------------------

    #[test]
    fn repeated_encode_is_deterministic() {
        let jp2 = std::fs::read(fixture_path()).expect("fixture file missing");
        let first = convert_bytes(&jp2).expect("first encode failed");
        let second = convert_bytes(&jp2).expect("second encode failed");
        assert_eq!(first, second, "encoding the same input twice gave different output");
    }

    #[test]
    fn output_is_rgb_not_rgba() {
        let jp2_bytes = std::fs::read(fixture_path()).expect("fixture file missing");
        let data_url = convert_bytes(&jp2_bytes).expect("convert_bytes failed");
        let b64 = data_url.strip_prefix("data:image/png;base64,").unwrap();
        let png_bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .expect("base64 decode failed");

        let decoded = image::load_from_memory(&png_bytes).expect("failed to load PNG");
        assert!(
            matches!(decoded.color(), image::ColorType::Rgb8),
            "expected Rgb8 output, got {:?}",
            decoded.color()
        );
    }
}