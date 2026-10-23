# Builds the project: generates Rust types from the shared IDL in DDS/ into
# build/gen/rust, then builds the Rust workspace. Pass -Test to run the tests.
#
# It never cleans the Cargo target directory, so dependencies (including the
# pinned fork of cyclonedds-rust) are not rebuilt unless build/ is removed.

[CmdletBinding()]
param(
    [switch]$Test
)

$ErrorActionPreference = 'Stop'

# Repository root, resolved from this script's own location (no '..' paths).
$repoRoot = Split-Path -Parent $PSScriptRoot

# The external prefix holds installed components (CycloneDDS, cyclonedds-cxx,
# HarfBuzz, SDL3).
$extPathPrefix = if ($env:EXT_PATH_PREFIX) { $env:EXT_PATH_PREFIX } else { 'C:\Libraries' }

# Expose the prefix to build scripts (harfbuzz's build.rs) and provision the
# HarfBuzz runtime for build and test without changing the user PATH.
$env:EXT_PATH_PREFIX = $extPathPrefix
$harfbuzzBin = Join-Path $extPathPrefix 'harfbuzz\bin'
if (Test-Path $harfbuzzBin) {
    $env:PATH = "$harfbuzzBin;$env:PATH"
}

# The fork's cyclonedds-idlc is Rust codegen (it wraps cyclonedds-build). The C
# idlc.exe emits C/C++, not Rust.
$forkUrl = 'https://github.com/a101010/cyclonedds-rust'
$forkRev = 'acef68438ef3c8eae56c45afbecdbb89f3d3e115'

$codegenTargetDir = Join-Path $repoRoot 'build\rust\codegen'
$installRoot      = Join-Path $repoRoot 'build'
$genDir           = Join-Path $repoRoot 'build\gen\rust'
$ddsDir           = Join-Path $repoRoot 'DDS'

New-Item -ItemType Directory -Force -Path $codegenTargetDir, $genDir | Out-Null

# Build/install the tool from the pinned rev. --target-dir caches the compile,
# so re-running is a no-op unless the rev changes; it is only rebuilt when
# build/ is removed. cargo install places the binary in <root>\bin.
& cargo install --git $forkUrl --rev $forkRev cyclonedds-idlc --locked --force `
    --target-dir $codegenTargetDir --root $installRoot
if ($LASTEXITCODE -ne 0) { throw "cargo install cyclonedds-idlc failed ($LASTEXITCODE)" }

$cycloneddsIdlc = Join-Path $installRoot 'bin\cyclonedds-idlc.exe'

# Run with --no-idlc so it never invokes the C idlc; the built-in parser produces
# the Rust. #[dds_typename] is emitted by default.
& $cycloneddsIdlc `
    --input (Join-Path $ddsDir 'hello_world.idl') `
    --output-dir $genDir `
    --include-dir $ddsDir `
    --no-idlc
if ($LASTEXITCODE -ne 0) { throw "IDL code generation failed ($LASTEXITCODE)" }

Write-Host "Generated $(Join-Path $genDir 'hello_world.rs')"

# Build the Rust workspace. Cargo reads .cargo/config.toml at the repository root
# (target-dir and REPO_ROOT), so it must be invoked from there.
Push-Location $repoRoot
try {
    if ($Test) {
        & cargo test --manifest-path 'rust\Cargo.toml'
        if ($LASTEXITCODE -ne 0) { throw "cargo test failed ($LASTEXITCODE)" }
        # cargo test does not always emit the tool binary; build it for the scan.
        & cargo build --manifest-path 'rust\Cargo.toml' --bin dds-contracts
        if ($LASTEXITCODE -ne 0) { throw "cargo build dds-contracts failed ($LASTEXITCODE)" }
        $toolDir = 'build\rust\debug'
    } else {
        & cargo build --release --manifest-path 'rust\Cargo.toml'
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
        $toolDir = 'build\rust\release'
    }
}
finally {
    Pop-Location
}

# Validate the service contracts, scan the application crates for raw DDS
# topic-API bypasses, and regenerate the readable summary. The scan scope is
# the crates being built (here the application/tool crates), never the shared
# libraries, so the Node implementation is out of scope. No application crate
# uses DDS yet, so the scan currently passes vacuously.
$ddsContracts = Join-Path $repoRoot "$toolDir\dds-contracts.exe"
$scanArgs = @()
foreach ($src in @('rust\ui\lil-d\src', 'rust\tool\equirect-to-cubemap\src', 'rust\tool\dds-echo\src', 'rust\service\hello-view-service\src')) {
    $scanArgs += @('--scan', (Join-Path $repoRoot $src))
}
& $ddsContracts '--contracts' (Join-Path $repoRoot 'contracts') @scanArgs '--summary' (Join-Path $repoRoot 'build\topics.md')
if ($LASTEXITCODE -ne 0) { throw "dds-contracts failed ($LASTEXITCODE)" }
Write-Host "Validated contracts and wrote build\topics.md"

# Assemble the deployable layout. Only build/deploy is durable; everything else
# under build/ is temporary. Each story that adds an executable or library
# extends this step. The release build is deployed so the display is a GUI
# application with no console window (see lil-d's windows_subsystem attribute).
if (-not $Test) {
    $deployRoot = Join-Path $repoRoot 'build\deploy'
    $deployBin = Join-Path $deployRoot 'bin'
    $deployConfig = Join-Path $deployRoot 'config'
    New-Item -ItemType Directory -Force -Path $deployBin, $deployConfig | Out-Null
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\lil-d.exe') -Destination $deployBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\display_lib.dll') -Destination $deployBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\dds-echo.exe') -Destination $deployBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\hello-view-service.exe') -Destination $deployBin -Force

    # The display library loads its font and (later) the earth cube map from
    # <deploy>/assets, and links the HarfBuzz runtime from next to the exe.
    Copy-Item -Path (Join-Path $repoRoot 'assets') -Destination $deployRoot -Recurse -Force
    Copy-Item -Path (Join-Path $extPathPrefix 'harfbuzz\bin\harfbuzz.dll') -Destination $deployBin -Force
    Copy-Item -Path (Join-Path $extPathPrefix 'harfbuzz\bin\harfbuzz-gpu.dll') -Destination $deployBin -Force

    # The shared DDS configuration for the StationsDDS loopback adapter, and the
    # contracts the services and the display library read from <deploy>/config.
    Copy-Item -Path (Join-Path $repoRoot 'DDS\cyclonedds-config.xml') -Destination $deployConfig -Force
    Copy-Item -Path (Join-Path $repoRoot 'contracts\topics.toml') -Destination $deployConfig -Force
    Copy-Item -Path (Join-Path $repoRoot 'contracts\services') -Destination $deployConfig -Recurse -Force
    Write-Host "Deployed lil-d.exe, hello-view-service.exe, dds-echo.exe, display_lib.dll, assets, the HarfBuzz runtime, and the DDS config + contracts"
}
