param(
    [int]$MistralPort = 8686,
    [int]$BackendPort = 7878,
    [int]$LabPort = 8787,
    [switch]$KeepMistral,
    [switch]$DryRun,
    [switch]$ForceAll,
    [switch]$Help
)

$ErrorActionPreference = "SilentlyContinue"

if ($Help) {
    Write-Host "Stop the local Gemma/rust-norion/rustgpt-lab test stack."
    Write-Host ""
    Write-Host "Default behavior stops only confirmed local test-stack processes on the configured ports."
    Write-Host "Backend must report rust-norion runtime_mode=gemma-http or built-in; Web Lab must point at that backend."
    Write-Host "The Gemma runtime port must be owned by a mistralrs process."
    Write-Host "Use -DryRun to inspect targets first. Use -ForceAll to stop all matching process names."
    Write-Host ""
    Write-Host "Port map:"
    Write-Host "  7878 = rust-norion backend; Web Lab forwards prompts there after gates."
    Write-Host "  8787 = rustgpt-lab browser UI and local SSE proxy."
    Write-Host "  8686 = optional Gemma/mistralrs runtime behind rust-norion; -KeepMistral leaves it running."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\rustgpt-lab\stop-gemma-lab.cmd -DryRun"
    Write-Host "  .\tools\rustgpt-lab\stop-gemma-lab.cmd -KeepMistral"
    Write-Host "  .\tools\rustgpt-lab\stop-gemma-lab.cmd -ForceAll"
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

    $connections = @(Get-NetTCPConnection -LocalAddress 127.0.0.1 -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
    $rows = @()
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
            WorkingSetGB = [math]::Round($process.WorkingSet64 / 1GB, 2)
            Path = $process.Path
            Process = $process
        }
    }
    return $rows
}

function Get-ExpectedPortOwnerProcesses {
    param(
        [string]$Component,
        [int]$Port,
        [string]$ExpectedProcessName
    )

    $rows = @(Get-PortOwnerProcesses -Component $Component -Port $Port)
    $matches = @()
    foreach ($row in $rows) {
        if ($row.ProcessName -eq $ExpectedProcessName) {
            $matches += $row
        } else {
            Write-Host "Skipping $Component port $Port owner pid=$($row.Id): process=$($row.ProcessName), expected=$ExpectedProcessName."
        }
    }
    return $matches
}

function Get-ForceAllProcesses {
    param([bool]$KeepMistral)

    $names = @("rustgpt-lab", "rust-norion")
    if (-not $KeepMistral) {
        $names += "mistralrs"
    }

    $rows = @()
    foreach ($name in $names) {
        $processes = @(Get-Process -Name $name -ErrorAction SilentlyContinue)
        foreach ($process in $processes) {
            $rows += [pscustomobject]@{
                Component = "force-all"
                Port = "any"
                Id = $process.Id
                ProcessName = $process.ProcessName
                WorkingSetGB = [math]::Round($process.WorkingSet64 / 1GB, 2)
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
        Write-Host "No confirmed local test-stack processes were found on the configured ports."
        return
    }

    Write-Host "Stop targets:"
    $Rows |
        Select-Object Component, Port, Id, ProcessName, WorkingSetGB, Path |
        Format-Table -AutoSize

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

$targets = @()
if ($ForceAll) {
    Write-Warning "DANGER: -ForceAll stops all matching rust-norion/rustgpt-lab/mistralrs process names, including processes outside the configured ports."
    $targets += Get-ForceAllProcesses -KeepMistral:$KeepMistral
} else {
    $backendHealth = Get-HealthJson -Url "http://127.0.0.1:$BackendPort/health"
    if ($backendHealth -and $backendHealth.service -eq "rust-norion" -and (@("gemma-http", "built-in") -contains $backendHealth.runtime_mode)) {
        $targets += Get-ExpectedPortOwnerProcesses -Component "rust-norion-$($backendHealth.runtime_mode)" -Port $BackendPort -ExpectedProcessName "rust-norion"
    } else {
        Write-Host "Skipping backend port ${BackendPort}: /health did not confirm rust-norion runtime_mode=gemma-http or built-in."
    }

    $expectedBackend = "127.0.0.1:$BackendPort"
    $labHealth = Get-HealthJson -Url "http://127.0.0.1:$LabPort/health"
    if ($labHealth -and $labHealth.service -eq "rustgpt-lab" -and $labHealth.backend -eq $expectedBackend) {
        $targets += Get-ExpectedPortOwnerProcesses -Component "rustgpt-lab" -Port $LabPort -ExpectedProcessName "rustgpt-lab"
    } else {
        Write-Host "Skipping Web Lab port ${LabPort}: /health did not confirm rustgpt-lab backend=$expectedBackend."
    }

    if (-not $KeepMistral) {
        $targets += Get-ExpectedPortOwnerProcesses -Component "mistralrs" -Port $MistralPort -ExpectedProcessName "mistralrs"
    }
}

$targets = @(Select-UniqueProcessRows -Rows $targets)
Stop-TargetRows -Rows $targets -DryRun:$DryRun

Start-Sleep -Seconds 1

$remaining = @(Get-Process -Name mistralrs,rust-norion,rustgpt-lab -ErrorAction SilentlyContinue)
if ($remaining.Count -gt 0) {
    Write-Host ""
    Write-Host "Remaining matching processes:"
    $remaining |
        Select-Object Id, ProcessName, CPU, @{Name = "PrivateGB"; Expression = { [math]::Round($_.PrivateMemorySize64 / 1GB, 2) } }, @{Name = "WorkingSetGB"; Expression = { [math]::Round($_.WorkingSet64 / 1GB, 2) } }, Path |
        Format-Table -AutoSize
    if (-not $ForceAll) {
        Write-Host "Use -ForceAll only if these are disposable local test-stack processes."
    }
} elseif (-not $DryRun) {
    Write-Host "Gemma/rustgpt-lab local test stack stopped."
}
