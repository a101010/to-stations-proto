//! The display library.
//!
//! Renders the active format into the region of the window that the display
//! application delegates to it. The application owns the window, the GL
//! context, and the layout; it passes the format region and the framebuffer
//! height to [`Display::render`].
//!
//! Every function that calls OpenGL assumes a glow context is current on the
//! calling thread (glow marks every GL call `unsafe` and has no token that
//! proves currentness). The display application upholds this by doing all GL
//! work on its event-loop thread. This documented assumption is limited to the
//! GL-context-current precondition; other safety concerns are handled normally.

mod default_format;
mod gl;

use std::sync::Arc;

use glow::HasContext as _;

/// A rectangle in physical pixels, origin top-left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// An error building the renderer.
#[derive(Debug)]
pub enum DisplayError {
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
    pub fn new(gl: Arc<glow::Context>) -> Result<Self, DisplayError> {
        let vertices = default_format::vertices();
        let default = gl::ColoredMesh::new(&gl, &vertices)?;
        Ok(Self { gl, default })
    }

    /// Render the default format into `region`.
    ///
    /// `framebuffer_height` is the whole framebuffer height, used to flip
    /// `region`'s top-left origin into GL's bottom-left origin.
    ///
    /// Assumes a glow context is current on the calling thread.
    pub fn render(&mut self, framebuffer_height: i32, region: Region) {
        if region.width <= 0 || region.height <= 0 {
            return;
        }
        let gl = &self.gl;
        let y_gl = framebuffer_height - region.y - region.height;
        let bg = default_format::BACKGROUND;
        unsafe {
            // Background: a scissor clear of the region.
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(region.x, y_gl, region.width, region.height);
            gl.clear_color(bg[0], bg[1], bg[2], bg[3]);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.disable(glow::SCISSOR_TEST);

            // The x, in the region's NDC.
            gl.viewport(region.x, y_gl, region.width, region.height);
            gl.disable(glow::DEPTH_TEST);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            self.default.draw(gl);
            gl.disable(glow::BLEND);
        }
    }
}
