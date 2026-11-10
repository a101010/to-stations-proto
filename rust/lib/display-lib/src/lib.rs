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
mod format_source;
mod formats;
mod shader;
mod text;
mod view;

use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use format_list::FormatList;
use stations_dds::contract::TopicRegistry;
use stations_dds::dds::Node;

/// ABI version implemented by this library. Mirrors `DISPLAY_LIB_ABI_VERSION`
/// in `include/display_lib.h`.
const ABI_VERSION: u32 = 3;

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
    /// The formats contract could not be loaded or names a format we cannot build.
    Formats(String),
}

impl std::fmt::Display for DisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DisplayError::Shader(msg) => write!(f, "shader error: {msg}"),
            DisplayError::Io(msg) => write!(f, "io error: {msg}"),
            DisplayError::Font(msg) => write!(f, "font error: {msg}"),
            DisplayError::Image(msg) => write!(f, "image error: {msg}"),
            DisplayError::Formats(msg) => write!(f, "formats error: {msg}"),
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
    /// Format-manager assignments and logical input events.
    format_source: Option<format_source::FormatSource>,
    /// This display's unique id, used to stamp and filter format-manager
    /// samples. Empty until the display application sets it.
    display_id: String,
    /// The latest received view sample, shared with the formats.
    state: view::ViewState,
}

impl Display {
    fn new(loader: impl FnMut(&str) -> *const c_void) -> Result<Self, DisplayError> {
        let gl = unsafe { glow::Context::from_loader_function(loader) };
        let state = view::new_state();
        let config = config_root();
        let formats = FormatList::load(&config.join("formats.toml"))
            .map_err(|e| DisplayError::Formats(e.to_string()))?;
        let format_manager =
            format_manager::FormatManager::new(&gl, &assets_root(), state.clone(), &formats)?;

        // Best-effort: without DDS/config the format renders black and local
        // clicks do not reach a controller.
        let node = TopicRegistry::load(&config.join("topics.toml"))
            .ok()
            .and_then(|registry| Node::new(0, registry).ok());
        let view = node.as_ref().and_then(|node| view::ViewSource::new(node).ok());
        let format_source = node
            .as_ref()
            .and_then(|node| format_source::FormatSource::new(node).ok());

        Ok(Self {
            gl,
            format_manager,
            view,
            format_source,
            display_id: String::new(),
            state,
        })
    }

    /// Render the active format into the region (physical pixels, top-left
    /// origin). `framebuffer_height` is the whole framebuffer height.
    fn render(&mut self, framebuffer_height: i32, x: i32, y: i32, width: i32, height: i32) {
        if let Some(view) = &self.view {
            view.poll(&self.state);
        }
        if let Some(format_source) = &self.format_source {
            format_source.poll(&mut self.format_manager, &self.display_id);
        }
        self.format_manager
            .render(&self.gl, framebuffer_height, x, y, width, height);
    }

    /// Forward a mouse button event (physical pixels, top-left origin). A left
    /// press inside a format window publishes a logical `Next` event.
    fn mouse_button(&mut self, x: i32, y: i32, button: i32, pressed: bool) {
        if button != 0 || !pressed {
            return;
        }
        if let Some(window) = self.format_manager.hit_window(x, y) {
            if let Some(format_source) = &self.format_source {
                format_source.send_next(window, &self.display_id);
            }
        }
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

/// The contracts directory: the deploy root's `config/`, else the current
/// working directory's `contracts/` for a development build.
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
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("contracts")
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

/// Sets this display's unique identity. Must be called after
/// [`display_lib_create`] and before the first render.
#[no_mangle]
pub extern "C" fn display_lib_set_identity(display: *mut c_void, display_id: *const c_char) {
    if display.is_null() || display_id.is_null() {
        return;
    }
    let display = unsafe { &mut *(display as *mut Display) };
    let display_id = unsafe { CStr::from_ptr(display_id) }
        .to_string_lossy()
        .into_owned();
    display.display_id = display_id;
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
