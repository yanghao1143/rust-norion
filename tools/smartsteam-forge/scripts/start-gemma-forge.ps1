param(
    [string]$RepoRoot = "D:\rust-norion",
    [string]$Snapshot = "D:\hf-cache\hub\models--google--gemma-4-12B-it\snapshots\5926caa4ec0cac5cbfadaf4077420520de1d5205",
    [string]$HfCache = "D:\hf-cache",
    [int]$MistralPort = 8686,
    [int]$BackendPort = 7878,
    [int]$LabPort = 8787,
    [switch]$SkipBuild,
    [switch]$Force,
    [string]$StateDir = "",
    [switch]$UseProjectState,
    [switch]$KeepExistingBackend,
    [int]$TimeoutSecs = 900,
    [switch]$NoForge,
    [switch]$CheckOnly,
    [switch]$Help,
    [double]$MinFreeRamGB = 18.0,
    [double]$MinFreeGpuGB = 13.0
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Start Gemma 12B, rust-norion, rustgpt-lab, then SmartSteam Forge."
    Write-Host "Heavy path: this can start mistralrs/Gemma 12B. For safe built-in backend testing, use start-forge-stack.cmd."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\start-gemma-forge.cmd"
    Write-Host "  .\tools\smartsteam-forge\start-gemma-forge.cmd -CheckOnly"
    Write-Host "  .\tools\smartsteam-forge\start-gemma-forge.cmd -NoForge"
    Write-Host "  .\tools\smartsteam-forge\scripts\start-gemma-forge.ps1 -Force"
    Write-Host ""
    Write-Host "Common options:"
    Write-Host "  -CheckOnly            Inspect snapshot, ports, RAM/VRAM, and backend health; start nothing."
    Write-Host "  -NoForge              Start model/backend/web lab, then skip the TUI."
    Write-Host "  -SkipBuild            Reuse existing binaries."
    Write-Host "  -Force                Override RAM/VRAM startup preflight."
    Write-Host "  -StateDir <path>      Put rust-norion memory/experience/adaptive/trace files here; default is target\manual-gemma-service\forge-state."
    Write-Host "  -UseProjectState      Use repo-root .ndkv files instead of the default isolated state."
    Write-Host "  -KeepExistingBackend  Do not replace an occupied non-Gemma backend port."
    Write-Host "  -TimeoutSecs <n>      Forge total stream/request timeout; default 900."
    Write-Host "  -Snapshot <path>      Gemma snapshot directory."
    Write-Host "  -HfCache <path>       Hugging Face cache directory."
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

function Get-BackendHealth {
    param([int]$Port)
    try {
        return Invoke-RestMethod -Uri "http://127.0.0.1:$Port/health" -TimeoutSec 2
    } catch {
        return $null
    }
}

function Convert-ToFullPath {
    param(
        [string]$Path,
        [string]$BasePath
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }

    $candidate = $Path
    if (-not [System.IO.Path]::IsPathRooted($candidate)) {
        $candidate = Join-Path $BasePath $candidate
    }

    return [System.IO.Path]::GetFullPath($candidate)
}

function Get-HealthExperienceFile {
    param([object]$Health)

    if ($null -eq $Health) {
        return ""
    }

    $hygieneProperty = $Health.PSObject.Properties["experience_hygiene"]
    if ($null -eq $hygieneProperty -or $null -eq $hygieneProperty.Value) {
        return ""
    }

    $fileProperty = $hygieneProperty.Value.PSObject.Properties["experience_file"]
    if ($null -eq $fileProperty -or $null -eq $fileProperty.Value) {
        return ""
    }

    return [string]$fileProperty.Value
}

function Test-BackendStateMatches {
    param(
        [object]$Health,
        [string]$RepoRoot,
        [string]$ResolvedStateDir
    )

    if ([string]::IsNullOrWhiteSpace($ResolvedStateDir)) {
        return $true
    }

    $experienceFile = Get-HealthExperienceFile -Health $Health
    if ([string]::IsNullOrWhiteSpace($experienceFile)) {
        return $false
    }

    $actual = Convert-ToFullPath -Path $experienceFile -BasePath $RepoRoot
    $expected = Convert-ToFullPath -Path (Join-Path $ResolvedStateDir "experience.ndkv") -BasePath $RepoRoot
    return $actual -eq $expected
}

function Get-FreeRamGB {
    try {
        $os = Get-CimInstance -ClassName Win32_OperatingSystem
        if ($null -eq $os) {
            return $null
        }
        return [math]::Round($os.FreePhysicalMemory / 1MB, 2)
    } catch {
        return $null
    }
}

function Get-GpuMemoryInfo {
    try {
        $nvidia = Get-Command nvidia-smi -ErrorAction SilentlyContinue
        if ($null -eq $nvidia) {
            return @()
        }

        $lines = & $nvidia.Source --query-gpu=name,memory.total,memory.used,utilization.gpu,power.draw,temperature.gpu --format=csv,noheader,nounits 2>$null
        if (-not $lines) {
            return @()
        }

        $items = @()
        foreach ($line in $lines) {
            $parts = $line -split ","
            if ($parts.Count -lt 6) {
                continue
            }

            [double]$totalMB = 0
            [double]$usedMB = 0
            [double]$utilization = 0
            [double]$power = 0
            [double]$temperature = 0
            if (-not [double]::TryParse($parts[1].Trim(), [ref]$totalMB)) {
                continue
            }
            if (-not [double]::TryParse($parts[2].Trim(), [ref]$usedMB)) {
                continue
            }
            [void][double]::TryParse($parts[3].Trim(), [ref]$utilization)
            [void][double]::TryParse($parts[4].Trim(), [ref]$power)
            [void][double]::TryParse($parts[5].Trim(), [ref]$temperature)

            $items += [pscustomobject]@{
                Name = $parts[0].Trim()
                TotalGB = [math]::Round($totalMB / 1024, 2)
                UsedGB = [math]::Round($usedMB / 1024, 2)
                FreeGB = [math]::Round(($totalMB - $usedMB) / 1024, 2)
                Utilization = [math]::Round($utilization, 0)
                PowerW = [math]::Round($power, 2)
                TemperatureC = [math]::Round($temperature, 0)
            }
        }

        return $items
    } catch {
        return @()
    }
}

function Invoke-ForgeStartupCheck {
    param(
        [string]$RepoRoot,
        [string]$Snapshot,
        [string]$ResolvedStateDir,
        [int]$MistralPort,
        [int]$BackendPort,
        [int]$LabPort,
        [double]$MinFreeRamGB,
        [double]$MinFreeGpuGB,
        [bool]$KeepExistingBackend
    )

    $ok = $true
    Write-Host "SmartSteam Forge startup check"
    Write-Host ""

    if ([string]::IsNullOrWhiteSpace($ResolvedStateDir)) {
        Write-Host "state: project root .ndkv files (-UseProjectState)"
    } else {
        Write-Host "state: isolated $ResolvedStateDir"
        Write-Host "state_files: memory.ndkv experience.ndkv adaptive.ndkv trace-http-runtime-*.jsonl"
    }

    if (Test-Path -LiteralPath $Snapshot) {
        Write-Host "snapshot: OK $Snapshot"
    } else {
        Write-Warning "snapshot: missing $Snapshot"
        $ok = $false
    }

    $mistralListening = Test-LocalPort -Port $MistralPort
    $backendListening = Test-LocalPort -Port $BackendPort
    $labListening = Test-LocalPort -Port $LabPort
    Write-Host "ports: mistralrs:$MistralPort=$mistralListening rust-norion:$BackendPort=$backendListening rustgpt-lab:$LabPort=$labListening"

    if (-not $mistralListening) {
        $freeRamGB = Get-FreeRamGB
        if ($null -eq $freeRamGB) {
            Write-Warning "ram: unable to read free system RAM"
            $ok = $false
        } else {
            Write-Host ("ram: free={0:N1}GB minimum={1:N1}GB" -f $freeRamGB, $MinFreeRamGB)
            if ($freeRamGB -lt $MinFreeRamGB) {
                Write-Warning ("ram: below startup minimum by {0:N1}GB" -f ($MinFreeRamGB - $freeRamGB))
                $ok = $false
            }
        }

        $gpuInfo = @(Get-GpuMemoryInfo)
        if ($gpuInfo.Count -eq 0) {
            Write-Warning "gpu: nvidia-smi unavailable or returned no GPUs"
            $ok = $false
        } else {
            $bestGpu = $gpuInfo | Sort-Object FreeGB -Descending | Select-Object -First 1
            Write-Host ("gpu: best_free={0:N2}GB total={1:N2}GB used={2:N2}GB util={3}% temp={4}C name={5}" -f $bestGpu.FreeGB, $bestGpu.TotalGB, $bestGpu.UsedGB, $bestGpu.Utilization, $bestGpu.TemperatureC, $bestGpu.Name)
            if ($bestGpu.FreeGB -lt $MinFreeGpuGB) {
                Write-Warning ("gpu: below startup minimum by {0:N2}GB; close GPU-heavy apps or lower -MinFreeGpuGB only for tight VRAM tests" -f ($MinFreeGpuGB - $bestGpu.FreeGB))
                $ok = $false
            }
        }
    } else {
        Write-Host "resources: Gemma runtime is already listening; startup RAM/VRAM preflight would be skipped."
    }

    $health = Get-BackendHealth -Port $BackendPort
    if ($health) {
        $configured = Test-GemmaBackendConfig -Health $health -MistralPort $MistralPort
        $experienceFile = Get-HealthExperienceFile -Health $health
        $service = if ($health.PSObject.Properties["service"]) { $health.service } else { "unknown" }
        if (-not [string]::IsNullOrWhiteSpace($experienceFile)) {
            Write-Host "backend_state: experience_file=$experienceFile"
        }
        Write-Host "backend: reachable service=$service runtime_mode=$($health.runtime_mode) gemma_server=$($health.gemma_runtime_server) gemma_reachable=$($health.gemma_runtime_reachable) readiness_ok=$($health.readiness_ok) safe_device_ok=$($health.safe_device_ok)"
        if ($service -ne "rust-norion") {
            Write-Warning "backend: port $BackendPort returned /health JSON, but service is '$service' instead of rust-norion. Choose another -BackendPort or stop that service before starting Gemma."
            $ok = $false
        } elseif ($KeepExistingBackend -and -not $configured) {
            Write-Warning "backend: -KeepExistingBackend is set but port $BackendPort is not the expected Gemma backend"
            $ok = $false
        } elseif (-not $configured) {
            Write-Host "backend: existing rust-norion is not the expected Gemma backend; full start will replace rust-norion/rustgpt-lab unless -KeepExistingBackend is set."
        } else {
            Write-Host "backend: existing Gemma backend matches the expected runtime server."
        }
        if ($configured -and -not (Test-BackendStateMatches -Health $health -RepoRoot $RepoRoot -ResolvedStateDir $ResolvedStateDir)) {
            Write-Warning "backend: existing Gemma backend does not report the expected isolated experience file. Stop it or run without -KeepExistingBackend so the script can restart it with StateDir."
            $ok = $false
        }
    } elseif ($backendListening -and $KeepExistingBackend) {
        Write-Warning "backend: port $BackendPort is occupied but /health is not readable and -KeepExistingBackend is set"
        $ok = $false
    } elseif ($backendListening) {
        Write-Host "backend: port $BackendPort is occupied but /health is not readable; start script will replace rust-norion unless -KeepExistingBackend is set."
    } else {
        Write-Host "backend: not running; start script will launch rust-norion."
    }

    Write-Host ""
    if ($ok) {
        Write-Host "startup_check: PASS"
        Write-Host "Next: .\tools\smartsteam-forge\start-gemma-forge.cmd"
        return $true
    }

    Write-Warning "startup_check: FAIL"
    Write-Host "Fix the warnings above, then rerun -CheckOnly. Use -Force only when you intentionally accept the risk of CPU/disk fallback or tight VRAM."
    return $false
}

function Get-ExpectedPortOwnerProcesses {
    param(
        [string]$Component,
        [int]$Port,
        [string]$ExpectedProcessName
    )

    $rows = @()
    $connections = @(Get-NetTCPConnection -LocalAddress 127.0.0.1 -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
    foreach ($connection in $connections) {
        $process = Get-Process -Id $connection.OwningProcess -ErrorAction SilentlyContinue
        if ($null -eq $process) {
            continue
        }
        if ($process.ProcessName -ne $ExpectedProcessName) {
            Write-Warning "Not stopping $Component port $Port owner pid=$($process.Id): process=$($process.ProcessName), expected=$ExpectedProcessName."
            continue
        }

        $rows += [pscustomobject]@{
            Component = $Component
            Port = $Port
            Id = $process.Id
            ProcessName = $process.ProcessName
            WorkingSetGB = [math]::Round($process.WorkingSet64 / 1GB, 2)
        }
    }
    return $rows
}

function Stop-ExistingBackend {
    param(
        [int]$BackendPort,
        [int]$LabPort
    )

    $targets = @()
    $targets += Get-ExpectedPortOwnerProcesses -Component "rust-norion" -Port $BackendPort -ExpectedProcessName "rust-norion"
    $targets += Get-ExpectedPortOwnerProcesses -Component "rustgpt-lab" -Port $LabPort -ExpectedProcessName "rustgpt-lab"

    if ($targets.Count -eq 0) {
        Write-Host "No confirmed rust-norion/rustgpt-lab port owners were stopped."
        return
    }

    Write-Host "Stopping existing backend stack port owners:"
    $targets | Select-Object Component, Port, Id, ProcessName, WorkingSetGB | Format-Table -AutoSize
    foreach ($target in $targets) {
        Stop-Process -Id $target.Id -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Seconds 1
}

function Stop-StaleProcessForClosedPort {
    param(
        [string]$Name,
        [int]$Port
    )

    if (Test-LocalPort -Port $Port) {
        return
    }

    $processes = @(Get-Process -Name $Name -ErrorAction SilentlyContinue)
    if ($processes.Count -eq 0) {
        return
    }

    Write-Host "Stopping stale $Name process because port $Port is not listening."
    $processes | Stop-Process -Force
    Start-Sleep -Seconds 1
}

function Test-GemmaBackendConfig {
    param(
        [object]$Health,
        [int]$MistralPort
    )

    if ($null -eq $Health) {
        return $false
    }

    $expectedServer = "http://127.0.0.1:$MistralPort"
    return $Health.runtime_mode -eq "gemma-http" -and
        $Health.gemma_runtime_server -eq $expectedServer
}

function Resolve-GemmaForgeStateDir {
    param(
        [string]$RepoRoot,
        [string]$StateDir,
        [bool]$UseProjectState
    )

    if ($UseProjectState) {
        return ""
    }

    $resolvedStateDir = $StateDir
    if ([string]::IsNullOrWhiteSpace($resolvedStateDir)) {
        $resolvedStateDir = Join-Path $RepoRoot "target\manual-gemma-service\forge-state"
    } elseif (-not [System.IO.Path]::IsPathRooted($resolvedStateDir)) {
        $resolvedStateDir = Join-Path $RepoRoot $resolvedStateDir
    }

    return $resolvedStateDir
}

function Test-GemmaBackendReady {
    param(
        [object]$Health,
        [int]$MistralPort
    )

    if (-not (Test-GemmaBackendConfig -Health $Health -MistralPort $MistralPort)) {
        return $false
    }

    return $Health.gemma_runtime_reachable -eq $true
}

function Wait-GemmaBackendReady {
    param(
        [int]$BackendPort,
        [int]$MistralPort,
        [int]$TimeoutSeconds = 90
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $health = Get-BackendHealth -Port $BackendPort
        if (Test-GemmaBackendReady -Health $health -MistralPort $MistralPort) {
            return $health
        }
        Start-Sleep -Seconds 2
    }

    return Get-BackendHealth -Port $BackendPort
}

$startScript = Join-Path $RepoRoot "tools\rustgpt-lab\scripts\start-gemma-lab.ps1"
if (-not (Test-Path -LiteralPath $startScript)) {
    throw "Missing startup script: $startScript"
}

$forgeDir = Join-Path $RepoRoot "tools\smartsteam-forge"
if (-not (Test-Path -LiteralPath $forgeDir)) {
    throw "Missing SmartSteam Forge directory: $forgeDir"
}

if ($UseProjectState -and -not [string]::IsNullOrWhiteSpace($StateDir)) {
    throw "Use either -StateDir or -UseProjectState, not both."
}

$resolvedStateDir = Resolve-GemmaForgeStateDir -RepoRoot $RepoRoot -StateDir $StateDir -UseProjectState:$UseProjectState

if ($CheckOnly) {
    $ok = Invoke-ForgeStartupCheck `
        -RepoRoot $RepoRoot `
        -Snapshot $Snapshot `
        -ResolvedStateDir $resolvedStateDir `
        -MistralPort $MistralPort `
        -BackendPort $BackendPort `
        -LabPort $LabPort `
        -MinFreeRamGB $MinFreeRamGB `
        -MinFreeGpuGB $MinFreeGpuGB `
        -KeepExistingBackend:$KeepExistingBackend
    if ($ok) {
        exit 0
    }
    exit 1
}

Stop-StaleProcessForClosedPort -Name "mistralrs" -Port $MistralPort
Stop-StaleProcessForClosedPort -Name "rust-norion" -Port $BackendPort
Stop-StaleProcessForClosedPort -Name "rustgpt-lab" -Port $LabPort

$backendHealth = Get-BackendHealth -Port $BackendPort
$backendLooksConfigured = Test-GemmaBackendConfig -Health $backendHealth -MistralPort $MistralPort
$backendStateMatches = Test-BackendStateMatches -Health $backendHealth -RepoRoot $RepoRoot -ResolvedStateDir $resolvedStateDir

if ($backendHealth -and $backendHealth.PSObject.Properties["service"] -and $backendHealth.service -ne "rust-norion") {
    throw "Backend port $BackendPort returned /health service=$($backendHealth.service), not rust-norion. Stop that service or choose another -BackendPort."
}

if ($backendLooksConfigured -and -not $backendStateMatches -and (Test-LocalPort -Port $BackendPort)) {
    $experienceFile = Get-HealthExperienceFile -Health $backendHealth
    if ($KeepExistingBackend) {
        Write-Warning "Backend port $BackendPort is Gemma-ready but uses experience_file=$experienceFile instead of $resolvedStateDir. Keeping it because -KeepExistingBackend was set."
    } else {
        Write-Host "Replacing existing Gemma backend on port $BackendPort because its experience_file is not under the expected StateDir. current_experience_file=$experienceFile"
        Stop-ExistingBackend -BackendPort $BackendPort -LabPort $LabPort
    }
} elseif (-not $backendLooksConfigured -and (Test-LocalPort -Port $BackendPort)) {
    if ($KeepExistingBackend) {
        Write-Warning "Backend port $BackendPort is already occupied but is not the expected gemma-http backend. Keeping it because -KeepExistingBackend was set."
    } else {
        $mode = if ($backendHealth) { $backendHealth.runtime_mode } else { "unknown" }
        Write-Host "Replacing existing rust-norion backend on port $BackendPort; current runtime_mode=$mode"
        Stop-ExistingBackend -BackendPort $BackendPort -LabPort $LabPort
        if (Test-LocalPort -Port $BackendPort) {
            throw "Backend port $BackendPort is still occupied after stopping known rust-norion/rustgpt-lab processes. Stop that process or choose another -BackendPort."
        }
    }
}

& $startScript `
    -RepoRoot $RepoRoot `
    -Snapshot $Snapshot `
    -HfCache $HfCache `
    -MistralPort $MistralPort `
    -BackendPort $BackendPort `
    -LabPort $LabPort `
    -SkipBuild:$SkipBuild `
    -Force:$Force `
    -StateDir $resolvedStateDir `
    -MinFreeRamGB $MinFreeRamGB `
    -MinFreeGpuGB $MinFreeGpuGB

$health = Wait-GemmaBackendReady -BackendPort $BackendPort -MistralPort $MistralPort
if (-not (Test-GemmaBackendReady -Health $health -MistralPort $MistralPort)) {
    $mode = if ($health) { $health.runtime_mode } else { "unreachable" }
    $server = if ($health) { $health.gemma_runtime_server } else { "unknown" }
    $reachable = if ($health) { $health.gemma_runtime_reachable } else { "unknown" }
    throw "Backend is not ready for Gemma Forge testing. runtime_mode=$mode gemma_runtime_server=$server gemma_runtime_reachable=$reachable"
}

Write-Host ""
Write-Host "Gemma + rust-norion are ready."
Write-Host "Backend: http://127.0.0.1:$BackendPort"
Write-Host "Web lab: http://127.0.0.1:$LabPort"
if ([string]::IsNullOrWhiteSpace($resolvedStateDir)) {
    Write-Host "State: project root .ndkv files"
} else {
    Write-Host "State: $resolvedStateDir"
}

if ($NoForge) {
    Write-Host "Forge launch skipped because -NoForge was set."
    return
}

Write-Host "Starting SmartSteam Forge TUI..."
Push-Location $forgeDir
try {
    cargo run -- --backend "127.0.0.1:$BackendPort" --mode chat --require-health --require-safe-device --timeout-secs $TimeoutSecs
} finally {
    Pop-Location
}
