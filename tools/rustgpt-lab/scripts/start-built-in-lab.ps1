param(
    [string]$RepoRoot = "",
    [int]$BackendPort = 7878,
    [Alias("LabPort")]
    [int]$WebPort = 8787,
    [string]$StateDir = "",
    [int]$LabBackendTimeoutSeconds = 900,
    [switch]$SkipBuild,
    [switch]$NoOpen,
    [switch]$CheckOnly,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

function Show-Help {
    Write-Host "Start rustgpt-lab with the built-in rust-norion backend."
    Write-Host ""
    Write-Host "This is the safe Web Lab path: it does not start Gemma 12B or mistralrs."
    Write-Host "State defaults to target\manual-web-lab-service\built-in-lab-state."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\rustgpt-lab\start-built-in-lab.cmd"
    Write-Host "  .\tools\rustgpt-lab\start-built-in-lab.cmd -NoOpen -SkipBuild"
    Write-Host "  .\tools\rustgpt-lab\start-built-in-lab.cmd -StateDir target\manual-web-lab-service\built-in-lab-state"
    Write-Host "  .\tools\rustgpt-lab\start-built-in-lab.cmd -CheckOnly"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -BackendPort <n>  rust-norion service port, default 7878"
    Write-Host "  -WebPort <n>      rustgpt-lab Web UI port, default 8787"
    Write-Host "  -StateDir <path>  isolated state directory for memory/experience/adaptive .ndkv"
    Write-Host "  -LabBackendTimeoutSeconds <n>  Web Lab -> rust-norion total streaming window, default 900"
    Write-Host "  -SkipBuild        use existing target\debug binaries"
    Write-Host "  -NoOpen           do not open the browser"
    Write-Host "  -CheckOnly        print plan and validate ports/paths without starting or writing state"
    Write-Host ""
    Write-Host "Port map:"
    Write-Host "  7878 = rust-norion built-in backend for safe local UI tests."
    Write-Host "  8787 = rustgpt-lab Web UI and local SSE proxy."
    Write-Host "  8686 = optional Gemma/mistralrs runtime; this built-in path does not use it."
}

if ($Help) {
    Show-Help
    return
}

function Resolve-AbsolutePath {
    param(
        [string]$Path,
        [string]$BasePath
    )

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }
    return [System.IO.Path]::GetFullPath((Join-Path $BasePath $Path))
}

function Assert-Port {
    param(
        [string]$Name,
        [int]$Port
    )

    if ($Port -lt 1 -or $Port -gt 65535) {
        throw "$Name must be between 1 and 65535; got $Port"
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

function Get-HealthJson {
    param([string]$Url)
    try {
        return Invoke-RestMethod -Uri $Url -TimeoutSec 2
    } catch {
        return $null
    }
}

function Get-NormalizedPath {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }
    return [System.IO.Path]::GetFullPath($Path).TrimEnd("\", "/").ToLowerInvariant()
}

function Test-PathUnder {
    param(
        [string]$Child,
        [string]$Parent
    )

    $childPath = Get-NormalizedPath -Path $Child
    $parentPath = Get-NormalizedPath -Path $Parent
    if ([string]::IsNullOrWhiteSpace($childPath) -or [string]::IsNullOrWhiteSpace($parentPath)) {
        return $false
    }
    return $childPath -eq $parentPath -or $childPath.StartsWith($parentPath + "\")
}

function Assert-ExistingBackendIsSafe {
    param(
        [int]$Port,
        [string]$ExpectedStateDir
    )

    $health = Get-HealthJson -Url "http://127.0.0.1:$Port/health"
    if ($null -eq $health -or $health.service -ne "rust-norion") {
        throw "Port $Port is already listening, but it is not a rust-norion /health endpoint. Stop it or choose -BackendPort."
    }
    if ($health.runtime_mode -ne "built-in") {
        throw "Port $Port already has rust-norion runtime_mode=$($health.runtime_mode). This safe entry only uses built-in; stop it or choose -BackendPort."
    }

    $experienceFile = $health.experience_hygiene.experience_file
    if (-not [string]::IsNullOrWhiteSpace($experienceFile) -and -not (Test-PathUnder -Child $experienceFile -Parent $ExpectedStateDir)) {
        throw "Port $Port has built-in rust-norion, but its experience file is outside this StateDir: $experienceFile"
    }

    Write-Host "Existing built-in rust-norion is safe on 127.0.0.1:$Port"
}

function Assert-ExistingLabMatches {
    param(
        [int]$Port,
        [int]$BackendPort
    )

    $health = Get-HealthJson -Url "http://127.0.0.1:$Port/health"
    if ($null -eq $health -or $health.service -ne "rustgpt-lab") {
        throw "Port $Port is already listening, but it is not rustgpt-lab. Stop it or choose -WebPort."
    }

    $expectedBackend = "127.0.0.1:$BackendPort"
    if ($health.backend -ne $expectedBackend) {
        throw "Port $Port has rustgpt-lab for backend $($health.backend), expected $expectedBackend. Stop it or choose -WebPort."
    }

    Write-Host "Existing rustgpt-lab is already connected to $expectedBackend on 127.0.0.1:$Port"
}

function Join-ProcessArguments {
    param([string[]]$Arguments)

    return ($Arguments | ForEach-Object {
        $value = [string]$_
        if ($value -match '[\s"]') {
            '"' + ($value -replace '"', '\"') + '"'
        } else {
            $value
        }
    }) -join " "
}

function Invoke-CargoBuild {
    param([string]$WorkingDirectory)

    Push-Location $WorkingDirectory
    try {
        cargo build
    } finally {
        Pop-Location
    }
}

Assert-Port -Name "BackendPort" -Port $BackendPort
Assert-Port -Name "WebPort" -Port $WebPort
if ($BackendPort -eq $WebPort) {
    throw "BackendPort and WebPort must be different."
}

$scriptDir = Split-Path -Parent $PSCommandPath
$labRoot = Resolve-AbsolutePath -Path ".." -BasePath $scriptDir
if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = Resolve-AbsolutePath -Path "..\.." -BasePath $labRoot
} else {
    $RepoRoot = Resolve-AbsolutePath -Path $RepoRoot -BasePath (Get-Location).Path
}

if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot "Cargo.toml"))) {
    throw "RepoRoot does not look like rust-norion: $RepoRoot"
}
if (-not (Test-Path -LiteralPath (Join-Path $labRoot "Cargo.toml"))) {
    throw "rustgpt-lab Cargo.toml not found: $labRoot"
}

$traceDir = Join-Path $RepoRoot "target\manual-web-lab-service"
if ([string]::IsNullOrWhiteSpace($StateDir)) {
    $resolvedStateDir = Join-Path $traceDir "built-in-lab-state"
} else {
    $resolvedStateDir = Resolve-AbsolutePath -Path $StateDir -BasePath $RepoRoot
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$memoryPath = Join-Path $resolvedStateDir "memory.ndkv"
$experiencePath = Join-Path $resolvedStateDir "experience.ndkv"
$adaptivePath = Join-Path $resolvedStateDir "adaptive.ndkv"
$tracePath = Join-Path $traceDir "trace-built-in-$stamp.jsonl"
$backendExe = Join-Path $RepoRoot "target\debug\rust-norion.exe"
$labExe = Join-Path $labRoot "target\debug\rustgpt-lab.exe"

Write-Host "Built-in Web Lab safe path"
Write-Host "This script starts rust-norion built-in backend + rustgpt-lab Web UI only."
Write-Host "It does not start Gemma 12B, mistralrs, or write repo-root noiron-experience.ndkv."
Write-Host ""
Write-Host "RepoRoot: $RepoRoot"
Write-Host "StateDir: $resolvedStateDir"
Write-Host "Backend:  http://127.0.0.1:$BackendPort"
Write-Host "Web UI:   http://127.0.0.1:$WebPort/"
Write-Host "Lab backend response window: ${LabBackendTimeoutSeconds}s"
Write-Host "Memory:   $memoryPath"
Write-Host "Experience: $experiencePath"
Write-Host "Adaptive: $adaptivePath"
Write-Host ""

if ($CheckOnly) {
    Write-Host "CheckOnly: no build, no process start, no browser open, no .ndkv writes."
    if ($SkipBuild) {
        if (-not (Test-Path -LiteralPath $backendExe)) {
            Write-Warning "SkipBuild was set but rust-norion.exe was not found: $backendExe"
        }
        if (-not (Test-Path -LiteralPath $labExe)) {
            Write-Warning "SkipBuild was set but rustgpt-lab.exe was not found: $labExe"
        }
    }

    if (Test-LocalPort -Port $BackendPort) {
        Assert-ExistingBackendIsSafe -Port $BackendPort -ExpectedStateDir $resolvedStateDir
    } else {
        Write-Host "Backend port $BackendPort is free."
    }
    if (Test-LocalPort -Port $WebPort) {
        Assert-ExistingLabMatches -Port $WebPort -BackendPort $BackendPort
    } else {
        Write-Host "Web port $WebPort is free."
    }
    return
}

New-Item -ItemType Directory -Force -Path $traceDir | Out-Null
New-Item -ItemType Directory -Force -Path $resolvedStateDir | Out-Null

if (-not $SkipBuild) {
    Write-Host "Building rust-norion..."
    Invoke-CargoBuild -WorkingDirectory $RepoRoot
    Write-Host "Building rustgpt-lab..."
    Invoke-CargoBuild -WorkingDirectory $labRoot
}

if (Test-LocalPort -Port $BackendPort) {
    Assert-ExistingBackendIsSafe -Port $BackendPort -ExpectedStateDir $resolvedStateDir
} else {
    if (-not (Test-Path -LiteralPath $backendExe)) {
        throw "rust-norion.exe not found: $backendExe. Run without -SkipBuild first."
    }

    $backendOut = Join-Path $traceDir "rust-norion-built-in-$stamp.out.log"
    $backendErr = Join-Path $traceDir "rust-norion-built-in-$stamp.err.log"
    $backendArgs = @(
        "--serve",
        "--serve-bind", "127.0.0.1:$BackendPort",
        "--memory", $memoryPath,
        "--experience", $experiencePath,
        "--adaptive", $adaptivePath,
        "--trace", $tracePath,
        "web lab built-in safe service"
    )

    $backendProcess = Start-Process `
        -FilePath $backendExe `
        -ArgumentList (Join-ProcessArguments -Arguments $backendArgs) `
        -WorkingDirectory $resolvedStateDir `
        -WindowStyle Hidden `
        -RedirectStandardOutput $backendOut `
        -RedirectStandardError $backendErr `
        -PassThru

    Write-Host "rust-norion built-in pid: $($backendProcess.Id)"
    Write-Host "rust-norion logs: $backendOut / $backendErr"
    if (-not (Wait-LocalPort -Port $BackendPort -TimeoutSeconds 30)) {
        throw "rust-norion did not open port $BackendPort. See $backendErr"
    }
    Assert-ExistingBackendIsSafe -Port $BackendPort -ExpectedStateDir $resolvedStateDir
}

if (Test-LocalPort -Port $WebPort) {
    Assert-ExistingLabMatches -Port $WebPort -BackendPort $BackendPort
} else {
    if (-not (Test-Path -LiteralPath $labExe)) {
        throw "rustgpt-lab.exe not found: $labExe. Run without -SkipBuild first."
    }

    $labOut = Join-Path $traceDir "rustgpt-lab-$stamp.out.log"
    $labErr = Join-Path $traceDir "rustgpt-lab-$stamp.err.log"
    $labArgs = @(
        "--bind", "127.0.0.1:$WebPort",
        "--backend", "127.0.0.1:$BackendPort",
        "--backend-timeout-secs", $LabBackendTimeoutSeconds.ToString()
    )
    $labProcess = Start-Process `
        -FilePath $labExe `
        -ArgumentList (Join-ProcessArguments -Arguments $labArgs) `
        -WorkingDirectory $labRoot `
        -WindowStyle Hidden `
        -RedirectStandardOutput $labOut `
        -RedirectStandardError $labErr `
        -PassThru

    Write-Host "rustgpt-lab pid: $($labProcess.Id)"
    Write-Host "rustgpt-lab logs: $labOut / $labErr"
    if (-not (Wait-LocalPort -Port $WebPort -TimeoutSeconds 30)) {
        throw "rustgpt-lab did not open port $WebPort. See $labErr"
    }
    Assert-ExistingLabMatches -Port $WebPort -BackendPort $BackendPort
}

Write-Host ""
Write-Host "Built-in Web Lab ready: http://127.0.0.1:$WebPort/"
Write-Host "Backend health: http://127.0.0.1:$BackendPort/health"
Write-Host "State directory: $resolvedStateDir"
if (-not $NoOpen) {
    Start-Process "http://127.0.0.1:$WebPort/"
}
