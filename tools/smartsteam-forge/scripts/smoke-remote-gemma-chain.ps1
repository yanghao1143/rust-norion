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
    [int]$RemoteModelPort = 8686,
    [int]$LocalModelPort = 8686,
    [string]$PoolWorkerRoles = "summary,router,review,index,test-gate",
    [int]$BackendPort = 7979,
    [int]$LabPort = 8789,
    [int]$TimeoutSec = 120,
    [switch]$SkipBuild,
    [switch]$NoForgeCli,
    [switch]$EnablePoolWorkers,
    [switch]$EnableIndexWorker,
    [switch]$EnableSpareWorker,
    [switch]$AllowLargePoolWorkerModels,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Run a SmartSteam remote Gemma chain smoke."
    Write-Host ""
    Write-Host "This starts or reuses the remote Mac model box, SSH tunnel, local rust-norion backend,"
    Write-Host "and Web Lab, then sends tiny Web Lab SSE and Forge CLI prompts and requires OK."
    Write-Host "Before sending prompts it also requires read-only /v1/model-pool/status to expose"
    Write-Host "a ready quality worker whose base_url matches the local model tunnel."
    Write-Host "Each prompt is also guarded by gemma-chain chain-status -RequireAction before"
    Write-Host "the request is sent, so busy/unsafe/dirty states fail before burning model time."
    Write-Host "It uses isolated state under target\remote-gemma-chain and does not copy Rust source"
    Write-Host "to the remote Mac. The chain remains running after the smoke so you can keep testing."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\smoke-remote-gemma-chain.cmd"
    Write-Host "  .\tools\smartsteam-forge\smoke-remote-gemma-chain.cmd -SkipBuild"
    Write-Host "  .\tools\smartsteam-forge\smoke-remote-gemma-chain.cmd -NoForgeCli"
    Write-Host "  .\tools\smartsteam-forge\smoke-remote-gemma-chain.cmd -LocalModelPort 8696 -BackendPort 7979 -LabPort 8789"
    Write-Host "  .\tools\smartsteam-forge\smoke-remote-gemma-chain.cmd -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf"
    Write-Host "  .\tools\smartsteam-forge\smoke-remote-gemma-chain.cmd -EnablePoolWorkers -PoolWorkerRoles review,index -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf"
    Write-Host ""
    Write-Host "Pool worker smoke:"
    Write-Host "  Default smoke validates only the quality worker. -EnablePoolWorkers requires the selected"
    Write-Host "  summary/router/review/index/test-gate workers to be ready in /v1/model-pool/status before prompts."
    Write-Host "  Helper workers reject the quality model path and obvious 12B+ model names by default;"
    Write-Host "  pass -AllowLargePoolWorkerModels only for an explicit stress test."
    return
}

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$Action
    )

    Write-Host ""
    Write-Host "remote smoke step: $Name"
    & $Action
    Write-Host "remote smoke step PASS: $Name"
}

function Invoke-JsonGet {
    param(
        [string]$Url,
        [int]$TimeoutSec = 10
    )

    try {
        return Invoke-RestMethod -Uri $Url -TimeoutSec $TimeoutSec
    } catch {
        throw "GET $Url failed: $($_.Exception.Message)"
    }
}

function Normalize-PoolWorkerRole {
    param([string]$Role)

    $normalized = $Role.Trim().ToLowerInvariant()
    if ($normalized -eq "spare") {
        return "index"
    }
    return $normalized
}

function Get-RequestedPoolWorkerRoles {
    $requested = @($PoolWorkerRoles -split "," | ForEach-Object { Normalize-PoolWorkerRole $_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if (($EnableIndexWorker -or $EnableSpareWorker) -and "index" -notin $requested) {
        $requested += "index"
    }
    return @($requested | Select-Object -Unique)
}

function Assert-ModelApiReady {
    param(
        [string]$Url,
        [int]$TimeoutSec = 20
    )

    $models = Invoke-JsonGet $Url $TimeoutSec
    if ($null -eq $models.models -and $null -eq $models.data) {
        throw "model API did not return models/data"
    }
}

function Assert-BackendModelPoolStatus {
    param(
        [string]$Url,
        [int]$LocalModelPort,
        [string[]]$RequiredWorkerRoles = @(),
        [int]$TimeoutSec = 10
    )

    $status = Invoke-JsonGet $Url $TimeoutSec
    if (-not $status.ok) {
        throw "model pool status ok=false: $($status.error)"
    }
    if ($status.read_only -ne $true) {
        throw "model pool status must be read_only=true"
    }
    if ($status.sends_prompt -ne $false) {
        throw "model pool status must not send prompts"
    }
    if ($status.launches_process -ne $false) {
        throw "model pool status must not launch processes"
    }
    $workers = @($status.workers)
    if ($workers.Count -lt 1) {
        throw "model pool status returned no workers"
    }
    $qualityWorkers = @($workers | Where-Object { $_.role -eq "quality" })
    if ($qualityWorkers.Count -ne 1) {
        throw "model pool status requires exactly one quality worker, got $($qualityWorkers.Count)"
    }
    $quality = $qualityWorkers[0]
    $expectedBaseUrl = "http://127.0.0.1:$LocalModelPort"
    $actualBaseUrl = ([string]$quality.base_url).TrimEnd("/")
    if ($actualBaseUrl -ne $expectedBaseUrl) {
        throw "quality worker base_url mismatch: got $actualBaseUrl expected $expectedBaseUrl"
    }
    if ($quality.port -and [int]$quality.port -ne $LocalModelPort) {
        throw "quality worker port mismatch: got $($quality.port) expected $LocalModelPort"
    }
    if ($quality.ready -ne $true -and $quality.role_ready -ne $true -and $quality.health_ok -ne $true) {
        throw "quality worker is not ready in model pool status"
    }
    if ($status.launch_allowed -ne $true) {
        throw "model pool launch_allowed is not true: $($status.reason)"
    }
    $healthy = if ($null -ne $status.healthy_worker_count) {
        [int]$status.healthy_worker_count
    } else {
        @($workers | Where-Object { $_.ready -eq $true -or $_.role_ready -eq $true -or $_.health_ok -eq $true }).Count
    }
    if ($healthy -lt 1) {
        throw "model pool healthy_worker_count must be at least 1"
    }

    $expectedWorkerPorts = @{
        summary = 8687
        review = 8688
        "router" = 8689
        "test-gate" = 8688
        index = 8690
    }
    foreach ($role in @($RequiredWorkerRoles)) {
        if ([string]::IsNullOrWhiteSpace($role)) {
            continue
        }
        if (-not $expectedWorkerPorts.ContainsKey($role)) {
            throw "unknown required pool worker role=$role"
        }
        $matches = @($workers | Where-Object { $_.role -eq $role })
        if ($matches.Count -ne 1) {
            throw "model pool status requires exactly one $role worker, got $($matches.Count)"
        }
        $worker = $matches[0]
        $expectedPort = [int]$expectedWorkerPorts[$role]
        $expectedWorkerBaseUrl = "http://127.0.0.1:$expectedPort"
        $actualWorkerBaseUrl = ([string]$worker.base_url).TrimEnd("/")
        if ($actualWorkerBaseUrl -ne $expectedWorkerBaseUrl) {
            throw "$role worker base_url mismatch: got $actualWorkerBaseUrl expected $expectedWorkerBaseUrl"
        }
        if ($worker.port -and [int]$worker.port -ne $expectedPort) {
            throw "$role worker port mismatch: got $($worker.port) expected $expectedPort"
        }
        if ($worker.ready -ne $true -and $worker.role_ready -ne $true -and $worker.health_ok -ne $true) {
            throw "$role worker is not ready in model pool status"
        }
    }

    $requiredText = if (@($RequiredWorkerRoles).Count -gt 0) {
        " required_workers=$(@($RequiredWorkerRoles) -join ',')"
    } else {
        ""
    }
    Write-Host "model pool status ok: healthy=$healthy/$($status.worker_count) launch_allowed=$($status.launch_allowed) quality=$actualBaseUrl$requiredText"
}

function Invoke-WebLabSmokePrompt {
    param(
        [string]$Url,
        [int]$TimeoutSec
    )

    $body = '{"messages":[{"role":"user","content":"Reply only with OK."}],"profile":"general","output":"raw"}'
    $response = $body | curl.exe -sS -N --max-time $TimeoutSec `
        -H "Content-Type: application/json" `
        --data-binary "@-" `
        $Url
    if ($LASTEXITCODE -ne 0) {
        throw "curl failed with exit code $LASTEXITCODE"
    }
    $text = $response -join "`n"
    Write-Host $text.TrimEnd()

    if ($text -notmatch 'event:\s*delta\s+data:\s*OK') {
        throw "Web Lab SSE smoke did not stream delta=OK"
    }
    if ($text -notmatch 'event:\s*done\s+data:\s*\[DONE\]') {
        throw "Web Lab SSE smoke did not finish with done=[DONE]"
    }
    if ($text -match 'event:\s*error') {
        throw "Web Lab SSE smoke returned an error event"
    }
}

function Wait-BackendIdle {
    param(
        [string]$Url,
        [int]$TimeoutSec
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    do {
        $health = Invoke-JsonGet $Url 10
        if ($health.engine_busy -ne $true) {
            Write-Host "backend idle: requests_seen=$($health.requests_seen) active=$($health.active_engine_requests)"
            return
        }
        $active = @($health.active_requests)
        if ($active.Count -gt 0) {
            $first = $active[0]
            Write-Host "backend busy: request_id=$($first.request_id) endpoint=$($first.endpoint) elapsed_ms=$($first.elapsed_ms)"
        } else {
            Write-Host "backend busy"
        }
        Start-Sleep -Seconds 2
    } while ((Get-Date) -lt $deadline)

    throw "backend did not become idle within ${TimeoutSec}s"
}

function Invoke-ChainActionGate {
    param(
        [string]$Action
    )

    if (-not (Test-Path -LiteralPath $gemmaChain)) {
        throw "gemma-chain command not found: $gemmaChain"
    }

    $gateArgs = @(
        "chain-status",
        "-ModelBaseUrl", "http://127.0.0.1:$LocalModelPort",
        "-BackendBaseUrl", "http://127.0.0.1:$BackendPort",
        "-LabBaseUrl", "http://127.0.0.1:$LabPort",
        "-TimeoutSec", "10",
        "-RequireAction", $Action,
        "-JsonStatus",
        "-FailIfBlocked"
    )
    $outputLines = & $gemmaChain @gateArgs 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($outputLines | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "gemma-chain action gate failed for $Action with exit code $exitCode"
    }

    try {
        $status = $text | ConvertFrom-Json
    } catch {
        throw "gemma-chain action gate returned non-json output for ${Action}: $($_.Exception.Message)"
    }
    if ($status.schema_version -ne 1) {
        throw "gemma-chain action gate schema_version mismatch for ${Action}: $($status.schema_version)"
    }
    if ($status.contract_version -ne "gemma-chain.v1") {
        throw "gemma-chain action gate contract_version mismatch for ${Action}: $($status.contract_version)"
    }
    if ($status.machine_summary.read_only -ne $true -or $status.machine_summary.sends_prompt -ne $false -or $status.machine_summary.launches_process -ne $false) {
        throw "gemma-chain action gate contract violation for ${Action}: gate must be read-only and non-launching"
    }
    if ($status.require_action -ne $Action) {
        throw "gemma-chain action gate require_action mismatch: got $($status.require_action) expected $Action"
    }
    if ($status.require_action_allowed -ne $true) {
        throw "gemma-chain action gate blocked ${Action}: classification=$($status.classification) next_step=$($status.next_step)"
    }

    Write-Host "gemma-chain action gate ok: action=$Action classification=$($status.classification)"
}

function Invoke-ForgeCliSmokePrompt {
    param(
        [string]$ForgeDir,
        [string]$Backend,
        [int]$TimeoutSec
    )

    Push-Location $ForgeDir
    try {
        $oldErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $output = (& cargo run -- --backend $Backend --mode chat --prompt "Reply only with OK." --require-health --require-safe-device --timeout-secs $TimeoutSec 2>&1 |
                ForEach-Object { $_.ToString() } |
                Out-String)
            $exitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $oldErrorActionPreference
        }
    } finally {
        Pop-Location
    }

    if (-not [string]::IsNullOrWhiteSpace($output)) {
        Write-Host $output.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "Forge CLI smoke failed with exit code $exitCode"
    }
    if ($output -notmatch '(?m)^\s*OK\s*$') {
        throw "Forge CLI smoke did not print an OK answer line"
    }
    if ($output -notmatch 'status:\s*final ok=true') {
        throw "Forge CLI smoke did not report final ok=true"
    }
}

if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot "Cargo.toml"))) {
    throw "RepoRoot does not look like rust-norion: $RepoRoot"
}

$allowedPoolRoles = @("summary", "router", "review", "index", "test-gate")
$requestedPoolWorkerRoles = @(Get-RequestedPoolWorkerRoles)
$requiredPoolWorkerRoles = if ($EnablePoolWorkers) { @($requestedPoolWorkerRoles) } else { @() }
$unknownPoolRoles = @($requestedPoolWorkerRoles | Where-Object { $_ -notin $allowedPoolRoles })
if ($unknownPoolRoles.Count -gt 0) {
    throw "unknown PoolWorkerRoles: $($unknownPoolRoles -join ','); allowed roles: $($allowedPoolRoles -join ',')"
}
if ($EnablePoolWorkers -and [string]::IsNullOrWhiteSpace($RemoteSmallModel)) {
    throw "-EnablePoolWorkers requires -RemoteSmallModel pointing at a smaller or lower-quant GGUF on the remote Mac"
}
if ($EnablePoolWorkers -and $requestedPoolWorkerRoles.Count -eq 0) {
    throw "-EnablePoolWorkers selected no roles; set -PoolWorkerRoles summary,router,review,index,test-gate"
}

$forgeDir = Join-Path $RepoRoot "tools\smartsteam-forge"
$gemmaChain = Join-Path $RepoRoot "tools\gemma-chain\gemma-chain.cmd"
$startChain = Join-Path $forgeDir "scripts\start-remote-gemma-chain.ps1"
$statusChain = Join-Path $forgeDir "status-remote-gemma-chain.cmd"
$modelUrl = "http://127.0.0.1:$LocalModelPort/v1/models"
$backendUrl = "http://127.0.0.1:$BackendPort/health"
$modelPoolStatusUrl = "http://127.0.0.1:$BackendPort/v1/model-pool/status"
$labHealthUrl = "http://127.0.0.1:$LabPort/health"
$labStreamUrl = "http://127.0.0.1:$LabPort/api/chat-stream"

function Invoke-StartRemoteChain {
    param([switch]$OnlyModelTunnel)

    $startArgs = @{
        RepoRoot          = $RepoRoot
        RemoteHost       = $RemoteHost
        RemoteUser       = $RemoteUser
        IdentityFile     = $IdentityFile
        RemoteRoot       = $RemoteRoot
        RemoteLlamaServer = $RemoteLlamaServer
        RemoteModel      = $RemoteModel
        RemoteSmallLlamaServer = $RemoteSmallLlamaServer
        RemoteSmallModel = $RemoteSmallModel
        RemoteModelPort  = $RemoteModelPort
        LocalModelPort   = $LocalModelPort
        PoolWorkerRoles  = $PoolWorkerRoles
        BackendPort      = $BackendPort
        LabPort          = $LabPort
    }
    if ($SkipBuild -or $OnlyModelTunnel) {
        $startArgs.SkipBuild = $true
    }
    if ($OnlyModelTunnel) {
        $startArgs.NoBackend = $true
        $startArgs.NoLab = $true
    }
    if ($EnablePoolWorkers) {
        $startArgs.EnablePoolWorkers = $true
    }
    if ($EnableIndexWorker) {
        $startArgs.EnableIndexWorker = $true
    }
    if ($EnableSpareWorker) {
        $startArgs.EnableSpareWorker = $true
    }
    if ($AllowLargePoolWorkerModels) {
        $startArgs.AllowLargePoolWorkerModels = $true
    }

    & $startChain @startArgs
    if ($LASTEXITCODE -ne 0) {
        throw "start remote chain failed with exit code $LASTEXITCODE"
    }
}

Write-Host "SmartSteam remote Gemma smoke"
Write-Host "Remote: $RemoteUser@$RemoteHost"
Write-Host "Model API: $modelUrl"
Write-Host "Backend: $backendUrl"
Write-Host "Web Lab: http://127.0.0.1:$LabPort/"
Write-Host "Forge CLI: backend 127.0.0.1:$BackendPort"
if ($EnablePoolWorkers) {
    Write-Host "Pool workers: $($requestedPoolWorkerRoles -join ',')"
}

Invoke-Step "start or reuse remote chain" {
    Invoke-StartRemoteChain
}

Invoke-Step "status" {
    & $statusChain `
        -RemoteHost $RemoteHost `
        -RemoteUser $RemoteUser `
        -IdentityFile $IdentityFile `
        -RemoteRoot $RemoteRoot `
        -RemoteLlamaServer $RemoteLlamaServer `
        -RemoteModel $RemoteModel `
        -RemoteModelPort $RemoteModelPort `
        -LocalModelPort $LocalModelPort `
        -BackendPort $BackendPort `
        -LabPort $LabPort
    if ($LASTEXITCODE -ne 0) {
        throw "status remote chain failed with exit code $LASTEXITCODE"
    }
}

Invoke-Step "model API" {
    Assert-ModelApiReady $modelUrl 20
}

Invoke-Step "backend health" {
    $health = Invoke-JsonGet $backendUrl 10
    if (-not $health.ok) {
        throw "backend health ok=false"
    }
    if ($health.runtime_mode -ne "gemma-http") {
        throw "backend runtime_mode is $($health.runtime_mode), expected gemma-http"
    }
    if ($health.gemma_runtime_reachable -ne $true) {
        throw "backend reports gemma_runtime_reachable=false"
    }
    Write-Host "backend runtime=$($health.runtime_mode) gemma_runtime_reachable=$($health.gemma_runtime_reachable)"
}

Invoke-Step "backend model pool status" {
    Assert-BackendModelPoolStatus -Url $modelPoolStatusUrl -LocalModelPort $LocalModelPort -RequiredWorkerRoles $requiredPoolWorkerRoles -TimeoutSec 10
}

Invoke-Step "web lab health" {
    $health = Invoke-JsonGet $labHealthUrl 10
    if (-not $health.ok) {
        throw "web lab health ok=false"
    }
    Write-Host "web lab backend=$($health.backend)"
}

Invoke-Step "wait for idle before Web Lab prompt" {
    Wait-BackendIdle -Url $backendUrl -TimeoutSec $TimeoutSec
}

Invoke-Step "Web Lab action gate" {
    Invoke-ChainActionGate -Action "web_lab_prompt"
}

Invoke-Step "web lab SSE prompt" {
    Invoke-WebLabSmokePrompt -Url $labStreamUrl -TimeoutSec $TimeoutSec
}

if (-not $NoForgeCli) {
    Invoke-Step "wait for idle before Forge CLI prompt" {
        Wait-BackendIdle -Url $backendUrl -TimeoutSec $TimeoutSec
    }

    Invoke-Step "Forge CLI action gate" {
        Invoke-ChainActionGate -Action "forge_cli_prompt"
    }

    Invoke-Step "Forge CLI one-shot prompt" {
        Invoke-ForgeCliSmokePrompt -ForgeDir $forgeDir -Backend "127.0.0.1:$BackendPort" -TimeoutSec $TimeoutSec
    }
}

Invoke-Step "final chain readiness" {
    try {
        Assert-ModelApiReady $modelUrl 20
    } catch {
        Write-Warning "model API was not reachable after smoke; rebuilding only the SSH tunnel..."
        Invoke-StartRemoteChain -OnlyModelTunnel
        Assert-ModelApiReady $modelUrl 20
    }

    $health = Invoke-JsonGet $backendUrl 10
    if (-not $health.ok) {
        throw "backend health ok=false after smoke"
    }
    if ($health.gemma_runtime_reachable -ne $true) {
        throw "backend reports gemma_runtime_reachable=false after smoke"
    }

    $labHealth = Invoke-JsonGet $labHealthUrl 10
    if (-not $labHealth.ok) {
        throw "web lab health ok=false after smoke"
    }
}

Write-Host ""
Write-Host "remote smoke: PASS"
Write-Host "remote chain remains running for interactive testing."
