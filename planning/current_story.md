# Current story: lil-d

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the small display application (`lil-d`): a window and an egui console area, with the region delegated to the display library painted a dark unsaturated green placeholder. The console has a single power button that toggles the format window. Takes `--station N` on the command line. Extends `scripts/build.ps1` to build and deploy the executable to `build/deploy`, and adds `scripts/launch-rust.ps1` and `scripts/cleanup.ps1`; later stories that add executables or libraries extend these scripts as needed.

- **Depends on:** none.
- **Minimal test:** the window opens showing the console and the green region; the power button toggles the format window; `launch-rust.ps1` starts it and `cleanup.ps1` stops it.

## Context

`rust-render` proved the windowing/rendering stack (winit 0.30.13, glutin 0.32.3, glutin-winit 0.5.0, glow 0.17, egui/egui-winit/egui_glow 0.36.2) with a temporary smoke test under `build/rust-render-smoke`; that directory is gitignored and is the reference for the window/GL/egui setup. No committed display crate exists: `rust/` holds only `lib/stations-dds` and `tool/equirect-to-cubemap`, and there is no `rust/ui/` yet.

`scripts/build.ps1` currently generates IDL into `build/gen/rust` and builds the Rust workspace with the Cargo target directory `build/rust`; it has no deploy step, and `scripts/launch-rust.ps1` and `scripts/cleanup.ps1` do not exist.

`lil-d` is the display application that will host the display library. In this story the display library does not exist yet, so the region it will own is painted a placeholder color.

## Decisions

* **Crate.** `lil-d` lives at `rust/ui/lil-d`, a binary crate (`architecture.md` puts GUI applications under `ui/`).
* **Stack reuse.** Follow the smoke test's winit + glutin/glow + egui_glow setup; the windowing/rendering dependencies come from `[workspace.dependencies]` in `rust/Cargo.toml`.
* **Window.** One OS window: a square format region (the region that will be delegated to the display library) painted a named dark unsaturated green constant, with the console area around it. The power button lives in the console and toggles the format region on/off.
* **Arguments.** `--station N` via `clap`, default `0`. `--topics` is not added yet; it arrives with `dds-and-service-contracts`.
* **No DDS, no `InputEvent`, no display library.** `lil-d` does not forward input yet; the display application forwards input to the display library when `format-manager` needs it.
* **Build and deploy are script-owned and grow with the stories.** `scripts/build.ps1` gains a step that assembles `build/deploy/bin` with `lil-d.exe`; the debug binary is deployed for now (a release switch can come later). `scripts/launch-rust.ps1` and `scripts/cleanup.ps1` are new, location-agnostic (resolve the repository root from `$PSScriptRoot`; no drive letter), and PowerShell 5.1 compatible.
* **Launch and cleanup operate only on `build/deploy`.** `launch-rust.ps1` starts `build/deploy/bin/lil-d.exe` with its station argument; the model and view services join it in later stories. `cleanup.ps1` stops all configuration processes.

## Deliverables

1. `rust/ui/lil-d/Cargo.toml` - the binary crate, using the workspace dependencies.
2. `rust/ui/lil-d/src/main.rs` - argument parsing, window/GL/egui setup, the render loop, the console with the power button, and the green format region.
3. `rust/Cargo.toml` - add `ui/lil-d` to `members`.
4. `scripts/build.ps1` - assemble `build/deploy/bin` after the build.
5. `scripts/launch-rust.ps1` - new; start `build/deploy/bin/lil-d.exe`.
6. `scripts/cleanup.ps1` - new; stop all configuration processes.
7. Planning: this file, `planning/backlog.md`, `planning/architecture.md`.

## Window layout

* The format region is a square; the console is the area around it. For the first version, keep the geometry simple: the square occupies the upper portion of the window, sized to the smaller of the window width and the available height, and the console is a strip (for example along the bottom) holding the single power button.
* The placeholder color is a named constant (dark, unsaturated green); its exact value is chosen during implementation.
* The power button toggles the format region: off hides it (the console remains). Exact off-state rendering (hidden region, resized window, or a blank area) is decided during implementation.

## Build and launch

`scripts/build.ps1` (extended):
* After the workspace build, create `build/deploy/bin` and copy `build/rust/debug/lil-d.exe` into it.
* Keep the existing behavior: no clean, repository-relative paths, `$PSScriptRoot` for the repository root.

`scripts/launch-rust.ps1` (new):
* Resolve the repository root from `$PSScriptRoot`.
* Start `build/deploy/bin/lil-d.exe --station 0`. Later stories add the model service and the view service.

`scripts/cleanup.ps1` (new):
* Stop all processes of all configurations by image name for the known applications (`lil-d` for now; later stories add the services and the other displays).

## Tests

The minimal test is manual: the window opens, the console and the green region are visible, the power button toggles the format region, and the launch/cleanup scripts start and stop the application. A headless unit test is not practical for the windowed shell.

## Minimal test

```
.\scripts\build.ps1
.\scripts\launch-rust.ps1
```

The `lil-d` window opens; the power button toggles the format window; `.\scripts\cleanup.ps1` stops it.

## Risks and follow-ups

* **Window/console geometry** and the **placeholder color** are provisional and may be refined when the display library takes over the region.
* **Deploy build profile.** Debug is deployed for now; switching to release is a later decision.
* **`cleanup.ps1` reliability.** Stopping by image name is simple but broad; a pid file or a job object can replace it if needed.
* **Layout duplication.** `scripts/launch-rust.ps1` and `scripts/cleanup.ps1` start small and grow as stories add executables; keep them location-agnostic and free of hardcoded drive letters.

## Files

* Authored: `rust/ui/lil-d/Cargo.toml`, `rust/ui/lil-d/src/main.rs`, `scripts/launch-rust.ps1`, `scripts/cleanup.ps1`.
* Changed: `rust/Cargo.toml`, `scripts/build.ps1`, `planning/backlog.md`, `planning/architecture.md`, `planning/current_story.md`.
* Reference (gitignored, not committed): `build/rust-render-smoke/`.
