# Current story: dds-echo-discovery-attribution

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Give `dds-echo` a discovery-only CycloneDDS participant and use the fork's builtin publication/subscription readers to learn writer GUID -> topic, independent of passive SEDP capture; keep raw UDP capture for the datagram list. Re-attribute retained records and topic counts when endpoints are learned, including on Stop. Resolve the registry and CycloneDDS configuration relative to the deployed executable, ignore inherited `CYCLONEDDS_URI`, and remove runtime `REPO_ROOT` defaults from the deployed executables and libraries.

- **Depends on:** none.
- **Minimal test:** double-click `build\deploy\bin\dds-echo.exe` with the services running and no shell environment; `endpoints` is greater than zero, `HelloWorldViewTopic` samples are categorized under Known, and pressing Stop leaves retained rows attributed. The same holds with `dds-echo` started before or after the services and from an unrelated working directory.

## Context

`dds-echo` captures UDP datagrams and parses RTPS/SEDP passively. A non-participant never receives the SEDP directed to the view service's matched reader, so passive capture alone cannot attribute `HelloWorldViewTopic`. The first discovery-participant attempt is committed but failed when the deploy exe was double-clicked: the screenshot shows the Known group loaded but `endpoints: 0` and all rows `unidentified`.

The existing pieces are in place:

- `stations-dds::dds::Node` wraps a participant and exposes `participant()`.
- The pinned `cyclonedds` fork provides `create_builtin_publication_reader` and `create_builtin_subscription_reader`.
- `Discovery` learns endpoints from captured SEDP and attributes records by writer GUID.
- `History` retains up to 1000 records; `TopicCounts` aggregates observed topics and the `unidentified` bucket.
- `dropped`/`trimmed`, the virtualized `ListView`, the 250 ms rebuild throttle, the stop fix, and follow-tail pause/resume are in place.

## Findings

- **The exe-relative config is not authoritative.** `install_dds_config` currently prefers an inherited `CYCLONEDDS_URI` over the exe-relative configuration. Double-clicking inherits any user/system environment value, so a stale or wrong value can silently make the discovery participant use different interfaces while capture still works.
- **Runtime `REPO_ROOT` paths are embedded.** `dds-echo` defaults its registry to a compile-time `REPO_ROOT/contracts/topics.toml` and its config fallback to `REPO_ROOT/DDS/cyclonedds-config.xml`. `hello-view-service` and `display-lib` also fall back to `REPO_ROOT/contracts`. These violate the self-contained deploy rule.
- **Registry currently gates discovery.** The discovery participant is created with `Node::new(domain, registry)`, so a registry failure disables attribution. Discovery does not need the registry.
- `BuiltinEndpointSample` exposes `key()` (full writer/reader GUID), `participant_key()`, `topic_name()`, and `type_name_value()`. Publication samples are writers; subscription samples are readers.
- `DomainParticipant` and `DataReader` have no `Send` implementation in the pinned binding, so polling on the UI thread is the safe first implementation. A 500 ms poll of two builtin readers is cheap.
- `read()` returns the current samples on every poll; `Discovery` deduplicates by GUID.
- Records trimmed from history before an endpoint is learned cannot be re-attributed; their counts may remain `unidentified`.
- The release GUI has no console, so discovery failures are currently invisible when the exe is double-clicked.

## Decisions

1. **Config is exe-relative.** Explicit `--dds-config` wins; otherwise use `<exe_dir>/../config/cyclonedds-config.xml`; otherwise a development CWD fallback `<cwd>/DDS/cyclonedds-config.xml`. Ignore and remove any inherited `CYCLONEDDS_URI`. Create the DDS domain explicitly with the resolved XML through `dds_create_domain`, so CycloneDDS never has to read configuration from the environment.
2. **Registry is exe-relative.** Explicit `--registry` wins; otherwise `<exe_dir>/../config/topics.toml`; otherwise `<cwd>/contracts/topics.toml` for development. No `REPO_ROOT` default.
3. **Discovery is independent of the registry.** Create the participant and builtin readers from a raw `DomainParticipant`, not `Node::new`. The registry is only for Known/Unknown grouping.
4. Use builtin publication and subscription readers only; skip endpoints with empty topic names and built-in `DCPS*` topics.
5. Poll on the UI thread every `DISCOVERY_POLL_MS = 500`.
6. Keep passive SEDP parsing as a fallback.
7. Attribute new records from the discovery map; re-attribute retained history and adjust `TopicCounts` when new endpoints are learned.
8. While the table is paused, update stats/topics counts but defer table rebuilds; on Stop, force one final refresh.
9. **Self-contained deploy artifacts.** Remove runtime `REPO_ROOT` defaults from `hello-view-service` and `display-lib`; use exe-relative deploy paths and development CWD fallbacks. No deployed binary may require `REPO_ROOT` or a repository checkout.
10. Add a discovery log file because the release GUI has no console.

## Implementation

### 1. Path resolution

Add helpers that derive the deploy root from `current_exe()` using parent-directory composition:

- Config: explicit `--dds-config`, else `<deploy>/config/cyclonedds-config.xml`, else `<cwd>/DDS/cyclonedds-config.xml` for development.
- Registry: explicit `--registry`, else `<deploy>/config/topics.toml`, else `<cwd>/contracts/topics.toml` for development.
- Log the chosen path and whether it exists.
- Ignore and remove an inherited `CYCLONEDDS_URI`; log it. Create the domain explicitly with the resolved XML through `dds_create_domain` before creating the participant.

Do not use `REPO_ROOT` or embed absolute repository paths in deployed code.

### 2. Discovery participant

Replace `Node`/`TopicRegistry` ownership in `DiscoverySource` with a raw `DomainParticipant`:

```rust
struct DiscoverySource {
    publications: DataReader<BuiltinEndpointSample>,
    subscriptions: DataReader<BuiltinEndpointSample>,
    _participant: DomainParticipant,
    _domain: Option<DomainGuard>,
    last_poll_ms: u64,
}
```

- Create the DDS domain explicitly with the resolved XML through `dds_create_domain`, then create the participant in that domain.
- Keep the domain in a `DomainGuard` that deletes it after the participant on drop.
- Create the builtin readers from the participant.
- Keep readers declared before the participant so they drop first.
- Poll every 500 ms; map samples as before; return the number of newly learned endpoints.

### 3. Attribution and re-attribution

- Keep `Discovery::learn_endpoint`, `topic_for_writer`, and the passive SEDP path.
- Keep `History::reattribute` and `TopicCounts::reattribute`.
- Poll before draining new records; on new endpoints, re-attribute retained history and counts, mark `pending_view`, and refresh the topic pane.
- If paused, defer the table rebuild; on Stop force a rebuild.

### 4. Discovery diagnostics

Write a log (default `<deploy>/dds-echo-discovery.log`, with an optional `--discovery-log <path>`):

- resolved registry and config paths, and whether they exist;
- whether an inherited `CYCLONEDDS_URI` was ignored;
- domain and discovery enabled/disabled;
- participant creation success/error;
- each poll’s publication/subscription sample counts;
- learned endpoint GUIDs, topics, and types;
- re-attribution changes.

If the log path is not writable, fall back to the temporary directory. Log a clear diagnostic if discovery cannot be created.

### 5. Remove runtime `REPO_ROOT` defaults

- `hello-view-service`: explicit `--config`, then `<exe_dir>/../config`, then `<cwd>/contracts`; remove the `REPO_ROOT` fallback.
- `display-lib`: exe-relative `config` directory, then `<cwd>/contracts`; remove the `REPO_ROOT` fallback.
- Keep tests and build-time code generation on `REPO_ROOT`; they are not deployed.

## Tests and verification

Unit tests:

- Discovery/re-attribution tests continue to pass.
- Existing 64 `dds-echo` tests continue to pass.

Manual verification:

1. Double-click `T:\build\deploy\bin\dds-echo.exe` from Explorer with the services running and no shell environment.
2. Confirm `endpoints > 0` and `HelloWorldViewTopic` under **Known**.
3. Repeat with a deliberately bad user/system `CYCLONEDDS_URI`; it must be ignored.
4. Run from an unrelated working directory.
5. Test both start orders and only `hello-view-service`.
6. Press Stop; retained rows stay categorized and the UI stays responsive.
7. Confirm `lil-d` and `hello-view-service` still work from the deploy layout after removing their `REPO_ROOT` fallbacks.
8. Check that the deployed binaries no longer contain the repository path as a runtime default.
9. If endpoints remain zero, use the discovery log to distinguish participant creation, config/domain, reader samples, and GUID mapping.

## Risks and limitations

- Records trimmed before an endpoint is learned cannot be re-attributed; their counts may remain `unidentified`.
- The discovery participant adds SPDP/SEDP traffic; its own builtin endpoints are filtered, and passive capture may count its SPDP.
- Polling on the UI thread avoids `Send` requirements; keep the interval low-frequency.
- A custom deployment can override the exe-relative config with `--dds-config`.
- Development CWD fallbacks are not part of the deploy contract; the deploy layout is exe-relative only.
- If no config is found, dds-echo creates a default DDS domain and logs a diagnostic; discovery may then see no StationsDDS endpoints.

## Implementation status

- The first discovery-participant attempt failed because setting `CYCLONEDDS_URI` inside the process does not make CycloneDDS use the config; a double-clicked deploy exe therefore used default settings and saw `endpoints: 0`.
- Implemented: authoritative exe-relative config and registry paths, explicit `dds_create_domain` with the config XML, raw `DomainParticipant` discovery independent of the registry, inherited environment removal, discovery logging, and removal of runtime `REPO_ROOT` defaults from `dds-echo`, `hello-view-service`, and `display-lib`.
- Added `--remap-path-prefix` to the release Rust build so deployed binaries do not carry repository paths in source/panic metadata.
- Verified with the `T:\` deploy exe, no shell environment, from an unrelated working directory:
  - both start orders learn the `HelloWorldViewTopic` writer/reader;
  - `total_endpoints` becomes 2 and the first data record is attributed to `HelloWorldViewTopic`;
  - a deliberately bad inherited `CYCLONEDDS_URI` is ignored;
  - Stop remains responsive.
- The 64 `dds-echo` tests pass; deployed binaries no longer contain the repository path.
- The story remains **in progress** pending the user's own test.

## Bookkeeping

This story stays `in progress` until the user asks to plan the next story. Do not move it to `completed_stories.md` before then.
