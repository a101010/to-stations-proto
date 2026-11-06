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
* The to-stations fork of `cyclonedds-rust` (`https://github.com/a101010/cyclonedds-rust`, branch `multifile`, rev `952906b`), consumed as a pinned git dependency (`planning/cyclonedds-build-plan.md`).
* HarfBuzz 14.6.0 with the experimental GPU library (`libharfbuzz-gpu`), installed to the external prefix (see the `harfbuzz` story).

Third-party libraries are built/installed to an external prefix (for example `C:\Libraries`), never committed. Each install story records its component in `docs/versions.md`.

**Done when.** `launch-rust.ps1` brings up the model service, the view service, and `lil-d` from `build/deploy`; `lil-d` shows the tilted, cube-mapped earth lit from the upper right with the greeting text rendered in the loaded font, its power button works, and the format manager switches between the default and hello-world formats. `dds-echo` observes every topic in the registry, `--topics` and `build/topics.md` match `contracts/`, and `cleanup.ps1` stops everything.

## Later increments

### dds-echo-message-classification
Classify `dds-echo` capture records into separate groups: known topics, unregistered topics, DDS discovery, RTPS control, and unidentified data. Show Discovery, RTPS control, and Unidentified as their own topics-pane groups; keep the Known and Unknown topic rows; update counts, filtering, table labels, detail category, and Hide self. Use builtin writer IDs as a fallback for discovery and non-topic DDS control/liveliness detection.
- **Depends on:** none.
- **Minimal test:** capture a known topic, an unregistered topic, SPDP/SEDP discovery, a heartbeat-only datagram, and an unattributed DATA datagram; each appears in its own group and filters correctly.
- **Status:** in progress.

### multi-lil-d
Add display identity/scoping to the format-manager chain so two `lil-d` instances with different `--station` values can be controlled independently. Add station/display fields to the format-manager topics, identity to the controller/view services, `display_lib_set_identity` and ABI v3, and stamp/filter in the display library; update launch/cleanup for multiple displays.
- **Depends on:** none.
- **Minimal test:** run `lil-d` for station 0 and station 1; clicking one format window changes only that display's format.
- **Status:** todo.

### format-menu
Add a three-dot menu to the format manager, drawn in the upper left over all formats, with one entry per format.
- **Depends on:** none.
- **Minimal test:** the menu switches the active format.
- **Status:** todo.

### format-chooser-format
Create the format-chooser-format: a format for choosing formats.
- **Depends on:** format-menu.
- **Minimal test:** a format can be chosen from the format.
- **Status:** todo.

### random-greeting
Change `hello-service` to choose greetings randomly from its Latin-script list instead of advancing through a fixed sequence; avoid repeating the same greeting twice in a row where practical.
- **Depends on:** none.
- **Minimal test:** run `hello-service` and observe greetings change over the configured interval in a non-fixed order.
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

### dds-echo-send-sequences
Extend the `dds-echo` Send form to edit and send sequence-bearing topics, including `FormatManagerCtlTopic` and `FormatManagerViewTopic`. Render sequence elements as addable/removable rows and write the typed sample.
- **Depends on:** none.
- **Minimal test:** add a window-assignment element in the Send form, publish it, and verify the sample carries the sequence.
- **Status:** todo.

Not yet broken into stories:
* C++ mirror of the Rust libraries, tools, and apps and the `c++` configuration.
* Hybrid configuration and `launch-hybrid.ps1`; `launch-cpp.ps1`.
* Remaining top-level module IDL (control, joystick, per-station/display) and the full topic registry (input, control, joystick, format window/mapping).
* Joystick control service and real game-controller binding.
* Per-station views.
