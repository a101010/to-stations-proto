//! The display library, built as a C-ABI DLL (`display_lib.dll`).
//!
//! Implements the ABI in `include/display_lib.h`. A display application loads
//! the DLL and calls these functions; it supplies the GL proc-address loader,
//! so the library is independent of the windowing stack and no Rust types cross
//! the boundary.
//!
//! Every function that calls OpenGL assumes a glow context is current on the
//! calling thread (see `architecture.md`, "OpenGL context and safety").

mod formats;
mod text;

use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CString};

/// ABI version implemented by this library. Mirrors `DISPLAY_LIB_ABI_VERSION`
/// in `include/display_lib.h`.
const ABI_VERSION: u32 = 1;

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
}

impl std::fmt::Display for DisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayError::Shader(msg) => write!(f, "shader error: {msg}"),
            DisplayError::Io(msg) => write!(f, "io error: {msg}"),
            DisplayError::Font(msg) => write!(f, "font error: {msg}"),
        }
    }
}

impl std::error::Error for DisplayError {}

/// The state behind the opaque `display_lib*` handle.
struct Display {
    gl: glow::Context,
    hello_world: formats::hello_world::HelloWorld,
}

impl Display {
    fn new(loader: impl FnMut(&str) -> *const c_void) -> Result<Self, DisplayError> {
        let gl = unsafe { glow::Context::from_loader_function(loader) };
        let hello_world = formats::hello_world::HelloWorld::new(&gl, &assets_root())?;
        Ok(Self { gl, hello_world })
    }

    /// Render the active format into the region (physical pixels, top-left
    /// origin). `framebuffer_height` is the whole framebuffer height.
    fn render(&mut self, framebuffer_height: i32, x: i32, y: i32, width: i32, height: i32) {
        self.hello_world
            .render(&self.gl, framebuffer_height, x, y, width, height);
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
