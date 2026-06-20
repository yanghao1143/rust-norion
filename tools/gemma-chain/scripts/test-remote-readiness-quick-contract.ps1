param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

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

function Assert-ReadOnlyFailClosed {
    param(
        [object]$Value,
        [string]$Name
    )

    Assert-True ($Value.read_only -eq $true) "$Name must be read-only"
    Assert-True ($Value.starts_process -eq $false) "$Name must not start processes"
    Assert-True ($Value.sends_prompt -eq $false) "$Name must not send prompts"
    Assert-True ($Value.touches_remote -eq $false) "$Name must not touch remote"
    Assert-True ($Value.writes_files -eq $false) "$Name must not write files"
    if ($null -ne $Value.PSObject.Properties["writes_model_weights"]) {
        Assert-True ($Value.writes_model_weights -eq $false) "$Name must not write model weights"
    }
    Assert-True ($Value.authorization.can_authorize_daemon -eq $false) "$Name must not authorize daemon"
    Assert-True ($Value.authorization.can_authorize_launch -eq $false) "$Name must not authorize launch"
    Assert-True ($Value.authorization.can_authorize_prompt -eq $false) "$Name must not authorize prompt"
    Assert-True ($Value.authorization.can_authorize_ssh -eq $false) "$Name must not authorize ssh"
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$readinessScript = Join-Path $PSScriptRoot "read-remote-readiness-contract.ps1"
$gapReportScript = Join-Path $PSScriptRoot "read-remote-residency-gap-report.ps1"
$packageStatusScript = Join-Path $PSScriptRoot "read-remote-evidence-package-status.ps1"
$ownerFlowHandoffScript = Join-Path $PSScriptRoot "read-remote-owner-flow-handoff.ps1"
$consumerPreflightScript = Join-Path $PSScriptRoot "read-remote-consumer-preflight.ps1"
$surfacePreflightScript = Join-Path $PSScriptRoot "read-remote-surface-preflight.ps1"
$linkBoundaryScript = Join-Path $PSScriptRoot "read-remote-link-boundary.ps1"
$dashboardStatusScript = Join-Path $PSScriptRoot "read-remote-dashboard-status.ps1"
$actionMatrixScript = Join-Path $PSScriptRoot "read-remote-action-matrix.ps1"
$evolutionLoopGuardScript = Join-Path $PSScriptRoot "read-remote-evolution-loop-guard.ps1"
$modelPoolGuardScript = Join-Path $PSScriptRoot "read-remote-model-pool-guard.ps1"
$contractManifestScript = Join-Path $PSScriptRoot "read-remote-contract-manifest.ps1"

$readiness = Invoke-JsonScript -ScriptPath $readinessScript -ScriptArgs @("-RepoRoot", $root)
$gapReport = Invoke-JsonScript -ScriptPath $gapReportScript -ScriptArgs @("-RepoRoot", $root)
$packageStatus = Invoke-JsonScript -ScriptPath $packageStatusScript -ScriptArgs @("-RepoRoot", $root)
$ownerFlowHandoff = Invoke-JsonScript -ScriptPath $ownerFlowHandoffScript -ScriptArgs @("-RepoRoot", $root)
$consumerPreflight = Invoke-JsonScript -ScriptPath $consumerPreflightScript -ScriptArgs @("-RepoRoot", $root)
$surfacePreflight = Invoke-JsonScript -ScriptPath $surfacePreflightScript -ScriptArgs @("-RepoRoot", $root)
$linkBoundary = Invoke-JsonScript -ScriptPath $linkBoundaryScript -ScriptArgs @("-RepoRoot", $root)
$dashboardStatus = Invoke-JsonScript -ScriptPath $dashboardStatusScript -ScriptArgs @("-RepoRoot", $root)
$actionMatrix = Invoke-JsonScript -ScriptPath $actionMatrixScript -ScriptArgs @("-RepoRoot", $root)
$evolutionLoopGuard = Invoke-JsonScript -ScriptPath $evolutionLoopGuardScript -ScriptArgs @("-RepoRoot", $root)
$modelPoolGuard = Invoke-JsonScript -ScriptPath $modelPoolGuardScript -ScriptArgs @("-RepoRoot", $root)
$contractManifest = Invoke-JsonScript -ScriptPath $contractManifestScript -ScriptArgs @("-RepoRoot", $root)

Assert-ReadOnlyFailClosed -Value $readiness -Name "readiness contract"
Assert-ReadOnlyFailClosed -Value $gapReport -Name "residency gap report"
Assert-ReadOnlyFailClosed -Value $packageStatus -Name "evidence package status"
Assert-ReadOnlyFailClosed -Value $ownerFlowHandoff -Name "owner-flow handoff"
Assert-ReadOnlyFailClosed -Value $consumerPreflight -Name "consumer preflight"
Assert-ReadOnlyFailClosed -Value $surfacePreflight -Name "surface preflight"
Assert-ReadOnlyFailClosed -Value $linkBoundary -Name "link boundary"
Assert-ReadOnlyFailClosed -Value $dashboardStatus -Name "dashboard status"
Assert-ReadOnlyFailClosed -Value $actionMatrix -Name "action matrix"
Assert-ReadOnlyFailClosed -Value $evolutionLoopGuard -Name "evolution loop guard"
Assert-ReadOnlyFailClosed -Value $modelPoolGuard -Name "model-pool guard"
Assert-ReadOnlyFailClosed -Value $contractManifest -Name "contract manifest"

Assert-True ($readiness.summary.reader_contracts_ok -eq $true) "readiness reader contracts must be ok"
Assert-True ($readiness.summary.authorization_fail_closed -eq $true) "readiness authorization must fail closed"
Assert-True ($readiness.summary.consumer_contract_validated -eq $true) "consumer contract must be valid"
Assert-True ($readiness.summary.consumer_allowed_count -eq 0) "consumers must remain blocked"
Assert-True ($readiness.summary.unsafe_safe_command_count -eq 0) "safe commands must remain safe"
Assert-True ($gapReport.safety.unsafe_safe_command_count -eq 0) "gap report safe commands must remain safe"
Assert-True ($gapReport.safety.unresolved_checklist_safe_command_count -eq 0) "gap report checklist commands must resolve"
Assert-True ($packageStatus.summary.consumer_allowed_count -eq 0) "package status consumers must remain blocked"
Assert-True ($packageStatus.summary.unsafe_safe_command_count -eq 0) "package status safe commands must remain safe"
Assert-True ($ownerFlowHandoff.operator_boundary.this_script_collects_evidence -eq $false) "handoff must not collect evidence"
Assert-True ($ownerFlowHandoff.operator_boundary.this_script_writes_artifacts -eq $false) "handoff must not write artifacts"
Assert-True ($ownerFlowHandoff.operator_boundary.this_script_touches_remote -eq $false) "handoff must not touch remote"
Assert-True ($ownerFlowHandoff.operator_boundary.this_script_authorizes_actions -eq $false) "handoff must not authorize actions"
Assert-True (@($ownerFlowHandoff.handoff_items | Where-Object { ($_.writes_artifacts -eq $true -or $_.touches_remote -eq $true) -and $_.requires_explicit_user_authorization -ne $true }).Count -eq 0) "handoff write/remote items must require explicit authorization"
Assert-True (@($ownerFlowHandoff.read_only_verifiers | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "handoff verifiers must be read-only"
Assert-True ($consumerPreflight.summary.allowed_count -eq 0) "consumer preflight must keep consumers blocked"
Assert-True ($consumerPreflight.summary.unsafe_preflight_count -eq 0) "consumer preflight must keep safe commands valid"
Assert-True (@($consumerPreflight.consumers | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "consumer preflight consumers must fail closed"
Assert-True (@($consumerPreflight.consumers | Where-Object { $_.safe_command_resolved -ne $true -or $_.safe_command_safe -ne $true }).Count -eq 0) "consumer preflight safe commands must resolve and stay safe"
Assert-True ($surfacePreflight.status.consumer_allowed_count -eq 0) "surface preflight must keep consumers blocked"
Assert-True ($surfacePreflight.status.unsafe_safe_command_count -eq 0) "surface preflight must keep safe commands valid"
Assert-True (@($surfacePreflight.consumers | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "surface preflight consumers must fail closed"
Assert-True ($linkBoundary.summary.realtime_ports_verified_by_this_script -eq $false) "link boundary must not claim live port verification"
Assert-True ($linkBoundary.summary.historical_snapshot_authorizes_current_residency -eq $false) "link boundary must not authorize from historical snapshots"
Assert-True (@($linkBoundary.consumer_projection | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "link boundary consumers must fail closed"
Assert-True ($dashboardStatus.status.consumer_allowed_count -eq 0) "dashboard status must keep consumers blocked"
Assert-True (@($dashboardStatus.consumers | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "dashboard status consumers must fail closed"
Assert-True (@($dashboardStatus.dashboard_cards | Where-Object { $_.id -eq "action_lock" -and $_.value -eq "blocked" }).Count -eq 1) "dashboard status must expose blocked action lock"
Assert-True ($dashboardStatus.topology.external_remote_ports.observed_by_this_script -eq $false) "dashboard status must not claim remote port observation"
Assert-True ($actionMatrix.summary.action_count -eq 7) "action matrix must expose 7 actions"
Assert-True ($actionMatrix.summary.allowed_count -eq 0) "action matrix must keep actions blocked"
Assert-True ($actionMatrix.summary.ui_enabled_count -eq 0) "action matrix must keep UI actions disabled"
Assert-True ($actionMatrix.summary.cli_executable_count -eq 0) "action matrix must keep CLI actions non-executable"
Assert-True ($evolutionLoopGuard.summary.may_send_prompt_round -eq $false) "evolution loop guard must block prompt rounds"
Assert-True ($evolutionLoopGuard.summary.may_start_or_resume_daemon -eq $false) "evolution loop guard must block daemon"
Assert-True ($evolutionLoopGuard.summary.may_enter_resident_loop -eq $false) "evolution loop guard must block resident loop"
Assert-True ($evolutionLoopGuard.guard_exit_code -eq 2) "evolution loop guard must advertise blocked exit code"
Assert-True ($modelPoolGuard.summary.may_launch_worker -eq $false) "model-pool guard must block worker launch"
Assert-True ($modelPoolGuard.summary.may_expand_pool -eq $false) "model-pool guard must block expansion"
Assert-True ($modelPoolGuard.summary.may_reuse_snapshot_as_current_capacity -eq $false) "model-pool guard must not reuse historical snapshot as current capacity"
Assert-True ($modelPoolGuard.guard_exit_code -eq 2) "model-pool guard must advertise blocked exit code"
Assert-True ($contractManifest.safety.blocked_exit_code -eq 2 -and $contractManifest.safety.unknown_consumer_exit_code -eq 3) "contract manifest exit code convention must match"
Assert-True (@($contractManifest.readers | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "contract manifest readers must stay safe"

$readinessMissing = @($readiness.summary.missing_evidence | Sort-Object)
$statusMissing = @($packageStatus.summary.missing_evidence | Sort-Object)
$handoffMissing = @($ownerFlowHandoff.summary.missing_evidence | Sort-Object)
Assert-True (($readinessMissing -join "|") -eq ($statusMissing -join "|")) "readiness and package status missing_evidence must match"
Assert-True (($readinessMissing -join "|") -eq ($handoffMissing -join "|")) "readiness and owner-flow handoff missing_evidence must match"
Assert-True (($readinessMissing -join "|") -eq ((@($consumerPreflight.summary.missing_evidence | Sort-Object)) -join "|")) "readiness and consumer preflight missing_evidence must match"
Assert-True (($readinessMissing -join "|") -eq ((@($surfacePreflight.status.missing_evidence | Sort-Object)) -join "|")) "readiness and surface preflight missing_evidence must match"
Assert-True ($packageStatus.summary.package_ready_for_external_gate -eq $ownerFlowHandoff.summary.package_ready_for_external_gate) "package status and handoff readiness must match"
Assert-True ($packageStatus.summary.package_ready_for_external_gate -eq $surfacePreflight.status.package_ready_for_external_gate) "package status and surface preflight readiness must match"
Assert-True ($packageStatus.summary.ready_item_count -eq $ownerFlowHandoff.summary.ready_item_count) "ready item counts must match"
Assert-True ($packageStatus.summary.total_item_count -eq $ownerFlowHandoff.summary.total_item_count) "total item counts must match"
if ($packageStatus.summary.package_ready_for_external_gate -eq $true) {
    Assert-True ($packageStatus.summary.evidence_fresh_all -eq $true) "ready package must have fresh snapshot"
    Assert-True ($packageStatus.summary.continuous_window_present -eq $true) "ready package must have observation window"
    Assert-True ($packageStatus.summary.resource_window_present -eq $true) "ready package must have resource window"
}

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.quick-readiness-contract-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        readiness_contract = $readiness.contract_version
        gap_report_contract = $gapReport.contract_version
        evidence_package_status_contract = $packageStatus.contract_version
        owner_flow_handoff_contract = $ownerFlowHandoff.contract_version
        consumer_preflight_contract = $consumerPreflight.contract_version
        surface_preflight_contract = $surfacePreflight.contract_version
        link_boundary_contract = $linkBoundary.contract_version
        dashboard_status_contract = $dashboardStatus.contract_version
        action_matrix_contract = $actionMatrix.contract_version
        evolution_loop_guard_contract = $evolutionLoopGuard.contract_version
        model_pool_guard_contract = $modelPoolGuard.contract_version
        contract_manifest_contract = $contractManifest.contract_version
        snapshot_classification = $readiness.summary.snapshot_classification
        evidence_fresh_all = $readiness.summary.evidence_fresh_all
        observation_window_status = $readiness.summary.observation_window_status
        continuous_window_present = $readiness.summary.continuous_window_present
        resource_window_status = $readiness.summary.resource_window_status
        resource_window_present = $readiness.summary.resource_window_present
        package_ready_for_external_gate = $packageStatus.summary.package_ready_for_external_gate
        can_support_external_residency_review = $readiness.summary.can_support_external_residency_review
        ready_item_count = $packageStatus.summary.ready_item_count
        total_item_count = $packageStatus.summary.total_item_count
        missing_evidence = @($readiness.summary.missing_evidence)
        pending_external_gates = @($readiness.summary.pending_external_gates)
        next_owner_flow_item_ids = @($ownerFlowHandoff.summary.next_owner_flow_item_ids)
        consumer_allowed_count = $readiness.summary.consumer_allowed_count
        unsafe_safe_command_count = $readiness.summary.unsafe_safe_command_count
        consumer_preflight_count = $consumerPreflight.summary.consumer_count
        surface_preflight_consumer_count = @($surfacePreflight.consumers).Count
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "quick_contract_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "Gemma remote readiness quick contract selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "package_ready_for_external_gate=$($result.summary.package_ready_for_external_gate) ready_items=$($result.summary.ready_item_count)/$($result.summary.total_item_count)"
Write-Host "snapshot=$($result.summary.snapshot_classification) evidence_fresh_all=$($result.summary.evidence_fresh_all)"
Write-Host "observation_window=$($result.summary.observation_window_status) continuous_window_present=$($result.summary.continuous_window_present)"
Write-Host "resource_window=$($result.summary.resource_window_status) resource_window_present=$($result.summary.resource_window_present)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
