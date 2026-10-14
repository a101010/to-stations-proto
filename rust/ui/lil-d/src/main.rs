//! The `lil-d` small display application.
//!
//! Opens a window with an egui console and a square region delegated to the
//! display library, painted a placeholder color until the library exists.

// Release builds are a GUI application (no console window); debug builds keep
// the console for logs and panics.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli;
mod console;
mod gl_window;
mod layout;

use clap::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::Args::parse();
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = app::App::new(args);
    event_loop.run_app(&mut app)?;
    Ok(())
}
