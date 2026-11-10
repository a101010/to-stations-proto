//! The three-dot format menu overlay.
//!
//! The menu button is drawn in the upper-left of a format window over every
//! format. Selecting a row publishes a logical `Select` event through the
//! display library's format source.

use std::collections::BTreeMap;
use std::path::Path;

use glow::HasContext as _;

use crate::text::{TextRenderer, FONT_FILE};
use crate::DisplayError;

const BUTTON_BG: [f32; 4] = [0.08, 0.08, 0.10, 1.0];
const PANEL_BG: [f32; 4] = [0.10, 0.10, 0.13, 1.0];
const ACTIVE_BG: [f32; 4] = [0.25, 0.25, 0.31, 1.0];
const TEXT_COLOR: [f32; 4] = [0.95, 0.95, 0.95, 1.0];
const DOTS: &str = "...";

/// A rectangle in physical pixels, top-left origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) w: i32,
    pub(crate) h: i32,
}

impl Rect {
    fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

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
    pub(crate) rows: Vec<Rect>,
    pub(crate) font_size: f32,
    pub(crate) padding: i32,
}

impl MenuLayout {
    pub(crate) fn new(region: (i32, i32, i32, i32), count: usize) -> Self {
        let (x, y, width, height) = region;
        let zero = Rect {
            x,
            y,
            w: 0,
            h: 0,
        };
        if width <= 0 || height <= 0 || count == 0 {
            return Self {
                button: zero,
                panel: zero,
                rows: vec![zero; count],
                font_size: 0.0,
                padding: 0,
            };
        }

        let scale = (height as f32 / 720.0).clamp(0.75, 2.0);
        let margin = (12.0 * scale).round() as i32;
        let button_size = (40.0 * scale).round() as i32;
        let row_height = (40.0 * scale).round() as i32;
        let padding = (8.0 * scale).round() as i32;
        let font_size = 20.0 * scale;

        let button = Rect {
            x: x + margin,
            y: y + margin,
            w: button_size.min((width - 2 * margin).max(1)),
            h: button_size.min((height - 2 * margin).max(1)),
        };
        let panel_x = button.x;
        let panel_y = button.y + button.h + margin / 2;
        let panel = Rect {
            x: panel_x,
            y: panel_y,
            w: ((220.0 * scale).round() as i32)
                .min((x + width - margin - panel_x).max(1)),
            h: (count as i32 * row_height + 2 * padding)
                .min((y + height - margin - panel_y).max(1)),
        };

        let row_width = (panel.w as i32 - 2 * padding).max(1);
        let mut rows = Vec::with_capacity(count);
        for index in 0..count {
            let row_y = panel.y + padding + index as i32 * row_height;
            let remaining = panel.y + panel.h - padding - row_y;
            rows.push(Rect {
                x: panel.x + padding,
                y: row_y,
                w: row_width,
                h: row_height.min(remaining).max(0),
            });
        }

        Self {
            button,
            panel,
            rows,
            font_size,
            padding,
        }
    }

    fn row_at(&self, x: i32, y: i32) -> Option<usize> {
        self.rows.iter().position(|row| row.contains(x, y))
    }
}

/// The open/closed state for one format window's menu.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MenuState {
    open: bool,
}

impl MenuState {
    fn click(&mut self, layout: &MenuLayout, names: &[String], x: i32, y: i32) -> MenuAction {
        if layout.button.contains(x, y) {
            self.open = !self.open;
            return MenuAction::Consumed;
        }

        if self.open {
            if let Some(index) = layout.row_at(x, y) {
                if let Some(name) = names.get(index) {
                    self.open = false;
                    return MenuAction::Select(name.clone());
                }
            }
            self.open = false;
            return MenuAction::Consumed;
        }

        MenuAction::Ignored
    }
}

/// The menu renderer and per-window state.
pub(crate) struct FormatMenu {
    names: Vec<String>,
    renderer: TextRenderer,
    states: BTreeMap<i32, MenuState>,
}

impl FormatMenu {
    pub(crate) fn new(
        gl: &glow::Context,
        assets_root: &Path,
        names: &[String],
    ) -> Result<Self, DisplayError> {
        let renderer = TextRenderer::new(gl, &assets_root.join(FONT_FILE))?;
        Ok(Self {
            names: names.to_vec(),
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
        let layout = MenuLayout::new(region, self.names.len());
        self.states
            .entry(window)
            .or_default()
            .click(&layout, &self.names, x, y)
    }

    /// Draw the button, and the open panel, over the active format.
    pub(crate) fn render(
        &mut self,
        gl: &glow::Context,
        framebuffer_height: i32,
        region: (i32, i32, i32, i32),
        window: i32,
        active_format: &str,
    ) {
        let layout = MenuLayout::new(region, self.names.len());
        if layout.button.w <= 0 || layout.button.h <= 0 {
            return;
        }

        clear_rect(gl, framebuffer_height, layout.button, BUTTON_BG);

        let open = self
            .states
            .get(&window)
            .map(|state| state.open)
            .unwrap_or(false);
        if open {
            clear_rect(gl, framebuffer_height, layout.panel, PANEL_BG);
            if let Some(index) = self.names.iter().position(|name| name.as_str() == active_format) {
                if let Some(row) = layout.rows.get(index) {
                    clear_rect(gl, framebuffer_height, *row, ACTIVE_BG);
                }
            }
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

        if open {
            let names = self.names.clone();
            for (index, name) in names.iter().enumerate() {
                let Some(row) = layout.rows.get(index) else {
                    break;
                };
                if row.h <= 0 {
                    continue;
                }
                let top = ((row.h as f32 - layout.font_size) / 2.0).max(0.0);
                self.draw_text(
                    gl,
                    region,
                    *row,
                    name,
                    layout.font_size,
                    layout.padding as f32,
                    top,
                );
            }
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

fn clear_rect(
    gl: &glow::Context,
    framebuffer_height: i32,
    rect: Rect,
    color: [f32; 4],
) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<String> {
        vec!["hello-world".to_string(), "default".to_string()]
    }

    fn layout() -> MenuLayout {
        MenuLayout::new((0, 0, 720, 720), names().len())
    }

    fn center(rect: Rect) -> (i32, i32) {
        (rect.x + rect.w / 2, rect.y + rect.h / 2)
    }

    #[test]
    fn layout_stays_inside_the_region() {
        let layout = layout();
        assert_eq!(layout.rows.len(), 2);
        for rect in [layout.button, layout.panel, layout.rows[0], layout.rows[1]] {
            assert!(rect.x >= 0 && rect.y >= 0);
            assert!(rect.x + rect.w <= 720);
            assert!(rect.y + rect.h <= 720);
        }
    }

    #[test]
    fn button_click_toggles_open_and_closed() {
        let mut state = MenuState::default();
        let layout = layout();
        let names = names();
        let (x, y) = center(layout.button);
        assert_eq!(
            state.click(&layout, &names, x, y),
            MenuAction::Consumed
        );
        assert!(state.open);
        assert_eq!(
            state.click(&layout, &names, x, y),
            MenuAction::Consumed
        );
        assert!(!state.open);
    }

    #[test]
    fn row_click_selects_and_closes() {
        let mut state = MenuState::default();
        let layout = layout();
        let names = names();
        let (x, y) = center(layout.button);
        state.click(&layout, &names, x, y);
        let (x, y) = center(layout.rows[1]);
        assert_eq!(
            state.click(&layout, &names, x, y),
            MenuAction::Select("default".to_string())
        );
        assert!(!state.open);
    }

    #[test]
    fn click_outside_open_menu_closes_it() {
        let mut state = MenuState::default();
        let layout = layout();
        let names = names();
        state.open = true;
        assert_eq!(
            state.click(&layout, &names, 700, 700),
            MenuAction::Consumed
        );
        assert!(!state.open);
    }

    #[test]
    fn closed_menu_outside_click_is_ignored() {
        let mut state = MenuState::default();
        assert_eq!(
            state.click(&layout(), &names(), 700, 700),
            MenuAction::Ignored
        );
    }
}
