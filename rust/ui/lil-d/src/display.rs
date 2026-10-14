//! Loads the display library DLL and wraps its C ABI (`include/display_lib.h`).
//!
//! The display library is not linked in; it is loaded at runtime so the app can
//! use either the Rust or the C++ implementation (`--display-lib`).

use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::{Path, PathBuf};

use libloading::Library;

/// ABI version this app expects. Mirrors `DISPLAY_LIB_ABI_VERSION` in
/// `include/display_lib.h`.
const EXPECTED_ABI_VERSION: u32 = 1;

/// A GL proc-address loader, as passed to the display library.
pub type GlLoader = extern "C" fn(*const c_char, *mut c_void) -> *const c_void;

type CreateFn = unsafe extern "C" fn(GlLoader, *mut c_void) -> *mut c_void;
type RenderFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, c_int, c_int, c_int);
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type AbiVersionFn = unsafe extern "C" fn() -> u32;
type LastErrorFn = unsafe extern "C" fn() -> *const c_char;

/// A loaded display library.
pub struct Display {
    _lib: Library, // keeps the DLL loaded for the lifetime of the handle
    handle: *mut c_void,
    render: RenderFn,
    destroy: DestroyFn,
}

impl Display {
    /// Load the display library at `path` and create a handle.
    ///
    /// `loader` and `userdata` are used only during creation (the library
    /// resolves its GL function pointers then).
    pub fn load(path: &Path, loader: GlLoader, userdata: *mut c_void) -> Result<Self, String> {
        let lib = unsafe { Library::new(path) }
            .map_err(|e| format!("failed to load display library {}: {e}", path.display()))?;

        unsafe {
            let abi_version: AbiVersionFn = *lib
                .get(b"display_lib_abi_version\0")
                .map_err(|e| format!("missing display_lib_abi_version: {e}"))?;
            let found = abi_version();
            if found != EXPECTED_ABI_VERSION {
                return Err(format!(
                    "display library ABI version {found} != expected {EXPECTED_ABI_VERSION}"
                ));
            }

            let create: CreateFn = *lib
                .get(b"display_lib_create\0")
                .map_err(|e| format!("missing display_lib_create: {e}"))?;
            let render: RenderFn = *lib
                .get(b"display_lib_render\0")
                .map_err(|e| format!("missing display_lib_render: {e}"))?;
            let destroy: DestroyFn = *lib
                .get(b"display_lib_destroy\0")
                .map_err(|e| format!("missing display_lib_destroy: {e}"))?;
            let last_error: LastErrorFn = *lib
                .get(b"display_lib_last_error\0")
                .map_err(|e| format!("missing display_lib_last_error: {e}"))?;

            let handle = create(loader, userdata);
            if handle.is_null() {
                let message = last_error();
                return Err(if message.is_null() {
                    "display_lib_create failed".to_string()
                } else {
                    CStr::from_ptr(message).to_string_lossy().into_owned()
                });
            }

            Ok(Self {
                _lib: lib,
                handle,
                render,
                destroy,
            })
        }
    }

    /// Render into `rect` (physical pixels, top-left origin).
    pub fn render(&mut self, framebuffer_height: i32, rect: crate::layout::Rect) {
        unsafe {
            (self.render)(
                self.handle,
                framebuffer_height,
                rect.x,
                rect.y,
                rect.w,
                rect.h,
            );
        }
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe { (self.destroy)(self.handle) }
    }
}

/// Resolve the display library path: a relative path is taken from the
/// directory of the running executable.
pub fn resolve_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    match std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        Some(dir) => dir.join(path),
        None => path.to_path_buf(),
    }
}
