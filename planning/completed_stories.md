# Completed stories

Stories that are done, moved out of `backlog.md` so it stays focused on remaining work. Grouped by increment and listed in the order they were completed; each timestamp is the local commit time (UTC-06:00).

## Increment 1 - Rust MVP

### repo-structure
Create the top-level folders and a `.gitignore` that excludes `build/`.
- **Completed:** 2026-10-06 19:03:27.
- **Depends on:** none.

### host-toolchain
Confirm the Visual Studio Build Tools C++ toolchain and CMake work from a shell, and document how to invoke them (VS generator or developer shell).
- **Completed:** 2026-10-06 19:36:42.
- **Depends on:** none.
- **Minimal test:** configure, build, and run a C++ hello world with the VS generator; record the exact commands in `README.md`.
- **Records:** MSVC and CMake in `docs/versions.md`.

### rust-toolchain
Install rustup with the `stable-x86_64-pc-windows-msvc` toolchain and pin it for the repository.
- **Completed:** 2026-10-06 19:52:21.
- **Depends on:** none.
- **Minimal test:** `cargo run` a hello world; commit `rust/rust-toolchain.toml`.
- **Records:** rustup/rustc in `docs/versions.md`.

### cyclonedds-core
Build and install the Cyclone DDS C library (11.x) to the external prefix with IPv6 enabled.
- **Completed:** 2026-10-06 20:08:07.
- **Depends on:** none.
- **Minimal test:** build with `-DBUILD_EXAMPLES=ON -DENABLE_IPV6=ON` and run the bundled HelloworldPublisher/HelloworldSubscriber.
- **Records:** Cyclone DDS C in `docs/versions.md`.

### short-path
Add `scripts/subst-repo.ps1`, which maps the repository root to a short drive letter (`T:` by default), and `scripts/install-subst-startup.ps1`, which installs that mapping to run at logon via a Startup-folder wrapper. Neither is run by `scripts/build.ps1`.
- **Completed:** 2026-10-06 22:44:44.
- **Depends on:** none.
- **Minimal test:** `subst-repo.ps1` makes `subst` show `T:` pointing at the repository; `install-subst-startup.ps1` creates the Startup entry.

### cyclonedds-cxx
Build and install the C++ binding against the `cyclonedds-core` install.
- **Completed:** 2026-10-06 22:44:44.
- **Depends on:** none.
- **Minimal test:** build and run the C++ hello world example.
- **Records:** cyclonedds-cxx in `docs/versions.md`.

### dds-config
Author `DDS/cyclonedds-config.xml` (IPv6 multicast on `StationsDDS`) and `scripts/verify-loopback.ps1` to verify the loopback adapter and its IPv6 prerequisites. Creating the adapter requires administrator rights and is a documented manual step; document it in `docs/`.
- **Completed:** 2026-10-07 12:23:07.
- **Depends on:** none.
- **Minimal test:** a publisher and subscriber exchange samples bound to the adapter configuration; confirm IPv6 multicast.

### rust-workspace
Create the `rust/` Cargo workspace with one compiling crate.
- **Completed:** 2026-10-07 13:42:22.
- **Depends on:** none.
- **Minimal test:** `cargo build` from `rust/`.

### rust-render
Add the Rust windowing/rendering crates.
- **Completed:** 2026-10-07 14:23:28.
- **Depends on:** none.
- **Minimal test:** a window that renders one egui frame.
- **Records:** the windowing/rendering crates in `docs/versions.md`.

### assets
Add the earth cube-map faces and a font under `assets/`.
- **Completed:** 2026-10-08 09:07:36.
- **Depends on:** none.
- **Minimal test:** both load and render/validate.

### rust-dds-binding
Add the `cyclonedds` Rust crate and `cyclonedds-build` from the to-stations fork of `cyclonedds-rust` (branch `multifile`, rev `acef684`) as git dependencies pinned to that rev. The fork's copy of CycloneDDS C is built from the external prefix (`CYCLONEDDS_SRC`, `CYCLONEDDS_BUILD`) and linked statically so no runtime DLL is needed. IDL codegen is deferred to `hello-world-idl`; this story uses a `#[derive(DdsType)]` type.
- **Completed:** 2026-10-09 16:37:58.
- **Depends on:** none.
- **Minimal test:** a Rust publisher and subscriber exchange one sample of an IDL-defined type.
- **Records:** the `cyclonedds`/`cyclonedds-build` fork in `docs/versions.md`.

### hello-world-idl
Author the hello_world IDL as nested IDL modules and generate Rust into `build/gen/rust` with `scripts/build.ps1`, which runs the fork's `cyclonedds-idlc` Rust codegen (`--no-idlc`) and then builds the Rust workspace. `DDS/math.idl` holds `module math { struct Quaternion { double x; double y; double z; double w; }; };`; `DDS/hello_world.idl` (the compiled root) is `#include "math.idl"` then `module hello_world { struct HelloWorldTopic { string greeting; math::Quaternion attitude; }; };`. One top-level `module` per file, all of that module's types in that one file (a module is not reopened across files); nested IDL `module` maps to a Rust `pub mod`; the DDS type name is the fully-qualified IDL scope (`#[dds_typename]`). The crate has no `build.rs`; it `include!`s the generated file, located via the `REPO_ROOT` environment variable (no `..` paths; Cargo supplies `REPO_ROOT` from `.cargo/config.toml`). The crate moves to `rust/lib/stations-dds`. Generated files are never committed.
- **Completed:** 2026-10-13 12:27:30.
- **Depends on:** none.
- **Minimal test:** `scripts/build.ps1 -Test` generates `build/gen/rust/hello_world.rs`, builds, and passes with `hello_world::HelloWorldTopic::type_name() == "hello_world::HelloWorldTopic"` and `math::Quaternion::type_name() == "math::Quaternion"`.

### lil-d
Create the small display application (`lil-d`): a window and an egui console area, with the region delegated to the display library painted a dark unsaturated green placeholder. The console has a single power button that toggles the format window. Takes `--station N` on the command line. Extends `scripts/build.ps1` to build and deploy the executable to `build/deploy`, and adds `scripts/launch-rust.ps1` and `scripts/cleanup.ps1`; later stories that add executables or libraries extend these scripts as needed.
- **Completed:** 2026-10-14 10:25:12.
- **Depends on:** none.
- **Minimal test:** the window opens showing the console and the green region; the power button toggles the format window; `launch-rust.ps1` starts it and `cleanup.ps1` stops it.

### display-lib
Create the display library (`display-lib`) as a C-ABI DLL and have `lil-d` load it and delegate the format-window region to it. It renders the default format, hardcoded: a solid color (different from the original `lil-d` color) with an x in a different color, owning the entire region delegated to the library. The ABI is the shared header `include/display_lib.h` (implemented in Rust and, later, C++), so a display app in either language can use a display library in either language (the hybrid deployment's in-process interop); `lil-d` selects the DLL with `--display-lib <path>`.
- **Completed:** 2026-10-14 16:18:27.
- **Depends on:** lil-d.
- **Minimal test:** the delegated region shows the default format when `lil-d` loads `display_lib.dll` through the C ABI.

### harfbuzz
Build and install HarfBuzz 14.6.0 with the experimental GPU library (`libharfbuzz-gpu`) to the external prefix, and add the Rust FFI crate `rust/lib/harfbuzz` (raw bindings plus thin wrappers) that links it. The CMake port builds and installs `harfbuzz` and `harfbuzz-gpu`; `hb-gpu.h` is installed under `include/harfbuzz`.
- **Completed:** 2026-10-15 15:36:42.
- **Depends on:** none.
- **Minimal test:** a temporary smoke program under `build/` shapes "Hello, world!" with FiraSans, encodes a glyph with `hb_gpu_draw_encode`, and asserts a non-empty blob and a non-null GLSL fragment shader source.
- **Records:** HarfBuzz (and `libharfbuzz-gpu`) in `docs/versions.md`.

### font-handling
Load a font from `assets/fonts` in the display library and render the hello_world greeting with HarfBuzz (`hb-gpu`) as part of the hello-world format. The greeting is hardcoded for now.
- **Completed:** 2026-10-15 17:01:35.
- **Depends on:** display-lib, harfbuzz.
- **Minimal test:** the greeting renders in the loaded font.

### earth-rendering
Render the cube map on a sphere in the display library, orient it with the attitude quaternion, and light it from the upper right, completing the hello-world format. The cube map, attitude, and light direction are hardcoded for now.
- **Completed:** 2026-10-20 10:05:40.
- **Depends on:** display-lib.
- **Minimal test:** the tilted, lit earth is visible in the delegated region.

### format-manager
Add the format manager to the display library: a registry of the default and hello-world formats and the active format. `lil-d` forwards mouse and other window input events to the library; clicking in the region delegated to the display library switches the active format.
- **Completed:** 2026-10-20 10:43:08.
- **Depends on:** display-lib, font-handling, earth-rendering.
- **Minimal test:** clicking in the delegated region switches between the default and hello-world formats.

### dds-and-service-contracts
Define the DDS layer and the service contracts in `stations-dds`: create the participant from `DDS/cyclonedds-config.xml` and typed readers/writers; define the topic-contract schema and loader for `contracts/topics.toml` and `contracts/services/<app>.toml`; validate each service's declared topics at startup; support a `--topics` flag that prints a service's contract; and write a readable summary to `build/topics.md`. Provisional: the nature and scope of the DDS layer and the contracts still need refinement, and this may be split again.
- **Completed:** 2026-10-21 11:28:57.
- **Depends on:** none.
- **Minimal test:** a typed publish/subscribe round-trip; a sample contract loads, validates, prints with `--topics`, and matches `build/topics.md`.
