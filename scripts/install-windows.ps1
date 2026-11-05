# Run from an elevated PowerShell after unpacking the Windows build.
param([string]$Source = (Join-Path $PSScriptRoot '..'))
$ErrorActionPreference = 'Stop'
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { throw 'Run PowerShell as administrator once to install the background task and CPU sensor driver.' }
$dest = Join-Path $env:ProgramFiles 'W-Connect'
New-Item -ItemType Directory -Force $dest | Out-Null
$binary = Join-Path $Source 'w-connect.exe'
if (-not (Test-Path $binary)) { $binary = Join-Path $Source 'target/release/w-connect.exe' }
if (-not (Test-Path $binary)) { throw 'w-connect.exe was not found. Build with cargo build --release first.' }
$helper = Join-Path (Split-Path $binary) 'sensors'
if (-not (Test-Path (Join-Path $helper 'w-connect-sensors.exe'))) { throw 'Publish windows/Sensors first. Place its output in the sensors folder beside w-connect.exe.' }
Stop-ScheduledTask -TaskName 'W-Connect' -ErrorAction SilentlyContinue
Get-Process -Name 'w-connect','w-connect-sensors' -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path.StartsWith($dest + '\', [StringComparison]::OrdinalIgnoreCase) } | Wait-Process -Timeout 10
Copy-Item $binary $dest -Force
Copy-Item (Join-Path $helper '*') $dest -Force -Recurse
Copy-Item (Join-Path $Source 'assets/w-connect.ico') $dest -Force
$exe = Join-Path $dest 'w-connect.exe'
$action = New-ScheduledTaskAction -Execute $exe -WorkingDirectory $dest
$user = [Security.Principal.WindowsIdentity]::GetCurrent().Name
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $user
$principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Highest
$settings = New-ScheduledTaskSettingsSet -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) -ExecutionTimeLimit ([TimeSpan]::Zero) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
Register-ScheduledTask -TaskName 'W-Connect' -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force | Out-Null
Start-ScheduledTask -TaskName 'W-Connect'
$shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'W-Connect.url'
Set-Content $shortcut "[InternetShortcut]`r`nURL=http://127.0.0.1:7676`r`nIconFile=$dest\w-connect.ico`r`nIconIndex=0"
if (-not (Get-Service -Name 'PawnIO' -ErrorAction SilentlyContinue)) { Write-Warning 'CPU temperature requires PawnIO: install the signed driver from https://pawnio.eu/ . Missing CPU readings keep CPU-dependent curves at full speed.' }
Write-Host 'W-Connect installed. Open http://127.0.0.1:7676. Close L-Connect before use.'
