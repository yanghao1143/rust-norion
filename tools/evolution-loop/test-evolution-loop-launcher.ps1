param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate SmartSteam evolution-loop launcher command assembly without backend calls, process launch, or prompt sending."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\evolution-loop\test-evolution-loop-launcher.cmd"
    Write-Host ""
    Write-Host "Checks:"
    Write-Host "  - -CheckOnly exits before cargo/backend/prompt work"
    Write-Host "  - -RefreshPoolArtifacts supplies manifest/status/route artifact paths"
    Write-Host "  - -PoolAlignmentGate forwards --pool-alignment-gate"
    Write-Host "  - -RemoteModelPoolGate enables the full remote model-pool gate and helper execution bundle"
    return
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot not found: $RepoRoot"
}

$launcher = Join-Path $RepoRoot "tools\evolution-loop\start-evolution-loop.ps1"
if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) {
    throw "start-evolution-loop.ps1 not found: $launcher"
}

function Invoke-LauncherCase {
    param(
        [string]$Name,
        [string[]]$ArgumentList
    )

    Write-Host ""
    Write-Host "launcher_case=$Name"
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $launcher @ArgumentList 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "launcher case '$Name' failed with exit code $exitCode"
    }

    return $text
}

function Assert-Contains {
    param(
        [string]$Name,
        [string]$Text,
        [string]$Pattern
    )

    if ($Text -notmatch [regex]::Escape($Pattern)) {
        throw "launcher case '$Name' did not contain expected text: $Pattern"
    }
}

function Assert-NotContains {
    param(
        [string]$Name,
        [string]$Text,
        [string]$Pattern
    )

    if ($Text -match [regex]::Escape($Pattern)) {
        throw "launcher case '$Name' unexpectedly contained text: $Pattern"
    }
}

function Get-CommandLine {
    param([string]$Text)

    return (($Text -split "`n") | Where-Object { $_.StartsWith("command=") } | Select-Object -First 1)
}

$commonArgs = @(
    "-CheckOnly",
    "-Backend", "127.0.0.1:7979",
    "-Rounds", "1"
)

$refreshText = Invoke-LauncherCase -Name "refresh_pool_artifacts_alignment_check_only" -ArgumentList (
    $commonArgs + @("-RefreshPoolArtifacts", "-PoolAlignmentGate", "-RequirePoolRoute")
)
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "check_only=true"
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "touches_remote=false"
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "starts_process=false"
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "sends_prompt=false"
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "pool_manifest_json: target\evolution\pool-manifest.json"
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "pool_status_json: target\evolution\pool-status.json"
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "pool_route_json: target\evolution\pool-route-review.json"
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "--pool-manifest-json target\evolution\pool-manifest.json"
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "--pool-status-json target\evolution\pool-status.json"
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "--pool-route-json target\evolution\pool-route-review.json"
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "--pool-alignment-gate"
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "--refresh-pool-artifacts"
Assert-Contains -Name "refresh_pool_artifacts_alignment_check_only" -Text $refreshText -Pattern "--require-pool-route"
Write-Host "launcher_case_result=refresh_pool_artifacts_alignment_check_only PASS"

$postRunReportText = Invoke-LauncherCase -Name "run_mode_report_json_is_post_run_report" -ArgumentList (
    $commonArgs + @(
        "-Ledger", "target\evolution\launcher-post-run-ledger.jsonl",
        "-ReportJson", "target\evolution\launcher-post-run-report.json",
        "-PostRunReportGate",
        "-PostRunContinuationGate",
        "-RemoteChainStatusJson", "target\remote-gemma-chain\status-with-model-cache.json",
        "-RemoteChainGate",
        "-RequireHelperStageRoles", "summary,router,review,index,test-gate",
        "-RequireLatestHelperStageRoles", "summary,router,review,index,test-gate",
        "-RequirePoolBudgetPolicy",
        "-RequireTestGatePass",
        "-RequireConfiguredValidationRun"
    )
)
$postRunCommand = Get-CommandLine -Text $postRunReportText
Assert-Contains -Name "run_mode_report_json_is_post_run_report" -Text $postRunReportText -Pattern "post_run_report: enabled"
Assert-Contains -Name "run_mode_report_json_is_post_run_report" -Text $postRunReportText -Pattern "post_run_report_gate: enabled"
Assert-Contains -Name "run_mode_report_json_is_post_run_report" -Text $postRunReportText -Pattern "post_run_continuation_gate: enabled"
Assert-Contains -Name "run_mode_report_json_is_post_run_report" -Text $postRunReportText -Pattern "post_run_report_command=cargo run"
Assert-Contains -Name "run_mode_report_json_is_post_run_report" -Text $postRunReportText -Pattern "--report --report-json target\evolution\launcher-post-run-report.json --report-gate"
Assert-Contains -Name "run_mode_report_json_is_post_run_report" -Text $postRunReportText -Pattern "--report-continuation-gate"
Assert-Contains -Name "run_mode_report_json_is_post_run_report" -Text $postRunReportText -Pattern "--remote-chain-status-json target\remote-gemma-chain\status-with-model-cache.json --remote-chain-gate"
Assert-Contains -Name "run_mode_report_json_is_post_run_report" -Text $postRunReportText -Pattern "--require-helper-stage-roles summary,router,review,index,test-gate"
Assert-Contains -Name "run_mode_report_json_is_post_run_report" -Text $postRunReportText -Pattern "--require-latest-helper-stage-roles summary,router,review,index,test-gate"
Assert-Contains -Name "run_mode_report_json_is_post_run_report" -Text $postRunReportText -Pattern "--require-pool-budget-policy"
Assert-Contains -Name "run_mode_report_json_is_post_run_report" -Text $postRunReportText -Pattern "--require-test-gate-pass"
Assert-Contains -Name "run_mode_report_json_is_post_run_report" -Text $postRunReportText -Pattern "--require-configured-validation-run"
Assert-NotContains -Name "run_mode_report_json_is_post_run_report" -Text $postRunCommand -Pattern "--report-json"
Assert-NotContains -Name "run_mode_report_json_is_post_run_report" -Text $postRunCommand -Pattern "--report"
Write-Host "launcher_case_result=run_mode_report_json_is_post_run_report PASS"

$foreverReportText = Invoke-LauncherCase -Name "forever_report_json_refreshes_in_run_mode" -ArgumentList (
    $commonArgs + @(
        "-Forever",
        "-Ledger", "target\evolution\launcher-forever-ledger.jsonl",
        "-ReportJson", "target\evolution\launcher-forever-report.json",
        "-PostRunReportGate",
        "-PostRunContinuationGate"
    )
)
$foreverReportCommand = Get-CommandLine -Text $foreverReportText
Assert-Contains -Name "forever_report_json_refreshes_in_run_mode" -Text $foreverReportText -Pattern "run_report_refresh: enabled"
Assert-Contains -Name "forever_report_json_refreshes_in_run_mode" -Text $foreverReportCommand -Pattern "--forever"
Assert-Contains -Name "forever_report_json_refreshes_in_run_mode" -Text $foreverReportCommand -Pattern "--run-report-json target\evolution\launcher-forever-report.json"
Assert-Contains -Name "forever_report_json_refreshes_in_run_mode" -Text $foreverReportCommand -Pattern "--run-report-gate"
Assert-Contains -Name "forever_report_json_refreshes_in_run_mode" -Text $foreverReportCommand -Pattern "--run-report-continuation-gate"
Assert-NotContains -Name "forever_report_json_refreshes_in_run_mode" -Text $foreverReportCommand -Pattern "--report-json"
Assert-NotContains -Name "forever_report_json_refreshes_in_run_mode" -Text $foreverReportCommand -Pattern "--report "
Write-Host "launcher_case_result=forever_report_json_refreshes_in_run_mode PASS"

$remoteGateText = Invoke-LauncherCase -Name "remote_model_pool_gate_bundle_check_only" -ArgumentList (
    $commonArgs + @("-RemoteModelPoolGate")
)
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "check_only=true"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "touches_remote=false"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "starts_process=false"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "sends_prompt=false"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "remote_model_pool_gate: enabled"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "pool_alignment_gate: enabled"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "pool_budget_fairness_json: target\evolution\model-pool-budget-fairness.json"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "remote_chain_status_json: target\remote-gemma-chain\status-with-model-cache.json"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "model_cache_status_json: target\remote-gemma-chain\model-cache-status.json"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "remote_chain_runtime_probe: enabled"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "--remote-chain-status-json target\remote-gemma-chain\status-with-model-cache.json"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "--remote-chain-gate"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "--pool-capacity-gate"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "--pool-alignment-gate"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "--pool-budget-fairness-gate"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "--pool-stage-route-gate"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "execute_pool_stage_calls: enabled"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "--execute-pool-stage-calls"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "--require-helper-stage-roles summary,review,test-gate"
Assert-Contains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "--require-latest-helper-stage-roles summary,review,test-gate"
Assert-NotContains -Name "remote_model_pool_gate_bundle_check_only" -Text $remoteGateText -Pattern "remote chain status refresh failed"
Write-Host "launcher_case_result=remote_model_pool_gate_bundle_check_only PASS"

Write-Host ""
Write-Host "evolution_loop_launcher_selftest=PASS"
Write-Host "touches_remote=false"
Write-Host "starts_process=false"
Write-Host "sends_prompt=false"
