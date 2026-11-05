//! The format manager: named format renderers and the active assignment per
//! format window.
//!
//! The display library owns the renderers and applies assignments received from
//! the format-manager view service. Local clicks are hit-tested by the display
//! library and published as logical events; they no longer switch formats
//! locally.

use std::collections::BTreeMap;
use std::path::Path;

use format_list::FormatList;
use stations_dds::format_manager::FormatWindowAssignment;

use crate::formats::{self, Format};
use crate::DisplayError;

/// The active formats by format-window id.
pub(crate) struct FormatManager {
    formats: Vec<Box<dyn Format>>,
    assignments: BTreeMap<i32, String>,
    /// The last region `render` was given, as `(x, y, width, height)` in
    /// physical pixels with a top-left origin.
    region: (i32, i32, i32, i32),
}

impl FormatManager {
    pub(crate) fn new(
        gl: &glow::Context,
        assets_root: &Path,
        state: crate::view::ViewState,
        formats: &FormatList,
    ) -> Result<Self, DisplayError> {
        Ok(Self {
            formats: formats::registry(gl, assets_root, state, formats)?,
            assignments: BTreeMap::new(),
            region: (0, 0, 0, 0),
        })
    }

    /// Render format window 0 into the region (physical pixels, top-left
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
        let name = self
            .assignments
            .get(&0)
            .map(String::as_str)
            .or_else(|| self.formats.first().map(|format| format.name()))
            .unwrap_or("");
        if let Some(format) = self.formats.iter_mut().find(|format| format.name() == name) {
            format.render(gl, framebuffer_height, x, y, width, height);
        }
    }

    /// Replace the current assignments with a view sample. Unknown format
    /// names are ignored; unassigned windows fall back to the first format.
    pub(crate) fn set_assignments(&mut self, assignments: &[FormatWindowAssignment]) {
        self.assignments.clear();
        for assignment in assignments {
            if self
                .formats
                .iter()
                .any(|format| format.name() == assignment.format)
            {
                self.assignments
                    .insert(assignment.window, assignment.format.clone());
            }
        }
    }

    /// The format window under `(x, y)`, if any. The small display has one
    /// format window, so this is always window 0.
    pub(crate) fn hit_window(&self, x: i32, y: i32) -> Option<i32> {
        contains(self.region, x, y).then_some(0)
    }

    #[cfg(test)]
    fn with_region(formats: Vec<Box<dyn Format>>, region: (i32, i32, i32, i32)) -> Self {
        Self {
            formats,
            assignments: BTreeMap::new(),
            region,
        }
    }

    #[cfg(test)]
    fn assigned(&self, window: i32) -> Option<&str> {
        self.assignments.get(&window).map(String::as_str)
    }
}

/// Whether `(x, y)` lies inside `region` (half-open: the far edges are
/// exclusive).
fn contains(region: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    let (rx, ry, rw, rh) = region;
    x >= rx && x < rx + rw && y >= ry && y < ry + rh
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(&'static str);

    impl Format for Fake {
        fn name(&self) -> &'static str {
            self.0
        }

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

    fn fake_formats() -> Vec<Box<dyn Format>> {
        vec![Box::new(Fake("hello-world")), Box::new(Fake("default"))]
    }

    fn assignment(window: i32, format: &str) -> FormatWindowAssignment {
        FormatWindowAssignment {
            window,
            format: format.to_string(),
        }
    }

    #[test]
    fn contains_inside_and_boundaries() {
        let region = (10, 20, 100, 50);
        assert!(contains(region, 10, 20));
        assert!(contains(region, 109, 69));
        assert!(!contains(region, 110, 69));
        assert!(!contains(region, 109, 70));
        assert!(!contains(region, 9, 20));
        assert!(!contains(region, 10, 19));
    }

    #[test]
    fn hit_window_uses_the_last_region() {
        let m = FormatManager::with_region(fake_formats(), (0, 0, 100, 100));
        assert_eq!(m.hit_window(50, 50), Some(0));
        assert_eq!(m.hit_window(150, 50), None);
    }

    #[test]
    fn assignments_apply_by_name_and_ignore_unknowns() {
        let mut m = FormatManager::with_region(fake_formats(), (0, 0, 100, 100));
        m.set_assignments(&[assignment(0, "default")]);
        assert_eq!(m.assigned(0), Some("default"));

        m.set_assignments(&[assignment(0, "missing")]);
        assert_eq!(m.assigned(0), None);
    }

    #[test]
    fn assignments_are_independent_per_window() {
        let mut m = FormatManager::with_region(fake_formats(), (0, 0, 100, 100));
        m.set_assignments(&[assignment(0, "default"), assignment(1, "hello-world")]);
        assert_eq!(m.assigned(0), Some("default"));
        assert_eq!(m.assigned(1), Some("hello-world"));
    }
}
