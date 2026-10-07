# to-stations-proto
prototype of distributed stations

## Toolchain

The prototype is built on Windows with the Visual Studio C++ toolchain and CMake.

Required:

- Visual Studio Build Tools with the "Desktop development with C++" workload (MSVC).
- CMake 3.16 or later.

Verified on the development machine:

- Visual Studio Build Tools 2026 `18.9.12120.119` (MSVC `14.51.36231`).
- CMake `4.4.3`.

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

Verified on the development machine: rustc/cargo `1.99.0`, rustup `1.29.1`.

`rust/rust-toolchain.toml` pins the toolchain for the workspace.
