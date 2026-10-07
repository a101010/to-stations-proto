# Backlog - increment 1 (Rust MVP)

This backlog is ordered so that no story starts before its dependencies exist. Stories are individual deliverables; a group is an epic only when it is broken into multiple stories.

The increment target is the **minimum viable code product**: the `rust` hello_world constellation end to end. Milestones A-C install and smoke-test each piece of the toolchain and third-party stack; Milestones D-E build the MVP on top. Milestone F holds everything deferred to later increments. See `current_increment.md` for the locked scope and `architecture.md` for the design.

Conventions:
* **Depends on** lists prerequisite stories (toolchain installs are environment prerequisites, not steps inside `build.ps1`).
* **Minimal test** is the smallest runnable check that proves the story is done; toolchain smoke tests are temporary build artifacts under `build/`, with commands documented in `README.md`.
* IDs, topic names, and file paths follow `architecture.md`.

## Milestone A - host toolchain

### Story: A1 MSVC and CMake verification
Confirm the Visual Studio Build Tools C++ toolchain and CMake work from a shell, and document how to invoke them (VS generator or developer shell).
- **Depends on:** none.
- **Minimal test:** configure, build, and run a C++ hello world with the VS generator; record the exact commands in `README.md`.

### Story: A2 Rust toolchain
Install rustup with the `stable-x86_64-pc-windows-msvc` toolchain and pin it for the repository.
- **Depends on:** A1.
- **Minimal test:** `cargo run` a hello world; commit `rust/rust-toolchain.toml`.

### Story: A3 Repository short-path scripts
Add `scripts/subst-repo.ps1`, which maps the repository root to a short drive letter (`T:` by default), and `scripts/install-subst-startup.ps1`, which installs that mapping to run at logon via a Startup-folder wrapper. Neither is run by `scripts/build.ps1`.
- **Depends on:** none.
- **Minimal test:** `subst-repo.ps1` makes `subst` show `T:` pointing at the repository; `install-subst-startup.ps1` creates the Startup entry.

## Milestone B - DDS toolchain

### Story: B1 Eclipse Cyclone DDS C core
Build and install the Cyclone DDS C library (11.x) to the external prefix with IPv6 enabled.
- **Depends on:** A1.
- **Minimal test:** build with `-DBUILD_EXAMPLES=ON -DENABLE_IPV6=ON` and run the bundled HelloworldPublisher/HelloworldSubscriber.

### Story: B2 cyclonedds-cxx
Build and install the C++ binding against the B1 install.
- **Depends on:** B1.
- **Minimal test:** build and run the C++ hello world example.

### Story: B3 Rust Cyclone DDS binding and IDL codegen
Add the `cyclonedds` Rust crate (3.0.1) with `cyclonedds-build` (3.0.1) for IDL codegen, generating Rust types into `build/gen/rust`. The crate's copy of CycloneDDS C is built from the external prefix (`CYCLONEDDS_SRC`, `CYCLONEDDS_BUILD`) and linked statically so no runtime DLL is needed.
- **Depends on:** A2, A3, B1.
- **Minimal test:** a Rust publisher and subscriber exchange one sample of an IDL-defined type.

### Story: B4 DDS configuration and StationsDDS adapter
Author `DDS/cyclonedds-config.xml` (IPv6 multicast on `StationsDDS`) and `scripts/stationsdds.ps1` to create or verify the loopback adapter. Requires administrator rights; document the manual fallback in `docs/`.
- **Depends on:** B1, B3.
- **Minimal test:** a publisher and subscriber exchange samples bound to the adapter configuration; confirm IPv6 multicast.

### Story: B5 Third-party version pins
Record the pinned versions and install locations of every toolchain/third-party component in `docs/`.
- **Depends on:** B1, B2, B3, C1, C2.
- **Minimal test:** a fresh shell following `docs/` reproduces the B/C smoke tests.

## Milestone C - rendering toolchain

### Story: C1 SDL3, glad, and Dear ImGui
Install SDL3 to the external prefix and add glad and Dear ImGui sources under `cpp/third_party/`.
- **Depends on:** A1.
- **Minimal test:** a window that renders one Dear ImGui frame over a clear color.

### Story: C2 winit, glutin/glow, and egui
Add the Rust windowing/rendering crates.
- **Depends on:** A2.
- **Minimal test:** a window that renders one egui frame.

### Story: C3 Assets
Add the earth cube-map faces and a font under `assets/`.
- **Depends on:** none.
- **Minimal test:** both load and render/validate.

## Milestone D - project infrastructure

### Story: D1 Repository structure and ignore rules
Create the top-level folders and a `.gitignore` that excludes `build/`.
- **Depends on:** none.
- **Status:** done.

### Story: D2 Rust workspace skeleton
Create the `rust/` Cargo workspace with one compiling crate.
- **Depends on:** A2.
- **Minimal test:** `cargo build` from `rust/`.

### Story: D3 CMake project skeleton
Create the `cpp/` CMake project with one compiling target.
- **Depends on:** A1.
- **Minimal test:** configure and build one runnable target.

### Story: D4 Minimal IDL types
Author `DDS/hello_world.idl` (`Quaternion`, `HelloWorldModel`).
- **Depends on:** B2, B3.
- **Minimal test:** code generation succeeds for both languages.

### Story: D5 IDL code generation into both build systems
Wire Cyclone DDS IDL code generation into the Cargo build and the CMake build so generated sources land in `build/gen/{rust,cpp}` and compile into the `stations-dds` libraries. Generated files are never committed.
- **Depends on:** D2, D3, D4.
- **Minimal test:** a clean build produces the generated sources and compiles them.

### Story: D6 Minimal topic registry and service contracts
Author `contracts/topics.toml` and `contracts/services/<app>.toml` for the MVP services (`stations/hello_world/model`, `stations/hello_world/view`).
- **Depends on:** none.
- **Minimal test:** the files parse and validate against the registry.

### Story: D7 stations-dds library (Rust)
Implement participant creation from `DDS/cyclonedds-config.xml`, typed readers and writers, validation of each application's contract at startup, and a `--topics` flag.
- **Depends on:** B4, D5, D6.
- **Minimal test:** typed publish/subscribe round-trip plus `--topics` output.

### Story: D8 dds-echo tool (Rust)
Subscribe to any named topic and print received samples.
- **Depends on:** D7.
- **Minimal test:** prints samples from a publisher.

### Story: D9 dds-contracts tool (Rust)
Read `contracts/` and write a readable summary to `build/topics.md`.
- **Depends on:** D6.
- **Minimal test:** generated `topics.md` matches the registry.

### Story: D10 build orchestrator
Write `scripts/build.ps1` (PowerShell 5.1 compatible) that resolves the repository root from its own location and uses repository-relative paths: IDL codegen into `build/gen/{rust,cpp}`, the Rust build with `CARGO_TARGET_DIR=build/rust`, the CMake build from `build/cpp` with install prefix `build/deploy`, assembly of `build/deploy`, and regeneration of `build/topics.md`. It hardcodes no drive letter; the repository must be placed at a short path because the `cyclonedds` Rust crate's CycloneDDS CMake build can exceed the Windows 260-character path limit (see `architecture.md`). It locates the externally installed Cyclone DDS and SDL3 prefixes (hardcoded, for example `C:\Libraries\cyclonedds`) and provisions the runtime without changing the user PATH: it uses a process-scoped `$env:PATH` during build/test and copies the required runtime DLLs (`ddsc.dll`, `ddscxx.dll`, SDL3's) into `build/deploy/bin`. It supports a dry-run mode.
- **Depends on:** D2, D3, D5, D8, D9; B1-B3 installed as environment prerequisites.
- **Minimal test:** `scripts/build.ps1 -DryRun` prints the ordered steps, and a full run succeeds once the workspace and tools exist.

## Milestone E - MVP vertical slice (Rust)

### Story: E1 display library (Rust)
Create the window and OpenGL context, run the render loop, render the console (egui), render the `ViewData` it receives in a format window, and publish `InputEvent` for mouse, touchscreen, and key input.
- **Depends on:** C2, C3, D7.
- **Minimal test:** a window showing text that publishes an input event.

### Story: E2 small display (Rust)
Host the display library: one square format window plus a console area with a single power button, `--station N`, and `--topics`. The power button toggles the format window and emits an input event for later wiring.
- **Depends on:** E1.
- **Minimal test:** the power button toggles the format window.

### Story: E3 model service (Rust)
Publish `HelloWorldModel` (greeting text and quaternion attitude) at a fixed rate: a 23.5 degree tilt with one revolution every twelve minutes.
- **Depends on:** D5, D7.
- **Minimal test:** `dds-echo` shows `HelloWorldModel` samples.

### Story: E4 view service (Rust)
Subscribe to `HelloWorldModel` and publish `ViewData`: the earth cube map and sphere parameters, the model quaternion, the upper-right light direction, and the greeting text with its font.
- **Depends on:** D5, D6, D7.
- **Minimal test:** `dds-echo` shows `ViewData` samples.

### Story: E5 font handling (Rust)
Load a font from `assets/fonts` and render text (the hello_world greeting) inside the display library.
- **Depends on:** C3, E1.
- **Minimal test:** the greeting renders in the loaded font.

### Story: E6 earth format rendering (Rust)
Render the cube map on a sphere in the display library, orient it with the attitude quaternion from the `ViewData`, and light it from the upper right.
- **Depends on:** C3, E1, E4.
- **Minimal test:** the tilted, rotating, lit earth is visible on the small display.

### Story: E7 launch and cleanup scripts
Write `scripts/launch-rust.ps1` to start the MVP constellation from `build/deploy` and `scripts/cleanup.ps1` to stop all processes of all configurations.
- **Depends on:** D10, E1-E6.
- **Minimal test:** `launch-rust.ps1` brings up the constellation and `cleanup.ps1` stops it.

## Milestone F - later increments

Deferred until the Rust MVP is proven:
* C++ mirror of D7-E7 (`cpp/` libraries, tools, and apps) and the `c++` configuration.
* Hybrid configuration and `launch-hybrid.ps1`; `launch-cpp.ps1`.
* Remaining IDL types (`DDS/mvc.idl`, `DDS/controller.idl`) and full topic registry (input, control, joystick, format window/mapping).
* Joystick control service and controller window; real game-controller binding.
* Format manager and per-station views.
* Large display and launcher window.

## Mapping from the original backlog
* `repository structure and ignore rules` -> D1 (done).
* `build orchestrator` -> D10.
* `IDL code generation into both build systems` -> D5.
* `topic registry and service contracts` -> D6.
* `stations-dds library` -> D7.
* `dds-contracts tool` -> D9.
* `dds-echo tool` -> D8.
* `IDL types` -> D4 (minimal) then Milestone F (full).
* `display library` -> E1.
* `small display application` -> E2.
* `model service` -> E3.
* `view service` -> E4.
* `font handling` -> E5.
* `earth format rendering` -> E6.
* `launch and cleanup scripts` -> E7.
* `stationsdds setup` -> B4.
* `controller service`, `joystick control service`, `controller window`, `format manager`, C++/hybrid, `verification` -> Milestone F.
* New: Milestones A-C toolchain/third-party stories (including A3 short-path scripts), D2/D3 skeletons, B5 version pins.

## Done when
The increment's "Done when" is stated in `current_increment.md`.
