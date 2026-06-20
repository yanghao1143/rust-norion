param(
    [string]$RepoRoot = "D:\rust-norion",
    [string]$RemoteHost = "192.168.10.11",
    [string]$RemoteUser = "xinghuan",
    [string]$IdentityFile = "$env:USERPROFILE\.ssh\smartsteam_mac_ed25519",
    [string]$RemoteRoot = "/Users/xinghuan/smartsteam-model-box",
    [string]$RemoteLlamaServer = "/Users/xinghuan/smartsteam-model-box/bin/llama-b9616/llama-server",
    [string]$RemoteModel = "/Users/xinghuan/smartsteam-model-box/models/Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf",
    [int]$BackendPort = 7979,
    [int]$LabPort = 8789,
    [int]$MaxRuntimeSecs = 3600,
    [int]$MaxTotalTokens = 20000,
    [int]$MaxNoFeedbackRounds = 3,
    [int]$MaxFailures = 3,
    [int]$IntervalSecs = 5,
    [int]$BusyWaitSecs = 2,
    [int]$TimeoutSecs = 900,
    [int]$MaxTokens = 4096,
    [int]$SelfImproveLimit = 1,
    [string]$ArtifactDir = "",
    [string]$LedgerPath = "",
    [string]$DaemonWorkDir = "target\evolution\daemon",
    [string]$ModelCacheStatusJson = "",
    [string]$RemoteChainStatusJson = "",
    [string]$Prompt = "SmartSteam unattended evolution: propose one small safe improvement for model-pool routing, experience hygiene, streaming UX, or eval/report gates. Keep it modular and testable.",
    [switch]$Status,
    [switch]$JsonStatus,
    [switch]$NoStartChain,
    [switch]$NoPoolWorkers,
    [switch]$NoModelCacheRefresh,
    [switch]$NoReportGate,
    [switch]$BusinessGate,
    [switch]$TraceGate,
    [switch]$RequireCompleteHelperFeedbackGate,
    [switch]$RequireTestGatePass,
    [switch]$RequireSafeTestGateValidationCommand,
    [switch]$RequireTestGateValidationRun,
    [switch]$UseTestGateValidationCommand,
    [switch]$EnableTestGateValidationRun,
    [switch]$SkipBuild,
    [switch]$RestartRemote,
    [switch]$CheckOnly,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Run SmartSteam remote Gemma in bounded unattended self-evolution mode."
    Write-Host ""
    Write-Host "This is a safe-default wrapper around run-remote-gemma-evolution-loop.ps1."
    Write-Host "It always uses -Forever, but also sets bounded budgets so the loop returns"
    Write-Host "to a report gate instead of running open-ended by accident."
    Write-Host "It also inherits the remote model-cache SHA refresh and remote-chain gate"
    Write-Host "from run-remote-gemma-evolution-loop.ps1."
    Write-Host ""
    Write-Host "Default budgets:"
    Write-Host "  MaxRuntimeSecs=$MaxRuntimeSecs"
    Write-Host "  MaxTotalTokens=$MaxTotalTokens"
    Write-Host "  MaxNoFeedbackRounds=$MaxNoFeedbackRounds"
    Write-Host "  MaxFailures=$MaxFailures"
    Write-Host "  IntervalSecs=$IntervalSecs"
    Write-Host "  BusyWaitSecs=$BusyWaitSecs"
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-unattended.cmd -Status"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-unattended.cmd -JsonStatus"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-unattended.cmd -CheckOnly"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-unattended.cmd"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-unattended.cmd -MaxRuntimeSecs 7200 -MaxTotalTokens 50000"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-unattended.cmd -EnableTestGateValidationRun"
    Write-Host "  .\tools\smartsteam-forge\run-remote-gemma-unattended.cmd -NoStartChain"
    return
}

function Resolve-RepoRoot {
    param([string]$Path)

    $resolved = Resolve-Path -LiteralPath $Path -ErrorAction Stop
    return $resolved.Path
}

$RepoRoot = Resolve-RepoRoot $RepoRoot
$loopScript = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\run-remote-gemma-evolution-loop.ps1"
if (-not (Test-Path -LiteralPath $loopScript -PathType Leaf)) {
    throw "run-remote-gemma-evolution-loop.ps1 not found: $loopScript"
}
$daemonScript = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\evolution-daemon.ps1"
if (-not (Test-Path -LiteralPath $daemonScript -PathType Leaf)) {
    throw "evolution-daemon.ps1 not found: $daemonScript"
}

if ($Status -or $JsonStatus) {
    $statusArgs = @("-WorkDir", $DaemonWorkDir)
    if ($JsonStatus) {
        $statusArgs += "-JsonStatus"
    } else {
        $statusArgs += "-Status"
    }
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemonScript @statusArgs
    exit $LASTEXITCODE
}

if ([string]::IsNullOrWhiteSpace($ArtifactDir)) {
    $ArtifactDir = Join-Path $RepoRoot "target\remote-gemma-unattended"
}
if ([string]::IsNullOrWhiteSpace($LedgerPath)) {
    $LedgerPath = Join-Path $ArtifactDir "evolution-ledger.jsonl"
}

$loopParams = @{
    RepoRoot = $RepoRoot
    RemoteHost = $RemoteHost
    RemoteUser = $RemoteUser
    IdentityFile = $IdentityFile
    RemoteRoot = $RemoteRoot
    RemoteLlamaServer = $RemoteLlamaServer
    RemoteModel = $RemoteModel
    BackendPort = $BackendPort
    LabPort = $LabPort
    Forever = $true
    IntervalSecs = $IntervalSecs
    BusyWaitSecs = $BusyWaitSecs
    MaxFailures = $MaxFailures
    MaxTotalTokens = $MaxTotalTokens
    MaxRuntimeSecs = $MaxRuntimeSecs
    MaxNoFeedbackRounds = $MaxNoFeedbackRounds
    TimeoutSecs = $TimeoutSecs
    MaxTokens = $MaxTokens
    SelfImproveLimit = $SelfImproveLimit
    ArtifactDir = $ArtifactDir
    LedgerPath = $LedgerPath
    ModelCacheStatusJson = $ModelCacheStatusJson
    RemoteChainStatusJson = $RemoteChainStatusJson
    Prompt = $Prompt
}

if ($NoStartChain) { $loopParams.NoStartChain = $true }
if ($NoPoolWorkers) { $loopParams.NoPoolWorkers = $true }
if ($NoModelCacheRefresh) { $loopParams.NoModelCacheRefresh = $true }
if ($NoReportGate) { $loopParams.NoReportGate = $true }
if ($BusinessGate) { $loopParams.BusinessGate = $true }
if ($TraceGate) { $loopParams.TraceGate = $true }
if ($RequireCompleteHelperFeedbackGate) { $loopParams.RequireCompleteHelperFeedbackGate = $true }
if ($RequireTestGatePass) { $loopParams.RequireTestGatePass = $true }
if ($RequireSafeTestGateValidationCommand) { $loopParams.RequireSafeTestGateValidationCommand = $true }
if ($RequireTestGateValidationRun) { $loopParams.RequireTestGateValidationRun = $true }
if ($UseTestGateValidationCommand) { $loopParams.UseTestGateValidationCommand = $true }
if ($EnableTestGateValidationRun) { $loopParams.EnableTestGateValidationRun = $true }
if ($SkipBuild) { $loopParams.SkipBuild = $true }
if ($RestartRemote) { $loopParams.RestartRemote = $true }
if ($CheckOnly) { $loopParams.CheckOnly = $true }

Write-Host "SmartSteam unattended remote Gemma evolution"
Write-Host "repo:      $RepoRoot"
Write-Host "artifacts: $ArtifactDir"
Write-Host "ledger:    $LedgerPath"
Write-Host "budgets:   runtime=${MaxRuntimeSecs}s tokens=$MaxTotalTokens no_feedback_rounds=$MaxNoFeedbackRounds failures=$MaxFailures"
Write-Host ""

& $loopScript @loopParams
exit $LASTEXITCODE
