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
    [string]$PoolWorkerRoles = "summary,router,review,index,test-gate",
    [string]$RequiredPoolWorkerRoles = "",
    [int]$SummaryPort = 8687,
    [int]$ReviewPort = 8688,
    [int]$RouterPort = 8689,
    [int]$TestGatePort = 8688,
    [int]$IndexPort = 8690,
    [int]$BackendPort = 7979,
    [int]$LabPort = 8789,
    [int]$ContextTokens = 65536,
    [int]$DefaultMaxTokens = 4096,
    [int]$SummaryContextTokens = 8192,
    [int]$RouterContextTokens = 4096,
    [int]$ReviewContextTokens = 8192,
    [int]$TestGateContextTokens = 4096,
    [int]$IndexContextTokens = 4096,
    [int]$SummaryDefaultMaxTokens = 768,
    [int]$RouterDefaultMaxTokens = 512,
    [int]$ReviewDefaultMaxTokens = 1536,
    [int]$TestGateDefaultMaxTokens = 1536,
    [int]$IndexDefaultMaxTokens = 512,
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
    [string]$RunDir = "",
    [string]$ModelPoolManifest = "",
    [string]$ModelCacheStatusJson = "",
    [switch]$NoManifest,
    [switch]$SkipBuild,
    [switch]$NoLab,
    [switch]$EnablePoolWorkers,
    [switch]$EnableIndexWorker,
    [switch]$EnableSpareWorker,
    [switch]$AllowLargePoolWorkerModels,
    [switch]$UseMac32GBModelPool,
    [string]$RemoteGemma3TinyModel = "/Users/xinghuan/smartsteam-model-box/models/gemma-3-270m-it-qat-Q4_0.gguf",
    [string]$RemoteFunctionGemmaModel = "/Users/xinghuan/smartsteam-model-box/models/functiongemma-270m-it-Q4_K_M.gguf",
    [string]$RemoteE2BModel = "/Users/xinghuan/smartsteam-model-box/models/gemma-4-E2B-it-Q4_K_M.gguf",
    [string]$RemoteE4BModel = "/Users/xinghuan/smartsteam-model-box/models/gemma-4-E4B-it-Q4_K_M.gguf",
    [switch]$RestartRemote,
    [switch]$NoTunnel,
    [switch]$NoForge,
    [switch]$CheckOnly,
    [switch]$Status,
    [switch]$JsonStatus,
    [switch]$ProbeRemoteRuntime,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

function Write-RemoteGemmaForgeOperatorGuide {
    Write-Host "Recommended topology:"
    Write-Host "  quality: exactly one Qwen 14B quality worker on remote $RemoteModelPort, tunneled to local $LocalModelPort"
    Write-Host "  helpers: optional small or low-quant summary/router/review/index/test-gate workers on 8687-8690"
    Write-Host "  Apple Silicon: do not run multiple large quality workers; they compete for unified memory and GPU"
    Write-Host ""
    Write-Host "One-command path:"
    Write-Host "  CheckOnly: .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly"
    Write-Host "  Start:     .\tools\smartsteam-forge\start-remote-gemma-forge.cmd"
    Write-Host "  Helpers:   .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf"
    Write-Host "  Mac32GB:   .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -UseMac32GBModelPool -NoForge"
}

if ($Help) {
    Write-Host "Start remote Gemma, local rust-norion backend, optional Web Lab, model-pool manifest, and SmartSteam Forge."
    Write-Host ""
    Write-RemoteGemmaForgeOperatorGuide
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-forge.cmd"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -SkipBuild"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -NoForge"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -EnablePoolWorkers -RemoteSummaryModel /models/summary-Q4.gguf -RemoteRouterModel /models/function-router-Q4.gguf -RemoteReviewModel /models/review-Q4.gguf -RemoteTestGateModel /models/test-gate-Q4.gguf"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -UseMac32GBModelPool -NoForge"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -Status"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -Status -JsonStatus"
    Write-Host "  .\tools\smartsteam-forge\start-remote-gemma-forge.cmd -Status -JsonStatus -ProbeRemoteRuntime"
    Write-Host ""
    Write-Host "Defaults:"
    Write-Host "  model tunnel 127.0.0.1:$LocalModelPort -> ${RemoteHost}:$RemoteModelPort"
    Write-Host "  backend      127.0.0.1:$BackendPort"
    Write-Host "  web lab      127.0.0.1:$LabPort"
    Write-Host "  manifest     generated from .\tools\gemma-chain\gemma-chain.cmd pool-manifest"
    Write-Host ""
    Write-Host "Notes:"
    Write-Host "  This wrapper starts the existing remote chain launcher with -LaunchForge."
    Write-Host "  -CheckOnly generates and validates the local manifest, then exits before SSH/start/prompt."
    Write-Host "  -Status -JsonStatus emits local-only machine JSON; it does not SSH or start processes."
    Write-Host "  -ProbeRemoteRuntime adds read-only SSH/lsof/ps runtime placement to -Status -JsonStatus."
    Write-Host "  -EnablePoolWorkers starts optional small workers on 8687-8690; use -RemoteSmallModel or per-role Remote*Model overrides."
    Write-Host "  Helper workers reject the quality model path and obvious 12B+ model names by default; pass -AllowLargePoolWorkerModels only for an explicit stress test."
    Write-Host "  -EnableIndexWorker explicitly includes 8690/index; -EnableSpareWorker is a deprecated alias."
    Write-Host "  The remote Mac receives only model-serving commands; Rust source stays local."
    Write-Host "  Use -NoManifest to start without a model-pool manifest."
    return
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot not found: $RepoRoot"
}

function Set-ContentWithRetry {
    param(
        [string]$Path,
        [string]$Value
    )

    for ($attempt = 1; $attempt -le 5; $attempt++) {
        try {
            Set-Content -LiteralPath $Path -Value $Value -Encoding UTF8
            return
        } catch {
            if ($attempt -ge 5) {
                throw
            }
            Start-Sleep -Milliseconds (150 * $attempt)
        }
    }
}

if ([string]::IsNullOrWhiteSpace($RunDir)) {
    $RunDir = Join-Path $RepoRoot "target\remote-gemma-chain"
}

$chainScript = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\start-remote-gemma-chain.ps1"
if (-not (Test-Path -LiteralPath $chainScript -PathType Leaf)) {
    throw "remote chain launcher not found: $chainScript"
}

if ($UseMac32GBModelPool) {
    $EnablePoolWorkers = $true
    $PoolWorkerRoles = "summary,router,review,index,test-gate"
    if (-not $PSBoundParameters.ContainsKey("ContextTokens")) {
        $ContextTokens = 65536
    }
    if (-not $PSBoundParameters.ContainsKey("DefaultMaxTokens")) {
        $DefaultMaxTokens = 4096
    }
    if ([string]::IsNullOrWhiteSpace($RequiredPoolWorkerRoles)) {
        $RequiredPoolWorkerRoles = $PoolWorkerRoles
    }
    if ([string]::IsNullOrWhiteSpace($RemoteSummaryModel)) {
        $RemoteSummaryModel = $RemoteGemma3TinyModel
    }
    if ([string]::IsNullOrWhiteSpace($RemoteRouterModel)) {
        $RemoteRouterModel = $RemoteFunctionGemmaModel
    }
    if ([string]::IsNullOrWhiteSpace($RemoteIndexModel)) {
        $RemoteIndexModel = $RemoteE2BModel
    }
    if ([string]::IsNullOrWhiteSpace($RemoteReviewModel)) {
        $RemoteReviewModel = $RemoteE4BModel
    }
    if ([string]::IsNullOrWhiteSpace($RemoteTestGateModel)) {
        $RemoteTestGateModel = $RemoteE4BModel
    }
    $SummaryPort = 8687
    $RouterPort = 8689
    $IndexPort = 8690
    $ReviewPort = 8688
    $TestGatePort = 8688
    $SummaryContextTokens = 8192
    $RouterContextTokens = 4096
    $IndexContextTokens = 8192
    $ReviewContextTokens = 4096
    $TestGateContextTokens = 4096
    $SummaryDefaultMaxTokens = 768
    $RouterDefaultMaxTokens = 512
    $IndexDefaultMaxTokens = 512
    $ReviewDefaultMaxTokens = 1536
    $TestGateDefaultMaxTokens = 1536
    $SummaryGpuLayers = 999
    $RouterGpuLayers = 999
    $IndexGpuLayers = 999
    $ReviewGpuLayers = 999
    $TestGateGpuLayers = 999
    $IndexDevice = ""
    $ReviewDevice = ""
    $TestGateDevice = ""
    $SmallParallelSlots = 1
    $SmallBatchSize = 128
    $SmallUbatchSize = 32
    $SmallCacheRamMiB = 0
}

if ($Status) {
    $statusArgs = @(
        "-RepoRoot", $RepoRoot,
        "-RemoteHost", $RemoteHost,
        "-RemoteUser", $RemoteUser,
        "-IdentityFile", $IdentityFile,
        "-RemoteRoot", $RemoteRoot,
        "-RemoteLlamaServer", $RemoteLlamaServer,
        "-RemoteModel", $RemoteModel,
        "-RemoteModelPort", $RemoteModelPort,
        "-LocalModelPort", $LocalModelPort,
        "-PoolWorkerRoles", $PoolWorkerRoles,
        "-SummaryPort", $SummaryPort,
        "-RouterPort", $RouterPort,
        "-ReviewPort", $ReviewPort,
        "-TestGatePort", $TestGatePort,
        "-IndexPort", $IndexPort,
        "-BackendPort", $BackendPort,
        "-LabPort", $LabPort,
        "-SummaryContextTokens", $SummaryContextTokens,
        "-RouterContextTokens", $RouterContextTokens,
        "-ReviewContextTokens", $ReviewContextTokens,
        "-TestGateContextTokens", $TestGateContextTokens,
        "-IndexContextTokens", $IndexContextTokens,
        "-RunDir", $RunDir,
        "-Status"
    )
    if (-not [string]::IsNullOrWhiteSpace($RequiredPoolWorkerRoles)) {
        $statusArgs += @("-RequiredPoolWorkerRoles", $RequiredPoolWorkerRoles)
    }
    if (-not [string]::IsNullOrWhiteSpace($RemoteSmallLlamaServer)) {
        $statusArgs += @("-RemoteSmallLlamaServer", $RemoteSmallLlamaServer)
    }
    if (-not [string]::IsNullOrWhiteSpace($RemoteSmallModel)) {
        $statusArgs += @("-RemoteSmallModel", $RemoteSmallModel)
    }
    foreach ($roleConfig in @(
        @{ ServerArg = "-RemoteSummaryLlamaServer"; ServerValue = $RemoteSummaryLlamaServer; ModelArg = "-RemoteSummaryModel"; ModelValue = $RemoteSummaryModel },
        @{ ServerArg = "-RemoteRouterLlamaServer"; ServerValue = $RemoteRouterLlamaServer; ModelArg = "-RemoteRouterModel"; ModelValue = $RemoteRouterModel },
        @{ ServerArg = "-RemoteReviewLlamaServer"; ServerValue = $RemoteReviewLlamaServer; ModelArg = "-RemoteReviewModel"; ModelValue = $RemoteReviewModel },
        @{ ServerArg = "-RemoteTestGateLlamaServer"; ServerValue = $RemoteTestGateLlamaServer; ModelArg = "-RemoteTestGateModel"; ModelValue = $RemoteTestGateModel },
        @{ ServerArg = "-RemoteIndexLlamaServer"; ServerValue = $RemoteIndexLlamaServer; ModelArg = "-RemoteIndexModel"; ModelValue = $RemoteIndexModel }
    )) {
        if (-not [string]::IsNullOrWhiteSpace($roleConfig.ServerValue)) {
            $statusArgs += @($roleConfig.ServerArg, $roleConfig.ServerValue)
        }
        if (-not [string]::IsNullOrWhiteSpace($roleConfig.ModelValue)) {
            $statusArgs += @($roleConfig.ModelArg, $roleConfig.ModelValue)
        }
    }
    if ($JsonStatus) {
        $statusArgs += "-JsonStatus"
    }
    if ($ProbeRemoteRuntime) {
        $statusArgs += "-ProbeRemoteRuntime"
    }
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $chainScript @statusArgs
    exit $LASTEXITCODE
}

function Normalize-PoolWorkerRole {
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

function Get-PoolWorkerContextTokens {
    param([string]$Role)

    switch ($Role) {
        "summary" { return $SummaryContextTokens }
        "router" { return $RouterContextTokens }
        "review" { return $ReviewContextTokens }
        "test-gate" { return $TestGateContextTokens }
        "index" { return $IndexContextTokens }
        default { return 0 }
    }
}

function Get-PoolWorkerDefaultMaxTokens {
    param([string]$Role)

    switch ($Role) {
        "summary" { return $SummaryDefaultMaxTokens }
        "router" { return $RouterDefaultMaxTokens }
        "review" { return $ReviewDefaultMaxTokens }
        "test-gate" { return $TestGateDefaultMaxTokens }
        "index" { return $IndexDefaultMaxTokens }
        default { return 0 }
    }
}

function Get-PoolWorkerPort {
    param([string]$Role)

    switch ($Role) {
        "summary" { return $SummaryPort }
        "router" { return $RouterPort }
        "review" { return $ReviewPort }
        "test-gate" { return $TestGatePort }
        "index" { return $IndexPort }
        default { return 0 }
    }
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
    param([string[]]$Roles)

    $missingModels = @($Roles | Where-Object { [string]::IsNullOrWhiteSpace((Get-PoolWorkerModel $_)) })
    if ($missingModels.Count -gt 0) {
        throw "-EnablePoolWorkers requires -RemoteSmallModel or per-role Remote*Model values for: $($missingModels -join ',')"
    }
    if ($AllowLargePoolWorkerModels) {
        return
    }

    $qualityModel = Normalize-RemoteModelPath $RemoteModel
    $sameAsQuality = @($Roles | Where-Object {
        (Normalize-RemoteModelPath (Get-PoolWorkerModel $_)) -eq $qualityModel
    })
    if ($sameAsQuality.Count -gt 0) {
        throw "-EnablePoolWorkers would start helper roles with the same model as quality ($RemoteModel): $($sameAsQuality -join ','). Use a small/low-quant helper model, or pass -AllowLargePoolWorkerModels only for an explicit stress test."
    }

    $largeModels = @($Roles | Where-Object { Test-PoolWorkerModelLooksLarge (Get-PoolWorkerModel $_) })
    if ($largeModels.Count -gt 0) {
        $details = @($largeModels | ForEach-Object { "$_=$(Get-PoolWorkerModel $_)" }) -join "; "
        throw "-EnablePoolWorkers rejects helper model paths that look 12B+ by default: $details. Use small helpers for normal development, or pass -AllowLargePoolWorkerModels only for an explicit stress test."
    }
}

function Set-ManifestWorkerBudget {
    param(
        [object]$Manifest,
        [string]$Role,
        [int]$ContextTokens,
        [int]$DefaultMaxTokens
    )

    $worker = Get-ManifestWorker -Manifest $Manifest -Role $Role
    if ($null -eq $worker) {
        return
    }
    $worker | Add-Member -NotePropertyName default_context_tokens -NotePropertyValue $ContextTokens -Force
    $worker | Add-Member -NotePropertyName default_max_tokens -NotePropertyValue $DefaultMaxTokens -Force
}

function Set-ManifestWorkerRuntime {
    param(
        [object]$Manifest,
        [string]$Role,
        [int]$GpuLayerCount,
        [string]$Device
    )

    $worker = Get-ManifestWorker -Manifest $Manifest -Role $Role
    if ($null -eq $worker) {
        return
    }
    $deviceText = ([string]$Device).Trim().ToLowerInvariant()
    $usesCpu = $GpuLayerCount -eq 0 -or $deviceText -eq "none" -or $deviceText -eq "cpu"
    $worker | Add-Member -NotePropertyName runtime_backend -NotePropertyValue "llama.cpp" -Force
    if ($usesCpu) {
        $worker | Add-Member -NotePropertyName runtime_device -NotePropertyValue "cpu" -Force
        $worker | Add-Member -NotePropertyName runtime_accelerator -NotePropertyValue "accelerate" -Force
    } else {
        $worker | Add-Member -NotePropertyName runtime_device -NotePropertyValue "metal" -Force
        $worker | Add-Member -NotePropertyName runtime_accelerator -NotePropertyValue "metal" -Force
    }
    $worker | Add-Member -NotePropertyName gpu_layers -NotePropertyValue $GpuLayerCount -Force
}

function Set-ManifestWorkerEndpoint {
    param(
        [object]$Manifest,
        [string]$Role,
        [int]$Port
    )

    $worker = Get-ManifestWorker -Manifest $Manifest -Role $Role
    if ($null -eq $worker) {
        return
    }
    $worker | Add-Member -NotePropertyName port -NotePropertyValue $Port -Force
    $worker | Add-Member -NotePropertyName base_url -NotePropertyValue "http://127.0.0.1:$Port" -Force
}

function Update-ManifestCapacityPolicyBudgets {
    param(
        [object]$Manifest,
        [string[]]$HelperRoles,
        [int]$QualityContextTokens
    )

    $capacityPolicy = $Manifest.PSObject.Properties["capacity_policy"]
    if ($null -eq $capacityPolicy) {
        return
    }

    $helperContextTotal = 0
    $helperDefaultMaxTotal = 0
    foreach ($role in $HelperRoles) {
        $worker = Get-ManifestWorker -Manifest $Manifest -Role $role
        if ($null -eq $worker) {
            continue
        }
        $helperContextTotal += [int]$worker.default_context_tokens
        $helperDefaultMaxTotal += [int]$worker.default_max_tokens
    }

    $capacityPolicy.Value | Add-Member -NotePropertyName quality_required_context_tokens -NotePropertyValue $QualityContextTokens -Force
    $capacityPolicy.Value | Add-Member -NotePropertyName helper_context_tokens_total -NotePropertyValue $helperContextTotal -Force
    $capacityPolicy.Value | Add-Member -NotePropertyName helper_default_max_tokens_total -NotePropertyValue $helperDefaultMaxTotal -Force
    $capacityPolicy.Value | Add-Member -NotePropertyName expansion_gate -NotePropertyValue "quality worker must be reachable, prompt-ready, context>=$QualityContextTokens, and Metal/GPU accelerated before helper expansion" -Force
}

$allowedPoolRoles = @("summary", "router", "review", "index", "test-gate")
$requestedPoolRoles = @($PoolWorkerRoles -split "," | ForEach-Object { Normalize-PoolWorkerRole $_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
if (($EnableIndexWorker -or $EnableSpareWorker) -and "index" -notin $requestedPoolRoles) {
    $requestedPoolRoles += "index"
}
$unknownPoolRoles = @($requestedPoolRoles | Where-Object { $_ -notin $allowedPoolRoles })
if ($unknownPoolRoles.Count -gt 0) {
    throw "unknown PoolWorkerRoles: $($unknownPoolRoles -join ','); allowed roles: $($allowedPoolRoles -join ',')"
}
if ($EnablePoolWorkers) {
    Assert-PoolWorkerLaunchInputs -Roles $requestedPoolRoles
}
$poolWorkerPorts = @{
    summary = $SummaryPort
    router = $RouterPort
    review = $ReviewPort
    "test-gate" = $TestGatePort
    index = $IndexPort
}

function Normalize-ManifestBaseUrl {
    param([string]$BaseUrl)

    if ([string]::IsNullOrWhiteSpace($BaseUrl)) {
        return ""
    }
    return $BaseUrl.Trim().TrimEnd("/")
}

function Get-ManifestWorker {
    param(
        [object]$Manifest,
        [string]$Role
    )

    $matches = @($Manifest.workers | Where-Object { $_.role -eq $Role })
    if ($matches.Count -gt 1) {
        throw "ModelPoolManifest has duplicate worker role=$Role"
    }
    if ($matches.Count -eq 0) {
        return $null
    }
    return $matches[0]
}

function Assert-ManifestWorkerEndpoint {
    param(
        [object]$Manifest,
        [string]$Role,
        [int]$ExpectedPort
    )

    $worker = Get-ManifestWorker -Manifest $Manifest -Role $Role
    if ($null -eq $worker) {
        throw "ModelPoolManifest requires worker role=$Role"
    }
    if ($worker.port -and [int]$worker.port -ne $ExpectedPort) {
        throw "ModelPoolManifest worker role=$Role has port=$($worker.port), expected $ExpectedPort"
    }

    $expectedBaseUrl = "http://127.0.0.1:$ExpectedPort"
    $actualBaseUrl = Normalize-ManifestBaseUrl ([string]$worker.base_url)
    if ($actualBaseUrl -ne $expectedBaseUrl) {
        throw "ModelPoolManifest worker role=$Role has base_url=$actualBaseUrl, expected $expectedBaseUrl for this remote chain"
    }
    return $worker
}

if (-not $NoManifest -and [string]::IsNullOrWhiteSpace($ModelPoolManifest)) {
    $gemmaChain = Join-Path $RepoRoot "tools\gemma-chain\gemma-chain.cmd"
    if (-not (Test-Path -LiteralPath $gemmaChain -PathType Leaf)) {
        throw "gemma-chain command not found: $gemmaChain"
    }

    $manifestDir = Join-Path $RepoRoot "target\gemma-chain"
    New-Item -ItemType Directory -Force -Path $manifestDir | Out-Null
    $ModelPoolManifest = Join-Path $manifestDir "apple-model-pool.generated.json"

    Write-Host "generating model-pool manifest: $ModelPoolManifest"
    $manifestJson = & $gemmaChain pool-manifest -JsonStatus -ModelBaseUrl "http://127.0.0.1:$LocalModelPort"
    if ($LASTEXITCODE -ne 0) {
        throw "gemma-chain pool-manifest failed with exit code $LASTEXITCODE"
    }
    $manifestObject = $manifestJson | ConvertFrom-Json
    Set-ManifestWorkerBudget `
        -Manifest $manifestObject `
        -Role "quality" `
        -ContextTokens $ContextTokens `
        -DefaultMaxTokens $DefaultMaxTokens
    foreach ($role in $allowedPoolRoles) {
        Set-ManifestWorkerBudget `
            -Manifest $manifestObject `
            -Role $role `
            -ContextTokens (Get-PoolWorkerContextTokens $role) `
            -DefaultMaxTokens (Get-PoolWorkerDefaultMaxTokens $role)
        Set-ManifestWorkerEndpoint `
            -Manifest $manifestObject `
            -Role $role `
            -Port (Get-PoolWorkerPort $role)
        Set-ManifestWorkerRuntime `
            -Manifest $manifestObject `
            -Role $role `
            -GpuLayerCount (Get-PoolWorkerGpuLayers $role) `
            -Device (Get-PoolWorkerDevice $role)
    }
    Update-ManifestCapacityPolicyBudgets `
        -Manifest $manifestObject `
        -HelperRoles $allowedPoolRoles `
        -QualityContextTokens $ContextTokens
    Set-ManifestWorkerRuntime `
        -Manifest $manifestObject `
        -Role "quality" `
        -GpuLayerCount $GpuLayers `
        -Device ""
    $manifestJson = $manifestObject | ConvertTo-Json -Depth 20
    Set-ContentWithRetry -Path $ModelPoolManifest -Value $manifestJson
}

$manifestCapacityPolicy = $null
$manifestAdvice = $null
if (-not [string]::IsNullOrWhiteSpace($ModelPoolManifest)) {
    if (-not (Test-Path -LiteralPath $ModelPoolManifest -PathType Leaf)) {
        throw "ModelPoolManifest not found: $ModelPoolManifest"
    }
    $manifestCheck = Get-Content -LiteralPath $ModelPoolManifest -Raw | ConvertFrom-Json
    if ($manifestCheck.schema_version -and [int]$manifestCheck.schema_version -ne 1) {
        throw "ModelPoolManifest has unsupported schema_version=$($manifestCheck.schema_version)"
    }
    if ($manifestCheck.manifest_kind -and $manifestCheck.manifest_kind -ne "rust-norion.model-pool") {
        throw "ModelPoolManifest has unsupported manifest_kind=$($manifestCheck.manifest_kind)"
    }
    if ($manifestCheck.contract_version -and $manifestCheck.contract_version -ne "gemma-chain.v1") {
        throw "ModelPoolManifest has unsupported contract_version=$($manifestCheck.contract_version)"
    }
    if (@($manifestCheck.workers).Count -lt 1) {
        throw "ModelPoolManifest requires workers"
    }
    $capacityPolicyProperty = $manifestCheck.PSObject.Properties["capacity_policy"]
    if ($null -ne $capacityPolicyProperty) {
        $manifestCapacityPolicy = $capacityPolicyProperty.Value
        if ($manifestCapacityPolicy.policy -and $manifestCapacityPolicy.policy -ne "one_quality_plus_small_helpers") {
            throw "ModelPoolManifest capacity_policy.policy=$($manifestCapacityPolicy.policy), expected one_quality_plus_small_helpers"
        }
        if ($manifestCapacityPolicy.avoid_extra_12b -ne $true) {
            throw "ModelPoolManifest capacity_policy must set avoid_extra_12b=true"
        }
        if ($manifestCapacityPolicy.max_quality_12b_workers -and [int]$manifestCapacityPolicy.max_quality_12b_workers -gt 1) {
            throw "ModelPoolManifest capacity_policy.max_quality_12b_workers=$($manifestCapacityPolicy.max_quality_12b_workers), expected 1 or less"
        }
        if ($manifestCapacityPolicy.helper_model_size_policy -and $manifestCapacityPolicy.helper_model_size_policy -ne "small_or_low_quant_only") {
            throw "ModelPoolManifest capacity_policy.helper_model_size_policy=$($manifestCapacityPolicy.helper_model_size_policy), expected small_or_low_quant_only"
        }
        if ($manifestCapacityPolicy.large_helper_model_guard -and $manifestCapacityPolicy.large_helper_model_guard -notmatch "AllowLargePoolWorkerModels") {
            throw "ModelPoolManifest capacity_policy.large_helper_model_guard must document -AllowLargePoolWorkerModels override"
        }
        if ($manifestCapacityPolicy.guard_validation_command -and $manifestCapacityPolicy.guard_validation_command -notmatch "test-remote-model-pool-guards") {
            throw "ModelPoolManifest capacity_policy.guard_validation_command must point at test-remote-model-pool-guards"
        }
        if ($EnablePoolWorkers -and $manifestCapacityPolicy.helper_roles) {
            $manifestHelperRoles = @($manifestCapacityPolicy.helper_roles)
            foreach ($role in $requestedPoolRoles) {
                if ($role -notin $manifestHelperRoles) {
                    throw "ModelPoolManifest capacity_policy.helper_roles does not include requested role=$role"
                }
            }
        }
    }
    $adviceProperty = $manifestCheck.PSObject.Properties["advice"]
    if ($null -eq $adviceProperty) {
        throw "ModelPoolManifest requires advice generated by model-pool-advice-core"
    }
    $manifestAdvice = $adviceProperty.Value
    if ([string]::IsNullOrWhiteSpace([string]$manifestAdvice.decision_source)) {
        throw "ModelPoolManifest advice.decision_source is required"
    }
    if ($manifestAdvice.decision_source -ne "model-pool-advice-core") {
        throw "ModelPoolManifest advice.decision_source=$($manifestAdvice.decision_source), expected model-pool-advice-core"
    }
    if ($manifestAdvice.policy -and $manifestAdvice.policy -ne "one_quality_12b_plus_small_helpers") {
        throw "ModelPoolManifest advice.policy=$($manifestAdvice.policy), expected one_quality_12b_plus_small_helpers"
    }
    if ($manifestAdvice.avoid_extra_12b -ne $true) {
        throw "ModelPoolManifest advice must set avoid_extra_12b=true"
    }
    if ($manifestAdvice.max_quality_12b_workers -and [int]$manifestAdvice.max_quality_12b_workers -gt 1) {
        throw "ModelPoolManifest advice.max_quality_12b_workers=$($manifestAdvice.max_quality_12b_workers), expected 1 or less"
    }
    if ($manifestAdvice.extra_quality_12b_detected -eq $true) {
        throw "ModelPoolManifest advice detects extra quality 12B workers; keep one quality model and small helpers"
    }
    if ($manifestAdvice.safe_to_enable_pool_workers -ne $true) {
        throw "ModelPoolManifest advice blocks helper expansion: next_step=$($manifestAdvice.next_step) reason=$($manifestAdvice.reason)"
    }
    if ([string]::IsNullOrWhiteSpace([string]$manifestAdvice.next_step)) {
        throw "ModelPoolManifest advice.next_step is required"
    }
    if ([string]::IsNullOrWhiteSpace([string]$manifestAdvice.reason)) {
        throw "ModelPoolManifest advice.reason is required"
    }
    if ([int]$manifestAdvice.quality_worker_count -ne 1) {
        throw "ModelPoolManifest advice.quality_worker_count=$($manifestAdvice.quality_worker_count), expected 1"
    }
    if ($manifestAdvice.helper_target_worker_count -and $EnablePoolWorkers -and [int]$manifestAdvice.helper_target_worker_count -ne $requestedPoolRoles.Count) {
        throw "ModelPoolManifest advice.helper_target_worker_count=$($manifestAdvice.helper_target_worker_count), expected $($requestedPoolRoles.Count)"
    }
    if ($manifestAdvice.worker_shape) {
        if ([int]$manifestAdvice.worker_shape.quality -ne 1) {
            throw "ModelPoolManifest advice.worker_shape.quality=$($manifestAdvice.worker_shape.quality), expected 1"
        }
        if ($EnablePoolWorkers -and [int]$manifestAdvice.worker_shape.helper_target -ne $requestedPoolRoles.Count) {
            throw "ModelPoolManifest advice.worker_shape.helper_target=$($manifestAdvice.worker_shape.helper_target), expected $($requestedPoolRoles.Count)"
        }
        if ($EnablePoolWorkers -and [int]$manifestAdvice.worker_shape.helpers_visible -lt $requestedPoolRoles.Count) {
            throw "ModelPoolManifest advice.worker_shape.helpers_visible=$($manifestAdvice.worker_shape.helpers_visible), expected at least $($requestedPoolRoles.Count)"
        }
    } else {
        throw "ModelPoolManifest advice.worker_shape is required"
    }
    if ($EnablePoolWorkers -and $manifestAdvice.helper_roles) {
        $manifestAdviceHelperRoles = @($manifestAdvice.helper_roles)
        foreach ($role in $requestedPoolRoles) {
            if ($role -notin $manifestAdviceHelperRoles) {
                throw "ModelPoolManifest advice.helper_roles does not include requested role=$role"
            }
        }
    }

    Assert-ManifestWorkerEndpoint -Manifest $manifestCheck -Role "quality" -ExpectedPort $LocalModelPort | Out-Null
    if ($EnablePoolWorkers) {
        foreach ($role in $requestedPoolRoles) {
            Assert-ManifestWorkerEndpoint -Manifest $manifestCheck -Role $role -ExpectedPort ([int]$poolWorkerPorts[$role]) | Out-Null
        }
    }
}

$chainArgs = @(
    "-RepoRoot", $RepoRoot,
    "-RemoteHost", $RemoteHost,
    "-RemoteUser", $RemoteUser,
    "-IdentityFile", $IdentityFile,
    "-RemoteRoot", $RemoteRoot,
    "-RemoteLlamaServer", $RemoteLlamaServer,
    "-RemoteModel", $RemoteModel,
    "-RemoteModelPort", $RemoteModelPort,
    "-LocalModelPort", $LocalModelPort,
    "-PoolWorkerRoles", $PoolWorkerRoles,
    "-SummaryPort", $SummaryPort,
    "-RouterPort", $RouterPort,
    "-ReviewPort", $ReviewPort,
    "-TestGatePort", $TestGatePort,
    "-IndexPort", $IndexPort,
    "-BackendPort", $BackendPort,
    "-LabPort", $LabPort,
    "-ContextTokens", $ContextTokens,
    "-DefaultMaxTokens", $DefaultMaxTokens,
    "-SummaryContextTokens", $SummaryContextTokens,
    "-RouterContextTokens", $RouterContextTokens,
    "-ReviewContextTokens", $ReviewContextTokens,
    "-TestGateContextTokens", $TestGateContextTokens,
    "-IndexContextTokens", $IndexContextTokens,
    "-GpuLayers", $GpuLayers,
    "-SmallGpuLayers", $SmallGpuLayers,
    "-SummaryGpuLayers", $SummaryGpuLayers,
    "-RouterGpuLayers", $RouterGpuLayers,
    "-ReviewGpuLayers", $ReviewGpuLayers,
    "-TestGateGpuLayers", $TestGateGpuLayers,
    "-IndexGpuLayers", $IndexGpuLayers,
    "-SmallParallelSlots", $SmallParallelSlots,
    "-SmallBatchSize", $SmallBatchSize,
    "-SmallUbatchSize", $SmallUbatchSize,
    "-SmallCacheRamMiB", $SmallCacheRamMiB,
    "-Reasoning", $Reasoning,
    "-SmallReasoning", $SmallReasoning,
    "-RuntimeTimeoutMs", $RuntimeTimeoutMs,
    "-LabBackendTimeoutSeconds", $LabBackendTimeoutSeconds,
    "-RunDir", $RunDir
)

if (-not [string]::IsNullOrWhiteSpace($RequiredPoolWorkerRoles)) {
    $chainArgs += @("-RequiredPoolWorkerRoles", $RequiredPoolWorkerRoles)
}
if (-not [string]::IsNullOrWhiteSpace($RemoteSmallLlamaServer)) {
    $chainArgs += @("-RemoteSmallLlamaServer", $RemoteSmallLlamaServer)
}
if (-not [string]::IsNullOrWhiteSpace($RemoteSmallModel)) {
    $chainArgs += @("-RemoteSmallModel", $RemoteSmallModel)
}
foreach ($deviceConfig in @(
    @{ Arg = "-SmallDevice"; Value = $SmallDevice },
    @{ Arg = "-SummaryDevice"; Value = $SummaryDevice },
    @{ Arg = "-RouterDevice"; Value = $RouterDevice },
    @{ Arg = "-ReviewDevice"; Value = $ReviewDevice },
    @{ Arg = "-TestGateDevice"; Value = $TestGateDevice },
    @{ Arg = "-IndexDevice"; Value = $IndexDevice }
)) {
    if (-not [string]::IsNullOrWhiteSpace($deviceConfig.Value)) {
        $chainArgs += @($deviceConfig.Arg, $deviceConfig.Value)
    }
}
foreach ($roleConfig in @(
    @{ ServerArg = "-RemoteSummaryLlamaServer"; ServerValue = $RemoteSummaryLlamaServer; ModelArg = "-RemoteSummaryModel"; ModelValue = $RemoteSummaryModel },
    @{ ServerArg = "-RemoteRouterLlamaServer"; ServerValue = $RemoteRouterLlamaServer; ModelArg = "-RemoteRouterModel"; ModelValue = $RemoteRouterModel },
    @{ ServerArg = "-RemoteReviewLlamaServer"; ServerValue = $RemoteReviewLlamaServer; ModelArg = "-RemoteReviewModel"; ModelValue = $RemoteReviewModel },
    @{ ServerArg = "-RemoteTestGateLlamaServer"; ServerValue = $RemoteTestGateLlamaServer; ModelArg = "-RemoteTestGateModel"; ModelValue = $RemoteTestGateModel },
    @{ ServerArg = "-RemoteIndexLlamaServer"; ServerValue = $RemoteIndexLlamaServer; ModelArg = "-RemoteIndexModel"; ModelValue = $RemoteIndexModel }
)) {
    if (-not [string]::IsNullOrWhiteSpace($roleConfig.ServerValue)) {
        $chainArgs += @($roleConfig.ServerArg, $roleConfig.ServerValue)
    }
    if (-not [string]::IsNullOrWhiteSpace($roleConfig.ModelValue)) {
        $chainArgs += @($roleConfig.ModelArg, $roleConfig.ModelValue)
    }
}
if (-not [string]::IsNullOrWhiteSpace($ModelPoolManifest)) {
    $chainArgs += @("-ModelPoolManifest", $ModelPoolManifest)
}
if (-not [string]::IsNullOrWhiteSpace($ModelCacheStatusJson)) {
    $chainArgs += @("-ModelCacheStatusJson", $ModelCacheStatusJson)
}
if ($SkipBuild) {
    $chainArgs += "-SkipBuild"
}
if ($NoLab) {
    $chainArgs += "-NoLab"
}
if ($EnablePoolWorkers) {
    $chainArgs += "-EnablePoolWorkers"
}
if ($EnableIndexWorker) {
    $chainArgs += "-EnableIndexWorker"
}
if ($EnableSpareWorker) {
    $chainArgs += "-EnableSpareWorker"
}
if ($AllowLargePoolWorkerModels) {
    $chainArgs += "-AllowLargePoolWorkerModels"
}
if ($RestartRemote) {
    $chainArgs += "-RestartRemote"
}
if ($NoTunnel) {
    $chainArgs += "-NoTunnel"
}
if (-not $NoForge) {
    $chainArgs += "-LaunchForge"
}

if ($CheckOnly) {
    Write-Host "SmartSteam remote Gemma Forge preflight: PASS"
    Write-Host "check_only=true"
    Write-Host "touches_remote=false"
    Write-Host "starts_process=false"
    Write-Host "sends_prompt=false"
    Write-Host "recommended_topology=one_12b_quality_plus_small_helpers"
    Write-Host "avoid_multiple_12b_workers=true"
    Write-Host "mac32gb_model_pool_preset=$($UseMac32GBModelPool.IsPresent)"
    Write-Host "check_only_command=.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly"
    Write-Host "start_command=.\tools\smartsteam-forge\start-remote-gemma-forge.cmd"
    $helperStartCommand = if ($UseMac32GBModelPool) {
        ".\tools\smartsteam-forge\start-remote-gemma-forge.cmd -UseMac32GBModelPool"
    } elseif (-not [string]::IsNullOrWhiteSpace($RemoteSmallModel)) {
        ".\tools\smartsteam-forge\start-remote-gemma-forge.cmd -EnablePoolWorkers -RemoteSmallModel $RemoteSmallModel"
    } else {
        ".\tools\smartsteam-forge\start-remote-gemma-forge.cmd -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf"
    }
    Write-Host "helper_start_command=$helperStartCommand"
    Write-Host "repo_root=$RepoRoot"
    Write-Host "remote=$RemoteUser@$RemoteHost"
    Write-Host "model_tunnel=127.0.0.1:$LocalModelPort -> ${RemoteHost}:$RemoteModelPort"
    Write-Host "backend=127.0.0.1:$BackendPort"
    Write-Host "web_lab=127.0.0.1:$LabPort"
    Write-Host "context_tokens=$ContextTokens"
    Write-Host "default_max_tokens=$DefaultMaxTokens"
    Write-Host "runtime_timeout_ms=$RuntimeTimeoutMs"
    Write-Host "lab_backend_timeout_seconds=$(if ($LabBackendTimeoutSeconds -gt 0) { $LabBackendTimeoutSeconds } else { [Math]::Max(900, [int][Math]::Ceiling($RuntimeTimeoutMs / 1000.0) + 5) })"
    Write-Host "launch_forge=$(-not $NoForge)"
    Write-Host "pool_workers_enabled=$($EnablePoolWorkers.IsPresent)"
    Write-Host "pool_worker_large_model_guard=$(-not $AllowLargePoolWorkerModels.IsPresent)"
    if ($EnablePoolWorkers) {
        $poolWorkerTunnelText = @($requestedPoolRoles | ForEach-Object {
            $port = $poolWorkerPorts[$_]
            "127.0.0.1:$port -> ${RemoteHost}:$port"
        }) -join "; "
        Write-Host "pool_worker_roles=$($requestedPoolRoles -join ',')"
        Write-Host "pool_worker_tunnels=$poolWorkerTunnelText"
        Write-Host "remote_small_model=$RemoteSmallModel"
        foreach ($role in $requestedPoolRoles) {
            Write-Host ("pool_worker_{0}_model={1}" -f $role, (Get-PoolWorkerModel $role))
            Write-Host ("pool_worker_{0}_llama_server={1}" -f $role, (Get-PoolWorkerLlamaServer $role))
            Write-Host ("pool_worker_{0}_port={1}" -f $role, (Get-PoolWorkerPort $role))
            Write-Host ("pool_worker_{0}_context_tokens={1}" -f $role, (Get-PoolWorkerContextTokens $role))
            Write-Host ("pool_worker_{0}_default_max_tokens={1}" -f $role, (Get-PoolWorkerDefaultMaxTokens $role))
            Write-Host ("pool_worker_{0}_gpu_layers={1}" -f $role, (Get-PoolWorkerGpuLayers $role))
            Write-Host ("pool_worker_{0}_device={1}" -f $role, (Get-PoolWorkerDevice $role))
        }
    }
    if ([string]::IsNullOrWhiteSpace($ModelPoolManifest)) {
        Write-Host "model_pool_manifest=disabled"
    } else {
        Write-Host "model_pool_manifest=$ModelPoolManifest"
        Write-Host "model_pool_manifest_contract=gemma-chain.v1"
        Write-Host "model_pool_manifest_quality=http://127.0.0.1:$LocalModelPort"
        if ($manifestCapacityPolicy) {
            Write-Host "model_pool_capacity_policy=$($manifestCapacityPolicy.policy)"
            Write-Host "model_pool_avoid_extra_12b=$($manifestCapacityPolicy.avoid_extra_12b)"
            Write-Host "model_pool_max_quality_12b_workers=$($manifestCapacityPolicy.max_quality_12b_workers)"
            Write-Host "model_pool_helper_roles=$(@($manifestCapacityPolicy.helper_roles) -join ',')"
            Write-Host "model_pool_helper_context_tokens_total=$($manifestCapacityPolicy.helper_context_tokens_total)"
            Write-Host "model_pool_helper_default_max_tokens_total=$($manifestCapacityPolicy.helper_default_max_tokens_total)"
            Write-Host "model_pool_helper_model_size_policy=$($manifestCapacityPolicy.helper_model_size_policy)"
            Write-Host "model_pool_guard_validation_command=$($manifestCapacityPolicy.guard_validation_command)"
            Write-Host "model_pool_recommended_launch_order=$(@($manifestCapacityPolicy.recommended_launch_order) -join ',')"
        }
        if ($manifestAdvice) {
            Write-Host "model_pool_advice_source=$($manifestAdvice.decision_source)"
            Write-Host "model_pool_safe_to_enable_pool_workers=$($manifestAdvice.safe_to_enable_pool_workers)"
            Write-Host "model_pool_next_step=$($manifestAdvice.next_step)"
            Write-Host "model_pool_advice_reason=$($manifestAdvice.reason)"
            Write-Host "model_pool_extra_quality_12b_detected=$($manifestAdvice.extra_quality_12b_detected)"
            Write-Host "model_pool_quality_worker_count=$($manifestAdvice.quality_worker_count)"
            Write-Host "model_pool_helper_worker_count=$($manifestAdvice.helper_worker_count)"
            Write-Host "model_pool_helper_target_worker_count=$($manifestAdvice.helper_target_worker_count)"
            if ($manifestAdvice.worker_shape) {
                Write-Host "model_pool_worker_shape=quality:$($manifestAdvice.worker_shape.quality) helpers_visible:$($manifestAdvice.worker_shape.helpers_visible) helper_target:$($manifestAdvice.worker_shape.helper_target)"
            }
            if ($manifestAdvice.operator_checks) {
                Write-Host "model_pool_operator_checks=$($manifestAdvice.operator_checks)"
            }
        }
        if ($EnablePoolWorkers) {
            $manifestWorkerText = @($requestedPoolRoles | ForEach-Object {
                $port = $poolWorkerPorts[$_]
                "$_=http://127.0.0.1:$port"
            }) -join "; "
            Write-Host "model_pool_manifest_workers=$manifestWorkerText"
        }
    }
    if (-not [string]::IsNullOrWhiteSpace($ModelCacheStatusJson)) {
        Write-Host "model_cache_status_json=$ModelCacheStatusJson"
    }
    Write-Host "next_command=.\tools\smartsteam-forge\start-remote-gemma-forge.cmd"
    return
}

& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $chainScript @chainArgs
exit $LASTEXITCODE
