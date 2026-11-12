//! The format manager: named format renderers and the active assignment per
//! format window.
//!
//! The display library owns the renderers and applies assignments received from
//! the format-manager view service. Local clicks are hit-tested by the display
//! library; the active format may consume them or request a selection, which is
//! published as a logical `Select` event. The manager does not generate `Next`.

use std::collections::BTreeMap;
use std::path::Path;

use format_list::FormatList;
use stations_dds::format_manager::FormatWindowAssignment;

use crate::format_menu::{FormatMenu, MenuAction};
use crate::formats::{self, Format, FormatInput, FORMAT_CHOOSER};
use crate::DisplayError;

/// The active formats by format-window id.
pub(crate) struct FormatManager {
    formats: Vec<Box<dyn Format>>,
    menu: Option<FormatMenu>,
    assignments: BTreeMap<i32, String>,
    /// The last region `render` was given, as `(x, y, width, height)` in
    /// physical pixels with a top-left origin.
    region: (i32, i32, i32, i32),
}

/// A logical format-manager event produced by local input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FormatAction {
    Select { window: i32, format: String },
}

impl FormatManager {
    pub(crate) fn new(
        gl: &glow::Context,
        assets_root: &Path,
        state: crate::view::ViewState,
        formats: &FormatList,
    ) -> Result<Self, DisplayError> {
        let renderers = formats::registry(gl, assets_root, state, formats)?;
        let menu = FormatMenu::new(gl, assets_root)?;
        Ok(Self {
            formats: renderers,
            menu: Some(menu),
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
        let name = self.active_format_name(0);
        if let Some(name) = &name {
            if let Some(format) = self
                .formats
                .iter_mut()
                .find(|format| format.name() == name.as_str())
            {
                format.render(gl, framebuffer_height, x, y, width, height);
            }
        }
        if let Some(menu) = &mut self.menu {
            if name.as_deref() == Some(FORMAT_CHOOSER) {
                menu.close(0);
            } else {
                menu.render(gl, framebuffer_height, self.region, 0);
            }
        }
    }

    /// Handle a left-button press in window coordinates.
    pub(crate) fn mouse_button(&mut self, x: i32, y: i32) -> Option<FormatAction> {
        let active = self.active_format_name(0);

        if active.as_deref() != Some(FORMAT_CHOOSER) {
            if let Some(menu) = &mut self.menu {
                match menu.click(0, self.region, x, y) {
                    MenuAction::Consumed => return None,
                    MenuAction::Select(format) => {
                        return Some(FormatAction::Select { window: 0, format })
                    }
                    MenuAction::Ignored => {}
                }
            }
        }

        if !contains(self.region, x, y) {
            return None;
        }
        let name = active?;
        let format = self
            .formats
            .iter_mut()
            .find(|format| format.name() == name.as_str())?;
        match format.mouse_button(self.region, x, y) {
            FormatInput::Ignored | FormatInput::Consumed => None,
            FormatInput::Select(format) => Some(FormatAction::Select { window: 0, format }),
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

    /// The active format name for `window`, or the registry's first format when
    /// the window is unassigned.
    fn active_format_name(&self, window: i32) -> Option<String> {
        self.assignments
            .get(&window)
            .cloned()
            .or_else(|| self.formats.first().map(|format| format.name().to_string()))
    }

    #[cfg(test)]
    fn with_region(formats: Vec<Box<dyn Format>>, region: (i32, i32, i32, i32)) -> Self {
        Self {
            formats,
            menu: None,
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

    struct Fake {
        name: &'static str,
        input: FormatInput,
    }

    impl Format for Fake {
        fn name(&self) -> &'static str {
            self.name
        }

        fn title(&self) -> &'static str {
            self.name
        }

        fn icon_file(&self) -> &'static str {
            ""
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

        fn mouse_button(&mut self, _region: (i32, i32, i32, i32), _x: i32, _y: i32) -> FormatInput {
            self.input.clone()
        }
    }

    fn fake(name: &'static str, input: FormatInput) -> Box<dyn Format> {
        Box::new(Fake { name, input })
    }

    fn manager(input: FormatInput) -> FormatManager {
        FormatManager::with_region(
            vec![
                fake("hello-world", input),
                fake("default", FormatInput::Ignored),
            ],
            (0, 0, 100, 100),
        )
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
    fn ignored_active_input_produces_no_action() {
        let mut m = manager(FormatInput::Ignored);
        assert_eq!(m.mouse_button(50, 50), None);
    }

    #[test]
    fn consumed_active_input_produces_no_action() {
        let mut m = manager(FormatInput::Consumed);
        assert_eq!(m.mouse_button(50, 50), None);
    }

    #[test]
    fn selected_active_input_produces_a_select_action() {
        let mut m = manager(FormatInput::Select("default".to_string()));
        assert_eq!(
            m.mouse_button(50, 50),
            Some(FormatAction::Select {
                window: 0,
                format: "default".to_string(),
            })
        );
    }

    #[test]
    fn click_outside_region_produces_no_action() {
        let mut m = manager(FormatInput::Select("default".to_string()));
        assert_eq!(m.mouse_button(150, 50), None);
    }

    #[test]
    fn assignments_apply_by_name_and_ignore_unknowns() {
        let mut m = manager(FormatInput::Ignored);
        m.set_assignments(&[assignment(0, "default")]);
        assert_eq!(m.assigned(0), Some("default"));

        m.set_assignments(&[assignment(0, "missing")]);
        assert_eq!(m.assigned(0), None);
    }

    #[test]
    fn assignments_are_independent_per_window() {
        let mut m = manager(FormatInput::Ignored);
        m.set_assignments(&[assignment(0, "default"), assignment(1, "hello-world")]);
        assert_eq!(m.assigned(0), Some("default"));
        assert_eq!(m.assigned(1), Some("hello-world"));
    }
}
