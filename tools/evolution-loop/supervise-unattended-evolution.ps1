param(
    [switch]$Status,
    [switch]$Stop,
    [switch]$CheckOnly,
    [switch]$Once,
    [string]$Backend = "127.0.0.1:7979",
    [string]$WorkDir = "target\evolution\daemon",
    [string]$Prompt = "",
    [int]$PollSecs = 60,
    [int]$MaxTokens = 4096,
    [int]$MaxTotalTokens = 2048,
    [int]$MaxRuntimeSecs = 3600,
    [int]$MaxFailures = 3,
    [int]$MaxNoFeedbackRounds = 3,
    [int]$TimeoutSecs = 900,
    [int]$ValidationTimeoutSecs = 300,
    [int]$MinRuntimeContext = 0,
    [switch]$EnableConfiguredValidationRun,
    [switch]$EnableTestGateValidationRun,
    [switch]$DisableConfiguredValidationRun,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
$DaemonScript = Join-Path $ScriptDir "daemon-evolution-loop.ps1"
$PowerShellExe = "powershell.exe"

Set-Location $RepoRoot

if ($Help) {
    Write-Host "Supervise the SmartSteam strict unattended evolution daemon."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\evolution-loop\supervise-unattended-evolution.cmd"
    Write-Host "  .\tools\evolution-loop\supervise-unattended-evolution.cmd -Status"
    Write-Host "  .\tools\evolution-loop\supervise-unattended-evolution.cmd -Stop"
    Write-Host "  .\tools\evolution-loop\supervise-unattended-evolution.cmd -Once"
    Write-Host "  .\tools\evolution-loop\supervise-unattended-evolution.cmd -CheckOnly"
    Write-Host ""
    Write-Host "The supervisor is foreground by design. It only starts the daemon when"
    Write-Host "strict status shows the daemon process is not running or has a stale PID."
    exit 0
}

if (-not (Test-Path -LiteralPath $DaemonScript -PathType Leaf)) {
    throw "daemon-evolution-loop.ps1 not found: $DaemonScript"
}

$WorkDirPath = if ([System.IO.Path]::IsPathRooted($WorkDir)) {
    $WorkDir
} else {
    Join-Path $RepoRoot $WorkDir
}
$SupervisorPidFile = Join-Path $WorkDirPath "supervisor.pid"
$SupervisorStdoutLog = Join-Path $WorkDirPath "supervisor.out.log"
$SupervisorStderrLog = Join-Path $WorkDirPath "supervisor.err.log"

if ($PollSecs -lt 5) {
    throw "-PollSecs must be at least 5."
}

if ($EnableConfiguredValidationRun -and $EnableTestGateValidationRun) {
    throw "-EnableConfiguredValidationRun and -EnableTestGateValidationRun are mutually exclusive."
}

if ($DisableConfiguredValidationRun -and $EnableConfiguredValidationRun) {
    throw "-DisableConfiguredValidationRun and -EnableConfiguredValidationRun are mutually exclusive."
}

function Quote-CommandArgument {
    param([object]$Value)

    $text = [string]$Value
    if ($text -match '^[A-Za-z0-9_./:\\-]+$') {
        return $text
    }
    return '"' + ($text -replace '"', '\"') + '"'
}

function Build-StatusArgs {
    return @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $DaemonScript,
        "-JsonStatus",
        "-StrictUnattendedEvolution",
        "-FailOnUnhealthy",
        "-Backend", $Backend,
        "-WorkDir", $WorkDir
    )
}

function Build-StartArgs {
    $argumentList = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $DaemonScript,
        "-Start",
        "-StrictUnattendedEvolution",
        "-Backend", $Backend,
        "-WorkDir", $WorkDir,
        "-MaxTokens", $MaxTokens,
        "-MaxTotalTokens", $MaxTotalTokens,
        "-MaxRuntimeSecs", $MaxRuntimeSecs,
        "-MaxFailures", $MaxFailures,
        "-MaxNoFeedbackRounds", $MaxNoFeedbackRounds,
        "-TimeoutSecs", $TimeoutSecs,
        "-ValidationTimeoutSecs", $ValidationTimeoutSecs
    )

    if ($MinRuntimeContext -gt 0) {
        $argumentList += @("-MinRuntimeContext", $MinRuntimeContext)
    }
    if ($Prompt.Trim().Length -gt 0) {
        $argumentList += @("-Prompt", $Prompt)
    }
    if ($EnableTestGateValidationRun) {
        $argumentList += "-EnableTestGateValidationRun"
    } elseif ($DisableConfiguredValidationRun) {
        $argumentList += "-DisableConfiguredValidationRun"
    } else {
        $argumentList += "-EnableConfiguredValidationRun"
    }

    return $argumentList
}

function Format-CommandPreview {
    param([object[]]$ArgumentList)

    return "$PowerShellExe " + ((@($ArgumentList) | ForEach-Object { Quote-CommandArgument $_ }) -join " ")
}

function Get-SupervisorProcess {
    if (-not (Test-Path -LiteralPath $SupervisorPidFile -PathType Leaf)) {
        return $null
    }
    $text = (Get-Content -LiteralPath $SupervisorPidFile -Raw).Trim()
    if ($text -notmatch "^\d+$") {
        return $null
    }
    try {
        return Get-Process -Id ([int]$text) -ErrorAction Stop
    } catch {
        return $null
    }
}

function Get-SupervisorPidText {
    if (-not (Test-Path -LiteralPath $SupervisorPidFile -PathType Leaf)) {
        return ""
    }
    return (Get-Content -LiteralPath $SupervisorPidFile -Raw).Trim()
}

function Read-LastNonEmptyLine {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return ""
    }
    $lines = @(Get-Content -LiteralPath $Path -Tail 20 -ErrorAction SilentlyContinue)
    for ($i = $lines.Count - 1; $i -ge 0; $i--) {
        $line = [string]$lines[$i]
        if ($line.Trim().Length -gt 0) {
            return $line
        }
    }
    return ""
}

function Write-SupervisorStatus {
    $process = Get-SupervisorProcess
    $pidText = Get-SupervisorPidText
    $pidFileExists = Test-Path -LiteralPath $SupervisorPidFile -PathType Leaf
    $stalePidFile = $pidFileExists -and $null -eq $process
    Write-Host "read_only=true"
    Write-Host "starts_process=false"
    Write-Host "sends_prompt=false"
    Write-Host "touches_remote=false"
    Write-Host "supervisor_running=$($null -ne $process)"
    if ($null -ne $process) {
        Write-Host "supervisor_pid=$($process.Id)"
    } else {
        Write-Host "supervisor_pid="
    }
    Write-Host "supervisor_pid_file_exists=$pidFileExists"
    Write-Host "supervisor_stale_pid_file=$stalePidFile"
    Write-Host "supervisor_stale_pid=$pidText"
    Write-Host "supervisor_pid_file=$SupervisorPidFile"
    Write-Host "supervisor_stdout_log=$SupervisorStdoutLog"
    Write-Host "supervisor_stderr_log=$SupervisorStderrLog"
    Write-Host "supervisor_last_stdout=$(Read-LastNonEmptyLine -Path $SupervisorStdoutLog)"
    Write-Host "supervisor_last_stderr=$(Read-LastNonEmptyLine -Path $SupervisorStderrLog)"
}

function Invoke-DaemonStatus {
    $statusArgumentList = Build-StatusArgs
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $PowerShellExe @statusArgumentList 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    try {
        $status = $text | ConvertFrom-Json
    } catch {
        throw "strict daemon status did not return parseable JSON (exit=$exitCode): $($_.Exception.Message)`n$text"
    }

    return [pscustomobject]@{
        ExitCode = $exitCode
        Text = $text
        Status = $status
    }
}

function Should-StartDaemon {
    param([object]$Status)

    $daemon = $Status.loop.daemon
    if ($null -eq $daemon) {
        return $true
    }
    if ($daemon.running -eq $true) {
        return $false
    }
    if ($daemon.stale_pid_file -eq $true) {
        return $true
    }
    return $daemon.activity_state -in @("not_running", "stale_pid")
}

function Invoke-DaemonStart {
    $startArgumentList = Build-StartArgs
    $output = & $PowerShellExe @startArgumentList 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if ($exitCode -ne 0) {
        throw "daemon start failed with exit code $exitCode`n$text"
    }
    return $text
}

$statusArgs = Build-StatusArgs
$startArgs = Build-StartArgs

if ($Status) {
    Write-SupervisorStatus
    exit 0
}

if ($Stop) {
    $process = Get-SupervisorProcess
    $pidText = Get-SupervisorPidText
    if ($CheckOnly) {
        $wouldStopPid = if ($null -ne $process) { [string]$process.Id } else { "" }
        Write-Host "check_only=true"
        Write-Host "starts_process=false"
        Write-Host "sends_prompt=false"
        Write-Host "touches_remote=false"
        Write-Host "would_stop_pid=$wouldStopPid"
        Write-Host "stale_pid=$pidText"
        Write-Host "pid_file=$SupervisorPidFile"
        exit 0
    }
    if ($null -ne $process) {
        Stop-Process -Id $process.Id -Force
        Remove-Item -LiteralPath $SupervisorPidFile -Force -ErrorAction SilentlyContinue
        Write-Host "supervisor_stop: stopped pid=$($process.Id)"
        exit 0
    }
    Remove-Item -LiteralPath $SupervisorPidFile -Force -ErrorAction SilentlyContinue
    Write-Host "supervisor_stop: not_running stale_pid=$pidText"
    exit 0
}

if ($CheckOnly) {
    Write-Host "check_only=true"
    Write-Host "starts_process=false"
    Write-Host "sends_prompt=false"
    Write-Host "touches_remote=false"
    Write-Host "supervisor_foreground=true"
    Write-Host "poll_secs=$PollSecs"
    Write-Host "once=$([bool]$Once)"
    Write-Host "pid_file=$SupervisorPidFile"
    Write-Host "stdout_log=$SupervisorStdoutLog"
    Write-Host "stderr_log=$SupervisorStderrLog"
    Write-Host "status_command=$(Format-CommandPreview -ArgumentList $statusArgs)"
    Write-Host "start_command=$(Format-CommandPreview -ArgumentList $startArgs)"
    exit 0
}

if (-not $Once) {
    New-Item -ItemType Directory -Force -Path $WorkDirPath | Out-Null
    $existingSupervisor = Get-SupervisorProcess
    if ($null -ne $existingSupervisor -and $existingSupervisor.Id -ne $PID) {
        Write-Host "supervisor_start: already_running pid=$($existingSupervisor.Id)"
        exit 0
    }
    Set-Content -Encoding ASCII -LiteralPath $SupervisorPidFile -Value ([string]$PID)
}

do {
    $statusResult = Invoke-DaemonStatus
    $status = $statusResult.Status
    $daemon = $status.loop.daemon
    $round = if ($null -ne $daemon) { $daemon.active_round } else { "" }
    $state = if ($null -ne $daemon) { $daemon.activity_state } else { "missing" }
    $running = if ($null -ne $daemon) { $daemon.running } else { $false }

    if (Should-StartDaemon -Status $status) {
        Write-Host "supervisor: daemon_start_required running=$running state=$state active_round=$round"
        $startOutput = Invoke-DaemonStart
        if (-not [string]::IsNullOrWhiteSpace($startOutput)) {
            Write-Host $startOutput.TrimEnd()
        }
    } else {
        Write-Host "supervisor: daemon_ok running=$running state=$state active_round=$round readiness=$($status.loop.readiness.ready)"
    }

    if ($Once) {
        break
    }
    Start-Sleep -Seconds $PollSecs
} while ($true)
