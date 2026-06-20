param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-consumer-preflight.ps1"),
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

function Invoke-Preflight {
    param(
        [string]$InputRepoRoot,
        [string]$ConsumerId = "",
        [switch]$FailOnBlocked,
        [int]$ExpectedExitCode = 0
    )

    $args = @("-RepoRoot", $InputRepoRoot)
    if (-not [string]::IsNullOrWhiteSpace($ConsumerId)) {
        $args += @("-ConsumerId", $ConsumerId)
    }
    if ($FailOnBlocked) {
        $args += "-FailOnBlocked"
    }

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath @args -Json
    if ($LASTEXITCODE -ne $ExpectedExitCode) {
        throw "consumer preflight exited with $LASTEXITCODE, expected $ExpectedExitCode"
    }
    return ($jsonText | ConvertFrom-Json)
}

function Assert-PreflightContract {
    param([object]$Preflight)

    Assert-True ($Preflight.read_only -eq $true) "consumer preflight must be read-only"
    Assert-True ($Preflight.starts_process -eq $false) "consumer preflight must not start processes"
    Assert-True ($Preflight.sends_prompt -eq $false) "consumer preflight must not send prompts"
    Assert-True ($Preflight.touches_remote -eq $false) "consumer preflight must not touch remote"
    Assert-True ($Preflight.writes_files -eq $false) "consumer preflight must not write files"
    Assert-True ($Preflight.writes_model_weights -eq $false) "consumer preflight must not write model weights"
    Assert-True ($Preflight.authorization.can_authorize_daemon -eq $false) "consumer preflight must not authorize daemon"
    Assert-True ($Preflight.authorization.can_authorize_launch -eq $false) "consumer preflight must not authorize launch"
    Assert-True ($Preflight.authorization.can_authorize_prompt -eq $false) "consumer preflight must not authorize prompt"
    Assert-True ($Preflight.authorization.can_authorize_ssh -eq $false) "consumer preflight must not authorize ssh"
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$all = Invoke-Preflight -InputRepoRoot $root
$webLab = Invoke-Preflight -InputRepoRoot $root -ConsumerId "web_lab_prompt"
$unknown = Invoke-Preflight -InputRepoRoot $root -ConsumerId "unknown_consumer_for_selftest"
$blockedExit = Invoke-Preflight -InputRepoRoot $root -ConsumerId "web_lab_prompt" -FailOnBlocked -ExpectedExitCode 2
$unknownExit = Invoke-Preflight -InputRepoRoot $root -ConsumerId "unknown_consumer_for_selftest" -FailOnBlocked -ExpectedExitCode 3

Assert-PreflightContract -Preflight $all
Assert-PreflightContract -Preflight $webLab
Assert-PreflightContract -Preflight $unknown
Assert-PreflightContract -Preflight $blockedExit
Assert-PreflightContract -Preflight $unknownExit
Assert-True ($all.contract_version -eq "smartsteam.remote-gemma-unattended.consumer-preflight.v1") "consumer preflight contract version mismatch"
Assert-True (@($all.known_consumer_ids).Count -ge 7) "consumer preflight must expose known consumer ids"
Assert-True ($all.summary.consumer_count -eq @($all.consumers).Count) "consumer count mismatch"
Assert-True ($all.summary.allowed_count -eq 0) "consumer preflight must keep consumers blocked"
Assert-True ($all.summary.unsafe_preflight_count -eq 0) "consumer preflight must resolve safe commands"
Assert-True ($all.summary.consumer_contract_validated -eq $true) "consumer contract must validate"
Assert-True (@($all.consumers | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "all consumers must fail closed"
Assert-True (@($all.consumers | Where-Object { $_.safe_command_resolved -ne $true -or $_.safe_command_safe -ne $true }).Count -eq 0) "all consumer safe commands must resolve and stay safe"
Assert-True (@($all.consumers | Where-Object { $_.entrypoint_kind -eq "prompt" -and $_.downstream_sends_prompt -ne $true }).Count -eq 0) "prompt consumers must be marked prompt-producing"
Assert-True (@($all.consumers | Where-Object { $_.entrypoint_kind -eq "launch" -and $_.downstream_launches_process -ne $true }).Count -eq 0) "launch consumers must be marked launch-producing"
Assert-True (@($all.consumers | Where-Object { $_.entrypoint_kind -eq "ssh" -and $_.downstream_touches_remote -ne $true }).Count -eq 0) "ssh consumers must be marked remote-touching"
Assert-True ($webLab.consumer_found -eq $true) "web_lab_prompt should be found"
Assert-True ($webLab.summary.consumer_count -eq 1) "single consumer preflight should return one item"
Assert-True ($webLab.consumers[0].id -eq "web_lab_prompt") "single consumer preflight returned wrong id"
Assert-True ($webLab.consumers[0].current_allowed -eq $false) "web_lab_prompt must stay blocked"
Assert-True ($unknown.consumer_found -eq $false) "unknown consumer should not be found"
Assert-True ($unknown.summary.consumer_count -eq 0) "unknown consumer should return no consumer item"
Assert-True ($unknown.summary.error -eq "unknown_consumer_id") "unknown consumer should expose error"
Assert-True ($all.fail_on_blocked_exit_code -eq 2) "blocked consumer set should advertise exit code 2"
Assert-True ($webLab.fail_on_blocked_exit_code -eq 2) "blocked single consumer should advertise exit code 2"
Assert-True ($blockedExit.fail_on_blocked -eq $true -and $blockedExit.fail_on_blocked_exit_code -eq 2) "FailOnBlocked should preserve blocked exit metadata"
Assert-True ($unknownExit.fail_on_blocked -eq $true -and $unknownExit.fail_on_blocked_exit_code -eq 3) "FailOnBlocked should preserve unknown exit metadata"

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.consumer-preflight-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        consumer_preflight_contract = $all.contract_version
        known_consumer_count = @($all.known_consumer_ids).Count
        consumer_count = $all.summary.consumer_count
        allowed_count = $all.summary.allowed_count
        blocked_count = $all.summary.blocked_count
        unsafe_preflight_count = $all.summary.unsafe_preflight_count
        snapshot_classification = $all.summary.readiness_snapshot_classification
        evidence_fresh_all = $all.summary.evidence_fresh_all
        missing_evidence = @($all.summary.missing_evidence)
        pending_external_gates = @($all.summary.pending_external_gates)
        fail_on_blocked_exit_code = $all.fail_on_blocked_exit_code
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "consumer_preflight_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-consumer-preflight selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "consumers=$($result.summary.consumer_count) allowed=$($result.summary.allowed_count) blocked=$($result.summary.blocked_count) unsafe_preflight_count=$($result.summary.unsafe_preflight_count)"
Write-Host "snapshot=$($result.summary.snapshot_classification) evidence_fresh_all=$($result.summary.evidence_fresh_all)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
