# Current story: dds-echo-send

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add a **Send** tab to `dds-echo`. The window becomes two tabs: **Capture** (the current UI unchanged) and **Send**. In Send, choose a topic registered in `contracts/topics.toml`, edit its fields through a form generated from `DdsFieldMetadata`, and publish the sample once or periodically at a chosen interval. Publishing uses `stations-dds` (`Node` typed writer).

- **Depends on:** none; `topic-field-metadata` is complete and provides `DdsFieldMetadata`.
- **Minimal test:** with the process fleet running, launch `dds-echo` midstream, select `HelloWorldTopic`, fill the hello-world fields, and press Send; the fleet's subscribers receive it and the sample appears on the Capture tab decoded as a known topic with sender `self`. With no matching subscriber, Send reports that nothing was transmitted. dds-echo's own periodic SPDP appears as `self` discovery and can be hidden.

## Context

`dds-echo` is currently a capture-only inspector: it records DDSI-RTPS traffic, attributes writers through a discovery-only participant, and decodes registered samples. It has no way to originate traffic.

`topic-field-metadata` added generated `DdsFieldMetadata` implementations for the hello_world types. The trait exposes each type's `DynamicTypeSchema`, conversion to and from `DynamicValue`, and field get/set helpers. That is enough to generate a typed editor for any registered topic without runtime reflection.

The send form must eventually serve `tentacles-injector`, but this story keeps the `idl_type` dispatch local to `dds-echo`; extract it to `stations-dds` when the second consumer appears.

## Decisions

1. **Form coverage: current registry kinds.** Generate editors for strings, all primitives, bool, enum, bitmask, and nested structs. Optional, sequence, array, union, and map fields render as unsupported and block sending with a clear error. No registered topic needs them yet; the form remains schema-driven so adding them later is localized.
2. **One DDS participant.** Add `Node::from_participant` to `stations-dds` and share the existing discovery participant. This keeps the `participants` stat accurate and avoids a second participant. `Node::new` remains unchanged for the services.
3. **Repeat semantics.** Provide `Send Once`, `Start`, `Stop`, and an interval in milliseconds. Repeating continues while viewing Capture; changing the selected topic stops repeating.
4. **Keep dispatch local for now.** The typed `idl_type` -> schema/writer mapping lives in `dds-echo/src/send.rs`. Extract it to `stations-dds` when `tentacles-injector` needs the same mapping.
5. **No new dependencies and no new executable.** `dds-echo` already depends on `slint`, `stations-dds`, and `clap`. `scripts/build.ps1` already builds and deploys `dds-echo`; only its code and UI change.
6. **No service contract.** `dds-echo` is a tool, not a launched service; its send topics are chosen at runtime. `contracts/services/` is unchanged.

## Interfaces

### `stations-dds`

Add to `rust/lib/stations-dds/src/dds.rs`:

```rust
impl Node {
    /// Create a contract-aware Node on an existing participant.
    pub fn from_participant(
        participant: DomainParticipant,
        registry: TopicRegistry,
    ) -> Result<Self, Error>;
}
```

It validates the registry and stores the participant. `Node::new` remains unchanged.

### `dds-echo` send module (`src/send.rs`, new)

- `schema_for(idl_type: &str) -> Option<&'static DynamicTypeSchema>` maps every registered type to its generated metadata.
- `SendForm`:
  - built recursively from a `DynamicTypeSchema`;
  - flattening: structs become a group row plus indented child rows; primitives, strings, bools, enums, and bitmasks become editable rows;
  - stores edited text/bool values in Rust, not in the Slint model;
  - `to_dynamic_value(&self) -> Result<DynamicValue, String>` parses values, applies them to `schema.default_value()`, and validates against the schema.
- `TopicWriter`:
  - variants for `HelloWorldTopic` and `HelloWorldViewTopic`;
  - `new(node: &Node, topic: &str, idl_type: &str) -> Result<Self, String>`;
  - `write_dynamic(&self, value: &DynamicValue) -> Result<(), String>`, which converts via `from_dynamic_value` and calls `DataWriter::write`.

Unsupported rows are reported by path so the status line can identify the field.

### `dds-echo` resource handling

Introduce a shared resource owner in `dds-echo`:

```rust
struct DdsResources {
    participant: DomainParticipant,
    node: Option<Node>,
    _domain: Option<DomainGuard>,
}
```

- `DdsResources::new(domain, config, registry)` creates the domain guard, the participant, and the `Node` via `Node::from_participant`.
- Move domain creation out of `DiscoverySource`; `DiscoverySource` receives or clones the shared `Rc<DdsResources>` and creates its builtin readers on `resources.participant`.
- Field order keeps participants and nodes dropping before `_domain`.
- When discovery is enabled, resources are created at startup. With `--no-discovery`, resources are created lazily on first Send, so passive capture remains passive until the user actually sends.

### UI (`ui/dds_echo.slint`)

New struct:

```slint
export struct SendField {
    path: string,
    label: string,
    kind: string,       // "string" | "bool" | "int" | "float" | "enum" | "bitmask" | "group" | "unsupported"
    indent: int,
    value: string,
    bool-value: bool,
    group: bool,
    unsupported: bool,
}
```

New properties and callbacks:

- `tab` (0 Capture, 1 Send)
- `send-topics: [string]`, `send-topic-index: int`
- `send-fields: [SendField]`
- `send-interval-ms: int` (default 100)
- `send-repeating: bool`, `send-ready: bool`, `send-count: int`
- `send-status: string`
- callbacks: `select-tab(int)`, `select-send-topic(int)`, `send-field-edited(string, string)`, `send-field-toggled(string, bool)`, `send-interval-edited(int)`, `send-once()`, `send-start()`, `send-stop()`

Layout:

- Two-button tab bar at the top.
- **Capture** tab: the entire current toolbar, stats, and main layout moved under `if root.tab == 0`.
- **Send** tab:
  - topic `ComboBox`;
  - `ScrollView` form with group labels, `LineEdit` for text and number rows, `CheckBox` for bool rows, and an "unsupported" marker for unsupported kinds;
  - interval `SpinBox` (1-60000 ms, step 10);
  - `Send Once`, `Start`, and `Stop` buttons;
  - status line showing readiness, sent count, or the last error.
- Do not rewrite the row model on each keystroke; keep edits in Rust so Slint's input cursor is not disturbed. Rebuild the model only when the selected topic changes.

## Implementation steps

1. **Planning bookkeeping.**
   - The completed `topic-field-metadata` story has been moved to `completed_stories.md` with completion time `2026-10-27 15:31:10` and removed from `backlog.md`; `dds-echo-send` is now the active story.
2. **`stations-dds`.**
   - Add `Node::from_participant` in `rust/lib/stations-dds/src/dds.rs`.
   - Extend `rust/lib/stations-dds/tests/node_contract.rs` to create a participant and construct a `Node` through the new constructor; verify registered and mis-typed topic behavior.
3. **`dds-echo` resources.**
   - Move `DomainGuard` and domain creation out of `DiscoverySource`.
   - Add the shared `DdsResources`; wire startup discovery to it.
   - Keep inherited `CYCLONEDDS_URI` ignored; the explicit deploy/config path remains authoritative.
4. **`dds-echo` send logic.**
   - Add `src/send.rs` with the form model, schema flattening, typed dispatch, and tests.
   - Parse and validate on send, not on every keystroke.
   - Support bool toggles, numeric and text edits, and nested struct paths such as `attitude.x`.
5. **Wire `main.rs`.**
   - Load the registry into the send runtime (clone from the validated registry).
   - Build the Send tab topic list and select the first topic by default.
   - Create or recreate the `TopicWriter` on topic selection; stop repeating on topic change.
   - Wire field edits and toggles, `Send Once`, `Start`, `Stop`, and interval changes.
   - Use a `slint::Timer` for periodic sends; on write failure, stop repeating and show the error in `send-status`.
6. **Update `ui/dds_echo.slint`.**
   - Add the tab bar, the Send tab, `SendField`, properties, and callbacks.
   - Preserve all current Capture behavior and the topic context menu.
7. **Documentation.**
   - `README.md`: add a short paragraph under the `dds-echo` section describing the Send tab, that it uses `stations-dds` and the shared DDS configuration, and that a unique greeting helps identify sent samples.
8. **Verification.**
   - `.\scripts\build.ps1 -Test`
   - `.\scripts\build.ps1`
   - One manual UI run.
9. **Record completion** in `current_story.md` while leaving the backlog status `in progress` until the user asks to plan the next story.

## Acceptance test

Automated, in `rust/tool/dds-echo/src/send.rs` tests:

- `HelloWorldTopic` and `HelloWorldViewTopic` schemas enumerate the expected fields, including nested `attitude.x/y/z/w`.
- A form built from the schema produces a valid typed sample; edited values survive `to_dynamic_value` -> `from_dynamic_value`.
- Bool editing covers `HelloWorldViewTopic.valid`.
- Invalid numeric input reports the offending field path.
- Every registered topic has a sender handler, so adding a registry entry without a sender fails the test.

Manual, one run (fleet workflow):

1. Start the fleet with `launch-rust.ps1`.
2. Launch `build/deploy/bin/dds-echo.exe` midstream and start Capture.
3. Switch to Send, select `HelloWorldTopic`, set greeting to a unique value (for example `dds-echo-send-test`) and `attitude.w = 1`.
4. Confirm the Send status shows at least one matched subscriber, then press **Send Once**.
5. Verify the view service/display receives the greeting and Capture shows a Known `HelloWorldTopic` record with sender `self` and the decoded greeting.
6. Set the interval to 100 ms, press **Start**, switch to Capture, and verify packets and samples continue; press **Stop** and verify sending stops.
7. Select `HelloWorldViewTopic`, set a unique greeting and `valid`, send; verify the display receives it and Capture decodes `valid`.
8. Enter an invalid float (for example `abc`); verify a Send error appears and no sample is published.
9. Stop the fleet and press **Send Once**; verify Send reports that no subscriber matched and does not claim success.
10. Verify dds-echo's own periodic discovery is labelled `self`/`discovery`, and use **Hide self** to make Capture empty.
11. Confirm Capture Start/Stop/Clear, topic filtering, selection, and the context menu still work.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\build\deploy\bin\dds-echo.exe
```

- `build.ps1 -Test` must pass once.
- `build.ps1` must deploy once.
- No screenshots and no repeated UI runs.
- If a Slint binding or DDS lifecycle problem survives three iterations, stop and report.

## Risks

- **Slint input binding:** updating model rows while typing can move the cursor; keep edits in Rust and rebuild only on topic change.
- **Domain lifetime:** the domain guard must outlive every participant clone; keep it in the shared `Rc<DdsResources>` with participants and nodes declared before `_domain`.
- **`--no-discovery`:** lazy Send resource creation must not silently enable discovery readers; it only creates a participant for publishing.
- **UI-thread writes:** periodic writes run on the Slint event loop. At very high rates this can jitter; validate the interval and treat a dedicated send thread as a follow-up if needed.
- **Registry growth:** the "every registered topic has a handler" test intentionally fails when a topic is added without a sender, forcing the mapping to be updated.
- **QoS and capture visibility:** Send uses the registry QoS; a best-effort sample may be missed by capture, so retry or send once more during acceptance.
- **DDS matching:** a writer sends user data only to matched readers. With no subscriber, the write is local-only; Send must report that rather than claim success.
- **Multicast visibility:** if subscribers receive a sample but Capture does not, the writer chose unicast. Diagnose the shared `AddrsetCosts`/`AllowMulticast` configuration; do not substitute a synthetic record.
- **Self discovery:** dds-echo's own SPDP is expected at ~8 s with the default 10 s lease. It must be labelled `self`/`discovery` and filterable, not shown as unidentified.

## Files touched

- `planning/current_story.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `rust/lib/stations-dds/src/dds.rs`
- `rust/lib/stations-dds/tests/node_contract.rs`
- `rust/tool/dds-echo/src/send.rs` (new)
- `rust/tool/dds-echo/src/main.rs`
- `rust/tool/dds-echo/src/model.rs`
- `rust/tool/dds-echo/ui/dds_echo.slint`
- `README.md`

## Follow-up fixes (active)

The initial Send implementation works, but use revealed two problems:

1. Sending with no matched subscriber reports success while nothing reaches the wire. DDS writers transmit user data only to matched readers; dds-echo's Capture is a raw UDP snooper, not a subscriber.
2. dds-echo's own SPDP appears as `unidentified` every ~8 s. The shared config leaves the CycloneDDS defaults: a 10 s participant lease and `SPDPInterval` at ~80% of the lease (8 s).

The target use case is the running process fleet: launch the fleet, launch dds-echo midstream, send a topic, and all subscribers in the fleet receive it. The follow-up must make that workflow observable and explicit.

### Matched subscribers

- Add `TopicWriter::matched_readers()` backed by `DataWriter::matched_subscriptions()`.
- Show the matched count in the Send status.
- On Send Once/Start, if the count is zero, do not claim success: report "no matching subscriber; sample not transmitted" and do not increment the sent count.
- When the count is positive, perform the normal typed DDS write and report the subscriber count.
- Do not add a raw-RTPS fallback. It would not reach fleet subscribers and would misrepresent DDS delivery.

### Fleet acceptance and multicast visibility

- Reproduce with `launch-rust.ps1` running, then launch dds-echo midstream.
- Confirm `HelloWorldTopic` matches `hello-view-service` and `HelloWorldViewTopic` matches `lil-d`, and that a unique greeting reaches the display.
- Confirm Capture shows the same sample with sender `self`.
- If subscribers receive the sample but Capture does not, diagnose multicast versus unicast: compare with `traffic-throughput.ps1` (which proves separate-process DDS traffic is capturable), inspect the shared `AddrsetCosts`/`AllowMulticast` configuration, and fix the writer's address selection. Do not hide the problem with a synthetic record.

### Self discovery labeling and filtering

- Get dds-echo's own participant GUID prefix with `dds_get_guid` (`DdsEntity` + `cyclonedds_rust_sys`).
- Add a `self` flag to `model::Record` and show `self` in the table's sender column; the detail pane keeps the actual address.
- Classify SPDP/SEDP as `discovery` rather than `unidentified`; show `SPDP participant announcement` / `SEDP endpoint announcement` in the detail pane.
- Exclude self from the remote `participants`/`endpoints` counters while retaining its endpoints for topic attribution.
- Add a **Hide self** checkbox so Capture can be made completely empty when only dds-echo is running.
- Document the ~8 s self SPDP in `README.md`.

### Follow-up tests

- `TopicWriter::matched_readers()` reports zero without a reader and positive with one.
- Self/discovery classification: self records are marked, SPDP/SEDP are bucketed as discovery, and self is excluded from remote counters.
- Hide-self filtering removes self records without hiding remote or user data.
- Fleet manual test above.

## Result

Initial implementation complete; two use issues are being addressed by the follow-up fixes above:

- `Node::from_participant` was added to `stations-dds` and covered by `node_shares_an_existing_participant`.
- `dds-echo` now shares one DDS participant between discovery and Send; `--no-discovery` creates it lazily on first Send.
- `rust/tool/dds-echo/src/send.rs` generates forms from `DdsFieldMetadata`, maps both registered types to typed writers, and rejects unsupported field kinds.
- The window has Capture and Send tabs. Send chooses a registered topic, edits generated fields, and publishes once or periodically at the chosen interval; repeating continues across tabs and stops on topic change.
- Tests cover field enumeration, typed conversion, invalid input, registry coverage, and an end-to-end typed write/read.
- `.\scripts\build.ps1 -Test` passed and `.\scripts\build.ps1` deployed successfully.
- `README.md` documents the Send tab.
- The manual UI acceptance run remains for the user; the release binary starts and creates the shared participant.

## Bookkeeping

- `topic-field-metadata` was completed at `2026-10-27 15:31:10` and moved to `completed_stories.md`.
- `dds-echo-send` is `in progress` in `backlog.md`; its follow-up fixes are active.
- `dds-echo-copy-detail` is the planned next story in `backlog.md`.
- Do not move `dds-echo-send` to `completed_stories.md` until the user asks to plan the next story.
