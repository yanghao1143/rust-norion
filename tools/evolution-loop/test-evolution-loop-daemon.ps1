param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate SmartSteam evolution-loop daemon launcher without starting background work."
    exit 0
}

$daemon = Join-Path $RepoRoot "tools\evolution-loop\daemon-evolution-loop.ps1"
if (-not (Test-Path -LiteralPath $daemon -PathType Leaf)) {
    throw "daemon-evolution-loop.ps1 not found: $daemon"
}

$supervisor = Join-Path $RepoRoot "tools\evolution-loop\supervise-unattended-evolution.ps1"
if (-not (Test-Path -LiteralPath $supervisor -PathType Leaf)) {
    throw "supervise-unattended-evolution.ps1 not found: $supervisor"
}
$transitionConsumerFixture = Join-Path $RepoRoot "tools\evolution-loop\fixtures\daemon-round-transition-status-v1.consumer.example.json"
if (-not (Test-Path -LiteralPath $transitionConsumerFixture -PathType Leaf)) {
    throw "transition consumer fixture not found: $transitionConsumerFixture"
}
$nextRoundDecisionFixture = Join-Path $RepoRoot "tools\evolution-loop\fixtures\next-round-decision-evidence-v1.report.example.json"
if (-not (Test-Path -LiteralPath $nextRoundDecisionFixture -PathType Leaf)) {
    throw "next-round decision fixture not found: $nextRoundDecisionFixture"
}

function Assert-DaemonRoundTransitionConsumerStatus {
    param(
        [object]$Status,
        [string]$Name,
        [string]$ExpectedKind,
        [bool]$ExpectedRoundInProgress
    )

    if ($null -eq $Status) {
        throw "$Name transition status missing"
    }
    if ($Status.schema -ne "daemon_round_transition_status_v1") {
        throw "$Name transition status schema is not daemon_round_transition_status_v1"
    }
    if ($Status.transition_kind -ne $ExpectedKind) {
        throw "$Name transition kind expected $ExpectedKind but got $($Status.transition_kind)"
    }
    if ($Status.read_only -ne $true -or $Status.starts_process -ne $false -or $Status.sends_prompt -ne $false) {
        throw "$Name transition status broke report-only contract"
    }
    if ($Status.round_in_progress -ne $ExpectedRoundInProgress) {
        throw "$Name transition round_in_progress expected $ExpectedRoundInProgress but got $($Status.round_in_progress)"
    }
}

$transitionConsumerFixtureJson = Get-Content -Raw -LiteralPath $transitionConsumerFixture | ConvertFrom-Json
if ($transitionConsumerFixtureJson.consumer_contract.daemon_json_path -ne "daemon.daemon_round_transition_status") {
    throw "transition consumer fixture daemon json path changed"
}
if ($transitionConsumerFixtureJson.consumer_contract.log_prose_required -ne $false -or $transitionConsumerFixtureJson.consumer_contract.operator_summary_required -ne $false) {
    throw "transition consumer fixture requires prose scraping"
}
$seenTransitionKinds = @{}
foreach ($example in @($transitionConsumerFixtureJson.examples)) {
    $status = $example.daemon.daemon_round_transition_status
    $expectedRoundInProgress = $status.transition_kind -eq "normal_in_progress"
    Assert-DaemonRoundTransitionConsumerStatus -Status $status -Name "daemon-fixture:$($example.name)" -ExpectedKind $status.transition_kind -ExpectedRoundInProgress $expectedRoundInProgress
    $seenTransitionKinds[$status.transition_kind] = $true
}
foreach ($requiredKind in @("normal_in_progress", "round_done_waiting_ledger_commit")) {
    if (-not $seenTransitionKinds.ContainsKey($requiredKind)) {
        throw "transition consumer fixture missing $requiredKind"
    }
}

function Assert-NextRoundDecisionFixture {
    param(
        [string]$Path,
        [object]$TransitionFixture
    )

    $fixture = Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
    if ($fixture.schema -ne "next_round_decision_evidence_surface_v1.report_fixture") {
        throw "next-round decision fixture schema mismatch"
    }
    if ($fixture.report_only -ne $true -or $fixture.read_only -ne $true -or $fixture.side_effects -ne $false) {
        throw "next-round decision fixture broke report-only/read-only contract"
    }
    if ($fixture.starts_process -ne $false -or $fixture.sends_prompt -ne $false) {
        throw "next-round decision fixture introduced process or prompt side effects"
    }
    if ($fixture.changes_daemon_loop_behavior -ne $false -or $fixture.changes_prompt_content -ne $false -or $fixture.changes_report_gate_stop_semantics -ne $false) {
        throw "next-round decision fixture changed daemon loop, prompt, or report gate boundaries"
    }
    if ($fixture.changes_runtime_calls -ne $false -or $fixture.changes_model_pool_behavior -ne $false) {
        throw "next-round decision fixture changed runtime or model pool boundaries"
    }
    if ($fixture.consumes.transition_status_path -ne "live_status_bundle.daemon.daemon_round_transition_status" -or $fixture.consumes.report_gate_path -ne "live_status_bundle.report_gate") {
        throw "next-round decision fixture consumed paths changed"
    }
    if ($fixture.consumes.requires_log_prose -ne $false -or $fixture.consumes.requires_operator_summary -ne $false) {
        throw "next-round decision fixture requires prose scraping"
    }

    $transitionExamples = @{}
    foreach ($example in @($TransitionFixture.examples)) {
        $transitionExamples[$example.name] = $example.daemon.daemon_round_transition_status
    }

    $seenStates = @{}
    $reportGatePassedExamples = 0
    foreach ($example in @($fixture.examples)) {
        $status = $example.live_status_bundle.daemon.daemon_round_transition_status
        $decision = $example.next_round_decision
        $sourceName = [string]$example.input_refs.transition_fixture_example
        if (-not $transitionExamples.ContainsKey($sourceName)) {
            throw "next-round decision example $($example.name) references missing transition fixture example $sourceName"
        }
        if ($status.transition_kind -ne $transitionExamples[$sourceName].transition_kind) {
            throw "next-round decision example $($example.name) drifted from transition fixture kind"
        }
        if ($decision.schema -ne "next_round_decision_evidence_v1" -or $decision.side_effects -ne $false) {
            throw "next-round decision example $($example.name) broke decision schema or side_effects=false"
        }
        if ($decision.read_only -ne $true -or $decision.report_only -ne $true -or $decision.starts_process -eq $true -or $decision.sends_prompt -eq $true) {
            throw "next-round decision example $($example.name) broke read-only/report-only process contract"
        }
        if ($decision.evidence.transition_kind -ne $status.transition_kind -or $decision.evidence.report_gate_passed -ne $example.live_status_bundle.report_gate.passed) {
            throw "next-round decision example $($example.name) evidence does not match inputs"
        }
        if ($example.live_status_bundle.report_gate.passed -eq $true) {
            $reportGatePassedExamples++
        }

        switch ($decision.display_state) {
            "safe-to-wait" {
                if ($status.transition_kind -ne "normal_in_progress" -or $status.round_in_progress -ne $true -or $example.live_status_bundle.report_gate.passed -ne $true) {
                    throw "safe-to-wait example $($example.name) lacks active busy normal_in_progress passed-gate evidence"
                }
                if ($decision.wait_for_current_round -ne $true -or $decision.operator_attention_required -ne $false) {
                    throw "safe-to-wait example $($example.name) has wrong operator decision flags"
                }
                if ($decision.safe_to_wait_current_round_active -ne $true -or $decision.safe_to_continue_after_current_round -ne $false -or $decision.operator_attention_blocked -ne $false) {
                    throw "safe-to-wait example $($example.name) has wrong pure decision booleans"
                }
            }
            "safe-to-continue-after-current-round" {
                if ($status.transition_kind -ne "round_done_waiting_ledger_commit" -or $status.round_in_progress -ne $false -or $example.live_status_bundle.report_gate.passed -ne $true) {
                    throw "safe-to-continue example $($example.name) lacks done-waiting-ledger passed-gate evidence"
                }
                if ($decision.continue_after_current_round -ne $true -or $decision.operator_attention_required -ne $false) {
                    throw "safe-to-continue example $($example.name) has wrong operator decision flags"
                }
                if ($decision.safe_to_wait_current_round_active -ne $false -or $decision.safe_to_continue_after_current_round -ne $true -or $decision.operator_attention_blocked -ne $false) {
                    throw "safe-to-continue example $($example.name) has wrong pure decision booleans"
                }
            }
            "blocked-operator-attention" {
                if ($decision.operator_attention_required -ne $true -or $decision.may_display_unattended_continuation -ne $false) {
                    throw "blocked example $($example.name) is not blocked for operator attention"
                }
                if ($decision.safe_to_wait_current_round_active -ne $false -or $decision.safe_to_continue_after_current_round -ne $false -or $decision.operator_attention_blocked -ne $true) {
                    throw "blocked example $($example.name) has wrong pure decision booleans"
                }
            }
            default {
                throw "unknown next-round decision display state $($decision.display_state)"
            }
        }
        $seenStates[$decision.display_state] = $true
    }
    foreach ($requiredState in @("safe-to-wait", "safe-to-continue-after-current-round", "blocked-operator-attention")) {
        if (-not $seenStates.ContainsKey($requiredState)) {
            throw "next-round decision fixture missing $requiredState"
        }
    }
    if ($reportGatePassedExamples -lt 2) {
        throw "next-round decision fixture does not cover report gate passed safe states"
    }
}

Assert-NextRoundDecisionFixture -Path $nextRoundDecisionFixture -TransitionFixture $transitionConsumerFixtureJson

$supervisorWorkDir = "tools\evolution-loop\target\evolution\supervisor-selftest"
$supervisorWorkDirPath = Join-Path $RepoRoot $supervisorWorkDir
$supervisorPidFile = Join-Path $supervisorWorkDirPath "supervisor.pid"
$supervisorCheckOnlyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $supervisor -CheckOnly -Once -WorkDir $supervisorWorkDir -MaxTokens 32 -MaxTotalTokens 64 -MaxRuntimeSecs 120 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "supervisor check-only failed with exit code $LASTEXITCODE"
}
$supervisorCheckOnlyOutput = ($supervisorCheckOnlyText | ForEach-Object { $_.ToString() }) -join "`n"
if ($supervisorCheckOnlyOutput -notmatch "check_only=true") {
    throw "supervisor check-only missing check_only=true"
}
if ($supervisorCheckOnlyOutput -notmatch "starts_process=false") {
    throw "supervisor check-only missing starts_process=false"
}
if ($supervisorCheckOnlyOutput -notmatch "sends_prompt=false") {
    throw "supervisor check-only missing sends_prompt=false"
}
if ($supervisorCheckOnlyOutput -notmatch "status_command=.*-JsonStatus") {
    throw "supervisor check-only status command missing JsonStatus"
}
if ($supervisorCheckOnlyOutput -notmatch "status_command=.*-StrictUnattendedEvolution") {
    throw "supervisor check-only status command missing StrictUnattendedEvolution"
}
if ($supervisorCheckOnlyOutput -notmatch "status_command=.*-FailOnUnhealthy") {
    throw "supervisor check-only status command missing FailOnUnhealthy"
}
if ($supervisorCheckOnlyOutput -notmatch "start_command=.*-Start") {
    throw "supervisor check-only start command missing Start"
}
if ($supervisorCheckOnlyOutput -notmatch "start_command=.*-StrictUnattendedEvolution") {
    throw "supervisor check-only start command missing StrictUnattendedEvolution"
}
if ($supervisorCheckOnlyOutput -notmatch "start_command=.*-MaxTotalTokens 64") {
    throw "supervisor check-only start command missing token budget"
}
if ($supervisorCheckOnlyOutput -notmatch "start_command=.*-MaxRuntimeSecs 120") {
    throw "supervisor check-only start command missing runtime budget"
}
if ($supervisorCheckOnlyOutput -notmatch "start_command=.*-EnableConfiguredValidationRun") {
    throw "supervisor check-only start command missing configured validation run"
}
if ($supervisorCheckOnlyOutput -notmatch "pid_file=.*tools\\evolution-loop\\target\\evolution\\supervisor-selftest\\supervisor.pid") {
    throw "supervisor check-only missing supervisor pid file path"
}

New-Item -ItemType Directory -Force -Path $supervisorWorkDirPath | Out-Null
Set-Content -Encoding ASCII -LiteralPath $supervisorPidFile -Value ([string]$PID)
$supervisorStatusText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $supervisor -Status -WorkDir $supervisorWorkDir 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "supervisor status failed with exit code $LASTEXITCODE"
}
$supervisorStatusOutput = ($supervisorStatusText | ForEach-Object { $_.ToString() }) -join "`n"
if ($supervisorStatusOutput -notmatch "read_only=true") {
    throw "supervisor status missing read_only=true"
}
if ($supervisorStatusOutput -notmatch "starts_process=false") {
    throw "supervisor status missing starts_process=false"
}
if ($supervisorStatusOutput -notmatch "sends_prompt=false") {
    throw "supervisor status missing sends_prompt=false"
}
if ($supervisorStatusOutput -notmatch "supervisor_running=True") {
    throw "supervisor status did not report running supervisor"
}
if ($supervisorStatusOutput -notmatch "supervisor_pid=$PID") {
    throw "supervisor status did not report expected supervisor pid"
}

$supervisorStopCheckOnlyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $supervisor -Stop -CheckOnly -WorkDir $supervisorWorkDir 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "supervisor stop check-only failed with exit code $LASTEXITCODE"
}
$supervisorStopCheckOnlyOutput = ($supervisorStopCheckOnlyText | ForEach-Object { $_.ToString() }) -join "`n"
if ($supervisorStopCheckOnlyOutput -notmatch "check_only=true") {
    throw "supervisor stop check-only missing check_only=true"
}
if ($supervisorStopCheckOnlyOutput -notmatch "would_stop_pid=$PID") {
    throw "supervisor stop check-only did not report expected pid"
}
Remove-Item -LiteralPath $supervisorPidFile -Force -ErrorAction SilentlyContinue

$workDir = "tools\evolution-loop\target\evolution\daemon-selftest"
$text = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -Start -CheckOnly -WorkDir $workDir -MaxTokens 32 -MaxTotalTokens 64 -MaxRuntimeSecs 120 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon check-only start failed with exit code $LASTEXITCODE"
}
$output = ($text | ForEach-Object { $_.ToString() }) -join "`n"
if ($output -notmatch "check_only=true") {
    throw "daemon check-only missing check_only=true"
}
if ($output -notmatch "starts_process=false") {
    throw "daemon check-only missing starts_process=false"
}
if ($output -notmatch "sends_prompt=false") {
    throw "daemon check-only missing sends_prompt=false"
}
if ($output -notmatch "-Forever") {
    throw "daemon check-only command missing -Forever"
}
if ($output -notmatch "-MaxTotalTokens 64") {
    throw "daemon check-only command missing MaxTotalTokens budget"
}
if ($output -notmatch "-MaxRuntimeSecs 120") {
    throw "daemon check-only command missing MaxRuntimeSecs budget"
}
if ($output -notmatch "-TimeoutSecs 900") {
    throw "daemon check-only command missing default 900s stream TimeoutSecs"
}
if ($output -notmatch "-ValidationTimeoutSecs 300") {
    throw "daemon check-only command missing independent 300s ValidationTimeoutSecs"
}
if ($output -notmatch "-StateConsistencyGate") {
    throw "daemon check-only command missing StateConsistencyGate"
}
if ($output -notmatch "-ExecutePoolStageCalls") {
    throw "daemon check-only command missing ExecutePoolStageCalls"
}
if ($output -notmatch "-PostRunReportGate") {
    throw "daemon check-only command missing PostRunReportGate"
}
if ($output -notmatch "-RemoteChainStatusJson .*target\\remote-gemma-chain\\status-with-model-cache\.json") {
    throw "daemon check-only command missing RemoteChainStatusJson provenance path"
}
if ($output -notmatch "-ModelCacheStatusJson .*target\\remote-gemma-chain\\model-cache-status\.json") {
    throw "daemon check-only command missing ModelCacheStatusJson provenance path"
}
if ($output -notmatch "-PoolBudgetFairnessJson") {
    throw "daemon check-only command missing PoolBudgetFairnessJson"
}
if ($output -notmatch "-RequirePoolBudgetPolicy") {
    throw "daemon check-only command missing RequirePoolBudgetPolicy"
}
if ($output -notmatch "-RequireHelperStageRoles summary,router,review,index,test-gate") {
    throw "daemon check-only command missing RequireHelperStageRoles"
}
if ($output -notmatch "-RequireLatestHelperStageRoles summary,router,review,index,test-gate") {
    throw "daemon check-only command missing RequireLatestHelperStageRoles"
}
if ($output -notmatch "-RequireCompleteLatestHelperStageFeedback") {
    throw "daemon check-only command missing RequireCompleteLatestHelperStageFeedback"
}
if ($output -notmatch "-RequireCleanHelperStageFeedback") {
    throw "daemon check-only command missing RequireCleanHelperStageFeedback"
}
if ($output -notmatch "-RequireTestGatePass") {
    throw "daemon check-only command missing RequireTestGatePass"
}
if ($output -notmatch "-RequireSafeTestGateValidationCommand") {
    throw "daemon check-only command missing RequireSafeTestGateValidationCommand"
}
if ($output -match "-PoolBudgetFairnessGate") {
    throw "daemon check-only should not run PoolBudgetFairnessGate before the first artifact exists"
}
if ($output -match "-UseTestGateValidationCommand") {
    throw "daemon check-only should not enable test-gate validation run unless requested"
}
if ($output -match "-RequireTestGateValidationRun") {
    throw "daemon check-only should not require test-gate validation run unless requested"
}
if ($output -notmatch "-ValidationCommand ""cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\evolution-loop-daemon-check""") {
    throw "daemon check-only command missing default configured ValidationCommand"
}
if ($output -notmatch "-RequireConfiguredValidationRun") {
    throw "daemon check-only command missing default RequireConfiguredValidationRun"
}

$workDirPath = Join-Path $RepoRoot $workDir
$pidFile = Join-Path $workDirPath "evolution-loop.pid"
New-Item -ItemType Directory -Force -Path $workDirPath | Out-Null
Set-Content -Encoding ASCII -LiteralPath $pidFile -Value ([string]$PID)
$runningCheckOnlyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -Start -CheckOnly -WorkDir $workDir -MaxTokens 32 -MaxTotalTokens 64 -MaxRuntimeSecs 120 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon running check-only start failed with exit code $LASTEXITCODE"
}
$runningCheckOnlyOutput = ($runningCheckOnlyText | ForEach-Object { $_.ToString() }) -join "`n"
if ($runningCheckOnlyOutput -notmatch "existing_running=True") {
    throw "daemon running check-only did not report existing_running=True"
}
if ($runningCheckOnlyOutput -notmatch "existing_pid=$PID") {
    throw "daemon running check-only did not report existing pid"
}
if ($runningCheckOnlyOutput -notmatch "command=.*(powershell|pwsh)\.exe") {
    throw "daemon running check-only did not print command preview"
}
if ($runningCheckOnlyOutput -notmatch "-ValidationCommand ""cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\evolution-loop-daemon-check""") {
    throw "daemon running check-only command missing default configured ValidationCommand"
}
if ($runningCheckOnlyOutput -notmatch "-TimeoutSecs 900") {
    throw "daemon running check-only command missing default 900s stream TimeoutSecs"
}
if ($runningCheckOnlyOutput -notmatch "-ValidationTimeoutSecs 300") {
    throw "daemon running check-only command missing independent 300s ValidationTimeoutSecs"
}
if ($runningCheckOnlyOutput -notmatch "-RequireConfiguredValidationRun") {
    throw "daemon running check-only command missing default RequireConfiguredValidationRun"
}
$stopCheckOnlyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -Stop -CheckOnly -WorkDir $workDir 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon stop check-only failed with exit code $LASTEXITCODE"
}
$stopCheckOnlyOutput = ($stopCheckOnlyText | ForEach-Object { $_.ToString() }) -join "`n"
if ($stopCheckOnlyOutput -notmatch "check_only=true") {
    throw "daemon stop check-only missing check_only=true"
}
if ($stopCheckOnlyOutput -notmatch "would_stop_pid=$PID") {
    throw "daemon stop check-only did not report root pid"
}
if ($stopCheckOnlyOutput -notmatch "would_stop_descendant_count=\d+") {
    throw "daemon stop check-only did not report descendant count"
}
if ($stopCheckOnlyOutput -notmatch "would_stop_tree_pids=.*$PID") {
    throw "daemon stop check-only did not report process tree pids"
}
Remove-Item -LiteralPath $pidFile -Force -ErrorAction SilentlyContinue

Set-Content -Encoding ASCII -LiteralPath $pidFile -Value "999999"
$staleStopCheckOnlyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -Stop -CheckOnly -WorkDir $workDir 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon stale stop check-only failed with exit code $LASTEXITCODE"
}
$staleStopCheckOnlyOutput = ($staleStopCheckOnlyText | ForEach-Object { $_.ToString() }) -join "`n"
if ($staleStopCheckOnlyOutput -notmatch "check_only=true") {
    throw "daemon stale stop check-only missing check_only=true"
}
if ($staleStopCheckOnlyOutput -notmatch "stale_pid=999999") {
    throw "daemon stale stop check-only did not report stale pid"
}
if ($staleStopCheckOnlyOutput -notmatch "would_stop_orphan_count=\d+") {
    throw "daemon stale stop check-only did not report orphan count"
}
$staleStopText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -Stop -WorkDir $workDir 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon stale stop failed with exit code $LASTEXITCODE"
}
$staleStopOutput = ($staleStopText | ForEach-Object { $_.ToString() }) -join "`n"
if ($staleStopOutput -notmatch "daemon_stop: not_running|daemon_stop: stopped_orphans") {
    throw "daemon stale stop did not report not_running or stopped_orphans"
}
if (Test-Path -LiteralPath $pidFile -PathType Leaf) {
    throw "daemon stale stop did not remove stale pid file"
}

$disabledConfiguredText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -Start -CheckOnly -WorkDir $workDir -MaxTokens 32 -MaxTotalTokens 64 -MaxRuntimeSecs 120 -DisableConfiguredValidationRun 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon disabled configured validation check-only start failed with exit code $LASTEXITCODE"
}
$disabledConfiguredOutput = ($disabledConfiguredText | ForEach-Object { $_.ToString() }) -join "`n"
if ($disabledConfiguredOutput -match "-ValidationCommand") {
    throw "daemon disabled configured validation check-only should not include ValidationCommand"
}
if ($disabledConfiguredOutput -match "-RequireConfiguredValidationRun") {
    throw "daemon disabled configured validation check-only should not include RequireConfiguredValidationRun"
}

$validationText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -Start -CheckOnly -WorkDir $workDir -MaxTokens 32 -MaxTotalTokens 64 -MaxRuntimeSecs 120 -EnableTestGateValidationRun 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon validation check-only start failed with exit code $LASTEXITCODE"
}
$validationOutput = ($validationText | ForEach-Object { $_.ToString() }) -join "`n"
if ($validationOutput -notmatch "-UseTestGateValidationCommand") {
    throw "daemon validation check-only command missing UseTestGateValidationCommand"
}
if ($validationOutput -notmatch "-RequireSafeTestGateValidationCommand") {
    throw "daemon validation check-only command missing RequireSafeTestGateValidationCommand"
}
if ($validationOutput -notmatch "-RequireTestGateValidationRun") {
    throw "daemon validation check-only command missing RequireTestGateValidationRun"
}
if ($validationOutput -match "-RequireConfiguredValidationRun") {
    throw "daemon test-gate validation check-only should not require configured validation run"
}

$configuredValidationText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -Start -CheckOnly -WorkDir $workDir -MaxTokens 32 -MaxTotalTokens 64 -MaxRuntimeSecs 120 -EnableConfiguredValidationRun 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon configured validation check-only start failed with exit code $LASTEXITCODE"
}
$configuredValidationOutput = ($configuredValidationText | ForEach-Object { $_.ToString() }) -join "`n"
if ($configuredValidationOutput -notmatch "-ValidationCommand ""cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\\evolution-loop-daemon-check""") {
    throw "daemon configured validation check-only command missing fixed ValidationCommand"
}
if ($configuredValidationOutput -notmatch "-RequireConfiguredValidationRun") {
    throw "daemon configured validation check-only command missing RequireConfiguredValidationRun"
}
if ($configuredValidationOutput -match "-RequireTestGateValidationRun") {
    throw "daemon configured validation check-only should not require test-gate validation run"
}

$jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -JsonStatus -WorkDir $workDir -SkipBackend -SkipRemoteChain 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon json status failed with exit code $LASTEXITCODE"
}
$json = ($jsonText | Out-String | ConvertFrom-Json)
if ($json.daemon.read_only -ne $true) {
    throw "daemon status read_only contract failed"
}
if ($json.daemon.starts_process -ne $false) {
    throw "daemon status starts_process contract failed"
}
if ($json.daemon.sends_prompt -ne $false) {
    throw "daemon status sends_prompt contract failed"
}
if ([string]$json.daemon.remote_chain_status_json -notmatch "target\\remote-gemma-chain\\status-with-model-cache\.json") {
    throw "daemon status did not expose remote chain status provenance path"
}
if ([string]$json.daemon.model_cache_status_json -notmatch "target\\remote-gemma-chain\\model-cache-status\.json") {
    throw "daemon status did not expose model cache provenance path"
}

$strictWorkDir = "tools\evolution-loop\target\evolution\daemon-strict-selftest"
$strictWorkDirPath = Join-Path $RepoRoot $strictWorkDir
New-Item -ItemType Directory -Force -Path $strictWorkDirPath | Out-Null
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictWorkDirPath "evolution-loop.pid") -Value ([string]$PID)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictWorkDirPath "evolution-ledger.jsonl") -Value '{"round":7,"case":"daemon-strict-selftest-0007","success":true,"runtime_tokens":34,"elapsed_ms":300,"feedback_applied":4,"self_improve_passed":true,"validation_checked":true,"validation_passed":true,"validation_command_source":"configured","validation_command_safety":"explicit","validation_status_code":0,"validation_elapsed_ms":321,"helper_stage_feedback_by_role":{"summary":["task_kind=summary preview=memory_update: keep"],"router":["task_kind=router preview=route_intent: index"],"review":["task_kind=review preview=change_request: tune"],"index":["task_kind=index preview=clean_gist: keep"],"test-gate":["task_kind=test-gate preview=verdict: pass / validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"]},"helper_stage_contract_by_role":{"summary":{"fields":{"memory_update":"keep"}},"router":{"fields":{"route_intent":"index"}},"review":{"fields":{"change_request":"tune"}},"index":{"fields":{"clean_gist":"keep","tags":"role=index;case=daemon-strict-selftest-0007;round=7;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123","dependency_link":"review.change_request","source_origin":"review.change_request","validation_timestamp":"1781770123","retention":"keep"}},"test-gate":{"fields":{"verdict":"pass","validation_command":"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"}}}}'
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictWorkDirPath "evolution-loop.out.log") -Value @(
    "[round 7] case=daemon-strict-selftest-0007",
    "[round 7] stage generate:start"
)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictWorkDirPath "evolution-loop.err.log") -Value "Running tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --validation-command cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --require-configured-validation-run"
$strictDaemonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -JsonStatus -WorkDir $strictWorkDir -SkipBackend -SkipRemoteChain -StrictUnattendedEvolution -FailOnUnhealthy 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon strict unattended evolution status should pass, got $LASTEXITCODE"
}
$strictDaemon = ($strictDaemonText | Out-String | ConvertFrom-Json)
if ($strictDaemon.daemon.validation_execution_required -ne $true -or $strictDaemon.daemon.validation_execution_ok -ne $true) {
    throw "daemon strict unattended evolution should require and pass validation execution"
}
if ($strictDaemon.loop.strict_unattended_evolution -ne $true) {
    throw "daemon strict unattended evolution did not pass profile flag to status script"
}
if ($strictDaemon.loop.ledger_source -ne "daemon") {
    throw "daemon strict unattended evolution should use daemon ledger in loop status"
}
if ($strictDaemon.loop.daemon.checked -ne $true -or $strictDaemon.loop.daemon.activity_state -ne "active") {
    throw "daemon strict unattended evolution did not let loop status inspect daemon activity"
}
if ($strictDaemon.loop.readiness.ready -ne $true) {
    throw "daemon strict unattended evolution fixture should be ready"
}

$strictMissingWorkDir = "tools\evolution-loop\target\evolution\daemon-strict-missing-selftest"
$strictMissingWorkDirPath = Join-Path $RepoRoot $strictMissingWorkDir
New-Item -ItemType Directory -Force -Path $strictMissingWorkDirPath | Out-Null
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictMissingWorkDirPath "evolution-loop.pid") -Value ([string]$PID)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictMissingWorkDirPath "evolution-ledger.jsonl") -Value '{"round":8,"case":"daemon-strict-missing-selftest-0008","success":true,"runtime_tokens":34,"elapsed_ms":300,"feedback_applied":4,"self_improve_passed":true,"validation_checked":true,"validation_passed":true,"validation_command_source":"configured","validation_command_safety":"explicit","validation_status_code":0,"validation_elapsed_ms":321}'
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictMissingWorkDirPath "evolution-loop.out.log") -Value @(
    "[round 8] case=daemon-strict-missing-selftest-0008",
    "[round 8] stage generate:start"
)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictMissingWorkDirPath "evolution-loop.err.log") -Value "Running tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --validation-command cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --require-configured-validation-run"
$strictMissingDaemonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -JsonStatus -WorkDir $strictMissingWorkDir -SkipBackend -SkipRemoteChain -StrictUnattendedEvolution -FailOnUnhealthy 2>&1
if ($LASTEXITCODE -eq 0) {
    throw "daemon strict unattended evolution missing helper/test-gate evidence should exit nonzero"
}
$strictMissingDaemon = ($strictMissingDaemonText | Out-String | ConvertFrom-Json)
if ($strictMissingDaemon.loop.readiness.ready -ne $false) {
    throw "daemon strict unattended evolution missing helper/test-gate evidence should be not-ready"
}
$strictMissingDaemonFailures = @($strictMissingDaemon.loop.readiness.failures) -join ","
if ($strictMissingDaemonFailures -notmatch "latest_helper_stage_roles_missing" -or $strictMissingDaemonFailures -notmatch "latest_test_gate_not_pass") {
    throw "daemon strict unattended evolution did not expose expected strict failures: $strictMissingDaemonFailures"
}

$daemonWorkDir = Join-Path $RepoRoot $workDir
New-Item -ItemType Directory -Force -Path $daemonWorkDir | Out-Null
$pidFile = Join-Path $daemonWorkDir "evolution-loop.pid"
$ledgerFile = Join-Path $daemonWorkDir "evolution-ledger.jsonl"
$outLog = Join-Path $daemonWorkDir "evolution-loop.out.log"
$errLog = Join-Path $daemonWorkDir "evolution-loop.err.log"
Set-Content -Encoding ASCII -LiteralPath $pidFile -Value "999999"
Set-Content -Encoding ASCII -LiteralPath $outLog -Value @(
    "SmartSteam evolution-loop",
    "stopping: runtime token budget reached (36/32)",
    "post_run_report: generating tools\evolution-loop\target\evolution\daemon-selftest\report.json"
)
Set-Content -Encoding ASCII -LiteralPath $errLog -Value "cargo check placeholder"

$staleText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -JsonStatus -WorkDir $workDir -SkipBackend -SkipRemoteChain 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon stale status failed with exit code $LASTEXITCODE"
}
$stale = ($staleText | Out-String | ConvertFrom-Json)
if ($stale.daemon.stale_pid_file -ne $true) {
    throw "daemon status did not mark stale pid file"
}
if ($stale.daemon.stale_pid -ne 999999) {
    throw "daemon status did not expose stale pid"
}
if ($stale.daemon.last_stop_reason -ne "stopping: runtime token budget reached (36/32)") {
    throw "daemon status did not expose last stop reason"
}
if (@($stale.daemon.stdout_tail).Count -lt 2) {
    throw "daemon status did not expose stdout tail"
}

Set-Content -Encoding ASCII -LiteralPath $pidFile -Value ([string]$PID)
Set-Content -Encoding ASCII -LiteralPath $ledgerFile -Value '{"round":41,"case":"smartsteam-evolution-loop-0041","success":true,"feedback_applied":1,"runtime_tokens":80,"elapsed_ms":120000}'
Set-Content -Encoding ASCII -LiteralPath $errLog -Value '     Running `tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --validation-command "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\evolution-loop-daemon-check" --validation-timeout-secs 300 --validation-phase pre --require-configured-validation-run --require-test-gate-pass`'
Set-Content -Encoding ASCII -LiteralPath $outLog -Value @(
    "[round 41] case=smartsteam-evolution-loop-0041",
    "[round 41] stage ledger_append:done",
    "[round 41] ok runtime_tokens=80 elapsed_ms=120000",
    "[round 42] case=smartsteam-evolution-loop-0042",
    "[round 42] stage generate:start",
    "remote_chain_gate: passed",
    "pool_artifact_refresh: wrote manifest",
    "pool_stage_route_gate: passed",
    "pool_alignment_gate: passed",
    "state_consistency_gate: passed records=41",
    "pool_route_gate: passed",
    "health_gate: model=gemma-4-12b-it-Q8_0.gguf",
    "experience_audit_gate: passed",
    "route_probe: still waiting",
    "status_probe: still waiting",
    "tail_probe: still waiting"
)
$progressText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -JsonStatus -WorkDir $workDir -SkipBackend -SkipRemoteChain 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon progress status failed with exit code $LASTEXITCODE"
}
$progress = ($progressText | Out-String | ConvertFrom-Json)
if ($progress.daemon.log_summary.latest_round -ne 42) {
    throw "daemon progress summary did not keep latest round after visible tail scrolled"
}
if ($progress.daemon.log_summary.latest_completed_round -ne 41) {
    throw "daemon progress summary did not keep latest completed round"
}
if ($progress.daemon.log_summary.latest_round_state -ne "in_progress") {
    throw "daemon progress summary did not mark latest round in_progress"
}
if ($progress.daemon.log_summary.round_in_progress -ne $true) {
    throw "daemon progress summary did not expose round_in_progress=true"
}
if ([string]$progress.daemon.log_summary.latest_round_line_preview -ne "[round 42] stage generate:start") {
    throw "daemon progress summary did not expose latest round line preview"
}
if ($progress.daemon.active_round -ne 42) {
    throw "daemon progress summary did not expose active round"
}
if ($progress.daemon.ledger_latest_round -ne 41) {
    throw "daemon progress summary did not expose ledger latest round"
}
if ($progress.daemon.ledger_lag_rounds -ne 1) {
    throw "daemon progress summary did not expose ledger lag"
}
if ($progress.daemon.log_summary.latest_done_round -ne $null) {
    throw "daemon progress summary should not set latest_done_round for normal in-progress rounds"
}
if ($progress.daemon.stdout_freshness.exists -ne $true -or $progress.daemon.ledger_freshness.exists -ne $true) {
    throw "daemon progress summary did not expose file freshness"
}
if ($progress.daemon.stdout_freshness.age_seconds -lt 0 -or $progress.daemon.ledger_freshness.age_seconds -lt 0) {
    throw "daemon progress summary exposed invalid file freshness age"
}
if ($progress.daemon.activity.state -ne "active" -or $progress.daemon.activity.ok -ne $true) {
    throw "daemon progress summary did not classify fresh in-progress activity"
}
if ($progress.daemon.daemon_round_transition_status.schema -ne "daemon_round_transition_status_v1") {
    throw "daemon progress summary did not expose transition status schema"
}
if ($progress.daemon.daemon_round_transition_status.transition_kind -ne "normal_in_progress") {
    throw "daemon progress summary did not expose normal_in_progress transition kind"
}
if ($progress.daemon.daemon_round_transition_status.activity_reason -ne "round_in_progress_stdout_recent") {
    throw "daemon progress transition did not expose active reason"
}
if ($progress.daemon.daemon_round_transition_status.read_only -ne $true -or $progress.daemon.daemon_round_transition_status.starts_process -ne $false -or $progress.daemon.daemon_round_transition_status.sends_prompt -ne $false) {
    throw "daemon progress transition broke report-only contract"
}
Assert-DaemonRoundTransitionConsumerStatus -Status $progress.daemon.daemon_round_transition_status -Name "daemon-json:normal_in_progress" -ExpectedKind "normal_in_progress" -ExpectedRoundInProgress $true
if ([string]$progress.daemon.operator_summary -notmatch "state=active" -or [string]$progress.daemon.operator_summary -notmatch "active_round=42" -or [string]$progress.daemon.operator_summary -notmatch "lag=1") {
    throw "daemon progress summary did not expose operator summary"
}
if ($progress.daemon.launch_validation.mode -ne "configured") {
    throw "daemon launch validation summary did not classify configured mode"
}
if ($progress.daemon.launch_validation.validation_execution_enforced -ne $true) {
    throw "daemon launch validation summary did not mark validation execution enforced"
}
if ($progress.daemon.launch_validation.validation_command_present -ne $true) {
    throw "daemon launch validation summary did not expose validation command"
}
if ($progress.daemon.launch_validation.require_configured_validation_run -ne $true) {
    throw "daemon launch validation summary did not expose configured validation gate"
}
if ($progress.daemon.launch_validation.require_test_gate_validation_run -ne $false) {
    throw "daemon launch validation summary should not require test-gate validation in configured mode"
}
if ([string]$progress.daemon.launch_validation.next_step -notmatch "configured validation execution") {
    throw "daemon launch validation summary did not expose configured next step"
}

$progressHealthyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -JsonStatus -WorkDir $workDir -SkipBackend -SkipRemoteChain -FailOnUnhealthy 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon FailOnUnhealthy should exit 0 for fresh active activity, got $LASTEXITCODE"
}
$progressHealthy = ($progressHealthyText | Out-String | ConvertFrom-Json)
if ($progressHealthy.daemon.activity.ok -ne $true -or $progressHealthy.daemon.activity.state -ne "active") {
    throw "daemon FailOnUnhealthy did not preserve fresh active JSON status"
}
if ($progressHealthy.loop.read_only -ne $true -or $progressHealthy.loop.starts_process -ne $false -or $progressHealthy.loop.sends_prompt -ne $false) {
    throw "daemon FailOnUnhealthy broke loop status read-only contract"
}

$progressValidationHealthyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -JsonStatus -WorkDir $workDir -SkipBackend -SkipRemoteChain -RequireValidationExecution -FailOnUnhealthy 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon RequireValidationExecution should exit 0 when configured validation is enforced, got $LASTEXITCODE"
}
$progressValidationHealthy = ($progressValidationHealthyText | Out-String | ConvertFrom-Json)
if ($progressValidationHealthy.daemon.validation_execution_required -ne $true -or $progressValidationHealthy.daemon.validation_execution_ok -ne $true) {
    throw "daemon RequireValidationExecution did not expose passing validation execution gate"
}

Set-Content -Encoding ASCII -LiteralPath $errLog -Value '     Running `tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --require-test-gate-pass`'
$missingValidationText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -JsonStatus -WorkDir $workDir -SkipBackend -SkipRemoteChain -RequireValidationExecution -FailOnUnhealthy 2>&1
if ($LASTEXITCODE -eq 0) {
    throw "daemon RequireValidationExecution should exit nonzero when validation execution is not enforced"
}
$missingValidation = ($missingValidationText | Out-String | ConvertFrom-Json)
if ($missingValidation.daemon.validation_execution_ok -ne $false) {
    throw "daemon RequireValidationExecution did not expose failing validation execution gate"
}
if ([string]$missingValidation.daemon.validation_execution_failure -notmatch "does not enforce validation execution") {
    throw "daemon RequireValidationExecution did not expose validation execution failure"
}

(Get-Item -LiteralPath $outLog).LastWriteTime = (Get-Date).AddMinutes(-20)
$staleProgressText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -JsonStatus -WorkDir $workDir -SkipBackend -SkipRemoteChain 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon stale progress status failed with exit code $LASTEXITCODE"
}
$staleProgress = ($staleProgressText | Out-String | ConvertFrom-Json)
if ($staleProgress.daemon.activity.state -ne "stale_in_progress") {
    throw "daemon progress summary did not classify stale in-progress activity"
}
if ($staleProgress.daemon.activity.ok -ne $false) {
    throw "daemon progress summary did not mark stale in-progress activity unhealthy"
}
if ($staleProgress.daemon.daemon_round_transition_status.transition_kind -ne "stale_no_activity") {
    throw "daemon stale progress did not expose stale_no_activity transition kind"
}
if ($staleProgress.daemon.daemon_round_transition_status.activity_reason -ne "round_in_progress_stdout_stale") {
    throw "daemon stale progress transition did not expose stale reason"
}
if ([string]$staleProgress.daemon.operator_summary -notmatch "state=stale_in_progress") {
    throw "daemon stale progress summary did not expose operator summary"
}

$staleProgressFailText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -JsonStatus -WorkDir $workDir -SkipBackend -SkipRemoteChain -FailOnUnhealthy 2>&1
if ($LASTEXITCODE -eq 0) {
    throw "daemon FailOnUnhealthy should exit nonzero for stale in-progress activity"
}
$staleProgressFail = ($staleProgressFailText | Out-String | ConvertFrom-Json)
if ($staleProgressFail.daemon.activity.state -ne "stale_in_progress") {
    throw "daemon FailOnUnhealthy did not print stale activity JSON"
}
if ($staleProgressFail.daemon.read_only -ne $true -or $staleProgressFail.daemon.starts_process -ne $false -or $staleProgressFail.daemon.sends_prompt -ne $false) {
    throw "daemon FailOnUnhealthy broke daemon read-only contract"
}

Set-Content -Encoding ASCII -LiteralPath $ledgerFile -Value '{"round":43,"case":"smartsteam-evolution-loop-0043","success":true,"feedback_applied":1,"runtime_tokens":90,"elapsed_ms":130000}'
Set-Content -Encoding ASCII -LiteralPath $outLog -Value @(
    "[round 43] case=smartsteam-evolution-loop-0043",
    "[round 43] stage generate:start",
    "[round 43] stage generate:done",
    "[round 43] stage ledger_append:start",
    "[round 43] stage ledger_append:done",
    "remote_chain_gate: passed",
    "pool_artifact_refresh: wrote manifest",
    "pool_stage_route_gate: passed",
    "pool_alignment_gate: passed",
    "state_consistency_gate: passed records=43",
    "pool_route_gate: passed",
    "health_gate: model=gemma-4-12b-it-Q8_0.gguf",
    "experience_audit_gate: passed"
)
$completedText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -JsonStatus -WorkDir $workDir -SkipBackend -SkipRemoteChain 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon completed status failed with exit code $LASTEXITCODE"
}
$completed = ($completedText | Out-String | ConvertFrom-Json)
if ($completed.daemon.log_summary.latest_round -ne 43) {
    throw "daemon completed summary did not keep latest round"
}
if ($completed.daemon.log_summary.latest_completed_round -ne 43) {
    throw "daemon completed summary did not treat ledger_append:done as completed"
}
if ($completed.daemon.log_summary.latest_round_state -ne "completed") {
    throw "daemon completed summary did not mark latest round completed"
}
if ($completed.daemon.log_summary.round_in_progress -ne $false) {
    throw "daemon completed summary did not expose round_in_progress=false"
}
if ($completed.daemon.active_round -ne 43) {
    throw "daemon completed summary did not expose active round"
}
if ($completed.daemon.ledger_latest_round -ne 43) {
    throw "daemon completed summary did not expose ledger latest round"
}
if ($completed.daemon.ledger_lag_rounds -ne 0) {
    throw "daemon completed summary did not clear ledger lag"
}
if ($completed.daemon.stdout_freshness.exists -ne $true -or $completed.daemon.ledger_freshness.exists -ne $true) {
    throw "daemon completed summary did not expose file freshness"
}
if ($completed.daemon.stdout_freshness.length_bytes -le 0 -or $completed.daemon.ledger_freshness.length_bytes -le 0) {
    throw "daemon completed summary exposed invalid file freshness size"
}
if ($completed.daemon.activity.state -ne "idle_completed" -or $completed.daemon.activity.ok -ne $true) {
    throw "daemon completed summary did not classify idle completed activity"
}
if ([string]$completed.daemon.operator_summary -notmatch "state=idle_completed" -or [string]$completed.daemon.operator_summary -notmatch "active_round=43" -or [string]$completed.daemon.operator_summary -notmatch "lag=0") {
    throw "daemon completed summary did not expose operator summary"
}

Set-Content -Encoding ASCII -LiteralPath $ledgerFile -Value '{"round":43,"case":"smartsteam-evolution-loop-0043","success":true,"runtime_tokens":80,"elapsed_ms":120000,"feedback_applied":5,"self_improve_passed":true}'
Set-Content -Encoding ASCII -LiteralPath $outLog -Value @(
    "[round 43] case=smartsteam-evolution-loop-0043",
    "[round 43] stage ledger_append:done",
    "[round 43] ok runtime_tokens=80 elapsed_ms=120000",
    "[round 44] case=smartsteam-evolution-loop-0044",
    "[round 44] stage report_gate:done",
    "[round 44] done [DONE]"
)
$doneLagText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemon -JsonStatus -WorkDir $workDir -SkipBackend -SkipRemoteChain 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon done-marker ledger-lag status failed with exit code $LASTEXITCODE"
}
$doneLag = ($doneLagText | Out-String | ConvertFrom-Json)
if ($doneLag.daemon.log_summary.latest_round -ne 44) {
    throw "daemon done-marker ledger-lag summary did not keep latest round"
}
if ($doneLag.daemon.log_summary.latest_done_round -ne 44) {
    throw "daemon done-marker ledger-lag summary did not expose latest_done_round"
}
if ($doneLag.daemon.log_summary.latest_round_state -ne "round_done_waiting_ledger_commit") {
    throw "daemon done-marker ledger-lag summary did not expose waiting-for-ledger state"
}
if ($doneLag.daemon.log_summary.round_in_progress -ne $false) {
    throw "daemon done-marker ledger-lag summary should not expose round_in_progress=true"
}
if ($doneLag.daemon.activity.state -ne "round_done_waiting_ledger_commit" -or $doneLag.daemon.activity.reason -ne "stdout_done_marker_seen_waiting_for_ledger_commit") {
    throw "daemon done-marker ledger-lag activity did not explain ledger commit wait"
}
if ($doneLag.daemon.daemon_round_transition_status.transition_kind -ne "round_done_waiting_ledger_commit") {
    throw "daemon done-marker ledger-lag transition kind was not exposed"
}
if ($doneLag.daemon.daemon_round_transition_status.latest_done_round -ne 44 -or $doneLag.daemon.daemon_round_transition_status.round_in_progress -ne $false) {
    throw "daemon done-marker ledger-lag transition did not expose done round and non-progress state"
}
Assert-DaemonRoundTransitionConsumerStatus -Status $doneLag.daemon.daemon_round_transition_status -Name "daemon-json:round_done_waiting_ledger_commit" -ExpectedKind "round_done_waiting_ledger_commit" -ExpectedRoundInProgress $false
if ($doneLag.daemon.ledger_lag_rounds -ne 1) {
    throw "daemon done-marker ledger-lag summary did not expose ledger lag"
}

Write-Host "evolution_loop_daemon_selftest=PASS"
Write-Host "starts_process=false"
Write-Host "sends_prompt=false"
