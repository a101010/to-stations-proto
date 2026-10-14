//! Command-line arguments for `lil-d`.

use std::path::PathBuf;

use clap::Parser;

/// The to-stations small display.
#[derive(Parser, Debug)]
#[command(name = "lil-d", about = "to-stations small display")]
pub struct Args {
    /// Station number.
    #[arg(long, default_value_t = 0)]
    pub station: u32,

    /// Path to the display library DLL (default: display_lib.dll next to the exe).
    #[arg(long, default_value = "display_lib.dll")]
    pub display_lib: PathBuf,
}
