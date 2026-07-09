param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate SmartSteam remote model-cache sync provenance mode without copying or downloading models."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\scripts\test-sync-remote-gemma-model-cache.ps1"
    return
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot not found: $RepoRoot"
}

$script = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\sync-remote-gemma-model-cache.ps1"
if (-not (Test-Path -LiteralPath $script -PathType Leaf)) {
    throw "sync-remote-gemma-model-cache.ps1 not found: $script"
}

$workDir = Join-Path $RepoRoot "target\remote-gemma-chain\sync-selftest"
$localDir = Join-Path $workDir "models"
$outputJson = Join-Path $workDir "model-cache-status.json"
$stdoutJson = Join-Path $workDir "stdout.json"

if (Test-Path -LiteralPath $workDir) {
    Remove-Item -LiteralPath $workDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $localDir | Out-Null

$modelNames = @(
    "quality-test.gguf",
    "gemma-3-270m-it-qat-Q4_0.gguf",
    "gemma-4-E4B-it-Q4_K_M.gguf",
    "functiongemma-270m-it-Q4_K_M.gguf",
    "gemma-4-E2B-it-Q4_K_M.gguf"
)
foreach ($name in $modelNames) {
    Set-Content -Encoding ASCII -LiteralPath (Join-Path $localDir $name) -Value "smartsteam-sync-selftest:$name"
}

$previousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script `
        -CheckOnly `
        -JsonStatus `
        -OutputJson $outputJson `
        -LocalModelDir $localDir `
        -QualityModelPath (Join-Path $localDir "quality-test.gguf") `
        -RemoteHost "127.0.0.1" `
        -RemoteUser "nobody" `
        -IdentityFile (Join-Path $workDir "missing-test-key") `
        -RemoteModelDir "/tmp/smartsteam-sync-selftest" 2>&1
    $exitCode = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $previousErrorActionPreference
}
$text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
Set-Content -Encoding UTF8 -LiteralPath $stdoutJson -Value $text

if ($exitCode -ne 0) {
    throw "sync provenance selftest command failed with exit code $exitCode`: $text"
}
if (-not (Test-Path -LiteralPath $outputJson -PathType Leaf)) {
    throw "sync provenance selftest did not write OutputJson: $outputJson"
}

$status = Get-Content -Raw -LiteralPath $outputJson | ConvertFrom-Json
$models = @($status.models)
if (-not $status.read_only -or $status.starts_process -or $status.sends_prompt) {
    throw "sync provenance selftest contract flags are wrong"
}
if ($status.copy_allowed -or $status.download_allowed -or $status.copies_files -or $status.downloads_files) {
    throw "sync provenance selftest unexpectedly allowed or performed copy/download"
}
if (-not $status.writes_files) {
    throw "sync provenance selftest should report writes_files=true because OutputJson was requested"
}
if ($models.Count -ne 5) {
    throw "sync provenance selftest expected 5 model rows, got $($models.Count)"
}
if (@($models | Where-Object { -not $_.local_exists -or [string]::IsNullOrWhiteSpace($_.local_sha256) }).Count -ne 0) {
    throw "sync provenance selftest missing local sha256 evidence"
}
if (@($models | Where-Object { [string]::IsNullOrWhiteSpace($_.remote_error) }).Count -ne 0) {
    throw "sync provenance selftest expected remote errors for unreachable localhost SSH"
}
if ($status.all_ok) {
    throw "sync provenance selftest should not report all_ok when remote metadata is unreachable"
}

$fakeSshDir = Join-Path $workDir "fake-ssh"
New-Item -ItemType Directory -Force -Path $fakeSshDir | Out-Null
Set-Content -Encoding ASCII -LiteralPath (Join-Path $fakeSshDir "ssh.cmd") -Value @"
@echo off
echo 4096	remoteonlysha
"@
$remoteOnlyOutputJson = Join-Path $workDir "remote-only-model-cache-status.json"
$missingLocalDir = Join-Path $workDir "missing-local-models"
$oldPath = $env:PATH
$env:PATH = "$fakeSshDir;$oldPath"
try {
    $remoteOnlyOutput = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script `
        -CheckOnly `
        -JsonStatus `
        -OutputJson $remoteOnlyOutputJson `
        -LocalModelDir $missingLocalDir `
        -QualityModelPath (Join-Path $missingLocalDir "quality-test.gguf") `
        -RemoteHost "fake-remote" `
        -RemoteUser "nobody" `
        -IdentityFile (Join-Path $workDir "missing-test-key") `
        -RemoteModelDir "/tmp/smartsteam-sync-selftest" 2>&1
    $remoteOnlyExitCode = $LASTEXITCODE
} finally {
    $env:PATH = $oldPath
}
if ($remoteOnlyExitCode -ne 0) {
    throw "remote-only check-only selftest failed with exit code $remoteOnlyExitCode`: $($remoteOnlyOutput -join "`n")"
}
$remoteOnlyStatus = Get-Content -Raw -LiteralPath $remoteOnlyOutputJson | ConvertFrom-Json
$remoteOnlyModels = @($remoteOnlyStatus.models)
if (-not $remoteOnlyStatus.all_ok) {
    throw "remote-only check-only selftest should pass when every remote model exists"
}
if (@($remoteOnlyModels | Where-Object { $_.local_exists -or -not $_.remote_exists -or -not $_.remote_only_check_ok -or -not $_.ok }).Count -ne 0) {
    throw "remote-only check-only rows did not use remote existence evidence"
}

Write-Host "smartsteam_remote_model_cache_sync_selftest=PASS"
Write-Host "read_only=$($status.read_only) starts_process=$($status.starts_process) sends_prompt=$($status.sends_prompt)"
Write-Host "writes_files=$($status.writes_files) copy_allowed=$($status.copy_allowed) download_allowed=$($status.download_allowed) copies_files=$($status.copies_files) downloads_files=$($status.downloads_files)"
Write-Host "models=$($models.Count) local_hashes=$(@($models | Where-Object { $_.local_sha256 }).Count) remote_errors=$(@($models | Where-Object { $_.remote_error }).Count)"
Write-Host "remote_only_check_only_models=$($remoteOnlyModels.Count) all_ok=$($remoteOnlyStatus.all_ok)"
