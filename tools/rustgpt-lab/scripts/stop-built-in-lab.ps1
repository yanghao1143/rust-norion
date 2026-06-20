param(
    [int]$BackendPort = 7878,
    [Alias("LabPort")]
    [int]$WebPort = 8787,
    [switch]$DryRun,
    [switch]$ForceAll,
    [switch]$Help
)

$ErrorActionPreference = "SilentlyContinue"

if ($Help) {
    Write-Host "Stop the built-in rust-norion + rustgpt-lab Web Lab."
    Write-Host ""
    Write-Host "Default behavior is conservative: it only stops local port owners that are confirmed"
    Write-Host "by /health as rust-norion runtime_mode=built-in or rustgpt-lab pointing at that backend."
    Write-Host "Use -DryRun to inspect targets first."
    Write-Host ""
    Write-Host "DANGER: -ForceAll stops every local process named rust-norion or rustgpt-lab, even if"
    Write-Host "it is not on the configured ports or cannot be proven to be this safe Web Lab."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\rustgpt-lab\stop-built-in-lab.cmd -DryRun"
    Write-Host "  .\tools\rustgpt-lab\stop-built-in-lab.cmd"
    Write-Host "  .\tools\rustgpt-lab\stop-built-in-lab.cmd -BackendPort 7878 -WebPort 8787"
    Write-Host "  .\tools\rustgpt-lab\stop-built-in-lab.cmd -ForceAll"
    Write-Host ""
    Write-Host "Port map:"
    Write-Host "  7878 = rust-norion built-in backend for safe local UI tests."
    Write-Host "  8787 = rustgpt-lab Web UI and local SSE proxy."
    Write-Host "  8686 = optional Gemma/mistralrs runtime; this built-in stop path does not target it."
    return
}

function Get-HealthJson {
    param([string]$Url)
    try {
        return Invoke-RestMethod -Uri $Url -TimeoutSec 2
    } catch {
        return $null
    }
}

function Get-PortOwnerProcesses {
    param(
        [string]$Component,
        [int]$Port
    )

    $rows = @()
    $connections = @(Get-NetTCPConnection -LocalAddress 127.0.0.1 -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
    foreach ($connection in $connections) {
        $process = Get-Process -Id $connection.OwningProcess -ErrorAction SilentlyContinue
        if ($null -eq $process) {
            continue
        }
        $rows += [pscustomobject]@{
            Component = $Component
            Port = $Port
            Id = $process.Id
            ProcessName = $process.ProcessName
            WorkingSetMB = [math]::Round($process.WorkingSet64 / 1MB, 1)
            Path = $process.Path
            Process = $process
        }
    }
    return $rows
}

function Get-ForceAllProcesses {
    $rows = @()
    foreach ($name in @("rustgpt-lab", "rust-norion")) {
        $processes = @(Get-Process -Name $name -ErrorAction SilentlyContinue)
        foreach ($process in $processes) {
            $rows += [pscustomobject]@{
                Component = "force-all"
                Port = "any"
                Id = $process.Id
                ProcessName = $process.ProcessName
                WorkingSetMB = [math]::Round($process.WorkingSet64 / 1MB, 1)
                Path = $process.Path
                Process = $process
            }
        }
    }
    return $rows
}

function Select-UniqueProcessRows {
    param([object[]]$Rows)

    $seen = @{}
    $unique = @()
    foreach ($row in $Rows) {
        if ($seen.ContainsKey($row.Id)) {
            continue
        }
        $seen[$row.Id] = $true
        $unique += $row
    }
    return $unique
}

function Stop-TargetRows {
    param(
        [object[]]$Rows,
        [bool]$DryRun
    )

    if ($Rows.Count -eq 0) {
        Write-Host "No confirmed built-in Web Lab processes were found on the configured ports."
        return
    }

    Write-Host "Stop targets:"
    $Rows | Select-Object Component, Port, Id, ProcessName, WorkingSetMB, Path | Format-Table -AutoSize

    if ($DryRun) {
        Write-Host "Dry run only; no processes were stopped."
        return
    }

    foreach ($row in $Rows) {
        Write-Host "Stopping $($row.ProcessName) pid=$($row.Id) component=$($row.Component) port=$($row.Port)"
        Stop-Process -Id $row.Id -ErrorAction SilentlyContinue
    }

    Start-Sleep -Seconds 2

    foreach ($row in $Rows) {
        $remaining = Get-Process -Id $row.Id -ErrorAction SilentlyContinue
        if ($remaining) {
            Write-Warning "Process pid=$($row.Id) is still running; forcing stop."
            Stop-Process -Id $row.Id -Force -ErrorAction SilentlyContinue
        }
    }
}

$expectedBackend = "127.0.0.1:$BackendPort"
$targets = @()

if ($ForceAll) {
    Write-Warning "DANGER: -ForceAll stops all rust-norion/rustgpt-lab processes by name, including processes outside ports $BackendPort/$WebPort."
    $targets += Get-ForceAllProcesses
} else {
    $backendHealth = Get-HealthJson -Url "http://127.0.0.1:$BackendPort/health"
    if ($backendHealth -and $backendHealth.service -eq "rust-norion" -and $backendHealth.runtime_mode -eq "built-in") {
        $targets += Get-PortOwnerProcesses -Component "rust-norion-built-in" -Port $BackendPort
    } else {
        Write-Host "Skipping backend port ${BackendPort}: /health did not confirm rust-norion runtime_mode=built-in."
    }

    $labHealth = Get-HealthJson -Url "http://127.0.0.1:$WebPort/health"
    if ($labHealth -and $labHealth.service -eq "rustgpt-lab" -and $labHealth.backend -eq $expectedBackend) {
        $targets += Get-PortOwnerProcesses -Component "rustgpt-lab" -Port $WebPort
    } else {
        Write-Host "Skipping Web port ${WebPort}: /health did not confirm rustgpt-lab backend=$expectedBackend."
    }
}

$targets = @(Select-UniqueProcessRows -Rows $targets)
$targets = @($targets | Sort-Object @{ Expression = { if ($_.Component -eq "rustgpt-lab") { 0 } else { 1 } } }, Id)
Stop-TargetRows -Rows $targets -DryRun:$DryRun

if (-not $DryRun) {
    Start-Sleep -Seconds 1
}

$remaining = @(Get-Process -Name rust-norion,rustgpt-lab -ErrorAction SilentlyContinue)
if ($remaining.Count -gt 0) {
    Write-Host ""
    Write-Host "Remaining rust-norion/rustgpt-lab processes:"
    $remaining |
        Select-Object Id, ProcessName, @{Name = "WorkingSetMB"; Expression = { [math]::Round($_.WorkingSet64 / 1MB, 1) } }, Path |
        Format-Table -AutoSize
    if (-not $ForceAll -and -not $DryRun) {
        Write-Host "Default stop leaves processes that are not confirmed as the safe built-in Web Lab."
    }
} elseif (-not $DryRun) {
    Write-Host "Built-in Web Lab stopped."
}
