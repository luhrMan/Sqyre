# Launch smoke test for a built sqyre.exe on a real Windows desktop session.
#
# Checks: --version / --help exit cleanly, the GUI starts and reports the
# Windows backends (GDI capture, input, low-level hotkey hook), it keeps
# running, and a second launch exits via the single-instance lock.
# Uses the real user profile (%APPDATA%\sqyre, ~\.sqyre): run on CI / VMs only.
param(
    [string]$Exe = "target\debug\sqyre.exe",
    [int]$StartupSeconds = 90,
    [int]$AliveSeconds = 15
)
$ErrorActionPreference = "Stop"

function Fail([string]$msg) {
    Write-Host "::error::smoke: $msg"
    exit 1
}

$Exe = (Resolve-Path $Exe).Path
$tmp = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$out = Join-Path $tmp "sqyre-smoke"
New-Item -ItemType Directory -Force -Path $out | Out-Null

$version = (& $Exe --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $version -notmatch '^\d+\.\d+') { Fail "--version printed '$version' (exit $LASTEXITCODE)" }
Write-Host "version: $version"
$help = (& $Exe --help | Out-String)
if ($LASTEXITCODE -ne 0 -or $help -notmatch 'Usage: sqyre') { Fail "--help output unexpected: $help" }

# Fresh profile: skip the first-start permissions popup, which holds the
# capture/input/hotkey probes back until confirmed.
$dataDir = Join-Path $env:USERPROFILE ".sqyre"
$settingsFile = Join-Path $dataDir "settings.yaml"
if (-not (Test-Path $settingsFile)) {
    New-Item -ItemType Directory -Force -Path $dataDir | Out-Null
    Set-Content -Path $settingsFile -Value "permissions_intro_done: true" -Encoding ascii
}

$log1 = Join-Path $out "first.log"
$app = Start-Process -FilePath $Exe -PassThru -NoNewWindow `
    -RedirectStandardError $log1 -RedirectStandardOutput (Join-Path $out "first.out")
$null = $app.Handle  # cache the handle so ExitCode is readable after exit

$required = @(
    'SQYRE_SESSION type=windows',
    'SQYRE_CAP=ok backend=gdi',
    'SQYRE_INPUT=ok',
    'SQYRE_HOTKEY=ok backend=win32-llhook'
)
$deadline = (Get-Date).AddSeconds($StartupSeconds)
do {
    Start-Sleep -Seconds 1
    $text = if (Test-Path $log1) { Get-Content -Raw $log1 } else { "" }
    if ($null -eq $text) { $text = "" }
    $missing = @($required | Where-Object { -not $text.Contains($_) })
    if ($app.HasExited) { Get-Content $log1; Fail "app exited during startup (code $($app.ExitCode))" }
} while ($missing.Count -gt 0 -and (Get-Date) -lt $deadline)
if ($missing.Count -gt 0) { Get-Content $log1; Stop-Process -Id $app.Id -Force; Fail "missing log lines: $($missing -join ', ')" }

Start-Sleep -Seconds $AliveSeconds
if ($app.HasExited) { Get-Content $log1; Fail "app exited after startup (code $($app.ExitCode))" }

$log2 = Join-Path $out "second.log"
$second = Start-Process -FilePath $Exe -PassThru -NoNewWindow `
    -RedirectStandardError $log2 -RedirectStandardOutput (Join-Path $out "second.out")
$null = $second.Handle
if (-not $second.WaitForExit(30000)) { Stop-Process -Id $second.Id -Force; Stop-Process -Id $app.Id -Force; Fail "second instance did not exit" }
$text2 = Get-Content -Raw $log2
if ($second.ExitCode -ne 0 -or -not "$text2".Contains("already running")) {
    Stop-Process -Id $app.Id -Force
    Fail "second instance: exit $($second.ExitCode), log: $text2"
}
if ($app.HasExited) { Fail "first instance died when the second launched" }

Stop-Process -Id $app.Id -Force
Get-Content $log1
if (-not (Test-Path (Join-Path $dataDir "db.yaml"))) { Fail "db.yaml was not created in $dataDir" }
Write-Host "smoke: ok"
