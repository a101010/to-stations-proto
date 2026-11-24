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
$cycloneddsBin = Join-Path $extPathPrefix 'cyclonedds\bin'
$cycloneddsCxxBin = Join-Path $extPathPrefix 'cyclonedds-cxx\bin'
$env:PATH = "$cycloneddsBin;$cycloneddsCxxBin;$harfbuzzBin;$env:PATH"

# The fork's cyclonedds-idlc is Rust codegen (it wraps cyclonedds-build). The C
# idlc.exe emits C/C++, not Rust.
$forkUrl = 'https://github.com/a101010/cyclonedds-rust'
$forkRev = '952906b5018831683c0863cd11001bdac427ee10'

# Key the cargo install target by revision: Cargo otherwise can reuse a cached
# cyclonedds-idlc binary when the git rev changes but the package version does
# not, producing code with the old generator.
$codegenTargetDir = Join-Path $repoRoot "build\rust\codegen\$($forkRev.Substring(0, 8))"
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
#
# Deployed binaries must not carry the repository path in panic/source metadata.
$remap = "--remap-path-prefix=$repoRoot=/app"
if ($env:RUSTFLAGS) { $env:RUSTFLAGS = "$env:RUSTFLAGS $remap" } else { $env:RUSTFLAGS = $remap }
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
# libraries, so the Node implementation is out of scope. The application crates
# create endpoints only through `Node`, so the scan passes.
$ddsContracts = Join-Path $repoRoot "$toolDir\dds-contracts.exe"
$scanArgs = @()
foreach ($src in @('rust\ui\lil-d\src', 'rust\ui\paws\src', 'rust\tool\equirect-to-cubemap\src', 'rust\tool\dds-echo\src', 'rust\service\hello-service\src', 'rust\service\hello-view-service\src', 'rust\service\hello-ctl-service\src', 'rust\service\format-ctl-service\src', 'rust\service\format-view-service\src')) {
    $scanArgs += @('--scan', (Join-Path $repoRoot $src))
}
& $ddsContracts '--contracts' (Join-Path $repoRoot 'contracts') @scanArgs '--summary' (Join-Path $repoRoot 'build\topics.md')
if ($LASTEXITCODE -ne 0) { throw "dds-contracts failed ($LASTEXITCODE)" }
Write-Host "Validated contracts and wrote build\topics.md"

$cppSrc = Join-Path $repoRoot 'cpp'
$cppBuild = Join-Path $repoRoot 'build\cpp'
$cppConfig = 'Release'
$cppExtPath = $extPathPrefix -replace '\\', '/'
$cppInstall = (Join-Path $repoRoot 'build\deploy\cpp') -replace '\\', '/'
$cppPrefixPaths = @(
    (Join-Path $extPathPrefix 'sdl3'),
    (Join-Path $extPathPrefix 'cyclonedds'),
    (Join-Path $extPathPrefix 'cyclonedds-cxx')
) | ForEach-Object { $_ -replace '\\', '/' }
$cppPrefixPath = $cppPrefixPaths -join ';'

$requiredPackages = @(
    (Join-Path $extPathPrefix 'sdl3\cmake\SDL3Config.cmake'),
    (Join-Path $extPathPrefix 'cyclonedds\lib\cmake\CycloneDDS\CycloneDDSConfig.cmake'),
    (Join-Path $extPathPrefix 'cyclonedds-cxx\lib\cmake\CycloneDDS-CXX\CycloneDDS-CXXConfig.cmake')
)
foreach ($config in $requiredPackages) {
    if (-not (Test-Path $config)) {
        throw "CMake package not found at $config"
    }
}

$cmakeArgs = @(
    '-S', $cppSrc,
    '-B', $cppBuild,
    '-G', 'Visual Studio 18 2026',
    '-A', 'x64',
    "-DEXT_PATH_PREFIX=$cppExtPath",
    "-DCMAKE_INSTALL_PREFIX=$cppInstall",
    "-DCMAKE_PREFIX_PATH=$cppPrefixPath"
)
& cmake @cmakeArgs
if ($LASTEXITCODE -ne 0) { throw "CMake configure failed ($LASTEXITCODE)" }

& cmake --build $cppBuild --config $cppConfig --parallel
if ($LASTEXITCODE -ne 0) { throw "CMake build failed ($LASTEXITCODE)" }

if ($Test) {
    Push-Location $cppBuild
    try {
        & ctest -C $cppConfig --output-on-failure
        if ($LASTEXITCODE -ne 0) { throw "CTest failed ($LASTEXITCODE)" }
    }
    finally {
        Pop-Location
    }
}

# Assemble the deployable layout. Only build/deploy is durable; everything else
# under build/ is temporary. Each story that adds an executable or library
# extends this step. The release build is deployed so the display is a GUI
# application with no console window (see lil-d's windows_subsystem attribute).
if (-not $Test) {
    $deployRoot = Join-Path $repoRoot 'build/deploy'
    $deployRustBin = Join-Path $deployRoot 'rust/bin'
    $deployCppBin = Join-Path $deployRoot 'cpp/bin'
    $deployConfig = Join-Path $deployRoot 'config'
    $legacyDeployBin = Join-Path $deployRoot 'bin'
    if (Test-Path $legacyDeployBin) {
        Remove-Item -LiteralPath $legacyDeployBin -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $deployRustBin, $deployCppBin, $deployConfig | Out-Null
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\lil-d.exe') -Destination $deployRustBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\paws.exe') -Destination $deployRustBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\display_lib.dll') -Destination $deployRustBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\dds-echo.exe') -Destination $deployRustBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\hello-service.exe') -Destination $deployRustBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\hello-view-service.exe') -Destination $deployRustBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\hello-ctl-service.exe') -Destination $deployRustBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\format-ctl-service.exe') -Destination $deployRustBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\format-view-service.exe') -Destination $deployRustBin -Force

    # The C++ display loads its C++ display library and runtime DLLs from next to its executable.
    Copy-Item -Path (Join-Path $repoRoot 'build\cpp\Release\lil-d.exe') -Destination $deployCppBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\cpp\Release\hello-service.exe') -Destination $deployCppBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\cpp\Release\hello-view-service.exe') -Destination $deployCppBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\cpp\Release\hello-ctl-service.exe') -Destination $deployCppBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\cpp\Release\format-ctl-service.exe') -Destination $deployCppBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\cpp\Release\format-view-service.exe') -Destination $deployCppBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\cpp\Release\paws.exe') -Destination $deployCppBin -Force
    Copy-Item -Path (Join-Path $repoRoot 'build\cpp\Release\display_lib.dll') -Destination $deployCppBin -Force
    Copy-Item -Path (Join-Path $extPathPrefix 'harfbuzz/bin/harfbuzz.dll') -Destination $deployCppBin -Force
    Copy-Item -Path (Join-Path $extPathPrefix 'harfbuzz/bin/harfbuzz-gpu.dll') -Destination $deployCppBin -Force
    Copy-Item -Path (Join-Path $extPathPrefix 'cyclonedds/bin/ddsc.dll') -Destination $deployCppBin -Force
    Copy-Item -Path (Join-Path $extPathPrefix 'cyclonedds-cxx/bin/ddscxx.dll') -Destination $deployCppBin -Force

    # The display library loads its font and (later) the earth cube map from
    # <deploy>/assets, and links the HarfBuzz runtime from next to the exe.
    Copy-Item -Path (Join-Path $repoRoot 'assets') -Destination $deployRoot -Recurse -Force
    Copy-Item -Path (Join-Path $extPathPrefix 'harfbuzz\bin\harfbuzz.dll') -Destination $deployRustBin -Force
    Copy-Item -Path (Join-Path $extPathPrefix 'harfbuzz\bin\harfbuzz-gpu.dll') -Destination $deployRustBin -Force

    # The shared DDS configuration for the StationsDDS loopback adapter, and the
    # contracts the services and the display library read from <deploy>/config.
    Copy-Item -Path (Join-Path $repoRoot 'DDS\cyclonedds-config.xml') -Destination $deployConfig -Force
    Copy-Item -Path (Join-Path $repoRoot 'contracts\topics.toml') -Destination $deployConfig -Force
    Copy-Item -Path (Join-Path $repoRoot 'contracts\formats.toml') -Destination $deployConfig -Force
    Copy-Item -Path (Join-Path $repoRoot 'contracts\services') -Destination $deployConfig -Recurse -Force
    Write-Host "Deployed Rust artifacts to build/deploy/rust/bin and C++ artifacts to build/deploy/cpp/bin, with assets, runtime DLLs, and the DDS config + contracts"
}
