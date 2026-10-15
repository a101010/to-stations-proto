//! The hello-world format: a dark background with the greeting text.
//!
//! The earth sphere is added by `earth-rendering`; the greeting and attitude
//! come from the view service in `hello-view-service`. For now the greeting is
//! hardcoded.

use std::path::Path;

use glow::HasContext as _;

use crate::text::TextRenderer;
use crate::DisplayError;

/// Background of the hello-world format.
pub(crate) const BACKGROUND: [f32; 4] = [0.02, 0.03, 0.08, 1.0];
/// Greeting color.
pub(crate) const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
/// The hardcoded greeting.
pub(crate) const GREETING: &str = "Hello, world!";
/// Font, relative to the assets root.
const FONT_FILE: &str = "fonts/FiraSans-Regular.ttf";
/// Text size as a fraction of the region height.
const FONT_SIZE_FRACTION: f32 = 0.15;

/// The hello-world format.
pub(crate) struct HelloWorld {
    text: TextRenderer,
}

impl HelloWorld {
    /// Build the format, loading the font from `assets_root`.
    pub(crate) fn new(gl: &glow::Context, assets_root: &Path) -> Result<Self, DisplayError> {
        let text = TextRenderer::new(gl, &assets_root.join(FONT_FILE))?;
        Ok(Self { text })
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
        }

        self.text.draw(
            gl,
            width,
            height,
            GREETING,
            TEXT_COLOR,
            height as f32 * FONT_SIZE_FRACTION,
        );
    }
}
