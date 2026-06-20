param(
    [string]$RepoRoot = "D:\rust-norion",
    [int]$BackendPort = 7891,
    [switch]$SkipBuild,
    [switch]$KeepBackend,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Run a non-Gemma SmartSteam Forge integration smoke."
    Write-Host ""
    Write-Host "This starts a built-in rust-norion backend on a temporary port, runs"
    Write-Host "Forge health/doctor/preflight/UI preflight checks, then stops that backend."
    Write-Host "It never starts Gemma 12B, mistralrs, rustgpt-lab, or a real prompt."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\smoke-forge-stack.cmd"
    Write-Host "  .\tools\smartsteam-forge\smoke-forge-stack.cmd -SkipBuild"
    Write-Host "  .\tools\smartsteam-forge\smoke-forge-stack.cmd -BackendPort 7892"
    return
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

function Wait-BackendHealth {
    param(
        [int]$Port,
        [int]$TimeoutSeconds = 30
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        try {
            $health = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/health" -TimeoutSec 2
            if ($health) {
                Write-Host "backend health ready: runtime_mode=$($health.runtime_mode) readiness_ok=$($health.readiness_ok)"
                return
            }
        } catch {
            Start-Sleep -Milliseconds 500
        }
    }
    throw "backend /health did not become ready on 127.0.0.1:$Port"
}

function Get-SmokeBackendProcesses {
    param(
        [string]$RepoRoot,
        [int]$BackendPort
    )

    $bind = "127.0.0.1:$BackendPort"
    $repo = (Resolve-Path -LiteralPath $RepoRoot).Path
    Get-CimInstance Win32_Process -Filter "name = 'rust-norion.exe'" -ErrorAction SilentlyContinue |
        Where-Object {
            $_.CommandLine -like "*--serve*" -and
            $_.CommandLine -like "*--serve-bind*" -and
            $_.CommandLine -like "*$bind*" -and
            $_.ExecutablePath -like "$repo*"
        }
}

function Stop-SmokeBackend {
    param(
        [string]$RepoRoot,
        [int]$BackendPort
    )

    $processes = @(Get-SmokeBackendProcesses -RepoRoot $RepoRoot -BackendPort $BackendPort)
    foreach ($process in $processes) {
        Write-Host "Stopping smoke backend pid: $($process.ProcessId)"
        Stop-Process -Id $process.ProcessId -Force -ErrorAction SilentlyContinue
    }
}

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$Action
    )

    Write-Host ""
    Write-Host "smoke step: $Name"
    & $Action
    Write-Host "smoke step PASS: $Name"
}

function Invoke-CargoForge {
    param([string[]]$ForgeArgs)

    Push-Location (Join-Path $RepoRoot "tools\smartsteam-forge")
    try {
        & cargo @ForgeArgs
        if ($LASTEXITCODE -ne 0) {
            throw "cargo failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}

function Invoke-CargoForgeGuarded {
    param(
        [string[]]$ForgeArgs,
        [string[]]$AllowedFailurePatterns
    )

    Push-Location (Join-Path $RepoRoot "tools\smartsteam-forge")
    try {
        $oldErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $output = (& cargo @ForgeArgs 2>&1 | ForEach-Object { $_.ToString() } | Out-String)
            $exitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $oldErrorActionPreference
        }
        if (-not [string]::IsNullOrWhiteSpace($output)) {
            Write-Host $output.TrimEnd()
        }
        if ($exitCode -eq 0) {
            return
        }
        foreach ($pattern in $AllowedFailurePatterns) {
            if ($output -like "*$pattern*") {
                Write-Host "guarded check PASS: blocked by expected guard pattern '$pattern'"
                return
            }
        }
        throw "cargo failed with exit code $exitCode"
    } finally {
        Pop-Location
    }
}

function Invoke-PowerShellScriptGuarded {
    param(
        [string]$Script,
        [string[]]$Arguments,
        [string[]]$AllowedFailurePatterns
    )

    $oldErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = (& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Script @Arguments 2>&1 | ForEach-Object { $_.ToString() } | Out-String)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldErrorActionPreference
    }
    if (-not [string]::IsNullOrWhiteSpace($output)) {
        Write-Host $output.TrimEnd()
    }
    if ($exitCode -eq 0) {
        return
    }
    foreach ($pattern in $AllowedFailurePatterns) {
        if ($output -like "*$pattern*") {
            Write-Host "guarded check PASS: blocked by expected guard pattern '$pattern'"
            return
        }
    }
    throw "PowerShell script failed with exit code $exitCode"
}

Assert-RepoLayout -RepoRoot $RepoRoot

if ($null -eq (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "cargo was not found on PATH"
}

if (Test-LocalPort -Port $BackendPort) {
    throw "Port 127.0.0.1:$BackendPort is already in use. Pick -BackendPort or stop that process."
}

$forgeDir = Join-Path $RepoRoot "tools\smartsteam-forge"
$startStack = Join-Path $forgeDir "scripts\start-forge-stack.ps1"
$startUi = Join-Path $forgeDir "scripts\start-forge-ui.ps1"
$backend = "127.0.0.1:$BackendPort"
$smokeStateDir = Join-Path $RepoRoot ("target\manual-forge-service\smoke-state-" + (Get-Date -Format "yyyyMMdd-HHmmss"))

Write-Host "SmartSteam Forge smoke"
Write-Host "Gemma 12B: not started"
Write-Host "Backend: $backend"
Write-Host "State: $smokeStateDir"

try {
    Invoke-Step "start built-in backend" {
        & $startStack `
            -RepoRoot $RepoRoot `
            -BackendPort $BackendPort `
            -NoForge `
            -NoLab `
            -ServeMaxRequests 16 `
            -StateDir $smokeStateDir `
            -SkipBuild:$SkipBuild
        Wait-BackendHealth -Port $BackendPort
    }

    Invoke-Step "Forge /health" {
        Invoke-CargoForge -ForgeArgs @("run", "--", "--backend", $backend, "--connect-timeout-ms", "1000", "--read-timeout-ms", "2000", "--health")
    }

    Invoke-Step "Forge doctor" {
        Invoke-CargoForge -ForgeArgs @("run", "--", "--backend", $backend, "--connect-timeout-ms", "1000", "--read-timeout-ms", "2000", "--doctor")
    }

    Invoke-Step "Forge read-only cleanup audit" {
        Invoke-CargoForge -ForgeArgs @("run", "--", "--backend", $backend, "--connect-timeout-ms", "1000", "--read-timeout-ms", "2000", "--audit", "--audit-limit", "3")
    }

    Invoke-Step "Forge preflight" {
        Invoke-CargoForgeGuarded `
            -ForgeArgs @("run", "--", "--backend", $backend, "--connect-timeout-ms", "1000", "--read-timeout-ms", "2000", "--preflight") `
            -AllowedFailurePatterns @("experience_hygiene", "experience hygiene", "backend experience hygiene failed")
    }

    Invoke-Step "Forge UI preflight" {
        Invoke-PowerShellScriptGuarded `
            -Script $startUi `
            -Arguments @("-Backend", $backend, "-AllowBuiltIn", "-NoSafeDevice", "-CheckOnly") `
            -AllowedFailurePatterns @("experience_hygiene", "experience hygiene", "backend experience hygiene failed")
    }

    Write-Host ""
    Write-Host "smoke: PASS"
} finally {
    if ($KeepBackend) {
        Write-Host "Keeping smoke backend because -KeepBackend was set."
    } else {
        Stop-SmokeBackend -RepoRoot $RepoRoot -BackendPort $BackendPort
    }
}
