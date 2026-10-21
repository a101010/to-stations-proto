//! Scanner tests against the committed fixtures under `tests/fixtures/scan`.

use std::path::PathBuf;

use dds_contracts::scan::scan;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/scan")
}

#[test]
fn flags_raw_calls_and_ignores_node_calls() {
    let root = fixtures();
    let found = scan(&[root.clone()], &[]).unwrap();

    // `bypass.rs` and `allowed/bypass.rs` each hold one raw call on line 3.
    assert_eq!(found.len(), 2, "{found:?}");
    for bypass in &found {
        assert_eq!(bypass.name, "create_topic");
        assert_eq!(bypass.line, 3);
    }
    assert!(found.iter().any(|b| b.file.ends_with("bypass.rs")));
    assert!(found
        .iter()
        .any(|b| b.file.to_string_lossy().contains("allowed")));
    // `ok.rs` (a `Node::writer` call) must not appear.
    assert!(!found.iter().any(|b| b.file.ends_with("ok.rs")));
}

#[test]
fn allowed_subtree_is_skipped() {
    let root = fixtures();
    let allowed = root.join("allowed");
    let found = scan(&[root], &[allowed]).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].file.ends_with("bypass.rs"));
}
