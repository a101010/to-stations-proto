# Current story: cpp-hello-view-ctl

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create C++ per-display `hello-view-service` and `hello-ctl-service`: Auto/Inspect, local attitude and inertia, light, color/HSV, overlay snapshots, frozen Lux Pos attitude, Lux Color/Lux Pos actions, guards, and view relay. Replace the Rust hello services in `launch-cpp.ps1`.

- **Depends on:** none.
- **Minimal test:** with C++ hello service and a display, mode/arcball/light/color actions and two-display isolation match the Rust behavior.

## Context

- `cpp-hello-service` is complete at `2026-11-24 11:41:36`.
- The C++ configuration runs C++ `hello-service`, C++ format services, and C++ `lil-d` with Rust `hello-view-service`, `hello-ctl-service`, and `paws` stand-ins.
- The Rust hello view/control services are the behavior reference.
- `HelloWorldTopic`, `HelloWorldViewTopic`, `HelloWorldInputTopic`, and `HelloWorldCtlTopic` already exist.
- The service contracts `hello-view-service.toml` and `hello-ctl-service.toml` already exist.
- C++ `format-list` currently has hello actions 0–7 but is missing the color/light actions 8–14.
- The C++ display library uses placeholder overlays; the real C++ overlays come later.
- Hello view state is intentionally not persisted.
- `launch-rust.ps1` must remain unchanged.

## Decisions

1. **Port the Rust relay logic into the C++ view service.**
   - Keep `ViewRelay` app-local under `hello-view-service`.
   - Do not create a shared C++ hello-state library yet.
2. **Extract the controller relay filter.**
   - Put it in a small `control` helper so the matching and ignored-display cases are testable.
3. **Extend shared C++ `format-list` constants.**
   - Add the missing `kHelloActionColor*` and `kHelloActionLight*` constants.
   - Add aliases in display-lib’s compatibility header.
4. **Services implement overlay state; overlays remain out of scope.**
   - The view service owns Lux Color and Lux Pos state, snapshots, and commits/cancels.
   - C++ overlay rendering remains in `cpp-lux-color` and `cpp-lux-pos`.
5. **After this story, only Rust `paws` remains a C++ launch stand-in.**
6. **No IDL, contract, state-path, or version changes.**

## Scope

**In scope**

- C++ hello view relay and tests.
- C++ `hello-view-service`.
- C++ `hello-ctl-service`.
- C++ format-list hello action constants.
- CMake targets, deploy copying, and C++ launch switch.
- Architecture and planning updates.

**Out of scope**

- C++ Lux Color and Lux Pos rendering.
- C++ `paws`.
- Hello state persistence.
- Changes to Rust services.
- Changes to `launch-rust.ps1`.

## Files

```text
cpp/apps/formats/hello-world/
  CMakeLists.txt
  hello-view-service/
    CMakeLists.txt
    main.cpp
    relay.hpp
    relay.cpp
    tests/main.cpp
  hello-ctl-service/
    CMakeLists.txt
    main.cpp
    control.hpp
    control.cpp
    tests/main.cpp
```

Other changes:

```text
cpp/libs/format-list/include/format_list/format_list.hpp
cpp/libs/format-list/tests/main.cpp
cpp/libs/display-lib/src/format_list.hpp
scripts/build.ps1
scripts/launch-cpp.ps1
planning/architecture.md
planning/backlog.md
planning/completed_stories.md
planning/current_story.md
```

No `docs/versions.md`, `DDS/`, or `contracts/` changes are needed.

## Implementation

### 1. Shared C++ format constants

Add to `format_list`:

- `kHelloActionColorHue = 8`
- `kHelloActionColorSv = 9`
- `kHelloActionColorCommit = 10`
- `kHelloActionColorCancel = 11`
- `kHelloActionLight = 12`
- `kHelloActionLightCommit = 13`
- `kHelloActionLightCancel = 14`

Add aliases for these in `cpp/libs/display-lib/src/format_list.hpp`.

Extend `format-list-tests` with checks for the action values.

### 2. C++ hello view relay

Port `rust/service/hello-view-service/src/relay.rs` to `hello_view_service::ViewRelay`.

API:

```text
ViewRelay(Clock::time_point started, uint32_t display_id)
update(const HelloWorldTopic&, Clock::time_point now)
control(const HelloWorldCtlTopic&, Clock::time_point now)
std::optional<HelloWorldViewTopic> view(Clock::time_point now)
```

Use `std::chrono::steady_clock` and saturating elapsed-time helpers.

State:

- latest model sample and last-received time;
- Auto/Inspect mode;
- arcball enabled;
- local attitude quaternion;
- inertia velocity vector;
- light direction;
- RGB color;
- HSV values;
- optional color snapshot;
- optional pre-light snapshot;
- optional frozen Lux Pos attitude;
- overlay state;
- optional drag state;
- last inertia tick.

Exact constants and defaults:

- model timeout `2s`;
- inertia damping `4.5`;
- minimum inertia speed `0.05`;
- stick rotation `pi` radians at full deflection;
- initial light `(0.6, 0.8, 0.7)`;
- initial color white;
- initial HSV `(0, 0, 1)`;
- identity quaternion as local attitude.

Port exactly:

- `trackball` projection onto the near hemisphere;
- shortest-arc `rotation_between`, including the antipodal fallback axis;
- quaternion-vector conversion choosing the shorter arc;
- quaternion multiplication and axis-angle construction;
- HSV→RGB sector conversion;
- `NoConn hh:mm:ss` formatting;
- inertia integration, damping, and stopping;
- Auto/Inspect transitions;
- mode toggle cancelling the active overlay first;
- grab/drag/release behavior and inertia;
- arcball toggle and right-stick rotation;
- Lux Color open/cancel/commit, snapshot restore, HSV clamping;
- Lux Pos open/cancel/commit, light normalization with `z >= 0`, fallback `+Z`, frozen attitude;
- mutual exclusion between Lux Color and Lux Pos;
- overlay-specific action filtering;
- display-id filtering.

`view()` behavior:

- Before two seconds without any model sample: return no view.
- At/after two seconds without model: return invalid NoConn view with identity or last attitude as appropriate.
- With fresh model: valid, using the model greeting and automatic attitude.
- On timeout after a model sample: invalid NoConn view, retaining the last attitude.
- In Inspect mode: publish the local attitude.
- In Lux Pos: publish the frozen attitude.
- Advance inertia only when no overlay is open.

Expose a small velocity accessor for tests if needed to port the Rust inertia test.

### 3. C++ hello-view-service executable

CLI:

- `--domain` default `0`;
- `--rate-ms` default `10`;
- `--config <dir>`;
- `--display-id`;
- `--topics`.

Behavior:

- Resolve config through explicit path, deploy config, then `./contracts`.
- Load and validate `topics.toml` and `services/hello-view-service.toml`.
- `--topics` prints the contract and exits without a guard or DDS.
- Require `--display-id`.
- Acquire `hello-view-service-display-<id>`.
- Create one node and:
  - writer `HelloWorldViewTopic`;
  - reader `HelloWorldTopic`;
  - reader `HelloWorldCtlTopic`.
- Period is `max(rate_ms, 1)` milliseconds.
- Loop:
  - take model samples and update the relay with the last valid sample;
  - take control samples and apply each;
  - publish `relay.view(now)` when present;
  - on reader exceptions, print a diagnostic and continue;
  - on writer failure, exit non-zero;
  - sleep for the period.

### 4. C++ hello-ctl-service executable

CLI:

- `--domain` default `0`;
- `--config <dir>`;
- `--display-id`;
- `--topics`.

Behavior:

- Resolve config and validate `topics.toml` and `services/hello-ctl-service.toml`.
- `--topics` prints and exits without DDS.
- Require `--display-id`.
- Acquire `hello-ctl-service-display-<id>`.
- Create reader `HelloWorldInputTopic` and writer `HelloWorldCtlTopic`.
- Poll every 10 ms.
- Relay only input addressed to the configured display, copying `display_id`, `action`, `x`, `y`, `z`, and `pressed`.
- Propagate reader/writer failures as errors.

`control.hpp/cpp` exposes the filter/relay helper for testing.

### 5. CMake, deploy, and launch

- Add both service subdirectories to `cpp/apps/formats/hello-world/CMakeLists.txt`.
- Add executables:
  - `hello-view-service`: `main.cpp`, `relay.cpp`; link `support`, `stations-dds`, `format-list`, `hello_world_types`.
  - `hello-ctl-service`: `main.cpp`, `control.cpp`; link `support`, `stations-dds`, `hello_world_types`.
- Add tests:
  - `hello-view-service-tests`: relay implementation plus all Rust relay tests.
  - `hello-ctl-service-tests`: matching/ignored display tests.
  - `hello-view-service-topics`.
  - `hello-ctl-service-topics`.
- Update `scripts/build.ps1` to copy both executables to `build/deploy/cpp/bin`.
- Update `scripts/launch-cpp.ps1`:
  - `$helloCtl` and `$helloView` resolve to `$cppBin`;
  - description says all C++ hello, format, and display components run from `cpp/bin`, with only Rust `paws` remaining.
- No cleanup-script change; image names are unchanged.

### 6. Architecture

Update the C++ runtime launch paragraph:

- C++ launch now starts C++ hello-service, hello-view/control services, format services, and displays.
- Rust `paws` is the only remaining stand-in.

Mention that C++ `format-list` includes the complete hello action constants.

## Tests

Port the Rust relay tests:

- no view before the startup timeout;
- NoConn view at two seconds;
- fresh model relaying;
- timeout retains the last attitude and counts up;
- recovery on a new model sample;
- Auto/Inspect toggle;
- arcball toggle and display isolation;
- drag rotation and inertia;
- inertia decay and stopping;
- overlay mutual exclusion;
- Lux Pos attitude freeze and light updates;
- light commit and light ignored outside Lux Pos;
- HSV updates and color commit;
- color cancel restores the snapshot;
- Lux Pos/mode toggle cancels Lux Color first;
- HSV primary-color conversion;
- right-stick rotation only when arcball is active.

Add controller tests:

- matching display input is relayed unchanged;
- other display input is ignored.

Expected CTest count: existing 9 plus relay tests, control tests, and two `--topics` checks → 13/13.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
```

Manual/integration verification with a temporary `LOCALAPPDATA`:

1. `launch-cpp.ps1` starts C++ hello-service, C++ hello-view/control services, C++ format services, and C++ `lil-d`.
2. Confirm no Rust `hello-service`, `hello-view-service`, or `hello-ctl-service` remains.
3. Confirm Rust `paws` still runs.
4. Use a temporary DDS harness or `dds-echo` plus display menu to verify:
   - view starts as NoConn after two seconds without the model;
   - model data becomes valid;
   - Auto/Inspect and arcball work;
   - mouse/controller drag and inertia change the local attitude;
   - Lux Color and Lux Pos actions update overlay/color/light state;
   - display 0 and display 1 remain independent.
5. Verify `launch-rust.ps1` still runs the Rust services unchanged.
6. Remove only the temporary `LOCALAPPDATA`.

## Risks

- Quaternion and inertia math must match Rust numerically; port formulas verbatim and keep tolerance-based tests.
- Inertia tests need deterministic time points; avoid real sleeps.
- C++ generated `Quaternion` defaults to zero, not identity; construct identity explicitly.
- Color/light action constants must exactly match Rust.
- Overlay-specific actions must be ignored outside their overlay.
- Lux Pos must freeze the attitude but still relay the live light.
- Do not persist any hello view state.
- Avoid starting Rust hello services from `launch-cpp.ps1`.

## Bookkeeping

- `cpp-hello-service` is completed at `2026-11-24 11:41:36`.
- `cpp-hello-view-ctl` is `in progress` in `backlog.md`.
- `cpp-hello-view-ctl` has no remaining story prerequisites.
- Do not move `cpp-hello-view-ctl` to `completed_stories.md` until the user asks to plan the next story.
