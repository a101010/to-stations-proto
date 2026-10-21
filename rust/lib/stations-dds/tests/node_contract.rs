//! End-to-end test of the contract-aware [`Node`]: a bad topic fails through
//! the public writer/reader path, not just in the internal check.
//!
//! Because the check runs before any topic is created, this needs a participant
//! (and so the `StationsDDS` adapter) but no discovery or round-trip.

use std::path::PathBuf;
use std::sync::Once;

use stations_dds::contract::TopicRegistry;
use stations_dds::dds::{Error, Node};
use stations_dds::{hello_world::HelloWorldTopic, math::Quaternion};

static INIT: Once = Once::new();

fn repo_root() -> PathBuf {
    PathBuf::from(env!("REPO_ROOT"))
}

fn init() {
    INIT.call_once(|| {
        std::env::set_var(
            "CYCLONEDDS_URI",
            format!("file://{}/DDS/cyclonedds-config.xml", repo_root().display()),
        );
    });
}

#[test]
fn node_rejects_bad_topics() {
    init();

    let registry = TopicRegistry::load(&repo_root().join("contracts/topics.toml")).unwrap();
    let node = Node::new(0, registry).unwrap();

    assert!(matches!(
        node.writer::<HelloWorldTopic>("NotRegistered"),
        Err(Error::UnregisteredTopic(t)) if t == "NotRegistered"
    ));
    assert!(matches!(
        node.writer::<Quaternion>("HelloWorldTopic"),
        Err(Error::TypeMismatch { .. })
    ));
    assert!(node.writer::<HelloWorldTopic>("HelloWorldTopic").is_ok());
}
