# to-stations-proto

Prototype of distributed stations.

## Dependencies

See [dependencies.md](dependencies.md) for the complete build dependency inventory, including transitive dependencies, source URLs, and approximate sizes. Pinned versions and install locations are in [docs/versions.md](docs/versions.md).

## Repository path

On Windows the repository must be placed at a short path. For example, map a drive with:

```powershell
.\scripts\subst-repo.ps1
```

This maps the repository root to `T:` by default. Use `-Drive` to choose another drive. To install the mapping at logon, run:

```powershell
.\scripts\install-subst-startup.ps1
```

Use `-Uninstall` to remove it. These scripts are not run automatically by `build.ps1`.

The short path is required because the `cyclonedds` Rust crate builds a copy of CycloneDDS with CMake inside the Cargo target directory, and the nested paths can exceed the Windows 260-character limit.

## Toolchain

Required:

- Windows 10/11 x64.
- Visual Studio Build Tools with the "Desktop development with C++" workload.
- CMake 3.16 or later.
- Rustup with the `stable-x86_64-pc-windows-msvc` toolchain.
- Git.
- PowerShell.
- The external C/C++ components listed below.

Install Rust with:

```powershell
winget install Rustlang.Rustup
rustup default stable-x86_64-pc-windows-msvc
```

The workspace pins the toolchain in `rust/rust-toolchain.toml`.

The external components are installed under `EXT_PATH_PREFIX` (default `C:\Libraries`) and are never committed.

### SDL3

```powershell
git clone --branch release-3.4.20 --depth 1 https://github.com/libsdl-org/SDL.git C:\Libraries\src\sdl3
cmake -S C:\Libraries\src\sdl3 -B C:\Libraries\src\sdl3\build -G "Visual Studio 18 2026" -A x64 -DCMAKE_INSTALL_PREFIX=C:/Libraries/sdl3 -DSDL_SHARED=OFF -DSDL_STATIC=ON -DSDL_TEST_LIBRARY=OFF -DSDL_TESTS=OFF
cmake --build C:\Libraries\src\sdl3\build --config Release --parallel
cmake --install C:\Libraries\src\sdl3\build --config Release
```

### Cyclone DDS (C)

```powershell
git clone --branch 11.0.1 --depth 1 https://github.com/eclipse-cyclonedds/cyclonedds.git C:\Libraries\src\cyclonedds
cmake -S C:\Libraries\src\cyclonedds -B C:\Libraries\src\cyclonedds\build -G "Visual Studio 18 2026" -A x64 -DCMAKE_INSTALL_PREFIX=C:/Libraries/cyclonedds -DBUILD_EXAMPLES=ON -DENABLE_IPV6=ON -DENABLE_SSL=NO -DBUILD_TESTING=OFF
cmake --build C:\Libraries\src\cyclonedds\build --config Release --parallel
cmake --install C:\Libraries\src\cyclonedds\build --config Release
```

### Cyclone DDS (C++)

```powershell
git clone --branch 11.0.1 --depth 1 https://github.com/eclipse-cyclonedds/cyclonedds-cxx.git C:\Libraries\src\cyclonedds-cxx
cmake -S C:\Libraries\src\cyclonedds-cxx -B C:\Libraries\src\cyclonedds-cxx\build -G "Visual Studio 18 2026" -A x64 -DCMAKE_INSTALL_PREFIX=C:/Libraries/cyclonedds-cxx -DCMAKE_PREFIX_PATH=C:/Libraries/cyclonedds -DBUILD_EXAMPLES=ON -DBUILD_TESTING=OFF
cmake --build C:\Libraries\src\cyclonedds-cxx\build --config Release --parallel
cmake --install C:\Libraries\src\cyclonedds-cxx\build --config Release
```

### CycloneDDS static build for Rust

```powershell
cmake -S C:\Libraries\src\cyclonedds -B C:\Libraries\cyclonedds-rust -G "Visual Studio 18 2026" -A x64 -DBUILD_SHARED_LIBS=OFF -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DBUILD_TESTING=OFF -DBUILD_IDLC=OFF -DBUILD_DDSPERF=OFF -DBUILD_EXAMPLES=OFF -DENABLE_LTO=OFF -DENABLE_SECURITY=OFF -DENABLE_SSL=OFF
cmake --build C:\Libraries\cyclonedds-rust --target ddsc --config Release
```

### HarfBuzz

HarfBuzz includes the experimental GPU library used for text rendering.

```powershell
git clone --branch 14.6.0 --depth 1 https://github.com/harfbuzz/harfbuzz.git C:\Libraries\src\harfbuzz
cmake -S C:\Libraries\src\harfbuzz -B C:\Libraries\src\harfbuzz\build -G "Visual Studio 18 2026" -A x64 -DCMAKE_INSTALL_PREFIX=C:/Libraries/harfbuzz -DBUILD_SHARED_LIBS=ON -DHB_BUILD_GPU=ON -DHB_BUILD_GPU_DEMO=OFF -DHB_BUILD_SUBSET=OFF -DHB_BUILD_RASTER=OFF -DHB_BUILD_VECTOR=OFF -DHB_BUILD_UTILS=OFF
cmake --build C:\Libraries\src\harfbuzz\build --config Release --parallel
cmake --install C:\Libraries\src\harfbuzz\build --config Release
```

If the HarfBuzz CMake GPU target does not build, its Meson fallback is documented in [dependencies.md](dependencies.md).

## Build and test

Build from the repository root:

```powershell
.\scripts\build.ps1
```

Run tests:

```powershell
.\scripts\build.ps1 -Test
```

`build.ps1` generates the Rust IDL code, builds the Rust workspace, configures and builds the C++ project, runs CTest when `-Test` is passed, and assembles `build/deploy`.

The Rust Cargo target directory is `build/rust`; do not clean it unless you intend to rebuild all dependencies.

## Run

Build first, then launch one of the configurations:

```powershell
.\scripts\launch-rust.ps1
.\scripts\launch-cpp.ps1
```

Both scripts accept `-DisplayId` with one or more display ids; the default is `0,1`.

Stop all components with:

```powershell
.\scripts\cleanup.ps1
```

The deploy layout is `build/deploy/rust/bin`, `build/deploy/cpp/bin`, and shared `build/deploy/assets` and `build/deploy/config`.

## DDS configuration

DDS uses IPv6 multicast on a Microsoft loopback adapter named `StationsDDS`. Verify it with:

```powershell
.\scripts\verify-loopback.ps1
```

The shared configuration is `DDS/cyclonedds-config.xml`. For direct tools, set:

```powershell
$env:CYCLONEDDS_URI = 'file://T:/DDS/cyclonedds-config.xml'
```

See [docs/loopback-adapter.md](docs/loopback-adapter.md) for adapter creation and troubleshooting.

## Tools

- `build/deploy/rust/bin/dds-echo.exe` captures DDSI-RTPS multicast and provides a Send tab for registered topics. Select a record to inspect and copy its decoded detail.
- `dds-contracts` validates contracts and writes `build/topics.md`.
- `equirect-to-cubemap` converts an equirectangular image into the six earth cube-map faces.
- `scripts/traffic-*.ps1` provide traffic generators for `dds-echo`.

## Documentation

- [dependencies.md](dependencies.md) — complete build dependency inventory.
- [docs/versions.md](docs/versions.md) — pinned versions and install locations.
- [planning/architecture.md](planning/architecture.md) — design, layout, and interop rules.
- [docs/loopback-adapter.md](docs/loopback-adapter.md) — loopback adapter setup.
