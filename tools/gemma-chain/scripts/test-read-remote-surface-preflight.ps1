param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-surface-preflight.ps1"),
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

function Invoke-SurfacePreflight {
    param(
        [string]$InputRepoRoot,
        [string]$ConsumerId = "",
        [switch]$FailOnBlocked,
        [int]$ExpectedExitCode = 0
    )

    $scriptArgs = @("-RepoRoot", $InputRepoRoot)
    if (-not [string]::IsNullOrWhiteSpace($ConsumerId)) {
        $scriptArgs += @("-ConsumerId", $ConsumerId)
    }
    if ($FailOnBlocked) {
        $scriptArgs += "-FailOnBlocked"
    }

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath @scriptArgs -Json
    if ($LASTEXITCODE -ne $ExpectedExitCode) {
        throw "surface preflight exited with $LASTEXITCODE, expected $ExpectedExitCode"
    }
    return ($jsonText | ConvertFrom-Json)
}

function Assert-SurfaceContract {
    param([object]$Value)

    Assert-True ($Value.read_only -eq $true) "surface preflight must be read-only"
    Assert-True ($Value.starts_process -eq $false) "surface preflight must not start processes"
    Assert-True ($Value.sends_prompt -eq $false) "surface preflight must not send prompts"
    Assert-True ($Value.touches_remote -eq $false) "surface preflight must not touch remote"
    Assert-True ($Value.writes_files -eq $false) "surface preflight must not write files"
    Assert-True ($Value.writes_model_weights -eq $false) "surface preflight must not write model weights"
    Assert-True ($Value.authorization.can_authorize_daemon -eq $false) "surface preflight must not authorize daemon"
    Assert-True ($Value.authorization.can_authorize_launch -eq $false) "surface preflight must not authorize launch"
    Assert-True ($Value.authorization.can_authorize_prompt -eq $false) "surface preflight must not authorize prompt"
    Assert-True ($Value.authorization.can_authorize_ssh -eq $false) "surface preflight must not authorize ssh"
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$all = Invoke-SurfacePreflight -InputRepoRoot $root
$webLab = Invoke-SurfacePreflight -InputRepoRoot $root -ConsumerId "web_lab_prompt"
$unknown = Invoke-SurfacePreflight -InputRepoRoot $root -ConsumerId "unknown_consumer_for_selftest"
$blockedExit = Invoke-SurfacePreflight -InputRepoRoot $root -ConsumerId "web_lab_prompt" -FailOnBlocked -ExpectedExitCode 2
$unknownExit = Invoke-SurfacePreflight -InputRepoRoot $root -ConsumerId "unknown_consumer_for_selftest" -FailOnBlocked -ExpectedExitCode 3

Assert-SurfaceContract -Value $all
Assert-SurfaceContract -Value $webLab
Assert-SurfaceContract -Value $unknown
Assert-SurfaceContract -Value $blockedExit
Assert-SurfaceContract -Value $unknownExit
Assert-True ($all.contract_version -eq "smartsteam.remote-gemma-unattended.surface-preflight.v1") "surface preflight contract version mismatch"
Assert-True (@($all.known_consumer_ids).Count -ge 7) "surface preflight must expose known consumer ids"
Assert-True (@($all.consumers).Count -ge 7) "surface preflight should return all consumers by default"
Assert-True (@($all.consumers | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "surface preflight consumers must fail closed"
Assert-True (@($all.consumers | Where-Object { $null -eq $_.safe_command -or $_.safe_command.read_only -ne $true -or $_.safe_command.starts_process -ne $false -or $_.safe_command.sends_prompt -ne $false -or $_.safe_command.touches_remote -ne $false -or $_.safe_command.writes_files -ne $false }).Count -eq 0) "surface preflight safe commands must resolve and stay read-only"
Assert-True ($all.status.consumer_contract_validated -eq $true) "surface preflight must expose validated consumer contract"
Assert-True ($all.status.consumer_allowed_count -eq 0) "surface preflight must keep consumers blocked"
Assert-True ($all.status.unsafe_safe_command_count -eq 0) "surface preflight must keep safe commands valid"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$all.display.severity)) "surface preflight must expose display severity"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$all.display.headline)) "surface preflight must expose display headline"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$all.display.detail)) "surface preflight must expose display detail"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$all.display.next_action_label)) "surface preflight must expose display next action"
Assert-True ($all.display.badge -in @("blocked", "external_gate_required")) "surface preflight display badge must be known"
Assert-True (@($all.missing_evidence_actions).Count -eq @($all.status.missing_evidence).Count) "surface preflight missing evidence actions must match missing evidence"
Assert-True (@($all.pending_external_gate_actions).Count -eq @($all.status.pending_external_gates).Count) "surface preflight pending gate actions must match pending gates"
Assert-True (@($all.quick_commands | Where-Object { $_.read_only -ne $true -or $_.command -match '\bssh\b|ssh\.exe|plink|\bStart\b|Start-Process|prompt_round|model_pool_launch' }).Count -eq 0) "surface preflight quick commands must stay read-only"
Assert-True ($webLab.consumer_found -eq $true) "web_lab_prompt should be found"
Assert-True (@($webLab.consumers).Count -eq 1) "single surface preflight should return one consumer"
Assert-True ($webLab.consumers[0].id -eq "web_lab_prompt") "single surface preflight returned wrong consumer"
Assert-True ($webLab.consumers[0].current_allowed -eq $false) "web_lab_prompt must stay blocked"
Assert-True ($unknown.consumer_found -eq $false) "unknown consumer should not be found"
Assert-True (@($unknown.consumers).Count -eq 0) "unknown consumer should return no consumers"
Assert-True ($unknown.status.error -eq "unknown_consumer_id") "unknown consumer should expose error"
Assert-True ($all.fail_on_blocked_exit_code -eq 2) "blocked package should advertise exit code 2"
Assert-True ($webLab.fail_on_blocked_exit_code -eq 2) "blocked single consumer should advertise exit code 2"
Assert-True ($blockedExit.fail_on_blocked -eq $true -and $blockedExit.fail_on_blocked_exit_code -eq 2) "FailOnBlocked should preserve blocked exit metadata"
Assert-True ($unknownExit.fail_on_blocked -eq $true -and $unknownExit.fail_on_blocked_exit_code -eq 3) "FailOnBlocked should preserve unknown exit metadata"

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.surface-preflight-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        surface_preflight_contract = $all.contract_version
        consumer_count = @($all.consumers).Count
        consumer_allowed_count = $all.status.consumer_allowed_count
        unsafe_safe_command_count = $all.status.unsafe_safe_command_count
        package_ready_for_external_gate = $all.status.package_ready_for_external_gate
        snapshot_classification = $all.status.snapshot_classification
        evidence_fresh_all = $all.status.evidence_fresh_all
        observation_window_status = $all.status.observation_window_status
        resource_window_status = $all.status.resource_window_status
        display_severity = $all.display.severity
        display_headline = $all.display.headline
        missing_evidence = @($all.status.missing_evidence)
        pending_external_gates = @($all.status.pending_external_gates)
        fail_on_blocked_exit_code = $all.fail_on_blocked_exit_code
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "surface_preflight_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-surface-preflight selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "consumers=$($result.summary.consumer_count) allowed=$($result.summary.consumer_allowed_count) unsafe_safe_command_count=$($result.summary.unsafe_safe_command_count)"
Write-Host "package_ready_for_external_gate=$($result.summary.package_ready_for_external_gate) snapshot=$($result.summary.snapshot_classification) evidence_fresh_all=$($result.summary.evidence_fresh_all)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
