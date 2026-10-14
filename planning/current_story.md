# Current story: display-lib

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the display library (`display-lib`) and have `lil-d` delegate the format-window region to it. It renders the default format, hardcoded: a solid color (different from the original `lil-d` color) with an x in a different color, owning the entire region delegated to the library.

- **Depends on:** lil-d.
- **Minimal test:** the delegated region shows the default format.

## Context

`lil-d` currently owns the whole window: `gl_window.rs` (winit + glutin/glow), `layout.rs` (format region + console), `console.rs` (egui), and `app.rs` (render loop). It paints the format region itself with a GL scissor clear of `FORMAT_GREEN` (`app.rs:25,182`).

The display library is a shared `lib/` crate (`architecture.md`), separate from the app. The app keeps the window, the GL context, the layout, and the console; the library renders the format into the region the app gives it. `display-lib` must not depend on winit/egui — only `glow`.

The default format is the first format; the format registry and switching arrive with `format-manager`. Input forwarding is not part of this story.

## Decisions

* **Crate.** `display-lib` lives at `rust/lib/display-lib` (package `display-lib`, lib `display_lib`); its only dependency is `glow`.
* **App/​library boundary.** The app owns the window, GL context, and layout; it passes the format region (physical pixels, top-left origin) and the framebuffer height to the library. The library owns everything inside that region.
* **Rendering.** The library uses a small GLSL program and a baked vertex buffer; the region background is a scissor clear, and the x is two triangles (two diagonal bars) drawn through the program. This establishes the shader-based renderer the earth rendering will extend.
* **GL context (option A).** `GlWindow::new` requests an OpenGL 3.3 core context (dropping the version-less + GLES fallback), and the library's shaders are `#version 330 core`. Requesting the version makes the shader version we depend on guaranteed rather than whatever the driver picks by default. Core profile drops deprecated fixed-function features; the shader is modern. Trade-off: the GLES fallback is gone, so a non-desktop-GL system fails context creation loudly instead of getting a context our shader cannot use.
* **GL context precondition (documented, not encoded).** `glow` marks every GL call `unsafe` because a context must be current on the calling thread, and it offers no token that proves currentness. Following `egui_glow`, the library's public API is safe and documents the assumption that a glow context is current on the calling thread; the display application upholds it by doing all GL work on its event-loop thread. This relaxation is limited to the GL-context-current precondition; other genuine safety concerns are handled normally.
* **No format abstraction yet.** `Display` renders the default format directly; the format registry/active-format state arrives with `format-manager`. `render` takes `&mut self` so that state can be added without an API break.
* **Colors and geometry.** Amber background `[0.90, 0.62, 0.10, 1.0]`, green x `[0.05, 0.45, 0.15, 1.0]`. The x runs corner to corner of the region (`MARGIN = 0.0`); the bar ends are clipped at the region edges.

## Crate layout

```
rust/lib/display-lib/
  Cargo.toml
  src/
    lib.rs              public API: Display, Region, DisplayError
    gl.rs               ColoredMesh: shader program + VAO/VBO
    default_format.rs   default-format colors and geometry
```

## Cargo.toml

```toml
[package]
name = "display-lib"
version = "0.0.0"
edition = "2021"
publish = false

[lib]
name = "display_lib"
path = "src/lib.rs"

[dependencies]
glow = { workspace = true }
```

`rust/Cargo.toml`: add `"lib/display-lib"` to `members` and `display-lib = { path = "lib/display-lib" }` to `[workspace.dependencies]`. `rust/ui/lil-d/Cargo.toml`: add `display-lib = { workspace = true }`.

## Public API (`lib.rs`)

```rust
use std::sync::Arc;

/// A rectangle in physical pixels, origin top-left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region { pub x: i32, pub y: i32, pub width: i32, pub height: i32 }

#[derive(Debug)]
pub enum DisplayError { Shader(String) }
impl std::fmt::Display for DisplayError { /* "shader error: {msg}" */ }
impl std::error::Error for DisplayError {}

/// The display library: renders the active format into a region.
pub struct Display {
    gl: Arc<glow::Context>,
    default: gl::ColoredMesh,
}

impl Display {
    /// Compile the renderer. `gl` is the app's context (shared with egui_glow).
    ///
    /// Assumes a glow context is current on the calling thread; the display
    /// application upholds this by calling on its event-loop thread.
    pub fn new(gl: Arc<glow::Context>) -> Result<Self, DisplayError>;

    /// Render the default format into `region`.
    ///
    /// `framebuffer_height` is the whole framebuffer height, used to flip
    /// `region`'s top-left origin into GL's bottom-left origin.
    ///
    /// Assumes a glow context is current on the calling thread.
    pub fn render(&mut self, framebuffer_height: i32, region: Region);
}
```

## Internal: `gl.rs`

```rust
pub(crate) struct ColoredMesh {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    vertex_count: i32,
}

impl ColoredMesh {
    /// Vertices are interleaved `[x, y, r, g, b, a, ...]` in NDC.
    ///
    /// Assumes a glow context is current on the calling thread.
    pub(crate) fn new(gl: &glow::Context, vertices: &[f32]) -> Result<Self, crate::DisplayError>;

    /// Assumes a glow context is current on the calling thread.
    pub(crate) fn draw(&self, gl: &glow::Context);
}
```

Pseudocode (every glow call is `unsafe`; each function wraps its glow calls in an `unsafe { }` block, and the documented assumption is that a context is current on the calling thread):

```
const VERTEX_SRC: &str = r#"#version 330 core
layout (location = 0) in vec2 a_pos;
layout (location = 1) in vec4 a_color;
out vec4 v_color;
void main() { v_color = a_color; gl_Position = vec4(a_pos, 0.0, 1.0); }"#;

const FRAGMENT_SRC: &str = r#"#version 330 core
in vec4 v_color;
out vec4 f_color;
void main() { f_color = v_color; }"#;

fn compile(gl, kind, src) -> Result<glow::Shader, DisplayError>:  // unsafe { glow calls }
    let shader = gl.create_shader(kind)?
    gl.shader_source(shader, src)
    gl.compile_shader(shader)
    if !gl.get_shader_compile_status(shader):
        return Err(DisplayError::Shader(gl.get_shader_info_log(shader)))
    Ok(shader)

fn link(gl, vs_src, fs_src) -> Result<glow::Program, DisplayError>:  // unsafe { glow calls }
    let vs = compile(gl, VERTEX_SHADER, vs_src)?
    let fs = compile(gl, FRAGMENT_SHADER, fs_src)?
    let program = gl.create_program()?
    gl.attach_shader(program, vs); gl.attach_shader(program, fs)
    gl.link_program(program)
    if !gl.get_program_link_status(program):
        return Err(DisplayError::Shader(gl.get_program_info_log(program)))
    gl.detach_shader(program, vs); gl.detach_shader(program, fs)
    gl.delete_shader(vs); gl.delete_shader(fs)
    Ok(program)

fn new(gl, vertices):  // unsafe { glow calls }
    let program = link(gl, VERTEX_SRC, FRAGMENT_SRC)?
    let vao = gl.create_vertex_array()?
    let vbo = gl.create_buffer()?
    gl.bind_vertex_array(Some(vao))
    gl.bind_buffer(ARRAY_BUFFER, Some(vbo))
    let bytes = slice::from_raw_parts(vertices.as_ptr() as *const u8, vertices.len() * 4)
    gl.buffer_data_u8_slice(ARRAY_BUFFER, bytes, STATIC_DRAW)
    let stride = 6 * 4
    gl.enable_vertex_attrib_array(0)
    gl.vertex_attrib_pointer_f32(0, 2, FLOAT, false, stride, 0)
    gl.enable_vertex_attrib_array(1)
    gl.vertex_attrib_pointer_f32(1, 4, FLOAT, false, stride, 2 * 4)
    gl.bind_vertex_array(None)
    Ok(ColoredMesh { program, vao, vbo, vertex_count: (vertices.len() / 6) as i32 })

fn draw(gl):  // unsafe { glow calls }
    gl.use_program(Some(self.program))
    gl.bind_vertex_array(Some(self.vao))
    gl.draw_arrays(TRIANGLES, 0, self.vertex_count)
    gl.bind_vertex_array(None)
    gl.use_program(None)
```

## Internal: `default_format.rs`

```rust
pub(crate) const BACKGROUND: [f32; 4] = [0.90, 0.62, 0.10, 1.0]; // amber
pub(crate) const X_COLOR: [f32; 4] = [0.05, 0.45, 0.15, 1.0];    // green
const MARGIN: f32 = 0.0;         // 0.0 = the x runs corner to corner
const HALF_THICKNESS: f32 = 0.05; // NDC half-width of an x bar

/// Interleaved `[x, y, r, g, b, a]` for the x (two bars, 12 vertices).
pub(crate) fn vertices() -> Vec<f32>;
```

Pseudocode:

```
fn push_bar(out, a: [f32;2], b: [f32;2], half, color):
    d = normalize(b - a)
    n = [-d[1], d[0]] * half
    p0 = a + n; p1 = a - n; p2 = b + n; p3 = b - n
    for p in [p0, p1, p2,  p2, p1, p3]:
        out.extend([p[0], p[1], color[0], color[1], color[2], color[3]])

fn vertices():
    let mut out = Vec::new()
    let a = 1.0 - MARGIN
    push_bar(&mut out, [-a, -a], [a, a], HALF_THICKNESS, X_COLOR)  // "\"
    push_bar(&mut out, [-a,  a], [a, -a], HALF_THICKNESS, X_COLOR) // "/"
    out
```

## `Display::new` / `Display::render` pseudocode

```
fn new(gl):
    let vertices = default_format::vertices()
    let default = ColoredMesh::new(&gl, &vertices)?   // documented: context current on this thread
    Ok(Display { gl, default })

fn render(&mut self, framebuffer_height, region):
    if region.width <= 0 || region.height <= 0: return
    let gl = &self.gl
    let y_gl = framebuffer_height - region.y - region.height   // GL origin bottom-left
    unsafe {   // documented: context current on this thread
        gl.enable(SCISSOR_TEST)
        gl.scissor(region.x, y_gl, region.width, region.height)
        gl.clear_color(BACKGROUND...)
        gl.clear(COLOR_BUFFER_BIT)
        gl.disable(SCISSOR_TEST)

        gl.viewport(region.x, y_gl, region.width, region.height)
        gl.disable(DEPTH_TEST)
        gl.enable(BLEND)
        gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)
        self.default.draw(gl)
        gl.disable(BLEND)
    }
```

The region is square (the app guarantees it), so NDC maps 1:1 and the x is not stretched. Geometry outside NDC `[-1, 1]` is clipped at the region edges, so the corner-to-corner bar ends are trimmed.

## App changes

### `rust/ui/lil-d/src/app.rs`
- Add `use display_lib::{Display, Region};` and an `App` field `display: Option<Display>`.
- `setup`: after the glow context is created, `let display = Display::new(Arc::clone(&gl)).expect("display-lib init");` and store it.
- `render`, step 1: keep clearing the whole window to `CONSOLE_BG`; when `self.state.format_on`, call `display.render(size.height as i32, Region { x: layout.format.x, y: layout.format.y, width: layout.format.w, height: layout.format.h })`. Otherwise the region stays `CONSOLE_BG`.
- Remove the `FORMAT_GREEN` constant and the `paint_format` function (the library owns format rendering). Keep `clear` for the window background.
- The console (egui) path and the console-height derivation are unchanged.

### `rust/ui/lil-d/src/gl_window.rs`
Replace the version-less context attributes + GLES fallback (`gl_window.rs:56-68`) with a single OpenGL 3.3 core request:

```rust
let context_attributes = glutin::context::ContextAttributesBuilder::new()
    .with_context_api(glutin::context::ContextApi::OpenGl(Some(
        glutin::context::Version::new(3, 3),
    )))
    .with_profile(glutin::context::GlProfile::Core)
    .build(raw_window_handle);
let not_current_gl_context = gl_display
    .create_context(&gl_config, &context_attributes)
    .expect("failed to create GL 3.3 context");
```

## Tests

A `#[cfg(test)]` module in `default_format.rs` (no GL): `vertices()` returns `12 * 6` floats; `push_bar` appends 6 vertices; every coordinate is within `[-1.0, 1.0]` (allowing for the clipped bar ends, assert the finite/expected range for the endpoints). The window itself is verified by the minimal test.

## Minimal test

```
.\scripts\build.ps1
.\scripts\launch-rust.ps1
```

The delegated region shows the default format (amber background with a green x from corner to corner); the power button still toggles the region; `.\scripts\cleanup.ps1` stops it.

## Risks and follow-ups

* **GL version / shader compatibility.** Requesting 3.3 core makes `#version 330 core` reliable on a real GPU; it drops the GLES fallback. If a software/EGL-only path is ever needed, the library would need a GLES shader variant.
* **Unenforced GL-context precondition.** The "context current on the calling thread" assumption is documented, not checked by the compiler (as in `egui_glow`/`glow`); calling the library from a thread without a current context is undefined behavior. This is the only precondition relaxed this way; other safety concerns are handled normally.
* **Square-region assumption.** The x geometry is NDC-based and assumes a square region; a non-square region would stretch it.
* **Resource lifetime.** `Display` does not free GL objects on exit (the context is torn down with the process); add a `destroy` when a format can be swapped at runtime (`format-manager`).
* **No format abstraction yet.** `Display` hardcodes the default format; the registry/active-format design lands with `format-manager`.
* **Colors/geometry** are provisional placeholders.

## Files

* Authored: `rust/lib/display-lib/Cargo.toml`, `rust/lib/display-lib/src/{lib,gl,default_format}.rs`.
* Changed: `rust/Cargo.toml`, `rust/ui/lil-d/Cargo.toml`, `rust/ui/lil-d/src/app.rs`, `rust/ui/lil-d/src/gl_window.rs`.
* Planning: `planning/backlog.md` (status -> done when complete), `planning/current_story.md`.
