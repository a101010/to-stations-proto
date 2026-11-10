<#
.SYNOPSIS
Launches the Rust configuration.

.DESCRIPTION
Starts the applications of the Rust configuration from build/deploy: the
hello_world model service, the view service, one format-manager service pair per
display, and the small displays (lil-d).

Every display has a unique numeric display id. The same id is given to its
format controller, format view service, and lil-d. After everything starts, the
lil-d windows are raised above the service console windows.

All participants read the shared DDS configuration; this script sets
CYCLONEDDS_URI before starting them.

Run scripts/build.ps1 first to populate build/deploy.

.PARAMETER DisplayId
Display ids to launch. Defaults to two small displays, ids 0 and 1.
#>
[CmdletBinding()]
param(
    [uint32[]]$DisplayId = @(0, 1)
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

$displayProcesses = @()
foreach ($id in $DisplayId) {
    Start-Process -FilePath $formatCtl -ArgumentList '--display-id', $id
    Start-Sleep -Milliseconds 300
    Start-Process -FilePath $formatView -ArgumentList '--display-id', $id
    Start-Sleep -Milliseconds 300
    $displayProcesses += Start-Process -FilePath $display -ArgumentList '--display-id', $id -PassThru
    Start-Sleep -Milliseconds 300
}

if (-not ('ToStations.WindowActivation' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;

namespace ToStations {
    public static class WindowActivation {
        [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
        [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
        [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
        [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
        [DllImport("user32.dll")] public static extern void SwitchToThisWindow(IntPtr hWnd, bool fUnknown);
    }
}
'@
}

function Show-DisplayWindow([System.Diagnostics.Process]$Process) {
    for ($attempt = 0; $attempt -lt 50; $attempt++) {
        $Process.Refresh()
        $hwnd = $Process.MainWindowHandle
        if ($hwnd -ne [IntPtr]::Zero) {
            $flags = 0x0002 -bor 0x0001 -bor 0x0040 # NOMOVE | NOSIZE | SHOWWINDOW
            [ToStations.WindowActivation]::ShowWindow($hwnd, 9) | Out-Null # SW_RESTORE
            [ToStations.WindowActivation]::SetWindowPos($hwnd, [IntPtr]::Zero, 0, 0, 0, 0, $flags) | Out-Null
            [ToStations.WindowActivation]::BringWindowToTop($hwnd) | Out-Null
            if (-not [ToStations.WindowActivation]::SetForegroundWindow($hwnd)) {
                [ToStations.WindowActivation]::SwitchToThisWindow($hwnd, $true)
            }
            return
        }
        Start-Sleep -Milliseconds 100
    }
}

foreach ($process in $displayProcesses) {
    Show-DisplayWindow $process
}

Write-Host "Started hello-service, hello-view-service, and lil-d for display ids: $($DisplayId -join ', ')"
