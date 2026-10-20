# Current story: earth-rendering

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Render the earth cube map on a sphere in the display library, orient it with the attitude quaternion, and light it from the upper right, completing the hello-world format. The cube map, attitude, and light direction are hardcoded for now.

- **Depends on:** display-lib.
- **Minimal test:** the tilted, lit earth is visible in the delegated region.

## Context

`formats/hello_world.rs` currently draws a dark background and the centered greeting via `text::TextRenderer` (`hello_world.rs:39`). The assets already ship: `assets/earth/{px,nx,py,ny,pz,nz}.jpg` (1024x1024, ~80 KB each) with the exact face convention in `assets/earth/CREDITS.md`, which matches OpenGL cube-map sampling (`rust/tool/equirect-to-cubemap/src/cubemap.rs:45` `face_direction` is the inverse of the GL spec's face selection). `scripts/build.ps1` copies all of `assets/` recursively, so no deploy change is needed. `image` is already a workspace dependency with `jpeg` (`rust/Cargo.toml`).

Two facts constrain the design:

* `lil-d` currently requests **no depth buffer** (`rust/ui/lil-d/src/gl_window.rs:42`, `with_depth_size(0)`), and the ABI carries no depth information. The earth uses depth testing, so the display application must request a depth buffer (a documented precondition of the display library).
* Shader `compile`/`link` are private to `text.rs` (`text.rs:440`, `text.rs:452`). Earth is the **second user**, the architecture's "extract shared code when a second user appears" case.

## Decisions

* **Shared helper module `shader.rs`.** Move `compile`/`link` out of `text.rs` into a focused top-level `shader` module, used by both `text` and `earth`. The name is deliberately narrow: the only universal GL concern the two share is shader program compilation and linking.
* **Earth capability `earth.rs`.** A top-level module (parallel to `text.rs`) owning the cube map, the sphere mesh, the shader, and the draw. Format-specific values (attitude, light) live in `hello_world.rs`.
* **Perspective projection.** Unit sphere at the origin, camera at `(0, 0, d)` looking down `-Z`; a combined `u_viewproj` (mat4) uniform. `fovy = 45 deg`, `d = 1 / sin(GLOBE_FRACTION * fovy/2)` with `GLOBE_FRACTION = 0.9`, so the globe fills about 90% of the half-fov; `near`/`far` bracket the sphere.
* **Depth buffer in `lil-d`.** `gl_window.rs` requests `with_depth_size(24)`; `app.rs`'s `clear()` clears `COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT` each frame. `earth.render` enables `DEPTH_TEST` with `depth_func(LESS)` and disables it before the text; back-face culling is kept as a cheap optimization.
* **Cube map.** Six JPEGs loaded with `image::open(..).to_rgb8()`, uploaded as a `GL_TEXTURE_CUBE_MAP` in face order `px, nx, py, ny, pz, nz`. Rows are uploaded in memory order **without vertical flip**, because the CREDITS `t` axis runs downward and GL's first row is `t = 0`. `LINEAR_MIPMAP_LINEAR` min / `LINEAR` mag, `CLAMP_TO_EDGE` S/T, `generate_mipmap`.
* **Sphere.** A unit-direction UV sphere generated on the CPU, `+Y` = north (matching the cube convention). The only vertex attribute is `a_dir: vec3` (position = normal = cube-sample direction). Indexed draw via a `GL_ELEMENT_ARRAY_BUFFER`; `sphere_indices` emits CCW-outward winding.
* **Orientation and lighting.** Vertex shader: `view = u_attitude * a_dir`, passing `v_dir = a_dir` (object space, for the cube sample) and `v_normal = view` (view space, so the light stays fixed to the region while the texture rotates). Fragment: `tex * (ambient + max(dot(normalize(v_normal), normalize(u_light)), 0))`, `ambient = 0.15`, `u_light = normalize(vec3(0.6, 0.8, 0.7))` (upper right, toward the viewer).
* **Attitude placeholder.** 23.5 deg about `+X`: `(sin(11.75 deg), 0, 0, cos(11.75 deg))`.
* **Error.** Add `DisplayError::Image(String)` for a face that fails to load or decode.
* **Deploy.** No `build.ps1` change (`assets/` is already copied recursively). Add `image = { workspace = true }` to `display-lib/Cargo.toml`.

## Interfaces

### `rust/lib/display-lib/src/shader.rs` (new)

```rust
pub(crate) fn compile(gl: &glow::Context, kind: u32, src: &str) -> Result<glow::Shader, DisplayError>;
pub(crate) fn link(gl: &glow::Context, vs_src: &str, fs_src: &str) -> Result<glow::Program, DisplayError>;
```

### `rust/lib/display-lib/src/earth.rs` (new)

```rust
pub(crate) const FACE_FILES: [&str; 6] =
    ["earth/px.jpg", "earth/nx.jpg", "earth/py.jpg",
     "earth/ny.jpg", "earth/pz.jpg", "earth/nz.jpg"];
pub(crate) const STACKS: u32 = 32;   // latitude bands
pub(crate) const SLICES: u32 = 64;   // longitude segments
pub(crate) const FOV_Y: f32 = std::f32::consts::FRAC_PI_4;   // 45 deg
pub(crate) const GLOBE_FRACTION: f32 = 0.9;                  // of the half-fov

pub(crate) struct Earth {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    texture: glow::Texture,
    index_count: i32,
    viewproj_loc: Option<glow::UniformLocation>,
    attitude_loc: Option<glow::UniformLocation>,
    light_loc:    Option<glow::UniformLocation>,
    cube_loc:     Option<glow::UniformLocation>,
}
impl Earth {
    pub(crate) fn new(gl: &glow::Context, assets_root: &std::path::Path) -> Result<Earth, DisplayError>;
    /// Draw into the region; the viewport and scissor are already set to (width, height).
    pub(crate) fn render(&self, gl: &glow::Context, width: i32, height: i32,
                         attitude: [f32; 4], light: [f32; 3]);
}

pub(crate) fn sphere_vertices(stacks: u32, slices: u32) -> Vec<[f32; 3]>;   // unit directions
pub(crate) fn sphere_indices(stacks: u32, slices: u32) -> Vec<u32>;         // CCW outward
pub(crate) fn quat_to_mat3(q: [f32; 4]) -> [f32; 9];                        // column-major
pub(crate) fn perspective_view_proj(aspect: f32) -> [f32; 16];             // column-major
```

### `rust/lib/display-lib/src/formats/hello_world.rs` (changed)

```rust
/// Axial-tilt placeholder: 23.5 deg about +X.
const ATTITUDE: [f32; 4] = [0.20364, 0.0, 0.0, 0.97908];
/// Light direction (view space): upper right, toward the viewer.
const LIGHT_DIR: [f32; 3] = [0.6, 0.8, 0.7];

pub(crate) struct HelloWorld { earth: Earth, text: TextRenderer }
// new():    earth = Earth::new(gl, assets_root)?; text = TextRenderer::new(gl, font)?
// render(): if width <= 0 || height <= 0: return
//           scissor + clear color; viewport; disable DEPTH_TEST
//           earth.render(gl, width, height, ATTITUDE, LIGHT_DIR)
//           text.draw(gl, width, height, GREETING, TEXT_COLOR, height * FONT_SIZE_FRACTION)
```

### `rust/lib/display-lib/src/lib.rs` (changed)

```rust
mod earth; mod shader; mod text; mod formats;

enum DisplayError { Shader(String), Io(String), Font(String), Image(String) }   // +Image
```

### `rust/ui/lil-d/src/gl_window.rs` / `rust/ui/lil-d/src/app.rs` (changed)

```rust
// gl_window.rs: ConfigTemplateBuilder ... .with_depth_size(24) ...
// app.rs clear(): gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
```

## Pseudocode

### `Earth::new`

```
program = shader::link(gl, VERTEX_SRC, FRAGMENT_SRC)?
locs = get_uniform_location for { u_viewproj, u_attitude, u_light, u_cube }
verts = sphere_vertices(STACKS, SLICES); idx = sphere_indices(STACKS, SLICES)
vao, vbo, ebo = create
bind vao
    bind ARRAY_BUFFER vbo; buffer_data_u8_slice(as_bytes(verts), STATIC_DRAW)
    enable_vertex_attrib_array(0); vertex_attrib_pointer_f32(0, 3, FLOAT, false, 12, 0)
    bind ELEMENT_ARRAY_BUFFER ebo; buffer_data_u8_slice(as_bytes(idx), STATIC_DRAW)
unbind vao
texture = load_cube_map(gl, assets_root)?
return Earth { .., index_count: idx.len() as i32 }
```

### `load_cube_map`

```
tex = create_texture(); bind TEXTURE_CUBE_MAP
size = None
for (i, file) in FACE_FILES.enumerate():
    img = image::open(assets_root.join(file)).map_err(DisplayError::Image)?.to_rgb8()
    if size.is_none(): size = (img.width(), img.height())
    tex_image_2d(CUBE_MAP_POSITIVE_X + i, 0, RGB8, w, h, 0, RGB, UNSIGNED_BYTE, Some(img.as_raw()))
tex_parameter_i32(MIN_FILTER, LINEAR_MIPMAP_LINEAR); tex_parameter_i32(MAG_FILTER, LINEAR)
tex_parameter_i32(WRAP_S, CLAMP_TO_EDGE); tex_parameter_i32(WRAP_T, CLAMP_TO_EDGE)
generate_mipmap(TEXTURE_CUBE_MAP)
```

### `Earth::render`

```
if width <= 0 || height <= 0: return
use_program(program)
uniform_matrix_4_f32_slice(viewproj_loc, false, &perspective_view_proj(width as f32 / height as f32))
uniform_matrix_3_f32_slice(attitude_loc, false, &quat_to_mat3(attitude))
uniform_3_f32(light_loc, normalize(light))
active_texture(TEXTURE0); bind_texture(TEXTURE_CUBE_MAP, texture); uniform_1_i32(cube_loc, 0)
enable(DEPTH_TEST); depth_func(LESS)
enable(CULL_FACE); cull_face(BACK); front_face(CCW)
bind_vertex_array(vao); draw_elements(TRIANGLES, index_count, UNSIGNED_INT, 0)
bind_vertex_array(None); disable(CULL_FACE); disable(DEPTH_TEST); use_program(None)
```

### Shaders

```glsl
// vertex
#version 330 core
layout(location = 0) in vec3 a_dir;
uniform mat4 u_viewproj;
uniform mat3 u_attitude;
out vec3 v_dir;
out vec3 v_normal;
void main() {
    vec3 view = u_attitude * a_dir;
    v_dir = a_dir;
    v_normal = view;
    gl_Position = u_viewproj * vec4(view, 1.0);
}

// fragment
#version 330 core
in vec3 v_dir;
in vec3 v_normal;
uniform samplerCube u_cube;
uniform vec3 u_light;
out vec4 f_color;
void main() {
    float diff = max(dot(normalize(v_normal), normalize(u_light)), 0.0);
    vec3 tex = texture(u_cube, normalize(v_dir)).rgb;
    f_color = vec4(tex * (0.15 + diff), 1.0);
}
```

### `sphere_vertices` / `sphere_indices`

```
for i in 0..=stacks:
    phi = pi * i / stacks
    for j in 0..=slices:
        theta = 2 * pi * j / slices
        push [ sin(phi) * cos(theta), cos(phi), sin(phi) * sin(theta) ]
for i in 0..stacks:
    for j in 0..slices:
        a = i * (slices + 1) + j; b = a + slices + 1
        push a, b, a + 1   and   a + 1, b, b + 1
```

### `perspective_view_proj`

```
d = 1.0 / (GLOBE_FRACTION * (FOV_Y / 2).sin())
near = d - 1.5; far = d + 1.5; f = 1.0 / (FOV_Y / 2).tan()
P = [ f/aspect, 0, 0, 0,
      0, f, 0, 0,
      0, 0, (far + near) / (near - far), -1,
      0, 0, 2 * far * near / (near - far), 0 ]   // column-major
V = translate(0, 0, -d)
return P * V
```

### `quat_to_mat3` (x, y, z, w)

```
xx=x*x, yy=y*y, zz=z*z, xy=x*y, xz=x*z, yz=y*z, wx=w*x, wy=w*y, wz=w*z
[ 1-2(yy+zz), 2(xy+wz),   2(xz-wy),
  2(xy-wz),   1-2(xx+zz), 2(yz+wx),
  2(xz+wy),   2(yz-wx),   1-2(xx+yy) ]
```

## Tests

No GL context:

* `quat_to_mat3`: identity quaternion gives the identity matrix; 90 deg about Z maps `(1, 0, 0)` to `(0, 1, 0)`; the columns stay orthonormal.
* `sphere_vertices`: count is `(STACKS + 1) * (SLICES + 1)`, every vertex is unit length, and `(0, 1, 0)` is at the north pole.
* `sphere_indices`: count is `STACKS * SLICES * 6`, every index is in range, and a known triangle winds CCW outward.
* `perspective_view_proj`: the sphere's front point `(0, 0, d - 1)` projects near the NDC center with `w > 0`.
* Assets: load the six `REPO_ROOT/assets/earth/*.jpg`, assert they are all equal square sizes.
* Existing `text.rs` tests still pass (they now reach `crate::shader::link`).

## Minimal test

```
.\scripts\build.ps1
.\scripts\launch-rust.ps1
```

The delegated region shows the tilted, lit earth behind "Hello, world!"; the power button still toggles the region; `.\scripts\cleanup.ps1` stops it.

Negative check: rename `build\deploy\assets\earth\px.jpg`; `lil-d` fails at startup with the loader's `last_error` (image).

## Risks and follow-ups

* **Depth buffer is a new display-application requirement.** `lil-d` must clear depth each frame, and the future C++ display app must request and clear a depth buffer too. The ABI still carries no depth information (a documented precondition).
* **Winding / `front_face`.** An inverted winding makes the globe render inside-out; this is visible in the minimal test.
* **Cube orientation.** Verify continents are upright and not mirrored; the rows are deliberately uploaded unflipped.
* **`shader` module boundary.** If a broader set of universal GL helpers emerges (for example the deferred `mesh`), revisit the boundary then.
* **C++ side.** The C++ display library reimplements this pipeline against the same cube faces.
* **`docs/versions.md`.** Add the `image` crate to the recorded Rust dependencies (optional).

## Module layout

Post-2018 layout (`foo.rs` plus a `foo/` directory; no `mod.rs`):

```
src/lib.rs                    ABI, Display, DisplayError, assets_root
src/shader.rs                 shared shader compile/link
src/text.rs                   text rendering (Font, TextRenderer)
src/earth.rs                  earth rendering (cube map, sphere, shader)
src/formats.rs                declares the live formats
src/formats/hello_world.rs    the hello-world format (background, earth, text)
src/formats/default.rs        the default format (dormant)
```

* `shader.rs` is a new shared capability: `text.rs` and `earth.rs` both link shader programs.
* `earth.rs` is a new rendering capability, parallel to `text.rs`.
* `formats/hello_world.rs` composes the background, the earth, and the text, and owns the format's values (attitude, light, greeting, colors).

## Files

* Authored: `rust/lib/display-lib/src/earth.rs`, `rust/lib/display-lib/src/shader.rs`.
* Changed: `rust/lib/display-lib/src/formats/hello_world.rs`, `rust/lib/display-lib/src/text.rs`, `rust/lib/display-lib/src/lib.rs`, `rust/lib/display-lib/Cargo.toml`, `rust/ui/lil-d/src/gl_window.rs`, `rust/ui/lil-d/src/app.rs`, `planning/{architecture,backlog,current_story}.md`, optionally `docs/versions.md`.
