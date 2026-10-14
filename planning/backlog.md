# Backlog

All future work lives here. The backlog is continually groomed; the story order reflects the current best understanding of priority.

Conventions:
* A story's title is a short, unique name. The body is a description followed by `Depends on`, `Minimal test`, and `Status`.
* **Depends on** lists prerequisite stories that are not yet done (toolchain installs are environment prerequisites, not steps inside `build.ps1`).
* **Minimal test** is the smallest runnable check that proves the story is done; toolchain smoke tests are temporary build artifacts under `build/`.
* **Status** is one of `todo`, `in progress`, or `done`. The active story's detailed plan lives in `current_story.md`.
* IDs, topic names, and file paths follow `architecture.md`.

## Increment 1 - Rust MVP

**Goal.** The minimum viable code product: the `rust` hello_world constellation end to end on one machine.

**Scope.**
* Language: Rust only. C++ and hybrid are later increments.
* DDS: Eclipse Cyclone DDS; all topics over IPv6 multicast on the `StationsDDS` loopback adapter; shared IDL in `DDS/`.
* Rendering: winit + glutin/glow + egui.
* The model service and the view service are separate: the model publishes the greeting and the attitude (a 23.5 degree tilt, as a quaternion; one revolution every twelve minutes is added later); the view publishes the display message.
* Earth projection: a cube map sampled on a sphere, oriented by the attitude quaternion, lit from the upper right.
* Font handling (loading and rendering the greeting) is in scope.
* Small display (`lil-d`): one square format window plus a console area with a single power button, `--station N`, and `--topics`.
* The display library hosts a default format and the hello-world format and a format manager that switches the active format on a click in the region delegated to it.
* Input handling is added per feature; the first is the format-manager click.
* Out of scope: the C++/hybrid configurations and their launch scripts; joystick control service and `paws`; the format-manager controller service, the three-dot menu, and the format-chooser-format; `big-d` and `tentacles`; real game-controller binding; per-station views.

**Prerequisites** (environment, installed once, not built by `scripts/build.ps1`):
* MSVC (Visual Studio Build Tools) and CMake.
* Rust toolchain `stable-x86_64-pc-windows-msvc`.
* A short repository path on Windows (`scripts/subst-repo.ps1`; see `architecture.md`, "Path length").
* Eclipse Cyclone DDS C core (11.x) and the Microsoft loopback adapter named `StationsDDS` with IPv6 multicast.
* The to-stations fork of `cyclonedds-rust` (`https://github.com/a101010/cyclonedds-rust`, branch `multifile`, rev `acef684`), consumed as a pinned git dependency (`planning/cyclonedds-build-plan.md`).

Third-party libraries are built/installed to an external prefix (for example `C:\Libraries`), never committed. Each install story records its component in `docs/versions.md`.

**Done when.** `launch-rust.ps1` brings up the model service, the view service, and `lil-d` from `build/deploy`; `lil-d` shows the tilted, cube-mapped earth lit from the upper right with the greeting text rendered in the loaded font, its power button works, and the format manager switches between the default and hello-world formats. `dds-echo` observes every topic in the registry, `--topics` and `build/topics.md` match `contracts/`, and `cleanup.ps1` stops everything.

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
- **Depends on:** none.
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
- **Depends on:** none.
- **Minimal test:** build with `-DBUILD_EXAMPLES=ON -DENABLE_IPV6=ON` and run the bundled HelloworldPublisher/HelloworldSubscriber.
- **Status:** done.
- **Records:** Cyclone DDS C in `docs/versions.md`.

### cyclonedds-cxx
Build and install the C++ binding against the `cyclonedds-core` install.
- **Depends on:** none.
- **Minimal test:** build and run the C++ hello world example.
- **Status:** done.
- **Records:** cyclonedds-cxx in `docs/versions.md`.

### rust-dds-binding
Add the `cyclonedds` Rust crate and `cyclonedds-build` from the to-stations fork of `cyclonedds-rust` (branch `multifile`, rev `acef684`) as git dependencies pinned to that rev. The fork's copy of CycloneDDS C is built from the external prefix (`CYCLONEDDS_SRC`, `CYCLONEDDS_BUILD`) and linked statically so no runtime DLL is needed. IDL codegen is deferred to `hello-world-idl`; this story uses a hand-written `#[derive(DdsType)]` type.
- **Depends on:** none.
- **Minimal test:** a Rust publisher and subscriber exchange one sample of an IDL-defined type.
- **Status:** done.
- **Records:** the `cyclonedds`/`cyclonedds-build` fork in `docs/versions.md`.

### dds-config
Author `DDS/cyclonedds-config.xml` (IPv6 multicast on `StationsDDS`) and `scripts/verify-loopback.ps1` to verify the loopback adapter and its IPv6 prerequisites. Creating the adapter requires administrator rights and is a documented manual step; document it in `docs/`.
- **Depends on:** none.
- **Minimal test:** a publisher and subscriber exchange samples bound to the adapter configuration; confirm IPv6 multicast.
- **Status:** done.

### rust-workspace
Create the `rust/` Cargo workspace with one compiling crate.
- **Depends on:** none.
- **Minimal test:** `cargo build` from `rust/`.
- **Status:** done.

### rust-render
Add the Rust windowing/rendering crates.
- **Depends on:** none.
- **Minimal test:** a window that renders one egui frame.
- **Status:** done.
- **Records:** the windowing/rendering crates in `docs/versions.md`.

### assets
Add the earth cube-map faces and a font under `assets/`.
- **Depends on:** none.
- **Minimal test:** both load and render/validate.
- **Status:** done.

### hello-world-idl
Author the hello_world IDL as nested IDL modules and generate Rust into `build/gen/rust` with `scripts/build.ps1`, which runs the fork's `cyclonedds-idlc` Rust codegen (`--no-idlc`) and then builds the Rust workspace. `DDS/math.idl` holds `module math { struct Quaternion { double x; double y; double z; double w; }; };`; `DDS/hello_world.idl` (the compiled root) is `#include "math.idl"` then `module hello_world { struct HelloWorldTopic { string greeting; math::Quaternion attitude; }; };`. One top-level `module` per file, all of that module's types in that one file (a module is not reopened across files); nested IDL `module` maps to a Rust `pub mod`; the DDS type name is the fully-qualified IDL scope (`#[dds_typename]`). The crate has no `build.rs`; it `include!`s the generated file, located via the `REPO_ROOT` environment variable (no `..` paths; Cargo supplies `REPO_ROOT` from `.cargo/config.toml`). The crate moves to `rust/lib/stations-dds`. Generated files are never committed.
- **Depends on:** none.
- **Minimal test:** `scripts/build.ps1 -Test` generates `build/gen/rust/hello_world.rs`, builds, and passes with `hello_world::HelloWorldTopic::type_name() == "hello_world::HelloWorldTopic"` and `math::Quaternion::type_name() == "math::Quaternion"`.
- **Status:** done.

### lil-d
Create the small display application (`lil-d`): a window and an egui console area, with the region delegated to the display library painted a dark unsaturated green placeholder. The console has a single power button that toggles the format window. Takes `--station N` on the command line. Extends `scripts/build.ps1` to build and deploy the executable to `build/deploy`, and adds `scripts/launch-rust.ps1` and `scripts/cleanup.ps1`; later stories that add executables or libraries extend these scripts as needed.
- **Depends on:** none.
- **Minimal test:** the window opens showing the console and the green region; the power button toggles the format window; `launch-rust.ps1` starts it and `cleanup.ps1` stops it.
- **Status:** todo.

### display-lib
Create the display library (`display-lib`) and have `lil-d` delegate the format-window region to it. It renders the default format, hardcoded: a solid color (different from the original `lil-d` color) with an x in a different color, owning the entire region delegated to the library.
- **Depends on:** lil-d.
- **Minimal test:** the delegated region shows the default format.
- **Status:** todo.

### font-handling
Load a font from `assets/fonts` and render text (the hello_world greeting) in the display library as part of the hello-world format. The greeting and attitude are hardcoded for now.
- **Depends on:** display-lib.
- **Minimal test:** the greeting renders in the loaded font.
- **Status:** todo.

### earth-rendering
Render the cube map on a sphere in the display library, orient it with the attitude quaternion, and light it from the upper right, completing the hello-world format. The cube map, attitude, and light direction are hardcoded for now.
- **Depends on:** display-lib.
- **Minimal test:** the tilted, lit earth is visible in the delegated region.
- **Status:** todo.

### format-manager
Add the format manager to the display library: a registry of the default and hello-world formats and the active format. `lil-d` forwards mouse and other window input events to the library; clicking in the region delegated to the display library switches the active format.
- **Depends on:** display-lib, font-handling, earth-rendering.
- **Minimal test:** clicking in the delegated region switches between the default and hello-world formats.
- **Status:** todo.

### dds-and-service-contracts
Define the DDS layer and the service contracts in `stations-dds`: create the participant from `DDS/cyclonedds-config.xml` and typed readers/writers; define the topic-contract schema and loader for `contracts/topics.toml` and `contracts/services/<app>.toml`; validate each service's declared topics at startup; support a `--topics` flag that prints a service's contract; and write a readable summary to `build/topics.md`. Provisional: the nature and scope of the DDS layer and the contracts still need refinement, and this may be split again.
- **Depends on:** none.
- **Minimal test:** a typed publish/subscribe round-trip; a sample contract loads, validates, prints with `--topics`, and matches `build/topics.md`.
- **Status:** todo.

### dds-echo
Subscribe to any named topic and print received samples.
- **Depends on:** dds-and-service-contracts.
- **Minimal test:** prints samples from a publisher.
- **Status:** todo.

### hello-view-service
Create the hello_world view service (`hello-view-service`): add the trivial `HelloWorldViewTopic` to `DDS/hello_world.idl` (an exact copy of `HelloWorldTopic`; rotation is added later) and publish it; extend the display library to subscribe to `HelloWorldViewTopic` and render the hello-world format from the received greeting and attitude instead of the hardcoded values. The model data moves out of the display library to this service; later `hello-service` becomes its source.
- **Depends on:** font-handling, earth-rendering, dds-and-service-contracts.
- **Minimal test:** `dds-echo` shows `HelloWorldViewTopic` samples and the display renders the published view.
- **Status:** todo.

### hello-service
Create the hello_world model service (`hello-service`): publish `HelloWorldTopic` (greeting text and quaternion attitude) at a fixed rate, a 23.5 degree tilt. The model data moves out of the view service to this service, and the view service now subscribes to it and relays. One revolution every twelve minutes is added later.
- **Depends on:** dds-and-service-contracts.
- **Minimal test:** `dds-echo` shows `HelloWorldTopic` samples and the display shows the model-driven world.
- **Status:** todo.

## Later increments

### format-ctl-service
Create the format-manager controller service: the controller of the per-display format-manager constellation, controlling which format each display shows.
- **Depends on:** format-manager.
- **Minimal test:** the service controls the active format on a display.
- **Status:** todo.

### format-menu
Add a three-dot menu to the format manager, drawn in the upper left over all formats, with one entry per format.
- **Depends on:** format-ctl-service.
- **Minimal test:** the menu switches the active format.
- **Status:** todo.

### format-chooser-format
Create the format-chooser-format: a format for choosing formats.
- **Depends on:** format-menu.
- **Minimal test:** a format can be chosen from the format.
- **Status:** todo.

### hello-ctl-service
Create the hello_world control service (`hello-ctl-service`): consume input events from the display libraries; user rotations and changes of the light vector and color, to be refined later.
- **Depends on:** hello-view-service.
- **Minimal test:** input events change the displayed world.
- **Status:** todo.

### big-d
Create the large display application (`big-d`): a large rectangular window with three square format windows and an eyebrow window, surrounded by a console.
- **Depends on:** display-lib.
- **Minimal test:** the window opens with three format windows and the eyebrow.
- **Status:** todo.

### tentacles
Create the launcher application (`tentacles`): a window that can start and stop the applications for a configuration.
- **Depends on:** none.
- **Minimal test:** the launcher starts and stops a configuration.
- **Status:** todo.

### paws
Create the game-controller window application (`paws`): a window with a virtual controller that can be bound to a game controller.
- **Depends on:** none.
- **Minimal test:** the window shows a virtual controller.
- **Status:** todo.

### sdl3-imgui
Install SDL3 to the external prefix and add glad and Dear ImGui sources under `cpp/third_party/`.
- **Depends on:** none.
- **Minimal test:** a window that renders one Dear ImGui frame over a clear color.
- **Status:** todo.
- **Records:** SDL3, glad, and Dear ImGui in `docs/versions.md`.

### cmake-skeleton
Create the `cpp/` CMake project with one compiling target.
- **Depends on:** none.
- **Minimal test:** configure and build one runnable target.
- **Status:** todo.

Not yet broken into stories:
* C++ mirror of the Rust libraries, tools, and apps and the `c++` configuration.
* Hybrid configuration and `launch-hybrid.ps1`; `launch-cpp.ps1`.
* Remaining top-level module IDL (control, joystick, per-station/display) and the full topic registry (input, control, joystick, format window/mapping).
* Joystick control service and real game-controller binding.
* Per-station views.
