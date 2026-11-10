<#
.SYNOPSIS
Stops all configuration processes.

.DESCRIPTION
Stops the applications of every configuration by image name. The launch scripts
start them from build/deploy; this script stops all of them.

The list of applications grows as stories add executables; the Rust launch
script starts the hello_world model service, view service, one format-manager
service pair per display id, and one small display (lil-d) per display id. The
image-name stop below stops every instance.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

# Every application launched by a configuration launch script. Later stories
# append the services and the other displays as they are added.
$apps = @('lil-d', 'hello-service', 'hello-view-service', 'format-ctl-service', 'format-view-service')

foreach ($name in $apps) {
    $procs = Get-Process -Name $name -ErrorAction SilentlyContinue
    if ($procs) {
        $procs | Stop-Process -Force -ErrorAction SilentlyContinue
        Write-Host "Stopped $name"
    }
}
