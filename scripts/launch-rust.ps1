<#
.SYNOPSIS
Launches the Rust configuration.

.DESCRIPTION
Starts the applications of the Rust configuration from build/deploy: the
hello_world view service and the small display (lil-d). Later stories add the
model service and the other displays.

All participants read the shared DDS configuration; this script sets
CYCLONEDDS_URI before starting them.

Run scripts/build.ps1 first to populate build/deploy.

.PARAMETER Station
Station number passed to the display. Defaults to 0.
#>
[CmdletBinding()]
param(
    [int]$Station = 0
)

$ErrorActionPreference = 'Stop'

# Stop a previous run first: repeated launches must not stack services/displays
# (which doubles streams and DDS discovery).
& (Join-Path $PSScriptRoot 'cleanup.ps1')

$repoRoot = Split-Path -Parent $PSScriptRoot
$bin = Join-Path $repoRoot 'build\deploy\bin'
$config = Join-Path $repoRoot 'build\deploy\config\cyclonedds-config.xml'

$service = Join-Path $bin 'hello-view-service.exe'
$display = Join-Path $bin 'lil-d.exe'
if (-not (Test-Path $service) -or -not (Test-Path $display) -or -not (Test-Path $config)) {
    throw "Not built: run scripts\build.ps1 first."
}

# All participants (the service and the display library) read the shared
# configuration from CYCLONEDDS_URI; child processes inherit it.
$env:CYCLONEDDS_URI = 'file://' + ($config -replace '\\', '/')

Start-Process -FilePath $service
Start-Sleep -Milliseconds 500
Start-Process -FilePath $display -ArgumentList '--station', $Station
Write-Host "Started hello-view-service and lil-d (station $Station)"
