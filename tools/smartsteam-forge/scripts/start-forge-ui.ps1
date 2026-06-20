param(
    [string]$Backend = "127.0.0.1:7878",
    [string]$Mode = "chat",
    [int]$TimeoutSecs = 900,
    [switch]$AllowBuiltIn,
    [switch]$NoSafeDevice,
    [switch]$WaitReady,
    [int]$ReadyTimeoutSecs = 300,
    [switch]$CheckOnly,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Start SmartSteam Forge TUI against an already running rust-norion backend."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\start-forge-ui.cmd"
    Write-Host "  .\tools\smartsteam-forge\start-forge-ui.cmd -AllowBuiltIn"
    Write-Host "  .\tools\smartsteam-forge\start-forge-ui.cmd -Mode business-cycle"
    Write-Host "  .\tools\smartsteam-forge\start-forge-ui.cmd -Backend 127.0.0.1:7878"
    Write-Host "  .\tools\smartsteam-forge\start-forge-ui.cmd -WaitReady"
    Write-Host "  .\tools\smartsteam-forge\start-forge-ui.cmd -CheckOnly"
    Write-Host ""
    Write-Host "If the backend is not running, start the safe built-in stack first:"
    Write-Host "  .\tools\smartsteam-forge\start-forge-stack.cmd"
    Write-Host ""
    Write-Host "For explicit Gemma 12B full-stack startup, use:"
    Write-Host "  .\tools\smartsteam-forge\start-gemma-forge.cmd"
    return
}

function Get-BackendBaseUrl {
    param([string]$Backend)
    if ($Backend.StartsWith("http://") -or $Backend.StartsWith("https://")) {
        return $Backend.TrimEnd("/")
    }
    return "http://$Backend"
}

function Get-BackendHealth {
    param([string]$BaseUrl)
    try {
        return Invoke-RestMethod -Uri "$BaseUrl/health" -TimeoutSec 2
    } catch {
        return $null
    }
}

function Join-HealthFailures {
    param([object]$Failures)
    $items = @($Failures) | Where-Object { $_ }
    if ($items.Count -eq 0) {
        return "none"
    }
    return $items -join "; "
}

function Get-OrUnknown {
    param([object]$Value)
    if ($null -ne $Value) {
        return $Value
    }
    return "unknown"
}

function Get-HealthBusyDetail {
    param([object]$Health)
    if ($null -eq $Health) {
        return "active=unknown busy=unknown"
    }

    $active = Get-OrUnknown -Value $Health.active_engine_requests
    $busy = Get-OrUnknown -Value $Health.engine_busy
    $requests = @($Health.active_requests) | Where-Object { $_ }
    if ($requests.Count -eq 0) {
        return "active=$active busy=$busy"
    }

    $first = $requests | Select-Object -First 1
    $requestId = Get-OrUnknown -Value $first.request_id
    $endpoint = Get-OrUnknown -Value $first.endpoint
    $elapsed = Get-OrUnknown -Value $first.elapsed_ms
    $preview = Get-OrUnknown -Value $first.prompt_preview
    return "active=$active busy=$busy request_id=$requestId endpoint=$endpoint elapsed_ms=$elapsed prompt_preview=$preview"
}

function Get-HealthDeviceDetail {
    param([object]$Health)
    if ($null -eq $Health) {
        return "device=unknown lane=unknown memory=unknown accelerators=unknown"
    }

    $device = Get-OrUnknown -Value $Health.device_profile
    $lane = Get-OrUnknown -Value $Health.device_primary_lane
    $memory = Get-OrUnknown -Value $Health.device_memory_mode
    $accelerators = Get-OrUnknown -Value $Health.device_accelerators
    return "device=$device lane=$lane memory=$memory accelerators=$accelerators"
}

function Test-PositiveNumber {
    param([object]$Value)
    if ($null -eq $Value) {
        return $false
    }

    [long]$number = 0
    if (-not [long]::TryParse($Value.ToString(), [ref]$number)) {
        return $false
    }
    return $number -gt 0
}

function Get-ExperienceHygieneProblem {
    param([object]$Health)
    if ($null -eq $Health -or $null -eq $Health.experience_hygiene) {
        return $null
    }

    $hygiene = $Health.experience_hygiene
    if (Test-PositiveNumber -Value $hygiene.quarantine_candidates) {
        return "experience_hygiene quarantine_candidates=$($hygiene.quarantine_candidates). Run Forge --audit or inspect /v1/experience-hygiene before sending prompts."
    }

    if ($null -ne $hygiene.repair -and (Test-PositiveNumber -Value $hygiene.repair.repairable_legacy_metadata_lessons)) {
        return "experience_repair repairable_legacy_metadata_lessons=$($hygiene.repair.repairable_legacy_metadata_lessons). Run repair dry-run before sending prompts."
    }

    if ($hygiene.clean -eq $false) {
        $findings = if ($null -ne $hygiene.findings) { $hygiene.findings } else { "unknown" }
        return "experience_hygiene clean=false findings=$findings. Inspect /v1/experience-hygiene before sending prompts."
    }

    return $null
}

function Get-ForgeDiagnosticCommands {
    param(
        [string]$BaseUrl,
        [string]$Backend
    )
    $backendPort = Get-BackendPort -Backend $Backend
    $statusCommand = if ($backendPort -eq 7979) {
        ".\tools\smartsteam-forge\status-gemma-forge.cmd"
    } else {
        ".\tools\smartsteam-forge\status-forge.cmd -BackendPort $backendPort"
    }
    return "Read-only diagnostics: curl.exe -s $BaseUrl/health; $statusCommand; cargo run -- --backend $Backend --connect-timeout-ms 500 --read-timeout-ms 500 --doctor; nvidia-smi. Note: --read-timeout-ms is the per-read poll/heartbeat interval; use --timeout-secs for the total Gemma stream/request window."
}

function Get-BackendPort {
    param([string]$Backend)
    try {
        $uri = [System.Uri](Get-BackendBaseUrl -Backend $Backend)
        return $uri.Port
    } catch {
        return 7878
    }
}

function Test-ForgeReadiness {
    param(
        [object]$Health,
        [bool]$RequireSafeDevice
    )

    if ($null -eq $Health) {
        return $false
    }

    if ($Health.readiness_ok -eq $false) {
        return $false
    }

    if ($null -ne (Get-ExperienceHygieneProblem -Health $Health)) {
        return $false
    }

    if ($RequireSafeDevice -and $Health.safe_device_ok -eq $false) {
        return $false
    }

    return $true
}

function Write-ReadinessError {
    param(
        [object]$Health,
        [bool]$RequireSafeDevice,
        [string]$BaseUrl,
        [string]$Backend
    )

    if ($Health.readiness_ok -eq $false) {
        $failures = Join-HealthFailures -Failures $Health.readiness_failures
        $busy = Get-HealthBusyDetail -Health $Health
        $commands = Get-ForgeDiagnosticCommands -BaseUrl $BaseUrl -Backend $Backend
        Write-Error "Backend is not ready: $failures. $busy. Use -WaitReady to wait for it to become idle. $commands"
        return
    }

    $hygieneProblem = Get-ExperienceHygieneProblem -Health $Health
    if ($null -ne $hygieneProblem) {
        $commands = Get-ForgeDiagnosticCommands -BaseUrl $BaseUrl -Backend $Backend
        Write-Error "Experience hygiene guard failed: $hygieneProblem $commands"
        return
    }

    if ($RequireSafeDevice -and $Health.safe_device_ok -eq $false) {
        $failures = Join-HealthFailures -Failures $Health.safe_device_failures
        $device = Get-HealthDeviceDetail -Health $Health
        $commands = Get-ForgeDiagnosticCommands -BaseUrl $BaseUrl -Backend $Backend
        Write-Error "safe-device failed: $failures. $device. Gemma 12B is not GPU-first; use -NoSafeDevice only for tiny CPU fallback tests. $commands"
        return
    }
}

function Wait-ForgeReadiness {
    param(
        [string]$BaseUrl,
        [bool]$RequireSafeDevice,
        [int]$TimeoutSeconds
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $health = Get-BackendHealth -BaseUrl $BaseUrl
        if (Test-ForgeReadiness -Health $health -RequireSafeDevice $RequireSafeDevice) {
            return $health
        }

        if ($health) {
            $busy = Get-HealthBusyDetail -Health $health
            Write-Host "Waiting for backend readiness... $busy"
        } else {
            Write-Host "Waiting for backend readiness... backend unreachable"
        }
        Start-Sleep -Seconds 2
    }

    return Get-BackendHealth -BaseUrl $BaseUrl
}

$baseUrl = Get-BackendBaseUrl -Backend $Backend
$health = Get-BackendHealth -BaseUrl $baseUrl
if ($null -eq $health) {
    $commands = Get-ForgeDiagnosticCommands -BaseUrl $baseUrl -Backend $Backend
    Write-Error "Backend is not reachable at $baseUrl. cargo run -- --backend $Backend only starts the Forge client; it does not start the backend or model. $commands. For safe built-in backend testing, start another window with: .\tools\smartsteam-forge\start-forge-stack.cmd"
    exit 1
}

if (-not $AllowBuiltIn -and $health.runtime_mode -ne "gemma-http") {
    $device = Get-HealthDeviceDetail -Health $health
    Write-Error "Backend is reachable but is not Gemma HTTP runtime. runtime_mode=$($health.runtime_mode) $device. Pass -AllowBuiltIn for safe built-in backend testing; use -CheckOnly/--doctor before real 12B testing."
    exit 1
}

if ($health.gemma_runtime_server -and $health.gemma_runtime_reachable -eq $false) {
    $commands = Get-ForgeDiagnosticCommands -BaseUrl $baseUrl -Backend $Backend
    Write-Error "Gemma runtime is configured but not reachable: $($health.gemma_runtime_server). Forge did not send a prompt. $commands"
    exit 1
}

if ($WaitReady) {
    $health = Wait-ForgeReadiness -BaseUrl $baseUrl -RequireSafeDevice (-not $NoSafeDevice) -TimeoutSeconds $ReadyTimeoutSecs
}

if (-not (Test-ForgeReadiness -Health $health -RequireSafeDevice (-not $NoSafeDevice))) {
    Write-ReadinessError -Health $health -RequireSafeDevice (-not $NoSafeDevice) -BaseUrl $baseUrl -Backend $Backend
    exit 1
}

if ($CheckOnly) {
    $busy = Get-HealthBusyDetail -Health $health
    $device = Get-HealthDeviceDetail -Health $health
    Write-Host "Forge UI preflight: PASS runtime_mode=$($health.runtime_mode) gemma_reachable=$($health.gemma_runtime_reachable) readiness_ok=$($health.readiness_ok) safe_device_ok=$($health.safe_device_ok) $busy $device"
    return
}

$forgeDir = Split-Path -Parent $PSScriptRoot
Push-Location $forgeDir
try {
    $cargoArgs = @(
        "run",
        "--",
        "--backend", $Backend,
        "--mode", $Mode,
        "--require-health",
        "--timeout-secs", $TimeoutSecs.ToString()
    )

    if (-not $NoSafeDevice) {
        $cargoArgs += "--require-safe-device"
    }

    cargo @cargoArgs
} finally {
    Pop-Location
}
