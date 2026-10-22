# to-stations-proto
prototype of distributed stations

## Repository path

On Windows the repository must be placed at a short path - for example map a drive with `subst T: C:\Users\you\to-stations-proto` and work from `T:`, or clone to a short path such as `C:\t\to-stations-proto`.

Two scripts help with this:

- `scripts/subst-repo.ps1` maps the repository root to `T:` (override with `-Drive`).
- `scripts/install-subst-startup.ps1` installs that mapping to run at logon, by writing a small wrapper into your Startup folder. It accepts `-Uninstall` to remove the entry.

Neither script is run by `scripts/build.ps1`; run them yourself:

```
.\scripts\subst-repo.ps1
.\scripts\install-subst-startup.ps1
```

Reason: the `cyclonedds` Rust crate builds a copy of CycloneDDS with CMake inside the Cargo target directory, and those nested paths exceed the Windows 260-character limit otherwise. `scripts/build.ps1` uses repository-relative paths and hardcodes no drive letter, so nothing else changes. See `planning/architecture.md` ("Path length") for details.

## Toolchain

The prototype is built on Windows with the Visual Studio C++ toolchain and CMake.

Required:

- Visual Studio Build Tools with the "Desktop development with C++" workload (MSVC).
- CMake 3.16 or later.

Build a C++ target with the Visual Studio generator. No developer shell is required:

```
cmake -S <src> -B <build> -G "Visual Studio 18 2026" -A x64
cmake --build <build> --config Release
```

The executable is written under `<build>/Release/`. If `cmake` reports a different generator name, list the available ones with `cmake -E capabilities`.

To invoke `cl` directly, import the MSVC environment first (adjust the path to your installation):

```
cmd /c "call \"C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat\" && cl ..."
```

### Rust

Rust is installed with rustup using the `stable-x86_64-pc-windows-msvc` toolchain.

```
winget install Rustlang.Rustup
rustup default stable-x86_64-pc-windows-msvc
```

`rust/rust-toolchain.toml` pins the toolchain for the workspace.

Build from the repository root:

```
.\scripts\build.ps1
```

`build.ps1` runs the fork's `cyclonedds-idlc` to generate `build/gen/rust/hello_world.rs`, then builds the Rust workspace. Pass `-Test` to run the tests:

```
.\scripts\build.ps1 -Test
```

Cargo reads `.cargo/config.toml` at the repository root, which sets the target directory `build/rust` and `REPO_ROOT` (the absolute repository root, used by the generated-code `include!`). Because Cargo supplies `REPO_ROOT`, a plain `cargo build`/`cargo test` run from within the repository works too — but the generated file must exist first, so run `build.ps1` once after a clean. Never clean the target directory: dependencies (including the fork) are cached there and should not be rebuilt.

### Display library

The small display app (`lil-d`) renders its format region through a display library that it loads at runtime as a DLL (`display_lib.dll`), not by linking. The ABI is the shared header `include/display_lib.h`; a display app in either language can load a display library in either language (the hybrid deployment).

`scripts/build.ps1` builds `display_lib.dll` and deploys it next to `lil-d.exe` in `build/deploy/bin`. `lil-d` takes `--display-lib <path>` to choose the DLL (default `display_lib.dll`, resolved next to the exe). Because `lil-d` does not link the library, `cargo run -p lil-d` alone does not build the DLL; run `scripts/build.ps1` (or `cargo build` for the whole workspace) first.

`build.ps1` also copies `assets/` to `build/deploy/assets` and the HarfBuzz runtime (`harfbuzz.dll`, `harfbuzz-gpu.dll`) next to the display library. The library loads its font from `<deploy>/assets` (resolved from the executable path, so the exe must live in `build/deploy/bin`); run it through the launch script rather than `cargo run`.

### Cyclone DDS (C)

Eclipse Cyclone DDS `11.0.1` is built from source and installed to `C:\Libraries\cyclonedds`; the source is kept at `C:\Libraries\src\cyclonedds`.

```
git clone --branch 11.0.1 --depth 1 https://github.com/eclipse-cyclonedds/cyclonedds.git C:\Libraries\src\cyclonedds
cmake -S C:\Libraries\src\cyclonedds -B C:\Libraries\src\cyclonedds\build -G "Visual Studio 18 2026" -A x64 -DCMAKE_INSTALL_PREFIX=C:/Libraries/cyclonedds -DBUILD_EXAMPLES=ON -DENABLE_IPV6=ON -DENABLE_SSL=NO -DBUILD_TESTING=OFF
cmake --build C:\Libraries\src\cyclonedds\build --config Release --parallel
cmake --install C:\Libraries\src\cyclonedds\build --config Release
```

The install contains `bin\ddsc.dll`, `bin\idlc.exe`, `lib\ddsc.lib`, `include\ddsc`, and the CMake package at `lib\cmake\CycloneDDS`.

The install prefix (`EXT_PATH_PREFIX`, default `C:\Libraries`) is used by `scripts/build.ps1`, which supplies the runtime: it uses a process-scoped `PATH` during the build and copies `ddsc.dll` into `build/deploy/bin`. No system or user `PATH` change is required.

### Cyclone DDS (C++)

The Cyclone DDS C++ binding (`cyclonedds-cxx`) `11.0.1` is built from source against the core install and installed to `C:\Libraries\cyclonedds-cxx`; the source is kept at `C:\Libraries\src\cyclonedds-cxx`.

```
git clone --branch 11.0.1 --depth 1 https://github.com/eclipse-cyclonedds/cyclonedds-cxx.git C:\Libraries\src\cyclonedds-cxx
cmake -S C:\Libraries\src\cyclonedds-cxx -B C:\Libraries\src\cyclonedds-cxx\build -G "Visual Studio 18 2026" -A x64 -DCMAKE_INSTALL_PREFIX=C:/Libraries/cyclonedds-cxx -DCMAKE_PREFIX_PATH=C:/Libraries/cyclonedds -DBUILD_EXAMPLES=ON -DBUILD_TESTING=OFF
cmake --build C:\Libraries\src\cyclonedds-cxx\build --config Release --parallel
cmake --install C:\Libraries\src\cyclonedds-cxx\build --config Release
```

The install contains `bin\ddscxx.dll`, `bin\cycloneddsidlcxx.dll` (the IDL C++ backend), `lib\ddscxx.lib`, `include\ddscxx`, and the CMake package at `lib\cmake\CycloneDDS-CXX`.

Like the C install, the prefix (`EXT_PATH_PREFIX`, default `C:\Libraries`) is used by `scripts/build.ps1`, which supplies the runtime; no system or user `PATH` change is required.

### Rust DDS binding

The Rust DDS binding uses the `cyclonedds` crate from the to-stations fork of `cyclonedds-rust` (`https://github.com/a101010/cyclonedds-rust`, branch `multifile`, rev `acef68438ef3c8eae56c45afbecdbb89f3d3e115`), pinned as a git dependency in `rust/Cargo.toml`. The fork adds nested `module` support, `#include`/`import`, scoped type references, `@optional` members, and `#[dds_typename]` for C++/IDL parity.

IDL code generation uses the fork's `cyclonedds-idlc`, a **Rust** CLI that wraps the fork's `cyclonedds-build` library. It is built from the same pinned rev by `scripts/build.ps1` and run with `--no-idlc`. Do not confuse it with the C `idlc.exe` installed with CycloneDDS (see "Cyclone DDS (C)" above): the C tool emits C/C++, not Rust.

- IDL types are generated into `build/gen/rust`.
- The Cargo target directory is `build/rust`.
- The crate builds its own copy of CycloneDDS. It is pointed at the external prefix with `CYCLONEDDS_SRC=C:\Libraries\src\cyclonedds` and `CYCLONEDDS_BUILD=C:\Libraries\cyclonedds-rust`, and is linked statically so no CycloneDDS DLL is needed at runtime. No `PATH` change is required.
- `CYCLONEDDS_BUILD` must already contain a built library; build the static copy once with:

```
cmake -S C:\Libraries\src\cyclonedds -B C:\Libraries\cyclonedds-rust -G "Visual Studio 18 2026" -A x64 -DBUILD_SHARED_LIBS=OFF -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DBUILD_TESTING=OFF -DBUILD_IDLC=OFF -DBUILD_DDSPERF=OFF -DBUILD_EXAMPLES=OFF -DENABLE_LTO=OFF -DENABLE_SECURITY=OFF -DENABLE_SSL=OFF
cmake --build C:\Libraries\cyclonedds-rust --target ddsc --config Release
```

### HarfBuzz

Format text is rendered with HarfBuzz's experimental GPU library (`libharfbuzz-gpu`, the Slug algorithm). HarfBuzz `14.6.0` is built from source and installed to `C:\Libraries\harfbuzz`; the source is kept at `C:\Libraries\src\harfbuzz`.

```
git clone --branch 14.6.0 --depth 1 https://github.com/harfbuzz/harfbuzz.git C:\Libraries\src\harfbuzz
cmake -S C:\Libraries\src\harfbuzz -B C:\Libraries\src\harfbuzz\build -G "Visual Studio 18 2026" -A x64 -DCMAKE_INSTALL_PREFIX=C:/Libraries/harfbuzz -DBUILD_SHARED_LIBS=ON -DHB_BUILD_GPU=ON -DHB_BUILD_GPU_DEMO=OFF -DHB_BUILD_SUBSET=OFF -DHB_BUILD_RASTER=OFF -DHB_BUILD_VECTOR=OFF -DHB_BUILD_UTILS=OFF
cmake --build C:\Libraries\src\harfbuzz\build --config Release --parallel
cmake --install C:\Libraries\src\harfbuzz\build --config Release
```

The install contains `bin\harfbuzz.dll`, `bin\harfbuzz-gpu.dll`, `lib\harfbuzz.lib`, `lib\harfbuzz-gpu.lib`, and `include\harfbuzz\hb-gpu.h`. The Rust crate `rust/lib/harfbuzz` links these from the external prefix (`EXT_PATH_PREFIX`, default `C:\Libraries`); `scripts/build.ps1` sets the variable and prepends `C:\Libraries\harfbuzz\bin` to the process `PATH` so the DLLs are found during build and test. No system or user `PATH` change is required.

The HarfBuzz CMake port is community-maintained; if the GPU target does not build, Meson is the fallback (`meson setup build --prefix=C:/Libraries/harfbuzz -Dgpu=enabled -Dsubset=disabled -Draster=disabled -Dvector=disabled -Dutilities=disabled`).

### DDS configuration

All DDS traffic uses IPv6 multicast on a Microsoft loopback adapter named `StationsDDS`. The shared configuration is `DDS/cyclonedds-config.xml`, applied through the `CYCLONEDDS_URI` environment variable.

Verify the adapter and its IPv6 prerequisites (creating it is a one-time manual step):

```
.\scripts\verify-loopback.ps1
```

See `docs/loopback-adapter.md` for creating the adapter and for troubleshooting.

Point CycloneDDS at the configuration (use `file://` with two slashes, or a plain path):

```
$env:CYCLONEDDS_URI = 'file://T:/DDS/cyclonedds-config.xml'
```

### dds-echo traffic generators

`dds-echo` captures DDSI-RTPS multicast so traffic can be inspected. To generate traffic while it is capturing, use the committed wrappers under `scripts/` (each points the source at `DDS/cyclonedds-config.xml` and makes `ddsc.dll` available):

```
.\scripts\traffic-throughput.ps1 -Seconds 30     # throughput example (subscriber + publisher)
.\scripts\traffic-listtopics.ps1                 # ~10 s of discovery
.\scripts\traffic-roundtrip.ps1                  # stations-dds round_trip test
.\scripts\traffic-fake-topic.ps1                 # crafted SEDP 'FakeTopic' (no DDS participant)
```

The throughput and listtopics wrappers use the CycloneDDS example binaries built into the source build tree under `$EXT_PATH_PREFIX\src\cyclonedds\build\bin\Release` (build CycloneDDS with `-DBUILD_EXAMPLES=ON`).

`traffic-fake-topic.ps1` sends crafted RTPS datagrams to the multicast group without a CycloneDDS participant: several datagrams covering every named submessage kind, SEDP endpoint announcements for two unregistered topics (`FakeTopicOne`, `FakeTopicTwo`) and a reader, an SPDP participant announcement, a `HelloWorldTopic` announcement plus a CDR-encoded sample (a registered topic, so it shows decoded fields), and user data for the unregistered topics. It verifies `dds-echo`'s parsing, topic attribution, decoding, and UI end to end; it is not a substitute for capturing real CycloneDDS SEDP.

`dds-echo`'s counts are per **datagram**, not per sample: CycloneDDS coalesces several samples into one RTPS datagram (a `ThroughputPublisher` burst arrives as far fewer, larger datagrams), so the captured datagram count is lower than the publisher's sample count even when nothing is dropped. Sample-level counting needs the RTPS submessage parsing done in the `dds-echo-sedp` story.
