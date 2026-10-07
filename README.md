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

### Cyclone DDS (C)

Eclipse Cyclone DDS `11.0.1` is built from source and installed to `C:\Libraries\cyclonedds`; the source is kept at `C:\Libraries\src\cyclonedds`.

```
git clone --branch 11.0.1 --depth 1 https://github.com/eclipse-cyclonedds/cyclonedds.git C:\Libraries\src\cyclonedds
cmake -S C:\Libraries\src\cyclonedds -B C:\Libraries\src\cyclonedds\build -G "Visual Studio 18 2026" -A x64 -DCMAKE_INSTALL_PREFIX=C:/Libraries/cyclonedds -DBUILD_EXAMPLES=ON -DENABLE_IPV6=ON -DENABLE_SSL=NO -DBUILD_TESTING=OFF
cmake --build C:\Libraries\src\cyclonedds\build --config Release --parallel
cmake --install C:\Libraries\src\cyclonedds\build --config Release
```

The install contains `bin\ddsc.dll`, `bin\idlc.exe`, `lib\ddsc.lib`, `include\ddsc`, and the CMake package at `lib\cmake\CycloneDDS`.

The install prefix is hardcoded in `scripts/build.ps1`, which supplies the runtime: it uses a process-scoped `PATH` during the build and copies `ddsc.dll` into `build/deploy/bin`. No system or user `PATH` change is required.

### Cyclone DDS (C++)

The Cyclone DDS C++ binding (`cyclonedds-cxx`) `11.0.1` is built from source against the core install and installed to `C:\Libraries\cyclonedds-cxx`; the source is kept at `C:\Libraries\src\cyclonedds-cxx`.

```
git clone --branch 11.0.1 --depth 1 https://github.com/eclipse-cyclonedds/cyclonedds-cxx.git C:\Libraries\src\cyclonedds-cxx
cmake -S C:\Libraries\src\cyclonedds-cxx -B C:\Libraries\src\cyclonedds-cxx\build -G "Visual Studio 18 2026" -A x64 -DCMAKE_INSTALL_PREFIX=C:/Libraries/cyclonedds-cxx -DCMAKE_PREFIX_PATH=C:/Libraries/cyclonedds -DBUILD_EXAMPLES=ON -DBUILD_TESTING=OFF
cmake --build C:\Libraries\src\cyclonedds-cxx\build --config Release --parallel
cmake --install C:\Libraries\src\cyclonedds-cxx\build --config Release
```

The install contains `bin\ddscxx.dll`, `bin\cycloneddsidlcxx.dll` (the IDL C++ backend), `lib\ddscxx.lib`, `include\ddscxx`, and the CMake package at `lib\cmake\CycloneDDS-CXX`.

Like the C install, the prefix is hardcoded in `scripts/build.ps1`, which supplies the runtime; no system or user `PATH` change is required.

### Rust DDS binding

The Rust DDS binding uses the `cyclonedds` crate `3.0.1` with `cyclonedds-build` `3.0.1` for IDL code generation.

- IDL types are generated into `build/gen/rust`.
- The Cargo target directory is `build/rust`.
- The crate builds its own copy of CycloneDDS. It is pointed at the external prefix with `CYCLONEDDS_SRC=C:\Libraries\src\cyclonedds` and `CYCLONEDDS_BUILD=C:\Libraries\cyclonedds-rust`, and is linked statically so no CycloneDDS DLL is needed at runtime. No `PATH` change is required.
- `CYCLONEDDS_BUILD` must already contain a built library; build the static copy once with:

```
cmake -S C:\Libraries\src\cyclonedds -B C:\Libraries\cyclonedds-rust -G "Visual Studio 18 2026" -A x64 -DBUILD_SHARED_LIBS=OFF -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DBUILD_TESTING=OFF -DBUILD_IDLC=OFF -DBUILD_DDSPERF=OFF -DBUILD_EXAMPLES=OFF -DENABLE_LTO=OFF -DENABLE_SECURITY=OFF -DENABLE_SSL=OFF
cmake --build C:\Libraries\cyclonedds-rust --target ddsc --config Release
```

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
