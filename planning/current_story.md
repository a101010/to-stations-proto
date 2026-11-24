# Current story: cpp-hello-service

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create C++ `hello-service`: sequential greeting list, attitude model, and 10 Hz `HelloWorldTopic`. Replace the Rust model service in `launch-cpp.ps1`.

- **Depends on:** none.
- **Minimal test:** Rust `dds-echo` shows C++ `HelloWorldTopic` samples with changing greetings and attitude.

## Context

- `cpp-format-services` is complete at `2026-11-24 09:33:18`.
- The C++ configuration currently runs C++ `format-ctl-service`, `format-view-service`, and `lil-d` with Rust `hello-service`, `hello-view-service`, `hello-ctl-service`, and `paws` stand-ins.
- `launch-rust.ps1` must remain unchanged.
- The Rust model service is the behavior reference:
  - `HelloWorldTopic` at 10 Hz by default;
  - a fixed 17-entry greeting list, advancing sequentially at random 15–60 second intervals;
  - the earth attitude model with a 55-degree pole view angle, 45-degree screen tilt, `-100`-degree initial longitude, and a 12-minute spin.
- `contracts/services/hello-service.toml`, `contracts/topics.toml`, and `DDS/hello_world.idl` already define the service and topic.
- C++ generated DDS types are available from `hello_world_types`.
- C++ service conventions are established by the format services: deploy-relative config, contract validation, `support::SingleInstance`, `stations-dds::Node`, and `--topics`.

## Decisions

1. **Keep model logic app-local for now.**
   - Put greeting and attitude code under the C++ `hello-service` app.
   - Do not create a shared C++ hello-model library until the hello view/control services need shared code.
2. **Port the Rust model behavior exactly.**
   - Same greeting strings and order.
   - Same random interval and SplitMix64 generator.
   - Same attitude constants, quaternion math, and timing.
3. **Implement only the global model service in this story.**
   - C++ `hello-view-service`, C++ `hello-ctl-service`, and C++ `paws` are out of scope.
   - `launch-cpp.ps1` keeps Rust hello view/control and `paws`.
4. **Test the model logic without DDS.**
   - A dedicated `hello-service-tests` executable compiles the greeting and attitude modules.
   - `hello-service-topics` checks the service contract.
5. **No IDL, contract, state, or version changes.**
   - No persistence is added.
   - No `docs/versions.md` change.

## Scope

**In scope**

- C++ `hello-service`.
- C++ greeting and attitude modules.
- C++ model tests.
- CMake target and deploy copy.
- `launch-cpp.ps1` switch to the C++ model service.
- Architecture and planning updates.

**Out of scope**

- C++ hello view/control services.
- C++ `paws`.
- Lux overlays.
- Random greeting order; that remains the separate `random-greeting` story.
- Changes to Rust `hello-service`.
- Changes to `launch-rust.ps1`.

## Files

```text
cpp/apps/formats/
  CMakeLists.txt
  hello-world/
    CMakeLists.txt
    hello-service/
      CMakeLists.txt
      main.cpp
      greeting.hpp
      greeting.cpp
      attitude.hpp
      attitude.cpp
      tests/
        main.cpp
```

Other changes:

```text
cpp/CMakeLists.txt
scripts/build.ps1
scripts/launch-cpp.ps1
planning/architecture.md
planning/backlog.md
planning/completed_stories.md
planning/current_story.md
```

No `docs/versions.md` change is needed.

## Implementation

### 1. Greeting module

Port `rust/service/hello-service/src/greeting.rs`.

- Use exactly these 17 greetings in order:
  `Hello, world!`, `Hola, mundo!`, `Bonjour le monde!`, `Hallo, Welt!`,
  `Ciao, mondo!`, `Olá, mundo!`, `Hallo, wereld!`, `Hej, världen!`,
  `Hei, maailma!`, `Witaj, świecie!`, `Merhaba dünya!`, `Salut, lume!`,
  `Salve, munde!`, `toki, ma o!`, `Halo, wol!`, `Habari, dunia!`,
  `Pẹlẹ o, ayé!`.
- Enable MSVC `/utf-8` for the service and test targets.
- Implement `GreetingCycle`:
  - start at greeting 0;
  - choose the first change from 15–60 seconds inclusive;
  - advance sequentially and wrap;
  - advance multiple times when the current time has overrun;
  - expose `current()`;
  - expose `advance_if_due(now)`;
  - expose the next-change time or an equivalent accessor for tests.
- Seed from wall-clock nanoseconds; provide deterministic `with_seed(now, seed)` for tests.
- Port SplitMix64 exactly:
  - add `0x9e37_79b9_7f4a_7c15`;
  - mix with the same shifts and multipliers;
  - use `uint64_t` wrapping arithmetic.
- Random interval:
  - span is 46 values;
  - result is `15 + next_u64() % 46` seconds.

Tests:

- 100 intervals stay within 15–60 seconds.
- Sequence advances sequentially and wraps through all greetings.
- No early advance at the start time.

### 2. Attitude module

Port `rust/service/hello-service/src/attitude.rs` using generated `math::Quaternion`.

- Constants:
  - `POLE_VIEW_ANGLE_DEG = 55.0`;
  - `POLE_SCREEN_TILT_DEG = 45.0`;
  - `SPIN_PERIOD_S = 12.0 * 60.0`;
  - `INITIAL_LONGITUDE_DEG = -100.0`.
- Use an internal `std::array<double, 3>` vector type.
- Port helpers: dot, cross, scale, subtract, normalize, unit quaternion, quaternion multiply, and vector rotation.
- Implement:
  - `pole_in_view()`;
  - `initial_attitude()`;
  - `attitude_at(std::chrono::duration<double> elapsed)`.
- Use `std::numbers::pi_v<double>` rather than non-standard constants.
- Preserve the Rust multiplication order:
  - `initial_attitude` places the polar axis and then the sub-camera point;
  - `attitude_at` returns `initial_attitude * spin`.

Tests:

- Polar axis is placed by the view angle and screen tilt.
- Initial attitude points the pole and the Americas at the camera.
- Spin preserves the pole and makes equatorial points antipodal after six minutes.
- A full 12-minute revolution returns to the initial orientation.
- Use dot-product tolerances matching the Rust tests.

### 3. Service executable

Create `hello-service` using the established C++ service pattern.

CLI:

- `--domain` default `0`;
- `--rate-ms` default `100`;
- `--config <dir>`;
- `--topics`.

Config resolution:

1. explicit `--config`;
2. `<deploy>/config` through `support::deploy_path` when it contains `topics.toml`;
3. `./contracts` for development.

Startup order:

1. load and validate `topics.toml`;
2. load and validate `services/hello-service.toml`;
3. print the rendered contract and exit for `--topics`, without a guard or DDS;
4. acquire singleton guard `hello-service`;
5. create the `stations-dds::Node` with the deploy `cyclonedds-config.xml`;
6. create the writer for `HelloWorldTopic`.

Publish loop:

- period is `std::chrono::milliseconds(std::max(rate_ms, 1))`;
- record the steady-clock start time;
- construct the greeting cycle from the start time;
- each iteration:
  - read the current steady-clock time;
  - advance the greeting if due;
  - set the greeting;
  - set the attitude from the elapsed time since start;
  - write the sample;
  - sleep for the period.

Errors print to stderr as `hello-service: <message>` and exit non-zero.

### 4. CMake, deploy, and launch

- Add `apps/formats` to `cpp/CMakeLists.txt`.
- Add `apps/formats/hello-world` and its `hello-service` subdirectory.
- Add `hello-service` executable linking:
  - `support`;
  - `stations-dds`;
  - `hello_world_types`.
- Add `hello-service-tests` with:
  - `greeting.cpp`;
  - `attitude.cpp`;
  - `tests/main.cpp`;
  - link `hello_world_types`.
- Add CTest:
  - `hello-service-tests`;
  - `hello-service-topics` with `--topics --config "${REPO_ROOT}/contracts"`.
- Add `/utf-8` compile options for the service and tests on MSVC.
- Update `scripts/build.ps1` to copy `build\cpp\Release\hello-service.exe` to `build/deploy/cpp/bin`.
- Update `scripts/launch-cpp.ps1`:
  - `$model` resolves to `$cppBin\hello-service.exe`;
  - description says C++ hello-service, format services, and displays run with Rust hello view/control and `paws` stand-ins.
- No `cleanup.ps1` change; the image name is unchanged.

### 5. Architecture

Update the C++ runtime paragraph:

- The C++ launch script starts the C++ `hello-service`, C++ format services, and C++ displays.
- Rust `hello-view-service`, `hello-ctl-service`, and `paws` remain stand-ins.

The folder tree already lists the planned C++ hello format directory.

## Tests

- `hello-service-tests`:
  - greeting interval range;
  - sequential advance and wrap;
  - no early advance;
  - all four attitude tests.
- `hello-service-topics`.
- Existing CTest suites stay green.
- Manual integration:
  - `launch-cpp.ps1` starts the C++ model service from `cpp/bin`;
  - no Rust `hello-service.exe` remains running;
  - Rust hello view/control and `paws` still run;
  - `dds-echo` shows `HelloWorldTopic` with changing greeting and attitude;
  - `launch-rust.ps1` still uses the Rust model service.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
```

Expected CTest result: 9/9 passing (the existing 7 plus `hello-service-tests` and `hello-service-topics`).

Manual verification with a temporary `LOCALAPPDATA`:

1. Run `.\scripts\launch-cpp.ps1`.
2. Confirm `hello-service.exe` runs from `build/deploy/cpp/bin`.
3. Confirm the other C++ and Rust stand-in processes from the launch script.
4. Use `dds-echo` to observe `HelloWorldTopic`:
   - greeting starts at `Hello, world!`;
   - greeting changes within 15–60 seconds;
   - attitude changes continuously.
5. Confirm `hello-service --topics` prints the contract without DDS.
6. Stop with `cleanup.ps1`.
7. Run `launch-rust.ps1` briefly to confirm the Rust configuration is unchanged.
8. Remove only the temporary `LOCALAPPDATA`.

## Risks

- Non-ASCII greeting literals need a defined source encoding; use MSVC `/utf-8`.
- Greeting tests must use deterministic seeds and explicit next-change accessors.
- Quaternion helper order must match Rust exactly; keep the numerical tests.
- `--topics` must not acquire the guard or create DDS objects.
- `launch-cpp.ps1` must not leave a Rust `hello-service` running.
- Do not modify the Rust model or `launch-rust.ps1`.

## Bookkeeping

- `cpp-format-services` is completed at `2026-11-24 09:33:18`.
- `cpp-hello-service` is `in progress` in `backlog.md`.
- `cpp-hello-service` has no remaining story prerequisites.
- Do not move `cpp-hello-service` to `completed_stories.md` until the user asks to plan the next story.
