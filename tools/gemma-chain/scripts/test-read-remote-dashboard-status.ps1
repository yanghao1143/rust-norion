param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-dashboard-status.ps1"),
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

function Invoke-Dashboard {
    param([string]$InputRepoRoot)

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $InputRepoRoot -Json
    if ($LASTEXITCODE -ne 0) {
        throw "dashboard status exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$dashboard = Invoke-Dashboard -InputRepoRoot $root

Assert-True ($dashboard.contract_version -eq "smartsteam.remote-gemma-unattended.dashboard-status.v1") "dashboard contract version mismatch"
Assert-True ($dashboard.read_only -eq $true) "dashboard must be read-only"
Assert-True ($dashboard.starts_process -eq $false) "dashboard must not start processes"
Assert-True ($dashboard.sends_prompt -eq $false) "dashboard must not send prompts"
Assert-True ($dashboard.touches_remote -eq $false) "dashboard must not touch remote"
Assert-True ($dashboard.writes_files -eq $false) "dashboard must not write files"
Assert-True ($dashboard.writes_model_weights -eq $false) "dashboard must not write model weights"
Assert-True ($dashboard.authorization.can_authorize_daemon -eq $false) "dashboard must not authorize daemon"
Assert-True ($dashboard.authorization.can_authorize_launch -eq $false) "dashboard must not authorize launch"
Assert-True ($dashboard.authorization.can_authorize_prompt -eq $false) "dashboard must not authorize prompt"
Assert-True ($dashboard.authorization.can_authorize_ssh -eq $false) "dashboard must not authorize ssh"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$dashboard.display.severity)) "dashboard must expose display severity"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$dashboard.headline)) "dashboard must expose headline"
Assert-True (@($dashboard.dashboard_cards).Count -ge 6) "dashboard must expose status cards"
Assert-True (@($dashboard.dashboard_cards | Where-Object { [string]::IsNullOrWhiteSpace([string]$_.id) -or [string]::IsNullOrWhiteSpace([string]$_.title) -or [string]::IsNullOrWhiteSpace([string]$_.severity) }).Count -eq 0) "dashboard cards must have id, title, and severity"
Assert-True (@($dashboard.dashboard_cards | Where-Object { $_.id -eq "action_lock" -and $_.value -eq "blocked" }).Count -eq 1) "dashboard must expose blocked action lock"
Assert-True (@($dashboard.dashboard_cards | Where-Object { $_.id -eq "worker_pool_snapshot" -and $_.detail -match "snapshot-only" }).Count -eq 1) "dashboard worker card must be snapshot-only"
Assert-True (@($dashboard.package_items).Count -ge 4) "dashboard must expose evidence package items"
Assert-True ($dashboard.status.consumer_allowed_count -eq 0) "dashboard must keep consumers blocked"
Assert-True ($dashboard.status.unsafe_safe_command_count -eq 0) "dashboard must keep safe commands valid"
Assert-True (@($dashboard.consumers | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "dashboard consumers must fail closed"
Assert-True (@($dashboard.topology.worker_ports_snapshot).Count -ge 6) "dashboard must expose worker topology"
Assert-True (@($dashboard.topology.worker_ports_snapshot | Where-Object { $_.realtime_verified_by_this_script -ne $false }).Count -eq 0) "dashboard worker topology must stay snapshot-only"
Assert-True ($dashboard.topology.backend.port -eq 7979) "dashboard backend port mismatch"
Assert-True ($dashboard.topology.web_lab.port -eq 8789) "dashboard Web Lab port mismatch"
Assert-True ($dashboard.topology.external_remote_ports.observed_by_this_script -eq $false) "dashboard must not claim remote port observation"
Assert-True ($dashboard.topology.external_remote_ports.freshness -eq "not_asserted_by_this_script") "dashboard must not assert remote sync freshness"
Assert-True (@($dashboard.recommended_read_only_entrypoints | Where-Object { $_.read_only -ne $true -or $_.command -match '\bssh\b|ssh\.exe|plink|\bStart\b|Start-Process|prompt_round|model_pool_launch' }).Count -eq 0) "dashboard entrypoints must stay read-only"

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.dashboard-status-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        dashboard_contract = $dashboard.contract_version
        display_severity = $dashboard.display.severity
        package_ready_for_external_gate = $dashboard.status.package_ready_for_external_gate
        ready_item_count = $dashboard.status.ready_item_count
        total_item_count = $dashboard.status.total_item_count
        consumer_allowed_count = $dashboard.status.consumer_allowed_count
        dashboard_card_count = @($dashboard.dashboard_cards).Count
        package_item_count = @($dashboard.package_items).Count
        remote_ports_observed_by_this_script = $dashboard.topology.external_remote_ports.observed_by_this_script
        missing_evidence = @($dashboard.status.missing_evidence)
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "dashboard_status_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-dashboard-status selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "display=$($result.summary.display_severity) package_ready_for_external_gate=$($result.summary.package_ready_for_external_gate) ready_items=$($result.summary.ready_item_count)/$($result.summary.total_item_count)"
Write-Host "cards=$($result.summary.dashboard_card_count) package_items=$($result.summary.package_item_count) consumer_allowed_count=$($result.summary.consumer_allowed_count)"
Write-Host "remote_ports_observed_by_this_script=$($result.summary.remote_ports_observed_by_this_script) missing_evidence=$($result.summary.missing_evidence -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
