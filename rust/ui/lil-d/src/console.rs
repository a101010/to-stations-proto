//! The console strip: the station label and the power button.

use egui::{Color32, Frame, Panel};

/// Console fill color.
pub const CONSOLE_FILL: Color32 = Color32::from_rgb(15, 15, 18);

/// What the console requested this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConsoleAction {
    pub toggle_power: bool,
}

/// Draw the console into the root egui `Ui` and report the user's action.
///
/// `height` is the console strip height in logical points, taken from the
/// computed layout so the egui panel matches the GL geometry.
pub fn draw(ui: &mut egui::Ui, station: u32, format_on: bool, height: f32) -> ConsoleAction {
    let mut action = ConsoleAction::default();

    Panel::bottom("lil-d-console")
        .exact_size(height)
        .frame(Frame::new().fill(CONSOLE_FILL))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(format!("station {station}"));
                ui.separator();
                let label = if format_on { "Power: on" } else { "Power: off" };
                if ui.button(label).clicked() {
                    action.toggle_power = true;
                }
            });
        });

    action
}
