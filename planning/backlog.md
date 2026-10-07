# Backlog - increment 1

Stories are individual deliverables. A group is marked as an epic only when it is broken into multiple stories. See `current_increment.md` for the locked scope and `architecture.md` for the design.

## Epic: build and deploy pipeline

### Story: repository structure and ignore rules
Create the top-level folders (`DDS/`, `contracts/`, `rust/`, `cpp/`, `scripts/`, `assets/`, `docs/`, `build/`) and a `.gitignore` that excludes `build/`.

### Story: build orchestrator
Write `scripts/build.ps1` that runs the four build steps in order (IDL codegen into `build/scratch/gen/{rust,cpp}`, Rust build with `CARGO_TARGET_DIR=build/scratch/rust`, CMake build from `build/scratch/cpp` with install prefix `build/deploy`, assembly of `build/deploy`), and regenerates `build/scratch/topics.md`.

### Story: IDL code generation into both build systems
Wire Cyclone DDS IDL code generation into the Cargo build and the CMake build so generated sources land in `build/scratch/gen/` and are compiled into the `stations-dds` libraries. Generated files are never committed.

## Epic: DDS layer and topic contracts

### Story: topic registry and service contracts
Author `contracts/topics.toml` (topic name, IDL type, QoS, key fields) and `contracts/services/<app>.toml` for every application, covering the topics listed below:

| Topic | Type | Published by | Subscribed by | Key fields |
|---|---|---|---|---|
| `stations/hello_world/model` | `HelloWorldModel` | hello_world model service | hello_world view service | none (global) |
| `stations/hello_world/view` | `ViewData` | hello_world view service | display library | none (global) |
| `stations/hello_world/input` | `InputEvent` | display library | hello_world controller service | none (global) |
| `stations/hello_world/control` | `ControlEvent` | joystick control service | hello_world controller service | `station` |
| `stations/joystick/virtual` | `XboxController` | controller window | joystick control service | `station` |
| `stations/joystick/state` | `XboxController` | joystick control service | observers, `dds-echo` | `station` |
| `stations/format/window` | `FormatWindow` | display application | format manager | `station`, `display` |
| `stations/format/mapping` | `FormatMapping` | format manager | display library | `station`, `display` |

### Story: stations-dds library
Implement the shared DDS wrapper in both languages: participant creation from `DDS/cyclonedds-config.xml`, typed readers and writers, validation of each application's contract at startup from `contracts/`, and a `--topics` flag that prints the application's published and subscribed topics.

### Story: dds-contracts tool
A tool that reads `contracts/` and writes a readable summary to `build/scratch/topics.md`, invoked by `scripts/build.ps1`.

### Story: dds-echo tool
A tool that subscribes to any named topic and prints received samples, used to verify every topic during testing.

### Story: IDL types
Author `DDS/hello_world.idl` (`Quaternion`, `HelloWorldModel`), `DDS/mvc.idl` (`ViewData`, `InputEvent`, `ControlEvent`, `FormatWindow`, `FormatMapping`, and the display element type), and `DDS/controller.idl` (`XboxController`, the default Xbox layout).

## Epic: display infrastructure

### Story: display library
A library hosted by each display application that creates the window and OpenGL context, runs the render loop, renders the console (Dear ImGui / egui), renders the `ViewData` it receives in a format window, and publishes `InputEvent` for mouse, touchscreen, and key input over that window. Applications delegate all window rendering to it.

### Story: font handling
Load a font from `assets/fonts` in both languages and render text (the hello_world greeting) inside the display library, so every format can draw text.

### Story: small display application
The small display: one square format window surrounded by a console area with a single power button, a `--station N` command-line argument, and `--topics` support. The power button toggles the format window and emits an input event for later wiring.

## Epic: hello_world format constellation

### Story: model service
Publishes `HelloWorldModel` (greeting text and quaternion attitude) at a fixed rate. The attitude is a 23.5 degree tilt with one revolution every twelve minutes.

### Story: view service
Subscribes to `HelloWorldModel` and publishes the `ViewData` display message: the earth cube map and sphere parameters, the model quaternion, the upper-right light direction, and the greeting text with its font.

### Story: controller service
Subscribes to `stations/hello_world/input` and `stations/hello_world/control`, tracks the current input and control state for its format, and is easy to observe (console output in this increment).

### Story: earth format rendering
Render the cube map on a sphere in the display library, orient it with the attitude quaternion from the `ViewData`, and light it from the upper right.

## Epic: controls

### Story: joystick control service
A service per station that reads the virtual controller input, publishes the current `XboxController` values on `stations/joystick/state`, and forwards control events to the format's controller service. Runs headless or with a console window. No hardware binding in this increment.

### Story: controller window
A window with axis sliders and buttons for the default Xbox-controller layout, publishing `stations/joystick/virtual`. Binding a physical game controller is a later increment.

## Stories outside the epics

### Story: format manager
One service per display: reads the format windows that display publishes and publishes the format mapping that assigns hello_world to the small display's format window for its stations.

### Story: stationsdds setup
`scripts/stationsdds.ps1` creates or verifies the Microsoft loopback adapter named `StationsDDS` and enables IPv6 multicast on it. Requires administrator rights; document the manual fallback in `docs/`.

### Story: launch and cleanup scripts
`launch-rust.ps1`, `launch-cpp.ps1`, and `launch-hybrid.ps1` start every display and service of that configuration from `build/deploy`; `cleanup.ps1` stops all processes of all configurations.

### Story: verification
Manual end-to-end run of each configuration plus cross-language checks (a Rust publisher seen by a C++ subscriber and the reverse) for every topic in the registry.

## Done when

Each configuration's launch script brings up the hello_world constellation, the small displays for stations 0 and 1, the joystick service and its window, and a format manager per display. The small display shows the tilted, rotating, cube-mapped earth lit from the upper right with the greeting text rendered in the loaded font, and its power button works. Display input events and joystick control events reach the hello_world controller service, `--topics` and `build/scratch/topics.md` match `contracts/`, `stations/joystick/state` shows the slider values, and `cleanup.ps1` stops everything.
