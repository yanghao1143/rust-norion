param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-evidence-package-status.ps1"),
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

function Invoke-Status {
    param([string]$InputRepoRoot)

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $InputRepoRoot -Json
    if ($LASTEXITCODE -ne 0) {
        throw "evidence package status exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$status = Invoke-Status -InputRepoRoot $root

Assert-True ($status.contract_version -eq "smartsteam.remote-gemma-unattended.evidence-package-status.v1") "evidence package status contract version mismatch"
Assert-True ($status.read_only -eq $true) "evidence package status must be read-only"
Assert-True ($status.starts_process -eq $false) "evidence package status must not start processes"
Assert-True ($status.sends_prompt -eq $false) "evidence package status must not send prompts"
Assert-True ($status.touches_remote -eq $false) "evidence package status must not touch remote"
Assert-True ($status.writes_files -eq $false) "evidence package status must not write files"
Assert-True ($status.writes_model_weights -eq $false) "evidence package status must not write model weights"
Assert-True ($status.authorization.can_authorize_daemon -eq $false) "evidence package status must not authorize daemon"
Assert-True ($status.authorization.can_authorize_launch -eq $false) "evidence package status must not authorize launch"
Assert-True ($status.authorization.can_authorize_prompt -eq $false) "evidence package status must not authorize prompt"
Assert-True ($status.authorization.can_authorize_ssh -eq $false) "evidence package status must not authorize ssh"
Assert-True ($status.summary.total_item_count -eq @($status.package_items).Count) "item count must match package_items"
Assert-True (@($status.package_items | Where-Object { [string]::IsNullOrWhiteSpace([string]$_.id) -or [string]::IsNullOrWhiteSpace([string]$_.status) -or [string]::IsNullOrWhiteSpace([string]$_.required_evidence) -or [string]::IsNullOrWhiteSpace([string]$_.verifier_command) -or [string]::IsNullOrWhiteSpace([string]$_.proof_source) }).Count -eq 0) "package items must expose id, status, required_evidence, verifier_command, and proof_source"
Assert-True (@($status.package_items | Where-Object { $_.id -eq "fresh_snapshot" }).Count -eq 1) "status must include fresh snapshot item"
Assert-True (@($status.package_items | Where-Object { $_.id -eq "unattended_report_ledger_consistency" }).Count -eq 1) "status must include unattended report/ledger consistency item"
Assert-True (@($status.package_items | Where-Object { $_.id -eq "continuous_port_worker_window" }).Count -eq 1) "status must include observation window item"
Assert-True (@($status.package_items | Where-Object { $_.id -eq "remote_resource_headroom_window" }).Count -eq 1) "status must include resource window item"
Assert-True (@($status.package_items | Where-Object { $_.id -eq "fail_closed_contracts" }).Count -eq 1) "status must include fail-closed contract item"
Assert-True ($null -ne $status.summary.PSObject.Properties["report_ledger_round_mismatch"]) "status must expose report/ledger mismatch flag"
Assert-True ($null -ne $status.summary.PSObject.Properties["latest_ledger_failed"]) "status must expose latest ledger failure flag"
Assert-True ($null -ne $status.summary.PSObject.Properties["requires_unattended_report_refresh"]) "status must expose unattended report refresh flag"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$status.artifact_locations.observation_window_dir)) "status must expose observation window dir"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$status.artifact_locations.resource_window_dir)) "status must expose resource window dir"
Assert-True (@($status.artifact_locations.snapshot_outputs).Count -ge 4) "status must expose snapshot outputs"
Assert-True (@($status.next_read_only_verifiers).Count -ge 5) "status must expose read-only verifiers"
Assert-True (@($status.next_read_only_verifiers | Where-Object { $_.command -match '\bssh\b|ssh\.exe|plink|\bStart\b|Start-Process|prompt_round|model_pool_launch' }).Count -eq 0) "read-only verifiers must not contain SSH, launch, or prompt actions"
Assert-True ($status.summary.consumer_allowed_count -eq 0) "consumers must remain blocked"
Assert-True ($status.summary.unsafe_safe_command_count -eq 0) "safe command catalog must remain safe"
if ($status.summary.package_ready_for_external_gate -eq $true) {
    Assert-True ($status.summary.evidence_fresh_all -eq $true) "ready package requires fresh evidence"
    Assert-True ($status.summary.requires_unattended_report_refresh -eq $false) "ready package requires report/ledger consistency"
    Assert-True ($status.summary.continuous_window_present -eq $true) "ready package requires observation window"
    Assert-True ($status.summary.resource_window_present -eq $true) "ready package requires resource window"
    Assert-True ($status.summary.fail_closed_contracts_ok -eq $true) "ready package requires fail-closed contracts"
}

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.evidence-package-status-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        status_contract = $status.contract_version
        package_ready_for_external_gate = $status.summary.package_ready_for_external_gate
        can_support_external_residency_review = $status.summary.can_support_external_residency_review
        snapshot_classification = $status.summary.snapshot_classification
        evidence_fresh_all = $status.summary.evidence_fresh_all
        report_ledger_round_mismatch = $status.summary.report_ledger_round_mismatch
        latest_ledger_failed = $status.summary.latest_ledger_failed
        requires_unattended_report_refresh = $status.summary.requires_unattended_report_refresh
        observation_window_status = $status.summary.observation_window_status
        continuous_window_present = $status.summary.continuous_window_present
        resource_window_status = $status.summary.resource_window_status
        resource_window_present = $status.summary.resource_window_present
        ready_item_count = $status.summary.ready_item_count
        total_item_count = $status.summary.total_item_count
        missing_evidence = @($status.summary.missing_evidence)
        pending_external_gates = @($status.summary.pending_external_gates)
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "evidence_package_status_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-evidence-package-status selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "package_ready_for_external_gate=$($result.summary.package_ready_for_external_gate) can_support_external_residency_review=$($result.summary.can_support_external_residency_review)"
Write-Host "snapshot=$($result.summary.snapshot_classification) evidence_fresh_all=$($result.summary.evidence_fresh_all)"
Write-Host "report_ledger_round_mismatch=$($result.summary.report_ledger_round_mismatch) latest_ledger_failed=$($result.summary.latest_ledger_failed) requires_report_refresh=$($result.summary.requires_unattended_report_refresh)"
Write-Host "observation_window=$($result.summary.observation_window_status) continuous_window_present=$($result.summary.continuous_window_present)"
Write-Host "resource_window=$($result.summary.resource_window_status) resource_window_present=$($result.summary.resource_window_present)"
Write-Host "ready_items=$($result.summary.ready_item_count)/$($result.summary.total_item_count) missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
