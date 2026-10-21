//! Service contracts: the topic registry and the per-service pub/sub
//! declarations.
//!
//! `contracts/topics.toml` records the topics; `contracts/services/<app>.toml`
//! records each service's publishes and subscribes. The registry is validated
//! against the generated DDS types and the QoS names the DDS layer supports; a
//! service contract is validated against the registry. [`coverage`] and
//! [`summary`] feed `build/topics.md`, and [`ServiceContract::render`] is the
//! `--topics` text.

use std::collections::HashSet;
use std::path::Path;

use serde::Deserialize;

/// The topic registry (`contracts/topics.toml`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TopicRegistry {
    #[serde(default)]
    pub topics: Vec<TopicDef>,
}

/// One registered topic.
#[derive(Debug, Clone, Deserialize)]
pub struct TopicDef {
    /// Short wire/registry name, e.g. `HelloWorldTopic`.
    pub name: String,
    /// Scoped IDL/DDS type name, e.g. `hello_world::HelloWorldTopic`.
    #[serde(rename = "type")]
    pub idl_type: String,
    /// QoS name (see [`crate::dds::qos`]).
    pub qos: String,
    /// Key member names. Recorded but not validated yet.
    #[serde(default)]
    pub key: Vec<String>,
}

/// A service's contract (`contracts/services/<app>.toml`).
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceContract {
    pub service: ServiceMeta,
    #[serde(default)]
    pub publish: Vec<TopicRef>,
    #[serde(default)]
    pub subscribe: Vec<TopicRef>,
}

/// The service's identifying metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceMeta {
    pub name: String,
}

/// A reference to a registered topic.
#[derive(Debug, Clone, Deserialize)]
pub struct TopicRef {
    pub topic: String,
}

/// Which services publish and subscribe a topic (for the summary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicUsage {
    pub topic: String,
    pub publishers: Vec<String>,
    pub subscribers: Vec<String>,
}

/// A contract that failed to load or validate.
#[derive(Debug)]
pub enum ContractError {
    Io(String),
    Toml(String),
    UnknownTopic(String),
    UnknownType(String),
    UnknownQos(String),
    NameMismatch { expected: String, found: String },
    Duplicate(String),
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractError::Io(msg) => write!(f, "io error: {msg}"),
            ContractError::Toml(msg) => write!(f, "toml error: {msg}"),
            ContractError::UnknownTopic(t) => write!(f, "unknown topic: {t}"),
            ContractError::UnknownType(t) => write!(f, "unknown IDL type: {t}"),
            ContractError::UnknownQos(q) => write!(f, "unknown QoS: {q}"),
            ContractError::NameMismatch { expected, found } => {
                write!(f, "service name `{found}` does not match file stem `{expected}`")
            }
            ContractError::Duplicate(t) => write!(f, "duplicate topic: {t}"),
        }
    }
}

impl std::error::Error for ContractError {}

/// Scoped DDS type names of the generated topic types.
pub fn known_types() -> &'static [&'static str] {
    &["hello_world::HelloWorldTopic"]
}

impl TopicRegistry {
    /// Parse a registry from TOML text.
    pub fn from_toml(text: &str) -> Result<Self, ContractError> {
        toml::from_str(text).map_err(|e| ContractError::Toml(e.to_string()))
    }

    /// Load a registry from `path`.
    pub fn load(path: &Path) -> Result<Self, ContractError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| ContractError::Io(format!("read {}: {e}", path.display())))?;
        Self::from_toml(&text)
    }

    /// Every entry has a known IDL type and QoS, and names are unique.
    pub fn validate(&self) -> Result<(), ContractError> {
        let mut seen = HashSet::new();
        for topic in &self.topics {
            if !seen.insert(topic.name.as_str()) {
                return Err(ContractError::Duplicate(topic.name.clone()));
            }
            if !known_types().contains(&topic.idl_type.as_str()) {
                return Err(ContractError::UnknownType(topic.idl_type.clone()));
            }
            if crate::dds::qos(&topic.qos).is_none() {
                return Err(ContractError::UnknownQos(topic.qos.clone()));
            }
        }
        Ok(())
    }

    /// The entry registered under `name`.
    pub fn get(&self, name: &str) -> Option<&TopicDef> {
        self.topics.iter().find(|t| t.name == name)
    }
}

impl ServiceContract {
    /// Parse a service contract from TOML text.
    pub fn from_toml(text: &str) -> Result<Self, ContractError> {
        toml::from_str(text).map_err(|e| ContractError::Toml(e.to_string()))
    }

    /// Load a service contract from `path`, checking that its `service.name`
    /// matches the file stem.
    pub fn load(path: &Path) -> Result<Self, ContractError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| ContractError::Io(format!("read {}: {e}", path.display())))?;
        let contract = Self::from_toml(&text)?;
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        contract.check_name(stem)?;
        Ok(contract)
    }

    fn check_name(&self, stem: &str) -> Result<(), ContractError> {
        if self.service.name == stem {
            Ok(())
        } else {
            Err(ContractError::NameMismatch {
                expected: stem.to_string(),
                found: self.service.name.clone(),
            })
        }
    }

    /// Every referenced topic is in `registry`.
    pub fn validate(&self, registry: &TopicRegistry) -> Result<(), ContractError> {
        for reference in self.publish.iter().chain(self.subscribe.iter()) {
            if registry.get(&reference.topic).is_none() {
                return Err(ContractError::UnknownTopic(reference.topic.clone()));
            }
        }
        Ok(())
    }

    /// The human-readable contract, printed by `--topics`.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("service {}\n", self.service.name));
        out.push_str("  publish:\n");
        push_topics(&mut out, &self.publish);
        out.push_str("  subscribe:\n");
        push_topics(&mut out, &self.subscribe);
        out
    }
}

fn push_topics(out: &mut String, refs: &[TopicRef]) {
    if refs.is_empty() {
        out.push_str("    (none)\n");
    }
    for reference in refs {
        out.push_str(&format!("    {}\n", reference.topic));
    }
}

/// Which services publish and subscribe each registered topic.
pub fn coverage(registry: &TopicRegistry, services: &[ServiceContract]) -> Vec<TopicUsage> {
    registry
        .topics
        .iter()
        .map(|topic| TopicUsage {
            topic: topic.name.clone(),
            publishers: services
                .iter()
                .filter(|s| s.publish.iter().any(|r| r.topic == topic.name))
                .map(|s| s.service.name.clone())
                .collect(),
            subscribers: services
                .iter()
                .filter(|s| s.subscribe.iter().any(|r| r.topic == topic.name))
                .map(|s| s.service.name.clone())
                .collect(),
        })
        .collect()
}

/// The `build/topics.md` summary: the registry, the per-topic usage, and each
/// service's rendered contract.
pub fn summary(registry: &TopicRegistry, services: &[ServiceContract]) -> String {
    let mut out = String::new();
    out.push_str("# Topics\n\n## Registry\n\n");
    out.push_str("| Topic | Type | QoS | Key |\n|---|---|---|---|\n");
    for topic in &registry.topics {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            topic.name,
            topic.idl_type,
            topic.qos,
            topic.key.join(", ")
        ));
    }

    out.push_str("\n## Usage\n\n");
    out.push_str("| Topic | Publishers | Subscribers |\n|---|---|---|\n");
    for usage in coverage(registry, services) {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            usage.topic,
            usage.publishers.join(", "),
            usage.subscribers.join(", ")
        ));
    }

    out.push_str("\n## Services\n");
    for service in services {
        out.push_str(&format!("\n```\n{}```\n", service.render()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY: &str = r#"
[[topics]]
name = "HelloWorldTopic"
type = "hello_world::HelloWorldTopic"
qos = "default"
key = []
"#;

    const SERVICE: &str = r#"
[service]
name = "hello-service"
[[publish]]
topic = "HelloWorldTopic"
"#;

    fn registry() -> TopicRegistry {
        TopicRegistry::from_toml(REGISTRY).unwrap()
    }

    fn service() -> ServiceContract {
        ServiceContract::from_toml(SERVICE).unwrap()
    }

    #[test]
    fn parses_and_validates_registry() {
        let r = registry();
        assert_eq!(r.topics.len(), 1);
        assert!(r.validate().is_ok());
        assert_eq!(r.get("HelloWorldTopic").unwrap().idl_type, "hello_world::HelloWorldTopic");
        assert!(r.get("Nope").is_none());
    }

    #[test]
    fn rejects_unknown_type() {
        let r = TopicRegistry::from_toml(
            r#"
[[topics]]
name = "X"
type = "x::Y"
qos = "default"
"#,
        )
        .unwrap();
        assert!(matches!(r.validate(), Err(ContractError::UnknownType(t)) if t == "x::Y"));
    }

    #[test]
    fn rejects_unknown_qos() {
        let r = TopicRegistry::from_toml(
            r#"
[[topics]]
name = "HelloWorldTopic"
type = "hello_world::HelloWorldTopic"
qos = "warp"
"#,
        )
        .unwrap();
        assert!(matches!(r.validate(), Err(ContractError::UnknownQos(q)) if q == "warp"));
    }

    #[test]
    fn rejects_duplicate_topic() {
        let r = TopicRegistry::from_toml(
            r#"
[[topics]]
name = "HelloWorldTopic"
type = "hello_world::HelloWorldTopic"
qos = "default"
[[topics]]
name = "HelloWorldTopic"
type = "hello_world::HelloWorldTopic"
qos = "default"
"#,
        )
        .unwrap();
        assert!(matches!(r.validate(), Err(ContractError::Duplicate(t)) if t == "HelloWorldTopic"));
    }

    #[test]
    fn rejects_service_name_mismatch() {
        assert!(matches!(
            service().check_name("other"),
            Err(ContractError::NameMismatch { expected, found }) if expected == "other" && found == "hello-service"
        ));
        assert!(service().check_name("hello-service").is_ok());
    }

    #[test]
    fn rejects_unknown_topic_reference() {
        let bad = ServiceContract::from_toml(
            r#"
[service]
name = "hello-service"
[[publish]]
topic = "Nope"
"#,
        )
        .unwrap();
        assert!(matches!(
            bad.validate(&registry()),
            Err(ContractError::UnknownTopic(t)) if t == "Nope"
        ));
        assert!(service().validate(&registry()).is_ok());
    }

    #[test]
    fn render_lists_pub_sub() {
        let text = service().render();
        assert!(text.contains("service hello-service"));
        assert!(text.contains("HelloWorldTopic"));
        assert!(text.contains("subscribe:\n    (none)"));
    }

    #[test]
    fn coverage_reports_publishers_only() {
        let usage = coverage(&registry(), &[service()]);
        assert_eq!(
            usage,
            vec![TopicUsage {
                topic: "HelloWorldTopic".to_string(),
                publishers: vec!["hello-service".to_string()],
                subscribers: vec![],
            }]
        );
    }

    #[test]
    fn summary_contains_each_render() {
        let summary = summary(&registry(), &[service()]);
        assert!(summary.contains(&service().render()));
        assert!(summary.contains("| HelloWorldTopic |"));
    }
}
