param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-evolution-loop-guard.ps1"),
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
        throw "evolution loop guard exited with $LASTEXITCODE, expected $ExpectedExitCode"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$guard = Invoke-Guard -InputRepoRoot $root
$blockedExit = Invoke-Guard -InputRepoRoot $root -FailOnBlocked -ExpectedExitCode 2

Assert-True ($guard.contract_version -eq "smartsteam.remote-gemma-unattended.evolution-loop-guard.v1") "guard contract version mismatch"
Assert-True ($guard.read_only -eq $true) "guard must be read-only"
Assert-True ($guard.starts_process -eq $false) "guard must not start processes"
Assert-True ($guard.sends_prompt -eq $false) "guard must not send prompts"
Assert-True ($guard.touches_remote -eq $false) "guard must not touch remote"
Assert-True ($guard.writes_files -eq $false) "guard must not write files"
Assert-True ($guard.writes_model_weights -eq $false) "guard must not write model weights"
Assert-True ($guard.authorization.can_authorize_daemon -eq $false) "guard must not authorize daemon"
Assert-True ($guard.authorization.can_authorize_launch -eq $false) "guard must not authorize launch"
Assert-True ($guard.authorization.can_authorize_prompt -eq $false) "guard must not authorize prompt"
Assert-True ($guard.authorization.can_authorize_ssh -eq $false) "guard must not authorize ssh"
Assert-True ($guard.summary.may_send_prompt_round -eq $false) "guard must block prompt rounds"
Assert-True ($guard.summary.may_start_or_resume_daemon -eq $false) "guard must block daemon start/resume"
Assert-True ($guard.summary.may_enter_resident_loop -eq $false) "guard must block resident loop"
Assert-True ($guard.summary.evolution_loop_prompt_round_allowed -eq $false) "prompt round action must be blocked"
Assert-True ($guard.summary.forge_daemon_residency_allowed -eq $false) "daemon residency action must be blocked"
Assert-True ($guard.guard_exit_code -eq 2) "blocked guard exit code must be 2"
Assert-True ($blockedExit.fail_on_blocked -eq $true -and $blockedExit.guard_exit_code -eq 2) "FailOnBlocked metadata must stay blocked"
Assert-True (@($guard.guarded_actions).Count -eq 2) "guard must expose two guarded actions"
Assert-True (@($guard.guarded_actions | Where-Object { $_.current_allowed -ne $false -or $_.may_execute -ne $false }).Count -eq 0) "guarded actions must not execute"
Assert-True (@($guard.guarded_actions | Where-Object { $_.verifier_read_only -ne $true -or $_.verifier_command -match '\bssh\b|ssh\.exe|plink|Start-Process|prompt_round|model_pool_launch' }).Count -eq 0) "guard verifiers must stay read-only hints"
Assert-True ($guard.owner_flow_handoff.operator_boundary.this_script_authorizes_actions -eq $false) "guard owner-flow boundary must not authorize actions"

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.evolution-loop-guard-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        guard_contract = $guard.contract_version
        prompt_round_allowed = $guard.summary.evolution_loop_prompt_round_allowed
        daemon_residency_allowed = $guard.summary.forge_daemon_residency_allowed
        may_enter_resident_loop = $guard.summary.may_enter_resident_loop
        guard_exit_code = $guard.guard_exit_code
        ready_item_count = $guard.summary.ready_item_count
        total_item_count = $guard.summary.total_item_count
        missing_evidence = @($guard.summary.missing_evidence)
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "evolution_loop_guard_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-evolution-loop-guard selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "prompt_round_allowed=$($result.summary.prompt_round_allowed) daemon_residency_allowed=$($result.summary.daemon_residency_allowed) may_enter_resident_loop=$($result.summary.may_enter_resident_loop)"
Write-Host "guard_exit_code=$($result.summary.guard_exit_code) ready_items=$($result.summary.ready_item_count)/$($result.summary.total_item_count)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
