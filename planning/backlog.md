# Backlog

All remaining work lives here; completed stories move to `completed_stories.md`. The backlog is continually groomed; the story order reflects the current best understanding of priority.

Conventions:
* A story's title is a short, unique name. The body is a description followed by `Depends on`, `Minimal test`, and `Status`.
* **Depends on** lists prerequisite stories that are not yet done (toolchain installs are environment prerequisites, not steps inside `build.ps1`).
* **Minimal test** is the smallest runnable check that proves the story is done; toolchain smoke tests are temporary build artifacts under `build/`.
* **Status** is one of `todo` or `in progress`; a story moves to `completed_stories.md` only when the user asks to plan the next story. The active story's detailed plan lives in `current_story.md`.
* IDs, topic names, and file paths follow `architecture.md`.

## Stories

### cmake-skeleton
Create the `cpp/` CMake project and integrate configure/build into `scripts/build.ps1`. Use the Visual Studio 18 2026 generator, C++20, `EXT_PATH_PREFIX`, the install prefix `build/deploy/cpp`, and the repository root resolved without `..`. The project has no committed executable yet; `sdl3-imgui` adds the first real target. No C++ deployment.
- **Depends on:** none.
- **Minimal test:** `scripts/build.ps1 -Test` configures `build/cpp`; the existing Rust tests still pass and no C++ smoke test is generated or committed.
- **Status:** in progress.

### sdl3-imgui
Install SDL3 to the external prefix and add glad and Dear ImGui sources under `cpp/third_party/`. Build a C++ target that opens an SDL3 window on OpenGL 3.3 and renders one Dear ImGui frame over a clear color.
- **Depends on:** cmake-skeleton.
- **Minimal test:** the target opens an SDL3 window, renders one Dear ImGui frame, and closes cleanly.
- **Status:** todo.
- **Records:** SDL3, glad, and Dear ImGui in `docs/versions.md`.

### cpp-dds
Generate C++ types from `DDS/` with the installed `idlc -l cxx` backend into `build/gen/cpp`; add the C++ `stations-dds` library with participant/node creation, typed readers/writers, contract loading and validation, deploy-relative `CYCLONEDDS_URI` setup, and CycloneDDS-CXX linking. Add a header-only TOML parser under `cpp/third_party/` for contracts and state.
- **Depends on:** cmake-skeleton.
- **Minimal test:** a C++ publisher and subscriber exchange `HelloWorldTopic`; an unregistered topic fails validation; generated types match the shared registry.
- **Status:** todo.
- **Records:** TOML parser in `docs/versions.md`.

### cpp-support
Add shared C++ support libraries for named-mutex single-instance guards, window placement state under `%LOCALAPPDATA%\to-stations`, and deploy-relative asset/config lookup. Use the same guard names and state paths as the Rust applications.
- **Depends on:** cpp-dds.
- **Minimal test:** duplicate guard detection, window-state round-trip, and path resolution from a relocated deploy tree.
- **Status:** todo.

### cpp-display-abi
Implement the C++ display library DLL against ABI v4 with the default format. Cover version, create/destroy, identity, render, mouse button/motion/wheel, error reporting, and glad loaded through the ABI's GL loader.
- **Depends on:** sdl3-imgui.
- **Minimal test:** Rust `lil-d` loads the C++ `display_lib.dll`, passes ABI v4, and renders the C++ default format.
- **Status:** todo.

### cpp-display-manager
Add the C++ display manager: format registry from `contracts/formats.toml`, default and format-chooser formats, PNG/JPG icon loading, top-band menu, virtual cursor, input routing, display identity, and the format-manager DDS input/control/view/command endpoints.
- **Depends on:** cpp-display-abi; cpp-dds; cpp-support.
- **Minimal test:** Rust `lil-d` with the C++ display DLL and Rust format services switches formats through the menu and chooser; DDS identity filtering works.
- **Status:** todo.
- **Records:** icon decoder in `docs/versions.md`.

### cpp-lil-d
Create the C++ `lil-d` using SDL3 and Dear ImGui: OpenGL 3.3 window, console and power toggle, display id, display-library loading, full mouse event forwarding, native-cursor hiding over the format, single-instance guard, window placement, and deploy under `build/deploy/cpp/bin` with the SDL3, HarfBuzz, and CycloneDDS runtime DLLs. Create `launch-cpp.ps1`, initially for C++ `lil-d`.
- **Depends on:** sdl3-imgui; cpp-display-abi; cpp-display-manager; cpp-support.
- **Minimal test:** C++ `lil-d` shows the default/chooser formats, forwards mouse input, hides the native cursor over the format, restores window state, and runs from the relocated deploy tree.
- **Status:** todo.

### cpp-format-services
Create C++ `format-ctl-service` and `format-view-service` with per-display assignments, persistence, command relay, contracts, instance guards, and deploy-relative config. Add both executables to the C++ deploy and `launch-cpp.ps1`.
- **Depends on:** cpp-dds; cpp-lil-d; cpp-support.
- **Minimal test:** C++ `lil-d` with C++ format services switches and persists per-window formats; two display ids remain independent.
- **Status:** todo.

### cpp-hello-service
Create C++ `hello-service`: sequential greeting list, attitude model, and 10 Hz `HelloWorldTopic`. Add it to the C++ deploy and `launch-cpp.ps1`.
- **Depends on:** cpp-dds; cpp-support.
- **Minimal test:** Rust `dds-echo` shows C++ `HelloWorldTopic` samples with changing greetings and attitude.
- **Status:** todo.

### cpp-hello-view-ctl
Create C++ per-display `hello-view-service` and `hello-ctl-service`: Auto/Inspect, local attitude and inertia, light, color/HSV, overlay snapshots, frozen Lux Pos attitude, Lux Color/Lux Pos actions, guards, and view relay. Add both to the C++ deploy and `launch-cpp.ps1`.
- **Depends on:** cpp-dds; cpp-support.
- **Minimal test:** with C++ hello service and a display, mode/arcball/light/color actions and two-display isolation match the Rust behavior.
- **Status:** todo.

### cpp-display-hello
Add the C++ hello-world format to the display library: HarfBuzz GPU text, Earth cube map, greeting, Auto/Inspect, arcball and inertia, light/color rendering, menu labels, and chooser integration.
- **Depends on:** cpp-lil-d; cpp-display-manager; cpp-format-services; cpp-hello-service; cpp-hello-view-ctl; cpp-support.
- **Minimal test:** C++ `lil-d` with C++ services shows the live greeting and Earth, rotates with mouse and controller commands, and switches Auto/Inspect.
- **Status:** todo.

### cpp-lux-color
Add the C++ Lux Color overlay with the hue ring, saturation/value triangle, preview, shared Back/Done row, focus and capture, mouse and controller picking, wheel latch, and view actions.
- **Depends on:** cpp-display-hello; cpp-hello-view-ctl.
- **Minimal test:** the Rust Lux Color acceptance test passes with C++ `lil-d` and C++ services.
- **Status:** todo.

### cpp-lux-pos
Add the C++ Lux Pos overlay with the frozen globe, live lighting, spotlight can, front-hemisphere mapping, Back/Done, rate-based right-stick control, R3 no-op, crosshair retention, and overlay transitions.
- **Depends on:** cpp-lux-color; cpp-display-hello; cpp-hello-view-ctl.
- **Minimal test:** the Rust Lux Pos acceptance test passes with C++ `lil-d` and C++ services.
- **Status:** todo.

### cpp-paws
Create the C++ `paws` using SDL3, Dear ImGui, and SDL gamepad input: the virtual Xbox controller, real controller selection, per-display persistence, R3 checkbox, Start/B/Back/A/D-pad/right-stick behavior, instance guard, window placement, and the same DDS commands as Rust `paws`. Add it to the C++ deploy and `launch-cpp.ps1`. Finish the pure C++ two-display configuration.
- **Depends on:** cpp-lil-d; cpp-format-services; cpp-hello-service; cpp-hello-view-ctl; cpp-display-hello; cpp-lux-color; cpp-lux-pos; cpp-support; cpp-dds; sdl3-imgui.
- **Minimal test:** `launch-cpp.ps1` with two displays and a temporary `LOCALAPPDATA` runs the pure C++ configuration; hello-world, menu, arcball, Lux Color, Lux Pos, C++ `paws`, persistence, and display isolation work; the Rust configuration still runs.
- **Status:** todo.

### readme-cleanup
Rewrite `README.md` to focus on setup, build, run, and tool usage. Remove temporary smoke-test details, implementation history, and duplicated architecture/version notes; keep exact external dependency install commands, build/test commands, launch and cleanup commands, and short pointers to `planning/architecture.md` and `docs/versions.md`.
- **Depends on:** none.
- **Minimal test:** follow the README from setup through build, launch, and cleanup without needing another source; no temporary smoke-test artifacts are documented.
- **Status:** todo.

### paws-two
Create a game-controller window application (`paws-two`): a Slint window with a Saitek X-52 virtual controller. DDS and real-controller connectivity are out of scope.
- **Depends on:** none
- **Minimal test:** launch paws-two; the window shows a Saitek X-52 virtual controller and its controls work in `Virtual` mode.
- **Status:** todo.

### saitek-x-52
Connect a Saitek X-52 to the paws-two app. Allow a `Virtual` mode and a controller id for any connected Saitek X-52. The paws-two window reflects the connected controller, or can be controlled by mouse in `Virtual` mode. DDS connectivity remains out of scope.
- **Depends on:** paws-two
- **Minimal test:** with an X-52 connected, select it and verify the paws-two controls mirror it; select `Virtual` and verify mouse control; disconnect and verify fallback to `Virtual`.
- **Status:** todo.

### paws-two-ctrl
Determine control bindings to the format-chooser and hello-world formats for paws-two. After this story, any change that affects controller behavior should determine a common abstract command and a binding to that command for both paws and paws-two.
- **Depends on:** paws-two; saitek-x-52
- **Minimal test:** with paws-two bound to an X-52, its controls drive the format chooser and hello-world through the shared abstract commands; the same commands work from paws.
- **Status:** todo.

### random-greeting
Change `hello-service` to choose greetings randomly from its Latin-script list instead of advancing through a fixed sequence; avoid repeating the same greeting twice in a row where practical.
- **Depends on:** none.
- **Minimal test:** run `hello-service` and observe greetings change over the configured interval in a non-fixed order.
- **Status:** todo.

### big-d
Create the large display application (`big-d`): a large rectangular window with three square format windows and an eyebrow window, surrounded by a console.
- **Depends on:** none.
- **Minimal test:** the window opens with three format windows and the eyebrow.
- **Status:** todo.

### tentacles
Create the launcher application (`tentacles`): a Slint window that can start and stop the applications for a configuration, with an option to launch services with or without console windows.
- **Depends on:** none.
- **Minimal test:** the launcher starts and stops a configuration; services can be launched with or without console windows.
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

## Proto-stories
Here are some backlog items not yet broken into stories.
* C++ mirror of the developer tools (`dds-echo`, `dds-contracts`).
* Hybrid configuration and `launch-hybrid.ps1`.
* Remaining top-level module IDL (control, joystick, display) and the full topic registry (input, control, joystick, format window/mapping).
* more formats
  * A top-down map view
  * A waypoint manager
  * Alpha-numeric entry format overlay (for waypoint mgr and text message format and anything else that needs it)
  * A text message format
  * Turret targeting view?
* A vehicle platform concept (multi-turret tank-like? helicopter? ship? spaceship?)
* An 'out-the-window' view for big-d, and maybe even lil-d
