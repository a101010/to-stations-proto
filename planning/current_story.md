# Current story: hello-world-idl

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Author the hello_world IDL as nested IDL modules and wire `cyclonedds-build` IDL code generation permanently into the `stations-dds` Cargo build, so the generated Rust types are part of the library. (This merges the former `hello-world-topic` and `idl-codegen` stories.)

- **Depends on:** none (uses the fork rev pinned in `rust-dds-binding`).
- **Minimal test:** a clean `cargo build` generates `build/gen/rust/hello_world.rs` and compiles it into `stations-dds`; `hello_world::HelloWorldTopic::type_name() == "hello_world::HelloWorldTopic"` and `math::Quaternion::type_name() == "math::Quaternion"`.

## Context

`DDS/` currently holds only `cyclonedds-config.xml`. The fork pinned in `rust-dds-binding` provides IDL-to-Rust code generation (`cyclonedds-build::compile_idl_with_options`) with nested-module support, `#include`/`import`, scoped type references, and `#[dds_typename]`. This story authors the first IDL and wires code generation into the build for real (no throwaway smoke crate).

The approach follows the wider Rust DDS ecosystem (HDDS `hddsgen`, `dust_dds_gen`): an IDL `module` is a namespace that maps to a Rust `pub mod`; the DDS type name is the fully-qualified IDL scope; and a **single root IDL file** is compiled with its `#include`s inlined into one generated Rust file, pulled into the crate with `include!`. Nothing about that requires a fork change.

## Decisions

* **Nested IDL `module` -> nested Rust `pub mod`**; the DDS type name is the fully-qualified IDL scope, emitted via `#[dds_typename]` (fork behavior).
* **No vendor root namespace.** Modules are top-level (`math`, `hello_world`); the DDS names are `math::Quaternion` and `hello_world::HelloWorldTopic`.
* **Module-sized files: one top-level IDL `module` per file, all of that module's types in the one file.** Small files are preferred, but a file is not split per struct, and a module is not reopened across files (the fork, like HDDS/dust_dds, does not merge reopened modules). A format's model/view/control structs therefore all live in that format's module file: the view payload is added to `hello_world.idl` alongside the model, not to a separate `hello_world_view.idl`.
* **Single compile unit.** `compile_idl_with_options` compiles one root IDL (`DDS/hello_world.idl`); its `#include "math.idl"` is inlined. Distinct top-level modules mean no reopened-module problem; no umbrella, no fragments, no fork change.
* **Shared value type up front.** `Quaternion` lives in a top-level `module math` now so the view can reuse it later (IDL inheritance is out of scope, so reuse is by embedding).
* **Naming.** No "Model" in type, service, or code names. A topic's type name and topic name are identical and end in `Topic`: `HelloWorldTopic` (model role), `HelloWorldViewTopic`, `HelloWorldCtlTopic`. Embedded value structs are bare (`Quaternion`). Service executables end in `Service`.
* **Parser.** `try_idlc = false` (built-in parser); `idlc`/C++ type-name parity is the fork's own test concern.
* **Generated files live in `build/gen/rust` and are never committed.**

## Deliverables

1. `DDS/math.idl` - `module math { struct Quaternion { double x; double y; double z; double w; }; };`
2. `DDS/hello_world.idl` - `#include "math.idl"` then `module hello_world { struct HelloWorldTopic { string greeting; math::Quaternion attitude; }; };`
3. `rust/crates/stations-dds/Cargo.toml` - add `[build-dependencies] cyclonedds-build = { workspace = true }` (keep the `cyclonedds` dependency).
4. `rust/crates/stations-dds/build.rs` - call `compile_idl_with_options`.
5. `rust/crates/stations-dds/src/lib.rs` - `include!` the generated file and add the `#[test]` assertions.
6. Planning: this file, `planning/backlog.md`, `planning/architecture.md` (drop the stale `hello_world_view.idl` and state that modules are not reopened).

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

`rust/crates/stations-dds/build.rs`:
- Resolve the repository root from `CARGO_MANIFEST_DIR` (`rust/crates/stations-dds` -> `../../..`; `rust/lib/stations-dds` is the same depth, so the path survives the later move).
- `std::fs::create_dir_all(<repo>/build/gen/rust)`.
- Build `let options = CompileOptions { output_dir: Some(<repo>/build/gen/rust), include_dirs: vec![<repo>/DDS], emit_dds_typename: true, try_idlc: false, ..Default::default() };` and call `compile_idl_with_options(<repo>/DDS/hello_world.idl, &options)`.
- Optionally `println!("cargo:rerun-if-changed={}", <repo>/DDS)`. The fork already emits one `rerun-if-changed` per preprocessed file (the root plus each `#include`), so this directory watch is only needed if adding or removing files under `DDS/` should also trigger a rebuild.

`rust/crates/stations-dds/src/lib.rs` appends:

```
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../build/gen/rust/hello_world.rs"));
```

so `stations_dds::math::Quaternion` and `stations_dds::hello_world::HelloWorldTopic` exist at the crate root.

## Tests

A `#[cfg(test)]` module in `rust/crates/stations-dds/src/lib.rs`. The generated modules sit at the crate root, so the test module must bring them into scope:

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
CARGO_TARGET_DIR=T:/build/rust \
CYCLONEDDS_SRC=C:/Libraries/src/cyclonedds \
CYCLONEDDS_BUILD=C:/Libraries/cyclonedds-rust \
cargo test --manifest-path rust/Cargo.toml -p stations-dds
```

A clean run generates `build/gen/rust/hello_world.rs`, compiles it, and the two
assertions pass.

## Risks and follow-ups

* **Modules are not reopened across files.** The fork neither merges reopened modules nor deduplicates includes, so a format's types all live in that format's module file: `HelloWorldViewTopic` (and later `HelloWorldCtlTopic`) are added to `DDS/hello_world.idl`, not to a separate `hello_world_view.idl`. This supersedes the `hello_world_view.idl` wording in `backlog.md` and `architecture.md` (both updated by this story).
* **Diamond includes of shared modules.** If a later format also needs `math`, only one file may `#include "math.idl"`: with no include-once, including it twice emits two `pub mod math`. Either the single root includes `math.idl` first and each format refers to the already-inlined `math`, or the fork gains include-once / reopened-module merge. Decide when the second format appears; deferred now.
* **One root per build.** Compile one root IDL rather than several roots into several generated files, because shared modules such as `math` would otherwise be duplicated across generated files that are all `include!`d.
* `include!` depends on the crate's location; the `stations-dds` story's `crates/` -> `lib/` move keeps the same depth (`../../..`), so the path is stable, but re-verify there.
* The short-path requirement (`README.md`, `architecture.md` "Path length") still applies to the fork's `cyclonedds-rust-sys` `$OUT_DIR` CMake probe.
* The to-stations planning docs use the `math`/`hello_world`/`HelloWorldTopic` names; the fork spec `planning/cyclonedds-build-plan.md` keeps its own `dds::hello_world::HelloWorldModel` examples (those describe the fork's internal tests).

## Files

* Authored/changed: `DDS/math.idl`, `DDS/hello_world.idl`, `rust/crates/stations-dds/Cargo.toml`, `rust/crates/stations-dds/build.rs`, `rust/crates/stations-dds/src/lib.rs`.
* Generated, gitignored: `build/gen/rust/hello_world.rs`.
* Planning: `planning/current_story.md`, `planning/backlog.md`, `planning/architecture.md`.
