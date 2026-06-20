param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-model-pool-guard.ps1"),
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

function Invoke-Guard {
    param(
        [string]$InputRepoRoot,
        [switch]$FailOnBlocked,
        [int]$ExpectedExitCode = 0
    )

    $scriptArgs = @("-RepoRoot", $InputRepoRoot)
    if ($FailOnBlocked) {
        $scriptArgs += "-FailOnBlocked"
    }

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath @scriptArgs -Json
    if ($LASTEXITCODE -ne $ExpectedExitCode) {
        throw "model-pool guard exited with $LASTEXITCODE, expected $ExpectedExitCode"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$guard = Invoke-Guard -InputRepoRoot $root
$blockedExit = Invoke-Guard -InputRepoRoot $root -FailOnBlocked -ExpectedExitCode 2

Assert-True ($guard.contract_version -eq "smartsteam.remote-gemma-unattended.model-pool-guard.v1") "model-pool guard contract version mismatch"
Assert-True ($guard.read_only -eq $true) "model-pool guard must be read-only"
Assert-True ($guard.starts_process -eq $false) "model-pool guard must not start processes"
Assert-True ($guard.sends_prompt -eq $false) "model-pool guard must not send prompts"
Assert-True ($guard.touches_remote -eq $false) "model-pool guard must not touch remote"
Assert-True ($guard.writes_files -eq $false) "model-pool guard must not write files"
Assert-True ($guard.writes_model_weights -eq $false) "model-pool guard must not write model weights"
Assert-True ($guard.authorization.can_authorize_daemon -eq $false) "model-pool guard must not authorize daemon"
Assert-True ($guard.authorization.can_authorize_launch -eq $false) "model-pool guard must not authorize launch"
Assert-True ($guard.authorization.can_authorize_prompt -eq $false) "model-pool guard must not authorize prompt"
Assert-True ($guard.authorization.can_authorize_ssh -eq $false) "model-pool guard must not authorize ssh"
Assert-True ($guard.summary.model_pool_launch_allowed -eq $false) "model-pool launch action must be blocked"
Assert-True ($guard.summary.may_launch_worker -eq $false) "model-pool guard must block launch"
Assert-True ($guard.summary.may_expand_pool -eq $false) "model-pool guard must block expansion"
Assert-True ($guard.summary.may_reuse_snapshot_as_current_capacity -eq $false) "model-pool guard must not treat snapshots as current capacity"
Assert-True ($guard.summary.worker_count_snapshot -ge 6) "model-pool guard should expose worker snapshot"
Assert-True ($guard.summary.healthy_worker_count_snapshot -eq $guard.summary.worker_count_snapshot) "worker snapshot should show current historical healthy count"
Assert-True ($guard.summary.realtime_ports_verified_by_this_script -eq $false) "model-pool guard must not claim live port verification"
Assert-True ($guard.guard_exit_code -eq 2) "blocked guard exit code must be 2"
Assert-True ($blockedExit.fail_on_blocked -eq $true -and $blockedExit.guard_exit_code -eq 2) "FailOnBlocked metadata must stay blocked"
Assert-True ($guard.guarded_action.current_allowed -eq $false -and $guard.guarded_action.may_execute -eq $false) "guarded model-pool action must not execute"
Assert-True ($guard.guarded_action.verifier_read_only -eq $true) "model-pool verifier must be read-only"
Assert-True ($guard.guarded_action.verifier_command -notmatch '\bssh\b|ssh\.exe|plink|Start-Process|prompt_round|model_pool_launch') "model-pool verifier must not be direct launch, prompt, or SSH"
Assert-True (@($guard.worker_snapshot.worker_ports | Where-Object { $_.realtime_verified_by_this_script -ne $false }).Count -eq 0) "worker snapshot must remain snapshot-only"
Assert-True ($guard.worker_snapshot.external_remote_ports.observed_by_this_script -eq $false) "model-pool guard must not claim remote observation"

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.model-pool-guard-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        guard_contract = $guard.contract_version
        model_pool_launch_allowed = $guard.summary.model_pool_launch_allowed
        may_launch_worker = $guard.summary.may_launch_worker
        may_expand_pool = $guard.summary.may_expand_pool
        worker_count_snapshot = $guard.summary.worker_count_snapshot
        healthy_worker_count_snapshot = $guard.summary.healthy_worker_count_snapshot
        guard_exit_code = $guard.guard_exit_code
        missing_evidence = @($guard.summary.missing_evidence)
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "model_pool_guard_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-model-pool-guard selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "model_pool_launch_allowed=$($result.summary.model_pool_launch_allowed) may_launch_worker=$($result.summary.may_launch_worker) may_expand_pool=$($result.summary.may_expand_pool)"
Write-Host "workers=$($result.summary.healthy_worker_count_snapshot)/$($result.summary.worker_count_snapshot) guard_exit_code=$($result.summary.guard_exit_code)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
