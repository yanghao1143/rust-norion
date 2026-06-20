param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-owner-flow-handoff.ps1"),
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

function Invoke-Handoff {
    param([string]$InputRepoRoot)

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $InputRepoRoot -Json
    if ($LASTEXITCODE -ne 0) {
        throw "owner-flow handoff exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$handoff = Invoke-Handoff -InputRepoRoot $root

Assert-True ($handoff.contract_version -eq "smartsteam.remote-gemma-unattended.owner-flow-handoff.v1") "owner-flow handoff contract version mismatch"
Assert-True ($handoff.read_only -eq $true) "owner-flow handoff must be read-only"
Assert-True ($handoff.starts_process -eq $false) "owner-flow handoff must not start processes"
Assert-True ($handoff.sends_prompt -eq $false) "owner-flow handoff must not send prompts"
Assert-True ($handoff.touches_remote -eq $false) "owner-flow handoff must not touch remote"
Assert-True ($handoff.writes_files -eq $false) "owner-flow handoff must not write files"
Assert-True ($handoff.writes_model_weights -eq $false) "owner-flow handoff must not write model weights"
Assert-True ($handoff.authorization.can_authorize_daemon -eq $false) "owner-flow handoff must not authorize daemon"
Assert-True ($handoff.authorization.can_authorize_launch -eq $false) "owner-flow handoff must not authorize launch"
Assert-True ($handoff.authorization.can_authorize_prompt -eq $false) "owner-flow handoff must not authorize prompt"
Assert-True ($handoff.authorization.can_authorize_ssh -eq $false) "owner-flow handoff must not authorize ssh"
Assert-True ($handoff.operator_boundary.this_script_collects_evidence -eq $false) "handoff must not collect evidence"
Assert-True ($handoff.operator_boundary.this_script_writes_artifacts -eq $false) "handoff must not write artifacts"
Assert-True ($handoff.operator_boundary.this_script_touches_remote -eq $false) "handoff must not touch remote"
Assert-True ($handoff.operator_boundary.this_script_authorizes_actions -eq $false) "handoff must not authorize actions"
Assert-True ($handoff.operator_boundary.explicit_user_authorization_required_for_artifact_writes -eq $true) "handoff must require artifact-write authorization"
Assert-True ($handoff.operator_boundary.explicit_user_authorization_required_for_remote_touch -eq $true) "handoff must require remote-touch authorization"
Assert-True ($handoff.operator_boundary.explicit_user_authorization_required_for_prompt_launch_or_daemon_start -eq $true) "handoff must require prompt/launch authorization"
Assert-True (@($handoff.handoff_items).Count -ge 5) "handoff must expose staged items"
Assert-True (@($handoff.handoff_items | Where-Object { [string]::IsNullOrWhiteSpace([string]$_.id) -or [string]::IsNullOrWhiteSpace([string]$_.owner) -or [string]::IsNullOrWhiteSpace([string]$_.status) -or [string]::IsNullOrWhiteSpace([string]$_.goal) }).Count -eq 0) "handoff items must expose id, owner, status, and goal"
Assert-True (@($handoff.handoff_items | Where-Object { ($_.writes_artifacts -eq $true -or $_.touches_remote -eq $true) -and $_.requires_explicit_user_authorization -ne $true }).Count -eq 0) "artifact-writing or remote-touching handoff items must require explicit authorization"
Assert-True (@($handoff.handoff_items | Where-Object { $_.starts_process -eq $true -or $_.sends_prompt -eq $true }).Count -eq 0) "handoff items must not describe prompt or process-start execution"
Assert-True (@($handoff.handoff_items | Where-Object { $_.id -eq "collect_fresh_snapshot_package" }).Count -eq 1) "handoff must include fresh snapshot item"
Assert-True (@($handoff.handoff_items | Where-Object { $_.id -eq "collect_observation_window_package" }).Count -eq 1) "handoff must include observation window item"
Assert-True (@($handoff.handoff_items | Where-Object { $_.id -eq "collect_resource_window_package" }).Count -eq 1) "handoff must include resource window item"
Assert-True (@($handoff.handoff_items | Where-Object { $_.id -eq "external_residency_gate_review" }).Count -eq 1) "handoff must include external residency gate item"
Assert-True (@($handoff.read_only_verifiers).Count -ge 5) "handoff must expose read-only verifiers"
Assert-True (@($handoff.read_only_verifiers | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "handoff verifiers must stay read-only"
Assert-True (@($handoff.read_only_verifiers | Where-Object { $_.command -match '\bssh\b|ssh\.exe|plink|\bStart\b|Start-Process|prompt_round|model_pool_launch' }).Count -eq 0) "handoff verifiers must not contain SSH, launch, or prompt actions"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$handoff.artifact_locations.observation_window_dir)) "handoff must expose observation artifact dir"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$handoff.artifact_locations.resource_window_dir)) "handoff must expose resource artifact dir"
Assert-True (@($handoff.artifact_locations.snapshot_outputs).Count -ge 4) "handoff must expose snapshot outputs"

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.owner-flow-handoff-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        handoff_contract = $handoff.contract_version
        package_ready_for_external_gate = $handoff.summary.package_ready_for_external_gate
        ready_item_count = $handoff.summary.ready_item_count
        total_item_count = $handoff.summary.total_item_count
        blocked_handoff_item_count = $handoff.summary.blocked_handoff_item_count
        next_owner_flow_item_ids = @($handoff.summary.next_owner_flow_item_ids)
        missing_evidence = @($handoff.summary.missing_evidence)
        pending_external_gates = @($handoff.summary.pending_external_gates)
        read_only_verifier_count = @($handoff.read_only_verifiers).Count
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "owner_flow_handoff_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-owner-flow-handoff selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "package_ready_for_external_gate=$($result.summary.package_ready_for_external_gate) ready_items=$($result.summary.ready_item_count)/$($result.summary.total_item_count)"
Write-Host "blocked_handoff_item_count=$($result.summary.blocked_handoff_item_count) next_owner_flow_item_ids=$($result.summary.next_owner_flow_item_ids -join ',')"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
