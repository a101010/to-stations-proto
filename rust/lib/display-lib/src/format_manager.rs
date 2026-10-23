//! The format manager: a registry of formats and the active one.
//!
//! Its first form lives inside the display library. It renders the active
//! format into the region it is given, remembers that region, and switches to
//! the next format when a left-button press lands inside it (see
//! `architecture.md`, "Format windows and the format manager").

use std::path::Path;

use crate::formats::{self, Format};
use crate::DisplayError;

/// The left mouse button, per the display library ABI.
const BUTTON_LEFT: i32 = 0;

/// A registry of formats and the active format.
pub(crate) struct FormatManager {
    formats: Vec<Box<dyn Format>>,
    active: usize,
    /// The last region `render` was given, as `(x, y, width, height)` in
    /// physical pixels with a top-left origin.
    region: (i32, i32, i32, i32),
}

impl FormatManager {
    /// Build the registry and select the initial format.
    pub(crate) fn new(
        gl: &glow::Context,
        assets_root: &Path,
        state: crate::view::ViewState,
    ) -> Result<Self, DisplayError> {
        Ok(Self {
            formats: formats::registry(gl, assets_root, state)?,
            active: 0,
            region: (0, 0, 0, 0),
        })
    }

    /// Render the active format into the region (physical pixels, top-left
    /// origin). `framebuffer_height` is the whole framebuffer height.
    pub(crate) fn render(
        &mut self,
        gl: &glow::Context,
        framebuffer_height: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        self.region = (x, y, width, height);
        if let Some(format) = self.formats.get_mut(self.active) {
            format.render(gl, framebuffer_height, x, y, width, height);
        }
    }

    /// Handle a mouse button event. A left-button press inside the delegated
    /// region advances to the next format.
    pub(crate) fn mouse_button(&mut self, x: i32, y: i32, button: i32, pressed: bool) {
        if button != BUTTON_LEFT || !pressed {
            return;
        }
        if !self.formats.is_empty() && contains(self.region, x, y) {
            self.active = advance(self.active, self.formats.len());
        }
    }

    #[cfg(test)]
    fn with_region(formats: Vec<Box<dyn Format>>, region: (i32, i32, i32, i32)) -> Self {
        Self {
            formats,
            active: 0,
            region,
        }
    }

    #[cfg(test)]
    fn active(&self) -> usize {
        self.active
    }
}

/// Whether `(x, y)` lies inside `region` (half-open: the far edges are
/// exclusive).
fn contains(region: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    let (rx, ry, rw, rh) = region;
    x >= rx && x < rx + rw && y >= ry && y < ry + rh
}

/// The next format index, wrapping.
fn advance(active: usize, len: usize) -> usize {
    (active + 1) % len
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;

    impl Format for Fake {
        fn render(
            &mut self,
            _gl: &glow::Context,
            _framebuffer_height: i32,
            _x: i32,
            _y: i32,
            _width: i32,
            _height: i32,
        ) {
        }
    }

    fn fake_formats(n: usize) -> Vec<Box<dyn Format>> {
        (0..n).map(|_| Box::new(Fake) as Box<dyn Format>).collect()
    }

    #[test]
    fn contains_inside_and_boundaries() {
        let region = (10, 20, 100, 50);
        assert!(contains(region, 10, 20)); // top-left corner is inside
        assert!(contains(region, 109, 69)); // last inside pixel
        assert!(!contains(region, 110, 69)); // right edge is exclusive
        assert!(!contains(region, 109, 70)); // bottom edge is exclusive
        assert!(!contains(region, 9, 20));
        assert!(!contains(region, 10, 19));
    }

    #[test]
    fn advance_increments_and_wraps() {
        assert_eq!(advance(0, 2), 1);
        assert_eq!(advance(1, 2), 0);
        assert_eq!(advance(2, 3), 0);
    }

    #[test]
    fn left_press_inside_advances() {
        let mut m = FormatManager::with_region(fake_formats(2), (0, 0, 100, 100));
        m.mouse_button(50, 50, BUTTON_LEFT, true);
        assert_eq!(m.active(), 1);
        m.mouse_button(50, 50, BUTTON_LEFT, true);
        assert_eq!(m.active(), 0);
    }

    #[test]
    fn press_outside_does_not_advance() {
        let mut m = FormatManager::with_region(fake_formats(2), (0, 0, 100, 100));
        m.mouse_button(150, 50, BUTTON_LEFT, true);
        assert_eq!(m.active(), 0);
    }

    #[test]
    fn other_button_and_release_do_not_advance() {
        let mut m = FormatManager::with_region(fake_formats(2), (0, 0, 100, 100));
        m.mouse_button(50, 50, 1, true); // right press
        m.mouse_button(50, 50, BUTTON_LEFT, false); // left release
        assert_eq!(m.active(), 0);
    }
}
