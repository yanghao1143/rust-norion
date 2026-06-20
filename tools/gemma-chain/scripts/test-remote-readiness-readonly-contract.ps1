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

function Invoke-SelfTest {
    param([string]$ScriptPath)

    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath
    if ($LASTEXITCODE -ne 0) {
        throw "$ScriptPath exited with $LASTEXITCODE"
    }
    return ($output -join "`n")
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$snapshotScript = Join-Path $PSScriptRoot "read-remote-unattended-snapshot.ps1"
$observationScript = Join-Path $PSScriptRoot "read-remote-observation-window.ps1"
$resourceScript = Join-Path $PSScriptRoot "read-remote-resource-window.ps1"
$readinessContractScript = Join-Path $PSScriptRoot "read-remote-readiness-contract.ps1"
$gapReportScript = Join-Path $PSScriptRoot "read-remote-residency-gap-report.ps1"
$evidencePackagePlanScript = Join-Path $PSScriptRoot "read-remote-evidence-package-plan.ps1"
$evidencePackageStatusScript = Join-Path $PSScriptRoot "read-remote-evidence-package-status.ps1"
$ownerFlowHandoffScript = Join-Path $PSScriptRoot "read-remote-owner-flow-handoff.ps1"
$consumerPreflightScript = Join-Path $PSScriptRoot "read-remote-consumer-preflight.ps1"
$surfacePreflightScript = Join-Path $PSScriptRoot "read-remote-surface-preflight.ps1"
$linkBoundaryScript = Join-Path $PSScriptRoot "read-remote-link-boundary.ps1"
$dashboardStatusScript = Join-Path $PSScriptRoot "read-remote-dashboard-status.ps1"
$actionMatrixScript = Join-Path $PSScriptRoot "read-remote-action-matrix.ps1"
$evolutionLoopGuardScript = Join-Path $PSScriptRoot "read-remote-evolution-loop-guard.ps1"
$modelPoolGuardScript = Join-Path $PSScriptRoot "read-remote-model-pool-guard.ps1"
$contractManifestScript = Join-Path $PSScriptRoot "read-remote-contract-manifest.ps1"
$snapshotSelfTest = Join-Path $PSScriptRoot "test-read-remote-unattended-snapshot.ps1"
$observationSelfTest = Join-Path $PSScriptRoot "test-read-remote-observation-window.ps1"
$resourceSelfTest = Join-Path $PSScriptRoot "test-read-remote-resource-window.ps1"
$readinessContractSelfTest = Join-Path $PSScriptRoot "test-read-remote-readiness-contract.ps1"
$gapReportSelfTest = Join-Path $PSScriptRoot "test-read-remote-residency-gap-report.ps1"
$evidencePackagePlanSelfTest = Join-Path $PSScriptRoot "test-read-remote-evidence-package-plan.ps1"
$evidencePackageStatusSelfTest = Join-Path $PSScriptRoot "test-read-remote-evidence-package-status.ps1"
$ownerFlowHandoffSelfTest = Join-Path $PSScriptRoot "test-read-remote-owner-flow-handoff.ps1"
$consumerPreflightSelfTest = Join-Path $PSScriptRoot "test-read-remote-consumer-preflight.ps1"
$surfacePreflightSelfTest = Join-Path $PSScriptRoot "test-read-remote-surface-preflight.ps1"
$linkBoundarySelfTest = Join-Path $PSScriptRoot "test-read-remote-link-boundary.ps1"
$dashboardStatusSelfTest = Join-Path $PSScriptRoot "test-read-remote-dashboard-status.ps1"
$actionMatrixSelfTest = Join-Path $PSScriptRoot "test-read-remote-action-matrix.ps1"
$evolutionLoopGuardSelfTest = Join-Path $PSScriptRoot "test-read-remote-evolution-loop-guard.ps1"
$modelPoolGuardSelfTest = Join-Path $PSScriptRoot "test-read-remote-model-pool-guard.ps1"
$contractManifestSelfTest = Join-Path $PSScriptRoot "test-read-remote-contract-manifest.ps1"
$consumerContractSelfTest = Join-Path $PSScriptRoot "test-remote-consumer-contract.ps1"

$selfTestOutputs = [ordered]@{
    snapshot = Invoke-SelfTest -ScriptPath $snapshotSelfTest
    observation_window = Invoke-SelfTest -ScriptPath $observationSelfTest
    resource_window = Invoke-SelfTest -ScriptPath $resourceSelfTest
    readiness_contract = Invoke-SelfTest -ScriptPath $readinessContractSelfTest
    gap_report = Invoke-SelfTest -ScriptPath $gapReportSelfTest
    evidence_package_plan = Invoke-SelfTest -ScriptPath $evidencePackagePlanSelfTest
    evidence_package_status = Invoke-SelfTest -ScriptPath $evidencePackageStatusSelfTest
    owner_flow_handoff = Invoke-SelfTest -ScriptPath $ownerFlowHandoffSelfTest
    consumer_preflight = Invoke-SelfTest -ScriptPath $consumerPreflightSelfTest
    surface_preflight = Invoke-SelfTest -ScriptPath $surfacePreflightSelfTest
    link_boundary = Invoke-SelfTest -ScriptPath $linkBoundarySelfTest
    dashboard_status = Invoke-SelfTest -ScriptPath $dashboardStatusSelfTest
    action_matrix = Invoke-SelfTest -ScriptPath $actionMatrixSelfTest
    evolution_loop_guard = Invoke-SelfTest -ScriptPath $evolutionLoopGuardSelfTest
    model_pool_guard = Invoke-SelfTest -ScriptPath $modelPoolGuardSelfTest
    contract_manifest = Invoke-SelfTest -ScriptPath $contractManifestSelfTest
    consumer_contract = Invoke-SelfTest -ScriptPath $consumerContractSelfTest
}

$snapshot = Invoke-JsonScript -ScriptPath $snapshotScript -ScriptArgs @("-RepoRoot", $root)
$observation = Invoke-JsonScript -ScriptPath $observationScript -ScriptArgs @("-RepoRoot", $root)
$resource = Invoke-JsonScript -ScriptPath $resourceScript -ScriptArgs @("-RepoRoot", $root)
$consumerContract = Invoke-JsonScript -ScriptPath $consumerContractSelfTest -ScriptArgs @("-RepoRoot", $root)
$readinessContract = Invoke-JsonScript -ScriptPath $readinessContractScript -ScriptArgs @("-RepoRoot", $root)
$gapReport = Invoke-JsonScript -ScriptPath $gapReportScript -ScriptArgs @("-RepoRoot", $root)
$evidencePackagePlan = Invoke-JsonScript -ScriptPath $evidencePackagePlanScript -ScriptArgs @("-RepoRoot", $root)
$evidencePackageStatus = Invoke-JsonScript -ScriptPath $evidencePackageStatusScript -ScriptArgs @("-RepoRoot", $root)
$ownerFlowHandoff = Invoke-JsonScript -ScriptPath $ownerFlowHandoffScript -ScriptArgs @("-RepoRoot", $root)
$consumerPreflight = Invoke-JsonScript -ScriptPath $consumerPreflightScript -ScriptArgs @("-RepoRoot", $root)
$surfacePreflight = Invoke-JsonScript -ScriptPath $surfacePreflightScript -ScriptArgs @("-RepoRoot", $root)
$linkBoundary = Invoke-JsonScript -ScriptPath $linkBoundaryScript -ScriptArgs @("-RepoRoot", $root)
$dashboardStatus = Invoke-JsonScript -ScriptPath $dashboardStatusScript -ScriptArgs @("-RepoRoot", $root)
$actionMatrix = Invoke-JsonScript -ScriptPath $actionMatrixScript -ScriptArgs @("-RepoRoot", $root)
$evolutionLoopGuard = Invoke-JsonScript -ScriptPath $evolutionLoopGuardScript -ScriptArgs @("-RepoRoot", $root)
$modelPoolGuard = Invoke-JsonScript -ScriptPath $modelPoolGuardScript -ScriptArgs @("-RepoRoot", $root)
$contractManifest = Invoke-JsonScript -ScriptPath $contractManifestScript -ScriptArgs @("-RepoRoot", $root)

Assert-True ($snapshot.read_only -eq $true -and $snapshot.starts_process -eq $false -and $snapshot.sends_prompt -eq $false -and $snapshot.touches_remote -eq $false -and $snapshot.writes_files -eq $false) "snapshot reader must keep read-only contract"
Assert-True ($observation.read_only -eq $true -and $observation.starts_process -eq $false -and $observation.sends_prompt -eq $false -and $observation.touches_remote -eq $false -and $observation.writes_files -eq $false) "observation reader must keep read-only contract"
Assert-True ($resource.read_only -eq $true -and $resource.starts_process -eq $false -and $resource.sends_prompt -eq $false -and $resource.touches_remote -eq $false -and $resource.writes_files -eq $false -and $resource.writes_model_weights -eq $false) "resource reader must keep read-only contract"
Assert-True ($snapshot.authorization.can_authorize_daemon -eq $false -and $snapshot.authorization.can_authorize_launch -eq $false -and $snapshot.authorization.can_authorize_prompt -eq $false -and $snapshot.authorization.can_authorize_ssh -eq $false) "snapshot must fail closed"
Assert-True ($observation.authorization.can_authorize_daemon -eq $false -and $observation.authorization.can_authorize_launch -eq $false -and $observation.authorization.can_authorize_prompt -eq $false -and $observation.authorization.can_authorize_ssh -eq $false) "observation window must fail closed"
Assert-True ($resource.authorization.can_authorize_daemon -eq $false -and $resource.authorization.can_authorize_launch -eq $false -and $resource.authorization.can_authorize_prompt -eq $false -and $resource.authorization.can_authorize_ssh -eq $false) "resource window must fail closed"
Assert-True (@($snapshot.consumer_projection | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "snapshot consumer projection must fail closed"
Assert-True (@($snapshot.safe_next_read_only_commands | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "safe command list must stay read-only"
Assert-True (@($snapshot.safe_next_read_only_commands | Where-Object { $_.command -match '\bsmoke\b|\bStart\b|\bssh\b|ssh\.exe|plink|Start-Process|forge_cli_prompt|web_lab_prompt|backend_cli_direct_prompt|evolution_loop_prompt_round|model_pool_launch' }).Count -eq 0) "safe command list must not contain prompt, launch, or SSH actions"
Assert-True (@($snapshot.safe_next_read_only_commands | Where-Object { $_.id -eq "forge_daemon_start_check" -and $_.command -match "-StartCheck" }).Count -eq 1) "safe command list should include dry-run StartCheck only"
Assert-True (@($snapshot.safe_next_read_only_commands | Group-Object -Property id | Where-Object { $_.Count -ne 1 }).Count -eq 0) "safe command ids must be unique"
Assert-True (@($snapshot.evidence_checklist).Count -ge 6) "evidence checklist must be present"
Assert-True (@($snapshot.evidence_checklist | Where-Object {
    [string]::IsNullOrWhiteSpace([string]$_.id) -or
    [string]::IsNullOrWhiteSpace([string]$_.gap_id) -or
    [string]::IsNullOrWhiteSpace([string]$_.status) -or
    [string]::IsNullOrWhiteSpace([string]$_.required_evidence) -or
    [string]::IsNullOrWhiteSpace([string]$_.proof_source) -or
    [string]::IsNullOrWhiteSpace([string]$_.safe_command_id)
}).Count -eq 0) "each evidence checklist item must expose id, gap_id, status, required_evidence, proof_source, and safe_command_id"
Assert-True (@($snapshot.evidence_checklist | Where-Object { $_.blocks_authorization -ne $true }).Count -eq 0) "evidence checklist items must block authorization"
foreach ($item in @($snapshot.evidence_checklist)) {
    Assert-True (@($snapshot.safe_next_read_only_commands | Where-Object { $_.id -eq $item.safe_command_id }).Count -eq 1) "evidence checklist safe command $($item.safe_command_id) must exist"
}
Assert-True ($consumerContract.summary.consumer_allowed_count -eq 0) "consumer contract selftest must keep consumers blocked"
Assert-True ($consumerContract.summary.invalid_safe_command_count -eq 0) "consumer contract selftest must keep safe commands valid"
Assert-True ($readinessContract.read_only -eq $true -and $readinessContract.starts_process -eq $false -and $readinessContract.sends_prompt -eq $false -and $readinessContract.touches_remote -eq $false -and $readinessContract.writes_files -eq $false -and $readinessContract.writes_model_weights -eq $false) "readiness contract reader must keep read-only contract"
Assert-True ($readinessContract.authorization.can_authorize_daemon -eq $false -and $readinessContract.authorization.can_authorize_launch -eq $false -and $readinessContract.authorization.can_authorize_prompt -eq $false -and $readinessContract.authorization.can_authorize_ssh -eq $false) "readiness contract reader must fail closed"
Assert-True ($readinessContract.summary.consumer_contract_validated -eq $true) "readiness contract must validate consumer contract"
Assert-True ($readinessContract.summary.consumer_allowed_count -eq 0) "readiness contract must keep consumers blocked"
Assert-True ($readinessContract.summary.unsafe_safe_command_count -eq 0) "readiness contract must keep safe commands valid"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$readinessContract.generated_at_utc)) "readiness contract must expose generated_at_utc"
Assert-True ($null -ne $readinessContract.PSObject.Properties["source_status"]) "readiness contract must expose source_status"
Assert-True ($null -ne $readinessContract.source_status.snapshot.PSObject.Properties["evidence"]) "readiness contract source_status must expose snapshot evidence"
Assert-True ($readinessContract.summary.fresh_minutes -ge 0) "readiness contract must expose freshness window"
Assert-True (@($readinessContract.missing_evidence_actions).Count -eq @($readinessContract.summary.missing_evidence).Count) "readiness contract should map each missing evidence item to an action"
Assert-True (@($readinessContract.missing_evidence_actions | Where-Object { $_.blocks_authorization -ne $true }).Count -eq 0) "missing evidence actions must block authorization"
Assert-True (@($readinessContract.pending_external_gate_actions).Count -eq @($readinessContract.summary.pending_external_gates).Count) "readiness contract should map each pending external gate to an action"
Assert-True (@($readinessContract.pending_external_gate_actions | Where-Object { $_.blocks_authorization -ne $true }).Count -eq 0) "pending external gate actions must block authorization"
foreach ($action in @($readinessContract.missing_evidence_actions)) {
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$action.id)) "missing evidence action must have id"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$action.status)) "missing evidence action $($action.id) must have status"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$action.description)) "missing evidence action $($action.id) must have description"
    Assert-True (@($action.safe_command_ids).Count -eq @($action.safe_commands).Count) "missing evidence action $($action.id) must keep safe_command_ids array aligned"
    foreach ($command in @($action.safe_commands)) {
        Assert-True ($command.read_only -eq $true -and $command.starts_process -eq $false -and $command.sends_prompt -eq $false -and $command.touches_remote -eq $false -and $command.writes_files -eq $false) "missing evidence action $($action.id) safe command must stay read-only"
    }
}
foreach ($action in @($readinessContract.pending_external_gate_actions)) {
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$action.id)) "pending external gate action must have id"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$action.status)) "pending external gate action $($action.id) must have status"
    Assert-True (@($action.safe_command_ids).Count -eq @($action.safe_commands).Count) "pending external gate action $($action.id) must keep safe_command_ids array aligned"
    foreach ($command in @($action.safe_commands)) {
        Assert-True ($command.read_only -eq $true -and $command.starts_process -eq $false -and $command.sends_prompt -eq $false -and $command.touches_remote -eq $false -and $command.writes_files -eq $false) "pending external gate action $($action.id) safe command must stay read-only"
    }
}
Assert-True ($gapReport.read_only -eq $true -and $gapReport.starts_process -eq $false -and $gapReport.sends_prompt -eq $false -and $gapReport.touches_remote -eq $false -and $gapReport.writes_files -eq $false -and $gapReport.writes_model_weights -eq $false) "gap report must keep read-only contract"
Assert-True ($gapReport.authorization.can_authorize_daemon -eq $false -and $gapReport.authorization.can_authorize_launch -eq $false -and $gapReport.authorization.can_authorize_prompt -eq $false -and $gapReport.authorization.can_authorize_ssh -eq $false) "gap report must fail closed"
Assert-True ($gapReport.decision.authorized -eq $false) "gap report decision must not authorize actions"
Assert-True ($gapReport.snapshot_claims.historical_only -eq $true) "gap report snapshot claims must be historical-only"
Assert-True ($gapReport.safety.unsafe_safe_command_count -eq 0) "gap report must keep safe commands valid"
Assert-True ($gapReport.safety.unresolved_checklist_safe_command_count -eq 0) "gap report must resolve checklist safe commands"
Assert-True ($gapReport.safety.consumer_allowed_count -eq 0) "gap report must keep consumers blocked"
Assert-True (@($gapReport.checklist).Count -eq @($readinessContract.evidence_checklist).Count) "gap report checklist should mirror readiness checklist"
Assert-True (@($gapReport.consumers | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "gap report consumers must fail closed"
Assert-True ($evidencePackagePlan.read_only -eq $true -and $evidencePackagePlan.starts_process -eq $false -and $evidencePackagePlan.sends_prompt -eq $false -and $evidencePackagePlan.touches_remote -eq $false -and $evidencePackagePlan.writes_files -eq $false -and $evidencePackagePlan.writes_model_weights -eq $false) "evidence package plan must keep read-only contract"
Assert-True ($evidencePackagePlan.authorization.can_authorize_daemon -eq $false -and $evidencePackagePlan.authorization.can_authorize_launch -eq $false -and $evidencePackagePlan.authorization.can_authorize_prompt -eq $false -and $evidencePackagePlan.authorization.can_authorize_ssh -eq $false) "evidence package plan must fail closed"
Assert-True ($evidencePackagePlan.plan.operator_boundary.this_script_collects_evidence -eq $false) "evidence package plan must not collect evidence"
Assert-True ($evidencePackagePlan.plan.operator_boundary.this_script_writes_artifacts -eq $false) "evidence package plan must not write artifacts"
Assert-True ($evidencePackagePlan.plan.operator_boundary.approved_owner_flow_required_to_write_artifacts -eq $true) "evidence package plan must require approved owner flow"
Assert-True (@($evidencePackagePlan.plan.observation_window_package.required_files_per_sample).Count -eq 4) "evidence package plan must define observation sample files"
Assert-True ($evidencePackagePlan.plan.resource_window_package.requirements.approved_owner_flow_required -eq $true) "evidence package plan must define approved resource owner flow"
Assert-True ($evidencePackagePlan.safety.source_unsafe_safe_command_count -eq 0) "evidence package plan must inherit safe commands"
Assert-True ($evidencePackagePlan.safety.source_unresolved_checklist_safe_command_count -eq 0) "evidence package plan must inherit resolved checklist commands"
Assert-True ($evidencePackageStatus.read_only -eq $true -and $evidencePackageStatus.starts_process -eq $false -and $evidencePackageStatus.sends_prompt -eq $false -and $evidencePackageStatus.touches_remote -eq $false -and $evidencePackageStatus.writes_files -eq $false -and $evidencePackageStatus.writes_model_weights -eq $false) "evidence package status must keep read-only contract"
Assert-True ($evidencePackageStatus.authorization.can_authorize_daemon -eq $false -and $evidencePackageStatus.authorization.can_authorize_launch -eq $false -and $evidencePackageStatus.authorization.can_authorize_prompt -eq $false -and $evidencePackageStatus.authorization.can_authorize_ssh -eq $false) "evidence package status must fail closed"
Assert-True (@($evidencePackageStatus.package_items).Count -ge 4) "evidence package status must expose package items"
Assert-True ($evidencePackageStatus.summary.consumer_allowed_count -eq 0) "evidence package status must keep consumers blocked"
Assert-True ($evidencePackageStatus.summary.unsafe_safe_command_count -eq 0) "evidence package status must keep safe commands valid"
if ($evidencePackageStatus.summary.package_ready_for_external_gate -eq $true) {
    Assert-True ($evidencePackageStatus.summary.evidence_fresh_all -eq $true) "ready evidence package requires fresh snapshot"
    Assert-True ($evidencePackageStatus.summary.continuous_window_present -eq $true) "ready evidence package requires observation window"
    Assert-True ($evidencePackageStatus.summary.resource_window_present -eq $true) "ready evidence package requires resource window"
}
Assert-True ($ownerFlowHandoff.read_only -eq $true -and $ownerFlowHandoff.starts_process -eq $false -and $ownerFlowHandoff.sends_prompt -eq $false -and $ownerFlowHandoff.touches_remote -eq $false -and $ownerFlowHandoff.writes_files -eq $false -and $ownerFlowHandoff.writes_model_weights -eq $false) "owner-flow handoff must keep read-only contract"
Assert-True ($ownerFlowHandoff.authorization.can_authorize_daemon -eq $false -and $ownerFlowHandoff.authorization.can_authorize_launch -eq $false -and $ownerFlowHandoff.authorization.can_authorize_prompt -eq $false -and $ownerFlowHandoff.authorization.can_authorize_ssh -eq $false) "owner-flow handoff must fail closed"
Assert-True ($ownerFlowHandoff.operator_boundary.this_script_collects_evidence -eq $false -and $ownerFlowHandoff.operator_boundary.this_script_writes_artifacts -eq $false -and $ownerFlowHandoff.operator_boundary.this_script_touches_remote -eq $false) "owner-flow handoff must not collect, write, or touch remote"
Assert-True (@($ownerFlowHandoff.handoff_items | Where-Object { ($_.writes_artifacts -eq $true -or $_.touches_remote -eq $true) -and $_.requires_explicit_user_authorization -ne $true }).Count -eq 0) "owner-flow handoff artifact/remote items must require explicit authorization"
Assert-True (@($ownerFlowHandoff.read_only_verifiers | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "owner-flow handoff verifiers must stay read-only"
Assert-True ($consumerPreflight.read_only -eq $true -and $consumerPreflight.starts_process -eq $false -and $consumerPreflight.sends_prompt -eq $false -and $consumerPreflight.touches_remote -eq $false -and $consumerPreflight.writes_files -eq $false -and $consumerPreflight.writes_model_weights -eq $false) "consumer preflight must keep read-only contract"
Assert-True ($consumerPreflight.authorization.can_authorize_daemon -eq $false -and $consumerPreflight.authorization.can_authorize_launch -eq $false -and $consumerPreflight.authorization.can_authorize_prompt -eq $false -and $consumerPreflight.authorization.can_authorize_ssh -eq $false) "consumer preflight must fail closed"
Assert-True ($consumerPreflight.summary.allowed_count -eq 0) "consumer preflight must keep consumers blocked"
Assert-True ($consumerPreflight.summary.unsafe_preflight_count -eq 0) "consumer preflight must keep safe commands valid"
Assert-True (@($consumerPreflight.consumers | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "consumer preflight consumers must fail closed"
Assert-True ($surfacePreflight.read_only -eq $true -and $surfacePreflight.starts_process -eq $false -and $surfacePreflight.sends_prompt -eq $false -and $surfacePreflight.touches_remote -eq $false -and $surfacePreflight.writes_files -eq $false -and $surfacePreflight.writes_model_weights -eq $false) "surface preflight must keep read-only contract"
Assert-True ($surfacePreflight.authorization.can_authorize_daemon -eq $false -and $surfacePreflight.authorization.can_authorize_launch -eq $false -and $surfacePreflight.authorization.can_authorize_prompt -eq $false -and $surfacePreflight.authorization.can_authorize_ssh -eq $false) "surface preflight must fail closed"
Assert-True ($surfacePreflight.status.consumer_allowed_count -eq 0) "surface preflight must keep consumers blocked"
Assert-True ($surfacePreflight.status.unsafe_safe_command_count -eq 0) "surface preflight must keep safe commands valid"
Assert-True (@($surfacePreflight.consumers | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "surface preflight consumers must fail closed"
Assert-True ($linkBoundary.read_only -eq $true -and $linkBoundary.starts_process -eq $false -and $linkBoundary.sends_prompt -eq $false -and $linkBoundary.touches_remote -eq $false -and $linkBoundary.writes_files -eq $false -and $linkBoundary.writes_model_weights -eq $false) "link boundary must keep read-only contract"
Assert-True ($linkBoundary.authorization.can_authorize_daemon -eq $false -and $linkBoundary.authorization.can_authorize_launch -eq $false -and $linkBoundary.authorization.can_authorize_prompt -eq $false -and $linkBoundary.authorization.can_authorize_ssh -eq $false) "link boundary must fail closed"
Assert-True ($linkBoundary.summary.realtime_ports_verified_by_this_script -eq $false) "link boundary must not claim live port verification"
Assert-True ($linkBoundary.summary.historical_snapshot_authorizes_current_residency -eq $false) "link boundary must not authorize from historical snapshots"
Assert-True (@($linkBoundary.consumer_projection | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "link boundary consumers must fail closed"
Assert-True ($dashboardStatus.read_only -eq $true -and $dashboardStatus.starts_process -eq $false -and $dashboardStatus.sends_prompt -eq $false -and $dashboardStatus.touches_remote -eq $false -and $dashboardStatus.writes_files -eq $false -and $dashboardStatus.writes_model_weights -eq $false) "dashboard status must keep read-only contract"
Assert-True ($dashboardStatus.authorization.can_authorize_daemon -eq $false -and $dashboardStatus.authorization.can_authorize_launch -eq $false -and $dashboardStatus.authorization.can_authorize_prompt -eq $false -and $dashboardStatus.authorization.can_authorize_ssh -eq $false) "dashboard status must fail closed"
Assert-True ($dashboardStatus.status.consumer_allowed_count -eq 0) "dashboard status must keep consumers blocked"
Assert-True (@($dashboardStatus.consumers | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "dashboard status consumers must fail closed"
Assert-True (@($dashboardStatus.dashboard_cards | Where-Object { $_.id -eq "action_lock" -and $_.value -eq "blocked" }).Count -eq 1) "dashboard status must expose blocked action lock"
Assert-True ($dashboardStatus.topology.external_remote_ports.observed_by_this_script -eq $false) "dashboard status must not claim remote port observation"
Assert-True ($actionMatrix.read_only -eq $true -and $actionMatrix.starts_process -eq $false -and $actionMatrix.sends_prompt -eq $false -and $actionMatrix.touches_remote -eq $false -and $actionMatrix.writes_files -eq $false -and $actionMatrix.writes_model_weights -eq $false) "action matrix must keep read-only contract"
Assert-True ($actionMatrix.authorization.can_authorize_daemon -eq $false -and $actionMatrix.authorization.can_authorize_launch -eq $false -and $actionMatrix.authorization.can_authorize_prompt -eq $false -and $actionMatrix.authorization.can_authorize_ssh -eq $false) "action matrix must fail closed"
Assert-True ($actionMatrix.summary.action_count -eq 7) "action matrix must expose 7 actions"
Assert-True ($actionMatrix.summary.allowed_count -eq 0) "action matrix must keep actions blocked"
Assert-True ($actionMatrix.summary.ui_enabled_count -eq 0) "action matrix must keep UI actions disabled"
Assert-True ($actionMatrix.summary.cli_executable_count -eq 0) "action matrix must keep CLI actions non-executable"
Assert-True ($evolutionLoopGuard.read_only -eq $true -and $evolutionLoopGuard.starts_process -eq $false -and $evolutionLoopGuard.sends_prompt -eq $false -and $evolutionLoopGuard.touches_remote -eq $false -and $evolutionLoopGuard.writes_files -eq $false -and $evolutionLoopGuard.writes_model_weights -eq $false) "evolution loop guard must keep read-only contract"
Assert-True ($evolutionLoopGuard.authorization.can_authorize_daemon -eq $false -and $evolutionLoopGuard.authorization.can_authorize_launch -eq $false -and $evolutionLoopGuard.authorization.can_authorize_prompt -eq $false -and $evolutionLoopGuard.authorization.can_authorize_ssh -eq $false) "evolution loop guard must fail closed"
Assert-True ($evolutionLoopGuard.summary.may_send_prompt_round -eq $false) "evolution loop guard must block prompt rounds"
Assert-True ($evolutionLoopGuard.summary.may_start_or_resume_daemon -eq $false) "evolution loop guard must block daemon"
Assert-True ($evolutionLoopGuard.summary.may_enter_resident_loop -eq $false) "evolution loop guard must block resident loop"
Assert-True ($evolutionLoopGuard.guard_exit_code -eq 2) "evolution loop guard must advertise blocked exit code"
Assert-True ($modelPoolGuard.read_only -eq $true -and $modelPoolGuard.starts_process -eq $false -and $modelPoolGuard.sends_prompt -eq $false -and $modelPoolGuard.touches_remote -eq $false -and $modelPoolGuard.writes_files -eq $false -and $modelPoolGuard.writes_model_weights -eq $false) "model-pool guard must keep read-only contract"
Assert-True ($modelPoolGuard.authorization.can_authorize_daemon -eq $false -and $modelPoolGuard.authorization.can_authorize_launch -eq $false -and $modelPoolGuard.authorization.can_authorize_prompt -eq $false -and $modelPoolGuard.authorization.can_authorize_ssh -eq $false) "model-pool guard must fail closed"
Assert-True ($modelPoolGuard.summary.may_launch_worker -eq $false) "model-pool guard must block worker launch"
Assert-True ($modelPoolGuard.summary.may_expand_pool -eq $false) "model-pool guard must block expansion"
Assert-True ($modelPoolGuard.summary.may_reuse_snapshot_as_current_capacity -eq $false) "model-pool guard must not reuse historical snapshot as current capacity"
Assert-True ($modelPoolGuard.guard_exit_code -eq 2) "model-pool guard must advertise blocked exit code"
Assert-True ($contractManifest.read_only -eq $true -and $contractManifest.starts_process -eq $false -and $contractManifest.sends_prompt -eq $false -and $contractManifest.touches_remote -eq $false -and $contractManifest.writes_files -eq $false -and $contractManifest.writes_model_weights -eq $false) "contract manifest must keep read-only contract"
Assert-True ($contractManifest.authorization.can_authorize_daemon -eq $false -and $contractManifest.authorization.can_authorize_launch -eq $false -and $contractManifest.authorization.can_authorize_prompt -eq $false -and $contractManifest.authorization.can_authorize_ssh -eq $false) "contract manifest must fail closed"
Assert-True ($contractManifest.safety.blocked_exit_code -eq 2 -and $contractManifest.safety.unknown_consumer_exit_code -eq 3) "contract manifest exit code convention must match"
Assert-True (@($contractManifest.readers | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "contract manifest readers must stay safe"
Assert-True (@($contractManifest.selftests | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "contract manifest selftests must stay safe"

$missingEvidence = @()
if ($snapshot.summary.evidence_fresh_all -ne $true) {
    $missingEvidence += "fresh_snapshot"
}
if ($snapshot.residency_decision.can_proceed_to_resident_loop -ne $true) {
    # The resident-loop gate is an external gate, not missing evidence for the readiness package.
}
if ($observation.summary.continuous_window_present -ne $true) {
    $missingEvidence += "continuous_port_worker_window"
}
if ($resource.summary.resource_window_present -ne $true) {
    $missingEvidence += "remote_resource_headroom_window"
}

$canSupportExternalReview = (
    $snapshot.summary.evidence_fresh_all -eq $true -and
    $observation.summary.continuous_window_present -eq $true -and
    $resource.summary.resource_window_present -eq $true
)

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.readonly-contract-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    selftests = [pscustomobject]@{
        snapshot = $selfTestOutputs.snapshot
        observation_window = $selfTestOutputs.observation_window
        resource_window = $selfTestOutputs.resource_window
        readiness_contract = $selfTestOutputs.readiness_contract
        gap_report = $selfTestOutputs.gap_report
        evidence_package_plan = $selfTestOutputs.evidence_package_plan
        evidence_package_status = $selfTestOutputs.evidence_package_status
        owner_flow_handoff = $selfTestOutputs.owner_flow_handoff
        consumer_preflight = $selfTestOutputs.consumer_preflight
        surface_preflight = $selfTestOutputs.surface_preflight
        link_boundary = $selfTestOutputs.link_boundary
        dashboard_status = $selfTestOutputs.dashboard_status
        action_matrix = $selfTestOutputs.action_matrix
        evolution_loop_guard = $selfTestOutputs.evolution_loop_guard
        model_pool_guard = $selfTestOutputs.model_pool_guard
        contract_manifest = $selfTestOutputs.contract_manifest
    }
    summary = [pscustomobject]@{
        snapshot_classification = $snapshot.residency_decision.classification
        evidence_fresh_all = $snapshot.summary.evidence_fresh_all
        readiness_generated_at_utc = $readinessContract.generated_at_utc
        readiness_max_evidence_age_seconds = $readinessContract.summary.max_evidence_age_seconds
        observation_window_status = $observation.summary.status
        continuous_window_present = $observation.summary.continuous_window_present
        resource_window_status = $resource.summary.status
        resource_window_present = $resource.summary.resource_window_present
        consumer_allowed_count = @($snapshot.consumer_projection | Where-Object { $_.current_allowed -eq $true }).Count
        consumer_contract_validated = $true
        readiness_contract_validated = $true
        gap_report_validated = $true
        evidence_package_plan_validated = $true
        evidence_package_status_validated = $true
        owner_flow_handoff_validated = $true
        consumer_preflight_validated = $true
        surface_preflight_validated = $true
        link_boundary_validated = $true
        dashboard_status_validated = $true
        action_matrix_validated = $true
        evolution_loop_guard_validated = $true
        model_pool_guard_validated = $true
        contract_manifest_validated = $true
        contract_manifest_reader_count = @($contractManifest.readers).Count
        contract_manifest_selftest_count = @($contractManifest.selftests).Count
        missing_evidence_action_count = @($readinessContract.missing_evidence_actions).Count
        pending_external_gate_action_count = @($readinessContract.pending_external_gate_actions).Count
        unsafe_safe_command_count = @($snapshot.safe_next_read_only_commands | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count
        can_support_external_residency_review = $canSupportExternalReview
        missing_evidence = $missingEvidence
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "contract_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "Gemma remote readiness read-only contract selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "snapshot=$($result.summary.snapshot_classification) evidence_fresh_all=$($result.summary.evidence_fresh_all)"
Write-Host "observation_window=$($result.summary.observation_window_status) continuous_window_present=$($result.summary.continuous_window_present)"
Write-Host "resource_window=$($result.summary.resource_window_status) resource_window_present=$($result.summary.resource_window_present)"
Write-Host "consumer_allowed_count=$($result.summary.consumer_allowed_count) unsafe_safe_command_count=$($result.summary.unsafe_safe_command_count)"
Write-Host "can_support_external_residency_review=$($result.summary.can_support_external_residency_review) missing_evidence=$($result.summary.missing_evidence -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
