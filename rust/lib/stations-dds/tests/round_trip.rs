//! End-to-end typed publish/subscribe round-trip.
//!
//! Requires the `StationsDDS` loopback adapter and the shared configuration;
//! see `docs/loopback-adapter.md`. The test points CycloneDDS at
//! `DDS/cyclonedds-config.xml` and retries writes until the reader (on the same
//! participant) receives a sample.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use stations_dds::contract::TopicRegistry;
use stations_dds::dds::Node;
use stations_dds::{hello_world::HelloWorldTopic, math::Quaternion};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("REPO_ROOT"))
}

#[test]
fn round_trip() {
    std::env::set_var(
        "CYCLONEDDS_URI",
        format!("file://{}/DDS/cyclonedds-config.xml", repo_root().display()),
    );

    let registry = TopicRegistry::load(&repo_root().join("contracts/topics.toml")).unwrap();
    let node = Node::new(0, registry).unwrap();
    let writer = node.writer::<HelloWorldTopic>("HelloWorldTopic").unwrap();
    let reader = node.reader::<HelloWorldTopic>("HelloWorldTopic").unwrap();

    let sample = HelloWorldTopic {
        greeting: "Hello, world!".to_string(),
        attitude: Quaternion {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        },
    };

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        writer.write(&sample).unwrap();
        if let Some(received) = reader.take().unwrap().into_iter().next() {
            assert_eq!(received, sample);
            return;
        }
        assert!(Instant::now() < deadline, "no sample received");
        std::thread::sleep(Duration::from_millis(50));
    }
}
