//! The display library's format-manager DDS endpoints.
//!
//! It subscribes to `FormatManagerViewTopic` assignments and publishes logical
//! format events on `FormatManagerInputTopic`. The display application knows
//! nothing about these topics.

use format_list::{
    FORMAT_COMMAND_CURSOR_MOVE, FORMAT_COMMAND_NAVIGATE, FORMAT_COMMAND_POINTER,
    FORMAT_COMMAND_SELECT,
};
use stations_dds::cyclonedds::{DataReader, DataWriter};
use stations_dds::dds::Node;
use stations_dds::format_manager::{
    FormatManagerCommandCtlTopic, FormatManagerInputTopic, FormatManagerViewTopic,
};

use crate::format_manager::{FormatAction, FormatManager};
use crate::formats::FormatCommand;

pub(crate) struct FormatSource {
    reader: DataReader<FormatManagerViewTopic>,
    command_reader: Option<DataReader<FormatManagerCommandCtlTopic>>,
    writer: DataWriter<FormatManagerInputTopic>,
}

impl FormatSource {
    pub(crate) fn new(node: &Node) -> Result<Self, String> {
        let reader = node
            .reader::<FormatManagerViewTopic>("FormatManagerViewTopic")
            .map_err(|e| e.to_string())?;
        let command_reader = node
            .reader::<FormatManagerCommandCtlTopic>("FormatManagerCommandCtlTopic")
            .ok();
        let writer = node
            .writer::<FormatManagerInputTopic>("FormatManagerInputTopic")
            .map_err(|e| e.to_string())?;
        Ok(Self {
            reader,
            command_reader,
            writer,
        })
    }

    /// Apply the latest view assignment for `display_id`, if one arrived.
    pub(crate) fn poll(&self, manager: &mut FormatManager, display_id: Option<u32>) {
        if let Ok(samples) = self.reader.take() {
            if let Some(display_id) = display_id {
                if let Some(sample) = latest_matching(samples, display_id) {
                    manager.set_assignments(sample.windows.as_slice());
                }
            }
        }

        let Some(display_id) = display_id else {
            return;
        };
        let Some(reader) = &self.command_reader else {
            return;
        };
        if let Ok(samples) = reader.take() {
            for sample in samples {
                if sample.display_id != display_id {
                    continue;
                }
                let Some(command) = to_command(&sample) else {
                    continue;
                };
                if let Some(FormatAction::Select { window, format }) =
                    manager.command(sample.window, command)
                {
                    self.send_select(window, &format, Some(display_id));
                }
            }
        }
    }

    /// Publish a logical `Select` event for a format window.
    pub(crate) fn send_select(&self, window: i32, format: &str, display_id: Option<u32>) {
        self.send(
            window,
            FORMAT_COMMAND_SELECT,
            format.to_string(),
            display_id,
        );
    }

    fn send(&self, window: i32, command: i32, format: String, display_id: Option<u32>) {
        let Some(display_id) = display_id else {
            return;
        };
        let event = FormatManagerInputTopic {
            display_id,
            window,
            command,
            format,
            x: 0,
            y: 0,
            button: 0,
            pressed: false,
        };
        if let Err(error) = self.writer.write(&event) {
            eprintln!("display-lib: format input write failed: {error}");
        }
    }
}

/// Convert a command-control sample into the display-library command.
fn to_command(sample: &FormatManagerCommandCtlTopic) -> Option<FormatCommand> {
    match sample.command {
        FORMAT_COMMAND_CURSOR_MOVE => Some(FormatCommand::CursorMove {
            dx: sample.x,
            dy: sample.y,
        }),
        FORMAT_COMMAND_NAVIGATE => Some(FormatCommand::Navigate {
            dx: sample.x,
            dy: sample.y,
        }),
        FORMAT_COMMAND_POINTER => Some(FormatCommand::Pointer {
            button: sample.button,
            pressed: sample.pressed,
        }),
        _ => None,
    }
}

/// The latest sample addressed to `display_id`.
fn latest_matching(
    samples: Vec<FormatManagerViewTopic>,
    display_id: u32,
) -> Option<FormatManagerViewTopic> {
    samples
        .into_iter()
        .filter(|sample| sample.display_id == display_id)
        .last()
}

#[cfg(test)]
mod tests {
    use super::*;
    use stations_dds::cyclonedds::DdsSequence;
    use stations_dds::format_manager::{FormatManagerCommandCtlTopic, FormatWindowAssignment};

    fn view(display_id: u32, format: &str) -> FormatManagerViewTopic {
        FormatManagerViewTopic {
            display_id,
            windows: DdsSequence::from_vec(vec![FormatWindowAssignment {
                window: 0,
                format: format.to_string(),
            }])
            .unwrap(),
        }
    }

    #[test]
    fn maps_control_commands() {
        let sample = FormatManagerCommandCtlTopic {
            display_id: 0,
            window: 0,
            command: FORMAT_COMMAND_NAVIGATE,
            x: -1,
            y: 0,
            button: 0,
            pressed: false,
        };
        assert_eq!(
            to_command(&sample),
            Some(FormatCommand::Navigate { dx: -1, dy: 0 })
        );

        let sample = FormatManagerCommandCtlTopic {
            command: FORMAT_COMMAND_POINTER,
            button: 1,
            pressed: true,
            ..sample
        };
        assert_eq!(
            to_command(&sample),
            Some(FormatCommand::Pointer {
                button: 1,
                pressed: true
            })
        );
    }

    #[test]
    fn latest_matching_ignores_other_displays() {
        let samples = vec![
            view(0, "hello-world"),
            view(1, "default"),
            view(0, "default"),
        ];
        let sample = latest_matching(samples, 0).expect("matching sample");
        assert_eq!(sample.display_id, 0);
        assert_eq!(sample.windows.as_slice()[0].format, "default");
    }
}
