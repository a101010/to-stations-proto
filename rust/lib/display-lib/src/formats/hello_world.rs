//! The hello-world format: a dark background with the earth and the greeting
//! text, rendered from the view data the display library receives.
//!
//! Until the view service's first sample arrives, the region is rendered black.

use std::path::Path;

use glow::HasContext as _;

use format_list::{
    HELLO_ACTION_DRAG, HELLO_ACTION_GRAB, HELLO_ACTION_MODE_TOGGLE, HELLO_ACTION_RELEASE,
    HELLO_ACTION_ROTATE, HELLO_ACTION_TOGGLE_ARCBALL, HELLO_ACTION_TOGGLE_LUX_COLOR,
    HELLO_ACTION_TOGGLE_LUX_POS, HELLO_MENU_LUX_COLOR, HELLO_MENU_LUX_POS, HELLO_MENU_MODE,
    HELLO_MODE_INSPECT, HELLO_OVERLAY_COLOR, HELLO_OVERLAY_POS,
};

use crate::earth::Earth;
use crate::formats::{FormatCommand, FormatInput, MenuItem};
use crate::text::{TextRenderer, FONT_FILE};
use crate::view::{HelloWorldInput, View, ViewState};
use crate::DisplayError;

/// Background of the hello-world format (once view data has arrived).
const BACKGROUND: [f32; 4] = [0.02, 0.03, 0.08, 1.0];
/// The region is cleared to black while there is no view data.
const NO_DATA_BACKGROUND: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
/// Greeting color.
const TEXT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
/// Text size as a fraction of the region height.
const FONT_SIZE_FRACTION: f32 = 0.15;
/// Fraction of the region width available to centered text.
const TEXT_WIDTH_FRACTION: f32 = 0.9;
/// The widest view string: all greetings and `NoConn hh:mm:ss` fit within it.
/// Keep this in sync when adding a greeting.
const MAX_TEXT: &str = "Bonjour le monde!";
/// Right-stick movement is converted to this fraction of a region half-width.
const STICK_FRACTION: f64 = 0.5;

/// The hello-world format.
pub(crate) struct HelloWorld {
    earth: Earth,
    text: TextRenderer,
    /// The latest view sample, shared with the display library's DDS poller.
    state: ViewState,
    input: Option<HelloWorldInput>,
    dragging: bool,
    last_pointer: (f64, f64),
    region: (i32, i32, i32, i32),
}

impl HelloWorld {
    /// Build the format, loading its assets from `assets_root`.
    pub(crate) fn new(
        gl: &glow::Context,
        assets_root: &Path,
        state: ViewState,
        input: Option<HelloWorldInput>,
    ) -> Result<Self, DisplayError> {
        let earth = Earth::new(gl, assets_root)?;
        let text = TextRenderer::new(gl, &assets_root.join(FONT_FILE))?;
        Ok(Self {
            earth,
            text,
            state,
            input,
            dragging: false,
            last_pointer: (0.0, 0.0),
            region: (0, 0, 0, 0),
        })
    }

    fn send(&self, action: i32, x: f64, y: f64, pressed: bool) {
        if let Some(input) = &self.input {
            input.send(action, x, y, 0.0, pressed);
        }
    }

    fn arcball_enabled(&self) -> bool {
        self.state
            .borrow()
            .as_ref()
            .map(|view| view.arcball)
            .unwrap_or(false)
    }
}

impl crate::formats::Format for HelloWorld {
    fn name(&self) -> &'static str {
        "hello-world"
    }

    fn title(&self) -> &'static str {
        "Hello World"
    }

    fn icon_file(&self) -> &'static str {
        "icons/hello-world.png"
    }

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
        self.region = (x, y, width, height);
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
            light,
            color,
            mode,
            ..
        }) = view
        else {
            return; // black until the first view sample arrives
        };

        if let Some(attitude) = attitude {
            self.earth.render(gl, width, height, attitude, light, color);
        }
        if mode == HELLO_MODE_INSPECT {
            return;
        }
        let font_size_px = self.text.font_size_for(
            MAX_TEXT,
            width as f32 * TEXT_WIDTH_FRACTION,
            height as f32 * FONT_SIZE_FRACTION,
        );
        self.text
            .draw(gl, width, height, &greeting, TEXT_COLOR, font_size_px);
    }

    fn mouse_button(
        &mut self,
        region: (i32, i32, i32, i32),
        x: i32,
        y: i32,
        button: i32,
        pressed: bool,
    ) -> FormatInput {
        if button != 0 {
            return FormatInput::Ignored;
        }
        if pressed {
            let Some(point) = normalized(region, x, y) else {
                return FormatInput::Ignored;
            };
            if point.0 * point.0 + point.1 * point.1 > 1.0 {
                return FormatInput::Ignored;
            }
            self.dragging = true;
            self.last_pointer = point;
            self.send(HELLO_ACTION_GRAB, point.0, point.1, true);
            return FormatInput::Consumed;
        }
        if self.dragging {
            self.dragging = false;
            self.send(
                HELLO_ACTION_RELEASE,
                self.last_pointer.0,
                self.last_pointer.1,
                false,
            );
            return FormatInput::Consumed;
        }
        FormatInput::Ignored
    }

    fn mouse_motion(&mut self, region: (i32, i32, i32, i32), x: i32, y: i32) -> FormatInput {
        if !self.dragging {
            return FormatInput::Ignored;
        }
        let Some(point) = normalized(region, x, y) else {
            return FormatInput::Ignored;
        };
        self.last_pointer = point;
        self.send(HELLO_ACTION_DRAG, point.0, point.1, true);
        FormatInput::Consumed
    }

    fn mouse_wheel(&mut self, _region: (i32, i32, i32, i32), _delta: i32) -> FormatInput {
        FormatInput::Ignored
    }

    fn command(&mut self, command: FormatCommand) -> FormatInput {
        match command {
            FormatCommand::ToggleArcball => {
                self.send(HELLO_ACTION_TOGGLE_ARCBALL, 0.0, 0.0, true);
                FormatInput::Consumed
            }
            FormatCommand::CursorMove { dx, dy } if self.arcball_enabled() => {
                let half = self.region.2.min(self.region.3).max(1) as f64 * 0.5;
                self.send(
                    HELLO_ACTION_ROTATE,
                    dx as f64 / half * STICK_FRACTION,
                    -(dy as f64) / half * STICK_FRACTION,
                    true,
                );
                FormatInput::Consumed
            }
            _ => FormatInput::Ignored,
        }
    }

    fn menu_items(&self) -> Vec<MenuItem> {
        let view = self.state.borrow();
        let (mode, overlay) = view
            .as_ref()
            .map(|view| (view.mode, view.overlay))
            .unwrap_or((0, 0));
        hello_menu_items(mode, overlay)
    }

    fn menu_action(&mut self, id: &str) -> FormatInput {
        match id {
            HELLO_MENU_MODE => self.send(HELLO_ACTION_MODE_TOGGLE, 0.0, 0.0, true),
            HELLO_MENU_LUX_COLOR => self.send(HELLO_ACTION_TOGGLE_LUX_COLOR, 0.0, 0.0, true),
            HELLO_MENU_LUX_POS => self.send(HELLO_ACTION_TOGGLE_LUX_POS, 0.0, 0.0, true),
            _ => return FormatInput::Ignored,
        }
        FormatInput::Consumed
    }
}

/// The hello-world format-specific menu items after `Formats`.
fn hello_menu_items(mode: i32, overlay: i32) -> Vec<MenuItem> {
    vec![
        MenuItem {
            id: HELLO_MENU_MODE.to_string(),
            label: if mode == HELLO_MODE_INSPECT {
                "Auto".to_string()
            } else {
                "Inspect".to_string()
            },
        },
        MenuItem {
            id: HELLO_MENU_LUX_COLOR.to_string(),
            label: if overlay == HELLO_OVERLAY_COLOR {
                "World".to_string()
            } else {
                "Lux Color".to_string()
            },
        },
        MenuItem {
            id: HELLO_MENU_LUX_POS.to_string(),
            label: if overlay == HELLO_OVERLAY_POS {
                "World".to_string()
            } else {
                "Lux Pos".to_string()
            },
        },
    ]
}

/// Map a region point to normalized arcball coordinates, `y` up.
fn normalized(region: (i32, i32, i32, i32), x: i32, y: i32) -> Option<(f64, f64)> {
    let (rx, ry, width, height) = region;
    let half = width.min(height) as f64 * 0.5;
    if half <= 0.0 {
        return None;
    }
    let cx = rx as f64 + width as f64 / 2.0;
    let cy = ry as f64 + height as f64 / 2.0;
    Some(((x as f64 - cx) / half, (cy - y as f64) / half))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_labels_cycle_all_three_format_items() {
        let labels = |mode, overlay| {
            hello_menu_items(mode, overlay)
                .into_iter()
                .map(|item| item.label)
                .collect::<Vec<_>>()
        };
        assert_eq!(labels(0, 0), vec!["Inspect", "Lux Color", "Lux Pos"]);
        assert_eq!(
            labels(HELLO_MODE_INSPECT, HELLO_OVERLAY_COLOR),
            vec!["Auto", "World", "Lux Pos"]
        );
        assert_eq!(
            labels(0, HELLO_OVERLAY_POS),
            vec!["Inspect", "Lux Color", "World"]
        );
    }

    #[test]
    fn max_text_is_at_least_as_wide_as_no_conn() {
        let root = std::env::var("REPO_ROOT").expect("REPO_ROOT is set by .cargo/config.toml");
        let font = crate::text::Font::load(
            &std::path::PathBuf::from(root).join("assets/fonts/FiraSans-Regular.ttf"),
        )
        .expect("load font");
        let advance =
            |text: &str| -> f32 { font.shape(text).iter().map(|glyph| glyph.x_advance).sum() };
        assert!(advance(MAX_TEXT) >= advance("NoConn 00:00:00"));
    }

    #[test]
    fn normalized_center_and_edges() {
        assert_eq!(normalized((10, 20, 100, 100), 60, 70), Some((0.0, 0.0)));
        assert_eq!(normalized((10, 20, 100, 100), 10, 20), Some((-1.0, 1.0)));
        assert_eq!(normalized((10, 20, 0, 100), 10, 20), None);
    }
}
