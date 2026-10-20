//! Shared shader-program helpers.
//!
//! `compile` and `link` are the one universal GL concern shared by the
//! rendering capabilities (`text`, `earth`), so they live here rather than in
//! either. Every function assumes a glow context is current on the calling
//! thread (see `architecture.md`, "OpenGL context and safety").

use glow::HasContext as _;

use crate::DisplayError;

/// Compile one shader stage.
pub(crate) fn compile(gl: &glow::Context, kind: u32, src: &str) -> Result<glow::Shader, DisplayError> {
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

/// Compile and link a vertex/fragment shader program.
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
