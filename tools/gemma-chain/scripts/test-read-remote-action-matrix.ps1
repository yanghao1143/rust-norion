param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-action-matrix.ps1"),
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

function Invoke-ActionMatrix {
    param([string]$InputRepoRoot)

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $InputRepoRoot -Json
    if ($LASTEXITCODE -ne 0) {
        throw "action matrix exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$matrix = Invoke-ActionMatrix -InputRepoRoot $root

Assert-True ($matrix.contract_version -eq "smartsteam.remote-gemma-unattended.action-matrix.v1") "action matrix contract version mismatch"
Assert-True ($matrix.read_only -eq $true) "action matrix must be read-only"
Assert-True ($matrix.starts_process -eq $false) "action matrix must not start processes"
Assert-True ($matrix.sends_prompt -eq $false) "action matrix must not send prompts"
Assert-True ($matrix.touches_remote -eq $false) "action matrix must not touch remote"
Assert-True ($matrix.writes_files -eq $false) "action matrix must not write files"
Assert-True ($matrix.writes_model_weights -eq $false) "action matrix must not write model weights"
Assert-True ($matrix.authorization.can_authorize_daemon -eq $false) "action matrix must not authorize daemon"
Assert-True ($matrix.authorization.can_authorize_launch -eq $false) "action matrix must not authorize launch"
Assert-True ($matrix.authorization.can_authorize_prompt -eq $false) "action matrix must not authorize prompt"
Assert-True ($matrix.authorization.can_authorize_ssh -eq $false) "action matrix must not authorize ssh"
Assert-True ($matrix.summary.action_count -eq 7) "action matrix must expose 7 known actions"
Assert-True ($matrix.summary.allowed_count -eq 0) "action matrix must keep all actions blocked"
Assert-True ($matrix.summary.ui_enabled_count -eq 0) "action matrix UI actions must be disabled"
Assert-True ($matrix.summary.cli_executable_count -eq 0) "action matrix CLI actions must not execute"
Assert-True ($matrix.action_policy.fail_closed_default -eq $true) "action matrix must be fail-closed"
Assert-True ($matrix.action_policy.blocked_actions_must_not_execute -eq $true) "blocked actions must not execute"
Assert-True ($matrix.action_policy.verifier_commands_are_hints_not_auto_run -eq $true) "verifiers must be hints only"
Assert-True ($matrix.action_policy.blocked_exit_code -eq 2) "blocked exit code mismatch"
Assert-True ($matrix.action_policy.unknown_consumer_exit_code -eq 3) "unknown consumer exit code mismatch"
Assert-True (@($matrix.actions | Where-Object { $_.current_allowed -ne $false -or $_.ui_enabled -ne $false -or $_.cli_may_execute -ne $false }).Count -eq 0) "all actions must remain disabled"
Assert-True (@($matrix.actions | Where-Object { [string]::IsNullOrWhiteSpace([string]$_.id) -or [string]::IsNullOrWhiteSpace([string]$_.label) -or [string]::IsNullOrWhiteSpace([string]$_.surface) -or [string]::IsNullOrWhiteSpace([string]$_.entrypoint_kind) }).Count -eq 0) "actions must include display fields"
Assert-True (@($matrix.actions | Where-Object { @($_.blocked_by).Count -eq 0 -or [string]::IsNullOrWhiteSpace([string]$_.reason) -or [string]::IsNullOrWhiteSpace([string]$_.tooltip) }).Count -eq 0) "blocked actions must include reasons"
Assert-True (@($matrix.actions | Where-Object { $_.verifier_read_only -ne $true -or $_.verifier_starts_process -ne $false -or $_.verifier_sends_prompt -ne $false -or $_.verifier_touches_remote -ne $false -or $_.verifier_writes_files -ne $false }).Count -eq 0) "action verifiers must stay read-only"
Assert-True (@($matrix.actions | Where-Object { $_.verifier_command -match '\bssh\b|ssh\.exe|plink|\bStart\b|Start-Process|prompt_round|model_pool_launch' }).Count -eq 0) "action verifiers must not be direct prompt, launch, or SSH commands"
Assert-True (@($matrix.actions | Where-Object { $_.id -eq "web_lab_prompt" -and $_.downstream_sends_prompt -eq $true }).Count -eq 1) "web lab action must mark prompt risk"
Assert-True (@($matrix.actions | Where-Object { $_.id -eq "model_pool_launch" -and $_.downstream_launches_process -eq $true }).Count -eq 1) "model pool action must mark launch risk"
Assert-True (@($matrix.actions | Where-Object { $_.id -eq "ssh_remote_probe" -and $_.downstream_touches_remote -eq $true }).Count -eq 1) "ssh action must mark remote risk"

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.action-matrix-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        action_matrix_contract = $matrix.contract_version
        action_count = $matrix.summary.action_count
        allowed_count = $matrix.summary.allowed_count
        ui_enabled_count = $matrix.summary.ui_enabled_count
        cli_executable_count = $matrix.summary.cli_executable_count
        blocked_exit_code = $matrix.action_policy.blocked_exit_code
        missing_evidence = @($matrix.summary.missing_evidence)
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "action_matrix_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-action-matrix selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "actions=$($result.summary.action_count) allowed=$($result.summary.allowed_count) ui_enabled=$($result.summary.ui_enabled_count) cli_executable=$($result.summary.cli_executable_count)"
Write-Host "blocked_exit_code=$($result.summary.blocked_exit_code) missing_evidence=$($result.summary.missing_evidence -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
