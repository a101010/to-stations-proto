use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, ImageError, RgbImage};

use crate::cli::Format;

/// Load an image and convert it to RGB.
pub fn load_source(path: &Path) -> Result<RgbImage, ImageError> {
    Ok(image::open(path)?.to_rgb8())
}

/// Encode one face to bytes in the requested format.
pub fn encode_face(image: &RgbImage, format: Format, quality: u8) -> Result<Vec<u8>, ImageError> {
    let mut bytes = Vec::new();
    match format {
        Format::Png => {
            PngEncoder::new(&mut bytes).write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgb8,
            )?;
        }
        Format::Jpg => {
            JpegEncoder::new_with_quality(&mut bytes, quality).encode_image(image)?;
        }
    }
    Ok(bytes)
}

/// Encode and write one face.
pub fn save_face(
    image: &RgbImage,
    path: &Path,
    format: Format,
    quality: u8,
) -> Result<(), ImageError> {
    let bytes = encode_face(image, format, quality)?;
    std::fs::write(path, bytes)?;
    Ok(())
}
