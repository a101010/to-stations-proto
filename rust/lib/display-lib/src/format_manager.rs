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
use crate::formats::{self, Format, FormatCommand, FormatInput, FORMAT_CHOOSER};
use crate::ui::{self, Rect};
use crate::DisplayError;

/// The active formats by format-window id.
pub(crate) struct FormatManager {
    formats: Vec<Box<dyn Format>>,
    menu: Option<FormatMenu>,
    assignments: BTreeMap<i32, String>,
    /// The last region `render` was given, as `(x, y, width, height)` in
    /// physical pixels with a top-left origin.
    region: (i32, i32, i32, i32),
    /// The virtual cursor position, if it has been moved at least once.
    cursor: Option<(i32, i32)>,
}

/// Cursor bar color.
const CURSOR_COLOR: [f32; 4] = [0.90, 1.0, 0.92, 1.0];
/// Cursor shadow color.
const CURSOR_SHADOW: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
/// Distance the cursor center stays from the region edge.
const CURSOR_MARGIN: i32 = 8;
/// Half-length of a cursor bar.
const CURSOR_HALF: i32 = 7;

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
            cursor: None,
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
        self.render_cursor(gl, framebuffer_height);
    }

    /// Handle a high-level controller command for `window`.
    pub(crate) fn command(&mut self, window: i32, command: FormatCommand) -> Option<FormatAction> {
        if window != 0 {
            return None;
        }
        if let FormatCommand::CursorMove { dx, dy } = command {
            self.move_cursor(dx, dy);
        }

        let active = self.active_format_name(0);
        if active.as_deref() != Some(FORMAT_CHOOSER) {
            if let Some(menu) = &mut self.menu {
                let cursor = self.cursor.unwrap_or((
                    self.region.0 + self.region.2 / 2,
                    self.region.1 + self.region.3 / 2,
                ));
                match menu.command(0, self.region, cursor, command) {
                    MenuAction::Consumed => return None,
                    MenuAction::Select(format) => {
                        return Some(FormatAction::Select { window: 0, format })
                    }
                    MenuAction::Ignored => {}
                }
            }
        }

        let name = active?;
        let format = self
            .formats
            .iter_mut()
            .find(|format| format.name() == name.as_str())?;
        match format.command(command) {
            FormatInput::Select(format) => Some(FormatAction::Select { window: 0, format }),
            FormatInput::Ignored | FormatInput::Consumed => None,
        }
    }

    fn move_cursor(&mut self, dx: i32, dy: i32) {
        let (x, y, width, height) = self.region;
        if width <= 0 || height <= 0 {
            return;
        }
        let (current_x, current_y) = self.cursor.unwrap_or((x + width / 2, y + height / 2));
        let left = x + CURSOR_MARGIN.min(width / 2);
        let right = x + width - CURSOR_MARGIN.min(width / 2);
        let top = y + CURSOR_MARGIN.min(height / 2);
        let bottom = y + height - CURSOR_MARGIN.min(height / 2);
        self.cursor = Some((
            (current_x + dx).clamp(left, right.max(left)),
            (current_y + dy).clamp(top, bottom.max(top)),
        ));
    }

    fn render_cursor(&self, gl: &glow::Context, framebuffer_height: i32) {
        let Some((x, y)) = self.cursor else {
            return;
        };
        let shadow_horizontal = Rect::new(x - CURSOR_HALF - 1, y - 2, CURSOR_HALF * 2 + 3, 4);
        let shadow_vertical = Rect::new(x - 2, y - CURSOR_HALF - 1, 4, CURSOR_HALF * 2 + 3);
        ui::clear_rect(gl, framebuffer_height, shadow_horizontal, CURSOR_SHADOW);
        ui::clear_rect(gl, framebuffer_height, shadow_vertical, CURSOR_SHADOW);
        let horizontal = Rect::new(x - CURSOR_HALF, y - 1, CURSOR_HALF * 2 + 1, 2);
        let vertical = Rect::new(x - 1, y - CURSOR_HALF, 2, CURSOR_HALF * 2 + 1);
        ui::clear_rect(gl, framebuffer_height, horizontal, CURSOR_COLOR);
        ui::clear_rect(gl, framebuffer_height, vertical, CURSOR_COLOR);
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
            cursor: None,
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

        fn command(&mut self, _command: FormatCommand) -> FormatInput {
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
    #[test]
    fn commands_route_to_the_active_format() {
        let mut m = manager(FormatInput::Select("default".to_string()));
        assert_eq!(
            m.command(0, FormatCommand::Navigate { dx: 1, dy: 0 }),
            Some(FormatAction::Select {
                window: 0,
                format: "default".to_string(),
            })
        );
    }

    #[test]
    fn commands_for_other_windows_are_ignored() {
        let mut m = manager(FormatInput::Select("default".to_string()));
        assert_eq!(m.command(1, FormatCommand::Navigate { dx: 1, dy: 0 }), None);
    }

    #[test]
    fn cursor_starts_at_center_and_clamps() {
        let mut m = manager(FormatInput::Ignored);
        m.command(0, FormatCommand::CursorMove { dx: 0, dy: 0 });
        assert_eq!(m.cursor, Some((50, 50)));
        m.command(0, FormatCommand::CursorMove { dx: 1000, dy: 1000 });
        assert_eq!(m.cursor, Some((92, 92)));
        m.command(
            0,
            FormatCommand::CursorMove {
                dx: -1000,
                dy: -1000,
            },
        );
        assert_eq!(m.cursor, Some((8, 8)));
    }
}
