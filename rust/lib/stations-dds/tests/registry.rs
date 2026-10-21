//! Completeness: every `*Topic` struct in the IDL must be listed in
//! `known_types()` and registered in `contracts/topics.toml`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use stations_dds::contract::{known_types, TopicRegistry};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("REPO_ROOT"))
}

#[test]
fn idl_topics_match_known_types_and_registry() {
    let mut idl = BTreeSet::new();
    for entry in std::fs::read_dir(repo_root().join("DDS")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("idl") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        if let Some(module) = first_module(&text) {
            for name in topic_structs(&text) {
                idl.insert(format!("{module}::{name}"));
            }
        }
    }

    let known: BTreeSet<String> = known_types().iter().map(|s| s.to_string()).collect();
    assert_eq!(idl, known, "IDL topic set does not match known_types()");

    let registry = TopicRegistry::load(&repo_root().join("contracts/topics.toml")).unwrap();
    let registered: BTreeSet<&str> = registry.topics.iter().map(|t| t.idl_type.as_str()).collect();
    for topic in &known {
        assert!(
            registered.contains(topic.as_str()),
            "topic type `{topic}` is not registered"
        );
    }
}

/// The first `module <name>` in the text.
fn first_module(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("module ") {
            let name = identifier(rest);
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

/// Every `struct <name>` whose name ends in `Topic`.
fn topic_structs(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("struct "))
        .map(identifier)
        .filter(|name| name.ends_with("Topic"))
        .collect()
}

fn identifier(rest: &str) -> String {
    rest.chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}
