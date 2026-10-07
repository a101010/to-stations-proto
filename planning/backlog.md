# Backlog

All future work lives here. The backlog is continually groomed; the story order reflects the current best understanding of priority.

Conventions:
* A story's title is a short, unique name. The body is a description followed by `Depends on`, `Minimal test`, and `Status`.
* **Depends on** lists prerequisite stories (toolchain installs are environment prerequisites, not steps inside `build.ps1`).
* **Minimal test** is the smallest runnable check that proves the story is done; toolchain smoke tests are temporary build artifacts under `build/`.
* **Status** is one of `todo`, `in progress`, or `done`. The active story's detailed plan lives in `current_story.md`.
* IDs, topic names, and file paths follow `architecture.md`.

## Increment 1 - Rust MVP

**Goal.** The minimum viable code product: the `rust` hello_world constellation end to end on one machine.

**Scope.**
* Language: Rust only. C++ and hybrid are later increments.
* DDS: Eclipse Cyclone DDS; all topics over IPv6 multicast on the `StationsDDS` loopback adapter; shared IDL in `DDS/`.
* Rendering: winit + glutin/glow + egui.
* The model service and the view service are separate: the model publishes the greeting and the attitude (a 23.5 degree tilt, one revolution every twelve minutes, as a quaternion); the view publishes the display message.
* Earth projection: a cube map sampled on a sphere, oriented by the attitude quaternion, lit from the upper right.
* Font handling (loading and rendering the greeting) is in scope.
* Small display: one square format window plus a console area with a single power button, `--station N`, and `--topics`.
* Input events are published as `InputEvent`; wiring them into a controller is a later increment.
* Out of scope: the C++/hybrid configurations and their launch scripts; joystick control service, controller window, format manager, large display, launcher; real game-controller binding; per-station views.

**Prerequisites** (environment, installed once, not built by `scripts/build.ps1`):
* MSVC (Visual Studio Build Tools) and CMake.
* Rust toolchain `stable-x86_64-pc-windows-msvc`.
* A short repository path on Windows (`scripts/subst-repo.ps1`; see `architecture.md`, "Path length").
* Eclipse Cyclone DDS C core (11.x) and the Microsoft loopback adapter named `StationsDDS` with IPv6 multicast.

Third-party libraries are built/installed to an external prefix (for example `C:\Libraries`), never committed. Each install story records its component in `docs/versions.md`.

**Done when.** `launch-rust.ps1` brings up the model service, the view service, and the small display from `build/deploy`; the small display shows the tilted, rotating, cube-mapped earth lit from the upper right with the greeting text rendered in the loaded font, and its power button works. `dds-echo` observes every topic in the registry, `--topics` and `build/topics.md` match `contracts/`, and `cleanup.ps1` stops everything.

### repo-structure
Create the top-level folders and a `.gitignore` that excludes `build/`.
- **Depends on:** none.
- **Status:** done.

### host-toolchain
Confirm the Visual Studio Build Tools C++ toolchain and CMake work from a shell, and document how to invoke them (VS generator or developer shell).
- **Depends on:** none.
- **Minimal test:** configure, build, and run a C++ hello world with the VS generator; record the exact commands in `README.md`.
- **Status:** done.
- **Records:** MSVC and CMake in `docs/versions.md`.

### rust-toolchain
Install rustup with the `stable-x86_64-pc-windows-msvc` toolchain and pin it for the repository.
- **Depends on:** host-toolchain.
- **Minimal test:** `cargo run` a hello world; commit `rust/rust-toolchain.toml`.
- **Status:** done.
- **Records:** rustup/rustc in `docs/versions.md`.

### short-path
Add `scripts/subst-repo.ps1`, which maps the repository root to a short drive letter (`T:` by default), and `scripts/install-subst-startup.ps1`, which installs that mapping to run at logon via a Startup-folder wrapper. Neither is run by `scripts/build.ps1`.
- **Depends on:** none.
- **Minimal test:** `subst-repo.ps1` makes `subst` show `T:` pointing at the repository; `install-subst-startup.ps1` creates the Startup entry.
- **Status:** done.

### cyclonedds-core
Build and install the Cyclone DDS C library (11.x) to the external prefix with IPv6 enabled.
- **Depends on:** host-toolchain.
- **Minimal test:** build with `-DBUILD_EXAMPLES=ON -DENABLE_IPV6=ON` and run the bundled HelloworldPublisher/HelloworldSubscriber.
- **Status:** done.
- **Records:** Cyclone DDS C in `docs/versions.md`.

### cyclonedds-cxx
Build and install the C++ binding against the `cyclonedds-core` install.
- **Depends on:** cyclonedds-core.
- **Minimal test:** build and run the C++ hello world example.
- **Status:** done.
- **Records:** cyclonedds-cxx in `docs/versions.md`.

### rust-dds-binding
Add the `cyclonedds` Rust crate (3.0.1) with `cyclonedds-build` (3.0.1) for IDL codegen, generating Rust types into `build/gen/rust`. The crate's copy of CycloneDDS C is built from the external prefix (`CYCLONEDDS_SRC`, `CYCLONEDDS_BUILD`) and linked statically so no runtime DLL is needed.
- **Depends on:** rust-toolchain, short-path, cyclonedds-core.
- **Minimal test:** a Rust publisher and subscriber exchange one sample of an IDL-defined type.
- **Status:** done.
- **Records:** the `cyclonedds` crate in `docs/versions.md`.

### dds-config
Author `DDS/cyclonedds-config.xml` (IPv6 multicast on `StationsDDS`) and `scripts/verify-loopback.ps1` to verify the loopback adapter and its IPv6 prerequisites. Creating the adapter requires administrator rights and is a documented manual step; document it in `docs/`.
- **Depends on:** cyclonedds-core, rust-dds-binding.
- **Minimal test:** a publisher and subscriber exchange samples bound to the adapter configuration; confirm IPv6 multicast.
- **Status:** done.

### rust-workspace
Create the `rust/` Cargo workspace with one compiling crate.
- **Depends on:** rust-toolchain.
- **Minimal test:** `cargo build` from `rust/`.
- **Status:** done.

### rust-render
Add the Rust windowing/rendering crates.
- **Depends on:** rust-toolchain, rust-workspace.
- **Minimal test:** a window that renders one egui frame.
- **Status:** in progress.
- **Records:** the windowing/rendering crates in `docs/versions.md`.

### assets
Add the earth cube-map faces and a font under `assets/`.
- **Depends on:** none.
- **Minimal test:** both load and render/validate.
- **Status:** todo.

### idl-types
Author `DDS/hello_world.idl` (`Quaternion`, `HelloWorldModel`).
- **Depends on:** rust-dds-binding.
- **Minimal test:** code generation succeeds.
- **Status:** todo.

### idl-codegen
Wire Cyclone DDS IDL code generation into the Cargo build so generated sources land in `build/gen/rust` and compile into the `stations-dds` library. Generated files are never committed.
- **Depends on:** rust-workspace, idl-types.
- **Minimal test:** a clean build produces the generated sources and compiles them.
- **Status:** todo.

### topic-contracts
Author `contracts/topics.toml` and `contracts/services/<app>.toml` for the MVP services (`stations/hello_world/model`, `stations/hello_world/view`).
- **Depends on:** none.
- **Minimal test:** the files parse and validate against the registry.
- **Status:** todo.

### stations-dds
Implement participant creation from `DDS/cyclonedds-config.xml`, typed readers and writers, validation of each application's contract at startup, and a `--topics` flag.
- **Depends on:** dds-config, idl-codegen, topic-contracts.
- **Minimal test:** typed publish/subscribe round-trip plus `--topics` output.
- **Status:** todo.

### dds-echo
Subscribe to any named topic and print received samples.
- **Depends on:** stations-dds.
- **Minimal test:** prints samples from a publisher.
- **Status:** todo.

### dds-contracts
Read `contracts/` and write a readable summary to `build/topics.md`.
- **Depends on:** topic-contracts.
- **Minimal test:** generated `topics.md` matches the registry.
- **Status:** todo.

### build-orchestrator
Write `scripts/build.ps1` (PowerShell 5.1 compatible) that resolves the repository root from its own location and uses repository-relative paths: IDL codegen into `build/gen/rust`, the Rust build with `CARGO_TARGET_DIR=build/rust`, assembly of `build/deploy`, and regeneration of `build/topics.md`. It hardcodes no drive letter; the repository must be placed at a short path because the `cyclonedds` Rust crate's CycloneDDS CMake build can exceed the Windows 260-character path limit (see `architecture.md`). It locates the externally installed Cyclone DDS prefix (hardcoded, for example `C:\Libraries\cyclonedds`) and provisions the runtime without changing the user PATH: a process-scoped `$env:PATH` during build/test. It supports a dry-run mode. The CMake/SDL3 halves are added when the C++ increment is scheduled.
- **Depends on:** rust-workspace, idl-codegen, dds-echo, dds-contracts.
- **Minimal test:** `scripts/build.ps1 -DryRun` prints the ordered steps, and a full run succeeds once the workspace and tools exist.
- **Status:** todo.

### display-lib
Create the window and OpenGL context, run the render loop, render the console (egui), render the `ViewData` it receives in a format window, and publish `InputEvent` for mouse, touchscreen, and key input.
- **Depends on:** rust-render, assets, stations-dds.
- **Minimal test:** a window showing text that publishes an input event.
- **Status:** todo.

### small-display
Host the display library: one square format window plus a console area with a single power button, `--station N`, and `--topics`. The power button toggles the format window and emits an input event for later wiring.
- **Depends on:** display-lib.
- **Minimal test:** the power button toggles the format window.
- **Status:** todo.

### model-service
Publish `HelloWorldModel` (greeting text and quaternion attitude) at a fixed rate: a 23.5 degree tilt with one revolution every twelve minutes.
- **Depends on:** idl-codegen, stations-dds.
- **Minimal test:** `dds-echo` shows `HelloWorldModel` samples.
- **Status:** todo.

### view-service
Subscribe to `HelloWorldModel` and publish `ViewData`: the earth cube map and sphere parameters, the model quaternion, the upper-right light direction, and the greeting text with its font.
- **Depends on:** idl-codegen, topic-contracts, stations-dds.
- **Minimal test:** `dds-echo` shows `ViewData` samples.
- **Status:** todo.

### font-handling
Load a font from `assets/fonts` and render text (the hello_world greeting) inside the display library.
- **Depends on:** assets, display-lib.
- **Minimal test:** the greeting renders in the loaded font.
- **Status:** todo.

### earth-rendering
Render the cube map on a sphere in the display library, orient it with the attitude quaternion from the `ViewData`, and light it from the upper right.
- **Depends on:** assets, display-lib, view-service.
- **Minimal test:** the tilted, rotating, lit earth is visible on the small display.
- **Status:** todo.

### launch-cleanup
Write `scripts/launch-rust.ps1` to start the MVP constellation from `build/deploy` and `scripts/cleanup.ps1` to stop all processes of all configurations.
- **Depends on:** build-orchestrator, display-lib, small-display, model-service, view-service, font-handling, earth-rendering.
- **Minimal test:** `launch-rust.ps1` brings up the constellation and `cleanup.ps1` stops it.
- **Status:** todo.

## Later increments

### sdl3-imgui
Install SDL3 to the external prefix and add glad and Dear ImGui sources under `cpp/third_party/`.
- **Depends on:** host-toolchain.
- **Minimal test:** a window that renders one Dear ImGui frame over a clear color.
- **Status:** todo.
- **Records:** SDL3, glad, and Dear ImGui in `docs/versions.md`.

### cmake-skeleton
Create the `cpp/` CMake project with one compiling target.
- **Depends on:** host-toolchain.
- **Minimal test:** configure and build one runnable target.
- **Status:** todo.

Not yet broken into stories:
* C++ mirror of the Rust libraries, tools, and apps and the `c++` configuration.
* Hybrid configuration and `launch-hybrid.ps1`; `launch-cpp.ps1`.
* Remaining IDL types (`DDS/mvc.idl`, `DDS/controller.idl`) and full topic registry (input, control, joystick, format window/mapping).
* Joystick control service and controller window; real game-controller binding.
* Format manager and per-station views.
* Large display and launcher window.
