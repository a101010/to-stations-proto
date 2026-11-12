# Backlog

All remaining work lives here; completed stories move to `completed_stories.md`. The backlog is continually groomed; the story order reflects the current best understanding of priority.

Conventions:
* A story's title is a short, unique name. The body is a description followed by `Depends on`, `Minimal test`, and `Status`.
* **Depends on** lists prerequisite stories that are not yet done (toolchain installs are environment prerequisites, not steps inside `build.ps1`).
* **Minimal test** is the smallest runnable check that proves the story is done; toolchain smoke tests are temporary build artifacts under `build/`.
* **Status** is one of `todo` or `in progress`; a story moves to `completed_stories.md` only when the user asks to plan the next story. The active story's detailed plan lives in `current_story.md`.
* IDs, topic names, and file paths follow `architecture.md`.

## Stories

### paws
Create the game-controller window application (`paws`): a Slint window with an Xbox-style virtual controller, local stick and button interaction, and a reserved controller-selection dropdown that shows `N/A` until binding exists. Add it to the Rust build/deploy and launch/cleanup scripts. Game-controller binding, station identity, and DDS remain later work.
- **Depends on:** none.
- **Minimal test:** `launch-rust.ps1` brings up the `paws` window showing the virtual controller; sticks move and buttons highlight.
- **Status:** in progress.

### xbox-controller
Connect an xbox-controller to the paws app. Allow a 'virtual' mode (replacing N/A in the dropdown) and a controller id for any connected xbox-style controller. The paws window controls reflect the connected controller, or can be controlled by mouse if in virtual mode. DDS connectivity remains out of scope.

### paws-format-ctl
DDS connectivity for the paws app. Show a virtual cursor controlled by the right joystick. An 'A' button press acts the same as a mouse right-click. D-pad moves the current icon selection in the format-chooser-format. The first format is selected by default. paws sends high-level format independent commands to the format manager controller service. The paws app is started with a display id to map it to the appropriate controller services.

### hello-ctl-service
Create the hello_world control service (`hello-ctl-service`): consume input events from the display libraries; user rotations and changes of the light vector and color, specific controls to be refined later. Also allow control via paws.
- **Depends on:** paws-format-ctl
- **Minimal test:** input events change the displayed world.
- **Status:** todo.

### paws-two
Create a game controller window application (`paws-two`): a Slint windows with a saitek-x-52 virtual controller. DDS and real controller connectivity are out of scope.

### saitek-x-52
Connect a saitek x-52 to the paws-two app. Allow a 'virtual' mode (replacing N/A in the dropdown) and a controller id for any connected saitek x-52 controller. DDS connectivity remains out of scope.
- **Depends on:** paws-two

### paws-two-ctrl
Determine control bindings to the format-chooser and hello-world formats for paws-two. After this story, any change that affects controller behavior should determine a common abstract command and a binding to that command for both paws and paws2.
- **Depends on:** paws-two

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
* A vehicle platform concept (multi-turrent tank-like? helecopter? ship? space ship?)
* An 'out-the-window' view for big-d, and maybe even lil-d

