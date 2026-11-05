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

    /// Apply the latest view assignment, if one arrived.
    pub(crate) fn poll(&self, manager: &mut FormatManager) {
        if let Ok(samples) = self.reader.take() {
            if let Some(sample) = samples.into_iter().last() {
                manager.set_assignments(sample.windows.as_slice());
            }
        }
    }

    /// Publish a logical `Next` event for a format window.
    pub(crate) fn send_next(&self, window: i32) {
        let event = FormatManagerInputTopic {
            window,
            command: FORMAT_COMMAND_NEXT,
            format: String::new(),
        };
        if let Err(error) = self.writer.write(&event) {
            eprintln!("display-lib: format input write failed: {error}");
        }
    }
}
