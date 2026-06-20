param(
    [string]$RepoRoot = "",
    [string]$SnapshotJson = "target\evolution\strict-status.json",
    [int]$MaxSnapshotAgeSeconds = 900,
    [switch]$JsonStatus,
    [switch]$FailOnNotReady,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if ($RepoRoot.Trim().Length -eq 0) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
}

if ($Help) {
    Write-Host "Verify a previously written SmartSteam strict unattended evolution status snapshot."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\evolution-loop\verify-strict-status-snapshot.cmd [-JsonStatus] [-FailOnNotReady]"
    Write-Host "  .\tools\evolution-loop\verify-strict-status-snapshot.cmd [-SnapshotJson PATH] [-MaxSnapshotAgeSeconds 900]"
    Write-Host ""
    Write-Host "Contracts:"
    Write-Host "  read_only=true"
    Write-Host "  starts_process=false"
    Write-Host "  sends_prompt=false"
    exit 0
}

function Resolve-RepoPath {
    param([string]$Path)

    if ($Path.Trim().Length -eq 0) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return Join-Path $RepoRoot $Path
}

function Has-Property {
    param(
        [object]$Object,
        [string]$Name
    )

    return $null -ne $Object -and $Object.PSObject.Properties.Name -contains $Name
}

function Get-NestedValue {
    param(
        [object]$Value,
        [string[]]$Path
    )

    $cursor = $Value
    foreach ($segment in $Path) {
        if ($null -eq $cursor -or -not (Has-Property -Object $cursor -Name $segment)) {
            return $null
        }
        $cursor = $cursor.$segment
    }
    return $cursor
}

$SnapshotPath = Resolve-RepoPath $SnapshotJson
$exists = Test-Path -LiteralPath $SnapshotPath -PathType Leaf
$ageSeconds = $null
$snapshot = $null
$parseError = ""
if ($exists) {
    $item = Get-Item -LiteralPath $SnapshotPath
    $ageSeconds = [int][Math]::Max(0, ((Get-Date) - $item.LastWriteTime).TotalSeconds)
    try {
        $snapshot = Get-Content -LiteralPath $SnapshotPath -Raw | ConvertFrom-Json
    } catch {
        $parseError = $_.Exception.Message
    }
}

$strict = if ($null -ne $snapshot -and (Has-Property -Object $snapshot -Name "strict_unattended_evolution")) { [bool]$snapshot.strict_unattended_evolution } else { $false }
$ledgerSource = if ($null -ne $snapshot -and (Has-Property -Object $snapshot -Name "ledger_source")) { [string]$snapshot.ledger_source } else { "" }
$statusReady = if ($null -ne $snapshot) { [bool](Get-NestedValue -Value $snapshot -Path @("readiness", "ready")) } else { $false }
$latestRound = Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "round")
$activeRound = Get-NestedValue -Value $snapshot -Path @("daemon", "active_round")
$daemonState = Get-NestedValue -Value $snapshot -Path @("daemon", "activity_state")
$latestCase = Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "case")
$latestSuccess = Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "success")
$feedbackApplied = Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "feedback_applied")
$selfImprovePassed = Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "self_improve_passed")
$validationPassed = Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "validation_passed")
$validationSource = Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "validation_command_source")
$validationStatusCode = Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "validation_status_code")
$helperRoles = @(Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "helper_stage_roles") | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
$helperContractComplete = Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "helper_stage_contract_complete")
$testGatePassed = Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "test_gate_passed")
$testGateVerdict = Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "test_gate_verdict")
$testGateValidationSafety = Get-NestedValue -Value $snapshot -Path @("ledger", "latest", "test_gate_validation_command_safety")
$remoteChainReady = Get-NestedValue -Value $snapshot -Path @("remote_chain", "ready")
$backendModel = Get-NestedValue -Value $snapshot -Path @("backend", "gemma_runtime_model")
$backendBusy = Get-NestedValue -Value $snapshot -Path @("backend", "engine_busy")
$successRate = Get-NestedValue -Value $snapshot -Path @("ledger", "success_rate")
$totalRecords = Get-NestedValue -Value $snapshot -Path @("ledger", "total_records")
$nextRoundDecision = Get-NestedValue -Value $snapshot -Path @("next_round_decision")
$nextRoundDecisionReportV1 = Get-NestedValue -Value $snapshot -Path @("next_round_decision_report_v1")
$nextRoundDownstreamStatusConsumersV1 = Get-NestedValue -Value $snapshot -Path @("next_round_downstream_status_consumers_v1")
$nextRoundDecisionDisplayState = Get-NestedValue -Value $snapshot -Path @("next_round_decision", "display_state")
$safeToWaitCurrentRoundActive = Get-NestedValue -Value $snapshot -Path @("next_round_decision", "safe_to_wait_current_round_active")
$safeToContinueAfterCurrentRound = Get-NestedValue -Value $snapshot -Path @("next_round_decision", "safe_to_continue_after_current_round")
$operatorAttentionBlocked = Get-NestedValue -Value $snapshot -Path @("next_round_decision", "operator_attention_blocked")
if ($null -eq $nextRoundDecisionDisplayState) {
    $nextRoundDecisionDisplayState = Get-NestedValue -Value $snapshot -Path @("next_round_decision_report_v1", "display_state")
}
if ($null -eq $safeToWaitCurrentRoundActive) {
    $safeToWaitCurrentRoundActive = Get-NestedValue -Value $snapshot -Path @("next_round_decision_report_v1", "safe_to_wait_current_round_active")
}
if ($null -eq $safeToContinueAfterCurrentRound) {
    $safeToContinueAfterCurrentRound = Get-NestedValue -Value $snapshot -Path @("next_round_decision_report_v1", "safe_to_continue_after_current_round")
}
if ($null -eq $operatorAttentionBlocked) {
    $operatorAttentionBlocked = Get-NestedValue -Value $snapshot -Path @("next_round_decision_report_v1", "operator_attention_blocked")
}

$failures = @()
if (-not $exists) {
    $failures += "snapshot_missing"
}
if ($parseError.Trim().Length -gt 0) {
    $failures += "snapshot_invalid"
}
if ($exists -and $null -ne $ageSeconds -and [int]$MaxSnapshotAgeSeconds -gt 0 -and $ageSeconds -gt [int]$MaxSnapshotAgeSeconds) {
    $failures += "snapshot_stale"
}
if ($null -ne $snapshot -and -not $strict) {
    $failures += "not_strict_unattended_evolution"
}
if ($null -ne $snapshot -and $ledgerSource -ne "daemon") {
    $failures += "ledger_source_not_daemon"
}
if ($null -ne $snapshot -and -not $statusReady) {
    $failures += "strict_status_not_ready"
}

$verification = [pscustomobject][ordered]@{
    schema_version = 1
    contract_version = "smartsteam.evolution-loop.strict-status-snapshot.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    snapshot = [pscustomobject][ordered]@{
        path = $SnapshotPath
        exists = $exists
        age_seconds = $ageSeconds
        max_age_seconds = [int]$MaxSnapshotAgeSeconds
        parse_error = $parseError
    }
    strict_unattended_evolution = $strict
    ledger_source = $ledgerSource
    readiness = [pscustomobject][ordered]@{
        ready = $failures.Count -eq 0
        failures = $failures
        strict_status_ready = $statusReady
    }
    latest_round = $latestRound
    active_round = $activeRound
    daemon_state = $daemonState
    next_round_decision = $nextRoundDecision
    next_round_decision_report_v1 = $nextRoundDecisionReportV1
    next_round_downstream_status_consumers_v1 = $nextRoundDownstreamStatusConsumersV1
    summary = [pscustomobject][ordered]@{
        latest_round = $latestRound
        active_round = $activeRound
        daemon_state = $daemonState
        next_round_decision_report_v1 = $nextRoundDecisionReportV1
        next_round_downstream_status_consumers_v1 = $nextRoundDownstreamStatusConsumersV1
        next_round_decision_display_state = $nextRoundDecisionDisplayState
        safe_to_wait_current_round_active = $safeToWaitCurrentRoundActive
        safe_to_continue_after_current_round = $safeToContinueAfterCurrentRound
        operator_attention_blocked = $operatorAttentionBlocked
        latest_case = $latestCase
        latest_success = $latestSuccess
        feedback_applied = $feedbackApplied
        self_improve_passed = $selfImprovePassed
        validation_passed = $validationPassed
        validation_source = $validationSource
        validation_status_code = $validationStatusCode
        helper_stage_roles = @($helperRoles)
        helper_stage_role_count = @($helperRoles).Count
        helper_stage_contract_complete = $helperContractComplete
        test_gate_passed = $testGatePassed
        test_gate_verdict = $testGateVerdict
        test_gate_validation_command_safety = $testGateValidationSafety
        remote_chain_ready = $remoteChainReady
        backend_model = $backendModel
        backend_busy = $backendBusy
        success_rate = $successRate
        total_records = $totalRecords
    }
    next_step = if ($failures.Count -eq 0) { "strict status snapshot is ready" } else { "refresh strict status snapshot or inspect failures" }
}

$exitCode = if ($FailOnNotReady -and -not $verification.readiness.ready) { 2 } else { 0 }
if ($JsonStatus) {
    $verification | ConvertTo-Json -Depth 8
    exit $exitCode
}

Write-Host "SmartSteam strict status snapshot verification"
Write-Host "read_only=true starts_process=false sends_prompt=false"
Write-Host "snapshot=$SnapshotPath exists=$exists age_seconds=$ageSeconds max_age_seconds=$MaxSnapshotAgeSeconds"
Write-Host "ready=$($verification.readiness.ready) failures=$($verification.readiness.failures -join ',') strict_status_ready=$statusReady strict=$strict ledger_source=$ledgerSource latest_round=$latestRound active_round=$activeRound daemon_state=$daemonState"
Write-Host "summary: latest_case=$latestCase feedback=$feedbackApplied self_improve=$selfImprovePassed validation_passed=$validationPassed helper_roles=$($helperRoles -join ',') test_gate=$testGateVerdict test_gate_safety=$testGateValidationSafety next_round_decision=$nextRoundDecisionDisplayState remote_chain_ready=$remoteChainReady backend_model=$backendModel"
Write-Host "next_step: $($verification.next_step)"
exit $exitCode
