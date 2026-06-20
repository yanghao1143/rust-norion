param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate SmartSteam remote Gemma local-backend reload command without stopping or starting processes."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\test-remote-gemma-backend-reload.cmd"
    return
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot not found: $RepoRoot"
}

$reloadScript = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\reload-remote-gemma-backend.ps1"
if (-not (Test-Path -LiteralPath $reloadScript -PathType Leaf)) {
    throw "reload-remote-gemma-backend.ps1 not found: $reloadScript"
}

function Assert-Contains {
    param(
        [string]$Text,
        [string]$Pattern
    )
    if ($Text -notmatch [regex]::Escape($Pattern)) {
        throw "expected output did not contain: $Pattern"
    }
}

$testRunDir = Join-Path $RepoRoot "target\smartsteam-backend-reload-tests\missing-backend"
New-Item -ItemType Directory -Force -Path $testRunDir | Out-Null
$output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $reloadScript `
    -RepoRoot $RepoRoot `
    -RunDir $testRunDir `
    -BackendPort 17979 `
    -LocalModelPort 18686 `
    -NoModelPoolManifest `
    -CheckOnly 2>&1
$exitCode = $LASTEXITCODE
$text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
if (-not [string]::IsNullOrWhiteSpace($text)) {
    Write-Host $text.TrimEnd()
}
if ($exitCode -ne 0) {
    throw "backend reload CheckOnly failed with exit code $exitCode"
}

Assert-Contains -Text $text -Pattern "SmartSteam remote Gemma backend reload preflight: PASS"
Assert-Contains -Text $text -Pattern "check_only=true"
Assert-Contains -Text $text -Pattern "touches_remote=false"
Assert-Contains -Text $text -Pattern "stops_remote=false"
Assert-Contains -Text $text -Pattern "stops_tunnel=false"
Assert-Contains -Text $text -Pattern "stops_web_lab=false"
Assert-Contains -Text $text -Pattern "stops_backend=none"
Assert-Contains -Text $text -Pattern "starts_process=false"
Assert-Contains -Text $text -Pattern "sends_prompt=false"
Assert-Contains -Text $text -Pattern "model_pool_manifest=disabled"

Write-Host "remote_gemma_backend_reload_selftest=PASS"
Write-Host "touches_remote=false"
Write-Host "starts_process=false"
Write-Host "stops_process=false"
Write-Host "sends_prompt=false"
