# Current story: font-handling

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Load `FiraSans-Regular.ttf` from the assets directory in the display library and render the hardcoded hello_world greeting with HarfBuzz + hb-gpu as the active format. The `default_format` is retained but not rendered until the format manager exists.

- **Depends on:** display-lib, harfbuzz.
- **Minimal test:** the delegated region shows the greeting rendered in the loaded font.

## Context

`display-lib` is a `cdylib` implementing `include/display_lib.h`; its `Display` owns a `glow::Context` built from the app-supplied loader and renders `default_format` (`lib.rs:58`). The `harfbuzz` crate (story done) exposes shaping and the `hb-gpu` encoder, and its DLLs are linked from the external prefix. The ABI is unchanged: the library resolves its own assets from the process exe path, and `display_lib_render` gains an internal `&mut` (no signature change). `architecture.md` ("Text rendering") records the approach.

## Decisions

* **Assets.** The library resolves its assets from the process exe path: `<exe_dir>/../assets` (exe at `build/deploy/bin`, assets at `build/deploy/assets`). No asset path crosses the ABI.
* **Font.** `assets/fonts/FiraSans-Regular.ttf`, loaded at `display_lib_create`.
* **Active format.** `render` draws the hello-world format; `default` is retained (unused) with `#[allow(dead_code)]`.
* **Text.** Shape with `hb_shape` at the face's units per em; encode glyphs on demand into a growable `GL_TEXTURE_BUFFER` atlas (`GL_RGBA16I`); rasterize with the hb-gpu Slug fragment shader.
* **Mutable render.** The atlas grows during drawing, so `Display::render` takes `&mut self` and `display_lib_render` casts to `*mut Display`.
* **Colors.** Hello-world background `[0.02, 0.03, 0.08, 1.0]`, text `[1.0, 1.0, 1.0, 1.0]` (placeholders).
* **Offsets.** Shaping `x_offset`/`y_offset` are applied to each glyph's drawn position (needed for mark attachment and complex scripts); only advances move the pen.
* **Deploy.** `scripts/build.ps1` copies `assets/` to `build/deploy/assets` and `harfbuzz.dll` + `harfbuzz-gpu.dll` to `build/deploy/bin`.

## Interfaces

### Used from `rust/lib/harfbuzz` (existing)

```rust
Blob::from_bytes(&[u8]) -> Blob
Face::new(&Blob) -> Face ;  Face::upem(&self) -> u32
Font::new(&Face) -> Font ;  Font::set_scale(&mut self, i32, i32) ; Font::scale(&self) -> (i32, i32)
Buffer::new() -> Buffer ; Buffer::add_str(&mut self, &str) ; Buffer::guess_segment_properties(&mut self)
Buffer::shape(&mut self, &Font) ; Buffer::glyph_infos(&self) -> Vec<hb_glyph_info_t> ; Buffer::glyph_positions(&self) -> Vec<hb_glyph_position_t>
GpuDraw::new() -> Option<GpuDraw> ; GpuDraw::glyph(&mut self, &Font, u32) ; GpuDraw::encode(&mut self) -> (Vec<u8>, hb_glyph_extents_t)
shader_source(ShaderStage, ShaderLang) -> Option<&'static str> ; draw_shader_source(ShaderStage, ShaderLang) -> Option<&'static str>
// hb_glyph_info_t { codepoint: u32, .. }
// hb_glyph_position_t { x_advance, y_advance, x_offset, y_offset: i32, .. }
// hb_glyph_extents_t { x_bearing, y_bearing, width, height: i32 }
```

### `rust/lib/display-lib/src/text.rs` (new)

```rust
pub(crate) const INITIAL_ATLAS_BYTES: usize = 4096;   // multiple of 8

pub(crate) struct Font {
    _blob: harfbuzz::Blob,
    _face: harfbuzz::Face,
    font: harfbuzz::Font,      // scale set to upem
    upem: f32,
}
impl Font {
    pub fn load(path: &std::path::Path) -> Result<Font, DisplayError>;
    pub fn shape(&self, text: &str) -> Vec<Glyph>;     // font units
}

pub(crate) struct Glyph { pub id: u32, pub x_advance: f32, pub y_advance: f32, pub x_offset: f32, pub y_offset: f32 }

struct AtlasGlyph { loc: u32, min_x: f32, max_x: f32, min_y: f32, max_y: f32, empty: bool }

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex { pos: [f32; 2], tex: [f32; 2], normal: [f32; 2], em_per_pos: f32, glyph_loc: u32 } // 32 bytes

pub(crate) struct TextRenderer {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    atlas_buffer: glow::Buffer,       // GL_TEXTURE_BUFFER
    atlas_texture: glow::Texture,     // GL_RGBA16I view
    atlas_loc: Option<glow::UniformLocation>,
    mvp_loc: Option<glow::UniformLocation>,
    viewport_loc: Option<glow::UniformLocation>,
    fg_loc: Option<glow::UniformLocation>,
    atlas_data: Vec<u8>,              // CPU copy; length multiple of 8
    atlas_capacity: usize,
    glyphs: std::collections::HashMap<u32, AtlasGlyph>,
    encoder: harfbuzz::GpuDraw,
    font: Font,
    vertices: Vec<Vertex>,
}
impl TextRenderer {
    pub fn new(gl: &glow::Context, font_path: &std::path::Path) -> Result<TextRenderer, DisplayError>;
    /// Draw `text` centered in a `width`x`height` pixel region (viewport already set).
    pub fn draw(&mut self, gl: &glow::Context, width: i32, height: i32,
                text: &str, color: [f32; 4], font_size_px: f32);

    fn ensure_glyph(&mut self, gl: &glow::Context, gid: u32);
    fn build_vertices(&self, glyphs: &[Glyph], width: i32, height: i32, font_size_px: f32) -> Vec<Vertex>;
}

fn assemble_vertex_shader() -> String;
fn assemble_fragment_shader() -> String;
fn ortho_pixels(width: i32, height: i32) -> [f32; 16];   // column-major, pixels -> NDC
fn push_quad(v: &mut Vec<Vertex>, g: &AtlasGlyph, pen_x: f32, pen_y: f32, scale: f32, em_per_pos: f32);
```

### `rust/lib/display-lib/src/hello_world.rs` (new)

```rust
pub(crate) const BACKGROUND: [f32; 4];   // [0.02, 0.03, 0.08, 1.0]
pub(crate) const TEXT_COLOR: [f32; 4];   // [1.0, 1.0, 1.0, 1.0]
pub(crate) const GREETING: &str = "Hello, world!";
const FONT_FILE: &str = "fonts/FiraSans-Regular.ttf";
const FONT_SIZE_FRACTION: f32 = 0.15;    // of region height

pub(crate) struct HelloWorld { text: text::TextRenderer }
impl HelloWorld {
    pub fn new(gl: &glow::Context, assets_root: &std::path::Path) -> Result<HelloWorld, DisplayError>;
    pub fn render(&mut self, gl: &glow::Context, framebuffer_height: i32, x: i32, y: i32, w: i32, h: i32);
}
```

### `rust/lib/display-lib/src/lib.rs` (changed)

```rust
mod hello_world; mod text;

enum DisplayError { Shader(String), Io(String), Font(String) }   // +Io, +Font

struct Display {
    gl: glow::Context,
    hello_world: hello_world::HelloWorld,
    #[allow(dead_code)] default: gl::ColoredMesh,   // used by format-manager later
}
impl Display {
    fn new(loader: impl FnMut(&str) -> *const c_void) -> Result<Self, DisplayError>;
    fn render(&mut self, framebuffer_height: i32, x: i32, y: i32, width: i32, height: i32);
}
fn assets_root() -> std::path::PathBuf;   // current_exe().parent().parent().join("assets")
```

`display_lib_render` casts to `*mut Display` and calls `display.render(...)`.

### `rust/lib/display-lib/src/gl.rs` (changed)

`fn link` -> `pub(crate) fn link` (reused by `text.rs`).

### `rust/lib/display-lib/Cargo.toml` (changed)

Add `harfbuzz = { workspace = true }`.

### `scripts/build.ps1` (changed)

In the deploy step, copy `assets/` to `build/deploy/assets`, and copy `harfbuzz.dll` and `harfbuzz-gpu.dll` from `$extPathPrefix\harfbuzz\bin` to `build/deploy/bin`.

## Pseudocode

### `Font::load` / `Font::shape`

```
load(path):
    bytes = read(path)  -> DisplayError::Io on failure
    blob = Blob::from_bytes(bytes)
    face = Face::new(blob);  upem = face.upem()
    font = Font::new(face);  font.set_scale(upem, upem)
    return Font { _blob: blob, _face: face, font, upem: upem as f32 }

shape(text):
    buf = Buffer::new(); buf.add_str(text); buf.guess_segment_properties(); buf.shape(&self.font)
    zip(buf.glyph_infos(), buf.glyph_positions())
        .map(|(i, p)| Glyph {
            id: i.codepoint,
            x_advance: p.x_advance as f32, y_advance: p.y_advance as f32,
            x_offset: p.x_offset as f32, y_offset: p.y_offset as f32 })
        .collect()
```

### `TextRenderer::new`

```
font = Font::load(font_path)?
program = gl::link(gl, assemble_vertex_shader(), assemble_fragment_shader())?
locs = get_uniform_location(program, name) for { hb_gpu_atlas, u_matViewProjection, u_viewport, u_foreground }
vao = create_vertex_array(); vbo = create_buffer()
bind_vertex_array(vao); bind_buffer(ARRAY_BUFFER, vbo)
for i in 0..=4: enable_vertex_attrib_array(i)
vertex_attrib_pointer_f32(0, 2, FLOAT, false, 32, 0)
vertex_attrib_pointer_f32(1, 2, FLOAT, false, 32, 8)
vertex_attrib_pointer_f32(2, 2, FLOAT, false, 32, 16)
vertex_attrib_pointer_f32(3, 1, FLOAT, false, 32, 24)
vertex_attrib_pointer_i32(4, 1, UNSIGNED_INT, 32, 28)
bind_vertex_array(None)
atlas_buffer = create_buffer(); bind_buffer(TEXTURE_BUFFER, atlas_buffer)
buffer_data_size(TEXTURE_BUFFER, INITIAL_ATLAS_BYTES, DYNAMIC_DRAW)
atlas_texture = create_texture(); bind_texture(TEXTURE_BUFFER, atlas_texture)
tex_buffer(TEXTURE_BUFFER, RGBA16I, atlas_buffer)
encoder = GpuDraw::new() or Err(DisplayError::Font("gpu encoder"))
return TextRenderer { ..., atlas_data: [], atlas_capacity: INITIAL_ATLAS_BYTES, glyphs: {}, font, vertices: [] }
```

### `ensure_glyph`

Make sure the atlas holds glyph `gid`, encoding and uploading it on first use; idempotent.

```
if glyphs.contains_key(gid): return
encoder.glyph(&font.font, gid)
(bytes, ext) = encoder.encode()
if bytes.is_empty():                      // no ink (space)
    glyphs.insert(gid, AtlasGlyph::empty()); return
loc = atlas_data.len() / 8
atlas_data.extend_from_slice(&bytes)
bind_buffer(TEXTURE_BUFFER, atlas_buffer)
if atlas_data.len() > atlas_capacity:
    atlas_capacity = max(atlas_capacity * 2, atlas_data.len())
    buffer_data_size(TEXTURE_BUFFER, atlas_capacity, DYNAMIC_DRAW)
    buffer_sub_data_u8_slice(TEXTURE_BUFFER, 0, &atlas_data)
else:
    buffer_sub_data_u8_slice(TEXTURE_BUFFER, loc * 8, &bytes)
glyphs.insert(gid, AtlasGlyph {
    loc, empty: false,
    min_x: ext.x_bearing as f32, max_x: (ext.x_bearing + ext.width) as f32,
    min_y: (ext.y_bearing + ext.height) as f32, max_y: ext.y_bearing as f32 })
```

### `push_quad`

```
for (cx, cy) in [(0,0),(1,0),(0,1),(0,1),(1,0),(1,1)]:
    ex = lerp(g.min_x, g.max_x, cx);  ey = lerp(g.min_y, g.max_y, cy)
    v.push(Vertex {
        pos: [pen_x + scale * ex, pen_y - scale * ey],
        tex: [ex, ey],
        normal: [2 * cx - 1, 1 - 2 * cy],
        em_per_pos, glyph_loc: g.loc })
```

### `TextRenderer::draw`

```
glyphs = font.shape(text)
for g in glyphs: ensure_glyph(gl, g.id)
scale = font_size_px / font.upem
total = sum(g.x_advance) * scale
pen_x = (width as f32 - total) / 2.0
pen_y = height as f32 / 2.0 + 0.5 * font_size_px     // em-box baseline
em_per_pos = font.upem / font_size_px
vertices.clear()
for g in glyphs:
    if let Some(a) = glyphs.get(g.id) and !a.empty:
        push_quad(vertices, a, pen_x + scale * g.x_offset, pen_y - scale * g.y_offset, scale, em_per_pos)
    pen_x += g.x_advance * scale
bind_buffer(ARRAY_BUFFER, vbo); buffer_data_u8_slice(ARRAY_BUFFER, as_bytes(vertices), DYNAMIC_DRAW)
use_program(program)
uniform_matrix_4_f32_slice(mvp_loc, false, &ortho_pixels(width, height))
uniform_2_f32(viewport_loc, width as f32, height as f32)
uniform_4_f32(fg_loc, color[0], color[1], color[2], color[3])
active_texture(TEXTURE0); bind_texture(TEXTURE_BUFFER, atlas_texture); uniform_1_i32(atlas_loc, 0)
enable(BLEND); blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)
bind_vertex_array(vao); draw_arrays(TRIANGLES, 0, vertices.len())
disable(BLEND); bind_vertex_array(None); use_program(None)
```

### Shader assembly

```
assemble_vertex_shader() =
    "#version 330\n" + shader_source(Vertex, Glsl) + draw_shader_source(Vertex, Glsl) + VERTEX_MAIN

VERTEX_MAIN =
    layout(location=0) in vec2 a_position;
    layout(location=1) in vec2 a_texcoord;
    layout(location=2) in vec2 a_normal;
    layout(location=3) in float a_emPerPos;
    layout(location=4) in uint a_glyphLoc;
    uniform mat4 u_matViewProjection; uniform vec2 u_viewport;
    out vec2 v_texcoord; flat out uint v_glyphLoc;
    main():
        pos = a_position; tex = a_texcoord; jac = vec4(a_emPerPos, 0, 0, -a_emPerPos)
        hb_gpu_dilate(pos, tex, a_normal, jac, u_matViewProjection, u_viewport)
        gl_Position = u_matViewProjection * vec4(pos, 0, 1)
        v_texcoord = tex; v_glyphLoc = a_glyphLoc

assemble_fragment_shader() =
    "#version 330\n" + shader_source(Fragment, Glsl) + draw_shader_source(Fragment, Glsl) + FRAGMENT_MAIN

FRAGMENT_MAIN =
    uniform vec4 u_foreground;
    in vec2 v_texcoord; flat in uint v_glyphLoc; out vec4 f_color;
    main(): f_color = vec4(u_foreground.rgb, u_foreground.a * hb_gpu_draw(v_texcoord, v_glyphLoc))
```

### `ortho_pixels`

```
[ 2/w, 0, 0, 0,   0, -2/h, 0, 0,   0, 0, -1, 0,   -1, 1, 0, 1 ]   // column-major; (0,0)->(-1,1), (w,h)->(1,-1)
```

### `HelloWorld::render`

```
if w <= 0 or h <= 0: return
y_gl = framebuffer_height - y - h
enable(SCISSOR_TEST); scissor(x, y_gl, w, h); clear_color(BACKGROUND); clear(COLOR_BUFFER_BIT); disable(SCISSOR_TEST)
viewport(x, y_gl, w, h)
text.draw(gl, w, h, GREETING, TEXT_COLOR, h as f32 * FONT_SIZE_FRACTION)
```

### `Display::new` / `render` / `assets_root`

```
new(loader):
    gl = glow::Context::from_loader_function(loader)
    hello_world = HelloWorld::new(&gl, &assets_root())?
    default = gl::ColoredMesh::new(&gl, &default_format::vertices())?
    return Display { gl, hello_world, default }

render(fh, x, y, w, h): hello_world.render(&self.gl, fh, x, y, w, h)

assets_root():
    exe = current_exe(); dir = exe.parent()
    return dir.parent().map(|d| d.join("assets")).unwrap_or(PathBuf::from("assets"))
```

## Tests

* `text.rs` unit test (no GL): `build_vertices` with synthetic `Glyph`/`AtlasGlyph` asserts 6 vertices per non-empty glyph, total-advance centering, `em_per_pos` values, and that non-zero `x_offset`/`y_offset` shift the quad.
* `text.rs` unit test (no GL): `Font::load(REPO_ROOT/assets/fonts/FiraSans-Regular.ttf)` then `shape("Hello, world!")` asserts non-empty and some `x_advance > 0`.
* `default_format.rs` tests unchanged.
* No GL test; the shader compiles and the atlas uploads only with a live context.

## Minimal test

```
.\scripts\build.ps1
.\scripts\launch-rust.ps1
```

The delegated region shows a dark background with "Hello, world!" rendered in Fira Sans, centered and resolution-independent; the power button still toggles the region; `.\scripts\cleanup.ps1` stops it.

Negative check: remove or rename `build\deploy\assets\fonts\FiraSans-Regular.ttf`; `lil-d` fails at startup with the loader's `last_error` (font I/O).

## Risks and follow-ups

* **Atlas growth.** `buffer_data_size` reallocates in place and re-uploads the whole atlas; the texture-buffer association survives reallocation.
* **hb-gpu vertex mapping.** Corner/normal/`jac`/`emPerPos` follow the manual; verify dilation and coverage at small sizes.
* **Assets resolution.** `<exe_dir>/../assets` matches `build/deploy`; `cargo run` (exe under `build/rust`) will not find assets, so use the launch script.
* **Baseline.** Vertical centering approximates the em box (`center + 0.5 * font_size`); refine with font extents if needed.
* **Offsets.** Applied, but zero for the Latin greeting, so the minimal test does not exercise them; the `build_vertices` unit test covers a non-zero case.
* **C++ side.** The C++ display library reimplements this pipeline against the same HarfBuzz shaders.

## Result

Done and verified. `display-lib` loads `assets/fonts/FiraSans-Regular.ttf`, shapes the greeting with HarfBuzz, encodes glyphs into a growable `GL_TEXTURE_BUFFER` atlas (`GL_RGBA16I`), and rasterizes them with the hb-gpu Slug shader; the region shows "Hello, world!" in Fira Sans, centered on the dark background, with the console and power button intact. `scripts/build.ps1` now copies `assets/` to `build/deploy/assets` and `harfbuzz.dll` + `harfbuzz-gpu.dll` to `build/deploy/bin`. `scripts/build.ps1 -Test` and the deploy build pass; `display-lib` has 5 tests (2 new: vertex layout with offsets, font load + shape).

Deviations from the plan: `build_vertices` is a free function (taking `upem`, the atlas, and the glyphs) rather than a method, so it is unit-testable without a GL context; `DisplayError` keeps the planned `Shader`/`Io`/`Font` variants and uses `Shader` for GL object-creation failures. The default format and `ColoredMesh::draw` are marked `#[allow(dead_code)]` while retained for the format manager.

## Files

* Authored: `rust/lib/display-lib/src/text.rs`, `rust/lib/display-lib/src/hello_world.rs`.
* Changed: `rust/lib/display-lib/src/{lib.rs,gl.rs}`, `rust/lib/display-lib/Cargo.toml`, `scripts/build.ps1`, `README.md`, `planning/{backlog,current_story}.md`.
