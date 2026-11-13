//! Mapping gamepad state into the paws window and virtual mouse input.

use gilrs::{Axis, Button};

use crate::MainWindow;

const DEADZONE: f32 = 0.08;
const TRIGGER_THRESHOLD: f32 = 0.5;

/// The controls shown in the paws window.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Controls {
    pub(crate) left: (f32, f32),
    pub(crate) right: (f32, f32),
    pub(crate) a: bool,
    pub(crate) b: bool,
    pub(crate) x: bool,
    pub(crate) y: bool,
    pub(crate) lb: bool,
    pub(crate) rb: bool,
    pub(crate) lt: bool,
    pub(crate) rt: bool,
    pub(crate) up: bool,
    pub(crate) down: bool,
    pub(crate) left_dpad: bool,
    pub(crate) right_dpad: bool,
    pub(crate) back: bool,
    pub(crate) start: bool,
}

/// The subset of gamepad state the mapper reads. Implemented for gilrs
/// controllers and by a fake in tests.
pub(crate) trait PadState {
    fn axis(&self, axis: Axis) -> f32;
    fn pressed(&self, button: Button) -> bool;
}

impl PadState for gilrs::Gamepad<'_> {
    fn axis(&self, axis: Axis) -> f32 {
        self.value(axis)
    }

    fn pressed(&self, button: Button) -> bool {
        self.is_pressed(button)
    }
}

impl Controls {
    pub(crate) fn neutral() -> Self {
        Self::default()
    }

    pub(crate) fn read(pad: &impl PadState) -> Self {
        Self {
            left: stick(pad.axis(Axis::LeftStickX), pad.axis(Axis::LeftStickY)),
            right: stick(pad.axis(Axis::RightStickX), pad.axis(Axis::RightStickY)),
            a: pad.pressed(Button::South),
            b: pad.pressed(Button::East),
            x: pad.pressed(Button::West),
            y: pad.pressed(Button::North),
            lb: pad.pressed(Button::LeftTrigger),
            rb: pad.pressed(Button::RightTrigger),
            lt: pad.axis(Axis::LeftZ) > TRIGGER_THRESHOLD || pad.pressed(Button::LeftTrigger2),
            rt: pad.axis(Axis::RightZ) > TRIGGER_THRESHOLD || pad.pressed(Button::RightTrigger2),
            up: pad.pressed(Button::DPadUp),
            down: pad.pressed(Button::DPadDown),
            left_dpad: pad.pressed(Button::DPadLeft),
            right_dpad: pad.pressed(Button::DPadRight),
            back: pad.pressed(Button::Select),
            start: pad.pressed(Button::Start),
        }
    }

    pub(crate) fn apply(&self, ui: &MainWindow) {
        ui.set_left_x(self.left.0);
        ui.set_left_y(self.left.1);
        ui.set_right_x(self.right.0);
        ui.set_right_y(self.right.1);
        ui.set_a_down(self.a);
        ui.set_b_down(self.b);
        ui.set_x_down(self.x);
        ui.set_y_down(self.y);
        ui.set_lb_down(self.lb);
        ui.set_rb_down(self.rb);
        ui.set_lt_down(self.lt);
        ui.set_rt_down(self.rt);
        ui.set_up_down(self.up);
        ui.set_down_down(self.down);
        ui.set_left_down(self.left_dpad);
        ui.set_right_down(self.right_dpad);
        ui.set_back_down(self.back);
        ui.set_start_down(self.start);
    }
}

/// Apply a virtual-mode mouse press to the matching UI control.
pub(crate) fn set_virtual_button(ui: &MainWindow, control: &str, down: bool) {
    match control {
        "A" => ui.set_a_down(down),
        "B" => ui.set_b_down(down),
        "X" => ui.set_x_down(down),
        "Y" => ui.set_y_down(down),
        "LB" => ui.set_lb_down(down),
        "RB" => ui.set_rb_down(down),
        "LT" => ui.set_lt_down(down),
        "RT" => ui.set_rt_down(down),
        "Up" => ui.set_up_down(down),
        "Down" => ui.set_down_down(down),
        "Left" => ui.set_left_down(down),
        "Right" => ui.set_right_down(down),
        "Back" => ui.set_back_down(down),
        "Start" => ui.set_start_down(down),
        _ => {}
    }
}

/// Apply a deadzone and clamp one axis to `[-1, 1]`.
fn axis(value: f32) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    let value = value.clamp(-1.0, 1.0);
    if value.abs() <= DEADZONE {
        0.0
    } else {
        let scaled = (value.abs() - DEADZONE) / (1.0 - DEADZONE);
        value.signum() * scaled
    }
}

/// Map a gamepad stick to the window's top-left-origin coordinates.
fn stick(x: f32, y: f32) -> (f32, f32) {
    (axis(x), -axis(y))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        axes: Vec<(Axis, f32)>,
        buttons: Vec<Button>,
    }

    impl PadState for Fake {
        fn axis(&self, axis: Axis) -> f32 {
            self.axes
                .iter()
                .find(|(candidate, _)| *candidate == axis)
                .map(|(_, value)| *value)
                .unwrap_or(0.0)
        }

        fn pressed(&self, button: Button) -> bool {
            self.buttons.contains(&button)
        }
    }

    fn fake() -> Fake {
        Fake {
            axes: Vec::new(),
            buttons: Vec::new(),
        }
    }

    #[test]
    fn neutral_is_empty() {
        assert_eq!(Controls::read(&fake()), Controls::neutral());
    }

    #[test]
    fn sticks_are_inverted_and_deadzoned() {
        let pad = Fake {
            axes: vec![
                (Axis::LeftStickX, 0.54),
                (Axis::LeftStickY, 0.54),
                (Axis::RightStickX, -0.08),
                (Axis::RightStickY, -0.54),
            ],
            buttons: Vec::new(),
        };
        let controls = Controls::read(&pad);
        assert!(!controls.right.0.is_sign_negative());
        assert!(controls.right.0.abs() < 1e-6);
        assert!(controls.left.0 > 0.49);
        assert!(controls.left.1 < -0.49);
        assert!(controls.right.1 > 0.49);
    }

    #[test]
    fn buttons_map_to_face_and_shoulders() {
        let pad = Fake {
            axes: Vec::new(),
            buttons: vec![
                Button::South,
                Button::East,
                Button::West,
                Button::North,
                Button::LeftTrigger,
                Button::RightTrigger,
                Button::DPadUp,
                Button::DPadDown,
                Button::DPadLeft,
                Button::DPadRight,
                Button::Select,
                Button::Start,
            ],
        };
        let controls = Controls::read(&pad);
        assert!(controls.a && controls.b && controls.x && controls.y);
        assert!(controls.lb && controls.rb);
        assert!(controls.up && controls.down && controls.left_dpad && controls.right_dpad);
        assert!(controls.back && controls.start);
        assert!(!controls.lt && !controls.rt);
    }

    #[test]
    fn triggers_use_analog_axes() {
        let pad = Fake {
            axes: vec![(Axis::LeftZ, 0.75), (Axis::RightZ, 0.2)],
            buttons: Vec::new(),
        };
        let controls = Controls::read(&pad);
        assert!(controls.lt);
        assert!(!controls.rt);
    }

    #[test]
    fn axis_deadzone_and_clamping() {
        assert_eq!(axis(0.0), 0.0);
        assert_eq!(axis(-0.08), 0.0);
        assert_eq!(axis(2.0), 1.0);
        assert_eq!(axis(-2.0), -1.0);
        assert!(axis(0.54) > 0.49);
    }
}
