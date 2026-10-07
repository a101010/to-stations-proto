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

Display applications take a parameter on the command line that indicates their 'station number'.

Each configuration launches a large and a small display for stations 0 and 1.

Each station has a service that represents a joystick controller. For each of these services, a separate window application with a virtual controller will be launched. This window can also be bound to one game controller.

Each configuration launches a group of service applications that can run either headless or with a displayed console window.

It also launches a window that can start and stop the applications for that configuration.

All the applications communicate using Distributed Data Services configured to use IPv6 multicast on a Microsoft loopback adapter named StationsDDS.

## Configurations
The three configurations (`rust`, `c++`, `hybrid`) differ only in which language implementation of each application is launched. Applications interoperate because they share the same IDL types and speak DDSI-RTPS; a hybrid launch simply mixes the two implementations.

## Formats and the distributed MVC constellation
A *format* is a distributable model-view-controller constellation. Each format owns the following services:

* **Model service** - owns or derives the data for the format.
* **View service** - sends all data to be displayed to the display library of each interested display via a DDS message.
* **Display library** (the view) - renders the data it receives; sends mouse, touchscreen, and other input events to the controller service via DDS.
* **Controller service** - consumes input events from the display libraries and control events from control services.
* **Control services** - interface to game controls (for example, a joystick controller service) and send events to the display control services.

The display applications themselves know very little; most of their work is delegated to the display library instance each of them hosts.

Each format has its own instance of this constellation, so formats are independent and can be distributed across the system. An editor format, for example, could display different data on different displays for the same station by using station- or display-scoped view services.

### Scope: system, station, or display
View and controller elements of a constellation can be system-wide, per station, or per display, depending on the requirements of that format.

* All topics travel over IPv6 multicast, so every topic is reachable everywhere on the network.
* A topic that varies carries the relevant `station` and/or `display` field in its IDL type; a system-wide topic carries neither. Fields are added to a type only when a format requires them.
* A view or controller service that serves one station or display receives that id as a command-line argument and publishes the corresponding field.
* A display library receives its station (and display) id from its display application's command line; the format's own implementation decides which instances of a topic it listens to. That choice is part of the format's implementation, not separate run-time configuration.

The hello_world format is global (system-wide) in the first increment; the ability to select the view per station comes in a later increment.

## Format windows and the format manager
A *format window* is a place where a format can be displayed: one square format window on the small display, three square format windows and an eyebrow window on the large display.

The format manager is **per display** (not per station) and is itself a model-view-controller constellation. It tells that display's display library which format to display in each format window, and that combination is different for each display. It runs as a separate service process, not inside the display application.

## Topic contracts
Every service declares the topics it publishes and subscribes to so that the contract of a service is easy to read:

* `contracts/topics.toml` - the topic registry: topic name, IDL type, QoS, and key fields.
* `contracts/services/<app>.toml` - for each service, the topics it publishes and the topics it subscribes to.

Contracts are validated when a service starts, each service can print its own contract with `--topics`, and `scripts/build.ps1` regenerates a readable summary into `build/topics.md`.

There is deliberately no scope field in a contract: wiring is expressed by the IDL type's fields and by command-line arguments, and decided by each format's implementation.

## Files and folders
* **Authored files** (written by people or agents) live in `planning/`, `docs/`, `DDS/`, `contracts/`, `scripts/`, `assets/`, `rust/`, and `cpp/`.
* **Build artifacts** live only under `build/`, which is never committed. `build/deploy/` is the only durable output; everything else under `build/` is temporary and freely deletable:
  * `build/deploy/` - the deployable layout: binaries, libraries, assets, and DDS configuration.
  * `build/gen/{rust,cpp}/` - IDL-generated sources.
  * `build/rust/` - the Cargo target directory.
  * `build/cpp/` - the CMake binary directory.
  * `build/topics.md` - the generated contract summary.
* `DDS/` holds the IDL types and the Cyclone DDS configuration shared by all applications; it is authored, not generated.

## Folder structure
```
to-stations-proto/
  planning/            authored: architecture.md, current_increment.md, backlog.md
  docs/                authored: reference documentation
  DDS/                 authored: hello_world.idl, mvc.idl, controller.idl,
                       cyclonedds-config.xml
  contracts/           authored: topics.toml, services/<app>.toml
  scripts/             authored: build.ps1, verify-loopback.ps1, cleanup.ps1,
                       launch-rust.ps1, launch-cpp.ps1, launch-hybrid.ps1,
                       subst-repo.ps1, install-subst-startup.ps1
  assets/              authored: earth cube faces, fonts
  rust/                authored: Cargo workspace
    crates/            stations-dds, display-lib, format-manager,
                       tools/{dds-echo, dds-contracts}
    displays/          small-display, large-display, launcher
    controls/          joystick-control-service, controller-window
    formats/           hello-world/{model-service, view-service, controller-service}
  cpp/                 authored: CMake project
    libs/              stations-dds, display-lib
    tools/             dds-echo, dds-contracts
    apps/displays/     small-display, large-display, launcher
    apps/controls/     joystick-control-service, controller-window
    apps/formats/      hello-world/{model-service, view-service, controller-service}
    apps/format-manager/
  build/               gitignored: temporary artifacts plus one durable output
    deploy/            bin/, lib/, assets/, config/
    gen/{rust,cpp}/    IDL-generated sources (temporary)
    rust/              Cargo target directory (temporary)
    cpp/               CMake binary directory (temporary)
    topics.md          generated contract summary (temporary)
```

## Build and launch
`scripts/build.ps1` is the single build orchestrator. It is location-agnostic: it resolves the repository root from its own location and uses repository-relative paths. It generates code from `DDS/` into `build/gen/{rust,cpp}`, builds Rust with `CARGO_TARGET_DIR=build/rust`, builds C++ with the CMake binary directory `build/cpp` and install prefix `build/deploy`, assembles `build/deploy`, and regenerates `build/topics.md`.

It locates the externally installed Cyclone DDS and SDL3 prefixes (hardcoded under a short path such as `C:\Libraries`) and provisions the runtime without changing the user `PATH`: a process-scoped `PATH` during build and test, with runtime DLLs copied into `build/deploy/bin`. The repository must be placed at a short path on Windows; see "Path length" below.

### Path length
Windows limits file paths to 260 characters unless long-path support is enabled. The `cyclonedds` Rust crate is not only a binding: `cyclonedds-rust-sys` compiles a copy of CycloneDDS with CMake as part of `cargo build`, by default under the Cargo target directory (`$OUT_DIR/cyclonedds-build/<source>/...`), and it also CMake-builds a small ABI probe under `$OUT_DIR`. Those nested paths exceed 260 characters unless the repository itself sits at a short path; when they do, MSBuild's `GetOutOfDateItems` task fails the build.

The repository is therefore required to live at a short path on Windows. `scripts/subst-repo.ps1` maps the repository root to a short drive letter (`T:` by default), and `scripts/install-subst-startup.ps1` installs that mapping to run at logon via a wrapper in the Startup folder; alternatively the repository can simply be cloned to a short real path such as `C:\t\to-stations-proto`. These scripts are separate from the build and are never run by `scripts/build.ps1`, which is location-agnostic (it resolves the repository root from its own location, uses repository-relative paths, and hardcodes no drive letter). The Rust binding's CycloneDDS build is additionally redirected to the external prefix (`CYCLONEDDS_SRC`, `CYCLONEDDS_BUILD`) so it is built static there rather than inside the target directory. The requirement is restated in `README.md`.

Each configuration has a launch script; all configurations share `cleanup.ps1`. `scripts/verify-loopback.ps1` verifies the loopback adapter and its IPv6 prerequisites; creating the adapter is a documented manual step (see `docs/`). Launch and cleanup scripts operate only on `build/deploy/`.


