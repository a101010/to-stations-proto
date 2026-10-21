//! The raw-topic-API bypass scan.
//!
//! The DDS topic-naming API must be reachable only through the `stations-dds`
//! `Node` implementation. This scan reports raw calls found in the caller's
//! roots, so the build can fail when a crate reaches around `Node`.
//!
//! The scanner is **repo-agnostic**: the caller supplies the roots (and any
//! allowed subtrees). It has no knowledge of the repository layout.

use std::path::{Path, PathBuf};

/// The raw CycloneDDS topic-naming functions.
pub const RAW_TOPIC_APIS: &[&str] = &[
    "create_topic",
    "create_topic_with_qos",
    "create_topic_from_descriptor",
    "create_topic_from_descriptor_with_qos",
    "create_topic_from_type_info",
    "create_topic_from_type_info_with_qos",
    "create_topic_from_sertype",
    "create_topic_from_sertype_with_qos",
    "find_topic",
    "dynamic_publish",
    "dynamic_subscribe",
];

/// A raw topic-API call found in the scanned roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bypass {
    pub file: PathBuf,
    pub line: usize,
    pub name: String,
}

/// Scan `roots` for raw topic-API calls, skipping any path under `allow`.
///
/// Only `.rs` files are scanned and `target/` directories are skipped.
pub fn scan(roots: &[PathBuf], allow: &[PathBuf]) -> Result<Vec<Bypass>, std::io::Error> {
    let mut out = Vec::new();
    for root in roots {
        scan_path(root, allow, &mut out)?;
    }
    out.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.name.cmp(&b.name))
    });
    Ok(out)
}

fn scan_path(path: &Path, allow: &[PathBuf], out: &mut Vec<Bypass>) -> Result<(), std::io::Error> {
    if allow.iter().any(|a| path.starts_with(a)) {
        return Ok(());
    }
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let child = entry.path();
            if child.is_dir() {
                if child.file_name().and_then(|n| n.to_str()) == Some("target") {
                    continue;
                }
                scan_path(&child, allow, out)?;
            } else if is_rust(&child) {
                scan_file(&child, out)?;
            }
        }
    } else if is_rust(path) {
        scan_file(path, out)?;
    }
    Ok(())
}

fn is_rust(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("rs")
}

fn scan_file(path: &Path, out: &mut Vec<Bypass>) -> Result<(), std::io::Error> {
    let text = std::fs::read_to_string(path)?;
    for (index, line) in text.lines().enumerate() {
        for name in RAW_TOPIC_APIS {
            let mut from = 0;
            while let Some(offset) = line[from..].find(name) {
                let start = from + offset;
                let after = line[start + name.len()..].trim_start();
                if after.starts_with('(') || after.starts_with("::") {
                    out.push(Bypass {
                        file: path.to_path_buf(),
                        line: index + 1,
                        name: name.to_string(),
                    });
                }
                from = start + name.len();
                if from >= line.len() {
                    break;
                }
            }
        }
    }
    Ok(())
}
