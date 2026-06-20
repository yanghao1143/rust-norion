param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-residency-gap-report.ps1"),
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

function Invoke-GapReport {
    param([string]$InputRepoRoot)

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $InputRepoRoot -Json
    if ($LASTEXITCODE -ne 0) {
        throw "gap report exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$report = Invoke-GapReport -InputRepoRoot $root

Assert-True ($report.contract_version -eq "smartsteam.remote-gemma-unattended.residency-gap-report.v1") "gap report contract version mismatch"
Assert-True ($report.read_only -eq $true) "gap report must be read-only"
Assert-True ($report.starts_process -eq $false) "gap report must not start processes"
Assert-True ($report.sends_prompt -eq $false) "gap report must not send prompts"
Assert-True ($report.touches_remote -eq $false) "gap report must not touch remote"
Assert-True ($report.writes_files -eq $false) "gap report must not write files"
Assert-True ($report.writes_model_weights -eq $false) "gap report must not write model weights"
Assert-True ($report.decision.authorized -eq $false) "gap report must not authorize actions"
Assert-True ($report.authorization.can_authorize_daemon -eq $false) "gap report must not authorize daemon"
Assert-True ($report.authorization.can_authorize_launch -eq $false) "gap report must not authorize launch"
Assert-True ($report.authorization.can_authorize_prompt -eq $false) "gap report must not authorize prompt"
Assert-True ($report.authorization.can_authorize_ssh -eq $false) "gap report must not authorize ssh"
Assert-True ($report.snapshot_claims.historical_only -eq $true) "snapshot claims must be marked historical-only"
Assert-True ($null -ne $report.snapshot_claims.model_cache) "gap report must expose model cache claims"
Assert-True ($null -ne $report.snapshot_claims.model_pool) "gap report must expose model pool claims"
Assert-True ($null -ne $report.snapshot_claims.unattended) "gap report must expose unattended claims"
Assert-True (@($report.freshness.evidence_files).Count -ge 4) "gap report must expose evidence file freshness"
Assert-True (@($report.checklist).Count -ge 6) "gap report must expose checklist"
Assert-True (@($report.checklist | Where-Object {
    [string]::IsNullOrWhiteSpace([string]$_.id) -or
    [string]::IsNullOrWhiteSpace([string]$_.gap_id) -or
    [string]::IsNullOrWhiteSpace([string]$_.status) -or
    [string]::IsNullOrWhiteSpace([string]$_.required_evidence) -or
    [string]::IsNullOrWhiteSpace([string]$_.proof_source) -or
    [string]::IsNullOrWhiteSpace([string]$_.safe_command_id)
}).Count -eq 0) "checklist items must include id, gap_id, status, required_evidence, proof_source, and safe_command_id"
Assert-True (@($report.checklist | Where-Object { $_.blocks_authorization -ne $true }).Count -eq 0) "checklist items must block authorization"
Assert-True ($report.safety.unresolved_checklist_safe_command_count -eq 0) "all checklist safe_command_id values must resolve"
Assert-True ($report.safety.unsafe_safe_command_count -eq 0) "safe commands must stay read-only and avoid prompt/launch/SSH text"
Assert-True (@($report.safety.safe_commands | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false -or $_.unsafe_text_match -ne $false }).Count -eq 0) "safe command catalog must remain safe"
Assert-True ($report.safety.consumer_allowed_count -eq 0) "consumer surfaces must remain blocked"
Assert-True (@($report.consumers | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "consumer projection in gap report must fail closed"
Assert-True (@($report.decision.missing_evidence).Count -eq @($report.missing_evidence_actions).Count) "missing evidence should map to actions"
Assert-True (@($report.decision.pending_external_gates).Count -eq @($report.pending_external_gate_actions).Count) "pending external gates should map to actions"

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.residency-gap-report-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        gap_report_contract = $report.contract_version
        snapshot_classification = $report.decision.snapshot_classification
        evidence_fresh_all = $report.decision.evidence_fresh_all
        can_support_external_residency_review = $report.decision.can_support_external_residency_review
        missing_evidence = @($report.decision.missing_evidence)
        pending_external_gates = @($report.decision.pending_external_gates)
        checklist_count = @($report.checklist).Count
        safe_command_count = $report.safety.safe_command_count
        unsafe_safe_command_count = $report.safety.unsafe_safe_command_count
        unresolved_checklist_safe_command_count = $report.safety.unresolved_checklist_safe_command_count
        consumer_allowed_count = $report.safety.consumer_allowed_count
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "gap_report_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-residency-gap-report selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "snapshot=$($result.summary.snapshot_classification) evidence_fresh_all=$($result.summary.evidence_fresh_all)"
Write-Host "checklist_count=$($result.summary.checklist_count) safe_command_count=$($result.summary.safe_command_count) unsafe_safe_command_count=$($result.summary.unsafe_safe_command_count) unresolved_checklist_safe_command_count=$($result.summary.unresolved_checklist_safe_command_count)"
Write-Host "can_support_external_residency_review=$($result.summary.can_support_external_residency_review) missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
