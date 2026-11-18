# To-Stations Prototype
The to-stations prototype is a set of distributed displays and services.
There are three configurations:
* rust
* c++
* hybrid

Each configuration has a launch script. They share a cleanup script. The launch script starts all of the displays and services for that configuration. The cleanup script stops all processes for all configurations.

Each configuration has two display applications. 
* Small display - Has a single square window that can display one 'format'. This window is surrounded by a console area that can display buttons and controls.
* Large display - Has a large rectangular windows that can display three square formats, with a narrow 'eyebrow' window at the top. This rectangular window is surrounded by a console area that can display buttons and controls.

Each application is in charge of rendering the console. It delegates rendering the window to a display library.

Display applications take a unique numeric display id on the command line.

Each configuration launches a large and a small display, each with its own unique numeric display id.

Each station has a service that represents a joystick controller. For each of these services, a separate window application with a virtual controller will be launched. This window can also be bound to one game controller.

Each configuration launches a group of service applications that can run either headless or with a displayed console window.

It also launches a window that can start and stop the applications for that configuration.

All the applications communicate using Distributed Data Services configured to use IPv6 multicast on a Microsoft loopback adapter named StationsDDS.

## Stack
* DDS: Eclipse Cyclone DDS for all configurations - C core with `cyclonedds-cxx` (C++) and the `cyclonedds` Rust crate; shared IDL in `DDS/`; all topics over IPv6 multicast on the `StationsDDS` loopback adapter. The shared configuration (`DDS/cyclonedds-config.xml`) tunes CycloneDDS's address-set cost model (`AddrsetCosts`) to prefer multicast for user data as well as discovery, since its defaults send a lone reader's data by unicast.
* Rendering: the Rust display apps use winit + glutin/glow + egui and the display library renders through their GL context; the Rust tool, launcher, and controller GUIs (`dds-echo`, `tentacles`, `paws`) use Slint. C++ display apps and GUIs use SDL3 + glad + Dear ImGui, and C++ `paws` uses SDL gamepad input. Slint C++ remains a future option for sharing `.slint` files.
* Text: HarfBuzz (`libharfbuzz` and the experimental `libharfbuzz-gpu`) for format text; the console uses egui's own text.
* Build: `scripts/build.ps1` orchestrator; all artifacts under `build/`, where only `build/deploy/` is durable and everything else is temporary. The repository must be placed at a short path on Windows (see "Path length").

### OpenGL context and safety
`glow` marks every GL call `unsafe` because a GL context must be current on the calling thread, and the binding has no token that proves currentness. The Rust rendering libraries follow `egui_glow`: their public APIs are safe and document the assumption that a glow context is current on the calling thread, which the display application upholds by doing all GL work on its event-loop thread. This documented-assumption approach is limited to the GL-context-current precondition; other safety concerns are handled normally. The context is requested explicitly (OpenGL 3.3 core) so the shaders the libraries compile are guaranteed. The display application also requests a depth buffer, which the earth rendering uses for depth testing; the ABI carries no depth information, so a depth buffer is a documented precondition the display application must uphold.

### Display library ABI
A display application and its display library are separate components joined by a shared C ABI (`include/display_lib.h`), not by static linking. The Rust and C++ display libraries both implement that ABI, and the Rust and C++ display applications both consume it, so any display application can use either language's display library (part of the hybrid deployment). The application supplies the OpenGL proc-address loader, so the ABI is independent of the windowing stack (winit/glutin or SDL3); the ABI carries only C types. The display library is deployed as a DLL next to its display application in that configuration's bin directory (`build/deploy/bin` for Rust, `build/deploy/cpp/bin` for C++), and the launch script or configuration chooses which implementation's DLL is loaded. The ABI is versioned (`display_lib_abi_version`) so a mismatched pair fails cleanly. The ABI also carries input: the display application forwards the window events a feature needs to the display library, which hit-tests the region it was last asked to render and routes input to the active format. ABI v4 adds `display_lib_mouse_motion` and `display_lib_mouse_wheel`, and the display application forwards all mouse buttons, motion, and wheel events. Touch input is deferred; the display application forwards mouse events only in this increment. Format-specific input is published by the display library on DDS. The library locates its own assets (fonts, format icons, and the earth cube map) at the deploy root's `assets` directory, derived from the process executable path; no asset path crosses the ABI.

### Text rendering
Format text is rendered with HarfBuzz's experimental GPU library (`libharfbuzz-gpu`, the Slug algorithm). The display library shapes text with `hb_shape`, encodes each glyph's outline on the CPU into a compact `RGBA16I` blob, uploads the blobs into a single `GL_TEXTURE_BUFFER` atlas, and rasterizes them in a `#version 330` fragment shader supplied by HarfBuzz: the library concatenates `hb_gpu_shader_source` and `hb_gpu_draw_shader_source` ahead of its own `main()`. The result is resolution-independent and antialiased, and the same shader and blob format are used by the Rust and C++ display libraries. It requires OpenGL 3.3, which the display applications already request (see "OpenGL context and safety"); there is no font hinting. The `harfbuzz` story installs the library and the `harfbuzz` Rust crate wraps it.

## Configurations
The three configurations (`rust`, `c++`, `hybrid`) differ only in which language implementation of each application is launched. Applications interoperate because they share the same IDL types and speak DDSI-RTPS; a hybrid launch simply mixes the two implementations. A display application can also use either language's display library: both implement the same C ABI (`include/display_lib.h`), so a Rust display app can load the C++ display library and vice versa. That in-process interop is the hybrid deployment's other half, alongside the DDS/IDL interop.

The C++ runtime deploys under `build/deploy/cpp/bin` so it can coexist with the Rust runtime in `build/deploy/bin`; both use the shared `build/deploy/assets` and `build/deploy/config`. C++ executables resolve config and assets from their deploy location. SDL3 is installed as a static library under the external prefix; glad and Dear ImGui source is copied into `cpp/third_party/` and built as static C++ libraries. `scripts/build.ps1` generates C++ IDL with the installed `idlc -l cxx` backend into `build/gen/cpp`, builds the `cpp/` CMake project into `build/cpp`, runs C++ tests through CTest when the project defines them, and assembles the C++ deploy directory. C++ services and shared libraries use the same contracts, instance guards, state paths, and DDS topics as Rust. The C++ `stations-dds` library mirrors the Rust contract-aware Node and generated-type workflow, using toml++ for contract parsing.

## Paws
`paws` is the game-controller window application (`rust/ui/paws`), a Slint UI with an Xbox-style virtual controller. One instance is launched per display id; `paws` takes a required `--display-id`. Its controller dropdown lists `Virtual` plus each connected Xbox-style controller with its controller id, using gilrs's XInput backend on Windows. `Virtual` mode uses mouse input; controller mode mirrors the selected controller and ignores mouse input. The selected controller is persisted per display id in `paws\<display_id>.toml`: startup restores the remembered controller if it is connected, otherwise `Virtual`; a remembered controller that appears later is selected automatically, and no other controller is auto-selected. Selecting `Virtual` clears the remembered controller. The window also restores and remembers its position and inner size per display id through the shared `window-position` library, using the same monitor, resolution, visibility, and cascade rules as `lil-d`. `paws` publishes high-level format commands on `FormatManagerInputTopic` using its display id: right-stick cursor moves, D-pad navigation, A/right-click pointer events, right-stick press toggles, Start menu toggles, B menu cancels, and Back overlay cancels. A local, clearly visible `R3` checkbox-style toggle next to the right stick represents the virtual right-stick press; it shows `Off`/`On`, defaults unchecked, toggles with the physical press or a click, and does not subscribe to view state or know about formats. Outside overlays R3 toggles arcball; inside Lux Color it switches hue-ring/triangle focus only when the cursor is over one of those controls; inside Lux Pos it does nothing. Paws also emits a zero `CursorMove` when the right stick reaches rest, which resets the Lux Color wheel direction latch. `format-ctl-service` relays those commands on `FormatManagerCommandCtlTopic` to the display library. `paws` has no station identity, and the joystick control service is later work. Each display id has at most one `paws` instance.

## Formats and the distributed MVC constellation
A *format* is a distributable model-view-controller constellation. Each format owns the following services:

* **Model service** - owns or derives the data for the format.
* **View service** - sends all data to be displayed to the display library of each interested display via a DDS message.
* **Display library** (the view) - renders the data it receives; handles the input events its display application forwards to it and, when a format requires it, sends events to the controller service via DDS. Input handling is added per feature.
* **Controller service** - consumes input events from the display libraries and control events from control services.
* **Control services** - interface to game controls (for example, a joystick controller service) and send events to the display control services.

The display applications themselves know very little; most of their work is delegated to the display library instance each of them hosts.

Each format has its own instance of this constellation, so formats are independent and can be distributed across the system. An editor format, for example, could display different data on different displays by using display-scoped view services.

### Scope: system or display
View and controller elements of a constellation can be system-wide or per display, depending on the requirements of that format.

* All topics travel over IPv6 multicast, so every topic is reachable everywhere on the network.
* A topic that varies carries the relevant `display_id` field in its IDL type; a system-wide topic carries none. Fields are added to a type only when a format requires them.
* A view or controller service that serves one display receives that display id as a command-line argument and publishes the corresponding field.
* A display library receives its display id from its display application's command line; the format's own implementation decides which instances of a topic it listens to. That choice is part of the format's implementation, not separate run-time configuration.

The hello_world format has a global model but per-display view and control: each display has its own `hello-view-service` and `hello-ctl-service`, which own that display's view mode, local attitude, overlay state, light color, light position, the color overlay's HSV state and pre-overlay snapshot, and the light overlay's frozen attitude and pre-overlay light. The global model supplies the greeting and automatic attitude.

## Format windows and the format manager
A *format window* is a place where a format can be displayed: one on the small display (`lil-d`) and three on the large display (`big-d`). The eyebrow on `big-d` is a separate window; it is not a format window and is not controlled by the format manager.

The format manager is **per display** and is itself a model-view-controller constellation. It tells that display's display library which format to display in each format window, and that combination is different for each display. It runs as separate controller and view service processes (`format-ctl-service` and `format-view-service`), not inside the display application.

The display application remains format- and topic-agnostic: it loads the display library, gives it the render regions, and forwards local pointer events. It hides the native cursor over the format region so the display library's crosshair is the only cursor there; the console keeps the native cursor. The display library tracks the local pointer position and pressed buttons and routes them to the active format. The display library owns the format-manager DDS: it subscribes to assignments and command control, hit-tests local input against the format windows, creates logical `Select` events, and publishes them to the controller (which also accepts externally published `Next` and high-level commands from `paws`). The controller owns the per-format-window assignments, using the shared `contracts/formats.toml`; it publishes control to the view service, which forwards assignments to the display library. For high-level control commands it validates the display id and relays them unchanged on `FormatManagerCommandCtlTopic` to the display library; commands are transient and are not persisted.

Each display has a unique numeric display id. Its controller and view service are launched with that id, and the format-manager topics carry `display_id`; the display library stamps its input events and filters incoming assignments by it. The display ABI v3 added `display_lib_set_identity` for this purpose; ABI v4 adds mouse motion and wheel input.

The display library draws a three-dot button in the top-left band of each format window, except while the format chooser is active. The button and the open menu occupy the same top band, flush with the format window's left edge; opening the menu hides the button, and the former button area becomes the first item. The menu is a single horizontal row of equal, fixed-width, left-aligned item slots with centered labels and tight consistent gaps; it has no wrap, shrink-to-fit, or overflow fallback. `Formats` is always the first item and publishes a `Select` event for the `format-chooser`; the active format contributes its own items after it. For `hello-world`, the format-specific items are `Inspect`/`Auto`, `Lux Color`/`World`, and `Lux Pos`/`World`; labels come from the per-display view state. `Lux Color` opens the real color overlay described below; `Lux Pos` opens the real light-position overlay described below. Local left-clicking outside an open menu closes it without changing the format; local left-clicks elsewhere in the format window are delegated to the active format, which may consume them or select another format. Local mouse right-clicks do nothing. The controller can also operate the menu: moving the virtual cursor over the three-dot button and pressing A/right-click opens it, Start toggles it open or closed, B closes it, D-pad left/right moves the item selection, and A/right-click activates the selected item. These controller commands are separate from local mouse button handling. The item highlight is a transient command selection, not a current-format indicator. The `format-chooser` format is a normal format that lists the other formats in a 4x4 grid of 64x64 CRT-green icon-and-title tiles, with a reserved bottom strip for future paging controls; it excludes itself from its choices. The first format is selected by default. D-pad navigation wraps spatially among occupied tiles in the current row or column, and A/right-click activates the selected tile. The display library routes high-level commands to the menu first (when not in the chooser) and then to the active format. It draws one crosshair cursor over the format region, following the last local mouse or controller cursor input. The display application hides the native mouse cursor over the format region and shows it again over the console. Formats other than the chooser ignore menu-specific pointer and navigation input, and the display library no longer generates `Next`; the menu and chooser are the only ways to choose a format. Selecting another menu item while an overlay is open cancels the current overlay first. In the color overlay, R3 affects the hue ring or triangle only when the cursor is over that control; in the light-position overlay R3 does nothing and the right stick always moves the light. Back or B cancels, Done commits, and the World menu item cancels.

### Lux Color overlay

`Lux Color` opens an opaque black overlay in the format window. The Earth and greeting are hidden while it is open. The overlay layout is a hue ring around a saturation/value triangle, an output-color preview rectangle, and then `Back` immediately left of `Done`. The hue marker sits in the ring; the saturation/value marker sits in the triangle; the preview shows the current output RGB.

Mouse picking is left-button only: press/drag on the ring sets hue; press/drag on the triangle sets saturation/value. Local right-click remains a no-op. The view service owns RGB and HSV; opening the overlay snapshots both, `Done` commits the current color, and `Back`, `B`, or the `World` menu item cancels and restores the snapshot. Selecting another menu item cancels/restores Lux Color first. No color or overlay state is persisted.

Lux Color opens with the hue wheel focused and the controller cursor captured. Local mouse motion or a local mouse press releases capture. Controller focus order is `HueRing -> Triangle -> Back -> Done`, reverse in the opposite direction, and D-pad focus changes retain capture. While focus is Back/Done the right stick does nothing and capture remains active; A activates the focused button. Cursor+A activates a button under the cursor, otherwise activates focused Back/Done, otherwise picks/captures the ring or triangle under the cursor, otherwise captures the focused ring/triangle. While captured, the virtual cursor is hidden. B releases capture when the menu is closed; with capture released, B cancels/restores the overlay. Start releases capture and opens the menu. R3 switches ring/triangle focus while captured and does not toggle arcball inside the overlay.

The wheel uses latched direction: the first non-rest stick movement chooses clockwise/counterclockwise from the sign of `dx - dy`; direction is kept through circular stick motion; speed comes from stick magnitude; a zero `CursorMove` emitted by Paws when the stick returns to rest resets the latch. The triangle control moves its cursor in screen x-y, clamps it inside the triangle, and derives saturation/value from the barycentric position.

### Lux Pos overlay

`Lux Pos` opens a black overlay with the Earth frozen at the attitude shown when the overlay opened, a live light preview, and an iconic 2D spotlight-can marker pointing toward the globe center. The globe is rendered in a square area above the Back/Done row; Back and Done use the same positions as Lux Color, with Back left of Done and default D-pad focus on Back. A/right-click or a mouse left-click activates the focused button.

Light is a view-space direction constrained to the user-facing hemisphere (`z >= 0`). Mouse left-drag on the globe maps the pointer to a front-hemisphere direction; the controller right stick moves the light rate-based in screen x-y while the virtual crosshair stays at its last position. Light control is always active, there is no capture mode, and R3 does nothing in this overlay. The spotlight can is placed at the projected light direction, rotates to point toward the globe center, and uses the current light color for its lens/glow.

The view service snapshots the light and effective attitude when Lux Pos opens, freezes the attitude for the overlay, and publishes the live light. `Done` commits the current light; `Back`, `B`, or the `World` menu item cancels and restores the snapshot. Selecting Lux Color, Formats, Inspect, or Auto cancels/restores Lux Pos first, and selecting Lux Pos while Lux Color is open cancels Lux Color first. No light or overlay state is persisted.

The format-manager view service persists the current per-window assignments for its display id. It loads `format-view-service\<display_id>.toml` at startup; if the file is missing, unreadable, or names an unknown format, the effective assignment is `hello-world` for every format window. It publishes the loaded/default assignments immediately and writes the file after every accepted control assignment. The controller reads the same file once at startup, without writing it, so Select starts from the restored state; the view service is the only writer. The display library treats an unassigned window as `hello-world`.

## Service and display instances
Every service and display process acquires a per-instance Windows named-mutex guard from the `single-instance` library before creating DDS participants, opening a window, or reading or writing runtime state. If the guard is already held, the process prints a diagnostic and exits with a failure status. The guard name is `Local\to-stations-<application>` for a singleton and `Local\to-stations-<application>-<discriminator>` when multiple instances are legitimate. The only discriminator in the first increment is the numeric display id: `format-ctl-service-display-<id>`, `format-view-service-display-<id>`, `hello-view-service-display-<id>`, `hello-ctl-service-display-<id>`, `lil-d-display-<id>`, and `paws-display-<id>`. `hello-service` is a singleton. Argument-only diagnostics such as `--topics` do not start an instance and do not acquire the guard. Multiple `paws` windows are legitimate only when they use different display ids.

## Runtime state
Per-user runtime state is not deploy configuration. It lives under `%LOCALAPPDATA%\to-stations\<application>\<discriminator>.toml`, where the discriminator is the numeric display id for per-display applications. Distinct instances use distinct files, so they do not contend, and the per-instance guards prevent two processes from sharing one discriminator. Files are written atomically so readers never observe a partial update.

Examples:
* `lil-d\<display_id>.toml` - logical monitor name, monitor resolution, monitor-relative window position, and inner window size.
* `format-view-service\<display_id>.toml` - current per-window format assignments for that display.
* `paws\<display_id>.toml` - last selected Xbox-style controller (name, UUID, and gilrs id) or `Virtual`, plus the window position and inner size.
* `dds-echo\window.toml` - monitor-relative window position and inner window size for the singleton dds-echo window.

A `lil-d`, `paws`, or `dds-echo` placement is restored only when the named monitor is present with the same resolution and the window rectangle remains sufficiently visible there. Otherwise it falls back to a validated position on the main monitor, then to the cascade. The resolved state (including the resolution) is rewritten on startup, so stale or legacy state self-heals. Hello-view state (mode, local attitude, overlay state, RGB/HSV color, and light position) is intentionally not persisted.

## Topic contracts
Every service declares the topics it publishes and subscribes to so that the contract of a service is easy to read:

* `contracts/topics.toml` - the topic registry: the short wire/registry name, the scoped IDL type name, QoS, and key fields. The wire name ends in `Topic` (for example `HelloWorldTopic`); the IDL type is its fully-qualified scope (`hello_world::HelloWorldTopic`, the value of `DdsType::type_name()`).
* `contracts/services/<app>.toml` - for each service, the topics it publishes and the topics it subscribes to.
* `contracts/formats.toml` - the ordered format names shared by the format-manager controller, the display library, and the format menu/chooser.

`FormatManagerInputTopic` carries both logical display events and high-level control commands from `paws`; `format-ctl-service` relays control commands on `FormatManagerCommandCtlTopic`. The `paws` service contract declares the input topic it publishes. The hello_world constellation adds `HelloWorldInputTopic` from display libraries to the per-display `hello-ctl-service`, `HelloWorldCtlTopic` from that controller to the per-display `hello-view-service`, and a display-scoped `HelloWorldViewTopic` that carries model data, mode/arcball, light, RGB color, HSV color, and overlay state.

Contracts are validated when a service starts, each service can print its own contract with `--topics`, and `scripts/build.ps1` regenerates a readable summary into `build/topics.md`.

The DDS layer lives in `stations-dds`: it creates the domain participant and typed readers and writers. CycloneDDS applies the shared configuration (`DDS/cyclonedds-config.xml`) through the `CYCLONEDDS_URI` environment variable; `scripts/build.ps1` deploys the configuration to `build/deploy/config/`. The `dds-contracts` tool (`rust/tool/dds-contracts`) validates the contracts and writes `build/topics.md`, including which services use each topic. Endpoint creation is contract-aware: a service creates its writers and readers through the validated registry, so an unregistered topic or a type mismatch fails at startup. A completeness check keeps the registry in step with the IDL. Because topic names enter the system only through that API, the `dds-contracts` scanner (a repo-agnostic check whose roots the caller supplies) fails the build when the source of the crate being built calls the raw CycloneDDS topic-naming API directly (a bypass); the Node implementation is the only place those calls are allowed, and it is never in the scan's scope.

There is deliberately no scope field in a contract: wiring is expressed by the IDL type's fields and by command-line arguments, and decided by each format's implementation.

## Naming conventions
* **Topics.** A topic's wire topic name is the short `<Format>Topic` (for example `HelloWorldTopic`); its IDL/DDS type name is the scoped `format::<Format>Topic` (the fully-qualified IDL scope, what `DdsType::type_name()` returns). The topic registry records both. The format's model payload is `<Format>Topic`, the view payload `<Format>ViewTopic`, and the control payload `<Format>CtlTopic`.
* **Value types.** Embedded structs that are not topics are bare nouns (for example `Quaternion`).
* **Services.** Service executables (and their crate names) end in `Service`: `<Format>Service`, `<Format>ViewService`, `<Format>CtlService`; generic control services (for example the joystick controller service) follow the same rule.
* **`Model` is not a name.** `Model` is never used as a type, service, or code name; `Model`, `View`, and `Controller` appear only as role descriptions (see "Formats and the distributed MVC constellation").
* **IDL modules are namespaces.** An IDL `module` maps to a Rust `pub mod`; the DDS type name is the fully-qualified IDL scope. Modules are top-level; there is no vendor root namespace.

## Files and folders
* **Authored files** (written by people or agents) live in `planning/`, `docs/`, `DDS/`, `contracts/`, `include/`, `scripts/`, `assets/`, `rust/`, and `cpp/`.
* **Build artifacts** live only under `build/`, which is never committed. `build/deploy/` is the only durable output; everything else under `build/` is temporary and freely deletable:
  * `build/deploy/` - the deployable layout: binaries, libraries, assets, and DDS configuration.
  * `build/gen/{rust,cpp}/` - IDL-generated sources.
  * `build/rust/` - the Cargo target directory.
  * `build/cpp/` - the CMake binary directory.
  * `build/topics.md` - the generated contract summary.
* `DDS/` holds the IDL types and the Cyclone DDS configuration shared by all applications; it is authored, not generated.
* `include/` holds shared C ABI headers (for example `display_lib.h`) implemented by both the Rust and C++ sides.

## IDL organization
* `DDS/` holds authored IDL as module-sized files, one top-level IDL `module` per file (`math.idl` holds `module math`; `hello_world.idl` holds `module hello_world`). A module is not reopened across files: all of a module's types live in its one file, so a format's model/view/control structs share the format's file.
* A single root IDL is compiled per build and its `#include`s are inlined into one generated Rust file (the `cyclonedds-build` model, matching HDDS and `dust_dds_gen`). Each file is included exactly once - no diamond includes - because the generator neither merges reopened modules nor deduplicates includes.
* Generated code is written to `build/gen/rust` (never committed) and pulled into `stations-dds` with `include!`. The generated file is located through the `REPO_ROOT` environment variable (the absolute repository root) rather than a `..`-relative path; Cargo supplies it from `.cargo/config.toml` (`[env] REPO_ROOT = { value = ".", relative = true }`).

## Folder structure
```
to-stations-proto/
  planning/            authored: architecture.md, backlog.md, completed_stories.md,
                       current_story.md, cyclonedds-build-plan.md
  docs/                authored: reference documentation
  DDS/                 authored: math.idl, hello_world.idl,
                       input.idl, control.idl, format_manager.idl,
                       cyclonedds-config.xml
  contracts/           authored: topics.toml, formats.toml, services/<app>.toml
  include/             authored: display_lib.h (shared C ABI)
  scripts/             authored: build.ps1, verify-loopback.ps1, cleanup.ps1,
                       launch-rust.ps1, launch-cpp.ps1, launch-hybrid.ps1,
                       subst-repo.ps1, install-subst-startup.ps1
  assets/              authored: earth cube faces, fonts
  .cargo/              authored: config.toml (Cargo target dir, REPO_ROOT)
  rust/                authored: Cargo workspace
    ui/                big-d, lil-d, tentacles, paws
    service/           hello-service, hello-view-service, hello-ctl-service,
                       format-ctl-service, format-view-service
    lib/               stations-dds, display-lib, format-list, window-position,
                       and other shared libraries
    tool/              dds-echo (Slint GUI), dds-contracts, equirect-to-cubemap
    test/              (deferred)
  cpp/                 authored: CMake project
    libs/              stations-dds, display-lib, support
    third_party/       glad, Dear ImGui, TOML parser, image decoder
    tools/             dds-echo, dds-contracts
    apps/displays/     big-d, lil-d
    apps/launcher/     tentacles
    apps/controls/     joystick-control-service, paws
    apps/formats/      hello-world/{hello-service, hello-view-service, hello-ctl-service}
    apps/format-manager/
  build/               gitignored: temporary artifacts plus one durable output
    deploy/            bin/ (Rust), cpp/bin/ (C++), lib/, assets/, config/
    gen/{rust,cpp}/    IDL-generated sources (temporary)
    rust/              Cargo target directory (temporary)
    cpp/               CMake binary directory (temporary)
    topics.md          generated contract summary (temporary)
```

### Rust crates
* A crate defines a single main library and/or executable; any additional executables are tests.
* `ui/` holds GUI applications, `service/` holds headless services, `lib/` holds shared libraries, `tool/` holds developer tools; `test/` is deferred.
* The display applications use egui; GUI tools and launchers (`dds-echo`, `tentacles`, `paws`) use Slint. A tool GUI may live under `tool/` (for example `dds-echo`).
* A `-service` crate holds a format's model (non-display business logic); `-view-service` and `-ctl-service` crates are services tied to the display library.
* Common DDS support lives in `stations-dds`; other common support lives in one or more `lib/` crates. `window-position` holds the shared window position and inner-size persistence used by `lil-d`, `paws`, and `dds-echo`.
* `harfbuzz` (at `rust/lib/harfbuzz`) is an FFI binding to the installed HarfBuzz, including the experimental `libharfbuzz-gpu`; the display library uses it for format text.
* Rust programs use `clap` for command-line parsing.
* `stations-dds` lives at `rust/lib/stations-dds`.

## Build and launch
`scripts/build.ps1` is the single build orchestrator. It is location-agnostic: it resolves the repository root from its own location and uses repository-relative paths. It generates code from `DDS/` into `build/gen/{rust,cpp}`, builds Rust (target directory `build/rust`), builds C++ with the CMake binary directory `build/cpp` and install prefix `build/deploy/cpp`, assembles `build/deploy`, and regenerates `build/topics.md`. It does not clean the Cargo target directory, so dependencies are rebuilt only when `build/` is removed. The Rust target directory and `REPO_ROOT` are set in `.cargo/config.toml` at the repository root; running Cargo from the repository root finds it.

It locates the externally installed Cyclone DDS and SDL3 prefixes under the external prefix (`EXT_PATH_PREFIX`, default `C:\Libraries`) and provisions the runtime without changing the user `PATH`: a process-scoped `PATH` during build and test, with runtime DLLs copied into the configuration's bin directory (`build/deploy/bin` for Rust, `build/deploy/cpp/bin` for C++). The repository must be placed at a short path on Windows; see "Path length" below.

DDS Rust types are generated from `DDS/` by the to-stations fork of `cyclonedds-rust`'s `cyclonedds-idlc` CLI (which wraps `cyclonedds-build`), consumed as a pinned git dependency and built from the pinned rev by `scripts/build.ps1`. The CLI is run with `--no-idlc`; the C `idlc` compiler emits C/C++, not Rust. The fork adds nested modules, `#include`/`import`, scoped type references, and `#[dds_typename]` so Rust `DdsType::type_name()` matches the C `idlc`/C++ name. IDL uses top-level modules and a single compiled root; see "IDL organization". The fork spec is `planning/cyclonedds-build-plan.md`; the pinned rev is recorded in `docs/versions.md`. C++ types are generated from the same IDL with the installed `idlc` C++ backend (`-l cxx`) into `build/gen/cpp`; the C++ `stations-dds` library links the installed CycloneDDS-CXX CMake package.

### Repository root
The repository root is the directory containing `.git`. Each build system resolves it once from its own root file, using that system's built-in base, rather than a shared environment variable set by a script:

* Cargo: `.cargo/config.toml` at the repository root (`[env] REPO_ROOT = { value = ".", relative = true }`, resolved against the parent of the `.cargo` directory).
* MSBuild: `Directory.Build.props` at the repository root (`$(MSBuildThisFileDirectory)`).
* CMake: the top-level `CMakeLists.txt` (`get_filename_component(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}" DIRECTORY)`).
* PowerShell: `$PSScriptRoot`.

Paths never use `..`.

### Path length
Windows limits file paths to 260 characters unless long-path support is enabled. The `cyclonedds` Rust crate is not only a binding: `cyclonedds-rust-sys` compiles a copy of CycloneDDS with CMake as part of `cargo build`, by default under the Cargo target directory (`$OUT_DIR/cyclonedds-build/<source>/...`), and it also CMake-builds a small ABI probe under `$OUT_DIR`. Those nested paths exceed 260 characters unless the repository itself sits at a short path; when they do, MSBuild's `GetOutOfDateItems` task fails the build.

The repository is therefore required to live at a short path on Windows. `scripts/subst-repo.ps1` maps the repository root to a short drive letter (`T:` by default), and `scripts/install-subst-startup.ps1` installs that mapping to run at logon via a wrapper in the Startup folder; alternatively the repository can simply be cloned to a short real path such as `C:\t\to-stations-proto`. These scripts are separate from the build and are never run by `scripts/build.ps1`, which is location-agnostic (it resolves the repository root from its own location, uses repository-relative paths, and hardcodes no drive letter). The Rust binding's CycloneDDS build is additionally redirected to the external prefix (`CYCLONEDDS_SRC`, `CYCLONEDDS_BUILD`) so it is built static there rather than inside the target directory. The requirement is restated in `README.md`.

Each configuration has a launch script; all configurations share `cleanup.ps1`. `scripts/verify-loopback.ps1` verifies the loopback adapter and its IPv6 prerequisites; creating the adapter is a documented manual step (see `docs/`). Launch and cleanup scripts operate only on `build/deploy/`. `cleanup.ps1` asks the displays to close first so they can persist their per-display state, then force-stops any remaining processes.

### Single-instance guards
The shared `single-instance` library provides the named-mutex guard and a best-effort window-activation helper. `dds-echo` uses the guard as a manually launched singleton: a second launch finds the existing window by its stable title, restores and foregrounds it, then exits without creating DDS participants, logging a second startup, or creating another window. It persists its window placement in `dds-echo\window.toml`. Fleet-launched services and displays use the same guard only to reject a duplicate instance with the same identity; they exit with an error instead of foregrounding another window. The guard name includes the identity described in "Service and display instances". Argument parsing happens before the guard, so `--help` and `--version` still work. Window titles are the activation key for the GUI tools that foreground an existing window, so those tools must keep a stable title.

## Deferred decisions
* displayLib internal architecture: its top-level modules are `formats` (one submodule per format, including `format-chooser`), the rendering capabilities `text`, `earth`, `icon`, and `shader`, the shared 2D UI helpers `ui`, and `format_manager`, `format_menu`, `color_picker`, and `light_pos`; `mesh` is added later. `shader` holds the shader compile/link helpers shared by `text` and `earth`; `mesh` is extracted when a second mesh user appears. `icon` loads the per-format 64x64 PNGs into textures and draws them; it is used by the chooser. `ui` holds the rectangle, scissor-clear, and scale helpers shared by the menu and chooser. `format_manager` owns the format renderers and applies assignments from the format-manager view service; it hit-tests forwarded pointer input against the format windows it last rendered, lets the active format handle local and high-level command input, draws the virtual cursor, and creates logical `Select` events (it does not generate `Next`). It also hides the menu while the chooser is active. `format_menu` owns the three-dot button, its open/closed state, the horizontal top-band item bar, and the overlay rendering over the active format. `color_picker` owns the Lux Color hue ring, saturation/value triangle, output preview, Back/Done hit testing, focus/capture state, and picker rendering. `light_pos` owns the Lux Pos globe-area/button layout, front-hemisphere light mapping, spotlight marker, and spotlight-can rendering. The button and item bar share the top band; the button is hidden while the menu is open. `Formats` is always first; the active format contributes format-specific items and labels, and there is no current-format highlight. The `format-chooser` format lists the other formats in a 4x4 grid (16 per page) of icon-and-title tiles and selects one through its format input; it excludes itself, reserves a bottom pager strip for future paging controls, highlights a selected tile, and moves that selection with D-pad commands. The controller owns the distributed format-window assignments. A format's unique details (colors, geometry, strings, its icon file, its mesh and shaders) live in its own submodule; shared code is extracted only when a second format needs it. The default format is retained at `formats/default.rs` and reactivated by assignment. It may still split into smaller libraries. The format manager has a view service, a control service, and one or more displayLib modules. The eyebrow is separate and is not a format window.
* `test/` layout.
* C++ tools and test layout.
