# Backlog

All remaining work lives here; completed stories move to `completed_stories.md`. The backlog is continually groomed; the story order reflects the current best understanding of priority.

Conventions:
* A story's title is a short, unique name. The body is a description followed by `Depends on`, `Minimal test`, and `Status`.
* **Depends on** lists prerequisite stories that are not yet done (toolchain installs are environment prerequisites, not steps inside `build.ps1`).
* **Minimal test** is the smallest runnable check that proves the story is done; toolchain smoke tests are temporary build artifacts under `build/`.
* **Status** is one of `todo` or `in progress`; a story moves to `completed_stories.md` only when the user asks to plan the next story. The active story's detailed plan lives in `current_story.md`.
* IDs, topic names, and file paths follow `architecture.md`.

## Increment 1 - Rust MVP

**Goal.** The minimum viable code product: the `rust` hello_world constellation end to end on one machine.

**Scope.**
* Language: Rust only. C++ and hybrid are later increments.
* DDS: Eclipse Cyclone DDS; all topics over IPv6 multicast on the `StationsDDS` loopback adapter; shared IDL in `DDS/`.
* Rendering: winit + glutin/glow + egui.
* The model service and the view service are separate: the model publishes the greeting and the attitude (a 23.5 degree tilt, as a quaternion; one revolution every twelve minutes is added later); the view publishes the display message.
* Earth projection: a cube map sampled on a sphere, oriented by the attitude quaternion, lit from the upper right.
* Font handling (loading and rendering the greeting) is in scope; format text uses HarfBuzz's experimental GPU library (`libharfbuzz-gpu`).
* Small display (`lil-d`): one square format window plus a console area with a single power button, `--station N`, and `--topics`.
* The display library hosts a default format and the hello-world format and a format manager that switches the active format on a click in the region delegated to it.
* Input handling is added per feature; the first is the format-manager click.
* Out of scope: the C++/hybrid configurations and their launch scripts; joystick control service and `paws`; the format-manager controller service, the three-dot menu, and the format-chooser-format; `big-d` and `tentacles`; real game-controller binding; per-station views.

**Prerequisites** (environment, installed once, not built by `scripts/build.ps1`):
* MSVC (Visual Studio Build Tools) and CMake.
* Rust toolchain `stable-x86_64-pc-windows-msvc`.
* A short repository path on Windows (`scripts/subst-repo.ps1`; see `architecture.md`, "Path length").
* Eclipse Cyclone DDS C core (11.x) and the Microsoft loopback adapter named `StationsDDS` with IPv6 multicast.
* The to-stations fork of `cyclonedds-rust` (`https://github.com/a101010/cyclonedds-rust`, branch `multifile`, rev `acef684`), consumed as a pinned git dependency (`planning/cyclonedds-build-plan.md`).
* HarfBuzz 14.6.0 with the experimental GPU library (`libharfbuzz-gpu`), installed to the external prefix (see the `harfbuzz` story).

Third-party libraries are built/installed to an external prefix (for example `C:\Libraries`), never committed. Each install story records its component in `docs/versions.md`.

**Done when.** `launch-rust.ps1` brings up the model service, the view service, and `lil-d` from `build/deploy`; `lil-d` shows the tilted, cube-mapped earth lit from the upper right with the greeting text rendered in the loaded font, its power button works, and the format manager switches between the default and hello-world formats. `dds-echo` observes every topic in the registry, `--topics` and `build/topics.md` match `contracts/`, and `cleanup.ps1` stops everything.

### robust-dds-echo
Make `dds-echo` survive many concurrent high-rate streams (for example several 100 Hz topics), not just one: a bounded capture queue with frequency-first eviction that protects low-frequency and one-shot topics; separate visible `dropped` (capture-queue evictions) and `trimmed` (history evictions) counters; lazy hex/decoded formatting on selection rather than per record; a bounded history with a throttled view rebuild instead of per-record O(n) `remove(0)`; and no formatting of records the topic filter excludes. Reopened for real-use bug fixes: the stop check, virtualized `ListView`, and rebuild throttle are in place; scrolling up must pause live table updates and selection must stay responsive while paused.
- **Depends on:** none.
- **Minimal test:** with N concurrent 100 Hz writers plus a one-shot topic, the UI stays responsive, memory stays bounded, `dropped` and `trimmed` are reported, and the one-shot topic's records survive while high-frequency records are trimmed; pressing Stop under live traffic returns immediately; scrolling up freezes the table while stats continue, selecting a row is responsive, and scrolling to the bottom resumes live updates.
- **Status:** in progress.

### dds-echo-discovery-attribution
Give `dds-echo` a discovery-only CycloneDDS participant and use the fork's builtin publication/subscription readers to learn writer GUID -> topic, independent of passive SEDP capture; keep raw UDP capture for the datagram list. Starting `dds-echo` first does not fix attribution: SEDP is not visible to a non-participant sniffer. Re-attribute retained records and topic counts when endpoints are learned, including on Stop. Split out of `hello-view-service-fixes`, where the `unidentified` view samples were accepted as a passive-capture limitation.
- **Depends on:** none.
- **Minimal test:** with `dds-echo` started before or after the services, `HelloWorldViewTopic` samples are categorized; pressing Stop leaves retained rows attributed.
- **Status:** todo.

### hello-service
Create the hello_world model service (`hello-service`): publish `HelloWorldTopic` (greeting text and quaternion attitude) at a fixed rate, a 23.5 degree tilt. The model data moves out of the view service to this service, and the view service now subscribes to it and relays. One revolution every twelve minutes is added later.
- **Depends on:** none.
- **Minimal test:** `dds-echo` shows `HelloWorldTopic` samples and the display shows the model-driven world.
- **Status:** todo.

### topic-field-metadata
Extend the fork's code generation (the `cyclonedds-build` codegen or `cyclonedds-derive`) to emit a per-struct field schema (reusing the generated `DynamicTypeSchema`/`DynamicValue` shapes) plus field get/set, so tools can build generic field views for display and editing without runtime reflection (Rust has none). Consumed by `dds-echo`'s detail pane and send form and by `tentacles-injector`.
- **Depends on:** none (a fork change; re-pin the fork rev).
- **Minimal test:** enumerate a generated topic's fields from the metadata and round-trip a value to and from the typed sample.
- **Status:** todo.

### dds-echo-send
Add a send screen to `dds-echo`: the window becomes two tabs, **Capture** (the current UI) and **Send**. In the Send tab, choose a registered topic from `contracts/topics.toml`, fill in its fields, and publish the sample **once or periodically** at a chosen rate. Publishing uses `stations-dds` (a `Node` typed writer); per-type field forms come from `topic-field-metadata`.
- **Depends on:** topic-field-metadata.
- **Minimal test:** in the Send tab, fill the hello-world fields and press Send; the sample appears on the Capture tab.
- **Status:** todo.

## Later increments

### format-ctl-service
Create the format-manager controller service: the controller of the per-display format-manager constellation, controlling which format each display shows.
- **Depends on:** none.
- **Minimal test:** the service controls the active format on a display.
- **Status:** todo.

### format-menu
Add a three-dot menu to the format manager, drawn in the upper left over all formats, with one entry per format.
- **Depends on:** format-ctl-service.
- **Minimal test:** the menu switches the active format.
- **Status:** todo.

### format-chooser-format
Create the format-chooser-format: a format for choosing formats.
- **Depends on:** format-menu.
- **Minimal test:** a format can be chosen from the format.
- **Status:** todo.

### hello-ctl-service
Create the hello_world control service (`hello-ctl-service`): consume input events from the display libraries; user rotations and changes of the light vector and color, to be refined later.
- **Depends on:** none.
- **Minimal test:** input events change the displayed world.
- **Status:** todo.

### big-d
Create the large display application (`big-d`): a large rectangular window with three square format windows and an eyebrow window, surrounded by a console.
- **Depends on:** none.
- **Minimal test:** the window opens with three format windows and the eyebrow.
- **Status:** todo.

### tentacles
Create the launcher application (`tentacles`): a Slint window that can start and stop the applications for a configuration.
- **Depends on:** none.
- **Minimal test:** the launcher starts and stops a configuration.
- **Status:** todo.

### paws
Create the game-controller window application (`paws`): a (Slint) window with a virtual controller that can be bound to a game controller.
- **Depends on:** none.
- **Minimal test:** the window shows a virtual controller.
- **Status:** todo.

### sdl3-imgui
Install SDL3 to the external prefix and add glad and Dear ImGui sources under `cpp/third_party/`.
- **Depends on:** none.
- **Minimal test:** a window that renders one Dear ImGui frame over a clear color.
- **Status:** todo.
- **Records:** SDL3, glad, and Dear ImGui in `docs/versions.md`.

### cmake-skeleton
Create the `cpp/` CMake project with one compiling target.
- **Depends on:** none.
- **Minimal test:** configure and build one runnable target.
- **Status:** todo.

### dds-override-layer
A communication layer in the DDS stack (`stations-dds`) through which a controller can override individual fields of the messages a service is about to publish. A writer consults the active override set (keyed by topic and, when applicable, key) before serializing, so the real message flow is preserved and only the chosen fields change. The mechanism (a DDS control topic consumed by each service, an in-process hook in `stations-dds`, or shared state) is not yet decided.
- **Depends on:** none.
- **Minimal test:** an override set for a topic field causes an otherwise-unmodified published sample to carry the overridden value.
- **Status:** todo.

### tentacles-injector
Add a known-message inspector/injector as a bottom panel of the `tentacles` launcher GUI. Unlike `dds-echo`, which originates stand-alone samples, it does not send messages of its own: it intercepts the messages a service is about to publish and overrides individual fields of them, leaving the rest of the real flow intact. The panel reuses the per-topic field knowledge from `topic-field-metadata` and drives the `dds-override-layer`.
- **Depends on:** tentacles; topic-field-metadata; dds-override-layer.
- **Minimal test:** in the tentacles panel, select a known topic, override one field, and observe the published sample carrying the overridden value while every other field is unchanged.
- **Status:** todo.

Not yet broken into stories:
* C++ mirror of the Rust libraries, tools, and apps and the `c++` configuration.
* Hybrid configuration and `launch-hybrid.ps1`; `launch-cpp.ps1`.
* Remaining top-level module IDL (control, joystick, per-station/display) and the full topic registry (input, control, joystick, format window/mapping).
* Joystick control service and real game-controller binding.
* Per-station views.
