param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate remote Gemma chain status JSON without SSH, process launch, or prompt sending."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\test-remote-gemma-chain-status.cmd"
    return
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot not found: $RepoRoot"
}

$chainScript = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\start-remote-gemma-chain.ps1"
if (-not (Test-Path -LiteralPath $chainScript -PathType Leaf)) {
    throw "start-remote-gemma-chain.ps1 not found: $chainScript"
}

$workDir = Join-Path $RepoRoot "target\remote-gemma-chain\status-selftest"
$modelCacheStatus = Join-Path $workDir "model-cache-status.json"
$outputJson = Join-Path $workDir "status-with-model-cache.json"
if (Test-Path -LiteralPath $workDir) {
    Remove-Item -LiteralPath $workDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $workDir | Out-Null

$modelCacheJson = @'
{
  "schema_version": 1,
  "contract_version": "smartsteam.remote-model-cache-sync.v1",
  "read_only": true,
  "all_ok": true,
  "remote": {
    "host": "192.168.10.11",
    "user": "xinghuan",
    "model_dir": "/Users/xinghuan/smartsteam-model-box/models"
  },
  "models": [
    {
      "role": "summary",
      "name": "gemma-3-270m-it-qat-Q4_0.gguf",
      "ok": true,
      "local_bytes": 241410624,
      "remote_bytes": 241410624,
      "size_matches": true,
      "sha256_matches": true,
      "local_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "remote_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "remote_path": "/Users/xinghuan/smartsteam-model-box/models/gemma-3-270m-it-qat-Q4_0.gguf",
      "remote_error": ""
    },
    {
      "role": "router",
      "name": "functiongemma-270m-it-Q4_K_M.gguf",
      "ok": true,
      "local_bytes": 253127904,
      "remote_bytes": 253127904,
      "size_matches": true,
      "sha256_matches": true,
      "local_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "remote_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "remote_path": "/Users/xinghuan/smartsteam-model-box/models/functiongemma-270m-it-Q4_K_M.gguf",
      "remote_error": ""
    }
  ]
}
'@
Set-Content -Encoding UTF8 -LiteralPath $modelCacheStatus -Value $modelCacheJson

$output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $chainScript `
    -Status `
    -JsonStatus `
    -RunDir $workDir `
    -ModelCacheStatusJson $modelCacheStatus `
    -OutputJson $outputJson `
    -LocalModelPort 65501 `
    -BackendPort 65502 `
    -LabPort 65503 `
    -RemoteModelPort 65504
$exitCode = $LASTEXITCODE
if ($exitCode -ne 0) {
    throw "status JSON selftest command failed with exit code $exitCode"
}

$status = ($output | Out-String) | ConvertFrom-Json
if (-not (Test-Path -LiteralPath $outputJson -PathType Leaf)) {
    throw "status JSON selftest did not write OutputJson"
}
$statusFromFile = Get-Content -LiteralPath $outputJson -Raw | ConvertFrom-Json
if (-not $status.read_only -or $status.starts_process -or $status.sends_prompt -or $status.touches_remote) {
    throw "status JSON selftest contract flags are wrong"
}
if ($statusFromFile.contract_version -ne $status.contract_version) {
    throw "status JSON selftest OutputJson did not match stdout contract"
}
if (-not $status.remote_probe_skipped) {
    throw "status JSON selftest should skip remote probing"
}
if ($status.touches_remote -ne $false) {
    throw "status JSON selftest should not touch remote by default"
}
if ($status.remote_runtime.probed -ne $false) {
    throw "status JSON selftest should not probe remote runtime by default"
}
if (-not $status.model_cache.exists -or -not $status.model_cache.all_ok) {
    throw "status JSON selftest did not include a passing model_cache summary"
}
if ($status.readiness.model_cache_all_ok -ne $true) {
    throw "status JSON selftest did not project model_cache_all_ok into readiness"
}
if ([int]$status.model_cache.model_count -ne 2 -or [int]$status.model_cache.ok_count -ne 2) {
    throw "status JSON selftest model_cache counts are wrong"
}
if ($status.model_cache.models[0].role -ne "summary") {
    throw "status JSON selftest model_cache rows were not preserved"
}

$failOutputJson = Join-Path $workDir "status-fail-on-not-ready.json"
$failOutput = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $chainScript `
    -Status `
    -JsonStatus `
    -RunDir $workDir `
    -ModelCacheStatusJson $modelCacheStatus `
    -OutputJson $failOutputJson `
    -LocalModelPort 65501 `
    -BackendPort 65502 `
    -LabPort 65503 `
    -RemoteModelPort 65504 `
    -FailOnNotReady
$failExitCode = $LASTEXITCODE
if ($failExitCode -eq 0) {
    throw "status JSON FailOnNotReady should exit nonzero when readiness is false"
}
$failStatus = ($failOutput | Out-String) | ConvertFrom-Json
if (-not (Test-Path -LiteralPath $failOutputJson -PathType Leaf)) {
    throw "status JSON FailOnNotReady did not write OutputJson before nonzero exit"
}
if ($failStatus.readiness.ready -ne $false) {
    throw "status JSON FailOnNotReady fixture should be not ready"
}
if (-not $failStatus.read_only -or $failStatus.starts_process -or $failStatus.sends_prompt -or $failStatus.touches_remote) {
    throw "status JSON FailOnNotReady broke status contract flags"
}
if (-not $failStatus.remote_probe_skipped -or $failStatus.remote_runtime.probed -ne $false) {
    throw "status JSON FailOnNotReady should not probe remote by default"
}

Write-Host "smartsteam_remote_gemma_chain_status_selftest=PASS"
Write-Host "read_only=$($status.read_only) starts_process=$($status.starts_process) sends_prompt=$($status.sends_prompt) touches_remote=$($status.touches_remote)"
Write-Host "model_cache all_ok=$($status.model_cache.all_ok) ok=$($status.model_cache.ok_count)/$($status.model_cache.model_count)"
