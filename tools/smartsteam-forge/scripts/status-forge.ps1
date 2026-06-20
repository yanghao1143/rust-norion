param(
    [string]$RepoRoot = "D:\rust-norion",
    [string]$RemoteHost = "192.168.10.11",
    [string]$RemoteUser = "xinghuan",
    [string]$RunDir = "",
    [int]$MistralPort = 8686,
    [int]$BackendPort = 7878,
    [int]$LabPort = 8787,
    [switch]$Raw,
    [switch]$NoModelPool,
    [switch]$NoGpu,
    [switch]$Help
)

$ErrorActionPreference = "SilentlyContinue"

if ([string]::IsNullOrWhiteSpace($RunDir)) {
    $RunDir = Join-Path $RepoRoot "target\remote-gemma-chain"
}

if ($Help) {
    Write-Host "Show SmartSteam Forge, rust-norion, Gemma runtime, and Web lab status."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\status-forge.cmd"
    Write-Host "  .\tools\smartsteam-forge\status-forge.cmd -Raw"
    Write-Host "  .\tools\smartsteam-forge\status-forge.cmd -NoModelPool"
    Write-Host "  .\tools\smartsteam-forge\status-forge.cmd -NoGpu"
    Write-Host "  .\tools\smartsteam-forge\status-gemma-forge.cmd  # defaults to BackendPort=7979 LabPort=8789"
    Write-Host ""
    Write-Host "Remote-chain diagnostics are local-only: TCP checks, pid files, and /health."
    return
}

function Test-TcpEndpoint {
    param(
        [string]$HostName,
        [int]$Port,
        [int]$TimeoutMs = 1500
    )
    try {
        $client = [System.Net.Sockets.TcpClient]::new()
        $async = $client.BeginConnect($HostName, $Port, $null, $null)
        $ready = $async.AsyncWaitHandle.WaitOne($TimeoutMs)
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

function Test-LocalPort {
    param([int]$Port)
    return Test-TcpEndpoint -HostName "127.0.0.1" -Port $Port -TimeoutMs 250
}

function Get-HealthJson {
    param([string]$Url)
    try {
        return Invoke-RestMethod -Uri $Url -TimeoutSec 2
    } catch {
        return $null
    }
}

function Test-ModelApi {
    param([int]$Port)
    try {
        $response = Invoke-WebRequest -Uri "http://127.0.0.1:$Port/v1/models" -UseBasicParsing -TimeoutSec 4
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

function Get-OrUnknown {
    param([object]$Value)
    if ($null -ne $Value) {
        return $Value
    }
    return "unknown"
}

function Join-Values {
    param([object]$Values)
    $items = @($Values) | Where-Object { $_ }
    if ($items.Count -eq 0) {
        return "none"
    }
    return $items -join "; "
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

function Get-StatusReason {
    param([object]$Status)
    $reason = Get-JsonProperty -Object $Status -Name "reason"
    if ($null -eq $reason) {
        $reason = Get-JsonProperty -Object $Status -Name "launch_block_reason"
    }
    return $reason
}

function Get-PortOwnerRows {
    param([int[]]$Ports)
    $rows = @()
    foreach ($port in $Ports) {
        $connections = @(Get-NetTCPConnection -LocalAddress 127.0.0.1 -LocalPort $port -State Listen -ErrorAction SilentlyContinue)
        foreach ($connection in $connections) {
            $process = Get-Process -Id $connection.OwningProcess -ErrorAction SilentlyContinue
            $rows += [pscustomobject]@{
                Port = $port
                Pid = $connection.OwningProcess
                ProcessName = if ($process) { $process.ProcessName } else { "unknown" }
                WorkingSetGB = if ($process) { [math]::Round($process.WorkingSet64 / 1GB, 2) } else { "unknown" }
                Path = if ($process) { $process.Path } else { "unknown" }
            }
        }
    }
    return $rows
}

function Get-PidFileProcessStatus {
    param(
        [string]$Path,
        [string]$ExpectedName,
        [string[]]$ExpectedCommandText
    )

    $exists = Test-Path -LiteralPath $Path
    if (-not $exists) {
        return [pscustomobject]@{
            Path = $Path
            Exists = $false
            Pid = $null
            State = "missing"
            ProcessName = $null
            CommandMatches = $false
        }
    }

    $raw = Get-Content -LiteralPath $Path -ErrorAction SilentlyContinue | Select-Object -First 1
    [int]$processId = 0
    if (-not [int]::TryParse($raw, [ref]$processId)) {
        return [pscustomobject]@{
            Path = $Path
            Exists = $true
            Pid = $raw
            State = "invalid"
            ProcessName = $null
            CommandMatches = $false
        }
    }

    $process = Get-CimInstance Win32_Process -Filter "ProcessId = $processId" -ErrorAction SilentlyContinue
    if ($null -eq $process) {
        return [pscustomobject]@{
            Path = $Path
            Exists = $true
            Pid = $processId
            State = "stale"
            ProcessName = $null
            CommandMatches = $false
        }
    }

    $commandMatches = $true
    foreach ($expectedText in @($ExpectedCommandText)) {
        if (-not [string]::IsNullOrWhiteSpace($expectedText) -and -not ([string]$process.CommandLine).Contains($expectedText)) {
            $commandMatches = $false
            break
        }
    }
    $nameMatches = [string]::IsNullOrWhiteSpace($ExpectedName) -or $process.Name -eq $ExpectedName
    $state = if ($nameMatches -and $commandMatches) { "running" } else { "mismatched" }

    return [pscustomobject]@{
        Path = $Path
        Exists = $true
        Pid = $processId
        State = $state
        ProcessName = $process.Name
        CommandMatches = $commandMatches
    }
}

function Write-BackendSummary {
    param([object]$Health)
    if ($null -eq $Health) {
        Write-Host "backend_health: unreachable"
        return
    }

    Write-Host ("backend_health: runtime={0} gemma_server={1} gemma_reachable={2} readiness_ok={3} safe_device_ok={4} busy={5} active={6}" -f `
        (Get-OrUnknown -Value $Health.runtime_mode), `
        (Get-OrUnknown -Value $Health.gemma_runtime_server), `
        (Get-OrUnknown -Value $Health.gemma_runtime_reachable), `
        (Get-OrUnknown -Value $Health.readiness_ok), `
        (Get-OrUnknown -Value $Health.safe_device_ok), `
        (Get-OrUnknown -Value $Health.engine_busy), `
        (Get-OrUnknown -Value $Health.active_engine_requests))

    $hygiene = Get-JsonProperty -Object $Health -Name "experience_hygiene"
    if ($hygiene) {
        Write-Host ("experience: file={0} checked={1} clean={2} quarantine_candidates={3} findings={4} repairable_legacy_metadata_lessons={5}" -f `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $hygiene -Name "experience_file")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $hygiene -Name "checked")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $hygiene -Name "clean")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $hygiene -Name "quarantine_candidates")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object $hygiene -Name "findings")), `
            (Get-OrUnknown -Value (Get-JsonProperty -Object (Get-JsonProperty -Object $hygiene -Name "repair") -Name "repairable_legacy_metadata_lessons")))
    }

    $readinessFailures = Join-Values -Values $Health.readiness_failures
    $safeDeviceFailures = Join-Values -Values $Health.safe_device_failures
    Write-Host "readiness_failures: $readinessFailures"
    Write-Host "safe_device_failures: $safeDeviceFailures"

    $requests = @($Health.active_requests) | Where-Object { $_ }
    if ($requests.Count -gt 0) {
        Write-Host "active_requests:"
        $requests |
            Select-Object request_id, endpoint, elapsed_ms, prompt_preview |
            Format-Table -AutoSize
    }

    Write-Host ("device: profile={0} lane={1} fallback={2} memory={3} accelerators={4} pressure={5}" -f `
        (Get-OrUnknown -Value $Health.device_profile), `
        (Get-OrUnknown -Value $Health.device_primary_lane), `
        (Get-OrUnknown -Value $Health.device_fallback_lane), `
        (Get-OrUnknown -Value $Health.device_memory_mode), `
        (Get-OrUnknown -Value $Health.device_accelerators), `
        (Get-OrUnknown -Value $Health.device_pressure))

    $lastInference = Get-JsonProperty -Object $Health -Name "last_inference"
    if ($lastInference) {
        Write-Host ("last_inference: request_id={0} endpoint={1} elapsed_ms={2} runtime_model={3} tokens={4} action={5} error={6}" -f `
            (Get-OrUnknown -Value $lastInference.request_id), `
            (Get-OrUnknown -Value $lastInference.endpoint), `
            (Get-OrUnknown -Value $lastInference.elapsed_ms), `
            (Get-OrUnknown -Value $lastInference.runtime_model), `
            (Get-OrUnknown -Value $lastInference.runtime_token_count), `
            (Get-OrUnknown -Value $lastInference.action), `
            (Get-OrUnknown -Value $lastInference.error))
    } else {
        Write-Host "last_inference: none"
    }
}

function Write-ModelRuntimeDiagnosis {
    param([object]$Health)

    $modelListening = Test-LocalPort -Port $MistralPort
    $modelHealthy = Test-ModelApi -Port $MistralPort
    $remoteSshReachable = Test-TcpEndpoint -HostName $RemoteHost -Port 22 -TimeoutMs 1500
    $expectedForward = "$MistralPort`:127.0.0.1:$MistralPort"
    $expectedTarget = "$RemoteUser@$RemoteHost"
    $tunnelPid = Get-PidFileProcessStatus `
        -Path (Join-Path $RunDir "ssh-tunnel.pid") `
        -ExpectedName "ssh.exe" `
        -ExpectedCommandText @("-L", $expectedForward, $expectedTarget)

    Write-Host ""
    Write-Host "Model runtime diagnosis:"
    Write-Host ("model_runtime: endpoint=http://127.0.0.1:{0}/v1/models listening={1} healthy={2}" -f `
        $MistralPort, `
        (Get-OrUnknown -Value $modelListening), `
        (Get-OrUnknown -Value $modelHealthy))

    if ($Health) {
        Write-Host ("backend_points_to: gemma_runtime_server={0} gemma_runtime_reachable={1} metadata_error={2}" -f `
            (Get-OrUnknown -Value $Health.gemma_runtime_server), `
            (Get-OrUnknown -Value $Health.gemma_runtime_reachable), `
            (Get-OrUnknown -Value $Health.gemma_runtime_metadata_error))
    }

    Write-Host ("remote_ssh_probe: target={0}@{1}:22 tcp={2} read_only=true" -f `
        $RemoteUser, `
        $RemoteHost, `
        (Get-OrUnknown -Value $remoteSshReachable))
    Write-Host ("ssh_tunnel_pid: file={0} exists={1} pid={2} state={3} process={4}" -f `
        $tunnelPid.Path, `
        $tunnelPid.Exists, `
        (Get-OrUnknown -Value $tunnelPid.Pid), `
        $tunnelPid.State, `
        (Get-OrUnknown -Value $tunnelPid.ProcessName))

    if ($modelHealthy) {
        Write-Host "diagnosis: model runtime is reachable; prompts can proceed if backend readiness is true and not busy."
        return
    }

    if (-not $modelListening -and $tunnelPid.State -in @("missing", "stale", "invalid")) {
        Write-Host "diagnosis: local 8686 quality-worker entrypoint is down; the backend is correctly blocking prompts."
    } elseif ($modelListening) {
        Write-Host "diagnosis: local 8686 is occupied but /v1/models is not healthy; inspect the port owner or restart the remote chain."
    } else {
        Write-Host "diagnosis: local 8686 is not serving the model API."
    }

    $restoreCommand = ".\tools\smartsteam-forge\start-remote-gemma-chain.cmd -BackendPort $BackendPort -LabPort $LabPort -LocalModelPort $MistralPort -SkipBuild -NoBackend -NoLab"

    if (-not $remoteSshReachable) {
        Write-Host "apple_host: SSH TCP probe failed; the Mac may be asleep, offline, off-network, or SSH may be disabled."
        Write-Host "next_step: verify the Apple host is awake, on the same network/VPN, and SSH is reachable, then run $restoreCommand"
    } else {
        Write-Host "next_step: run $restoreCommand"
    }

    Write-Host "restart_remote: add -RestartRemote only if you intentionally want to stop/start the remote llama-server."
}

function Write-ModelPoolSummary {
    param([object]$Status)
    if ($null -eq $Status) {
        Write-Host "model_pool: unavailable"
        return
    }

    Write-Host ("model_pool: launch_allowed={0} reason={1} workers={2} healthy={3} min_context_tokens={4}" -f `
        (Get-OrUnknown -Value $Status.launch_allowed), `
        (Get-OrUnknown -Value (Get-StatusReason -Status $Status)), `
        (Get-OrUnknown -Value $Status.worker_count), `
        (Get-OrUnknown -Value $Status.healthy_worker_count), `
        (Get-OrUnknown -Value $Status.min_context_tokens))

    $capacity = Get-JsonProperty -Object $Status -Name "capacity"
    if ($capacity) {
        Write-Host ("model_pool_capacity: policy={0} expansion_allowed={1} recommendation={2} helpers={3}/{4} runtime=metal:{5} cpu:{6} unknown:{7} gpu0:{8} quality_accelerated={9}" -f `
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
    }

    $routeMetrics = Get-JsonProperty -Object $Status -Name "route_metrics"
    if ($routeMetrics) {
        Write-Host ("model_pool_route_metrics: routes={0} selected={1} blocked={2} in_flight={3} success={4} failure={5} avg_latency_ms={6}" -f `
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
        Write-Host "model_pool_workers:"
        $workers |
            Select-Object `
                role, status, ready, base_url, context_window, default_max_tokens, `
                in_flight, route_count, selected_count, blocked_count, success_count, failure_count, avg_latency_ms, `
                role_block_reason |
            Format-Table -AutoSize
    }

    $workerMetrics = @($Status.worker_metrics) | Where-Object { $_ }
    if ($workerMetrics.Count -gt 0) {
        Write-Host "model_pool_worker_metrics:"
        $workerMetrics |
            Select-Object role, route_count, selected_count, blocked_count, in_flight, success_count, failure_count, avg_latency_ms |
            Format-Table -AutoSize
    }
}

Write-Host "SmartSteam Forge status"
Write-Host ""

@(
    [pscustomobject]@{ Component = "mistralrs"; Port = $MistralPort; Listening = Test-LocalPort -Port $MistralPort },
    [pscustomobject]@{ Component = "rust-norion"; Port = $BackendPort; Listening = Test-LocalPort -Port $BackendPort },
    [pscustomobject]@{ Component = "rustgpt-lab"; Port = $LabPort; Listening = Test-LocalPort -Port $LabPort }
) | Format-Table -AutoSize

$owners = @(Get-PortOwnerRows -Ports @($MistralPort, $BackendPort, $LabPort))
if ($owners.Count -gt 0) {
    Write-Host ""
    Write-Host "Port owners:"
    $owners | Format-Table -AutoSize
}

$processes = @(Get-Process -Name mistralrs,rust-norion,rustgpt-lab,smartsteam-forge -ErrorAction SilentlyContinue)
if ($processes.Count -gt 0) {
    Write-Host ""
    Write-Host "Matching processes:"
    $processes |
        Select-Object Id, ProcessName, CPU, @{Name = "PrivateGB"; Expression = { [math]::Round($_.PrivateMemorySize64 / 1GB, 2) } }, @{Name = "WorkingSetGB"; Expression = { [math]::Round($_.WorkingSet64 / 1GB, 2) } }, Path |
        Format-Table -AutoSize
} else {
    Write-Host ""
    Write-Host "No mistralrs/rust-norion/rustgpt-lab/smartsteam-forge processes are running."
}

$backendHealth = Get-HealthJson -Url "http://127.0.0.1:$BackendPort/health"
Write-Host ""
Write-BackendSummary -Health $backendHealth
Write-ModelRuntimeDiagnosis -Health $backendHealth

if (-not $NoModelPool) {
    $modelPoolStatus = Get-HealthJson -Url "http://127.0.0.1:$BackendPort/v1/model-pool/status"
    Write-Host ""
    Write-Host "Model pool:"
    if ($Raw -and $modelPoolStatus) {
        $modelPoolStatus | ConvertTo-Json -Compress -Depth 10
    } else {
        Write-ModelPoolSummary -Status $modelPoolStatus
    }
}

$labHealth = Get-HealthJson -Url "http://127.0.0.1:$LabPort/api/backend-health"
if ($labHealth) {
    Write-Host ""
    Write-Host "web_lab_backend_health:"
    if ($Raw) {
        $labHealth | ConvertTo-Json -Compress -Depth 8
    } else {
        Write-Host ("lab_backend: ok={0} service={1} runtime={2} gemma_reachable={3} busy={4} active={5} readiness_ok={6} safe_device_ok={7}" -f `
            (Get-OrUnknown -Value $labHealth.ok), `
            (Get-OrUnknown -Value $labHealth.service), `
            (Get-OrUnknown -Value $labHealth.runtime_mode), `
            (Get-OrUnknown -Value $labHealth.gemma_runtime_reachable), `
            (Get-OrUnknown -Value $labHealth.engine_busy), `
            (Get-OrUnknown -Value $labHealth.active_engine_requests), `
            (Get-OrUnknown -Value $labHealth.readiness_ok), `
            (Get-OrUnknown -Value $labHealth.safe_device_ok))
    }
}

if ($Raw -and $backendHealth) {
    Write-Host ""
    Write-Host "raw_backend_health:"
    $backendHealth | ConvertTo-Json -Compress -Depth 10
}

if (-not $NoGpu) {
    Write-Host ""
    Write-Host "GPU summary:"
    $nvidia = Get-Command nvidia-smi -ErrorAction SilentlyContinue
    if ($null -eq $nvidia) {
        Write-Host "nvidia-smi: unavailable"
    } else {
        & $nvidia.Source --query-gpu=name,memory.total,memory.used,utilization.gpu,power.draw,temperature.gpu --format=csv,noheader,nounits
    }
}
