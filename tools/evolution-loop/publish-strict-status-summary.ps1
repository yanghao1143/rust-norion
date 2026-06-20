param(
    [string]$RepoRoot = "",
    [string]$SnapshotJson = "target\evolution\strict-status.json",
    [string]$OutJson = "target\evolution\strict-status-summary.json",
    [int]$MaxSnapshotAgeSeconds = 900,
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
    Write-Host "Publish a compact SmartSteam strict unattended evolution status summary artifact."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\evolution-loop\publish-strict-status-summary.cmd [-JsonStatus] [-FailOnNotReady]"
    Write-Host "  .\tools\evolution-loop\publish-strict-status-summary.cmd [-SnapshotJson PATH] [-OutJson PATH]"
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

$verifyScript = Join-Path $ScriptDir "verify-strict-status-snapshot.ps1"
if (-not (Test-Path -LiteralPath $verifyScript -PathType Leaf)) {
    throw "verify-strict-status-snapshot.ps1 not found: $verifyScript"
}

$snapshotPath = Resolve-RepoPath $SnapshotJson
$outPath = Resolve-RepoPath $OutJson
$verifyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $verifyScript -RepoRoot $RepoRoot -SnapshotJson $snapshotPath -MaxSnapshotAgeSeconds $MaxSnapshotAgeSeconds -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "strict status snapshot verifier failed unexpectedly with exit code $LASTEXITCODE"
}
$verification = $verifyText | Out-String | ConvertFrom-Json
$summary = $verification.summary

$published = [pscustomobject][ordered]@{
    schema_version = 1
    contract_version = "smartsteam.evolution-loop.strict-status-summary.v1"
    starts_process = $false
    sends_prompt = $false
    source_snapshot = $verification.snapshot.path
    snapshot_age_seconds = $verification.snapshot.age_seconds
    max_snapshot_age_seconds = $verification.snapshot.max_age_seconds
    readiness = $verification.readiness
    summary = $summary
    next_step = $verification.next_step
}

$parent = Split-Path -Parent $outPath
if ($parent -and $parent.Trim().Length -gt 0) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}
$published | ConvertTo-Json -Depth 8 | Set-Content -Encoding ASCII -LiteralPath $outPath

$exitCode = if ($FailOnNotReady -and -not $published.readiness.ready) { 2 } else { 0 }
if ($JsonStatus) {
    $published | ConvertTo-Json -Depth 8
    exit $exitCode
}

Write-Host "strict_status_summary=$outPath"
Write-Host "ready=$($published.readiness.ready) failures=$($published.readiness.failures -join ',') latest_round=$($summary.latest_round) active_round=$($summary.active_round) daemon_state=$($summary.daemon_state) next_round_decision=$($summary.next_round_decision_display_state) test_gate=$($summary.test_gate_verdict)"
exit $exitCode
