//! The three-dot format menu overlay.
//!
//! The menu button is drawn in the upper-left of a format window over every
//! format except the chooser. Its single `Formats` row publishes a logical
//! `Select` event for the chooser through the display library's format source.

use std::collections::BTreeMap;
use std::path::Path;

use glow::HasContext as _;

use crate::formats::FORMAT_CHOOSER;
use crate::text::{TextRenderer, FONT_FILE};
use crate::ui::{self, Rect};
use crate::DisplayError;

const BUTTON_BG: [f32; 4] = [0.08, 0.08, 0.10, 1.0];
const PANEL_BG: [f32; 4] = [0.10, 0.10, 0.13, 1.0];
const TEXT_COLOR: [f32; 4] = [0.95, 0.95, 0.95, 1.0];
const DOTS: &str = "...";
const LABEL: &str = "Formats";

/// What a left-button click did in menu terms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MenuAction {
    Ignored,
    Consumed,
    Select(String),
}

/// The menu's geometry for the last format region.
#[derive(Clone, Debug)]
pub(crate) struct MenuLayout {
    pub(crate) button: Rect,
    pub(crate) panel: Rect,
    pub(crate) row: Rect,
    pub(crate) font_size: f32,
    pub(crate) padding: i32,
}

impl MenuLayout {
    pub(crate) fn new(region: (i32, i32, i32, i32)) -> Self {
        let (x, y, width, height) = region;
        let zero = Rect::new(x, y, 0, 0);
        if width <= 0 || height <= 0 {
            return Self {
                button: zero,
                panel: zero,
                row: zero,
                font_size: 0.0,
                padding: 0,
            };
        }

        let scale = ui::scale_for_height(height);
        let margin = (12.0 * scale).round() as i32;
        let button_size = (40.0 * scale).round() as i32;
        let row_height = (40.0 * scale).round() as i32;
        let padding = (8.0 * scale).round() as i32;
        let font_size = 20.0 * scale;

        let button = Rect::new(
            x + margin,
            y + margin,
            button_size.min((width - 2 * margin).max(1)),
            button_size.min((height - 2 * margin).max(1)),
        );
        let panel_x = button.x;
        let panel_y = button.y + button.h + margin / 2;
        let panel = Rect::new(
            panel_x,
            panel_y,
            ((220.0 * scale).round() as i32).min((x + width - margin - panel_x).max(1)),
            (row_height + 2 * padding).min((y + height - margin - panel_y).max(1)),
        );

        let row_width = (panel.w - 2 * padding).max(1);
        let row_y = panel.y + padding;
        let remaining = panel.y + panel.h - padding - row_y;
        let row = Rect::new(
            panel.x + padding,
            row_y,
            row_width,
            row_height.min(remaining).max(0),
        );

        Self {
            button,
            panel,
            row,
            font_size,
            padding,
        }
    }
}

/// The open/closed state for one format window's menu.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MenuState {
    open: bool,
}

impl MenuState {
    fn click(&mut self, layout: &MenuLayout, x: i32, y: i32) -> MenuAction {
        if layout.button.contains(x, y) {
            self.open = !self.open;
            return MenuAction::Consumed;
        }

        if self.open {
            self.open = false;
            if layout.row.contains(x, y) {
                return MenuAction::Select(FORMAT_CHOOSER.to_string());
            }
            return MenuAction::Consumed;
        }

        MenuAction::Ignored
    }
}

/// The menu renderer and per-window state.
pub(crate) struct FormatMenu {
    renderer: TextRenderer,
    states: BTreeMap<i32, MenuState>,
}

impl FormatMenu {
    pub(crate) fn new(gl: &glow::Context, assets_root: &Path) -> Result<Self, DisplayError> {
        let renderer = TextRenderer::new(gl, &assets_root.join(FONT_FILE))?;
        Ok(Self {
            renderer,
            states: BTreeMap::new(),
        })
    }

    /// Handle a left-button press.
    pub(crate) fn click(
        &mut self,
        window: i32,
        region: (i32, i32, i32, i32),
        x: i32,
        y: i32,
    ) -> MenuAction {
        let layout = MenuLayout::new(region);
        self.states.entry(window).or_default().click(&layout, x, y)
    }

    /// Close the menu, for example when the chooser becomes active.
    pub(crate) fn close(&mut self, window: i32) {
        self.states.entry(window).or_default().open = false;
    }

    /// Draw the button, and the open panel, over the active format.
    pub(crate) fn render(
        &mut self,
        gl: &glow::Context,
        framebuffer_height: i32,
        region: (i32, i32, i32, i32),
        window: i32,
    ) {
        let layout = MenuLayout::new(region);
        if layout.button.w <= 0 || layout.button.h <= 0 {
            return;
        }

        ui::clear_rect(gl, framebuffer_height, layout.button, BUTTON_BG);

        let open = self
            .states
            .get(&window)
            .map(|state| state.open)
            .unwrap_or(false);
        if open {
            ui::clear_rect(gl, framebuffer_height, layout.panel, PANEL_BG);
        }

        let y_gl = framebuffer_height - region.1 - region.3;
        unsafe {
            gl.viewport(region.0, y_gl, region.2, region.3);
            gl.disable(glow::DEPTH_TEST);
        }

        let dot_size = (layout.button.h as f32 * 0.55).max(10.0);
        let dots_width = self.renderer.text_width(DOTS, dot_size);
        self.draw_text(
            gl,
            region,
            layout.button,
            DOTS,
            dot_size,
            ((layout.button.w as f32 - dots_width) / 2.0).max(0.0),
            ((layout.button.h as f32 - dot_size) / 2.0).max(0.0),
        );

        if open && layout.row.h > 0 {
            let top = ((layout.row.h as f32 - layout.font_size) / 2.0).max(0.0);
            self.draw_text(
                gl,
                region,
                layout.row,
                LABEL,
                layout.font_size,
                layout.padding as f32,
                top,
            );
        }
    }

    fn draw_text(
        &mut self,
        gl: &glow::Context,
        region: (i32, i32, i32, i32),
        rect: Rect,
        text: &str,
        font_size: f32,
        left: f32,
        top: f32,
    ) {
        self.renderer.draw_at(
            gl,
            region.2,
            region.3,
            text,
            TEXT_COLOR,
            font_size,
            (rect.x - region.0) as f32 + left,
            (rect.y - region.1) as f32 + top,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> MenuLayout {
        MenuLayout::new((0, 0, 720, 720))
    }

    fn center(rect: Rect) -> (i32, i32) {
        (rect.x + rect.w / 2, rect.y + rect.h / 2)
    }

    #[test]
    fn layout_stays_inside_the_region() {
        let layout = layout();
        for rect in [layout.button, layout.panel, layout.row] {
            assert!(rect.x >= 0 && rect.y >= 0);
            assert!(rect.x + rect.w <= 720);
            assert!(rect.y + rect.h <= 720);
        }
    }

    #[test]
    fn button_click_toggles_open_and_closed() {
        let mut state = MenuState::default();
        let layout = layout();
        let (x, y) = center(layout.button);
        assert_eq!(state.click(&layout, x, y), MenuAction::Consumed);
        assert!(state.open);
        assert_eq!(state.click(&layout, x, y), MenuAction::Consumed);
        assert!(!state.open);
    }

    #[test]
    fn row_click_selects_the_chooser_and_closes() {
        let mut state = MenuState::default();
        let layout = layout();
        let (x, y) = center(layout.button);
        state.click(&layout, x, y);
        let (x, y) = center(layout.row);
        assert_eq!(
            state.click(&layout, x, y),
            MenuAction::Select(FORMAT_CHOOSER.to_string())
        );
        assert!(!state.open);
    }

    #[test]
    fn click_outside_open_menu_closes_it() {
        let mut state = MenuState::default();
        let layout = layout();
        state.open = true;
        assert_eq!(state.click(&layout, 700, 700), MenuAction::Consumed);
        assert!(!state.open);
    }

    #[test]
    fn closed_menu_outside_click_is_ignored() {
        let mut state = MenuState::default();
        assert_eq!(state.click(&layout(), 700, 700), MenuAction::Ignored);
    }
}
