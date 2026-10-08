pub mod cli;
pub mod cubemap;
pub mod image_io;
pub mod sampling;

use std::error::Error as StdError;
use std::fmt;

use image::{ImageError, RgbImage};

use cli::{Args, Format};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Image(ImageError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Image(e) => write!(f, "{e}"),
        }
    }
}

impl StdError for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<ImageError> for Error {
    fn from(e: ImageError) -> Self {
        Error::Image(e)
    }
}

/// Highest JPEG quality (1..=100) whose encoded face is at or below `target` bytes.
fn best_quality(image: &RgbImage, format: Format, target: u64) -> u8 {
    if format != Format::Jpg {
        return 0;
    }
    let (mut lo, mut hi) = (1u8, 100u8);
    let mut best = 1u8;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let size = image_io::encode_face(image, format, mid)
            .map(|b| b.len() as u64)
            .unwrap_or(u64::MAX);
        if size <= target {
            best = mid;
            if mid == 100 {
                break;
            }
            lo = mid + 1;
        } else {
            if mid == 1 {
                break;
            }
            hi = mid - 1;
        }
    }
    best
}

pub fn run(args: &Args) -> Result<(), Error> {
    let source = image_io::load_source(&args.input)?;
    if source.width() != 2 * source.height() {
        eprintln!(
            "warning: source is {}x{} (expected 2:1 equirectangular)",
            source.width(),
            source.height()
        );
    }

    std::fs::create_dir_all(&args.output_dir)?;

    for (face, image) in cubemap::convert(&source, args.size, &mut |f| {
        println!("rendering {}", f.stem());
    }) {
        let quality = match args.target_bytes {
            Some(target) => best_quality(&image, args.format, target),
            None => args.jpeg_quality,
        };
        let bytes = image_io::encode_face(&image, args.format, quality)?;
        let path = args
            .output_dir
            .join(format!("{}.{}", face.stem(), args.format.extension()));
        std::fs::write(&path, &bytes)?;
        println!(
            "wrote {} ({} bytes, quality {})",
            path.display(),
            bytes.len(),
            quality
        );
    }

    println!("done: 6 faces at {}x{}", args.size, args.size);
    Ok(())
}
