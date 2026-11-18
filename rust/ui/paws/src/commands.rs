//! High-level commands from paws to the format-manager controller.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use format_list::{
    FORMAT_COMMAND_CURSOR_MOVE, FORMAT_COMMAND_MENU_CANCEL, FORMAT_COMMAND_MENU_TOGGLE,
    FORMAT_COMMAND_NAVIGATE, FORMAT_COMMAND_OVERLAY_CANCEL, FORMAT_COMMAND_POINTER,
    FORMAT_COMMAND_TOGGLE_ARCBALL, FORMAT_POINTER_RIGHT,
};
use stations_dds::contract::{ServiceContract, TopicRegistry};
use stations_dds::cyclonedds::DataWriter;
use stations_dds::dds::Node;
use stations_dds::format_manager::FormatManagerInputTopic;

use crate::MainWindow;

const CURSOR_INTERVAL: Duration = Duration::from_millis(33);
const CURSOR_SPEED: f32 = 20.0;

#[derive(Clone, Copy, Debug, Default)]
struct Inputs {
    right: (f32, f32),
    up: bool,
    down: bool,
    left: bool,
    right_dpad: bool,
    a: bool,
    b: bool,
    back: bool,
    start: bool,
    r3_checked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WireCommand {
    command: i32,
    x: i32,
    y: i32,
    button: i32,
    pressed: bool,
}

#[derive(Default)]
struct CommandState {
    last_cursor: Option<Instant>,
    last_up: bool,
    last_down: bool,
    last_left: bool,
    last_right: bool,
    last_a: bool,
    last_b: bool,
    last_back: bool,
    last_start: bool,
    last_r3_checked: bool,
}

impl CommandState {
    fn update(&mut self, inputs: Inputs, now: Instant) -> Vec<WireCommand> {
        let mut commands = Vec::new();

        let magnitude = inputs.right.0.abs().max(inputs.right.1.abs());
        let cursor_due = self
            .last_cursor
            .is_none_or(|last| now.duration_since(last) >= CURSOR_INTERVAL);
        if magnitude > 0.1 && cursor_due {
            let dx = (inputs.right.0 * CURSOR_SPEED).round() as i32;
            let dy = (inputs.right.1 * CURSOR_SPEED).round() as i32;
            if dx != 0 || dy != 0 {
                commands.push(WireCommand {
                    command: FORMAT_COMMAND_CURSOR_MOVE,
                    x: dx,
                    y: dy,
                    button: 0,
                    pressed: false,
                });
                self.last_cursor = Some(now);
            }
        }

        for (pressed, was, dx, dy) in [
            (inputs.up, self.last_up, 0, -1),
            (inputs.down, self.last_down, 0, 1),
            (inputs.left, self.last_left, -1, 0),
            (inputs.right_dpad, self.last_right, 1, 0),
        ] {
            if pressed && !was {
                commands.push(WireCommand {
                    command: FORMAT_COMMAND_NAVIGATE,
                    x: dx,
                    y: dy,
                    button: 0,
                    pressed: false,
                });
            }
        }
        self.last_up = inputs.up;
        self.last_down = inputs.down;
        self.last_left = inputs.left;
        self.last_right = inputs.right_dpad;

        if inputs.a && !self.last_a {
            commands.push(WireCommand {
                command: FORMAT_COMMAND_POINTER,
                x: 0,
                y: 0,
                button: FORMAT_POINTER_RIGHT,
                pressed: true,
            });
        } else if !inputs.a && self.last_a {
            commands.push(WireCommand {
                command: FORMAT_COMMAND_POINTER,
                x: 0,
                y: 0,
                button: FORMAT_POINTER_RIGHT,
                pressed: false,
            });
        }
        self.last_a = inputs.a;

        if inputs.b && !self.last_b {
            commands.push(WireCommand {
                command: FORMAT_COMMAND_MENU_CANCEL,
                x: 0,
                y: 0,
                button: 0,
                pressed: true,
            });
        }
        self.last_b = inputs.b;

        if inputs.back && !self.last_back {
            commands.push(WireCommand {
                command: FORMAT_COMMAND_OVERLAY_CANCEL,
                x: 0,
                y: 0,
                button: 0,
                pressed: true,
            });
        }
        self.last_back = inputs.back;

        if inputs.start && !self.last_start {
            commands.push(WireCommand {
                command: FORMAT_COMMAND_MENU_TOGGLE,
                x: 0,
                y: 0,
                button: 0,
                pressed: true,
            });
        }
        self.last_start = inputs.start;

        if inputs.r3_checked != self.last_r3_checked {
            commands.push(WireCommand {
                command: FORMAT_COMMAND_TOGGLE_ARCBALL,
                x: 0,
                y: 0,
                button: 0,
                pressed: true,
            });
            self.last_r3_checked = inputs.r3_checked;
        }

        commands
    }
}

pub(crate) struct CommandSender {
    writer: Option<DataWriter<FormatManagerInputTopic>>,
    display_id: u32,
    state: CommandState,
}

impl CommandSender {
    pub(crate) fn new(display_id: u32, domain: u32, config: Option<&Path>) -> Self {
        Self {
            writer: open_writer(domain, config),
            display_id,
            state: CommandState::default(),
        }
    }

    pub(crate) fn tick(&mut self, ui: &MainWindow) {
        let inputs = Inputs {
            right: (ui.get_right_x(), ui.get_right_y()),
            up: ui.get_up_down(),
            down: ui.get_down_down(),
            left: ui.get_left_down(),
            right_dpad: ui.get_right_down(),
            a: ui.get_a_down(),
            b: ui.get_b_down(),
            back: ui.get_back_down(),
            start: ui.get_start_down(),
            r3_checked: ui.get_r3_checked(),
        };
        for command in self.state.update(inputs, Instant::now()) {
            self.send(command);
        }
    }

    fn send(&self, command: WireCommand) {
        let Some(writer) = &self.writer else {
            return;
        };
        let sample = FormatManagerInputTopic {
            display_id: self.display_id,
            window: 0,
            command: command.command,
            format: String::new(),
            x: command.x,
            y: command.y,
            button: command.button,
            pressed: command.pressed,
        };
        if let Err(error) = writer.write(&sample) {
            eprintln!("paws: command write failed: {error}");
        }
    }
}

fn open_writer(domain: u32, config: Option<&Path>) -> Option<DataWriter<FormatManagerInputTopic>> {
    let dir = contracts_dir(config);
    let registry = match TopicRegistry::load(&dir.join("topics.toml")) {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("paws: contract load failed: {error}");
            return None;
        }
    };
    if let Err(error) = registry.validate() {
        eprintln!("paws: contract validation failed: {error}");
        return None;
    }
    let contract = match ServiceContract::load(&dir.join("services/paws.toml")) {
        Ok(contract) => contract,
        Err(error) => {
            eprintln!("paws: service contract load failed: {error}");
            return None;
        }
    };
    if let Err(error) = contract.validate(&registry) {
        eprintln!("paws: service contract validation failed: {error}");
        return None;
    }
    let node = match Node::new(domain, registry) {
        Ok(node) => node,
        Err(error) => {
            eprintln!("paws: DDS participant failed: {error}");
            return None;
        }
    };
    match node.writer::<FormatManagerInputTopic>("FormatManagerInputTopic") {
        Ok(writer) => Some(writer),
        Err(error) => {
            eprintln!("paws: command writer failed: {error}");
            None
        }
    }
}

fn contracts_dir(explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }
    let deployed = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .and_then(|bin| bin.parent().map(|deploy| deploy.join("config")));
    if let Some(dir) = &deployed {
        if dir.join("topics.toml").is_file() {
            return dir.clone();
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("contracts")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> Inputs {
        Inputs::default()
    }

    #[test]
    fn neutral_input_sends_nothing() {
        let mut state = CommandState::default();
        assert!(state.update(inputs(), Instant::now()).is_empty());
    }

    #[test]
    fn cursor_moves_are_throttled() {
        let mut state = CommandState::default();
        let now = Instant::now();
        let mut held = inputs();
        held.right = (1.0, 0.0);
        assert_eq!(
            state.update(held, now),
            vec![WireCommand {
                command: FORMAT_COMMAND_CURSOR_MOVE,
                x: 20,
                y: 0,
                button: 0,
                pressed: false,
            }]
        );
        assert!(state
            .update(held, now + Duration::from_millis(10))
            .is_empty());
        assert_eq!(
            state.update(held, now + CURSOR_INTERVAL),
            vec![WireCommand {
                command: FORMAT_COMMAND_CURSOR_MOVE,
                x: 20,
                y: 0,
                button: 0,
                pressed: false,
            }]
        );
    }

    #[test]
    fn dpad_sends_only_on_press_edges() {
        let mut state = CommandState::default();
        let now = Instant::now();
        let mut pressed = inputs();
        pressed.right_dpad = true;
        assert_eq!(
            state.update(pressed, now),
            vec![WireCommand {
                command: FORMAT_COMMAND_NAVIGATE,
                x: 1,
                y: 0,
                button: 0,
                pressed: false,
            }]
        );
        assert!(state.update(pressed, now).is_empty());
        assert!(state.update(inputs(), now).is_empty());
        assert_eq!(state.update(pressed, now).len(), 1);
    }

    #[test]
    fn r3_checkbox_sends_a_toggle_on_each_change() {
        let mut state = CommandState::default();
        let now = Instant::now();
        let mut checked = inputs();
        checked.r3_checked = true;
        assert_eq!(
            state.update(checked, now),
            vec![WireCommand {
                command: FORMAT_COMMAND_TOGGLE_ARCBALL,
                x: 0,
                y: 0,
                button: 0,
                pressed: true,
            }]
        );
        assert!(state.update(checked, now).is_empty());
        assert_eq!(state.update(inputs(), now).len(), 1);
    }

    #[test]
    fn b_and_start_send_menu_commands_on_press_edges() {
        let mut state = CommandState::default();
        let now = Instant::now();
        let mut b = inputs();
        b.b = true;
        assert_eq!(
            state.update(b, now),
            vec![WireCommand {
                command: FORMAT_COMMAND_MENU_CANCEL,
                x: 0,
                y: 0,
                button: 0,
                pressed: true,
            }]
        );
        assert!(state.update(b, now).is_empty());
        assert!(state.update(inputs(), now).is_empty());

        let mut back = inputs();
        back.back = true;
        assert_eq!(
            state.update(back, now),
            vec![WireCommand {
                command: FORMAT_COMMAND_OVERLAY_CANCEL,
                x: 0,
                y: 0,
                button: 0,
                pressed: true,
            }]
        );
        assert!(state.update(back, now).is_empty());
        assert!(state.update(inputs(), now).is_empty());

        let mut start = inputs();
        start.start = true;
        assert_eq!(
            state.update(start, now),
            vec![WireCommand {
                command: FORMAT_COMMAND_MENU_TOGGLE,
                x: 0,
                y: 0,
                button: 0,
                pressed: true,
            }]
        );
        assert!(state.update(start, now).is_empty());
        assert!(state.update(inputs(), now).is_empty());
    }

    #[test]
    fn a_sends_press_and_release_edges() {
        let mut state = CommandState::default();
        let now = Instant::now();
        let mut pressed = inputs();
        pressed.a = true;
        assert_eq!(
            state.update(pressed, now),
            vec![WireCommand {
                command: FORMAT_COMMAND_POINTER,
                x: 0,
                y: 0,
                button: FORMAT_POINTER_RIGHT,
                pressed: true,
            }]
        );
        assert!(state.update(pressed, now).is_empty());
        assert_eq!(
            state.update(inputs(), now),
            vec![WireCommand {
                command: FORMAT_COMMAND_POINTER,
                x: 0,
                y: 0,
                button: FORMAT_POINTER_RIGHT,
                pressed: false,
            }]
        );
    }
}
