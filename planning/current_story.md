# Current story: cpp-format-services

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create C++ `format-ctl-service` and `format-view-service` with per-display assignments, persistence, command relay, contracts, instance guards, and deploy-relative config. Extract the shared C++ format-list/assignment-state library used by the display library and services. Replace the Rust format services in `launch-cpp.ps1` while keeping the Rust hello services and `paws` as stand-ins.

- **Depends on:** none.
- **Minimal test:** C++ `lil-d` with C++ format services switches and persists per-window formats; two display ids remain independent.

## Context

- `periodic-current-format` is complete at `2026-11-23 16:43:52`.
- `FormatManagerViewTopic` is best effort, revisioned, and periodically republished.
- The C++ display library consumes the format view topic and filters revisions.
- `launch-cpp.ps1` currently starts the Rust `format-ctl-service` and `format-view-service` as stand-ins.
- The C++ display library has a private format-list parser and constants that the services also need.
- The Rust services define the required behavior and DDS topics:
  - controller: `FormatManagerInputTopic` → `FormatManagerCtlTopic` and `FormatManagerCommandCtlTopic`;
  - view service: `FormatManagerCtlTopic` → `FormatManagerViewTopic`.
- Both services share the format assignment state file `%LOCALAPPDATA%\to-stations\format-view-service\<display-id>.toml`.
- C++ services are console executables; they do not use SDL or the display library.
- C++ runtime DLLs for DDS are already deployed next to C++ executables.

## Decisions

1. **Extract a shared C++ `format-list` library.**
   - Format names, command constants, menu/hello constants.
   - `FormatList`: parse, catalog order, `contains`, `is_known`, `first`, `next`.
   - Saved assignment state: load/save, validation, sorting, legacy/default behavior.
   - Reject `format-chooser` in `formats.toml`; treat it as implicit.
2. **Implement native C++ console services.**
   - `format-ctl-service` and `format-view-service`.
   - Same CLI, contracts, guard names, state paths, and DDS behavior as Rust.
3. **Keep the Rust hello services and `paws` in `launch-cpp.ps1`.**
   - Replace only the two format services.
   - `launch-rust.ps1` remains unchanged.
4. **Keep format view delivery best effort and periodic.**
   - C++ view service publishes immediately, after changes, and every 250 ms.
   - Use a monotonic wall-clock base for revisions so service restarts do not go backward.
5. **C++ format controller publishes full assignment state.**
   - `FormatManagerCtlTopic` remains transient-local/reliable.
   - It never writes the state file; the view service owns persistence.
6. **No ACK protocol or control-topic QoS changes.**
   - Existing reliable control topics remain unchanged.
7. **No C++ hello services or C++ `paws` in this story.**

## Scope

**In scope**

- C++ `format-list` shared library and tests.
- C++ `format-ctl-service`.
- C++ `format-view-service`.
- CMake targets and C++ deploy updates.
- `launch-cpp.ps1` switch to C++ format services.
- `--topics` CTest checks for both services.
- Architecture and backlog updates.

**Out of scope**

- C++ hello services.
- C++ `paws`.
- Lux overlays.
- ACK protocol.
- Position/window-placement changes.
- README restructuring.

## Files

```text
cpp/libs/format-list/
  CMakeLists.txt
  include/format_list/format_list.hpp
  src/format_list.cpp
  tests/main.cpp
cpp/apps/format-manager/
  CMakeLists.txt
  format-ctl-service/CMakeLists.txt
  format-ctl-service/main.cpp
  format-view-service/CMakeLists.txt
  format-view-service/main.cpp
```

Other changes:

```text
cpp/CMakeLists.txt
cpp/libs/display-lib/CMakeLists.txt
cpp/libs/display-lib/src/*                 (use shared format-list)
scripts/build.ps1
scripts/launch-cpp.ps1
planning/architecture.md
planning/backlog.md
planning/completed_stories.md
planning/current_story.md
```

No `docs/versions.md` change is needed.

## Implementation

### 1. Shared C++ `format-list` library

Create namespace `format_list` containing:

- Format constants:
  - `hello-world`, `default`, implicit `format-chooser`;
  - Next/Select/CursorMove/Navigate/Pointer/ToggleArcball/MenuToggle/MenuCancel/OverlayCancel;
  - pointer-right;
  - hello mode/action/menu/overlay constants.
- `is_control_command(int)`.
- `FormatList`:
  - parse ordered `formats.toml`;
  - reject explicit `format-chooser`;
  - `contains`, `is_known`, `first`, `next`.
- Saved assignments:
  - `FormatAssignment { int window; std::string format; }`;
  - `SavedAssignments` with load/save of the exact Rust TOML schema;
  - `validated(FormatList)` dropping duplicates, sorting by window, replacing unknown formats with `hello-world`, and preserving implicit `format-chooser`;
  - default window 0 = `hello-world`;
  - state path remains supplied by `support`.

Update `display-lib` to include/link the shared library and remove its private format-list implementation.

Add `format-list-tests` covering:

- catalog order;
- chooser rejection;
- `contains` vs `is_known`;
- `next` wrapping and chooser exclusion;
- assignment round-trip;
- validation, deduplication, sorting, unknown fallback, and chooser preservation.

### 2. Common service behavior

Each service:

- Parses `--domain` (default 0), `--config <path>`, `--display-id <id>`, and `--topics`.
- Requires `--display-id` unless `--topics` is used.
- Resolves the config directory:
  - explicit `--config`;
  - otherwise `<deploy>/config` through `support`;
  - otherwise `./contracts` for development.
- Loads and validates:
  - `topics.toml`;
  - its service contract under `config/services/`;
  - `formats.toml`.
- Prints the service contract and exits for `--topics`.
- Acquires the same guard names as Rust:
  - `format-ctl-service-display-<id>`;
  - `format-view-service-display-<id>`.
- Creates one `stations_dds::Node` with the deploy DDS config.
- Polls every 10 ms.
- Prints errors to stderr and exits non-zero.

### 3. C++ `format-ctl-service`

- Read saved assignments from:
  `support::state_path("format-view-service", display_id)`.
- Initialize the assignment map from the validated saved state.
- Create:
  - reader `FormatManagerInputTopic`;
  - writer `FormatManagerCtlTopic`;
  - writer `FormatManagerCommandCtlTopic`.
- For each sample addressed to this display:
  - if it is a control command, relay it unchanged on `FormatManagerCommandCtlTopic`;
  - if `Next`, advance using `FormatList::next` with wrapping;
  - if `Select`, accept only `FormatList::is_known` formats;
  - on a changed assignment, publish the complete assignment map on `FormatManagerCtlTopic`.
- Do not write the state file.
- Do not publish state at startup; the view service publishes current state.

### 4. C++ `format-view-service`

- Read and validate saved assignments from:
  `support::state_path("format-view-service", display_id)`.
- Create:
  - reader `FormatManagerCtlTopic`;
  - writer `FormatManagerViewTopic`.
- Add `--rate-ms` with default 250.
- Publish immediately at startup with a revision.
- On control messages for this display:
  - convert the full `windows` set to `SavedAssignments`;
  - validate against the format catalog;
  - write the state file only when the assignment changes;
  - mark state changed.
- Publish when changed or every periodic interval.
- Every publish:
  - increments a revision using a wall-clock-based monotonic base;
  - writes the full `FormatManagerViewTopic` snapshot.
- Keep `FormatManagerViewTopic` best effort.

### 5. CMake, deploy, and launch

- Add `format-list` static library.
- Add `format-ctl-service` and `format-view-service` console executables under `cpp/apps/format-manager`.
- Link `support`, `stations-dds`, `format-list`, and generated format-manager/hello-world types as needed.
- Add the C++ apps directory to `cpp/CMakeLists.txt`.
- Update `scripts/build.ps1` to copy both C++ service executables to `build/deploy/cpp/bin`.
- Update `scripts/launch-cpp.ps1`:
  - Rust hello services and `paws` from `build/deploy/rust/bin`;
  - C++ `format-ctl-service`, `format-view-service`, and `lil-d` from `build/deploy/cpp/bin`;
  - validate all paths.
- No `cleanup.ps1` change; image names are unchanged.

### 6. Architecture

Update `planning/architecture.md`:

- Add C++ `format-list` to shared C++ libraries.
- Add `cpp/apps/format-manager/`.
- State that C++ format services are drop-in replacements.
- Update the C++ launch description: C++ format services/displays with Rust hello and `paws` stand-ins.
- Keep the existing best-effort periodic view and future ACK policies.
- Do not add an ACK implementation.

## Tests

- `format-list-tests` as above.
- CTest `--topics` checks for both services using `${REPO_ROOT}/contracts`.
- Existing CTest suites stay green.
- Manual integration:
  - `launch-cpp.ps1` starts C++ format services and C++ `lil-d`;
  - formats switch and persist across restart;
  - two display ids remain independent;
  - no Rust format services are left running by `launch-cpp.ps1`;
  - `launch-rust.ps1` still uses Rust format services.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\launch-cpp.ps1 -DisplayId 0,1
# change formats on both displays
.\scripts\cleanup.ps1
.\scripts\launch-cpp.ps1 -DisplayId 0,1
# verify both formats restore
.\scripts\cleanup.ps1
.\scripts\launch-rust.ps1 -DisplayId 0,1
# verify the Rust configuration still works
```

Use a temporary `LOCALAPPDATA` for automated/manual verification. Confirm the state files are shared and both display ids remain independent.

## Risks

- Moving format constants and parsing into a shared library touches display-lib compile paths and tests.
- C++ assignment state serialization must match Rust’s TOML schema exactly.
- The C++ view revision must remain monotonic across service restarts.
- Only the view service writes format state.
- `launch-cpp.ps1` must not leave Rust format services running.
- Control-topic behavior remains unchanged; do not add ACK logic.
- Do not touch real `%LOCALAPPDATA%`; use temporary state for GUI verification.

## Bookkeeping

- `periodic-current-format` is completed at `2026-11-23 16:43:52`.
- `cpp-format-services` is `in progress` in `backlog.md`.
- `cpp-format-services` has no remaining story prerequisites.
- Do not move `cpp-format-services` to `completed_stories.md` until the user asks to plan the next story.
