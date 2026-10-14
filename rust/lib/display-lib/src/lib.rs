//! The display library, built as a C-ABI DLL (`display_lib.dll`).
//!
//! Implements the ABI in `include/display_lib.h`. A display application loads
//! the DLL and calls these functions; it supplies the GL proc-address loader,
//! so the library is independent of the windowing stack and no Rust types cross
//! the boundary.
//!
//! Every function that calls OpenGL assumes a glow context is current on the
//! calling thread (see `architecture.md`, "OpenGL context and safety").

mod default_format;
mod gl;

use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CString};

use glow::HasContext as _;

/// ABI version implemented by this library. Mirrors `DISPLAY_LIB_ABI_VERSION`
/// in `include/display_lib.h`.
const ABI_VERSION: u32 = 1;

/// A GL proc-address loader supplied by the display application.
type GlLoader = extern "C" fn(*const c_char, *mut c_void) -> *const c_void;

#[derive(Debug)]
enum DisplayError {
    /// A shader failed to compile or link; the string is the GL info log.
    Shader(String),
}

impl std::fmt::Display for DisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayError::Shader(msg) => write!(f, "shader error: {msg}"),
        }
    }
}

impl std::error::Error for DisplayError {}

/// The state behind the opaque `display_lib*` handle.
struct Display {
    gl: glow::Context,
    default: gl::ColoredMesh,
}

impl Display {
    fn new(loader: impl FnMut(&str) -> *const c_void) -> Result<Self, DisplayError> {
        let gl = unsafe { glow::Context::from_loader_function(loader) };
        let vertices = default_format::vertices();
        let default = gl::ColoredMesh::new(&gl, &vertices)?;
        Ok(Self { gl, default })
    }

    /// Render the default format into the region (physical pixels, top-left
    /// origin). `framebuffer_height` is the whole framebuffer height.
    fn render(&self, framebuffer_height: i32, x: i32, y: i32, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        let gl = &self.gl;
        let y_gl = framebuffer_height - y - height;
        let bg = default_format::BACKGROUND;
        unsafe {
            // Background: a scissor clear of the region.
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(x, y_gl, width, height);
            gl.clear_color(bg[0], bg[1], bg[2], bg[3]);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.disable(glow::SCISSOR_TEST);

            // The x, in the region's NDC.
            gl.viewport(x, y_gl, width, height);
            gl.disable(glow::DEPTH_TEST);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            self.default.draw(gl);
            gl.disable(glow::BLEND);
        }
    }
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
    let display = unsafe { &*(display as *const Display) };
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
