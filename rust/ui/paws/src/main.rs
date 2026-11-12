#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod stick;

use slint::ComponentHandle as _;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;

    let weak = ui.as_weak();
    ui.on_stick_drag(move |index, dx, dy, radius| {
        if let Some(ui) = weak.upgrade() {
            let (x, y) = stick::clamp_stick(dx, dy, radius);
            match index {
                0 => {
                    ui.set_left_x(x);
                    ui.set_left_y(y);
                }
                1 => {
                    ui.set_right_x(x);
                    ui.set_right_y(y);
                }
                _ => {}
            }
        }
    });

    let weak = ui.as_weak();
    ui.on_stick_release(move |index| {
        if let Some(ui) = weak.upgrade() {
            match index {
                0 => {
                    ui.set_left_x(0.0);
                    ui.set_left_y(0.0);
                }
                1 => {
                    ui.set_right_x(0.0);
                    ui.set_right_y(0.0);
                }
                _ => {}
            }
        }
    });

    ui.run()
}
