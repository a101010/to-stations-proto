//! The display library's hello_world DDS endpoints.
//!
//! The library subscribes to `HelloWorldViewTopic` on the given domain and keeps
//! the latest sample for this display in shared state. It also publishes local
//! input on `HelloWorldInputTopic`; the per-display controller relays that to
//! the view service.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use format_list::{HELLO_MODE_AUTO, HELLO_OVERLAY_NONE};
use stations_dds::cyclonedds::{DataReader, DataWriter};
use stations_dds::dds::Node;
use stations_dds::hello_world::{HelloWorldInputTopic, HelloWorldViewTopic};

/// The latest view sample received for this display.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct View {
    pub greeting: String,
    /// The last valid attitude; `None` until the first valid sample.
    pub attitude: Option<[f32; 4]>,
    pub valid: bool,
    pub mode: i32,
    pub arcball: bool,
    pub light: [f32; 3],
    pub color: [f32; 3],
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
    pub overlay: i32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            greeting: String::new(),
            attitude: None,
            valid: false,
            mode: HELLO_MODE_AUTO,
            arcball: false,
            light: [0.6, 0.8, 0.7],
            color: [1.0, 1.0, 1.0],
            hue: 0.0,
            saturation: 0.0,
            value: 1.0,
            overlay: HELLO_OVERLAY_NONE,
        }
    }
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

    /// Drain the reader and store the latest sample for `display_id`. The
    /// attitude is retained from the last valid sample so an invalid sample
    /// cannot erase the last received position.
    pub(crate) fn poll(&self, state: &ViewState, display_id: Option<u32>) {
        if let Ok(samples) = self.reader.take() {
            if let Some(sample) = latest_matching(samples, display_id) {
                let q = sample.attitude;
                let attitude = [q.x as f32, q.y as f32, q.z as f32, q.w as f32];
                if sample.valid {
                    self.last_attitude.set(Some(attitude));
                }
                *state.borrow_mut() = Some(View {
                    greeting: sample.greeting,
                    attitude: self.last_attitude.get(),
                    valid: sample.valid,
                    mode: sample.mode,
                    arcball: sample.arcball,
                    light: [
                        sample.light.x as f32,
                        sample.light.y as f32,
                        sample.light.z as f32,
                    ],
                    color: [
                        sample.color.r as f32,
                        sample.color.g as f32,
                        sample.color.b as f32,
                    ],
                    hue: sample.hue as f32,
                    saturation: sample.saturation as f32,
                    value: sample.value as f32,
                    overlay: sample.overlay,
                });
            }
        }
    }
}

/// The latest view sample for `display_id`, ignoring other displays and an
/// unset identity.
fn latest_matching(
    samples: Vec<HelloWorldViewTopic>,
    display_id: Option<u32>,
) -> Option<HelloWorldViewTopic> {
    let display_id = display_id?;
    samples
        .into_iter()
        .filter(|sample| sample.display_id == display_id)
        .last()
}

/// A `HelloWorldInputTopic` writer stamped with this display's id.
#[derive(Clone)]
pub(crate) struct HelloWorldInput {
    inner: Rc<InputInner>,
}

struct InputInner {
    writer: DataWriter<HelloWorldInputTopic>,
    display_id: Cell<u32>,
}

impl HelloWorldInput {
    pub(crate) fn new(node: &Node) -> Result<Self, String> {
        let writer = node
            .writer::<HelloWorldInputTopic>("HelloWorldInputTopic")
            .map_err(|e| e.to_string())?;
        Ok(Self {
            inner: Rc::new(InputInner {
                writer,
                display_id: Cell::new(0),
            }),
        })
    }

    pub(crate) fn set_display_id(&self, display_id: u32) {
        self.inner.display_id.set(display_id);
    }

    pub(crate) fn send(&self, action: i32, x: f64, y: f64, z: f64, pressed: bool) {
        let sample = HelloWorldInputTopic {
            display_id: self.inner.display_id.get(),
            action,
            x,
            y,
            z,
            pressed,
        };
        if let Err(error) = self.inner.writer.write(&sample) {
            eprintln!("display-lib: hello input write failed: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use format_list::{HELLO_MODE_INSPECT, HELLO_OVERLAY_NONE};
    use stations_dds::hello_world::HelloWorldViewTopic;
    use stations_dds::math::{Color, Quaternion, Vector3};

    fn view(display_id: u32, greeting: &str) -> HelloWorldViewTopic {
        HelloWorldViewTopic {
            display_id,
            greeting: greeting.to_string(),
            attitude: Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            valid: true,
            mode: HELLO_MODE_INSPECT,
            arcball: true,
            light: Vector3 {
                x: 0.1,
                y: 0.2,
                z: 0.3,
            },
            color: Color {
                r: 0.4,
                g: 0.5,
                b: 0.6,
            },
            hue: 0.25,
            saturation: 0.5,
            value: 0.75,
            overlay: HELLO_OVERLAY_NONE,
        }
    }

    #[test]
    fn latest_matching_filters_by_display() {
        assert!(latest_matching(vec![view(0, "zero")], None).is_none());
        let samples = vec![view(0, "zero"), view(1, "one"), view(0, "newer")];
        assert_eq!(
            latest_matching(samples, Some(0)).map(|sample| sample.greeting),
            Some("newer".to_string())
        );
    }

    #[test]
    fn view_defaults_are_auto_and_white() {
        let view = View::default();
        assert_eq!(view.mode, HELLO_MODE_AUTO);
        assert!(!view.arcball);
        assert_eq!(view.color, [1.0, 1.0, 1.0]);
        assert_eq!(view.saturation, 0.0);
        assert_eq!(view.value, 1.0);
        assert_eq!(view.overlay, HELLO_OVERLAY_NONE);
    }

    #[test]
    fn view_sample_maps_every_field() {
        let sample = view(2, "hello");
        let state = new_state();
        *state.borrow_mut() = Some(View {
            greeting: sample.greeting.clone(),
            attitude: Some([0.0, 0.0, 0.0, 1.0]),
            valid: sample.valid,
            mode: sample.mode,
            arcball: sample.arcball,
            light: [
                sample.light.x as f32,
                sample.light.y as f32,
                sample.light.z as f32,
            ],
            color: [
                sample.color.r as f32,
                sample.color.g as f32,
                sample.color.b as f32,
            ],
            hue: sample.hue as f32,
            saturation: sample.saturation as f32,
            value: sample.value as f32,
            overlay: sample.overlay,
        });
        let view = state.borrow().clone().unwrap();
        assert_eq!(view.mode, HELLO_MODE_INSPECT);
        assert!(view.arcball);
        assert_eq!(view.light, [0.1, 0.2, 0.3]);
        assert_eq!(view.color, [0.4, 0.5, 0.6]);
        assert_eq!(view.hue, 0.25);
        assert_eq!(view.saturation, 0.5);
        assert_eq!(view.value, 0.75);
    }
}
