param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [int]$FreshMinutes = 30,
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

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$generatedAt = Get-Date
$generatedAtUtc = $generatedAt.ToUniversalTime()
$snapshotScript = Join-Path $PSScriptRoot "read-remote-unattended-snapshot.ps1"
$packageStatusScript = Join-Path $PSScriptRoot "read-remote-evidence-package-status.ps1"

$snapshot = Invoke-JsonScript -ScriptPath $snapshotScript -ScriptArgs @("-RepoRoot", $root, "-FreshMinutes", "$FreshMinutes")
$packageStatus = Invoke-JsonScript -ScriptPath $packageStatusScript -ScriptArgs @("-RepoRoot", $root)

$evidenceRows = @($snapshot.evidence.PSObject.Properties | ForEach-Object {
    [pscustomobject]@{
        id = $_.Name
        path = $_.Value.path
        exists = $_.Value.exists
        fresh = $_.Value.fresh
        age_seconds = $_.Value.age_seconds
        last_write_time_utc = $_.Value.last_write_time_utc
        parse_error = $_.Value.parse_error
    }
})

$freshRows = @($evidenceRows | Where-Object { $_.fresh -eq $true })
$staleRows = @($evidenceRows | Where-Object { $_.fresh -ne $true })
$parseErrorRows = @($evidenceRows | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.parse_error) })
$missingRows = @($evidenceRows | Where-Object { $_.exists -ne $true })

$latestLedgerRound = $snapshot.latest_ledger.round
$latestLedgerSuccess = $snapshot.latest_ledger.success
$reportRounds = $snapshot.unattended.rounds
$reportSuccess = $snapshot.unattended.success
$reportLedgerRoundMismatch = ($null -ne $latestLedgerRound -and $null -ne $reportRounds -and [int]$latestLedgerRound -gt [int]$reportRounds)
$latestLedgerFailed = ($latestLedgerSuccess -eq $false)

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.evidence-freshness.v1"
    generated_at = $generatedAt.ToString("yyyy-MM-dd HH:mm:ss zzz")
    generated_at_utc = $generatedAtUtc.ToString("o")
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    fresh_minutes = $FreshMinutes
    summary = [pscustomobject]@{
        evidence_fresh_all = $snapshot.summary.evidence_fresh_all
        evidence_file_count = @($evidenceRows).Count
        fresh_file_count = @($freshRows).Count
        stale_file_count = @($staleRows).Count
        missing_file_count = @($missingRows).Count
        parse_error_count = @($parseErrorRows).Count
        snapshot_classification = $snapshot.residency_decision.classification
        package_ready_for_external_gate = $packageStatus.summary.package_ready_for_external_gate
        ready_item_count = $packageStatus.summary.ready_item_count
        total_item_count = $packageStatus.summary.total_item_count
        missing_evidence = @($packageStatus.summary.missing_evidence)
        pending_external_gates = @($packageStatus.summary.pending_external_gates)
        unattended_report_rounds = $reportRounds
        unattended_report_success = $reportSuccess
        latest_ledger_round = $latestLedgerRound
        latest_ledger_success = $latestLedgerSuccess
        report_ledger_round_mismatch = $reportLedgerRoundMismatch
        latest_ledger_failed = $latestLedgerFailed
        requires_unattended_report_refresh = ($reportLedgerRoundMismatch -or $latestLedgerFailed)
    }
    evidence_files = $evidenceRows
    stale_evidence_files = $staleRows
    freshness_interpretation = [pscustomobject]@{
        historical_cache_worker_claims = "cache/model-pool snapshots are display evidence only; they do not authorize current launch, prompt, daemon, or SSH actions"
        stale_report_warning = if ($reportLedgerRoundMismatch) { "latest ledger round is newer than evolution report; refresh or reconcile unattended report before any residency gate" } else { "" }
        failed_ledger_warning = if ($latestLedgerFailed) { "latest ledger entry is not successful; do not treat older report success as current unattended readiness" } else { "" }
        next_safe_reader = ".\tools\gemma-chain\scripts\read-remote-unattended-snapshot.ps1 -Json"
    }
    source_contracts = [pscustomobject]@{
        snapshot = $snapshot.contract_version
        evidence_package_status = $packageStatus.contract_version
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "evidence_freshness_is_read_only_and_cannot_authorize_actions"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 10
    exit 0
}

Write-Host "Gemma remote evidence freshness"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "fresh_files=$($result.summary.fresh_file_count)/$($result.summary.evidence_file_count) stale_files=$($result.summary.stale_file_count) evidence_fresh_all=$($result.summary.evidence_fresh_all)"
Write-Host "report_rounds=$($result.summary.unattended_report_rounds) latest_ledger_round=$($result.summary.latest_ledger_round) latest_ledger_success=$($result.summary.latest_ledger_success)"
Write-Host "report_ledger_round_mismatch=$($result.summary.report_ledger_round_mismatch) requires_unattended_report_refresh=$($result.summary.requires_unattended_report_refresh)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
