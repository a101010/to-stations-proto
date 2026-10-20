//! The hello-world format: a dark background with the earth and the greeting
//! text.
//!
//! The attitude, light, and greeting are hardcoded for now; they come from the
//! view service in `hello-view-service`.

use std::path::Path;

use glow::HasContext as _;

use crate::earth::{self, Earth};
use crate::text::TextRenderer;
use crate::DisplayError;

/// Background of the hello-world format.
pub(crate) const BACKGROUND: [f32; 4] = [0.02, 0.03, 0.08, 1.0];
/// Greeting color.
pub(crate) const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
/// The hardcoded greeting.
pub(crate) const GREETING: &str = "Hello, world!";
/// Latitude of the point the camera faces (North America).
const VIEW_LATITUDE_DEG: f32 = 45.0;
/// Longitude of the point the camera faces (North America).
const VIEW_LONGITUDE_DEG: f32 = -100.0;
/// Light direction (view space): upper right, toward the viewer.
const LIGHT_DIR: [f32; 3] = [0.6, 0.8, 0.7];
/// Font, relative to the assets root.
const FONT_FILE: &str = "fonts/FiraSans-Regular.ttf";
/// Text size as a fraction of the region height.
const FONT_SIZE_FRACTION: f32 = 0.15;

/// The attitude that faces `(VIEW_LATITUDE_DEG, VIEW_LONGITUDE_DEG)` toward the
/// camera with the north pole up.
///
/// `R = Rx(lat) * Ry(-lon)`; the camera looks down `-Z`, so the surface point
/// at `R^T * +Z` is the one facing the viewer. Its latitude is `lat` and its
/// longitude is `lon`, and the north pole (`R * +Y`) tilts into the upper,
/// visible hemisphere.
fn view_attitude() -> [f32; 4] {
    let lat = VIEW_LATITUDE_DEG.to_radians();
    let lon = VIEW_LONGITUDE_DEG.to_radians();
    earth::quat_mul(
        earth::quat_from_axis_angle([1.0, 0.0, 0.0], lat),
        earth::quat_from_axis_angle([0.0, 1.0, 0.0], -lon),
    )
}

/// The hello-world format.
pub(crate) struct HelloWorld {
    earth: Earth,
    text: TextRenderer,
    attitude: [f32; 4],
}

impl HelloWorld {
    /// Build the format, loading its assets from `assets_root`.
    pub(crate) fn new(gl: &glow::Context, assets_root: &Path) -> Result<Self, DisplayError> {
        let earth = Earth::new(gl, assets_root)?;
        let text = TextRenderer::new(gl, &assets_root.join(FONT_FILE))?;
        Ok(Self {
            earth,
            text,
            attitude: view_attitude(),
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

        self.earth
            .render(gl, width, height, self.attitude, LIGHT_DIR);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_faces_north_america_with_north_up() {
        let m = earth::quat_to_mat3(view_attitude());
        // The camera looks down -Z, so the surface point facing it is R^T * +Z,
        // i.e. the third row of R (the third column of R^T) in column-major `m`.
        let dir = [m[2], m[5], m[8]];
        let lat = dir[1].asin().to_degrees();
        let lon = dir[0].atan2(dir[2]).to_degrees();
        assert!((lat - VIEW_LATITUDE_DEG).abs() < 0.1, "lat {lat}");
        assert!((lon - VIEW_LONGITUDE_DEG).abs() < 0.1, "lon {lon}");

        // The north pole (R * +Y, the second column) tilts toward the viewer.
        let pole = [m[3], m[4], m[5]];
        assert!(pole[2] > 0.0, "north pole not visible: {pole:?}");
    }
}
