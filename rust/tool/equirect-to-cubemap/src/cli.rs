use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// Convert an equirectangular image into six cube-map faces.
#[derive(Parser, Debug)]
#[command(
    name = "equirect-to-cubemap",
    about = "Convert an equirectangular image to six cube faces"
)]
pub struct Args {
    /// Input equirectangular image.
    pub input: PathBuf,

    /// Output directory (created if missing).
    pub output_dir: PathBuf,

    /// Face size in pixels.
    #[arg(long, default_value_t = 1024)]
    pub size: u32,

    /// Output image format.
    #[arg(long, value_enum, default_value_t = Format::Jpg)]
    pub format: Format,

    /// JPEG quality (1-100).
    #[arg(long, default_value_t = 75)]
    pub jpeg_quality: u8,

    /// Optional per-face JPEG size target in bytes; picks the highest quality
    /// whose encoded face is at or below this size.
    #[arg(long)]
    pub target_bytes: Option<u64>,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpg,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpg => "jpg",
        }
    }
}
