# Measures the Windows build: working set, private bytes and CPU time, first for five idle minutes in the tray and then
# with the settings window open. Run it with the application already built:
#
#   powershell -File docs\windows\measure-windows.ps1
#
# The numbers are written next to this script (measure-windows-idle.txt, measure-windows-window.txt).
param(
    [string]$Exe = "$PSScriptRoot\..\..\windows\target\release\titan-control.exe",
    [int]$Minutes = 5
)

$exe = (Resolve-Path $Exe).Path
$docs = $PSScriptRoot
$utf8 = New-Object System.Text.UTF8Encoding($false)
$code = @'
using System;
using System.Runtime.InteropServices;
public class TrayControl {
  [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, int msg, IntPtr w, IntPtr l);
}
'@
Add-Type -TypeDefinition $code

function Stop-App {
    Get-Process titan-control -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 800
}

function Read-App {
    Get-Process titan-control -ErrorAction SilentlyContinue | Select-Object -First 1
}

Write-Host "== idle in the tray =="
Stop-App
Start-Process -FilePath $exe -ArgumentList "--minimized"
Start-Sleep -Seconds 40   # the start-up scan of the monitor plus the one-shot working-set trim 30 s after it

$process = Read-App
if (-not $process) { throw "the application did not stay running" }
$start = Get-Date
$cpuStart = $process.TotalProcessorTime.TotalSeconds
$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("# Titan Control (Windows) - idle in the tray")
$lines.Add("")
$lines.Add("sampled: $($start.ToString('yyyy-MM-dd HH:mm:ss'))")
$lines.Add("binary:  $((Get-Item $exe).Length) bytes")
$lines.Add("")
$lines.Add("| minute | working set (MB) | private bytes (MB) | CPU time (s) | threads |")
$lines.Add("| --- | --- | --- | --- | --- |")
$peakWorking = 0.0
$peakPrivate = 0.0
for ($minute = 0; $minute -le $Minutes; $minute++) {
    if ($minute -gt 0) { Start-Sleep -Seconds 60 }
    $current = Read-App
    if (-not $current) { $lines.Add("| $minute | process exited | | | |"); break }
    $working = [math]::Round($current.WorkingSet64 / 1MB, 2)
    $private = [math]::Round($current.PrivateMemorySize64 / 1MB, 2)
    $peakWorking = [math]::Max($peakWorking, $working)
    $peakPrivate = [math]::Max($peakPrivate, $private)
    $lines.Add("| $minute | $working | $private | $([math]::Round($current.TotalProcessorTime.TotalSeconds - $cpuStart, 3)) | $($current.Threads.Count) |")
}
$end = Read-App
if ($end) {
    $cpu = [math]::Round($end.TotalProcessorTime.TotalSeconds - $cpuStart, 3)
    $lines.Add("")
    $lines.Add("peak working set: $peakWorking MB")
    $lines.Add("peak private bytes: $peakPrivate MB")
    $lines.Add("CPU time over ${Minutes} min: $cpu s = $([math]::Round($cpu / ($Minutes * 60) * 100, 4)) % of one core")
    $lines.Add("threads: $($end.Threads.Count)")
}
[IO.File]::WriteAllLines((Join-Path $docs "measure-windows-idle.txt"), $lines, $utf8)
$lines | ForEach-Object { Write-Host $_ }

Write-Host ""
Write-Host "== settings window open =="
Stop-App
Start-Process -FilePath $exe
Start-Sleep -Seconds 40   # start-up scan and the idle trim

$process = Read-App
$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("# Titan Control (Windows) - settings window open, idle")
$lines.Add("")
$lines.Add("sampled: $((Get-Date).ToString('yyyy-MM-dd HH:mm:ss'))")
$lines.Add("")
$lines.Add("| sample | working set (MB) | private bytes (MB) | threads |")
$lines.Add("| --- | --- | --- | --- |")
for ($index = 0; $index -lt 6; $index++) {
    if ($index -gt 0) { Start-Sleep -Seconds 10 }
    $current = Read-App
    if (-not $current) { $lines.Add("| $index | process exited | | |"); break }
    $lines.Add("| $index | $([math]::Round($current.WorkingSet64 / 1MB, 2)) | $([math]::Round($current.PrivateMemorySize64 / 1MB, 2)) | $($current.Threads.Count) |")
}
[IO.File]::WriteAllLines((Join-Path $docs "measure-windows-window.txt"), $lines, $utf8)
$lines | ForEach-Object { Write-Host $_ }

Write-Host ""
Write-Host "The window is still open; quit the application from its tray menu when you are done."
