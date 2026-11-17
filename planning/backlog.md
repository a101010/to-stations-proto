# Backlog

All remaining work lives here; completed stories move to `completed_stories.md`. The backlog is continually groomed; the story order reflects the current best understanding of priority.

Conventions:
* A story's title is a short, unique name. The body is a description followed by `Depends on`, `Minimal test`, and `Status`.
* **Depends on** lists prerequisite stories that are not yet done (toolchain installs are environment prerequisites, not steps inside `build.ps1`).
* **Minimal test** is the smallest runnable check that proves the story is done; toolchain smoke tests are temporary build artifacts under `build/`.
* **Status** is one of `todo` or `in progress`; a story moves to `completed_stories.md` only when the user asks to plan the next story. The active story's detailed plan lives in `current_story.md`.
* IDs, topic names, and file paths follow `architecture.md`.

## Stories

### hello-ctl-service
Create the per-display hello_world controller and view foundation: a per-display `hello-ctl-service`, a per-display `hello-view-service`, ABI v4 mouse motion/wheel input with pointer tracking, a top-band horizontal format menu with four fixed slots (`Formats`, `Inspect`/`Auto`, `Lux Color`/`World`, `Lux Pos`/`World`), Auto/Inspect mode with arcball rotation and inertia, per-display light color and position, Lux placeholder label cycling, a visible paws right-stick `R3` toggle, and Start/B menu toggle/cancel commands. The Lux items are selectable but open no overlays yet. Hello-view state is not persisted.
- **Depends on:** none.
- **Minimal test:** open the menu and verify four left-aligned, equally spaced items with centered labels; cycle the Lux labels; use Start to toggle the menu and B to cancel it; left-drag the globe to enter Inspect and rotate it; right-click does nothing; release quickly to impart inertia; toggle Auto/Inspect from the menu and from the paws `R3` toggle; verify one crosshair follows the mouse, the native arrow is hidden over the format, and a second display is unaffected.
- **Status:** in progress.

### hello-lux-color
Activate the existing `Lux Color` menu item as the color-picker overlay: a hue ring around a saturation/brightness triangle, mouse and controller picking, and the menu item becomes `World` while open. The paws right-stick toggle changes the ring/triangle control only when the cursor is over the ring or triangle. Back/B cancel and restore the previous color; Done commits; World cancels. No persistence.
- **Depends on:** hello-ctl-service.
- **Minimal test:** open `Lux Color`, pick a color, and press Done; the light color changes. Reopen, change the color, and press Back or B; the previous color returns.
- **Status:** todo.

### hello-lux-pos
Activate the existing `Lux Pos` menu item as the light-position overlay: show the globe at its last attitude with a live preview of the moved light, controlled with mouse/controller arcball. The menu item becomes `World` while open. The paws right-stick toggle works anywhere in this overlay. Back/B cancel and restore the previous light position; Done commits; World cancels. No persistence.
- **Depends on:** hello-lux-color.
- **Minimal test:** open `Lux Pos`, move the light, and press Done; the lighting changes. Reopen, move it again, and press Back or B; the previous light position returns.
- **Status:** todo.

### TODO
Need to shift to C++ here. Bring it up to date with hello-lux-pos. Replace this with C++ stories to do so.

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

