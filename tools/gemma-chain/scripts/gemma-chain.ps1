param(
    [ValidateSet("health", "diagnose", "smoke", "selftest", "pool-plan", "pool-manifest", "pool-status", "pool-route-plan", "prompt-gate", "loop-status", "chain-status", "entrypoint-matrix", "recovery-plan", "status-bundle", "contract-audit", "wrapper-manifest", "contract-fixture", "handoff-report", "secret-scan")]
    [string]$Action = "diagnose",
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ModelBaseUrl = "http://127.0.0.1:8686",
    [string]$BackendBaseUrl = "http://127.0.0.1:7979",
    [string]$LabBaseUrl = "http://127.0.0.1:8789",
    [int]$TimeoutSec = 8,
    [int]$SmokeTimeoutSec = 60,
    [switch]$WaitIfBusy,
    [int]$WaitTimeoutSec = 300,
    [switch]$JsonPlan,
    [switch]$JsonStatus,
    [ValidateSet("auto", "quality", "summary", "router", "tool-call", "preflight", "review", "test-gate", "index", "spare")]
    [string]$TaskKind = "summary",
    [switch]$FailIfBlocked,
    [ValidateSet("any_prompt", "smoke", "web_lab_prompt", "forge_cli_prompt", "backend_cli_direct_prompt", "evolution_loop_prompt_round", "model_pool_launch")]
    [string]$RequireAction = "any_prompt",
    [switch]$WaitReady,
    [int]$PollIntervalSec = 5,
    [int64]$MinContextTokens = 0,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
$script:GemmaChainSchemaVersion = 1
$script:GemmaChainContractVersion = "gemma-chain.v1"

if ($Help) {
    Write-Host "Gemma chain diagnostics for the shared local integration stack."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd health"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd diagnose"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd smoke"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd smoke -WaitIfBusy"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd selftest"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd pool-plan"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd pool-plan -JsonPlan"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd pool-manifest > .\target\gemma-chain\apple-model-pool.generated.json"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd pool-status"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd pool-status -JsonStatus"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind review"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind review -JsonStatus"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind index -JsonStatus"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd prompt-gate"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd prompt-gate -JsonStatus -FailIfBlocked"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd loop-status"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd loop-status -JsonStatus -FailIfBlocked"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd chain-status"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd chain-status -JsonStatus -FailIfBlocked"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd entrypoint-matrix"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd entrypoint-matrix -JsonStatus -FailIfBlocked"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -FailIfBlocked"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction evolution_loop_prompt_round -MinContextTokens 262144 -FailIfBlocked"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -WaitReady -FailIfBlocked"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd recovery-plan"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd recovery-plan -JsonStatus"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd status-bundle"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd status-bundle -JsonStatus -FailIfBlocked"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd contract-audit"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd contract-audit -JsonStatus"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd wrapper-manifest"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd wrapper-manifest -JsonStatus"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd contract-fixture -JsonStatus"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd handoff-report"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd handoff-report -JsonStatus"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd secret-scan"
    Write-Host "  .\tools\gemma-chain\gemma-chain.cmd secret-scan -JsonStatus"
    Write-Host ""
    Write-Host "Defaults:"
    Write-Host "  model   $ModelBaseUrl"
    Write-Host "  backend $BackendBaseUrl"
    Write-Host "  lab     $LabBaseUrl"
    exit 0
}

function Write-Section {
    param([string]$Name)
    Write-Host ""
    Write-Host "== $Name =="
}

function Test-SensitiveName {
    param([string]$Name)
    return (
        $Name -match '(?i)(password|passwd|secret|api[_-]?key|authorization|cookie|private[_-]?key|identityfile|credential)' -or
        $Name -match '(?i)^(token|access[_-]?token|id[_-]?token|refresh[_-]?token|session[_-]?token)$'
    )
}

function Test-SensitiveText {
    param([AllowNull()][string]$Text)
    if ([string]::IsNullOrEmpty($Text)) {
        return $false
    }
    return $Text -match '(?i)(bearer\s+[a-z0-9._~+/-]{12,}|sk-[a-z0-9_-]{12,}|password\s*[:=]|token\s*[:=]|api[_-]?key\s*[:=]|authorization\s*[:=]|-----BEGIN [A-Z ]*PRIVATE KEY-----)'
}

function Format-SafePreview {
    param(
        [AllowNull()][string]$Text,
        [int]$MaxLength = 120
    )
    if ($null -eq $Text) {
        return $null
    }
    if (Test-SensitiveText $Text) {
        return "<redacted-sensitive-preview length=$($Text.Length)>"
    }
    $safe = $Text -replace '[\r\n\t]+', ' '
    if ($safe.Length -gt $MaxLength) {
        return $safe.Substring(0, $MaxLength) + "...<truncated length=$($safe.Length)>"
    }
    return $safe
}

function Format-SafeLogLine {
    param([AllowNull()][string]$Line)
    if ($null -eq $Line) {
        return $null
    }
    if (Test-SensitiveText $Line) {
        return "<redacted-sensitive-log-line length=$($Line.Length)>"
    }
    if ($Line -match '(?i)(prompt=)(.*)$') {
        $prefix = $Line.Substring(0, $Line.IndexOf($Matches[1]) + $Matches[1].Length)
        return $prefix + "<redacted-prompt length=$($Matches[2].Length)>"
    }
    return Format-SafePreview $Line 260
}

function ConvertTo-SafeValue {
    param(
        [Parameter(ValueFromPipeline = $true)] $Value,
        [int]$Depth = 0
    )
    if ($null -eq $Value) {
        return $null
    }
    if ($Depth -gt 10) {
        return "<truncated-depth>"
    }
    if ($Value -is [string]) {
        return Format-SafePreview $Value 240
    }
    if ($Value -is [bool] -or $Value -is [int] -or $Value -is [long] -or $Value -is [double] -or $Value -is [decimal]) {
        return $Value
    }
    if ($Value -is [System.Collections.IDictionary]) {
        $map = [ordered]@{}
        foreach ($key in $Value.Keys) {
            $name = [string]$key
            if (Test-SensitiveName $name) {
                $map[$name] = "<redacted-field>"
            } elseif ($name -match '(?i)(prompt|preview)') {
                $map[$name] = Format-SafePreview ([string]$Value[$key])
            } else {
                $map[$name] = ConvertTo-SafeValue -Value $Value[$key] -Depth ($Depth + 1)
            }
        }
        return [pscustomobject]$map
    }
    if ($Value -is [System.Collections.IEnumerable] -and -not ($Value -is [string])) {
        $items = @()
        $count = 0
        foreach ($item in $Value) {
            if ($count -ge 50) {
                $items += "<truncated-list>"
                break
            }
            $items += ConvertTo-SafeValue -Value $item -Depth ($Depth + 1)
            $count += 1
        }
        return $items
    }

    $props = @($Value.PSObject.Properties | Where-Object {
        $_.MemberType -in @("AliasProperty", "CodeProperty", "NoteProperty", "Property", "ScriptProperty")
    })
    if ($props.Count -gt 0) {
        $object = [ordered]@{}
        foreach ($prop in $props) {
            if (Test-SensitiveName $prop.Name) {
                $object[$prop.Name] = "<redacted-field>"
            } elseif ($prop.Name -match '(?i)(prompt|preview)') {
                $object[$prop.Name] = Format-SafePreview ([string]$prop.Value)
            } else {
                $object[$prop.Name] = ConvertTo-SafeValue -Value $prop.Value -Depth ($Depth + 1)
            }
        }
        return [pscustomobject]$object
    }
    return $Value
}

function ConvertTo-SafeJson {
    param(
        [Parameter(ValueFromPipeline = $true)] $Value,
        [int]$MaxLength = 1200
    )
    $safe = ConvertTo-SafeValue $Value
    $text = $safe | ConvertTo-Json -Depth 12 -Compress
    if ($text.Length -gt $MaxLength) {
        return $text.Substring(0, $MaxLength) + "...<truncated>"
    }
    return $text
}

function ConvertTo-StatusJson {
    param(
        [Parameter(ValueFromPipeline = $true)] $Value,
        [int]$Depth = 16
    )
    return ($Value | ConvertTo-Json -Depth $Depth -Compress)
}

function Get-PropertyValue {
    param(
        $Value,
        [string[]]$Names
    )
    if ($null -eq $Value) {
        return $null
    }
    foreach ($name in $Names) {
        $prop = $Value.PSObject.Properties[$name]
        if ($null -ne $prop -and $null -ne $prop.Value) {
            return $prop.Value
        }
    }
    return $null
}

function Get-RuntimeAcceleratorValue {
    param($Value)
    $explicit = Get-PropertyValue $Value @("runtime_accelerator", "accelerator")
    if ($null -ne $explicit) {
        return $explicit
    }
    $metal = Get-PropertyValue $Value @("metal", "uses_metal", "metal_enabled")
    if ($metal -eq $true -or ([string]$metal).ToLowerInvariant() -eq "true") {
        return "metal"
    }
    $gpu = Get-PropertyValue $Value @("gpu", "uses_gpu", "gpu_enabled")
    if ($gpu -eq $true -or ([string]$gpu).ToLowerInvariant() -eq "true") {
        return "gpu"
    }
    return $null
}

function ConvertTo-Int64OrNull {
    param($Value)
    if ($null -eq $Value) {
        return $null
    }
    $out = 0L
    if ([int64]::TryParse(([string]$Value), [ref]$out)) {
        return $out
    }
    return $null
}

function Invoke-JsonGet {
    param(
        [string]$Url,
        [int]$Timeout = $TimeoutSec
    )
    try {
        $started = Get-Date
        $value = Invoke-RestMethod -Uri $Url -TimeoutSec $Timeout
        $elapsed = [int]((Get-Date) - $started).TotalMilliseconds
        return [pscustomobject]@{
            Ok = $true
            Url = $Url
            Value = $value
            Error = $null
            ElapsedMs = $elapsed
        }
    } catch {
        if ($null -eq $started) {
            $started = Get-Date
        }
        $elapsed = [int]((Get-Date) - $started).TotalMilliseconds
        return [pscustomobject]@{
            Ok = $false
            Url = $Url
            Value = $null
            Error = $_.Exception.Message
            ElapsedMs = $elapsed
        }
    }
}

function Test-TcpPort {
    param(
        [string]$BaseUrl,
        [int]$TimeoutMs = 3000
    )
    $uri = [Uri]$BaseUrl
    try {
        $client = [System.Net.Sockets.TcpClient]::new()
        $async = $client.BeginConnect($uri.Host, $uri.Port, $null, $null)
        $connected = $async.AsyncWaitHandle.WaitOne([TimeSpan]::FromMilliseconds($TimeoutMs))
        if ($connected) {
            $client.EndConnect($async)
        }
        $client.Close()
        return $connected
    } catch {
        return $false
    }
}

function Show-Endpoint {
    param(
        [string]$Name,
        [string]$Url
    )
    $result = Invoke-JsonGet $Url
    if ($result.Ok) {
        Write-Host "$Name OK $Url elapsed_ms=$($result.ElapsedMs)"
        Write-Host ($result.Value | ConvertTo-SafeJson)
    } else {
        Write-Host "$Name ERROR $Url elapsed_ms=$($result.ElapsedMs)"
        Write-Host "  $($result.Error)"
    }
    return $result
}

function Get-EndpointStatus {
    param(
        [string]$Name,
        [string]$BaseUrl
    )
    $tcp = Test-TcpPort $BaseUrl
    $health = Invoke-JsonGet "$BaseUrl/health"
    return [pscustomobject]@{
        name = $Name
        base_url = $BaseUrl
        tcp_reachable = $tcp
        health_ok = $health.Ok
        health_elapsed_ms = $health.ElapsedMs
        health_error = if ($health.Ok) { $null } else { $health.Error }
        health_value = if ($health.Ok) { $health.Value } else { $null }
    }
}

function Show-PortResponsibilities {
    Write-Section "port responsibilities"
    Write-Host "model   $ModelBaseUrl owns llama-server compatible inference and model metadata"
    Write-Host "backend $BackendBaseUrl owns rust-norion routing, readiness, device gates, business-cycle streams, and health projection"
    Write-Host "web-lab $LabBaseUrl owns browser UI, backend health proxy, and chat SSE proxy"
}

function Get-BackendHealth {
    $result = Invoke-JsonGet "$BackendBaseUrl/health"
    if (-not $result.Ok) {
        throw "Backend health unavailable: $($result.Error)"
    }
    return $result.Value
}

function Test-BackendCanPrompt {
    param($Health)
    return (
        $Health.engine_busy -ne $true -and
        $Health.gemma_runtime_reachable -eq $true -and
        $Health.readiness_ok -eq $true -and
        $Health.safe_device_ok -eq $true
    )
}

function Get-PromptBlockReason {
    param($Health)
    $reasons = @()
    if ($Health.engine_busy -eq $true) {
        $reasons += "engine_busy=true"
    }
    if ($Health.gemma_runtime_reachable -ne $true) {
        $reasons += "gemma_runtime_reachable is not true"
    }
    if ($Health.readiness_ok -ne $true) {
        $reasons += "readiness_ok is not true"
    }
    if ($Health.safe_device_ok -ne $true) {
        $reasons += "safe_device_ok is not true"
    }
    if ($reasons.Count -eq 0) {
        return "none"
    }
    return ($reasons -join "; ")
}

function New-PromptGateEntrypointDecision {
    param(
        [string]$Id,
        [string]$Action,
        [bool]$Allowed,
        [string]$Reason,
        [int64]$StandardRequiredContextTokens,
        [bool]$SendsPromptAfterGate,
        [AllowNull()][string]$CompatibilityAlias = $null
    )
    $gateCommand = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction $Id"
    if ($StandardRequiredContextTokens -gt 0) {
        $gateCommand += " -MinContextTokens $StandardRequiredContextTokens"
    }
    $gateCommand += " -JsonStatus -FailIfBlocked"

    return [pscustomobject]@{
        id = $Id
        action = $Action
        allowed = $Allowed
        reason = $Reason
        prompt_gate_read_only = $true
        prompt_gate_command = ".\tools\gemma-chain\gemma-chain.cmd prompt-gate -JsonStatus -FailIfBlocked"
        standard_gate_command = $gateCommand
        blocked_exit_code = 2
        requested_min_context_tokens = $MinContextTokens
        standard_required_context_tokens = $StandardRequiredContextTokens
        sends_prompt_after_gate = $SendsPromptAfterGate
        compatibility_alias = $CompatibilityAlias
    }
}

function New-PromptGateEntrypointDecisions {
    param(
        [bool]$Allowed,
        [string]$Reason
    )
    $decisions = @()
    $decisions += (New-PromptGateEntrypointDecision -Id "smoke" -Action "smoke" -Allowed $Allowed -Reason $Reason -StandardRequiredContextTokens 0 -SendsPromptAfterGate $true)
    $decisions += (New-PromptGateEntrypointDecision -Id "web_lab_prompt" -Action "web-lab prompt" -Allowed $Allowed -Reason $Reason -StandardRequiredContextTokens 0 -SendsPromptAfterGate $true -CompatibilityAlias "web_lab_manual_prompt")
    $decisions += (New-PromptGateEntrypointDecision -Id "forge_cli_prompt" -Action "forge cli prompt" -Allowed $Allowed -Reason $Reason -StandardRequiredContextTokens 0 -SendsPromptAfterGate $true)
    $decisions += (New-PromptGateEntrypointDecision -Id "backend_cli_direct_prompt" -Action "backend cli/direct prompt" -Allowed $Allowed -Reason $Reason -StandardRequiredContextTokens 0 -SendsPromptAfterGate $true)
    $decisions += (New-PromptGateEntrypointDecision -Id "evolution_loop_prompt_round" -Action "evolution-loop prompt round" -Allowed $Allowed -Reason $Reason -StandardRequiredContextTokens 262144 -SendsPromptAfterGate $true)
    $decisions += (New-PromptGateEntrypointDecision -Id "model_pool_launch" -Action "model-pool launch" -Allowed $Allowed -Reason $Reason -StandardRequiredContextTokens 262144 -SendsPromptAfterGate $false)
    return $decisions
}

function Get-PromptGateStatus {
    param($Health)
    $canPrompt = Test-BackendCanPrompt $Health
    $reason = Get-PromptBlockReason $Health
    $contextWindow = ConvertTo-Int64OrNull (Get-PropertyValue $Health @("gemma_runtime_context_window", "runtime_context_window", "context_window", "n_ctx"))
    $contextReady = ($MinContextTokens -le 0 -or ($contextWindow -and $contextWindow -ge $MinContextTokens))
    $contextLabel = if ($contextWindow) { [string]$contextWindow } else { "missing" }
    $contextReason = if ($contextReady) { "none" } else { "context_window $contextLabel is below required $MinContextTokens" }
    $entrypoints = [ordered]@{}
    foreach ($entrypoint in @(
        "smoke",
        "web_lab_manual_prompt",
        "web_lab_prompt",
        "forge_cli_prompt",
        "backend_cli_direct_prompt",
        "evolution_loop_prompt_round",
        "model_pool_launch"
    )) {
        $entryAllowed = ($canPrompt -and $contextReady)
        $entrypoints[$entrypoint] = [pscustomobject]@{
            allowed = $entryAllowed
            reason = if ($entryAllowed) { "none" } elseif (-not $canPrompt) { $reason } else { $contextReason }
        }
    }
    $entryAllowed = ($canPrompt -and $contextReady)
    $entryReason = if ($entryAllowed) { "none" } elseif (-not $canPrompt) { $reason } else { $contextReason }
    return [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        backend = $BackendBaseUrl
        engine_busy = $Health.engine_busy
        gemma_runtime_reachable = $Health.gemma_runtime_reachable
        readiness_ok = $Health.readiness_ok
        safe_device_ok = $Health.safe_device_ok
        prompt_ready = $canPrompt
        block_reason = $reason
        context_window = $contextWindow
        min_context_tokens = $MinContextTokens
        context_ready = $contextReady
        context_block_reason = $contextReason
        quality_worker_prerequisite_passed = ($Health.gemma_runtime_reachable -eq $true -and $Health.readiness_ok -eq $true)
        entrypoints = [pscustomobject]$entrypoints
        entrypoint_decisions = @(New-PromptGateEntrypointDecisions -Allowed $entryAllowed -Reason $entryReason)
        contract = [pscustomobject]@{
            schema_version = $script:GemmaChainSchemaVersion
            contract_version = $script:GemmaChainContractVersion
            read_only = $true
            sends_prompt = $false
            launches_process = $false
            scope = "backend-health-only"
            stable_action_ids = @(Get-PromptActionIds)
            compatibility_aliases = [pscustomobject]@{
                web_lab_manual_prompt = "web_lab_prompt"
            }
        }
    }
}

function Get-AllowedActions {
    param(
        [string]$Classification,
        $PromptGate
    )
    $qualityReady = ($PromptGate -and $PromptGate.prompt_ready -eq $true)
    $contextReady = (-not $PromptGate -or $PromptGate.context_ready -eq $true)
    $promptBlockReason = if (-not $qualityReady) { $Classification } elseif (-not $contextReady) { $PromptGate.context_block_reason } else { $Classification }
    $webLabReady = ($qualityReady -and $contextReady -and $Classification -eq "prompt_ready")
    $backendPromptAllowed = ($qualityReady -and $contextReady -and $Classification -in @("prompt_ready", "web_lab_down"))
    return @(
        [pscustomobject]@{ id = "selftest"; action = "selftest"; allowed = $true; reason = "local fixture check only" },
        [pscustomobject]@{ id = "diagnose"; action = "diagnose"; allowed = $true; reason = "read-only health and metadata probes" },
        [pscustomobject]@{ id = "chain_status"; action = "chain-status"; allowed = $true; reason = "read-only endpoint status and action table" },
        [pscustomobject]@{ id = "entrypoint_matrix"; action = "entrypoint-matrix"; allowed = $true; reason = "read-only prompt and launch entrypoint matrix" },
        [pscustomobject]@{ id = "prompt_gate"; action = "prompt-gate"; allowed = $true; reason = "read-only backend health gate" },
        [pscustomobject]@{ id = "loop_status"; action = "loop-status"; allowed = $true; reason = "read-only ledger and daemon log status" },
        [pscustomobject]@{ id = "pool_plan"; action = "pool-plan"; allowed = $true; reason = "prints plan only" },
        [pscustomobject]@{ id = "pool_manifest"; action = "pool-manifest"; allowed = $true; reason = "prints rust-norion model-pool manifest only" },
        [pscustomobject]@{ id = "pool_status"; action = "pool-status"; allowed = $true; reason = "read-only model-pool port health probes" },
        [pscustomobject]@{ id = "pool_route_plan"; action = "pool-route-plan"; allowed = $true; reason = "read-only model-pool task routing plan" },
        [pscustomobject]@{ id = "recovery_plan"; action = "recovery-plan"; allowed = $true; reason = "read-only recovery owner handoff" },
        [pscustomobject]@{ id = "status_bundle"; action = "status-bundle"; allowed = $true; reason = "read-only owner handoff bundle" },
        [pscustomobject]@{ id = "contract_audit"; action = "contract-audit"; allowed = $true; reason = "read-only wrapper contract verifier" },
        [pscustomobject]@{ id = "wrapper_manifest"; action = "wrapper-manifest"; allowed = $true; reason = "read-only wrapper field consumption manifest" },
        [pscustomobject]@{ id = "contract_fixture"; action = "contract-fixture"; allowed = $true; reason = "offline fixture contract sample; no endpoint probes" },
        [pscustomobject]@{ id = "handoff_report"; action = "handoff-report"; allowed = $true; reason = "read-only main-window handoff summary" },
        [pscustomobject]@{ id = "secret_scan"; action = "secret-scan"; allowed = $true; reason = "read-only secret hygiene scan over Gemma chain docs and tools" },
        [pscustomobject]@{ id = "smoke"; action = "smoke"; allowed = $webLabReady; reason = if ($webLabReady) { "prompt-ready Web Lab path" } else { $promptBlockReason } },
        [pscustomobject]@{ id = "web_lab_prompt"; action = "web-lab prompt"; allowed = $webLabReady; reason = if ($webLabReady) { "prompt-ready Web Lab path" } else { $promptBlockReason } },
        [pscustomobject]@{ id = "forge_cli_prompt"; action = "forge cli prompt"; allowed = $backendPromptAllowed; reason = if ($backendPromptAllowed) { "quality worker ready; backend-only path allowed with operator coordination" } else { $promptBlockReason } },
        [pscustomobject]@{ id = "backend_cli_direct_prompt"; action = "backend cli/direct prompt"; allowed = $backendPromptAllowed; reason = if ($backendPromptAllowed) { "quality worker ready; backend-only path allowed with operator coordination" } else { $promptBlockReason } },
        [pscustomobject]@{ id = "evolution_loop_prompt_round"; action = "evolution-loop prompt round"; allowed = $backendPromptAllowed; reason = if ($backendPromptAllowed) { "quality worker ready and operator coordination required" } else { $promptBlockReason } },
        [pscustomobject]@{ id = "model_pool_launch"; action = "model-pool launch"; allowed = $backendPromptAllowed; reason = if ($backendPromptAllowed) { "quality worker prerequisite passed" } else { $promptBlockReason } }
    )
}

function Test-RequiredActionAllowed {
    param(
        $Status,
        [string]$RequiredAction = "any_prompt"
    )
    $promptActionIds = @("smoke", "web_lab_prompt", "forge_cli_prompt", "backend_cli_direct_prompt", "evolution_loop_prompt_round", "model_pool_launch")
    if ($RequiredAction -eq "any_prompt") {
        return @($Status.allowed_actions | Where-Object { $_.allowed -eq $true -and $_.id -in $promptActionIds }).Count -gt 0
    }
    return @($Status.allowed_actions | Where-Object { $_.id -eq $RequiredAction -and $_.allowed -eq $true }).Count -gt 0
}

function Get-ChainStatusMaybeWaitReady {
    param([string]$RequiredAction = "any_prompt")
    if (-not $WaitReady) {
        return Get-ChainStatus
    }

    $deadline = (Get-Date).AddSeconds($WaitTimeoutSec)
    $lastStatus = $null
    while ($true) {
        $lastStatus = Get-ChainStatus
        if (Test-RequiredActionAllowed -Status $lastStatus -RequiredAction $RequiredAction) {
            return $lastStatus
        }
        if ((Get-Date) -ge $deadline) {
            return $lastStatus
        }
        Start-Sleep -Seconds ([Math]::Max(1, $PollIntervalSec))
    }
}

function Get-ChainStatusFromSnapshots {
    param(
        $Model,
        $Backend,
        $Lab
    )
    $backendHealth = $backend.health_value
    $promptGate = $null
    if ($backend.health_ok -and $null -ne $backendHealth) {
        $promptGate = Get-PromptGateStatus $backendHealth
    }

    $classification = "unknown"
    $nextStep = "run diagnose and coordinate with the chain owner"
    if (-not $backend.tcp_reachable -or -not $backend.health_ok) {
        $classification = "backend_down"
        $nextStep = "do not send prompts; coordinate with backend owner before starting another backend"
    } elseif ($backendHealth.engine_busy -eq $true) {
        $classification = "engine_busy"
        $nextStep = "do not send prompts; wait for active request or evolution round to finish"
    } elseif ($backendHealth.gemma_runtime_reachable -ne $true -or $backendHealth.readiness_ok -ne $true) {
        $classification = "quality_worker_down"
        $nextStep = "do not send prompts; restore quality worker or tunnel ownership first"
    } elseif (-not $lab.tcp_reachable -or -not $lab.health_ok) {
        $classification = "web_lab_down"
        $nextStep = "Web Lab is unavailable; keep UI prompts blocked and repair Web Lab separately"
    } elseif ($promptGate -and $promptGate.prompt_ready -eq $true) {
        $classification = "prompt_ready"
        $nextStep = "operator may run smoke before Web Lab, Forge, or evolution-loop prompts"
    } else {
        $classification = "prompt_blocked"
        $nextStep = "do not send prompts until prompt-gate is ready"
    }

    return [pscustomobject]@{
        classification = $classification
        next_step = $nextStep
        endpoints = [pscustomobject]@{
            quality_worker = $Model
            backend = $Backend
            web_lab = $Lab
        }
        prompt_gate = $promptGate
        allowed_actions = Get-AllowedActions -Classification $classification -PromptGate $promptGate
    }
}

function Get-ChainStatus {
    $model = Get-EndpointStatus "quality-worker" $ModelBaseUrl
    $backend = Get-EndpointStatus "backend" $BackendBaseUrl
    $lab = Get-EndpointStatus "web-lab" $LabBaseUrl
    return Get-ChainStatusFromSnapshots -Model $model -Backend $backend -Lab $lab
}

function Show-AllowedActions {
    param($Actions)
    Write-Section "allowed actions"
    foreach ($item in $Actions) {
        Write-Host "$($item.action) allowed=$($item.allowed) reason=$($item.reason)"
    }
}

function Get-PromptActionIds {
    return @(
        "smoke",
        "web_lab_prompt",
        "forge_cli_prompt",
        "backend_cli_direct_prompt",
        "evolution_loop_prompt_round",
        "model_pool_launch"
    )
}

function New-AllowedActionTablePublic {
    param($Actions)
    $promptActionIds = @(Get-PromptActionIds)
    $promptSendingIds = @(
        "smoke",
        "web_lab_prompt",
        "forge_cli_prompt",
        "backend_cli_direct_prompt",
        "evolution_loop_prompt_round"
    )
    return @($Actions | ForEach-Object {
        $category = "read_only"
        if ($_.id -eq "model_pool_launch") {
            $category = "launch"
        } elseif ($_.id -in $promptActionIds) {
            $category = "prompt"
        }
        [pscustomobject]@{
            id = $_.id
            action = $_.action
            allowed = [bool]$_.allowed
            reason = $_.reason
            category = $category
            status_command_sends_prompt = $false
            downstream_sends_prompt = ($_.id -in $promptSendingIds)
            downstream_launches_process = ($_.id -eq "model_pool_launch")
        }
    })
}

function New-EntrypointGatesPublic {
    param($AllowedActions)
    $gateSpecs = @(
        [pscustomobject]@{
            id = "smoke"
            action = "smoke"
            gate_command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -JsonStatus -FailIfBlocked"
            required_context_tokens = 0
            sends_prompt_after_gate = $true
        },
        [pscustomobject]@{
            id = "web_lab_prompt"
            action = "web-lab prompt"
            gate_command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction web_lab_prompt -JsonStatus -FailIfBlocked"
            required_context_tokens = 0
            sends_prompt_after_gate = $true
        },
        [pscustomobject]@{
            id = "forge_cli_prompt"
            action = "forge cli prompt"
            gate_command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction forge_cli_prompt -JsonStatus -FailIfBlocked"
            required_context_tokens = 0
            sends_prompt_after_gate = $true
        },
        [pscustomobject]@{
            id = "backend_cli_direct_prompt"
            action = "backend cli/direct prompt"
            gate_command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction backend_cli_direct_prompt -JsonStatus -FailIfBlocked"
            required_context_tokens = 0
            sends_prompt_after_gate = $true
        },
        [pscustomobject]@{
            id = "evolution_loop_prompt_round"
            action = "evolution-loop prompt round"
            gate_command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction evolution_loop_prompt_round -MinContextTokens 262144 -JsonStatus -FailIfBlocked"
            required_context_tokens = 262144
            sends_prompt_after_gate = $true
        },
        [pscustomobject]@{
            id = "model_pool_launch"
            action = "model-pool launch"
            gate_command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked"
            required_context_tokens = 262144
            sends_prompt_after_gate = $false
        }
    )
    return @($gateSpecs | ForEach-Object {
        $gate = $_
        $decision = @($AllowedActions | Where-Object { $_.id -eq $gate.id } | Select-Object -First 1)
        [pscustomobject]@{
            id = $gate.id
            action = $gate.action
            allowed = if ($decision.Count -gt 0) { [bool]$decision[0].allowed } else { $false }
            reason = if ($decision.Count -gt 0) { $decision[0].reason } else { "missing allowed action decision" }
            gate_command = $gate.gate_command
            gate_read_only = $true
            blocked_exit_code = 2
            required_context_tokens = $gate.required_context_tokens
            sends_prompt_after_gate = $gate.sends_prompt_after_gate
        }
    })
}

function New-IntegrationEntrypointsPublic {
    param(
        $EntrypointGates,
        [string]$WrapperDecision = "read_only_only",
        [bool]$FailClosedRequired = $true
    )
    $entrypointConsumers = @{
        smoke = [pscustomobject]@{ surface = "web-lab-sse-smoke"; consumer = "gemma-chain smoke"; entrypoint_kind = "smoke" }
        web_lab_prompt = [pscustomobject]@{ surface = "web-lab"; consumer = "browser chat"; entrypoint_kind = "prompt" }
        forge_cli_prompt = [pscustomobject]@{ surface = "forge-cli"; consumer = "tools/smartsteam-forge"; entrypoint_kind = "prompt" }
        backend_cli_direct_prompt = [pscustomobject]@{ surface = "backend-cli-direct"; consumer = "direct backend caller"; entrypoint_kind = "prompt" }
        evolution_loop_prompt_round = [pscustomobject]@{ surface = "evolution-loop"; consumer = "tools/evolution-loop"; entrypoint_kind = "prompt" }
        model_pool_launch = [pscustomobject]@{ surface = "model-pool"; consumer = "Apple Silicon pool launcher"; entrypoint_kind = "launch" }
    }
    return @($EntrypointGates | ForEach-Object {
        $meta = $entrypointConsumers[$_.id]
        [pscustomobject]@{
            id = $_.id
            action = $_.action
            surface = $meta.surface
            consumer = $meta.consumer
            entrypoint_kind = $meta.entrypoint_kind
            current_allowed = [bool]$_.allowed
            blocked_by = if ($_.allowed) { "none" } else { $_.reason }
            gate_command = $_.gate_command
            gate_read_only = $_.gate_read_only
            blocked_exit_code = $_.blocked_exit_code
            required_context_tokens = $_.required_context_tokens
            downstream_sends_prompt = $_.sends_prompt_after_gate
            downstream_launches_process = ($_.id -eq "model_pool_launch")
            wrapper_decision = $WrapperDecision
            fail_closed_required = $FailClosedRequired
            allowed_after_gate_only = ($_.allowed -eq $true -and $WrapperDecision -eq "action_gate_required")
            safe_when_blocked = ($_.allowed -ne $true -and $WrapperDecision -eq "read_only_only")
        }
    })
}

function Convert-EndpointPublicStatus {
    param($Endpoint)
    return [pscustomobject]@{
        name = $Endpoint.name
        base_url = $Endpoint.base_url
        tcp_reachable = $Endpoint.tcp_reachable
        health_ok = $Endpoint.health_ok
        health_elapsed_ms = $Endpoint.health_elapsed_ms
        health_error = $Endpoint.health_error
        service = Get-PropertyValue $Endpoint.health_value @("service")
    }
}

function New-ChainStatusPublic {
    param(
        $Status,
        [string]$RequiredAction = "any_prompt",
        [bool]$WaitReadyValue = [bool]$WaitReady,
        [int]$WaitTimeoutSecValue = $WaitTimeoutSec
    )
    $requiredActionAllowed = Test-RequiredActionAllowed -Status $Status -RequiredAction $RequiredAction
    $promptActionIds = @(Get-PromptActionIds)
    $allowedActionTable = @(New-AllowedActionTablePublic $Status.allowed_actions)
    $allowedPromptActions = @($Status.allowed_actions | Where-Object { $_.id -in $promptActionIds -and $_.allowed -eq $true })
    $blockedPromptActions = @($Status.allowed_actions | Where-Object { $_.id -in $promptActionIds -and $_.allowed -ne $true })
    $allowedReadOnlyActions = @($Status.allowed_actions | Where-Object { $_.id -notin $promptActionIds -and $_.allowed -eq $true })
    $modelPoolLaunch = @($Status.allowed_actions | Where-Object { $_.id -eq "model_pool_launch" } | Select-Object -First 1)
    $entrypointGates = @(New-EntrypointGatesPublic -AllowedActions $Status.allowed_actions)
    $chainWrapperDecision = if (@($allowedPromptActions).Count -gt 0) { "action_gate_required" } else { "read_only_only" }
    $chainFailClosedRequired = (@($allowedPromptActions).Count -eq 0)
    $integrationEntrypoints = @(New-IntegrationEntrypointsPublic -EntrypointGates $entrypointGates -WrapperDecision $chainWrapperDecision -FailClosedRequired $chainFailClosedRequired)
    return [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        classification = $Status.classification
        next_step = $Status.next_step
        require_action = $RequiredAction
        require_action_allowed = $requiredActionAllowed
        wait_ready = $WaitReadyValue
        wait_timeout_sec = $WaitTimeoutSecValue
        endpoints = [pscustomobject]@{
            quality_worker = Convert-EndpointPublicStatus $Status.endpoints.quality_worker
            backend = Convert-EndpointPublicStatus $Status.endpoints.backend
            web_lab = Convert-EndpointPublicStatus $Status.endpoints.web_lab
        }
        prompt_gate = $Status.prompt_gate
        allowed_actions = $Status.allowed_actions
        allowed_action_table = $allowedActionTable
        entrypoint_gates = $entrypointGates
        integration_entrypoints = $integrationEntrypoints
        machine_summary = [pscustomobject]@{
            schema_version = $script:GemmaChainSchemaVersion
            contract_version = $script:GemmaChainContractVersion
            read_only = $true
            sends_prompt = $false
            launches_process = $false
            classification = $Status.classification
            require_action = $RequiredAction
            require_action_allowed = $requiredActionAllowed
            prompt_ready = ($Status.prompt_gate -and $Status.prompt_gate.prompt_ready -eq $true)
            any_prompt_allowed = (@($allowedPromptActions).Count -gt 0)
            blocked_prompt_action_count = @($blockedPromptActions).Count
            allowed_read_only_action_count = @($allowedReadOnlyActions).Count
            model_pool_launch_allowed = if (@($modelPoolLaunch).Count -gt 0) { [bool]$modelPoolLaunch[0].allowed } else { $false }
            model_pool_launch_reason = if (@($modelPoolLaunch).Count -gt 0) { $modelPoolLaunch[0].reason } else { "missing allowed action decision" }
            quality_worker_tcp_reachable = $Status.endpoints.quality_worker.tcp_reachable
            backend_health_ok = $Status.endpoints.backend.health_ok
            web_lab_health_ok = $Status.endpoints.web_lab.health_ok
            engine_busy = if ($Status.prompt_gate) { $Status.prompt_gate.engine_busy } else { $null }
            block_reason = if ($Status.prompt_gate) { $Status.prompt_gate.block_reason } else { "chain prompt gate unavailable" }
            next_read_only_gate = ".\tools\gemma-chain\gemma-chain.cmd chain-status -JsonStatus"
        }
    }
}

function Show-ChainStatus {
    $status = Get-ChainStatusMaybeWaitReady -RequiredAction $RequireAction
    $requiredActionAllowed = Test-RequiredActionAllowed -Status $status -RequiredAction $RequireAction
    if ($JsonStatus) {
        $publicStatus = New-ChainStatusPublic -Status $status -RequiredAction $RequireAction -WaitReadyValue ([bool]$WaitReady) -WaitTimeoutSecValue $WaitTimeoutSec
        Write-Host ($publicStatus | ConvertTo-StatusJson)
        if ($FailIfBlocked -and -not $requiredActionAllowed) {
            exit 2
        }
        return
    }

    Write-Section "chain status"
    Write-Host "schema_version=$($status.prompt_gate.schema_version) contract_version=$($status.prompt_gate.contract_version)"
    Write-Host "classification=$($status.classification)"
    Write-Host "next_step=$($status.next_step)"
    Write-Host "require_action=$RequireAction allowed=$requiredActionAllowed wait_ready=$([bool]$WaitReady)"

    Write-Section "endpoints"
    foreach ($endpoint in @($status.endpoints.quality_worker, $status.endpoints.backend, $status.endpoints.web_lab)) {
        Write-Host "$($endpoint.name) url=$($endpoint.base_url) tcp=$($endpoint.tcp_reachable) health=$($endpoint.health_ok) elapsed_ms=$($endpoint.health_elapsed_ms)"
        if (-not $endpoint.health_ok -and $endpoint.health_error) {
            Write-Host "  error=$($endpoint.health_error)"
        }
    }

    if ($status.prompt_gate) {
        Write-Section "prompt gate"
        Write-Host "prompt_ready=$($status.prompt_gate.prompt_ready)"
        Write-Host "block_reason=$($status.prompt_gate.block_reason)"
        Write-Host "engine_busy=$($status.prompt_gate.engine_busy)"
        Write-Host "gemma_runtime_reachable=$($status.prompt_gate.gemma_runtime_reachable)"
        Write-Host "readiness_ok=$($status.prompt_gate.readiness_ok)"
        Write-Host "safe_device_ok=$($status.prompt_gate.safe_device_ok)"
        Write-Host "context_window=$($status.prompt_gate.context_window)"
        Write-Host "min_context_tokens=$($status.prompt_gate.min_context_tokens)"
        Write-Host "context_ready=$($status.prompt_gate.context_ready)"
        Write-Host "context_block_reason=$($status.prompt_gate.context_block_reason)"
    }
    Show-AllowedActions $status.allowed_actions
    $publicStatus = New-ChainStatusPublic -Status $status -RequiredAction $RequireAction -WaitReadyValue ([bool]$WaitReady) -WaitTimeoutSecValue $WaitTimeoutSec
    Write-Section "integration entrypoints"
    foreach ($entrypoint in $publicStatus.integration_entrypoints) {
        Write-Host "$($entrypoint.id) surface=$($entrypoint.surface) consumer=$($entrypoint.consumer) kind=$($entrypoint.entrypoint_kind) allowed=$($entrypoint.current_allowed) blocked_by=$($entrypoint.blocked_by)"
        Write-Host "  gate=$($entrypoint.gate_command)"
    }
    if ($FailIfBlocked) {
        Write-Section "required action"
        Write-Host "require_action=$RequireAction allowed=$requiredActionAllowed"
    }
    if ($FailIfBlocked -and -not $requiredActionAllowed) {
        exit 2
    }
}

function Get-EntrypointMatrix {
    $status = Get-ChainStatusMaybeWaitReady -RequiredAction $RequireAction
    $publicStatus = New-ChainStatusPublic -Status $status -RequiredAction $RequireAction -WaitReadyValue ([bool]$WaitReady) -WaitTimeoutSecValue $WaitTimeoutSec
    return [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        read_only = $true
        sends_prompt = $false
        launches_process = $false
        classification = $publicStatus.classification
        next_step = $publicStatus.next_step
        require_action = $publicStatus.require_action
        require_action_allowed = $publicStatus.require_action_allowed
        wait_ready = $publicStatus.wait_ready
        wait_timeout_sec = $publicStatus.wait_timeout_sec
        prompt_ready = $publicStatus.machine_summary.prompt_ready
        any_prompt_allowed = $publicStatus.machine_summary.any_prompt_allowed
        model_pool_launch_allowed = $publicStatus.machine_summary.model_pool_launch_allowed
        quality_worker_tcp_reachable = $publicStatus.machine_summary.quality_worker_tcp_reachable
        backend_health_ok = $publicStatus.machine_summary.backend_health_ok
        web_lab_health_ok = $publicStatus.machine_summary.web_lab_health_ok
        entrypoints = $publicStatus.integration_entrypoints
        policy = "read-only matrix only; run the row gate_command before any prompt or model-pool launch"
    }
}

function Show-EntrypointMatrix {
    $matrix = Get-EntrypointMatrix
    if ($JsonStatus) {
        Write-Host ($matrix | ConvertTo-StatusJson -Depth 16)
        if ($FailIfBlocked -and -not $matrix.require_action_allowed) {
            exit 2
        }
        return
    }

    Write-Section "entrypoint matrix"
    Write-Host "schema_version=$($matrix.schema_version) contract_version=$($matrix.contract_version)"
    Write-Host "classification=$($matrix.classification)"
    Write-Host "require_action=$($matrix.require_action) allowed=$($matrix.require_action_allowed) wait_ready=$($matrix.wait_ready)"
    Write-Host "prompt_ready=$($matrix.prompt_ready) any_prompt_allowed=$($matrix.any_prompt_allowed)"
    Write-Host "model_pool_launch_allowed=$($matrix.model_pool_launch_allowed)"
    Write-Host "quality_worker_tcp_reachable=$($matrix.quality_worker_tcp_reachable) backend_health_ok=$($matrix.backend_health_ok) web_lab_health_ok=$($matrix.web_lab_health_ok)"
    foreach ($entrypoint in $matrix.entrypoints) {
        Write-Host "$($entrypoint.id) surface=$($entrypoint.surface) consumer=$($entrypoint.consumer) kind=$($entrypoint.entrypoint_kind) allowed=$($entrypoint.current_allowed) blocked_by=$($entrypoint.blocked_by)"
        Write-Host "  gate=$($entrypoint.gate_command)"
    }
    if ($FailIfBlocked -and -not $matrix.require_action_allowed) {
        exit 2
    }
}

function Get-RecoveryPlan {
    $status = Get-ChainStatusMaybeWaitReady -RequiredAction $RequireAction
    $promptGate = $status.prompt_gate
    $requiredActionAllowed = Test-RequiredActionAllowed -Status $status -RequiredAction $RequireAction
    $handoffFields = [ordered]@{
        classification = $status.classification
        require_action = $RequireAction
        require_action_allowed = $requiredActionAllowed
        wait_ready = [bool]$WaitReady
        wait_timeout_sec = $WaitTimeoutSec
        quality_worker_url = $status.endpoints.quality_worker.base_url
        quality_worker_tcp_reachable = $status.endpoints.quality_worker.tcp_reachable
        quality_worker_health_ok = $status.endpoints.quality_worker.health_ok
        backend_url = $status.endpoints.backend.base_url
        backend_tcp_reachable = $status.endpoints.backend.tcp_reachable
        backend_health_ok = $status.endpoints.backend.health_ok
        web_lab_url = $status.endpoints.web_lab.base_url
        web_lab_tcp_reachable = $status.endpoints.web_lab.tcp_reachable
        web_lab_health_ok = $status.endpoints.web_lab.health_ok
        prompt_ready = if ($promptGate) { $promptGate.prompt_ready } else { $false }
        block_reason = if ($promptGate) { $promptGate.block_reason } else { "backend health unavailable" }
        context_window = if ($promptGate) { $promptGate.context_window } else { $null }
        min_context_tokens = if ($promptGate) { $promptGate.min_context_tokens } else { $MinContextTokens }
        context_ready = if ($promptGate) { $promptGate.context_ready } else { $false }
        context_block_reason = if ($promptGate) { $promptGate.context_block_reason } else { "backend health unavailable" }
        engine_busy = if ($promptGate) { $promptGate.engine_busy } else { $null }
        gemma_runtime_reachable = if ($promptGate) { $promptGate.gemma_runtime_reachable } else { $null }
        readiness_ok = if ($promptGate) { $promptGate.readiness_ok } else { $null }
        safe_device_ok = if ($promptGate) { $promptGate.safe_device_ok } else { $null }
    }

    $blockedPromptActions = @($status.allowed_actions | Where-Object {
        $_.allowed -ne $true -and $_.action -in @(
            "smoke",
            "web-lab prompt",
            "forge cli prompt",
            "backend cli/direct prompt",
            "evolution-loop prompt round",
            "model-pool launch"
        )
    } | ForEach-Object { $_.action })

    $steps = switch ($status.classification) {
        "backend_down" {
            @(
                "Do not start another backend from diagnostics.",
                "Coordinate with the backend owner and confirm intended port 7979.",
                "After owner recovery, rerun chain-status and diagnose before any prompt."
            )
        }
        "engine_busy" {
            @(
                "Do not send new prompts.",
                "Wait for the active request or evolution round to finish.",
                "Rerun prompt-gate or chain-status before smoke."
            )
        }
        "quality_worker_down" {
            @(
                "Do not use smoke, Web Lab, Forge, CLI, evolution-loop, or model-pool launch as probes.",
                "Hand this safe status summary to the model-chain owner.",
                "Owner should restore the 8686 quality worker or tunnel outside diagnostics.",
                "After recovery, rerun chain-status, diagnose, and prompt-gate; run smoke only when prompt_ready=true."
            )
        }
        "web_lab_down" {
            @(
                "Keep Web Lab UI prompts and Web Lab smoke blocked.",
                "Repair Web Lab separately while backend-only paths remain subject to the allowed action table.",
                "Rerun chain-status before UI testing."
            )
        }
        "prompt_ready" {
            @(
                "Run tiny smoke before manual prompts.",
                "Keep max_tokens small unless a long-context test is explicitly scheduled.",
                "Coordinate before resuming evolution-loop prompt rounds."
            )
        }
        default {
            @(
                "Keep prompt-producing actions blocked.",
                "Run diagnose and chain-status again.",
                "Escalate only with the safe handoff fields, not raw logs or secrets."
            )
        }
    }

    return [pscustomobject]@{
        status = [pscustomobject]$handoffFields
        blocked_prompt_actions = $blockedPromptActions
        allowed_actions = $status.allowed_actions
        recovery_steps = $steps
        post_recovery_validation = Get-PostRecoveryValidationCommands
        post_recovery_release_sequence = Get-PostRecoveryReleaseSequence
        safety_notes = @(
            "This command is read-only and does not send prompts.",
            "Do not paste credentials, SSH commands with secrets, tokens, or private keys into reports.",
            "Do not restart or SSH from diagnostics."
        )
    }
}

function Get-PostRecoveryValidationCommands {
    return @(
        ".\tools\gemma-chain\gemma-chain.cmd chain-status",
        ".\tools\gemma-chain\gemma-chain.cmd diagnose",
        ".\tools\gemma-chain\gemma-chain.cmd prompt-gate",
        ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -JsonStatus -FailIfBlocked",
        ".\tools\gemma-chain\gemma-chain.cmd smoke"
    )
}

function Get-PostRecoveryReleaseSequence {
    return @(
        [pscustomobject]@{
            order = 1
            id = "chain_status"
            phase = "read_only_preflight"
            command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -JsonStatus"
            downstream_action = $null
            read_only = $true
            sends_prompt = $false
            launches_process = $false
            requires_previous_success = $false
            require_action = "any_prompt"
            required_context_tokens = 0
            allowed_only_when = "classification is prompt_ready or an intentional backend-only web_lab_down case"
        },
        [pscustomobject]@{
            order = 2
            id = "entrypoint_matrix"
            phase = "read_only_preflight"
            command = ".\tools\gemma-chain\gemma-chain.cmd entrypoint-matrix -JsonStatus"
            downstream_action = $null
            read_only = $true
            sends_prompt = $false
            launches_process = $false
            requires_previous_success = $true
            require_action = "any_prompt"
            required_context_tokens = 0
            allowed_only_when = "all wrapper-facing rows are understood; unknown contract means fail closed"
        },
        [pscustomobject]@{
            order = 3
            id = "diagnose"
            phase = "read_only_preflight"
            command = ".\tools\gemma-chain\gemma-chain.cmd diagnose"
            downstream_action = $null
            read_only = $true
            sends_prompt = $false
            launches_process = $false
            requires_previous_success = $true
            require_action = "any_prompt"
            required_context_tokens = 0
            allowed_only_when = "operator needs endpoint details; metadata timeouts remain warnings unless generation also fails"
        },
        [pscustomobject]@{
            order = 4
            id = "prompt_gate"
            phase = "read_only_preflight"
            command = ".\tools\gemma-chain\gemma-chain.cmd prompt-gate -JsonStatus"
            downstream_action = $null
            read_only = $true
            sends_prompt = $false
            launches_process = $false
            requires_previous_success = $true
            require_action = "any_prompt"
            required_context_tokens = 0
            allowed_only_when = "prompt_ready=true, engine_busy=false, runtime reachable, readiness OK, safe device OK"
        },
        [pscustomobject]@{
            order = 5
            id = "smoke_gate"
            phase = "smoke_gate"
            command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -JsonStatus -FailIfBlocked"
            downstream_action = "smoke"
            read_only = $true
            sends_prompt = $false
            launches_process = $false
            requires_previous_success = $true
            require_action = "smoke"
            required_context_tokens = 0
            allowed_only_when = "gate exits 0 and reports require_action_allowed=true"
        },
        [pscustomobject]@{
            order = 6
            id = "smoke"
            phase = "first_prompt"
            command = ".\tools\gemma-chain\gemma-chain.cmd smoke"
            downstream_action = "tiny Web Lab SSE prompt"
            read_only = $false
            sends_prompt = $true
            launches_process = $false
            requires_previous_success = $true
            require_action = "smoke"
            required_context_tokens = 0
            allowed_only_when = "smoke_gate succeeded immediately before this command"
        },
        [pscustomobject]@{
            order = 7
            id = "web_lab_prompt_gate"
            phase = "manual_prompt_gate"
            command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction web_lab_prompt -JsonStatus -FailIfBlocked"
            downstream_action = "open Web Lab and send one tiny manual prompt"
            read_only = $true
            sends_prompt = $false
            launches_process = $false
            requires_previous_success = $true
            require_action = "web_lab_prompt"
            required_context_tokens = 0
            allowed_only_when = "smoke passed and Web Lab row reports allowed"
        },
        [pscustomobject]@{
            order = 8
            id = "forge_cli_prompt_gate"
            phase = "manual_prompt_gate"
            command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction forge_cli_prompt -JsonStatus -FailIfBlocked"
            downstream_action = "run a tiny Forge CLI prompt"
            read_only = $true
            sends_prompt = $false
            launches_process = $false
            requires_previous_success = $true
            require_action = "forge_cli_prompt"
            required_context_tokens = 0
            allowed_only_when = "smoke passed and Forge CLI row reports allowed"
        },
        [pscustomobject]@{
            order = 9
            id = "evolution_loop_prompt_gate"
            phase = "loop_gate"
            command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction evolution_loop_prompt_round -MinContextTokens 262144 -JsonStatus -FailIfBlocked"
            downstream_action = "resume or allow one evolution-loop prompt round after ownership coordination"
            read_only = $true
            sends_prompt = $false
            launches_process = $false
            requires_previous_success = $true
            require_action = "evolution_loop_prompt_round"
            required_context_tokens = 262144
            allowed_only_when = "quality worker is stable, full context gate passes, and no active shared-chain owner is running"
        },
        [pscustomobject]@{
            order = 10
            id = "model_pool_launch_gate"
            phase = "pool_gate"
            command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked"
            downstream_action = "launch one 12B quality worker plus summary/router/review/index/test-gate helpers only after owner approval"
            read_only = $true
            sends_prompt = $false
            launches_process = $false
            requires_previous_success = $true
            require_action = "model_pool_launch"
            required_context_tokens = 262144
            allowed_only_when = "8686 quality worker reachable/readiness OK and pool launch row reports allowed"
        },
        [pscustomobject]@{
            order = 11
            id = "model_pool_launch"
            phase = "pool_launch"
            command = $null
            downstream_action = "model-chain owner starts small workers outside diagnostics"
            read_only = $false
            sends_prompt = $false
            launches_process = $true
            requires_previous_success = $true
            require_action = "model_pool_launch"
            required_context_tokens = 262144
            allowed_only_when = "model_pool_launch_gate succeeded immediately before launch; diagnostics never launch workers"
        }
    )
}

function Show-RecoveryPlan {
    $plan = Get-RecoveryPlan
    if ($JsonStatus) {
        Write-Host ($plan | ConvertTo-StatusJson)
        if ($FailIfBlocked -and $plan.status.require_action_allowed -ne $true) {
            exit 2
        }
        return
    }

    Write-Section "recovery plan"
    Write-Host "classification=$($plan.status.classification)"
    Write-Host "require_action=$($plan.status.require_action) allowed=$($plan.status.require_action_allowed)"
    Write-Host "prompt_ready=$($plan.status.prompt_ready)"
    Write-Host "block_reason=$($plan.status.block_reason)"
    Write-Host "context_window=$($plan.status.context_window)"
    Write-Host "min_context_tokens=$($plan.status.min_context_tokens)"
    Write-Host "context_ready=$($plan.status.context_ready)"
    Write-Host "context_block_reason=$($plan.status.context_block_reason)"
    Write-Host "quality_worker=$($plan.status.quality_worker_url) tcp=$($plan.status.quality_worker_tcp_reachable) health=$($plan.status.quality_worker_health_ok)"
    Write-Host "backend=$($plan.status.backend_url) tcp=$($plan.status.backend_tcp_reachable) health=$($plan.status.backend_health_ok)"
    Write-Host "web_lab=$($plan.status.web_lab_url) tcp=$($plan.status.web_lab_tcp_reachable) health=$($plan.status.web_lab_health_ok)"

    Write-Section "blocked prompt actions"
    if ($plan.blocked_prompt_actions.Count -eq 0) {
        Write-Host "none"
    } else {
        foreach ($action in $plan.blocked_prompt_actions) {
            Write-Host "$action"
        }
    }

    Write-Section "recovery steps"
    foreach ($step in $plan.recovery_steps) {
        Write-Host "- $step"
    }

    Write-Section "post recovery validation"
    foreach ($command in $plan.post_recovery_validation) {
        Write-Host $command
    }

    Write-Section "post recovery release sequence"
    foreach ($step in $plan.post_recovery_release_sequence) {
        Write-Host "$($step.order). $($step.id) phase=$($step.phase) read_only=$($step.read_only) sends_prompt=$($step.sends_prompt) launches_process=$($step.launches_process)"
        if ($step.command) {
            Write-Host "   command=$($step.command)"
        }
        if ($step.downstream_action) {
            Write-Host "   downstream=$($step.downstream_action)"
        }
        Write-Host "   allowed_only_when=$($step.allowed_only_when)"
    }

    if ($FailIfBlocked -and $plan.status.require_action_allowed -ne $true) {
        exit 2
    }
}

function New-StatusBundlePublic {
    param(
        $Chain,
        $Pool,
        $Recovery,
        $Loop = $null
    )
    $promptReady = ($Chain.prompt_gate -and $Chain.prompt_gate.prompt_ready -eq $true)
    $safeCommands = @(
        ".\tools\gemma-chain\gemma-chain.cmd diagnose",
        ".\tools\gemma-chain\gemma-chain.cmd chain-status -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd entrypoint-matrix -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd loop-status -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd pool-manifest -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd pool-status -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind summary -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd recovery-plan -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd contract-audit -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd wrapper-manifest -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd contract-fixture -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd handoff-report -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd secret-scan -JsonStatus"
    )
    $safeCommandActions = @(
        "diagnose",
        "chain-status",
        "entrypoint-matrix",
        "loop-status",
        "pool-manifest",
        "pool-status",
        "pool-route-plan",
        "recovery-plan",
        "contract-audit",
        "wrapper-manifest",
        "contract-fixture",
        "handoff-report",
        "secret-scan"
    )
    if ($promptReady) {
        $safeCommands += ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -JsonStatus -FailIfBlocked"
        $safeCommands += ".\tools\gemma-chain\gemma-chain.cmd smoke"
        $safeCommandActions += "chain-status"
        $safeCommandActions += "smoke"
    }
    $promptActionIds = @(Get-PromptActionIds)
    $readOnlyActions = @($Chain.allowed_actions | Where-Object {
        $_.allowed -eq $true -and $_.id -notin $promptActionIds
    } | ForEach-Object { $_.action })
    $blockedPromptActions = @($Chain.allowed_actions | Where-Object {
        $_.id -in $promptActionIds -and $_.allowed -ne $true
    } | ForEach-Object { $_.action })
    $allowedPromptActions = @($Chain.allowed_actions | Where-Object {
        $_.id -in $promptActionIds -and $_.allowed -eq $true
    } | ForEach-Object { $_.action })
    $promptActionAllowed = (@($allowedPromptActions).Count -gt 0)
    $safeNextReadOnlySubsetRequired = (-not $promptActionAllowed)
    $safeNextSubsetValid = $true
    if ($safeNextReadOnlySubsetRequired) {
        foreach ($action in $safeCommandActions) {
            if (@($readOnlyActions | Where-Object { $_ -eq $action }).Count -lt 1) {
                $safeNextSubsetValid = $false
                break
            }
        }
    }
    $contractSupported = (
        $Chain.schema_version -eq $script:GemmaChainSchemaVersion -and
        $Chain.contract_version -eq $script:GemmaChainContractVersion
    )
    $failClosedRequired = (
        $contractSupported -ne $true -or
        $safeNextSubsetValid -ne $true -or
        @($allowedPromptActions).Count -eq 0
    )
    $wrapperDecision = "read_only_only"
    if ($contractSupported -ne $true) {
        $wrapperDecision = "unsupported_contract_fail_closed"
    } elseif ($safeNextSubsetValid -ne $true) {
        $wrapperDecision = "unsafe_safe_next_commands_fail_closed"
    } elseif (@($allowedPromptActions).Count -gt 0) {
        $wrapperDecision = "action_gate_required"
    }
    $wrapperAudit = [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        unknown_contract_policy = "fail_closed"
        contract_supported = $contractSupported
        safe_next_read_only_subset_required = $safeNextReadOnlySubsetRequired
        safe_next_actions_subset_of_allowed_read_only = $safeNextSubsetValid
        any_prompt_allowed = $promptActionAllowed
        model_pool_launch_allowed = $Pool.launch_allowed
        fail_closed_required = $failClosedRequired
        wrapper_decision = $wrapperDecision
        required_gate_before_prompt = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction <action> -JsonStatus -FailIfBlocked"
        required_gate_before_model_pool_launch = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked"
    }
    $entrypointGates = @(New-EntrypointGatesPublic -AllowedActions $Chain.allowed_actions)
    $integrationEntrypoints = @(New-IntegrationEntrypointsPublic -EntrypointGates $entrypointGates -WrapperDecision $wrapperAudit.wrapper_decision -FailClosedRequired $wrapperAudit.fail_closed_required)

    return [pscustomobject]@{
        read_only = $true
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        generated_at = (Get-Date).ToString("o")
        require_action = $Chain.require_action
        require_action_allowed = $Chain.require_action_allowed
        classification = $Chain.classification
        prompt_ready = $promptReady
        quality_worker_reachable = $Chain.prompt_gate.gemma_runtime_reachable
        engine_busy = $Chain.prompt_gate.engine_busy
        loop_classification = if ($Loop) { $Loop.classification } else { $null }
        loop_action = if ($Loop) { $Loop.action } else { $null }
        model_pool_launch_allowed = $Pool.launch_allowed
        model_pool_launch_block_reason = $Pool.launch_block_reason
        machine_summary = [pscustomobject]@{
            schema_version = $script:GemmaChainSchemaVersion
            contract_version = $script:GemmaChainContractVersion
            read_only = $true
            sends_prompt = $false
            launches_process = $false
            classification = $Chain.classification
            require_action = $Chain.require_action
            require_action_allowed = $Chain.require_action_allowed
            prompt_ready = $promptReady
            any_prompt_allowed = (@($allowedPromptActions).Count -gt 0)
            blocked_prompt_action_count = @($blockedPromptActions).Count
            allowed_read_only_action_count = @($readOnlyActions).Count
            quality_worker_reachable = $Chain.prompt_gate.gemma_runtime_reachable
            engine_busy = $Chain.prompt_gate.engine_busy
            loop_classification = if ($Loop) { $Loop.classification } else { $null }
            model_pool_launch_allowed = $Pool.launch_allowed
            model_pool_launch_block_reason = $Pool.launch_block_reason
            safe_next_command_actions = $safeCommandActions
            wrapper_decision = $wrapperAudit.wrapper_decision
            fail_closed_required = $wrapperAudit.fail_closed_required
        }
        allowed_action_table = $Chain.allowed_action_table
        integration_entrypoints = $integrationEntrypoints
        prompt_policy = [pscustomobject]@{
            prompt_ready = $promptReady
            any_prompt_allowed = (@($allowedPromptActions).Count -gt 0)
            allowed_prompt_actions = $allowedPromptActions
            blocked_prompt_actions = $blockedPromptActions
            allowed_read_only_actions = $readOnlyActions
            entrypoint_gates = $entrypointGates
            integration_entrypoints = $integrationEntrypoints
            block_reason = if ($Chain.prompt_gate) { $Chain.prompt_gate.block_reason } else { "chain prompt gate unavailable" }
            next_gate = if ($promptReady) {
                ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -JsonStatus -FailIfBlocked"
            } else {
                ".\tools\gemma-chain\gemma-chain.cmd chain-status -JsonStatus"
            }
            wrapper_audit = $wrapperAudit
        }
        wrapper_audit = $wrapperAudit
        chain = $Chain
        loop = $Loop
        pool = $Pool
        recovery = [pscustomobject]@{
            handoff = $Recovery.status
            blocked_prompt_actions = $Recovery.blocked_prompt_actions
            recovery_steps = $Recovery.recovery_steps
            post_recovery_validation = $Recovery.post_recovery_validation
            post_recovery_release_sequence = $Recovery.post_recovery_release_sequence
            safety_notes = $Recovery.safety_notes
        }
        safe_next_commands = $safeCommands
        safe_next_command_actions = $safeCommandActions
        safety_notes = @(
            "status-bundle is read-only and does not send prompts.",
            "Do not use smoke, Web Lab, Forge, CLI, evolution-loop, or model-pool launch until the relevant RequireAction gate passes.",
            "Do not paste credentials, SSH commands with secrets, tokens, or private keys into reports."
        )
    }
}

function Get-StatusBundle {
    $chainStatus = Get-ChainStatusMaybeWaitReady -RequiredAction $RequireAction
    $chainPublic = New-ChainStatusPublic -Status $chainStatus -RequiredAction $RequireAction -WaitReadyValue ([bool]$WaitReady) -WaitTimeoutSecValue $WaitTimeoutSec
    $poolStatus = Get-ModelPoolStatus
    $loopStatus = Get-LoopStatus
    $recoveryPlan = Get-RecoveryPlan
    return New-StatusBundlePublic -Chain $chainPublic -Pool $poolStatus -Recovery $recoveryPlan -Loop $loopStatus
}

function Show-StatusBundle {
    $bundle = Get-StatusBundle
    if ($JsonStatus) {
        Write-Host ($bundle | ConvertTo-StatusJson -Depth 20)
        if ($FailIfBlocked -and $bundle.require_action_allowed -ne $true) {
            exit 2
        }
        return
    }

    Write-Section "status bundle"
    Write-Host "schema_version=$($bundle.schema_version) contract_version=$($bundle.contract_version)"
    Write-Host "classification=$($bundle.classification)"
    Write-Host "require_action=$($bundle.require_action) allowed=$($bundle.require_action_allowed)"
    Write-Host "prompt_ready=$($bundle.prompt_ready)"
    Write-Host "engine_busy=$($bundle.engine_busy)"
    Write-Host "quality_worker_reachable=$($bundle.quality_worker_reachable)"
    Write-Host "loop_classification=$($bundle.loop_classification)"
    Write-Host "loop_action=$($bundle.loop_action)"
    Write-Host "model_pool_launch_allowed=$($bundle.model_pool_launch_allowed) reason=$($bundle.model_pool_launch_block_reason)"

    Write-Section "machine summary"
    Write-Host "read_only=$($bundle.machine_summary.read_only) sends_prompt=$($bundle.machine_summary.sends_prompt) launches_process=$($bundle.machine_summary.launches_process)"
    Write-Host "any_prompt_allowed=$($bundle.machine_summary.any_prompt_allowed)"
    Write-Host "blocked_prompt_action_count=$($bundle.machine_summary.blocked_prompt_action_count)"
    Write-Host "allowed_read_only_action_count=$($bundle.machine_summary.allowed_read_only_action_count)"
    Write-Host "wrapper_decision=$($bundle.machine_summary.wrapper_decision)"
    Write-Host "fail_closed_required=$($bundle.machine_summary.fail_closed_required)"

    Write-Section "wrapper audit"
    Write-Host "contract_supported=$($bundle.wrapper_audit.contract_supported)"
    Write-Host "unknown_contract_policy=$($bundle.wrapper_audit.unknown_contract_policy)"
    Write-Host "safe_next_read_only_subset_required=$($bundle.wrapper_audit.safe_next_read_only_subset_required)"
    Write-Host "safe_next_actions_subset_of_allowed_read_only=$($bundle.wrapper_audit.safe_next_actions_subset_of_allowed_read_only)"
    Write-Host "fail_closed_required=$($bundle.wrapper_audit.fail_closed_required)"
    Write-Host "wrapper_decision=$($bundle.wrapper_audit.wrapper_decision)"
    Write-Host "required_gate_before_model_pool_launch=$($bundle.wrapper_audit.required_gate_before_model_pool_launch)"

    Write-Section "prompt policy"
    Write-Host "any_prompt_allowed=$($bundle.prompt_policy.any_prompt_allowed)"
    Write-Host "block_reason=$($bundle.prompt_policy.block_reason)"
    Write-Host "next_gate=$($bundle.prompt_policy.next_gate)"
    Write-Host "allowed_read_only_actions=$($bundle.prompt_policy.allowed_read_only_actions -join ',')"
    Write-Host "blocked_prompt_actions=$($bundle.prompt_policy.blocked_prompt_actions -join ',')"

    Write-Section "entrypoint gates"
    foreach ($gate in $bundle.prompt_policy.entrypoint_gates) {
        Write-Host "$($gate.id) allowed=$($gate.allowed) reason=$($gate.reason) read_only_gate=$($gate.gate_read_only) exit_if_blocked=$($gate.blocked_exit_code)"
        Write-Host "  gate=$($gate.gate_command)"
    }

    Write-Section "integration entrypoints"
    foreach ($entrypoint in $bundle.integration_entrypoints) {
        Write-Host "$($entrypoint.id) surface=$($entrypoint.surface) consumer=$($entrypoint.consumer) kind=$($entrypoint.entrypoint_kind) allowed=$($entrypoint.current_allowed) blocked_by=$($entrypoint.blocked_by)"
        Write-Host "  gate=$($entrypoint.gate_command)"
    }

    Write-Section "allowed action table"
    foreach ($action in $bundle.allowed_action_table) {
        Write-Host "$($action.id) category=$($action.category) allowed=$($action.allowed) reason=$($action.reason)"
    }

    Write-Section "safe next commands"
    foreach ($command in $bundle.safe_next_commands) {
        Write-Host $command
    }

    Write-Section "recovery steps"
    foreach ($step in $bundle.recovery.recovery_steps) {
        Write-Host "- $step"
    }

    if ($FailIfBlocked -and $bundle.require_action_allowed -ne $true) {
        exit 2
    }
}

function New-ContractAuditCheck {
    param(
        [string]$Id,
        [bool]$Passed,
        [string]$Severity,
        [string]$Evidence
    )
    return [pscustomobject]@{
        id = $Id
        passed = $Passed
        severity = $Severity
        evidence = $Evidence
    }
}

function New-ContractAuditPublic {
    param($Bundle)

    $entrypoints = @($Bundle.integration_entrypoints)
    $entrypointGates = @($Bundle.prompt_policy.entrypoint_gates)
    $allowedTable = @($Bundle.allowed_action_table)
    $releaseSequence = @($Bundle.recovery.post_recovery_release_sequence)
    $safeNextActions = @($Bundle.safe_next_command_actions)
    $allowedReadOnly = @($Bundle.prompt_policy.allowed_read_only_actions)

    $modelPoolAction = @($allowedTable | Where-Object { $_.id -eq "model_pool_launch" } | Select-Object -First 1)
    $modelPoolEntrypoint = @($entrypoints | Where-Object { $_.id -eq "model_pool_launch" } | Select-Object -First 1)
    $modelPoolGate = @($entrypointGates | Where-Object { $_.id -eq "model_pool_launch" } | Select-Object -First 1)
    $smokeReleaseGate = @($releaseSequence | Where-Object { $_.id -eq "smoke_gate" } | Select-Object -First 1)
    $smokeRelease = @($releaseSequence | Where-Object { $_.id -eq "smoke" } | Select-Object -First 1)
    $poolReleaseGate = @($releaseSequence | Where-Object { $_.id -eq "model_pool_launch_gate" } | Select-Object -First 1)
    $poolRelease = @($releaseSequence | Where-Object { $_.id -eq "model_pool_launch" } | Select-Object -First 1)
    $readOnlyReleaseViolations = @($releaseSequence | Where-Object {
        $_.read_only -eq $true -and ($_.sends_prompt -eq $true -or $_.launches_process -eq $true)
    })

    $safeNextSubsetValid = $true
    foreach ($action in $safeNextActions) {
        if (@($allowedReadOnly | Where-Object { $_ -eq $action }).Count -lt 1) {
            $safeNextSubsetValid = $false
            break
        }
    }

    $qualityDownImpliesPoolBlocked = $true
    if ($Bundle.quality_worker_reachable -eq $false) {
        $qualityDownImpliesPoolBlocked = ($Bundle.model_pool_launch_allowed -eq $false)
    }

    $checks = @()
    $checks += New-ContractAuditCheck "contract_version_supported" (
        $Bundle.schema_version -eq $script:GemmaChainSchemaVersion -and
        $Bundle.contract_version -eq $script:GemmaChainContractVersion -and
        $Bundle.wrapper_audit.contract_supported -eq $true
    ) "fatal" "top-level and wrapper audit contract versions match expected values"
    $checks += New-ContractAuditCheck "status_bundle_read_only" (
        $Bundle.read_only -eq $true -and
        $Bundle.machine_summary.sends_prompt -eq $false -and
        $Bundle.machine_summary.launches_process -eq $false
    ) "fatal" "status-bundle and machine_summary are read-only status objects"
    $checks += New-ContractAuditCheck "safe_next_subset_read_only" (
        $Bundle.wrapper_audit.safe_next_actions_subset_of_allowed_read_only -eq $true -and
        $safeNextSubsetValid -eq $true
    ) "fatal" "safe_next_command_actions are a subset of allowed read-only actions"
    $checks += New-ContractAuditCheck "entrypoint_matrix_complete" (
        $entrypoints.Count -eq 6 -and
        @($entrypoints | Where-Object { $_.gate_read_only -eq $true -and $_.blocked_exit_code -eq 2 }).Count -eq 6
    ) "fatal" "six integration entrypoints expose read-only gates with blocked exit code 2"
    $checks += New-ContractAuditCheck "model_pool_launch_gate_shape" (
        $null -ne $modelPoolAction -and
        $null -ne $modelPoolEntrypoint -and
        $null -ne $modelPoolGate -and
        $modelPoolAction.category -eq "launch" -and
        $modelPoolAction.downstream_launches_process -eq $true -and
        $modelPoolAction.downstream_sends_prompt -eq $false -and
        $modelPoolGate.required_context_tokens -eq 262144 -and
        $modelPoolGate.sends_prompt_after_gate -eq $false -and
        $modelPoolEntrypoint.downstream_launches_process -eq $true -and
        $modelPoolEntrypoint.downstream_sends_prompt -eq $false
    ) "fatal" "model_pool_launch is a process launch gate, not a prompt gate, and requires 262144 context tokens"
    $checks += New-ContractAuditCheck "quality_down_blocks_model_pool" (
        $qualityDownImpliesPoolBlocked
    ) "fatal" "quality_worker_reachable=false implies model_pool_launch_allowed=false"
    $checks += New-ContractAuditCheck "release_sequence_safe_order" (
        $releaseSequence.Count -ge 10 -and
        $null -ne $smokeReleaseGate -and
        $null -ne $smokeRelease -and
        $smokeReleaseGate.order -lt $smokeRelease.order -and
        $readOnlyReleaseViolations.Count -eq 0
    ) "fatal" "post-recovery release sequence gates smoke before prompt and read-only rows do not prompt or launch"
    $checks += New-ContractAuditCheck "release_sequence_pool_gate" (
        $null -ne $poolReleaseGate -and
        $null -ne $poolRelease -and
        $poolReleaseGate.required_context_tokens -eq 262144 -and
        $poolReleaseGate.read_only -eq $true -and
        $poolReleaseGate.sends_prompt -eq $false -and
        $poolReleaseGate.launches_process -eq $false -and
        $poolRelease.required_context_tokens -eq 262144 -and
        $poolRelease.sends_prompt -eq $false -and
        $poolRelease.launches_process -eq $true -and
        $null -eq $poolRelease.command
    ) "fatal" "model-pool launch is released only after a read-only 262144-token gate; diagnostics expose no launch command"

    $failedChecks = @($checks | Where-Object { $_.passed -ne $true })
    return [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        read_only = $true
        sends_prompt = $false
        launches_process = $false
        audit_passed = ($failedChecks.Count -eq 0)
        failed_check_ids = @($failedChecks | ForEach-Object { $_.id })
        classification = $Bundle.classification
        require_action = $Bundle.require_action
        require_action_allowed = $Bundle.require_action_allowed
        prompt_ready = $Bundle.prompt_ready
        engine_busy = $Bundle.engine_busy
        quality_worker_reachable = $Bundle.quality_worker_reachable
        model_pool_launch_allowed = $Bundle.model_pool_launch_allowed
        wrapper_decision = $Bundle.wrapper_audit.wrapper_decision
        fail_closed_required = $Bundle.wrapper_audit.fail_closed_required
        safe_next_actions_subset_of_allowed_read_only = $Bundle.wrapper_audit.safe_next_actions_subset_of_allowed_read_only
        checks = $checks
        next_gate = if ($Bundle.model_pool_launch_allowed -eq $true) {
            ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked"
        } else {
            ".\tools\gemma-chain\gemma-chain.cmd chain-status -JsonStatus"
        }
    }
}

function Get-ContractAudit {
    $bundle = Get-StatusBundle
    return New-ContractAuditPublic -Bundle $bundle
}

function Show-ContractAudit {
    $audit = Get-ContractAudit
    if ($JsonStatus) {
        Write-Host ($audit | ConvertTo-StatusJson -Depth 18)
        if ($FailIfBlocked -and ($audit.audit_passed -ne $true -or $audit.require_action_allowed -ne $true)) {
            exit 2
        }
        return
    }

    Write-Section "contract audit"
    Write-Host "schema_version=$($audit.schema_version) contract_version=$($audit.contract_version)"
    Write-Host "read_only=$($audit.read_only) sends_prompt=$($audit.sends_prompt) launches_process=$($audit.launches_process)"
    Write-Host "audit_passed=$($audit.audit_passed)"
    Write-Host "classification=$($audit.classification)"
    Write-Host "require_action=$($audit.require_action) allowed=$($audit.require_action_allowed)"
    Write-Host "prompt_ready=$($audit.prompt_ready) engine_busy=$($audit.engine_busy) quality_worker_reachable=$($audit.quality_worker_reachable)"
    Write-Host "model_pool_launch_allowed=$($audit.model_pool_launch_allowed)"
    Write-Host "wrapper_decision=$($audit.wrapper_decision) fail_closed_required=$($audit.fail_closed_required)"
    Write-Host "safe_next_actions_subset_of_allowed_read_only=$($audit.safe_next_actions_subset_of_allowed_read_only)"
    Write-Host "next_gate=$($audit.next_gate)"

    Write-Section "contract checks"
    foreach ($check in $audit.checks) {
        Write-Host "$($check.id) passed=$($check.passed) severity=$($check.severity)"
        Write-Host "  evidence=$($check.evidence)"
    }

    if ($audit.failed_check_ids.Count -gt 0) {
        Write-Section "failed checks"
        foreach ($id in $audit.failed_check_ids) {
            Write-Host $id
        }
    }

    if ($FailIfBlocked -and ($audit.audit_passed -ne $true -or $audit.require_action_allowed -ne $true)) {
        exit 2
    }
}

function New-WrapperManifestPublic {
    param(
        $Bundle,
        $Audit
    )

    $fieldContract = @(
        "schema_version",
        "contract_version",
        "read_only",
        "sends_prompt",
        "launches_process",
        "classification",
        "require_action",
        "require_action_allowed",
        "prompt_ready",
        "quality_worker_reachable",
        "engine_busy",
        "model_pool_launch_allowed",
        "wrapper_decision",
        "fail_closed_required"
    )

    $entries = @($Bundle.integration_entrypoints | ForEach-Object {
        $statusCommand = ".\tools\gemma-chain\gemma-chain.cmd entrypoint-matrix -RequireAction $($_.id)"
        if ($_.required_context_tokens -gt 0) {
            $statusCommand += " -MinContextTokens $($_.required_context_tokens)"
        }
        $statusCommand += " -JsonStatus -FailIfBlocked"

        [pscustomobject]@{
            id = $_.id
            surface = $_.surface
            consumer = $_.consumer
            entrypoint_kind = $_.entrypoint_kind
            current_allowed = $_.current_allowed
            blocked_by = $_.blocked_by
            status_command = $statusCommand
            gate_command = $_.gate_command
            gate_read_only = $_.gate_read_only
            blocked_exit_code = $_.blocked_exit_code
            required_context_tokens = $_.required_context_tokens
            downstream_sends_prompt = $_.downstream_sends_prompt
            downstream_launches_process = $_.downstream_launches_process
            consume_fields = @(
                "schema_version",
                "contract_version",
                "entrypoints[].id",
                "entrypoints[].current_allowed",
                "entrypoints[].blocked_by",
                "entrypoints[].gate_command",
                "entrypoints[].blocked_exit_code",
                "entrypoints[].required_context_tokens",
                "entrypoints[].downstream_sends_prompt",
                "entrypoints[].downstream_launches_process"
            )
            proceed_only_if = @(
                "schema_version == 1",
                "contract_version == gemma-chain.v1",
                "matching entrypoints[].id exists",
                "current_allowed == true",
                "the matching gate_command exits 0 immediately before the downstream action"
            )
            fail_closed_if = @(
                "schema or contract version is missing or unknown",
                "matching entrypoint row is missing",
                "current_allowed is not true",
                "gate_command exits nonzero, including exit 2",
                "blocked_by is not none"
            )
        }
    })

    return [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        read_only = $true
        sends_prompt = $false
        launches_process = $false
        generated_at = (Get-Date).ToString("o")
        classification = $Bundle.classification
        require_action = $Bundle.require_action
        require_action_allowed = $Bundle.require_action_allowed
        prompt_ready = $Bundle.prompt_ready
        quality_worker_reachable = $Bundle.quality_worker_reachable
        engine_busy = $Bundle.engine_busy
        model_pool_launch_allowed = $Bundle.model_pool_launch_allowed
        wrapper_decision = $Bundle.wrapper_audit.wrapper_decision
        fail_closed_required = $Bundle.wrapper_audit.fail_closed_required
        audit_passed = $Audit.audit_passed
        audit_failed_check_ids = $Audit.failed_check_ids
        unknown_contract_policy = "fail_closed"
        status_commands = [pscustomobject]@{
            compact_matrix = ".\tools\gemma-chain\gemma-chain.cmd entrypoint-matrix -JsonStatus"
            full_bundle = ".\tools\gemma-chain\gemma-chain.cmd status-bundle -JsonStatus"
            contract_audit = ".\tools\gemma-chain\gemma-chain.cmd contract-audit -JsonStatus"
            pool_launch_gate = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked"
        }
        required_top_level_fields = $fieldContract
        consumption_rules = @(
            "Use wrapper-manifest or entrypoint-matrix as a read-only status source only.",
            "Run the listed gate_command immediately before any downstream prompt or process launch.",
            "Treat missing fields, unknown schema_version, or unknown contract_version as blocked.",
            "Do not substitute Web Lab, Forge, backend CLI, evolution-loop, or model-pool launch for diagnostics.",
            "When quality_worker_reachable=false, all prompt and model-pool launch entrypoints must stay blocked."
        )
        entrypoints = $entries
        safe_next_command_actions = $Bundle.safe_next_command_actions
    }
}

function Get-WrapperManifest {
    $bundle = Get-StatusBundle
    $audit = New-ContractAuditPublic -Bundle $bundle
    return New-WrapperManifestPublic -Bundle $bundle -Audit $audit
}

function Show-WrapperManifest {
    $manifest = Get-WrapperManifest
    if ($JsonStatus) {
        Write-Host ($manifest | ConvertTo-StatusJson -Depth 18)
        if ($FailIfBlocked -and ($manifest.audit_passed -ne $true -or $manifest.require_action_allowed -ne $true)) {
            exit 2
        }
        return
    }

    Write-Section "wrapper manifest"
    Write-Host "schema_version=$($manifest.schema_version) contract_version=$($manifest.contract_version)"
    Write-Host "read_only=$($manifest.read_only) sends_prompt=$($manifest.sends_prompt) launches_process=$($manifest.launches_process)"
    Write-Host "classification=$($manifest.classification)"
    Write-Host "require_action=$($manifest.require_action) allowed=$($manifest.require_action_allowed)"
    Write-Host "prompt_ready=$($manifest.prompt_ready) engine_busy=$($manifest.engine_busy) quality_worker_reachable=$($manifest.quality_worker_reachable)"
    Write-Host "model_pool_launch_allowed=$($manifest.model_pool_launch_allowed)"
    Write-Host "wrapper_decision=$($manifest.wrapper_decision) fail_closed_required=$($manifest.fail_closed_required)"
    Write-Host "audit_passed=$($manifest.audit_passed)"

    Write-Section "status commands"
    Write-Host "compact_matrix=$($manifest.status_commands.compact_matrix)"
    Write-Host "full_bundle=$($manifest.status_commands.full_bundle)"
    Write-Host "contract_audit=$($manifest.status_commands.contract_audit)"
    Write-Host "pool_launch_gate=$($manifest.status_commands.pool_launch_gate)"

    Write-Section "entrypoint consumption"
    foreach ($entrypoint in $manifest.entrypoints) {
        Write-Host "$($entrypoint.id) surface=$($entrypoint.surface) consumer=$($entrypoint.consumer) kind=$($entrypoint.entrypoint_kind) allowed=$($entrypoint.current_allowed) blocked_by=$($entrypoint.blocked_by)"
        Write-Host "  status=$($entrypoint.status_command)"
        Write-Host "  gate=$($entrypoint.gate_command)"
        Write-Host "  downstream_sends_prompt=$($entrypoint.downstream_sends_prompt) downstream_launches_process=$($entrypoint.downstream_launches_process)"
    }

    if ($FailIfBlocked -and ($manifest.audit_passed -ne $true -or $manifest.require_action_allowed -ne $true)) {
        exit 2
    }
}

function New-ContractFixtureEndpoint {
    param(
        [string]$Name,
        [string]$BaseUrl,
        [bool]$TcpReachable,
        [bool]$HealthOk,
        $HealthValue = $null
    )
    return [pscustomobject]@{
        name = $Name
        base_url = $BaseUrl
        tcp_reachable = $TcpReachable
        health_ok = $HealthOk
        health_elapsed_ms = 0
        health_error = if ($HealthOk) { $null } else { "offline fixture unavailable" }
        health_value = $HealthValue
    }
}

function Get-ContractFixture {
    $quality = New-ContractFixtureEndpoint "quality-worker" "http://127.0.0.1:8686" $false $false
    $backend = New-ContractFixtureEndpoint "backend" "http://127.0.0.1:7979" $true $true ([pscustomobject]@{
        engine_busy = $false
        gemma_runtime_reachable = $false
        readiness_ok = $false
        safe_device_ok = $true
        gemma_runtime_context_window = $null
        runtime_mode = "gemma-http"
    })
    $lab = New-ContractFixtureEndpoint "web-lab" "http://127.0.0.1:8789" $true $true ([pscustomobject]@{
        ok = $true
        service = "rustgpt-lab"
    })

    $chainStatus = Get-ChainStatusFromSnapshots -Model $quality -Backend $backend -Lab $lab
    $chainPublic = New-ChainStatusPublic -Status $chainStatus -RequiredAction "model_pool_launch" -WaitReadyValue $false -WaitTimeoutSecValue 0
    $entrypointMatrix = [pscustomobject]@{
        schema_version = $chainPublic.schema_version
        contract_version = $chainPublic.contract_version
        read_only = $true
        sends_prompt = $false
        launches_process = $false
        classification = $chainPublic.classification
        require_action = $chainPublic.require_action
        require_action_allowed = $chainPublic.require_action_allowed
        wait_ready = $chainPublic.wait_ready
        wait_timeout_sec = $chainPublic.wait_timeout_sec
        prompt_ready = $chainPublic.prompt_gate.prompt_ready
        any_prompt_allowed = $chainPublic.machine_summary.any_prompt_allowed
        model_pool_launch_allowed = $chainPublic.machine_summary.model_pool_launch_allowed
        quality_worker_tcp_reachable = $chainPublic.machine_summary.quality_worker_tcp_reachable
        backend_health_ok = $chainPublic.machine_summary.backend_health_ok
        web_lab_health_ok = $chainPublic.machine_summary.web_lab_health_ok
        entrypoints = $chainPublic.integration_entrypoints
    }

    $pool = [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        summary = "offline fixture pool status"
        blocked_policy = "model-pool launch is blocked while the quality worker is down"
        launch_gate = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked"
        launch_allowed = $false
        launch_block_reason = "quality_worker_down"
        min_context_tokens = 262144
        context_gate_source = "quality default"
        chain_classification = "quality_worker_down"
        chain_next_step = "restore quality worker first"
        prompt_gate = $chainPublic.prompt_gate
        workers = @(
            [pscustomobject]@{
                port = 8686
                role = "quality"
                base_url = "http://127.0.0.1:8686"
                enabled_by_default = $true
                launches_process = $false
                requires_quality_gate = $true
                tcp_reachable = $false
                health_ok = $false
                status = "unreachable"
                role_ready = $false
                role_block_reason = "tcp_unreachable"
            }
        )
    }

    $recovery = [pscustomobject]@{
        status = [pscustomobject]@{
            classification = "quality_worker_down"
            require_action = "model_pool_launch"
            require_action_allowed = $false
            wait_ready = $false
            wait_timeout_sec = 0
            quality_worker_url = "http://127.0.0.1:8686"
            quality_worker_tcp_reachable = $false
            quality_worker_health_ok = $false
            backend_url = "http://127.0.0.1:7979"
            backend_tcp_reachable = $true
            backend_health_ok = $true
            web_lab_url = "http://127.0.0.1:8789"
            web_lab_tcp_reachable = $true
            web_lab_health_ok = $true
            prompt_ready = $false
            block_reason = "gemma_runtime_reachable is not true; readiness_ok is not true"
            context_window = $null
            min_context_tokens = 262144
            context_ready = $false
            context_block_reason = "quality worker down"
            engine_busy = $false
            gemma_runtime_reachable = $false
            readiness_ok = $false
            safe_device_ok = $true
        }
        blocked_prompt_actions = @("smoke", "web-lab prompt", "forge cli prompt", "backend cli/direct prompt", "evolution-loop prompt round", "model-pool launch")
        recovery_steps = @("restore quality worker first", "rerun chain-status and prompt-gate", "run smoke only after the smoke gate passes")
        post_recovery_validation = Get-PostRecoveryValidationCommands
        post_recovery_release_sequence = Get-PostRecoveryReleaseSequence
        safety_notes = @("offline fixture only; do not infer runtime health from this sample")
    }

    $loop = [pscustomobject]@{
        prompt_gate = $chainPublic.prompt_gate
        evolution_dir = "fixture://target/evolution"
        ledger = $null
        daemon_out = $null
        daemon_err = $null
        classification = "quality_worker_gate_blocked"
        action = "restore quality worker/tunnel ownership first, then rerun diagnose and prompt-gate."
    }

    $bundle = New-StatusBundlePublic -Chain $chainPublic -Pool $pool -Recovery $recovery -Loop $loop
    $audit = New-ContractAuditPublic -Bundle $bundle
    $manifest = New-WrapperManifestPublic -Bundle $bundle -Audit $audit

    return [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        read_only = $true
        sends_prompt = $false
        launches_process = $false
        offline_fixture = $true
        touches_network = $false
        fixture_name = "quality_worker_down"
        fixture_policy = "Shape sample only; use live gates before any prompt or process launch."
        chain_status = $chainPublic
        entrypoint_matrix = $entrypointMatrix
        status_bundle = $bundle
        contract_audit = $audit
        wrapper_manifest = $manifest
    }
}

function Show-ContractFixture {
    $fixture = Get-ContractFixture
    if ($JsonStatus) {
        Write-Host ($fixture | ConvertTo-StatusJson -Depth 22)
        return
    }

    Write-Section "contract fixture"
    Write-Host "schema_version=$($fixture.schema_version) contract_version=$($fixture.contract_version)"
    Write-Host "read_only=$($fixture.read_only) sends_prompt=$($fixture.sends_prompt) launches_process=$($fixture.launches_process)"
    Write-Host "offline_fixture=$($fixture.offline_fixture) touches_network=$($fixture.touches_network)"
    Write-Host "fixture_name=$($fixture.fixture_name)"
    Write-Host "classification=$($fixture.chain_status.classification)"
    Write-Host "prompt_ready=$($fixture.chain_status.prompt_gate.prompt_ready)"
    Write-Host "model_pool_launch_allowed=$($fixture.status_bundle.model_pool_launch_allowed)"
    Write-Host "wrapper_manifest_entrypoints=$(@($fixture.wrapper_manifest.entrypoints).Count)"
    Write-Host "policy=$($fixture.fixture_policy)"
}

function New-HandoffReportPublic {
    param(
        $Bundle,
        $Audit = $null
    )

    if ($null -eq $Audit) {
        $Audit = New-ContractAuditPublic -Bundle $Bundle
    }

    $qualityReady = ($Bundle.quality_worker_reachable -eq $true -and $Bundle.prompt_ready -eq $true)
    $currentState = switch ($Bundle.classification) {
        "quality_worker_down" { "backend/web-lab may be online, but the 8686 quality worker is not ready" }
        "engine_busy" { "quality chain is occupied; record state only and wait" }
        "backend_down" { "backend is down; coordinate with backend owner" }
        "web_lab_down" { "quality/backend path is ready, but Web Lab UI path is down" }
        "prompt_ready" { "quality chain is prompt-ready; run tiny smoke before manual or loop prompts" }
        default { "prompt-producing actions are blocked until gates pass" }
    }
    $operatorRecommendation = switch ($Bundle.classification) {
        "quality_worker_down" { "Do not run smoke, Web Lab prompt, Forge prompt, backend direct prompt, evolution-loop prompt rounds, or model-pool launch. Restore 8686 quality worker first." }
        "engine_busy" { "Do not send a new prompt. Wait for active owner or evolution-loop round, then rerun handoff-report and prompt-gate." }
        "prompt_ready" { "Run the exact smoke gate, then a tiny smoke, before any manual or loop prompt." }
        "web_lab_down" { "Keep Web Lab prompt/smoke blocked; backend-only paths still require their exact gate and operator coordination." }
        default { "Stay read-only and rerun diagnose, chain-status, and prompt-gate." }
    }

    $gates = @($Bundle.prompt_policy.entrypoint_gates | ForEach-Object {
        [pscustomobject]@{
            id = $_.id
            allowed = $_.allowed
            reason = $_.reason
            gate_command = $_.gate_command
            required_context_tokens = $_.required_context_tokens
            sends_prompt_after_gate = $_.sends_prompt_after_gate
            blocked_exit_code = $_.blocked_exit_code
        }
    })

    return [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        read_only = $true
        sends_prompt = $false
        launches_process = $false
        generated_at = (Get-Date).ToString("o")
        classification = $Bundle.classification
        current_state = $currentState
        operator_recommendation = $operatorRecommendation
        require_action = $Bundle.require_action
        require_action_allowed = $Bundle.require_action_allowed
        prompt_ready = $Bundle.prompt_ready
        engine_busy = $Bundle.engine_busy
        quality_worker_reachable = $Bundle.quality_worker_reachable
        model_pool_launch_allowed = $Bundle.model_pool_launch_allowed
        model_pool_launch_block_reason = $Bundle.model_pool_launch_block_reason
        wrapper_decision = $Bundle.wrapper_audit.wrapper_decision
        audit_passed = $Audit.audit_passed
        audit_failed_check_ids = $Audit.failed_check_ids
        endpoints = [pscustomobject]@{
            quality_worker = if ($Bundle.chain) { $Bundle.chain.endpoints.quality_worker } else { $null }
            backend = if ($Bundle.chain) { $Bundle.chain.endpoints.backend } else { $null }
            web_lab = if ($Bundle.chain) { $Bundle.chain.endpoints.web_lab } else { $null }
        }
        allowed_read_only_actions = $Bundle.prompt_policy.allowed_read_only_actions
        blocked_prompt_actions = $Bundle.prompt_policy.blocked_prompt_actions
        entrypoint_gates = $gates
        apple_silicon_pool_summary = [pscustomobject]@{
        recommendation = "Use one 12B quality worker on 8686 plus lightweight summary/router/review/index/test-gate helpers on 8687-8690 only after the quality gate is healthy."
            current_decision = if ($Bundle.model_pool_launch_allowed) { "pool launch gate may be checked immediately before launch" } else { "plan only; model-pool launch blocked" }
            required_gate = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked"
        }
        safe_next_commands = $Bundle.safe_next_commands
    }
}

function Get-HandoffReport {
    $bundle = Get-StatusBundle
    $audit = New-ContractAuditPublic -Bundle $bundle
    return New-HandoffReportPublic -Bundle $bundle -Audit $audit
}

function Show-HandoffReport {
    $report = Get-HandoffReport
    if ($JsonStatus) {
        Write-Host ($report | ConvertTo-StatusJson -Depth 18)
        if ($FailIfBlocked -and $report.require_action_allowed -ne $true) {
            exit 2
        }
        return
    }

    Write-Section "handoff report"
    Write-Host "schema_version=$($report.schema_version) contract_version=$($report.contract_version)"
    Write-Host "read_only=$($report.read_only) sends_prompt=$($report.sends_prompt) launches_process=$($report.launches_process)"
    Write-Host "classification=$($report.classification)"
    Write-Host "current_state=$($report.current_state)"
    Write-Host "recommendation=$($report.operator_recommendation)"
    Write-Host "require_action=$($report.require_action) allowed=$($report.require_action_allowed)"
    Write-Host "prompt_ready=$($report.prompt_ready) engine_busy=$($report.engine_busy) quality_worker_reachable=$($report.quality_worker_reachable)"
    Write-Host "model_pool_launch_allowed=$($report.model_pool_launch_allowed) reason=$($report.model_pool_launch_block_reason)"
    Write-Host "wrapper_decision=$($report.wrapper_decision) audit_passed=$($report.audit_passed)"

    Write-Section "endpoints"
    Write-Host "quality_worker=$($report.endpoints.quality_worker.base_url) tcp=$($report.endpoints.quality_worker.tcp_reachable) health=$($report.endpoints.quality_worker.health_ok)"
    Write-Host "backend=$($report.endpoints.backend.base_url) tcp=$($report.endpoints.backend.tcp_reachable) health=$($report.endpoints.backend.health_ok)"
    Write-Host "web_lab=$($report.endpoints.web_lab.base_url) tcp=$($report.endpoints.web_lab.tcp_reachable) health=$($report.endpoints.web_lab.health_ok)"

    Write-Section "blocked prompt actions"
    foreach ($action in $report.blocked_prompt_actions) {
        Write-Host $action
    }

    Write-Section "safe next commands"
    foreach ($command in $report.safe_next_commands) {
        Write-Host $command
    }

    Write-Section "apple silicon pool"
    Write-Host "recommendation=$($report.apple_silicon_pool_summary.recommendation)"
    Write-Host "decision=$($report.apple_silicon_pool_summary.current_decision)"
    Write-Host "required_gate=$($report.apple_silicon_pool_summary.required_gate)"

    if ($FailIfBlocked -and $report.require_action_allowed -ne $true) {
        exit 2
    }
}

function Get-SecretScanTargets {
    $targets = @()
    foreach ($pattern in @(
        "docs\runbooks\gemma*.md",
        "docs\architecture\integration*.md"
    )) {
        $targets += @(Get-ChildItem -LiteralPath $RepoRoot -File -Recurse -Filter (Split-Path $pattern -Leaf) |
            Where-Object { $_.FullName -like (Join-Path $RepoRoot ($pattern -replace '\*.*$', '*')) })
    }
    $toolPath = Join-Path $RepoRoot "tools\gemma-chain"
    if (Test-Path -LiteralPath $toolPath) {
        $targets += @(Get-ChildItem -LiteralPath $toolPath -File -Recurse | Where-Object {
            $_.Extension -in @(".ps1", ".cmd", ".md", ".json", ".txt", ".yml", ".yaml")
        })
    }
    return @($targets | Sort-Object FullName -Unique)
}

function Test-SecretScanAllowlistedLine {
    param([string]$Line)
    return (
        $Line -match 'sk-exampleSECRET1234567890' -or
        $Line -match '<redacted-sensitive-preview' -or
        $Line -match '<redacted-field>' -or
        $Line -match '<redacted-prompt' -or
        $Line -match '^\s*\$Line -match .*authoriz' -or
        $Line -match '^\s*authorization\s*=\s*\[pscustomobject\]@\{' -or
        $Line -match '^\s*authorization\s*=\s*\$[A-Za-z0-9_.]+\.authorization\b' -or
        $Line -match '^\s*Write-Host\s+"authorization:\s+daemon='
    )
}

function Get-SecretScanRules {
    return @(
        [pscustomobject]@{ id = "private_key_block"; severity = "critical"; pattern = '-----BEGIN [A-Z ]*PRIVATE KEY-----' },
        [pscustomobject]@{ id = "bearer_token"; severity = "high"; pattern = '(?i)\bbearer\s+[a-z0-9._~+/\-]{12,}' },
        [pscustomobject]@{ id = "openai_style_key"; severity = "high"; pattern = '(?i)\bsk-[a-z0-9_-]{12,}' },
        [pscustomobject]@{ id = "password_assignment"; severity = "high"; pattern = '(?i)\b(pass(word)?|passwd|pwd)\s*[:=]\s*\S+' },
        [pscustomobject]@{ id = "token_assignment"; severity = "high"; pattern = '(?i)\b(access[_-]?token|refresh[_-]?token|id[_-]?token|session[_-]?token|token)\s*[:=]\s*\S+' },
        [pscustomobject]@{ id = "api_key_assignment"; severity = "high"; pattern = '(?i)\b(api[_-]?key|authorization|cookie|secret)\s*[:=]\s*\S+' }
    )
}

function Get-SecretScan {
    $targets = @(Get-SecretScanTargets)
    $rules = @(Get-SecretScanRules)
    $findings = @()
    foreach ($file in $targets) {
        $lineNumber = 0
        foreach ($line in Get-Content -LiteralPath $file.FullName -ErrorAction Stop) {
            $lineNumber += 1
            if (Test-SecretScanAllowlistedLine $line) {
                continue
            }
            foreach ($rule in $rules) {
                if ($line -match $rule.pattern) {
                    $relativePath = $file.FullName.Substring($RepoRoot.Length).TrimStart('\')
                    $findings += [pscustomobject]@{
                        path = $relativePath
                        line = $lineNumber
                        rule_id = $rule.id
                        severity = $rule.severity
                        preview = Format-SafePreview $line 160
                    }
                }
            }
        }
    }

    return [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        read_only = $true
        sends_prompt = $false
        launches_process = $false
        scanned_at = (Get-Date).ToString("o")
        scanned_paths = @(
            "docs/runbooks/gemma*.md",
            "docs/architecture/integration*.md",
            "tools/gemma-chain/**"
        )
        file_count = $targets.Count
        rule_count = $rules.Count
        finding_count = $findings.Count
        passed = ($findings.Count -eq 0)
        findings = $findings
        policy = "If findings are present, do not paste raw matched text into handoff; fix or redact the source line first."
    }
}

function Show-SecretScan {
    $scan = Get-SecretScan
    if ($JsonStatus) {
        Write-Host ($scan | ConvertTo-StatusJson -Depth 12)
        if ($FailIfBlocked -and $scan.passed -ne $true) {
            exit 2
        }
        return
    }

    Write-Section "secret scan"
    Write-Host "schema_version=$($scan.schema_version) contract_version=$($scan.contract_version)"
    Write-Host "read_only=$($scan.read_only) sends_prompt=$($scan.sends_prompt) launches_process=$($scan.launches_process)"
    Write-Host "file_count=$($scan.file_count) rule_count=$($scan.rule_count) finding_count=$($scan.finding_count) passed=$($scan.passed)"
    Write-Host "scanned_paths=$($scan.scanned_paths -join ',')"
    if ($scan.finding_count -gt 0) {
        Write-Section "findings"
        foreach ($finding in $scan.findings) {
            Write-Host "$($finding.path):$($finding.line) rule=$($finding.rule_id) severity=$($finding.severity) preview=$($finding.preview)"
        }
    }
    Write-Host "policy=$($scan.policy)"
    if ($FailIfBlocked -and $scan.passed -ne $true) {
        exit 2
    }
}

function Show-Health {
    Write-Section "tcp"
    foreach ($entry in @(
        @("model", $ModelBaseUrl),
        @("backend", $BackendBaseUrl),
        @("web-lab", $LabBaseUrl)
    )) {
        $reachable = Test-TcpPort $entry[1]
        Write-Host "$($entry[0]) $($entry[1]) tcp_reachable=$reachable"
    }

    Write-Section "http health"
    $modelHealth = Show-Endpoint "model health" "$ModelBaseUrl/health"
    $backendHealth = Show-Endpoint "backend health" "$BackendBaseUrl/health"
    $labHealth = Show-Endpoint "web lab health" "$LabBaseUrl/health"
    $labBackend = Show-Endpoint "web lab backend health" "$LabBaseUrl/api/backend-health"

    if ($backendHealth.Ok) {
        $h = $backendHealth.Value
        Write-Section "backend summary"
        Write-Host "runtime_mode=$($h.runtime_mode)"
        Write-Host "model=$($h.gemma_runtime_model)"
        Write-Host "context_window=$($h.gemma_runtime_context_window)"
        Write-Host "train_context_window=$($h.gemma_runtime_train_context_window)"
        Write-Host "engine_busy=$($h.engine_busy)"
        Write-Host "active_engine_requests=$($h.active_engine_requests)"
        if ($h.engine_busy -eq $true -and $h.active_requests) {
            $active = @($h.active_requests)
            foreach ($request in $active) {
                $preview = Format-SafePreview ([string]$request.prompt_preview)
                Write-Host "active request_id=$($request.request_id) endpoint=$($request.endpoint) elapsed_ms=$($request.elapsed_ms) prompt_preview=$preview"
            }
        }
    }

    return [pscustomobject]@{
        ModelHealth = $modelHealth
        BackendHealth = $backendHealth
        LabHealth = $labHealth
        LabBackendHealth = $labBackend
    }
}

function Show-Diagnose {
    Show-PortResponsibilities
    $state = Show-Health

    Write-Section "metadata probes"
    $metadata = Show-Endpoint "model metadata" "$ModelBaseUrl/metadata"
    $models = Show-Endpoint "model list" "$ModelBaseUrl/v1/models"
    if (-not $metadata.Ok) {
        if (-not $state.ModelHealth.Ok) {
            Write-Host "metadata unavailable: model health is unreachable; do not send prompts or restart services from diagnose."
        } elseif ($metadata.Error -match '(?i)(timeout|timed out|10060|operation has timed out|connection timed out|无法连接|连接.*失败)') {
            Write-Host "metadata timeout: treat as transient if backend health still reports gemma_runtime_reachable=true; retry after active inference ends."
        } else {
            Write-Host "metadata note: /metadata may be unsupported or temporarily unavailable; backend /health projection is the primary source."
        }
    }
    if (-not $models.Ok) {
        if (-not $state.ModelHealth.Ok) {
            Write-Host "model list unavailable: model health is unreachable."
        } else {
            Write-Host "model list note: /v1/models may be unsupported by this llama-server build."
        }
    }

    Write-Section "diagnosis"
    if (-not $state.BackendHealth.Ok) {
        Write-Host "backend health failed; do not send smoke prompts."
        exit 1
    }

    $h = $state.BackendHealth.Value
    $canPrompt = Test-BackendCanPrompt $h
    if ($canPrompt) {
        Write-Host "backend is idle and ready; tiny smoke is allowed."
    } else {
        Write-Host "prompt blocked: $(Get-PromptBlockReason $h)"
        Write-Host "diagnose records state only; smoke must not send a prompt until the block clears."
    }

    if ($h.gemma_runtime_reachable -ne $true) {
        Write-Host "model runtime is not reachable through backend health."
    }
    if ($h.gemma_runtime_metadata_error) {
        Write-Host "metadata warning: $($h.gemma_runtime_metadata_error)"
    }
    if ($h.gemma_runtime_context_window) {
        Write-Host "context window from backend: $($h.gemma_runtime_context_window)"
    }
    Show-ContextBudgetDiagnosis $h $metadata.Value $models.Value
    if ($h.readiness_ok -ne $true) {
        Write-Host "readiness gate is not OK: $($h.readiness_failures -join ', ')"
    }
    if ($h.safe_device_ok -ne $true) {
        Write-Host "safe-device gate is not OK: $($h.safe_device_failures -join ', ')"
    }
    Show-StreamContinuityGuidance $h
}

function Show-ContextBudgetDiagnosis {
    param(
        $BackendHealth,
        $Metadata,
        $Models
    )

    Write-Section "context and max_tokens"
    $runtimeContext = Get-PropertyValue $BackendHealth @("gemma_runtime_context_window", "runtime_context_window", "context_window", "n_ctx")
    $trainContext = Get-PropertyValue $BackendHealth @("gemma_runtime_train_context_window", "train_context_window", "n_ctx_train")
    $defaultMax = Get-PropertyValue $BackendHealth @("gemma_runtime_default_max_tokens", "default_max_tokens", "max_tokens", "n_predict")
    $metadataContext = Get-PropertyValue $Metadata @("n_ctx", "context_window", "gemma_runtime_context_window")
    $metadataDefaultMax = Get-PropertyValue $Metadata @("n_predict", "default_max_tokens", "max_tokens")
    $modelListMeta = $null
    if ($null -ne $Models -and $null -ne $Models.PSObject.Properties["data"] -and $null -ne $Models.data.PSObject.Properties["meta"]) {
        $modelListMeta = $Models.data.meta
    }
    if (-not $metadataContext -and $modelListMeta) {
        $metadataContext = Get-PropertyValue $modelListMeta @("n_ctx", "context_window", "gemma_runtime_context_window")
    }
    $metadataTrainContext = Get-PropertyValue $modelListMeta @("n_ctx_train", "train_context_window")

    Write-Host "backend_context_window=$runtimeContext"
    Write-Host "backend_train_context_window=$trainContext"
    Write-Host "backend_default_max_tokens=$defaultMax"
    if ($metadataContext) {
        Write-Host "metadata_context_window=$metadataContext"
    }
    if ($metadataTrainContext) {
        Write-Host "metadata_train_context_window=$metadataTrainContext"
    }
    if ($metadataDefaultMax) {
        Write-Host "metadata_default_max_tokens=$metadataDefaultMax"
    }

    $runtimeContextInt = ConvertTo-Int64OrNull $runtimeContext
    $trainContextInt = ConvertTo-Int64OrNull $trainContext
    $metadataContextInt = ConvertTo-Int64OrNull $metadataContext
    $metadataTrainContextInt = ConvertTo-Int64OrNull $metadataTrainContext
    $defaultMaxInt = ConvertTo-Int64OrNull $defaultMax

    if ($runtimeContextInt -and $trainContextInt -and ($runtimeContextInt -ne $trainContextInt)) {
        Write-Host "context mismatch: runtime context and train context differ; prefer the smaller value for scheduling."
    }
    if ($runtimeContextInt -and $metadataContextInt -and ($runtimeContextInt -ne $metadataContextInt)) {
        Write-Host "context mismatch: backend health and model metadata disagree; avoid long-context tests until metadata is stable."
    }
    if ($trainContextInt -and $metadataTrainContextInt -and ($trainContextInt -ne $metadataTrainContextInt)) {
        Write-Host "context mismatch: backend train context and model metadata train context disagree."
    }
    if ($runtimeContextInt -and $defaultMaxInt -and ($defaultMaxInt -gt $runtimeContextInt)) {
        Write-Host "max_tokens mismatch: default max_tokens exceeds runtime context; keep smoke tests tiny and inspect backend launch defaults."
    } elseif ($runtimeContextInt -and -not $defaultMaxInt) {
        Write-Host "max_tokens note: backend health does not expose default max_tokens; use explicit small MaxTokens in evolution-loop and smoke callers."
    }
}

function Show-StreamContinuityGuidance {
    param($BackendHealth)

    Write-Section "stream continuity"
    Write-Host "expected terminal sequence: delta events followed by event: done with data: [DONE], or a terminal event: error."
    Write-Host "diagnose is read-only and does not open a stream; use smoke only after prompt blocks clear."
    if (Test-BackendCanPrompt $BackendHealth) {
        Write-Host "current state: prompt-ready, so smoke can verify delta/done continuity with a tiny OK prompt."
    } else {
        Write-Host "current state: prompt blocked ($(Get-PromptBlockReason $BackendHealth)), so no stream prompt should be sent."
    }
}

function Test-SseContinuity {
    param([string]$Text)
    $hasDelta = $Text -match 'event:\s*delta'
    $hasDone = $Text -match 'event:\s*done\s+data:\s*\[DONE\]'
    $hasError = $Text -match 'event:\s*error'
    $lastFrameComplete = $Text -match '(\r?\n){2}$'
    return [pscustomobject]@{
        HasDelta = $hasDelta
        HasDone = $hasDone
        HasError = $hasError
        LastFrameComplete = $lastFrameComplete
        IsComplete = (($hasDone -or $hasError) -and $lastFrameComplete)
    }
}

function Invoke-SelfTest {
    function Assert-SelfTest {
        param(
            [bool]$Condition,
            [string]$Message
        )
        if (-not $Condition) {
            throw $Message
        }
    }

    function New-FixtureEndpoint {
        param(
            [string]$Name,
            [bool]$TcpReachable,
            [bool]$HealthOk,
            $HealthValue = $null
        )
        return [pscustomobject]@{
            name = $Name
            base_url = "fixture://$Name"
            tcp_reachable = $TcpReachable
            health_ok = $HealthOk
            health_elapsed_ms = 0
            health_error = if ($HealthOk) { $null } else { "fixture unavailable" }
            health_value = $HealthValue
        }
    }

    function Get-FixtureAction {
        param(
            $Status,
            [string]$Action
        )
        return @($Status.allowed_actions | Where-Object { $_.action -eq $Action })[0]
    }

    function Assert-ObjectHasProperties {
        param(
            $Object,
            [string[]]$Names,
            [string]$Label
        )
        foreach ($name in $Names) {
            Assert-SelfTest ($null -ne $Object.PSObject.Properties[$name]) "$Label missing field $name"
        }
    }

    Write-Section "redaction selftest"
    $secretPreview = Format-SafePreview "token=sk-exampleSECRET1234567890"
    Write-Host "secret_preview=$secretPreview"
    Assert-SelfTest ($secretPreview -match '^<redacted-sensitive-preview') "redaction selftest failed"

    Write-Section "sse continuity selftest"
    $complete = Test-SseContinuity "event: delta`ndata: OK`n`nevent: done`ndata: [DONE]`n`n"
    $truncated = Test-SseContinuity "event: delta`ndata: OK"
    Write-Host "complete_stream=$($complete | ConvertTo-SafeJson)"
    Write-Host "truncated_stream=$($truncated | ConvertTo-SafeJson)"
    Assert-SelfTest ($complete.IsComplete -eq $true) "complete SSE fixture was not recognized"
    Assert-SelfTest ($truncated.IsComplete -ne $true) "truncated SSE fixture was not rejected"

    Write-Section "chain classification selftest"
    $modelUp = New-FixtureEndpoint "quality-worker" $true $true ([pscustomobject]@{ status = "ok" })
    $modelDown = New-FixtureEndpoint "quality-worker" $false $false
    $labUp = New-FixtureEndpoint "web-lab" $true $true ([pscustomobject]@{ ok = $true; service = "rustgpt-lab" })
    $labDown = New-FixtureEndpoint "web-lab" $false $false
    $backendDown = New-FixtureEndpoint "backend" $false $false
    $backendHealthy = New-FixtureEndpoint "backend" $true $true ([pscustomobject]@{
        engine_busy = $false
        gemma_runtime_reachable = $true
        readiness_ok = $true
        safe_device_ok = $true
        gemma_runtime_context_window = 262144
    })
    $backendSmallContext = New-FixtureEndpoint "backend" $true $true ([pscustomobject]@{
        engine_busy = $false
        gemma_runtime_reachable = $true
        readiness_ok = $true
        safe_device_ok = $true
        gemma_runtime_context_window = 8192
    })
    $backendBusy = New-FixtureEndpoint "backend" $true $true ([pscustomobject]@{
        engine_busy = $true
        gemma_runtime_reachable = $true
        readiness_ok = $true
        safe_device_ok = $true
    })
    $backendQualityDown = New-FixtureEndpoint "backend" $true $true ([pscustomobject]@{
        engine_busy = $false
        gemma_runtime_reachable = $false
        readiness_ok = $false
        safe_device_ok = $true
    })

    $backendDownStatus = Get-ChainStatusFromSnapshots -Model $modelUp -Backend $backendDown -Lab $labUp
    $busyStatus = Get-ChainStatusFromSnapshots -Model $modelUp -Backend $backendBusy -Lab $labUp
    $qualityDownStatus = Get-ChainStatusFromSnapshots -Model $modelDown -Backend $backendQualityDown -Lab $labUp
    $webLabDownStatus = Get-ChainStatusFromSnapshots -Model $modelUp -Backend $backendHealthy -Lab $labDown
    $promptReadyStatus = Get-ChainStatusFromSnapshots -Model $modelUp -Backend $backendHealthy -Lab $labUp

    Write-Host "backend_down=$($backendDownStatus.classification)"
    Write-Host "engine_busy=$($busyStatus.classification)"
    Write-Host "quality_worker_down=$($qualityDownStatus.classification)"
    Write-Host "web_lab_down=$($webLabDownStatus.classification)"
    Write-Host "prompt_ready=$($promptReadyStatus.classification)"

    Assert-SelfTest ($backendDownStatus.classification -eq "backend_down") "backend_down classification failed"
    Assert-SelfTest ($busyStatus.classification -eq "engine_busy") "engine_busy classification failed"
    Assert-SelfTest ($qualityDownStatus.classification -eq "quality_worker_down") "quality_worker_down classification failed"
    Assert-SelfTest ($webLabDownStatus.classification -eq "web_lab_down") "web_lab_down classification failed"
    Assert-SelfTest ($promptReadyStatus.classification -eq "prompt_ready") "prompt_ready classification failed"

    Assert-SelfTest ((Get-FixtureAction $qualityDownStatus "model-pool launch").allowed -eq $false) "model-pool launch should be blocked when quality worker is down"
    Assert-SelfTest ((Get-FixtureAction $qualityDownStatus "entrypoint-matrix").allowed -eq $true) "entrypoint-matrix should remain read-only when quality worker is down"
    Assert-SelfTest ((Get-FixtureAction $qualityDownStatus "pool-manifest").allowed -eq $true) "pool-manifest should remain read-only when quality worker is down"
    Assert-SelfTest ((Get-FixtureAction $qualityDownStatus "pool-status").allowed -eq $true) "pool-status should remain read-only when quality worker is down"
    Assert-SelfTest ((Get-FixtureAction $qualityDownStatus "pool-route-plan").allowed -eq $true) "pool-route-plan should remain read-only when quality worker is down"
    Assert-SelfTest ((Get-FixtureAction $qualityDownStatus "contract-audit").allowed -eq $true) "contract-audit should remain read-only when quality worker is down"
    Assert-SelfTest ((Get-FixtureAction $qualityDownStatus "wrapper-manifest").allowed -eq $true) "wrapper-manifest should remain read-only when quality worker is down"
    Assert-SelfTest ((Get-FixtureAction $qualityDownStatus "contract-fixture").allowed -eq $true) "contract-fixture should remain offline/read-only when quality worker is down"
    Assert-SelfTest ((Get-FixtureAction $qualityDownStatus "handoff-report").allowed -eq $true) "handoff-report should remain read-only when quality worker is down"
    Assert-SelfTest ((Get-FixtureAction $qualityDownStatus "secret-scan").allowed -eq $true) "secret-scan should remain read-only when quality worker is down"
    Assert-SelfTest ((Get-FixtureAction $webLabDownStatus "web-lab prompt").allowed -eq $false) "web-lab prompt should be blocked when Web Lab is down"
    Assert-SelfTest ((Get-FixtureAction $webLabDownStatus "forge cli prompt").allowed -eq $true) "forge cli prompt should remain backend-allowed when only Web Lab is down"
    Assert-SelfTest ((Get-FixtureAction $promptReadyStatus "smoke").allowed -eq $true) "smoke should be allowed when chain is prompt-ready"
    Assert-SelfTest ((Test-RequiredActionAllowed -Status $qualityDownStatus -RequiredAction "model_pool_launch") -eq $false) "RequireAction model_pool_launch should fail when quality worker is down"
    Assert-SelfTest ((Test-RequiredActionAllowed -Status $webLabDownStatus -RequiredAction "smoke") -eq $false) "RequireAction smoke should fail when Web Lab is down"
    Assert-SelfTest ((Test-RequiredActionAllowed -Status $webLabDownStatus -RequiredAction "forge_cli_prompt") -eq $true) "RequireAction forge_cli_prompt should pass when only Web Lab is down"

    $MinContextTokens = 262144
    $smallContextStatus = Get-ChainStatusFromSnapshots -Model $modelUp -Backend $backendSmallContext -Lab $labUp
    Write-Host "small_context_context_ready=$($smallContextStatus.prompt_gate.context_ready)"
    Assert-SelfTest ($smallContextStatus.prompt_gate.context_ready -eq $false) "context gate should fail for undersized context"
    Assert-SelfTest ((Test-RequiredActionAllowed -Status $smallContextStatus -RequiredAction "evolution_loop_prompt_round") -eq $false) "RequireAction evolution_loop_prompt_round should fail for undersized context"

    Write-Section "prompt-gate contract selftest"
    $qualityDownPromptGate = Get-PromptGateStatus $backendQualityDown.health_value
    $promptGateRoundTrip = $qualityDownPromptGate | ConvertTo-StatusJson -Depth 18 | ConvertFrom-Json
    Assert-ObjectHasProperties $promptGateRoundTrip @(
        "schema_version",
        "contract_version",
        "prompt_ready",
        "block_reason",
        "context_ready",
        "entrypoints",
        "entrypoint_decisions",
        "contract"
    ) "prompt-gate json"
    Assert-SelfTest ($promptGateRoundTrip.schema_version -eq 1) "prompt-gate json schema_version mismatch"
    Assert-SelfTest ($promptGateRoundTrip.contract_version -eq "gemma-chain.v1") "prompt-gate json contract_version mismatch"
    Assert-SelfTest ($promptGateRoundTrip.contract.schema_version -eq 1) "prompt-gate nested contract schema_version mismatch"
    Assert-SelfTest ($promptGateRoundTrip.contract.contract_version -eq "gemma-chain.v1") "prompt-gate nested contract_version mismatch"
    Assert-SelfTest ($promptGateRoundTrip.contract.read_only -eq $true) "prompt-gate contract must be read-only"
    Assert-SelfTest ($promptGateRoundTrip.contract.sends_prompt -eq $false) "prompt-gate contract must not send prompts"
    Assert-SelfTest ($promptGateRoundTrip.contract.launches_process -eq $false) "prompt-gate contract must not launch processes"
    Assert-SelfTest ($promptGateRoundTrip.entrypoints.web_lab_manual_prompt.allowed -eq $false) "prompt-gate legacy Web Lab alias should remain blocked"
    Assert-SelfTest ($promptGateRoundTrip.entrypoints.web_lab_prompt.allowed -eq $false) "prompt-gate Web Lab action id should remain blocked"
    Assert-SelfTest (@($promptGateRoundTrip.entrypoint_decisions).Count -eq 6) "prompt-gate should expose six standard entrypoint decisions"
    Assert-SelfTest (@($promptGateRoundTrip.entrypoint_decisions | Where-Object { $_.id -eq "web_lab_prompt" -and $_.compatibility_alias -eq "web_lab_manual_prompt" }).Count -eq 1) "prompt-gate should map legacy Web Lab alias"
    Assert-SelfTest (@($promptGateRoundTrip.entrypoint_decisions | Where-Object { $_.id -eq "evolution_loop_prompt_round" -and $_.standard_required_context_tokens -eq 262144 }).Count -eq 1) "prompt-gate should expose evolution-loop context gate"
    Assert-SelfTest (@($promptGateRoundTrip.entrypoint_decisions | Where-Object { $_.id -eq "model_pool_launch" -and $_.standard_required_context_tokens -eq 262144 -and $_.sends_prompt_after_gate -eq $false }).Count -eq 1) "prompt-gate should expose model-pool launch gate"
    Assert-SelfTest (@($promptGateRoundTrip.entrypoint_decisions | Where-Object { $_.prompt_gate_read_only -eq $true -and $_.blocked_exit_code -eq 2 }).Count -eq 6) "prompt-gate entrypoint decisions should be read-only gates"

    Write-Section "pool plan selftest"
    $plan = Get-AppleSiliconPoolPlan
    Assert-SelfTest ($plan.SchemaVersion -eq 1) "pool plan schema version mismatch"
    Assert-SelfTest ($plan.ContractVersion -eq "gemma-chain.v1") "pool plan contract version mismatch"
    Assert-SelfTest ($plan.BlockedPolicy -match "model-pool launch is blocked") "pool plan must state blocked launch policy"
    Assert-SelfTest ($plan.QualityGateCommand -match "model_pool_launch") "pool plan must expose model_pool_launch gate"
    Assert-SelfTest (@($plan.Validation | Where-Object { $_ -match "\bsmoke\b" }).Count -eq 0) "read-only pool validation must not include smoke"
    Assert-SelfTest (@($plan.Validation | Where-Object { $_ -match "test-remote-model-pool-guards" }).Count -eq 1) "read-only pool validation must include helper model guard selftest"
    Assert-SelfTest (@($plan.PromptValidationAfterGate | Where-Object { $_ -match "\bsmoke\b" }).Count -gt 0) "pool plan should list smoke only after gate"
    $manifest = Get-ModelPoolManifest
    $manifestRoundTrip = $manifest | ConvertTo-StatusJson -Depth 10 | ConvertFrom-Json
    Assert-ObjectHasProperties $manifestRoundTrip @(
        "schema_version",
        "contract_version",
        "read_only",
        "sends_prompt",
        "launches_process",
        "manifest_kind",
        "capacity_policy",
        "advice",
        "decision_source",
        "safe_to_enable_pool_workers",
        "next_step",
        "reason",
        "extra_quality_12b_detected",
        "quality_worker_count",
        "helper_worker_count",
        "helper_target_worker_count",
        "helper_roles",
        "capacity_recommendation",
        "worker_shape",
        "workers"
    ) "pool-manifest json"
    Assert-SelfTest ($manifestRoundTrip.schema_version -eq 1) "pool-manifest schema_version mismatch"
    Assert-SelfTest ($manifestRoundTrip.contract_version -eq "gemma-chain.v1") "pool-manifest contract_version mismatch"
    Assert-SelfTest ($manifestRoundTrip.read_only -eq $true) "pool-manifest must be read-only"
    Assert-SelfTest ($manifestRoundTrip.sends_prompt -eq $false) "pool-manifest must not send prompts"
    Assert-SelfTest ($manifestRoundTrip.launches_process -eq $false) "pool-manifest must not launch processes"
    Assert-SelfTest ($manifestRoundTrip.manifest_kind -eq "rust-norion.model-pool") "pool-manifest kind mismatch"
    Assert-SelfTest ($manifestRoundTrip.capacity_policy.policy -eq "one_quality_plus_small_helpers") "pool-manifest capacity policy mismatch"
    Assert-SelfTest ($manifestRoundTrip.capacity_policy.avoid_extra_12b -eq $true) "pool-manifest should avoid extra 12B workers"
    Assert-SelfTest ($manifestRoundTrip.capacity_policy.max_quality_12b_workers -eq 1) "pool-manifest should cap quality 12B workers"
    Assert-SelfTest ($manifestRoundTrip.capacity_policy.helper_context_tokens_total -eq 28672) "pool-manifest helper context budget mismatch"
    Assert-SelfTest ($manifestRoundTrip.capacity_policy.helper_default_max_tokens_total -eq 4864) "pool-manifest helper output budget mismatch"
    Assert-SelfTest ($manifestRoundTrip.capacity_policy.helper_model_size_policy -eq "small_or_low_quant_only") "pool-manifest helper model size policy mismatch"
    Assert-SelfTest ($manifestRoundTrip.capacity_policy.large_helper_model_guard -match "AllowLargePoolWorkerModels") "pool-manifest should expose large helper guard override"
    Assert-SelfTest ($manifestRoundTrip.capacity_policy.guard_validation_command -match "test-remote-model-pool-guards") "pool-manifest should expose helper model guard validation command"
    Assert-SelfTest (@($manifestRoundTrip.capacity_policy.recommended_launch_order | Where-Object { $_ -eq "summary" }).Count -eq 1) "pool-manifest should recommend summary helper launch"
    Assert-SelfTest ($manifestRoundTrip.advice.decision_source -eq "model-pool-advice-core") "pool-manifest advice source mismatch"
    Assert-SelfTest ($manifestRoundTrip.decision_source -eq "model-pool-advice-core") "pool-manifest flattened advice source mismatch"
    Assert-SelfTest ($manifestRoundTrip.safe_to_enable_pool_workers -eq $true) "pool-manifest should allow helper planning by default"
    Assert-SelfTest ($manifestRoundTrip.next_step -eq "run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls") "pool-manifest next step mismatch"
    Assert-SelfTest ($manifestRoundTrip.reason -eq "full_helper_pool_visible") "pool-manifest advice reason mismatch"
    Assert-SelfTest ($manifestRoundTrip.extra_quality_12b_detected -eq $false) "pool-manifest should not plan extra quality 12B workers"
    Assert-SelfTest ($manifestRoundTrip.quality_worker_count -eq 1) "pool-manifest quality worker count mismatch"
    Assert-SelfTest ($manifestRoundTrip.helper_worker_count -eq 5) "pool-manifest helper worker count mismatch"
    Assert-SelfTest ($manifestRoundTrip.helper_target_worker_count -eq 5) "pool-manifest helper target mismatch"
    Assert-SelfTest ($manifestRoundTrip.capacity_recommendation -eq "run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls") "pool-manifest capacity recommendation mismatch"
    Assert-SelfTest ($manifestRoundTrip.worker_shape.quality -eq 1 -and $manifestRoundTrip.worker_shape.helpers_visible -eq 5 -and $manifestRoundTrip.worker_shape.helper_target -eq 5) "pool-manifest worker shape mismatch"
    Assert-SelfTest (@($manifestRoundTrip.helper_roles | Where-Object { $_ -eq "router" }).Count -eq 1) "pool-manifest helper roles should include router"
    Assert-SelfTest (@($manifestRoundTrip.helper_roles | Where-Object { $_ -eq "test-gate" }).Count -eq 1) "pool-manifest helper roles should include test-gate"
    Assert-SelfTest ($manifestRoundTrip.advice.safe_to_enable_pool_workers -eq $manifestRoundTrip.safe_to_enable_pool_workers) "pool-manifest flattened advice should match nested advice"
    Assert-SelfTest (@($manifestRoundTrip.workers).Count -eq 6) "pool-manifest should include six planned workers"
    Assert-SelfTest (@($manifestRoundTrip.workers | Where-Object { $_.launches_process -eq $true }).Count -eq 0) "pool-manifest workers must describe endpoints without launching processes"
    foreach ($role in @("quality", "summary", "router", "review", "test-gate", "index")) {
        Assert-SelfTest (@($manifestRoundTrip.workers | Where-Object { $_.role -eq $role }).Count -eq 1) "pool-manifest should include exactly one $role worker"
    }
    $qualityManifestWorker = @($manifestRoundTrip.workers | Where-Object { $_.role -eq "quality" })[0]
    Assert-SelfTest ($qualityManifestWorker.base_url -eq $ModelBaseUrl) "pool-manifest quality base_url should follow ModelBaseUrl"
    Assert-SelfTest ($qualityManifestWorker.port -eq ([Uri]$ModelBaseUrl).Port) "pool-manifest quality port should follow ModelBaseUrl"
    Assert-SelfTest ($qualityManifestWorker.default_context_tokens -eq 262144 -and $qualityManifestWorker.default_max_tokens -eq 262144 -and $qualityManifestWorker.low_priority -eq $false) "pool-manifest should expose quality worker defaults"
    Assert-SelfTest (@($manifestRoundTrip.workers | Where-Object { $_.role -eq "summary" -and $_.base_url -eq "http://127.0.0.1:8687" -and $_.default_context_tokens -eq 8192 -and $_.default_max_tokens -eq 768 -and $_.low_priority -eq $true }).Count -eq 1) "pool-manifest should expose summary helper defaults"
    Assert-SelfTest (@($manifestRoundTrip.workers | Where-Object { $_.role -eq "review" -and $_.base_url -eq "http://127.0.0.1:8688" -and $_.default_max_tokens -eq 1536 -and $_.low_priority -eq $true }).Count -eq 1) "pool-manifest should expose review worker defaults"
    Assert-SelfTest (@($manifestRoundTrip.workers | Where-Object { $_.role -eq "router" -and $_.base_url -eq "http://127.0.0.1:8689" -and $_.default_context_tokens -eq 4096 -and $_.default_max_tokens -eq 512 -and $_.low_priority -eq $true }).Count -eq 1) "pool-manifest should expose router helper defaults"
    Assert-SelfTest (@($manifestRoundTrip.workers | Where-Object { $_.role -eq "test-gate" -and $_.base_url -eq "http://127.0.0.1:8688" -and $_.default_context_tokens -eq 4096 -and $_.default_max_tokens -eq 1536 -and $_.low_priority -eq $true }).Count -eq 1) "pool-manifest should expose test-gate helper defaults"
    Assert-SelfTest (@($manifestRoundTrip.workers | Where-Object { $_.role -eq "index" -and $_.base_url -eq "http://127.0.0.1:8690" -and $_.default_max_tokens -eq 512 -and $_.enabled_by_default -eq $true -and $_.low_priority -eq $true }).Count -eq 1) "pool-manifest should expose index helper defaults"
    $previousModelBaseUrl = $ModelBaseUrl
    try {
        $ModelBaseUrl = "http://127.0.0.1:8696"
        $customManifestRoundTrip = Get-ModelPoolManifest | ConvertTo-StatusJson -Depth 10 | ConvertFrom-Json
        $customQualityWorker = @($customManifestRoundTrip.workers | Where-Object { $_.role -eq "quality" })[0]
        Assert-SelfTest ($customQualityWorker.base_url -eq "http://127.0.0.1:8696") "pool-manifest custom quality base_url should follow overridden ModelBaseUrl"
        Assert-SelfTest ($customQualityWorker.port -eq 8696) "pool-manifest custom quality port should follow overridden ModelBaseUrl"
        Assert-SelfTest (@($customManifestRoundTrip.workers | Where-Object { $_.role -eq "review" -and $_.base_url -eq "http://127.0.0.1:8688" -and $_.port -eq 8688 }).Count -eq 1) "pool-manifest custom ModelBaseUrl should not move review helper port"
    } finally {
        $ModelBaseUrl = $previousModelBaseUrl
    }

    Write-Section "post recovery validation selftest"
    $postRecoveryValidation = @(Get-PostRecoveryValidationCommands)
    $smokeGateIndex = [array]::IndexOf($postRecoveryValidation, ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -JsonStatus -FailIfBlocked")
    $smokeIndex = [array]::IndexOf($postRecoveryValidation, ".\tools\gemma-chain\gemma-chain.cmd smoke")
    Write-Host "smoke_gate_index=$smokeGateIndex smoke_index=$smokeIndex"
    Assert-SelfTest ($smokeGateIndex -ge 0) "post-recovery validation must include smoke gate"
    Assert-SelfTest ($smokeIndex -ge 0) "post-recovery validation must include smoke command"
    Assert-SelfTest ($smokeGateIndex -lt $smokeIndex) "post-recovery validation must gate smoke before sending prompt"
    $releaseSequence = @(Get-PostRecoveryReleaseSequence)
    $releaseIds = @($releaseSequence | ForEach-Object { $_.id })
    $releaseSmokeGateIndex = [array]::IndexOf($releaseIds, "smoke_gate")
    $releaseSmokeIndex = [array]::IndexOf($releaseIds, "smoke")
    $releaseLoopGate = @($releaseSequence | Where-Object { $_.id -eq "evolution_loop_prompt_gate" })[0]
    $releasePoolGate = @($releaseSequence | Where-Object { $_.id -eq "model_pool_launch_gate" })[0]
    $releasePoolLaunch = @($releaseSequence | Where-Object { $_.id -eq "model_pool_launch" })[0]
    Write-Host "release_smoke_gate_index=$releaseSmokeGateIndex release_smoke_index=$releaseSmokeIndex"
    Assert-SelfTest (@($releaseSequence).Count -ge 10) "post-recovery release sequence should expose structured release steps"
    Assert-SelfTest ($releaseSmokeGateIndex -ge 0) "post-recovery release sequence must include smoke gate"
    Assert-SelfTest ($releaseSmokeIndex -ge 0) "post-recovery release sequence must include smoke command"
    Assert-SelfTest ($releaseSmokeGateIndex -lt $releaseSmokeIndex) "post-recovery release sequence must gate smoke before prompt"
    Assert-SelfTest (@($releaseSequence | Where-Object { $_.read_only -eq $true -and $_.sends_prompt -eq $true }).Count -eq 0) "read-only release sequence rows must not send prompts"
    Assert-SelfTest ($releaseLoopGate.required_context_tokens -eq 262144) "post-recovery release sequence must context-gate evolution loop"
    Assert-SelfTest ($releasePoolGate.required_context_tokens -eq 262144) "post-recovery release sequence must context-gate model pool launch"
    Assert-SelfTest ($releasePoolGate.read_only -eq $true -and $releasePoolGate.sends_prompt -eq $false -and $releasePoolGate.launches_process -eq $false) "model-pool launch gate must be read-only"
    Assert-SelfTest ($releasePoolLaunch.launches_process -eq $true -and $releasePoolLaunch.sends_prompt -eq $false) "model-pool launch row should describe process launch without prompt"
    Assert-SelfTest ($releasePoolLaunch.command -eq $null) "diagnostics must not provide an executable model-pool launch command"

    Write-Section "pool context gate selftest"
    $defaultPoolGate = Get-ModelPoolContextGate -RequestedMinContextTokens 0
    Write-Host "default_pool_min_context_tokens=$($defaultPoolGate.min_context_tokens) source=$($defaultPoolGate.context_gate_source)"
    Assert-SelfTest ($defaultPoolGate.min_context_tokens -eq 262144) "pool-status default context gate should use quality worker context"
    Assert-SelfTest ($defaultPoolGate.context_gate_source -eq "quality default") "pool-status default context gate should record quality default source"
    $overridePoolGate = Get-ModelPoolContextGate -RequestedMinContextTokens 8192
    Write-Host "override_pool_min_context_tokens=$($overridePoolGate.min_context_tokens) source=$($overridePoolGate.context_gate_source)"
    Assert-SelfTest ($overridePoolGate.min_context_tokens -eq 8192) "pool-status should honor explicit MinContextTokens"
    Assert-SelfTest ($overridePoolGate.context_gate_source -eq "cli") "pool-status override should record cli source"

    Write-Section "machine json contract selftest"
    $chainPublic = New-ChainStatusPublic -Status $qualityDownStatus -RequiredAction "model_pool_launch" -WaitReadyValue $false -WaitTimeoutSecValue 7
    $chainRoundTrip = $chainPublic | ConvertTo-StatusJson | ConvertFrom-Json
    Assert-ObjectHasProperties $chainRoundTrip @(
        "schema_version",
        "contract_version",
        "classification",
        "next_step",
        "require_action",
        "require_action_allowed",
        "wait_ready",
        "wait_timeout_sec",
        "endpoints",
        "prompt_gate",
        "allowed_actions",
        "allowed_action_table",
        "entrypoint_gates",
        "integration_entrypoints",
        "machine_summary"
    ) "chain-status json"
    Assert-SelfTest ($chainRoundTrip.schema_version -eq 1) "chain-status json schema_version mismatch"
    Assert-SelfTest ($chainRoundTrip.contract_version -eq "gemma-chain.v1") "chain-status json contract_version mismatch"
    Assert-SelfTest ($chainRoundTrip.machine_summary.schema_version -eq 1) "chain-status machine_summary schema_version mismatch"
    Assert-SelfTest ($chainRoundTrip.machine_summary.contract_version -eq "gemma-chain.v1") "chain-status machine_summary contract_version mismatch"
    Assert-SelfTest ($chainRoundTrip.classification -eq "quality_worker_down") "chain-status json classification mismatch"
    Assert-SelfTest ($chainRoundTrip.require_action -eq "model_pool_launch") "chain-status json require_action mismatch"
    Assert-SelfTest ($chainRoundTrip.require_action_allowed -eq $false) "chain-status json should block model_pool_launch"
    Assert-SelfTest ($chainRoundTrip.prompt_gate.prompt_ready -eq $false) "chain-status json prompt_gate should block prompts"
    Assert-SelfTest ($chainRoundTrip.endpoints.quality_worker.tcp_reachable -eq $false) "chain-status json should include quality worker TCP state"
    Assert-SelfTest (@($chainRoundTrip.allowed_actions | Where-Object { $_.id -eq "pool_status" -and $_.allowed -eq $true }).Count -eq 1) "chain-status json should allow read-only pool_status"
    Assert-SelfTest (@($chainRoundTrip.allowed_actions | Where-Object { $_.id -eq "entrypoint_matrix" -and $_.allowed -eq $true }).Count -eq 1) "chain-status json should allow read-only entrypoint_matrix"
    Assert-SelfTest (@($chainRoundTrip.allowed_actions | Where-Object { $_.id -eq "model_pool_launch" -and $_.allowed -eq $false }).Count -eq 1) "chain-status json should block model_pool_launch"
    Assert-SelfTest ($chainRoundTrip.machine_summary.read_only -eq $true) "chain-status machine_summary must be read-only"
    Assert-SelfTest ($chainRoundTrip.machine_summary.any_prompt_allowed -eq $false) "chain-status machine_summary should block prompts"
    Assert-SelfTest ($chainRoundTrip.machine_summary.model_pool_launch_allowed -eq $false) "chain-status machine_summary should block model pool launch"
    Assert-SelfTest ($chainRoundTrip.machine_summary.quality_worker_tcp_reachable -eq $false) "chain-status machine_summary should include quality worker TCP state"
    Assert-SelfTest (@($chainRoundTrip.allowed_action_table | Where-Object { $_.id -eq "model_pool_launch" -and $_.category -eq "launch" -and $_.downstream_launches_process -eq $true -and $_.status_command_sends_prompt -eq $false }).Count -eq 1) "chain-status allowed_action_table should describe model pool launch"
    Assert-SelfTest (@($chainRoundTrip.allowed_action_table | Where-Object { $_.id -eq "diagnose" -and $_.category -eq "read_only" -and $_.status_command_sends_prompt -eq $false }).Count -eq 1) "chain-status allowed_action_table should describe read-only diagnose"
    Assert-SelfTest (@($chainRoundTrip.entrypoint_gates).Count -eq 6) "chain-status should expose six entrypoint gates"
    Assert-SelfTest (@($chainRoundTrip.integration_entrypoints).Count -eq 6) "chain-status should expose six integration entrypoints"
    Assert-SelfTest (@($chainRoundTrip.integration_entrypoints | Where-Object { $_.id -eq "web_lab_prompt" -and $_.surface -eq "web-lab" -and $_.current_allowed -eq $false }).Count -eq 1) "chain-status integration matrix should block Web Lab prompt"
    Assert-SelfTest (@($chainRoundTrip.integration_entrypoints | Where-Object { $_.id -eq "evolution_loop_prompt_round" -and $_.required_context_tokens -eq 262144 -and $_.safe_when_blocked -eq $true }).Count -eq 1) "chain-status integration matrix should block evolution loop safely"
    Assert-SelfTest (@($chainRoundTrip.integration_entrypoints | Where-Object { $_.id -eq "model_pool_launch" -and $_.entrypoint_kind -eq "launch" -and $_.downstream_launches_process -eq $true -and $_.downstream_sends_prompt -eq $false }).Count -eq 1) "chain-status integration matrix should describe model-pool launch"
    $entrypointMatrixFixture = [pscustomobject]@{
        schema_version = $chainRoundTrip.schema_version
        contract_version = $chainRoundTrip.contract_version
        read_only = $true
        sends_prompt = $false
        launches_process = $false
        classification = $chainRoundTrip.classification
        require_action = $chainRoundTrip.require_action
        require_action_allowed = $chainRoundTrip.require_action_allowed
        entrypoints = $chainRoundTrip.integration_entrypoints
    }
    Assert-SelfTest ($entrypointMatrixFixture.read_only -eq $true) "entrypoint-matrix fixture must be read-only"
    Assert-SelfTest ($entrypointMatrixFixture.sends_prompt -eq $false) "entrypoint-matrix fixture must not send prompts"
    Assert-SelfTest ($entrypointMatrixFixture.launches_process -eq $false) "entrypoint-matrix fixture must not launch processes"
    Assert-SelfTest (@($entrypointMatrixFixture.entrypoints).Count -eq 6) "entrypoint-matrix fixture should expose six rows"
    Assert-SelfTest (@($entrypointMatrixFixture.entrypoints | Where-Object { $_.id -eq "model_pool_launch" -and $_.current_allowed -eq $false }).Count -eq 1) "entrypoint-matrix fixture should block model-pool launch"

    $fixtureWorkers = @(
        [pscustomobject]@{
            port = 8686
            role = "quality"
            base_url = "fixture://quality-worker"
            enabled_by_default = $true
            suggested_quant = "Q8"
            launches_process = $false
            requires_quality_gate = $true
            tcp_reachable = $false
            health_ok = $false
            health_elapsed_ms = 0
            health_error = "fixture unavailable"
            service = $null
            model = $null
            context_window = $null
            default_max_tokens = $null
            runtime_backend = $null
            runtime_device = $null
            runtime_accelerator = $null
            gpu_layers = $null
        }
    )
    $poolPublic = New-ModelPoolStatusPublic -Plan $plan -LaunchGate ([pscustomobject]@{
        classification = "quality_worker_down"
        next_step = "do not launch model pool; restore quality worker first"
        launch_allowed = $false
        min_context_tokens = 262144
        context_gate_source = "quality default"
        prompt_gate = $qualityDownStatus.prompt_gate
    }) -Workers $fixtureWorkers
    $poolRoundTrip = $poolPublic | ConvertTo-StatusJson | ConvertFrom-Json
    Assert-ObjectHasProperties $poolRoundTrip @(
        "schema_version",
        "contract_version",
        "summary",
        "blocked_policy",
        "launch_gate",
        "launch_allowed",
        "launch_block_reason",
        "min_context_tokens",
        "context_gate_source",
        "chain_classification",
        "chain_next_step",
        "prompt_gate",
        "capacity",
        "workers"
    ) "pool-status json"
    Assert-SelfTest ($poolRoundTrip.schema_version -eq 1) "pool-status json schema_version mismatch"
    Assert-SelfTest ($poolRoundTrip.contract_version -eq "gemma-chain.v1") "pool-status json contract_version mismatch"
    Assert-SelfTest ($poolRoundTrip.launch_allowed -eq $false) "pool-status json should block launch"
    Assert-SelfTest ($poolRoundTrip.launch_block_reason -eq "quality_worker_down") "pool-status json block reason mismatch"
    Assert-SelfTest ($poolRoundTrip.min_context_tokens -eq 262144) "pool-status json min_context_tokens mismatch"
    Assert-SelfTest ($poolRoundTrip.context_gate_source -eq "quality default") "pool-status json context gate source mismatch"
    Assert-SelfTest (@($poolRoundTrip.workers).Count -eq 1) "pool-status json should include worker rows"
    Assert-SelfTest (@($poolRoundTrip.workers | Where-Object { $_.role -eq "quality" -and $_.launches_process -eq $false }).Count -eq 1) "pool-status json should mark worker probes as non-launching"
    Assert-SelfTest ($poolRoundTrip.capacity.policy -eq "one_quality_plus_small_helpers") "pool-status capacity policy mismatch"
    Assert-SelfTest ($poolRoundTrip.capacity.expansion_allowed -eq $false) "pool-status capacity should block expansion while quality gate is down"
    Assert-SelfTest ($poolRoundTrip.capacity.recommendation -eq "restore_quality_gate_first") "pool-status capacity recommendation should restore quality first"

    $blockedRoute = New-ModelPoolRoutePlanPublic -PoolStatus $poolPublic -Kind "quality"
    $blockedRouteRoundTrip = $blockedRoute | ConvertTo-StatusJson -Depth 20 | ConvertFrom-Json
    Assert-ObjectHasProperties $blockedRouteRoundTrip @(
        "schema_version",
        "contract_version",
        "read_only",
        "launches_process",
        "sends_prompt",
        "task_kind",
        "route_allowed",
        "route_block_reason",
        "selected_role",
        "role_candidates",
        "candidate_workers",
        "quality_gate"
    ) "pool-route-plan blocked json"
    Assert-SelfTest ($blockedRouteRoundTrip.schema_version -eq 1) "pool-route-plan schema_version mismatch"
    Assert-SelfTest ($blockedRouteRoundTrip.contract_version -eq "gemma-chain.v1") "pool-route-plan contract_version mismatch"
    Assert-SelfTest ($blockedRouteRoundTrip.read_only -eq $true) "pool-route-plan must be read-only"
    Assert-SelfTest ($blockedRouteRoundTrip.launches_process -eq $false) "pool-route-plan must not launch workers"
    Assert-SelfTest ($blockedRouteRoundTrip.sends_prompt -eq $false) "pool-route-plan must not send prompts"
    Assert-SelfTest ($blockedRouteRoundTrip.route_allowed -eq $false) "pool-route-plan should block when launch gate is blocked"
    Assert-SelfTest ($blockedRouteRoundTrip.route_block_reason -match "model_pool_launch_blocked") "pool-route-plan should explain launch gate block"

    $routeWorkers = @()
    foreach ($role in @("summary", "router", "quality", "index")) {
        $routeWorkerPlan = @($plan.Ports | Where-Object { $_.Role -eq $role })[0]
        $routeEndpoint = New-FixtureEndpoint "$role-worker" $true $true ([pscustomobject]@{
            service = "fixture"
            model = "$role-model"
            n_ctx = $routeWorkerPlan.DefaultContextTokens
            n_predict = $routeWorkerPlan.DefaultMaxTokens
            backend = "llama.cpp"
            device = "metal"
            metal = $true
            n_gpu_layers = 99
        })
        $routeWorkers += Convert-PoolWorkerPublicStatus -Worker $routeWorkerPlan -Endpoint $routeEndpoint
    }
    $readyPoolPublic = New-ModelPoolStatusPublic -Plan $plan -LaunchGate ([pscustomobject]@{
        classification = "prompt_ready"
        next_step = "model pool launch gate is ready"
        launch_allowed = $true
        min_context_tokens = 262144
        context_gate_source = "quality default"
        prompt_gate = $promptReadyStatus.prompt_gate
    }) -Workers $routeWorkers
    $readyPoolRoundTrip = $readyPoolPublic | ConvertTo-StatusJson -Depth 20 | ConvertFrom-Json
    Assert-SelfTest ($readyPoolRoundTrip.capacity.expansion_allowed -eq $true) "ready pool capacity should allow helper expansion when runtime metadata is healthy"
    Assert-SelfTest ($readyPoolRoundTrip.capacity.metal_worker_count -eq 4) "ready pool capacity should count metal workers"
    Assert-SelfTest ($readyPoolRoundTrip.capacity.unknown_runtime_worker_count -eq 0) "ready pool capacity should not report unknown runtime workers"
    $summaryRoute = New-ModelPoolRoutePlanPublic -PoolStatus $readyPoolPublic -Kind "summary"
    $summaryRouteRoundTrip = $summaryRoute | ConvertTo-StatusJson -Depth 20 | ConvertFrom-Json
    Assert-SelfTest ($summaryRouteRoundTrip.route_allowed -eq $true) "pool-route-plan should allow healthy summary route after launch gate"
    Assert-SelfTest ($summaryRouteRoundTrip.selected_role -eq "summary") "pool-route-plan should select summary worker for summary task"
    Assert-SelfTest (@($summaryRouteRoundTrip.role_candidates | Where-Object { $_ -eq "quality" }).Count -eq 0) "summary route should not fall back to quality by default"
    Assert-SelfTest (@($summaryRouteRoundTrip.candidate_workers | Where-Object { $_.role -eq "quality" }).Count -eq 0) "summary candidate workers should not include quality by default"
    Assert-SelfTest (@($summaryRouteRoundTrip.candidate_workers | Where-Object { $_.role -eq "summary" -and $_.can_accept_low_priority_task -eq $true }).Count -eq 1) "pool-route-plan should mark summary as low-priority capable"
    Assert-SelfTest (@($summaryRouteRoundTrip.candidate_workers | Where-Object { $_.role -eq "summary" -and $_.runtime_backend -eq "llama.cpp" -and $_.runtime_device -eq "metal" -and $_.runtime_accelerator -eq "metal" -and $_.gpu_layers -eq 99 }).Count -eq 1) "pool-route-plan should preserve worker runtime device metadata"
    $routerRoute = New-ModelPoolRoutePlanPublic -PoolStatus $readyPoolPublic -Kind "router"
    $routerRouteRoundTrip = $routerRoute | ConvertTo-StatusJson -Depth 20 | ConvertFrom-Json
    Assert-SelfTest ($routerRouteRoundTrip.route_allowed -eq $true) "pool-route-plan should allow healthy router route after launch gate"
    Assert-SelfTest ($routerRouteRoundTrip.selected_role -eq "router") "pool-route-plan should select router worker for router task"
    Assert-SelfTest (@($routerRouteRoundTrip.role_candidates | Where-Object { $_ -eq "quality" }).Count -eq 0) "router route should not fall back to quality by default"
    Assert-SelfTest (@($routerRouteRoundTrip.candidate_workers | Where-Object { $_.role -eq "router" -and $_.can_accept_low_priority_task -eq $true }).Count -eq 1) "pool-route-plan should mark router as low-priority capable"
    $indexRoute = New-ModelPoolRoutePlanPublic -PoolStatus $readyPoolPublic -Kind "index"
    $indexRouteRoundTrip = $indexRoute | ConvertTo-StatusJson -Depth 20 | ConvertFrom-Json
    Assert-SelfTest ($indexRouteRoundTrip.route_allowed -eq $true) "pool-route-plan should allow healthy index route after launch gate"
    Assert-SelfTest ($indexRouteRoundTrip.selected_role -eq "index") "pool-route-plan should select index worker for index task"
    Assert-SelfTest (@($indexRouteRoundTrip.role_candidates | Where-Object { $_ -eq "quality" }).Count -eq 0) "index route should not fall back to quality by default"

    $loopFixture = [pscustomobject]@{
        prompt_gate = $qualityDownStatus.prompt_gate
        evolution_dir = "fixture://target/evolution"
        ledger = [pscustomobject]@{
            path = "fixture://ledger.jsonl"
            last_write = "2026-06-13T00:00:00.0000000+08:00"
            age_seconds = 42
            bytes = 256
            last_record = [pscustomobject]@{
                round = 15
                case = "fixture-loop-case"
                success = $true
                error = ""
                runtime_tokens = 512
                runtime_model = "fixture-model"
                elapsed_ms = 151799
                business_cycle_passed = $true
                feedback_applied = 4
                validation_checked = $true
                validation_passed = $true
                self_improve_passed = $true
            }
        }
        daemon_out = $null
        daemon_err = $null
        classification = "quality_worker_gate_blocked"
        action = "restore quality worker/tunnel ownership first, then rerun diagnose and prompt-gate."
    }

    $bundlePublic = New-StatusBundlePublic -Chain $chainRoundTrip -Pool $poolRoundTrip -Recovery ([pscustomobject]@{
        status = [pscustomobject]@{
            classification = "quality_worker_down"
            require_action = "model_pool_launch"
            require_action_allowed = $false
            quality_worker_url = "http://127.0.0.1:8686"
            quality_worker_tcp_reachable = $false
            quality_worker_health_ok = $false
            backend_url = "http://127.0.0.1:7979"
            backend_health_ok = $true
            web_lab_url = "http://127.0.0.1:8789"
            web_lab_health_ok = $true
            prompt_ready = $false
            block_reason = "gemma_runtime_reachable is not true"
            engine_busy = $false
            gemma_runtime_reachable = $false
            readiness_ok = $false
            safe_device_ok = $true
        }
        blocked_prompt_actions = @("smoke", "model-pool launch")
        recovery_steps = @("restore quality worker first")
        post_recovery_validation = Get-PostRecoveryValidationCommands
        post_recovery_release_sequence = Get-PostRecoveryReleaseSequence
        safety_notes = @("fixture safety note")
    }) -Loop $loopFixture
    $bundleRoundTrip = $bundlePublic | ConvertTo-StatusJson -Depth 20 | ConvertFrom-Json
    Assert-ObjectHasProperties $bundleRoundTrip @(
        "read_only",
        "schema_version",
        "contract_version",
        "require_action",
        "require_action_allowed",
        "classification",
        "prompt_ready",
        "loop_classification",
        "loop_action",
        "model_pool_launch_allowed",
        "machine_summary",
        "allowed_action_table",
        "integration_entrypoints",
        "wrapper_audit",
        "prompt_policy",
        "chain",
        "loop",
        "pool",
        "recovery",
        "safe_next_commands",
        "safe_next_command_actions",
        "safety_notes"
    ) "status-bundle json"
    Assert-SelfTest ($bundleRoundTrip.read_only -eq $true) "status-bundle json must mark read_only"
    Assert-SelfTest ($bundleRoundTrip.schema_version -eq 1) "status-bundle json schema_version mismatch"
    Assert-SelfTest ($bundleRoundTrip.contract_version -eq "gemma-chain.v1") "status-bundle json contract_version mismatch"
    Assert-SelfTest ($bundleRoundTrip.machine_summary.schema_version -eq 1) "status-bundle machine_summary schema_version mismatch"
    Assert-SelfTest ($bundleRoundTrip.machine_summary.contract_version -eq "gemma-chain.v1") "status-bundle machine_summary contract_version mismatch"
    Assert-SelfTest ($bundleRoundTrip.machine_summary.wrapper_decision -eq "read_only_only") "status-bundle machine_summary wrapper decision mismatch"
    Assert-SelfTest ($bundleRoundTrip.machine_summary.fail_closed_required -eq $true) "status-bundle machine_summary should require fail closed while blocked"
    Assert-SelfTest ($bundleRoundTrip.wrapper_audit.contract_supported -eq $true) "status-bundle wrapper audit should support current contract"
    Assert-SelfTest ($bundleRoundTrip.wrapper_audit.unknown_contract_policy -eq "fail_closed") "status-bundle wrapper audit unknown contract policy mismatch"
    Assert-SelfTest ($bundleRoundTrip.wrapper_audit.safe_next_read_only_subset_required -eq $true) "status-bundle wrapper audit should require read-only subset while blocked"
    Assert-SelfTest ($bundleRoundTrip.wrapper_audit.safe_next_actions_subset_of_allowed_read_only -eq $true) "status-bundle wrapper audit should prove safe next subset"
    Assert-SelfTest ($bundleRoundTrip.wrapper_audit.wrapper_decision -eq "read_only_only") "status-bundle wrapper audit decision mismatch"
    Assert-SelfTest ($bundleRoundTrip.wrapper_audit.fail_closed_required -eq $true) "status-bundle wrapper audit should require fail closed while blocked"
    Assert-SelfTest ($bundleRoundTrip.prompt_policy.wrapper_audit.wrapper_decision -eq "read_only_only") "status-bundle prompt_policy should repeat wrapper audit"
    Assert-SelfTest (@($bundleRoundTrip.integration_entrypoints).Count -eq 6) "status-bundle should expose six integration entrypoints"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.integration_entrypoints).Count -eq 6) "status-bundle prompt_policy should repeat integration entrypoints"
    Assert-SelfTest (@($bundleRoundTrip.integration_entrypoints | Where-Object { $_.id -eq "web_lab_prompt" -and $_.surface -eq "web-lab" -and $_.current_allowed -eq $false }).Count -eq 1) "status-bundle integration matrix should block Web Lab prompt"
    Assert-SelfTest (@($bundleRoundTrip.integration_entrypoints | Where-Object { $_.id -eq "forge_cli_prompt" -and $_.surface -eq "forge-cli" -and $_.downstream_sends_prompt -eq $true }).Count -eq 1) "status-bundle integration matrix should include Forge CLI"
    Assert-SelfTest (@($bundleRoundTrip.integration_entrypoints | Where-Object { $_.id -eq "evolution_loop_prompt_round" -and $_.required_context_tokens -eq 262144 -and $_.safe_when_blocked -eq $true }).Count -eq 1) "status-bundle integration matrix should block evolution loop safely"
    Assert-SelfTest (@($bundleRoundTrip.integration_entrypoints | Where-Object { $_.id -eq "model_pool_launch" -and $_.entrypoint_kind -eq "launch" -and $_.downstream_launches_process -eq $true -and $_.downstream_sends_prompt -eq $false }).Count -eq 1) "status-bundle integration matrix should describe model-pool launch"
    Assert-SelfTest ($bundleRoundTrip.classification -eq "quality_worker_down") "status-bundle json classification mismatch"
    Assert-SelfTest ($bundleRoundTrip.require_action_allowed -eq $false) "status-bundle json should block model_pool_launch"
    Assert-SelfTest ($bundleRoundTrip.recovery.handoff.classification -eq "quality_worker_down") "status-bundle recovery handoff classification mismatch"
    Assert-SelfTest ($bundleRoundTrip.recovery.handoff.prompt_ready -eq $false) "status-bundle recovery handoff should preserve prompt block"
    Assert-SelfTest ($bundleRoundTrip.recovery.handoff.quality_worker_tcp_reachable -eq $false) "status-bundle recovery handoff should preserve quality worker reachability"
    Assert-SelfTest (@($bundleRoundTrip.recovery.post_recovery_release_sequence | Where-Object { $_.id -eq "model_pool_launch_gate" -and $_.required_context_tokens -eq 262144 }).Count -eq 1) "status-bundle recovery should expose model-pool release gate"
    Assert-SelfTest (@($bundleRoundTrip.recovery.post_recovery_release_sequence | Where-Object { $_.read_only -eq $true -and $_.sends_prompt -eq $true }).Count -eq 0) "status-bundle recovery read-only release rows must not send prompts"
    Assert-SelfTest ($bundleRoundTrip.loop_classification -eq "quality_worker_gate_blocked") "status-bundle json loop classification mismatch"
    Assert-SelfTest ($bundleRoundTrip.loop.launch_gate -eq $null) "status-bundle loop summary should not invent launch fields"
    Assert-SelfTest ($bundleRoundTrip.prompt_policy.any_prompt_allowed -eq $false) "status-bundle prompt_policy should block all prompts when quality worker is down"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.blocked_prompt_actions | Where-Object { $_ -eq "model-pool launch" }).Count -eq 1) "status-bundle prompt_policy should list blocked model-pool launch"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.allowed_read_only_actions | Where-Object { $_ -eq "diagnose" }).Count -eq 1) "status-bundle prompt_policy should list read-only diagnose"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.allowed_read_only_actions | Where-Object { $_ -eq "entrypoint-matrix" }).Count -eq 1) "status-bundle prompt_policy should list read-only entrypoint-matrix"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.allowed_read_only_actions | Where-Object { $_ -eq "pool-manifest" }).Count -eq 1) "status-bundle prompt_policy should list read-only pool-manifest"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.allowed_read_only_actions | Where-Object { $_ -eq "contract-audit" }).Count -eq 1) "status-bundle prompt_policy should list read-only contract-audit"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.allowed_read_only_actions | Where-Object { $_ -eq "wrapper-manifest" }).Count -eq 1) "status-bundle prompt_policy should list read-only wrapper-manifest"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.allowed_read_only_actions | Where-Object { $_ -eq "contract-fixture" }).Count -eq 1) "status-bundle prompt_policy should list read-only contract-fixture"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.allowed_read_only_actions | Where-Object { $_ -eq "handoff-report" }).Count -eq 1) "status-bundle prompt_policy should list read-only handoff-report"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.allowed_read_only_actions | Where-Object { $_ -eq "secret-scan" }).Count -eq 1) "status-bundle prompt_policy should list read-only secret-scan"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.allowed_prompt_actions).Count -eq 0) "status-bundle prompt_policy should not allow prompt actions while blocked"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.entrypoint_gates).Count -eq 6) "status-bundle prompt_policy should list all prompt entrypoint gates"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.entrypoint_gates | Where-Object { $_.id -eq "web_lab_prompt" -and $_.gate_command -match "web_lab_prompt" }).Count -eq 1) "status-bundle prompt_policy should include Web Lab gate"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.entrypoint_gates | Where-Object { $_.id -eq "evolution_loop_prompt_round" -and $_.gate_command -match "MinContextTokens 262144" }).Count -eq 1) "status-bundle prompt_policy should context-gate evolution loop"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.entrypoint_gates | Where-Object { $_.id -eq "model_pool_launch" -and $_.sends_prompt_after_gate -eq $false }).Count -eq 1) "status-bundle prompt_policy should mark model pool launch as non-prompt process gate"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.entrypoint_gates | Where-Object { $_.id -eq "smoke" -and $_.allowed -eq $false -and $_.reason -eq "quality_worker_down" }).Count -eq 1) "status-bundle prompt_policy should include smoke gate block reason"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.entrypoint_gates | Where-Object { $_.id -eq "model_pool_launch" -and $_.allowed -eq $false -and $_.reason -eq "quality_worker_down" }).Count -eq 1) "status-bundle prompt_policy should include model pool gate block reason"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.entrypoint_gates | Where-Object { $_.gate_read_only -eq $true -and $_.blocked_exit_code -eq 2 }).Count -eq 6) "status-bundle prompt_policy gates should be read-only and use blocked exit 2"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.entrypoint_gates | Where-Object { $_.id -eq "model_pool_launch" -and $_.required_context_tokens -eq 262144 }).Count -eq 1) "status-bundle prompt_policy should expose model pool context requirement"
    Assert-SelfTest (@($bundleRoundTrip.prompt_policy.entrypoint_gates | Where-Object { $_.id -eq "smoke" -and $_.required_context_tokens -eq 0 }).Count -eq 1) "status-bundle prompt_policy should expose smoke context default"
    Assert-SelfTest ($bundleRoundTrip.model_pool_launch_allowed -eq $false) "status-bundle json should block pool launch"
    Assert-SelfTest ($bundleRoundTrip.machine_summary.read_only -eq $true) "status-bundle machine_summary must be read-only"
    Assert-SelfTest ($bundleRoundTrip.machine_summary.any_prompt_allowed -eq $false) "status-bundle machine_summary should block all prompts"
    Assert-SelfTest ($bundleRoundTrip.machine_summary.model_pool_launch_allowed -eq $false) "status-bundle machine_summary should block model pool launch"
    Assert-SelfTest (@($bundleRoundTrip.allowed_action_table | Where-Object { $_.id -eq "model_pool_launch" -and $_.allowed -eq $false }).Count -eq 1) "status-bundle allowed_action_table should block model_pool_launch"
    Assert-SelfTest (@($bundleRoundTrip.safe_next_commands | Where-Object { $_ -match "\bloop-status\b" }).Count -eq 1) "status-bundle should include read-only loop-status in safe next commands"
    Assert-SelfTest (@($bundleRoundTrip.safe_next_command_actions | Where-Object { $_ -eq "entrypoint-matrix" }).Count -eq 1) "status-bundle should include read-only entrypoint-matrix in safe next command actions"
    Assert-SelfTest (@($bundleRoundTrip.safe_next_command_actions | Where-Object { $_ -eq "pool-manifest" }).Count -eq 1) "status-bundle should include read-only pool-manifest in safe next command actions"
    Assert-SelfTest (@($bundleRoundTrip.safe_next_command_actions | Where-Object { $_ -eq "contract-audit" }).Count -eq 1) "status-bundle should include read-only contract-audit in safe next command actions"
    Assert-SelfTest (@($bundleRoundTrip.safe_next_command_actions | Where-Object { $_ -eq "wrapper-manifest" }).Count -eq 1) "status-bundle should include read-only wrapper-manifest in safe next command actions"
    Assert-SelfTest (@($bundleRoundTrip.safe_next_command_actions | Where-Object { $_ -eq "contract-fixture" }).Count -eq 1) "status-bundle should include read-only contract-fixture in safe next command actions"
    Assert-SelfTest (@($bundleRoundTrip.safe_next_command_actions | Where-Object { $_ -eq "handoff-report" }).Count -eq 1) "status-bundle should include read-only handoff-report in safe next command actions"
    Assert-SelfTest (@($bundleRoundTrip.safe_next_command_actions | Where-Object { $_ -eq "secret-scan" }).Count -eq 1) "status-bundle should include read-only secret-scan in safe next command actions"
    Assert-SelfTest (@($bundleRoundTrip.safe_next_commands | Where-Object { $_ -match "\bsmoke\b" }).Count -eq 0) "status-bundle should not suggest smoke when prompt is blocked"
    foreach ($action in @($bundleRoundTrip.safe_next_command_actions)) {
        Assert-SelfTest (@($bundleRoundTrip.prompt_policy.allowed_read_only_actions | Where-Object { $_ -eq $action }).Count -ge 1) "blocked status-bundle safe command action '$action' must be allowed read-only"
    }

    Write-Section "contract audit selftest"
    $contractAudit = New-ContractAuditPublic -Bundle $bundleRoundTrip
    $contractAuditRoundTrip = $contractAudit | ConvertTo-StatusJson -Depth 18 | ConvertFrom-Json
    Assert-ObjectHasProperties $contractAuditRoundTrip @(
        "schema_version",
        "contract_version",
        "read_only",
        "sends_prompt",
        "launches_process",
        "audit_passed",
        "classification",
        "require_action",
        "require_action_allowed",
        "prompt_ready",
        "quality_worker_reachable",
        "model_pool_launch_allowed",
        "wrapper_decision",
        "checks",
        "next_gate"
    ) "contract-audit json"
    Assert-SelfTest ($contractAuditRoundTrip.schema_version -eq 1) "contract-audit schema_version mismatch"
    Assert-SelfTest ($contractAuditRoundTrip.contract_version -eq "gemma-chain.v1") "contract-audit contract_version mismatch"
    Assert-SelfTest ($contractAuditRoundTrip.read_only -eq $true) "contract-audit must be read-only"
    Assert-SelfTest ($contractAuditRoundTrip.sends_prompt -eq $false) "contract-audit must not send prompts"
    Assert-SelfTest ($contractAuditRoundTrip.launches_process -eq $false) "contract-audit must not launch processes"
    Assert-SelfTest ($contractAuditRoundTrip.audit_passed -eq $true) "contract-audit should pass for the fixture bundle"
    Assert-SelfTest (@($contractAuditRoundTrip.failed_check_ids).Count -eq 0) "contract-audit fixture should not have failed checks"
    Assert-SelfTest (@($contractAuditRoundTrip.checks).Count -ge 8) "contract-audit should expose check rows"
    Assert-SelfTest (@($contractAuditRoundTrip.checks | Where-Object { $_.id -eq "quality_down_blocks_model_pool" -and $_.passed -eq $true }).Count -eq 1) "contract-audit should enforce quality down blocks model pool"
    Assert-SelfTest (@($contractAuditRoundTrip.checks | Where-Object { $_.id -eq "release_sequence_pool_gate" -and $_.passed -eq $true }).Count -eq 1) "contract-audit should enforce pool release gate"

    Write-Section "wrapper manifest selftest"
    $wrapperManifest = New-WrapperManifestPublic -Bundle $bundleRoundTrip -Audit $contractAuditRoundTrip
    $wrapperManifestRoundTrip = $wrapperManifest | ConvertTo-StatusJson -Depth 18 | ConvertFrom-Json
    Assert-ObjectHasProperties $wrapperManifestRoundTrip @(
        "schema_version",
        "contract_version",
        "read_only",
        "sends_prompt",
        "launches_process",
        "classification",
        "require_action",
        "require_action_allowed",
        "audit_passed",
        "unknown_contract_policy",
        "status_commands",
        "required_top_level_fields",
        "consumption_rules",
        "entrypoints",
        "safe_next_command_actions"
    ) "wrapper-manifest json"
    Assert-SelfTest ($wrapperManifestRoundTrip.schema_version -eq 1) "wrapper-manifest schema_version mismatch"
    Assert-SelfTest ($wrapperManifestRoundTrip.contract_version -eq "gemma-chain.v1") "wrapper-manifest contract_version mismatch"
    Assert-SelfTest ($wrapperManifestRoundTrip.read_only -eq $true) "wrapper-manifest must be read-only"
    Assert-SelfTest ($wrapperManifestRoundTrip.sends_prompt -eq $false) "wrapper-manifest must not send prompts"
    Assert-SelfTest ($wrapperManifestRoundTrip.launches_process -eq $false) "wrapper-manifest must not launch processes"
    Assert-SelfTest ($wrapperManifestRoundTrip.audit_passed -eq $true) "wrapper-manifest should embed passing contract audit"
    Assert-SelfTest ($wrapperManifestRoundTrip.require_action -eq "model_pool_launch") "wrapper-manifest should preserve required action"
    Assert-SelfTest ($wrapperManifestRoundTrip.require_action_allowed -eq $false) "wrapper-manifest should preserve required action block"
    Assert-SelfTest ($wrapperManifestRoundTrip.unknown_contract_policy -eq "fail_closed") "wrapper-manifest unknown contract policy mismatch"
    Assert-SelfTest (@($wrapperManifestRoundTrip.entrypoints).Count -eq 6) "wrapper-manifest should expose six entrypoints"
    Assert-SelfTest (@($wrapperManifestRoundTrip.required_top_level_fields | Where-Object { $_ -eq "contract_version" }).Count -eq 1) "wrapper-manifest should require contract_version"
    Assert-SelfTest (@($wrapperManifestRoundTrip.consumption_rules | Where-Object { $_ -match "gate_command" }).Count -ge 1) "wrapper-manifest should require gate_command before downstream action"
    Assert-SelfTest (@($wrapperManifestRoundTrip.entrypoints | Where-Object { $_.id -eq "model_pool_launch" -and $_.entrypoint_kind -eq "launch" -and $_.downstream_launches_process -eq $true -and $_.downstream_sends_prompt -eq $false -and $_.gate_command -match "MinContextTokens 262144" }).Count -eq 1) "wrapper-manifest should describe model-pool launch gate"
    Assert-SelfTest (@($wrapperManifestRoundTrip.entrypoints | Where-Object { $_.gate_read_only -eq $true -and $_.blocked_exit_code -eq 2 }).Count -eq 6) "wrapper-manifest entrypoints should use read-only gates"

    Write-Section "contract fixture selftest"
    $contractFixture = Get-ContractFixture
    $contractFixtureRoundTrip = $contractFixture | ConvertTo-StatusJson -Depth 22 | ConvertFrom-Json
    Assert-ObjectHasProperties $contractFixtureRoundTrip @(
        "schema_version",
        "contract_version",
        "read_only",
        "sends_prompt",
        "launches_process",
        "offline_fixture",
        "touches_network",
        "fixture_name",
        "chain_status",
        "entrypoint_matrix",
        "status_bundle",
        "contract_audit",
        "wrapper_manifest"
    ) "contract-fixture json"
    Assert-SelfTest ($contractFixtureRoundTrip.schema_version -eq 1) "contract-fixture schema_version mismatch"
    Assert-SelfTest ($contractFixtureRoundTrip.contract_version -eq "gemma-chain.v1") "contract-fixture contract_version mismatch"
    Assert-SelfTest ($contractFixtureRoundTrip.read_only -eq $true) "contract-fixture must be read-only"
    Assert-SelfTest ($contractFixtureRoundTrip.sends_prompt -eq $false) "contract-fixture must not send prompts"
    Assert-SelfTest ($contractFixtureRoundTrip.launches_process -eq $false) "contract-fixture must not launch processes"
    Assert-SelfTest ($contractFixtureRoundTrip.offline_fixture -eq $true) "contract-fixture must mark offline_fixture"
    Assert-SelfTest ($contractFixtureRoundTrip.touches_network -eq $false) "contract-fixture must not touch network"
    Assert-SelfTest ($contractFixtureRoundTrip.chain_status.classification -eq "quality_worker_down") "contract-fixture should model quality_worker_down"
    Assert-SelfTest ($contractFixtureRoundTrip.status_bundle.model_pool_launch_allowed -eq $false) "contract-fixture should block model pool launch"
    Assert-SelfTest ($contractFixtureRoundTrip.contract_audit.audit_passed -eq $true) "contract-fixture embedded audit should pass"
    Assert-SelfTest (@($contractFixtureRoundTrip.wrapper_manifest.entrypoints).Count -eq 6) "contract-fixture wrapper manifest should expose six entrypoints"
    Assert-SelfTest (@($contractFixtureRoundTrip.wrapper_manifest.entrypoints | Where-Object { $_.id -eq "model_pool_launch" -and $_.current_allowed -eq $false -and $_.blocked_by -eq "quality_worker_down" }).Count -eq 1) "contract-fixture should block model_pool_launch"

    Write-Section "handoff report selftest"
    $handoffReport = New-HandoffReportPublic -Bundle $bundleRoundTrip -Audit $contractAuditRoundTrip
    $handoffReportRoundTrip = $handoffReport | ConvertTo-StatusJson -Depth 18 | ConvertFrom-Json
    Assert-ObjectHasProperties $handoffReportRoundTrip @(
        "schema_version",
        "contract_version",
        "read_only",
        "sends_prompt",
        "launches_process",
        "classification",
        "current_state",
        "operator_recommendation",
        "require_action",
        "require_action_allowed",
        "prompt_ready",
        "engine_busy",
        "quality_worker_reachable",
        "model_pool_launch_allowed",
        "blocked_prompt_actions",
        "entrypoint_gates",
        "apple_silicon_pool_summary",
        "safe_next_commands"
    ) "handoff-report json"
    Assert-SelfTest ($handoffReportRoundTrip.schema_version -eq 1) "handoff-report schema_version mismatch"
    Assert-SelfTest ($handoffReportRoundTrip.contract_version -eq "gemma-chain.v1") "handoff-report contract_version mismatch"
    Assert-SelfTest ($handoffReportRoundTrip.read_only -eq $true) "handoff-report must be read-only"
    Assert-SelfTest ($handoffReportRoundTrip.sends_prompt -eq $false) "handoff-report must not send prompts"
    Assert-SelfTest ($handoffReportRoundTrip.launches_process -eq $false) "handoff-report must not launch processes"
    Assert-SelfTest ($handoffReportRoundTrip.classification -eq "quality_worker_down") "handoff-report classification mismatch"
    Assert-SelfTest ($handoffReportRoundTrip.require_action_allowed -eq $false) "handoff-report should preserve blocked require action"
    Assert-SelfTest (@($handoffReportRoundTrip.blocked_prompt_actions | Where-Object { $_ -eq "model-pool launch" }).Count -eq 1) "handoff-report should list model-pool launch as blocked"
    Assert-SelfTest ($handoffReportRoundTrip.apple_silicon_pool_summary.current_decision -match "blocked") "handoff-report should summarize blocked pool decision"

    Write-Section "secret scan selftest"
    Assert-SelfTest ((Test-SecretScanAllowlistedLine "token=sk-exampleSECRET1234567890") -eq $true) "secret-scan should allowlist the redaction fixture"
    $authorizationObjectFixture = '    author' + 'ization = [pscustomobject]@{'
    $authorizationSummaryFixture = 'Write-Host "author' + 'ization: daemon=False launch=False prompt=False ssh=False reason=readonly"'
    $bearerAuthorizationFixture = 'Author' + 'ization: Bear' + 'er real-looking-value'
    Assert-SelfTest ((Test-SecretScanAllowlistedLine $authorizationObjectFixture) -eq $true) "secret-scan should allowlist structured authorization objects"
    Assert-SelfTest ((Test-SecretScanAllowlistedLine $authorizationSummaryFixture) -eq $true) "secret-scan should allowlist authorization status summaries"
    Assert-SelfTest ((Test-SecretScanAllowlistedLine $bearerAuthorizationFixture) -eq $false) "secret-scan should not allowlist bearer-scheme headers"
    $passwordFixture = "pass" + "word=real-looking-value"
    Assert-SelfTest ((Test-SecretScanAllowlistedLine $passwordFixture) -eq $false) "secret-scan should not allowlist password assignments"
    $secretScan = Get-SecretScan
    $secretScanRoundTrip = $secretScan | ConvertTo-StatusJson -Depth 12 | ConvertFrom-Json
    Assert-ObjectHasProperties $secretScanRoundTrip @(
        "schema_version",
        "contract_version",
        "read_only",
        "sends_prompt",
        "launches_process",
        "file_count",
        "rule_count",
        "finding_count",
        "passed",
        "findings"
    ) "secret-scan json"
    Assert-SelfTest ($secretScanRoundTrip.schema_version -eq 1) "secret-scan schema_version mismatch"
    Assert-SelfTest ($secretScanRoundTrip.contract_version -eq "gemma-chain.v1") "secret-scan contract_version mismatch"
    Assert-SelfTest ($secretScanRoundTrip.read_only -eq $true) "secret-scan must be read-only"
    Assert-SelfTest ($secretScanRoundTrip.sends_prompt -eq $false) "secret-scan must not send prompts"
    Assert-SelfTest ($secretScanRoundTrip.launches_process -eq $false) "secret-scan must not launch processes"
    Assert-SelfTest ($secretScanRoundTrip.passed -eq $true) "secret-scan should pass for current Gemma chain artifacts"
}

function Get-AppleSiliconPoolPlan {
    $qualityGateCommand = ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked"
    $readOnlyValidation = @(
        ".\tools\gemma-chain\gemma-chain.cmd selftest",
        ".\tools\smartsteam-forge\test-remote-model-pool-guards.cmd",
        ".\tools\gemma-chain\gemma-chain.cmd prompt-gate -MinContextTokens 262144 -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd diagnose",
        ".\tools\gemma-chain\gemma-chain.cmd pool-plan -JsonPlan",
        ".\tools\gemma-chain\gemma-chain.cmd pool-manifest -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind summary -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind router -JsonStatus",
        ".\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind index -JsonStatus",
        $qualityGateCommand
    )
    $promptValidation = @(
        ".\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -JsonStatus -FailIfBlocked",
        ".\tools\gemma-chain\gemma-chain.cmd smoke"
    )
    return [pscustomobject]@{
        SchemaVersion = $script:GemmaChainSchemaVersion
        ContractVersion = $script:GemmaChainContractVersion
        Summary = "Prefer one 12B high-quality worker plus lightweight summary/router/review/index/test-gate helpers. Do not default to multiple 12B Q8 instances on one Apple Silicon host."
        BlockedPolicy = "If 127.0.0.1:8686 quality worker is unreachable or readiness is false, model-pool launch is blocked. Run planning and read-only status only."
        QualityGateCommand = $qualityGateCommand
        Ports = @(
            [pscustomobject]@{
                Port = 8686
                Role = "quality"
                ModelClass = "Gemma 12B Q8 or best available local quality model"
                SuggestedQuant = "Q8 or best available quality quant"
                EnabledByDefault = $true
                LaunchesProcess = $false
                RequiresQualityGate = $true
                DefaultContextTokens = 262144
                DefaultMaxTokens = 262144
                UseFor = "architecture decisions, code generation, final synthesis, difficult debugging"
                Backpressure = "single-flight by default; if busy, queue quality work and route only cheap helper tasks to small workers"
            },
            [pscustomobject]@{
                Port = 8687
                Role = "summary"
                ModelClass = "small Gemma or low-quant local model"
                SuggestedQuant = "Q4 or Q5"
                EnabledByDefault = $true
                LaunchesProcess = $false
                RequiresQualityGate = $true
                DefaultContextTokens = 8192
                DefaultMaxTokens = 768
                UseFor = "log summarization, runbook condensation, ledger summaries"
                Backpressure = "parallel low-cost worker; drop or retry summary work before delaying quality work"
            },
            [pscustomobject]@{
                Port = 8688
                Role = "review"
                ModelClass = "small Gemma or low-quant local model"
                SuggestedQuant = "Q4 or Q5"
                EnabledByDefault = $true
                LaunchesProcess = $false
                RequiresQualityGate = $true
                DefaultContextTokens = 8192
                DefaultMaxTokens = 1536
                UseFor = "patch review, risk list, doc critique, quick second opinion"
                Backpressure = "bounded concurrency; never preempt the quality worker"
            },
            [pscustomobject]@{
                Port = 8689
                Role = "router"
                ModelClass = "FunctionGemma 270M or small function-calling/router model"
                SuggestedQuant = "Q4"
                EnabledByDefault = $true
                LaunchesProcess = $false
                RequiresQualityGate = $true
                DefaultContextTokens = 4096
                DefaultMaxTokens = 512
                UseFor = "intent routing, request preflight, tool-call/function-call argument shaping"
                Backpressure = "fast low-priority worker; keep outputs short and structured"
            },
            [pscustomobject]@{
                Port = 8688
                Role = "test-gate"
                ModelClass = "small Gemma or low-quant local model"
                SuggestedQuant = "Q4"
                EnabledByDefault = $true
                LaunchesProcess = $false
                RequiresQualityGate = $true
                DefaultContextTokens = 4096
                DefaultMaxTokens = 1536
                UseFor = "classify test output, suggest next validation command, failure triage"
                Backpressure = "optional worker; local deterministic tests remain source of truth"
            },
            [pscustomobject]@{
                Port = 8690
                Role = "index"
                ModelClass = "small Gemma, embedding-capable helper, or low-quant local index model"
                SuggestedQuant = "Q4"
                EnabledByDefault = $true
                LaunchesProcess = $false
                RequiresQualityGate = $true
                DefaultContextTokens = 4096
                DefaultMaxTokens = 512
                UseFor = "repository map summaries, symbol/file index hints, retrieval prefiltering"
                Backpressure = "low-priority helper; skip stale indexing work before delaying quality work"
            }
        )
        Routing = @(
            "Use 12B for tasks that change code, architecture, or final decisions.",
            "Use summary/router/review/index/test-gate workers for cheap parallel analysis, validation hints, and repository indexing.",
            "Low-priority helper routes do not fall back to the quality worker by default.",
            "If the 12B worker is busy, do not start another 12B Q8 by default; queue the high-quality task or reduce load.",
            "Do not send secrets to any worker. Gemini-style cloud adapters require separate credential handling outside this repo."
        )
        Risks = @(
            "Multiple 12B Q8 workers can overcommit unified memory, increase swap pressure, and slow every active request.",
            "Concurrent GPU-heavy workers can reduce tokens per second enough that total throughput falls.",
            "Index helpers should keep small context and output budgets so background indexing does not compete with the quality worker.",
            "Small-model reviewers may miss correctness issues; use them as filters, not final arbiters.",
            "Cloud Gemini adapters can improve throughput but must be isolated from local secret-free diagnostics."
        )
        CapacityPolicy = [pscustomobject]@{
            policy = "one_quality_plus_small_helpers"
            target_host = "apple_silicon"
            avoid_extra_12b = $true
            max_quality_12b_workers = 1
            quality_role = "quality"
            quality_required_context_tokens = 262144
            helper_roles = @("summary", "router", "review", "index", "test-gate")
            helper_context_tokens_total = 28672
            helper_default_max_tokens_total = 4864
            helper_model_size_policy = "small_or_low_quant_only"
            large_helper_model_guard = "start-remote-gemma-forge CheckOnly rejects helper models that match the quality model path or look 12B+ unless -AllowLargePoolWorkerModels is set"
            guard_validation_command = ".\tools\smartsteam-forge\test-remote-model-pool-guards.cmd"
            recommended_launch_order = @("quality", "summary", "router", "review", "index", "test-gate")
            expansion_gate = "quality worker must be reachable, prompt-ready, context>=262144, and Metal/GPU accelerated before helper expansion"
            next_step_when_quality_ready = "run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls"
        }
        Validation = $readOnlyValidation
        PromptValidationAfterGate = $promptValidation
    }
}

function Show-PoolPlan {
    $plan = Get-AppleSiliconPoolPlan
    if ($JsonPlan) {
        Write-Host ($plan | ConvertTo-StatusJson)
        return
    }

    Write-Section "apple silicon multi-model pool"
    Write-Host $plan.Summary
    Write-Host $plan.BlockedPolicy
    Write-Host "launch_gate=$($plan.QualityGateCommand)"
    Write-Host ""
    Write-Host "Recommended ports and roles:"
    foreach ($worker in $plan.Ports) {
        Write-Host "  $($worker.Port) role=$($worker.Role) context=$($worker.DefaultContextTokens) max_tokens=$($worker.DefaultMaxTokens) enabled_default=$($worker.EnabledByDefault)"
        Write-Host "      model=$($worker.ModelClass)"
        Write-Host "      quant=$($worker.SuggestedQuant) launches_process=$($worker.LaunchesProcess) requires_quality_gate=$($worker.RequiresQualityGate)"
        Write-Host "      use_for=$($worker.UseFor)"
        Write-Host "      backpressure=$($worker.Backpressure)"
    }

    Write-Section "routing policy"
    foreach ($item in $plan.Routing) {
        Write-Host "  - $item"
    }

    Write-Section "risks"
    foreach ($item in $plan.Risks) {
        Write-Host "  - $item"
    }

    Write-Section "validation commands"
    foreach ($item in $plan.Validation) {
        Write-Host "  $item"
    }

    Write-Section "prompt validation after gate"
    foreach ($item in $plan.PromptValidationAfterGate) {
        Write-Host "  $item"
    }
}

function New-ModelPoolManifestAdvice {
    param($Plan)

    $qualityWorkers = @($Plan.Ports | Where-Object { $_.Role -eq "quality" })
    $helperWorkers = @($Plan.Ports | Where-Object { $_.Role -ne "quality" })
    $helperTargetWorkerCount = @($Plan.CapacityPolicy.helper_roles).Count
    $helperVisibleRoles = @($helperWorkers | ForEach-Object { $_.Role })
    $allHelpersVisible = $true
    foreach ($role in @($Plan.CapacityPolicy.helper_roles)) {
        if ($role -notin $helperVisibleRoles) {
            $allHelpersVisible = $false
            break
        }
    }
    $hasSummary = "summary" -in $helperVisibleRoles
    $hasPartialHelpers = $hasSummary -and @($helperVisibleRoles | Where-Object { $_ -ne "summary" }).Count -gt 0
    $extraQualityDetected = $qualityWorkers.Count -gt $Plan.CapacityPolicy.max_quality_12b_workers
    $safeToEnablePoolWorkers = -not $extraQualityDetected
    $nextStep = if ($extraQualityDetected) {
        "stop_extra_quality_12b_workers_keep_one_quality_plus_helpers"
    } elseif ($allHelpersVisible) {
        "run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls"
    } elseif ($hasPartialHelpers) {
        "add_remaining_helper_roles_one_at_a_time"
    } elseif ($hasSummary) {
        "add_review_or_index_after_short_smoke"
    } else {
        "add_summary_worker_first"
    }
    $reason = if ($extraQualityDetected) {
        "extra_quality_12b_wastes_shared_apple_memory"
    } elseif ($allHelpersVisible) {
        "full_helper_pool_visible"
    } elseif ($hasPartialHelpers) {
        "partial_helper_pool_visible"
    } elseif ($hasSummary) {
        "summary_worker_visible"
    } else {
        "quality_chain_ready_no_helpers_visible"
    }

    return [pscustomobject]@{
        decision_source = "model-pool-advice-core"
        policy = "one_quality_12b_plus_small_helpers"
        safe_to_enable_pool_workers = $safeToEnablePoolWorkers
        next_step = $nextStep
        reason = $reason
        kind = if ($safeToEnablePoolWorkers) { "busy" } else { "error" }
        extra_quality_12b_detected = $extraQualityDetected
        avoid_extra_12b = $Plan.CapacityPolicy.avoid_extra_12b
        max_quality_12b_workers = $Plan.CapacityPolicy.max_quality_12b_workers
        quality_worker_count = $qualityWorkers.Count
        helper_worker_count = $helperWorkers.Count
        helper_target_worker_count = $helperTargetWorkerCount
        helper_roles = $Plan.CapacityPolicy.helper_roles
        recommended_launch_order = $Plan.CapacityPolicy.recommended_launch_order
        worker_shape = [pscustomobject]@{
            quality = $qualityWorkers.Count
            helpers_visible = $helperWorkers.Count
            helper_target = $helperTargetWorkerCount
        }
        operator_checks = "Activity Monitor GPU History and Memory Pressure must stay healthy before adding workers"
    }
}

function Get-ModelPoolManifest {
    $plan = Get-AppleSiliconPoolPlan
    $workers = @()
    foreach ($worker in $plan.Ports) {
        $baseUrl = Get-PoolWorkerBaseUrl $worker
        $port = if ($worker.Role -eq "quality") {
            ([Uri]$baseUrl).Port
        } else {
            $worker.Port
        }
        $workers += [pscustomobject]@{
            role = $worker.Role
            port = $port
            base_url = $baseUrl
            enabled_by_default = $worker.EnabledByDefault
            model_class = $worker.ModelClass
            suggested_quant = $worker.SuggestedQuant
            default_context_tokens = $worker.DefaultContextTokens
            default_max_tokens = $worker.DefaultMaxTokens
            low_priority = ($worker.Role -ne "quality")
        }
    }
    $advice = New-ModelPoolManifestAdvice -Plan $plan
    return [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        read_only = $true
        sends_prompt = $false
        launches_process = $false
        manifest_kind = "rust-norion.model-pool"
        capacity_policy = $plan.CapacityPolicy
        advice = $advice
        decision_source = $advice.decision_source
        safe_to_enable_pool_workers = $advice.safe_to_enable_pool_workers
        next_step = $advice.next_step
        reason = $advice.reason
        extra_quality_12b_detected = $advice.extra_quality_12b_detected
        quality_worker_count = $advice.quality_worker_count
        helper_worker_count = $advice.helper_worker_count
        helper_target_worker_count = $advice.helper_target_worker_count
        helper_roles = $advice.helper_roles
        capacity_recommendation = $advice.next_step
        worker_shape = $advice.worker_shape
        workers = $workers
    }
}

function Show-PoolManifest {
    $manifest = Get-ModelPoolManifest
    if ($JsonStatus -or $JsonPlan) {
        Write-Host ($manifest | ConvertTo-StatusJson -Depth 10)
        return
    }
    Write-Host ($manifest | ConvertTo-Json -Depth 10)
}

function Get-PoolWorkerBaseUrl {
    param($Worker)
    if ($Worker.Role -eq "quality") {
        return $ModelBaseUrl
    }
    $uri = [Uri]$ModelBaseUrl
    return "$($uri.Scheme)://$($uri.Host):$($Worker.Port)"
}

function Get-PoolEndpointStatus {
    param(
        [string]$Name,
        [string]$BaseUrl
    )
    $tcp = Test-TcpPort -BaseUrl $BaseUrl -TimeoutMs 500
    if (-not $tcp) {
        return [pscustomobject]@{
            name = $Name
            base_url = $BaseUrl
            tcp_reachable = $false
            health_ok = $false
            health_elapsed_ms = 0
            health_error = "tcp port unreachable"
            health_value = $null
        }
    }
    return Get-EndpointStatus -Name $Name -BaseUrl $BaseUrl
}

function Convert-PoolWorkerPublicStatus {
    param(
        $Worker,
        $Endpoint
    )
    $workerStatus = Get-PoolWorkerReadinessStatus $Endpoint
    return [pscustomobject]@{
        port = $Worker.Port
        role = $Worker.Role
        base_url = $Endpoint.base_url
        enabled_by_default = $Worker.EnabledByDefault
        suggested_quant = $Worker.SuggestedQuant
        launches_process = $Worker.LaunchesProcess
        requires_quality_gate = $Worker.RequiresQualityGate
        tcp_reachable = $Endpoint.tcp_reachable
        health_ok = $Endpoint.health_ok
        status = $workerStatus.status
        role_ready = $workerStatus.role_ready
        role_block_reason = $workerStatus.role_block_reason
        health_elapsed_ms = $Endpoint.health_elapsed_ms
        health_error = $Endpoint.health_error
        service = Get-PropertyValue $Endpoint.health_value @("service")
        model = Get-PropertyValue $Endpoint.health_value @("model", "runtime_model", "gemma_runtime_model")
        context_window = Get-PropertyValue $Endpoint.health_value @("n_ctx", "context_window", "gemma_runtime_context_window")
        default_max_tokens = Get-PropertyValue $Endpoint.health_value @("n_predict", "default_max_tokens", "max_tokens", "gemma_runtime_default_max_tokens")
        runtime_backend = Get-PropertyValue $Endpoint.health_value @("runtime_backend", "backend", "engine")
        runtime_device = Get-PropertyValue $Endpoint.health_value @("runtime_device", "device", "device_profile", "execution_device")
        runtime_accelerator = Get-RuntimeAcceleratorValue $Endpoint.health_value
        gpu_layers = Get-PropertyValue $Endpoint.health_value @("gpu_layers", "n_gpu_layers", "offloaded_gpu_layers")
    }
}

function Test-PoolWorkerReportsMetal {
    param($Worker)
    return (
        ([string]$Worker.runtime_accelerator).Equals("metal", [System.StringComparison]::OrdinalIgnoreCase) -or
        ([string]$Worker.runtime_device).Equals("metal", [System.StringComparison]::OrdinalIgnoreCase)
    )
}

function Test-PoolWorkerReportsCpu {
    param($Worker)
    return (
        ([string]$Worker.runtime_accelerator).Equals("cpu", [System.StringComparison]::OrdinalIgnoreCase) -or
        ([string]$Worker.runtime_device).Equals("cpu", [System.StringComparison]::OrdinalIgnoreCase)
    )
}

function Test-PoolWorkerRuntimeUnknown {
    param($Worker)
    return (
        $null -eq $Worker.runtime_backend -and
        $null -eq $Worker.runtime_device -and
        $null -eq $Worker.runtime_accelerator -and
        $null -eq $Worker.gpu_layers
    )
}

function Get-PoolWorkerAccelerationState {
    param($Worker)
    if (Test-PoolWorkerReportsMetal $Worker) {
        return $true
    }
    if ($null -ne $Worker.gpu_layers -and [int64]$Worker.gpu_layers -gt 0) {
        return $true
    }
    if (Test-PoolWorkerReportsCpu $Worker) {
        return $false
    }
    if ($null -ne $Worker.gpu_layers -and [int64]$Worker.gpu_layers -eq 0) {
        return $false
    }
    return $null
}

function New-ModelPoolCapacitySummary {
    param(
        $Workers,
        $LaunchGate
    )
    $workerRows = @($Workers)
    $healthyWorkers = @($workerRows | Where-Object { $_.role_ready -eq $true })
    $helperWorkers = @($workerRows | Where-Object { $_.role -ne "quality" })
    $healthyHelpers = @($helperWorkers | Where-Object { $_.role_ready -eq $true })
    $metalWorkers = @($healthyWorkers | Where-Object { Test-PoolWorkerReportsMetal $_ })
    $cpuWorkers = @($healthyWorkers | Where-Object { Test-PoolWorkerReportsCpu $_ })
    $unknownRuntimeWorkers = @($healthyWorkers | Where-Object { Test-PoolWorkerRuntimeUnknown $_ })
    $zeroGpuLayerWorkers = @($healthyWorkers | Where-Object { $null -ne $_.gpu_layers -and [int64]$_.gpu_layers -eq 0 })
    $qualityWorker = @($healthyWorkers | Where-Object { $_.role -eq "quality" } | Select-Object -First 1)
    $qualityRuntimeAccelerated = if ($qualityWorker.Count -gt 0) {
        Get-PoolWorkerAccelerationState $qualityWorker[0]
    } else {
        $null
    }
    $expansionAllowed = (
        $LaunchGate.launch_allowed -eq $true -and
        $qualityRuntimeAccelerated -ne $false -and
        $cpuWorkers.Count -eq 0 -and
        $zeroGpuLayerWorkers.Count -eq 0 -and
        $unknownRuntimeWorkers.Count -eq 0
    )
    $recommendation = if ($LaunchGate.launch_allowed -ne $true) {
        "restore_quality_gate_first"
    } elseif ($qualityRuntimeAccelerated -eq $false -or $cpuWorkers.Count -gt 0) {
        "fix_runtime_acceleration_before_adding_workers"
    } elseif ($unknownRuntimeWorkers.Count -gt 0) {
        "verify_worker_runtime_metadata_before_expansion"
    } elseif ($healthyHelpers.Count -eq 0) {
        "add_summary_worker_first"
    } elseif ($healthyHelpers.Count -eq 1) {
        "add_review_or_index_worker_after_short_smoke"
    } elseif ($healthyHelpers.Count -lt $helperWorkers.Count) {
        "restore_missing_helper_workers_before_more_concurrency"
    } else {
        "hold_or_add_optional_test_gate_if_memory_pressure_green"
    }
    return [pscustomobject]@{
        policy = "one_quality_plus_small_helpers"
        expansion_allowed = $expansionAllowed
        recommendation = $recommendation
        worker_count = $workerRows.Count
        healthy_worker_count = $healthyWorkers.Count
        helper_worker_count = $helperWorkers.Count
        healthy_helper_worker_count = $healthyHelpers.Count
        metal_worker_count = $metalWorkers.Count
        cpu_worker_count = $cpuWorkers.Count
        unknown_runtime_worker_count = $unknownRuntimeWorkers.Count
        zero_gpu_layer_worker_count = $zeroGpuLayerWorkers.Count
        quality_runtime_accelerated = $qualityRuntimeAccelerated
    }
}

function Get-PoolWorkerReadinessStatus {
    param($Endpoint)
    if ($Endpoint.health_ok -eq $true) {
        return [pscustomobject]@{
            status = "healthy"
            role_ready = $true
            role_block_reason = "none"
        }
    }
    if ($Endpoint.tcp_reachable -eq $true) {
        return [pscustomobject]@{
            status = "tcp_only"
            role_ready = $false
            role_block_reason = "health_failed"
        }
    }
    return [pscustomobject]@{
        status = "unreachable"
        role_ready = $false
        role_block_reason = "tcp_unreachable"
    }
}

function Get-ModelPoolContextGate {
    param([int64]$RequestedMinContextTokens = $MinContextTokens)
    $qualityDefaultContextTokens = 262144L
    try {
        $plan = Get-AppleSiliconPoolPlan
        $qualityWorker = @($plan.Ports | Where-Object { $_.Role -eq "quality" })[0]
        if ($qualityWorker -and $qualityWorker.DefaultContextTokens) {
            $qualityDefaultContextTokens = [int64]$qualityWorker.DefaultContextTokens
        }
    } catch {
        $qualityDefaultContextTokens = 262144L
    }

    $effectiveMinContextTokens = [int64]$RequestedMinContextTokens
    $contextGateSource = "cli"
    if ($effectiveMinContextTokens -le 0) {
        $effectiveMinContextTokens = $qualityDefaultContextTokens
        $contextGateSource = "quality default"
    }

    return [pscustomobject]@{
        min_context_tokens = $effectiveMinContextTokens
        context_gate_source = $contextGateSource
    }
}

function Get-ModelPoolLaunchGateSnapshot {
    $contextGate = Get-ModelPoolContextGate -RequestedMinContextTokens $MinContextTokens
    $effectiveMinContextTokens = $contextGate.min_context_tokens
    $contextGateSource = $contextGate.context_gate_source
    $previousMinContextTokens = $script:MinContextTokens
    try {
        $script:MinContextTokens = $effectiveMinContextTokens
        $health = Get-BackendHealth
        $promptGate = Get-PromptGateStatus $health
        $launchAllowed = ($promptGate.prompt_ready -eq $true -and $promptGate.context_ready -eq $true)
        $classification = "prompt_blocked"
        $nextStep = "do not launch model pool until prompt-gate is ready"
        if ($health.engine_busy -eq $true) {
            $classification = "engine_busy"
            $nextStep = "do not launch model pool; wait for active request to finish"
        } elseif ($health.gemma_runtime_reachable -ne $true -or $health.readiness_ok -ne $true) {
            $classification = "quality_worker_down"
            $nextStep = "do not launch model pool; restore quality worker first"
        } elseif ($promptGate.context_ready -ne $true) {
            $classification = "context_not_ready"
            $nextStep = "do not launch model pool; restore required context window first"
        } elseif ($launchAllowed) {
            $classification = "prompt_ready"
            $nextStep = "model pool launch gate is ready; operator may add small workers after smoke"
        }
        return [pscustomobject]@{
            classification = $classification
            next_step = $nextStep
            launch_allowed = $launchAllowed
            min_context_tokens = $effectiveMinContextTokens
            context_gate_source = $contextGateSource
            prompt_gate = $promptGate
        }
    } catch {
        return [pscustomobject]@{
            classification = "backend_down"
            next_step = "do not launch model pool; restore backend first"
            launch_allowed = $false
            min_context_tokens = $effectiveMinContextTokens
            context_gate_source = $contextGateSource
            prompt_gate = $null
        }
    } finally {
        $script:MinContextTokens = $previousMinContextTokens
    }
}

function Get-ModelPoolLaunchGateStatus {
    if (-not $WaitReady) {
        return Get-ModelPoolLaunchGateSnapshot
    }

    $deadline = (Get-Date).AddSeconds($WaitTimeoutSec)
    $lastStatus = $null
    while ($true) {
        $lastStatus = Get-ModelPoolLaunchGateSnapshot
        if ($lastStatus.launch_allowed) {
            return $lastStatus
        }
        if ((Get-Date) -ge $deadline) {
            return $lastStatus
        }
        Start-Sleep -Seconds ([Math]::Max(1, $PollIntervalSec))
    }
}

function New-ModelPoolStatusPublic {
    param(
        $Plan,
        $LaunchGate,
        $Workers
    )
    return [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        summary = $Plan.Summary
        blocked_policy = $Plan.BlockedPolicy
        launch_gate = $Plan.QualityGateCommand
        launch_allowed = $LaunchGate.launch_allowed
        launch_block_reason = if ($LaunchGate.launch_allowed) { "none" } else { $LaunchGate.classification }
        min_context_tokens = $LaunchGate.min_context_tokens
        context_gate_source = $LaunchGate.context_gate_source
        chain_classification = $LaunchGate.classification
        chain_next_step = $LaunchGate.next_step
        prompt_gate = $LaunchGate.prompt_gate
        capacity = New-ModelPoolCapacitySummary -Workers $Workers -LaunchGate $LaunchGate
        workers = $Workers
    }
}

function Get-ModelPoolStatus {
    $plan = Get-AppleSiliconPoolPlan
    $launchGate = Get-ModelPoolLaunchGateStatus
    $workers = @()
    foreach ($worker in $plan.Ports) {
        $baseUrl = Get-PoolWorkerBaseUrl $worker
        $endpoint = Get-PoolEndpointStatus "$($worker.Role)-worker" $baseUrl
        $workers += Convert-PoolWorkerPublicStatus -Worker $worker -Endpoint $endpoint
    }
    return New-ModelPoolStatusPublic -Plan $plan -LaunchGate $launchGate -Workers $workers
}

function Show-PoolStatus {
    $status = Get-ModelPoolStatus
    if ($JsonStatus) {
        Write-Host ($status | ConvertTo-StatusJson)
        if ($FailIfBlocked -and -not $status.launch_allowed) {
            exit 2
        }
        return
    }

    Write-Section "model pool status"
    Write-Host "schema_version=$($status.schema_version) contract_version=$($status.contract_version)"
    Write-Host $status.summary
    Write-Host $status.blocked_policy
    Write-Host "launch_gate=$($status.launch_gate)"
    Write-Host "launch_allowed=$($status.launch_allowed) reason=$($status.launch_block_reason)"
    Write-Host "min_context_tokens=$($status.min_context_tokens) source=$($status.context_gate_source)"
    Write-Host "chain_classification=$($status.chain_classification)"
    Write-Host "next_step=$($status.chain_next_step)"
    if ($status.capacity) {
        Write-Section "capacity"
        Write-Host "policy=$($status.capacity.policy) expansion_allowed=$($status.capacity.expansion_allowed) recommendation=$($status.capacity.recommendation)"
        Write-Host "workers=$($status.capacity.healthy_worker_count)/$($status.capacity.worker_count) helpers=$($status.capacity.healthy_helper_worker_count)/$($status.capacity.helper_worker_count)"
        Write-Host "runtime metal=$($status.capacity.metal_worker_count) cpu=$($status.capacity.cpu_worker_count) unknown=$($status.capacity.unknown_runtime_worker_count) gpu_layers_zero=$($status.capacity.zero_gpu_layer_worker_count) quality_accelerated=$($status.capacity.quality_runtime_accelerated)"
    }

    Write-Section "workers"
    foreach ($worker in $status.workers) {
        Write-Host "  $($worker.port) role=$($worker.role) enabled_default=$($worker.enabled_by_default) tcp=$($worker.tcp_reachable) health=$($worker.health_ok)"
        Write-Host "      url=$($worker.base_url) quant=$($worker.suggested_quant) launches_process=$($worker.launches_process)"
        if ($worker.health_ok) {
            Write-Host "      service=$($worker.service) model=$($worker.model) context=$($worker.context_window) max_tokens=$($worker.default_max_tokens)"
            Write-Host "      runtime_backend=$($worker.runtime_backend) runtime_device=$($worker.runtime_device) runtime_accelerator=$($worker.runtime_accelerator) gpu_layers=$($worker.gpu_layers)"
        } elseif ($worker.health_error) {
            Write-Host "      error=$($worker.health_error)"
        }
    }

    if ($FailIfBlocked -and -not $status.launch_allowed) {
        exit 2
    }
}

function Show-PromptGate {
    $health = Get-BackendHealth
    $status = Get-PromptGateStatus $health

    if ($JsonStatus) {
        Write-Host ($status | ConvertTo-StatusJson)
        if ($FailIfBlocked -and -not $status.prompt_ready) {
            exit 2
        }
        return
    }

    Write-Section "prompt gate"
    Write-Host "schema_version=$($status.schema_version) contract_version=$($status.contract_version)"
    Write-Host "backend=$($status.backend)"
    Write-Host "engine_busy=$($status.engine_busy)"
    Write-Host "gemma_runtime_reachable=$($status.gemma_runtime_reachable)"
    Write-Host "readiness_ok=$($status.readiness_ok)"
    Write-Host "safe_device_ok=$($status.safe_device_ok)"
    Write-Host "prompt_ready=$($status.prompt_ready)"
    Write-Host "block_reason=$($status.block_reason)"
    Write-Host "context_window=$($status.context_window)"
    Write-Host "min_context_tokens=$($status.min_context_tokens)"
    Write-Host "context_ready=$($status.context_ready)"
    Write-Host "context_block_reason=$($status.context_block_reason)"

    Write-Section "entrypoint decisions"
    foreach ($entrypoint in $status.entrypoint_decisions) {
        Write-Host "$($entrypoint.id) allowed=$($entrypoint.allowed) reason=$($entrypoint.reason) standard_context=$($entrypoint.standard_required_context_tokens)"
        Write-Host "  standard_gate=$($entrypoint.standard_gate_command)"
    }

    Write-Section "compatibility aliases"
    foreach ($entrypoint in $status.entrypoint_decisions | Where-Object { $_.compatibility_alias }) {
        Write-Host "$($entrypoint.compatibility_alias) -> $($entrypoint.id)"
    }

    Write-Section "quality worker prerequisite"
    if ($status.quality_worker_prerequisite_passed) {
        Write-Host "quality worker reachable/readiness prerequisite: PASS"
    } else {
        Write-Host "quality worker reachable/readiness prerequisite: FAIL"
        Write-Host "do not run smoke, Web Lab prompt, Forge prompt, CLI prompt, evolution-loop prompt rounds, or model-pool startup."
    }
    if ($FailIfBlocked -and -not $status.prompt_ready) {
        exit 2
    }
}

function Get-PoolTaskRoleCandidates {
    param([string]$Kind = $TaskKind)
    $normalizedKind = Normalize-PoolTaskKind $Kind
    switch ($normalizedKind) {
        "quality" { return @("quality") }
        "summary" { return @("summary") }
        "router" { return @("router", "summary") }
        "review" { return @("review") }
        "test-gate" { return @("test-gate", "review") }
        "index" { return @("index", "summary") }
        "auto" { return @("summary", "router", "review", "index", "test-gate") }
        default { return @("summary") }
    }
}

function Normalize-PoolTaskKind {
    param([string]$Kind = $TaskKind)
    if ([string]::IsNullOrWhiteSpace($Kind)) {
        return "auto"
    }
    switch ($Kind.Trim().ToLowerInvariant()) {
        "spare" { return "index" }
        "repo-index" { return "index" }
        "repository-index" { return "index" }
        "test" { return "test-gate" }
        "gate" { return "test-gate" }
        "tool-call" { return "router" }
        "tool_calls" { return "router" }
        "function" { return "router" }
        "function-call" { return "router" }
        "function_call" { return "router" }
        "preflight" { return "router" }
        "intent" { return "router" }
        default { return $Kind.Trim().ToLowerInvariant() }
    }
}

function Get-RoutePlanCandidateWorkers {
    param(
        $PoolStatus,
        [string[]]$RoleCandidates,
        [string]$Kind = $TaskKind
    )
    $ordered = @()
    foreach ($role in $RoleCandidates) {
        foreach ($worker in @($PoolStatus.workers | Where-Object { $_.role -eq $role })) {
            $canAcceptLowPriority = (
                $PoolStatus.launch_allowed -eq $true -and
                $worker.health_ok -eq $true -and
                $worker.role -ne "quality" -and
                $worker.enabled_by_default -eq $true
            )
            $ordered += [pscustomobject]@{
                port = $worker.port
                role = $worker.role
                base_url = $worker.base_url
                enabled_by_default = $worker.enabled_by_default
                suggested_quant = $worker.suggested_quant
                tcp_reachable = $worker.tcp_reachable
                health_ok = $worker.health_ok
                status = $worker.status
                role_ready = $worker.role_ready
                role_block_reason = $worker.role_block_reason
                can_accept_low_priority_task = $canAcceptLowPriority
                model = $worker.model
                context_window = $worker.context_window
                default_max_tokens = $worker.default_max_tokens
                runtime_backend = $worker.runtime_backend
                runtime_device = $worker.runtime_device
                runtime_accelerator = $worker.runtime_accelerator
                gpu_layers = $worker.gpu_layers
            }
        }
    }
    return $ordered
}

function New-ModelPoolRoutePlanPublic {
    param(
        $PoolStatus,
        [string]$Kind = $TaskKind
    )
    $normalizedKind = Normalize-PoolTaskKind $Kind
    $roleCandidates = @(Get-PoolTaskRoleCandidates -Kind $normalizedKind)
    $candidateWorkers = @(Get-RoutePlanCandidateWorkers -PoolStatus $PoolStatus -RoleCandidates $roleCandidates -Kind $normalizedKind)
    $selected = @($candidateWorkers | Where-Object {
        $_.health_ok -eq $true -and $_.enabled_by_default -eq $true
    } | Select-Object -First 1)

    $selectedWorker = if ($selected.Count -gt 0) { $selected[0] } else { $null }
    $routeAllowed = ($PoolStatus.launch_allowed -eq $true -and $null -ne $selectedWorker)
    $blockReason = "none"
    $nextStep = "route is ready; operator may send a task after confirming shared-chain ownership"
    if ($PoolStatus.launch_allowed -ne $true) {
        $blockReason = "model_pool_launch_blocked:$($PoolStatus.launch_block_reason)"
        $nextStep = $PoolStatus.chain_next_step
    } elseif ($null -eq $selectedWorker) {
        $blockReason = "no_healthy_candidate_worker"
        $nextStep = "start or restore one candidate worker before routing this task kind"
    }

    return [pscustomobject]@{
        schema_version = $script:GemmaChainSchemaVersion
        contract_version = $script:GemmaChainContractVersion
        read_only = $true
        launches_process = $false
        sends_prompt = $false
        task_kind = $normalizedKind
        route_allowed = $routeAllowed
        route_block_reason = $blockReason
        next_step = $nextStep
        selected_role = if ($selectedWorker) { $selectedWorker.role } else { $null }
        selected_worker = $selectedWorker
        role_candidates = $roleCandidates
        candidate_workers = $candidateWorkers
        quality_gate = [pscustomobject]@{
            launch_allowed = $PoolStatus.launch_allowed
            launch_block_reason = $PoolStatus.launch_block_reason
            min_context_tokens = $PoolStatus.min_context_tokens
            context_gate_source = $PoolStatus.context_gate_source
            chain_classification = $PoolStatus.chain_classification
        }
        policy = "read-only route plan only; helper routes do not borrow quality by default; do not launch workers or send prompts until route_allowed is true and the operator owns the shared chain"
    }
}

function Get-ModelPoolRoutePlan {
    $poolStatus = Get-ModelPoolStatus
    return New-ModelPoolRoutePlanPublic -PoolStatus $poolStatus -Kind $TaskKind
}

function Show-PoolRoutePlan {
    $route = Get-ModelPoolRoutePlan
    if ($JsonStatus) {
        Write-Host ($route | ConvertTo-StatusJson -Depth 18)
        if ($FailIfBlocked -and -not $route.route_allowed) {
            exit 2
        }
        return
    }

    Write-Section "model pool route plan"
    Write-Host "schema_version=$($route.schema_version) contract_version=$($route.contract_version)"
    Write-Host "task_kind=$($route.task_kind)"
    Write-Host "read_only=$($route.read_only) launches_process=$($route.launches_process) sends_prompt=$($route.sends_prompt)"
    Write-Host "route_allowed=$($route.route_allowed) reason=$($route.route_block_reason)"
    Write-Host "role_candidates=$($route.role_candidates -join ',')"
    Write-Host "selected_role=$($route.selected_role)"
    Write-Host "next_step=$($route.next_step)"

    Write-Section "candidate workers"
    foreach ($worker in $route.candidate_workers) {
        Write-Host "  $($worker.port) role=$($worker.role) status=$($worker.status) ready=$($worker.role_ready) low_priority=$($worker.can_accept_low_priority_task)"
        Write-Host "      url=$($worker.base_url) reason=$($worker.role_block_reason) enabled_default=$($worker.enabled_by_default)"
        Write-Host "      context=$($worker.context_window) max_tokens=$($worker.default_max_tokens) runtime_backend=$($worker.runtime_backend) runtime_device=$($worker.runtime_device) runtime_accelerator=$($worker.runtime_accelerator) gpu_layers=$($worker.gpu_layers)"
    }

    if ($FailIfBlocked -and -not $route.route_allowed) {
        exit 2
    }
}

function Get-LatestFile {
    param(
        [string]$Path,
        [string]$Filter = "*"
    )
    if (-not (Test-Path -LiteralPath $Path)) {
        return $null
    }
    return Get-ChildItem -LiteralPath $Path -File -Filter $Filter |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
}

function Show-FileSummary {
    param(
        [string]$Label,
        $File
    )
    if ($null -eq $File) {
        Write-Host "$Label missing"
        return
    }
    $age = [int]((Get-Date) - $File.LastWriteTime).TotalSeconds
    Write-Host "$Label path=$($File.FullName)"
    Write-Host "$Label last_write=$($File.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss')) age_seconds=$age bytes=$($File.Length)"
}

function Show-SafeTail {
    param(
        [string]$Label,
        $File,
        [int]$Lines = 12
    )
    if ($null -eq $File -or -not (Test-Path -LiteralPath $File.FullName)) {
        Write-Host "$Label tail unavailable"
        return
    }
    Write-Host "$Label tail:"
    $tail = Get-Content -LiteralPath $File.FullName -Tail $Lines
    foreach ($line in $tail) {
        Write-Host "  $(Format-SafeLogLine $line)"
    }
}

function Read-LastJsonLine {
    param($File)
    if ($null -eq $File -or -not (Test-Path -LiteralPath $File.FullName)) {
        return $null
    }
    $line = Get-Content -LiteralPath $File.FullName -Tail 1
    if ([string]::IsNullOrWhiteSpace($line)) {
        return $null
    }
    try {
        return $line | ConvertFrom-Json
    } catch {
        return [pscustomobject]@{
            parse_error = $_.Exception.Message
            raw_preview = Format-SafePreview $line
        }
    }
}

function Get-LoopStatus {
    $health = Get-BackendHealth
    $promptGate = Get-PromptGateStatus $health
    $evolutionDir = Join-Path $RepoRoot "target\evolution"
    $logDir = Join-Path $evolutionDir "daemon-logs"
    $latestLedger = Get-LatestFile -Path $evolutionDir -Filter "*.jsonl"
    $latestOut = Get-LatestFile -Path $logDir -Filter "*.out.log"
    $latestErr = Get-LatestFile -Path $logDir -Filter "*.err.log"

    $lastRecord = Read-LastJsonLine $latestLedger
    $errText = ""
    if ($latestErr -and (Test-Path -LiteralPath $latestErr.FullName)) {
        $errText = (Get-Content -LiteralPath $latestErr.FullName -Tail 30) -join "`n"
    }
    $outText = ""
    if ($latestOut -and (Test-Path -LiteralPath $latestOut.FullName)) {
        $outText = (Get-Content -LiteralPath $latestOut.FullName -Tail 30) -join "`n"
    }

    $classification = "no_recent_block_detected"
    $action = "diagnose is clean enough for an operator to decide whether to run smoke."
    if ($errText -match '(?i)health gate failed.*Gemma runtime is not reachable|backend readiness is false|n_ctx is missing') {
        $classification = "quality_worker_gate_blocked"
        $action = "restore quality worker/tunnel ownership first, then rerun diagnose and prompt-gate."
    } elseif ($outText -match '(?i)\[round\s+\d+\].*(generate:start|business cycle stream connected)') {
        $classification = "recent_loop_activity_seen"
        $action = "use prompt-gate and daemon timestamps before starting any new prompt source."
    } elseif (-not $promptGate.prompt_ready) {
        $classification = "prompt_blocked_by_backend_health"
        $action = "do not start evolution-loop prompt rounds."
    }

    return [pscustomobject]@{
        prompt_gate = $promptGate
        evolution_dir = $evolutionDir
        ledger = if ($latestLedger) {
            [pscustomobject]@{
                path = $latestLedger.FullName
                last_write = $latestLedger.LastWriteTime.ToString("o")
                age_seconds = [int]((Get-Date) - $latestLedger.LastWriteTime).TotalSeconds
                bytes = $latestLedger.Length
                last_record = if ($lastRecord) {
                    [pscustomobject]@{
                        round = $lastRecord.round
                        case = $lastRecord.case
                        success = $lastRecord.success
                        error = Format-SafePreview ([string]$lastRecord.error)
                        runtime_tokens = $lastRecord.runtime_tokens
                        runtime_model = $lastRecord.runtime_model
                        elapsed_ms = $lastRecord.elapsed_ms
                        business_cycle_passed = $lastRecord.business_cycle_passed
                        feedback_applied = $lastRecord.feedback_applied
                        validation_checked = $lastRecord.validation_checked
                        validation_passed = $lastRecord.validation_passed
                        self_improve_passed = $lastRecord.self_improve_passed
                    }
                } else { $null }
            }
        } else { $null }
        daemon_out = if ($latestOut) {
            [pscustomobject]@{
                path = $latestOut.FullName
                last_write = $latestOut.LastWriteTime.ToString("o")
                age_seconds = [int]((Get-Date) - $latestOut.LastWriteTime).TotalSeconds
                bytes = $latestOut.Length
            }
        } else { $null }
        daemon_err = if ($latestErr) {
            [pscustomobject]@{
                path = $latestErr.FullName
                last_write = $latestErr.LastWriteTime.ToString("o")
                age_seconds = [int]((Get-Date) - $latestErr.LastWriteTime).TotalSeconds
                bytes = $latestErr.Length
            }
        } else { $null }
        classification = $classification
        action = $action
    }
}

function Show-LoopStatus {
    $summary = Get-LoopStatus
    $promptGate = $summary.prompt_gate
    $classification = $summary.classification
    $action = $summary.action

    if ($JsonStatus) {
        Write-Host ($summary | ConvertTo-StatusJson)
        if ($FailIfBlocked -and (-not $promptGate.prompt_ready -or $classification -eq "quality_worker_gate_blocked")) {
            exit 2
        }
        return
    }

    $latestLedger = if ($summary.ledger -and (Test-Path -LiteralPath $summary.ledger.path)) {
        Get-Item -LiteralPath $summary.ledger.path
    } else { $null }
    $latestOut = if ($summary.daemon_out -and (Test-Path -LiteralPath $summary.daemon_out.path)) {
        Get-Item -LiteralPath $summary.daemon_out.path
    } else { $null }
    $latestErr = if ($summary.daemon_err -and (Test-Path -LiteralPath $summary.daemon_err.path)) {
        Get-Item -LiteralPath $summary.daemon_err.path
    } else { $null }
    $lastRecord = if ($summary.ledger) { $summary.ledger.last_record } else { $null }

    Write-Section "prompt gate"
    Write-Host "prompt_ready=$($promptGate.prompt_ready)"
    Write-Host "block_reason=$($promptGate.block_reason)"
    Write-Host "engine_busy=$($promptGate.engine_busy)"
    Write-Host "gemma_runtime_reachable=$($promptGate.gemma_runtime_reachable)"
    Write-Host "readiness_ok=$($promptGate.readiness_ok)"
    Write-Host "safe_device_ok=$($promptGate.safe_device_ok)"
    Write-Host "context_window=$($promptGate.context_window)"
    Write-Host "min_context_tokens=$($promptGate.min_context_tokens)"
    Write-Host "context_ready=$($promptGate.context_ready)"
    Write-Host "context_block_reason=$($promptGate.context_block_reason)"
    if (-not $promptGate.prompt_ready) {
        Write-Host "loop_prompt_policy=blocked; do not start new evolution prompt rounds."
    } else {
        Write-Host "loop_prompt_policy=allowed after operator confirms no shared-chain owner is active."
    }

    Write-Section "latest evolution artifacts"
    Show-FileSummary "ledger" $latestLedger
    Show-FileSummary "daemon_out" $latestOut
    Show-FileSummary "daemon_err" $latestErr

    if ($lastRecord) {
        Write-Section "latest ledger record"
        Write-Host ($lastRecord | ConvertTo-SafeJson -MaxLength 2000)
    }

    Write-Section "daemon tails"
    Show-SafeTail "daemon_out" $latestOut 10
    Show-SafeTail "daemon_err" $latestErr 10

    Write-Section "loop diagnosis"

    if ($classification -eq "quality_worker_gate_blocked") {
        Write-Host "classification=$classification"
        Write-Host "action=restore quality worker/tunnel ownership first, then rerun diagnose and prompt-gate."
    } elseif ($classification -eq "recent_loop_activity_seen") {
        Write-Host "classification=$classification"
        Write-Host "action=$action"
    } elseif ($classification -eq "prompt_blocked_by_backend_health") {
        Write-Host "classification=$classification"
        Write-Host "action=$action"
    } else {
        Write-Host "classification=$classification"
        Write-Host "action=$action"
    }
    if ($FailIfBlocked -and (-not $promptGate.prompt_ready -or $classification -eq "quality_worker_gate_blocked")) {
        exit 2
    }
}

function Wait-BackendIdle {
    $deadline = (Get-Date).AddSeconds($WaitTimeoutSec)
    while ($true) {
        $health = Get-BackendHealth
        if ($health.engine_busy -ne $true) {
            Write-Host "backend idle"
            return $health
        }
        if ((Get-Date) -ge $deadline) {
            throw "backend remained busy for ${WaitTimeoutSec}s; not sending smoke prompt"
        }
        $active = @($health.active_requests)
        if ($active.Count -gt 0) {
            $first = $active[0]
            Write-Host "backend busy: request_id=$($first.request_id) endpoint=$($first.endpoint) elapsed_ms=$($first.elapsed_ms)"
        } else {
            Write-Host "backend busy"
        }
        Start-Sleep -Seconds 5
    }
}

function Invoke-Smoke {
    Write-Section "preflight"
    $health = Get-BackendHealth
    Write-Host "engine_busy=$($health.engine_busy) model=$($health.gemma_runtime_model) context_window=$($health.gemma_runtime_context_window)"
    if ($health.engine_busy -eq $true) {
        if ($WaitIfBusy) {
            $health = Wait-BackendIdle
        } else {
            Write-Host "backend is busy; not sending smoke prompt. Re-run after the current round finishes, or pass -WaitIfBusy."
            exit 2
        }
    }
    if (-not (Test-BackendCanPrompt $health)) {
        Write-Host "backend is not prompt-ready: $(Get-PromptBlockReason $health)"
        Write-Host "not sending smoke prompt."
        exit 1
    }

    Write-Section "web lab sse smoke"
    $body = '{"messages":[{"role":"user","content":"Reply only with OK."}],"profile":"general","output":"raw"}'
    $response = $body | curl.exe -sS -N --max-time $SmokeTimeoutSec `
        -H "Content-Type: application/json" `
        --data-binary "@-" `
        "$LabBaseUrl/api/chat-stream"

    if ($LASTEXITCODE -ne 0) {
        throw "curl failed with exit code $LASTEXITCODE"
    }

    $text = ($response -join "`n")
    Write-Host $text.TrimEnd()

    $hasDone = $text -match 'event:\s*done\s+data:\s*\[DONE\]'
    $hasError = $text -match 'event:\s*error'
    $hasDelta = $text -match 'event:\s*delta'
    $hasOk = $text -match '(?i)data:\s*OK'
    $streamState = Test-SseContinuity $text

    Write-Section "stream summary"
    Write-Host "has_delta=$hasDelta"
    Write-Host "has_ok=$hasOk"
    Write-Host "has_done=$hasDone"
    Write-Host "has_error=$hasError"
    Write-Host "last_frame_complete=$($streamState.LastFrameComplete)"

    if ($hasError) {
        throw "smoke returned SSE error"
    }
    if (-not $hasDone) {
        throw "smoke did not finish with done=[DONE]"
    }
    if (-not $hasDelta) {
        throw "smoke did not stream any delta"
    }
    if (-not $streamState.LastFrameComplete) {
        throw "smoke ended with a partial SSE frame"
    }
}

switch ($Action) {
    "health" { Show-Health | Out-Null }
    "diagnose" { Show-Diagnose }
    "smoke" { Invoke-Smoke }
    "selftest" { Invoke-SelfTest }
    "pool-plan" { Show-PoolPlan }
    "pool-manifest" { Show-PoolManifest }
    "pool-status" { Show-PoolStatus }
    "pool-route-plan" { Show-PoolRoutePlan }
    "prompt-gate" { Show-PromptGate }
    "loop-status" { Show-LoopStatus }
    "chain-status" { Show-ChainStatus }
    "entrypoint-matrix" { Show-EntrypointMatrix }
    "recovery-plan" { Show-RecoveryPlan }
    "status-bundle" { Show-StatusBundle }
    "contract-audit" { Show-ContractAudit }
    "wrapper-manifest" { Show-WrapperManifest }
    "contract-fixture" { Show-ContractFixture }
    "handoff-report" { Show-HandoffReport }
    "secret-scan" { Show-SecretScan }
}
