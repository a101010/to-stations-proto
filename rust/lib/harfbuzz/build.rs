//! Links the installed HarfBuzz (`harfbuzz` and `harfbuzz-gpu`) from the
//! external prefix.
//!
//! The prefix is `EXT_PATH_PREFIX` (default `C:\Libraries`); the library is
//! installed at `<prefix>\harfbuzz`. `scripts/build.ps1` sets the variable so a
//! custom prefix is honored.

use std::env;
use std::path::PathBuf;

fn main() {
    let prefix = env::var("EXT_PATH_PREFIX").unwrap_or_else(|_| r"C:\Libraries".to_string());
    let lib = PathBuf::from(prefix).join("harfbuzz").join("lib");

    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=harfbuzz-gpu");
    println!("cargo:rustc-link-lib=harfbuzz");
    println!("cargo:rerun-if-env-changed=EXT_PATH_PREFIX");
}
