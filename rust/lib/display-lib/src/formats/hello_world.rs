//! The hello-world format: a dark background with the earth and the greeting
//! text, rendered from the view data the display library receives.
//!
//! Until the view service's first sample arrives, the region is rendered black.

use std::path::Path;

use glow::HasContext as _;

use format_list::FORMAT_POINTER_RIGHT;
use format_list::{
    HELLO_ACTION_COLOR_CANCEL, HELLO_ACTION_COLOR_COMMIT, HELLO_ACTION_COLOR_HUE,
    HELLO_ACTION_COLOR_SV, HELLO_ACTION_DRAG, HELLO_ACTION_GRAB, HELLO_ACTION_MODE_TOGGLE,
    HELLO_ACTION_RELEASE, HELLO_ACTION_ROTATE, HELLO_ACTION_TOGGLE_ARCBALL,
    HELLO_ACTION_TOGGLE_LUX_COLOR, HELLO_ACTION_TOGGLE_LUX_POS, HELLO_MENU_LUX_COLOR,
    HELLO_MENU_LUX_POS, HELLO_MENU_MODE, HELLO_MODE_INSPECT, HELLO_OVERLAY_COLOR,
    HELLO_OVERLAY_NONE, HELLO_OVERLAY_POS,
};

use crate::color_picker::{ColorPicker, PickerControl, PickerHit, PickerLayout, WheelLatch};
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
/// Rate at which the right stick moves the color picker selection.
const STICK_COLOR_RATE: f64 = 0.5;
/// Fraction of the supplied right-stick delta used for triangle x-y movement.
const TRIANGLE_STICK_STEP: f32 = 0.5;

/// The hello-world format.
pub(crate) struct HelloWorld {
    earth: Earth,
    text: TextRenderer,
    picker: ColorPicker,
    /// The latest view sample, shared with the display library's DDS poller.
    state: ViewState,
    input: Option<HelloWorldInput>,
    dragging: bool,
    last_pointer: (f64, f64),
    region: (i32, i32, i32, i32),
    picker_focus: PickerControl,
    picker_captured: bool,
    mouse_picking: Option<PickerHit>,
    triangle_cursor: Option<(f32, f32)>,
    wheel: WheelLatch,
    last_overlay: i32,
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
        let picker = ColorPicker::new(gl, assets_root)?;
        Ok(Self {
            earth,
            text,
            picker,
            state,
            input,
            dragging: false,
            last_pointer: (0.0, 0.0),
            region: (0, 0, 0, 0),
            picker_focus: PickerControl::HueRing,
            picker_captured: false,
            mouse_picking: None,
            triangle_cursor: None,
            wheel: WheelLatch::default(),
            last_overlay: HELLO_OVERLAY_NONE,
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

    fn overlay(&self) -> i32 {
        self.state
            .borrow()
            .as_ref()
            .map(|view| view.overlay)
            .unwrap_or(HELLO_OVERLAY_NONE)
    }

    fn color_state(&self) -> (f32, f32, f32) {
        self.state
            .borrow()
            .as_ref()
            .map(|view| (view.hue, view.saturation, view.value))
            .unwrap_or((0.0, 0.0, 1.0))
    }

    fn sync_overlay(&mut self, overlay: i32) {
        if overlay != self.last_overlay {
            let opening = overlay == HELLO_OVERLAY_COLOR;
            self.picker_focus = PickerControl::HueRing;
            self.picker_captured = opening;
            self.mouse_picking = None;
            self.triangle_cursor = None;
            self.wheel.clear();
            self.last_overlay = overlay;
        }
    }

    fn clear_capture(&mut self) {
        self.picker_captured = false;
        self.mouse_picking = None;
        self.triangle_cursor = None;
        self.wheel.clear();
    }

    fn send_commit(&mut self) {
        self.send(HELLO_ACTION_COLOR_COMMIT, 0.0, 0.0, true);
        self.clear_capture();
    }

    fn send_cancel(&mut self) {
        self.send(HELLO_ACTION_COLOR_CANCEL, 0.0, 0.0, true);
        self.clear_capture();
    }

    fn focus_picker(&mut self, focus: PickerControl, capture: bool) {
        self.picker_focus = focus;
        self.picker_captured = capture;
        self.triangle_cursor = if capture && focus == PickerControl::Triangle {
            let (_, saturation, value) = self.color_state();
            Some(PickerLayout::new(self.region).sv_marker(saturation, value))
        } else {
            None
        };
    }

    fn picker_hit_at(&self, cursor: Option<(i32, i32)>) -> PickerHit {
        let Some((x, y)) = cursor else {
            return PickerHit::None;
        };
        PickerLayout::new(self.region).hit(x as f32, y as f32)
    }

    fn toggle_picker_focus(&mut self) {
        let next = if self.picker_focus == PickerControl::HueRing {
            PickerControl::Triangle
        } else {
            PickerControl::HueRing
        };
        self.focus_picker(next, self.picker_captured);
    }

    fn move_triangle_cursor(&mut self, dx: i32, dy: i32) {
        let layout = PickerLayout::new(self.region);
        let (_, saturation, value) = self.color_state();
        let current = self
            .triangle_cursor
            .unwrap_or_else(|| layout.sv_marker(saturation, value));
        let point = layout.clamp_triangle(
            current.0 + dx as f32 * TRIANGLE_STICK_STEP,
            current.1 + dy as f32 * TRIANGLE_STICK_STEP,
        );
        self.triangle_cursor = Some(point);
        let (saturation, value) = layout.sv_at(point.0, point.1);
        self.send(HELLO_ACTION_COLOR_SV, saturation as f64, value as f64, true);
    }

    fn move_hue_wheel(&mut self, dx: i32, dy: i32) {
        let Some((direction, magnitude)) = self.wheel.motion(dx, dy) else {
            return;
        };
        let half = self.region.2.min(self.region.3).max(1) as f64 * 0.5;
        let (hue, _, _) = self.color_state();
        let hue = (hue as f64 + direction as f64 * magnitude as f64 / half * STICK_COLOR_RATE)
            .rem_euclid(1.0);
        self.send(HELLO_ACTION_COLOR_HUE, hue, 0.0, true);
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
        let overlay = view
            .as_ref()
            .map(|view| view.overlay)
            .unwrap_or(HELLO_OVERLAY_NONE);
        self.sync_overlay(overlay);
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
            hue,
            saturation,
            value,
            overlay,
            mode,
            ..
        }) = view
        else {
            return; // black until the first view sample arrives
        };

        if overlay == HELLO_OVERLAY_COLOR {
            let triangle_marker =
                if self.picker_captured && self.picker_focus == PickerControl::Triangle {
                    self.triangle_cursor
                } else {
                    None
                };
            self.picker.render(
                gl,
                framebuffer_height,
                self.region,
                hue,
                saturation,
                value,
                color,
                triangle_marker,
                self.picker_focus,
                self.picker_captured,
            );
            return;
        }

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
        let overlay = self.overlay();
        self.sync_overlay(overlay);
        if overlay == HELLO_OVERLAY_COLOR && pressed {
            self.clear_capture();
        }
        if button != 0 {
            return FormatInput::Ignored;
        }
        if overlay == HELLO_OVERLAY_COLOR {
            if pressed {
                let layout = PickerLayout::new(region);
                match layout.hit(x as f32, y as f32) {
                    PickerHit::Back => self.send_cancel(),
                    PickerHit::Done => self.send_commit(),
                    PickerHit::HueRing => {
                        self.focus_picker(PickerControl::HueRing, false);
                        self.mouse_picking = Some(PickerHit::HueRing);
                        let hue = layout.hue_at(x as f32, y as f32);
                        self.send(HELLO_ACTION_COLOR_HUE, hue as f64, 0.0, true);
                    }
                    PickerHit::Triangle => {
                        self.focus_picker(PickerControl::Triangle, false);
                        self.mouse_picking = Some(PickerHit::Triangle);
                        let (saturation, value) = layout.sv_at(x as f32, y as f32);
                        self.send(HELLO_ACTION_COLOR_SV, saturation as f64, value as f64, true);
                    }
                    PickerHit::None => {}
                }
            } else {
                self.mouse_picking = None;
            }
            return FormatInput::Consumed;
        }

        if pressed {
            let Some(point) = normalized(region, x, y) else {
                return FormatInput::Ignored;
            };
            if !on_globe(point) {
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
        let overlay = self.overlay();
        self.sync_overlay(overlay);
        if overlay == HELLO_OVERLAY_COLOR {
            if self.picker_captured {
                self.clear_capture();
            }
            let Some(picking) = self.mouse_picking else {
                return FormatInput::Ignored;
            };
            let layout = PickerLayout::new(region);
            match picking {
                PickerHit::HueRing => {
                    let hue = layout.hue_at(x as f32, y as f32);
                    self.send(HELLO_ACTION_COLOR_HUE, hue as f64, 0.0, true);
                }
                PickerHit::Triangle => {
                    let (saturation, value) = layout.sv_at(x as f32, y as f32);
                    self.send(HELLO_ACTION_COLOR_SV, saturation as f64, value as f64, true);
                }
                _ => {}
            }
            return FormatInput::Consumed;
        }

        if !self.dragging {
            return FormatInput::Ignored;
        }
        let Some(point) = normalized(region, x, y) else {
            return FormatInput::Ignored;
        };
        self.last_pointer = point;

        // Leaving the globe releases the drag immediately, even if the button
        // is still held. The final drag at the rim gives the view service a
        // velocity to carry into inertia.
        self.send(HELLO_ACTION_DRAG, point.0, point.1, true);
        if !on_globe(point) {
            self.send(HELLO_ACTION_RELEASE, point.0, point.1, false);
            self.dragging = false;
        }
        FormatInput::Consumed
    }

    fn mouse_wheel(&mut self, _region: (i32, i32, i32, i32), _delta: i32) -> FormatInput {
        FormatInput::Ignored
    }

    fn command(&mut self, command: FormatCommand, cursor: Option<(i32, i32)>) -> FormatInput {
        let overlay = self.overlay();
        self.sync_overlay(overlay);

        if overlay == HELLO_OVERLAY_COLOR {
            match command {
                FormatCommand::Navigate { dx, dy } => {
                    let direction = if dx < 0 || dy < 0 { -1 } else { 1 };
                    let focus = self.picker_focus.next(direction);
                    self.focus_picker(focus, self.picker_captured);
                    FormatInput::Consumed
                }
                FormatCommand::Pointer {
                    button,
                    pressed: true,
                } if button == FORMAT_POINTER_RIGHT => {
                    let hit = self.picker_hit_at(cursor);
                    match hit {
                        PickerHit::Back => self.send_cancel(),
                        PickerHit::Done => self.send_commit(),
                        PickerHit::HueRing | PickerHit::Triangle | PickerHit::None => {
                            match self.picker_focus {
                                PickerControl::Back => self.send_cancel(),
                                PickerControl::Done => self.send_commit(),
                                PickerControl::HueRing | PickerControl::Triangle => match hit {
                                    PickerHit::HueRing => {
                                        self.focus_picker(PickerControl::HueRing, true)
                                    }
                                    PickerHit::Triangle => {
                                        self.focus_picker(PickerControl::Triangle, true)
                                    }
                                    _ => {
                                        let focus = self.picker_focus;
                                        self.focus_picker(focus, true);
                                    }
                                },
                            }
                        }
                    }
                    FormatInput::Consumed
                }
                FormatCommand::Pointer { .. } => FormatInput::Consumed,
                FormatCommand::ToggleArcball => {
                    if self.picker_captured {
                        self.toggle_picker_focus();
                    } else {
                        match self.picker_hit_at(cursor) {
                            PickerHit::HueRing => {
                                self.picker_focus = PickerControl::HueRing;
                            }
                            PickerHit::Triangle => {
                                self.picker_focus = PickerControl::Triangle;
                            }
                            _ => {}
                        }
                    }
                    FormatInput::Consumed
                }
                FormatCommand::CursorMove { dx, dy } if self.picker_captured => {
                    if dx == 0 && dy == 0 {
                        self.wheel.clear();
                    } else {
                        match self.picker_focus {
                            PickerControl::HueRing => self.move_hue_wheel(dx, dy),
                            PickerControl::Triangle => self.move_triangle_cursor(dx, dy),
                            PickerControl::Back | PickerControl::Done => {}
                        }
                    }
                    FormatInput::Consumed
                }
                FormatCommand::CancelMenu => {
                    if self.picker_captured {
                        self.clear_capture();
                    } else {
                        self.send_cancel();
                    }
                    FormatInput::Consumed
                }
                FormatCommand::OverlayCancel => {
                    self.send_cancel();
                    FormatInput::Consumed
                }
                FormatCommand::ReleaseCapture => {
                    self.clear_capture();
                    FormatInput::Consumed
                }
                FormatCommand::CursorMove { .. } => FormatInput::Ignored,
                FormatCommand::ToggleMenu => FormatInput::Ignored,
            }
        } else {
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
    }

    fn cursor_hidden(&self) -> bool {
        self.picker_captured
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
        let overlay = self.overlay();
        match id {
            HELLO_MENU_MODE => {
                if overlay == HELLO_OVERLAY_COLOR {
                    self.send_cancel();
                }
                self.send(HELLO_ACTION_MODE_TOGGLE, 0.0, 0.0, true);
            }
            HELLO_MENU_LUX_COLOR => {
                let opening = overlay != HELLO_OVERLAY_COLOR;
                self.send(HELLO_ACTION_TOGGLE_LUX_COLOR, 0.0, 0.0, true);
                self.picker_focus = PickerControl::HueRing;
                self.picker_captured = opening;
                self.mouse_picking = None;
                self.triangle_cursor = None;
                self.wheel.clear();
            }
            HELLO_MENU_LUX_POS => {
                if overlay == HELLO_OVERLAY_COLOR {
                    self.send_cancel();
                }
                self.send(HELLO_ACTION_TOGGLE_LUX_POS, 0.0, 0.0, true);
                self.picker_captured = false;
                self.mouse_picking = None;
            }
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

/// Whether a normalized point is inside the globe's drag area.
fn on_globe(point: (f64, f64)) -> bool {
    point.0 * point.0 + point.1 * point.1 <= 1.0
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
    fn globe_hit_area_is_the_unit_circle() {
        assert!(on_globe((0.0, 0.0)));
        assert!(on_globe((0.5, -0.5)));
        assert!(on_globe((1.0, 0.0)));
        assert!(!on_globe((0.8, 0.8)));
        assert!(!on_globe((1.1, 0.0)));
    }

    #[test]
    fn normalized_center_and_edges() {
        assert_eq!(normalized((10, 20, 100, 100), 60, 70), Some((0.0, 0.0)));
        assert_eq!(normalized((10, 20, 100, 100), 10, 20), Some((-1.0, 1.0)));
        assert_eq!(normalized((10, 20, 0, 100), 10, 20), None);
    }
}
