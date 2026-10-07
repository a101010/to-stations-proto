# Increment 1 - Rust MVP

Increment 1 delivers the **minimum viable code product**: the `rust` configuration of the hello_world constellation, running end to end on one machine. It exists to retire the highest-risk unknowns first (host toolchain, Cyclone DDS on Windows, IPv6 multicast over the loopback adapter, and the rendering stack) in a single language before any C++ or hybrid work begins.

The story breakdown and dependency order are in `backlog.md`. The design is in `architecture.md`.

---

# Locked scope for increment 1

## Prerequisites (environment, not built by `scripts/build.ps1`)
These are installed once and verified by their own stories in Milestone A-C of `backlog.md`. `build.ps1` only locates them.

* MSVC (Visual Studio Build Tools) and CMake.
* Rust toolchain `stable-x86_64-pc-windows-msvc`.
* Eclipse Cyclone DDS C core (11.x) and `cyclonedds-cxx`.
* The Microsoft loopback adapter named `StationsDDS` with IPv6 multicast.
* C++ rendering libraries (SDL3, glad, Dear ImGui) - needed by Milestone C tests and by later C++ increments, not by the Rust MVP itself.

Third-party libraries are built/installed to an external prefix (for example `C:\Libraries`), never committed. Versions are pinned and documented in `docs/`.

## Stack
* DDS: Eclipse Cyclone DDS for all configurations - C core with `cyclonedds-cxx` (C++) and the `cyclonedds` Rust crate; shared IDL in `DDS/`; all topics over IPv6 multicast on the `StationsDDS` loopback adapter.
* Rendering: Rust = winit + glutin/glow + egui. C++ (later increment) = SDL3 + glad + Dear ImGui.
* Build: `scripts/build.ps1` orchestrator; all artifacts under `build/` split into `scratch/` and `deploy/`.
* Language: **Rust only** in this increment. C++ and hybrid are later increments.

## MVP constellation
* The model service and the view service are separate services: the model service publishes the greeting and the attitude; the view service turns that into the display message the display library renders.
* Attitude: a virtual world tilted 23.5 degrees and rotating about its axis once every twelve minutes, published as a quaternion.
* The format is global (system-wide) in this increment; per-station view selection is a later increment.
* Earth projection: a cube map of six faces sampled on a sphere, oriented by the attitude quaternion, lit from the upper right.
* Display library input events are published as `InputEvent`; wiring them into a controller service is a later increment.
* Font handling (loading and rendering the greeting text) is part of this increment.

## Displays and controls
* Small display: one square format window plus a console area containing a single power button; `--station N` on the command line; `--topics` support.
* No joystick, controller window, format manager, or large display in this increment.

## Out of scope
* The C++ and hybrid configurations and their launch scripts.
* Joystick control service, controller window, format manager, large display, and launcher window.
* Real game-controller binding and per-station views.

## Done when
`launch-rust.ps1` brings up the model service, the view service, and the small display from `build/deploy`; the small display shows the tilted, rotating, cube-mapped earth lit from the upper right with the greeting text rendered in the loaded font, and its power button works. `dds-echo` observes every topic in the registry, `--topics` and `build/scratch/topics.md` match `contracts/`, and `cleanup.ps1` stops everything.
