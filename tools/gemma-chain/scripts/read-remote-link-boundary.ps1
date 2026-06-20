param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
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
$readinessScript = Join-Path $PSScriptRoot "read-remote-readiness-contract.ps1"

$snapshot = Invoke-JsonScript -ScriptPath $snapshotScript -ScriptArgs @("-RepoRoot", $root)
$readiness = Invoke-JsonScript -ScriptPath $readinessScript -ScriptArgs @("-RepoRoot", $root)

$workerPorts = @($snapshot.model_pool.workers | ForEach-Object {
    [pscustomobject]@{
        role = $_.role
        port = $_.port
        status = $_.status
        ready = $_.ready
        runtime_backend = $_.runtime_backend
        runtime_accelerator = $_.runtime_accelerator
        model_cache_ok = $_.model_cache_ok
        evidence_source = "target\\remote-gemma-chain\\status-with-model-cache.json"
        evidence_kind = "local_snapshot"
        realtime_verified_by_this_script = $false
    }
})

$snapshotFiles = @(
    $snapshot.evidence.PSObject.Properties | ForEach-Object {
        [pscustomobject]@{
            id = $_.Name
            path = $_.Value.path
            exists = $_.Value.exists
            fresh = $_.Value.fresh
            age_seconds = $_.Value.age_seconds
            last_write_time_utc = $_.Value.last_write_time_utc
            evidence_kind = "local_snapshot_file"
        }
    }
)

$serviceTopology = [pscustomobject]@{
    model_workers = $workerPorts
    backend = [pscustomobject]@{
        host = "127.0.0.1"
        port = 7979
        ready_snapshot = $snapshot.chain.backend
        evidence_source = "target\\remote-gemma-chain\\status-with-model-cache.json"
        relationship = "Web Lab/CLI backend fan-out into the Gemma worker/model API chain."
        evidence_kind = "local_snapshot"
        realtime_verified_by_this_script = $false
    }
    web_lab = [pscustomobject]@{
        host = "127.0.0.1"
        port = 8789
        ready_snapshot = $snapshot.chain.web_lab
        evidence_source = "target\\remote-gemma-chain\\status-with-model-cache.json"
        relationship = "Browser-facing lab surface; prompts must still pass consumer/readiness gates."
        evidence_kind = "local_snapshot"
        realtime_verified_by_this_script = $false
    }
    external_remote_ports = [pscustomobject]@{
        host = "smartsteam-mac"
        ports = @(8686, 8687, 8688, 8689, 8690)
        externally_reported_state = "llama-server listening per controller-window sync"
        proof_source = "operator_context_not_local_artifact"
        freshness = "not_asserted_by_this_script"
        observed_by_this_script = $false
        evidence_kind = "external_sync_note_not_reverified"
        note = "This script does not SSH, probe ports, or touch remote; treat this as operator-supplied context only."
    }
}

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.link-boundary.v1"
    generated_at = $generatedAt.ToString("yyyy-MM-dd HH:mm:ss zzz")
    generated_at_utc = $generatedAtUtc.ToString("o")
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        snapshot_classification = $readiness.summary.snapshot_classification
        evidence_fresh_all = $readiness.summary.evidence_fresh_all
        missing_evidence = @($readiness.summary.missing_evidence)
        pending_external_gates = @($readiness.summary.pending_external_gates)
        worker_count_snapshot = $snapshot.model_pool.worker_count
        healthy_worker_count_snapshot = $snapshot.model_pool.healthy_worker_count
        model_cache_model_count_snapshot = $snapshot.model_cache.model_count
        model_cache_ok_count_snapshot = $snapshot.model_cache.ok_count
        unattended_rounds_snapshot = $snapshot.unattended.rounds
        unattended_success_snapshot = $snapshot.unattended.success
        backend_ready_snapshot = $snapshot.chain.backend
        web_lab_ready_snapshot = $snapshot.chain.web_lab
        realtime_ports_verified_by_this_script = $false
        historical_snapshot_authorizes_current_residency = $false
        package_ready_for_external_gate = $readiness.summary.can_support_external_residency_review
    }
    evidence_layers = [pscustomobject]@{
        local_snapshot_files = $snapshotFiles
        external_realtime_sync = $serviceTopology.external_remote_ports
        readiness_contract = [pscustomobject]@{
            contract_version = $readiness.contract_version
            generated_at_utc = $readiness.generated_at_utc
            can_support_external_residency_review = $readiness.summary.can_support_external_residency_review
            evidence_fresh_all = $readiness.summary.evidence_fresh_all
        }
    }
    service_topology = $serviceTopology
    consumer_projection = $readiness.consumer_projection
    next_read_only_verifiers = @(
        [pscustomobject]@{ id = "snapshot_summary"; command = ".\tools\gemma-chain\scripts\read-remote-unattended-snapshot.ps1 -Json"; read_only = $true },
        [pscustomobject]@{ id = "surface_preflight"; command = ".\tools\gemma-chain\scripts\read-remote-surface-preflight.ps1 -Json"; read_only = $true },
        [pscustomobject]@{ id = "observation_window"; command = ".\tools\gemma-chain\scripts\read-remote-observation-window.ps1 -Json"; read_only = $true },
        [pscustomobject]@{ id = "resource_window"; command = ".\tools\gemma-chain\scripts\read-remote-resource-window.ps1 -Json"; read_only = $true },
        [pscustomobject]@{ id = "evidence_package_status"; command = ".\tools\gemma-chain\scripts\read-remote-evidence-package-status.ps1 -Json"; read_only = $true }
    )
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "link_boundary_is_read_only_and_external_ports_are_not_verified_by_this_script"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 10
    exit 0
}

Write-Host "Gemma remote link boundary"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "snapshot=$($result.summary.snapshot_classification) evidence_fresh_all=$($result.summary.evidence_fresh_all) package_ready_for_external_gate=$($result.summary.package_ready_for_external_gate)"
Write-Host "workers_snapshot=$($result.summary.healthy_worker_count_snapshot)/$($result.summary.worker_count_snapshot) cache_snapshot=$($result.summary.model_cache_ok_count_snapshot)/$($result.summary.model_cache_model_count_snapshot) unattended_snapshot=$($result.summary.unattended_success_snapshot)/$($result.summary.unattended_rounds_snapshot)"
Write-Host "backend_7979_snapshot=$($result.summary.backend_ready_snapshot) web_lab_8789_snapshot=$($result.summary.web_lab_ready_snapshot) remote_ports_8686_8690_verified_by_this_script=False"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
