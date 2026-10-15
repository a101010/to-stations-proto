//! A minimal colored-triangle mesh: a shader program plus a VAO/VBO.
//!
//! Every function here assumes a glow context is current on the calling thread
//! and wraps its glow calls in `unsafe { }` blocks; see the crate docs.

use glow::HasContext as _;

use crate::DisplayError;

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

#[allow(dead_code)] // the default format is retained for the format manager
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
    #[allow(dead_code)] // the default format is retained for the format manager
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

pub(crate) fn link(gl: &glow::Context, vs_src: &str, fs_src: &str) -> Result<glow::Program, DisplayError> {
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
