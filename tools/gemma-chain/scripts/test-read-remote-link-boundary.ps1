param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-link-boundary.ps1"),
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

function Invoke-LinkBoundary {
    param([string]$InputRepoRoot)

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $InputRepoRoot -Json
    if ($LASTEXITCODE -ne 0) {
        throw "link boundary exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$boundary = Invoke-LinkBoundary -InputRepoRoot $root

Assert-True ($boundary.contract_version -eq "smartsteam.remote-gemma-unattended.link-boundary.v1") "link boundary contract version mismatch"
Assert-True ($boundary.read_only -eq $true) "link boundary must be read-only"
Assert-True ($boundary.starts_process -eq $false) "link boundary must not start processes"
Assert-True ($boundary.sends_prompt -eq $false) "link boundary must not send prompts"
Assert-True ($boundary.touches_remote -eq $false) "link boundary must not touch remote"
Assert-True ($boundary.writes_files -eq $false) "link boundary must not write files"
Assert-True ($boundary.writes_model_weights -eq $false) "link boundary must not write model weights"
Assert-True ($boundary.authorization.can_authorize_daemon -eq $false) "link boundary must not authorize daemon"
Assert-True ($boundary.authorization.can_authorize_launch -eq $false) "link boundary must not authorize launch"
Assert-True ($boundary.authorization.can_authorize_prompt -eq $false) "link boundary must not authorize prompt"
Assert-True ($boundary.authorization.can_authorize_ssh -eq $false) "link boundary must not authorize ssh"
Assert-True ($boundary.summary.historical_snapshot_authorizes_current_residency -eq $false) "historical snapshot must not authorize residency"
Assert-True ($boundary.summary.realtime_ports_verified_by_this_script -eq $false) "link boundary must not claim live port verification"
Assert-True ($boundary.evidence_layers.external_realtime_sync.observed_by_this_script -eq $false) "external sync must be marked not observed by this script"
Assert-True ($boundary.evidence_layers.external_realtime_sync.evidence_kind -eq "external_sync_note_not_reverified") "external sync evidence kind mismatch"
Assert-True (@($boundary.service_topology.external_remote_ports.ports).Count -eq 5) "external port map should list 8686-8690"
Assert-True ((@($boundary.service_topology.external_remote_ports.ports) -join ",") -eq "8686,8687,8688,8689,8690") "external port map mismatch"
Assert-True ($boundary.service_topology.backend.port -eq 7979) "backend port mismatch"
Assert-True ($boundary.service_topology.web_lab.port -eq 8789) "Web Lab port mismatch"
Assert-True (@($boundary.service_topology.model_workers).Count -ge 6) "worker topology should expose pool workers"
Assert-True (@($boundary.service_topology.model_workers | Where-Object { $_.realtime_verified_by_this_script -ne $false }).Count -eq 0) "workers must be marked snapshot-only"
Assert-True (@($boundary.evidence_layers.local_snapshot_files).Count -ge 4) "local snapshot files must be listed"
Assert-True (@($boundary.consumer_projection).Count -ge 7) "consumer projection must be exposed"
Assert-True (@($boundary.consumer_projection | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "consumers must remain fail-closed"
Assert-True (@($boundary.next_read_only_verifiers | Where-Object { $_.read_only -ne $true -or $_.command -match '\bssh\b|ssh\.exe|plink|\bStart\b|Start-Process|prompt_round|model_pool_launch' }).Count -eq 0) "next verifiers must stay read-only"

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.link-boundary-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        link_boundary_contract = $boundary.contract_version
        snapshot_classification = $boundary.summary.snapshot_classification
        evidence_fresh_all = $boundary.summary.evidence_fresh_all
        worker_count_snapshot = $boundary.summary.worker_count_snapshot
        healthy_worker_count_snapshot = $boundary.summary.healthy_worker_count_snapshot
        backend_ready_snapshot = $boundary.summary.backend_ready_snapshot
        web_lab_ready_snapshot = $boundary.summary.web_lab_ready_snapshot
        remote_ports_verified_by_this_script = $boundary.summary.realtime_ports_verified_by_this_script
        package_ready_for_external_gate = $boundary.summary.package_ready_for_external_gate
        missing_evidence = @($boundary.summary.missing_evidence)
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "link_boundary_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-link-boundary selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "workers=$($result.summary.healthy_worker_count_snapshot)/$($result.summary.worker_count_snapshot) backend_7979=$($result.summary.backend_ready_snapshot) web_lab_8789=$($result.summary.web_lab_ready_snapshot)"
Write-Host "remote_ports_verified_by_this_script=$($result.summary.remote_ports_verified_by_this_script) package_ready_for_external_gate=$($result.summary.package_ready_for_external_gate)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
