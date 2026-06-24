param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate NewAPI model discovery policy and tiering without network calls."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\evolution-loop\test-newapi-model-discovery.cmd"
    return
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot not found: $RepoRoot"
}

$script = Join-Path $RepoRoot "tools\evolution-loop\discover-newapi-models.ps1"
if (-not (Test-Path -LiteralPath $script -PathType Leaf)) {
    throw "discover-newapi-models.ps1 not found: $script"
}

$outDir = Join-Path $RepoRoot "target\evolution"
$json = Join-Path $outDir "newapi-model-discovery.selftest.json"
$markdown = Join-Path $outDir "newapi-model-matrix.selftest.md"

$previousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
try {
    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script -SelfTest -OutputJson $json -MatrixMarkdown $markdown 2>&1
    $exitCode = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $previousErrorActionPreference
}

$text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
if (-not [string]::IsNullOrWhiteSpace($text)) {
    Write-Host $text.TrimEnd()
}
if ($exitCode -ne 0) {
    throw "NewAPI discovery selftest failed with exit code $exitCode"
}
if ($text -notmatch "newapi_model_discovery_selftest=PASS") {
    throw "NewAPI discovery selftest did not report PASS"
}
if ($text -notmatch "api_key_written=false") {
    throw "NewAPI discovery selftest did not report api_key_written=false"
}
if (-not (Test-Path -LiteralPath $json -PathType Leaf)) {
    throw "NewAPI discovery selftest did not write JSON artifact"
}
if (-not (Test-Path -LiteralPath $markdown -PathType Leaf)) {
    throw "NewAPI discovery selftest did not write matrix markdown"
}

$report = Get-Content -LiteralPath $json -Raw | ConvertFrom-Json
foreach ($model in @("gpt-5", "openai/gpt-5.3", "gpt-6-preview")) {
    $failed = @($report.failed_models | Where-Object {
        $_.model -eq $model -and @($_.reasons) -contains "policy_excluded_gpt5_or_higher"
    })
    if ($failed.Count -ne 1) {
        throw "forbidden model missing from failed_models: $model"
    }
    $candidate = @($report.models | Where-Object {
        $_.model -eq $model -and $_.allowed_by_policy -eq $true
    })
    if ($candidate.Count -ne 0) {
        throw "forbidden model incorrectly allowed: $model"
    }
}

if (@($report.tiers.fast_router_reviewer) -notcontains "gpt-4.1-mini") {
    throw "fast_router_reviewer tier missing gpt-4.1-mini"
}
if (@($report.tiers.heavy_reasoning) -notcontains "deepseek-r1") {
    throw "heavy_reasoning tier missing deepseek-r1"
}
if (@($report.tiers.coding) -notcontains "qwen2.5-coder-32b-instruct") {
    throw "coding tier missing qwen2.5-coder-32b-instruct"
}
if (@($report.tiers.heavy_reasoning) -notcontains "qwen/qwen3.5-397b-a17b") {
    throw "heavy_reasoning tier missing qwen/qwen3.5-397b-a17b"
}
if (@($report.tiers.coding) -notcontains "qwen/qwen3.5-397b-a17b") {
    throw "coding tier missing qwen/qwen3.5-397b-a17b"
}
if (@($report.tiers.fast_router_reviewer) -contains "qwen/qwen3.5-397b-a17b") {
    throw "qwen/qwen3.5-397b-a17b must not be a fast_router_reviewer default"
}
if (@($report.tiers.fallback) -notcontains "moonshot-v1-8k") {
    throw "fallback tier missing moonshot-v1-8k"
}
$kimiFailure = @($report.failed_models | Where-Object {
    $_.model -eq "moonshotai/kimi-k2.6" -and @($_.reasons) -contains "manual_evidence_unstable_repetitive_output"
})
if ($kimiFailure.Count -ne 1) {
    throw "moonshotai/kimi-k2.6 should be marked unstable from manual evidence"
}

$markdownText = Get-Content -LiteralPath $markdown -Raw
$apiKeyAssignmentPattern = "NORION_NEWAPI_API_" + "KEY=.*"
if ($markdownText -match $apiKeyAssignmentPattern) {
    throw "matrix markdown appears to contain an API key assignment"
}
if ($markdownText -notmatch "policy_excluded_gpt5_or_higher") {
    throw "matrix markdown missing forbidden model reason"
}
if ($markdownText -notmatch "Rust coding probe succeeded") {
    throw "matrix markdown missing qwen/qwen3.5 coding evidence"
}

Write-Host "newapi_model_discovery_policy_selftest=PASS"
Write-Host "touches_network=false"
Write-Host "sends_prompt=false"
Write-Host "api_key_written=false"
