param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-contract-manifest.ps1"),
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

function Invoke-Manifest {
    param([string]$InputRepoRoot)

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $InputRepoRoot -Json
    if ($LASTEXITCODE -ne 0) {
        throw "contract manifest exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$manifest = Invoke-Manifest -InputRepoRoot $root

Assert-True ($manifest.contract_version -eq "smartsteam.remote-gemma-unattended.contract-manifest.v1") "contract manifest version mismatch"
Assert-True ($manifest.read_only -eq $true) "manifest must be read-only"
Assert-True ($manifest.starts_process -eq $false) "manifest must not start processes"
Assert-True ($manifest.sends_prompt -eq $false) "manifest must not send prompts"
Assert-True ($manifest.touches_remote -eq $false) "manifest must not touch remote"
Assert-True ($manifest.writes_files -eq $false) "manifest must not write files"
Assert-True ($manifest.writes_model_weights -eq $false) "manifest must not write model weights"
Assert-True ($manifest.authorization.can_authorize_daemon -eq $false) "manifest must not authorize daemon"
Assert-True ($manifest.authorization.can_authorize_launch -eq $false) "manifest must not authorize launch"
Assert-True ($manifest.authorization.can_authorize_prompt -eq $false) "manifest must not authorize prompt"
Assert-True ($manifest.authorization.can_authorize_ssh -eq $false) "manifest must not authorize ssh"
Assert-True (@($manifest.readers).Count -ge 5) "manifest must list readers"
Assert-True (@($manifest.selftests).Count -ge 2) "manifest must list selftests"
Assert-True (@($manifest.current_surface.known_consumer_ids).Count -ge 7) "manifest must expose consumer ids"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$manifest.current_surface.display.severity)) "manifest must expose current display severity"
Assert-True ($manifest.safety.blocked_exit_code -eq 2) "manifest blocked exit code mismatch"
Assert-True ($manifest.safety.unknown_consumer_exit_code -eq 3) "manifest unknown consumer exit code mismatch"
Assert-True (@($manifest.readers | Where-Object { [string]::IsNullOrWhiteSpace([string]$_.id) -or [string]::IsNullOrWhiteSpace([string]$_.command) -or [string]::IsNullOrWhiteSpace([string]$_.contract_version) -or $_.timeout_seconds -le 0 }).Count -eq 0) "reader entries must include id, command, contract_version, and timeout"
Assert-True (@($manifest.readers | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "reader entries must stay safe"
Assert-True (@($manifest.readers | Where-Object { $_.id -eq "evidence_freshness" -and $_.contract_version -eq "smartsteam.remote-gemma-unattended.evidence-freshness.v1" }).Count -eq 1) "manifest must describe evidence freshness reader"
Assert-True (@($manifest.readers | Where-Object { $_.id -eq "model_pool_guard" -and $_.contract_version -eq "smartsteam.remote-gemma-unattended.model-pool-guard.v1" -and $_.supports_fail_on_blocked -eq $true }).Count -eq 1) "manifest must describe model-pool guard reader"
Assert-True (@($manifest.readers | Where-Object { $_.id -eq "evolution_loop_guard" -and $_.contract_version -eq "smartsteam.remote-gemma-unattended.evolution-loop-guard.v1" -and $_.supports_fail_on_blocked -eq $true }).Count -eq 1) "manifest must describe evolution loop guard reader"
Assert-True (@($manifest.readers | Where-Object { $_.id -eq "action_matrix" -and $_.contract_version -eq "smartsteam.remote-gemma-unattended.action-matrix.v1" }).Count -eq 1) "manifest must describe action matrix reader"
Assert-True (@($manifest.readers | Where-Object { $_.id -eq "dashboard_status" -and $_.contract_version -eq "smartsteam.remote-gemma-unattended.dashboard-status.v1" }).Count -eq 1) "manifest must describe dashboard status reader"
Assert-True (@($manifest.readers | Where-Object { $_.id -eq "surface_preflight" -and $_.supports_fail_on_blocked -eq $true -and $_.supports_consumer_id -eq $true }).Count -eq 1) "manifest must describe surface preflight options"
Assert-True (@($manifest.readers | Where-Object { $_.id -eq "consumer_preflight" -and $_.supports_fail_on_blocked -eq $true -and $_.supports_consumer_id -eq $true }).Count -eq 1) "manifest must describe consumer preflight options"
Assert-True (@($manifest.selftests | Where-Object { $_.id -eq "dashboard_status" -and $_.contract_version -eq "smartsteam.remote-gemma-unattended.dashboard-status-selftest.v1" }).Count -eq 1) "manifest must describe dashboard status selftest"
Assert-True (@($manifest.selftests | Where-Object { $_.id -eq "action_matrix" -and $_.contract_version -eq "smartsteam.remote-gemma-unattended.action-matrix-selftest.v1" }).Count -eq 1) "manifest must describe action matrix selftest"
Assert-True (@($manifest.selftests | Where-Object { $_.id -eq "evolution_loop_guard" -and $_.contract_version -eq "smartsteam.remote-gemma-unattended.evolution-loop-guard-selftest.v1" }).Count -eq 1) "manifest must describe evolution loop guard selftest"
Assert-True (@($manifest.selftests | Where-Object { $_.id -eq "model_pool_guard" -and $_.contract_version -eq "smartsteam.remote-gemma-unattended.model-pool-guard-selftest.v1" }).Count -eq 1) "manifest must describe model-pool guard selftest"
Assert-True (@($manifest.selftests | Where-Object { $_.id -eq "evidence_freshness" -and $_.contract_version -eq "smartsteam.remote-gemma-unattended.evidence-freshness-selftest.v1" }).Count -eq 1) "manifest must describe evidence freshness selftest"
Assert-True (@($manifest.selftests | Where-Object { $_.id -eq "quick_contract" -and $_.runs_fixtures -eq $false }).Count -eq 1) "manifest must describe quick contract"
Assert-True (@($manifest.selftests | Where-Object { $_.id -eq "readonly_contract_full" -and $_.runs_fixtures -eq $true -and $_.timeout_seconds -ge 180 }).Count -eq 1) "manifest must describe full contract"
Assert-True (@($manifest.selftests | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "selftest entries must stay safe"

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.contract-manifest-selftest.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        manifest_contract = $manifest.contract_version
        reader_count = @($manifest.readers).Count
        selftest_count = @($manifest.selftests).Count
        consumer_count = @($manifest.current_surface.known_consumer_ids).Count
        current_display_severity = $manifest.current_surface.display.severity
        blocked_exit_code = $manifest.safety.blocked_exit_code
        unknown_consumer_exit_code = $manifest.safety.unknown_consumer_exit_code
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "contract_manifest_selftest_is_read_only_and_fail_closed"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
    exit 0
}

Write-Host "read-remote-contract-manifest selftest passed"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "readers=$($result.summary.reader_count) selftests=$($result.summary.selftest_count) consumers=$($result.summary.consumer_count)"
Write-Host "display=$($result.summary.current_display_severity) blocked_exit_code=$($result.summary.blocked_exit_code) unknown_consumer_exit_code=$($result.summary.unknown_consumer_exit_code)"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
