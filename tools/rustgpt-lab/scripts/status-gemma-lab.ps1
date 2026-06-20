param(
    [int]$MistralPort = 8686,
    [int]$BackendPort = 7878,
    [int]$LabPort = 8787,
    [switch]$Help
)

$ErrorActionPreference = "SilentlyContinue"

if ($Help) {
    Write-Host "Show the local Gemma/rust-norion/rustgpt-lab test-stack status."
    Write-Host ""
    Write-Host "This is read-only. It does not start Gemma, stop processes, or write .ndkv files."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\rustgpt-lab\status-gemma-lab.cmd"
    Write-Host "  .\tools\rustgpt-lab\status-gemma-lab.cmd -BackendPort 7878 -LabPort 8787"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -MistralPort <n>  optional Gemma/mistralrs runtime port, default 8686"
    Write-Host "  -BackendPort <n>  rust-norion model-service backend port, default 7878"
    Write-Host "  -LabPort <n>      rustgpt-lab Web UI/SSE proxy port, default 8787"
    Write-Host ""
    Write-Host "Port map:"
    Write-Host "  7878 = rust-norion backend; Web Lab forwards prompts there after gates."
    Write-Host "  8787 = rustgpt-lab browser UI and local SSE proxy."
    Write-Host "  8686 = optional Gemma/mistralrs runtime behind rust-norion; do not send prompts there directly."
    return
}

function Test-LocalPort {
    param([int]$Port)
    try {
        $client = [System.Net.Sockets.TcpClient]::new()
        $async = $client.BeginConnect("127.0.0.1", $Port, $null, $null)
        $ready = $async.AsyncWaitHandle.WaitOne(250)
        if (-not $ready) {
            $client.Close()
            return $false
        }
        $client.EndConnect($async)
        $client.Close()
        return $true
    } catch {
        return $false
    }
}

function Get-HealthJson {
    param([string]$Url)
    try {
        return Invoke-RestMethod -Uri $Url -TimeoutSec 2
    } catch {
        return $null
    }
}

Write-Host "Gemma lab status"
Write-Host ""

@(
    [pscustomobject]@{ Component = "mistralrs"; Port = $MistralPort; Listening = Test-LocalPort -Port $MistralPort },
    [pscustomobject]@{ Component = "rust-norion"; Port = $BackendPort; Listening = Test-LocalPort -Port $BackendPort },
    [pscustomobject]@{ Component = "rustgpt-lab"; Port = $LabPort; Listening = Test-LocalPort -Port $LabPort }
) | Format-Table -AutoSize

$processes = Get-Process -Name mistralrs,rust-norion,rustgpt-lab -ErrorAction SilentlyContinue
if ($processes) {
    Write-Host ""
    $processes |
        Select-Object Id, ProcessName, CPU, @{Name = "PrivateGB"; Expression = { [math]::Round($_.PrivateMemorySize64 / 1GB, 2) } }, @{Name = "WorkingSetGB"; Expression = { [math]::Round($_.WorkingSet64 / 1GB, 2) } } |
        Format-Table -AutoSize
} else {
    Write-Host ""
    Write-Host "No mistralrs/rust-norion/rustgpt-lab processes are running."
}

$backendHealth = Get-HealthJson -Url "http://127.0.0.1:$BackendPort/health"
if ($backendHealth) {
    Write-Host ""
    Write-Host "rust-norion /health:"
    $backendHealth | ConvertTo-Json -Compress -Depth 8
}

$labServiceHealth = Get-HealthJson -Url "http://127.0.0.1:$LabPort/health"
if ($labServiceHealth) {
    Write-Host ""
    Write-Host "rustgpt-lab /health:"
    $labServiceHealth | ConvertTo-Json -Compress -Depth 8
}

$labHealth = Get-HealthJson -Url "http://127.0.0.1:$LabPort/api/backend-health"
if ($labHealth) {
    Write-Host ""
    Write-Host "rustgpt-lab /api/backend-health:"
    $labHealth | ConvertTo-Json -Compress -Depth 8
}

$poolAdvice = Get-HealthJson -Url "http://127.0.0.1:$LabPort/api/model-pool-advice"
if ($poolAdvice) {
    Write-Host ""
    Write-Host "rustgpt-lab /api/model-pool-advice:"
    $poolAdvice | ConvertTo-Json -Compress -Depth 8
    if ($poolAdvice.advice) {
        Write-Host "Model pool advice: $($poolAdvice.advice)"
        Write-Host "Next step: $($poolAdvice.next_step) ($($poolAdvice.reason))"
    }
}

Write-Host ""
Write-Host "GPU summary:"
$nvidia = Get-Command nvidia-smi -ErrorAction SilentlyContinue
if ($null -eq $nvidia) {
    Write-Host "nvidia-smi: unavailable; GPU/VRAM usage cannot be read from this shell."
} else {
    & $nvidia.Source --query-gpu=name,memory.total,memory.used,utilization.gpu,power.draw,temperature.gpu --format=csv,noheader,nounits
}
