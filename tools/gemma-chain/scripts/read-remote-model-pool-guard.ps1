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
$linkBoundaryScript = Join-Path $PSScriptRoot "read-remote-link-boundary.ps1"
$packageStatusScript = Join-Path $PSScriptRoot "read-remote-evidence-package-status.ps1"

$actionMatrix = Invoke-JsonScript -ScriptPath $actionMatrixScript -ScriptArgs @("-RepoRoot", $root)
$linkBoundary = Invoke-JsonScript -ScriptPath $linkBoundaryScript -ScriptArgs @("-RepoRoot", $root)
$packageStatus = Invoke-JsonScript -ScriptPath $packageStatusScript -ScriptArgs @("-RepoRoot", $root)
$modelPoolAction = Select-Action -Actions @($actionMatrix.actions) -Id "model_pool_launch"

$guardBlocked = (
    $modelPoolAction.current_allowed -ne $true -or
    $packageStatus.summary.package_ready_for_external_gate -ne $true -or
    $linkBoundary.summary.realtime_ports_verified_by_this_script -ne $true
)
$guardExitCode = if ($guardBlocked) { 2 } else { 0 }

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.model-pool-guard.v1"
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
        model_pool_launch_allowed = [bool]$modelPoolAction.current_allowed
        may_launch_worker = $false
        may_expand_pool = $false
        may_reuse_snapshot_as_current_capacity = $false
        worker_count_snapshot = $linkBoundary.summary.worker_count_snapshot
        healthy_worker_count_snapshot = $linkBoundary.summary.healthy_worker_count_snapshot
        model_cache_ok_count_snapshot = $linkBoundary.summary.model_cache_ok_count_snapshot
        model_cache_model_count_snapshot = $linkBoundary.summary.model_cache_model_count_snapshot
        realtime_ports_verified_by_this_script = $linkBoundary.summary.realtime_ports_verified_by_this_script
        package_ready_for_external_gate = $packageStatus.summary.package_ready_for_external_gate
        ready_item_count = $packageStatus.summary.ready_item_count
        total_item_count = $packageStatus.summary.total_item_count
        missing_evidence = @($packageStatus.summary.missing_evidence)
        pending_external_gates = @($packageStatus.summary.pending_external_gates)
        blocked = $guardBlocked
        blocked_reason = if ($guardBlocked) { "model_pool_launch_and_expansion_are_blocked_until_fresh_evidence_windows_resource_headroom_and_external_gate_pass" } else { "external_gate_still_required_by_policy" }
    }
    guarded_action = [pscustomobject]@{
        id = $modelPoolAction.id
        label = $modelPoolAction.label
        current_allowed = $false
        may_execute = $false
        downstream_launches_process = $modelPoolAction.downstream_launches_process
        blocked_by = @($modelPoolAction.blocked_by)
        reason = $modelPoolAction.reason
        safe_command_id = $modelPoolAction.safe_command_id
        verifier_command = $modelPoolAction.verifier_command
        verifier_read_only = $modelPoolAction.verifier_read_only
    }
    worker_snapshot = [pscustomobject]@{
        evidence_kind = "local_snapshot_not_current_authorization"
        worker_ports = $linkBoundary.service_topology.model_workers
        backend = $linkBoundary.service_topology.backend
        web_lab = $linkBoundary.service_topology.web_lab
        external_remote_ports = $linkBoundary.service_topology.external_remote_ports
    }
    source_contracts = [pscustomobject]@{
        action_matrix = $actionMatrix.contract_version
        link_boundary = $linkBoundary.contract_version
        evidence_package_status = $packageStatus.contract_version
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "model_pool_guard_is_read_only_and_cannot_authorize_worker_launch_expansion_prompt_or_ssh"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 10
    if ($FailOnBlocked -and $guardExitCode -ne 0) {
        exit $guardExitCode
    }
    exit 0
}

Write-Host "Gemma remote model-pool guard"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "model_pool_launch_allowed=$($result.summary.model_pool_launch_allowed) may_launch_worker=False may_expand_pool=False"
Write-Host "workers_snapshot=$($result.summary.healthy_worker_count_snapshot)/$($result.summary.worker_count_snapshot) cache_snapshot=$($result.summary.model_cache_ok_count_snapshot)/$($result.summary.model_cache_model_count_snapshot)"
Write-Host "realtime_ports_verified_by_this_script=$($result.summary.realtime_ports_verified_by_this_script) package_ready_for_external_gate=$($result.summary.package_ready_for_external_gate)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "guard_exit_code=$($result.guard_exit_code)"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
if ($FailOnBlocked -and $guardExitCode -ne 0) {
    exit $guardExitCode
}
