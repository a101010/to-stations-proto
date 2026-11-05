# Current story: format-ctl-service

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the per-display format-manager controller and view services. The display library creates logical format events from local input and publishes them; `format-ctl-service` consumes them, owns per-format-window assignments using the shared formats contract, and publishes control; `format-view-service` forwards the assignments to the display library. `lil-d` remains format- and topic-agnostic; the display library owns all format-manager DDS.

- **Depends on:** none.
- **Minimal test:** launch the Rust fleet, click in `lil-d`'s format window, and verify the display switches `hello-world` <-> `default` while `dds-echo` shows and decodes the input/control/view topics, including the sequence-bearing control/view payloads.

## Terms

- **Format window** = a square region that displays a format: one on `lil-d`, three on `big-d`.
- The **eyebrow** is not a format, is not a format window, and is not controlled by the format manager.

## Context

The display library currently owns the format registry and switches formats locally when `lil-d` forwards a left click. This story moves the format-selection logic into a separate controller service and adds the view service that forwards assignments to the display library.

The display application remains intentionally ignorant of formats and topics: it loads the display library, renders the regions the library is given, and forwards local input events. The display library handles all format-manager DDS and turns local input into logical events.

This story is **single-display and unscoped**: there is one `lil-d` and one controller/view pair, so the topics do not yet carry `station`/`display` fields. The follow-up `multi-lil-d` story adds display identity, topic scoping, and the display ABI change.

## Decisions

1. **Logical events.** The display library creates `Next`/`Select` events for a format window from local input. `lil-d` does not create format events or know format names.
2. **Single-display, unscoped topics.** No `station`/`display` fields in this story; one controller/view pair. `multi-lil-d` adds identity/scoping and ABI v3 later.
3. **No ABI change.** `lil-d` continues to use the existing ABI; the display library handles the new DDS work internally.
4. **Shared formats contract.** Add `contracts/formats.toml` with the ordered format names. The controller uses it for next/select; the display library uses it to name/order formats.
5. **Controller owns per-window assignments.** It stores format-window ID -> format name, starting from the first format for unassigned windows.
6. **View service is a pass-through.** It receives controller assignments and republishes them on the view topic. It can add view-specific data later.
7. **Display library owns the DDS.** It subscribes to assignments, publishes logical input events, and applies assignments. It shares one DDS `Node` with the hello_world view reader where practical.
8. **Format windows only.** No event, assignment, or format ID is created for the eyebrow.
9. **`dds-echo` compatibility.** The three new topics are registered, so `dds-echo`'s Send dispatch gains schemas and typed writers for them to keep the registry-coverage test passing.
10. **One controller/view instance.** The services are per display by design, but this increment runs one of each for the single `lil-d`.

## Topics and IDL

Create `DDS/format_manager.idl`:

```idl
module format_manager {
  struct FormatWindowAssignment {
    long window;
    string format;
  };

  struct FormatManagerInputTopic {
    long window;
    long command;   // 0 = Next, 1 = Select
    string format;  // Select only; empty for Next
  };

  struct FormatManagerCtlTopic {
    sequence<FormatWindowAssignment> windows;
  };

  struct FormatManagerViewTopic {
    sequence<FormatWindowAssignment> windows;
  };
};
```

- Add `#include "format_manager.idl"` to the generated root (`DDS/hello_world.idl`).
- Add all three `*Topic` types to `stations-dds::contract::known_types()`.
- Add the `format_manager` module to generated-type tests.

Registry entries:

| Topic | Type | QoS |
|---|---|---|
| `FormatManagerInputTopic` | `format_manager::FormatManagerInputTopic` | `reliable` |
| `FormatManagerCtlTopic` | `format_manager::FormatManagerCtlTopic` | `transient_local` |
| `FormatManagerViewTopic` | `format_manager::FormatManagerViewTopic` | `transient_local` |

Service contracts:

- `contracts/services/format-ctl-service.toml`: publish `FormatManagerCtlTopic`, subscribe `FormatManagerInputTopic`.
- `contracts/services/format-view-service.toml`: publish `FormatManagerViewTopic`, subscribe `FormatManagerCtlTopic`.

## Shared formats contract

Add `contracts/formats.toml`:

```toml
[[formats]]
name = "hello-world"

[[formats]]
name = "default"
```

Add a shared loader, for example `rust/lib/format-list` (`format_list`):

- loads the ordered list;
- validates non-empty/unique names;
- provides `first()`, `contains()`, and `next()`.

Consumers:

- `format-ctl-service`: next/select logic and validation.
- `display-library`: build its format renderer registry in contract order and expose names.
- Later: `format-menu` and `format-chooser-format`.

Deploy `formats.toml` to `build/deploy/config/`.

## Responsibilities

### Display library

- Load `formats.toml`; build format renderers in contract order.
- Add `name()` to the `Format` trait; names for `hello-world`/`default`.
- Replace the single active format with per-format-window assignments:
  - unassigned window -> first format;
  - unknown name -> ignore;
  - render window `0` from the single region for `lil-d`.
- Share one DDS `Node` for:
  - the existing `HelloWorldViewTopic` reader;
  - a `FormatManagerViewTopic` reader;
  - a `FormatManagerInputTopic` writer.
- On a left press inside a format window, publish a logical `Next` event for that window (window `0` for `lil-d`).
- Apply assignments from the view reader before rendering.
- Remove local format cycling from the format manager.
- Keep the eyebrow out of all format-manager paths.

### `lil-d`

- Unchanged: load the display library, render, and forward unconsumed pointer events.
- No knowledge of formats, format names, format events, topics, or contracts.

### `format-ctl-service` (new crate)

Location: `rust/service/format-ctl-service`.

- Arguments: `--domain`, `--config`, `--topics` only. No format/input identity or selection from the CLI.
- Loads topics registry, its contract, and the formats contract.
- Subscribes `FormatManagerInputTopic`; publishes `FormatManagerCtlTopic`.
- Maintains `BTreeMap<i32, String>` format-window assignments.
- On command `0` (Next):
  - current = assigned format, else the first format;
  - next format in the list, wrapping;
  - update the map and publish the full assignment vector.
- On command `1` (Select):
  - validate the format name;
  - update the map and publish.
- Ignores unknown commands/formats.
- Stays alive to retain transient-local control history.

### `format-view-service` (new crate)

Location: `rust/service/format-view-service`.

- Arguments: `--domain`, `--config`, `--topics`.
- Subscribes `FormatManagerCtlTopic`; publishes `FormatManagerViewTopic`.
- Forwards the assignment vector.
- Stays alive so the last view sample is retained for late-joining displays.

## `dds-echo` compatibility

- Add `schema_for` and `TopicWriter` handling for the three new registered types.
- `FormatManagerInputTopic` is usable from the Send form (command values 0/1, fields editable), enabling manual `Next`/`Select` injection.
- The control/view topics contain sequences; the Send form marks those fields unsupported, which is acceptable. The registry-coverage test still requires a sender handler.

## Build, launch, and cleanup

- Add the `format-list`, `format-ctl-service`, and `format-view-service` crates to the workspace.
- Deploy both service executables and `formats.toml`.
- Add both service source roots to the `dds-contracts` scan.
- `launch-rust.ps1` starts both services before `lil-d`:
  - `format-ctl-service`
  - `format-view-service`
- `cleanup.ps1` stops both.
- Add both service contracts to `build/topics.md`.

## Architecture and backlog updates

- In `architecture.md`:
  - define **format window** explicitly and state that the eyebrow is separate;
  - document the display-library-owned DDS/event path and that `lil-d` remains format/topic-agnostic;
  - document the controller/view/display chain, shared formats contract, and single-display first implementation;
  - name `multi-lil-d` as the follow-up for display identity/scoping and ABI v3.
- In `backlog.md`:
  - make `format-ctl-service` the active story;
  - add `multi-lil-d` immediately after it for two `lil-d` instances and the ABI identity change;
  - add `random-greeting` as a small `hello-service` change.
- Move `dds-echo-copy-detail` to `completed_stories.md`.

## Implementation steps

1. **Planning bookkeeping.** Move `dds-echo-copy-detail` to `completed_stories.md` with completion time `2026-11-04 14:25:18`; make `format-ctl-service` active; add `multi-lil-d` and `random-greeting`.
2. **IDL and contracts.** Add `format_manager.idl`, include it from the root, register the three topics, add the two service contracts, and add `formats.toml`.
3. **Format list.** Add the `format-list` loader and tests.
4. **Controller.** Add `format-ctl-service` with the assignment/next/select logic and tests.
5. **View service.** Add `format-view-service` with forwarding and tests.
6. **Display library.** Load formats, add names, add view reader/input writer on a shared Node, apply assignments, create logical events from clicks, remove local cycling, and add tests.
7. **`dds-echo` Send dispatch.** Add the three topic schemas/writers for registry compatibility and manual input testing.
8. **`dds-echo` detail decoding.** Add decode arms and tests for the three format-manager topics, including sequence-bearing control/view samples, plus registry decoder coverage.
9. **Build/launch/cleanup.** Add the crates, deploy artifacts, service contracts, launch, cleanup, and scan roots.
10. **Architecture/backlog.** Record the format-window/eyebrow rule, chain, shared contract, and single-display limitation.
11. **Verify.** Run the tests/build, the manual fleet click-through, and the capture/detail decode check.

## Detail decoding follow-up (required for completion)

`dds-echo` currently recognizes the new topics but does not decode them because
`decode.rs` only handles the hello_world types. This story is not complete until
capture/detail decodes all three format-manager payloads.

- Add decode arms for:
  - `format_manager::FormatManagerInputTopic`
  - `format_manager::FormatManagerCtlTopic`
  - `format_manager::FormatManagerViewTopic`
- Confirm the `DdsSequence<FormatWindowAssignment>` fields in the control/view
  topics round-trip through `CdrSerializer`/`CdrDeserializer`.
- Add tests for the input topic, control/view sequence topics, and a
  registry-coverage test that fails when a registered type has no decoder.
- No Send sequence editing in this story; `dds-echo-send-sequences` is queued at
  the end of the backlog.

## Acceptance test

Automated:

- Format-list: order, wrap, contains, duplicate/empty rejection.
- Controller: first `Next` -> second format; wrap; `Select`; invalid select ignored; independent windows.
- View service: forwards assignments.
- Display library: click inside/outside a format window creates/does not create a `Next` event; assignment by name applies; unknown names ignored; unassigned window uses the first format.
- Registry/IDL completeness includes the three topics.
- `dds-echo` send-dispatch test covers the new registered topics.
- `dds-echo` detail decode tests cover the input topic and the sequence-bearing control/view topics.
- A registry-coverage test fails when a registered type lacks a decoder.

Manual:

1. `launch-rust.ps1` starts the controller/view services and `lil-d`.
2. Capture with `dds-echo`; verify the three format-manager topics are registered.
3. Click inside `lil-d`'s format window; verify `hello-world` -> `default`.
4. Click again; verify it wraps back.
5. Select each format-manager record in `dds-echo`; verify the detail decodes the fields and shows the window assignments for the control/view sequences.
6. Use `dds-echo` Send to publish a `Select` input event; verify the display switches.
7. Click the console/eyebrow area; verify no format event and no change.
8. Restart the view service/display; verify transient-local state restores the last assignment.
9. Stop the controller; verify clicks no longer change the display until restarted.

## Verification

```powershell
cargo test --manifest-path rust/Cargo.toml
.\scripts\build.ps1 -Test
.\scripts\build.ps1
```

Then the manual fleet click-through and `dds-echo` checks.

## Risks

- **Single-display limitation:** topics are unscoped in this story; multiple `lil-d` instances would cross-talk. `multi-lil-d` adds identity/scoping and ABI v3.
- **Shared formats contract:** display-library names/order must match `formats.toml`; invalid names fail at startup.
- **DDS sharing:** the display library must share one Node where practical; separate participants are acceptable but increase discovery traffic.
- **Startup ordering:** controller/view before `lil-d`; transient-local handles late joiners.
- **Input loss:** reliable input is not transient-local; a click before matching is lost.
- **Controller restart:** assignments are in-memory; the display keeps the retained view until changed.
- **Multi-window:** only window `0` renders in `lil-d`; the assignment model supports future `big-d` windows. The eyebrow is excluded.
- **Send-form limitations:** control/view sequences are not fully editable; input events are enough for manual testing.
- **No menu yet:** only `Next` is exercised locally; `Select` via tests and `dds-echo` until `format-menu`.

## Files touched

- `DDS/format_manager.idl`
- `DDS/hello_world.idl`
- `contracts/topics.toml`
- `contracts/formats.toml`
- `contracts/services/format-ctl-service.toml`
- `contracts/services/format-view-service.toml`
- `rust/Cargo.toml`
- `rust/Cargo.lock`
- `rust/lib/format-list/` (new)
- `rust/service/format-ctl-service/` (new)
- `rust/service/format-view-service/` (new)
- `rust/lib/stations-dds/src/contract.rs`
- `rust/lib/stations-dds/src/lib.rs`
- `rust/lib/display-lib/src/lib.rs`
- `rust/lib/display-lib/src/format_manager.rs`
- `rust/lib/display-lib/src/formats.rs`
- `rust/lib/display-lib/src/formats/*.rs`
- `rust/lib/display-lib/src/view.rs`
- `rust/lib/display-lib/src/format_source.rs` (new)
- `rust/tool/dds-echo/src/send.rs`
- `rust/tool/dds-echo/src/decode.rs`
- `scripts/build.ps1`
- `scripts/launch-rust.ps1`
- `scripts/cleanup.ps1`
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

`lil-d` is intentionally unchanged. The display ABI remains at version 2; the `multi-lil-d` story introduces display identity and ABI v3.

## Result

Core implementation complete; the story remains in progress until `dds-echo` decodes the sequence-bearing format-manager payloads:

- Added `DDS/format_manager.idl`; the input/control/view topic types are registered and generated with the existing IDL build.
- Added `contracts/formats.toml`, the shared `format-list` crate, and both service contracts.
- Added `format-ctl-service` (per-window assignments, next/select, full control publication) and `format-view-service` (assignment forwarding).
- The display library now owns the format-manager DDS on one shared participant, applies assignments by name, and creates logical `Next` events from local clicks. Local format cycling is removed.
- `lil-d` is unchanged; the display ABI remains version 2.
- `dds-echo`'s Send dispatch covers the three new registered topics.
- Build/deploy/launch/cleanup include both services and deploy `formats.toml`.
- `cargo test`/`build.ps1 -Test` pass and `build.ps1` deploys.
- Manual click-through passed: clicking the `lil-d` format window changed `hello-world` to `default` through the display-library -> controller -> view-service -> display-library chain.
- The wire command is an `i32` (`0` = Next, `1` = Select) because generated enum fields are not usable in the generated topic structs with this fork revision; the constants live in `format-list`.
- Pending completion: `dds-echo` detail decoding for the three format-manager topics, including control/view sequence fields.
- `dds-echo-message-classification` is the next queued story after this one is complete.
- `multi-lil-d` follows for two displays and ABI v3 identity.
- `dds-echo-send-sequences` is queued at the end of the backlog.

## Bookkeeping

- `dds-echo-copy-detail` was completed at `2026-11-04 14:25:18` and moved to `completed_stories.md`.
- `format-ctl-service` is `in progress` in `backlog.md`.
- `dds-echo-message-classification` is the next queued story after detail decoding is complete.
- `multi-lil-d` follows for two `lil-d` instances and the ABI identity change.
- `dds-echo-send-sequences` is queued at the end of the backlog.
- `random-greeting` is a queued `hello-service` change.
- Do not move `format-ctl-service` to `completed_stories.md` until the detail-decoding acceptance passes and the user asks to plan the next story.
