param(
    [string]$BaseUrl = $env:NORION_NEWAPI_BASE_URL,
    [string]$Models = $env:NORION_NEWAPI_ALLOWED_MODELS,
    [string]$ModelsFile = "tools/evolution-loop/config/newapi-models.txt",
    [string]$OutcomeJsonl = $env:NORION_NEWAPI_MODEL_OUTCOMES_PATH,
    [string]$OutputJson = "target/evolution/newapi-live-smoke-real.json",
    [int]$TimeoutSecs = 120,
    [int]$MinModels = 2,
    [int]$MaxTokens = 128,
    [string]$Exe = "tools/evolution-loop/target/debug/evolution-loop.exe",
    [switch]$ForceAllModels
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($BaseUrl)) {
    throw "missing BaseUrl; set NORION_NEWAPI_BASE_URL or pass -BaseUrl"
}
if ([string]::IsNullOrWhiteSpace($Models) -and -not [string]::IsNullOrWhiteSpace($ModelsFile) -and (Test-Path -LiteralPath $ModelsFile)) {
    $Models = (Get-Content -LiteralPath $ModelsFile |
        ForEach-Object { $_.Trim() } |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and -not $_.StartsWith("#") }) -join ","
}
if ([string]::IsNullOrWhiteSpace($Models)) {
    throw "missing Models; set NORION_NEWAPI_ALLOWED_MODELS, pass -Models, or provide -ModelsFile"
}
if (-not (Test-Path -LiteralPath $Exe)) {
    throw "missing evolution-loop executable: $Exe"
}

$oldBaseUrl = $env:NORION_NEWAPI_BASE_URL
$oldApiKey = $env:NORION_NEWAPI_API_KEY
$oldModels = $env:NORION_NEWAPI_ALLOWED_MODELS
$oldOutcomes = $env:NORION_NEWAPI_MODEL_OUTCOMES_PATH

try {
    $env:NORION_NEWAPI_BASE_URL = $BaseUrl
    $env:NORION_NEWAPI_ALLOWED_MODELS = $Models
    if (-not [string]::IsNullOrWhiteSpace($OutcomeJsonl)) {
        $env:NORION_NEWAPI_MODEL_OUTCOMES_PATH = $OutcomeJsonl
    }

    if ([string]::IsNullOrWhiteSpace($env:NORION_NEWAPI_API_KEY)) {
        $secret = Read-Host "NORION_NEWAPI_API_KEY" -AsSecureString
        $env:NORION_NEWAPI_API_KEY = [System.Net.NetworkCredential]::new("", $secret).Password
    }
    if ([string]::IsNullOrWhiteSpace($env:NORION_NEWAPI_API_KEY)) {
        throw "missing NORION_NEWAPI_API_KEY"
    }

    $args = @(
        "--newapi-live-smoke",
        "--min-newapi-live-models", $MinModels,
        "--newapi-live-smoke-json", $OutputJson,
        "--timeout-secs", $TimeoutSecs,
        "--max-tokens", $MaxTokens
    )
    if ($ForceAllModels) {
        $args += "--force-newapi-live-smoke-all"
    }

    & $Exe @args
    if ($LASTEXITCODE -ne 0) {
        throw "evolution-loop exited with code $LASTEXITCODE"
    }
}
finally {
    $env:NORION_NEWAPI_BASE_URL = $oldBaseUrl
    $env:NORION_NEWAPI_API_KEY = $oldApiKey
    $env:NORION_NEWAPI_ALLOWED_MODELS = $oldModels
    $env:NORION_NEWAPI_MODEL_OUTCOMES_PATH = $oldOutcomes
}
