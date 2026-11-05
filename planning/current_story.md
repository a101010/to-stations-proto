# Current story: dds-echo-message-classification

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Classify captured records into distinct message kinds and show DDS discovery and non-topic RTPS messages in separate topics-pane groups instead of Unknown/unidentified.

- **Depends on:** none.
- **Minimal test:** capture a known topic, an unregistered topic, SPDP/SEDP discovery, a heartbeat-only datagram, and an unattributed DATA datagram; each appears in its own group and filters correctly.

## Context

The `format-ctl-service` story added format-manager topics, and `dds-echo` now decodes them. Classification is a separate `dds-echo` model/UI concern.

Current behavior:

- `tick` forces discovery records to `topic = "discovery"`, so they appear as an unknown topic row.
- RTPS records with no topic, such as HEARTBEAT/ACKNACK/GAP/INFO-only datagrams, count as `unidentified`.
- The UI has only Known and Unknown groups.
- `Class` describes transport/known state, but the model has no message-category concept.

## Decisions

1. **Add a `MessageKind`** independent of the transport `Class`:
   - `KnownTopic`
   - `UnregisteredTopic`
   - `Discovery`
   - `RtpsControl`
   - `UnidentifiedData`
   - `Udp`
2. **Separate topics-pane groups**, as requested:
   - Known (existing)
   - Unknown (existing; unregistered topics only)
   - Discovery
   - RTPS control
   - Unidentified (includes a `udp` row for non-RTPS datagrams)
3. **Classification priority** for mixed datagrams:
   1. known registered topic
   2. unregistered identified topic
   3. discovery announcement or builtin discovery writer
   4. RTPS DATA without an attributed topic -> unidentified data
   5. RTPS without DATA -> RTPS control
   6. non-RTPS -> UDP
4. **Discovery fallback:** recognize SPDP/SEDP by builtin writer entity IDs (`000100c2`, `000003c2`, `000004c2`) in addition to decoded announcements, so failed payload parsing does not misclassify discovery.
5. **Keep filtering and Hide self** working across all groups.
6. **No DDS/service changes.** This is a `dds-echo` model/UI story.
7. **No Send-sequence work.** That remains `dds-echo-send-sequences` at the end of the backlog.

## Model changes

In `rust/tool/dds-echo/src/model.rs`:

- Add `MessageKind` and a `Record.kind` field.
- Add `Record::classify(&KnownTopics) -> MessageKind`.
- Extend `is_discovery()` with the builtin writer entity IDs.
- Add `Record::display_topic()` returning:
  - the topic for known/unregistered topics;
  - `discovery`, `rtps control`, `unidentified`, or `udp` for categories.
- Update `TopicCounts`:
  - keep per-topic counts for known/unregistered topics;
  - add per-kind counts for the non-topic categories;
  - track self-category counts so Hide self can subtract them;
  - return `TopicRow { name, kind, packets }` for all groups.
- Update `TopicSelection`/`History::matching` to filter by `display_topic()`.
- Update `History::reattribute` so a record moving from unidentified to a registered topic:
  - changes its kind;
  - moves its count from `UnidentifiedData` to the topic bucket;
  - preserves self-origin counts.

The `Class` enum remains for transport purposes; `MessageKind` drives grouping.

## UI changes

In `ui/dds_echo.slint`:

- Add models/totals:
  - `discovery-topics`, `discovery-total`
  - `rtps-control-topics`, `rtps-control-total`
  - `unidentified-topics`, `unidentified-total`
- Add three `TopicGroup`s with titles Discovery, RTPS control, and Unidentified.
- Keep Known and Unknown groups.
- The record table's Topic column shows the category label for non-topic records.
- The detail text includes a `kind:` line and uses the category label for `topic:` where appropriate.
- Filter selections use the category rows; the existing multi-select and Unselect All behavior remains.

## Implementation steps

1. **Planning bookkeeping.**
   - Move `format-ctl-service` to `completed_stories.md` with completion time `2026-11-05 16:19:54`; make `dds-echo-message-classification` active.
   - Keep `multi-lil-d` next after classification and `dds-echo-send-sequences` at the end of the backlog.
2. **Model.**
   - Add `MessageKind`, `Record.kind`, classification, display labels, and discovery fallback.
   - Extend `TopicCounts`, `TopicRow`, selection matching, and reattribution.
3. **Main.**
   - Set `record.kind` after `observe_and_attr` using the known registry.
   - Remove the synthetic `record.topic = "discovery"` assignment.
   - Partition rows into the five UI models and update all `refresh_topics` call sites.
4. **UI.**
   - Add the three groups/models/totals and category labels.
5. **Tests.**
   - Classification priority and each category.
   - Builtin SPDP/SEDP writer fallback.
   - Counts/groups, filtering, Hide self, reattribution from unidentified to known.
6. **Verify.**
   - `cargo test -p dds-echo`
   - `.\scripts\build.ps1 -Test`
   - `.\scripts\build.ps1`
   - One manual capture run.

## Acceptance test

Automated:

- SPDP/SEDP (decoded or builtin writer) -> Discovery.
- Heartbeat/ACKNACK-only -> RTPS control.
- DATA with no topic -> Unidentified data.
- Known/unregistered topics keep their current grouping.
- Mixed datagrams follow the priority above.
- Counts and Hide self affect each category.
- Reattribution moves a record/count from unidentified to its topic group.
- The format-manager topics remain decoded under Known.

Manual:

1. Capture the fleet and generate format-manager traffic.
2. Verify format-manager topics decode under Known.
3. Verify discovery appears under Discovery, not Unknown/unidentified.
4. Generate control-only traffic; verify RTPS control.
5. Verify unregistered topic data remains under Unknown.
6. Verify unattributed DATA appears under Unidentified.
7. Select each group/row and verify filtering.
8. Toggle Hide self and verify all groups update.

## Verification

```powershell
cargo test --manifest-path rust/Cargo.toml -p dds-echo
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\build\deploy\bin\dds-echo.exe
```

One manual capture run. No screenshots and no repeated UI runs.

## Risks

- **Mixed datagrams:** classification is per datagram, so the priority above must be explicit and tested.
- **Discovery parsing:** the builtin-writer fallback is required to avoid misclassifying discovery as control/unidentified.
- **Reattribution:** counts and kinds must move together when SEDP arrives late.
- **UI plumbing:** five topic groups/models require updating every `refresh_topics` call site.
- **Filtering:** category rows must participate in the existing multi-select filter and context menu.
- **Non-RTPS UDP:** represented as a `udp` row inside Unidentified rather than a sixth group.

## Files touched

- `rust/tool/dds-echo/src/model.rs`
- `rust/tool/dds-echo/src/main.rs`
- `rust/tool/dds-echo/ui/dds_echo.slint`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

## Result

Not started.

## Bookkeeping

- `format-ctl-service` was completed at `2026-11-05 16:19:54` and moved to `completed_stories.md`.
- `dds-echo-message-classification` is `in progress` in `backlog.md`.
- `multi-lil-d` follows; `dds-echo-send-sequences` remains at the end of the backlog.
- Do not move `dds-echo-message-classification` to `completed_stories.md` until the user asks to plan the next story.
