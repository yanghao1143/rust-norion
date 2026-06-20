param(
    [int]$BackendPort = 7878,
    [Alias("LabPort")]
    [int]$WebPort = 8787,
    [switch]$Help
)

$ErrorActionPreference = "SilentlyContinue"

if ($Help) {
    Write-Host "Show built-in rust-norion + rustgpt-lab Web Lab status."
    Write-Host ""
    Write-Host "This is read-only. It does not start Gemma, stop processes, or write .ndkv files."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\rustgpt-lab\status-built-in-lab.cmd"
    Write-Host "  .\tools\rustgpt-lab\status-built-in-lab.cmd -BackendPort 7878 -WebPort 8787"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -BackendPort <n>  rust-norion service port, default 7878"
    Write-Host "  -WebPort <n>      rustgpt-lab Web UI port, default 8787"
    Write-Host ""
    Write-Host "Port map:"
    Write-Host "  7878 = rust-norion built-in backend for safe local UI tests."
    Write-Host "  8787 = rustgpt-lab Web UI and local SSE proxy."
    Write-Host "  8686 = optional Gemma/mistralrs runtime; this built-in status path does not query it."
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

function Get-PortOwnerRows {
    param(
        [string]$Component,
        [int]$Port
    )

    $rows = @()
    $connections = @(Get-NetTCPConnection -LocalAddress 127.0.0.1 -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
    foreach ($connection in $connections) {
        $process = Get-Process -Id $connection.OwningProcess -ErrorAction SilentlyContinue
        $rows += [pscustomobject]@{
            Component = $Component
            Port = $Port
            Id = $connection.OwningProcess
            ProcessName = if ($process) { $process.ProcessName } else { "<unknown>" }
            WorkingSetMB = if ($process) { [math]::Round($process.WorkingSet64 / 1MB, 1) } else { $null }
            Path = if ($process) { $process.Path } else { "" }
        }
    }
    return $rows
}

Write-Host "Built-in Web Lab status"
Write-Host ""
Write-Host "Read-only check for rust-norion built-in backend + rustgpt-lab Web UI."
Write-Host "No Gemma/mistralrs process is started or queried."
Write-Host ""

$backendListening = Test-LocalPort -Port $BackendPort
$webListening = Test-LocalPort -Port $WebPort

@(
    [pscustomobject]@{ Component = "rust-norion"; Port = $BackendPort; Listening = $backendListening },
    [pscustomobject]@{ Component = "rustgpt-lab"; Port = $WebPort; Listening = $webListening }
) | Format-Table -AutoSize

$owners = @()
$owners += Get-PortOwnerRows -Component "rust-norion" -Port $BackendPort
$owners += Get-PortOwnerRows -Component "rustgpt-lab" -Port $WebPort
if ($owners.Count -gt 0) {
    Write-Host ""
    Write-Host "Port owners:"
    $owners | Select-Object Component, Port, Id, ProcessName, WorkingSetMB, Path | Format-Table -AutoSize
}

$backendHealth = Get-HealthJson -Url "http://127.0.0.1:$BackendPort/health"
if ($backendHealth) {
    Write-Host ""
    Write-Host "rust-norion /health:"
    $backendHealth | ConvertTo-Json -Compress -Depth 8

    if ($backendHealth.service -eq "rust-norion" -and $backendHealth.runtime_mode -eq "built-in") {
        Write-Host "Backend safety: confirmed rust-norion runtime_mode=built-in."
    } elseif ($backendHealth.service -eq "rust-norion") {
        Write-Warning "Backend safety: rust-norion is listening, but runtime_mode is '$($backendHealth.runtime_mode)' instead of built-in."
    } else {
        Write-Warning "Backend safety: port $BackendPort did not report service=rust-norion."
    }
} elseif ($backendListening) {
    Write-Warning "Backend port $BackendPort is listening, but /health did not return JSON."
}

$labHealth = Get-HealthJson -Url "http://127.0.0.1:$WebPort/health"
if ($labHealth) {
    Write-Host ""
    Write-Host "rustgpt-lab /health:"
    $labHealth | ConvertTo-Json -Compress -Depth 8

    $expectedBackend = "127.0.0.1:$BackendPort"
    if ($labHealth.service -eq "rustgpt-lab" -and $labHealth.backend -eq $expectedBackend) {
        Write-Host "Web Lab safety: confirmed rustgpt-lab backend=$expectedBackend."
    } elseif ($labHealth.service -eq "rustgpt-lab") {
        Write-Warning "Web Lab safety: rustgpt-lab is listening, but backend is '$($labHealth.backend)' instead of '$expectedBackend'."
    } else {
        Write-Warning "Web Lab safety: port $WebPort did not report service=rustgpt-lab."
    }
} elseif ($webListening) {
    Write-Warning "Web port $WebPort is listening, but /health did not return JSON."
}

$labBackendHealth = Get-HealthJson -Url "http://127.0.0.1:$WebPort/api/backend-health"
if ($labBackendHealth) {
    Write-Host ""
    Write-Host "rustgpt-lab /api/backend-health:"
    $labBackendHealth | ConvertTo-Json -Compress -Depth 8
}

$poolAdvice = Get-HealthJson -Url "http://127.0.0.1:$WebPort/api/model-pool-advice"
if ($poolAdvice) {
    Write-Host ""
    Write-Host "rustgpt-lab /api/model-pool-advice:"
    $poolAdvice | ConvertTo-Json -Compress -Depth 8
    if ($poolAdvice.advice) {
        Write-Host "Model pool advice: $($poolAdvice.advice)"
        Write-Host "Next step: $($poolAdvice.next_step) ($($poolAdvice.reason))"
    }
}
