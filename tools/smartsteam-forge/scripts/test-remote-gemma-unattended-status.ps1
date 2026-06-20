param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate unattended remote Gemma status wrapper without starting processes or sending prompts."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\test-remote-gemma-unattended-status.cmd"
    return
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot not found: $RepoRoot"
}

$script = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\run-remote-gemma-unattended.ps1"
if (-not (Test-Path -LiteralPath $script -PathType Leaf)) {
    throw "run-remote-gemma-unattended.ps1 not found: $script"
}

$workDir = Join-Path $RepoRoot "target\remote-gemma-unattended\status-selftest"
if (Test-Path -LiteralPath $workDir) {
    Remove-Item -LiteralPath $workDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $workDir | Out-Null

$output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script `
    -JsonStatus `
    -DaemonWorkDir $workDir 2>&1
$exitCode = $LASTEXITCODE
$text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
if ($exitCode -ne 0) {
    throw "unattended status selftest failed with exit code ${exitCode}: $text"
}

try {
    $status = $text | ConvertFrom-Json
} catch {
    throw "unattended status selftest did not return JSON: $($_.Exception.Message); output=$text"
}

if ($status.schema -ne "smartsteam.forge.evolution_status.v1") {
    throw "unattended status schema mismatch: $($status.schema)"
}
if (-not $status.read_only -or $status.starts_process -or $status.sends_prompt) {
    throw "unattended status contract flags are wrong"
}
if ($status.evolution_status.daemon.read_only -ne $true) {
    throw "unattended status did not delegate to read-only daemon status"
}
if ($status.unattended_start_plan.starts_process -or $status.unattended_start_plan.sends_prompt) {
    throw "unattended start plan must be read-only"
}

Write-Host "smartsteam_remote_gemma_unattended_status_selftest=PASS"
Write-Host "read_only=$($status.read_only) starts_process=$($status.starts_process) sends_prompt=$($status.sends_prompt)"
Write-Host "daemon_running=$($status.evolution_status.daemon.running)"
Write-Host "can_start=$($status.unattended_start_plan.can_start)"
