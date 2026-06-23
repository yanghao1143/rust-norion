param(
    [string]$RepoRoot = "D:\rust-norion",
    [int]$BackendPort = 7979,
    [int]$LocalModelPort = 8686,
    [string]$RuntimeModelId = "Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf",
    [int]$ContextTokens = 65536,
    [int]$DefaultMaxTokens = 4096,
    [int]$RuntimeTimeoutMs = 900000,
    [string]$RunDir = "",
    [string]$ModelPoolManifest = "",
    [switch]$NoModelPoolManifest,
    [switch]$SkipBuild,
    [switch]$CheckOnly,
    [int]$HealthTimeoutSecs = 90,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Reload only the local rust-norion backend for the remote Gemma chain."
    Write-Host ""
    Write-Host "This is meant after an external experience cleanup/quarantine/repair apply:"
    Write-Host "it stops only the local rust-norion.exe process from target\\remote-gemma-chain,"
    Write-Host "then starts it again against the same state directory. It does not SSH, does"
    Write-Host "not stop remote llama-server workers, does not stop tunnels, and does not stop Web Lab."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\reload-remote-gemma-backend.cmd -CheckOnly"
    Write-Host "  .\tools\smartsteam-forge\reload-remote-gemma-backend.cmd"
    Write-Host "  .\tools\smartsteam-forge\reload-remote-gemma-backend.cmd -SkipBuild"
    return
}

function Resolve-RepoRoot {
    param([string]$Path)

    $resolved = Resolve-Path -LiteralPath $Path -ErrorAction Stop
    return $resolved.Path
}

function Test-LocalPort {
    param([int]$Port)

    try {
        $client = [System.Net.Sockets.TcpClient]::new()
        $async = $client.BeginConnect("127.0.0.1", $Port, $null, $null)
        $ready = $async.AsyncWaitHandle.WaitOne(300)
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

function Wait-LocalPortClosed {
    param(
        [int]$Port,
        [int]$TimeoutSec = 15
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    do {
        if (-not (Test-LocalPort -Port $Port)) {
            return $true
        }
        Start-Sleep -Milliseconds 250
    } while ((Get-Date) -lt $deadline)
    return $false
}

function Wait-HttpOk {
    param(
        [string]$Url,
        [int]$TimeoutSec = 30
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    do {
        try {
            Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 2 | Out-Null
            return $true
        } catch {
            Start-Sleep -Seconds 1
        }
    } while ((Get-Date) -lt $deadline)
    return $false
}

function Get-BackendPidStatus {
    param(
        [string]$PidPath,
        [int]$Port
    )

    if (-not (Test-Path -LiteralPath $PidPath)) {
        return [pscustomobject]@{ State = "missing"; Pid = $null; Process = $null; Safe = $false }
    }
    $raw = Get-Content -LiteralPath $PidPath -ErrorAction SilentlyContinue | Select-Object -First 1
    [int]$processId = 0
    if (-not [int]::TryParse($raw, [ref]$processId)) {
        return [pscustomobject]@{ State = "invalid"; Pid = $raw; Process = $null; Safe = $false }
    }
    $process = $null
    try {
        $process = Get-CimInstance Win32_Process -Filter "ProcessId = $processId" -ErrorAction Stop
    } catch {
        $process = $null
    }
    if ($null -ne $process) {
        $command = [string]$process.CommandLine
        $safe = $process.Name -eq "rust-norion.exe" `
            -and $command.Contains("remote gemma via ssh tunnel") `
            -and $command.Contains("127.0.0.1:$Port")
        return [pscustomobject]@{ State = "running"; Pid = $processId; Process = $process; Safe = $safe }
    }

    $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
    if ($null -eq $process) {
        return [pscustomobject]@{ State = "stale"; Pid = $processId; Process = $null; Safe = $false }
    }
    $runDir = Split-Path -Parent $PidPath
    $buildDir = Join-Path $runDir "build"
    $processPath = [string]$process.Path
    $ownsBackendPort = $false
    try {
        $netstatRows = & netstat.exe -ano -p tcp 2>$null
        foreach ($row in $netstatRows) {
            $parts = $row.Trim() -split "\s+"
            if ($parts.Count -ge 5 `
                    -and $parts[0] -eq "TCP" `
                    -and $parts[1] -eq "127.0.0.1:$Port" `
                    -and $parts[3] -eq "LISTENING" `
                    -and $parts[4] -eq "$processId") {
                $ownsBackendPort = $true
                break
            }
        }
    } catch {
        $ownsBackendPort = $false
    }
    $safe = $process.ProcessName -eq "rust-norion" `
        -and -not [string]::IsNullOrWhiteSpace($processPath) `
        -and $processPath.StartsWith($buildDir, [System.StringComparison]::OrdinalIgnoreCase) `
        -and $processPath.EndsWith("rust-norion.exe", [System.StringComparison]::OrdinalIgnoreCase) `
        -and $ownsBackendPort
    return [pscustomobject]@{ State = "running"; Pid = $processId; Process = $process; Safe = $safe }
}

function Stop-BackendPid {
    param(
        [string]$PidPath,
        [int]$Port
    )

    $status = Get-BackendPidStatus -PidPath $PidPath -Port $Port
    if ($status.State -in @("missing", "stale", "invalid")) {
        Remove-Item -LiteralPath $PidPath -Force -ErrorAction SilentlyContinue
        return $status
    }
    if (-not $status.Safe) {
        throw "refusing to stop pid $($status.Pid) from $PidPath because it is not the remote Gemma chain rust-norion backend"
    }
    Write-Host "stopping local rust-norion backend pid $($status.Pid)"
    Stop-Process -Id $status.Pid -Force
    Remove-Item -LiteralPath $PidPath -Force -ErrorAction SilentlyContinue
    if (-not (Wait-LocalPortClosed -Port $Port -TimeoutSec 15)) {
        throw "backend port $Port stayed open after stopping pid $($status.Pid)"
    }
    return $status
}

$RepoRoot = Resolve-RepoRoot $RepoRoot
if ([string]::IsNullOrWhiteSpace($RunDir)) {
    $RunDir = Join-Path $RepoRoot "target\remote-gemma-chain"
}
$BuildDir = Join-Path $RunDir "build"
$StateDir = Join-Path $RunDir "state"
$LogDir = Join-Path $RunDir "logs"
$PidPath = Join-Path $RunDir "rust-norion.pid"
$BuildTargetDir = Join-Path $BuildDir "rust-norion"
if (-not $SkipBuild) {
    $buildStamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $BuildTargetDir = Join-Path $BuildDir "rust-norion-$buildStamp-$PID"
}
$backendExe = Join-Path $BuildTargetDir "debug\rust-norion.exe"
$experiencePath = Join-Path $StateDir "experience.ndkv"

if (-not $NoModelPoolManifest -and [string]::IsNullOrWhiteSpace($ModelPoolManifest)) {
    $defaultManifest = Join-Path $RepoRoot "target\gemma-chain\apple-model-pool.generated.json"
    if (Test-Path -LiteralPath $defaultManifest -PathType Leaf) {
        $ModelPoolManifest = $defaultManifest
    }
}
if (-not [string]::IsNullOrWhiteSpace($ModelPoolManifest)) {
    $ModelPoolManifest = (Resolve-Path -LiteralPath $ModelPoolManifest -ErrorAction Stop).Path
}

$pidStatus = Get-BackendPidStatus -PidPath $PidPath -Port $BackendPort
$portListening = Test-LocalPort -Port $BackendPort

Write-Host "SmartSteam remote Gemma backend reload"
Write-Host "repo:      $RepoRoot"
Write-Host "run_dir:   $RunDir"
Write-Host "backend:   127.0.0.1:$BackendPort"
Write-Host "model_api: http://127.0.0.1:$LocalModelPort"
Write-Host "model_id:  $RuntimeModelId"
Write-Host "state:     $StateDir"
Write-Host "experience=$experiencePath"
Write-Host "pid_file:  $PidPath state=$($pidStatus.State) pid=$($pidStatus.Pid) safe=$($pidStatus.Safe)"
Write-Host "model_pool_manifest=$(if ([string]::IsNullOrWhiteSpace($ModelPoolManifest)) { 'disabled' } else { $ModelPoolManifest })"

if ($CheckOnly) {
    Write-Host ""
    Write-Host "SmartSteam remote Gemma backend reload preflight: PASS"
    Write-Host "check_only=true"
    Write-Host "touches_remote=false"
    Write-Host "stops_remote=false"
    Write-Host "stops_tunnel=false"
    Write-Host "stops_web_lab=false"
    Write-Host "stops_backend=$(if ($pidStatus.State -eq 'running' -and $pidStatus.Safe) { 'would_stop' } else { 'none' })"
    Write-Host "starts_process=false"
    Write-Host "sends_prompt=false"
    return
}

New-Item -ItemType Directory -Force -Path $RunDir, $BuildDir, $StateDir, $LogDir | Out-Null

if (-not $SkipBuild) {
    Write-Host "building rust-norion into isolated target dir: $BuildTargetDir"
    Push-Location $RepoRoot
    try {
        & cargo build --target-dir $BuildTargetDir
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed for rust-norion"
        }
    } finally {
        Pop-Location
    }
}
if (-not (Test-Path -LiteralPath $backendExe -PathType Leaf)) {
    throw "backend binary not found: $backendExe"
}

Stop-BackendPid -PidPath $PidPath -Port $BackendPort | Out-Null
if (Test-LocalPort -Port $BackendPort) {
    throw "backend port $BackendPort is still occupied after local backend reload stop; refusing to start another backend"
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$backendOut = Join-Path $LogDir "rust-norion-reload-$stamp.out.log"
$backendErr = Join-Path $LogDir "rust-norion-reload-$stamp.err.log"
$backendArgs = @(
    "--serve", "--serve-bind", "127.0.0.1:$BackendPort",
    "--gemma-runtime-server", "http://127.0.0.1:$LocalModelPort",
    "--gemma-model-id", $RuntimeModelId,
    "--runtime-native-window", "$ContextTokens",
    "--max-tokens", "$DefaultMaxTokens",
    "--runtime-timeout-ms", "$RuntimeTimeoutMs",
    "--memory", (Join-Path $StateDir "memory.ndkv"),
    "--experience", $experiencePath,
    "--adaptive", (Join-Path $StateDir "adaptive.ndkv"),
    "--trace", (Join-Path $LogDir "trace-http-runtime-reload-$stamp.jsonl")
)
if (-not [string]::IsNullOrWhiteSpace($ModelPoolManifest)) {
    $backendArgs += @("--model-pool-manifest", $ModelPoolManifest)
}
$backendArgs += "remote gemma via ssh tunnel"

$process = Start-Process -FilePath $backendExe `
    -WorkingDirectory $StateDir `
    -WindowStyle Hidden `
    -PassThru `
    -RedirectStandardOutput $backendOut `
    -RedirectStandardError $backendErr `
    -ArgumentList $backendArgs
Set-Content -LiteralPath $PidPath -Value $process.Id
Write-Host "rust-norion backend reloaded pid $($process.Id): http://127.0.0.1:$BackendPort"
Write-Host "logs: $backendOut / $backendErr"
if (-not (Wait-HttpOk -Url "http://127.0.0.1:$BackendPort/health" -TimeoutSec $HealthTimeoutSecs)) {
    throw "backend health did not become ready after reload"
}
Write-Host "remote_gemma_backend_reload=PASS"
Write-Host "touches_remote=false"
Write-Host "stops_remote=false"
Write-Host "stops_tunnel=false"
Write-Host "stops_web_lab=false"
Write-Host "sends_prompt=false"
