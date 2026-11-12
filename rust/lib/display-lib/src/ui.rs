//! Shared 2D UI helpers.
//!
//! Geometry is in physical pixels with a top-left origin. The scissor-clear
//! and scale helpers are shared by the format menu and the format chooser.
//! Every function here assumes a glow context is current on the calling thread.

use glow::HasContext as _;

/// A rectangle in physical pixels, top-left origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: i32,
    pub(crate) h: i32,
}

impl Rect {
    pub(crate) const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    /// Whether `(x, y)` lies inside the rectangle (half-open: the far edges
    /// are exclusive).
    pub(crate) fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Clear `rect` to `color`, leaving the rest of the framebuffer untouched.
pub(crate) fn clear_rect(gl: &glow::Context, framebuffer_height: i32, rect: Rect, color: [f32; 4]) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    let y_gl = framebuffer_height - rect.y - rect.h;
    unsafe {
        gl.enable(glow::SCISSOR_TEST);
        gl.scissor(rect.x, y_gl, rect.w, rect.h);
        gl.clear_color(color[0], color[1], color[2], color[3]);
        gl.clear(glow::COLOR_BUFFER_BIT);
        gl.disable(glow::SCISSOR_TEST);
    }
}

/// The UI scale for a framebuffer region, using a 720 px reference height.
pub(crate) fn scale_for_height(height: i32) -> f32 {
    (height as f32 / 720.0).clamp(0.75, 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_inside_and_boundaries() {
        let rect = Rect::new(10, 20, 100, 50);
        assert!(rect.contains(10, 20));
        assert!(rect.contains(109, 69));
        assert!(!rect.contains(110, 69));
        assert!(!rect.contains(109, 70));
        assert!(!rect.contains(9, 20));
        assert!(!rect.contains(10, 19));
    }

    #[test]
    fn empty_rect_contains_nothing() {
        let rect = Rect::new(10, 20, 0, 0);
        assert!(!rect.contains(10, 20));
    }

    #[test]
    fn scale_is_clamped() {
        assert!((scale_for_height(720) - 1.0).abs() < 1e-6);
        assert!((scale_for_height(100) - 0.75).abs() < 1e-6);
        assert!((scale_for_height(4000) - 2.0).abs() < 1e-6);
    }
}
