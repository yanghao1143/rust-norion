param(
    [string]$RepoRoot = "",
    [string]$SnapshotJson = "target\evolution\strict-status.json",
    [string]$SummaryJson = "target\evolution\strict-status-summary.json",
    [string]$Ledger = "",
    [string]$DaemonWorkDir = "target\evolution\daemon",
    [int]$MaxSnapshotAgeSeconds = 900,
    [switch]$SkipBackend,
    [switch]$SkipRemoteChain,
    [switch]$JsonStatus,
    [switch]$FailOnNotReady,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if ($RepoRoot.Trim().Length -eq 0) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
}

if ($Help) {
    Write-Host "Refresh SmartSteam strict unattended evolution snapshot and compact summary artifacts."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\evolution-loop\refresh-strict-status-artifacts.cmd [-JsonStatus] [-FailOnNotReady]"
    Write-Host "  .\tools\evolution-loop\refresh-strict-status-artifacts.cmd [-SnapshotJson PATH] [-SummaryJson PATH]"
    Write-Host ""
    Write-Host "Contracts:"
    Write-Host "  starts_process=false"
    Write-Host "  sends_prompt=false"
    exit 0
}

function Resolve-RepoPath {
    param([string]$Path)

    if ($Path.Trim().Length -eq 0) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return Join-Path $RepoRoot $Path
}

$statusScript = Join-Path $ScriptDir "status-evolution-loop.ps1"
$publishScript = Join-Path $ScriptDir "publish-strict-status-summary.ps1"
if (-not (Test-Path -LiteralPath $statusScript -PathType Leaf)) {
    throw "status-evolution-loop.ps1 not found: $statusScript"
}
if (-not (Test-Path -LiteralPath $publishScript -PathType Leaf)) {
    throw "publish-strict-status-summary.ps1 not found: $publishScript"
}

$snapshotPath = Resolve-RepoPath $SnapshotJson
$summaryPath = Resolve-RepoPath $SummaryJson
$snapshotParent = Split-Path -Parent $snapshotPath
if ($snapshotParent -and $snapshotParent.Trim().Length -gt 0) {
    New-Item -ItemType Directory -Force -Path $snapshotParent | Out-Null
}

$statusArgs = @(
    "-NoProfile", "-ExecutionPolicy", "Bypass",
    "-File", $statusScript,
    "-RepoRoot", $RepoRoot,
    "-JsonStatus",
    "-StrictUnattendedEvolution",
    "-FailOnNotReady",
    "-SkipProcess",
    "-DaemonWorkDir", $DaemonWorkDir
)
if ($Ledger.Trim().Length -gt 0) {
    $statusArgs += @("-Ledger", $Ledger)
}
if ($SkipBackend) {
    $statusArgs += "-SkipBackend"
}
if ($SkipRemoteChain) {
    $statusArgs += "-SkipRemoteChain"
}

$snapshotText = & powershell.exe @statusArgs
$snapshotExitCode = $LASTEXITCODE
($snapshotText | Out-String).TrimEnd() | Set-Content -Encoding ASCII -LiteralPath $snapshotPath

$summaryText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $publishScript -RepoRoot $RepoRoot -SnapshotJson $snapshotPath -OutJson $summaryPath -MaxSnapshotAgeSeconds $MaxSnapshotAgeSeconds -JsonStatus
$summaryExitCode = $LASTEXITCODE
$summary = $null
try {
    $summary = $summaryText | Out-String | ConvertFrom-Json
} catch {
    $summary = $null
}
$ready = $false
$failures = @()
if ($null -ne $summary -and $null -ne $summary.readiness) {
    $ready = [bool]$summary.readiness.ready
    $failures = @($summary.readiness.failures)
} else {
    $failures += "summary_parse_failed"
}
if ($snapshotExitCode -ne 0 -and $failures -notcontains "snapshot_status_not_ready") {
    $failures += "snapshot_status_not_ready"
}
if ($summaryExitCode -ne 0 -and $failures -notcontains "summary_publish_failed") {
    $failures += "summary_publish_failed"
}

$result = [pscustomobject][ordered]@{
    schema_version = 1
    contract_version = "smartsteam.evolution-loop.strict-status-artifacts-refresh.v1"
    starts_process = $false
    sends_prompt = $false
    snapshot_json = $snapshotPath
    summary_json = $summaryPath
    snapshot_exit_code = $snapshotExitCode
    summary_exit_code = $summaryExitCode
    readiness = [pscustomobject][ordered]@{
        ready = $ready -and $snapshotExitCode -eq 0 -and $summaryExitCode -eq 0
        failures = @($failures | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | Sort-Object -Unique)
    }
    summary = if ($null -ne $summary) { $summary.summary } else { $null }
    next_step = if ($ready -and $snapshotExitCode -eq 0 -and $summaryExitCode -eq 0) { "strict status artifacts refreshed" } else { "inspect strict status artifacts and refresh failures" }
}

$exitCode = if ($FailOnNotReady -and -not $result.readiness.ready) { 2 } else { 0 }
if ($JsonStatus) {
    $result | ConvertTo-Json -Depth 8
    exit $exitCode
}

Write-Host "strict_status_snapshot=$snapshotPath"
Write-Host "strict_status_summary=$summaryPath"
Write-Host "ready=$($result.readiness.ready) failures=$($result.readiness.failures -join ',') snapshot_exit_code=$snapshotExitCode summary_exit_code=$summaryExitCode"
if ($null -ne $result.summary) {
    Write-Host "summary: latest_round=$($result.summary.latest_round) active_round=$($result.summary.active_round) daemon_state=$($result.summary.daemon_state) next_round_decision=$($result.summary.next_round_decision_display_state) test_gate=$($result.summary.test_gate_verdict)"
}
exit $exitCode
