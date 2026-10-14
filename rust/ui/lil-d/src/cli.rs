//! Command-line arguments for `lil-d`.

use clap::Parser;

/// The to-stations small display.
#[derive(Parser, Debug)]
#[command(name = "lil-d", about = "to-stations small display")]
pub struct Args {
    /// Station number.
    #[arg(long, default_value_t = 0)]
    pub station: u32,
}
