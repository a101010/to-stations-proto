# Current story: rust-render

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add the Rust windowing/rendering crates.

- **Depends on:** rust-toolchain (done), rust-workspace (done).
- **Minimal test:** a window that renders one egui frame.

## Decisions

* The crates are pinned in `rust/Cargo.toml` under `[workspace.dependencies]` so later crates (for example `display-lib`) consume them with `workspace = true`. There is no consumer crate yet.
* The smoke test is a temporary standalone crate under `build/rust-render-smoke/` (gitignored), matching the `rust-dds-binding` smoke crate pattern.
* Integration uses the raw stack from `architecture.md` (winit + glutin/glow + egui), not `eframe`; `egui_glow`'s `pure_glow` example is the reference.
* Versions (current): winit 0.30.13, glutin 0.32.3, glutin-winit 0.5.0, glow 0.17, egui 0.36.2, egui-winit 0.36.2, egui_glow 0.36.2 (feature `winit`).

## Deliverables

1. `rust/Cargo.toml` - add `[workspace.dependencies]` with the pins.
2. `build/rust-render-smoke/Cargo.toml` and `src/main.rs` - temporary smoke crate.
3. `docs/versions.md` - add a row for the windowing/rendering crates.
4. `README.md` - no change (smoke tests are not documented in the README).

## Contents

`rust/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/stations-dds"]

[workspace.dependencies]
winit = "0.30.13"
glutin = "0.32.3"
glutin-winit = "0.5.0"
glow = "0.17"
egui = "0.36.2"
egui-winit = "0.36.2"
egui_glow = { version = "0.36.2", features = ["winit"] }
```

`build/rust-render-smoke/Cargo.toml`:

```toml
[package]
name = "rust-render-smoke"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
winit = "0.30.13"
glutin = "0.32.3"
glutin-winit = "0.5.0"
glow = "0.17"
egui = "0.36.2"
egui_glow = { version = "0.36.2", features = ["winit"] }
```

`build/rust-render-smoke/src/main.rs`: a winit `ApplicationHandler` that, on `resumed`, creates the glutin context (via `glutin-winit::DisplayBuilder`), a `glow::Context`, and `egui_glow::EguiGlow`; on `RedrawRequested`, runs one egui UI, calls `paint`, swaps buffers, then exits the event loop.

## Minimal test

From `build/rust-render-smoke/`: `cargo run`; a window opens, renders one egui frame, and exits. Requires a desktop session.

## Files

* Authored/committed: `rust/Cargo.toml`, `docs/versions.md`.
* Temporary, gitignored: `build/rust-render-smoke/`.
* Planning: `planning/current_story.md`, `planning/backlog.md`.
