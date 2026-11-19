# Current story: cpp-support

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add a shared C++ support library for named-mutex single-instance guards, window placement state under `%LOCALAPPDATA%\to-stations`, and deploy-relative asset/config lookup. Use the same guard names, state paths, and placement behavior as the Rust applications.

- **Depends on:** none.
- **Minimal test:** duplicate guard detection, window-state round-trip, placement/cascade tests, and path resolution from a relocated deploy tree.

## Context

- `cpp-dds` is complete. The C++ project has CMake, third-party libraries, generated IDL types, and a DDS library with CTest coverage.
- The Rust `single-instance` library uses `Local\to-stations-<tool_name>` mutexes.
- The Rust `window-position` and `app-state` libraries persist placement under `%LOCALAPPDATA%\to-stations\<application>\<key>.toml`.
- The Rust placement logic handles preferred monitors, resolution changes, negative offsets, legacy state, visibility fallback, and cascade placement.
- C++ `lil-d` and `paws` will consume the support library later.
- No C++ application or deployment is part of this story.

## Decisions

1. **One static support library with three modules.** Put `single_instance`, `window_position`, and `paths` under `cpp/libs/support`.
2. **Keep the library independent of SDL/winit.** It operates on plain monitor/window rectangles; applications translate their SDL data.
3. **Use toml++ for state files.** It is already committed under `cpp/third_party/tomlplusplus`.
4. **Hide Win32 types from public headers.** `SingleInstance` uses a private implementation; public callers see only C++ types.
5. **Use Win32 for named mutexes and atomic replacement.** `CreateMutexW` for guards; `MoveFileExW` with replace semantics for state files.
6. **No window-activation helper.** It can be added later if a C++ tool needs foreground activation.
7. **No real `%LOCALAPPDATA%` access in tests.** Tests use temporary roots and unique mutex names.
8. **No new build-script smoke logic.** Existing CTest runs committed tests.
9. **No deployment or app behavior yet.** `cpp-lil-d` and `cpp-paws` consume the library later.

## Scope

**In scope**

- Add `cpp/libs/support/` with headers, sources, CMake, and tests.
- Add single-instance guard behavior and tests.
- Add window placement persistence, resolution/cascade behavior, and tests.
- Add deploy-relative path helpers and tests.
- Register `support-tests` with CTest.
- Update `cpp/CMakeLists.txt`, `planning/architecture.md`, and planning bookkeeping.

**Out of scope**

- C++ applications, display library, services, or tools.
- Window activation/foregrounding.
- C++ deployment, runtime DLL copying, or launch scripts.
- README restructuring.
- Changes to Rust support libraries.
- Temporary smoke projects or generated test programs.

## Library layout

```text
cpp/libs/support/
  CMakeLists.txt
  include/support/single_instance.hpp
  include/support/window_position.hpp
  include/support/paths.hpp
  src/single_instance.cpp
  src/window_position.cpp
  src/paths.cpp
  tests/main.cpp
```

CMake target `support`:

- static library
- public include directory `include`
- C++20
- links `tomlplusplus` privately
- uses private Windows definitions such as `WIN32_LEAN_AND_MEAN` and `NOMINMAX`

## Single-instance guard

`support::SingleInstance`:

- move-only RAII wrapper
- `static std::optional<SingleInstance> acquire(std::string_view tool_name)`
- returns `std::nullopt` when another instance owns the mutex
- closes the handle on destruction
- public header does not include `windows.h`

Mutex name:

```text
Local\to-stations-<tool_name>
```

This matches the Rust `single-instance` library. Applications supply the full logical name, for example:

- `lil-d-display-0`
- `format-ctl-service-display-1`
- `paws-display-0`

Tests:

- first acquire succeeds
- second acquire for the same name fails
- releasing allows reacquisition
- distinct names remain independent

Use unique names containing the process id so tests do not collide.

## Window placement

Public plain types:

```text
Point { int x; int y; }
Size { uint32_t width; uint32_t height; }
MonitorInfo { optional<string> name; Point position; Size size; }
SavedPosition { optional monitor, monitor_width, monitor_height; int x, y; optional width, height; }
ResolvedPosition { Point position; optional<Size> size; optional<SavedPosition> saved; }
```

Functions:

- `should_save(force, monitor_changed, since_last)`
- `load(path) -> optional<SavedPosition>`
- `save(path, SavedPosition)`
- `state_path(application, key)`
- `state_path_in(root, application, key)` for tests
- `restore_or_cascade(saved, monitors, primary, window_size, inner_size, cascade_slot)`

Behavior to mirror:

- match the saved monitor by name and resolution
- fall back to the primary monitor when the saved monitor is missing or its resolution changed
- restore legacy files without resolution and upgrade them on save
- support negative monitor offsets
- reject invalid saved sizes and use the current inner size
- require enough visible area and a reachable top-left
- cascade by slot: row `r` starts at `(10r, 5r)`, windows are one width apart within a row, and rows wrap by monitor width

State paths:

```text
%LOCALAPPDATA%\to-stations\<application>\<key>.toml
```

State files use the same TOML field names as Rust:

- `monitor`
- `monitor_width`
- `monitor_height`
- `x`
- `y`
- `width`
- `height`

Writes are atomic:

- create the parent directory
- write `<key>.toml.tmp`
- replace the target with `MoveFileExW(..., MOVEFILE_REPLACE_EXISTING)`

Tests:

- save/load round trip
- missing and malformed files return empty
- overwriting an existing file works
- save throttling
- preferred-monitor restore
- negative edge offset
- legacy-state upgrade
- saved-size handling
- resolution mismatch fallback
- missing monitor fallback
- cascade placement

All filesystem tests use temporary directories and remove only those directories.

## Deploy-relative paths

`support` path helpers:

- `module_path()` using `GetModuleFileNameW`
- `deploy_root_for(executable_path)` for pure testing
- `deploy_root()` for the current executable
- `deploy_path(relative)` for composing paths under the deploy root

For a C++ app at:

```text
<deploy>\cpp\bin\lil-d.exe
```

`deploy_root_for` returns:

```text
<deploy>
```

Tests:

- a fake executable under `<temp>\deploy\cpp\bin\tool.exe` resolves `<temp>\deploy`
- `deploy_path("assets")` and `deploy_path("config/cyclonedds-config.xml")` compose correctly
- runtime code does not use repository paths or `..` strings

## CMake and tests

`cpp/CMakeLists.txt`:

- `add_subdirectory(libs/support)`

`cpp/libs/support/CMakeLists.txt`:

- build `support`
- build `support-tests`
- link `support` and any required private dependencies
- `add_test(NAME support-tests COMMAND support-tests)`

`scripts/build.ps1` needs no new smoke logic; the existing CTest step runs `support-tests`.

## Documentation

`planning/architecture.md`:

- note that C++ `support` provides named-mutex guards, placement persistence, and deploy-relative lookup for C++ applications.

No new `docs/versions.md` entry is needed; toml++ is already recorded.

No README changes are required for this story; the broad README rewrite remains `readme-cleanup`.

## Verification

1. `scripts/build.ps1 -Test`
   - `support` and `support-tests` compile.
   - `support-tests` and `stations-dds-tests` pass through CTest.
2. `scripts/build.ps1`
   - Release build succeeds.
3. Confirm no real `%LOCALAPPDATA%\to-stations` state is touched.
4. `git diff --check`.

## Risks

- **Windows API surface:** keep Win32 types out of public headers.
- **Atomic replacement:** `std::filesystem::rename` does not replace reliably on Windows; use `MoveFileExW`.
- **TOML compatibility:** field names must match the Rust state files.
- **Placement parity:** mirror the Rust visibility, monitor, and cascade rules precisely.
- **Test isolation:** use unique mutex names and temporary state roots.
- **Deployment layout:** C++ executables live in `<deploy>/cpp/bin`; keep the pure path helper available for tests and explicit app paths.

## Files touched

- `cpp/CMakeLists.txt`
- `cpp/libs/support/` (new)
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

## Bookkeeping

- `cpp-dds` is completed at `2026-11-19 08:47:50`.
- `cpp-support` is `in progress` in `backlog.md`.
- `cpp-support` has no remaining story prerequisites.
- Do not move `cpp-support` to `completed_stories.md` until the user asks to plan the next story.
