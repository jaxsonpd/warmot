//! [doc comments unchanged]

use hayro_jpeg2000::{DecodeSettings, Image};
use image::{DynamicImage, ImageDecoder, ImageFormat};
use png::{BitDepth, ColorType, Compression, Encoder};
use std::io::Cursor;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JP2 decode error: {0}")]
    Decode(String),
    #[error("Image encode error: {0}")]
    Encode(#[from] image::ImageError),
    #[error("PNG encode error: {0}")]
    PngEncode(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Quality/speed tradeoff for PNG encoding.
#[derive(Debug, Clone, Copy, Default)]
pub enum PngSpeed {
    /// Smallest file, slowest (default `image` crate behaviour).
    Small,
    /// Good balance — recommended for most uses.
    #[default]
    Balanced,
    /// Fastest encode, largest file. Best for previewing/inspection.
    Fast,
}

pub fn convert_file(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<()> {
    convert_file_with_speed(input_path, output_path, PngSpeed::default())
}

pub fn convert_file_with_speed(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    speed: PngSpeed,
) -> Result<()> {
    let bytes = std::fs::read(input_path)?;
    let png = convert_bytes_with_speed(&bytes, speed)?;
    let output_path = output_path.as_ref();
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, png)?;
    Ok(())
}

pub fn convert_bytes(jp2_bytes: &[u8]) -> Result<Vec<u8>> {
    convert_bytes_with_speed(jp2_bytes, PngSpeed::default())
}

/// Convert JP2 bytes → PNG bytes with explicit speed control.
///
/// ```rust,no_run
/// let png = jp2_convert::convert_bytes_with_speed(&bytes, PngSpeed::Fast)?;
/// ```
pub fn convert_bytes_with_speed(jp2_bytes: &[u8], speed: PngSpeed) -> Result<Vec<u8>> {
    let image = decode_jp2(jp2_bytes)?;
    encode_png_fast(image, speed)
}

pub fn decode_to_image(jp2_bytes: &[u8]) -> Result<DynamicImage> {
    decode_jp2(jp2_bytes)
}

fn decode_jp2(bytes: &[u8]) -> Result<DynamicImage> {
    let decoder = Image::new(bytes, &DecodeSettings::default())
        .map_err(|e: hayro_jpeg2000::DecodeError| Error::Decode(e.to_string()))?;

    let color_type = decoder.color_type();
    let (width, height) = decoder.dimensions();
    let mut buf = vec![0u8; (width * height * color_type.channel_count() as u32) as usize];
    decoder
        .read_image(&mut buf)
        .map_err(|e: image::ImageError| Error::Decode(e.to_string()))?;

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
            return Err(Error::Decode(format!(
                "unexpected color type from hayro: {:?}",
                other
            )));
        }
    };
    Ok(img)
}

/// PNG encode using the `png` crate directly so we can control compression level.
///
/// The `image` crate's `write_to(..., ImageFormat::Png)` hardcodes a high
/// compression level with no way to override it.  Bypassing it and writing
/// directly via the `png` crate cuts encode time dramatically for large
/// Sentinel-2 TCI tiles (10980×10980 px).
fn encode_png_fast(image: DynamicImage, speed: PngSpeed) -> Result<Vec<u8>> {
    let compression = match speed {
        PngSpeed::Small => Compression::Best,
        PngSpeed::Balanced => Compression::Fast,  // zlib level 1
        PngSpeed::Fast => Compression::Rle,        // no LZ77, just RLE filter
    };

    let (width, height) = (image.width(), image.height());

    // Pre-allocate: uncompressed size is a reasonable upper bound for Fast,
    // and a useful starting point for the others.
    let channels = image.color().channel_count() as usize;
    let capacity = width as usize * height as usize * channels + height as usize; // +filter bytes
    let mut buf = Vec::with_capacity(capacity);

    let (color_type, bit_depth, raw_bytes) = match &image {
        DynamicImage::ImageLuma8(img) => (ColorType::Grayscale, BitDepth::Eight, img.as_raw().as_slice()),
        DynamicImage::ImageLumaA8(img) => (ColorType::GrayscaleAlpha, BitDepth::Eight, img.as_raw().as_slice()),
        DynamicImage::ImageRgb8(img) => (ColorType::Rgb, BitDepth::Eight, img.as_raw().as_slice()),
        DynamicImage::ImageRgba8(img) => (ColorType::Rgba, BitDepth::Eight, img.as_raw().as_slice()),
        // hayro only outputs 8-bit, but fall back gracefully for anything else
        _ => {
            // Re-encode via image crate as a safe fallback
            let mut fallback = Vec::new();
            image.write_to(&mut Cursor::new(&mut fallback), ImageFormat::Png)
                .map_err(Error::Encode)?;
            return Ok(fallback);
        }
    };

    let mut encoder = Encoder::new(&mut buf, width, height);
    encoder.set_color(color_type);
    encoder.set_depth(bit_depth);
    encoder.set_compression(compression);

    let mut writer = encoder
        .write_header()
        .map_err(|e| Error::PngEncode(e.to_string()))?;

    writer
        .write_image_data(raw_bytes)
        .map_err(|e| Error::PngEncode(e.to_string()))?;

    drop(writer); // flushes PNG IEND chunk
    Ok(buf)
}

// ── Parallel batch conversion ─────────────────────────────────────────────────

#[cfg(feature = "parallel")]
pub mod batch {
    use super::*;
    use rayon::prelude::*;

    /// Convert many JP2 files to PNG in parallel using all available CPU cores.
    ///
    /// Returns a `Vec` of `(input_path, Result)` so failures don't abort the batch.
    ///
    /// ```toml
    /// # Cargo.toml
    /// [features]
    /// parallel = ["rayon"]
    ///
    /// [dependencies]
    /// rayon = { version = "1", optional = true }
    /// ```
    pub fn convert_files_parallel<P: AsRef<Path> + Sync>(
        pairs: &[(P, P)],
        speed: PngSpeed,
    ) -> Vec<(&P, Result<()>)> {
        pairs
            .par_iter()
            .map(|(inp, out)| (inp, convert_file_with_speed(inp, out, speed)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_on_invalid_bytes() {
        assert!(convert_bytes(b"not a jp2 file").is_err());
    }

    #[test]
    fn error_message_is_readable() {
        let err = convert_bytes(b"garbage").unwrap_err();
        assert!(err.to_string().contains("JP2 decode error"));
    }

    #[test]
    fn png_speed_variants_compile() {
        // Just ensures all variants are reachable
        let _ = PngSpeed::Small;
        let _ = PngSpeed::Balanced;
        let _ = PngSpeed::Fast;
    }
}