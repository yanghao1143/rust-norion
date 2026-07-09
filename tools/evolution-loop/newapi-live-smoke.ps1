param(
    [string]$BaseUrl = $env:NORION_NEWAPI_BASE_URL,
    [string]$Models = $env:NORION_NEWAPI_ALLOWED_MODELS,
    [string]$OutputJson = "target/evolution/newapi-live-smoke-real.json",
    [int]$TimeoutSecs = 120,
    [int]$MinModels = 2,
    [string]$Exe = "tools/evolution-loop/target/debug/evolution-loop.exe"
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($BaseUrl)) {
    throw "missing BaseUrl; set NORION_NEWAPI_BASE_URL or pass -BaseUrl"
}
if ([string]::IsNullOrWhiteSpace($Models)) {
    throw "missing Models; set NORION_NEWAPI_ALLOWED_MODELS or pass -Models"
}
if (-not (Test-Path -LiteralPath $Exe)) {
    throw "missing evolution-loop executable: $Exe"
}

$oldBaseUrl = $env:NORION_NEWAPI_BASE_URL
$oldApiKey = $env:NORION_NEWAPI_API_KEY
$oldModels = $env:NORION_NEWAPI_ALLOWED_MODELS

try {
    $env:NORION_NEWAPI_BASE_URL = $BaseUrl
    $env:NORION_NEWAPI_ALLOWED_MODELS = $Models

    if ([string]::IsNullOrWhiteSpace($env:NORION_NEWAPI_API_KEY)) {
        $secret = Read-Host "NORION_NEWAPI_API_KEY" -AsSecureString
        $env:NORION_NEWAPI_API_KEY = [System.Net.NetworkCredential]::new("", $secret).Password
    }
    if ([string]::IsNullOrWhiteSpace($env:NORION_NEWAPI_API_KEY)) {
        throw "missing NORION_NEWAPI_API_KEY"
    }

    & $Exe `
        --newapi-live-smoke `
        --min-newapi-live-models $MinModels `
        --newapi-live-smoke-json $OutputJson `
        --timeout-secs $TimeoutSecs
    if ($LASTEXITCODE -ne 0) {
        throw "evolution-loop exited with code $LASTEXITCODE"
    }
}
finally {
    $env:NORION_NEWAPI_BASE_URL = $oldBaseUrl
    $env:NORION_NEWAPI_API_KEY = $oldApiKey
    $env:NORION_NEWAPI_ALLOWED_MODELS = $oldModels
}
