# Current story: cpp-dds

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Generate C++ types from `DDS/` with the installed `idlc -l cxx` backend into `build/gen/cpp`; add the C++ `stations-dds` library with participant/node creation, typed readers/writers, contract loading and validation, deploy-relative `CYCLONEDDS_URI` setup, and CycloneDDS-CXX linking. Use toml++ 3.4.0 for contracts and add committed CTest tests for contract validation, generated type names, and a DDS round trip.

- **Depends on:** none.
- **Minimal test:** a C++ publisher and subscriber exchange `HelloWorldTopic`; contract validation rejects an unregistered topic, type mismatch, and unknown QoS; generated type names match the shared registry.

## Context

- `sdl3-imgui` is complete: SDL3, glad, Dear ImGui, CMake targets, and C++ build integration exist.
- `cpp/` currently builds third-party libraries only.
- CycloneDDS 11.0.1 and CycloneDDS-CXX 11.0.1 are installed under the external prefix.
- The Rust `stations-dds` library already defines the contract and typed-endpoint behavior to mirror.
- The shared IDL files are `DDS/math.idl`, `DDS/format_manager.idl`, and `DDS/hello_world.idl`.
- `contracts/topics.toml` and `contracts/services/*.toml` define the registry and service contracts.
- No C++ services or display library components are part of this story.

## Decisions

1. **Use the official `idlcxx_generate` CMake helper.** It wraps `idlc -l <cycloneddsidlcxx>` and produces generated `.hpp`/`.cpp` targets.
2. **Generate all three IDL modules.** `hello_world.hpp` includes `math.hpp` and `format_manager.hpp`, so all three must be generated.
3. **Use toml++ 3.4.0 as a single-header TOML parser.** Copy the header and license under `cpp/third_party/tomlplusplus/`.
4. **Use exceptions for C++ errors.** Contract and DDS failures use clear exception messages; tests assert the failure categories.
5. **Keep generated types separate from the wrapper.** `hello_world_types` carries generated sources/includes; `stations-dds` provides the contract-aware Node and endpoint templates.
6. **Real tests are committed.** The C++ tests under `cpp/libs/stations-dds/tests/` are normal tests, not temporary smoke tests, and run through CTest.
7. **No smoke-test generation in `build.ps1`.** `-Test` runs committed CTest tests only.
8. **Set `CYCLONEDDS_URI` explicitly before participant creation.** The Node accepts the config path and sets the process environment from it.
9. **Use a high test domain.** The round-trip test uses domain 42 to avoid the running fleet.
10. **No deployment yet.** C++ runtime DLL deployment and launch scripts remain later stories.

## Scope

**In scope**

- Add `cpp/third_party/tomlplusplus/` with toml++ 3.4.0 and its license.
- Add a `tomlplusplus` interface target.
- Add `cpp/libs/stations-dds/` with contract parsing, validation, Node, typed reader/writer templates, and QoS mapping.
- Generate C++ IDL types into `build/gen/cpp`.
- Add committed C++ tests and CTest registration.
- Update `scripts/build.ps1` for external DDS paths and CTest.
- Update `README.md`, `docs/versions.md`, and `planning/architecture.md`.
- Update planning bookkeeping.

**Out of scope**

- C++ services, display library, `lil-d`, `paws`, or tools.
- C++ deployment, runtime DLL copying, or launch scripts.
- C++ `dds-echo` or `dds-contracts`.
- Changes to IDL, contracts, or Rust code.
- Temporary smoke projects or generated test programs in `build.ps1`.

## C++ IDL generation

In `cpp/libs/stations-dds/CMakeLists.txt`:

- `find_package(CycloneDDS REQUIRED)`
- `find_package(CycloneDDS-CXX REQUIRED)`
- `include("${CycloneDDS-CXX_DIR}/idlcxx/Generate.cmake")`

Call:

```cmake
idlcxx_generate(
    TARGET hello_world_types
    FILES
        ${REPO_ROOT}/DDS/math.idl
        ${REPO_ROOT}/DDS/format_manager.idl
        ${REPO_ROOT}/DDS/hello_world.idl
    BASE_DIR ${REPO_ROOT}/DDS
    OUTPUT_DIR ${REPO_ROOT}/build/gen/cpp
    INCLUDES ${REPO_ROOT}/DDS
    WARNINGS no-implicit-extensibility
)
```

Generated artifacts:

- `build/gen/cpp/math.hpp/.cpp`
- `build/gen/cpp/format_manager.hpp/.cpp`
- `build/gen/cpp/hello_world.hpp/.cpp`

`hello_world_types` is the generated-type target. Future C++ apps link it alongside `stations-dds`.

## `stations-dds` library

Proposed layout:

```text
cpp/libs/stations-dds/
  CMakeLists.txt
  include/stations_dds/contract.hpp
  include/stations_dds/dds.hpp
  src/contract.cpp
  src/dds.cpp
  tests/main.cpp
```

### Contract API

Mirror the Rust contract model:

- `TopicDef`
- `TopicRegistry`
- `ServiceContract`
- `ServiceMeta`
- `TopicRef`
- `known_types()`

Behavior:

- load registry and service contracts from TOML
- validate unique topic names
- validate known IDL types
- validate known QoS names
- reject unknown topic references
- verify `service.name` matches the contract file stem
- render a service contract for future `--topics` use

Use toml++ privately in `contract.cpp`.

### DDS API

`Node` owns:

- `dds::domain::DomainParticipant`
- `dds::pub::Publisher`
- `dds::sub::Subscriber`
- validated `TopicRegistry`

Constructor:

- accepts domain id, registry, and explicit `cyclonedds-config.xml` path
- validates the registry
- sets `CYCLONEDDS_URI` to `file://` plus the absolute config path before creating the participant

Endpoint templates:

- `writer<T>(topic)` checks registration and `dds::topic::topic_type_name<T>::value()`
- `reader<T>(topic)` performs the same checks
- apply the contract QoS:
  - `default`
  - `reliable`
  - `best_effort`
  - `transient_local`
- create the topic and typed endpoint

Error messages distinguish:

- unregistered topic
- type mismatch
- unknown QoS
- contract parse/validation failure

## CMake

`cpp/CMakeLists.txt`:

- `enable_testing()`
- `add_subdirectory(libs/stations-dds)`

`cpp/third_party/CMakeLists.txt`:

- add a `tomlplusplus` interface target with the toml++ include directory

`cpp/libs/stations-dds/CMakeLists.txt`:

- find CycloneDDS and CycloneDDS-CXX
- include the IDLCXX generator helper
- generate `hello_world_types`
- build `stations-dds`
- expose public wrapper includes
- link `CycloneDDS-CXX::ddscxx` publicly
- link `tomlplusplus` privately
- build and register `stations-dds-tests`

## Build script

`scripts/build.ps1`:

- Add `C:\Libraries\cyclonedds\bin` and `C:\Libraries\cyclonedds-cxx\bin` to the process `PATH`.
- Add `sdl3`, `cyclonedds`, and `cyclonedds-cxx` to `CMAKE_PREFIX_PATH`.
- Fail early with clear messages if the SDL3, CycloneDDS, or CycloneDDS-CXX CMake packages are missing.
- Under `-Test`, run committed CTest tests from `build/cpp`.
- Do not generate temporary smoke projects or add smoke-test source.

## Tests

Commit `stations-dds-tests` covering:

- valid registry parsing
- duplicate topic rejection
- unknown topic rejection
- unknown IDL type rejection
- unknown QoS rejection
- service-name/file-stem mismatch
- generated type names:
  - `hello_world::HelloWorldTopic`
  - `hello_world::HelloWorldViewTopic`
  - `hello_world::HelloWorldInputTopic`
  - `hello_world::HelloWorldCtlTopic`
  - format-manager topic types
- real DDS round trip on domain 42:
  - create reader and writer for `HelloWorldTopic`
  - wait for publication match
  - write a greeting and attitude
  - take and validate the sample
  - use `DDS/cyclonedds-config.xml`

The test gets the repository root through a test-only compile definition. The build script supplies the DDS DLL directories on `PATH` for CTest.

## Documentation

`docs/versions.md`:

- toml++ `3.4.0` at `cpp/third_party/tomlplusplus`

`README.md`:

- Note that the C++ build finds SDL3, CycloneDDS, and CycloneDDS-CXX through `CMAKE_PREFIX_PATH`.
- Do not add smoke-test details.

`planning/architecture.md`:

- Note that the C++ `stations-dds` library mirrors the Rust contract-aware Node and generated-type workflow.

## Verification

1. `scripts/build.ps1 -Test`
   - C++ IDL generation succeeds.
   - `stations-dds` and generated types compile.
   - CTest tests pass.
   - Existing Rust tests pass.
2. `scripts/build.ps1`
   - Release build succeeds.
3. Confirm generated files exist only under `build/gen/cpp`.
4. Confirm no generated or smoke files are committed.
5. `git diff --check`.

## Risks

- **IDL generation path:** all three modules must be listed so `hello_world.hpp` can include `math.hpp` and `format_manager.hpp`.
- **PATH setup:** `idlc` and both CycloneDDS DLL directories must be available before CMake generation and CTest.
- **QoS translation:** C++ policies must match the Rust contract names.
- **Environment setup:** `CYCLONEDDS_URI` must be set before participant creation.
- **Test isolation:** use domain 42 so the committed round-trip test does not collide with the fleet.
- **No deployment:** tests use the external prefix through the process PATH; deployment remains a later story.

## Files touched

- `cpp/CMakeLists.txt`
- `cpp/third_party/CMakeLists.txt`
- `cpp/third_party/tomlplusplus/` (new)
- `cpp/libs/stations-dds/` (new)
- `scripts/build.ps1`
- `README.md`
- `docs/versions.md`
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

## Bookkeeping

- `sdl3-imgui` is completed at `2026-11-18 16:36:02`.
- `cpp-dds` is `in progress` in `backlog.md`.
- `cpp-dds` has no remaining story prerequisites.
- Do not move `cpp-dds` to `completed_stories.md` until the user asks to plan the next story.
