param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-evidence-package-plan.ps1"),
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

function Invoke-Plan {
    param([string]$InputRepoRoot)

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $InputRepoRoot -Json
    if ($LASTEXITCODE -ne 0) {
        throw "evidence package plan exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$plan = Invoke-Plan -InputRepoRoot $root

Assert-True ($plan.contract_version -eq "smartsteam.remote-gemma-unattended.evidence-package-plan.v1") "evidence package plan contract version mismatch"
Assert-True ($plan.read_only -eq $true) "evidence package plan must be read-only"
Assert-True ($plan.starts_process -eq $false) "evidence package plan must not start processes"
Assert-True ($plan.sends_prompt -eq $false) "evidence package plan must not send prompts"
Assert-True ($plan.touches_remote -eq $false) "evidence package plan must not touch remote"
Assert-True ($plan.writes_files -eq $false) "evidence package plan must not write files"
Assert-True ($plan.writes_model_weights -eq $false) "evidence package plan must not write model weights"
Assert-True ($plan.authorization.can_authorize_daemon -eq $false) "evidence package plan must not authorize daemon"
Assert-True ($plan.authorization.can_authorize_launch -eq $false) "evidence package plan must not authorize launch"
Assert-True ($plan.authorization.can_authorize_prompt -eq $false) "evidence package plan must not authorize prompt"
Assert-True ($plan.authorization.can_authorize_ssh -eq $false) "evidence package plan must not authorize ssh"
Assert-True ($plan.plan.operator_boundary.this_script_collects_evidence -eq $false) "plan must not claim to collect evidence"
Assert-True ($plan.plan.operator_boundary.this_script_writes_artifacts -eq $false) "plan must not claim to write artifacts"
Assert-True ($plan.plan.operator_boundary.approved_owner_flow_required_to_write_artifacts -eq $true) "plan must require approved owner flow to write artifacts"
Assert-True ($plan.plan.operator_boundary.ssh_requires_explicit_user_authorization -eq $true) "plan must require explicit SSH authorization"
Assert-True ($plan.plan.operator_boundary.prompt_launch_or_daemon_start_requires_separate_gate -eq $true) "plan must require separate prompt/launch gate"
Assert-True (@($plan.plan.snapshot_refresh_package.required_outputs).Count -ge 4) "snapshot package must list required outputs"
Assert-True (@($plan.plan.snapshot_refresh_package.source_commands | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "snapshot package source commands must stay read-only"
Assert-True ($plan.plan.observation_window_package.window_dir -eq "target\remote-gemma-observation-window") "observation window dir mismatch"
Assert-True ($plan.plan.observation_window_package.requirements.min_samples -ge 3) "observation window must require at least three samples"
Assert-True ($plan.plan.observation_window_package.requirements.min_span_minutes -ge 10) "observation window must require a span"
Assert-True (@($plan.plan.observation_window_package.required_files_per_sample).Count -eq 4) "observation window must define four files per sample"
Assert-True (@($plan.plan.observation_window_package.required_files_per_sample | Where-Object { [string]::IsNullOrWhiteSpace([string]$_.source_command_id) -or @($_.required_fields).Count -eq 0 }).Count -eq 0) "observation files must map to source command ids and required fields"
Assert-True (@($plan.plan.observation_window_package.source_commands | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "observation source commands must stay read-only"
Assert-True ($plan.plan.observation_window_package.acceptance.continuous_window_present -eq $true) "observation acceptance must require continuous window"
Assert-True ($plan.plan.observation_window_package.acceptance.authorization_still_false -eq $true) "observation acceptance must stay fail-closed"
Assert-True ($plan.plan.resource_window_package.window_dir -eq "target\remote-gemma-resource-window") "resource window dir mismatch"
Assert-True ($plan.plan.resource_window_package.requirements.min_samples -ge 3) "resource window must require at least three samples"
Assert-True ($plan.plan.resource_window_package.requirements.min_span_minutes -ge 10) "resource window must require a span"
Assert-True ($plan.plan.resource_window_package.requirements.min_available_memory_gb -ge 8) "resource window must require memory headroom"
Assert-True ($plan.plan.resource_window_package.requirements.approved_owner_flow_required -eq $true) "resource window must require approved owner flow"
Assert-True (@($plan.plan.resource_window_package.accepted_file_names).Count -ge 3) "resource window must list accepted file names"
Assert-True (@($plan.plan.resource_window_package.required_fields_per_sample | Where-Object { $_ -match "approved_owner_flow" }).Count -ge 1) "resource window must require approved_owner_flow"
Assert-True (@($plan.plan.resource_window_package.required_fields_per_sample | Where-Object { $_ -match "memory" }).Count -ge 1) "resource window must require memory evidence"
Assert-True (@($plan.plan.resource_window_package.required_fields_per_sample | Where-Object { $_ -match "metal|gpu" }).Count -ge 1) "resource window must require Metal/GPU evidence"
Assert-True ($plan.plan.resource_window_package.acceptance.resource_window_present -eq $true) "resource acceptance must require resource window"
Assert-True ($plan.plan.resource_window_package.acceptance.authorization_still_false -eq $true) "resource acceptance must stay fail-closed"
Assert-True ($plan.safety.source_unsafe_safe_command_count -eq 0) "plan source safe commands must be safe"
Assert-True ($plan.safety.source_unresolved_checklist_safe_command_count -eq 0) "plan source checklist commands must resolve"
Assert-True (@($plan.safety.forbidden_without_explicit_user_authorization | Where-Object { $_ -eq "ssh" }).Count -eq 1) "plan must forbid SSH by default"
Assert-True (@($plan.current_state.missing_evidence).Count -ge 0) "plan must expose current missing evidence"
Assert-True (@($plan.current_state.pending_external_gates).Count -ge 0) "plan must expose pending external gates"

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.evidence-package-plan-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        plan_contract = $plan.contract_version
        snapshot_classification = $plan.current_state.snapshot_classification
        evidence_fresh_all = $plan.current_state.evidence_fresh_all
        missing_evidence = @($plan.current_state.missing_evidence)
        pending_external_gates = @($plan.current_state.pending_external_gates)
        observation_required_file_count = @($plan.plan.observation_window_package.required_files_per_sample).Count
        resource_accepted_file_name_count = @($plan.plan.resource_window_package.accepted_file_names).Count
        source_safe_command_count = $plan.safety.source_safe_command_count
        source_unsafe_safe_command_count = $plan.safety.source_unsafe_safe_command_count
        source_unresolved_checklist_safe_command_count = $plan.safety.source_unresolved_checklist_safe_command_count
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "evidence_package_plan_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-evidence-package-plan selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "snapshot=$($result.summary.snapshot_classification) evidence_fresh_all=$($result.summary.evidence_fresh_all)"
Write-Host "observation_required_file_count=$($result.summary.observation_required_file_count) resource_accepted_file_name_count=$($result.summary.resource_accepted_file_name_count)"
Write-Host "source_safe_command_count=$($result.summary.source_safe_command_count) source_unsafe_safe_command_count=$($result.summary.source_unsafe_safe_command_count) source_unresolved_checklist_safe_command_count=$($result.summary.source_unresolved_checklist_safe_command_count)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
