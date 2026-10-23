//! The display library, built as a C-ABI DLL (`display_lib.dll`).
//!
//! Implements the ABI in `include/display_lib.h`. A display application loads
//! the DLL and calls these functions; it supplies the GL proc-address loader,
//! so the library is independent of the windowing stack and no Rust types cross
//! the boundary.
//!
//! Every function that calls OpenGL assumes a glow context is current on the
//! calling thread (see `architecture.md`, "OpenGL context and safety").

mod earth;
mod format_manager;
mod formats;
mod shader;
mod text;
mod view;

use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CString};

/// ABI version implemented by this library. Mirrors `DISPLAY_LIB_ABI_VERSION`
/// in `include/display_lib.h`.
const ABI_VERSION: u32 = 2;

/// A GL proc-address loader supplied by the display application.
type GlLoader = extern "C" fn(*const c_char, *mut c_void) -> *const c_void;

#[derive(Debug)]
enum DisplayError {
    /// A GL shader or object failed; the string is the GL info log or message.
    Shader(String),
    /// A file (the font) could not be read.
    Io(String),
    /// The font or its GPU encoder could not be created.
    Font(String),
    /// An image asset (an earth cube-map face) could not be loaded or decoded.
    Image(String),
}

impl std::fmt::Display for DisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayError::Shader(msg) => write!(f, "shader error: {msg}"),
            DisplayError::Io(msg) => write!(f, "io error: {msg}"),
            DisplayError::Font(msg) => write!(f, "font error: {msg}"),
            DisplayError::Image(msg) => write!(f, "image error: {msg}"),
        }
    }
}

impl std::error::Error for DisplayError {}

/// The state behind the opaque `display_lib*` handle.
struct Display {
    gl: glow::Context,
    format_manager: format_manager::FormatManager,
    /// The DDS subscription for the hello_world view; `None` when it could not
    /// be created (the format then renders black).
    view: Option<view::ViewSource>,
    /// The latest received view sample, shared with the formats.
    state: view::ViewState,
}

impl Display {
    fn new(loader: impl FnMut(&str) -> *const c_void) -> Result<Self, DisplayError> {
        let gl = unsafe { glow::Context::from_loader_function(loader) };
        let state = view::new_state();
        let format_manager =
            format_manager::FormatManager::new(&gl, &assets_root(), state.clone())?;
        // Best-effort: without DDS/config the format simply renders black.
        let view = view::ViewSource::new(0, &config_root()).ok();
        Ok(Self {
            gl,
            format_manager,
            view,
            state,
        })
    }

    /// Render the active format into the region (physical pixels, top-left
    /// origin). `framebuffer_height` is the whole framebuffer height.
    fn render(&mut self, framebuffer_height: i32, x: i32, y: i32, width: i32, height: i32) {
        if let Some(view) = &self.view {
            view.poll(&self.state);
        }
        self.format_manager
            .render(&self.gl, framebuffer_height, x, y, width, height);
    }

    /// Forward a mouse button event (physical pixels, top-left origin).
    fn mouse_button(&mut self, x: i32, y: i32, button: i32, pressed: bool) {
        self.format_manager.mouse_button(x, y, button, pressed);
    }
}

/// The deploy root's assets directory: `<exe_dir>/../assets` (the exe lives in
/// `build/deploy/bin`, the assets in `build/deploy/assets`).
fn assets_root() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        .and_then(|bin| bin.parent().map(|deploy| deploy.join("assets")))
        .unwrap_or_else(|| std::path::PathBuf::from("assets"))
}

/// The contracts directory: the deploy root's `config/`, else the repository's
/// `contracts/` (so a debug build run from the repository still finds it).
fn config_root() -> std::path::PathBuf {
    let deployed = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        .and_then(|bin| bin.parent().map(|deploy| deploy.join("config")));
    if let Some(dir) = &deployed {
        if dir.join("topics.toml").is_file() {
            return dir.clone();
        }
    }
    std::path::PathBuf::from(env!("REPO_ROOT")).join("contracts")
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(message: String) {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = Some(CString::new(message).unwrap_or_default());
    });
}

/// Returns `DISPLAY_LIB_ABI_VERSION`.
#[no_mangle]
pub extern "C" fn display_lib_abi_version() -> u32 {
    ABI_VERSION
}

/// Creates a display library. Returns NULL on failure; see
/// [`display_lib_last_error`].
#[no_mangle]
pub extern "C" fn display_lib_create(loader: GlLoader, userdata: *mut c_void) -> *mut c_void {
    let result = Display::new(|name| {
        let cname = CString::new(name).expect("gl proc name");
        loader(cname.as_ptr(), userdata)
    });
    match result {
        Ok(display) => Box::into_raw(Box::new(display)) as *mut c_void,
        Err(err) => {
            set_last_error(err.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Renders into `(x, y, width, height)` (physical pixels, top-left origin).
#[no_mangle]
pub extern "C" fn display_lib_render(
    display: *mut c_void,
    framebuffer_height: c_int,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
) {
    if display.is_null() {
        return;
    }
    let display = unsafe { &mut *(display as *mut Display) };
    display.render(framebuffer_height, x, y, width, height);
}

/// Forwards a mouse button event: `(x, y)` are physical pixels from the
/// window's top-left, `button` is 0=left / 1=right / 2=middle / 3=back /
/// 4=forward / 5=other, and `pressed` is nonzero on press.
#[no_mangle]
pub extern "C" fn display_lib_mouse_button(
    display: *mut c_void,
    x: c_int,
    y: c_int,
    button: c_int,
    pressed: c_int,
) {
    if display.is_null() {
        return;
    }
    let display = unsafe { &mut *(display as *mut Display) };
    display.mouse_button(x, y, button, pressed != 0);
}

/// Destroys a display library created by [`display_lib_create`].
#[no_mangle]
pub extern "C" fn display_lib_destroy(display: *mut c_void) {
    if display.is_null() {
        return;
    }
    unsafe { drop(Box::from_raw(display as *mut Display)) };
}

/// The message from the last failed [`display_lib_create`], or NULL. Valid
/// until the next call.
#[no_mangle]
pub extern "C" fn display_lib_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| {
        cell.borrow()
            .as_ref()
            .map_or(std::ptr::null(), |s| s.as_ptr())
    })
}
