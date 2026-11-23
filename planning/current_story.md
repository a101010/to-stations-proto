# Current story: periodic-current-format

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Make view-service state delivery best effort and periodic, starting with the current format assignment. Change `FormatManagerViewTopic` to best effort, add a monotonic revision, and have `format-view-service` republish the current assignment immediately at startup, immediately after changes, and periodically. Fold in cross-language monitor identity so window placement survives Rust/C++ switches. Record the architecture rule for future reliable control delivery but do not implement an ACK protocol until a message requires it.

- **Depends on:** none.
- **Minimal test:** move windows and change formats in C++ `lil-d`, clean up, restart C++, then switch to Rust and back; positions and formats restore in both directions; `dds-echo` shows repeated best-effort `FormatManagerViewTopic` samples.

## Context

- `cpp-display-chooser` is complete at `2026-11-23 15:16:59`.
- Format assignments already persist in `%LOCALAPPDATA%\to-stations\format-view-service\<display-id>.toml`; the failure is startup delivery to the display library.
- `format-view-service` publishes the saved assignment once at startup and then only on accepted control changes. Displays that miss the one-shot publication keep `hello-world`.
- `HelloWorldViewTopic` is already best effort and republished at 100 Hz.
- `FormatManagerViewTopic` is currently transient-local. C++ and Rust transient-local QoS were made reliable during `cpp-display-chooser`.
- Window position state already lives in `%LOCALAPPDATA%\to-stations\lil-d\<display-id>.toml`.
- Rust/winit writes monitor names such as `\\.\DISPLAY1`; C++/SDL writes friendly names such as `Acer VG270K`. Placement restoration matches by name, so switching implementations can fall back or cascade and appear to reset the position.
- There is no current message that requires end-to-end reliable control with ACK. `FormatManagerCtlTopic` is the only transient-local control topic; command topics remain reliable but volatile.
- This story does not add a `reliable-control-ack` or `monitor-identity` story.

## Decisions

1. **All view-service state delivered to display libraries is best effort and periodically republished.**
   - `HelloWorldViewTopic` already satisfies this.
   - `FormatManagerViewTopic` changes from `transient_local` to `best_effort`.
2. **`format-view-service` republishes the current assignment.**
   - Immediately at startup.
   - Immediately after an accepted assignment change.
   - Periodically, recommended every 250 ms.
3. **Current format state is revisioned.**
   - Add a monotonically increasing `revision` to `FormatManagerViewTopic`.
   - Displays apply only a higher revision than the last applied revision.
   - When multiple samples are available, select the highest revision, not the last sample in the batch.
4. **No ACK or transient-local dependency for view state.**
   - A missed view sample is acceptable; the next periodic message converges.
   - Periodic republishes do not write the format state file.
5. **Monitor identity is canonical.**
   - C++ maps SDL display bounds to Win32 `MONITORINFOEX.szDevice` names such as `\\.\DISPLAY1`.
   - Rust/winit already uses those names.
   - A geometry fallback handles legacy C++ state with friendly monitor names.
6. **Control topics remain unchanged in this story.**
   - `FormatManagerCtlTopic` remains transient-local and reliable.
   - Command and input topics remain reliable but volatile.
   - No ACK protocol is implemented.
7. **Architecture records the future reliable-control rule.**
   - DDS reliable QoS is transport-level, not proof of application delivery.
   - A future reliable control writer must track outstanding application ACKs per command and expected receiver, retry until all ACKs or a deadline, and stop only when all required ACKs are present.
   - Receivers ACK after applying and deduplicate by command id.
   - Prefer idempotent set-state commands over non-idempotent events.
   - No current message requires this yet.

## Scope

**In scope**

- `FormatManagerViewTopic` QoS change to best effort.
- Periodic republishing by `format-view-service`.
- `FormatManagerViewTopic.revision`.
- Rust and C++ revision filtering.
- C++ canonical monitor device names.
- Legacy friendly-name placement fallback.
- Architecture updates for view-state delivery, monitor identity, and future ACKed control.
- Tests and manual verification.

**Out of scope**

- ACK protocol implementation.
- New reliable-control or monitor-identity stories.
- Changes to `HelloWorldViewTopic`; it already republishes at 100 Hz.
- Changing control-topic QoS.
- README restructuring.

## Files

```text
contracts/topics.toml
DDS/format_manager.idl
rust/service/format-view-service/src/main.rs
rust/lib/display-lib/src/format_source.rs
cpp/libs/display-lib/src/format_source.hpp/.cpp
cpp/libs/display-lib/src/window_state.cpp
cpp/libs/support/include/support/window_position.hpp
cpp/libs/support/src/window_position.cpp
rust/lib/window-position/src/lib.rs
cpp/libs/display-lib/tests/main.cpp
rust/lib/display-lib/src/format_source.rs tests
rust/service/format-view-service tests
planning/architecture.md
planning/backlog.md
planning/completed_stories.md
planning/current_story.md
```

No `docs/versions.md` change is needed.

## Implementation

### 1. Contract and IDL

`contracts/topics.toml`:

- Change `FormatManagerViewTopic.qos` from `transient_local` to `best_effort`.

`DDS/format_manager.idl`:

- Add `unsigned long long revision;` to `FormatManagerViewTopic`.
- Update generated Rust/C++ constructors, tests, and any field metadata.

### 2. `format-view-service`

Add a publish period, default 250 ms, preferably exposed as `--rate-ms`.

Behavior:

1. Load and validate saved assignments.
2. Publish immediately with revision 1.
3. In the main loop:
   - process control input;
   - write the state file only when the assignment actually changes;
   - publish when changed or when the periodic interval elapses;
   - increment `revision` on every publish.
4. Keep the writer best effort.
5. Do not write state on periodic republishes.

### 3. Display libraries

Rust `format_source` and C++ `FormatSource`:

- Track `last_revision` for the active display.
- Apply an assignment only when `incoming.revision > last_revision`.
- When multiple samples are available, select the highest revision for the display id.
- Treat repeated samples as idempotent.
- Continue filtering by display id.
- Do not add file fallback or ACK handling.

### 4. Monitor identity

Add a Windows C++ helper that:

- enumerates Win32 monitors with `EnumDisplayMonitors`/`GetMonitorInfoW`;
- maps SDL display bounds to `MONITORINFOEX.szDevice`;
- returns the device name when a match exists.

Use the canonical name when:

- building the monitor list for placement restore;
- saving the current window monitor.

Fall back to the SDL friendly name only if no Win32 match exists.

Legacy placement fallback:

- If the saved monitor name does not match, try a monitor with matching saved monitor dimensions.
- Use the unique resolution match.
- Otherwise retain the existing primary-monitor and cascade fallback.

Apply the geometry fallback in the shared placement code used by both Rust and C++ so old C++ friendly-name state migrates without a guaranteed reset.

### 5. Architecture

Update `planning/architecture.md` with:

- View-state topics are best effort, idempotent, revisioned where ordering matters, and periodically republished.
- Consumers tolerate repeats and ignore stale revisions.
- `HelloWorldViewTopic` already follows this policy; `FormatManagerViewTopic` does after this story.
- Window placement uses the Win32 monitor device name as the canonical monitor identity, with geometry fallback for legacy state.
- Reliable control delivery is an application-level ACK contract; DDS reliable QoS alone is insufficient.
- Writers of future reliable control track every required ACK and retry until all ACKs or a failure deadline.
- Receivers ACK after applying and deduplicate.
- Prefer idempotent current-state publication over ACK where the payload is state.
- The ACK rule is guidance for future work; no current topic implements it.

## Tests

- Contract test expects `FormatManagerViewTopic` QoS `best_effort`.
- `format-view-service` revision increments on every publish.
- State is written only when assignments change; periodic republishes do not write it.
- Rust `FormatSource` ignores older revisions and applies newer ones.
- C++ `FormatSource` ignores older revisions and applies newer ones.
- Multiple available samples resolve to the highest revision.
- Monitor device-name mapping and legacy geometry fallback tests where practical.
- Full `scripts/build.ps1 -Test` and CTest remain green.

## Manual Verification

1. Run `scripts/build.ps1 -Test`, then `scripts/build.ps1`.
2. With a temporary `LOCALAPPDATA`, launch C++ `lil-d` for displays 0 and 1.
3. Move both windows to known positions.
4. Change both formats.
5. Run `scripts/cleanup.ps1`.
6. Relaunch C++; verify positions and formats restore.
7. Clean up and launch Rust; verify the same positions and formats restore.
8. Clean up and switch back to C++; verify again.
9. Confirm `format-view-service\<id>.toml` retains the expected formats.
10. Confirm `lil-d\<id>.toml` uses `\\.\DISPLAY…` monitor names.
11. Use `dds-echo` to observe repeated best-effort `FormatManagerViewTopic` samples.
12. Remove only the temporary state and processes.

## Risks

- Best-effort samples can arrive out of order; revision handling is required.
- Periodic publishing must not rewrite the format state file.
- Legacy friendly-name state may require one migration; geometry fallback should minimize visible resets.
- High-DPI monitor bounds must match between SDL and Win32.
- Control-topic behavior remains unchanged; do not accidentally add ACK logic to this story.
- Do not touch real `%LOCALAPPDATA%`; use temporary state for GUI verification.

## Bookkeeping

- `cpp-display-chooser` is completed at `2026-11-23 15:16:59`.
- `periodic-current-format` is `in progress` in `backlog.md`.
- `periodic-current-format` has no remaining story prerequisites.
- Do not add `reliable-control-ack` or `monitor-identity` stories.
- Do not move `periodic-current-format` to `completed_stories.md` until the user asks to plan the next story.
