# Current story: rust-dds-binding

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Bind the `stations-dds` library to the `cyclonedds` Rust crate and `cyclonedds-build` built from the to-stations fork of `cyclonedds-rust`, pinned to the fork's `multifile` rev, and prove a typed publish/subscribe round-trip on this machine.

- **Depends on:** none.
- **Minimal test:** a Rust publisher and subscriber exchange one sample of an IDL-defined type over the `StationsDDS` loopback configuration.

## Context

The stock crates.io `cyclonedds`/`cyclonedds-build` cannot parse the shared IDL this project needs (nested `dds::` modules, `#include`, scoped type references, `@optional`) and does not emit `#[dds_typename]` for C++/IDL parity. The to-stations fork of `mzet97/cyclonedds-rust` extends `cyclonedds-build` (and fixes `cyclonedds-derive` for `Option<String>`); its Increment 1 is complete. The fork spec is `planning/cyclonedds-build-plan.md`.

Fork location and pin: `https://github.com/a101010/cyclonedds-rust`, branch `multifile`, rev `acef68438ef3c8eae56c45afbecdbb89f3d3e115`. The fork is identified by this rev, not by a semver: it tracks upstream `mzet97/cyclonedds-rust` `main` @ `feb7e7d`, whose manifest version and upstream's release numbering do not describe this tree.

## Decisions

* **Consumption:** a git dependency on the fork workspace pinned to `rev = acef684…`; Cargo resolves `cyclonedds`, `cyclonedds-derive`, `cyclonedds-rust-sys`, and `cyclonedds-src` by name from the same rev. No `version` is stated; the rev is the pin.
* **Feature set:** `cyclonedds = { workspace = true, default-features = false, features = ["native"] }` - the native sync API is enough for the services and avoids pulling the async (`tokio`) surface now. `security` stays off.
* **Static link unchanged:** `CYCLONEDDS_SRC=C:\Libraries\src\cyclonedds` and `CYCLONEDDS_BUILD=C:\Libraries\cyclonedds-rust` still point the fork's `cyclonedds-rust-sys` at the externally built static CycloneDDS, so no runtime DLL and no `PATH` change. The short-path requirement (`architecture.md`, "Path length") still applies to the fork's `$OUT_DIR` CMake probe.
* **No codegen in this story:** the round-trip uses a small hand-written `#[derive(DdsType)]` struct so the story proves linking and runtime only. IDL code generation is the separate `idl-codegen` story.
* **Smoke test is temporary:** a standalone crate under `build/dds-smoke/` (gitignored), following the `assets` precedent; nothing is committed except the Cargo manifest changes and the docs.

## Deliverables

1. `rust/Cargo.toml` - add `cyclonedds` and `cyclonedds-build` git dependencies pinned to the fork rev in `[workspace.dependencies]`.
2. `rust/crates/stations-dds/Cargo.toml` - depend on `cyclonedds`; the `cyclonedds-build` build-dependency is deferred to `idl-codegen`.
3. `build/dds-smoke/` - temporary publisher/subscriber round-trip using a hand-written `DdsType`.
4. `docs/versions.md` and `README.md` - record the fork repo, branch, and rev (no semver).
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
* If the fork is advanced, re-pin the rev and update `docs/versions.md`.

## Result

Done and verified. `rust/Cargo.toml` pins `cyclonedds` (`default-features = false, features = ["native"]`) and `cyclonedds-build` as git dependencies on the fork at rev `acef684…`; `stations-dds` depends on `cyclonedds` and re-exports it (`pub use cyclonedds;`) plus `type_name::<T>()`. Cargo resolved all four fork packages from that rev (their self-declared versions are carried in `Cargo.lock`) and linked the prebuilt static `ddsc.lib` from `C:\Libraries\cyclonedds-rust` (no DLL, no `PATH` change).

`build/dds-smoke/` (temporary) derives `DdsType` on a keyed `Smoke { id: i32, message: String }`, writes one sample and reads it back: `OK: round-tripped id=42 message="hello from the fork" type_name=Smoke`. It passes both over the default transport and with `CYCLONEDDS_URI=file://T:/DDS/cyclonedds-config.xml` (the `StationsDDS` adapter is present and up). `cargo build` and `cargo fmt --all -- --check` are clean for the workspace.

`docs/versions.md` and `README.md` record the fork repo, branch, and rev. `rust/Cargo.lock` records the resolved fork packages.

## Files

* Authored/changed: `rust/Cargo.toml`, `rust/Cargo.lock`, `rust/crates/stations-dds/Cargo.toml`, `rust/crates/stations-dds/src/lib.rs`, `docs/versions.md`, `README.md`.
* Temporary, gitignored: `build/dds-smoke/`, `build/rust/`, `build/dds-smoke-target/`.
* Planning: `planning/current_story.md`, `planning/backlog.md`, `planning/cyclonedds-build-plan.md`.
