//! The display library's format-manager DDS endpoints.
//!
//! It subscribes to `FormatManagerViewTopic` assignments and publishes logical
//! format events on `FormatManagerInputTopic`. The display application knows
//! nothing about these topics.

use format_list::FORMAT_COMMAND_NEXT;
use stations_dds::cyclonedds::{DataReader, DataWriter};
use stations_dds::dds::Node;
use stations_dds::format_manager::{FormatManagerInputTopic, FormatManagerViewTopic};

use crate::format_manager::FormatManager;

pub(crate) struct FormatSource {
    reader: DataReader<FormatManagerViewTopic>,
    writer: DataWriter<FormatManagerInputTopic>,
}

impl FormatSource {
    pub(crate) fn new(node: &Node) -> Result<Self, String> {
        let reader = node
            .reader::<FormatManagerViewTopic>("FormatManagerViewTopic")
            .map_err(|e| e.to_string())?;
        let writer = node
            .writer::<FormatManagerInputTopic>("FormatManagerInputTopic")
            .map_err(|e| e.to_string())?;
        Ok(Self { reader, writer })
    }

    /// Apply the latest view assignment for `display_id`, if one arrived.
    pub(crate) fn poll(&self, manager: &mut FormatManager, display_id: &str) {
        if let Ok(samples) = self.reader.take() {
            if let Some(sample) = latest_matching(samples, display_id) {
                manager.set_assignments(sample.windows.as_slice());
            }
        }
    }

    /// Publish a logical `Next` event for a format window.
    pub(crate) fn send_next(&self, window: i32, display_id: &str) {
        if display_id.is_empty() {
            return;
        }
        let event = FormatManagerInputTopic {
            display_id: display_id.to_string(),
            window,
            command: FORMAT_COMMAND_NEXT,
            format: String::new(),
        };
        if let Err(error) = self.writer.write(&event) {
            eprintln!("display-lib: format input write failed: {error}");
        }
    }
}

/// The latest sample addressed to `display_id`. An empty id matches nothing.
fn latest_matching(
    samples: Vec<FormatManagerViewTopic>,
    display_id: &str,
) -> Option<FormatManagerViewTopic> {
    if display_id.is_empty() {
        return None;
    }
    samples
        .into_iter()
        .filter(|sample| sample.display_id == display_id)
        .last()
}

#[cfg(test)]
mod tests {
    use super::*;
    use stations_dds::cyclonedds::DdsSequence;
    use stations_dds::format_manager::FormatWindowAssignment;

    fn view(display_id: &str, format: &str) -> FormatManagerViewTopic {
        FormatManagerViewTopic {
            display_id: display_id.to_string(),
            windows: DdsSequence::from_vec(vec![FormatWindowAssignment {
                window: 0,
                format: format.to_string(),
            }])
            .unwrap(),
        }
    }

    #[test]
    fn latest_matching_ignores_other_displays() {
        let samples = vec![
            view("lil-d-a", "hello-world"),
            view("lil-d-b", "default"),
            view("lil-d-a", "default"),
        ];
        let sample = latest_matching(samples, "lil-d-a").expect("matching sample");
        assert_eq!(sample.display_id, "lil-d-a");
        assert_eq!(sample.windows.as_slice()[0].format, "default");
    }

    #[test]
    fn latest_matching_ignores_empty_identity() {
        assert!(latest_matching(vec![view("", "default")], "").is_none());
    }
}
