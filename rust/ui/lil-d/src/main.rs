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
mod display;
mod gl_window;
mod layout;
mod position;

use clap::Parser;
use single_instance::Acquire;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::Args::parse();

    let _guard = match single_instance::acquire(&format!("lil-d-display-{}", args.display_id)) {
        Acquire::Primary(guard) => guard,
        Acquire::Duplicate => {
            let message = format!("display id {} is already running", args.display_id);
            eprintln!("lil-d: {message}");
            show_error("lil-d", &message);
            std::process::exit(1);
        }
    };

    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = app::App::new(args);
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(windows)]
fn show_error(title: &str, message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let wide = |text: &str| -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    };
    let title = wide(title);
    let message = wide(message);
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
fn show_error(title: &str, message: &str) {
    eprintln!("{title}: {message}");
}
