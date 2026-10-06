First, we need a folder structure and a set of backlog items that implement the basic architecture for the small display, the controller service, and a hello_world service displayed on the hello_world format.

The hello_world service provides a greeting text and a quaternion attitude. The quaternion attitude represents the orientation of a virtual world that is tilted 23.5 degrees and which rotates about its axis once every twelve minutes.

The hellow_world format displays a box projection of the earth onto a sphere, oriented per the attitude from the hello_world service.

The sphere is lit from the upper right.

The small display initially has only one 'power' button in the console area.

The controller service initially sends DDS messages reprsenting values of a default xbox-controller layout. It cannot initially connect to a controller, but the service's window can manipulate the values of the controls via axis sliders and buttons.

---

# Locked scope for increment 1

Every decision below is fixed for this increment. The story breakdown is in `backlog.md`.

## Stack
* DDS: Eclipse Cyclone DDS for all three configurations - C core with `cyclonedds-cxx` (C++) and the `cyclonedds` Rust crate; shared IDL in `DDS/`; all topics over IPv6 multicast on the `StationsDDS` loopback adapter.
* Rendering: C++ = SDL3 + glad + Dear ImGui; Rust = winit + glutin/glow + egui.
* Languages: `rust`, `c++`, and `hybrid` configurations are all delivered in this increment.
* Build: `scripts/build.ps1` orchestrator; all artifacts under `build/` split into `scratch/` and `deploy/`; launch and cleanup scripts operate on `build/deploy/`.

## hello_world constellation
* The model service and the view service are **separate services**: the model service publishes the greeting and the attitude; the view service turns that into the display message the display library renders.
* Attitude: a virtual world tilted 23.5 degrees and rotating about its axis once every twelve minutes, published as a quaternion.
* The format is global (system-wide) in this increment; per-station view selection is a later increment.
* Earth projection: a cube map of six faces sampled on a sphere, oriented by the attitude quaternion, lit from the upper right.
* Display library input events and joystick control events both end at the hello_world controller service.

## Displays and controls
* Small display: one square format window plus a console area containing a single power button; `--station N` on the command line.
* Format manager: a single service per display, mapping hello_world to that display's format window.
* Controller service: publishes a default Xbox-controller layout and forwards control events to the format's controller service; no hardware binding. Its window manipulates the values with axis sliders and buttons.
* Font handling (loading and rendering the greeting text) is part of this increment.

## Out of scope
* Large display content, launcher start/stop logic, real game-controller binding, and per-station views.

