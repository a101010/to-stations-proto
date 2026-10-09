# Current story: rust-dds-binding

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Bind the `stations-dds` library to the `cyclonedds` Rust crate and `cyclonedds-build` built from the to-stations fork of `cyclonedds-rust`, pinned to the fork's `multifile` rev, and prove a typed publish/subscribe round-trip on this machine.

- **Depends on:** none.
- **Minimal test:** a Rust publisher and subscriber exchange one sample of an IDL-defined type over the `StationsDDS` loopback configuration.

## Context

The stock crates.io `cyclonedds`/`cyclonedds-build` cannot parse the shared IDL this project needs (nested `dds::` modules, `#include`, scoped type references, `@optional`) and does not emit `#[dds_typename]` for C++/IDL parity. The to-stations fork of `mzet97/cyclonedds-rust` extends `cyclonedds-build` (and fixes `cyclonedds-derive` for `Option<String>`); its Increment 1 is complete. The fork spec is `planning/cyclonedds-build-plan.md`.

Fork location and pin: `https://github.com/a101010/cyclonedds-rust`, branch `multifile`, rev `acef68438ef3c8eae56c45afbecdbb89f3d3e115`. The fork workspace version is `3.0.0`; the earlier `docs/versions.md`/`README.md` note of `3.0.1` (the crates.io release) is superseded by the pinned rev.

## Decisions

* **Consumption:** a git dependency on the fork workspace pinned to `rev = acef684…`; Cargo resolves `cyclonedds`, `cyclonedds-derive`, `cyclonedds-rust-sys`, and `cyclonedds-src` by name from the same rev. No `version` is stated so it cannot conflict with the fork's `3.0.0`.
* **Feature set:** `cyclonedds = { workspace = true, default-features = false, features = ["native"] }` - the native sync API is enough for the services and avoids pulling the async (`tokio`) surface now. `security` stays off.
* **Static link unchanged:** `CYCLONEDDS_SRC=C:\Libraries\src\cyclonedds` and `CYCLONEDDS_BUILD=C:\Libraries\cyclonedds-rust` still point the fork's `cyclonedds-rust-sys` at the externally built static CycloneDDS, so no runtime DLL and no `PATH` change. The short-path requirement (`architecture.md`, "Path length") still applies to the fork's `$OUT_DIR` CMake probe.
* **No codegen in this story:** the round-trip uses a small hand-written `#[derive(DdsType)]` struct so the story proves linking and runtime only. IDL code generation is the separate `idl-codegen` story.
* **Smoke test is temporary:** a standalone crate under `build/dds-smoke/` (gitignored), following the `assets` precedent; nothing is committed except the Cargo manifest changes and the docs.

## Deliverables

1. `rust/Cargo.toml` - add `cyclonedds` and `cyclonedds-build` git dependencies pinned to the fork rev in `[workspace.dependencies]`.
2. `rust/crates/stations-dds/Cargo.toml` - depend on `cyclonedds`; the `cyclonedds-build` build-dependency is deferred to `idl-codegen`.
3. `build/dds-smoke/` - temporary publisher/subscriber round-trip using a hand-written `DdsType`.
4. `docs/versions.md` and `README.md` - record the fork URL/base/rev and correct the `3.0.1` note.
5. Planning: this file, `planning/backlog.md`, `planning/cyclonedds-build-plan.md`.

## Minimal test

`build/dds-smoke/` (temporary, standalone, gitignored): a crate with its own `Cargo.toml` (path dependency on `rust/crates/stations-dds` and `rust/crates`'s workspace git deps) that

* derives `DdsType` on a small struct,
* creates a `Participant` and `Topic` bound through `CYCLONEDDS_URI=file://T:/DDS/cyclonedds-config.xml`,
* writes one sample from a `DataWriter` and reads it back with a `DataReader`,
* asserts the value, prints `OK`, and exits 0.

If the `StationsDDS` adapter is not present, run the smoke test with `CYCLONEDDS_URI` unset (default transport) to isolate the binding from the loopback wiring; the loopback itself is owned by the `dds-config` story and `scripts/verify-loopback.ps1`.

## Risks and follow-ups

* The repo must be at a short path (`T:`) or the fork's `cyclonedds-rust-sys` CMake probe under `$OUT_DIR` can exceed the Windows 260-character limit (`README.md`, `planning/architecture.md` "Path length").
* If the pinned rev's `cyclonedds`/`cyclonedds-build` versions drift from `docs/versions.md`, re-pin and update `docs/versions.md`.
* Version-string reconciliation (`3.0.0` fork vs `3.0.1` crates.io) is documentation only.

## Files

* Authored/changed: `rust/Cargo.toml`, `rust/crates/stations-dds/Cargo.toml`, `docs/versions.md`, `README.md`.
* Temporary, gitignored: `build/dds-smoke/`.
* Planning: `planning/current_story.md`, `planning/backlog.md`, `planning/cyclonedds-build-plan.md`.
