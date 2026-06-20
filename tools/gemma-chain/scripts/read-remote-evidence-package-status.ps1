param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
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

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$generatedAt = Get-Date
$generatedAtUtc = $generatedAt.ToUniversalTime()

$readinessScript = Join-Path $PSScriptRoot "read-remote-readiness-contract.ps1"
$planScript = Join-Path $PSScriptRoot "read-remote-evidence-package-plan.ps1"
$observationScript = Join-Path $PSScriptRoot "read-remote-observation-window.ps1"
$resourceScript = Join-Path $PSScriptRoot "read-remote-resource-window.ps1"
$snapshotScript = Join-Path $PSScriptRoot "read-remote-unattended-snapshot.ps1"

$readiness = Invoke-JsonScript -ScriptPath $readinessScript -ScriptArgs @("-RepoRoot", $root)
$plan = Invoke-JsonScript -ScriptPath $planScript -ScriptArgs @("-RepoRoot", $root)
$observation = Invoke-JsonScript -ScriptPath $observationScript -ScriptArgs @("-RepoRoot", $root)
$resource = Invoke-JsonScript -ScriptPath $resourceScript -ScriptArgs @("-RepoRoot", $root)
$snapshot = Invoke-JsonScript -ScriptPath $snapshotScript -ScriptArgs @("-RepoRoot", $root)

$freshSnapshotReady = ($readiness.summary.evidence_fresh_all -eq $true)
$observationReady = ($observation.summary.continuous_window_present -eq $true)
$resourceReady = ($resource.summary.resource_window_present -eq $true)
$latestLedgerRound = $snapshot.latest_ledger.round
$latestLedgerSuccess = $snapshot.latest_ledger.success
$unattendedReportRounds = $snapshot.unattended.rounds
$unattendedReportSuccess = $snapshot.unattended.success
$reportLedgerRoundMismatch = ($null -ne $latestLedgerRound -and $null -ne $unattendedReportRounds -and [int]$latestLedgerRound -gt [int]$unattendedReportRounds)
$latestLedgerFailed = ($latestLedgerSuccess -eq $false)
$requiresUnattendedReportRefresh = ($reportLedgerRoundMismatch -or $latestLedgerFailed)
$unattendedReportLedgerReady = (-not $requiresUnattendedReportRefresh)
$safePlanReady = (
    $plan.safety.source_unsafe_safe_command_count -eq 0 -and
    $plan.safety.source_unresolved_checklist_safe_command_count -eq 0 -and
    $plan.authorization.can_authorize_daemon -eq $false -and
    $plan.authorization.can_authorize_launch -eq $false -and
    $plan.authorization.can_authorize_prompt -eq $false -and
    $plan.authorization.can_authorize_ssh -eq $false
)
$packageReadyForExternalGate = (
    $freshSnapshotReady -and
    $unattendedReportLedgerReady -and
    $observationReady -and
    $resourceReady -and
    $safePlanReady -and
    $readiness.summary.consumer_contract_validated -eq $true -and
    $readiness.summary.consumer_allowed_count -eq 0 -and
    $readiness.summary.unsafe_safe_command_count -eq 0
)

$items = @(
    [pscustomobject]@{
        id = "fresh_snapshot"
        ready = $freshSnapshotReady
        status = if ($freshSnapshotReady) { "ready_for_external_gate" } else { "missing_or_stale" }
        required_evidence = "Fresh snapshot evidence for model cache, chain status, unattended report, and ledger."
        verifier_command = ".\tools\gemma-chain\scripts\read-remote-unattended-snapshot.ps1 -Json"
        proof_source = "readiness.summary.evidence_fresh_all and readiness.source_status.snapshot.evidence[]"
    },
    [pscustomobject]@{
        id = "unattended_report_ledger_consistency"
        ready = $unattendedReportLedgerReady
        status = if ($unattendedReportLedgerReady) { "consistent" } else { "report_refresh_required" }
        required_evidence = "Unattended report and latest ledger agree on current round/success state; latest ledger must not be failed."
        verifier_command = ".\tools\gemma-chain\scripts\read-remote-evidence-freshness.ps1 -Json"
        proof_source = "snapshot.unattended.rounds/success and snapshot.latest_ledger.round/success"
    },
    [pscustomobject]@{
        id = "continuous_port_worker_window"
        ready = $observationReady
        status = $observation.summary.status
        required_evidence = "Continuous local observation window for model API/backend/Web Lab and worker health."
        verifier_command = $plan.plan.observation_window_package.verifier_command
        proof_source = "observation.summary.continuous_window_present and observation.samples[]"
    },
    [pscustomobject]@{
        id = "remote_resource_headroom_window"
        ready = $resourceReady
        status = $resource.summary.status
        required_evidence = "Approved-owner-flow remote resource/headroom window for memory and Metal/GPU availability."
        verifier_command = $plan.plan.resource_window_package.verifier_command
        proof_source = "resource.summary.resource_window_present and resource.samples[]"
    },
    [pscustomobject]@{
        id = "fail_closed_contracts"
        ready = $safePlanReady
        status = if ($safePlanReady) { "validated" } else { "contract_review_required" }
        required_evidence = "Reader contracts remain read-only and cannot authorize daemon, launch, prompt, or SSH."
        verifier_command = ".\tools\gemma-chain\scripts\test-remote-readiness-readonly-contract.ps1"
        proof_source = "plan.safety and readiness.authorization"
    }
)

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.evidence-package-status.v1"
    generated_at = $generatedAt.ToString("yyyy-MM-dd HH:mm:ss zzz")
    generated_at_utc = $generatedAtUtc.ToString("o")
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    source_contracts = [pscustomobject]@{
        readiness = $readiness.contract_version
        evidence_package_plan = $plan.contract_version
        observation_window = $observation.contract_version
        resource_window = $resource.contract_version
        snapshot = $snapshot.contract_version
    }
    summary = [pscustomobject]@{
        package_ready_for_external_gate = $packageReadyForExternalGate
        can_support_external_residency_review = $readiness.summary.can_support_external_residency_review
        snapshot_classification = $readiness.summary.snapshot_classification
        evidence_fresh_all = $freshSnapshotReady
        unattended_report_rounds = $unattendedReportRounds
        unattended_report_success = $unattendedReportSuccess
        latest_ledger_round = $latestLedgerRound
        latest_ledger_success = $latestLedgerSuccess
        report_ledger_round_mismatch = $reportLedgerRoundMismatch
        latest_ledger_failed = $latestLedgerFailed
        requires_unattended_report_refresh = $requiresUnattendedReportRefresh
        observation_window_status = $observation.summary.status
        continuous_window_present = $observationReady
        resource_window_status = $resource.summary.status
        resource_window_present = $resourceReady
        fail_closed_contracts_ok = $safePlanReady
        consumer_contract_validated = $readiness.summary.consumer_contract_validated
        consumer_allowed_count = $readiness.summary.consumer_allowed_count
        unsafe_safe_command_count = $readiness.summary.unsafe_safe_command_count
        missing_evidence = @($readiness.summary.missing_evidence)
        pending_external_gates = @($readiness.summary.pending_external_gates)
        ready_item_count = @($items | Where-Object { $_.ready -eq $true }).Count
        total_item_count = $items.Count
    }
    package_items = $items
    artifact_locations = [pscustomobject]@{
        observation_window_dir = $plan.plan.observation_window_package.window_dir
        resource_window_dir = $plan.plan.resource_window_package.window_dir
        snapshot_outputs = $plan.plan.snapshot_refresh_package.required_outputs
    }
    next_read_only_verifiers = @(
        [pscustomobject]@{ id = "readiness_contract"; command = ".\tools\gemma-chain\scripts\read-remote-readiness-contract.ps1 -Json" },
        [pscustomobject]@{ id = "gap_report"; command = ".\tools\gemma-chain\scripts\read-remote-residency-gap-report.ps1 -Json" },
        [pscustomobject]@{ id = "evidence_package_plan"; command = ".\tools\gemma-chain\scripts\read-remote-evidence-package-plan.ps1 -Json" },
        [pscustomobject]@{ id = "observation_window"; command = ".\tools\gemma-chain\scripts\read-remote-observation-window.ps1 -Json" },
        [pscustomobject]@{ id = "resource_window"; command = ".\tools\gemma-chain\scripts\read-remote-resource-window.ps1 -Json" }
    )
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "evidence_package_status_is_read_only_and_cannot_authorize_actions"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 10
    exit 0
}

Write-Host "Gemma remote evidence package status"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "generated_at=$($result.generated_at) generated_at_utc=$($result.generated_at_utc)"
Write-Host "package_ready_for_external_gate=$($result.summary.package_ready_for_external_gate) can_support_external_residency_review=$($result.summary.can_support_external_residency_review)"
Write-Host "snapshot=$($result.summary.snapshot_classification) evidence_fresh_all=$($result.summary.evidence_fresh_all)"
Write-Host "report_rounds=$($result.summary.unattended_report_rounds) latest_ledger_round=$($result.summary.latest_ledger_round) latest_ledger_success=$($result.summary.latest_ledger_success) requires_unattended_report_refresh=$($result.summary.requires_unattended_report_refresh)"
Write-Host "observation_window=$($result.summary.observation_window_status) continuous_window_present=$($result.summary.continuous_window_present)"
Write-Host "resource_window=$($result.summary.resource_window_status) resource_window_present=$($result.summary.resource_window_present)"
Write-Host "ready_items=$($result.summary.ready_item_count)/$($result.summary.total_item_count) missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
