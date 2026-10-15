//! Formats. Each format is a submodule.
//!
//! `default` is retained at `src/formats/default.rs` but intentionally not
//! declared here: it is dormant until the format manager reactivates it. An
//! undeclared file is not compiled (so there are no dead-code warnings); it is
//! kept so the format is not lost.

pub(crate) mod hello_world;
