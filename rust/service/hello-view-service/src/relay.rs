//! The relay state between the model stream and the view stream.

use std::time::{Duration, Instant};

use stations_dds::hello_world::{HelloWorldTopic, HelloWorldViewTopic};
use stations_dds::math::Quaternion;

const TIMEOUT: Duration = Duration::from_secs(2);

/// Keeps the latest model sample and decides what view sample to publish.
pub(crate) struct ViewRelay {
    started: Instant,
    latest: Option<HelloWorldTopic>,
    last_received: Option<Instant>,
}

impl ViewRelay {
    pub(crate) fn new(started: Instant) -> Self {
        Self {
            started,
            latest: None,
            last_received: None,
        }
    }

    pub(crate) fn update(&mut self, sample: HelloWorldTopic, now: Instant) {
        self.latest = Some(sample);
        self.last_received = Some(now);
    }

    /// The view sample to publish at `now`.
    ///
    /// Before the first model sample, nothing is published for two seconds.
    /// After that, the NoConn greeting is published. Once a model sample
    /// arrives, it is relayed as valid until two seconds pass; after that the
    /// last attitude is retained and the greeting reports the elapsed time.
    pub(crate) fn view(&self, now: Instant) -> Option<HelloWorldViewTopic> {
        let reference = self.last_received.unwrap_or(self.started);
        let elapsed = now.saturating_duration_since(reference);

        if let Some(sample) = &self.latest {
            if elapsed < TIMEOUT {
                return Some(HelloWorldViewTopic {
                    greeting: sample.greeting.clone(),
                    attitude: sample.attitude.clone(),
                    valid: true,
                });
            }
            return Some(HelloWorldViewTopic {
                greeting: no_conn(elapsed),
                attitude: sample.attitude.clone(),
                valid: false,
            });
        }

        if elapsed >= TIMEOUT {
            return Some(HelloWorldViewTopic {
                greeting: no_conn(elapsed),
                attitude: identity(),
                valid: false,
            });
        }
        None
    }
}

fn no_conn(elapsed: Duration) -> String {
    let total = elapsed.as_secs();
    format!(
        "NoConn {:02}:{:02}:{:02}",
        total / 3600,
        (total / 60) % 60,
        total % 60
    )
}

fn identity() -> Quaternion {
    Quaternion {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(greeting: &str, w: f64) -> HelloWorldTopic {
        HelloWorldTopic {
            greeting: greeting.to_string(),
            attitude: Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w,
            },
        }
    }

    #[test]
    fn no_view_before_the_startup_timeout() {
        let base = Instant::now();
        let relay = ViewRelay::new(base);
        assert!(relay.view(base + Duration::from_secs(1)).is_none());

        let view = relay
            .view(base + Duration::from_secs(2))
            .expect("NoConn view");
        assert!(!view.valid);
        assert_eq!(view.greeting, "NoConn 00:00:02");
        assert_eq!(view.attitude, identity());
    }

    #[test]
    fn relays_a_fresh_sample_as_valid() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base);
        relay.update(sample("Hello, world!", 0.5), base + Duration::from_secs(1));

        let view = relay
            .view(base + Duration::from_secs(2))
            .expect("valid view");
        assert!(view.valid);
        assert_eq!(view.greeting, "Hello, world!");
        assert_eq!(view.attitude.w, 0.5);
    }

    #[test]
    fn timeout_keeps_the_last_attitude_and_counts() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base);
        relay.update(sample("Hola, mundo!", 0.25), base);

        let view = relay
            .view(base + Duration::from_secs(2))
            .expect("NoConn view");
        assert!(!view.valid);
        assert_eq!(view.greeting, "NoConn 00:00:02");
        assert_eq!(view.attitude.w, 0.25);

        let later = relay
            .view(base + Duration::from_secs(65))
            .expect("NoConn view");
        assert_eq!(later.greeting, "NoConn 00:01:05");
        assert_eq!(later.attitude.w, 0.25);
    }

    #[test]
    fn recovers_when_a_new_sample_arrives() {
        let base = Instant::now();
        let mut relay = ViewRelay::new(base);
        relay.update(sample("Hola, mundo!", 0.25), base);
        relay.update(sample("Bonjour le monde!", 0.75), base + Duration::from_secs(3));

        let view = relay
            .view(base + Duration::from_secs(3))
            .expect("valid view");
        assert!(view.valid);
        assert_eq!(view.greeting, "Bonjour le monde!");
        assert_eq!(view.attitude.w, 0.75);
    }
}
