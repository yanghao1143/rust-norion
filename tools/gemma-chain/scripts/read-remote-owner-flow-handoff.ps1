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

$statusScript = Join-Path $PSScriptRoot "read-remote-evidence-package-status.ps1"
$planScript = Join-Path $PSScriptRoot "read-remote-evidence-package-plan.ps1"
$gapReportScript = Join-Path $PSScriptRoot "read-remote-residency-gap-report.ps1"

$status = Invoke-JsonScript -ScriptPath $statusScript -ScriptArgs @("-RepoRoot", $root)
$plan = Invoke-JsonScript -ScriptPath $planScript -ScriptArgs @("-RepoRoot", $root)
$gapReport = Invoke-JsonScript -ScriptPath $gapReportScript -ScriptArgs @("-RepoRoot", $root)

$readOnlyVerifiers = @($status.next_read_only_verifiers | ForEach-Object {
    [pscustomobject]@{
        id = $_.id
        command = $_.command
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    }
})

$handoffItems = @(
    [pscustomobject]@{
        id = "review_current_status"
        stage = 0
        owner = "this_window_or_any_read_only_consumer"
        status = "available_now"
        requires_explicit_user_authorization = $false
        writes_artifacts = $false
        touches_remote = $false
        starts_process = $false
        sends_prompt = $false
        goal = "Read current fail-closed evidence package status before planning any owner-flow work."
        verifier_ids = @("evidence_package_status", "readiness_contract", "gap_report")
    },
    [pscustomobject]@{
        id = "collect_fresh_snapshot_package"
        stage = 1
        owner = "approved_local_status_owner"
        status = if ($status.summary.evidence_fresh_all -eq $true) { "already_satisfied" } else { "needs_owner_flow" }
        requires_explicit_user_authorization = $true
        writes_artifacts = $true
        touches_remote = $false
        starts_process = $false
        sends_prompt = $false
        goal = "Refresh or otherwise prove current chain/model-cache/unattended snapshot evidence inside the freshness window."
        artifact_outputs = @($status.artifact_locations.snapshot_outputs)
        verifier_ids = @("readiness_contract", "evidence_package_status")
        acceptance = $plan.plan.snapshot_refresh_package.acceptance
    },
    [pscustomobject]@{
        id = "collect_observation_window_package"
        stage = 2
        owner = "approved_local_status_owner"
        status = if ($status.summary.continuous_window_present -eq $true) { "already_satisfied" } else { "needs_owner_flow" }
        requires_explicit_user_authorization = $true
        writes_artifacts = $true
        touches_remote = $false
        starts_process = $false
        sends_prompt = $false
        goal = "Write local observation-window samples proving model API/backend/Web Lab and worker health over time."
        artifact_outputs = @($plan.plan.observation_window_package.window_dir)
        required_files_per_sample = $plan.plan.observation_window_package.required_files_per_sample
        verifier_ids = @("observation_window", "evidence_package_status")
        acceptance = $plan.plan.observation_window_package.acceptance
    },
    [pscustomobject]@{
        id = "collect_resource_window_package"
        stage = 3
        owner = "approved_remote_resource_owner"
        status = if ($status.summary.resource_window_present -eq $true) { "already_satisfied" } else { "needs_explicit_remote_authorization" }
        requires_explicit_user_authorization = $true
        writes_artifacts = $true
        touches_remote = $true
        starts_process = $false
        sends_prompt = $false
        goal = "Write approved-owner-flow remote resource/headroom samples for memory and Metal/GPU availability."
        artifact_outputs = @($plan.plan.resource_window_package.window_dir)
        accepted_file_names = $plan.plan.resource_window_package.accepted_file_names
        required_fields_per_sample = $plan.plan.resource_window_package.required_fields_per_sample
        verifier_ids = @("resource_window", "evidence_package_status")
        acceptance = $plan.plan.resource_window_package.acceptance
    },
    [pscustomobject]@{
        id = "external_residency_gate_review"
        stage = 4
        owner = "residency_gate_owner"
        status = if ($status.summary.package_ready_for_external_gate -eq $true) { "ready_for_external_gate_review" } else { "blocked_until_package_ready" }
        requires_explicit_user_authorization = $true
        writes_artifacts = $false
        touches_remote = $false
        starts_process = $false
        sends_prompt = $false
        goal = "Review duplicate-runner, daemon/report-gate, prompt/launch gates, and user authorization after evidence package is complete."
        verifier_ids = @("readiness_contract", "evidence_package_status")
        acceptance = [pscustomobject]@{
            package_ready_for_external_gate = $true
            pending_external_gates = @("residency_external_gate")
            authorization_still_false_until_gate_passes = $true
        }
    }
)

$blockedItems = @($handoffItems | Where-Object { $_.status -like "needs*" -or $_.status -like "blocked*" })
$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.owner-flow-handoff.v1"
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
        evidence_package_status = $status.contract_version
        evidence_package_plan = $plan.contract_version
        residency_gap_report = $gapReport.contract_version
    }
    summary = [pscustomobject]@{
        package_ready_for_external_gate = $status.summary.package_ready_for_external_gate
        can_support_external_residency_review = $status.summary.can_support_external_residency_review
        ready_item_count = $status.summary.ready_item_count
        total_item_count = $status.summary.total_item_count
        missing_evidence = @($status.summary.missing_evidence)
        pending_external_gates = @($status.summary.pending_external_gates)
        blocked_handoff_item_count = $blockedItems.Count
        next_owner_flow_item_ids = @($blockedItems | ForEach-Object { $_.id })
    }
    operator_boundary = [pscustomobject]@{
        this_script_collects_evidence = $false
        this_script_writes_artifacts = $false
        this_script_touches_remote = $false
        this_script_authorizes_actions = $false
        explicit_user_authorization_required_for_artifact_writes = $true
        explicit_user_authorization_required_for_remote_touch = $true
        explicit_user_authorization_required_for_prompt_launch_or_daemon_start = $true
    }
    handoff_items = $handoffItems
    read_only_verifiers = $readOnlyVerifiers
    artifact_locations = $status.artifact_locations
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "owner_flow_handoff_is_read_only_and_cannot_authorize_actions"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 12
    exit 0
}

Write-Host "Gemma remote owner-flow handoff"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "generated_at=$($result.generated_at) generated_at_utc=$($result.generated_at_utc)"
Write-Host "package_ready_for_external_gate=$($result.summary.package_ready_for_external_gate) ready_items=$($result.summary.ready_item_count)/$($result.summary.total_item_count)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "next_owner_flow_item_ids=$($result.summary.next_owner_flow_item_ids -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
