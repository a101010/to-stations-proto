use clap::Parser;

use equirect_to_cubemap::cli::Args;
use equirect_to_cubemap::run;

fn main() {
    let args = Args::parse();
    if let Err(e) = run(&args) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
