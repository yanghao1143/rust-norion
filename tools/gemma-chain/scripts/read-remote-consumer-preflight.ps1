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

    return [pscustomobject]@{
        id = $command[0].id
        purpose = $command[0].purpose
        command = $command[0].command
        read_only = $command[0].read_only
        starts_process = $command[0].starts_process
        sends_prompt = $command[0].sends_prompt
        touches_remote = $command[0].touches_remote
        writes_files = $command[0].writes_files
    }
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

$knownConsumerIds = @($allConsumers | ForEach-Object { $_.id })
$consumerFound = ($selectedConsumers.Count -gt 0)
$preflights = @($selectedConsumers | ForEach-Object {
    $consumer = $_
    $safeCommand = Get-SafeCommandById -Commands $safeCommands -Id $consumer.safe_command_id
    $safeCommandResolved = ($null -ne $safeCommand)
    $safeCommandSafe = (
        $safeCommandResolved -and
        $safeCommand.read_only -eq $true -and
        $safeCommand.starts_process -eq $false -and
        $safeCommand.sends_prompt -eq $false -and
        $safeCommand.touches_remote -eq $false -and
        $safeCommand.writes_files -eq $false
    )

    [pscustomobject]@{
        id = $consumer.id
        surface = $consumer.surface
        entrypoint_kind = $consumer.entrypoint_kind
        current_allowed = $false
        downstream_sends_prompt = $consumer.downstream_sends_prompt
        downstream_launches_process = $consumer.downstream_launches_process
        downstream_touches_remote = $consumer.downstream_touches_remote
        blocked_by = @($consumer.blocked_by)
        decision = $consumer.decision
        reason = $consumer.reason
        safe_command_id = $consumer.safe_command_id
        safe_command_resolved = $safeCommandResolved
        safe_command_safe = $safeCommandSafe
        safe_command = $safeCommand
        next_user_visible_status = if ($consumer.current_allowed -eq $true) { "external_gate_required_before_action" } else { "blocked_collect_read_only_evidence" }
    }
})

$unsafePreflightCount = @($preflights | Where-Object {
    $_.current_allowed -ne $false -or
    $_.safe_command_resolved -ne $true -or
    $_.safe_command_safe -ne $true
}).Count

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.consumer-preflight.v1"
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
    known_consumer_ids = $knownConsumerIds
    consumer_found = $consumerFound
    fail_on_blocked = [bool]$FailOnBlocked
    source_contracts = [pscustomobject]@{
        readiness = $readiness.contract_version
        consumer_contract = $readiness.summary.consumer_contract_version
    }
    summary = [pscustomobject]@{
        consumer_count = $preflights.Count
        allowed_count = @($preflights | Where-Object { $_.current_allowed -eq $true }).Count
        blocked_count = @($preflights | Where-Object { $_.current_allowed -eq $false }).Count
        unsafe_preflight_count = $unsafePreflightCount
        readiness_snapshot_classification = $readiness.summary.snapshot_classification
        evidence_fresh_all = $readiness.summary.evidence_fresh_all
        package_ready_for_external_gate = $readiness.summary.can_support_external_residency_review
        missing_evidence = @($readiness.summary.missing_evidence)
        pending_external_gates = @($readiness.summary.pending_external_gates)
        consumer_contract_validated = $readiness.summary.consumer_contract_validated
    }
    consumers = $preflights
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "consumer_preflight_is_read_only_and_cannot_authorize_actions"
    }
}

if (-not $consumerFound -and -not [string]::IsNullOrWhiteSpace($ConsumerId)) {
    $result.summary | Add-Member -MemberType NoteProperty -Name "error" -Value "unknown_consumer_id"
}

$blockedForExit = (
    $consumerFound -ne $true -or
    $result.summary.allowed_count -ne $result.summary.consumer_count -or
    $result.summary.unsafe_preflight_count -ne 0
)
$failOnBlockedExitCode = if ($consumerFound -ne $true) { 3 } elseif ($blockedForExit) { 2 } else { 0 }
$result | Add-Member -MemberType NoteProperty -Name "fail_on_blocked_exit_code" -Value $failOnBlockedExitCode

if ($Json) {
    $result | ConvertTo-Json -Depth 10
    if ($FailOnBlocked -and $failOnBlockedExitCode -ne 0) {
        exit $failOnBlockedExitCode
    }
    exit 0
}

Write-Host "Gemma remote consumer preflight"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "generated_at=$($result.generated_at) generated_at_utc=$($result.generated_at_utc)"
Write-Host "consumer_count=$($result.summary.consumer_count) allowed=$($result.summary.allowed_count) blocked=$($result.summary.blocked_count) unsafe_preflight_count=$($result.summary.unsafe_preflight_count)"
Write-Host "snapshot=$($result.summary.readiness_snapshot_classification) evidence_fresh_all=$($result.summary.evidence_fresh_all) missing_evidence=$($result.summary.missing_evidence -join ',')"
foreach ($preflight in $result.consumers) {
    Write-Host "  $($preflight.id): allowed=$($preflight.current_allowed) kind=$($preflight.entrypoint_kind) safe_command=$($preflight.safe_command_id) status=$($preflight.next_user_visible_status)"
}
Write-Host "fail_on_blocked_exit_code=$($result.fail_on_blocked_exit_code)"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
if ($FailOnBlocked -and $failOnBlockedExitCode -ne 0) {
    exit $failOnBlockedExitCode
}
