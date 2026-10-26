<#
.SYNOPSIS
Stops all configuration processes.

.DESCRIPTION
Stops the applications of every configuration by image name. The launch scripts
start them from build/deploy; this script stops all of them.

The list of applications grows as stories add executables; the Rust launch
script starts the hello_world view service and the small display (lil-d).
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

# Every application launched by a configuration launch script. Later stories
# append the services and the other displays as they are added.
$apps = @('lil-d', 'hello-view-service')

foreach ($name in $apps) {
    $procs = Get-Process -Name $name -ErrorAction SilentlyContinue
    if ($procs) {
        $procs | Stop-Process -Force -ErrorAction SilentlyContinue
        Write-Host "Stopped $name"
    }
}
