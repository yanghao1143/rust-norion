param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate SmartSteam Forge evolution-daemon wrapper without starting processes or sending prompts."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\test-evolution-daemon.cmd"
    return
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot not found: $RepoRoot"
}

$script = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\evolution-daemon.ps1"
if (-not (Test-Path -LiteralPath $script -PathType Leaf)) {
    throw "evolution-daemon.ps1 not found: $script"
}

function Invoke-Case {
    param(
        [string]$Name,
        [string[]]$ArgumentList
    )

    Write-Host ""
    Write-Host "evolution_daemon_case=$Name"
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script @ArgumentList 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "case '$Name' failed with exit code $exitCode"
    }
    return $text
}

function Invoke-FailingCase {
    param(
        [string]$Name,
        [string[]]$ArgumentList
    )

    Write-Host ""
    Write-Host "evolution_daemon_case=$Name"
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script @ArgumentList 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -eq 0) {
        throw "case '$Name' unexpectedly succeeded"
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
        throw "case '$Name' did not contain expected text: $Pattern"
    }
}

function Assert-NotContains {
    param(
        [string]$Name,
        [string]$Text,
        [string]$Pattern
    )

    if ($Text -match [regex]::Escape($Pattern)) {
        throw "case '$Name' contained unexpected text: $Pattern"
    }
}

function Assert-NotExists {
    param(
        [string]$Name,
        [string]$Path
    )

    if (Test-Path -LiteralPath $Path) {
        throw "case '$Name' left unexpected path: $Path"
    }
}

$workDir = Join-Path $RepoRoot "target\evolution\forge-wrapper-selftest"
$pidFile = Join-Path $workDir "evolution-loop.pid"
if (Test-Path -LiteralPath $workDir) {
    Remove-Item -LiteralPath $workDir -Recurse -Force
}

$startText = Invoke-Case -Name "start_check_only" -ArgumentList @(
    "-StartCheck",
    "-WorkDir", "target\evolution\forge-wrapper-selftest",
    "-Backend", "127.0.0.1:7979",
    "-Prompt", "Check SmartSteam Forge daemon wrapper. Return evidence=one item.",
    "-IntervalSecs", "1",
    "-MaxTokens", "64",
    "-MaxTotalTokens", "96",
    "-MaxRuntimeSecs", "0",
    "-MaxFailures", "1",
    "-MaxNoFeedbackRounds", "0",
    "-TimeoutSecs", "300"
)
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "SmartSteam evolution daemon start"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "candidate_preflight read_only=true starts_process=false sends_prompt=false writes_files=false"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "candidate_preflight ready=true accepted_pending=0 implemented_validated=0 implemented_unvalidated=0 implemented_failed=0"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "report_gate_preflight read_only=true starts_process=false sends_prompt=false"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "continuation_state=no_report can_continue_unattended=false blocks_continuation=false block_reason=none"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "check_only=true"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "starts_process=false"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "sends_prompt=false"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-Backend 127.0.0.1:7979"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-Prompt"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-ExecutePoolStageCalls"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-PostRunReportGate"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-PostRunContinuationGate"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-RequireHelperStageRoles summary,router,review,index,test-gate"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-RequireLatestHelperStageRoles summary,router,review,index,test-gate"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-RequirePoolBudgetPolicy"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-RequireTestGatePass"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-IntervalSecs 1"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-MaxTokens 64"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-MaxTotalTokens 96"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-MaxRuntimeSecs 0"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-MaxFailures 1"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-MaxNoFeedbackRounds 0"
Assert-Contains -Name "start_check_only" -Text $startText -Pattern "-TimeoutSecs 300"
Assert-NotExists -Name "start_check_only" -Path $pidFile
Write-Host "evolution_daemon_case_result=start_check_only PASS"

$stopText = Invoke-Case -Name "stop_check_only" -ArgumentList @(
    "-StopCheck",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "stop_check_only" -Text $stopText -Pattern "SmartSteam evolution daemon stop"
Assert-Contains -Name "stop_check_only" -Text $stopText -Pattern "check_only=true"
Assert-Contains -Name "stop_check_only" -Text $stopText -Pattern "starts_process=false"
Assert-Contains -Name "stop_check_only" -Text $stopText -Pattern "sends_prompt=false"
Assert-Contains -Name "stop_check_only" -Text $stopText -Pattern "daemon_stop: not_running"
Assert-NotExists -Name "stop_check_only" -Path $pidFile
Write-Host "evolution_daemon_case_result=stop_check_only PASS"

$jsonStatusText = Invoke-Case -Name "json_status_enriched" -ArgumentList @(
    "-JsonStatus",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"schema":"smartsteam.forge.evolution_status.v1"'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"read_only":true'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"starts_process":false'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"sends_prompt":false'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"evolution_status":'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"report_gate_status":'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"report_exists":false'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"report_gate_preflight":'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"continuation_state":"no_report"'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"repair_hint":"no previous report; first unattended start is allowed"'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"inspect_status_command":".\\tools\\smartsteam-forge\\evolution-daemon.cmd -JsonStatus -WorkDir'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"start_check_command":".\\tools\\smartsteam-forge\\evolution-daemon.cmd -StartCheck -WorkDir'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"candidate_backlog":'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"exists":false'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"daemon_start_gate":'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"candidate_lifecycle_ready":true'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"blocks_unattended_start":false'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"unattended_start_plan":'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"can_start":true'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"block_reason":null'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"report_gate_continuation_state":"no_report"'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"report_gate_can_continue_unattended":false'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"report_gate_blocks_continuation":false'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"continuation_block_reason":null'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"stale_pid_file":false'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"stale_pid":null'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"stale_pid_blocks_start":false'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"stale_pid_cleanup_command":null'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"check_only_command":".\\tools\\smartsteam-forge\\evolution-daemon.cmd -StartCheck -WorkDir'
Assert-Contains -Name "json_status_enriched" -Text $jsonStatusText -Pattern '"start_command":".\\tools\\smartsteam-forge\\evolution-daemon.cmd -Start -WorkDir'
Assert-NotExists -Name "json_status_enriched" -Path $pidFile
Write-Host "evolution_daemon_case_result=json_status_enriched PASS"

Set-Content -Encoding ASCII -LiteralPath $pidFile -Value "2147483647"
$stalePidJsonStatusText = Invoke-Case -Name "json_status_stale_pid_plan" -ArgumentList @(
    "-JsonStatus",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "json_status_stale_pid_plan" -Text $stalePidJsonStatusText -Pattern '"unattended_start_plan":'
Assert-Contains -Name "json_status_stale_pid_plan" -Text $stalePidJsonStatusText -Pattern '"can_start":true'
Assert-Contains -Name "json_status_stale_pid_plan" -Text $stalePidJsonStatusText -Pattern '"stale_pid_file":true'
Assert-Contains -Name "json_status_stale_pid_plan" -Text $stalePidJsonStatusText -Pattern '"stale_pid":2147483647'
Assert-Contains -Name "json_status_stale_pid_plan" -Text $stalePidJsonStatusText -Pattern '"stale_pid_blocks_start":false'
Assert-Contains -Name "json_status_stale_pid_plan" -Text $stalePidJsonStatusText -Pattern '"stale_pid_cleanup_command":".\\tools\\smartsteam-forge\\evolution-daemon.cmd -Stop -WorkDir'
Assert-Contains -Name "json_status_stale_pid_plan" -Text $stalePidJsonStatusText -Pattern '"check_only_command":".\\tools\\smartsteam-forge\\evolution-daemon.cmd -StartCheck -WorkDir'
Assert-Contains -Name "json_status_stale_pid_plan" -Text $stalePidJsonStatusText -Pattern '"start_command":".\\tools\\smartsteam-forge\\evolution-daemon.cmd -Start -WorkDir'
Write-Host "evolution_daemon_case_result=json_status_stale_pid_plan PASS"

$stalePidStopText = Invoke-Case -Name "stop_cleans_stale_pid_file" -ArgumentList @(
    "-Stop",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "stop_cleans_stale_pid_file" -Text $stalePidStopText -Pattern "SmartSteam evolution daemon stop"
Assert-Contains -Name "stop_cleans_stale_pid_file" -Text $stalePidStopText -Pattern "starts_process=false"
Assert-Contains -Name "stop_cleans_stale_pid_file" -Text $stalePidStopText -Pattern "sends_prompt=false"
Assert-Contains -Name "stop_cleans_stale_pid_file" -Text $stalePidStopText -Pattern "daemon_stop: not_running"
Assert-NotExists -Name "stop_cleans_stale_pid_file" -Path $pidFile
Write-Host "evolution_daemon_case_result=stop_cleans_stale_pid_file PASS"

$candidatesText = Invoke-Case -Name "candidates_read_only" -ArgumentList @(
    "-Candidates",
    "-CandidatesLimit", "2",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "candidates_read_only" -Text $candidatesText -Pattern "SmartSteam evolution candidates"
Assert-Contains -Name "candidates_read_only" -Text $candidatesText -Pattern "read_only=true starts_process=false sends_prompt=false writes_files=false"
Assert-Contains -Name "candidates_read_only" -Text $candidatesText -Pattern "source=none count=0 limit=2"
Assert-NotExists -Name "candidates_read_only" -Path $pidFile
Write-Host "evolution_daemon_case_result=candidates_read_only PASS"

$candidatesSaveText = Invoke-Case -Name "candidates_save_empty" -ArgumentList @(
    "-Candidates",
    "-CandidatesSave",
    "-CandidatesBacklog", "target\evolution\forge-wrapper-selftest\candidate-backlog.jsonl",
    "-CandidatesLimit", "2",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "candidates_save_empty" -Text $candidatesSaveText -Pattern "SmartSteam evolution candidates"
Assert-Contains -Name "candidates_save_empty" -Text $candidatesSaveText -Pattern "read_only=true starts_process=false sends_prompt=false writes_files=true"
Assert-Contains -Name "candidates_save_empty" -Text $candidatesSaveText -Pattern "source=none count=0 limit=2"
Assert-Contains -Name "candidates_save_empty" -Text $candidatesSaveText -Pattern "backlog path=target\evolution\forge-wrapper-selftest\candidate-backlog.jsonl existing=0 appended=0 skipped_duplicate=0"
Assert-NotExists -Name "candidates_save_empty" -Path $pidFile
Write-Host "evolution_daemon_case_result=candidates_save_empty PASS"

$candidateBacklog = Join-Path $workDir "candidate-backlog.jsonl"
Set-Content -Encoding ASCII -LiteralPath $candidateBacklog -Value '{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-selftest","status":"new","source":"report.last","round":"1","case":"selftest","model":"model","tokens":"1","elapsed_ms":"1","feedback":"1","self_improve":"true","answer_preview":"candidate"}'
$candidateMarkText = Invoke-Case -Name "candidate_mark_accept" -ArgumentList @(
    "-CandidateMark", "smartsteam-candidate-selftest",
    "-CandidateStatus", "accepted",
    "-CandidateNote", "selftest accepted",
    "-CandidatesBacklog", "target\evolution\forge-wrapper-selftest\candidate-backlog.jsonl",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "candidate_mark_accept" -Text $candidateMarkText -Pattern "SmartSteam evolution candidate mark"
Assert-Contains -Name "candidate_mark_accept" -Text $candidateMarkText -Pattern "starts_process=false sends_prompt=false writes_files=true"
Assert-Contains -Name "candidate_mark_accept" -Text $candidateMarkText -Pattern "candidate_id=smartsteam-candidate-selftest"
Assert-Contains -Name "candidate_mark_accept" -Text $candidateMarkText -Pattern "previous_status=new"
Assert-Contains -Name "candidate_mark_accept" -Text $candidateMarkText -Pattern "status=accepted"
Assert-Contains -Name "candidate_mark_accept" -Text $candidateMarkText -Pattern "appended=true"
Assert-NotExists -Name "candidate_mark_accept" -Path $pidFile
Write-Host "evolution_daemon_case_result=candidate_mark_accept PASS"

$candidateListText = Invoke-Case -Name "candidate_list_accepted" -ArgumentList @(
    "-CandidateList",
    "-CandidateStatus", "accepted",
    "-CandidatesBacklog", "target\evolution\forge-wrapper-selftest\candidate-backlog.jsonl",
    "-CandidatesLimit", "5",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "candidate_list_accepted" -Text $candidateListText -Pattern "SmartSteam evolution candidate backlog"
Assert-Contains -Name "candidate_list_accepted" -Text $candidateListText -Pattern "read_only=true starts_process=false sends_prompt=false writes_files=false"
Assert-Contains -Name "candidate_list_accepted" -Text $candidateListText -Pattern "status_filter=accepted total=1 matched=1 invalid=0 limit=5"
Assert-Contains -Name "candidate_list_accepted" -Text $candidateListText -Pattern "id=smartsteam-candidate-selftest status=accepted"
Assert-Contains -Name "candidate_list_accepted" -Text $candidateListText -Pattern "note=selftest accepted"
Assert-NotExists -Name "candidate_list_accepted" -Path $pidFile
Write-Host "evolution_daemon_case_result=candidate_list_accepted PASS"

$candidateApplyCheckText = Invoke-Case -Name "candidate_apply_check_next" -ArgumentList @(
    "-CandidateApplyCheck", "next",
    "-CandidatesBacklog", "target\evolution\forge-wrapper-selftest\candidate-backlog.jsonl",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "candidate_apply_check_next" -Text $candidateApplyCheckText -Pattern "SmartSteam evolution candidate apply check"
Assert-Contains -Name "candidate_apply_check_next" -Text $candidateApplyCheckText -Pattern "read_only=true starts_process=false sends_prompt=false writes_files=false"
Assert-Contains -Name "candidate_apply_check_next" -Text $candidateApplyCheckText -Pattern "candidate_selector=next"
Assert-Contains -Name "candidate_apply_check_next" -Text $candidateApplyCheckText -Pattern "candidate_id=smartsteam-candidate-selftest status=accepted apply_ready=true status_gate=pass block_reason=none"
Assert-Contains -Name "candidate_apply_check_next" -Text $candidateApplyCheckText -Pattern "suggested_validation_command="
Assert-NotExists -Name "candidate_apply_check_next" -Path $pidFile
Write-Host "evolution_daemon_case_result=candidate_apply_check_next PASS"

$candidateValidateText = Invoke-Case -Name "candidate_validate" -ArgumentList @(
    "-CandidateValidate", "smartsteam-candidate-selftest",
    "-CandidateValidationCommand", "cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml",
    "-CandidateValidationStatus", "0",
    "-CandidateNote", "selftest validation green",
    "-CandidatesBacklog", "target\evolution\forge-wrapper-selftest\candidate-backlog.jsonl",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "candidate_validate" -Text $candidateValidateText -Pattern "SmartSteam evolution candidate validation"
Assert-Contains -Name "candidate_validate" -Text $candidateValidateText -Pattern "starts_process=false sends_prompt=false writes_files=true"
Assert-Contains -Name "candidate_validate" -Text $candidateValidateText -Pattern "candidate_id=smartsteam-candidate-selftest"
Assert-Contains -Name "candidate_validate" -Text $candidateValidateText -Pattern "validation_status_code=0"
Assert-Contains -Name "candidate_validate" -Text $candidateValidateText -Pattern "validation_passed=true"
Assert-Contains -Name "candidate_validate" -Text $candidateValidateText -Pattern "appended=true"
Assert-NotExists -Name "candidate_validate" -Path $pidFile
Write-Host "evolution_daemon_case_result=candidate_validate PASS"

$candidateListValidatedText = Invoke-Case -Name "candidate_list_validation" -ArgumentList @(
    "-CandidateList",
    "-CandidateStatus", "accepted",
    "-CandidatesBacklog", "target\evolution\forge-wrapper-selftest\candidate-backlog.jsonl",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "candidate_list_validation" -Text $candidateListValidatedText -Pattern "validation_passed=true validation_status_code=0"
Assert-Contains -Name "candidate_list_validation" -Text $candidateListValidatedText -Pattern "validation_command=cargo test -q --manifest-path tools/smartsteam-forge/Cargo.toml"
Assert-Contains -Name "candidate_list_validation" -Text $candidateListValidatedText -Pattern "validation_note=selftest validation green"
Assert-NotExists -Name "candidate_list_validation" -Path $pidFile
Write-Host "evolution_daemon_case_result=candidate_list_validation PASS"

$candidateMarkImplementedText = Invoke-Case -Name "candidate_mark_implemented" -ArgumentList @(
    "-CandidateMark", "smartsteam-candidate-selftest",
    "-CandidateStatus", "implemented",
    "-CandidateNote", "selftest implemented",
    "-CandidatesBacklog", "target\evolution\forge-wrapper-selftest\candidate-backlog.jsonl",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "candidate_mark_implemented" -Text $candidateMarkImplementedText -Pattern "SmartSteam evolution candidate mark"
Assert-Contains -Name "candidate_mark_implemented" -Text $candidateMarkImplementedText -Pattern "previous_status=accepted"
Assert-Contains -Name "candidate_mark_implemented" -Text $candidateMarkImplementedText -Pattern "status=implemented"
Assert-NotExists -Name "candidate_mark_implemented" -Path $pidFile
Write-Host "evolution_daemon_case_result=candidate_mark_implemented PASS"

$candidateGateText = Invoke-Case -Name "candidate_gate_ready" -ArgumentList @(
    "-CandidateGate",
    "-CandidatesBacklog", "target\evolution\forge-wrapper-selftest\candidate-backlog.jsonl",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "candidate_gate_ready" -Text $candidateGateText -Pattern "SmartSteam evolution candidate gate"
Assert-Contains -Name "candidate_gate_ready" -Text $candidateGateText -Pattern "read_only=true starts_process=false sends_prompt=false writes_files=false"
Assert-Contains -Name "candidate_gate_ready" -Text $candidateGateText -Pattern "candidate_lifecycle ready=true accepted_pending=0 implemented_validated=1 implemented_unvalidated=0 implemented_failed=0"
Assert-NotExists -Name "candidate_gate_ready" -Path $pidFile
Write-Host "evolution_daemon_case_result=candidate_gate_ready PASS"

$startWithCandidateGateText = Invoke-Case -Name "start_check_candidate_preflight_ready" -ArgumentList @(
    "-StartCheck",
    "-WorkDir", "target\evolution\forge-wrapper-selftest",
    "-Backend", "127.0.0.1:7979",
    "-Prompt", "Check SmartSteam Forge daemon wrapper after candidate validation.",
    "-CandidatesBacklog", "target\evolution\forge-wrapper-selftest\candidate-backlog.jsonl"
)
Assert-Contains -Name "start_check_candidate_preflight_ready" -Text $startWithCandidateGateText -Pattern "candidate_preflight read_only=true starts_process=false sends_prompt=false writes_files=false"
Assert-Contains -Name "start_check_candidate_preflight_ready" -Text $startWithCandidateGateText -Pattern "candidate_preflight ready=true accepted_pending=0 implemented_validated=1 implemented_unvalidated=0 implemented_failed=0"
Assert-Contains -Name "start_check_candidate_preflight_ready" -Text $startWithCandidateGateText -Pattern "report_gate_preflight read_only=true starts_process=false sends_prompt=false"
Assert-Contains -Name "start_check_candidate_preflight_ready" -Text $startWithCandidateGateText -Pattern "continuation_state=no_report can_continue_unattended=false blocks_continuation=false block_reason=none"
Assert-Contains -Name "start_check_candidate_preflight_ready" -Text $startWithCandidateGateText -Pattern "SmartSteam evolution daemon start"
Assert-NotExists -Name "start_check_candidate_preflight_ready" -Path $pidFile
Write-Host "evolution_daemon_case_result=start_check_candidate_preflight_ready PASS"

$dirtyCandidateBacklog = Join-Path $workDir "dirty-candidate-backlog.jsonl"
Set-Content -Encoding ASCII -LiteralPath $dirtyCandidateBacklog -Value '{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"smartsteam-candidate-dirty","status":"accepted","source":"report.last","round":"2","case":"selftest-dirty","model":"model","tokens":"1","elapsed_ms":"1","feedback":"1","self_improve":"true","answer_preview":"dirty candidate should block unattended start"}'
$dirtyStartText = Invoke-FailingCase -Name "start_check_candidate_preflight_blocks_dirty_backlog" -ArgumentList @(
    "-StartCheck",
    "-WorkDir", "target\evolution\forge-wrapper-selftest",
    "-Backend", "127.0.0.1:7979",
    "-Prompt", "This prompt must not reach daemon start when candidate preflight fails.",
    "-CandidatesBacklog", "target\evolution\forge-wrapper-selftest\dirty-candidate-backlog.jsonl"
)
Assert-Contains -Name "start_check_candidate_preflight_blocks_dirty_backlog" -Text $dirtyStartText -Pattern "starts_process=false sends_prompt=false writes_files=false"
Assert-Contains -Name "start_check_candidate_preflight_blocks_dirty_backlog" -Text $dirtyStartText -Pattern "ready=false accepted_pending=1"
Assert-Contains -Name "start_check_candidate_preflight_blocks_dirty_backlog" -Text $dirtyStartText -Pattern "implemented_validated=0 implemented_unvalidated=0 implemented_failed=0"
Assert-Contains -Name "start_check_candidate_preflight_blocks_dirty_backlog" -Text $dirtyStartText -Pattern "candidate lifecycle preflight failed before evolution daemon start"
Assert-NotContains -Name "start_check_candidate_preflight_blocks_dirty_backlog" -Text $dirtyStartText -Pattern "SmartSteam evolution daemon start"
Assert-NotExists -Name "start_check_candidate_preflight_blocks_dirty_backlog" -Path $pidFile
Write-Host "evolution_daemon_case_result=start_check_candidate_preflight_blocks_dirty_backlog PASS"

$reportPath = Join-Path $workDir "report.json"
Set-Content -Encoding ASCII -LiteralPath $reportPath -Value @'
{"report_gate":{"passed":false,"failures":["model_pool_alignment"]},"ledger_gate_report_v1":{"allow_next_round":false,"gate_blocked":true},"test_gate":{"latest_verdict":"fail","latest_validation_command_safety":"unsafe"},"model_pool_alignment":{"alignment_ok":false,"quality_workers":{"manifest":1,"status":1,"max":1},"helper_workers":{"manifest":5,"status":4,"target":5},"missing_manifest_helper_roles":[],"missing_status_helper_roles":["router"],"missing_status_roles":["router"],"unplanned_status_roles":[],"route_blocked_or_failed":[],"route_dependency_failures":["index:dependency_health_failed:required_roles=summary,router missing_roles=router unhealthy_roles=summary:tcp_only status_roles=quality,summary,index"],"missing_inputs":[]}}
'@
$jsonReportStatusText = Invoke-Case -Name "json_status_report_gate_status" -ArgumentList @(
    "-JsonStatus",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"report_gate_status":'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"report_exists":true'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"report_read_ok":true'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"repair_hint":"inspect report_gate_status failures and fix them before unattended continuation"'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"report_gate_passed":false'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"ledger_gate_allow_next_round":false'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"ledger_gate_blocked":true'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"model_pool_alignment_ok":false'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"model_pool_route_dependency_failure_count":1'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"model_pool_route_dependency_failures":["index:dependency_health_failed:required_roles=summary,router missing_roles=router unhealthy_roles=summary:tcp_only status_roles=quality,summary,index"]'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"model_pool_missing_status_roles":["router"]'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"model_pool_missing_status_helper_roles":["router"]'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"test_gate_verdict":"fail"'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"test_gate_validation_command_safety":"unsafe"'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"can_continue_unattended":false'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"can_start":false'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"block_reason":"report_gate_not_ready"'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"report_gate_continuation_state":"blocked"'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"report_gate_can_continue_unattended":false'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"report_gate_blocks_continuation":true'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"continuation_block_reason":"report_gate_not_ready"'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"report_gate_preflight":'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"blocks_continuation":true'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"block_reason":"report_gate_not_ready"'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"inspect_status_command":".\\tools\\smartsteam-forge\\evolution-daemon.cmd -JsonStatus -WorkDir'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"start_check_command":".\\tools\\smartsteam-forge\\evolution-daemon.cmd -StartCheck -WorkDir'
Assert-Contains -Name "json_status_report_gate_status" -Text $jsonReportStatusText -Pattern '"next_step":"blocked: fix report gate before unattended evolution"'
Assert-NotExists -Name "json_status_report_gate_status" -Path $pidFile
Write-Host "evolution_daemon_case_result=json_status_report_gate_status PASS"

$failedReportStartText = Invoke-FailingCase -Name "start_check_report_gate_blocks_continuation" -ArgumentList @(
    "-StartCheck",
    "-WorkDir", "target\evolution\forge-wrapper-selftest",
    "-Backend", "127.0.0.1:7979",
    "-Prompt", "This prompt must not reach daemon start when report gate blocks continuation."
)
Assert-Contains -Name "start_check_report_gate_blocks_continuation" -Text $failedReportStartText -Pattern "report_gate_preflight read_only=true starts_process=false sends_prompt=false"
Assert-Contains -Name "start_check_report_gate_blocks_continuation" -Text $failedReportStartText -Pattern "continuation_state=blocked"
Assert-Contains -Name "start_check_report_gate_blocks_continuation" -Text $failedReportStartText -Pattern "blocks_continuation=true"
Assert-Contains -Name "start_check_report_gate_blocks_continuation" -Text $failedReportStartText -Pattern "block_reason=report_gate_not_ready"
Assert-Contains -Name "start_check_report_gate_blocks_continuation" -Text $failedReportStartText -Pattern "report gate continuation preflight failed"
Assert-NotContains -Name "start_check_report_gate_blocks_continuation" -Text $failedReportStartText -Pattern "SmartSteam evolution daemon start"
Assert-NotExists -Name "start_check_report_gate_blocks_continuation" -Path $pidFile
Write-Host "evolution_daemon_case_result=start_check_report_gate_blocks_continuation PASS"

$watchText = Invoke-Case -Name "watch_once" -ArgumentList @(
    "-Watch",
    "-Count", "1",
    "-IntervalSecs", "1",
    "-WorkDir", "target\evolution\forge-wrapper-selftest"
)
Assert-Contains -Name "watch_once" -Text $watchText -Pattern "evolution_watch iteration=1"
Assert-Contains -Name "watch_once" -Text $watchText -Pattern "SmartSteam evolution daemon"
Assert-Contains -Name "watch_once" -Text $watchText -Pattern "read_only=true starts_process=false sends_prompt=false"
Assert-Contains -Name "watch_once" -Text $watchText -Pattern "report_gate passed=false"
Assert-Contains -Name "watch_once" -Text $watchText -Pattern "unattended_start_plan can_start=false candidate_lifecycle_ready=true block_reason=report_gate_not_ready"
Assert-Contains -Name "watch_once" -Text $watchText -Pattern "report_gate_continuation_state=blocked report_gate_can_continue_unattended=false report_gate_blocks_continuation=true continuation_block_reason=report_gate_not_ready"
Assert-Contains -Name "watch_once" -Text $watchText -Pattern "next_step=blocked: fix report gate before unattended evolution"
Assert-Contains -Name "watch_once" -Text $watchText -Pattern "model_pool_alignment ok=false quality_workers=1/1/1 helper_workers=5/4/5"
Assert-Contains -Name "watch_once" -Text $watchText -Pattern "route_dependency_failures=index:dependency_health_failed:required_roles=summary,router missing_roles=router unhealthy_roles=summary:tcp_only"
Assert-Contains -Name "watch_once" -Text $watchText -Pattern "model_pool_alignment_failures missing_manifest_helper_roles=none missing_status_helper_roles=router missing_status_roles=router"
Assert-NotExists -Name "watch_once" -Path $pidFile
Write-Host "evolution_daemon_case_result=watch_once PASS"

Write-Host ""
Write-Host "smartsteam_forge_evolution_daemon_selftest=PASS"
Write-Host "starts_process=false"
Write-Host "sends_prompt=false"
