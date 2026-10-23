//! The hello-world format: a dark background with the earth and the greeting
//! text, rendered from the view data the display library receives.
//!
//! Until the view service's first sample arrives, the region is rendered black.

use std::path::Path;

use glow::HasContext as _;

use crate::earth::Earth;
use crate::text::TextRenderer;
use crate::view::{View, ViewState};
use crate::DisplayError;

/// Background of the hello-world format (once view data has arrived).
const BACKGROUND: [f32; 4] = [0.02, 0.03, 0.08, 1.0];
/// The region is cleared to black while there is no view data.
const NO_DATA_BACKGROUND: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
/// Greeting color.
const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
/// Light direction (view space): upper right, toward the viewer.
const LIGHT_DIR: [f32; 3] = [0.6, 0.8, 0.7];
/// Font, relative to the assets root.
const FONT_FILE: &str = "fonts/FiraSans-Regular.ttf";
/// Text size as a fraction of the region height.
const FONT_SIZE_FRACTION: f32 = 0.15;

/// The hello-world format.
pub(crate) struct HelloWorld {
    earth: Earth,
    text: TextRenderer,
    /// The latest view sample, shared with the display library's DDS poller.
    state: ViewState,
}

impl HelloWorld {
    /// Build the format, loading its assets from `assets_root`.
    pub(crate) fn new(
        gl: &glow::Context,
        assets_root: &Path,
        state: ViewState,
    ) -> Result<Self, DisplayError> {
        let earth = Earth::new(gl, assets_root)?;
        let text = TextRenderer::new(gl, &assets_root.join(FONT_FILE))?;
        Ok(Self {
            earth,
            text,
            state,
        })
    }
}

impl crate::formats::Format for HelloWorld {
    /// Render into the region (physical pixels, top-left origin).
    /// `framebuffer_height` is the whole framebuffer height.
    fn render(
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

        let view = self.state.borrow().clone();
        let bg = if view.is_some() {
            BACKGROUND
        } else {
            NO_DATA_BACKGROUND
        };
        unsafe {
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(x, y_gl, width, height);
            gl.clear_color(bg[0], bg[1], bg[2], bg[3]);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.disable(glow::SCISSOR_TEST);

            gl.viewport(x, y_gl, width, height);
            gl.disable(glow::DEPTH_TEST);
        }

        let Some(View {
            greeting,
            attitude,
        }) = view
        else {
            return; // black until the first sample arrives
        };

        self.earth.render(gl, width, height, attitude, LIGHT_DIR);
        self.text.draw(
            gl,
            width,
            height,
            &greeting,
            TEXT_COLOR,
            height as f32 * FONT_SIZE_FRACTION,
        );
    }
}
