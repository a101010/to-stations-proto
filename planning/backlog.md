# Backlog

All remaining work lives here; completed stories move to `completed_stories.md`. The backlog is continually groomed; the story order reflects the current best understanding of priority.

Conventions:
* A story's title is a short, unique name. The body is a description followed by `Depends on`, `Minimal test`, and `Status`.
* **Depends on** lists prerequisite stories that are not yet done (toolchain installs are environment prerequisites, not steps inside `build.ps1`).
* **Minimal test** is the smallest runnable check that proves the story is done; toolchain smoke tests are temporary build artifacts under `build/`.
* **Status** is one of `todo` or `in progress`; a story moves to `completed_stories.md` only when the user asks to plan the next story. The active story's detailed plan lives in `current_story.md`.
* IDs, topic names, and file paths follow `architecture.md`.

## Stories

### window-placement
Persist and restore window position and inner size for `lil-d`, `paws`, and `dds-echo` through the shared `window-position` library. The existing `window-position` extraction and `paws` position restore are the first part of this story; extend them with size and `dds-echo`.
- **Depends on:** none.
- **Minimal test:** move and resize `lil-d`, `paws`, and `dds-echo`; close and relaunch each; verify both size and position restore. Legacy state without size still loads.
- **Status:** in progress.

### paws-format-ctl
DDS connectivity for the paws app. Show a virtual cursor controlled by the right joystick. An `A` button press acts the same as a mouse right-click. D-pad moves the current icon selection in the format-chooser format. The first format is selected by default. paws sends high-level format-independent commands to the format manager controller service. The paws app uses the display id already supplied for controller persistence to map it to the appropriate controller services.
- **Depends on:** none
- **Minimal test:** launch paws with a display id; the right stick moves the virtual cursor, `A` acts as a right-click, the D-pad moves the format-chooser selection with the first format selected by default, and the format controller receives the commands.
- **Status:** todo.

### hello-ctl-service
Create the hello_world control service (`hello-ctl-service`): consume input events from the display libraries; user rotations and changes of the light vector and color, specific controls to be refined later. Also allow control via paws.
- **Depends on:** paws-format-ctl
- **Minimal test:** input events change the displayed world, including inputs sent from paws.
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
- **Depends on:** paws-two; saitek-x-52; paws-format-ctl
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

## Proto-stories
Here are some backlog items not yet broken into stories.
* C++ mirror of the Rust libraries, tools, and apps and the `c++` configuration.
* Hybrid configuration and `launch-hybrid.ps1`; `launch-cpp.ps1`.
* Remaining top-level module IDL (control, joystick, display) and the full topic registry (input, control, joystick, format window/mapping).
* more formats
  * A top-down map view
  * A waypoint manager
  * Alpha-numeric entry format overlay (for waypoint mgr and text message format and anything else that needs it)
  * A text message format
  * Turret targeting view?
* A vehicle platform concept (multi-turret tank-like? helicopter? ship? spaceship?)
* An 'out-the-window' view for big-d, and maybe even lil-d

