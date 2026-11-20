# Current story: cpp-lil-d

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the C++ `lil-d` using SDL3 and Dear ImGui: OpenGL 3.3 window, console and power toggle, display id, display-library loading, full mouse event forwarding, native-cursor hiding over the format, single-instance guard, window placement, and deploy under `build/deploy/cpp/bin` with the C++ default-format display library. Create `launch-cpp.ps1`, which starts C++ `lil-d` with the existing Rust services and `paws` as temporary stand-ins.

- **Depends on:** none.
- **Minimal test:** C++ `lil-d` shows the C++ default format, forwards mouse input, hides the native cursor over the format, restores window state, and runs from the relocated deploy tree; `launch-cpp.ps1` starts C++ `lil-d` plus the Rust stand-ins.

## Context

- `cpp-display-abi` is complete at `2026-11-20 13:12:13`.
- The C++ display library implements ABI v4 and the default amber/green format.
- C++ `support` provides single-instance guards, window placement, and deploy-relative paths.
- SDL3, Dear ImGui, and glad are present.
- The Rust configuration already has working services and `paws` that can act as temporary stand-ins.
- Format manager, menu, chooser, hello-world, and overlays are not part of this story.
- The C++ launch script is the C++ configuration bootstrap with Rust stand-ins; it is not the hybrid configuration.

## Decisions

1. **Only the C++ default format is required.** No menu, chooser, hello-world, DDS, or format switching yet.
2. **Load the display library dynamically.** The app uses `LoadLibraryW` and `GetProcAddress`; it does not link the DLL.
3. **Use SDL3 + Dear ImGui + glad.** The app owns the window, console, GL context, and event loop.
4. **Use the C++ support library.** Reuse the guard, placement, and deploy-relative path helpers.
5. **Persist placement under `lil-d\<display-id>.toml`**, matching Rust.
6. **Use `lil-d-display-<id>` as the guard name**, matching Rust.
7. **Hide the native cursor over the format region.** Use a transparent SDL cursor over the format and the normal cursor over the console.
8. **Create `launch-cpp.ps1` now.** It starts C++ `lil-d` and the existing Rust services and `paws` as temporary stand-ins.
9. **No C++ services in this story.**
10. **No real `%LOCALAPPDATA%` in smoke tests.** Use a temporary directory.

## Scope

**In scope**

- Add `cpp/apps/displays/lil-d/`.
- SDL3 window and Dear ImGui console.
- Dynamic C++ display-library ABI loader.
- Default-format rendering through the C++ display library.
- Mouse button/motion/wheel forwarding.
- Native-cursor hiding over the format region.
- Single-instance guard and window placement.
- C++ app deploy to `build/deploy/cpp/bin`.
- `launch-cpp.ps1` using Rust stand-ins.
- CTest tests for pure layout and mouse mapping.

**Out of scope**

- Format manager, menu, chooser, icon decoder, hello-world, Earth, Lux Color, or Lux Pos.
- C++ services or C++ `paws`.
- `launch-hybrid.ps1`.
- README restructuring.
- Runtime DLL deployment beyond what the default-format app needs.

## Files

```text
cpp/apps/displays/lil-d/
  CMakeLists.txt
  src/main.cpp
  src/app.hpp/.cpp
  src/console.hpp/.cpp
  src/display_loader.hpp/.cpp
  src/layout.hpp/.cpp
  src/mouse.hpp/.cpp
  src/window_state.hpp/.cpp
  tests/main.cpp
```

Other changes:

- `cpp/CMakeLists.txt`
- `scripts/build.ps1`
- `scripts/launch-cpp.ps1` (new)
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

No `docs/versions.md` change is needed.

## App behavior

### Window and layout

- Initial logical window size: 720x720.
- Console strip: 64 logical pixels high at scale 1.0.
- Format region: the largest centered square above the console.
- Serialize the display-library render rectangle and mouse positions in physical pixels.
- Use ImGui logical coordinates for the console.
- Window title: `lil-d - <display-id>`.

### Display loader

- Resolve a relative `--display-lib` from the executable directory.
- Default to `display_lib.dll` next to the executable.
- Load the DLL and resolve every ABI v4 function.
- Check `display_lib_abi_version() == 4`.
- Pass SDL’s GL proc-address loader to `display_lib_create`.
- Report `display_lib_last_error()` on failure.
- Destroy the handle before unloading the DLL.

### Console

Use Dear ImGui for the bottom strip:

- `display <id>`
- separator
- `Power: on` / `Power: off` button
- disabling ImGui `.ini` files

When power is off:

- skip display-library rendering
- keep the console and window state active
- update the button label when toggled back on

### Input

Forward all buttons if the format is on:

- left = 0
- right = 1
- middle = 2
- back = 3
- forward = 4
- other = 5

Forward absolute mouse motion and wheel input in physical pixels. Do not gate forwarding on ImGui consumption; the display library hit-tests its own region. ImGui still receives events for the console.

### Cursor

- Over the format region: transparent SDL cursor.
- Over the console or outside the format: normal arrow.
- Re-evaluate on motion, resize, and power toggle.

### Guard and placement

- Acquire `support::SingleInstance::acquire("lil-d-display-<id>")`.
- On duplicate, show an SDL error message box and exit with failure.
- Convert SDL displays into `support::MonitorInfo`.
- Restore or cascade placement through `support`.
- Save on move, resize, focus loss, and close, using the support save throttle.
- Application name `lil-d` and key/display id.

## Build and deploy

`cpp/CMakeLists.txt`:

- `add_subdirectory(apps/displays/lil-d)`

`cpp/apps/displays/lil-d/CMakeLists.txt`:

- `WIN32` executable
- C++20
- link `support`, `imgui`, `glad`, and SDL3 through the existing targets
- include `SDL3/SDL_main.h` in the main translation unit

`scripts/build.ps1`:

- deploy `build/cpp/Release/lil-d.exe` to `build/deploy/cpp/bin/lil-d.exe`
- deploy `build/cpp/Release/display_lib.dll` to `build/deploy/cpp/bin/display_lib.dll`
- keep Rust artifacts in `build/deploy/bin`
- do not copy C++ artifacts into the Rust bin

No additional runtime DLLs are needed for the C++ default-format display library.

## launch-cpp.ps1

The script:

- calls `cleanup.ps1`
- requires C++ `lil-d` and its display library
- sets `CYCLONEDDS_URI` to the deploy config for the Rust stand-ins
- starts Rust:
  - `hello-service`
  - per-display `hello-ctl-service`
  - per-display `hello-view-service`
  - per-display `format-ctl-service`
  - per-display `format-view-service`
  - per-display `paws`
- starts C++ `lil-d` from `build/deploy/cpp/bin`
- raises the C++ `lil-d` windows and Rust `paws` windows
- supports `-DisplayId` like `launch-rust.ps1`

This is the C++ configuration bootstrap with Rust stand-ins, not the hybrid configuration.

## Tests

Committed CTest tests:

- layout geometry for a 720x720 window with a 64-pixel logical console
- centered square format region
- mouse button mapping to ABI values
- inside-format hit test for cursor hiding
- state-path/placement integration through the support library

Manual:

1. Run `scripts/build.ps1 -Test`.
2. Run `scripts/build.ps1`.
3. Launch C++ `lil-d` with a temporary `LOCALAPPDATA`.
4. Verify:
   - window opens with title `lil-d - <id>`
   - console shows display id and power button
   - format region shows the C++ amber/green default format
   - power toggle hides/shows the format
   - native cursor is hidden over the format
   - window position/size restore
   - duplicate display id is rejected
5. Run `launch-cpp.ps1` with one display and verify C++ `lil-d` plus Rust stand-ins start.
6. Clean up only the temporary directory.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\launch-cpp.ps1 -DisplayId 0
.\scripts\cleanup.ps1
```

## Risks

- **Physical vs logical coordinates:** keep display-library coordinates in physical pixels and ImGui coordinates logical.
- **Cursor hiding:** use a transparent cursor rather than globally hiding the cursor so the console keeps an arrow.
- **Window placement:** convert SDL display/window coordinates consistently into the support geometry types.
- **Display library path:** resolve relative paths from the executable directory, not the process working directory.
- **Launch script scope:** keep Rust stand-ins explicit; do not turn this into the hybrid launch.
- **No real runtime state:** always use a temporary `LOCALAPPDATA` for smoke tests.

## Bookkeeping

- `cpp-display-abi` is completed at `2026-11-20 13:12:13`.
- `cpp-lil-d` is `in progress` in `backlog.md`.
- `cpp-lil-d` has no remaining story prerequisites.
- Do not move `cpp-lil-d` to `completed_stories.md` until the user asks to plan the next story.
