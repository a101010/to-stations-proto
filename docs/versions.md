# Pinned third-party versions

The pinned version and install location of each external component. Each install story adds its component here. External components are installed to a prefix outside the repository (for example `C:\Libraries`) and are never committed.

| Component | Pinned version | Install location |
|---|---|---|
| Visual Studio Build Tools (MSVC) | 2026, "Desktop development with C++" | system |
| CMake | 3.16 or later | system |
| Rust toolchain | `stable-x86_64-pc-windows-msvc` (pinned in `rust/rust-toolchain.toml`) | system (rustup) |
| Eclipse Cyclone DDS (C) | 11.0.1 | `C:\Libraries\cyclonedds` (source `C:\Libraries\src\cyclonedds`) |
| cyclonedds-cxx | 11.0.1 | `C:\Libraries\cyclonedds-cxx` (source `C:\Libraries\src\cyclonedds-cxx`) |
| cyclonedds (Rust crate) | 3.0.1, with `cyclonedds-build` 3.0.1 | Cargo; static CycloneDDS at `C:\Libraries\cyclonedds-rust` |
| Rust rendering crates | winit 0.30.13, glutin 0.32.3, glutin-winit 0.5.0, glow 0.17, egui/egui-winit/egui_glow 0.36.2 | Cargo (`rust/Cargo.toml`, `[workspace.dependencies]`) |
