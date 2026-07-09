param(
    [string]$RepoRoot = "D:\rust-norion",
    [string]$RemoteHost = "192.168.10.11",
    [string]$RemoteUser = "xinghuan",
    [string]$IdentityFile = "$env:USERPROFILE\.ssh\smartsteam_mac_ed25519",
    [string]$RemoteRoot = "/Users/xinghuan/smartsteam-model-box",
    [string]$RemoteLlamaServer = "/Users/xinghuan/smartsteam-model-box/bin/llama-b9616/llama-server",
    [string]$RemoteModel = "/Users/xinghuan/smartsteam-model-box/models/Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf",
    [string]$RemoteSmallLlamaServer = "",
    [string]$RemoteSmallModel = "",
    [string]$RemoteSummaryLlamaServer = "",
    [string]$RemoteSummaryModel = "",
    [string]$RemoteRouterLlamaServer = "",
    [string]$RemoteRouterModel = "",
    [string]$RemoteReviewLlamaServer = "",
    [string]$RemoteReviewModel = "",
    [string]$RemoteTestGateLlamaServer = "",
    [string]$RemoteTestGateModel = "",
    [string]$RemoteIndexLlamaServer = "",
    [string]$RemoteIndexModel = "",
    [int]$RemoteModelPort = 8686,
    [int]$LocalModelPort = 8686,
    [string]$RuntimeModelId = "Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf",
    [string]$PoolWorkerRoles = "summary,router,review,index,test-gate",
    [string]$RequiredPoolWorkerRoles = "",
    [int]$SummaryPort = 8687,
    [int]$ReviewPort = 8688,
    [int]$RouterPort = 8689,
    [int]$TestGatePort = 8688,
    [int]$IndexPort = 8690,
    [int]$BackendPort = 7878,
    [int]$LabPort = 8787,
    [int]$ContextTokens = 65536,
    [int]$DefaultMaxTokens = 4096,
    [int]$SummaryContextTokens = 8192,
    [int]$RouterContextTokens = 4096,
    [int]$ReviewContextTokens = 8192,
    [int]$TestGateContextTokens = 4096,
    [int]$IndexContextTokens = 4096,
    [int]$GpuLayers = 999,
    [int]$SmallGpuLayers = 999,
    [int]$SummaryGpuLayers = -1,
    [int]$RouterGpuLayers = -1,
    [int]$ReviewGpuLayers = -1,
    [int]$TestGateGpuLayers = -1,
    [int]$IndexGpuLayers = 999,
    [string]$SmallDevice = "",
    [string]$SummaryDevice = "",
    [string]$RouterDevice = "",
    [string]$ReviewDevice = "",
    [string]$TestGateDevice = "",
    [string]$IndexDevice = "",
    [int]$SmallParallelSlots = 1,
    [int]$SmallBatchSize = 256,
    [int]$SmallUbatchSize = 64,
    [int]$SmallCacheRamMiB = 0,
    [ValidateSet("on", "off", "auto")]
    [string]$Reasoning = "off",
    [ValidateSet("on", "off", "auto")]
    [string]$SmallReasoning = "off",
    [int]$RuntimeTimeoutMs = 900000,
    [int]$LabBackendTimeoutSeconds = 0,
    [string]$ModelPoolManifest = "",
    [string]$ModelCacheStatusJson = "",
    [string]$RunDir = "",
    [switch]$SkipBuild,
    [switch]$NoBackend,
    [switch]$NoLab,
    [switch]$LaunchForge,
    [switch]$EnablePoolWorkers,
    [switch]$EnableIndexWorker,
    [switch]$EnableSpareWorker,
    [switch]$AllowLargePoolWorkerModels,
    [switch]$RestartRemote,
    [switch]$NoTunnel,
    [switch]$Status,
    [switch]$Watch,
    [switch]$CheckOnly,
    [int]$WatchIntervalSeconds = 5,
    [int]$WatchCount = 0,
    [switch]$JsonStatus,
    [switch]$ProbeRemoteRuntime,
    [string]$OutputJson = "",
    [switch]$FailOnNotReady,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Start the remote Gemma model box, SSH tunnel, local rust-norion backend, and Web Lab."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-chain.cmd"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-chain.cmd -ContextTokens 262144 -DefaultMaxTokens 262144"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-chain.cmd -ContextTokens 8192 -DefaultMaxTokens 4096 -RuntimeTimeoutMs 600000"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789 -ModelPoolManifest .\target\gemma-chain\apple-model-pool.generated.json -LaunchForge"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-chain.cmd -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-chain.cmd -EnablePoolWorkers -RemoteSummaryModel /models/summary-Q4.gguf -RemoteRouterModel /models/function-router-Q4.gguf -RemoteReviewModel /models/review-Q4.gguf -RemoteTestGateModel /models/test-gate-Q4.gguf"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-chain.cmd -LocalModelPort 8696 -BackendPort 7979 -LabPort 8789"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-chain.cmd -EnablePoolWorkers -CheckOnly"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-chain.cmd -Status"
    Write-Host "  .\tools\smartsteam-forge\status-remote-gemma-chain.cmd -JsonStatus -BackendPort 7979 -LabPort 8789"
    Write-Host "  .\tools\smartsteam-forge\status-remote-gemma-chain.cmd -JsonStatus -ProbeRemoteRuntime -BackendPort 7979 -LabPort 8789"
    Write-Host "  .\tools\smartsteam-forge\status-remote-gemma-chain.cmd -JsonStatus -OutputJson target\remote-gemma-chain\status-with-model-cache.json"
    Write-Host "  .\tools\smartsteam-forge\status-remote-gemma-chain.cmd -JsonStatus -FailOnNotReady"
    Write-Host "  .\tools\smartsteam-forge\status-remote-gemma-chain.cmd -Watch -WatchIntervalSeconds 5"
    Write-Host "  .\tools\smartsteam-forge\status-remote-gemma-chain.cmd -Watch -WatchIntervalSeconds 2 -WatchCount 3"
    Write-Host ""
    Write-Host "Notes:"
    Write-Host "  Uses SSH key auth only; no password or Hugging Face token is stored by this script."
    Write-Host "  The remote Mac only needs llama-server and the GGUF model; Rust source stays local."
    Write-Host "  Default 7878/8787 can be overridden with 7979/8789 to run beside another local stack."
    Write-Host "  -ModelPoolManifest is passed only to the local rust-norion backend."
    Write-Host "  -EnablePoolWorkers starts optional small workers on 8687-8690; use -RemoteSmallModel or per-role Remote*Model overrides."
    Write-Host "  Helper workers reject the quality model path and obvious 12B+ model names by default; pass -AllowLargePoolWorkerModels only for an explicit stress test."
    Write-Host "  -EnableIndexWorker explicitly includes 8690/index; -EnableSpareWorker is a deprecated alias."
    Write-Host "  -LaunchForge opens SmartSteam Forge after backend readiness preflight."
    Write-Host "  -CheckOnly validates local pool inputs/manifest selection and exits before SSH/start/prompt."
    Write-Host "  -Status is read-only and also shows backend /v1/model-pool/status when available."
    Write-Host "  -ModelCacheStatusJson points status at sync-remote-gemma-model-cache provenance, default RunDir\model-cache-status.json."
    Write-Host "  -JsonStatus emits local-only machine JSON for automation; it does not SSH or start processes unless -ProbeRemoteRuntime is explicit."
    Write-Host "  -FailOnNotReady is for -JsonStatus monitors: it prints JSON first, then exits nonzero when readiness.ready=false."
    Write-Host "  -ProbeRemoteRuntime adds read-only SSH/lsof/ps launch flags to JSON so automation can see actual GPU/CPU placement."
    Write-Host "  -OutputJson writes the -JsonStatus snapshot to a file while still echoing JSON to stdout."
    Write-Host "  -Watch repeats -Status until Ctrl+C; -WatchCount limits iterations for smoke checks."
    Write-Host "  If local 8686 is occupied, pass -LocalModelPort 8696 and use the same port for -Status."
    Write-Host ""
    Write-Host "Remote files expected:"
    Write-Host "  $RemoteLlamaServer"
    Write-Host "  $RemoteModel"
    return
}

if ([string]::IsNullOrWhiteSpace($RunDir)) {
    $RunDir = Join-Path $RepoRoot "target\remote-gemma-chain"
}

if ($LabBackendTimeoutSeconds -le 0) {
    $LabBackendTimeoutSeconds = [Math]::Max(900, [int][Math]::Ceiling($RuntimeTimeoutMs / 1000.0) + 5)
}

$BuildDir = Join-Path $RunDir "build"
$StateDir = Join-Path $RunDir "state"
$LogDir = Join-Path $RunDir "logs"
New-Item -ItemType Directory -Force -Path $RunDir, $BuildDir, $StateDir, $LogDir | Out-Null
if ([string]::IsNullOrWhiteSpace($ModelCacheStatusJson)) {
    $ModelCacheStatusJson = Join-Path $RunDir "model-cache-status.json"
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

function Wait-HttpOk {
    param(
        [string]$Url,
        [int]$TimeoutSec = 60
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

function Test-ModelApi {
    param([int]$Port)

    try {
        $response = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/v1/models" -UseBasicParsing -TimeoutSec 12
        if ($response.StatusCode -lt 200 -or $response.StatusCode -ge 300) {
            return $false
        }
        $content = [string]$response.Content
        try {
            $json = $content | ConvertFrom-Json -ErrorAction Stop
            return ($null -ne $json.models) -or ($null -ne $json.data)
        } catch {
            return $content.Contains('"models"') -or $content.Contains('"object":"list"')
        }
    } catch {
        return $false
    }
}

function Get-JsonProperty {
    param(
        [object]$Object,
        [string]$Name
    )
    if ($null -eq $Object) {
        return $null
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return $property.Value
}

function Assert-ModelPoolManifestAdvice {
    param(
        [string]$Path,
        [string[]]$ExpectedHelperRoles,
        [switch]$RequireHelpers
    )

    $manifest = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    $manifestWorkers = @($manifest.workers | Where-Object { $_ })
    if ($manifestWorkers.Count -lt 1) {
        throw "ModelPoolManifest requires workers"
    }
    $advice = Get-JsonProperty -Object $manifest -Name "advice"
    if ($null -eq $advice) {
        throw "ModelPoolManifest requires advice generated by model-pool-advice-core"
    }
    $decisionSource = Get-JsonProperty -Object $advice -Name "decision_source"
    if ([string]::IsNullOrWhiteSpace([string]$decisionSource)) {
        throw "ModelPoolManifest advice.decision_source is required"
    }
    if ($decisionSource -ne "model-pool-advice-core") {
        throw "ModelPoolManifest advice.decision_source=$decisionSource, expected model-pool-advice-core"
    }
    if ((Get-JsonProperty -Object $advice -Name "safe_to_enable_pool_workers") -ne $true) {
        throw "ModelPoolManifest advice blocks helper expansion: next_step=$((Get-JsonProperty -Object $advice -Name "next_step")) reason=$((Get-JsonProperty -Object $advice -Name "reason"))"
    }
    if ((Get-JsonProperty -Object $advice -Name "extra_quality_12b_detected") -eq $true) {
        throw "ModelPoolManifest advice detects extra quality 12B workers; keep one quality model and small helpers"
    }
    if ([int](Get-JsonProperty -Object $advice -Name "quality_worker_count") -ne 1) {
        throw "ModelPoolManifest advice.quality_worker_count=$((Get-JsonProperty -Object $advice -Name "quality_worker_count")), expected 1"
    }
    $workerShape = Get-JsonProperty -Object $advice -Name "worker_shape"
    if ($null -eq $workerShape) {
        throw "ModelPoolManifest advice.worker_shape is required"
    }
    if ([int](Get-JsonProperty -Object $workerShape -Name "quality") -ne 1) {
        throw "ModelPoolManifest advice.worker_shape.quality=$((Get-JsonProperty -Object $workerShape -Name "quality")), expected 1"
    }
    $qualityWorkers = @($manifestWorkers | Where-Object { [string]$_.role -eq "quality" })
    if ($qualityWorkers.Count -ne 1) {
        throw "ModelPoolManifest requires exactly one quality worker, found $($qualityWorkers.Count)"
    }
    Assert-ModelPoolManifestWorkerRuntime -Worker $qualityWorkers[0] -Role "quality" -RequireAccelerated
    if ($RequireHelpers) {
        $helperRoles = @($ExpectedHelperRoles | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
        $helperTarget = [int](Get-JsonProperty -Object $workerShape -Name "helper_target")
        if ($helperTarget -ne $helperRoles.Count) {
            throw "ModelPoolManifest advice.worker_shape.helper_target=$helperTarget, expected $($helperRoles.Count)"
        }
        $helpersVisible = [int](Get-JsonProperty -Object $workerShape -Name "helpers_visible")
        if ($helpersVisible -lt $helperRoles.Count) {
            throw "ModelPoolManifest advice.worker_shape.helpers_visible=$helpersVisible, expected at least $($helperRoles.Count)"
        }
        $manifestAdviceHelperRoles = @((Get-JsonProperty -Object $advice -Name "helper_roles"))
        foreach ($role in $helperRoles) {
            if ($role -notin $manifestAdviceHelperRoles) {
                throw "ModelPoolManifest advice.helper_roles does not include requested role=$role"
            }
            $roleWorkers = @($manifestWorkers | Where-Object { [string]$_.role -eq $role })
            if ($roleWorkers.Count -ne 1) {
                throw "ModelPoolManifest requires exactly one worker role=$role, found $($roleWorkers.Count)"
            }
            Assert-ModelPoolManifestWorkerRuntime -Worker $roleWorkers[0] -Role $role -RequireAccelerated
        }
    }
}

function Assert-ModelPoolManifestWorkerRuntime {
    param(
        [object]$Worker,
        [string]$Role,
        [switch]$RequireAccelerated
    )

    $runtimeBackend = [string](Get-JsonProperty -Object $Worker -Name "runtime_backend")
    if ([string]::IsNullOrWhiteSpace($runtimeBackend)) {
        throw "ModelPoolManifest worker role=$Role requires runtime_backend metadata"
    }
    if ($runtimeBackend -ne "llama.cpp") {
        throw "ModelPoolManifest worker role=$Role runtime_backend=$runtimeBackend, expected llama.cpp"
    }
    $runtimeDevice = [string](Get-JsonProperty -Object $Worker -Name "runtime_device")
    if ([string]::IsNullOrWhiteSpace($runtimeDevice)) {
        throw "ModelPoolManifest worker role=$Role requires runtime_device metadata"
    }
    $runtimeAccelerator = [string](Get-JsonProperty -Object $Worker -Name "runtime_accelerator")
    if ([string]::IsNullOrWhiteSpace($runtimeAccelerator)) {
        throw "ModelPoolManifest worker role=$Role requires runtime_accelerator metadata"
    }
    $gpuLayersValue = Get-JsonProperty -Object $Worker -Name "gpu_layers"
    if ($null -eq $gpuLayersValue -or [string]::IsNullOrWhiteSpace([string]$gpuLayersValue)) {
        throw "ModelPoolManifest worker role=$Role requires gpu_layers metadata"
    }
    if ($RequireAccelerated) {
        if ($runtimeDevice -ne "metal" -or $runtimeAccelerator -ne "metal") {
            throw "ModelPoolManifest worker role=$Role must be Metal accelerated, got runtime_device=$runtimeDevice runtime_accelerator=$runtimeAccelerator"
        }
        if ([int]$gpuLayersValue -le 0) {
            throw "ModelPoolManifest worker role=$Role gpu_layers=$gpuLayersValue, expected > 0"
        }
    }
}

function Get-DefaultModelPoolManifestPath {
    $manifestDir = Join-Path $RepoRoot "target\gemma-chain"
    return (Join-Path $manifestDir "apple-model-pool.generated.json")
}

function Get-OrUnknown {
    param([object]$Value)
    if ($null -ne $Value) {
        return $Value
    }
    return "unknown"
}

function Get-BoolOrUnknown {
    param([object]$Value)
    if ($null -eq $Value) {
        return "unknown"
    }
    if ($Value -eq $true) {
        return "true"
    }
    if ($Value -eq $false) {
        return "false"
    }
    return [string]$Value
}

function Get-HttpJson {
    param([string]$Url)
    try {
        return Invoke-RestMethod -Uri $Url -TimeoutSec 3
    } catch {
        return $null
    }
}

function Resolve-StatusArtifactPath {
    param([string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return Join-Path $RepoRoot $Path
}

function Read-ModelCacheStatus {
    $path = Resolve-StatusArtifactPath $ModelCacheStatusJson
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        return [pscustomobject]@{
            path = $path
            exists = $false
            parse_error = ""
            contract_version = $null
            read_only = $null
            all_ok = $null
            model_count = 0
            ok_count = 0
            remote_error_count = 0
            remote = $null
            models = @()
        }
    }

    try {
        $status = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    } catch {
        return [pscustomobject]@{
            path = $path
            exists = $true
            parse_error = $_.Exception.Message
            contract_version = $null
            read_only = $null
            all_ok = $false
            model_count = 0
            ok_count = 0
            remote_error_count = 0
            remote = $null
            models = @()
        }
    }

    $models = @($status.models | Where-Object { $_ })
    return [pscustomobject]@{
        path = $path
        exists = $true
        parse_error = ""
        contract_version = Get-JsonProperty -Object $status -Name "contract_version"
        read_only = Get-JsonProperty -Object $status -Name "read_only"
        all_ok = Get-JsonProperty -Object $status -Name "all_ok"
        model_count = $models.Count
        ok_count = @($models | Where-Object { (Get-JsonProperty -Object $_ -Name "ok") -eq $true }).Count
        remote_error_count = @($models | Where-Object { -not [string]::IsNullOrWhiteSpace([string](Get-JsonProperty -Object $_ -Name "remote_error")) }).Count
        remote = Get-JsonProperty -Object $status -Name "remote"
        models = @($models | ForEach-Object {
            [pscustomobject]@{
                role = Get-JsonProperty -Object $_ -Name "role"
                name = Get-JsonProperty -Object $_ -Name "name"
                ok = Get-JsonProperty -Object $_ -Name "ok"
                local_bytes = Get-JsonProperty -Object $_ -Name "local_bytes"
                remote_bytes = Get-JsonProperty -Object $_ -Name "remote_bytes"
                size_matches = Get-JsonProperty -Object $_ -Name "size_matches"
                sha256_matches = Get-JsonProperty -Object $_ -Name "sha256_matches"
                local_sha256 = Get-JsonProperty -Object $_ -Name "local_sha256"
                remote_sha256 = Get-JsonProperty -Object $_ -Name "remote_sha256"
                remote_path = Get-JsonProperty -Object $_ -Name "remote_path"
                remote_error = Get-JsonProperty -Object $_ -Name "remote_error"
            }
        })
    }
}

function Write-ModelCacheStatus {
    param([object]$Status)
    if ($null -eq $Status -or -not $Status.exists) {
        Write-Host "remote model-cache provenance: missing path=$ModelCacheStatusJson"
        Write-Host "remote model-cache next_step: .\tools\smartsteam-forge\sync-remote-gemma-model-cache.cmd -CheckOnly -JsonStatus -OutputJson $ModelCacheStatusJson"
        return
    }
    if (-not [string]::IsNullOrWhiteSpace([string]$Status.parse_error)) {
        Write-Host "remote model-cache provenance: parse_error path=$($Status.path) error=$($Status.parse_error)"
        return
    }
    $remoteHost = if ($null -eq $Status.remote) { "unknown" } else { Get-OrUnknown -Value (Get-JsonProperty -Object $Status.remote -Name "host") }
    $remoteDir = if ($null -eq $Status.remote) { "unknown" } else { Get-OrUnknown -Value (Get-JsonProperty -Object $Status.remote -Name "model_dir") }
    Write-Host ("remote model-cache provenance: all_ok={0} ok={1}/{2} remote_errors={3} read_only={4} path={5}" -f `
        (Get-BoolOrUnknown -Value $Status.all_ok), `
        (Get-OrUnknown -Value $Status.ok_count), `
        (Get-OrUnknown -Value $Status.model_count), `
        (Get-OrUnknown -Value $Status.remote_error_count), `
        (Get-BoolOrUnknown -Value $Status.read_only), `
        $Status.path)
    Write-Host ("remote model-cache target: host={0} model_dir={1}" -f $remoteHost, $remoteDir)
    if (@($Status.models).Count -gt 0) {
        Write-Host "remote model-cache models:"
        $Status.models |
            Select-Object role, name, ok, size_matches, sha256_matches, local_bytes, remote_bytes, remote_path |
            Format-Table -AutoSize
    }
}

function Get-StatusReason {
    param([object]$Status)
    $reason = Get-JsonProperty -Object $Status -Name "reason"
    if ($null -eq $reason) {
        $reason = Get-JsonProperty -Object $Status -Name "launch_block_reason"
    }
    return $reason
}

function Write-ModelPoolStatus {
    param([object]$Status)
    if ($null -eq $Status) {
        Write-Host "backend model-pool: unavailable"
        return
    }

    Write-Host ("backend model-pool: launch_allowed={0} reason={1} workers={2} healthy={3} min_context_tokens={4}" -f `
        (Get-OrUnknown -Value $Status.launch_allowed), `
        (Get-OrUnknown -Value (Get-StatusReason -Status $Status)), `
        (Get-OrUnknown -Value $Status.worker_count), `
        (Get-OrUnknown -Value $Status.healthy_worker_count), `
        (Get-OrUnknown -Value $Status.min_context_tokens))

    $capacity = Get-JsonProperty -Object $Status -Name "capacity"
    if ($capacity) {
        Write-Host ("backend model-pool capacity: policy={0} expansion_allowed={1} recommendation={2} helpers={3}/{4} runtime=metal:{5} cpu:{6} unknown:{7} gpu0:{8} quality_accelerated={9}" -f `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $capacity -Name "policy")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $capacity -Name "expansion_allowed")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $capacity -Name "recommendation")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $capacity -Name "healthy_helper_worker_count")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $capacity -Name "helper_worker_count")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $capacity -Name "metal_worker_count")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $capacity -Name "cpu_worker_count")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $capacity -Name "unknown_runtime_worker_count")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $capacity -Name "zero_gpu_layer_worker_count")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $capacity -Name "quality_runtime_accelerated")))
        Write-Host "backend model-pool runtime metadata: manifest/backend view; compare remote launch flags below for actual process placement."
    }

    $routeMetrics = Get-JsonProperty -Object $Status -Name "route_metrics"
    if ($routeMetrics) {
        Write-Host ("backend model-pool route metrics: routes={0} selected={1} blocked={2} in_flight={3} success={4} failure={5} avg_latency_ms={6}" -f `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $routeMetrics -Name "route_count")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $routeMetrics -Name "selected_count")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $routeMetrics -Name "blocked_count")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $routeMetrics -Name "in_flight")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $routeMetrics -Name "success_count")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $routeMetrics -Name "failure_count")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $routeMetrics -Name "avg_latency_ms")))
    }

    $workers = @($Status.workers) | Where-Object { $_ }
    if ($workers.Count -gt 0) {
        Write-Host "backend model-pool workers:"
        $workers |
            Select-Object role, status, ready, base_url, context_window, default_max_tokens, in_flight, route_count, selected_count, blocked_count, success_count, failure_count, avg_latency_ms, role_block_reason |
            Format-Table -AutoSize
    }
}

function Get-QualityWorkerReady {
    param([object]$Status)
    if ($null -eq $Status) {
        return $null
    }
    $quality = @($Status.workers | Where-Object { $_.role -eq "quality" } | Select-Object -First 1)
    if ($quality.Count -eq 0) {
        return $null
    }
    $worker = $quality[0]
    foreach ($name in @("ready", "role_ready", "health_ok")) {
        $value = Get-JsonProperty -Object $worker -Name $name
        if ($value -eq $true) {
            return $true
        }
    }
    foreach ($name in @("ready", "role_ready", "health_ok")) {
        $value = Get-JsonProperty -Object $worker -Name $name
        if ($value -eq $false) {
            return $false
        }
    }
    return $null
}

function Normalize-StatusWorkerRole {
    param([string]$Role)
    $normalized = $Role.Trim().ToLowerInvariant()
    if ($normalized -eq "spare") {
        return "index"
    }
    if ($normalized -in @("route", "intent", "intent-classify", "preflight", "tool-call", "tool_call", "tool-calls", "tool_calls", "function", "function-call", "function_call")) {
        return "router"
    }
    return $normalized
}

function Get-RequiredPoolWorkerRoleNames {
    if ([string]::IsNullOrWhiteSpace($RequiredPoolWorkerRoles)) {
        return @()
    }
    $seen = @{}
    $roles = @()
    foreach ($role in @($RequiredPoolWorkerRoles -split ",")) {
        $normalized = Normalize-StatusWorkerRole $role
        if ([string]::IsNullOrWhiteSpace($normalized)) {
            continue
        }
        if (-not $seen.ContainsKey($normalized)) {
            $seen[$normalized] = $true
            $roles += $normalized
        }
    }
    return $roles
}

function Get-WorkerReadyValue {
    param([object]$Worker)
    if ($null -eq $Worker) {
        return $null
    }
    foreach ($name in @("ready", "role_ready", "health_ok")) {
        $value = Get-JsonProperty -Object $Worker -Name $name
        if ($value -eq $true) {
            return $true
        }
    }
    foreach ($name in @("ready", "role_ready", "health_ok")) {
        $value = Get-JsonProperty -Object $Worker -Name $name
        if ($value -eq $false) {
            return $false
        }
    }
    return $null
}

function Get-ModelCacheRoleForWorker {
    param([string]$Role)
    $normalized = Normalize-StatusWorkerRole $Role
    if ($normalized -eq "test-gate") {
        return "review"
    }
    return $normalized
}

function Get-ModelCacheRowForWorker {
    param(
        [object]$ModelCacheStatus,
        [string]$Role
    )
    if ($null -eq $ModelCacheStatus -or -not $ModelCacheStatus.exists) {
        return $null
    }
    $cacheRole = Get-ModelCacheRoleForWorker $Role
    return @($ModelCacheStatus.models | Where-Object { [string]$_.role -eq $cacheRole } | Select-Object -First 1)
}

function New-RemoteChainWorkerSummaries {
    param(
        [object]$Status,
        [object]$ModelCacheStatus
    )
    if ($null -eq $Status) {
        return @()
    }
    return @($Status.workers | Where-Object { $_ } | ForEach-Object {
        $role = Get-JsonProperty -Object $_ -Name "role"
        $modelCacheRole = Get-ModelCacheRoleForWorker $role
        $modelCacheRow = Get-ModelCacheRowForWorker -ModelCacheStatus $ModelCacheStatus -Role $role
        [pscustomobject]@{
            role = $role
            port = Get-JsonProperty -Object $_ -Name "port"
            base_url = Get-JsonProperty -Object $_ -Name "base_url"
            status = Get-JsonProperty -Object $_ -Name "status"
            ready = Get-WorkerReadyValue -Worker $_
            context_window = Get-JsonProperty -Object $_ -Name "context_window"
            default_max_tokens = Get-JsonProperty -Object $_ -Name "default_max_tokens"
            runtime_backend = Get-JsonProperty -Object $_ -Name "runtime_backend"
            runtime_device = Get-JsonProperty -Object $_ -Name "runtime_device"
            runtime_accelerator = Get-JsonProperty -Object $_ -Name "runtime_accelerator"
            gpu_layers = Get-JsonProperty -Object $_ -Name "gpu_layers"
            role_block_reason = Get-JsonProperty -Object $_ -Name "role_block_reason"
            model_cache_role = $modelCacheRole
            model_cache_name = if ($null -eq $modelCacheRow) { $null } else { Get-JsonProperty -Object $modelCacheRow -Name "name" }
            model_cache_ok = if ($null -eq $modelCacheRow) { $null } else { Get-JsonProperty -Object $modelCacheRow -Name "ok" }
            model_cache_size_matches = if ($null -eq $modelCacheRow) { $null } else { Get-JsonProperty -Object $modelCacheRow -Name "size_matches" }
            model_cache_sha256_matches = if ($null -eq $modelCacheRow) { $null } else { Get-JsonProperty -Object $modelCacheRow -Name "sha256_matches" }
            model_cache_remote_path = if ($null -eq $modelCacheRow) { $null } else { Get-JsonProperty -Object $modelCacheRow -Name "remote_path" }
        }
    })
}

function Get-RequiredPoolWorkerReadiness {
    param(
        [object]$Status,
        [string[]]$RequiredRoles
    )
    $roles = @($RequiredRoles | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($roles.Count -eq 0) {
        return [pscustomobject]@{
            required_roles = @()
            required_roles_ready = $null
            missing_required_roles = @()
        }
    }
    $missing = @()
    foreach ($role in $roles) {
        $workers = @()
        if ($null -ne $Status) {
            $workers = @($Status.workers | Where-Object { (Normalize-StatusWorkerRole ([string]$_.role)) -eq $role } | Select-Object -First 1)
        }
        if ($workers.Count -eq 0) {
            $missing += $role
            continue
        }
        $ready = Get-WorkerReadyValue -Worker $workers[0]
        if ($ready -ne $true) {
            $missing += $role
        }
    }
    return [pscustomobject]@{
        required_roles = $roles
        required_roles_ready = ($missing.Count -eq 0)
        missing_required_roles = $missing
    }
}

function Get-CapacityExpansionAllowed {
    param([object]$Status)
    $capacity = Get-JsonProperty -Object $Status -Name "capacity"
    if ($null -eq $capacity) {
        return $null
    }
    return Get-JsonProperty -Object $capacity -Name "expansion_allowed"
}

function Get-CapacityRecommendation {
    param([object]$Status)
    $capacity = Get-JsonProperty -Object $Status -Name "capacity"
    if ($null -eq $capacity) {
        return $null
    }
    return Get-JsonProperty -Object $capacity -Name "recommendation"
}

function Get-RemoteChainNextStep {
    param(
        [bool]$ModelApiHealthy,
        [bool]$BackendListening,
        [bool]$LabListening,
        [object]$PoolStatus
    )

    $base = ".\tools\smartsteam-forge\start-remote-gemma-forge.cmd -BackendPort $BackendPort -LabPort $LabPort"
    $tunnelRepair = ".\tools\smartsteam-forge\start-remote-gemma-chain.cmd -BackendPort $BackendPort -LabPort $LabPort -LocalModelPort $LocalModelPort -SkipBuild -NoBackend -NoLab"
    if (-not $ModelApiHealthy) {
        if ($BackendListening -and $LabListening) {
            return $tunnelRepair
        }
        return "$base -SkipBuild -NoForge"
    }
    if (-not $BackendListening) {
        return "$base -SkipBuild -NoForge"
    }
    if (-not $LabListening) {
        return "$base -SkipBuild -NoForge"
    }
    if ($null -eq $PoolStatus) {
        return ".\tools\smartsteam-forge\status-remote-gemma-chain.cmd -BackendPort $BackendPort -LabPort $LabPort -Watch -WatchIntervalSeconds 5"
    }
    $launchAllowed = Get-JsonProperty -Object $PoolStatus -Name "launch_allowed"
    $capacityAllowed = Get-CapacityExpansionAllowed -Status $PoolStatus
    if ($launchAllowed -ne $true) {
        return "$base -SkipBuild -NoForge"
    }
    if ($capacityAllowed -eq $false) {
        $recommendation = Get-CapacityRecommendation -Status $PoolStatus
        if ([string]::IsNullOrWhiteSpace([string]$recommendation)) {
            return "ready_capacity_limited:no_more_workers"
        }
        return "ready_capacity_limited:no_more_workers:$recommendation"
    }
    if ($null -eq $capacityAllowed) {
        return ".\tools\smartsteam-forge\status-remote-gemma-chain.cmd -BackendPort $BackendPort -LabPort $LabPort"
    }
    return "ready: open http://127.0.0.1:$LabPort/ or run .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -SkipBuild"
}

function Write-RemoteChainReadiness {
    param(
        [bool]$ModelApiHealthy,
        [bool]$BackendListening,
        [bool]$LabListening,
        [object]$PoolStatus
    )

    $qualityWorkerReady = Get-QualityWorkerReady -Status $PoolStatus
    $launchAllowed = if ($null -eq $PoolStatus) { $null } else { Get-JsonProperty -Object $PoolStatus -Name "launch_allowed" }
    $capacityAllowed = Get-CapacityExpansionAllowed -Status $PoolStatus
    $chainReady = $ModelApiHealthy -and $BackendListening -and $LabListening -and ($launchAllowed -eq $true)
    Write-Host ("remote_chain_readiness: ready={0} model_api={1} backend={2} web_lab={3} quality_worker={4} model_pool_launch_allowed={5} capacity_expansion_allowed={6}" -f `
        (Get-BoolOrUnknown -Value $chainReady), `
        (Get-BoolOrUnknown -Value $ModelApiHealthy), `
        (Get-BoolOrUnknown -Value $BackendListening), `
        (Get-BoolOrUnknown -Value $LabListening), `
        (Get-BoolOrUnknown -Value $qualityWorkerReady), `
        (Get-BoolOrUnknown -Value $launchAllowed), `
        (Get-BoolOrUnknown -Value $capacityAllowed))
    Write-Host ("remote_chain_next_step: {0}" -f (Get-RemoteChainNextStep -ModelApiHealthy $ModelApiHealthy -BackendListening $BackendListening -LabListening $LabListening -PoolStatus $PoolStatus))
}

function Get-RemoteRuntimeLaunchStatus {
    if (-not $ProbeRemoteRuntime) {
        return [pscustomobject]@{
            probed = $false
            read_only = $true
            starts_process = $false
            sends_prompt = $false
            touches_remote = $false
            error = ""
            worker_count = 0
            cpu_or_no_gpu_count = 0
            cpu_or_no_gpu_roles = @()
            backend_metadata_may_differ_roles = @()
            acceleration_ok = $null
            acceleration_next_step = ""
            workers = @()
        }
    }

    $remoteWorkerItems = @((Get-PoolWorkerDefinitions -All) | ForEach-Object { "$($_.Role):$($_.Port)" }) -join " "
    $remoteProbe = @'
set -eu
print_row() {
  ROLE="$1"
  PORT="$2"
  PID="$(lsof -nP -iTCP:"$PORT" -sTCP:LISTEN -t 2>/dev/null | head -n 1 || true)"
  GPU_LAYERS="unavailable"
  DEVICE="unavailable"
  KV_OFFLOAD="unknown"
  CPU_OR_NO_GPU="unknown"
  if [ -n "$PID" ]; then
    COMMAND_LINE="$(ps -ww -p "$PID" -o command= 2>/dev/null || true)"
    GPU_LAYERS="$(printf '%s\n' "$COMMAND_LINE" | sed -n 's/.* -ngl \([^ ]*\).*/\1/p')"
    if [ -z "$GPU_LAYERS" ]; then
      GPU_LAYERS="unknown"
    fi
    DEVICE="default"
    if printf '%s\n' "$COMMAND_LINE" | grep -q -- '--device none'; then
      DEVICE="none"
    elif printf '%s\n' "$COMMAND_LINE" | grep -q -- '--device cpu'; then
      DEVICE="cpu"
    elif printf '%s\n' "$COMMAND_LINE" | grep -q -- '--device metal'; then
      DEVICE="metal"
    fi
    KV_OFFLOAD="on"
    if printf '%s\n' "$COMMAND_LINE" | grep -q -- '--no-kv-offload'; then
      KV_OFFLOAD="off"
    fi
    CPU_OR_NO_GPU="false"
    if [ "$GPU_LAYERS" = "0" ] || [ "$DEVICE" = "none" ] || [ "$DEVICE" = "cpu" ]; then
      CPU_OR_NO_GPU="true"
    fi
  fi
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$ROLE" "$PORT" "$PID" "$GPU_LAYERS" "$DEVICE" "$KV_OFFLOAD" "$CPU_OR_NO_GPU"
}
print_row "quality" "__REMOTE_MODEL_PORT__"
for ITEM in __REMOTE_POOL_WORKERS__; do
  ROLE="${ITEM%%:*}"
  PORT="${ITEM##*:}"
  print_row "$ROLE" "$PORT"
done
'@
    $remoteProbe = $remoteProbe.Replace("__REMOTE_MODEL_PORT__", [string]$RemoteModelPort)
    $remoteProbe = $remoteProbe.Replace("__REMOTE_POOL_WORKERS__", $remoteWorkerItems)

    $target = "$RemoteUser@$RemoteHost"
    $remoteProbeFile = [System.IO.Path]::GetTempFileName()
    $remoteProbeText = $remoteProbe -replace "`r`n", "`n" -replace "`r", "`n"
    [System.IO.File]::WriteAllText($remoteProbeFile, $remoteProbeText, [System.Text.Encoding]::ASCII)
    try {
        $remoteProbeCommand = 'type "{0}" | ssh.exe -i "{1}" -o BatchMode=yes {2} sh -s' -f $remoteProbeFile, $IdentityFile, $target
        $output = & cmd.exe /d /s /c $remoteProbeCommand 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        Remove-Item -LiteralPath $remoteProbeFile -Force -ErrorAction SilentlyContinue
    }
    $textLines = @($output | ForEach-Object { $_.ToString() } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($exitCode -ne 0) {
        return [pscustomobject]@{
            probed = $true
            read_only = $true
            starts_process = $false
            sends_prompt = $false
            touches_remote = $true
            error = (($textLines -join " ").Trim())
            worker_count = 0
            cpu_or_no_gpu_count = 0
            cpu_or_no_gpu_roles = @()
            backend_metadata_may_differ_roles = @()
            acceleration_ok = $null
            acceleration_next_step = ""
            workers = @()
        }
    }

    $workers = @()
    foreach ($line in $textLines) {
        $parts = @($line -split "`t", 7)
        if ($parts.Count -lt 7) {
            continue
        }
        $workers += [pscustomobject]@{
            role = $parts[0]
            port = [int]$parts[1]
            pid = if ([string]::IsNullOrWhiteSpace($parts[2])) { $null } else { [int]$parts[2] }
            gpu_layers = $parts[3]
            device = $parts[4]
            kv_offload = $parts[5]
            cpu_or_no_gpu = ($parts[6] -eq "true")
            backend_metadata_may_differ = ($parts[6] -eq "true")
        }
    }
    $cpuOrNoGpuWorkers = @($workers | Where-Object { $_.cpu_or_no_gpu -eq $true })
    $metadataMayDifferWorkers = @($workers | Where-Object { $_.backend_metadata_may_differ -eq $true })
    $blockingCpuOrNoGpuWorkers = @($cpuOrNoGpuWorkers)
    $accelerationNextStep = if ($blockingCpuOrNoGpuWorkers.Count -gt 0) {
        ".\tools\smartsteam-forge\run-remote-gemma-unattended.cmd -RestartRemote -SkipBuild"
    } else {
        ""
    }

    return [pscustomobject]@{
        probed = $true
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $true
        error = ""
        worker_count = $workers.Count
        cpu_or_no_gpu_count = $cpuOrNoGpuWorkers.Count
        cpu_or_no_gpu_roles = @($cpuOrNoGpuWorkers | ForEach-Object { $_.role })
        backend_metadata_may_differ_roles = @($metadataMayDifferWorkers | ForEach-Object { $_.role })
        acceleration_ok = ($blockingCpuOrNoGpuWorkers.Count -eq 0)
        acceleration_next_step = $accelerationNextStep
        workers = $workers
    }
}

function New-RemoteChainStatusSummary {
    param(
        [bool]$ModelApiHealthy,
        [bool]$ModelPortListening,
        [bool]$BackendListening,
        [object]$BackendHealth,
        [bool]$LabListening,
        [object]$PoolStatus,
        [object]$ModelCacheStatus,
        [object]$RemoteRuntimeStatus
    )

    $qualityWorkerReady = Get-QualityWorkerReady -Status $PoolStatus
    $launchAllowed = if ($null -eq $PoolStatus) { $null } else { Get-JsonProperty -Object $PoolStatus -Name "launch_allowed" }
    $capacity = if ($null -eq $PoolStatus) { $null } else { Get-JsonProperty -Object $PoolStatus -Name "capacity" }
    $capacityAllowed = Get-CapacityExpansionAllowed -Status $PoolStatus
    $requiredRoleNames = @(Get-RequiredPoolWorkerRoleNames)
    $requiredRoleStatus = Get-RequiredPoolWorkerReadiness -Status $PoolStatus -RequiredRoles $requiredRoleNames
    $chainReady = $ModelApiHealthy -and $BackendListening -and $LabListening -and ($launchAllowed -eq $true)
    if ($requiredRoleStatus.required_roles_ready -ne $null) {
        $chainReady = $chainReady -and ($requiredRoleStatus.required_roles_ready -eq $true)
    }
    $statusReason = if ($null -eq $PoolStatus) { $null } else { Get-StatusReason -Status $PoolStatus }
    $workerSummaries = @(New-RemoteChainWorkerSummaries -Status $PoolStatus -ModelCacheStatus $ModelCacheStatus)
    $qualityWorkerSummary = @($workerSummaries | Where-Object { [string]$_.role -eq "quality" } | Select-Object -First 1)
    $qualityModelCacheName = if ($qualityWorkerSummary.Count -eq 0) { $null } else { Get-JsonProperty -Object $qualityWorkerSummary[0] -Name "model_cache_name" }

    [pscustomobject]@{
        schema_version = 1
        contract_version = "smartsteam.remote-gemma-chain.status.v1"
        read_only = $true
        sends_prompt = $false
        starts_process = $false
        touches_remote = ($RemoteRuntimeStatus.touches_remote -eq $true)
        remote_probe_skipped = -not ($RemoteRuntimeStatus.probed -eq $true)
        remote = [pscustomobject]@{
            host = $RemoteHost
            user = $RemoteUser
            root = $RemoteRoot
            model_port = $RemoteModelPort
        }
        endpoints = [pscustomobject]@{
            model = "http://127.0.0.1:$LocalModelPort/v1/models"
            backend = "http://127.0.0.1:$BackendPort/health"
            web_lab = "http://127.0.0.1:$LabPort/"
        }
        readiness = [pscustomobject]@{
            ready = $chainReady
            model_api = $ModelApiHealthy
            backend = $BackendListening
            web_lab = $LabListening
            quality_worker = $qualityWorkerReady
            model_pool_launch_allowed = $launchAllowed
            capacity_expansion_allowed = $capacityAllowed
            required_roles_ready = $requiredRoleStatus.required_roles_ready
            model_cache_all_ok = if ($null -eq $ModelCacheStatus) { $null } else { Get-JsonProperty -Object $ModelCacheStatus -Name "all_ok" }
        }
        model_api = [pscustomobject]@{
            port = $LocalModelPort
            listening = $ModelPortListening
            healthy = $ModelApiHealthy
        }
        backend = [pscustomobject]@{
            port = $BackendPort
            listening = $BackendListening
            model = if ($null -eq $BackendHealth) { $null } else { Get-JsonProperty -Object $BackendHealth -Name "gemma_runtime_model" }
            readiness_ok = if ($null -eq $BackendHealth) { $null } else { Get-JsonProperty -Object $BackendHealth -Name "readiness_ok" }
            engine_busy = if ($null -eq $BackendHealth) { $null } else { Get-JsonProperty -Object $BackendHealth -Name "engine_busy" }
        }
        web_lab = [pscustomobject]@{
            port = $LabPort
            listening = $LabListening
        }
        model_pool = [pscustomobject]@{
            available = ($null -ne $PoolStatus)
            launch_allowed = $launchAllowed
            reason = $statusReason
            worker_count = if ($null -eq $PoolStatus) { $null } else { Get-JsonProperty -Object $PoolStatus -Name "worker_count" }
            healthy_worker_count = if ($null -eq $PoolStatus) { $null } else { Get-JsonProperty -Object $PoolStatus -Name "healthy_worker_count" }
            min_context_tokens = if ($null -eq $PoolStatus) { $null } else { Get-JsonProperty -Object $PoolStatus -Name "min_context_tokens" }
            quality_worker_ready = $qualityWorkerReady
            quality_model_cache_name = $qualityModelCacheName
            required_roles = $requiredRoleStatus.required_roles
            required_roles_ready = $requiredRoleStatus.required_roles_ready
            missing_required_roles = $requiredRoleStatus.missing_required_roles
            workers = $workerSummaries
            capacity = $capacity
        }
        model_cache = $ModelCacheStatus
        remote_runtime = $RemoteRuntimeStatus
        next_step = Get-RemoteChainNextStep -ModelApiHealthy $ModelApiHealthy -BackendListening $BackendListening -LabListening $LabListening -PoolStatus $PoolStatus
    }
}

function Show-StatusJson {
    $modelApiHealthy = Test-ModelApi $LocalModelPort
    $modelPortListening = Test-LocalPort $LocalModelPort
    $backendListening = Test-LocalPort $BackendPort
    $labListening = Test-LocalPort $LabPort
    $backendHealth = $null
    $poolStatus = $null
    if ($backendListening) {
        $backendHealth = Get-HttpJson "http://127.0.0.1:$BackendPort/health"
        $poolStatus = Get-HttpJson "http://127.0.0.1:$BackendPort/v1/model-pool/status"
    }
    $modelCacheStatus = Read-ModelCacheStatus
    $remoteRuntimeStatus = Get-RemoteRuntimeLaunchStatus

    New-RemoteChainStatusSummary `
        -ModelApiHealthy $modelApiHealthy `
        -ModelPortListening $modelPortListening `
        -BackendListening $backendListening `
        -BackendHealth $backendHealth `
        -LabListening $labListening `
        -PoolStatus $poolStatus `
        -ModelCacheStatus $modelCacheStatus `
        -RemoteRuntimeStatus $remoteRuntimeStatus |
        ConvertTo-Json -Compress -Depth 12
}

function Normalize-PoolWorkerRole {
    param([string]$Role)

    $normalized = $Role.Trim().ToLowerInvariant()
    if ($normalized -eq "spare") {
        return "index"
    }
    return $normalized
}

function Get-PoolWorkerDefinitions {
    param(
        [switch]$All,
        [switch]$IncludeIndex
    )

    $definitions = @(
        [pscustomobject]@{ Role = "summary"; Port = $SummaryPort; ContextTokens = $SummaryContextTokens; Model = (Get-PoolWorkerModel "summary"); LlamaServer = (Get-PoolWorkerLlamaServer "summary"); GpuLayers = (Get-PoolWorkerGpuLayers "summary"); Device = (Get-PoolWorkerDevice "summary") },
        [pscustomobject]@{ Role = "router"; Port = $RouterPort; ContextTokens = $RouterContextTokens; Model = (Get-PoolWorkerModel "router"); LlamaServer = (Get-PoolWorkerLlamaServer "router"); GpuLayers = (Get-PoolWorkerGpuLayers "router"); Device = (Get-PoolWorkerDevice "router") },
        [pscustomobject]@{ Role = "review"; Port = $ReviewPort; ContextTokens = $ReviewContextTokens; Model = (Get-PoolWorkerModel "review"); LlamaServer = (Get-PoolWorkerLlamaServer "review"); GpuLayers = (Get-PoolWorkerGpuLayers "review"); Device = (Get-PoolWorkerDevice "review") },
        [pscustomobject]@{ Role = "index"; Port = $IndexPort; ContextTokens = $IndexContextTokens; Model = (Get-PoolWorkerModel "index"); LlamaServer = (Get-PoolWorkerLlamaServer "index"); GpuLayers = (Get-PoolWorkerGpuLayers "index"); Device = (Get-PoolWorkerDevice "index") },
        [pscustomobject]@{ Role = "test-gate"; Port = $TestGatePort; ContextTokens = $TestGateContextTokens; Model = (Get-PoolWorkerModel "test-gate"); LlamaServer = (Get-PoolWorkerLlamaServer "test-gate"); GpuLayers = (Get-PoolWorkerGpuLayers "test-gate"); Device = (Get-PoolWorkerDevice "test-gate") }
    )
    if ($All) {
        return $definitions
    }

    $requested = @($PoolWorkerRoles -split "," | ForEach-Object { Normalize-PoolWorkerRole $_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($IncludeIndex -and "index" -notin $requested) {
        $requested += "index"
    }
    if ($requested.Count -eq 0) {
        return @()
    }

    return @($definitions | Where-Object { $_.Role -in $requested })
}

function Get-PoolWorkerModel {
    param([string]$Role)

    $roleModel = switch ($Role) {
        "summary" { $RemoteSummaryModel; break }
        "router" { $RemoteRouterModel; break }
        "review" { $RemoteReviewModel; break }
        "test-gate" { $RemoteTestGateModel; break }
        "index" { $RemoteIndexModel; break }
        default { "" }
    }
    if (-not [string]::IsNullOrWhiteSpace($roleModel)) {
        return $roleModel
    }
    return $RemoteSmallModel
}

function Get-PoolWorkerLlamaServer {
    param([string]$Role)

    $roleServer = switch ($Role) {
        "summary" { $RemoteSummaryLlamaServer; break }
        "router" { $RemoteRouterLlamaServer; break }
        "review" { $RemoteReviewLlamaServer; break }
        "test-gate" { $RemoteTestGateLlamaServer; break }
        "index" { $RemoteIndexLlamaServer; break }
        default { "" }
    }
    if (-not [string]::IsNullOrWhiteSpace($roleServer)) {
        return $roleServer
    }
    if (-not [string]::IsNullOrWhiteSpace($RemoteSmallLlamaServer)) {
        return $RemoteSmallLlamaServer
    }
    return $RemoteLlamaServer
}

function Get-PoolWorkerGpuLayers {
    param([string]$Role)

    $roleGpuLayers = switch ($Role) {
        "summary" { $SummaryGpuLayers; break }
        "router" { $RouterGpuLayers; break }
        "review" { $ReviewGpuLayers; break }
        "test-gate" { $TestGateGpuLayers; break }
        "index" { $IndexGpuLayers; break }
        default { -1 }
    }
    if ($roleGpuLayers -ge 0) {
        return $roleGpuLayers
    }
    return $SmallGpuLayers
}

function Get-PoolWorkerDevice {
    param([string]$Role)

    $roleDevice = switch ($Role) {
        "summary" { $SummaryDevice; break }
        "router" { $RouterDevice; break }
        "review" { $ReviewDevice; break }
        "test-gate" { $TestGateDevice; break }
        "index" { $IndexDevice; break }
        default { "" }
    }
    if (-not [string]::IsNullOrWhiteSpace($roleDevice)) {
        return $roleDevice
    }
    return $SmallDevice
}

function Normalize-RemoteModelPath {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return ""
    }
    return $Path.Trim().Replace("\", "/").TrimEnd("/")
}

function Test-PoolWorkerModelLooksLarge {
    param([string]$ModelPath)

    $normalized = Normalize-RemoteModelPath $ModelPath
    if ([string]::IsNullOrWhiteSpace($normalized)) {
        return $false
    }
    $name = @($normalized -split "/")[-1]
    return $name -match '(?i)(^|[^0-9])(12|13|27|30|32|34|65|70|72)[._ -]*b([^a-z0-9]|$)'
}

function Assert-PoolWorkerLaunchInputs {
    param([object[]]$Workers)

    $missingModels = @($Workers | Where-Object { [string]::IsNullOrWhiteSpace([string]$_.Model) } | ForEach-Object { $_.Role })
    if ($missingModels.Count -gt 0) {
        throw "-EnablePoolWorkers requires -RemoteSmallModel or per-role Remote*Model values for: $($missingModels -join ',')"
    }
    $missingServers = @($Workers | Where-Object { [string]::IsNullOrWhiteSpace([string]$_.LlamaServer) } | ForEach-Object { $_.Role })
    if ($missingServers.Count -gt 0) {
        throw "-EnablePoolWorkers requires -RemoteSmallLlamaServer or per-role Remote*LlamaServer values for: $($missingServers -join ',')"
    }
    if ($AllowLargePoolWorkerModels) {
        return
    }

    $qualityModel = Normalize-RemoteModelPath $RemoteModel
    $sameAsQuality = @($Workers | Where-Object {
        (Normalize-RemoteModelPath ([string]$_.Model)) -eq $qualityModel
    } | ForEach-Object { $_.Role })
    if ($sameAsQuality.Count -gt 0) {
        throw "-EnablePoolWorkers would start helper roles with the same model as quality ($RemoteModel): $($sameAsQuality -join ','). Use a small/low-quant helper model, or pass -AllowLargePoolWorkerModels only for an explicit stress test."
    }

    $largeModels = @($Workers | Where-Object { Test-PoolWorkerModelLooksLarge ([string]$_.Model) })
    if ($largeModels.Count -gt 0) {
        $details = @($largeModels | ForEach-Object { "$($_.Role)=$($_.Model)" }) -join "; "
        throw "-EnablePoolWorkers rejects helper model paths that look 12B+ by default: $details. Use small helpers for normal development, or pass -AllowLargePoolWorkerModels only for an explicit stress test."
    }
}

function Assert-PoolWorkerRolesValid {
    $allowed = @("summary", "router", "review", "index", "test-gate")
    $requested = @($PoolWorkerRoles -split "," | ForEach-Object { Normalize-PoolWorkerRole $_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if (($EnableIndexWorker -or $EnableSpareWorker) -and "index" -notin $requested) {
        $requested += "index"
    }
    $unknown = @($requested | Where-Object { $_ -notin $allowed })
    if ($unknown.Count -gt 0) {
        throw "unknown PoolWorkerRoles: $($unknown -join ','); allowed roles: $($allowed -join ',')"
    }
}

function Invoke-Remote {
    param([string]$Command)

    $target = "$RemoteUser@$RemoteHost"
    $remoteCommandFile = [System.IO.Path]::GetTempFileName()
    $remoteCommandText = $Command -replace "`r`n", "`n" -replace "`r", "`n"
    [System.IO.File]::WriteAllText($remoteCommandFile, $remoteCommandText, [System.Text.Encoding]::ASCII)
    try {
        $remoteInvokeCommand = 'type "{0}" | ssh.exe -i "{1}" -o BatchMode=yes {2} sh -s' -f $remoteCommandFile, $IdentityFile, $target
        & cmd.exe /d /s /c $remoteInvokeCommand
        $exitCode = $LASTEXITCODE
    } finally {
        Remove-Item -LiteralPath $remoteCommandFile -Force -ErrorAction SilentlyContinue
    }
    if ($exitCode -ne 0) {
        throw "remote ssh command failed with exit code $exitCode"
    }
}

function Stop-LocalPidFile {
    param(
        [string]$Path,
        [string]$ExpectedName,
        [string[]]$ExpectedCommandText
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }

    $raw = Get-Content -LiteralPath $Path -ErrorAction SilentlyContinue | Select-Object -First 1
    [int]$processId = 0
    if (-not [int]::TryParse($raw, [ref]$processId)) {
        Remove-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
        return
    }

    $process = Get-CimInstance Win32_Process -Filter "ProcessId = $processId" -ErrorAction SilentlyContinue
    if ($null -eq $process) {
        Remove-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
        return
    }

    $nameMatches = [string]::IsNullOrWhiteSpace($ExpectedName) -or $process.Name -eq $ExpectedName
    $commandMatches = $true
    foreach ($expectedText in @($ExpectedCommandText)) {
        if (-not [string]::IsNullOrWhiteSpace($expectedText) -and -not ([string]$process.CommandLine).Contains($expectedText)) {
            $commandMatches = $false
            break
        }
    }
    if (-not ($nameMatches -and $commandMatches)) {
        throw "refusing to stop pid $processId from $Path because it no longer looks like this remote Gemma chain process"
    }

    Write-Host "stopping stale local pid $processId from $Path"
    Stop-Process -Id $processId -Force
    Remove-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
}

function Start-LocalModelTunnel {
    param(
        [int]$LocalPort,
        [int]$RemotePort,
        [string]$PidName,
        [string]$Label
    )

    $tunnelPidFile = Join-Path $RunDir $PidName
    $forward = "$LocalPort`:127.0.0.1:$RemotePort"
    $target = "$RemoteUser@$RemoteHost"
    $expectedTunnelCommand = @("-L", $forward, $target)
    if ($RestartRemote) {
        Stop-LocalPidFile $tunnelPidFile "ssh.exe" $expectedTunnelCommand
    }

    if (Test-ModelApi $LocalPort) {
        Write-Host "local $Label model API already healthy on $LocalPort"
    } elseif (Test-LocalPort $LocalPort) {
        if (Test-Path -LiteralPath $tunnelPidFile) {
            Stop-LocalPidFile $tunnelPidFile "ssh.exe" $expectedTunnelCommand
        } else {
            throw "local port $LocalPort is already listening, but /v1/models is not healthy; choose another port or stop the process using that port"
        }
    }

    if (-not (Test-ModelApi $LocalPort)) {
        if (Test-LocalPort $LocalPort) {
            throw "local port $LocalPort is still occupied after tunnel cleanup"
        }
        $safeLabel = $Label -replace "[^a-zA-Z0-9_-]", "-"
        $tunnelLog = Join-Path $LogDir "ssh-tunnel-$safeLabel.log"
        $tunnelErr = Join-Path $LogDir "ssh-tunnel-$safeLabel.err.log"
        $process = Start-Process -FilePath "ssh.exe" `
            -WindowStyle Hidden `
            -PassThru `
            -RedirectStandardOutput $tunnelLog `
            -RedirectStandardError $tunnelErr `
            -ArgumentList @(
                "-i", $IdentityFile,
                "-o", "BatchMode=yes",
                "-o", "ExitOnForwardFailure=yes",
                "-o", "ServerAliveInterval=15",
                "-o", "ServerAliveCountMax=3",
                "-o", "TCPKeepAlive=yes",
                "-o", "ConnectTimeout=8",
                "-N",
                "-L", $forward,
                $target
            )
        Set-Content -LiteralPath $tunnelPidFile -Value $process.Id
        Write-Host "ssh tunnel started for $Label pid $($process.Id): http://127.0.0.1:$LocalPort -> ${RemoteHost}:$RemotePort"
    }

    if (Wait-HttpOk "http://127.0.0.1:$LocalPort/v1/models" 90) {
        Write-Host "local $Label model API ready: http://127.0.0.1:$LocalPort/v1/models"
    } else {
        throw "local $Label model API did not become ready on http://127.0.0.1:$LocalPort/v1/models"
    }
}

function New-RemotePoolWorkerStartScript {
    param($Worker)

    $script = @'
set -eu
mkdir -p "__REMOTE_ROOT__/logs"
if [ ! -x "__REMOTE_LLAMA_SERVER__" ]; then
  echo "missing llama-server for __ROLE__ worker: __REMOTE_LLAMA_SERVER__" >&2
  exit 21
fi
if [ ! -f "__REMOTE_MODEL__" ]; then
  echo "missing model for __ROLE__ worker: __REMOTE_MODEL__" >&2
  exit 22
fi
PID="__REMOTE_ROOT__/llama-server-__ROLE__.pid"
LOG="__REMOTE_ROOT__/logs/llama-server-__ROLE__.log"
actual_gpu_layers() {
  CHECK_PID="${1:-}"
  COMMAND_LINE="$(ps -ww -p "$CHECK_PID" -o command= 2>/dev/null || true)"
  GPU_LAYERS="$(printf '%s\n' "$COMMAND_LINE" | sed -n 's/.* -ngl \([^ ]*\).*/\1/p')"
  if [ -z "$GPU_LAYERS" ]; then
    GPU_LAYERS="unknown"
  fi
  printf '%s\n' "$GPU_LAYERS"
}
actual_device() {
  CHECK_PID="${1:-}"
  COMMAND_LINE="$(ps -ww -p "$CHECK_PID" -o command= 2>/dev/null || true)"
  DEVICE="default"
  if printf '%s\n' "$COMMAND_LINE" | grep -q -- '--device none'; then
    DEVICE="none"
  elif printf '%s\n' "$COMMAND_LINE" | grep -q -- '--device cpu'; then
    DEVICE="cpu"
  elif printf '%s\n' "$COMMAND_LINE" | grep -q -- '--device metal'; then
    DEVICE="metal"
  fi
  printf '%s\n' "$DEVICE"
}
launch_flags_match() {
  ACTUAL_GPU="$1"
  ACTUAL_DEVICE="$2"
  EXPECTED_DEVICE_MODE="__EXPECTED_DEVICE_MODE__"
  if [ "$ACTUAL_GPU" != "__GPU_LAYERS__" ]; then
    return 1
  fi
  if [ "$EXPECTED_DEVICE_MODE" = "any" ]; then
    return 0
  fi
  if [ "$EXPECTED_DEVICE_MODE" = "accelerated" ]; then
    [ "$ACTUAL_DEVICE" != "none" ] && [ "$ACTUAL_DEVICE" != "cpu" ]
    return $?
  fi
  [ "$ACTUAL_DEVICE" = "$EXPECTED_DEVICE_MODE" ]
}
ensure_existing_worker_matches() {
  EXISTING_PID="${1:-}"
  ACTUAL_GPU="$(actual_gpu_layers "$EXISTING_PID")"
  ACTUAL_DEVICE="$(actual_device "$EXISTING_PID")"
  EXPECTED_DEVICE_MODE="__EXPECTED_DEVICE_MODE__"
  if launch_flags_match "$ACTUAL_GPU" "$ACTUAL_DEVICE"; then
    return 0
  fi
  if [ "__RESTART_REMOTE__" = "1" ]; then
    echo "remote __ROLE__ worker existing pid $EXISTING_PID launch flags mismatch; restarting (expected_ngl=__GPU_LAYERS__ expected_device=$EXPECTED_DEVICE_MODE actual_ngl=$ACTUAL_GPU actual_device=$ACTUAL_DEVICE)"
    kill "$EXISTING_PID" 2>/dev/null || true
    sleep 1
    return 1
  fi
  echo "remote __ROLE__ worker already running pid $EXISTING_PID but launch flags mismatch: expected_ngl=__GPU_LAYERS__ actual_ngl=$ACTUAL_GPU expected_device=$EXPECTED_DEVICE_MODE actual_device=$ACTUAL_DEVICE; rerun with -RestartRemote" >&2
  exit 32
}
if [ "__RESTART_REMOTE__" = "1" ] && [ -f "$PID" ]; then
  OLD_PID="$(cat "$PID" 2>/dev/null || true)"
  if [ -n "$OLD_PID" ] && kill -0 "$OLD_PID" 2>/dev/null; then
    kill "$OLD_PID" 2>/dev/null || true
    sleep 1
  fi
  rm -f "$PID"
fi
if [ -f "$PID" ]; then
  OLD_PID="$(cat "$PID" 2>/dev/null || true)"
  if [ -n "$OLD_PID" ] && kill -0 "$OLD_PID" 2>/dev/null; then
    if lsof -nP -a -p "$OLD_PID" -iTCP:__REMOTE_MODEL_PORT__ -sTCP:LISTEN >/dev/null 2>&1; then
      if ensure_existing_worker_matches "$OLD_PID"; then
        echo "remote __ROLE__ worker already running pid $OLD_PID"
        exit 0
      fi
      rm -f "$PID"
    else
      echo "remote __ROLE__ worker pid $OLD_PID is running on a different port, ignoring stale pid file"
      rm -f "$PID"
    fi
  fi
fi
PORT_PID="$(lsof -iTCP:__REMOTE_MODEL_PORT__ -sTCP:LISTEN -n -P -t 2>/dev/null | head -n 1 || true)"
if [ -n "$PORT_PID" ]; then
  if ensure_existing_worker_matches "$PORT_PID"; then
    echo "remote __ROLE__ worker port __REMOTE_MODEL_PORT__ is already listening"
    exit 0
  fi
fi
nohup "__REMOTE_LLAMA_SERVER__" \
  -m "__REMOTE_MODEL__" \
  --host 127.0.0.1 \
  --port __REMOTE_MODEL_PORT__ \
  -ngl __GPU_LAYERS__ \
__DEVICE_ARG__  -np __PARALLEL_SLOTS__ \
  -b __BATCH_SIZE__ \
  -ub __UBATCH_SIZE__ \
  --cache-ram __CACHE_RAM_MIB__ \
  --no-warmup \
  --no-kv-offload \
  -c __CONTEXT_TOKENS__ \
  --reasoning __REASONING__ \
  --jinja \
  > "$LOG" 2>&1 &
echo $! > "$PID"
STARTED_PID="$(cat "$PID")"
echo "remote __ROLE__ worker started pid $STARTED_PID"
'@
    $script = $script.Replace("__REMOTE_ROOT__", $RemoteRoot)
    $script = $script.Replace("__REMOTE_LLAMA_SERVER__", $Worker.LlamaServer)
    $script = $script.Replace("__REMOTE_MODEL__", $Worker.Model)
    $script = $script.Replace("__ROLE__", $Worker.Role)
    $script = $script.Replace("__REMOTE_MODEL_PORT__", [string]$Worker.Port)
    $script = $script.Replace("__GPU_LAYERS__", [string]$Worker.GpuLayers)
    $deviceArg = ""
    $expectedDeviceMode = "any"
    if ([int]$Worker.GpuLayers -gt 0) {
        $expectedDeviceMode = "accelerated"
    }
    if (-not [string]::IsNullOrWhiteSpace([string]$Worker.Device)) {
        $deviceArg = "  --device `"$($Worker.Device)`" \`n"
        $expectedDeviceMode = ([string]$Worker.Device).Trim().ToLowerInvariant()
    }
    $script = $script.Replace("__EXPECTED_DEVICE_MODE__", $expectedDeviceMode)
    $script = $script.Replace("__DEVICE_ARG__", $deviceArg)
    $script = $script.Replace("__PARALLEL_SLOTS__", [string]$SmallParallelSlots)
    $script = $script.Replace("__BATCH_SIZE__", [string]$SmallBatchSize)
    $script = $script.Replace("__UBATCH_SIZE__", [string]$SmallUbatchSize)
    $script = $script.Replace("__CACHE_RAM_MIB__", [string]$SmallCacheRamMiB)
    $script = $script.Replace("__CONTEXT_TOKENS__", [string]$Worker.ContextTokens)
    $script = $script.Replace("__REASONING__", $SmallReasoning)
    $script = $script.Replace("__RESTART_REMOTE__", $restartValue)
    return $script
}

function Show-Status {
    $expectedForward = "$LocalModelPort`:127.0.0.1:$RemoteModelPort"
    $expectedTarget = "$RemoteUser@$RemoteHost"

    Write-Host "local model tunnel: http://127.0.0.1:$LocalModelPort/v1/models"
    $modelApiHealthy = Test-ModelApi $LocalModelPort
    $modelPortListening = Test-LocalPort $LocalModelPort
    if ($modelApiHealthy) {
        Write-Host "  local model api: ok"
    } elseif ($modelPortListening) {
        Write-Host "  local model port: listening, but /v1/models is not healthy"
    } else {
        Write-Host "  local model port: closed"
    }
    if (-not $modelApiHealthy) {
        $tunnelPidFile = Join-Path $RunDir "ssh-tunnel.pid"
        if (Test-Path -LiteralPath $tunnelPidFile) {
            $rawTunnelPid = Get-Content -LiteralPath $tunnelPidFile -ErrorAction SilentlyContinue | Select-Object -First 1
            [int]$tunnelProcessId = 0
            if ([int]::TryParse($rawTunnelPid, [ref]$tunnelProcessId)) {
                $tunnelProcess = Get-CimInstance Win32_Process -Filter "ProcessId = $tunnelProcessId" -ErrorAction SilentlyContinue
                if ($null -eq $tunnelProcess) {
                    Write-Host "  local model tunnel pid: stale $tunnelProcessId"
                } else {
                    $commandLine = [string]$tunnelProcess.CommandLine
                    $looksLikeTunnel = $tunnelProcess.Name -eq "ssh.exe" -and $commandLine.Contains($expectedForward) -and $commandLine.Contains($expectedTarget)
                    if ($looksLikeTunnel) {
                        Write-Host "  local model tunnel pid: running $tunnelProcessId, but /v1/models is not healthy"
                    } else {
                        Write-Host "  local model tunnel pid: mismatched $tunnelProcessId"
                    }
                }
                Write-Host "  repair: start-remote-gemma-chain.cmd -SkipBuild -NoBackend -NoLab -LocalModelPort $LocalModelPort -BackendPort $BackendPort -LabPort $LabPort"
            }
        }
    }

    Write-Host "pool worker local endpoints:"
    foreach ($worker in Get-PoolWorkerDefinitions -All) {
        $port = [int]$worker.Port
        $apiHealthy = Test-ModelApi $port
        $portListening = Test-LocalPort $port
        if ($apiHealthy) {
            Write-Host "  $($worker.Role): http://127.0.0.1:$port/v1/models ok"
        } elseif ($portListening) {
            Write-Host "  $($worker.Role): port $port listening, but /v1/models is not healthy"
        } else {
            Write-Host "  $($worker.Role): port $port closed"
        }
    }

    $backendListening = Test-LocalPort $BackendPort
    $poolStatus = $null
    if ($backendListening) {
        Write-Host "backend: http://127.0.0.1:$BackendPort/health"
        $poolStatus = Get-HttpJson "http://127.0.0.1:$BackendPort/v1/model-pool/status"
        Write-ModelPoolStatus $poolStatus
    } else {
        Write-Host "backend: closed"
    }

    $labListening = Test-LocalPort $LabPort
    if ($labListening) {
        Write-Host "web lab: http://127.0.0.1:$LabPort/"
    } else {
        Write-Host "web lab: closed"
    }

    $modelCacheStatus = Read-ModelCacheStatus
    Write-ModelCacheStatus $modelCacheStatus

    Write-RemoteChainReadiness -ModelApiHealthy $modelApiHealthy -BackendListening $backendListening -LabListening $labListening -PoolStatus $poolStatus

    $remoteWorkerItems = @((Get-PoolWorkerDefinitions -All) | ForEach-Object { "$($_.Role):$($_.Port)" }) -join " "
    $remoteStatus = @'
set -eu
print_launch_flags() {
  LABEL="$1"
  PROCESS_ID="${2-}"
  COMMAND_LINE="$(ps -ww -p "$PROCESS_ID" -o command= 2>/dev/null || true)"
  print_command_flags "$LABEL" "$COMMAND_LINE"
}
print_port_launch_flags() {
  LABEL="$1"
  PORT="${2-}"
  PORT_PID="$(lsof -nP -iTCP:"$PORT" -sTCP:LISTEN -t 2>/dev/null | head -n 1 || true)"
  if [ -n "$PORT_PID" ]; then
    print_launch_flags "$LABEL-port-$PORT" "$PORT_PID"
  fi
}
print_command_flags() {
  LABEL="$1"
  COMMAND_LINE="${2-}"
  if [ -z "$COMMAND_LINE" ]; then
    return 0
  fi
  GPU_LAYERS="$(printf '%s\n' "$COMMAND_LINE" | sed -n 's/.* -ngl \([^ ]*\).*/\1/p')"
  if [ -z "$GPU_LAYERS" ]; then
    GPU_LAYERS="unknown"
  fi
  DEVICE="default"
  if printf '%s\n' "$COMMAND_LINE" | grep -q -- '--device none'; then
    DEVICE="none"
  elif printf '%s\n' "$COMMAND_LINE" | grep -q -- '--device cpu'; then
    DEVICE="cpu"
  elif printf '%s\n' "$COMMAND_LINE" | grep -q -- '--device metal'; then
    DEVICE="metal"
  fi
  KV_OFFLOAD="on"
  if printf '%s\n' "$COMMAND_LINE" | grep -q -- '--no-kv-offload'; then
    KV_OFFLOAD="off"
  fi
  echo "remote $LABEL launch flags: gpu_layers=$GPU_LAYERS device=$DEVICE kv_offload=$KV_OFFLOAD"
  if [ "$GPU_LAYERS" = "0" ] || [ "$DEVICE" = "none" ] || [ "$DEVICE" = "cpu" ]; then
    echo "remote $LABEL launch warning: cpu_or_no_gpu=true backend_metadata_may_differ=true"
  fi
}
echo "remote root: __REMOTE_ROOT__"
if [ -f "__REMOTE_ROOT__/llama-server.pid" ]; then
  PID="$(cat "__REMOTE_ROOT__/llama-server.pid" 2>/dev/null || true)"
  if [ -n "$PID" ] && kill -0 "$PID" 2>/dev/null; then
    echo "remote llama-server: running pid $PID"
    print_launch_flags "llama-server" "$PID"
  else
    echo "remote llama-server: stale pid"
  fi
else
  echo "remote llama-server: no pid"
fi
if [ -f "__REMOTE_MODEL__" ]; then
  ls -lh "__REMOTE_MODEL__"
else
  echo "remote model: missing __REMOTE_MODEL__"
fi
if command -v curl >/dev/null 2>&1; then
  curl -fsS "http://127.0.0.1:__REMOTE_MODEL_PORT__/v1/models" >/dev/null 2>&1 && echo "remote model api: ok" || echo "remote model api: not ready"
fi
print_port_launch_flags "model" "__REMOTE_MODEL_PORT__"
for ITEM in __REMOTE_POOL_WORKERS__; do
  ROLE="${ITEM%%:*}"
  PORT="${ITEM##*:}"
  PID="__REMOTE_ROOT__/llama-server-$ROLE.pid"
  if [ -f "$PID" ]; then
    WORKER_PID="$(cat "$PID" 2>/dev/null || true)"
    if [ -n "$WORKER_PID" ] && kill -0 "$WORKER_PID" 2>/dev/null; then
      echo "remote $ROLE worker: running pid $WORKER_PID"
      print_launch_flags "$ROLE-worker" "$WORKER_PID"
    else
      echo "remote $ROLE worker: stale pid"
    fi
  else
    echo "remote $ROLE worker: no pid"
  fi
  if command -v curl >/dev/null 2>&1; then
    curl -fsS "http://127.0.0.1:$PORT/v1/models" >/dev/null 2>&1 && echo "remote $ROLE api: ok" || echo "remote $ROLE api: not ready"
  fi
  print_port_launch_flags "$ROLE" "$PORT"
done
'@
    $remoteStatus = $remoteStatus.Replace("__REMOTE_ROOT__", $RemoteRoot)
    $remoteStatus = $remoteStatus.Replace("__REMOTE_MODEL__", $RemoteModel)
    $remoteStatus = $remoteStatus.Replace("__REMOTE_MODEL_PORT__", [string]$RemoteModelPort)
    $remoteStatus = $remoteStatus.Replace("__REMOTE_POOL_WORKERS__", $remoteWorkerItems)
    Invoke-Remote $remoteStatus
}

function Show-StatusWatch {
    if ($WatchIntervalSeconds -le 0) {
        throw "-WatchIntervalSeconds requires a positive integer"
    }
    if ($WatchCount -lt 0) {
        throw "-WatchCount must be 0 for unlimited or a positive integer"
    }

    $iteration = 0
    while ($true) {
        $iteration += 1
        Write-Host ""
        Write-Host ("remote_gemma_status_watch iteration={0} timestamp={1}" -f $iteration, (Get-Date -Format o))
        Show-Status

        if ($WatchCount -gt 0 -and $iteration -ge $WatchCount) {
            return
        }
        Start-Sleep -Seconds $WatchIntervalSeconds
    }
}

if ($Status) {
    if ($JsonStatus -and $Watch) {
        throw "-JsonStatus does not support -Watch; run one JSON status snapshot at a time."
    }
    if ($FailOnNotReady -and -not $JsonStatus) {
        throw "-FailOnNotReady requires -Status -JsonStatus."
    }
    if ($OutputJson.Trim().Length -gt 0 -and -not $JsonStatus) {
        throw "-OutputJson requires -Status -JsonStatus."
    }
    if ($JsonStatus) {
        $jsonText = ((Show-StatusJson | Out-String).Trim())
        if ($OutputJson.Trim().Length -gt 0) {
            $outputPath = Resolve-StatusArtifactPath $OutputJson
            $outputParent = Split-Path -Parent $outputPath
            if ($outputParent -and $outputParent.Trim().Length -gt 0) {
                New-Item -ItemType Directory -Force -Path $outputParent | Out-Null
            }
            Set-Content -Encoding ASCII -LiteralPath $outputPath -Value $jsonText
        }
        Write-Output $jsonText
        $statusExitCode = 0
        if ($FailOnNotReady) {
            $statusJson = $jsonText | ConvertFrom-Json
            if (-not $statusJson.readiness.ready) {
                $statusExitCode = 2
            }
        }
        exit $statusExitCode
    }
    if ($Watch) {
        Show-StatusWatch
    } else {
        Show-Status
    }
    return
}

if ($Watch) {
    throw "-Watch requires -Status. Use status-remote-gemma-chain.cmd -Watch."
}

if ($OutputJson.Trim().Length -gt 0) {
    throw "-OutputJson requires -Status -JsonStatus."
}

if (-not (Test-Path -LiteralPath $RepoRoot)) {
    throw "RepoRoot not found: $RepoRoot"
}
Assert-PoolWorkerRolesValid
if ([string]::IsNullOrWhiteSpace($RemoteSmallLlamaServer)) {
    $RemoteSmallLlamaServer = $RemoteLlamaServer
}
$explicitModelPoolManifest = -not [string]::IsNullOrWhiteSpace($ModelPoolManifest)
$poolWorkers = @()
if ($EnablePoolWorkers) {
    $poolWorkers = @(Get-PoolWorkerDefinitions -IncludeIndex:($EnableIndexWorker -or $EnableSpareWorker))
    if ($poolWorkers.Count -eq 0) {
        throw "-EnablePoolWorkers selected no workers; set -PoolWorkerRoles summary,router,review,index,test-gate"
    }
    Assert-PoolWorkerLaunchInputs -Workers $poolWorkers
    if (-not $explicitModelPoolManifest) {
        $defaultModelPoolManifest = Get-DefaultModelPoolManifestPath
        if (-not (Test-Path -LiteralPath $defaultModelPoolManifest -PathType Leaf)) {
            throw "-EnablePoolWorkers requires -ModelPoolManifest, or a generated manifest at $defaultModelPoolManifest. Run .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -UseMac32GBModelPool -CheckOnly -NoForge first."
        }
        $ModelPoolManifest = $defaultModelPoolManifest
        Write-Host "auto_model_pool_manifest=$ModelPoolManifest"
    }
}
if (-not [string]::IsNullOrWhiteSpace($ModelPoolManifest)) {
    if (-not (Test-Path -LiteralPath $ModelPoolManifest -PathType Leaf)) {
        throw "ModelPoolManifest not found: $ModelPoolManifest"
    }
    $ModelPoolManifest = (Resolve-Path -LiteralPath $ModelPoolManifest).Path
}
if (-not [string]::IsNullOrWhiteSpace($ModelPoolManifest)) {
    Assert-ModelPoolManifestAdvice `
        -Path $ModelPoolManifest `
        -ExpectedHelperRoles @($poolWorkers | ForEach-Object { $_.Role }) `
        -RequireHelpers:$EnablePoolWorkers
}
if ($CheckOnly) {
    Write-Host "SmartSteam remote Gemma chain preflight: PASS"
    Write-Host "check_only=true"
    Write-Host "touches_remote=false"
    Write-Host "starts_process=false"
    Write-Host "sends_prompt=false"
    Write-Host "existing_worker_mismatch_policy=fail_without_restart_remote"
    Write-Host "existing_worker_mismatch_fix=rerun_with_-RestartRemote"
    Write-Host "enable_pool_workers=$($EnablePoolWorkers.IsPresent)"
    if ($EnablePoolWorkers) {
        Write-Host "pool_worker_roles=$((@($poolWorkers | ForEach-Object { $_.Role }) -join ','))"
        foreach ($worker in $poolWorkers) {
            $device = if ([string]::IsNullOrWhiteSpace([string]$worker.Device)) { "default" } else { [string]$worker.Device }
            Write-Host ("pool_worker_{0}_launch_flags=ngl:{1} device:{2}" -f $worker.Role, $worker.GpuLayers, $device)
        }
    }
    if ([string]::IsNullOrWhiteSpace($ModelPoolManifest)) {
        Write-Host "model_pool_manifest=disabled"
    } else {
        Write-Host "model_pool_manifest=$ModelPoolManifest"
        Write-Host "model_pool_manifest_contract=gemma-chain.v1"
        Write-Host "model_pool_manifest_runtime_metadata=present"
    }
    return
}
if (-not (Test-Path -LiteralPath $IdentityFile)) {
    throw "SSH identity file not found: $IdentityFile"
}

$restartValue = if ($RestartRemote) { "1" } else { "0" }
$remoteStart = @'
set -eu
mkdir -p "__REMOTE_ROOT__/logs"
if [ ! -x "__REMOTE_LLAMA_SERVER__" ]; then
  echo "missing llama-server: __REMOTE_LLAMA_SERVER__" >&2
  exit 11
fi
if [ ! -f "__REMOTE_MODEL__" ]; then
  echo "missing model: __REMOTE_MODEL__" >&2
  exit 12
fi
PID="__REMOTE_ROOT__/llama-server.pid"
LOG="__REMOTE_ROOT__/logs/llama-server.log"
if [ "__RESTART_REMOTE__" = "1" ] && [ -f "$PID" ]; then
  OLD_PID="$(cat "$PID" 2>/dev/null || true)"
  if [ -n "$OLD_PID" ] && kill -0 "$OLD_PID" 2>/dev/null; then
    kill "$OLD_PID" 2>/dev/null || true
    sleep 1
  fi
  rm -f "$PID"
fi
if [ -f "$PID" ]; then
  OLD_PID="$(cat "$PID" 2>/dev/null || true)"
  if [ -n "$OLD_PID" ] && kill -0 "$OLD_PID" 2>/dev/null; then
    echo "remote llama-server already running pid $OLD_PID"
    exit 0
  fi
fi
if lsof -iTCP:__REMOTE_MODEL_PORT__ -sTCP:LISTEN -n -P >/dev/null 2>&1; then
  echo "remote port __REMOTE_MODEL_PORT__ is already listening"
  exit 0
fi
nohup "__REMOTE_LLAMA_SERVER__" \
  -m "__REMOTE_MODEL__" \
  --host 127.0.0.1 \
  --port __REMOTE_MODEL_PORT__ \
  -ngl __GPU_LAYERS__ \
  -c __CONTEXT_TOKENS__ \
  --reasoning __REASONING__ \
  --jinja \
  > "$LOG" 2>&1 &
echo $! > "$PID"
STARTED_PID="$(cat "$PID")"
echo "remote llama-server started pid $STARTED_PID"
'@
$remoteStart = $remoteStart.Replace("__REMOTE_ROOT__", $RemoteRoot)
$remoteStart = $remoteStart.Replace("__REMOTE_LLAMA_SERVER__", $RemoteLlamaServer)
$remoteStart = $remoteStart.Replace("__REMOTE_MODEL__", $RemoteModel)
$remoteStart = $remoteStart.Replace("__REMOTE_MODEL_PORT__", [string]$RemoteModelPort)
$remoteStart = $remoteStart.Replace("__GPU_LAYERS__", [string]$GpuLayers)
$remoteStart = $remoteStart.Replace("__CONTEXT_TOKENS__", [string]$ContextTokens)
$remoteStart = $remoteStart.Replace("__REASONING__", $Reasoning)
$remoteStart = $remoteStart.Replace("__RESTART_REMOTE__", $restartValue)

Write-Host "starting remote Gemma model box on $RemoteHost..."
Write-Host "quality budget: context_tokens=$ContextTokens default_max_tokens=$DefaultMaxTokens runtime_timeout_ms=$RuntimeTimeoutMs lab_backend_timeout_seconds=$LabBackendTimeoutSeconds"
Invoke-Remote $remoteStart
if ($EnablePoolWorkers) {
    Write-Host "starting optional remote pool workers on ${RemoteHost}: $((@($poolWorkers | ForEach-Object { $_.Role }) -join ', '))"
    foreach ($worker in $poolWorkers) {
        Write-Host "  $($worker.Role): model=$($worker.Model) context_tokens=$($worker.ContextTokens) llama_server=$($worker.LlamaServer)"
        Invoke-Remote (New-RemotePoolWorkerStartScript $worker)
    }
}

if (-not $NoTunnel) {
    Start-LocalModelTunnel -LocalPort $LocalModelPort -RemotePort $RemoteModelPort -PidName "ssh-tunnel.pid" -Label "quality"
    if ($EnablePoolWorkers) {
        foreach ($worker in $poolWorkers) {
            Start-LocalModelTunnel -LocalPort $worker.Port -RemotePort $worker.Port -PidName "ssh-tunnel-$($worker.Role).pid" -Label $worker.Role
        }
    }
}

if (-not $SkipBuild) {
    Write-Host "building rust-norion and rustgpt-lab into isolated target dir..."
    Push-Location $RepoRoot
    try {
        & cargo build --target-dir (Join-Path $BuildDir "rust-norion")
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed for rust-norion"
        }
        & cargo build --manifest-path (Join-Path $RepoRoot "tools\rustgpt-lab\Cargo.toml") --target-dir (Join-Path $BuildDir "rustgpt-lab")
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed for rustgpt-lab"
        }
    } finally {
        Pop-Location
    }
}

if (-not $NoBackend) {
    if (Test-LocalPort $BackendPort) {
        Write-Host "backend port $BackendPort already listening"
    } else {
        $backendExe = Join-Path $BuildDir "rust-norion\debug\rust-norion.exe"
        if (-not (Test-Path -LiteralPath $backendExe)) {
            throw "backend binary not found: $backendExe"
        }
        $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
        $backendOut = Join-Path $LogDir "rust-norion-$stamp.out.log"
        $backendErr = Join-Path $LogDir "rust-norion-$stamp.err.log"
        $backendArgs = @(
            "--serve", "--serve-bind", "127.0.0.1:$BackendPort",
            "--gemma-runtime-server", "http://127.0.0.1:$LocalModelPort",
            "--gemma-model-id", $RuntimeModelId,
            "--runtime-native-window", "$ContextTokens",
            "--max-tokens", "$DefaultMaxTokens",
            "--runtime-timeout-ms", "$RuntimeTimeoutMs",
            "--memory", (Join-Path $StateDir "memory.ndkv"),
            "--experience", (Join-Path $StateDir "experience.ndkv"),
            "--adaptive", (Join-Path $StateDir "adaptive.ndkv"),
            "--trace", (Join-Path $LogDir "trace-http-runtime-$stamp.jsonl")
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
        Set-Content -LiteralPath (Join-Path $RunDir "rust-norion.pid") -Value $process.Id
        Write-Host "rust-norion backend started pid $($process.Id): http://127.0.0.1:$BackendPort"
        if (-not (Wait-HttpOk "http://127.0.0.1:$BackendPort/health" 30)) {
            Write-Warning "backend health is not ready yet"
        }
    }
}

if (-not $NoLab) {
    if (Test-LocalPort $LabPort) {
        Write-Host "web lab port $LabPort already listening"
    } else {
        $labExe = Join-Path $BuildDir "rustgpt-lab\debug\rustgpt-lab.exe"
        if (-not (Test-Path -LiteralPath $labExe)) {
            throw "web lab binary not found: $labExe"
        }
        $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
        $labOut = Join-Path $LogDir "rustgpt-lab-$stamp.out.log"
        $labErr = Join-Path $LogDir "rustgpt-lab-$stamp.err.log"
        $process = Start-Process -FilePath $labExe `
            -WorkingDirectory (Join-Path $RepoRoot "tools\rustgpt-lab") `
            -WindowStyle Hidden `
            -PassThru `
            -RedirectStandardOutput $labOut `
            -RedirectStandardError $labErr `
            -ArgumentList @(
                "--bind", "127.0.0.1:$LabPort",
                "--backend", "127.0.0.1:$BackendPort",
                "--backend-timeout-secs", $LabBackendTimeoutSeconds.ToString()
            )
        Set-Content -LiteralPath (Join-Path $RunDir "rustgpt-lab.pid") -Value $process.Id
        Write-Host "web lab started pid $($process.Id): http://127.0.0.1:$LabPort/"
    }
}

Write-Host ""
Write-Host "remote Gemma chain is starting."
Write-Host "model API: http://127.0.0.1:$LocalModelPort/v1/models"
Write-Host "backend:   http://127.0.0.1:$BackendPort/health"
Write-Host "web lab:   http://127.0.0.1:$LabPort/"
if ($EnablePoolWorkers) {
    Write-Host "pool workers:"
    foreach ($worker in $poolWorkers) {
        Write-Host "  $($worker.Role): http://127.0.0.1:$($worker.Port)/v1/models model=$($worker.Model)"
    }
} else {
    Write-Host "pool workers: disabled; only quality worker is started"
}
if (-not [string]::IsNullOrWhiteSpace($ModelPoolManifest)) {
    Write-Host "pool manifest: $ModelPoolManifest"
}
Write-Host "logs:      $LogDir"

if ($LaunchForge) {
    $forgeScript = Join-Path $PSScriptRoot "start-forge-ui.ps1"
    if (-not (Test-Path -LiteralPath $forgeScript)) {
        throw "Forge UI launcher not found: $forgeScript"
    }
    Write-Host "starting SmartSteam Forge UI against 127.0.0.1:$BackendPort..."
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $forgeScript `
        -Backend "127.0.0.1:$BackendPort" `
        -Mode "chat" `
        -WaitReady `
        -ReadyTimeoutSecs 300 `
        -TimeoutSecs ([Math]::Max(300, [int]($RuntimeTimeoutMs / 1000)))
    if ($LASTEXITCODE -ne 0) {
        throw "SmartSteam Forge UI exited with code $LASTEXITCODE"
    }
}
