# Current story: hello-world-idl

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Author the hello_world IDL as nested IDL modules and generate Rust into `build/gen/rust` with `scripts/build.ps1` (the fork's `cyclonedds-idlc` Rust codegen), which then builds the workspace, so the generated Rust types are part of the `stations-dds` library. (This merges the former `hello-world-topic` and `idl-codegen` stories.)

- **Depends on:** none.
- **Minimal test:** `scripts/build.ps1 -Test` generates `build/gen/rust/hello_world.rs`, builds, and runs the tests; `math::Quaternion::type_name() == "math::Quaternion"` and `hello_world::HelloWorldTopic::type_name() == "hello_world::HelloWorldTopic"`.

## Context

`DDS/` currently holds only `cyclonedds-config.xml`. The fork pinned in `rust-dds-binding` provides IDL-to-Rust code generation (`cyclonedds-build::compile_idl_with_options`) with nested-module support, `#include`/`import`, scoped type references, and `#[dds_typename]`. The fork also ships that engine as a standalone Rust CLI, `cyclonedds-idlc`, which is what this story runs.

The approach follows the wider Rust DDS ecosystem (HDDS `hddsgen`, `dust_dds_gen`): an IDL `module` is a namespace that maps to a Rust `pub mod`; the DDS type name is the fully-qualified IDL scope; and a **single root IDL file** is compiled with its `#include`s inlined into one generated Rust file, pulled into the crate with `include!`. Nothing about that requires a fork change.

Two different tools share the name "idlc": the C `idlc.exe` (installed with `cyclonedds-core`) emits C/C++, not Rust; the fork's `cyclonedds-idlc` is a **Rust** CLI that wraps `cyclonedds-build`. This story uses the fork's `cyclonedds-idlc`, run with `--no-idlc` so it never invokes the C tool.

## Decisions

* **Nested IDL `module` -> nested Rust `pub mod`**; the DDS type name is the fully-qualified IDL scope, emitted via `#[dds_typename]` (fork behavior).
* **No vendor root namespace.** Modules are top-level (`math`, `hello_world`); the DDS names are `math::Quaternion` and `hello_world::HelloWorldTopic`.
* **Module-sized files: one top-level IDL `module` per file, all of that module's types in the one file.** Small files are preferred, but a file is not split per struct, and a module is not reopened across files (the fork, like HDDS/dust_dds, does not merge reopened modules). A format's model/view/control structs therefore all live in that format's module file: the view payload is added to `hello_world.idl` alongside the model, not to a separate `hello_world_view.idl`.
* **Single compile unit.** The fork's `cyclonedds-idlc` compiles one root IDL (`DDS/hello_world.idl`); its `#include "math.idl"` is inlined. Distinct top-level modules mean no reopened-module problem; no umbrella, no fragments, no fork change.
* **Shared value type up front.** `Quaternion` lives in a top-level `module math` now so the view can reuse it later (IDL inheritance is out of scope, so reuse is by embedding).
* **Naming.** No "Model" in type, service, or code names. A topic's type name and topic name are identical and end in `Topic`: `HelloWorldTopic` (model role), `HelloWorldViewTopic`, `HelloWorldCtlTopic`. Embedded value structs are bare (`Quaternion`). Service executables end in `Service`.
* **The build is script-owned, not a Cargo build script.** `scripts/build.ps1` generates code with the fork's `cyclonedds-idlc` and then builds the Rust workspace; the crate has no `build.rs` and no `cyclonedds-build` build-dependency. This matches the architecture, where `scripts/build.ps1` is the single build orchestrator.
* **Fork consumption.** The fork stays a pinned git dependency: it is downloaded from the repository and built once, then cached in the `build/rust` target directory. `build.ps1` never cleans, so codegen and the Rust build do not rebuild the fork unless `build/` is wiped (`git clean -xdf`); that rebuild is acceptable.
* **No `..` in paths.** The crate reaches the generated file through `REPO_ROOT`, the absolute repository root, which Cargo computes from `.cargo/config.toml` (`[env] REPO_ROOT = { value = ".", relative = true }`) and passes to every rustc invocation. External components are addressed by absolute paths under `EXT_PATH_PREFIX` (default `C:\Libraries`). Each build system resolves the repository root from its own root file rather than a shared environment variable set by a script.
* **Generated files live in `build/gen/rust` and are never committed.**
* **Crate location.** `stations-dds` lives at `rust/lib/stations-dds` (moved out of `rust/crates/` in this story).

## Deliverables

1. `DDS/math.idl` - `module math { struct Quaternion { double x; double y; double z; double w; }; };`
2. `DDS/hello_world.idl` - `#include "math.idl"` then `module hello_world { struct HelloWorldTopic { string greeting; math::Quaternion attitude; }; };`
3. `scripts/build.ps1` - generate with the fork's `cyclonedds-idlc` into `build/gen/rust`, then build the Rust workspace (`-Test` runs the tests). `build-orchestrator` extends it.
4. `.cargo/config.toml` (repository root) - `target-dir = "build/rust"` and `[env] REPO_ROOT = { value = ".", relative = true }`; replaces `rust/.cargo/config.toml`.
5. `rust/Cargo.toml` - members `["lib/stations-dds", "tool/equirect-to-cubemap"]`; drop the now-unused `cyclonedds-build` workspace dependency.
6. `rust/lib/stations-dds/Cargo.toml` - keep the `cyclonedds` dependency; no build-dependencies.
7. `rust/lib/stations-dds/src/lib.rs` - `include!` the generated file and add the `#[test]` assertions.
8. `docs/versions.md` - note that the fork's `cyclonedds-idlc` is built from the same pinned rev.
9. `README.md` - build instructions and the `idlc.exe` vs `cyclonedds-idlc` distinction.
10. Planning: this file, `planning/backlog.md`, `planning/architecture.md`, `planning/cyclonedds-build-plan.md`.

## IDL

`DDS/math.idl`:
```
module math {
  struct Quaternion {
    double x;
    double y;
    double z;
    double w;
  };
};
```

`DDS/hello_world.idl` (the compiled root):
```
#include "math.idl"

module hello_world {
  struct HelloWorldTopic {
    string greeting;
    math::Quaternion attitude;
  };
};
```

The generated `build/gen/rust/hello_world.rs` contains `pub mod math { ... }` and
`pub mod hello_world { ... }`, with `HelloWorldTopic.attitude` typed as
`super::math::Quaternion` and `#[dds_typename("hello_world::HelloWorldTopic")]` on the topic.

## Build wiring

`scripts/build.ps1` (new; `build-orchestrator` extends it):
- Resolve the repository root from the script's own location (`Split-Path -Parent $PSScriptRoot`); use repository-relative paths, and no `..`.
- `$prefix = $env:EXT_PATH_PREFIX` (the external prefix for installed components; default `C:\Libraries`).
- Build the fork's `cyclonedds-idlc` from the pinned rev, cached under `build/`:
  `cargo install --git https://github.com/a101010/cyclonedds-rust --rev acef68438ef3c8eae56c45afbecdbb89f3d3e115 cyclonedds-idlc --locked --target-dir build/rust/codegen --root build`
  The `--target-dir` caches the tool's compilation; without it `cargo install` uses a fresh temporary directory and rebuilds every time. `cargo install` writes the binary to `build/bin/cyclonedds-idlc.exe`.
- Generate: `build/bin/cyclonedds-idlc.exe --input DDS/hello_world.idl --output-dir build/gen/rust --include-dir DDS --no-idlc` (defaults already emit `#[dds_typename]`).
- Build the Rust workspace from the repository root (so Cargo finds `.cargo/config.toml`): `cargo build --manifest-path rust\Cargo.toml`, or `cargo test --manifest-path rust\Cargo.toml` with `-Test`.
- No clean: do not run `cargo clean` and do not delete `build/`, so dependencies (including the fork) are not rebuilt.

`rust/lib/stations-dds/src/lib.rs` appends:

```
include!(concat!(env!("REPO_ROOT"), "/build/gen/rust/hello_world.rs"));
```

so `stations_dds::math::Quaternion` and `stations_dds::hello_world::HelloWorldTopic` exist at the crate root. `REPO_ROOT` is supplied by Cargo from `.cargo/config.toml` (its `relative = true` resolves to the repository root), so any `cargo build`/`test` run from within the repository has it; no script or caller sets it.

## Tests

A `#[cfg(test)]` module in `rust/lib/stations-dds/src/lib.rs`. The generated modules sit at the crate root, so the test module must bring them into scope:

```
#[cfg(test)]
mod tests {
    use super::{hello_world, math};
    use ::cyclonedds::DdsType;

    #[test]
    fn type_names_match_scopes() {
        assert_eq!(<math::Quaternion as DdsType>::type_name(), "math::Quaternion");
        assert_eq!(<hello_world::HelloWorldTopic as DdsType>::type_name(), "hello_world::HelloWorldTopic");
    }
}
```

`type_name()` reads the `#[dds_typename]` value and needs no DDS participant.

## Minimal test

```
./scripts/build.ps1 -Test
```

A clean run generates `build/gen/rust/hello_world.rs`, builds the workspace, and the two assertions pass.

## Risks and follow-ups

* **The generated file must exist before compiling.** `.cargo/config.toml` always supplies `REPO_ROOT`, but `build/gen/rust/hello_world.rs` only exists after `scripts/build.ps1` runs; a bare `cargo build` on a freshly cleaned tree fails with a missing-file error until then. Documented in `README.md`.
* **Path length.** `cyclonedds-rust-sys` runs a small CMake ABI probe under its `$OUT_DIR`, so the repository must stay at a short path on Windows (`T:`). The main CycloneDDS CMake build is redirected to the external prefix (`CYCLONEDDS_SRC`, `CYCLONEDDS_BUILD`).
* **Codegen build cache.** `cargo install --target-dir build/rust/codegen` caches the tool's compilation; wiping `build/` (for example `git clean -xdf`) forces a rebuild, which is acceptable.
* **Modules are not reopened across files.** The fork neither merges reopened modules nor deduplicates includes, so a format's types all live in that format's module file: `HelloWorldViewTopic` (and later `HelloWorldCtlTopic`) are added to `DDS/hello_world.idl`, not to a separate `hello_world_view.idl`. This supersedes the `hello_world_view.idl` wording in `backlog.md` and `architecture.md` (both updated by this story).
* **Diamond includes of shared modules.** If a later format also needs `math`, only one file may `#include "math.idl"`: with no include-once, including it twice emits two `pub mod math`. Either the single root includes `math.idl` first and each format refers to the already-inlined `math`, or the fork gains include-once / reopened-module merge. Decide when the second format appears; deferred now.
* The to-stations planning docs use the `math`/`hello_world`/`HelloWorldTopic` names; the fork spec `planning/cyclonedds-build-plan.md` keeps its own `dds::hello_world::HelloWorldModel` examples (those describe the fork's internal tests).

## Files

* Authored/changed: `DDS/math.idl`, `DDS/hello_world.idl`, `scripts/build.ps1`, `.cargo/config.toml`, `rust/Cargo.toml`, `rust/lib/stations-dds/Cargo.toml`, `rust/lib/stations-dds/src/lib.rs`, `README.md`, `docs/versions.md`.
* Moved: `rust/crates/stations-dds` -> `rust/lib/stations-dds` (dropping `build.rs`); `rust/.cargo/config.toml` -> `.cargo/config.toml`.
* Generated, gitignored: `build/gen/rust/hello_world.rs`, `build/bin/cyclonedds-idlc.exe`, `build/rust/codegen`.
* Planning: `planning/current_story.md`, `planning/backlog.md`, `planning/architecture.md`, `planning/cyclonedds-build-plan.md`.
