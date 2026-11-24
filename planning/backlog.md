# Backlog

All remaining work lives here; completed stories move to `completed_stories.md`. The backlog is continually groomed; the story order reflects the current best understanding of priority.

Conventions:
* A story's title is a short, unique name. The body is a description followed by `Depends on`, `Minimal test`, and `Status`.
* **Depends on** lists prerequisite stories that are not yet done (toolchain installs are environment prerequisites, not steps inside `build.ps1`).
* **Minimal test** is the smallest runnable check that proves the story is done; toolchain smoke tests are temporary build artifacts under `build/`.
* **Status** is one of `todo` or `in progress`; a story moves to `completed_stories.md` only when the user asks to plan the next story. The active story's detailed plan lives in `current_story.md`.
* IDs, topic names, and file paths follow `architecture.md`.

## Stories

### cpp-paws
Create the C++ `paws` using SDL3, Dear ImGui, and SDL gamepad input: the virtual Xbox controller, real controller selection, per-display persistence, R3 checkbox, Start/B/Back/A/D-pad/right-stick behavior, instance guard, window placement, and the same DDS commands as Rust `paws`. Add it to the C++ deploy and `launch-cpp.ps1`. Finish the pure C++ two-display configuration.
- **Depends on:** none.
- **Minimal test:** `launch-cpp.ps1` with two displays and a temporary `LOCALAPPDATA` runs the pure C++ configuration; hello-world, menu, arcball, Lux Color, Lux Pos, C++ `paws`, persistence, and display isolation work; the Rust configuration still runs.
- **Status:** in progress.

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
