<#
.SYNOPSIS
Launches the Rust configuration.

.DESCRIPTION
Starts the applications of the Rust configuration from build/deploy: the
hello_world model service, the view service, one format-manager service pair per
display, and the small displays (lil-d).

Every display has a unique display id. The same id is given to its format
controller, format view service, and lil-d.

All participants read the shared DDS configuration; this script sets
CYCLONEDDS_URI before starting them.

Run scripts/build.ps1 first to populate build/deploy.

.PARAMETER DisplayId
Display ids to launch. Defaults to two small displays, lil-d-a and lil-d-b.
#>
[CmdletBinding()]
param(
    [string[]]$DisplayId = @('lil-d-a', 'lil-d-b')
)

$ErrorActionPreference = 'Stop'

if ($DisplayId.Count -eq 0) {
    throw 'At least one display id is required.'
}

# Stop a previous run first: repeated launches must not stack services/displays
# (which doubles streams and DDS discovery).
& (Join-Path $PSScriptRoot 'cleanup.ps1')

$repoRoot = Split-Path -Parent $PSScriptRoot
$bin = Join-Path $repoRoot 'build\deploy\bin'
$config = Join-Path $repoRoot 'build\deploy\config\cyclonedds-config.xml'

$model = Join-Path $bin 'hello-service.exe'
$view = Join-Path $bin 'hello-view-service.exe'
$formatCtl = Join-Path $bin 'format-ctl-service.exe'
$formatView = Join-Path $bin 'format-view-service.exe'
$display = Join-Path $bin 'lil-d.exe'
if (-not (Test-Path $model) -or -not (Test-Path $view) -or -not (Test-Path $formatCtl) -or -not (Test-Path $formatView) -or -not (Test-Path $display) -or -not (Test-Path $config)) {
    throw "Not built: run scripts\build.ps1 first."
}

# All participants (the services and the display libraries) read the shared
# configuration from CYCLONEDDS_URI; child processes inherit it.
$env:CYCLONEDDS_URI = 'file://' + ($config -replace '\\', '/')

Start-Process -FilePath $model
Start-Sleep -Milliseconds 500
Start-Process -FilePath $view
Start-Sleep -Milliseconds 500

foreach ($id in $DisplayId) {
    Start-Process -FilePath $formatCtl -ArgumentList '--display-id', $id
    Start-Sleep -Milliseconds 300
    Start-Process -FilePath $formatView -ArgumentList '--display-id', $id
    Start-Sleep -Milliseconds 300
    Start-Process -FilePath $display -ArgumentList '--display-id', $id
    Start-Sleep -Milliseconds 300
}

Write-Host "Started hello-service, hello-view-service, and lil-d for display ids: $($DisplayId -join ', ')"
