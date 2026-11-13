# Completed stories

Stories that are done, moved out of `backlog.md` so it stays focused on remaining work. Grouped by increment and listed in the order they were completed; each timestamp is the local commit time (UTC-06:00).

## Increment 1 - Rust MVP

### repo-structure
Create the top-level folders and a `.gitignore` that excludes `build/`.
- **Completed:** 2026-10-06 19:03:27.
- **Depends on:** none.

### host-toolchain
Confirm the Visual Studio Build Tools C++ toolchain and CMake work from a shell, and document how to invoke them (VS generator or developer shell).
- **Completed:** 2026-10-06 19:36:42.
- **Depends on:** none.
- **Minimal test:** configure, build, and run a C++ hello world with the VS generator; record the exact commands in `README.md`.
- **Records:** MSVC and CMake in `docs/versions.md`.

### rust-toolchain
Install rustup with the `stable-x86_64-pc-windows-msvc` toolchain and pin it for the repository.
- **Completed:** 2026-10-06 19:52:21.
- **Depends on:** none.
- **Minimal test:** `cargo run` a hello world; commit `rust/rust-toolchain.toml`.
- **Records:** rustup/rustc in `docs/versions.md`.

### cyclonedds-core
Build and install the Cyclone DDS C library (11.x) to the external prefix with IPv6 enabled.
- **Completed:** 2026-10-06 20:08:07.
- **Depends on:** none.
- **Minimal test:** build with `-DBUILD_EXAMPLES=ON -DENABLE_IPV6=ON` and run the bundled HelloworldPublisher/HelloworldSubscriber.
- **Records:** Cyclone DDS C in `docs/versions.md`.

### short-path
Add `scripts/subst-repo.ps1`, which maps the repository root to a short drive letter (`T:` by default), and `scripts/install-subst-startup.ps1`, which installs that mapping to run at logon via a Startup-folder wrapper. Neither is run by `scripts/build.ps1`.
- **Completed:** 2026-10-06 22:44:44.
- **Depends on:** none.
- **Minimal test:** `subst-repo.ps1` makes `subst` show `T:` pointing at the repository; `install-subst-startup.ps1` creates the Startup entry.

### cyclonedds-cxx
Build and install the C++ binding against the `cyclonedds-core` install.
- **Completed:** 2026-10-06 22:44:44.
- **Depends on:** none.
- **Minimal test:** build and run the C++ hello world example.
- **Records:** cyclonedds-cxx in `docs/versions.md`.

### dds-config
Author `DDS/cyclonedds-config.xml` (IPv6 multicast on `StationsDDS`) and `scripts/verify-loopback.ps1` to verify the loopback adapter and its IPv6 prerequisites. Creating the adapter requires administrator rights and is a documented manual step; document it in `docs/`.
- **Completed:** 2026-10-07 12:23:07.
- **Depends on:** none.
- **Minimal test:** a publisher and subscriber exchange samples bound to the adapter configuration; confirm IPv6 multicast.

### rust-workspace
Create the `rust/` Cargo workspace with one compiling crate.
- **Completed:** 2026-10-07 13:42:22.
- **Depends on:** none.
- **Minimal test:** `cargo build` from `rust/`.

### rust-render
Add the Rust windowing/rendering crates.
- **Completed:** 2026-10-07 14:23:28.
- **Depends on:** none.
- **Minimal test:** a window that renders one egui frame.
- **Records:** the windowing/rendering crates in `docs/versions.md`.

### assets
Add the earth cube-map faces and a font under `assets/`.
- **Completed:** 2026-10-08 09:07:36.
- **Depends on:** none.
- **Minimal test:** both load and render/validate.

### rust-dds-binding
Add the `cyclonedds` Rust crate and `cyclonedds-build` from the to-stations fork of `cyclonedds-rust` (branch `multifile`, rev `acef684`) as git dependencies pinned to that rev. The fork's copy of CycloneDDS C is built from the external prefix (`CYCLONEDDS_SRC`, `CYCLONEDDS_BUILD`) and linked statically so no runtime DLL is needed. IDL codegen is deferred to `hello-world-idl`; this story uses a `#[derive(DdsType)]` type.
- **Completed:** 2026-10-09 16:37:58.
- **Depends on:** none.
- **Minimal test:** a Rust publisher and subscriber exchange one sample of an IDL-defined type.
- **Records:** the `cyclonedds`/`cyclonedds-build` fork in `docs/versions.md`.

### hello-world-idl
Author the hello_world IDL as nested IDL modules and generate Rust into `build/gen/rust` with `scripts/build.ps1`, which runs the fork's `cyclonedds-idlc` Rust codegen (`--no-idlc`) and then builds the Rust workspace. `DDS/math.idl` holds `module math { struct Quaternion { double x; double y; double z; double w; }; };`; `DDS/hello_world.idl` (the compiled root) is `#include "math.idl"` then `module hello_world { struct HelloWorldTopic { string greeting; math::Quaternion attitude; }; };`. One top-level `module` per file, all of that module's types in that one file (a module is not reopened across files); nested IDL `module` maps to a Rust `pub mod`; the DDS type name is the fully-qualified IDL scope (`#[dds_typename]`). The crate has no `build.rs`; it `include!`s the generated file, located via the `REPO_ROOT` environment variable (no `..` paths; Cargo supplies `REPO_ROOT` from `.cargo/config.toml`). The crate moves to `rust/lib/stations-dds`. Generated files are never committed.
- **Completed:** 2026-10-13 12:27:30.
- **Depends on:** none.
- **Minimal test:** `scripts/build.ps1 -Test` generates `build/gen/rust/hello_world.rs`, builds, and passes with `hello_world::HelloWorldTopic::type_name() == "hello_world::HelloWorldTopic"` and `math::Quaternion::type_name() == "math::Quaternion"`.

### lil-d
Create the small display application (`lil-d`): a window and an egui console area, with the region delegated to the display library painted a dark unsaturated green placeholder. The console has a single power button that toggles the format window. Takes `--station N` on the command line. Extends `scripts/build.ps1` to build and deploy the executable to `build/deploy`, and adds `scripts/launch-rust.ps1` and `scripts/cleanup.ps1`; later stories that add executables or libraries extend these scripts as needed.
- **Completed:** 2026-10-14 10:25:12.
- **Depends on:** none.
- **Minimal test:** the window opens showing the console and the green region; the power button toggles the format window; `launch-rust.ps1` starts it and `cleanup.ps1` stops it.

### display-lib
Create the display library (`display-lib`) as a C-ABI DLL and have `lil-d` load it and delegate the format-window region to it. It renders the default format, hardcoded: a solid color (different from the original `lil-d` color) with an x in a different color, owning the entire region delegated to the library. The ABI is the shared header `include/display_lib.h` (implemented in Rust and, later, C++), so a display app in either language can use a display library in either language (the hybrid deployment's in-process interop); `lil-d` selects the DLL with `--display-lib <path>`.
- **Completed:** 2026-10-14 16:18:27.
- **Depends on:** lil-d.
- **Minimal test:** the delegated region shows the default format when `lil-d` loads `display_lib.dll` through the C ABI.

### harfbuzz
Build and install HarfBuzz 14.6.0 with the experimental GPU library (`libharfbuzz-gpu`) to the external prefix, and add the Rust FFI crate `rust/lib/harfbuzz` (raw bindings plus thin wrappers) that links it. The CMake port builds and installs `harfbuzz` and `harfbuzz-gpu`; `hb-gpu.h` is installed under `include/harfbuzz`.
- **Completed:** 2026-10-15 15:36:42.
- **Depends on:** none.
- **Minimal test:** a temporary smoke program under `build/` shapes "Hello, world!" with FiraSans, encodes a glyph with `hb_gpu_draw_encode`, and asserts a non-empty blob and a non-null GLSL fragment shader source.
- **Records:** HarfBuzz (and `libharfbuzz-gpu`) in `docs/versions.md`.

### font-handling
Load a font from `assets/fonts` in the display library and render the hello_world greeting with HarfBuzz (`hb-gpu`) as part of the hello-world format. The greeting is hardcoded for now.
- **Completed:** 2026-10-15 17:01:35.
- **Depends on:** display-lib, harfbuzz.
- **Minimal test:** the greeting renders in the loaded font.

### earth-rendering
Render the cube map on a sphere in the display library, orient it with the attitude quaternion, and light it from the upper right, completing the hello-world format. The cube map, attitude, and light direction are hardcoded for now.
- **Completed:** 2026-10-20 10:05:40.
- **Depends on:** display-lib.
- **Minimal test:** the tilted, lit earth is visible in the delegated region.

### format-manager
Add the format manager to the display library: a registry of the default and hello-world formats and the active format. `lil-d` forwards mouse and other window input events to the library; clicking in the region delegated to the display library switches the active format.
- **Completed:** 2026-10-20 10:43:08.
- **Depends on:** display-lib, font-handling, earth-rendering.
- **Minimal test:** clicking in the delegated region switches between the default and hello-world formats.

### dds-and-service-contracts
Define the DDS layer and the service contracts in `stations-dds`: create the participant from `DDS/cyclonedds-config.xml` and typed readers/writers; define the topic-contract schema and loader for `contracts/topics.toml` and `contracts/services/<app>.toml`; validate each service's declared topics at startup; support a `--topics` flag that prints a service's contract; and write a readable summary to `build/topics.md`. Provisional: the nature and scope of the DDS layer and the contracts still need refinement, and this may be split again.
- **Completed:** 2026-10-21 11:28:57.
- **Depends on:** none.
- **Minimal test:** a typed publish/subscribe round-trip; a sample contract loads, validates, prints with `--topics`, and matches `build/topics.md`.

### dds-echo
Create the `dds-echo` traffic inspector as a Slint GUI mockup driven by simulated traffic: Start/Stop capture, a stats view, a collapsible topics view, and a datagram list with a detail/hex pane. The simulated feed is replaced by real capture in `dds-echo-capture`.
- **Completed:** 2026-10-21 14:45:17.
- **Depends on:** none.
- **Minimal test:** run the app; Start populates stats/topics/table from the simulated feed; Stop halts; selecting a row shows details and hex.

### dds-echo-capture
Replace the mockup's simulated feed with the real UDP capture backend: a multicast/broadcast/unicast listener with `--interface`/`--group`/`--port` (and `--domain`) filters, RTPS detection, and a live hex listing. Uses `socket2` and `if-addrs`; the known-topics tree then reads `contracts/topics.toml`.
- **Completed:** 2026-10-21 16:36:38.
- **Depends on:** dds-echo.
- **Minimal test:** with a publisher running, capture shows RTPS datagrams (flagged) with their hex.

### dds-echo-frame
Parse the RTPS header and submessages (writer GUID, sequence number, sizes) and surface them in the UI.
- **Completed:** 2026-10-21 17:02:33.
- **Depends on:** dds-echo-capture.
- **Minimal test:** a datagram's detail shows submessage ids, writer GUID, and sequence numbers.

### dds-echo-sedp
Parse SEDP/SPDP so the inspector learns the participants, endpoints, and topics (topic/type names) it observes, attributes data traffic to the topics learned from SEDP, and accumulates per-topic packet stats (grouped by topic; unattributable datagrams under the existing `unknown` row).
- **Completed:** 2026-10-22 08:22:37.
- **Depends on:** dds-echo-frame.
- **Minimal test:** the inspector lists the participants/endpoints/topics it observes; per-topic counts accumulate; unattributable datagrams stay under `unknown`.

### dds-echo-topic-filter
Make the left Topics pane a multi-select filter for the center datagram list: clicking a topic row toggles its selection (background-highlighted) and the center list shows only datagrams whose topic matches any selected row (union; nothing selected shows all). The `Topics (packets)` title clears the selection and is always visible; a right-click context menu offers **Unselect All**. Also, spare height in the detail pane collects below the hex rather than between sections.
- **Completed:** 2026-10-22 12:20:22.
- **Depends on:** none.
- **Minimal test:** select `FakeTopicOne` (only its data datagrams remain), add `FakeTopicTwo` (both appear), click the title (all traffic returns), and right-click **Unselect All** clears.

### dds-echo-decode
Decode known-type samples from the wire (`CdrDeserializer`) and show the decoded fields in the detail pane; raw hex otherwise. Also drives the **Known** topics from the registry (`contracts/topics.toml`) via `stations-dds` instead of the hardcoded list.
- **Completed:** 2026-10-22 14:47:12.
- **Depends on:** none.
- **Minimal test:** a known-topic sample is shown decoded (fields) under Known and an unknown sample as hex.

### hello-view-service
Add `HelloWorldViewTopic` and create the hello_world view service that publishes it (a permanent 23.5 deg axial tilt, spun about the tilted polar axis, one revolution per 12 minutes, at 100 Hz best-effort); the display library subscribes to it and renders the received greeting/attitude (black until a sample arrives), moving the model data out of the library.
- **Completed:** 2026-10-22 16:07:11.
- **Depends on:** none.
- **Minimal test:** `dds-echo` shows `HelloWorldViewTopic` samples and the display renders the published view.

### hello-view-service-fixes
Fix the problems seen when running the Rust configuration after `hello-view-service`: `lil-d` opens behind every other window; duplicate services/displays get launched; the view samples are all `unidentified`; and the earth's north-pole point moves on screen instead of the earth spinning about its tilted polar axis. Launching now stops the previous run first; `lil-d` raises and focuses itself; the rotation is pinned by a display-side convention test; the `unidentified` view samples are recorded as a passive-capture limitation, with `dds-echo-discovery-attribution` as the follow-up.
- **Completed:** 2026-10-26 12:51:14.
- **Depends on:** none.
- **Minimal test:** after `cleanup.ps1` and a single `launch-rust.ps1`, `lil-d` opens in front; exactly one `hello-view-service` and one `lil-d` run; the earth spins about a fixed tilted pole; view samples are categorized (or their non-categorization is understood).

### robust-dds-echo
Make `dds-echo` survive many concurrent high-rate streams (for example several 100 Hz topics), not just one: a bounded capture queue with frequency-first eviction that protects low-frequency and one-shot topics; separate visible `dropped` (capture-queue evictions) and `trimmed` (history evictions) counters; lazy hex/decoded formatting on selection rather than per record; a bounded history with a throttled view rebuild instead of per-record O(n) `remove(0)`; and no formatting of records the topic filter excludes. Reopened for real-use bug fixes and completed again: the receive loop observes stop during continuous traffic, the table uses a virtualized `ListView` with a lightweight row model and 250 ms rebuild throttle, and scrolling up freezes the table while stats continue until the user returns to the bottom.
- **Completed:** 2026-10-26 15:33:17.
- **Depends on:** none.
- **Minimal test:** with N concurrent 100 Hz writers plus a one-shot topic, the UI stays responsive, memory stays bounded, `dropped` and `trimmed` are reported, and the one-shot topic's records survive while high-frequency records are trimmed; pressing Stop under live traffic returns immediately; scrolling up freezes the table while stats continue, selecting a row is responsive, and scrolling to the bottom resumes live updates.

### dds-echo-discovery-attribution
Give `dds-echo` a discovery-only CycloneDDS participant and use the fork's builtin publication/subscription readers to learn writer GUID -> topic, independent of passive SEDP capture; keep raw UDP capture for the datagram list. Re-attribute retained records and topic counts when endpoints are learned, including on Stop. Make deployed artifacts self-contained: resolve registry and CycloneDDS configuration relative to the executable, create the domain explicitly with the config XML, ignore inherited `CYCLONEDDS_URI`, remove runtime `REPO_ROOT` defaults from deployed executables and libraries, and remap repository paths out of release binaries.
- **Completed:** 2026-10-27 07:48:08.
- **Depends on:** none.
- **Minimal test:** double-click `build\deploy\bin\dds-echo.exe` with the services running and no shell environment; `endpoints` is greater than zero, `HelloWorldViewTopic` samples are categorized under Known, and pressing Stop leaves retained rows attributed. The same holds with `dds-echo` started before or after the services, from an unrelated working directory, and with an invalid inherited `CYCLONEDDS_URI`; deployed binaries contain no repository path.

### hello-service
Create the hello_world model service (`hello-service`): publish `HelloWorldTopic` at 10 Hz with a greeting that cycles through Latin-script languages every 15-60 seconds, and with the earth attitude moved from the view service (23.5 degree tilt, 12-minute spin). The view service subscribes to `HelloWorldTopic` and relays `HelloWorldViewTopic` at 100 Hz. Add a validity flag to the view topic; after 2 seconds without model data, mark it invalid and show `NoConn hh:mm:ss` with the last received earth position. Before the service first connects, show the NoConn greeting after 2 seconds but no earth. Add the service to build/deploy, launch, and cleanup; update the view service contract.
- **Completed:** 2026-10-27 10:08:01.
- **Depends on:** none.
- **Minimal test:** `launch-rust.ps1` starts the model service, view service, and display; `dds-echo` shows `HelloWorldTopic` and `HelloWorldViewTopic`; the display renders the model-driven world; stopping `hello-service` shows the last earth and a counting `NoConn hh:mm:ss`; restarting resumes valid data; the greeting changes within 15-60 seconds; `cleanup.ps1` stops all three.

### topic-field-metadata
Consume the fork's `DdsFieldMetadata` implementation (fork rev `952906b`): re-pin the Rust dependencies and codegen rev, regenerate the hello_world types, and add a `stations-dds` test that enumerates a generated topic's fields and round-trips a value through the typed sample. `dds-echo` is unchanged; the send form and detail-pane integration come later.
- **Completed:** 2026-10-27 15:31:10.
- **Depends on:** none.
- **Minimal test:** `rust/lib/stations-dds/tests/field_metadata.rs` enumerates `HelloWorldTopic`/`HelloWorldViewTopic` fields from the metadata and round-trips typed values through the dynamic value conversions.

### dds-echo-send
Add a send screen to `dds-echo`: the window becomes two tabs, **Capture** (the current UI) and **Send**. In the Send tab, choose a registered topic from `contracts/topics.toml`, fill in its fields, and publish the sample **once or periodically** at a chosen rate. Publishing uses `stations-dds` (a `Node` typed writer); per-type field forms come from `topic-field-metadata`. Send reports matched subscribers and does not claim success when nothing is transmitted. Label dds-echo's own SPDP/SEDP as `self` discovery and make it filterable.
- **Completed:** 2026-11-04 13:23:16.
- **Depends on:** none.
- **Minimal test:** with the fleet running, fill the hello-world fields and press Send; subscribers receive the sample and it appears on Capture with sender `self`; with no subscriber, Send reports that it was not transmitted. The tool's own periodic discovery is labeled `self`/`discovery` and can be hidden.

### gui-single-instance
Add a reusable Windows named-mutex single-instance guard for manually launched singleton GUI tools, and use it in `dds-echo`. A second `dds-echo` launch restores and foregrounds the existing window, then exits without creating a DDS participant or window. The guard is not used by fleet-launched applications or per-station/per-display windows.
- **Completed:** 2026-11-04 13:48:29.
- **Depends on:** none.
- **Minimal test:** start `dds-echo`, minimize it, and launch a second `dds-echo`; the first window comes to the foreground, the second exits, and only one process/participant remains.

### dds-echo-copy-detail
Make the `dds-echo` detail pane selectable read-only text so the user can select any portion and copy it with Ctrl+C. Render the selected record's time, protocol, sender (including `self`), participant and writer GUIDs, size, topic, submessages, decoded fields, and hex in one selectable text view. No copy button is added.
- **Completed:** 2026-11-04 14:25:18.
- **Depends on:** none.
- **Minimal test:** select a record, select sender lines in the detail pane, press Ctrl+C, and paste into an editor; the pasted text is the selected portion.

### format-ctl-service
Create the per-display format-manager controller and view services. The display library creates logical format events from local input and publishes them; `format-ctl-service` consumes them, owns per-format-window assignments using the shared formats contract, and publishes control; `format-view-service` forwards the assignments to the display library. `lil-d` remains format- and topic-agnostic; the display library owns all format-manager DDS. The story is single-display and unscoped; display identity/scoping comes in `multi-lil-d`. `dds-echo` decodes all three format-manager topics, including the sequence-bearing control/view payloads.
- **Completed:** 2026-11-05 16:19:54.
- **Depends on:** none.
- **Minimal test:** launch the fleet, click in `lil-d`'s format window, and verify the display switches `hello-world` <-> `default` while `dds-echo` shows and decodes the input/control/view topics, including sequence-bearing payloads.

## Later increments

### dds-echo-message-classification
Classify `dds-echo` capture records into separate groups: known topics, unregistered topics, DDS discovery, RTPS control, and unidentified data. Show Discovery, RTPS control, and Unidentified as their own topics-pane groups; keep the Known and Unknown topic rows; update counts, filtering, table labels, detail category, and Hide self. Use builtin writer IDs as a fallback for discovery and non-topic DDS control/liveliness detection.
- **Completed:** 2026-11-09 16:47:08.
- **Depends on:** none.
- **Minimal test:** capture a known topic, an unregistered topic, SPDP/SEDP discovery, a heartbeat-only datagram, and an unattributed DATA datagram; each appears in its own group and filters correctly.
- **Note:** the manual captured-traffic group check remains for the user.

### multi-lil-d
Give every display instance a unique numeric display id and use it for all format-manager routing so multiple displays are independent. Each `big-d`/`lil-d` instance has its own id; there is no station id in the format-manager chain. Added `display_id` to the format-manager topics, identity to the controller/view services, `display_lib_set_identity` and ABI v3, and stamping/filtering in the display library. Persisted each display's window position (logical monitor name, resolution, and monitor-relative position) and its format-manager assignment in per-display state files; defaulted the format to hello-world and placed new displays in a cascade on the main monitor. Rejected duplicate service/display instances with the same display id. Updated launch/cleanup for multiple displays, saved display positions on focus loss/monitor changes, and raised the displays to the front.
- **Completed:** 2026-11-10 16:13:29.
- **Depends on:** none.
- **Minimal test:** run two `lil-d` instances with display ids 0 and 1; clicking one format window changes only that display's format; positions and formats restore after restart; a duplicate display id exits with an error.
- **Note:** the manual click-independence, saved-format restore, and duplicate-`lil-d` dialog checks remain for the user.

### format-menu
Add a three-dot menu to the format manager, drawn in the upper left over all formats, with one entry per format. Added the menu overlay, format-row selection through the existing `Select` path, active-format highlighting, top-left text drawing, and menu layout/hit-test tests.
- **Completed:** 2026-11-10 16:55:09.
- **Depends on:** none.
- **Minimal test:** open the menu, choose a format, and verify the display switches to it.
- **Note:** the manual two-display, outside-click/Next, and saved-format restore checks remain for the user.

### format-chooser-format
Create the format-chooser format: a normal format that presents the other formats in a 4x4 grid of 64x64 CRT-green icon-and-title tiles and switches to the chosen one. Added the chooser, its PNG icons, shared `ui`/`icon` modules, the single `Formats` menu row that is hidden while the chooser is active, and the active-format input API; removed display-library `Next` generation while leaving external `Next` support in the controller.
- **Completed:** 2026-11-12 10:46:13.
- **Depends on:** none.
- **Minimal test:** open the three-dot menu, choose `Formats`, choose a format tile, and verify the display switches to it.
- **Note:** the manual two-display, chooser/menu, and saved-format restore checks remain for the user.

### paws
Create the game-controller window application (`paws`): a Slint window with an Xbox-style virtual controller, local stick and button interaction, and a reserved controller-selection dropdown. Added the crate, virtual controller UI, stick helper and tests, and build/deploy/launch/cleanup integration.
- **Completed:** 2026-11-12 15:33:37.
- **Depends on:** none.
- **Minimal test:** `launch-rust.ps1` brings up the `paws` window showing the virtual controller; sticks move and buttons highlight.
- **Note:** the manual stick/button and second-instance checks remain for the user.
