# Current story: display-lib

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the display library (`display-lib`) and have `lil-d` delegate the format-window region to it. It renders the default format, hardcoded: a solid color (different from the original `lil-d` color) with an x in a different color, owning the entire region delegated to the library.

The display library is a C-ABI DLL (`display_lib.dll`) built from a shared header (`include/display_lib.h`); `lil-d` loads it at runtime. This is the hybrid interop seam: a display app in either language can use a display library in either language. This story implements the Rust side; the C++ implementation is a later increment.

- **Depends on:** lil-d.
- **Minimal test:** the delegated region shows the default format when `lil-d` loads `display_lib.dll` via the C ABI.

## Context

`lil-d` currently owns the window, GL context, layout, console, and (from the previous story) a `display-lib` Rust library that it links statically and calls through a Rust API (`Display::new` / `render`).

The hybrid deployment requires a display app and its display library to be separate components joined by a C ABI, so a Rust app can load the C++ display library and a C++ app can load the Rust one. That means the boundary must carry only C types and must not depend on the app's windowing stack; the app supplies the OpenGL proc-address loader.

`glow` exposes every GL call as `unsafe`; the documented GL-context-current assumption from the previous story still applies inside the library (see `architecture.md`, "OpenGL context and safety").

## Decisions

* **Shared ABI header.** `include/display_lib.h` is the hand-written source of truth, shared by both languages. C types only.
* **Rust implementation.** `display-lib` is a `cdylib` exporting the ABI with `extern "C"` + `#[no_mangle]`; it keeps its `glow`-only dependency and builds its own `glow::Context` from the app-provided loader.
* **Rust consumer.** `lil-d` does not link the library; it loads the DLL with `libloading` and calls `extern "C"` fn-pointer types. `--display-lib <path>` selects the DLL (default `display_lib.dll`, resolved against the exe's directory when relative), for the future hybrid choice.
* **ABI version.** `display_lib_abi_version()` returns `DISPLAY_LIB_ABI_VERSION` (1); the app checks it before use.
* **No Rust types cross.** The app passes a loader fn pointer + userdata; the library builds its own GL context, so no `glow::Context` crosses the boundary.
* **Rendering / default format.** Unchanged from the previous story: the amber background with the green corner-to-corner x, drawn with a `#version 330 core` program in the region's NDC.
* **Deploy.** `scripts/build.ps1` copies `display_lib.dll` next to `lil-d.exe` in `build/deploy/bin`.

## ABI: `include/display_lib.h`

```c
#ifndef DISPLAY_LIB_H
#define DISPLAY_LIB_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define DISPLAY_LIB_ABI_VERSION 1u

/* Opaque display library handle. */
typedef struct display_lib display_lib;

/* Returns the address of a GL function by name, for the current context. */
typedef void* (*display_lib_gl_loader)(const char* name, void* userdata);

uint32_t     display_lib_abi_version(void);

/* Returns a handle, or NULL on failure (see display_lib_last_error). */
display_lib* display_lib_create(display_lib_gl_loader loader, void* userdata);

/* framebuffer_height flips the top-left region into GL's bottom-left origin. */
void         display_lib_render(display_lib* d, int framebuffer_height,
                                int x, int y, int width, int height);

void         display_lib_destroy(display_lib* d);

/* Message from the last failed display_lib_create; valid until the next call. */
const char*  display_lib_last_error(void);

#ifdef __cplusplus
}
#endif

#endif /* DISPLAY_LIB_H */
```

The Rust implementation (`#[no_mangle] pub extern "C"`) and the Rust consumer's `extern "C"` fn-pointer types mirror this header by hand.

## Rust `display-lib` (`rust/lib/display-lib`)

`Cargo.toml`: `[lib] crate-type = ["cdylib"]` (keep `name = "display_lib"`); deps `glow` only.

`lib.rs`:

```rust
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CStr, CString};

const ABI_VERSION: u32 = 1;

/// A GL proc-address loader supplied by the display application.
type GlLoader = extern "C" fn(*const c_char, *mut c_void) -> *const c_void;

struct Display {
    gl: glow::Context,          // owned; built from the loader
    default: gl::ColoredMesh,
}

impl Display {
    fn new(loader: impl FnMut(&str) -> *const c_void) -> Result<Self, DisplayError>;
    fn render(&self, framebuffer_height: i32, x: i32, y: i32, width: i32, height: i32);
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}
```

Exports (pseudocode):

```
#[no_mangle] pub extern "C" fn display_lib_abi_version() -> u32:
    ABI_VERSION

#[no_mangle] pub extern "C" fn display_lib_create(loader: GlLoader, userdata: *mut c_void) -> *mut Display:
    let result = Display::new(|name| {
        let cname = CString::new(name).expect("gl proc name")
        loader(cname.as_ptr(), userdata)
    })
    match result:
        Ok(display) => Box::into_raw(Box::new(display))
        Err(err)    => { set_last_error(err.to_string()); null_mut() }

#[no_mangle] pub extern "C" fn display_lib_render(d: *mut Display, framebuffer_height: c_int, x: c_int, y: c_int, width: c_int, height: c_int):
    if d.is_null(): return
    unsafe { &*d }.render(framebuffer_height, x, y, width, height)

#[no_mangle] pub extern "C" fn display_lib_destroy(d: *mut Display):
    if d.is_null(): return
    unsafe { drop(Box::from_raw(d)) }

#[no_mangle] pub extern "C" fn display_lib_last_error() -> *const c_char:
    LAST_ERROR.with(|e| e.borrow().as_ref().map_or(null(), |s| s.as_ptr()))
```

`Display::new` builds the context with `glow::Context::from_loader_function(|name| loader(name))`, then `gl::ColoredMesh::new(&gl, &default_format::vertices())`. `render` is the previous story's region render (scissor clear + viewport + `ColoredMesh::draw`), unchanged. `gl.rs` and `default_format.rs` are unchanged.

`display_lib_destroy` calls no GL (the `ColoredMesh` has no `Drop`), so it is safe to call off the event-loop thread; keep that property when a `destroy` that deletes GL objects is added.

## `lil-d` (`rust/ui/lil-d`)

`Cargo.toml`: drop `display-lib`; add `libloading`.

`src/cli.rs`: add

```rust
/// Path to the display library DLL (default: display_lib.dll next to the exe).
#[arg(long, default_value = "display_lib.dll")]
pub display_lib: std::path::PathBuf,
```

`src/display.rs` (new):

```rust
pub type GlLoader = extern "C" fn(*const c_char, *mut c_void) -> *const c_void;

type CreateFn      = unsafe extern "C" fn(GlLoader, *mut c_void) -> *mut c_void;
type RenderFn      = unsafe extern "C" fn(*mut c_void, c_int, c_int, c_int, c_int, c_int);
type DestroyFn     = unsafe extern "C" fn(*mut c_void);
type AbiVersionFn  = unsafe extern "C" fn() -> u32;
type LastErrorFn   = unsafe extern "C" fn() -> *const c_char;

const EXPECTED_ABI_VERSION: u32 = 1;

pub struct Display {
    _lib: libloading::Library, // keeps the DLL loaded
    handle: *mut c_void,
    render: RenderFn,
    destroy: DestroyFn,
}

impl Display {
    pub fn load(path: &Path, loader: GlLoader, userdata: *mut c_void) -> Result<Self, String>;
    pub fn render(&mut self, framebuffer_height: i32, rect: crate::layout::Rect);
}

impl Drop for Display { fn drop(&mut self) { unsafe { (self.destroy)(self.handle) } } }
```

Pseudocode for `load`:

```
let lib = unsafe { Library::new(path) }.map_err(|e| format!("failed to load {}: {e}", path.display()))?
unsafe {
    let abi_version = *lib.get(b"display_lib_abi_version\0")?
    if abi_version() != EXPECTED_ABI_VERSION:
        return Err(format!("display-lib ABI {} != expected {}", abi_version(), EXPECTED_ABI_VERSION))
    let create      = *lib.get(b"display_lib_create\0")?
    let render      = *lib.get(b"display_lib_render\0")?
    let destroy     = *lib.get(b"display_lib_destroy\0")?
    let last_error  = *lib.get(b"display_lib_last_error\0")?
    let handle = create(loader, userdata)
    if handle.is_null():
        let msg = last_error()
        return Err(if msg.is_null() { "display_lib_create failed".into() } else { CStr::from_ptr(msg).to_string_lossy().into_owned() })
    Ok(Display { _lib: lib, handle, render, destroy })
}
```

The path is resolved in `main`/`setup`: if `args.display_lib` is relative, join it with `current_exe().parent()`.

`src/gl_window.rs`: add

```rust
pub type GlLoaderFn = extern "C" fn(*const c_char, *mut c_void) -> *const c_void;

impl GlWindow {
    /// The GL proc-address loader and its userdata (the glutin display), for
    /// the display library.
    pub fn gl_loader(&self) -> (GlLoaderFn, *mut c_void) {
        extern "C" fn load(name: *const c_char, userdata: *mut c_void) -> *const c_void {
            use glutin::display::GlDisplay as _;
            let display = unsafe { &*(userdata as *const glutin::display::Display) };
            display.get_proc_address(unsafe { CStr::from_ptr(name) })
        }
        (load, &self.gl_display as *const _ as *mut c_void)
    }
}
```

`src/app.rs`: `App.display: Option<display::Display>`; in `setup`, after the GL context, `let (loader, userdata) = gl_window.gl_loader(); let display = display::Display::load(&path, loader, userdata).expect("display-lib load");`; in `render`, when the format is on, `display.render(size.height as i32, layout.format)`. Remove the old `display_lib::{Display, Region}` import and the `Display::new(Arc::clone(&gl))` call.

## Workspace / build

* `rust/Cargo.toml`: remove the `display-lib = { path = "lib/display-lib" }` workspace-dependency entry (no member depends on it now); keep `lib/display-lib` as a member; add `libloading = "0.8"` to `[workspace.dependencies]`.
* `scripts/build.ps1`: in the deploy step, also copy `build/rust/release/display_lib.dll` into `build/deploy/bin`.
* `lil-d` no longer links the library, so `cargo run -p lil-d` alone will not build the DLL; the workspace build (`cargo build`, `scripts/build.ps1`) does. Record this in `README.md`.

## Tests

* `default_format.rs` unit tests (unchanged) still build from source under `cdylib`.
* No unit test for the ABI itself (it needs a GL context); the loader path is verified by the minimal test and the negative checks.

## Minimal test

```
.\scripts\build.ps1
.\scripts\launch-rust.ps1
```

The delegated region shows the default format (amber background, green corner-to-corner x) now rendered by `display_lib.dll` through the C ABI; the power button still toggles the region; `.\scripts\cleanup.ps1` stops it.

Negative checks:
* `.\build\deploy\bin\lil-d.exe --display-lib missing.dll` fails cleanly with a loader error.
* A DLL with a different ABI version fails with the version-mismatch error.

## Risks and follow-ups

* **ABI drift.** The header and the Rust `extern "C"` declarations are hand-written and can diverge; keep them in step. 
* **FFI discipline.** Only C types cross; `display_lib_create` catches `DisplayError`; `render` is infallible; `display_lib_destroy` must not call GL.
* **Loader lifetime.** The loader + userdata are only used during `display_lib_create` (glow resolves all pointers there), so the glutin display pointer only needs to be valid at setup.
* **`--display-lib` resolution.** Relative paths resolve against the exe directory; a stale DLL next to a rebuilt exe can shadow a fresh build.
* **`cargo run`.** The DLL is not built by `cargo run -p lil-d` alone.
* **C++ side.** The C++ display library and C++ display app that implement/consume this ABI are later increments.

## Files

* Authored: `include/display_lib.h`, `rust/ui/lil-d/src/display.rs`.
* Changed: `rust/lib/display-lib/Cargo.toml`, `rust/lib/display-lib/src/lib.rs`, `rust/ui/lil-d/Cargo.toml`, `rust/ui/lil-d/src/{cli,app,gl_window}.rs`, `rust/Cargo.toml`, `scripts/build.ps1`, `README.md`.
* Planning: `planning/architecture.md`, `planning/backlog.md`, `planning/current_story.md`.
