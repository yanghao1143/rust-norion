param(
    [string]$RepoRoot = "D:\rust-norion",
    [string]$RemoteHost = "192.168.10.11",
    [string]$RemoteUser = "xinghuan",
    [string]$IdentityFile = "$env:USERPROFILE\.ssh\smartsteam_mac_ed25519",
    [string]$RemoteRoot = "/Users/xinghuan/smartsteam-model-box",
    [string]$RemoteLlamaServer = "/Users/xinghuan/smartsteam-model-box/bin/llama-b9616/llama-server",
    [string]$RemoteModel = "/Users/xinghuan/smartsteam-model-box/models/Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf",
    [string]$RemoteSmallModel = "",
    [int]$BackendPort = 7979,
    [int]$LabPort = 8789,
    [int]$LocalModelPort = 8686,
    [int]$RemoteModelPort = 8686,
    [int]$Rounds = 1,
    [switch]$Forever,
    [int]$IntervalSecs = -1,
    [int]$BusyWaitSecs = -1,
    [int]$MaxFailures = -1,
    [int]$MaxTotalTokens = -1,
    [int]$MaxRuntimeSecs = -1,
    [int]$MaxNoFeedbackRounds = -1,
    [int]$ReportMinRounds = 0,
    [int]$TimeoutSecs = 900,
    [int]$MaxTokens = 4096,
    [int]$SelfImproveLimit = 1,
    [string]$PoolWorkerRoles = "summary,router,review,index,test-gate",
    [string]$RequiredHelperStageRoles = "",
    [string]$LedgerPath = "",
    [string]$ArtifactDir = "",
    [string]$ModelCacheStatusJson = "",
    [string]$RemoteChainStatusJson = "",
    [string]$Prompt = "",
    [string]$PromptFile = "",
    [switch]$NoPoolWorkers,
    [switch]$NoStartChain,
    [switch]$NoModelCacheRefresh,
    [switch]$NoReportGate,
    [switch]$BusinessGate,
    [switch]$NoUsefulHelperFeedbackGate,
    [switch]$RequireCompleteHelperFeedbackGate,
    [switch]$RequireTestGatePass,
    [switch]$RequireSafeTestGateValidationCommand,
    [switch]$RequireTestGateValidationRun,
    [switch]$UseTestGateValidationCommand,
    [switch]$EnableTestGateValidationRun,
    [switch]$TraceGate,
    [switch]$SkipBuild,
    [switch]$RestartRemote,
    [switch]$NoMac32GBModelPool,
    [switch]$CheckOnly,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Run SmartSteam remote Gemma + evolution-loop as one command."
    Write-Host ""
    Write-Host "Default behavior:"
    Write-Host "  1. Start or reuse the remote Mac Gemma model box on 8686."
    Write-Host "  2. Start or reuse local rust-norion backend on 7979."
    Write-Host "  3. Enable small helper workers on summary/router/review/index/test-gate."
    Write-Host "  4. Run one evolution-loop round with pool artifact refresh, route gates,"
    Write-Host "     helper stage calls, remote-chain/model-cache gate, experience audit, and report gate."
    Write-Host "  5. Refresh target\remote-gemma-chain status artifacts and run a report gate requiring latest actionable helper feedback."
    Write-Host ""
    Write-Host "Safety:"
    Write-Host "  -CheckOnly prints/preflights only; it does not start processes or send prompts."
    Write-Host "  Real runs refresh model-cache SHA provenance read-only before dispatch."
    Write-Host "  Helper workers still use start-remote-gemma-forge guard rails, so the 12B"
    Write-Host "  quality model is not reused as a helper unless that lower-level script is"
    Write-Host "  explicitly changed with its stress-test override."
    Write-Host "  By default helper workers use the Mac32GB model-pool preset:"
    Write-Host "  summary=Gemma 3 270M, router=FunctionGemma 270M, review/test-gate=Gemma E4B, index=Gemma E2B."
    Write-Host "  Pass -RemoteSmallModel for a legacy single-helper-model smoke run."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd -CheckOnly"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd -Rounds 3"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd -Forever -MaxRuntimeSecs 3600 -MaxTotalTokens 20000"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd -BusinessGate"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd -RequireCompleteHelperFeedbackGate"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd -EnableTestGateValidationRun"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd -NoPoolWorkers"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd -TraceGate"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd -Prompt `"Improve SmartSteam Forge streaming UX with one small tested change.`""
    Write-Host ""
    Write-Host "Trace:"
    Write-Host "  -TraceGate requires a backend trace schema gate path. Leave it off when the backend"
    Write-Host "  reports no configured trace schema gate."
    Write-Host ""
    Write-Host "Business gate:"
    Write-Host "  -BusinessGate enables the strict business-cycle state gate. It requires seeded"
    Write-Host "  business contract, rust-check replay, external feedback, and live evolution evidence."
    Write-Host ""
    Write-Host "Unattended mode:"
    Write-Host "  -Forever forwards evolution-loop --forever. Pair it with -MaxRuntimeSecs,"
    Write-Host "  -MaxTotalTokens, or -MaxNoFeedbackRounds for bounded unattended runs that"
    Write-Host "  still return to the report gate."
    return
}

function Resolve-RepoRoot {
    param([string]$Path)

    $resolved = Resolve-Path -LiteralPath $Path -ErrorAction Stop
    return $resolved.Path
}

function Join-Args {
    param([object[]]$Items)

    return ($Items | ForEach-Object {
        $text = [string]$_
        if ($text -match '[\s"]') {
            '"' + $text.Replace('"', '\"') + '"'
        } else {
            $text
        }
    }) -join " "
}

function Normalize-RoleList {
    param([string]$Roles)

    $items = @(
        $Roles -split "," |
            ForEach-Object { $_.Trim().ToLowerInvariant() } |
            Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
            ForEach-Object {
                if ($_ -eq "spare") { "index" } else { $_ }
            }
    )
    return @($items | Select-Object -Unique)
}

function Assert-ExitCode {
    param(
        [int]$Code,
        [string]$Label
    )

    if ($Code -ne 0) {
        throw "$Label failed with exit code $Code"
    }
}

function Write-ValidatedJsonFile {
    param(
        [string]$Path,
        [string]$Text,
        [string]$Label
    )

    if ([string]::IsNullOrWhiteSpace($Text)) {
        throw "$Label produced empty JSON"
    }
    try {
        $null = $Text | ConvertFrom-Json
    } catch {
        throw "$Label produced invalid JSON: $($_.Exception.Message)"
    }
    $parent = Split-Path -Parent $Path
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    Set-Content -Encoding UTF8 -LiteralPath $Path -Value $Text
}

function Test-LedgerHasTestGateValidationFeedback {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $false
    }

    $matches = Select-String `
        -LiteralPath $Path `
        -Pattern '"test-gate"', 'validation_command' `
        -SimpleMatch `
        -Quiet
    if (-not $matches) {
        return $false
    }

    $text = Get-Content -Raw -LiteralPath $Path
    return ($text.Contains('"test-gate"') -or $text.Contains('test-gate')) -and $text.Contains('validation_command')
}

function ConvertTo-TestGateBootstrapRunArgs {
    param([object[]]$Items)

    $valueOptionsToStrip = @(
        "--rounds",
        "--interval-secs",
        "--busy-wait-secs",
        "--max-failures",
        "--max-total-tokens",
        "--max-runtime-secs",
        "--max-no-feedback-rounds"
    )
    $result = @()
    $skipNext = $false

    foreach ($item in $Items) {
        if ($skipNext) {
            $skipNext = $false
            continue
        }

        $text = [string]$item
        if ($text -eq "--forever") {
            continue
        }
        if ($valueOptionsToStrip -contains $text) {
            $skipNext = $true
            continue
        }

        $result += $item
    }

    return @($result + @("--rounds", "1"))
}

function Set-ArgValue {
    param(
        [object[]]$Items,
        [string]$Name,
        [string]$Value
    )

    $result = @()
    $replaced = $false
    for ($i = 0; $i -lt $Items.Count; $i++) {
        $text = [string]$Items[$i]
        if ($text -eq $Name) {
            $result += $Items[$i]
            $result += $Value
            $i += 1
            $replaced = $true
            continue
        }
        $result += $Items[$i]
    }
    if (-not $replaced) {
        $result += @($Name, $Value)
    }

    return @($result)
}

function Refresh-ModelCacheStatus {
    if ($NoModelCacheRefresh) {
        Write-Host "model_cache_refresh=skipped"
        return
    }

    $remoteModelDir = "$($RemoteRoot.TrimEnd('/'))/models"
    Write-Host "refreshing remote model-cache provenance..."
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $modelCacheSyncScript `
        -RemoteHost $RemoteHost `
        -RemoteUser $RemoteUser `
        -IdentityFile $IdentityFile `
        -RemoteModelDir $remoteModelDir `
        -CheckOnly `
        -OutputJson $ModelCacheStatusJson
    Assert-ExitCode $LASTEXITCODE "remote model-cache provenance refresh"
}

function Refresh-RemoteChainStatus {
    $statusArgs = @(
        "-RepoRoot", $RepoRoot,
        "-RemoteHost", $RemoteHost,
        "-RemoteUser", $RemoteUser,
        "-IdentityFile", $IdentityFile,
        "-RemoteRoot", $RemoteRoot,
        "-RemoteModelPort", $RemoteModelPort,
        "-LocalModelPort", $LocalModelPort,
        "-BackendPort", $BackendPort,
        "-LabPort", $LabPort,
        "-PoolWorkerRoles", $stageKinds,
        "-ModelCacheStatusJson", $ModelCacheStatusJson,
        "-Status",
        "-JsonStatus",
        "-ProbeRemoteRuntime"
    )
    if (-not $NoPoolWorkers) {
        $statusArgs += @("-RequiredPoolWorkerRoles", $stageKinds)
    }

    Write-Host "refreshing remote chain status provenance..."
    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $chainStatusScript @statusArgs 2>&1
    Assert-ExitCode $LASTEXITCODE "remote chain status refresh"
    $text = (($output | ForEach-Object { $_.ToString() }) -join "`n").Trim()
    Write-ValidatedJsonFile -Path $RemoteChainStatusJson -Text $text -Label "remote chain status refresh"
}

$RepoRoot = Resolve-RepoRoot $RepoRoot
$forgeDir = Join-Path $RepoRoot "tools\smartsteam-forge"
$startForge = Join-Path $forgeDir "start-remote-gemma-forge.cmd"
$chainStatusScript = Join-Path $forgeDir "scripts\start-remote-gemma-chain.ps1"
$modelCacheSyncScript = Join-Path $forgeDir "scripts\sync-remote-gemma-model-cache.ps1"
$evolutionManifest = Join-Path $RepoRoot "tools\evolution-loop\Cargo.toml"

if (-not (Test-Path -LiteralPath $startForge -PathType Leaf)) {
    throw "start script not found: $startForge"
}
if (-not (Test-Path -LiteralPath $chainStatusScript -PathType Leaf)) {
    throw "remote chain status script not found: $chainStatusScript"
}
if (-not (Test-Path -LiteralPath $modelCacheSyncScript -PathType Leaf)) {
    throw "model cache sync script not found: $modelCacheSyncScript"
}
if (-not (Test-Path -LiteralPath $evolutionManifest -PathType Leaf)) {
    throw "evolution-loop manifest not found: $evolutionManifest"
}

if ([string]::IsNullOrWhiteSpace($ArtifactDir)) {
    $ArtifactDir = Join-Path $RepoRoot "target\remote-gemma-evolution"
}
if ([string]::IsNullOrWhiteSpace($LedgerPath)) {
    $LedgerPath = Join-Path $ArtifactDir "evolution-ledger.jsonl"
}
if ([string]::IsNullOrWhiteSpace($ModelCacheStatusJson)) {
    $ModelCacheStatusJson = Join-Path $RepoRoot "target\remote-gemma-chain\model-cache-status.json"
}
if ([string]::IsNullOrWhiteSpace($RemoteChainStatusJson)) {
    $RemoteChainStatusJson = Join-Path $RepoRoot "target\remote-gemma-chain\status-with-model-cache.json"
}

$manifestJson = Join-Path $ArtifactDir "pool-manifest.json"
$statusJson = Join-Path $ArtifactDir "pool-status.json"
$routeJson = Join-Path $ArtifactDir "pool-route-review.json"
$budgetJson = Join-Path $ArtifactDir "model-pool-budget-fairness.json"
$reportJson = Join-Path $ArtifactDir "evolution-report.json"
$leaseDir = Join-Path $ArtifactDir "leases"
$backend = "127.0.0.1:$BackendPort"
$roleItems = Normalize-RoleList $PoolWorkerRoles
$stageKinds = ($roleItems -join ",")
if ([string]::IsNullOrWhiteSpace($stageKinds)) {
    throw "PoolWorkerRoles must include at least one role"
}
if ($EnableTestGateValidationRun) {
    $UseTestGateValidationCommand = $true
    $RequireTestGatePass = $true
    $RequireSafeTestGateValidationCommand = $true
    $RequireTestGateValidationRun = $true
}
$defaultMac32GBRoleOrder = "summary,router,review,index,test-gate"
$useMac32GBModelPool = (-not $NoMac32GBModelPool) `
    -and (-not $NoPoolWorkers) `
    -and [string]::IsNullOrWhiteSpace($RemoteSmallModel) `
    -and ($stageKinds -eq $defaultMac32GBRoleOrder)
$helperStageRoleItems = @($roleItems | Where-Object { $_ -ne "review" })
$helperStageKinds = ($helperStageRoleItems -join ",")
if ($NoPoolWorkers) {
    $RequiredHelperStageRoles = ""
} elseif ([string]::IsNullOrWhiteSpace($RequiredHelperStageRoles)) {
    $RequiredHelperStageRoles = $helperStageKinds
}

$startArgs = @(
    "-RepoRoot", $RepoRoot,
    "-RemoteHost", $RemoteHost,
    "-RemoteUser", $RemoteUser,
    "-IdentityFile", $IdentityFile,
    "-RemoteRoot", $RemoteRoot,
    "-RemoteLlamaServer", $RemoteLlamaServer,
    "-RemoteModel", $RemoteModel,
    "-RemoteModelPort", $RemoteModelPort,
    "-LocalModelPort", $LocalModelPort,
    "-ModelCacheStatusJson", $ModelCacheStatusJson,
    "-PoolWorkerRoles", $stageKinds,
    "-BackendPort", $BackendPort,
    "-LabPort", $LabPort,
    "-NoForge"
)
if ($SkipBuild) {
    $startArgs += "-SkipBuild"
}
if ($RestartRemote) {
    $startArgs += "-RestartRemote"
}
if (-not $NoPoolWorkers) {
    $startArgs += @(
        "-RequiredPoolWorkerRoles", $stageKinds,
        "-EnablePoolWorkers"
    )
    if ($useMac32GBModelPool) {
        $startArgs += "-UseMac32GBModelPool"
    } elseif (-not [string]::IsNullOrWhiteSpace($RemoteSmallModel)) {
        $startArgs += @("-RemoteSmallModel", $RemoteSmallModel)
    }
    if ($roleItems -contains "index") {
        $startArgs += "-EnableIndexWorker"
    }
}

$runArgs = @(
    "run", "--manifest-path", $evolutionManifest, "--",
    "--backend", $backend,
    "--timeout-secs", $TimeoutSecs,
    "--max-tokens", $MaxTokens,
    "--self-improve-limit", $SelfImproveLimit,
    "--ledger", $LedgerPath,
    "--remote-chain-status-json", $RemoteChainStatusJson,
    "--remote-chain-gate",
    "--experience-audit-gate"
)
if ($Forever) {
    $runArgs += "--forever"
} else {
    $runArgs += @("--rounds", $Rounds)
}
if ($IntervalSecs -ge 0) {
    $runArgs += @("--interval-secs", $IntervalSecs)
}
if ($BusyWaitSecs -ge 0) {
    $runArgs += @("--busy-wait-secs", $BusyWaitSecs)
}
if ($MaxFailures -ge 0) {
    $runArgs += @("--max-failures", $MaxFailures)
}
if ($MaxTotalTokens -ge 0) {
    $runArgs += @("--max-total-tokens", $MaxTotalTokens)
}
if ($MaxRuntimeSecs -ge 0) {
    $runArgs += @("--max-runtime-secs", $MaxRuntimeSecs)
}
if ($MaxNoFeedbackRounds -ge 0) {
    $runArgs += @("--max-no-feedback-rounds", $MaxNoFeedbackRounds)
}
if ($BusinessGate) {
    $runArgs += "--business-gate"
}
if ($TraceGate) {
    $runArgs += "--trace-gate"
}
if (-not $NoPoolWorkers) {
    $runArgs += @(
        "--pool-manifest-json", $manifestJson,
        "--pool-status-json", $statusJson,
        "--pool-route-json", $routeJson,
        "--pool-budget-fairness-json", $budgetJson,
        "--refresh-pool-artifacts",
        "--pool-capacity-gate",
        "--pool-alignment-gate",
        "--require-pool-route",
        "--pool-lease-dir", $leaseDir
    )
    $runArgs += @(
        "--pool-stage-route-task-kinds", $stageKinds,
        "--pool-stage-route-gate",
        "--execute-pool-stage-calls"
    )
}
if ($UseTestGateValidationCommand) {
    $testGateValidationArgs = @("--use-test-gate-validation-command", "--validation-phase", "pre")
} else {
    $testGateValidationArgs = @()
}
if (-not [string]::IsNullOrWhiteSpace($Prompt)) {
    $runArgs += @("--prompt", $Prompt)
}
if (-not [string]::IsNullOrWhiteSpace($PromptFile)) {
    $runArgs += @("--prompt-file", $PromptFile)
}
$runArgsWithoutTestGateValidation = @($runArgs)
$runArgs += $testGateValidationArgs

$effectiveReportMinRounds = if ($ReportMinRounds -gt 0) {
    $ReportMinRounds
} elseif ($Forever) {
    1
} else {
    $Rounds
}

$reportArgs = @(
    "run", "--manifest-path", $evolutionManifest, "--",
    "--report",
    "--ledger", $LedgerPath,
    "--report-gate",
    "--report-json", $reportJson,
    "--remote-chain-status-json", $RemoteChainStatusJson,
    "--remote-chain-gate",
    "--min-report-rounds", $effectiveReportMinRounds,
    "--strict-ledger-hygiene",
    "--require-round-wall-clock-evidence"
)
if (-not $NoPoolWorkers) {
    $reportArgs += @(
        "--pool-manifest-json", $manifestJson,
        "--pool-status-json", $statusJson,
        "--pool-route-json", $routeJson,
        "--pool-stage-route-task-kinds", $stageKinds,
        "--pool-budget-fairness-json", $budgetJson,
        "--pool-alignment-gate"
    )
}
if (-not [string]::IsNullOrWhiteSpace($RequiredHelperStageRoles)) {
    $reportArgs += @("--require-latest-helper-stage-roles", $RequiredHelperStageRoles)
    if (-not $NoUsefulHelperFeedbackGate) {
        $reportArgs += "--require-useful-latest-helper-stage-feedback"
    }
    if ($RequireCompleteHelperFeedbackGate) {
        $reportArgs += "--require-complete-latest-helper-stage-feedback"
    }
}
if ($RequireTestGatePass) {
    $reportArgs += "--require-test-gate-pass"
}
if ($RequireSafeTestGateValidationCommand) {
    $reportArgs += "--require-safe-test-gate-validation-command"
}
if ($RequireTestGateValidationRun) {
    $reportArgs += "--require-test-gate-validation-run"
}

Write-Host "SmartSteam remote Gemma evolution-loop"
Write-Host "repo:      $RepoRoot"
Write-Host "backend:   $backend"
Write-Host "artifacts: $ArtifactDir"
Write-Host "ledger:    $LedgerPath"
Write-Host "model_cache_status_json: $ModelCacheStatusJson"
Write-Host "remote_chain_status_json: $RemoteChainStatusJson"
Write-Host "helpers:   $(if ($NoPoolWorkers) { 'disabled' } else { $stageKinds })"
Write-Host "mode:      $(if ($Forever) { 'forever' } else { "rounds=$Rounds" })"
if ($EnableTestGateValidationRun) {
    Write-Host "test_gate_validation_run: enabled"
    Write-Host "test_gate_validation_bootstrap: auto-if-missing"
    if ($MaxRuntimeSecs -ge 0) {
        Write-Host "test_gate_validation_bootstrap_budget: counts-against-max-runtime"
    }
}
if (-not $NoPoolWorkers) {
    Write-Host "helper_model_pool_preset: $(if ($useMac32GBModelPool) { 'mac32gb' } else { 'custom' })"
}
Write-Host ""
Write-Host "start command:"
Write-Host "  $startForge $(Join-Args -Items $startArgs)"
Write-Host ""
Write-Host "run command:"
Write-Host "  cargo $(Join-Args -Items $runArgs)"
if (-not $NoReportGate) {
    Write-Host ""
    Write-Host "report command:"
    Write-Host "  cargo $(Join-Args -Items $reportArgs)"
}

if ($CheckOnly) {
    Write-Host ""
    Write-Host "check_only=true"
    if (-not $NoStartChain) {
        Write-Host "running remote chain CheckOnly preflight..."
        & $startForge @startArgs -CheckOnly
        Assert-ExitCode $LASTEXITCODE "remote chain CheckOnly"
    }
    Write-Host "evolution_loop_check_only=PASS"
    Write-Host "starts_process=false"
    Write-Host "sends_prompt=false"
    return
}

New-Item -ItemType Directory -Force -Path $ArtifactDir | Out-Null
New-Item -ItemType Directory -Force -Path $leaseDir | Out-Null

Refresh-ModelCacheStatus

if (-not $NoStartChain) {
    Write-Host ""
    Write-Host "starting/reusing remote Gemma chain..."
    & $startForge @startArgs
    Assert-ExitCode $LASTEXITCODE "remote Gemma chain"
}

Refresh-RemoteChainStatus

Push-Location $RepoRoot
try {
    if ($EnableTestGateValidationRun -and $UseTestGateValidationCommand -and -not (Test-LedgerHasTestGateValidationFeedback -Path $LedgerPath)) {
        if ($NoPoolWorkers) {
            throw "test-gate validation bootstrap requires pool workers to produce test-gate feedback"
        }

        $bootstrapRunArgs = ConvertTo-TestGateBootstrapRunArgs -Items $runArgsWithoutTestGateValidation
        Write-Host ""
        Write-Host "bootstrapping test-gate validation feedback..."
        Write-Host "bootstrap command:"
        Write-Host "  cargo $(Join-Args -Items $bootstrapRunArgs)"
        $bootstrapTimer = [System.Diagnostics.Stopwatch]::StartNew()
        & cargo @bootstrapRunArgs
        $bootstrapTimer.Stop()
        Assert-ExitCode $LASTEXITCODE "test-gate validation bootstrap"

        if (-not (Test-LedgerHasTestGateValidationFeedback -Path $LedgerPath)) {
            throw "test-gate validation bootstrap completed but no test-gate validation_command was recorded in ledger"
        }
        $bootstrapElapsedSecs = [int][Math]::Ceiling($bootstrapTimer.Elapsed.TotalSeconds)
        Write-Host "test_gate_validation_bootstrap_elapsed_secs=$bootstrapElapsedSecs"

        if ($MaxRuntimeSecs -ge 0) {
            $remainingRuntimeSecs = $MaxRuntimeSecs - $bootstrapElapsedSecs
            Write-Host "test_gate_validation_remaining_runtime_secs=$remainingRuntimeSecs"
            if ($remainingRuntimeSecs -lt 1) {
                throw "test-gate validation bootstrap consumed the configured runtime budget (${bootstrapElapsedSecs}s >= ${MaxRuntimeSecs}s)"
            }
            $runArgs = Set-ArgValue -Items $runArgs -Name "--max-runtime-secs" -Value ([string]$remainingRuntimeSecs)
            Write-Host "adjusted run command after bootstrap:"
            Write-Host "  cargo $(Join-Args -Items $runArgs)"
        }
    }

    Write-Host ""
    Write-Host "running evolution-loop..."
    & cargo @runArgs
    Assert-ExitCode $LASTEXITCODE "evolution-loop run"

    if (-not $NoReportGate) {
        Write-Host ""
        Write-Host "running evolution-loop report gate..."
        & cargo @reportArgs
        Assert-ExitCode $LASTEXITCODE "evolution-loop report gate"
    }
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "remote_gemma_evolution_loop=PASS"
Write-Host "ledger=$LedgerPath"
Write-Host "report_json=$reportJson"
