//! The DDS layer: a contract-aware [`Node`] over a CycloneDDS participant.
//!
//! `Node` holds a validated [`TopicRegistry`]; every writer/reader it creates
//! must name a registered topic and carry the registry's type, so an
//! unregistered or mis-typed use of DDS fails at endpoint creation. CycloneDDS
//! applies the shared configuration (`DDS/cyclonedds-config.xml`) through the
//! `CYCLONEDDS_URI` environment variable, which the launch scripts set.

use cyclonedds::{DataReader, DataWriter, DdsType, DomainParticipant, Qos};

use crate::contract::{ContractError, TopicDef, TopicRegistry};

/// A [`Node`] or endpoint failure.
#[derive(Debug)]
pub enum Error {
    /// The underlying CycloneDDS call failed.
    Dds(cyclonedds::DdsError),
    /// The contract was invalid.
    Contract(ContractError),
    /// The requested topic is not in the registry.
    UnregisteredTopic(String),
    /// The requested type does not match the registry entry's `type`.
    TypeMismatch {
        topic: String,
        expected: String,
        found: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Dds(e) => write!(f, "dds error: {e}"),
            Error::Contract(e) => write!(f, "contract error: {e}"),
            Error::UnregisteredTopic(t) => write!(f, "unregistered topic: {t}"),
            Error::TypeMismatch {
                topic,
                expected,
                found,
            } => write!(
                f,
                "topic `{topic}` is registered as `{expected}` but `{found}` was requested"
            ),
        }
    }
}

impl std::error::Error for Error {}

impl From<cyclonedds::DdsError> for Error {
    fn from(e: cyclonedds::DdsError) -> Self {
        Error::Dds(e)
    }
}

impl From<ContractError> for Error {
    fn from(e: ContractError) -> Self {
        Error::Contract(e)
    }
}

/// A DDS participant bound to a validated contract.
pub struct Node {
    participant: DomainParticipant,
    registry: TopicRegistry,
}

impl Node {
    /// Create the participant and validate the registry. CycloneDDS reads its
    /// configuration from `CYCLONEDDS_URI`.
    pub fn new(domain: u32, registry: TopicRegistry) -> Result<Self, Error> {
        registry.validate()?;
        let participant = DomainParticipant::new(domain)?;
        Ok(Self {
            participant,
            registry,
        })
    }

    /// The underlying participant.
    pub fn participant(&self) -> &DomainParticipant {
        &self.participant
    }

    /// The validated registry.
    pub fn registry(&self) -> &TopicRegistry {
        &self.registry
    }

    /// A writer for `topic`: it must be registered, `T::type_name()` must equal
    /// the entry's `type`, and the entry's QoS is used.
    pub fn writer<T: DdsType>(&self, topic: &str) -> Result<DataWriter<T>, Error> {
        let entry = check::<T>(&self.registry, topic)?;
        let qos = qos(&entry.qos).ok_or_else(|| unknown_qos(entry))?;
        let dds_topic = self.participant.create_topic::<T>(topic)?;
        let publisher = self.participant.create_publisher()?;
        Ok(publisher.create_writer_with_qos::<T>(&dds_topic, &qos)?)
    }

    /// A reader for `topic`; the same checks as [`Node::writer`].
    pub fn reader<T: DdsType>(&self, topic: &str) -> Result<DataReader<T>, Error> {
        let entry = check::<T>(&self.registry, topic)?;
        let qos = qos(&entry.qos).ok_or_else(|| unknown_qos(entry))?;
        let dds_topic = self.participant.create_topic::<T>(topic)?;
        let subscriber = self.participant.create_subscriber()?;
        Ok(subscriber.create_reader_with_qos::<T>(&dds_topic, &qos)?)
    }
}

fn unknown_qos(entry: &TopicDef) -> Error {
    Error::Contract(ContractError::UnknownQos(entry.qos.clone()))
}

/// The registry/type check, factored out so it is testable without a participant.
fn check<'a, T: DdsType>(registry: &'a TopicRegistry, topic: &str) -> Result<&'a TopicDef, Error> {
    let entry = registry
        .get(topic)
        .ok_or_else(|| Error::UnregisteredTopic(topic.to_string()))?;
    let found = T::type_name();
    if entry.idl_type != found {
        return Err(Error::TypeMismatch {
            topic: topic.to_string(),
            expected: entry.idl_type.clone(),
            found: found.to_string(),
        });
    }
    Ok(entry)
}

/// The QoS named by a contract ("default" | "reliable" | "best_effort" |
/// "transient_local"), or `None` for an unknown name.
pub fn qos(name: &str) -> Option<Qos> {
    let builder = Qos::builder();
    match name {
        "default" => builder.build().ok(),
        "reliable" => builder.reliable().build().ok(),
        "best_effort" => builder.best_effort().build().ok(),
        "transient_local" => builder.transient_local().build().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hello_world::HelloWorldTopic;
    use crate::math::Quaternion;

    fn registry() -> TopicRegistry {
        TopicRegistry::from_toml(
            r#"
[[topics]]
name = "HelloWorldTopic"
type = "hello_world::HelloWorldTopic"
qos = "default"
"#,
        )
        .unwrap()
    }

    #[test]
    fn check_accepts_registered_type() {
        assert!(check::<HelloWorldTopic>(&registry(), "HelloWorldTopic").is_ok());
    }

    #[test]
    fn check_rejects_unregistered_topic() {
        assert!(matches!(
            check::<HelloWorldTopic>(&registry(), "NotRegistered"),
            Err(Error::UnregisteredTopic(t)) if t == "NotRegistered"
        ));
    }

    #[test]
    fn check_rejects_type_mismatch() {
        assert!(matches!(
            check::<Quaternion>(&registry(), "HelloWorldTopic"),
            Err(Error::TypeMismatch { .. })
        ));
    }
}
