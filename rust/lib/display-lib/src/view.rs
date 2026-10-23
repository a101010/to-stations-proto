//! The display library's DDS subscription: the latest hello_world view sample.
//!
//! The library subscribes to `HelloWorldViewTopic` on the given domain and keeps
//! the latest received greeting and attitude in shared state. A format that
//! needs view data renders black until the first sample arrives.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use stations_dds::contract::TopicRegistry;
use stations_dds::cyclonedds::DataReader;
use stations_dds::dds::Node;
use stations_dds::hello_world::HelloWorldViewTopic;

/// The latest view sample received from the view service.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct View {
    pub greeting: String,
    pub attitude: [f32; 4],
}

/// Shared latest view; `None` until the first sample arrives.
pub(crate) type ViewState = Rc<RefCell<Option<View>>>;

/// A new, empty view state.
pub(crate) fn new_state() -> ViewState {
    Rc::new(RefCell::new(None))
}

/// A `HelloWorldViewTopic` reader on one domain.
pub(crate) struct ViewSource {
    // Kept alive for the reader's lifetime.
    _node: Node,
    reader: DataReader<HelloWorldViewTopic>,
}

impl ViewSource {
    /// Create the participant and reader. The registry is loaded from
    /// `config_dir/topics.toml`.
    pub(crate) fn new(domain: u32, config_dir: &Path) -> Result<Self, String> {
        let registry =
            TopicRegistry::load(&config_dir.join("topics.toml")).map_err(|e| e.to_string())?;
        let node = Node::new(domain, registry).map_err(|e| e.to_string())?;
        let reader = node
            .reader::<HelloWorldViewTopic>("HelloWorldViewTopic")
            .map_err(|e| e.to_string())?;
        Ok(Self {
            _node: node,
            reader,
        })
    }

    /// Drain the reader and store the latest sample.
    pub(crate) fn poll(&self, state: &ViewState) {
        if let Ok(samples) = self.reader.take() {
            if let Some(sample) = samples.into_iter().last() {
                let q = sample.attitude;
                *state.borrow_mut() = Some(View {
                    greeting: sample.greeting,
                    attitude: [q.x as f32, q.y as f32, q.z as f32, q.w as f32],
                });
            }
        }
    }
}
