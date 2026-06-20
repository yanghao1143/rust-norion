param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
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

function Select-Action {
    param(
        [object[]]$Actions,
        [string]$Id
    )

    return @($Actions | Where-Object { $_.id -eq $Id } | Select-Object -First 1)[0]
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$generatedAt = Get-Date
$generatedAtUtc = $generatedAt.ToUniversalTime()
$actionMatrixScript = Join-Path $PSScriptRoot "read-remote-action-matrix.ps1"
$ownerFlowScript = Join-Path $PSScriptRoot "read-remote-owner-flow-handoff.ps1"
$packageStatusScript = Join-Path $PSScriptRoot "read-remote-evidence-package-status.ps1"

$actionMatrix = Invoke-JsonScript -ScriptPath $actionMatrixScript -ScriptArgs @("-RepoRoot", $root)
$ownerFlow = Invoke-JsonScript -ScriptPath $ownerFlowScript -ScriptArgs @("-RepoRoot", $root)
$packageStatus = Invoke-JsonScript -ScriptPath $packageStatusScript -ScriptArgs @("-RepoRoot", $root)

$promptRound = Select-Action -Actions @($actionMatrix.actions) -Id "evolution_loop_prompt_round"
$daemonResidency = Select-Action -Actions @($actionMatrix.actions) -Id "forge_daemon_residency"
$guardBlocked = (
    $promptRound.current_allowed -ne $true -or
    $daemonResidency.current_allowed -ne $true -or
    $packageStatus.summary.package_ready_for_external_gate -ne $true -or
    $packageStatus.summary.consumer_allowed_count -ne 0
)
$guardExitCode = if ($guardBlocked) { 2 } else { 0 }

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.evolution-loop-guard.v1"
    generated_at = $generatedAt.ToString("yyyy-MM-dd HH:mm:ss zzz")
    generated_at_utc = $generatedAtUtc.ToString("o")
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    fail_on_blocked = [bool]$FailOnBlocked
    guard_exit_code = $guardExitCode
    summary = [pscustomobject]@{
        evolution_loop_prompt_round_allowed = [bool]$promptRound.current_allowed
        forge_daemon_residency_allowed = [bool]$daemonResidency.current_allowed
        may_send_prompt_round = $false
        may_start_or_resume_daemon = $false
        may_enter_resident_loop = $false
        package_ready_for_external_gate = $packageStatus.summary.package_ready_for_external_gate
        ready_item_count = $packageStatus.summary.ready_item_count
        total_item_count = $packageStatus.summary.total_item_count
        missing_evidence = @($packageStatus.summary.missing_evidence)
        pending_external_gates = @($packageStatus.summary.pending_external_gates)
        next_owner_flow_item_ids = @($ownerFlow.summary.next_owner_flow_item_ids)
        blocked = $guardBlocked
        blocked_reason = if ($guardBlocked) { "evolution_loop_and_daemon_actions_are_blocked_until_fresh_evidence_windows_and_external_gate_pass" } else { "external_gate_still_required_by_policy" }
    }
    guarded_actions = @(
        [pscustomobject]@{
            id = $promptRound.id
            label = $promptRound.label
            current_allowed = $false
            may_execute = $false
            downstream_sends_prompt = $promptRound.downstream_sends_prompt
            downstream_launches_process = $promptRound.downstream_launches_process
            blocked_by = @($promptRound.blocked_by)
            reason = $promptRound.reason
            safe_command_id = $promptRound.safe_command_id
            verifier_command = $promptRound.verifier_command
            verifier_read_only = $promptRound.verifier_read_only
        },
        [pscustomobject]@{
            id = $daemonResidency.id
            label = $daemonResidency.label
            current_allowed = $false
            may_execute = $false
            downstream_sends_prompt = $daemonResidency.downstream_sends_prompt
            downstream_launches_process = $daemonResidency.downstream_launches_process
            blocked_by = @($daemonResidency.blocked_by)
            reason = $daemonResidency.reason
            safe_command_id = $daemonResidency.safe_command_id
            verifier_command = $daemonResidency.verifier_command
            verifier_read_only = $daemonResidency.verifier_read_only
        }
    )
    owner_flow_handoff = [pscustomobject]@{
        operator_boundary = $ownerFlow.operator_boundary
        next_owner_flow_item_ids = @($ownerFlow.summary.next_owner_flow_item_ids)
        blocked_handoff_item_count = $ownerFlow.summary.blocked_handoff_item_count
    }
    source_contracts = [pscustomobject]@{
        action_matrix = $actionMatrix.contract_version
        owner_flow_handoff = $ownerFlow.contract_version
        evidence_package_status = $packageStatus.contract_version
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "evolution_loop_guard_is_read_only_and_cannot_authorize_prompt_launch_daemon_or_ssh"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 10
    if ($FailOnBlocked -and $guardExitCode -ne 0) {
        exit $guardExitCode
    }
    exit 0
}

Write-Host "Gemma remote evolution-loop guard"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "prompt_round_allowed=$($result.summary.evolution_loop_prompt_round_allowed) daemon_residency_allowed=$($result.summary.forge_daemon_residency_allowed)"
Write-Host "may_send_prompt_round=False may_start_or_resume_daemon=False may_enter_resident_loop=False"
Write-Host "package_ready_for_external_gate=$($result.summary.package_ready_for_external_gate) ready_items=$($result.summary.ready_item_count)/$($result.summary.total_item_count)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "guard_exit_code=$($result.guard_exit_code)"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
if ($FailOnBlocked -and $guardExitCode -ne 0) {
    exit $guardExitCode
}
