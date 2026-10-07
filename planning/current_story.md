# Current story: rust-workspace

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the `rust/` Cargo workspace with one compiling crate.

- **Depends on:** rust-toolchain (done).
- **Minimal test:** `cargo build` from `rust/`.

## Decisions

* One crate: `crates/stations-dds` (the DDS library named in `architecture.md`). The `stations-dds` story fills it in; this story only requires it to compile.
* The Cargo target directory is `build/rust` (`architecture.md`). Set it in `rust/.cargo/config.toml` (`[build] target-dir = "../build/rust"`) so a bare `cargo build` writes under `build/`, as the artifacts rule requires. `build-orchestrator` sets `CARGO_TARGET_DIR` explicitly; the config file makes this story's minimal test correct without it.
* Workspace `resolver = "2"`, edition 2021 (matching the `rust-dds-binding` smoke crate).
* Rust-only increment: no C++ members.

## Deliverables

1. `rust/Cargo.toml` - workspace.
2. `rust/.cargo/config.toml` - target directory.
3. `rust/crates/stations-dds/Cargo.toml` - package `stations-dds`, lib.
4. `rust/crates/stations-dds/src/lib.rs` - placeholder that compiles.
5. `README.md` - a short "Rust workspace" note (build command only).

## Contents

`rust/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/stations-dds"]
```

`rust/.cargo/config.toml`:

```toml
[build]
target-dir = "../build/rust"
```

`rust/crates/stations-dds/Cargo.toml`:

```toml
[package]
name = "stations-dds"
version = "0.0.0"
edition = "2021"
publish = false

[lib]
name = "stations_dds"
path = "src/lib.rs"
```

`rust/crates/stations-dds/src/lib.rs`: a minimal placeholder (doc comment only).

## Minimal test

From `rust/`: `cargo build`; it succeeds and writes artifacts under `build/rust/debug/`.

## Result

Done. `cargo build` from `rust/` compiles `stations-dds` and writes to `build/rust/debug/`; no `rust/target/` is created, confirming the target-dir config. `rust/Cargo.lock` is generated and committed with the workspace.

## Files

* Authored/committed: `rust/Cargo.toml`, `rust/.cargo/config.toml`, `rust/crates/stations-dds/Cargo.toml`, `rust/crates/stations-dds/src/lib.rs`, `README.md`.
* Planning: `planning/current_story.md`, `planning/backlog.md`.
