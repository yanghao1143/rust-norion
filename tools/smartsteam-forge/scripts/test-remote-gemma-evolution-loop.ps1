param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate SmartSteam remote Gemma evolution-loop command assembly without SSH, process launch, or prompt sending."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\test-remote-gemma-evolution-loop.cmd"
    Write-Host ""
    Write-Host "Checks:"
    Write-Host "  - -CheckOnly -NoStartChain emits safe no-start/no-prompt markers"
    Write-Host "  - default helper mode includes helper route, call, and report-gate parameters"
    Write-Host "  - unattended -Forever exposes budget guard arguments and report-gate min rounds"
    Write-Host "  - strict helper completeness gate is opt-in for acceptance/night runs"
    Write-Host "  - -NoPoolWorkers omits helper stage calls and helper report requirements"
    Write-Host "  - run-remote-gemma-evolution-loop.ps1 is directly runnable by Windows PowerShell"
    return
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot not found: $RepoRoot"
}

$loopScript = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\run-remote-gemma-evolution-loop.ps1"
if (-not (Test-Path -LiteralPath $loopScript -PathType Leaf)) {
    throw "run-remote-gemma-evolution-loop.ps1 not found: $loopScript"
}
$unattendedScript = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\run-remote-gemma-unattended.ps1"
if (-not (Test-Path -LiteralPath $unattendedScript -PathType Leaf)) {
    throw "run-remote-gemma-unattended.ps1 not found: $unattendedScript"
}

function Invoke-LoopCase {
    param(
        [string]$Name,
        [string[]]$ArgumentList
    )

    Write-Host ""
    Write-Host "loop_case=$Name"
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $loopScript @ArgumentList 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "loop case '$Name' failed with exit code $exitCode"
    }

    return $text
}

function Invoke-UnattendedCase {
    param(
        [string]$Name,
        [string[]]$ArgumentList
    )

    Write-Host ""
    Write-Host "loop_case=$Name"
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $unattendedScript @ArgumentList 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "loop case '$Name' failed with exit code $exitCode"
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
        throw "loop case '$Name' did not contain expected text: $Pattern"
    }
}

function Assert-NotContains {
    param(
        [string]$Name,
        [string]$Text,
        [string]$Pattern
    )

    if ($Text -match [regex]::Escape($Pattern)) {
        throw "loop case '$Name' unexpectedly contained text: $Pattern"
    }
}

function Get-ReportCommandLine {
    param([string]$Text)

    $lines = $Text -split "`n"
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i].Trim() -eq "report command:") {
            for ($j = $i + 1; $j -lt $lines.Count; $j++) {
                $line = $lines[$j].Trim()
                if (-not [string]::IsNullOrWhiteSpace($line)) {
                    return $line
                }
            }
        }
    }
    return ""
}

$commonArgs = @(
    "-RepoRoot", $RepoRoot,
    "-CheckOnly",
    "-NoStartChain"
)

$defaultText = Invoke-LoopCase -Name "default_helpers_check_only_no_start" -ArgumentList $commonArgs
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "check_only=true"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "evolution_loop_check_only=PASS"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "starts_process=false"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "sends_prompt=false"
Assert-NotContains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "running remote chain CheckOnly preflight"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "model_cache_status_json:"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "target\remote-gemma-chain\model-cache-status.json"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "remote_chain_status_json:"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "target\remote-gemma-chain\status-with-model-cache.json"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "-ModelCacheStatusJson"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--remote-chain-status-json"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--remote-chain-gate"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--require-round-wall-clock-evidence"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--pool-stage-route-task-kinds"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--pool-stage-route-gate"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--execute-pool-stage-calls"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--pool-manifest-json"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--pool-route-json"
Assert-Contains -Name "default_helpers_report_command" -Text (Get-ReportCommandLine -Text $defaultText) -Pattern "--pool-alignment-gate"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--require-latest-helper-stage-roles summary,router,index,test-gate"
Assert-NotContains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--require-latest-helper-stage-roles summary,router,review,index,test-gate"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--require-useful-latest-helper-stage-feedback"
Assert-NotContains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--require-complete-latest-helper-stage-feedback"
Assert-NotContains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "--require-test-gate-validation-run"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "-RequiredPoolWorkerRoles"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "-EnablePoolWorkers"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "-UseMac32GBModelPool"
Assert-Contains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "helper_model_pool_preset: mac32gb"
Assert-NotContains -Name "default_helpers_check_only_no_start" -Text $defaultText -Pattern "gemma-small-Q4.gguf"
Write-Host "loop_case_result=default_helpers_check_only_no_start PASS"

$foreverText = Invoke-LoopCase `
    -Name "forever_budgeted_check_only_no_start" `
    -ArgumentList ($commonArgs + @(
        "-Forever",
        "-IntervalSecs", "2",
        "-BusyWaitSecs", "3",
        "-MaxFailures", "2",
        "-MaxTotalTokens", "2048",
        "-MaxRuntimeSecs", "60",
        "-MaxNoFeedbackRounds", "2"
    ))
Assert-Contains -Name "forever_budgeted_check_only_no_start" -Text $foreverText -Pattern "mode:      forever"
Assert-Contains -Name "forever_budgeted_check_only_no_start" -Text $foreverText -Pattern "--forever"
Assert-Contains -Name "forever_budgeted_check_only_no_start" -Text $foreverText -Pattern "--interval-secs 2"
Assert-Contains -Name "forever_budgeted_check_only_no_start" -Text $foreverText -Pattern "--busy-wait-secs 3"
Assert-Contains -Name "forever_budgeted_check_only_no_start" -Text $foreverText -Pattern "--max-failures 2"
Assert-Contains -Name "forever_budgeted_check_only_no_start" -Text $foreverText -Pattern "--max-total-tokens 2048"
Assert-Contains -Name "forever_budgeted_check_only_no_start" -Text $foreverText -Pattern "--max-runtime-secs 60"
Assert-Contains -Name "forever_budgeted_check_only_no_start" -Text $foreverText -Pattern "--max-no-feedback-rounds 2"
Assert-Contains -Name "forever_budgeted_check_only_no_start" -Text $foreverText -Pattern "--min-report-rounds 1"
Assert-NotContains -Name "forever_budgeted_check_only_no_start" -Text $foreverText -Pattern "--rounds 1"
Write-Host "loop_case_result=forever_budgeted_check_only_no_start PASS"

$unattendedText = Invoke-UnattendedCase `
    -Name "unattended_alias_check_only_no_start" `
    -ArgumentList @(
        "-RepoRoot", $RepoRoot,
        "-CheckOnly",
        "-NoStartChain",
        "-MaxRuntimeSecs", "120",
        "-MaxTotalTokens", "4096",
        "-MaxNoFeedbackRounds", "2",
        "-MaxFailures", "1"
    )
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "SmartSteam unattended remote Gemma evolution"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "budgets:   runtime=120s tokens=4096 no_feedback_rounds=2 failures=1"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "mode:      forever"
Assert-Contains -Name "unattended_alias_report_command" -Text (Get-ReportCommandLine -Text $unattendedText) -Pattern "--pool-alignment-gate"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "--forever"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "--max-runtime-secs 120"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "--max-total-tokens 4096"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "--max-no-feedback-rounds 2"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "--max-failures 1"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "target\remote-gemma-chain\model-cache-status.json"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "target\remote-gemma-chain\status-with-model-cache.json"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "--remote-chain-status-json"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "--remote-chain-gate"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "--require-round-wall-clock-evidence"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "target\remote-gemma-unattended"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "check_only=true"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "starts_process=false"
Assert-Contains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "sends_prompt=false"
Assert-NotContains -Name "unattended_alias_check_only_no_start" -Text $unattendedText -Pattern "running remote chain CheckOnly preflight"
Write-Host "loop_case_result=unattended_alias_check_only_no_start PASS"

$customSmallModel = "/Users/xinghuan/smartsteam-model-box/models/custom-small-Q4.gguf"
$customSmallText = Invoke-LoopCase `
    -Name "custom_small_model_check_only_no_start" `
    -ArgumentList ($commonArgs + @("-RemoteSmallModel", $customSmallModel))
Assert-Contains -Name "custom_small_model_check_only_no_start" -Text $customSmallText -Pattern "evolution_loop_check_only=PASS"
Assert-Contains -Name "custom_small_model_check_only_no_start" -Text $customSmallText -Pattern "helper_model_pool_preset: custom"
Assert-Contains -Name "custom_small_model_check_only_no_start" -Text $customSmallText -Pattern "-RemoteSmallModel $customSmallModel"
Assert-NotContains -Name "custom_small_model_check_only_no_start" -Text $customSmallText -Pattern "-UseMac32GBModelPool"
Write-Host "loop_case_result=custom_small_model_check_only_no_start PASS"

$completeGateText = Invoke-LoopCase -Name "complete_helper_gate_check_only" -ArgumentList ($commonArgs + @("-RequireCompleteHelperFeedbackGate"))
Assert-Contains -Name "complete_helper_gate_check_only" -Text $completeGateText -Pattern "evolution_loop_check_only=PASS"
Assert-Contains -Name "complete_helper_gate_check_only" -Text $completeGateText -Pattern "--require-useful-latest-helper-stage-feedback"
Assert-Contains -Name "complete_helper_gate_check_only" -Text $completeGateText -Pattern "--require-complete-latest-helper-stage-feedback"
Write-Host "loop_case_result=complete_helper_gate_check_only PASS"

$validationGateText = Invoke-LoopCase -Name "test_gate_validation_run_check_only" -ArgumentList ($commonArgs + @("-EnableTestGateValidationRun"))
Assert-Contains -Name "test_gate_validation_run_check_only" -Text $validationGateText -Pattern "test_gate_validation_run: enabled"
Assert-Contains -Name "test_gate_validation_run_check_only" -Text $validationGateText -Pattern "test_gate_validation_bootstrap: auto-if-missing"
Assert-Contains -Name "test_gate_validation_run_check_only" -Text $validationGateText -Pattern "--use-test-gate-validation-command"
Assert-Contains -Name "test_gate_validation_run_check_only" -Text $validationGateText -Pattern "--validation-phase pre"
Assert-Contains -Name "test_gate_validation_run_check_only" -Text $validationGateText -Pattern "--require-test-gate-pass"
Assert-Contains -Name "test_gate_validation_run_check_only" -Text $validationGateText -Pattern "--require-safe-test-gate-validation-command"
Assert-Contains -Name "test_gate_validation_run_check_only" -Text $validationGateText -Pattern "--require-test-gate-validation-run"
Write-Host "loop_case_result=test_gate_validation_run_check_only PASS"

$unattendedValidationText = Invoke-UnattendedCase `
    -Name "unattended_validation_alias_check_only" `
    -ArgumentList @(
        "-RepoRoot", $RepoRoot,
        "-CheckOnly",
        "-NoStartChain",
        "-EnableTestGateValidationRun",
        "-MaxRuntimeSecs", "120",
        "-MaxTotalTokens", "4096"
    )
Assert-Contains -Name "unattended_validation_alias_check_only" -Text $unattendedValidationText -Pattern "test_gate_validation_run: enabled"
Assert-Contains -Name "unattended_validation_alias_check_only" -Text $unattendedValidationText -Pattern "test_gate_validation_bootstrap: auto-if-missing"
Assert-Contains -Name "unattended_validation_alias_check_only" -Text $unattendedValidationText -Pattern "test_gate_validation_bootstrap_budget: counts-against-max-runtime"
Assert-Contains -Name "unattended_validation_alias_check_only" -Text $unattendedValidationText -Pattern "--use-test-gate-validation-command"
Assert-Contains -Name "unattended_validation_alias_check_only" -Text $unattendedValidationText -Pattern "--require-test-gate-validation-run"
Assert-Contains -Name "unattended_validation_alias_check_only" -Text $unattendedValidationText -Pattern "check_only=true"
Write-Host "loop_case_result=unattended_validation_alias_check_only PASS"

$noPoolText = Invoke-LoopCase -Name "no_pool_workers_omits_helper_requirements" -ArgumentList ($commonArgs + @("-NoPoolWorkers"))
Assert-Contains -Name "no_pool_workers_omits_helper_requirements" -Text $noPoolText -Pattern "helpers:   disabled"
Assert-Contains -Name "no_pool_workers_omits_helper_requirements" -Text $noPoolText -Pattern "evolution_loop_check_only=PASS"
Assert-Contains -Name "no_pool_workers_omits_helper_requirements" -Text $noPoolText -Pattern "starts_process=false"
Assert-Contains -Name "no_pool_workers_omits_helper_requirements" -Text $noPoolText -Pattern "sends_prompt=false"
Assert-NotContains -Name "no_pool_workers_omits_helper_requirements" -Text $noPoolText -Pattern "--execute-pool-stage-calls"
Assert-NotContains -Name "no_pool_workers_omits_helper_requirements" -Text $noPoolText -Pattern "--require-latest-helper-stage-roles"
Assert-NotContains -Name "no_pool_workers_omits_helper_requirements" -Text $noPoolText -Pattern "--require-useful-latest-helper-stage-feedback"
Assert-NotContains -Name "no_pool_workers_omits_helper_requirements" -Text $noPoolText -Pattern "--require-complete-latest-helper-stage-feedback"
Write-Host "loop_case_result=no_pool_workers_omits_helper_requirements PASS"

Write-Host ""
Write-Host "remote_gemma_evolution_loop_selftest=PASS"
Write-Host "touches_remote=false"
Write-Host "starts_process=false"
Write-Host "sends_prompt=false"
