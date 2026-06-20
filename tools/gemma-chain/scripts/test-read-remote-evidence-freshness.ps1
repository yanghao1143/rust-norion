param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-evidence-freshness.ps1"),
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

function Invoke-Freshness {
    param([string]$InputRepoRoot)

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $InputRepoRoot -Json
    if ($LASTEXITCODE -ne 0) {
        throw "evidence freshness exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$freshness = Invoke-Freshness -InputRepoRoot $root

Assert-True ($freshness.contract_version -eq "smartsteam.remote-gemma-unattended.evidence-freshness.v1") "freshness contract version mismatch"
Assert-True ($freshness.read_only -eq $true) "freshness reader must be read-only"
Assert-True ($freshness.starts_process -eq $false) "freshness reader must not start processes"
Assert-True ($freshness.sends_prompt -eq $false) "freshness reader must not send prompts"
Assert-True ($freshness.touches_remote -eq $false) "freshness reader must not touch remote"
Assert-True ($freshness.writes_files -eq $false) "freshness reader must not write files"
Assert-True ($freshness.writes_model_weights -eq $false) "freshness reader must not write model weights"
Assert-True ($freshness.authorization.can_authorize_daemon -eq $false) "freshness reader must not authorize daemon"
Assert-True ($freshness.authorization.can_authorize_launch -eq $false) "freshness reader must not authorize launch"
Assert-True ($freshness.authorization.can_authorize_prompt -eq $false) "freshness reader must not authorize prompt"
Assert-True ($freshness.authorization.can_authorize_ssh -eq $false) "freshness reader must not authorize ssh"
Assert-True ($freshness.summary.evidence_file_count -ge 4) "freshness reader must expose evidence files"
Assert-True (($freshness.summary.fresh_file_count + $freshness.summary.stale_file_count) -eq $freshness.summary.evidence_file_count) "fresh/stale counts must add up"
Assert-True (@($freshness.evidence_files | Where-Object { [string]::IsNullOrWhiteSpace([string]$_.id) -or [string]::IsNullOrWhiteSpace([string]$_.path) }).Count -eq 0) "evidence files must include id and path"
Assert-True ($null -ne $freshness.summary.PSObject.Properties["report_ledger_round_mismatch"]) "freshness reader must expose report/ledger mismatch flag"
Assert-True ($null -ne $freshness.summary.PSObject.Properties["latest_ledger_failed"]) "freshness reader must expose latest ledger failure flag"
Assert-True ($null -ne $freshness.summary.PSObject.Properties["requires_unattended_report_refresh"]) "freshness reader must expose report refresh flag"
Assert-True ($freshness.summary.package_ready_for_external_gate -eq $false) "freshness reader must not claim package readiness in current fail-closed state"
Assert-True (@($freshness.summary.missing_evidence | Where-Object { $_ -eq "fresh_snapshot" }).Count -eq 1) "freshness reader must expose fresh_snapshot gap"
Assert-True ($freshness.freshness_interpretation.next_safe_reader -match "read-remote-unattended-snapshot\.ps1 -Json") "freshness reader must point to snapshot reader"

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.evidence-freshness-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        freshness_contract = $freshness.contract_version
        evidence_fresh_all = $freshness.summary.evidence_fresh_all
        fresh_file_count = $freshness.summary.fresh_file_count
        stale_file_count = $freshness.summary.stale_file_count
        report_ledger_round_mismatch = $freshness.summary.report_ledger_round_mismatch
        latest_ledger_failed = $freshness.summary.latest_ledger_failed
        requires_unattended_report_refresh = $freshness.summary.requires_unattended_report_refresh
        missing_evidence = @($freshness.summary.missing_evidence)
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "evidence_freshness_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-evidence-freshness selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "evidence_fresh_all=$($result.summary.evidence_fresh_all) fresh_files=$($result.summary.fresh_file_count) stale_files=$($result.summary.stale_file_count)"
Write-Host "report_ledger_round_mismatch=$($result.summary.report_ledger_round_mismatch) latest_ledger_failed=$($result.summary.latest_ledger_failed) requires_report_refresh=$($result.summary.requires_unattended_report_refresh)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
