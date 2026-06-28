param(
    [string]$RepoRoot = "D:\rust-norion",
    [string]$Snapshot = "D:\hf-cache\hub\models--google--gemma-4-12B-it\snapshots\5926caa4ec0cac5cbfadaf4077420520de1d5205",
    [string]$HfCache = "D:\hf-cache",
    [int]$MistralPort = 8686,
    [int]$BackendPort = 7878,
    [int]$LabPort = 8787,
    [switch]$SkipBuild,
    [switch]$Force,
    [switch]$CheckOnly,
    [switch]$Help,
    [string]$StateDir = "",
    [switch]$UseProjectState,
    [int]$MaxTokens = 262144,
    [int]$MaxSeqLen = 262144,
    [int]$RuntimeTimeoutMs = 900000,
    [int]$LabBackendTimeoutSeconds = 900,
    [double]$MinFreeRamGB = 18.0,
    [double]$MinFreeGpuGB = 13.0
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Start the real Gemma 12B rust-norion + rustgpt-lab stack."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\rustgpt-lab\start-gemma-lab.cmd -CheckOnly"
    Write-Host "  .\tools\rustgpt-lab\start-gemma-lab.cmd -StateDir target\manual-gemma-service\lab-state"
    Write-Host ""
    Write-Host "Safety:"
    Write-Host "  - Without -CheckOnly, this script can build binaries and start Gemma, rust-norion, and rustgpt-lab."
    Write-Host "  - -CheckOnly is read-only: it starts no processes, builds nothing, and writes no state."
    Write-Host "  - Use an isolated -StateDir for experiments, or -UseProjectState for the versioned project state bucket."
    Write-Host ""
    Write-Host "Port map:"
    Write-Host "  7878 = rust-norion backend; Web Lab forwards prompts there after gates."
    Write-Host "  8787 = rustgpt-lab browser UI and local SSE proxy."
    Write-Host "  8686 = optional Gemma/mistralrs runtime behind rust-norion; do not send prompts there directly."
    Write-Host ""
    Write-Host "Useful options:"
    Write-Host "  -Snapshot <path>                 Gemma snapshot directory."
    Write-Host "  -HfCache <path>                  Hugging Face cache directory."
    Write-Host "  -MistralPort <port>              mistralrs server port."
    Write-Host "  -BackendPort <port>              rust-norion backend port."
    Write-Host "  -LabPort <port>                  rustgpt-lab Web Lab port."
    Write-Host "  -RuntimeTimeoutMs <ms>           rust-norion -> Gemma runtime request timeout, not the Web Lab read poll."
    Write-Host "  -LabBackendTimeoutSeconds <sec>  rustgpt-lab -> rust-norion total streaming window."
    Write-Host "  -MaxTokens <n>                   request generation budget sent as max_tokens."
    Write-Host "  -MaxSeqLen <n>                   model sequence/context length for the runtime."
    Write-Host "  -MinFreeRamGB <gb>               startup RAM preflight threshold."
    Write-Host "  -MinFreeGpuGB <gb>               startup VRAM preflight threshold."
    Write-Host "  -Force                           override resource preflight in startup mode."
    return
}

function Get-RustNorionProjectStateDir {
    param([string]$RepoRoot)

    $cargoToml = Join-Path $RepoRoot "Cargo.toml"
    $versionLine = Get-Content -LiteralPath $cargoToml |
        Where-Object { $_ -match '^\s*version\s*=\s*"([^"]+)"' } |
        Select-Object -First 1
    if ([string]::IsNullOrWhiteSpace($versionLine)) {
        throw "Could not read package version from $cargoToml"
    }

    $version = [regex]::Match($versionLine, '^\s*version\s*=\s*"([^"]+)"').Groups[1].Value
    return [System.IO.Path]::GetFullPath((Join-Path (Join-Path $RepoRoot "state") "rust-norion-v$version"))
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
        [int]$TimeoutSeconds = 180
    )
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        if (Test-LocalPort -Port $Port) {
            return $true
        }
        Start-Sleep -Seconds 2
    }
    return $false
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

        $lines = & $nvidia.Source --query-gpu=name,memory.total,memory.used --format=csv,noheader,nounits 2>$null
        if (-not $lines) {
            return @()
        }

        $items = @()
        foreach ($line in $lines) {
            $parts = $line -split ","
            if ($parts.Count -lt 3) {
                continue
            }

            [double]$totalMB = 0
            [double]$usedMB = 0
            if (-not [double]::TryParse($parts[1].Trim(), [ref]$totalMB)) {
                continue
            }
            if (-not [double]::TryParse($parts[2].Trim(), [ref]$usedMB)) {
                continue
            }

            $items += [pscustomobject]@{
                Name = $parts[0].Trim()
                TotalGB = [math]::Round($totalMB / 1024, 2)
                UsedGB = [math]::Round($usedMB / 1024, 2)
                FreeGB = [math]::Round(($totalMB - $usedMB) / 1024, 2)
            }
        }

        return $items
    } catch {
        return @()
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

function Assert-ExistingBackendIsGemma {
    param(
        [int]$Port,
        [int]$MistralPort
    )

    $health = Get-HealthJson -Url "http://127.0.0.1:$Port/health"
    if ($null -eq $health -or $health.service -ne "rust-norion") {
        throw "Port $Port is already listening, but it is not a rust-norion /health endpoint. Stop it or choose -BackendPort."
    }

    $expectedServer = "http://127.0.0.1:$MistralPort"
    if ($health.runtime_mode -ne "gemma-http" -or $health.gemma_runtime_server -ne $expectedServer) {
        throw "Port $Port already has rust-norion runtime_mode=$($health.runtime_mode) gemma_runtime_server=$($health.gemma_runtime_server), expected gemma-http at $expectedServer. Stop it or choose -BackendPort."
    }

    Write-Host "Existing rust-norion Gemma backend is safe on 127.0.0.1:$Port"
    return $health
}

function Assert-ExistingLabMatches {
    param(
        [int]$Port,
        [int]$BackendPort
    )

    $health = Get-HealthJson -Url "http://127.0.0.1:$Port/health"
    if ($null -eq $health -or $health.service -ne "rustgpt-lab") {
        throw "Port $Port is already listening, but it is not a rustgpt-lab /health endpoint. Stop it or choose -LabPort."
    }

    $expectedBackend = "127.0.0.1:$BackendPort"
    if ($health.backend -ne $expectedBackend) {
        throw "Port $Port has rustgpt-lab for backend $($health.backend), expected $expectedBackend. Stop it or choose -LabPort."
    }

    Write-Host "Existing rustgpt-lab is safe on 127.0.0.1:$Port and points at $expectedBackend"
}

function Write-GemmaExperienceSafety {
    param(
        [string]$RepoRoot,
        [string]$ResolvedStateDir,
        [switch]$UseProjectState,
        [object]$BackendHealth
    )

    if ($UseProjectState) {
        $projectMemory = Join-Path $ResolvedStateDir "memory.ndkv"
        $projectExperience = Join-Path $ResolvedStateDir "experience.ndkv"
        $projectAdaptive = Join-Path $ResolvedStateDir "adaptive.ndkv"
        Write-Host "experience_safety=versioned_project_state_requested"
        Write-Host "memory_file=$projectMemory"
        Write-Host "experience_file=$projectExperience"
        Write-Host "adaptive_file=$projectAdaptive"
        Write-Warning "UseProjectState was requested. Real Gemma prompts may read/write the versioned project state bucket."
    } else {
        $memoryFile = Join-Path $ResolvedStateDir "memory.ndkv"
        $experienceFile = Join-Path $ResolvedStateDir "experience.ndkv"
        $adaptiveFile = Join-Path $ResolvedStateDir "adaptive.ndkv"
        Write-Host "experience_safety=isolated_state_dir"
        Write-Host "memory_file=$memoryFile"
        Write-Host "experience_file=$experienceFile"
        Write-Host "adaptive_file=$adaptiveFile"
    }

    if ($null -eq $BackendHealth) {
        return
    }

    $activeExperience = $BackendHealth.experience_hygiene.experience_file
    if ([string]::IsNullOrWhiteSpace($activeExperience)) {
        Write-Host "active_backend_experience_file="
        return
    }

    Write-Host "active_backend_experience_file=$activeExperience"
    if ($UseProjectState) {
        if (Test-PathUnder -Child $activeExperience -Parent $ResolvedStateDir) {
            Write-Host "active_backend_experience_safety=versioned_project_state"
        } else {
            Write-Warning "Existing backend experience file is outside the versioned project state bucket: $activeExperience"
        }
        return
    }

    if (Test-PathUnder -Child $activeExperience -Parent $ResolvedStateDir) {
        Write-Host "active_backend_experience_safety=matches_state_dir"
    } else {
        Write-Warning "Existing backend experience file is outside this StateDir: $activeExperience"
    }
}

function Assert-BackendStateMatches {
    param(
        [object]$Health,
        [string]$ResolvedStateDir
    )

    if ([string]::IsNullOrWhiteSpace($ResolvedStateDir)) {
        return
    }

    $activeExperience = $Health.experience_hygiene.experience_file
    if ([string]::IsNullOrWhiteSpace($activeExperience)) {
        Write-Host "Existing backend did not report experience_hygiene.experience_file."
        Write-Host "Restart it with StateDir before sending prompts."
        exit 1
    }
    if (-not (Test-PathUnder -Child $activeExperience -Parent $ResolvedStateDir)) {
        Write-Host "Existing backend experience file is outside expected StateDir."
        Write-Host "expected_state_dir=$ResolvedStateDir"
        Write-Host "active_backend_experience_file=$activeExperience"
        exit 1
    }
}

function Get-GemmaStartupResourceIssues {
    param(
        [double]$MinFreeRamGB,
        [double]$MinFreeGpuGB
    )

    $issues = @()
    Write-Host "Resource preflight for Gemma 12B..."

    $freeRamGB = Get-FreeRamGB
    if ($null -eq $freeRamGB) {
        $issues += "Could not read free system RAM."
    } else {
        Write-Host ("Free system RAM: {0:N1} GB (minimum {1:N1} GB)" -f $freeRamGB, $MinFreeRamGB)
        if ($freeRamGB -lt $MinFreeRamGB) {
            $issues += ("Free system RAM is {0:N1} GB; expected at least {1:N1} GB." -f $freeRamGB, $MinFreeRamGB)
        }
    }

    $gpuInfo = @(Get-GpuMemoryInfo)
    if ($gpuInfo.Count -eq 0) {
        $issues += "Could not read NVIDIA GPU memory with nvidia-smi."
    } else {
        $bestGpu = $gpuInfo | Sort-Object FreeGB -Descending | Select-Object -First 1
        Write-Host ("Best GPU free VRAM: {0:N1} GB / {1:N1} GB ({2})" -f $bestGpu.FreeGB, $bestGpu.TotalGB, $bestGpu.Name)
        if ($bestGpu.FreeGB -lt $MinFreeGpuGB) {
            $issues += ("Best GPU has {0:N1} GB free VRAM; expected at least {1:N1} GB." -f $bestGpu.FreeGB, $MinFreeGpuGB)
        }
    }

    if ($issues.Count -eq 0) {
        return @()
    }

    return $issues
}

function Assert-GemmaStartupResources {
    param(
        [double]$MinFreeRamGB,
        [double]$MinFreeGpuGB,
        [switch]$Force
    )

    $issues = @(Get-GemmaStartupResourceIssues -MinFreeRamGB $MinFreeRamGB -MinFreeGpuGB $MinFreeGpuGB)
    if ($issues.Count -eq 0) {
        return
    }

    Write-Host ""
    Write-Warning "Gemma 12B startup preflight found resource pressure:"
    foreach ($issue in $issues) {
        Write-Warning "  $issue"
    }

    if (-not $Force) {
        throw "Preflight blocked startup. Free RAM/VRAM or rerun with -Force to override."
    }

    Write-Warning "Continuing because -Force was provided."
}

if ($UseProjectState -and -not [string]::IsNullOrWhiteSpace($StateDir)) {
    throw "Use either -StateDir or -UseProjectState, not both."
}

$resolvedStateDir = $StateDir
if ($UseProjectState) {
    $resolvedStateDir = Get-RustNorionProjectStateDir -RepoRoot $RepoRoot
} elseif ([string]::IsNullOrWhiteSpace($resolvedStateDir)) {
    $resolvedStateDir = Join-Path $RepoRoot "target\manual-gemma-service\lab-state"
} elseif (-not [System.IO.Path]::IsPathRooted($resolvedStateDir)) {
    $resolvedStateDir = Join-Path $RepoRoot $resolvedStateDir
}

if ($CheckOnly) {
    $snapshotExists = Test-Path -LiteralPath $Snapshot
    $hfCacheExists = Test-Path -LiteralPath $HfCache
    $mistralAlreadyListening = Test-LocalPort -Port $MistralPort
    $backendAlreadyListening = Test-LocalPort -Port $BackendPort
    $labAlreadyListening = Test-LocalPort -Port $LabPort

    Write-Host "Gemma lab preflight:"
    Write-Host "check_only=true"
    Write-Host "starts_process=false"
    Write-Host "builds_binaries=false"
    Write-Host "writes_state=false"
    Write-Host "repo_root=$RepoRoot"
    Write-Host "snapshot=$Snapshot"
    Write-Host "snapshot_exists=$snapshotExists"
    Write-Host "hf_cache=$HfCache"
    Write-Host "hf_cache_exists=$hfCacheExists"
    Write-Host "mistral_port=$MistralPort listening=$mistralAlreadyListening"
    Write-Host "backend_port=$BackendPort listening=$backendAlreadyListening"
    Write-Host "lab_port=$LabPort listening=$labAlreadyListening"
    Write-Host "state_dir=$resolvedStateDir"
    Write-Host "runtime_timeout_ms=$RuntimeTimeoutMs"
    Write-Host "lab_backend_timeout_seconds=$LabBackendTimeoutSeconds"
    Write-Host "max_tokens=$MaxTokens"
    Write-Host "max_seq_len=$MaxSeqLen"
    Write-Host "min_free_ram_gb=$MinFreeRamGB"
    Write-Host "min_free_gpu_gb=$MinFreeGpuGB"

    $backendHealth = $null
    if ($backendAlreadyListening) {
        $backendHealth = Get-HealthJson -Url "http://127.0.0.1:$BackendPort/health"
        if ($null -eq $backendHealth) {
            Write-Warning "backend_health=unreadable"
        } else {
            Write-Host "backend_service=$($backendHealth.service)"
            Write-Host "backend_runtime_mode=$($backendHealth.runtime_mode)"
            Write-Host "backend_gemma_runtime_server=$($backendHealth.gemma_runtime_server)"
        }
    }
    Write-GemmaExperienceSafety -RepoRoot $RepoRoot -ResolvedStateDir $resolvedStateDir -UseProjectState:$UseProjectState -BackendHealth $backendHealth

    if ($labAlreadyListening) {
        $labHealth = Get-HealthJson -Url "http://127.0.0.1:$LabPort/health"
        if ($null -eq $labHealth) {
            Write-Warning "lab_health=unreadable"
        } else {
            Write-Host "lab_service=$($labHealth.service)"
            Write-Host "lab_backend=$($labHealth.backend)"
        }
    }

    if ($mistralAlreadyListening) {
        Write-Host "resource_preflight=skipped_existing_runtime"
    } else {
        $resourceIssues = @(Get-GemmaStartupResourceIssues -MinFreeRamGB $MinFreeRamGB -MinFreeGpuGB $MinFreeGpuGB)
        if ($resourceIssues.Count -eq 0) {
            Write-Host "resource_preflight=ok"
        } else {
            Write-Host "resource_preflight=warning"
            foreach ($issue in $resourceIssues) {
                Write-Warning "  $issue"
            }
        }
    }

    if (-not $snapshotExists) {
        throw "Gemma snapshot not found: $Snapshot"
    }

    return
}

if (-not (Test-Path -LiteralPath $Snapshot)) {
    throw "Gemma snapshot not found: $Snapshot"
}

$mistralAlreadyListening = Test-LocalPort -Port $MistralPort
if (-not $mistralAlreadyListening) {
    Assert-GemmaStartupResources -MinFreeRamGB $MinFreeRamGB -MinFreeGpuGB $MinFreeGpuGB -Force:$Force
} else {
    Write-Host "Gemma runtime already listening; startup preflight skipped."
}

$traceDir = Join-Path $RepoRoot "target\manual-gemma-service"
New-Item -ItemType Directory -Force -Path $traceDir | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"

if (-not $SkipBuild) {
    Push-Location $RepoRoot
    cargo build
    Pop-Location

    Push-Location (Join-Path $RepoRoot "tools\rustgpt-lab")
    cargo build
    Pop-Location
}

Write-Host "Gemma lab timeouts: runtime_timeout_ms=$RuntimeTimeoutMs lab_backend_timeout_seconds=$LabBackendTimeoutSeconds"

if (-not $mistralAlreadyListening) {
    $mistralOut = Join-Path $traceDir "mistralrs-$stamp.out.log"
    $mistralErr = Join-Path $traceDir "mistralrs-$stamp.err.log"
    $mistralArgs = @(
        "serve",
        "--host", "127.0.0.1",
        "--port", $MistralPort.ToString(),
        "--no-ui", "auto",
        "--token-source", "none",
        "--isq", "4",
        "-m", $Snapshot,
        "--hf-cache", $HfCache,
        "--max-seq-len", $MaxSeqLen.ToString(),
        "--paged-attn", "off"
    )
    $mistralProcess = Start-Process -FilePath "mistralrs" -ArgumentList $mistralArgs -WorkingDirectory $RepoRoot -WindowStyle Hidden -RedirectStandardOutput $mistralOut -RedirectStandardError $mistralErr -PassThru
    Write-Host "Starting Gemma 12B runtime on 127.0.0.1:$MistralPort ..."
    Write-Host "mistralrs pid: $($mistralProcess.Id)"
    Write-Host "mistralrs logs: $mistralOut / $mistralErr"
    if (-not (Wait-LocalPort -Port $MistralPort -TimeoutSeconds 240)) {
        throw "mistralrs did not open port $MistralPort. See $mistralErr"
    }
} else {
    Write-Host "Gemma runtime already listening on 127.0.0.1:$MistralPort"
}

if (-not (Test-LocalPort -Port $BackendPort)) {
    $backendWorkingDirectory = $RepoRoot
    $trace = Join-Path $traceDir "trace-http-runtime-$stamp.jsonl"
    $stateArgs = @()
    if (-not [string]::IsNullOrWhiteSpace($resolvedStateDir)) {
        New-Item -ItemType Directory -Force -Path $resolvedStateDir | Out-Null
        $backendWorkingDirectory = $resolvedStateDir
        $trace = Join-Path $resolvedStateDir "trace-http-runtime-$stamp.jsonl"
        $stateArgs = @(
            "--memory", (Join-Path $resolvedStateDir "memory.ndkv"),
            "--experience", (Join-Path $resolvedStateDir "experience.ndkv"),
            "--adaptive", (Join-Path $resolvedStateDir "adaptive.ndkv")
        )
    }

    $backendOut = Join-Path $traceDir "rust-norion-$stamp.out.log"
    $backendErr = Join-Path $traceDir "rust-norion-$stamp.err.log"
    $backendArgs = @(
        "--serve",
        "--serve-bind", "127.0.0.1:$BackendPort",
        "--gemma-local-snapshot", $Snapshot,
        "--gemma-runtime-server", "http://127.0.0.1:$MistralPort",
        "--runtime-native-window", $MaxSeqLen.ToString(),
        "--max-tokens", $MaxTokens.ToString(),
        "--runtime-timeout-ms", $RuntimeTimeoutMs.ToString(),
        "--trace", $trace
    )
    $backendArgs += $stateArgs
    $backendArgs += @("manual gemma persistent service")
    $backendExe = Join-Path $RepoRoot "target\debug\rust-norion.exe"
    $backendProcess = Start-Process -FilePath $backendExe -ArgumentList $backendArgs -WorkingDirectory $backendWorkingDirectory -WindowStyle Hidden -RedirectStandardOutput $backendOut -RedirectStandardError $backendErr -PassThru
    Write-Host "rust-norion pid: $($backendProcess.Id)"
    Write-Host "rust-norion logs: $backendOut / $backendErr"
    if (-not [string]::IsNullOrWhiteSpace($resolvedStateDir)) {
        Write-Host "rust-norion state: $backendWorkingDirectory"
        Write-Host "rust-norion experience: $(Join-Path $resolvedStateDir "experience.ndkv")"
    }
    if (-not (Wait-LocalPort -Port $BackendPort -TimeoutSeconds 30)) {
        throw "rust-norion did not open port $BackendPort"
    }
} else {
    Write-Host "rust-norion already listening on 127.0.0.1:$BackendPort"
    $backendHealth = Assert-ExistingBackendIsGemma -Port $BackendPort -MistralPort $MistralPort
    Assert-BackendStateMatches -Health $backendHealth -ResolvedStateDir $resolvedStateDir
}

if (-not (Test-LocalPort -Port $LabPort)) {
    $labExe = Join-Path $RepoRoot "tools\rustgpt-lab\target\debug\rustgpt-lab.exe"
    $labArgs = @(
        "--bind", "127.0.0.1:$LabPort",
        "--backend", "127.0.0.1:$BackendPort",
        "--backend-timeout-secs", $LabBackendTimeoutSeconds.ToString()
    )
    $labOut = Join-Path $traceDir "rustgpt-lab-$stamp.out.log"
    $labErr = Join-Path $traceDir "rustgpt-lab-$stamp.err.log"
    $labProcess = Start-Process -FilePath $labExe -ArgumentList $labArgs -WorkingDirectory (Join-Path $RepoRoot "tools\rustgpt-lab") -WindowStyle Hidden -RedirectStandardOutput $labOut -RedirectStandardError $labErr -PassThru
    Write-Host "rustgpt-lab pid: $($labProcess.Id)"
    Write-Host "rustgpt-lab logs: $labOut / $labErr"
    if (-not (Wait-LocalPort -Port $LabPort -TimeoutSeconds 30)) {
        throw "rustgpt-lab did not open port $LabPort"
    }
} else {
    Write-Host "rustgpt-lab already listening on 127.0.0.1:$LabPort"
    Assert-ExistingLabMatches -Port $LabPort -BackendPort $BackendPort
}

Write-Host "Gemma lab ready: http://127.0.0.1:$LabPort/"
Write-Host "Backend health: http://127.0.0.1:$BackendPort/health"
Write-Host "Status check: .\tools\rustgpt-lab\status-gemma-lab.cmd"
