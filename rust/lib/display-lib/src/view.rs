//! The display library's DDS subscription: the latest hello_world view sample.
//!
//! The library subscribes to `HelloWorldViewTopic` on the given domain and keeps
//! the latest received greeting and attitude in shared state. A format that
//! needs view data renders black until the first sample arrives.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use stations_dds::cyclonedds::DataReader;
use stations_dds::dds::Node;
use stations_dds::hello_world::HelloWorldViewTopic;

/// The latest view sample received from the view service.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct View {
    pub greeting: String,
    /// The last valid attitude; `None` until the first valid sample.
    pub attitude: Option<[f32; 4]>,
    pub valid: bool,
}

/// Shared latest view; `None` until the first sample arrives.
pub(crate) type ViewState = Rc<RefCell<Option<View>>>;

/// A new, empty view state.
pub(crate) fn new_state() -> ViewState {
    Rc::new(RefCell::new(None))
}

/// A `HelloWorldViewTopic` reader on the display library's shared participant.
pub(crate) struct ViewSource {
    reader: DataReader<HelloWorldViewTopic>,
    last_attitude: Cell<Option<[f32; 4]>>,
}

impl ViewSource {
    pub(crate) fn new(node: &Node) -> Result<Self, String> {
        let reader = node
            .reader::<HelloWorldViewTopic>("HelloWorldViewTopic")
            .map_err(|e| e.to_string())?;
        Ok(Self {
            reader,
            last_attitude: Cell::new(None),
        })
    }

    /// Drain the reader and store the latest sample. The attitude is retained
    /// from the last valid sample so an invalid sample cannot erase the last
    /// received position.
    pub(crate) fn poll(&self, state: &ViewState) {
        if let Ok(samples) = self.reader.take() {
            if let Some(sample) = samples.into_iter().last() {
                let q = sample.attitude;
                let attitude = [q.x as f32, q.y as f32, q.z as f32, q.w as f32];
                if sample.valid {
                    self.last_attitude.set(Some(attitude));
                }
                *state.borrow_mut() = Some(View {
                    greeting: sample.greeting,
                    attitude: self.last_attitude.get(),
                    valid: sample.valid,
                });
            }
        }
    }
}
