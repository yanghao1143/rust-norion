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
$consumerScript = Join-Path $PSScriptRoot "read-remote-consumer-preflight.ps1"
$dashboardScript = Join-Path $PSScriptRoot "read-remote-dashboard-status.ps1"

$consumerPreflight = Invoke-JsonScript -ScriptPath $consumerScript -ScriptArgs @("-RepoRoot", $root)
$dashboard = Invoke-JsonScript -ScriptPath $dashboardScript -ScriptArgs @("-RepoRoot", $root)

function Get-ActionLabel {
    param([string]$ConsumerId)

    switch ($ConsumerId) {
        "web_lab_prompt" { "Web Lab prompt" }
        "forge_cli_prompt" { "Forge CLI prompt" }
        "backend_cli_direct_prompt" { "Backend direct prompt" }
        "evolution_loop_prompt_round" { "Evolution loop prompt round" }
        "model_pool_launch" { "Model pool launch/expand" }
        "forge_daemon_residency" { "Forge daemon residency" }
        "ssh_remote_probe" { "Remote SSH probe" }
        default { $ConsumerId }
    }
}

$actions = @($consumerPreflight.consumers | ForEach-Object {
    $consumer = $_
    [pscustomobject]@{
        id = $consumer.id
        label = Get-ActionLabel -ConsumerId $consumer.id
        surface = $consumer.surface
        entrypoint_kind = $consumer.entrypoint_kind
        current_allowed = $false
        ui_enabled = $false
        cli_may_execute = $false
        blocked_by = @($consumer.blocked_by)
        reason = $consumer.reason
        downstream_sends_prompt = $consumer.downstream_sends_prompt
        downstream_launches_process = $consumer.downstream_launches_process
        downstream_touches_remote = $consumer.downstream_touches_remote
        safe_command_id = $consumer.safe_command_id
        verifier_command = if ($null -ne $consumer.safe_command) { $consumer.safe_command.command } else { "" }
        verifier_read_only = if ($null -ne $consumer.safe_command) { $consumer.safe_command.read_only } else { $false }
        verifier_starts_process = if ($null -ne $consumer.safe_command) { $consumer.safe_command.starts_process } else { $true }
        verifier_sends_prompt = if ($null -ne $consumer.safe_command) { $consumer.safe_command.sends_prompt } else { $true }
        verifier_touches_remote = if ($null -ne $consumer.safe_command) { $consumer.safe_command.touches_remote } else { $true }
        verifier_writes_files = if ($null -ne $consumer.safe_command) { $consumer.safe_command.writes_files } else { $true }
        tooltip = "$($consumer.reason) Next safe verifier: $($consumer.safe_command_id)."
        next_user_visible_status = $consumer.next_user_visible_status
    }
})

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.action-matrix.v1"
    generated_at = $generatedAt.ToString("yyyy-MM-dd HH:mm:ss zzz")
    generated_at_utc = $generatedAtUtc.ToString("o")
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    display = $dashboard.display
    summary = [pscustomobject]@{
        action_count = @($actions).Count
        allowed_count = @($actions | Where-Object { $_.current_allowed -eq $true }).Count
        blocked_count = @($actions | Where-Object { $_.current_allowed -ne $true }).Count
        ui_enabled_count = @($actions | Where-Object { $_.ui_enabled -eq $true }).Count
        cli_executable_count = @($actions | Where-Object { $_.cli_may_execute -eq $true }).Count
        package_ready_for_external_gate = $dashboard.status.package_ready_for_external_gate
        evidence_fresh_all = $dashboard.status.evidence_fresh_all
        missing_evidence = @($dashboard.status.missing_evidence)
        pending_external_gates = @($dashboard.status.pending_external_gates)
    }
    actions = $actions
    action_policy = [pscustomobject]@{
        fail_closed_default = $true
        allowed_requires_external_gate = $true
        blocked_actions_must_not_execute = $true
        ui_should_disable_blocked_actions = $true
        cli_should_return_blocked_exit_code = $true
        blocked_exit_code = $consumerPreflight.fail_on_blocked_exit_code
        unknown_consumer_exit_code = 3
        verifier_commands_are_hints_not_auto_run = $true
    }
    source_contracts = [pscustomobject]@{
        consumer_preflight = $consumerPreflight.contract_version
        dashboard_status = $dashboard.contract_version
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "action_matrix_is_read_only_and_cannot_authorize_actions"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 10
    exit 0
}

Write-Host "Gemma remote action matrix"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "display=$($result.display.severity) actions=$($result.summary.action_count) allowed=$($result.summary.allowed_count) blocked=$($result.summary.blocked_count)"
Write-Host "ui_enabled=$($result.summary.ui_enabled_count) cli_executable=$($result.summary.cli_executable_count) package_ready_for_external_gate=$($result.summary.package_ready_for_external_gate)"
Write-Host "missing_evidence=$($result.summary.missing_evidence -join ',') pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
