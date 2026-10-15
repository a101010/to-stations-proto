//! The default format: an amber background with a green x.
//!
//! DORMANT: this file is retained but not declared in `formats.rs`, so it is not
//! compiled. The format manager reactivates it by declaring `mod default;`. It
//! is self-contained; when it returns, its shader helpers are reconciled with
//! the text renderer's.

use glow::HasContext as _;

use crate::DisplayError;

// --- Format data -----------------------------------------------------------

/// Amber background.
pub(crate) const BACKGROUND: [f32; 4] = [0.90, 0.62, 0.10, 1.0];
/// Green x.
pub(crate) const X_COLOR: [f32; 4] = [0.05, 0.45, 0.15, 1.0];
/// NDC inset from the region edge (`0.0` = the x runs corner to corner).
const MARGIN: f32 = 0.0;
/// NDC half-width of an x bar.
const HALF_THICKNESS: f32 = 0.05;

/// Interleaved `[x, y, r, g, b, a]` for the x (two bars, 12 vertices).
pub(crate) fn vertices() -> Vec<f32> {
    let mut out = Vec::new();
    let a = 1.0 - MARGIN;
    push_bar(&mut out, [-a, -a], [a, a], X_COLOR);
    push_bar(&mut out, [-a, a], [a, -a], X_COLOR);
    out
}

/// Append the two triangles of a bar from `a` to `b`, `HALF_THICKNESS` wide.
fn push_bar(out: &mut Vec<f32>, a: [f32; 2], b: [f32; 2], color: [f32; 4]) {
    let d = normalize(sub(b, a));
    let n = [-d[1] * HALF_THICKNESS, d[0] * HALF_THICKNESS];
    let p0 = add(a, n);
    let p1 = sub(a, n);
    let p2 = add(b, n);
    let p3 = sub(b, n);
    for p in [p0, p1, p2, p2, p1, p3] {
        out.extend_from_slice(&[p[0], p[1], color[0], color[1], color[2], color[3]]);
    }
}

fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn normalize(v: [f32; 2]) -> [f32; 2] {
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if len == 0.0 {
        v
    } else {
        [v[0] / len, v[1] / len]
    }
}

// --- Mesh ------------------------------------------------------------------

const VERTEX_SRC: &str = r#"#version 330 core
layout (location = 0) in vec2 a_pos;
layout (location = 1) in vec4 a_color;
out vec4 v_color;
void main() {
    v_color = a_color;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}"#;

const FRAGMENT_SRC: &str = r#"#version 330 core
in vec4 v_color;
out vec4 f_color;
void main() {
    f_color = v_color;
}"#;

/// Number of floats per vertex: `x, y, r, g, b, a`.
const FLOATS_PER_VERTEX: usize = 6;

/// A minimal colored-triangle mesh: a shader program plus a VAO/VBO.
pub(crate) struct ColoredMesh {
    program: glow::Program,
    vao: glow::VertexArray,
    #[allow(dead_code)] // kept for a future `destroy`
    vbo: glow::Buffer,
    vertex_count: i32,
}

impl ColoredMesh {
    /// Build the mesh from interleaved `[x, y, r, g, b, a]` vertices in NDC.
    ///
    /// Assumes a glow context is current on the calling thread.
    pub(crate) fn new(gl: &glow::Context, vertices: &[f32]) -> Result<Self, DisplayError> {
        let program = link(gl, VERTEX_SRC, FRAGMENT_SRC)?;
        unsafe {
            let vao = gl.create_vertex_array().map_err(DisplayError::Shader)?;
            let vbo = gl.create_buffer().map_err(DisplayError::Shader)?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

            let float_size = std::mem::size_of::<f32>();
            let bytes = std::slice::from_raw_parts(
                vertices.as_ptr() as *const u8,
                std::mem::size_of_val(vertices),
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);

            let stride = (FLOATS_PER_VERTEX * float_size) as i32;
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, stride, (2 * float_size) as i32);
            gl.bind_vertex_array(None);

            Ok(Self {
                program,
                vao,
                vbo,
                vertex_count: (vertices.len() / FLOATS_PER_VERTEX) as i32,
            })
        }
    }

    /// Draw the mesh. Assumes a glow context is current on the calling thread.
    pub(crate) fn draw(&self, gl: &glow::Context) {
        unsafe {
            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, self.vertex_count);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }
}

// --- Shader helpers --------------------------------------------------------

fn compile(gl: &glow::Context, kind: u32, src: &str) -> Result<glow::Shader, DisplayError> {
    unsafe {
        let shader = gl.create_shader(kind).map_err(DisplayError::Shader)?;
        gl.shader_source(shader, src);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            return Err(DisplayError::Shader(gl.get_shader_info_log(shader)));
        }
        Ok(shader)
    }
}

fn link(gl: &glow::Context, vs_src: &str, fs_src: &str) -> Result<glow::Program, DisplayError> {
    unsafe {
        let vs = compile(gl, glow::VERTEX_SHADER, vs_src)?;
        let fs = compile(gl, glow::FRAGMENT_SHADER, fs_src)?;
        let program = gl.create_program().map_err(DisplayError::Shader)?;
        gl.attach_shader(program, vs);
        gl.attach_shader(program, fs);
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            return Err(DisplayError::Shader(gl.get_program_info_log(program)));
        }
        gl.detach_shader(program, vs);
        gl.detach_shader(program, fs);
        gl.delete_shader(vs);
        gl.delete_shader(fs);
        Ok(program)
    }
}

// --- Format ----------------------------------------------------------------

/// The default format.
pub(crate) struct Default {
    mesh: ColoredMesh,
}

impl Default {
    /// Build the format.
    pub(crate) fn new(gl: &glow::Context) -> Result<Self, DisplayError> {
        let mesh = ColoredMesh::new(gl, &vertices())?;
        Ok(Self { mesh })
    }

    /// Render into the region (physical pixels, top-left origin).
    /// `framebuffer_height` is the whole framebuffer height.
    pub(crate) fn render(
        &mut self,
        gl: &glow::Context,
        framebuffer_height: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        if width <= 0 || height <= 0 {
            return;
        }
        let y_gl = framebuffer_height - y - height;
        let bg = BACKGROUND;
        unsafe {
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(x, y_gl, width, height);
            gl.clear_color(bg[0], bg[1], bg[2], bg[3]);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.disable(glow::SCISSOR_TEST);

            gl.viewport(x, y_gl, width, height);
            gl.disable(glow::DEPTH_TEST);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            self.mesh.draw(gl);
            gl.disable(glow::BLEND);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x_has_two_bars() {
        assert_eq!(vertices().len(), 12 * 6);
    }

    #[test]
    fn push_bar_adds_six_vertices() {
        let mut out = Vec::new();
        push_bar(&mut out, [-1.0, -1.0], [1.0, 1.0], X_COLOR);
        assert_eq!(out.len(), 6 * 6);
    }

    #[test]
    fn coordinates_are_near_ndc() {
        // Bar ends stick out past the corners by the half-thickness and are
        // clipped at the region edges, so allow that much overshoot.
        let limit = 1.0 + HALF_THICKNESS;
        for v in vertices().chunks(6) {
            assert!(v[0].abs() <= limit, "x out of range: {}", v[0]);
            assert!(v[1].abs() <= limit, "y out of range: {}", v[1]);
        }
    }
}
