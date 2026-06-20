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
$surfaceScript = Join-Path $PSScriptRoot "read-remote-surface-preflight.ps1"
$linkBoundaryScript = Join-Path $PSScriptRoot "read-remote-link-boundary.ps1"
$packageStatusScript = Join-Path $PSScriptRoot "read-remote-evidence-package-status.ps1"

$surface = Invoke-JsonScript -ScriptPath $surfaceScript -ScriptArgs @("-RepoRoot", $root)
$linkBoundary = Invoke-JsonScript -ScriptPath $linkBoundaryScript -ScriptArgs @("-RepoRoot", $root)
$packageStatus = Invoke-JsonScript -ScriptPath $packageStatusScript -ScriptArgs @("-RepoRoot", $root)

$blockedConsumers = @($surface.consumers | Where-Object { $_.current_allowed -ne $true })
$packageItems = @($packageStatus.package_items | ForEach-Object {
    [pscustomobject]@{
        id = $_.id
        ready = $_.ready
        status = $_.status
        required_evidence = $_.required_evidence
        verifier_command = $_.verifier_command
        proof_source = $_.proof_source
    }
})

$dashboardCards = @(
    [pscustomobject]@{
        id = "action_lock"
        title = "Action lock"
        severity = $surface.display.severity
        value = if ($blockedConsumers.Count -gt 0) { "blocked" } else { "external_gate_required" }
        detail = "prompt, launch, SSH, and daemon actions are not authorized by this dashboard."
        ready = $false
    },
    [pscustomobject]@{
        id = "freshness"
        title = "Evidence freshness"
        severity = if ($surface.status.evidence_fresh_all -eq $true) { "ok" } else { "blocked" }
        value = if ($surface.status.evidence_fresh_all -eq $true) { "fresh" } else { "stale" }
        detail = "max_evidence_age_seconds=$($surface.status.max_evidence_age_seconds); fresh_minutes=$($surface.status.fresh_minutes)"
        ready = [bool]$surface.status.evidence_fresh_all
    },
    [pscustomobject]@{
        id = "worker_pool_snapshot"
        title = "Worker pool snapshot"
        severity = if ($linkBoundary.summary.healthy_worker_count_snapshot -eq $linkBoundary.summary.worker_count_snapshot) { "snapshot_ok" } else { "snapshot_degraded" }
        value = "$($linkBoundary.summary.healthy_worker_count_snapshot)/$($linkBoundary.summary.worker_count_snapshot)"
        detail = "snapshot-only worker ports 8686-8690; realtime_verified_by_this_script=false"
        ready = $false
    },
    [pscustomobject]@{
        id = "web_lab_backend_snapshot"
        title = "Web Lab/backend snapshot"
        severity = if ($linkBoundary.summary.backend_ready_snapshot -eq $true -and $linkBoundary.summary.web_lab_ready_snapshot -eq $true) { "snapshot_ok" } else { "snapshot_degraded" }
        value = "backend_7979=$($linkBoundary.summary.backend_ready_snapshot); web_lab_8789=$($linkBoundary.summary.web_lab_ready_snapshot)"
        detail = "snapshot-only; prompts still require consumer/readiness gates."
        ready = $false
    },
    [pscustomobject]@{
        id = "evidence_package"
        title = "Evidence package"
        severity = if ($packageStatus.summary.package_ready_for_external_gate -eq $true) { "external_gate_ready" } else { "blocked" }
        value = "$($packageStatus.summary.ready_item_count)/$($packageStatus.summary.total_item_count)"
        detail = "ready package only enters external gate review; it never authorizes actions directly."
        ready = [bool]$packageStatus.summary.package_ready_for_external_gate
    },
    [pscustomobject]@{
        id = "next_gap"
        title = "Next evidence gap"
        severity = if (@($surface.status.missing_evidence).Count -gt 0) { "blocked" } else { "external_gate_required" }
        value = if (@($surface.status.missing_evidence).Count -gt 0) { @($surface.status.missing_evidence)[0] } else { @($surface.status.pending_external_gates)[0] }
        detail = $surface.display.next_action_label
        ready = $false
    }
)

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.dashboard-status.v1"
    generated_at = $generatedAt.ToString("yyyy-MM-dd HH:mm:ss zzz")
    generated_at_utc = $generatedAtUtc.ToString("o")
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    display = $surface.display
    headline = $surface.display.headline
    status = [pscustomobject]@{
        package_ready_for_external_gate = $packageStatus.summary.package_ready_for_external_gate
        can_support_external_residency_review = $packageStatus.summary.can_support_external_residency_review
        snapshot_classification = $surface.status.snapshot_classification
        evidence_fresh_all = $surface.status.evidence_fresh_all
        max_evidence_age_seconds = $surface.status.max_evidence_age_seconds
        observation_window_status = $surface.status.observation_window_status
        continuous_window_present = $surface.status.continuous_window_present
        resource_window_status = $surface.status.resource_window_status
        resource_window_present = $surface.status.resource_window_present
        ready_item_count = $packageStatus.summary.ready_item_count
        total_item_count = $packageStatus.summary.total_item_count
        consumer_allowed_count = $surface.status.consumer_allowed_count
        unsafe_safe_command_count = $surface.status.unsafe_safe_command_count
        missing_evidence = @($surface.status.missing_evidence)
        pending_external_gates = @($surface.status.pending_external_gates)
    }
    dashboard_cards = $dashboardCards
    package_items = $packageItems
    topology = [pscustomobject]@{
        worker_ports_snapshot = $linkBoundary.service_topology.model_workers
        backend = $linkBoundary.service_topology.backend
        web_lab = $linkBoundary.service_topology.web_lab
        external_remote_ports = $linkBoundary.service_topology.external_remote_ports
    }
    consumers = $surface.consumers
    source_contracts = [pscustomobject]@{
        surface_preflight = $surface.contract_version
        link_boundary = $linkBoundary.contract_version
        evidence_package_status = $packageStatus.contract_version
    }
    recommended_read_only_entrypoints = @(
        [pscustomobject]@{ id = "dashboard_status"; command = ".\tools\gemma-chain\scripts\read-remote-dashboard-status.ps1 -Json"; read_only = $true },
        [pscustomobject]@{ id = "surface_preflight"; command = ".\tools\gemma-chain\scripts\read-remote-surface-preflight.ps1 -Json"; read_only = $true },
        [pscustomobject]@{ id = "consumer_preflight"; command = ".\tools\gemma-chain\scripts\read-remote-consumer-preflight.ps1 -Json"; read_only = $true },
        [pscustomobject]@{ id = "link_boundary"; command = ".\tools\gemma-chain\scripts\read-remote-link-boundary.ps1 -Json"; read_only = $true },
        [pscustomobject]@{ id = "evidence_package_status"; command = ".\tools\gemma-chain\scripts\read-remote-evidence-package-status.ps1 -Json"; read_only = $true }
    )
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "dashboard_status_is_read_only_and_cannot_authorize_actions"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 10
    exit 0
}

Write-Host "Gemma remote dashboard status"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "display=$($result.display.severity) headline=$($result.headline)"
Write-Host "package_ready_for_external_gate=$($result.status.package_ready_for_external_gate) ready_items=$($result.status.ready_item_count)/$($result.status.total_item_count)"
Write-Host "workers_snapshot=$($linkBoundary.summary.healthy_worker_count_snapshot)/$($linkBoundary.summary.worker_count_snapshot) backend_7979=$($linkBoundary.summary.backend_ready_snapshot) web_lab_8789=$($linkBoundary.summary.web_lab_ready_snapshot)"
Write-Host "remote_ports_verified_by_this_script=$($linkBoundary.summary.realtime_ports_verified_by_this_script)"
Write-Host "missing_evidence=$($result.status.missing_evidence -join ',') pending_external_gates=$($result.status.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
