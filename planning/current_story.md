# Current story: multi-lil-d

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Give every display instance a unique numeric display id and use it for all format-manager routing, so multiple displays are independent. Each `big-d`/`lil-d` instance has its own id; there is no station id in the format-manager chain. Persist each display's window position and its format-manager assignment per display id, and reject duplicate instances of the same service or display.

- **Depends on:** none.
- **Minimal test:** run two `lil-d` instances with display ids 0 and 1; clicking one format window changes only that display's format; positions and formats restore after restart; a duplicate display id exits with an error.

## Context

- The format manager is per display, but the chain is currently single-display and unscoped.
- The working tree contains an initial string-based display-id implementation; this plan replaces it with numeric display ids and adds per-display state and instance guards.
- The format-manager topics carry no persisted state, so a restarted display returns to the first format instead of the last selected one.
- `lil-d` does not remember its window position and starts behind the service console windows.
- Services have no duplicate-instance guard; the `single-instance` library is currently described as GUI-only.

## Decisions

1. **One unique numeric display id per display instance.** It is a `u32` and is also the default cascade slot.
2. **No station id in the format-manager chain.** `FormatManagerInputTopic`, `FormatManagerCtlTopic`, and `FormatManagerViewTopic` carry only `display_id`. The services take only `--display-id`.
3. **No station id on `lil-d`.** It takes a required `--display-id <u32>`; the console and window title show it.
4. **One controller/view-service pair per display id.** Each pair is launched with the same id as its display.
5. **Stamp and filter at every hop.**
6. **ABI v3 identity function** takes `uint32_t display_id`; the display library stores it as `Option<u32>`.
7. **Per-display runtime state** lives under `%LOCALAPPDATA%\to-stations\<application>\<display_id>.toml`; one file per instance discriminator, written atomically.
8. **Window position** stores the preferred monitor and the monitor-relative position. If that monitor is gone, use the same relative position on the main monitor; if it does not fit, or there is no file, use the cascade.
9. **Cascade layout** uses literal row starts `(0,0)`, `(10,5)`, `(20,10)`, so rows overlap vertically by design.
10. **Duplicate instances are rejected.** Singleton services use a service-name guard; per-display services and displays use a guard with the display id. `--topics` is a diagnostic and does not acquire the guard.
11. **The format view service persists assignments.** It saves the current per-window formats for its display id, defaults to hello-world when the file is missing or invalid, and republishes them on startup. The controller reads the same file once at startup so Next/Select continue from the restored format.
12. **No C++ work, no `big-d`, no format menu/chooser, no sequence sending.** `dds-echo-send-sequences` remains out of scope.

## IDL

`DDS/format_manager.idl`:

```idl
module format_manager {
  struct FormatWindowAssignment {
    long window;
    string format;
  };

  struct FormatManagerInputTopic {
    unsigned long display_id;
    long window;
    long command;   // 0 = Next, 1 = Select
    string format;
  };

  struct FormatManagerCtlTopic {
    unsigned long display_id;
    sequence<FormatWindowAssignment> windows;
  };

  struct FormatManagerViewTopic {
    unsigned long display_id;
    sequence<FormatWindowAssignment> windows;
  };
};
```

`FormatWindowAssignment` stays `(window, format)`. Topic names and IDL type names do not change, so `contracts/topics.toml` needs no registry edits. All components are rebuilt together, so the CDR layout change has no compatibility requirement.

## Display library and ABI

`include/display_lib.h`:

- bump `DISPLAY_LIB_ABI_VERSION` to `3`;
- add:
  ```c
  /* Sets the display's numeric identity. Call after create and before the first render. */
  void display_lib_set_identity(display_lib* d, uint32_t display_id);
  ```

`rust/lib/display-lib/src/lib.rs`:

- bump `ABI_VERSION` to 3;
- store `display_id: Option<u32>` on `Display`;
- export `display_lib_set_identity`;
- pass the display id into `FormatSource` polling and event creation;
- leave `FormatManager` unchanged: assignments remain per window.

`rust/lib/display-lib/src/format_source.rs`:

- `poll` filters view samples by exact numeric display id before selecting the latest sample;
- `send_next` includes the display id in `FormatManagerInputTopic`;
- add a pure helper such as `latest_matching(...)` so the multi-writer filter is testable without DDS.

A missing display id is treated as unset: the library publishes no input events and applies no assignments. `lil-d` always sets it.

## Shared runtime state

Add a small `rust/lib/app-state` crate:

- `state_path(app: &str, key: &str) -> Option<PathBuf>` using `LOCALAPPDATA`:
  `%LOCALAPPDATA%\to-stations\<app>\<key>.toml`;
- `load<T: DeserializeOwned>(path) -> Option<T>`;
- `save<T: Serialize>(path, value) -> io::Result<()>` with an atomic temporary-file-then-rename write.

Add `SavedAssignments` to `rust/lib/format-list/src/lib.rs`:

- a list of `{ window: i32, format: String }`;
- `hello_world()` default containing window 0 = `hello-world`; other windows remain unassigned and fall back to hello-world;
- load/save helpers using `app-state`;
- validation against `FormatList`; unknown formats fall back to `hello-world`.

## `lil-d`

- `rust/ui/lil-d/src/cli.rs`: required `--display-id <u32>`; remove `--station`.
- `rust/ui/lil-d/src/main.rs`: acquire the per-display guard `lil-d-display-<id>` after argument parsing. On duplicate, show a native error dialog and exit with failure. Keep the guard alive for the process.
- `rust/ui/lil-d/src/display.rs`: bump the expected ABI to 3; call `display_lib_set_identity` immediately after create, before the first render.
- `rust/ui/lil-d/src/app.rs`: pass the display id to the library; use it in the window title; track window moves; restore/save position.
- `rust/ui/lil-d/src/console.rs`: show the display id.
- Add `rust/ui/lil-d/src/position.rs` for state loading, monitor selection, cascade calculation, and throttled saves.

### Position state

File: `%LOCALAPPDATA%\to-stations\lil-d\<display_id>.toml`

```toml
monitor = "\\\\.\\DISPLAY1"
x = 100
y = 50
```

- `monitor` is the winit monitor name.
- `x` and `y` are the window top-left offset from that monitor's top-left.
- If `LOCALAPPDATA` is unavailable, persistence is disabled without failing startup.

### Startup position

1. Load the display id's state file.
2. If the saved monitor is connected and the saved top-left lands inside it, restore there.
3. Else, if the main monitor exists and the same relative position lands inside it, restore on the main monitor.
4. Else, or if there is no state file, use the cascade.

Missing, corrupt, or off-screen state falls through to the next step.

### Cascade layout

- Main monitor origin `(mx, my)`; window outer size `(ww, wh)`.
- Slot = display id.
- Row `r` starts at `(mx + 10*r, my + 5*r)`.
- Row capacity: `max(1, max(0, main_width - 10*r) / ww)`.
- Walk rows until the slot is found; column `c` is at `x = mx + 10*r + c*ww`.
- Examples: display 0 → `(0,0)`; display 1 → `(ww,0)` if it fits; the first wrapped slot → `(10,5)`; the next row → `(20,10)`.

Rows overlap vertically by design. The capacity floor of 1 keeps the mapping finite on a monitor too narrow for a full window.

### Saving

- Track `WindowEvent::Moved`.
- Save at most every ~250 ms while moving; flush from `about_to_wait`.
- Save on `CloseRequested` and `exiting`.
- Because `cleanup.ps1` force-kills processes, the throttled saves are the reliable path.
- On save, store the current monitor name and the window position relative to it.

## Format controller service

`rust/service/format-ctl-service/src/main.rs`:

- add `--display-id <u32>`;
- acquire the guard `format-ctl-service-display-<id>` for normal operation; `--topics` prints without it;
- read `format-view-service\<display_id>.toml` once at startup and initialize the per-window assignment map from it; if missing/invalid, default window 0 to `hello-world`;
- ignore input events whose display id does not match;
- stamp published control samples with the service display id;
- add tests for numeric identity, saved-state initialization, and stamped output.

## Format view service

`rust/service/format-view-service/src/main.rs`:

- add `--display-id <u32>`;
- acquire the guard `format-view-service-display-<id>` for normal operation; `--topics` prints without it;
- load saved assignments from `format-view-service\<display_id>.toml`; if missing/unreadable/invalid, use hello-world for every window;
- publish the loaded/default assignments immediately on startup;
- ignore control samples whose display id does not match;
- validate format names against `contracts/formats.toml`;
- save the accepted assignments atomically after every control sample;
- publish view samples stamped with the service display id;
- add tests for matching/non-matching forwarding, the hello-world default, and save/load.

## Other services

- `hello-service` and `hello-view-service` acquire singleton guards (`hello-service`, `hello-view-service`) before creating DDS endpoints.
- On a duplicate, each service prints a diagnostic and exits with a failure status; there is no foreground attempt for services.
- `--topics` and `--help` do not acquire the guard.

## Single-instance library

- Update `rust/lib/single-instance/src/lib.rs` documentation: the guard is used both by GUI singletons and by fleet services/displays to reject duplicate identities.
- Keep `activate_existing_window` for `dds-echo`; services do not use it.
- Add a test that distinct guard names can be held simultaneously.

## Scripts

`scripts/launch-rust.ps1`:

- use numeric ids, defaulting to `0,1`, for example `[uint32[]]$DisplayId = @(0, 1)`;
- start the shared `hello-service` and `hello-view-service` once;
- for each display id, start `format-ctl-service --display-id <id>`, `format-view-service --display-id <id>`, and `lil-d --display-id <id>`;
- keep the launched `lil-d` processes with `-PassThru`;
- after everything starts, raise each `lil-d` window using a user32 P/Invoke helper modeled on `single-instance::activate_existing_window`: `ShowWindow`, `SetWindowPos(HWND_TOP, ...)`, `BringWindowToTop`, `SetForegroundWindow`, with `SwitchToThisWindow` fallback;
- retry briefly while each `MainWindowHandle` is zero;
- update the status message.

`scripts/cleanup.ps1`:

- no functional change is required because it already stops every process by image name; update the comment to mention multiple displays and their service pairs.

## `dds-echo`

The decoder prints generated fields generically. Update struct literals in:

- `rust/tool/dds-echo/src/decode.rs`;
- `rust/tool/dds-echo/src/main.rs`.

Add assertions that the numeric `display_id` appears in decoded output. `send.rs` needs no logic change: the generated form exposes the numeric field for `FormatManagerInputTopic`; sequence editing for Ctl/View remains deferred.

## Documentation

`planning/architecture.md` now records:

- numeric display ids and the per-display format-manager chain;
- the format view service's per-display assignment file and hello-world default;
- per-user runtime state paths and atomic writes;
- per-instance named-mutex guards for all services and displays;
- duplicate rejection for singleton services and per-display services;
- the updated role of the `single-instance` library.

`planning/backlog.md` now includes the expanded `multi-lil-d` description and the `tentacles` console-window option.

## Implementation steps

1. **Numeric display id conversion:** IDL, ABI, display library, services, `dds-echo` tests, scripts.
2. **Shared runtime state:** add `app-state`; add `SavedAssignments` to `format-list`.
3. **`lil-d` position persistence:** add `position.rs`; restore/fallback/cascade; throttled saves.
4. **`lil-d` duplicate guard:** per-display named mutex and error dialog.
5. **Format view service:** load/publish/save assignments, default hello-world, validate formats.
6. **Format controller:** read the view-service state at startup; initialize assignments.
7. **Other services:** add singleton guards to `hello-service` and `hello-view-service`.
8. **Scripts:** numeric ids and foreground activation.
9. **`dds-echo`:** update decode/detail tests.
10. **Verify:** tests, build/deploy, and one manual two-display run.

## Acceptance test

Automated:

- Controller ignores another display id and stamps its own id in control.
- View service ignores another display id and stamps its own id in view.
- Display library ignores another display id's view samples, selects the latest matching sample, and stamps outgoing events.
- Numeric `display_id` decodes in `dds-echo` tests.
- Saved assignments round-trip; missing/invalid files default to hello-world.
- Controller initializes from saved assignments and Next advances from the restored format.
- Distinct display ids use distinct state files.
- The single-instance guard allows distinct names and rejects the same name.
- Position selection: preferred monitor, main-monitor fallback, cascade, and slot mapping.
- Existing Next/Select, window-independence, contract, and decoder tests still pass.

Manual:

1. Launch display ids 0 and 1 with no state files; verify the cascade positions.
2. Verify both `lil-d` windows are above the service console windows.
3. Click each display; verify only that display changes format.
4. Move both windows, wait, force cleanup, relaunch; verify positions restore.
5. Point one state file at a missing monitor; verify main-monitor fallback; move its position outside the main monitor and verify the cascade.
6. Change a display's format, restart only `format-view-service`, and verify the saved format republishes.
7. Delete the view-service state file and restart it; verify hello-world.
8. Start a duplicate service with the same identity; verify it exits with an error.
9. Start a second format service with a different display id; verify it is allowed.
10. Start a duplicate `lil-d` display id; verify the error dialog and exit.
11. In `dds-echo`, verify the three format-manager topics carry numeric display ids and decode correctly.

## Verification

```powershell
cargo test --manifest-path rust/Cargo.toml
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\launch-rust.ps1
.\scripts\cleanup.ps1
```

One manual two-display run; no screenshots and no repeated UI runs.

## Risks

- **Duplicate display ids:** rejected by the guard; ids must otherwise be unique, or displays cross-talk.
- **Overlapping cascade rows:** intentional per the selected `(0,0)`, `(10,5)`, `(20,10)` layout.
- **Monitor identity:** winit monitor names (`\\.\DISPLAY1`) can change if the display topology changes; fallback covers a missing monitor.
- **State-file races:** the view service is the only writer and writes atomically; the controller reads once at startup.
- **Startup ordering:** if the controller starts before the view service and no file exists, both default to hello-world consistently.
- **Force-kill position saves:** the throttled move save bounds the loss to the save interval.
- **Display-id type change:** all topic literals, ABI calls, service args, scripts, and `dds-echo` tests must move from string to `u32`.
- **Guard placement:** acquire before DDS, windows, or state access; keep `--topics`/`--help` diagnostic modes guard-free.
- **No console in release `lil-d`:** duplicate display ids need a native message box, not only stderr.
- **Cleanup must kill all instances:** the existing image-name stop already does this.

## Files touched

- `DDS/format_manager.idl`
- `include/display_lib.h`
- `rust/Cargo.toml`
- `rust/lib/app-state/Cargo.toml`
- `rust/lib/app-state/src/lib.rs`
- `rust/lib/format-list/src/lib.rs`
- `rust/lib/display-lib/src/lib.rs`
- `rust/lib/display-lib/src/format_source.rs`
- `rust/lib/single-instance/src/lib.rs`
- `rust/ui/lil-d/Cargo.toml`
- `rust/ui/lil-d/src/cli.rs`
- `rust/ui/lil-d/src/main.rs`
- `rust/ui/lil-d/src/display.rs`
- `rust/ui/lil-d/src/app.rs`
- `rust/ui/lil-d/src/console.rs`
- `rust/ui/lil-d/src/position.rs`
- `rust/service/hello-service/Cargo.toml`
- `rust/service/hello-service/src/main.rs`
- `rust/service/hello-view-service/Cargo.toml`
- `rust/service/hello-view-service/src/main.rs`
- `rust/service/format-ctl-service/Cargo.toml`
- `rust/service/format-ctl-service/src/main.rs`
- `rust/service/format-view-service/Cargo.toml`
- `rust/service/format-view-service/src/main.rs`
- `rust/tool/dds-echo/src/decode.rs`
- `rust/tool/dds-echo/src/main.rs`
- `scripts/launch-rust.ps1`
- `scripts/cleanup.ps1`
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/current_story.md`

## Result

Implemented and verified by tests/build:

- Converted format-manager display ids to numeric `u32`/`unsigned long` across the IDL, ABI, display library, services, `dds-echo` tests, and launch script.
- Added `app-state` for per-user state paths and atomic TOML writes, and `SavedAssignments`/`SavedAssignment` to `format-list`.
- `lil-d` now persists its monitor-relative position per display id, restores the preferred monitor, falls back to the main monitor, and uses the literal `(0,0)`, `(10,5)`, `(20,10)` cascade rows.
- Position state now records the logical monitor name plus resolution; a saved position is restored only if the name and resolution match and the window remains sufficiently visible. Negative edge offsets such as `x = -5` are preserved; off-screen placements fall back to a validated main-monitor position or the cascade.
- Saved `x`/`y` remain relative to the monitor; the monitor's current virtual origin is added on restore. A small negative `x` can therefore represent a window at the monitor's left edge, not a wrong absolute position.
- Legacy state without resolution fields is restored by name when visible and rewritten with resolution on startup.
- Position writes also happen immediately on focus loss and monitor changes; failed writes keep the position dirty for retry.
- `cleanup.ps1` closes `lil-d` gracefully before force-stopping, so the final position is saved even when cleanup follows a move immediately.
- `lil-d` rejects a duplicate display id with an error dialog.
- `format-view-service` loads and republishes saved assignments, defaults to hello-world, validates formats, and saves accepted assignments.
- `format-ctl-service` initializes its assignment map from the view-service state file.
- `hello-service`, `hello-view-service`, `format-ctl-service`, and `format-view-service` reject duplicate instances with the same identity.
- `launch-rust.ps1` starts numeric display ids and raises the `lil-d` windows above the service consoles.
- `cargo test`/`build.ps1 -Test` pass; release `build.ps1` deploys.
- Launch checks verified cascade positions, foreground activation, position save/restore across a force-kill, and duplicate-service rejection.
- Moving a display to another monitor and immediately running cleanup/relaunch now restores the new monitor position; cleanup closes displays gracefully before force-stopping.
- Remaining manual checks: clicking each display independently, saved-format restore after a real format change, and the duplicate-`lil-d` error dialog.

## Bookkeeping

- `dds-echo-message-classification` was completed at `2026-11-09 16:47:08` and moved to `completed_stories.md` under a new "Later increments" section.
- `multi-lil-d` is `in progress` in `backlog.md`.
- Do not move `multi-lil-d` to `completed_stories.md` until the user asks to plan the next story.
