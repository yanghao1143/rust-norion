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
$surface = Invoke-JsonScript -ScriptPath $surfaceScript -ScriptArgs @("-RepoRoot", $root)

$readers = @(
    [pscustomobject]@{
        id = "evidence_freshness"
        contract_version = "smartsteam.remote-gemma-unattended.evidence-freshness.v1"
        command = ".\tools\gemma-chain\scripts\read-remote-evidence-freshness.ps1 -Json"
        recommended_for = @("freshness diagnostics", "operator handoff", "dashboard detail panel")
        timeout_seconds = 60
        supports_consumer_id = $false
        supports_fail_on_blocked = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "model_pool_guard"
        contract_version = "smartsteam.remote-gemma-unattended.model-pool-guard.v1"
        command = ".\tools\gemma-chain\scripts\read-remote-model-pool-guard.ps1 -Json"
        recommended_for = @("model-pool launch guard", "worker expansion guard", "capacity status panel")
        timeout_seconds = 75
        supports_consumer_id = $false
        supports_fail_on_blocked = $true
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "evolution_loop_guard"
        contract_version = "smartsteam.remote-gemma-unattended.evolution-loop-guard.v1"
        command = ".\tools\gemma-chain\scripts\read-remote-evolution-loop-guard.ps1 -Json"
        recommended_for = @("evolution-loop prompt guard", "Forge daemon residency guard", "resident loop preflight")
        timeout_seconds = 75
        supports_consumer_id = $false
        supports_fail_on_blocked = $true
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "action_matrix"
        contract_version = "smartsteam.remote-gemma-unattended.action-matrix.v1"
        command = ".\tools\gemma-chain\scripts\read-remote-action-matrix.ps1 -Json"
        recommended_for = @("Web Lab action buttons", "Forge/CLI command guards", "evolution-loop preflight UI")
        timeout_seconds = 60
        supports_consumer_id = $false
        supports_fail_on_blocked = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "dashboard_status"
        contract_version = "smartsteam.remote-gemma-unattended.dashboard-status.v1"
        command = ".\tools\gemma-chain\scripts\read-remote-dashboard-status.ps1 -Json"
        recommended_for = @("Web Lab dashboard", "Forge status panel", "CLI status summary")
        timeout_seconds = 45
        supports_consumer_id = $false
        supports_fail_on_blocked = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "link_boundary"
        contract_version = "smartsteam.remote-gemma-unattended.link-boundary.v1"
        command = ".\tools\gemma-chain\scripts\read-remote-link-boundary.ps1 -Json"
        recommended_for = @("topology display", "operator handoff", "Web/Forge/CLI status details")
        timeout_seconds = 30
        supports_consumer_id = $false
        supports_fail_on_blocked = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "surface_preflight"
        contract_version = "smartsteam.remote-gemma-unattended.surface-preflight.v1"
        command = ".\tools\gemma-chain\scripts\read-remote-surface-preflight.ps1 -Json"
        recommended_for = @("Web Lab status bar", "Forge status bar", "CLI frequent polling")
        timeout_seconds = 15
        supports_consumer_id = $true
        supports_fail_on_blocked = $true
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "consumer_preflight"
        contract_version = "smartsteam.remote-gemma-unattended.consumer-preflight.v1"
        command = ".\tools\gemma-chain\scripts\read-remote-consumer-preflight.ps1 -Json"
        recommended_for = @("per-consumer gate", "CLI command guard")
        timeout_seconds = 15
        supports_consumer_id = $true
        supports_fail_on_blocked = $true
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "readiness_contract"
        contract_version = "smartsteam.remote-gemma-unattended.readiness-contract.v1"
        command = ".\tools\gemma-chain\scripts\read-remote-readiness-contract.ps1 -Json"
        recommended_for = @("machine integration", "full readiness snapshot")
        timeout_seconds = 30
        supports_consumer_id = $false
        supports_fail_on_blocked = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "evidence_package_status"
        contract_version = "smartsteam.remote-gemma-unattended.evidence-package-status.v1"
        command = ".\tools\gemma-chain\scripts\read-remote-evidence-package-status.ps1 -Json"
        recommended_for = @("evidence package polling", "external gate preparation")
        timeout_seconds = 30
        supports_consumer_id = $false
        supports_fail_on_blocked = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "owner_flow_handoff"
        contract_version = "smartsteam.remote-gemma-unattended.owner-flow-handoff.v1"
        command = ".\tools\gemma-chain\scripts\read-remote-owner-flow-handoff.ps1 -Json"
        recommended_for = @("handoff to approved owner-flow", "operator planning")
        timeout_seconds = 45
        supports_consumer_id = $false
        supports_fail_on_blocked = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    }
)

$selftests = @(
    [pscustomobject]@{
        id = "evidence_freshness"
        contract_version = "smartsteam.remote-gemma-unattended.evidence-freshness-selftest.v1"
        command = ".\tools\gemma-chain\scripts\test-read-remote-evidence-freshness.ps1"
        recommended_for = @("freshness reader changes", "operator handoff")
        timeout_seconds = 90
        runs_fixtures = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "model_pool_guard"
        contract_version = "smartsteam.remote-gemma-unattended.model-pool-guard-selftest.v1"
        command = ".\tools\gemma-chain\scripts\test-read-remote-model-pool-guard.ps1"
        recommended_for = @("model-pool guard changes", "worker pool handoff")
        timeout_seconds = 150
        runs_fixtures = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "evolution_loop_guard"
        contract_version = "smartsteam.remote-gemma-unattended.evolution-loop-guard-selftest.v1"
        command = ".\tools\gemma-chain\scripts\test-read-remote-evolution-loop-guard.ps1"
        recommended_for = @("evolution-loop guard changes", "daemon residency handoff")
        timeout_seconds = 150
        runs_fixtures = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "action_matrix"
        contract_version = "smartsteam.remote-gemma-unattended.action-matrix-selftest.v1"
        command = ".\tools\gemma-chain\scripts\test-read-remote-action-matrix.ps1"
        recommended_for = @("action matrix reader changes", "Web/Forge/CLI gate handoff")
        timeout_seconds = 180
        runs_fixtures = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "dashboard_status"
        contract_version = "smartsteam.remote-gemma-unattended.dashboard-status-selftest.v1"
        command = ".\tools\gemma-chain\scripts\test-read-remote-dashboard-status.ps1"
        recommended_for = @("dashboard reader changes", "Web/Forge/CLI handoff")
        timeout_seconds = 90
        runs_fixtures = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "link_boundary"
        contract_version = "smartsteam.remote-gemma-unattended.link-boundary-selftest.v1"
        command = ".\tools\gemma-chain\scripts\test-read-remote-link-boundary.ps1"
        recommended_for = @("topology reader changes", "operator handoff")
        timeout_seconds = 60
        runs_fixtures = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "quick_contract"
        contract_version = "smartsteam.remote-gemma-unattended.quick-readiness-contract-selftest.v1"
        command = ".\tools\gemma-chain\scripts\test-remote-readiness-quick-contract.ps1"
        recommended_for = @("daily preflight", "Web/Forge/CLI startup guard")
        timeout_seconds = 180
        runs_fixtures = $false
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "readonly_contract_full"
        contract_version = "smartsteam.remote-gemma-unattended.readonly-contract-selftest.v1"
        command = ".\tools\gemma-chain\scripts\test-remote-readiness-readonly-contract.ps1"
        recommended_for = @("script changes", "handoff", "CI deep check")
        timeout_seconds = 540
        runs_fixtures = $true
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    }
)

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.contract-manifest.v1"
    generated_at = $generatedAt.ToString("yyyy-MM-dd HH:mm:ss zzz")
    generated_at_utc = $generatedAtUtc.ToString("o")
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    current_surface = [pscustomobject]@{
        contract_version = $surface.contract_version
        display = $surface.display
        known_consumer_ids = $surface.known_consumer_ids
        fail_on_blocked_exit_code = $surface.fail_on_blocked_exit_code
    }
    readers = $readers
    selftests = $selftests
    safety = [pscustomobject]@{
        default_authorization = "fail_closed"
        blocked_exit_code = 2
        unknown_consumer_exit_code = 3
        no_reader_starts_process = $true
        no_reader_sends_prompt = $true
        no_reader_touches_remote = $true
        no_reader_writes_files = $true
        no_reader_writes_model_weights = $true
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "contract_manifest_is_read_only_and_cannot_authorize_actions"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 10
    exit 0
}

Write-Host "Gemma remote contract manifest"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "generated_at=$($result.generated_at) generated_at_utc=$($result.generated_at_utc)"
Write-Host "surface=$($result.current_surface.display.severity) headline=$($result.current_surface.display.headline)"
Write-Host "readers=$($result.readers.Count) selftests=$($result.selftests.Count) blocked_exit_code=$($result.safety.blocked_exit_code) unknown_consumer_exit_code=$($result.safety.unknown_consumer_exit_code)"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
