param(
    [switch]$CheckOnly,
    [string]$Backend = "127.0.0.1:7979",
    [int]$Rounds = 5,
    [switch]$Forever,
    [int]$IntervalSecs = 5,
    [int]$BusyWaitSecs = 15,
    [int]$MaxFailures = 3,
    [int]$MaxTotalTokens = 0,
    [int]$MaxRuntimeSecs = 0,
    [int]$MaxNoFeedbackRounds = 3,
    [int]$MaxTokens = 4096,
    [int]$SelfImproveLimit = 1,
    [string]$Profile = "coding",
    [double]$FeedbackAmount = 0.5,
    [string]$CasePrefix = "smartsteam-evolution-loop",
    [string]$Ledger = "target\evolution\evolution-ledger.jsonl",
    [string]$PoolManifestJson = "",
    [string]$PoolStatusJson = "",
    [string]$PoolRouteJson = "",
    [string]$PoolBudgetFairnessJson = "",
    [string]$RemoteChainStatusJson = "",
    [string]$ModelCacheStatusJson = "",
    [switch]$RefreshRemoteChainStatus,
    [int]$RemoteChainBackendPort = 0,
    [int]$RemoteChainLabPort = 8789,
    [int]$RemoteChainLocalModelPort = 8686,
    [string]$RemoteChainRequiredPoolWorkerRoles = "",
    [switch]$RemoteChainGate,
    [switch]$RemoteModelPoolGate,
    [switch]$PoolBudgetFairnessGate,
    [switch]$RequirePoolBudgetPolicy,
    [switch]$PoolCapacityGate,
    [switch]$PoolAlignmentGate,
    [switch]$RefreshPoolArtifacts,
    [ValidateSet("summary", "review", "test-gate", "quality", "spare", "auto")]
    [string]$PoolRouteTaskKind = "review",
    [string]$PoolStageRouteTaskKinds = "",
    [switch]$PoolStageRouteGate,
    [switch]$ExecutePoolStageCalls,
    [switch]$RequirePoolRoute,
    [string]$PoolLeaseDir = "",
    [int]$PoolLeaseTtlSecs = 1800,
    [int]$PoolLeaseWaitSecs = 0,
    [int]$PoolLeasePollSecs = 5,
    [ValidateSet("fail", "wait", "skip-low-priority")]
    [string]$PoolLeaseBusyPolicy = "wait",
    [int]$MaxPoolLeaseSkips = 3,
    [string]$Prompt = "",
    [string]$PromptFile = "",
    [switch]$NoReportContext,
    [string]$RustCheckCode = "",
    [string]$RustCheckFile = "",
    [string]$RustCheckEdition = "2021",
    [string]$RustCheckCase = "",
    [string]$ValidationCommand = "",
    [string]$ValidationWorkdir = "",
    [int]$ValidationTimeoutSecs = 300,
    [ValidateSet("pre", "post", "both")]
    [string]$ValidationPhase = "pre",
    [switch]$UseTestGateValidationCommand,
    [int]$TimeoutSecs = 900,
    [switch]$ShowDelta,
    [switch]$BusinessGate,
    [switch]$TraceGate,
    [switch]$NoHealthGate,
    [int]$MinRuntimeContext = 0,
    [switch]$StateConsistencyGate,
    [switch]$ExperienceAuditGate,
    [int]$ExperienceAuditLimit = 25,
    [int]$MaxIndexNoisyRecords = 0,
    [double]$MaxIndexNoisePenalty = 0.0,
    [int]$MaxQuarantineCandidates = 0,
    [int]$MaxRepairableLegacyRecords = 0,
    [int]$MaxLegacyMetadataWithoutCleanGist = 0,
    [switch]$Report,
    [string]$ReportJson = "",
    [switch]$PostRunReportGate,
    [switch]$PostRunContinuationGate,
    [switch]$ReportGate,
    [switch]$ReportContinuationGate,
    [int]$MinReportRounds = 1,
    [double]$MinSuccessRate = -1,
    [int]$MinFeedbackTotal = 1,
    [int]$MinRustChecks = 0,
    [int]$MinRustFeedbackTotal = 0,
    [int]$MaxStreamTruncations = 0,
    [int]$MaxMissingFinal = 0,
    [int]$MaxRuntimeResponseFailures = 0,
    [string]$RequireHelperStageRoles = "",
    [string]$RequireLatestHelperStageRoles = "",
    [switch]$RequireUsefulLatestHelperStageFeedback,
    [switch]$RequireCompleteLatestHelperStageFeedback,
    [switch]$RequireCleanHelperStageFeedback,
    [switch]$RequireFinalJsonPoolStageDispatch,
    [switch]$RequireTestGatePass,
    [switch]$RequireSafeTestGateValidationCommand,
    [switch]$RequireConfiguredValidationRun,
    [switch]$RequireTestGateValidationRun,
    [switch]$StrictLedgerHygiene,
    [switch]$AllowLastFailure
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$Manifest = Join-Path $ScriptDir "Cargo.toml"

Set-Location $RepoRoot

function Get-PortFromEndpoint {
    param(
        [string]$Endpoint,
        [int]$Fallback
    )

    $trimmed = $Endpoint.Trim().TrimEnd("/")
    if ($trimmed -match ":(\d+)$") {
        return [int]$Matches[1]
    }
    return $Fallback
}

function Assert-RemoteChainStatusContract {
    param(
        [string]$Text,
        [string]$Path
    )

    try {
        $status = $Text | ConvertFrom-Json
    } catch {
        throw "remote chain status JSON is invalid ($Path): $($_.Exception.Message)"
    }

    if ($status.read_only -ne $true) {
        throw "remote chain status JSON contract failed ($Path): read_only is not true"
    }
    if ($status.sends_prompt -ne $false) {
        throw "remote chain status JSON contract failed ($Path): sends_prompt is not false"
    }
    if ($status.starts_process -ne $false) {
        throw "remote chain status JSON contract failed ($Path): starts_process is not false"
    }
    if ($status.touches_remote -eq $true) {
        if ($null -eq $status.remote_runtime -or $status.remote_runtime.probed -ne $true) {
            throw "remote chain status JSON contract failed ($Path): touches_remote is true but remote_runtime.probed is not true"
        }
    } elseif ($status.touches_remote -ne $false) {
        throw "remote chain status JSON contract failed ($Path): touches_remote is not a boolean"
    }
}

if ($RemoteModelPoolGate) {
    $RefreshRemoteChainStatus = $true
    $RemoteChainGate = $true
    $RefreshPoolArtifacts = $true
    $RequirePoolRoute = $true
    $PoolCapacityGate = $true
    $PoolAlignmentGate = $true
    $PoolBudgetFairnessGate = $true
    $RequirePoolBudgetPolicy = $true
    $PoolStageRouteGate = $true
    $ExecutePoolStageCalls = $true
    if ($PoolStageRouteTaskKinds.Trim().Length -eq 0) {
        $PoolStageRouteTaskKinds = "summary,review,test-gate"
    }
    if ($RemoteChainRequiredPoolWorkerRoles.Trim().Length -eq 0) {
        $RemoteChainRequiredPoolWorkerRoles = "summary,review,test-gate"
    }
    if ($PoolBudgetFairnessJson.Trim().Length -eq 0) {
        $PoolBudgetFairnessJson = "target\evolution\model-pool-budget-fairness.json"
    }
    if ($PoolLeaseDir.Trim().Length -eq 0) {
        $PoolLeaseDir = "target\evolution\pool-leases"
    }
    if ($RequireHelperStageRoles.Trim().Length -eq 0) {
        $RequireHelperStageRoles = "summary,review,test-gate"
    }
    if ($RequireLatestHelperStageRoles.Trim().Length -eq 0) {
        $RequireLatestHelperStageRoles = "summary,review,test-gate"
    }
}

if ($RefreshPoolArtifacts) {
    if ($PoolManifestJson.Trim().Length -eq 0) {
        $PoolManifestJson = "target\evolution\pool-manifest.json"
    }
    if ($PoolStatusJson.Trim().Length -eq 0) {
        $PoolStatusJson = "target\evolution\pool-status.json"
    }
    if ($PoolRouteJson.Trim().Length -eq 0) {
        $PoolRouteJson = "target\evolution\pool-route-$PoolRouteTaskKind.json"
    }
}

if ($RefreshRemoteChainStatus -and $RemoteChainStatusJson.Trim().Length -eq 0) {
    $RemoteChainStatusJson = "target\remote-gemma-chain\status-with-model-cache.json"
}

if ($RefreshRemoteChainStatus -and $ModelCacheStatusJson.Trim().Length -eq 0) {
    $ModelCacheStatusJson = "target\remote-gemma-chain\model-cache-status.json"
}

if ($RefreshRemoteChainStatus -and -not $CheckOnly) {
    $statusCmd = Join-Path (Join-Path $RepoRoot "tools\smartsteam-forge") "status-remote-gemma-chain.cmd"
    if (-not (Test-Path $statusCmd)) {
        throw "remote chain status command not found: $statusCmd"
    }
    $backendPort = if ($RemoteChainBackendPort -gt 0) {
        $RemoteChainBackendPort
    } else {
        Get-PortFromEndpoint -Endpoint $Backend -Fallback 7979
    }
    $statusArgs = @(
        "-BackendPort", $backendPort,
        "-LabPort", $RemoteChainLabPort,
        "-LocalModelPort", $RemoteChainLocalModelPort,
        "-JsonStatus",
        "-ProbeRemoteRuntime"
    )
    if ($RemoteChainRequiredPoolWorkerRoles.Trim().Length -gt 0) {
        $statusArgs += @("-RequiredPoolWorkerRoles", $RemoteChainRequiredPoolWorkerRoles)
    }
    if ($ModelCacheStatusJson.Trim().Length -gt 0) {
        $statusArgs += @("-ModelCacheStatusJson", $ModelCacheStatusJson)
    }
    $parent = Split-Path -Parent $RemoteChainStatusJson
    if ($parent -and $parent.Trim().Length -gt 0) {
        New-Item -ItemType Directory -Force $parent | Out-Null
    }
    $statusOutput = & $statusCmd @statusArgs
    if ($LASTEXITCODE -ne 0) {
        throw "remote chain status refresh failed with exit code $LASTEXITCODE"
    }
    $statusText = (($statusOutput | Out-String).Trim())
    Assert-RemoteChainStatusContract -Text $statusText -Path $RemoteChainStatusJson
    Set-Content -Encoding ASCII -Path $RemoteChainStatusJson -Value $statusText
}

$LoopArgs = @(
    "--backend", $Backend,
    "--interval-secs", $IntervalSecs,
    "--busy-wait-secs", $BusyWaitSecs,
    "--max-failures", $MaxFailures,
    "--max-total-tokens", $MaxTotalTokens,
    "--max-runtime-secs", $MaxRuntimeSecs,
    "--max-no-feedback-rounds", $MaxNoFeedbackRounds,
    "--max-tokens", $MaxTokens,
    "--self-improve-limit", $SelfImproveLimit,
    "--profile", $Profile,
    "--feedback-amount", $FeedbackAmount,
    "--case-prefix", $CasePrefix,
    "--ledger", $Ledger,
    "--timeout-secs", $TimeoutSecs
)
$ReportMode = $Report -or $ReportGate -or $ReportContinuationGate
$RunReportRefresh = (-not $ReportMode) -and $Forever -and $ReportJson.Trim().Length -gt 0

if ($Forever) {
    $LoopArgs += "--forever"
} else {
    $LoopArgs += @("--rounds", $Rounds)
}
if ($Prompt.Trim().Length -gt 0) {
    $LoopArgs += @("--prompt", $Prompt)
}
if ($PoolManifestJson.Trim().Length -gt 0) {
    $LoopArgs += @("--pool-manifest-json", $PoolManifestJson)
}
if ($PoolStatusJson.Trim().Length -gt 0) {
    $LoopArgs += @("--pool-status-json", $PoolStatusJson)
}
if ($PoolRouteJson.Trim().Length -gt 0) {
    $LoopArgs += @("--pool-route-json", $PoolRouteJson)
}
if ($PoolBudgetFairnessJson.Trim().Length -gt 0) {
    $LoopArgs += @("--pool-budget-fairness-json", $PoolBudgetFairnessJson)
}
if ($RemoteChainStatusJson.Trim().Length -gt 0) {
    $LoopArgs += @("--remote-chain-status-json", $RemoteChainStatusJson)
}
if ($RemoteChainGate) {
    $LoopArgs += "--remote-chain-gate"
}
if ($PoolBudgetFairnessGate) {
    $LoopArgs += "--pool-budget-fairness-gate"
}
if ($RequirePoolBudgetPolicy) {
    $LoopArgs += "--require-pool-budget-policy"
}
if ($PoolCapacityGate) {
    $LoopArgs += "--pool-capacity-gate"
}
if ($PoolAlignmentGate) {
    $LoopArgs += "--pool-alignment-gate"
}
if ($RefreshPoolArtifacts) {
    $LoopArgs += @("--refresh-pool-artifacts", "--pool-route-task-kind", $PoolRouteTaskKind)
}
if ($PoolStageRouteTaskKinds.Trim().Length -gt 0) {
    $LoopArgs += @("--pool-stage-route-task-kinds", $PoolStageRouteTaskKinds)
}
if ($PoolStageRouteGate) {
    $LoopArgs += "--pool-stage-route-gate"
}
if ($ExecutePoolStageCalls) {
    $LoopArgs += "--execute-pool-stage-calls"
}
if ($RequirePoolRoute) {
    $LoopArgs += "--require-pool-route"
}
if ($PoolLeaseDir.Trim().Length -gt 0) {
    $LoopArgs += @(
        "--pool-lease-dir", $PoolLeaseDir,
        "--pool-lease-ttl-secs", $PoolLeaseTtlSecs,
        "--pool-lease-wait-secs", $PoolLeaseWaitSecs,
        "--pool-lease-poll-secs", $PoolLeasePollSecs,
        "--pool-lease-busy-policy", $PoolLeaseBusyPolicy,
        "--max-pool-lease-skips", $MaxPoolLeaseSkips
    )
}
if ($PromptFile.Trim().Length -gt 0) {
    $LoopArgs += @("--prompt-file", $PromptFile)
}
if ($NoReportContext) {
    $LoopArgs += "--no-report-context"
}
if ($RustCheckCode.Trim().Length -gt 0) {
    $LoopArgs += @("--rust-check-code", $RustCheckCode)
}
if ($RustCheckFile.Trim().Length -gt 0) {
    $LoopArgs += @("--rust-check-file", $RustCheckFile)
}
if ($RustCheckEdition.Trim().Length -gt 0) {
    $LoopArgs += @("--rust-check-edition", $RustCheckEdition)
}
if ($RustCheckCase.Trim().Length -gt 0) {
    $LoopArgs += @("--rust-check-case", $RustCheckCase)
}
if ($ValidationCommand.Trim().Length -gt 0) {
    $LoopArgs += @(
        "--validation-command", $ValidationCommand,
        "--validation-timeout-secs", $ValidationTimeoutSecs,
        "--validation-phase", $ValidationPhase
    )
}
if ($UseTestGateValidationCommand) {
    $LoopArgs += @(
        "--use-test-gate-validation-command",
        "--validation-timeout-secs", $ValidationTimeoutSecs,
        "--validation-phase", $ValidationPhase
    )
}
if ($ValidationWorkdir.Trim().Length -gt 0) {
    $LoopArgs += @("--validation-workdir", $ValidationWorkdir)
}
if ($ShowDelta) {
    $LoopArgs += "--show-delta"
}
if ($BusinessGate) {
    $LoopArgs += "--business-gate"
}
if ($TraceGate) {
    $LoopArgs += "--trace-gate"
}
if ($NoHealthGate) {
    $LoopArgs += "--no-health-gate"
}
if ($MinRuntimeContext -gt 0) {
    $LoopArgs += @("--min-runtime-context", $MinRuntimeContext)
}
if ($StateConsistencyGate) {
    $LoopArgs += "--state-consistency-gate"
}
if ($ExperienceAuditGate) {
    $LoopArgs += @(
        "--experience-audit-gate",
        "--experience-audit-limit", $ExperienceAuditLimit,
        "--max-index-noisy-records", $MaxIndexNoisyRecords,
        "--max-index-noise-penalty", $MaxIndexNoisePenalty,
        "--max-quarantine-candidates", $MaxQuarantineCandidates,
        "--max-repairable-legacy-records", $MaxRepairableLegacyRecords,
        "--max-legacy-metadata-without-clean-gist", $MaxLegacyMetadataWithoutCleanGist
    )
}
if ($ReportMode) {
    if ($Report) {
        $LoopArgs += "--report"
    }
    if ($ReportJson.Trim().Length -gt 0) {
        $LoopArgs += @("--report-json", $ReportJson)
    }
    if ($ReportGate) {
        $LoopArgs += @("--report-gate", "--min-report-rounds", $MinReportRounds, "--min-feedback-total", $MinFeedbackTotal)
    }
    if ($ReportContinuationGate) {
        $LoopArgs += "--report-continuation-gate"
    }
    if ($MinRustChecks -gt 0) {
        $LoopArgs += @("--min-rust-checks", $MinRustChecks)
    }
    if ($MinRustFeedbackTotal -gt 0) {
        $LoopArgs += @("--min-rust-feedback-total", $MinRustFeedbackTotal)
    }
    if ($MaxStreamTruncations -ge 0) {
        $LoopArgs += @("--max-stream-truncations", $MaxStreamTruncations)
    }
    if ($MaxMissingFinal -ge 0) {
        $LoopArgs += @("--max-missing-final", $MaxMissingFinal)
    }
    if ($MaxRuntimeResponseFailures -ge 0) {
        $LoopArgs += @("--max-runtime-response-failures", $MaxRuntimeResponseFailures)
    }
    if ($RequireHelperStageRoles.Trim().Length -gt 0) {
        $LoopArgs += @("--require-helper-stage-roles", $RequireHelperStageRoles)
    }
    if ($RequireLatestHelperStageRoles.Trim().Length -gt 0) {
        $LoopArgs += @("--require-latest-helper-stage-roles", $RequireLatestHelperStageRoles)
    }
    if ($RequireUsefulLatestHelperStageFeedback) {
        $LoopArgs += "--require-useful-latest-helper-stage-feedback"
    }
    if ($RequireCompleteLatestHelperStageFeedback) {
        $LoopArgs += "--require-complete-latest-helper-stage-feedback"
    }
    if ($RequireCleanHelperStageFeedback) {
        $LoopArgs += "--require-clean-helper-stage-feedback"
    }
    if ($RequireFinalJsonPoolStageDispatch) {
        $LoopArgs += "--require-final-json-pool-stage-dispatch"
    }
    if ($RequireTestGatePass) {
        $LoopArgs += "--require-test-gate-pass"
    }
    if ($RequireSafeTestGateValidationCommand) {
        $LoopArgs += "--require-safe-test-gate-validation-command"
    }
    if ($RequireConfiguredValidationRun) {
        $LoopArgs += "--require-configured-validation-run"
    }
    if ($RequireTestGateValidationRun) {
        $LoopArgs += "--require-test-gate-validation-run"
    }
    if ($StrictLedgerHygiene) {
        $LoopArgs += "--strict-ledger-hygiene"
    }
    if ($MinSuccessRate -ge 0) {
        $LoopArgs += @("--min-success-rate", $MinSuccessRate)
    }
}
if (-not $ReportMode) {
    if ($MinRustChecks -gt 0) {
        $LoopArgs += @("--min-rust-checks", $MinRustChecks)
    }
    if ($MinRustFeedbackTotal -gt 0) {
        $LoopArgs += @("--min-rust-feedback-total", $MinRustFeedbackTotal)
    }
    if ($MaxStreamTruncations -ge 0) {
        $LoopArgs += @("--max-stream-truncations", $MaxStreamTruncations)
    }
    if ($MaxMissingFinal -ge 0) {
        $LoopArgs += @("--max-missing-final", $MaxMissingFinal)
    }
    if ($MaxRuntimeResponseFailures -ge 0) {
        $LoopArgs += @("--max-runtime-response-failures", $MaxRuntimeResponseFailures)
    }
    if ($RequireHelperStageRoles.Trim().Length -gt 0) {
        $LoopArgs += @("--require-helper-stage-roles", $RequireHelperStageRoles)
    }
    if ($RequireLatestHelperStageRoles.Trim().Length -gt 0) {
        $LoopArgs += @("--require-latest-helper-stage-roles", $RequireLatestHelperStageRoles)
    }
    if ($RequireUsefulLatestHelperStageFeedback) {
        $LoopArgs += "--require-useful-latest-helper-stage-feedback"
    }
    if ($RequireCompleteLatestHelperStageFeedback) {
        $LoopArgs += "--require-complete-latest-helper-stage-feedback"
    }
    if ($RequireCleanHelperStageFeedback) {
        $LoopArgs += "--require-clean-helper-stage-feedback"
    }
    if ($RequireFinalJsonPoolStageDispatch) {
        $LoopArgs += "--require-final-json-pool-stage-dispatch"
    }
    if ($RequireTestGatePass) {
        $LoopArgs += "--require-test-gate-pass"
    }
    if ($RequireSafeTestGateValidationCommand) {
        $LoopArgs += "--require-safe-test-gate-validation-command"
    }
    if ($RequireConfiguredValidationRun) {
        $LoopArgs += "--require-configured-validation-run"
    }
    if ($RequireTestGateValidationRun) {
        $LoopArgs += "--require-test-gate-validation-run"
    }
    if ($StrictLedgerHygiene) {
        $LoopArgs += "--strict-ledger-hygiene"
    }
    if ($MinSuccessRate -ge 0) {
        $LoopArgs += @("--min-success-rate", $MinSuccessRate)
    }
}
if ($AllowLastFailure) {
    $LoopArgs += "--allow-last-failure"
}

$PostRunReportArgs = @()
if (-not $ReportMode -and $ReportJson.Trim().Length -gt 0) {
    $PostRunReportArgs = @(
        "--backend", $Backend,
        "--ledger", $Ledger,
        "--report",
        "--report-json", $ReportJson
    )
    if ($PostRunReportGate) {
        $PostRunReportArgs += @("--report-gate", "--min-report-rounds", $MinReportRounds, "--min-feedback-total", $MinFeedbackTotal)
    }
    if ($PostRunContinuationGate) {
        $PostRunReportArgs += "--report-continuation-gate"
    }
    if ($PoolManifestJson.Trim().Length -gt 0) {
        $PostRunReportArgs += @("--pool-manifest-json", $PoolManifestJson)
    }
    if ($PoolStatusJson.Trim().Length -gt 0) {
        $PostRunReportArgs += @("--pool-status-json", $PoolStatusJson)
    }
    if ($PoolRouteJson.Trim().Length -gt 0) {
        $PostRunReportArgs += @("--pool-route-json", $PoolRouteJson)
    }
    if ($PoolBudgetFairnessJson.Trim().Length -gt 0) {
        $PostRunReportArgs += @("--pool-budget-fairness-json", $PoolBudgetFairnessJson)
    }
    if ($RemoteChainStatusJson.Trim().Length -gt 0) {
        $PostRunReportArgs += @("--remote-chain-status-json", $RemoteChainStatusJson)
    }
    if ($PostRunReportGate -and $RemoteChainGate) {
        $PostRunReportArgs += "--remote-chain-gate"
    }
    if ($PostRunReportGate) {
        if ($MinRustChecks -gt 0) {
            $PostRunReportArgs += @("--min-rust-checks", $MinRustChecks)
        }
        if ($MinRustFeedbackTotal -gt 0) {
            $PostRunReportArgs += @("--min-rust-feedback-total", $MinRustFeedbackTotal)
        }
        if ($MaxStreamTruncations -ge 0) {
            $PostRunReportArgs += @("--max-stream-truncations", $MaxStreamTruncations)
        }
        if ($MaxMissingFinal -ge 0) {
            $PostRunReportArgs += @("--max-missing-final", $MaxMissingFinal)
        }
        if ($MaxRuntimeResponseFailures -ge 0) {
            $PostRunReportArgs += @("--max-runtime-response-failures", $MaxRuntimeResponseFailures)
        }
        if ($RequireHelperStageRoles.Trim().Length -gt 0) {
            $PostRunReportArgs += @("--require-helper-stage-roles", $RequireHelperStageRoles)
        }
        if ($RequireLatestHelperStageRoles.Trim().Length -gt 0) {
            $PostRunReportArgs += @("--require-latest-helper-stage-roles", $RequireLatestHelperStageRoles)
        }
        if ($RequireUsefulLatestHelperStageFeedback) {
            $PostRunReportArgs += "--require-useful-latest-helper-stage-feedback"
        }
        if ($RequireCompleteLatestHelperStageFeedback) {
            $PostRunReportArgs += "--require-complete-latest-helper-stage-feedback"
        }
        if ($RequireCleanHelperStageFeedback) {
            $PostRunReportArgs += "--require-clean-helper-stage-feedback"
        }
        if ($RequireFinalJsonPoolStageDispatch) {
            $PostRunReportArgs += "--require-final-json-pool-stage-dispatch"
        }
        if ($RequirePoolBudgetPolicy) {
            $PostRunReportArgs += "--require-pool-budget-policy"
        }
        if ($RequireTestGatePass) {
            $PostRunReportArgs += "--require-test-gate-pass"
        }
        if ($RequireSafeTestGateValidationCommand) {
            $PostRunReportArgs += "--require-safe-test-gate-validation-command"
        }
        if ($RequireConfiguredValidationRun) {
            $PostRunReportArgs += "--require-configured-validation-run"
        }
        if ($RequireTestGateValidationRun) {
            $PostRunReportArgs += "--require-test-gate-validation-run"
        }
        if ($StrictLedgerHygiene) {
            $PostRunReportArgs += "--strict-ledger-hygiene"
        }
        if ($MinSuccessRate -ge 0) {
            $PostRunReportArgs += @("--min-success-rate", $MinSuccessRate)
        }
        if ($AllowLastFailure) {
            $PostRunReportArgs += "--allow-last-failure"
        }
    }
}
if ($RunReportRefresh) {
    $LoopArgs += @("--run-report-json", $ReportJson)
    if ($PostRunReportGate) {
        $LoopArgs += "--run-report-gate"
    }
    if ($PostRunContinuationGate) {
        $LoopArgs += "--run-report-continuation-gate"
    }
}

Write-Host "SmartSteam evolution-loop"
Write-Host "repo: $RepoRoot"
Write-Host "backend: $Backend"
Write-Host "ledger: $Ledger"
if ($PoolManifestJson.Trim().Length -gt 0) {
    Write-Host "pool_manifest_json: $PoolManifestJson"
}
if ($PoolStatusJson.Trim().Length -gt 0) {
    Write-Host "pool_status_json: $PoolStatusJson"
}
if ($PoolRouteJson.Trim().Length -gt 0) {
    Write-Host "pool_route_json: $PoolRouteJson"
}
if ($PoolBudgetFairnessJson.Trim().Length -gt 0) {
    Write-Host "pool_budget_fairness_json: $PoolBudgetFairnessJson"
}
if ($RemoteChainStatusJson.Trim().Length -gt 0) {
    Write-Host "remote_chain_status_json: $RemoteChainStatusJson"
}
if ($ModelCacheStatusJson.Trim().Length -gt 0) {
    Write-Host "model_cache_status_json: $ModelCacheStatusJson"
}
if ($RefreshRemoteChainStatus) {
    Write-Host "refresh_remote_chain_status: enabled"
    Write-Host "remote_chain_runtime_probe: enabled"
    Write-Host "remote_chain_backend_port: $(if ($RemoteChainBackendPort -gt 0) { $RemoteChainBackendPort } else { Get-PortFromEndpoint -Endpoint $Backend -Fallback 7979 })"
    Write-Host "remote_chain_lab_port: $RemoteChainLabPort"
    Write-Host "remote_chain_local_model_port: $RemoteChainLocalModelPort"
    if ($RemoteChainRequiredPoolWorkerRoles.Trim().Length -gt 0) {
        Write-Host "remote_chain_required_pool_worker_roles: $RemoteChainRequiredPoolWorkerRoles"
    }
}
if ($RemoteChainGate) {
    Write-Host "remote_chain_gate: enabled"
}
if ($RemoteModelPoolGate) {
    Write-Host "remote_model_pool_gate: enabled"
    Write-Host "remote_model_pool_gate_sets: refresh_remote_chain_status,remote_chain_gate,refresh_pool_artifacts,require_pool_route,pool_capacity_gate,pool_alignment_gate,pool_stage_route_gate,execute_pool_stage_calls,pool_budget_fairness_gate,require_pool_budget_policy,pool_lease,require_helper_stage_roles,require_latest_helper_stage_roles"
}
if ($PoolBudgetFairnessGate) {
    Write-Host "pool_budget_fairness_gate: enabled"
}
if ($RequirePoolBudgetPolicy) {
    Write-Host "require_pool_budget_policy: enabled"
}
if ($PoolCapacityGate) {
    Write-Host "pool_capacity_gate: enabled"
}
if ($PoolAlignmentGate) {
    Write-Host "pool_alignment_gate: enabled"
}
if ($RefreshPoolArtifacts) {
    Write-Host "refresh_pool_artifacts: enabled"
    Write-Host "pool_route_task_kind: $PoolRouteTaskKind"
    if ($PoolStageRouteTaskKinds.Trim().Length -gt 0) {
        Write-Host "pool_stage_route_task_kinds: $PoolStageRouteTaskKinds"
    }
}
if ($PoolStageRouteGate) {
    Write-Host "pool_stage_route_gate: enabled"
}
if ($ExecutePoolStageCalls) {
    Write-Host "execute_pool_stage_calls: enabled"
}
if ($RequirePoolRoute) {
    Write-Host "pool_route_gate: required"
}
if ($PoolLeaseDir.Trim().Length -gt 0) {
    Write-Host "pool_lease_dir: $PoolLeaseDir"
    Write-Host "pool_lease_ttl_secs: $PoolLeaseTtlSecs"
    Write-Host "pool_lease_wait_secs: $PoolLeaseWaitSecs"
    Write-Host "pool_lease_poll_secs: $PoolLeasePollSecs"
    Write-Host "pool_lease_busy_policy: $PoolLeaseBusyPolicy"
    Write-Host "max_pool_lease_skips: $MaxPoolLeaseSkips"
}
Write-Host "case_prefix: $CasePrefix"
Write-Host "max_tokens: $MaxTokens"
Write-Host "report_context: $(-not $NoReportContext)"
Write-Host "max_total_tokens: $MaxTotalTokens"
Write-Host "max_runtime_secs: $MaxRuntimeSecs"
Write-Host "max_no_feedback_rounds: $MaxNoFeedbackRounds"
if ($MinRuntimeContext -gt 0) {
    Write-Host "min_runtime_context: $MinRuntimeContext"
}
if ($StateConsistencyGate) {
    Write-Host "state_consistency_gate: enabled"
}
if ($ExperienceAuditGate) {
    Write-Host "experience_audit_gate: enabled"
    Write-Host "experience_audit_limit: $ExperienceAuditLimit"
    Write-Host "max_index_noisy_records: $MaxIndexNoisyRecords"
    Write-Host "max_index_noise_penalty: $MaxIndexNoisePenalty"
    Write-Host "max_quarantine_candidates: $MaxQuarantineCandidates"
    Write-Host "max_repairable_legacy_records: $MaxRepairableLegacyRecords"
    Write-Host "max_legacy_metadata_without_clean_gist: $MaxLegacyMetadataWithoutCleanGist"
}
if ($RustCheckCode.Trim().Length -gt 0 -or $RustCheckFile.Trim().Length -gt 0) {
    Write-Host "rust_check: enabled"
    Write-Host "rust_check_edition: $RustCheckEdition"
}
if ($ValidationCommand.Trim().Length -gt 0) {
    Write-Host "validation_gate: enabled"
    Write-Host "validation_phase: $ValidationPhase"
    Write-Host "validation_timeout_secs: $ValidationTimeoutSecs"
    if ($ValidationWorkdir.Trim().Length -gt 0) {
        Write-Host "validation_workdir: $ValidationWorkdir"
    }
}
if ($UseTestGateValidationCommand) {
    Write-Host "validation_gate: enabled from test-gate safe command"
    Write-Host "validation_phase: $ValidationPhase"
    Write-Host "validation_timeout_secs: $ValidationTimeoutSecs"
    if ($ValidationWorkdir.Trim().Length -gt 0) {
        Write-Host "validation_workdir: $ValidationWorkdir"
    }
}
if ($Report) {
    Write-Host "mode: report"
}
if ($ReportJson.Trim().Length -gt 0) {
    Write-Host "report_json: $ReportJson"
    if (-not $ReportMode) {
        Write-Host "post_run_report: enabled"
        if ($RunReportRefresh) {
            Write-Host "run_report_refresh: enabled"
        }
        if ($PostRunReportGate) {
            Write-Host "post_run_report_gate: enabled"
        }
        if ($PostRunContinuationGate) {
            Write-Host "post_run_continuation_gate: enabled"
        }
    }
}
if ($RequireHelperStageRoles.Trim().Length -gt 0) {
    Write-Host "require_helper_stage_roles: $RequireHelperStageRoles"
}
if ($RequireLatestHelperStageRoles.Trim().Length -gt 0) {
    Write-Host "require_latest_helper_stage_roles: $RequireLatestHelperStageRoles"
}
if ($RequireUsefulLatestHelperStageFeedback) {
    Write-Host "require_useful_latest_helper_stage_feedback: enabled"
}
if ($RequireCompleteLatestHelperStageFeedback) {
    Write-Host "require_complete_latest_helper_stage_feedback: enabled"
}
if ($RequireCleanHelperStageFeedback) {
    Write-Host "require_clean_helper_stage_feedback: enabled"
}
if ($RequireFinalJsonPoolStageDispatch) {
    Write-Host "require_final_json_pool_stage_dispatch: enabled"
}
if ($RequireTestGatePass) {
    Write-Host "require_test_gate_pass: enabled"
}
if ($RequireSafeTestGateValidationCommand) {
    Write-Host "require_safe_test_gate_validation_command: enabled"
}
if ($RequireConfiguredValidationRun) {
    Write-Host "require_configured_validation_run: enabled"
}
if ($RequireTestGateValidationRun) {
    Write-Host "require_test_gate_validation_run: enabled"
}
if ($ReportGate) {
    Write-Host "mode: report-gate"
    Write-Host "min_report_rounds: $MinReportRounds"
    Write-Host "min_success_rate: $MinSuccessRate"
    Write-Host "min_feedback_total: $MinFeedbackTotal"
    Write-Host "min_rust_checks: $MinRustChecks"
    Write-Host "min_rust_feedback_total: $MinRustFeedbackTotal"
    Write-Host "max_stream_truncations: $MaxStreamTruncations"
    Write-Host "max_missing_final: $MaxMissingFinal"
    Write-Host "max_runtime_response_failures: $MaxRuntimeResponseFailures"
    Write-Host "strict_ledger_hygiene: $StrictLedgerHygiene"
    Write-Host "allow_last_failure: $AllowLastFailure"
}
if ($ReportContinuationGate) {
    Write-Host "mode: report-continuation-gate"
    Write-Host "continuation_gate: latest/current evidence blocks unattended continuation; strict history remains advisory in JSON"
}
Write-Host ""

if ($CheckOnly) {
    Write-Host "check_only=true"
    Write-Host "touches_remote=false"
    Write-Host "starts_process=false"
    Write-Host "sends_prompt=false"
    Write-Host "command=cargo run --manifest-path $Manifest -- $($LoopArgs -join ' ')"
    if ($PostRunReportArgs.Count -gt 0) {
        Write-Host "post_run_report_command=cargo run --manifest-path $Manifest -- $($PostRunReportArgs -join ' ')"
    }
    exit 0
}

& cargo run --manifest-path $Manifest -- @LoopArgs
$loopExitCode = $LASTEXITCODE
if ($loopExitCode -ne 0) {
    exit $loopExitCode
}

if ($PostRunReportArgs.Count -gt 0) {
    Write-Host ""
    Write-Host "post_run_report: generating $ReportJson"
    & cargo run --manifest-path $Manifest -- @PostRunReportArgs
    exit $LASTEXITCODE
}

exit 0
