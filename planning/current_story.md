# Current story: multi-lil-d

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Give every display instance a unique display id and use that id for all format-manager routing, so multiple displays are independent. Each `big-d` and `lil-d` instance has its own id. There is no station id in the format-manager chain.

- **Depends on:** none.
- **Minimal test:** run two `lil-d` instances with distinct `--display-id` values; clicking one format window changes only that display's format.

## Context

- The format manager is per display, but the current chain is single-display and unscoped.
- The format-manager topics carry no identity, so two displays would consume each other's input and assignments.
- `format-ctl-service` and `format-view-service` have no identity arguments.
- `lil-d` takes `--station`, which is only used by the console; it is not a display identity.
- The display ABI is version 2 and has no way to give the display library an identity.
- `dds-echo` decodes the three format-manager topics and must be updated for the new field.

## Decisions

1. **One unique display id per display instance.** It is an opaque string, not a station id and not a per-station display number.
2. **No station id in the format-manager chain.** `FormatManagerInputTopic`, `FormatManagerCtlTopic`, and `FormatManagerViewTopic` carry only `display_id` as identity. The services take only `--display-id`.
3. **No station id on `lil-d`.** Remove `--station`; take a required `--display-id` instead. The console and window title show the display id.
4. **One controller/view-service pair per display id.** Each pair is launched with the same id as its display.
5. **Stamp and filter at every hop:**
   - display library stamps outgoing input events and filters incoming view samples;
   - controller filters input events and stamps control samples;
   - view service filters control samples and stamps view samples.
6. **ABI v3 identity function:** `display_lib_set_identity(display_lib*, const char* display_id)`. The library copies the string; the pointer need only be valid during the call.
7. **Topic names, QoS, and registry key lists stay unchanged.** Multiple `transient_local` writers are handled by display-id filtering; keying is not required.
8. **No C++ work, no `big-d`, no format menu/chooser, no sequence sending.** `dds-echo-send-sequences` remains out of scope.

## IDL

`DDS/format_manager.idl`:

```idl
module format_manager {
  struct FormatWindowAssignment {
    long window;
    string format;
  };

  struct FormatManagerInputTopic {
    string display_id;
    long window;
    long command;   // 0 = Next, 1 = Select
    string format;
  };

  struct FormatManagerCtlTopic {
    string display_id;
    sequence<FormatWindowAssignment> windows;
  };

  struct FormatManagerViewTopic {
    string display_id;
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
  /* Sets the display identity. Call after create and before the first render.
   * The string is copied; the pointer need only be valid for the call. */
  void display_lib_set_identity(display_lib* d, const char* display_id);
  ```

`rust/lib/display-lib/src/lib.rs`:

- bump `ABI_VERSION` to 3;
- store `display_id: String` on `Display`;
- export `display_lib_set_identity`, copying the incoming `CStr`;
- pass the display id into `FormatSource` polling and event creation;
- leave `FormatManager` unchanged: assignments remain per window.

`rust/lib/display-lib/src/format_source.rs`:

- `poll` filters view samples by exact `display_id` match before selecting the latest sample;
- `send_next` includes the display id in `FormatManagerInputTopic`;
- add a pure helper such as `latest_matching(...)` so the multi-writer filter is testable without DDS.

An empty display id is treated as unset: the library publishes no input events and applies no assignments. `lil-d` always sets a non-empty id.

## `lil-d`

- `rust/ui/lil-d/src/cli.rs`: remove `--station`; add a required `--display-id <id>`.
- `rust/ui/lil-d/src/display.rs`:
  - bump `EXPECTED_ABI_VERSION` to 3;
  - resolve and require `display_lib_set_identity`;
  - call it immediately after `display_lib_create`, before the first render.
- `rust/ui/lil-d/src/app.rs`: pass `args.display_id` when loading the library; use it in the window title, e.g. `lil-d - <display_id>`.
- `rust/ui/lil-d/src/console.rs`: show the display id instead of the station.

## Format controller service

`rust/service/format-ctl-service/src/main.rs`:

- replace `--station`/`--display` planning with `--display-id <id>`;
- use `Option<String>` and require a non-empty id for normal operation, so `--topics` can still print the contract without an id;
- ignore input events whose `display_id` does not exactly match;
- stamp published `FormatManagerCtlTopic` samples with the service display id;
- keep the existing per-window assignment map and Next/Select logic;
- add tests for matching/non-matching events and stamped output.

## Format view service

`rust/service/format-view-service/src/main.rs`:

- add `--display-id <id>` with the same `--topics` handling as the controller;
- ignore control samples whose `display_id` does not match;
- publish `FormatManagerViewTopic` stamped with the service display id and the forwarded windows;
- add unit tests for matching/non-matching forwarding.

## Scripts

`scripts/launch-rust.ps1`:

- remove the `-Station` parameter;
- add a display-id list, for example `[string[]]$DisplayId = @('lil-d-a', 'lil-d-b')`;
- start the shared `hello-service` and `hello-view-service` once;
- for each display id, start `format-ctl-service --display-id <id>`, `format-view-service --display-id <id>`, and `lil-d --display-id <id>`;
- update the status message.

`scripts/cleanup.ps1`:

- no functional change is required because it already stops every process by image name; update the comment to mention multiple displays and confirm both `lil-d` instances and both service pairs are stopped.

## `dds-echo`

The decoder already prints generated fields generically, so no decoding logic is needed beyond tests. Update struct literals in:

- `rust/tool/dds-echo/src/decode.rs` (`FormatManagerInputTopic`, `FormatManagerCtlTopic`, `FormatManagerViewTopic` test samples);
- `rust/tool/dds-echo/src/main.rs` (format-manager detail test).

Add assertions that `display_id` appears in decoded output. `send.rs` needs no logic change: the generated form exposes the new string field for `FormatManagerInputTopic`; sequence editing for Ctl/View remains deferred.

## Documentation

Update `planning/architecture.md`:

- replace the display applications' `--station` identity with a unique display id in the display/format-manager descriptions;
- state that the format manager is identified by a display id and that each display has its own controller/view-service pair;
- remove station/display scoping language from the format-manager section;
- keep station-related ideas for later increments only if they do not reintroduce station identity into the format-manager chain.

## Implementation steps

1. **Update the IDL** and regenerate the Rust types with `build.ps1`.
2. **ABI and display library:** bump to v3, add `display_lib_set_identity`, store and apply the display id, filter/stamp in `FormatSource`.
3. **`lil-d`:** remove `--station`, add `--display-id`, update loader/app/console/title.
4. **Controller service:** add `--display-id`, filter and stamp, update tests.
5. **View service:** add `--display-id`, filter and stamp, update tests.
6. **Scripts:** update `launch-rust.ps1` and the cleanup comment.
7. **`dds-echo`:** update decode/detail tests for `display_id`.
8. **Documentation:** update `architecture.md`.
9. **Verify:** tests, build/deploy, and one manual two-display run.

## Acceptance test

Automated:

- Controller ignores another display id and stamps its own id in control.
- View service ignores another display id and stamps its own id in view.
- Display library ignores another display id's view samples, selects the latest matching sample, and stamps outgoing events.
- Updated `dds-echo` decode tests show the new `display_id` field.
- Existing Next/Select, window-independence, contract, and decoder tests still pass.

Manual:

1. Build and run `launch-rust.ps1`; two `lil-d` windows appear with different display ids in their titles.
2. Both initially show `hello-world`.
3. Click the first display's format window; only it changes to `default`; the second stays `hello-world`.
4. Click the first again; it returns to `hello-world`.
5. Click the second display's window; only it changes.
6. In `dds-echo`, verify `FormatManagerInputTopic`, `FormatManagerCtlTopic`, and `FormatManagerViewTopic` samples carry the correct display id and decode correctly.
7. Run `cleanup.ps1`; both displays and both service pairs stop.

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

- **Duplicate display ids:** ids must be unique, or displays will cross-talk. The launch script assigns one id per display and passes the same id to its services.
- **Transient-local multi-writer traffic:** filter by display id *before* choosing the latest sample, or another display's assignment can be applied.
- **Identity ordering:** `display_lib_set_identity` must be called after create and before the first render.
- **C string lifetime:** the library must copy the id; the app's `CString` need not outlive the call.
- **ABI mismatch:** bump both the header and the `lil-d` expected version together.
- **Struct literals:** adding `display_id` breaks explicit constructions in service and `dds-echo` tests; update all of them.
- **Removing `--station`:** update the console, title, launch script, and any documentation that still passes it.
- **Send scope:** Ctl/View sequence editing remains unsupported; only the new scalar string field appears for InputTopic.
- **Duplicate process names:** cleanup must kill all `lil-d`, `format-ctl-service`, and `format-view-service` instances; the existing image-name stop already does this.

## Files touched

- `DDS/format_manager.idl`
- `include/display_lib.h`
- `rust/lib/display-lib/src/lib.rs`
- `rust/lib/display-lib/src/format_source.rs`
- `rust/ui/lil-d/src/cli.rs`
- `rust/ui/lil-d/src/display.rs`
- `rust/ui/lil-d/src/app.rs`
- `rust/ui/lil-d/src/console.rs`
- `rust/service/format-ctl-service/src/main.rs`
- `rust/service/format-view-service/src/main.rs`
- `rust/tool/dds-echo/src/decode.rs`
- `rust/tool/dds-echo/src/main.rs`
- `scripts/launch-rust.ps1`
- `scripts/cleanup.ps1`
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

## Bookkeeping

- `dds-echo-message-classification` was completed at `2026-11-09 16:47:08` and moved to `completed_stories.md` under a new "Later increments" section.
- `multi-lil-d` is `in progress` in `backlog.md`.
- Do not move `multi-lil-d` to `completed_stories.md` until the user asks to plan the next story.
