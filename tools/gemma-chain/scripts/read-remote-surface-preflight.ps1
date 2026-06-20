param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ConsumerId = "",
    [switch]$FailOnBlocked,
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Invoke-JsonScript {
    param(
        [string]$ScriptPath,
        [string[]]$ScriptArgs = @()
    )

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath @ScriptArgs -Json
    if ($LASTEXITCODE -ne 0) {
        throw "$ScriptPath exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

function Get-SafeCommandById {
    param(
        [object[]]$Commands,
        [string]$Id
    )

    $command = @($Commands | Where-Object { $_.id -eq $Id } | Select-Object -First 1)
    if ($command.Count -eq 0) {
        return $null
    }
    return $command[0]
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$generatedAt = Get-Date
$generatedAtUtc = $generatedAt.ToUniversalTime()
$readinessScript = Join-Path $PSScriptRoot "read-remote-readiness-contract.ps1"
$readiness = Invoke-JsonScript -ScriptPath $readinessScript -ScriptArgs @("-RepoRoot", $root)
$safeCommands = @($readiness.safe_next_read_only_commands)
$allConsumers = @($readiness.consumer_projection)

if (-not [string]::IsNullOrWhiteSpace($ConsumerId)) {
    $selectedConsumers = @($allConsumers | Where-Object { $_.id -eq $ConsumerId })
} else {
    $selectedConsumers = $allConsumers
}

$consumerCards = @($selectedConsumers | ForEach-Object {
    $consumer = $_
    $safeCommand = Get-SafeCommandById -Commands $safeCommands -Id $consumer.safe_command_id
    [pscustomobject]@{
        id = $consumer.id
        surface = $consumer.surface
        entrypoint_kind = $consumer.entrypoint_kind
        current_allowed = $false
        blocked_by = @($consumer.blocked_by)
        reason = $consumer.reason
        safe_command_id = $consumer.safe_command_id
        safe_command = if ($null -ne $safeCommand) {
            [pscustomobject]@{
                id = $safeCommand.id
                command = $safeCommand.command
                purpose = $safeCommand.purpose
                read_only = $safeCommand.read_only
                starts_process = $safeCommand.starts_process
                sends_prompt = $safeCommand.sends_prompt
                touches_remote = $safeCommand.touches_remote
                writes_files = $safeCommand.writes_files
            }
        } else {
            $null
        }
    }
})

$primaryMissingEvidence = @($readiness.summary.missing_evidence | Select-Object -First 1)
$displaySeverity = if ($readiness.summary.can_support_external_residency_review -eq $true) {
    "ready_for_external_gate"
} elseif ($readiness.summary.evidence_fresh_all -ne $true) {
    "blocked_stale_evidence"
} elseif ($readiness.summary.continuous_window_present -ne $true -or $readiness.summary.resource_window_present -ne $true) {
    "blocked_missing_window"
} else {
    "blocked_external_gate"
}
$displayHeadline = switch ($displaySeverity) {
    "ready_for_external_gate" { "Evidence package ready for external gate review." }
    "blocked_stale_evidence" { "Gemma evidence is stale; prompt, launch, SSH, and daemon actions are blocked." }
    "blocked_missing_window" { "Continuous health or resource window is missing; actions remain blocked." }
    default { "External residency gate is still required before actions." }
}
$displayDetail = "snapshot=$($readiness.summary.snapshot_classification); missing=$(@($readiness.summary.missing_evidence) -join ','); pending=$(@($readiness.summary.pending_external_gates) -join ',')"
$nextActionLabel = if ($primaryMissingEvidence.Count -gt 0) {
    "Collect read-only evidence for $($primaryMissingEvidence[0])."
} elseif (@($readiness.summary.pending_external_gates).Count -gt 0) {
    "Run external residency gate review."
} else {
    "Keep fail-closed until an authorized gate explicitly allows action."
}

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.surface-preflight.v1"
    generated_at = $generatedAt.ToString("yyyy-MM-dd HH:mm:ss zzz")
    generated_at_utc = $generatedAtUtc.ToString("o")
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    requested_consumer_id = $ConsumerId
    known_consumer_ids = @($allConsumers | ForEach-Object { $_.id })
    consumer_found = ([string]::IsNullOrWhiteSpace($ConsumerId) -or $consumerCards.Count -gt 0)
    fail_on_blocked = [bool]$FailOnBlocked
    source_contracts = [pscustomobject]@{
        readiness = $readiness.contract_version
        consumer_contract = $readiness.summary.consumer_contract_version
    }
    status = [pscustomobject]@{
        package_ready_for_external_gate = $readiness.summary.can_support_external_residency_review
        can_support_external_residency_review = $readiness.summary.can_support_external_residency_review
        snapshot_classification = $readiness.summary.snapshot_classification
        evidence_fresh_all = $readiness.summary.evidence_fresh_all
        max_evidence_age_seconds = $readiness.summary.max_evidence_age_seconds
        fresh_minutes = $readiness.summary.fresh_minutes
        observation_window_status = $readiness.summary.observation_window_status
        continuous_window_present = $readiness.summary.continuous_window_present
        resource_window_status = $readiness.summary.resource_window_status
        resource_window_present = $readiness.summary.resource_window_present
        consumer_contract_validated = $readiness.summary.consumer_contract_validated
        consumer_allowed_count = $readiness.summary.consumer_allowed_count
        unsafe_safe_command_count = $readiness.summary.unsafe_safe_command_count
        missing_evidence = @($readiness.summary.missing_evidence)
        pending_external_gates = @($readiness.summary.pending_external_gates)
    }
    display = [pscustomobject]@{
        severity = $displaySeverity
        headline = $displayHeadline
        detail = $displayDetail
        primary_missing_evidence = if ($primaryMissingEvidence.Count -gt 0) { $primaryMissingEvidence[0] } else { "" }
        next_action_label = $nextActionLabel
        badge = if ($readiness.summary.can_support_external_residency_review -eq $true) { "external_gate_required" } else { "blocked" }
    }
    consumers = $consumerCards
    missing_evidence_actions = $readiness.missing_evidence_actions
    pending_external_gate_actions = $readiness.pending_external_gate_actions
    quick_commands = @(
        [pscustomobject]@{ id = "surface_preflight"; command = ".\tools\gemma-chain\scripts\read-remote-surface-preflight.ps1 -Json"; read_only = $true },
        [pscustomobject]@{ id = "consumer_preflight"; command = ".\tools\gemma-chain\scripts\read-remote-consumer-preflight.ps1 -Json"; read_only = $true },
        [pscustomobject]@{ id = "quick_contract"; command = ".\tools\gemma-chain\scripts\test-remote-readiness-quick-contract.ps1"; read_only = $true }
    )
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "surface_preflight_is_read_only_and_cannot_authorize_actions"
    }
}

if (-not $result.consumer_found) {
    $result.status | Add-Member -MemberType NoteProperty -Name "error" -Value "unknown_consumer_id"
}

$blockedForExit = (
    $result.consumer_found -ne $true -or
    $result.status.package_ready_for_external_gate -ne $true -or
    $result.status.consumer_allowed_count -ne 0 -or
    @($result.consumers | Where-Object { $_.current_allowed -ne $true }).Count -gt 0
)
$failOnBlockedExitCode = if ($result.consumer_found -ne $true) { 3 } elseif ($blockedForExit) { 2 } else { 0 }
$result | Add-Member -MemberType NoteProperty -Name "fail_on_blocked_exit_code" -Value $failOnBlockedExitCode

if ($Json) {
    $result | ConvertTo-Json -Depth 10
    if ($FailOnBlocked -and $failOnBlockedExitCode -ne 0) {
        exit $failOnBlockedExitCode
    }
    exit 0
}

Write-Host "Gemma remote surface preflight"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "generated_at=$($result.generated_at) generated_at_utc=$($result.generated_at_utc)"
Write-Host "display=$($result.display.severity) headline=$($result.display.headline)"
Write-Host "package_ready_for_external_gate=$($result.status.package_ready_for_external_gate) snapshot=$($result.status.snapshot_classification) evidence_fresh_all=$($result.status.evidence_fresh_all)"
Write-Host "observation_window=$($result.status.observation_window_status) resource_window=$($result.status.resource_window_status)"
Write-Host "consumers=$($result.consumers.Count) allowed=$($result.status.consumer_allowed_count) missing_evidence=$($result.status.missing_evidence -join ',') pending_external_gates=$($result.status.pending_external_gates -join ',')"
Write-Host "fail_on_blocked_exit_code=$($result.fail_on_blocked_exit_code)"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
if ($FailOnBlocked -and $failOnBlockedExitCode -ne 0) {
    exit $failOnBlockedExitCode
}
