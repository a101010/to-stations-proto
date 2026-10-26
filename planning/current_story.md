# Current story: dds-echo-discovery-attribution

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Give `dds-echo` a discovery-only CycloneDDS participant and use the fork's builtin publication/subscription readers to learn writer GUID -> topic, independent of passive SEDP capture; keep raw UDP capture for the datagram list. Re-attribute retained records and topic counts when endpoints are learned, including on Stop.

- **Depends on:** none.
- **Minimal test:** with `dds-echo` started before or after the services, `HelloWorldViewTopic` samples are categorized; pressing Stop leaves retained rows attributed.

## Context

`dds-echo` currently captures UDP datagrams and parses RTPS/SEDP passively. A non-participant never receives the SEDP directed to the view service's matched reader, so starting `dds-echo` first does not make `HelloWorldViewTopic` attributable.

The existing pieces are in place:

- `stations-dds::dds::Node` wraps a participant and exposes `participant()`.
- The pinned `cyclonedds` fork provides `create_builtin_publication_reader`, `create_builtin_subscription_reader`, and `create_builtin_participant_reader`.
- `Discovery` learns endpoints from captured SEDP and attributes records by writer GUID.
- `History` retains up to 1000 records; `TopicCounts` aggregates observed topics and the `unidentified` bucket.
- `dropped`/`trimmed`, the virtualized `ListView`, the 250 ms rebuild throttle, the stop fix, and follow-tail pause/resume are in place.

`contracts/topics.toml` is loaded as `KnownTopics`; the `TopicRegistry` itself is not currently kept. The services read `CYCLONEDDS_URI` from the launch script, but a manually launched `dds-echo` must set it itself.

## Findings

- `BuiltinEndpointSample` exposes `key()` (full writer/reader GUID), `participant_key()`, `topic_name()`, and `type_name_value()`. Publication samples are writers; subscription samples are readers.
- `DomainParticipant` and `DataReader` have no `Send` implementation in the pinned binding, so polling on the UI thread is the safe first implementation. A 500 ms poll of two builtin readers is cheap.
- The builtin readers' `read()` returns the current samples on every poll; `Discovery` deduplicates by GUID.
- The passive SEDP path can remain as a fallback and feeds the same `Discovery` structure.
- Records trimmed from history before an endpoint is learned cannot be re-attributed; their counts may remain `unidentified`. Creating the discovery participant at startup minimizes this.
- `CYCLONEDDS_URI` must point at the shared IPv6/StationsDDS configuration before participant creation.

## Decisions

1. Create a discovery-only `Node` at app startup, not on Start, so endpoints are learned before capture. Keep it alive for the app lifetime.
2. Add `--dds-config` and `--no-discovery`. Resolve the config from an explicit path, then `CYCLONEDDS_URI`, then `<exe_dir>/../config/cyclonedds-config.xml`, then `REPO_ROOT/DDS/cyclonedds-config.xml`; set `CYCLONEDDS_URI` if unset.
3. Use builtin publication and subscription readers only; skip the builtin participant reader to avoid counting `dds-echo`'s own participant. Skip endpoints with empty topic names.
4. Poll on the UI thread every `DISCOVERY_POLL_MS = 500`. Revisit a background thread only if the types prove `Send` and polling shows measurable cost.
5. Keep passive SEDP parsing as a fallback.
6. Attribute new records from the discovery map; re-attribute retained history and adjust `TopicCounts` when new endpoints are learned.
7. While the table is paused (scrolled up), update stats/topics counts but defer table rebuilds; on Stop, force one final refresh so retained rows show their topics.

## Implementation

### 1. Registry and configuration

- Refactor `load_known` to also keep the `TopicRegistry` (or add a `load_registry` helper returning both the registry and `KnownTopics`).
- Add CLI arguments:
  - `--dds-config <path>`: optional explicit CycloneDDS XML.
  - `--no-discovery`: run without the discovery participant.
- Resolve and set `CYCLONEDDS_URI` before creating the participant, using forward-slash `file://` paths.
- Log clearly and continue without discovery if participant or reader creation fails.

### 2. `DiscoverySource`

Add a UI-thread struct in `main.rs`:

```rust
struct DiscoverySource {
    _node: Node,
    publications: DataReader<BuiltinEndpointSample>,
    subscriptions: DataReader<BuiltinEndpointSample>,
    last_poll_ms: u64,
}

impl DiscoverySource {
    fn new(domain: u32, registry: TopicRegistry) -> Result<Self, String>;
    fn poll(&mut self, now_ms: u64, discovery: &mut Discovery) -> usize;
}
```

- `poll` runs at most every 500 ms.
- Use `read()` on both readers; map samples:
  - `guid = hex(key().v)`
  - `prefix = hex(participant_key().v[..12])`
  - `kind = Writer` for publications, `Reader` for subscriptions
  - `topic = topic_name()`, `type_name = type_name_value()`
  - skip empty topic names
- Feed `Discovery::learn_endpoint`; return the number of newly inserted endpoints.
- Log read errors; capture continues.

### 3. Discovery API additions

In `model.rs`:

- Add `Discovery::learn_endpoint(endpoint) -> bool`.
- Add `Discovery::topic_for_writer(guid) -> Option<&str>`.
- Keep `observe_and_attr` as the passive SEDP path; make it use `learn_endpoint` and `topic_for_writer`.
- Optionally add `learn_participant` if participant counting is added later.

### 4. Re-attribution

- Add `History::reattribute(&Discovery) -> Vec<(Option<String>, String)>`:
  - Iterate retained records with `topic == None`.
  - Look up the writer GUID; when found, set `record.topic` and record `(None, topic)`.
- Add `TopicCounts::reattribute(old, new)`:
  - Decrement the old bucket (`unidentified` for `None`) and increment the new topic; remove zero buckets.
- After a discovery poll learns new endpoints:
  - Re-attribute history.
  - Apply each change to `TopicCounts`.
  - Mark `pending_view`; if following, allow the next throttled rebuild; if paused, keep the table frozen.
  - Refresh the topics pane immediately.
- On Stop: poll once more, re-attribute, refresh topics, and force one table rebuild so retained rows show their topics.

### 5. Lifecycle and state

- Add `discovery_source: RefCell<Option<DiscoverySource>>` to `State`, initialized at startup.
- In `tick`, poll discovery before draining new records so new data uses the latest map.
- Start/Clear do not recreate or stop the discovery participant. Clear may reset `Discovery`; the next poll repopulates it from current builtin samples.
- Stop performs the final poll/re-attribution/refresh described above.
- `--no-discovery` leaves the passive path as it is today.

## Tests

Unit tests:

- `Discovery::learn_endpoint`, `topic_for_writer`, deduplication, and the passive SEDP path.
- `History::reattribute`: only unattributed records change; attributed records are untouched; returned changes are correct.
- `TopicCounts::reattribute`: `unidentified` decreases, the topic increases, zero buckets are removed, and there is no underflow.
- Existing 61 `dds-echo` tests continue to pass.

Manual verification:

1. Start `dds-echo`, press Start, then launch the services: `HelloWorldViewTopic` is categorized.
2. Start the services first, then `dds-echo` and press Start: also categorized.
3. Press Stop: retained rows remain categorized and the UI stays responsive.
4. With 4 concurrent 100 Hz streams: attribution, `dropped`/`trimmed`, memory, follow-tail pause/resume, and Stop remain healthy.
5. Run with `--no-discovery` and confirm passive behavior and an appropriate diagnostic.
6. Run with `--dds-config` pointing at the deployed configuration to confirm the participant uses StationsDDS.

## Verification commands

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
```

Then launch the services and `build\deploy\bin\dds-echo.exe` in both start orders.

## Risks and limitations

- Records trimmed before an endpoint is learned cannot be re-attributed; their counts may remain `unidentified`. Startup discovery and retained-record re-attribution minimize this.
- The discovery participant generates normal SPDP/SEDP traffic; its own builtin endpoints are filtered by empty topic names. The passive capture may still observe `dds-echo`'s own SPDP, so the participant count may include it.
- Polling on the UI thread is chosen because the binding types are not `Send`; keep the interval low-frequency and the work small.
- A wrong or missing `CYCLONEDDS_URI` means the participant will not use the StationsDDS IPv6 configuration; log a clear diagnostic and allow `--dds-config`.
- No contract or topic changes are required; `stations-dds` already exposes the participant needed for the builtin readers.

## Bookkeeping

This story stays `in progress` until the user asks to plan the next story. Do not move it to `completed_stories.md` before then.
