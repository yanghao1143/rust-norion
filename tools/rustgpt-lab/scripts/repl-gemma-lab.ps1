param(
    [string]$RepoRoot = "D:\rust-norion",
    [string]$Snapshot = "D:\hf-cache\hub\models--google--gemma-4-12B-it\snapshots\5926caa4ec0cac5cbfadaf4077420520de1d5205",
    [string]$HfCache = "D:\hf-cache",
    [int]$MistralPort = 8686,
    [int]$BackendPort = 7878,
    [int]$LabPort = 8787,
    [int]$ContextMessages = 64,
    [int]$BackendTimeoutSeconds = 900,
    [int]$MaxTokens = 262144,
    [int]$MaxSeqLen = 262144,
    [int]$RuntimeTimeoutMs = 900000,
    [double]$MinFreeRamGB = 18.0,
    [double]$MinFreeGpuGB = 13.0,
    [string]$StateDir = "",
    [switch]$UseProjectState,
    [switch]$SkipStart,
    [switch]$SkipBuild,
    [switch]$Force,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Open the rustgpt-lab REPL against Gemma through rust-norion."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\rustgpt-lab\repl-gemma-lab.cmd"
    Write-Host "  .\tools\rustgpt-lab\repl-gemma-lab.cmd -SkipStart"
    Write-Host ""
    Write-Host "Important:"
    Write-Host "  Without -SkipStart, this script calls the Gemma start helper and may start the Gemma stack."
    Write-Host "  With -SkipStart, it only attaches to an already listening rust-norion backend."
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -BackendPort <n>              rust-norion backend port, default 7878"
    Write-Host "  -LabPort <n>                  Web Lab port used when starting the stack, default 8787"
    Write-Host "  -ContextMessages <2..256>     REPL short-context message count, default 64; not a token limit"
    Write-Host "  -BackendTimeoutSeconds <n>    rustgpt-lab -> rust-norion total streaming window, default 900"
    Write-Host "  -RuntimeTimeoutMs <n>         rust-norion -> Gemma runtime timeout passed to the start helper"
    Write-Host "  -Force                        pass through startup safety override"
    return
}

function Test-LocalPort {
    param([int]$Port)
    try {
        $client = [System.Net.Sockets.TcpClient]::new()
        $async = $client.BeginConnect("127.0.0.1", $Port, $null, $null)
        $ready = $async.AsyncWaitHandle.WaitOne(250)
        if (-not $ready) {
            $client.Close()
            return $false
        }
        $client.EndConnect($async)
        $client.Close()
        return $true
    } catch {
        return $false
    }
}

if ($ContextMessages -lt 2) {
    $ContextMessages = 2
} elseif ($ContextMessages -gt 256) {
    $ContextMessages = 256
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$startScript = Join-Path $scriptDir "start-gemma-lab.ps1"
$labDir = Join-Path $RepoRoot "tools\rustgpt-lab"
$labExe = Join-Path $labDir "target\debug\rustgpt-lab.exe"

if (-not $SkipStart) {
    $startArgs = @(
        "-RepoRoot", $RepoRoot,
        "-Snapshot", $Snapshot,
        "-HfCache", $HfCache,
        "-MistralPort", $MistralPort.ToString(),
        "-BackendPort", $BackendPort.ToString(),
        "-LabPort", $LabPort.ToString(),
        "-MaxTokens", $MaxTokens.ToString(),
        "-MaxSeqLen", $MaxSeqLen.ToString(),
        "-RuntimeTimeoutMs", $RuntimeTimeoutMs.ToString(),
        "-LabBackendTimeoutSeconds", $BackendTimeoutSeconds.ToString(),
        "-MinFreeRamGB", $MinFreeRamGB.ToString(),
        "-MinFreeGpuGB", $MinFreeGpuGB.ToString()
    )
    if (-not [string]::IsNullOrWhiteSpace($StateDir)) {
        $startArgs += @("-StateDir", $StateDir)
    }
    if ($UseProjectState) {
        $startArgs += "-UseProjectState"
    }
    if ($SkipBuild) {
        $startArgs += "-SkipBuild"
    }
    if ($Force) {
        $startArgs += "-Force"
    }

    & $startScript @startArgs
} elseif (-not (Test-LocalPort -Port $BackendPort)) {
    Write-Host ""
    Write-Host "rust-norion backend is not listening on 127.0.0.1:$BackendPort."
    Write-Host "-SkipStart is attach-only: it starts no model and only opens the REPL when the rust-norion backend is already up."
    Write-Host "Use tools\rustgpt-lab\status-gemma-lab.cmd and tools\rustgpt-lab\status-built-in-lab.cmd for read-only status checks."
    Write-Host "Use tools\rustgpt-lab\start-built-in-lab.cmd to start the local test UI/backend without starting Gemma."
    Write-Host "Only omit -SkipStart when you intentionally want the Gemma lab start helper to manage the stack."
    Write-Host "Port 8686 is the optional Gemma runtime behind rust-norion, not a REPL prompt target."
    exit 1
}

if (-not (Test-Path -LiteralPath $labExe)) {
    Push-Location $labDir
    cargo build
    Pop-Location
}

Write-Host ""
Write-Host "Opening rustgpt-lab REPL against 127.0.0.1:$BackendPort"
Write-Host "context_messages=$ContextMessages backend_timeout_seconds=$BackendTimeoutSeconds"
Write-Host "Use /help for commands, /status for health, /quit to exit."
Write-Host ""

& $labExe `
    --repl `
    --backend "127.0.0.1:$BackendPort" `
    --backend-timeout-secs $BackendTimeoutSeconds `
    --context-messages $ContextMessages
