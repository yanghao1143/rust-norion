param(
    [string]$RepoRoot = "D:\rust-norion",
    [int]$BackendPort = 7878,
    [int]$LabPort = 8787,
    [string]$Mode = "chat",
    [int]$TimeoutSecs = 900,
    [int]$ServeMaxRequests = 0,
    [string]$StateDir = "",
    [switch]$UseProjectState,
    [switch]$SkipBuild,
    [switch]$KeepExistingBackend,
    [switch]$NoLab,
    [switch]$NoForge,
    [switch]$CheckOnly,
    [switch]$WaitReady,
    [int]$ReadyTimeoutSecs = 90,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Start a safe SmartSteam Forge stack without Gemma 12B."
    Write-Host ""
    Write-Host "This starts rust-norion with the built-in heuristic backend, optionally starts"
    Write-Host "rustgpt-lab, then opens the Forge TUI. It never starts mistralrs or Gemma."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\start-forge-stack.cmd"
    Write-Host "  .\tools\smartsteam-forge\start-forge-stack.cmd -CheckOnly"
    Write-Host "  .\tools\smartsteam-forge\start-forge-stack.cmd -NoForge"
    Write-Host "  .\tools\smartsteam-forge\start-forge-stack.cmd -NoLab"
    Write-Host "  .\tools\smartsteam-forge\start-forge-stack.cmd -Mode business-cycle"
    Write-Host ""
    Write-Host "Common options:"
    Write-Host "  -ServeMaxRequests <n>  Stop the built-in backend after n HTTP requests; useful for checks."
    Write-Host "  -StateDir <path>       Isolate backend state files; default is target\manual-forge-service\forge-state."
    Write-Host "  -UseProjectState       Use repo-root .ndkv files instead of the default isolated state."
    Write-Host "  -SkipBuild             Reuse existing binaries."
    Write-Host "  -NoForge               Start backend/lab only."
    Write-Host "  -NoLab                 Skip rustgpt-lab."
    Write-Host ""
    Write-Host "For explicit Gemma 12B full-stack startup, use:"
    Write-Host "  .\tools\smartsteam-forge\start-gemma-forge.cmd -CheckOnly"
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

function Wait-LocalPort {
    param(
        [int]$Port,
        [int]$TimeoutSeconds = 30
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        if (Test-LocalPort -Port $Port) {
            return $true
        }
        Start-Sleep -Seconds 1
    }
    return $false
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

function Resolve-ForgeStateDir {
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
        $resolvedStateDir = Join-Path $RepoRoot "target\manual-forge-service\forge-state"
    } elseif (-not [System.IO.Path]::IsPathRooted($resolvedStateDir)) {
        $resolvedStateDir = Join-Path $RepoRoot $resolvedStateDir
    }

    return [System.IO.Path]::GetFullPath($resolvedStateDir)
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

function Test-BackendUsesProjectState {
    param(
        [object]$Health,
        [string]$RepoRoot
    )

    $experienceFile = Get-HealthExperienceFile -Health $Health
    if ([string]::IsNullOrWhiteSpace($experienceFile)) {
        return $false
    }

    $actual = Convert-ToFullPath -Path $experienceFile -BasePath $RepoRoot
    $project = Convert-ToFullPath -Path "noiron-experience.ndkv" -BasePath $RepoRoot
    return $actual -eq $project
}

function Assert-CargoAvailable {
    if ($null -eq (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "cargo was not found on PATH"
    }
}

function Assert-RepoLayout {
    param([string]$RepoRoot)

    if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot "Cargo.toml"))) {
        throw "RepoRoot does not look like rust-norion: $RepoRoot"
    }
    if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot "tools\smartsteam-forge\Cargo.toml"))) {
        throw "Missing SmartSteam Forge Cargo.toml under $RepoRoot"
    }
}

function Invoke-StartupCheck {
    param(
        [string]$RepoRoot,
        [int]$BackendPort,
        [int]$LabPort,
        [bool]$NoLab,
        [string]$ResolvedStateDir
    )

    $ok = $true
    Write-Host "SmartSteam Forge safe stack startup check"
    Write-Host "gemma: not started by this command"
    if ([string]::IsNullOrWhiteSpace($ResolvedStateDir)) {
        Write-Host "state: project root .ndkv files (-UseProjectState)"
    } else {
        Write-Host "state: isolated $ResolvedStateDir"
        Write-Host "state_files: memory.ndkv experience.ndkv adaptive.ndkv trace.jsonl"
    }
    Write-Host ""

    try {
        Assert-RepoLayout -RepoRoot $RepoRoot
        Write-Host "repo: OK $RepoRoot"
    } catch {
        Write-Warning $_.Exception.Message
        $ok = $false
    }

    if ($null -eq (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Warning "cargo: not found on PATH"
        $ok = $false
    } else {
        Write-Host "cargo: OK"
    }

    $backendListening = Test-LocalPort -Port $BackendPort
    $labListening = Test-LocalPort -Port $LabPort
    Write-Host "ports: rust-norion:$BackendPort=$backendListening rustgpt-lab:$LabPort=$labListening"

    $health = Get-BackendHealth -Port $BackendPort
    if ($health) {
        Write-Host "backend: reachable runtime_mode=$($health.runtime_mode) readiness_ok=$($health.readiness_ok) safe_device_ok=$($health.safe_device_ok)"
        $experienceFile = Get-HealthExperienceFile -Health $health
        if (-not [string]::IsNullOrWhiteSpace($experienceFile)) {
            Write-Host "backend_state: experience_file=$experienceFile"
        }
        if (-not [string]::IsNullOrWhiteSpace($ResolvedStateDir) -and (Test-BackendUsesProjectState -Health $health -RepoRoot $RepoRoot)) {
            Write-Warning "backend_state: existing backend is using repo-root noiron-experience.ndkv. Stop it or pass -UseProjectState if that is intentional."
            $ok = $false
        }
    } elseif ($backendListening) {
        Write-Warning "backend: port $BackendPort is occupied but /health is not readable"
        $ok = $false
    } else {
        Write-Host "backend: not running; start script will launch built-in rust-norion."
    }

    if ($NoLab) {
        Write-Host "web lab: skipped by -NoLab"
    } elseif ($labListening) {
        Write-Host "web lab: port $LabPort already listening; start script will leave it alone."
    } else {
        Write-Host "web lab: not running; start script will launch rustgpt-lab."
    }

    Write-Host ""
    if ($ok) {
        Write-Host "startup_check: PASS"
        Write-Host "Next: .\tools\smartsteam-forge\start-forge-stack.cmd"
        return $true
    }

    Write-Warning "startup_check: FAIL"
    return $false
}

function Start-BuiltInBackend {
    param(
        [string]$RepoRoot,
        [int]$BackendPort,
        [int]$ServeMaxRequests,
        [string]$StateDir,
        [bool]$SkipBuild
    )

    if (-not $SkipBuild) {
        Push-Location $RepoRoot
        try {
            cargo build
        } finally {
            Pop-Location
        }
    }

    $backendExe = Join-Path $RepoRoot "target\debug\rust-norion.exe"
    if (-not (Test-Path -LiteralPath $backendExe)) {
        throw "Missing backend binary: $backendExe. Rerun without -SkipBuild."
    }

    $traceDir = Join-Path $RepoRoot "target\manual-forge-service"
    New-Item -ItemType Directory -Force -Path $traceDir | Out-Null
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $backendOut = Join-Path $traceDir "rust-norion-built-in-$stamp.out.log"
    $backendErr = Join-Path $traceDir "rust-norion-built-in-$stamp.err.log"
    $backendArgs = @(
        "--serve",
        "--serve-bind", "127.0.0.1:$BackendPort"
    )
    $backendWorkingDirectory = $RepoRoot
    if (-not [string]::IsNullOrWhiteSpace($StateDir)) {
        $resolvedStateDir = $StateDir
        if (-not [System.IO.Path]::IsPathRooted($resolvedStateDir)) {
            $resolvedStateDir = Join-Path $RepoRoot $resolvedStateDir
        }
        New-Item -ItemType Directory -Force -Path $resolvedStateDir | Out-Null
        $backendWorkingDirectory = $resolvedStateDir
        $backendArgs += @(
            "--memory", (Join-Path $resolvedStateDir "memory.ndkv"),
            "--experience", (Join-Path $resolvedStateDir "experience.ndkv"),
            "--adaptive", (Join-Path $resolvedStateDir "adaptive.ndkv"),
            "--trace", (Join-Path $resolvedStateDir "trace.jsonl")
        )
    }
    if ($ServeMaxRequests -gt 0) {
        $backendMaxRequests = $ServeMaxRequests + 1
        $backendArgs += @("--serve-max-requests", $backendMaxRequests.ToString())
    }

    $process = Start-Process -FilePath $backendExe -ArgumentList $backendArgs -WorkingDirectory $backendWorkingDirectory -WindowStyle Hidden -RedirectStandardOutput $backendOut -RedirectStandardError $backendErr -PassThru
    Write-Host "rust-norion built-in backend pid: $($process.Id)"
    Write-Host "rust-norion logs: $backendOut / $backendErr"
    if (-not [string]::IsNullOrWhiteSpace($StateDir)) {
        Write-Host "rust-norion state: $backendWorkingDirectory"
    }

    if (-not (Wait-LocalPort -Port $BackendPort -TimeoutSeconds 30)) {
        throw "rust-norion did not open port $BackendPort"
    }
}

function Start-WebLab {
    param(
        [string]$RepoRoot,
        [int]$BackendPort,
        [int]$LabPort,
        [bool]$SkipBuild
    )

    $labDir = Join-Path $RepoRoot "tools\rustgpt-lab"
    if (-not (Test-Path -LiteralPath (Join-Path $labDir "Cargo.toml"))) {
        throw "Missing rustgpt-lab Cargo.toml: $labDir"
    }

    if (-not $SkipBuild) {
        Push-Location $labDir
        try {
            cargo build
        } finally {
            Pop-Location
        }
    }

    $labExe = Join-Path $labDir "target\debug\rustgpt-lab.exe"
    if (-not (Test-Path -LiteralPath $labExe)) {
        throw "Missing rustgpt-lab binary: $labExe. Rerun without -SkipBuild."
    }

    $traceDir = Join-Path $RepoRoot "target\manual-forge-service"
    New-Item -ItemType Directory -Force -Path $traceDir | Out-Null
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $labOut = Join-Path $traceDir "rustgpt-lab-built-in-$stamp.out.log"
    $labErr = Join-Path $traceDir "rustgpt-lab-built-in-$stamp.err.log"
    $labArgs = @("--bind", "127.0.0.1:$LabPort", "--backend", "127.0.0.1:$BackendPort")

    $process = Start-Process -FilePath $labExe -ArgumentList $labArgs -WorkingDirectory $labDir -WindowStyle Hidden -RedirectStandardOutput $labOut -RedirectStandardError $labErr -PassThru
    Write-Host "rustgpt-lab pid: $($process.Id)"
    Write-Host "rustgpt-lab logs: $labOut / $labErr"

    if (-not (Wait-LocalPort -Port $LabPort -TimeoutSeconds 30)) {
        throw "rustgpt-lab did not open port $LabPort"
    }
}

Assert-RepoLayout -RepoRoot $RepoRoot

if ($UseProjectState -and -not [string]::IsNullOrWhiteSpace($StateDir)) {
    throw "Use either -StateDir or -UseProjectState, not both."
}

$resolvedStateDir = Resolve-ForgeStateDir -RepoRoot $RepoRoot -StateDir $StateDir -UseProjectState:$UseProjectState

if ($CheckOnly) {
    $ok = Invoke-StartupCheck -RepoRoot $RepoRoot -BackendPort $BackendPort -LabPort $LabPort -NoLab:$NoLab -ResolvedStateDir $resolvedStateDir
    if ($ok) {
        exit 0
    }
    exit 1
}

Assert-CargoAvailable

$backendHealth = Get-BackendHealth -Port $BackendPort
if ($backendHealth) {
    Write-Host "Using existing rust-norion backend on 127.0.0.1:$BackendPort runtime_mode=$($backendHealth.runtime_mode)"
    $experienceFile = Get-HealthExperienceFile -Health $backendHealth
    if (-not [string]::IsNullOrWhiteSpace($experienceFile)) {
        Write-Host "Existing backend state: experience_file=$experienceFile"
    }
    if (-not [string]::IsNullOrWhiteSpace($resolvedStateDir) -and (Test-BackendUsesProjectState -Health $backendHealth -RepoRoot $RepoRoot)) {
        throw "Existing backend on port $BackendPort is using repo-root noiron-experience.ndkv. Stop it first, choose another -BackendPort, or pass -UseProjectState if that is intentional."
    }
} elseif (Test-LocalPort -Port $BackendPort) {
    if ($KeepExistingBackend) {
        Write-Warning "Backend port $BackendPort is occupied but /health is not readable. Keeping it because -KeepExistingBackend was set."
    } else {
        throw "Backend port $BackendPort is occupied but /health is not readable. Stop that process, choose -BackendPort, or pass -KeepExistingBackend."
    }
} else {
    Write-Host "Starting rust-norion built-in backend on 127.0.0.1:$BackendPort ..."
    Start-BuiltInBackend -RepoRoot $RepoRoot -BackendPort $BackendPort -ServeMaxRequests $ServeMaxRequests -StateDir $resolvedStateDir -SkipBuild:$SkipBuild
}

if (-not $NoLab) {
    if (Test-LocalPort -Port $LabPort) {
        Write-Host "rustgpt-lab already listening on 127.0.0.1:$LabPort"
    } else {
        Write-Host "Starting rustgpt-lab on 127.0.0.1:$LabPort ..."
        Start-WebLab -RepoRoot $RepoRoot -BackendPort $BackendPort -LabPort $LabPort -SkipBuild:$SkipBuild
    }
    Write-Host "Web lab: http://127.0.0.1:$LabPort"
}

Write-Host "Backend: http://127.0.0.1:$BackendPort"
Write-Host "Backend health: http://127.0.0.1:$BackendPort/health"
Write-Host "Gemma 12B: not started by start-forge-stack.cmd"

if ($NoForge) {
    Write-Host "Forge launch skipped because -NoForge was set."
    return
}

$forgeUiScript = Join-Path $PSScriptRoot "start-forge-ui.ps1"
Write-Host "Starting SmartSteam Forge TUI..."
& $forgeUiScript `
    -Backend "127.0.0.1:$BackendPort" `
    -Mode $Mode `
    -TimeoutSecs $TimeoutSecs `
    -AllowBuiltIn `
    -NoSafeDevice `
    -WaitReady:$WaitReady `
    -ReadyTimeoutSecs $ReadyTimeoutSecs
