//! The three-dot format menu overlay.
//!
//! The button and the open menu occupy the same top band at the format
//! window's left edge. The menu is a single horizontal row of equal,
//! fixed-width slots; it has no wrap or shrink-to-fit fallback.

use std::collections::BTreeMap;
use std::path::Path;

use glow::HasContext as _;

use format_list::FORMAT_POINTER_RIGHT;

use crate::formats::{FormatCommand, MenuItem, FORMAT_CHOOSER};
use crate::text::{TextRenderer, FONT_FILE};
use crate::ui::{self, Rect};
use crate::DisplayError;

const BUTTON_BG: [f32; 4] = [0.08, 0.08, 0.10, 1.0];
const PANEL_BG: [f32; 4] = [0.10, 0.10, 0.13, 1.0];
const SELECTED_ITEM_BG: [f32; 4] = [0.20, 0.32, 0.24, 1.0];
const TEXT_COLOR: [f32; 4] = [0.95, 0.95, 0.95, 1.0];
const DOTS: &str = "...";
/// Height of the top band at the 720 px reference size.
const BAND_HEIGHT: f32 = 56.0;
/// Menu label size at the 720 px reference size.
const FONT_SIZE: f32 = 20.0;
/// Left/right inset inside each fixed slot.
const SLOT_INSET: f32 = 14.0;
/// Gap between equal slots.
const SLOT_GAP: f32 = 2.0;

/// What menu input did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MenuAction {
    Ignored,
    Consumed,
    Select(String),
    Item(String),
}

/// The menu's geometry for the last format region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MenuLayout {
    pub(crate) button: Rect,
    pub(crate) panel: Rect,
    pub(crate) items: Vec<Rect>,
    pub(crate) font_size: i32,
    pub(crate) inset: i32,
}

impl MenuLayout {
    pub(crate) fn new(
        region: (i32, i32, i32, i32),
        count: usize,
        slot_width: i32,
        font_size: f32,
        inset: i32,
        gap: i32,
    ) -> Self {
        let (x, y, width, height) = region;
        let zero = Rect::new(x, y, 0, 0);
        if width <= 0 || height <= 0 {
            return Self {
                button: zero,
                panel: zero,
                items: Vec::new(),
                font_size: 0,
                inset: 0,
            };
        }

        let scale = ui::scale_for_height(height);
        let band = ((BAND_HEIGHT * scale).round() as i32).clamp(1, width.min(height).max(1));
        let button = Rect::new(x, y, band, band);
        let panel_width = if count == 0 {
            0
        } else {
            count as i32 * slot_width + (count as i32 - 1) * gap
        };
        let panel = Rect::new(x, y, panel_width, band);

        let items = (0..count)
            .map(|index| Rect::new(x + index as i32 * (slot_width + gap), y, slot_width, band))
            .collect();

        Self {
            button,
            panel,
            items,
            font_size: font_size.round() as i32,
            inset,
        }
    }

    fn item_at(&self, x: i32, y: i32) -> Option<usize> {
        self.items.iter().position(|rect| rect.contains(x, y))
    }
}

/// The open/closed state for one format window's menu.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MenuState {
    open: bool,
    selected: usize,
}

impl MenuState {
    fn activate(&mut self, items: &[MenuItem]) -> MenuAction {
        self.open = false;
        let Some(item) = items.get(self.selected) else {
            return MenuAction::Consumed;
        };
        let action = if item.id == FORMAT_CHOOSER {
            MenuAction::Select(item.id.clone())
        } else {
            MenuAction::Item(item.id.clone())
        };
        self.selected = 0;
        action
    }

    fn toggle(&mut self) {
        self.open = !self.open;
        self.selected = 0;
    }

    fn cancel(&mut self) -> bool {
        let was_open = self.open;
        self.open = false;
        self.selected = 0;
        was_open
    }

    fn click(&mut self, layout: &MenuLayout, items: &[MenuItem], x: i32, y: i32) -> MenuAction {
        if self.open {
            if let Some(index) = layout.item_at(x, y) {
                self.selected = index;
                return self.activate(items);
            }
            self.open = false;
            self.selected = 0;
            return MenuAction::Consumed;
        }

        if layout.button.contains(x, y) {
            self.open = true;
            self.selected = 0;
            return MenuAction::Consumed;
        }

        MenuAction::Ignored
    }

    fn command(
        &mut self,
        layout: &MenuLayout,
        items: &[MenuItem],
        cursor: (i32, i32),
        command: FormatCommand,
    ) -> MenuAction {
        match command {
            FormatCommand::Pointer {
                button,
                pressed: true,
            } if button == FORMAT_POINTER_RIGHT => {
                if self.open {
                    return self.activate(items);
                }
                if layout.button.contains(cursor.0, cursor.1) {
                    self.open = true;
                    self.selected = 0;
                    return MenuAction::Consumed;
                }
                MenuAction::Ignored
            }
            FormatCommand::Navigate { dx, .. } if self.open => {
                if dx != 0 && !items.is_empty() {
                    let direction = dx.signum();
                    let count = items.len() as i32;
                    self.selected = (self.selected as i32 + direction).rem_euclid(count) as usize;
                }
                MenuAction::Consumed
            }
            _ => MenuAction::Ignored,
        }
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

    /// The fixed-slot layout for `items`. The slot width comes from the widest
    /// planned label; there is deliberately no wrap or shrink fallback.
    fn layout(&self, region: (i32, i32, i32, i32), items: &[MenuItem]) -> MenuLayout {
        let scale = ui::scale_for_height(region.3);
        let font_size = FONT_SIZE * scale;
        let inset = (SLOT_INSET * scale).round() as i32;
        let gap = (SLOT_GAP * scale).round() as i32;
        let slot_width = items
            .iter()
            .map(|item| self.renderer.text_width(&item.label, font_size).ceil() as i32)
            .max()
            .unwrap_or(0)
            + 2 * inset;
        MenuLayout::new(region, items.len(), slot_width, font_size, inset, gap)
    }

    /// Handle a mouse press.
    pub(crate) fn click(
        &mut self,
        window: i32,
        region: (i32, i32, i32, i32),
        items: &[MenuItem],
        x: i32,
        y: i32,
    ) -> MenuAction {
        let layout = self.layout(region, items);
        self.states
            .entry(window)
            .or_default()
            .click(&layout, items, x, y)
    }

    /// Handle a high-level controller command aimed at the menu.
    pub(crate) fn command(
        &mut self,
        window: i32,
        region: (i32, i32, i32, i32),
        items: &[MenuItem],
        cursor: (i32, i32),
        command: FormatCommand,
    ) -> MenuAction {
        let layout = self.layout(region, items);
        self.states
            .entry(window)
            .or_default()
            .command(&layout, items, cursor, command)
    }

    /// Toggle the menu, used by the Xbox Start button.
    pub(crate) fn toggle(&mut self, window: i32) {
        self.states.entry(window).or_default().toggle();
    }

    /// Cancel the menu, used by the Xbox B button. Returns whether it was open.
    pub(crate) fn cancel(&mut self, window: i32) -> bool {
        self.states.entry(window).or_default().cancel()
    }

    /// Close the menu, for example when the chooser becomes active.
    pub(crate) fn close(&mut self, window: i32) {
        let state = self.states.entry(window).or_default();
        state.open = false;
        state.selected = 0;
    }

    /// Draw the closed button or the open item bar over the active format.
    pub(crate) fn render(
        &mut self,
        gl: &glow::Context,
        framebuffer_height: i32,
        region: (i32, i32, i32, i32),
        window: i32,
        items: &[MenuItem],
    ) {
        let layout = self.layout(region, items);
        if layout.button.w <= 0 || layout.button.h <= 0 {
            return;
        }

        let state = self.states.get(&window).copied().unwrap_or_default();
        if state.open {
            ui::clear_rect(gl, framebuffer_height, layout.panel, PANEL_BG);
            if let Some(rect) = layout.items.get(state.selected) {
                ui::clear_rect(gl, framebuffer_height, *rect, SELECTED_ITEM_BG);
            }
        } else {
            ui::clear_rect(gl, framebuffer_height, layout.button, BUTTON_BG);
        }

        let y_gl = framebuffer_height - region.1 - region.3;
        unsafe {
            gl.viewport(region.0, y_gl, region.2, region.3);
            gl.disable(glow::DEPTH_TEST);
        }

        if state.open {
            for (index, item) in items.iter().enumerate() {
                let Some(rect) = layout.items.get(index) else {
                    continue;
                };
                let font_size = layout.font_size as f32;
                let text_width = self.renderer.text_width(&item.label, font_size);
                let left = ((rect.w as f32 - text_width) / 2.0).max(0.0);
                let top = ((rect.h as f32 - font_size) / 2.0).max(0.0);
                self.draw_text(gl, region, *rect, &item.label, font_size, left, top);
            }
        } else {
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

    fn items() -> Vec<MenuItem> {
        vec![
            MenuItem {
                id: FORMAT_CHOOSER.to_string(),
                label: "Formats".to_string(),
            },
            MenuItem {
                id: "mode".to_string(),
                label: "Inspect".to_string(),
            },
            MenuItem {
                id: "lux-color".to_string(),
                label: "Lux Color".to_string(),
            },
            MenuItem {
                id: "lux-pos".to_string(),
                label: "Lux Pos".to_string(),
            },
        ]
    }

    fn layout() -> MenuLayout {
        MenuLayout::new((0, 0, 720, 720), 4, 120, 20.0, 14, 2)
    }

    fn center(rect: Rect) -> (i32, i32) {
        (rect.x + rect.w / 2, rect.y + rect.h / 2)
    }

    #[test]
    fn layout_is_left_aligned_and_uses_equal_fixed_slots() {
        let layout = layout();
        assert_eq!(layout.button.x, 0);
        assert_eq!(layout.button.y, 0);
        assert_eq!(layout.panel.x, 0);
        assert_eq!(layout.panel.y, 0);
        assert_eq!(layout.items.len(), 4);
        assert_eq!(layout.items[0].x, 0);
        assert!(layout.items.iter().all(|item| item.w == 120));
        assert!(layout.items.iter().all(|item| item.y == 0));
        assert!(layout.panel.w <= 720);
        for item in &layout.items {
            assert!(item.x + item.w <= 720);
        }
    }

    #[test]
    fn open_menu_replaces_the_button_area_with_the_first_item() {
        let mut state = MenuState::default();
        let layout = layout();
        assert_eq!(state.click(&layout, &items(), 10, 10), MenuAction::Consumed);
        assert!(state.open);
        assert_eq!(
            state.click(&layout, &items(), 10, 10),
            MenuAction::Select(FORMAT_CHOOSER.to_string())
        );
        assert!(!state.open);
    }

    #[test]
    fn start_style_toggle_opens_and_closes_the_menu() {
        let mut state = MenuState::default();
        state.toggle();
        assert!(state.open);
        state.toggle();
        assert!(!state.open);
    }

    #[test]
    fn cancel_closes_only_an_open_menu() {
        let mut state = MenuState::default();
        assert!(!state.cancel());
        state.open = true;
        assert!(state.cancel());
        assert!(!state.open);
    }

    #[test]
    fn clicking_an_open_item_activates_it() {
        let mut state = MenuState::default();
        let layout = layout();
        state.click(&layout, &items(), 10, 10);
        let item = layout.items[2];
        assert_eq!(
            state.click(&layout, &items(), center(item).0, center(item).1),
            MenuAction::Item("lux-color".to_string())
        );
        assert!(!state.open);
    }

    #[test]
    fn click_outside_open_menu_closes_it() {
        let mut state = MenuState::default();
        state.open = true;
        assert_eq!(
            state.click(&layout(), &items(), 700, 700),
            MenuAction::Consumed
        );
        assert!(!state.open);
    }

    #[test]
    fn controller_right_click_over_button_opens_menu() {
        let mut state = MenuState::default();
        let layout = layout();
        assert_eq!(
            state.command(
                &layout,
                &items(),
                center(layout.button),
                FormatCommand::Pointer {
                    button: FORMAT_POINTER_RIGHT,
                    pressed: true,
                },
            ),
            MenuAction::Consumed
        );
        assert!(state.open);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn controller_right_click_activates_the_selected_item() {
        let mut state = MenuState::default();
        let layout = layout();
        state.command(
            &layout,
            &items(),
            center(layout.button),
            FormatCommand::Pointer {
                button: FORMAT_POINTER_RIGHT,
                pressed: true,
            },
        );
        assert_eq!(
            state.command(
                &layout,
                &items(),
                (700, 700),
                FormatCommand::Pointer {
                    button: FORMAT_POINTER_RIGHT,
                    pressed: true,
                },
            ),
            MenuAction::Select(FORMAT_CHOOSER.to_string())
        );
        assert!(!state.open);
    }

    #[test]
    fn controller_navigation_cycles_all_four_items() {
        let mut state = MenuState {
            open: true,
            selected: 0,
        };
        for expected in [1, 2, 3, 0] {
            assert_eq!(
                state.command(
                    &layout(),
                    &items(),
                    (0, 0),
                    FormatCommand::Navigate { dx: 1, dy: 0 },
                ),
                MenuAction::Consumed
            );
            assert_eq!(state.selected, expected);
        }
        assert_eq!(
            state.command(
                &layout(),
                &items(),
                (0, 0),
                FormatCommand::Navigate { dx: -1, dy: 0 },
            ),
            MenuAction::Consumed
        );
        assert_eq!(state.selected, 3);
    }

    #[test]
    fn controller_navigation_is_ignored_when_closed() {
        assert_eq!(
            MenuState::default().command(
                &layout(),
                &items(),
                (0, 0),
                FormatCommand::Navigate { dx: 1, dy: 0 },
            ),
            MenuAction::Ignored
        );
    }

    #[test]
    fn controller_cursor_move_is_ignored_by_menu() {
        assert_eq!(
            MenuState::default().command(
                &layout(),
                &items(),
                (0, 0),
                FormatCommand::CursorMove { dx: 1, dy: 1 },
            ),
            MenuAction::Ignored
        );
    }
}
