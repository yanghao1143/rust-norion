param(
    [switch]$Start,
    [switch]$Stop,
    [switch]$Status,
    [switch]$JsonStatus,
    [switch]$CheckOnly,
    [string]$Backend = "127.0.0.1:7979",
    [string]$RemoteChainStatusJson = "",
    [string]$ModelCacheStatusJson = "",
    [string]$WorkDir = "target\evolution\daemon",
    [string]$Ledger = "",
    [string]$ReportJson = "",
    [string]$Prompt = "",
    [int]$IntervalSecs = 30,
    [int]$MaxTokens = 4096,
    [int]$MaxTotalTokens = 512,
    [int]$MaxRuntimeSecs = 900,
    [int]$MaxFailures = 3,
    [int]$MaxNoFeedbackRounds = 3,
    [int]$TimeoutSecs = 900,
    [int]$ValidationTimeoutSecs = 300,
    [int]$MinRuntimeContext = 0,
    [switch]$EnableConfiguredValidationRun,
    [switch]$DisableConfiguredValidationRun,
    [string]$ConfiguredValidationCommand = "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\evolution-loop-daemon-check",
    [switch]$EnableTestGateValidationRun,
    [switch]$StrictUnattendedEvolution,
    [switch]$RequireValidationExecution,
    [switch]$RefreshRemoteChainStatus,
    [switch]$SkipBackend,
    [switch]$SkipRemoteChain,
    [switch]$FailOnUnhealthy,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
Set-Location $RepoRoot

if ($Help) {
    Write-Host "Start/stop/read a budgeted SmartSteam evolution-loop daemon."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\evolution-loop\daemon-evolution-loop.cmd -Start"
    Write-Host "  .\tools\evolution-loop\daemon-evolution-loop.cmd -Status"
    Write-Host "  .\tools\evolution-loop\daemon-evolution-loop.cmd -Stop"
    Write-Host "  .\tools\evolution-loop\daemon-evolution-loop.cmd -Status -FailOnUnhealthy"
    Write-Host "  .\tools\evolution-loop\daemon-evolution-loop.cmd -JsonStatus -RequireValidationExecution -FailOnUnhealthy"
    Write-Host "  .\tools\evolution-loop\daemon-evolution-loop.cmd -JsonStatus -StrictUnattendedEvolution -FailOnUnhealthy"
    Write-Host "  .\tools\evolution-loop\daemon-evolution-loop.cmd -Start -DisableConfiguredValidationRun"
    Write-Host ""
    Write-Host "Defaults are budgeted: MaxTokens=4096 MaxTotalTokens=512 MaxRuntimeSecs=900 TimeoutSecs=900 ValidationTimeoutSecs=300."
    exit 0
}

if (-not $Start -and -not $Stop -and -not $Status -and -not $JsonStatus) {
    $Status = $true
}

if ($EnableConfiguredValidationRun -and $DisableConfiguredValidationRun) {
    throw "-EnableConfiguredValidationRun and -DisableConfiguredValidationRun are mutually exclusive."
}

if ($EnableConfiguredValidationRun -and $EnableTestGateValidationRun) {
    throw "-EnableConfiguredValidationRun and -EnableTestGateValidationRun are mutually exclusive because their report gates require different validation_command_source values."
}

$RequireValidationExecutionEffective = [bool]$RequireValidationExecution -or [bool]$StrictUnattendedEvolution

function Resolve-RepoPath {
    param([string]$Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return Join-Path $RepoRoot $Path
}

function Get-LiveProcess {
    param([string]$PidFile)

    if (-not (Test-Path -LiteralPath $PidFile -PathType Leaf)) {
        return $null
    }
    $text = (Get-Content -LiteralPath $PidFile -Raw).Trim()
    if ($text -notmatch "^\d+$") {
        return $null
    }
    try {
        return Get-Process -Id ([int]$text) -ErrorAction Stop
    } catch {
        return $null
    }
}

function Read-PidFileValue {
    param([string]$PidFile)

    if (-not (Test-Path -LiteralPath $PidFile -PathType Leaf)) {
        return $null
    }
    $text = (Get-Content -LiteralPath $PidFile -Raw).Trim()
    if ($text -notmatch "^\d+$") {
        return $null
    }
    return [int]$text
}

function Get-ChildProcessTree {
    param([int]$RootPid)

    $all = @()
    try {
        $all = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name, CommandLine)
    } catch {
        return @()
    }

    $childrenByParent = @{}
    foreach ($process in $all) {
        $parentId = [int]$process.ParentProcessId
        if (-not $childrenByParent.ContainsKey($parentId)) {
            $childrenByParent[$parentId] = @()
        }
        $childrenByParent[$parentId] += $process
    }

    $result = @()
    $queue = @($RootPid)
    while ($queue.Count -gt 0) {
        $parent = [int]$queue[0]
        if ($queue.Count -eq 1) {
            $queue = @()
        } else {
            $queue = @($queue[1..($queue.Count - 1)])
        }
        if (-not $childrenByParent.ContainsKey($parent)) {
            continue
        }
        foreach ($child in @($childrenByParent[$parent])) {
            $result += $child
            $queue += [int]$child.ProcessId
        }
    }

    return @($result | Sort-Object ProcessId -Unique)
}

function Format-ProcessTreePids {
    param(
        [int]$RootPid,
        [object[]]$Descendants
    )

    $ids = @()
    foreach ($descendant in @($Descendants)) {
        $ids += [int]$descendant.ProcessId
    }
    $ids += $RootPid
    return (@($ids | Sort-Object -Unique) -join ",")
}

function Stop-DaemonProcessTree {
    param(
        [object]$RootProcess,
        [object[]]$Descendants
    )

    $orderedDescendants = @($Descendants | Sort-Object ProcessId -Descending)
    foreach ($descendant in $orderedDescendants) {
        try {
            Stop-Process -Id ([int]$descendant.ProcessId) -ErrorAction Stop
        } catch {
            Write-Host "daemon_stop_warning: failed_to_stop_child pid=$($descendant.ProcessId) name=$($descendant.Name) error=$($_.Exception.Message)"
        }
    }
    Stop-Process -Id $RootProcess.Id -ErrorAction Stop
}

function Find-DaemonOrphanProcesses {
    param(
        [string]$LedgerPath,
        [int]$StalePid = 0
    )

    $normalizedLedger = [string]$LedgerPath
    if ($normalizedLedger.Trim().Length -eq 0) {
        return @()
    }
    $normalizedLedger = Normalize-ProcessMatchText $normalizedLedger
    $ledgerFile = Normalize-ProcessMatchText (Split-Path -Leaf $LedgerPath)
    $ledgerDir = Normalize-ProcessMatchText (Split-Path -Parent $LedgerPath)
    $ledgerDirTail = ""
    if ($ledgerDir -match '(target\\evolution\\daemon)$') {
        $ledgerDirTail = $Matches[1]
    }
    $ledgerRelativeTail = ""
    if ($ledgerFile.Trim().Length -gt 0 -and $ledgerDirTail.Trim().Length -gt 0) {
        $ledgerRelativeTail = "$ledgerDirTail\$ledgerFile"
    }

    try {
        $processes = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name, CommandLine)
    } catch {
        return @()
    }

    $matchedProcesses = @()
    foreach ($process in $processes) {
        $commandLine = [string]$process.CommandLine
        if ($commandLine.Trim().Length -eq 0) {
            continue
        }
        $normalizedCommand = Normalize-ProcessMatchText $commandLine
        $matchesLedger = $normalizedCommand.Contains($normalizedLedger)
        if (-not $matchesLedger -and $ledgerFile.Trim().Length -gt 0 -and $ledgerDirTail.Trim().Length -gt 0) {
            $matchesLedger = $normalizedCommand.Contains($ledgerFile) -and $normalizedCommand.Contains($ledgerDirTail)
        }
        if (-not $matchesLedger -and $ledgerRelativeTail.Trim().Length -gt 0) {
            $matchesLedger = $normalizedCommand.Contains($ledgerRelativeTail)
        }
        if (-not $matchesLedger -and $ledgerDirTail.Trim().Length -gt 0) {
            $matchesLedger = $normalizedCommand.Contains("evolution-loop.launch.ps1") -and $normalizedCommand.Contains($ledgerDirTail)
        }
        if (-not $matchesLedger) {
            continue
        }
        if ($normalizedCommand -notmatch 'evolution-loop') {
            continue
        }
        if ($normalizedCommand.Contains("daemon-evolution-loop.ps1") -or $normalizedCommand.Contains("status-evolution-loop.ps1")) {
            continue
        }
        if ([int]$process.ProcessId -eq $PID) {
            continue
        }
        if ($StalePid -gt 0 -and [int]$process.ProcessId -eq $StalePid) {
            continue
        }
        $matchedProcesses += $process
    }

    if ($matchedProcesses.Count -eq 0) {
        $rawLedger = [string]$LedgerPath
        $rawLedgerFile = [string](Split-Path -Leaf $LedgerPath)
        foreach ($process in $processes) {
            $commandLine = [string]$process.CommandLine
            if ($commandLine.Trim().Length -eq 0) {
                continue
            }
            $commandLower = $commandLine.ToLowerInvariant()
            $matchesRawLedger = $commandLine.Contains($rawLedger)
            if (-not $matchesRawLedger -and $rawLedgerFile.Trim().Length -gt 0) {
                $matchesRawLedger = $commandLine.Contains($rawLedgerFile) -and $commandLower.Contains("target\evolution\daemon")
            }
            if (-not $matchesRawLedger) {
                $matchesRawLedger = $commandLower.Contains("evolution-loop.launch.ps1") -and $commandLower.Contains("target\evolution\daemon")
            }
            if (-not $matchesRawLedger -or -not $commandLower.Contains("evolution-loop")) {
                continue
            }
            if ($commandLower.Contains("daemon-evolution-loop.ps1") -or $commandLower.Contains("status-evolution-loop.ps1")) {
                continue
            }
            if ([int]$process.ProcessId -eq $PID) {
                continue
            }
            if ($StalePid -gt 0 -and [int]$process.ProcessId -eq $StalePid) {
                continue
            }
            $matchedProcesses += $process
        }
    }

    return @($matchedProcesses | Sort-Object ProcessId -Unique)
}

function Select-DaemonAdoptionProcess {
    param([object[]]$Processes)

    $items = @($Processes)
    if ($items.Count -eq 0) {
        return $null
    }

    $ids = @{}
    foreach ($process in $items) {
        $ids[[int]$process.ProcessId] = $true
    }

    $ranked = @()
    foreach ($process in $items) {
        $name = ([string]$process.Name).ToLowerInvariant()
        $command = Normalize-ProcessMatchText ([string]$process.CommandLine)
        $rank = 50
        if (-not $ids.ContainsKey([int]$process.ParentProcessId)) {
            $rank -= 20
        }
        if ($command.Contains("evolution-loop.launch.ps1")) {
            $rank -= 10
        } elseif ($command.Contains("start-evolution-loop.ps1")) {
            $rank -= 8
        } elseif ($name -eq "cargo.exe") {
            $rank -= 6
        } elseif ($name -eq "evolution-loop.exe") {
            $rank -= 4
        }
        $ranked += [pscustomobject]@{
            rank = $rank
            pid = [int]$process.ProcessId
            process = $process
        }
    }

    $preferred = @($ranked | Sort-Object rank, pid | Select-Object -First 1)
    if ($preferred.Count -eq 0) {
        return $null
    }
    return $preferred[0].process
}

function Normalize-ProcessMatchText {
    param([string]$Text)

    return (([string]$Text).ToLowerInvariant() -replace '/', '\' -replace '\\+', '\')
}

function Stop-DaemonMatchedProcesses {
    param([object[]]$Processes)

    $ordered = @($Processes | Sort-Object ProcessId -Descending)
    foreach ($process in $ordered) {
        try {
            Stop-Process -Id ([int]$process.ProcessId) -ErrorAction Stop
        } catch {
            Write-Host "daemon_stop_warning: failed_to_stop_orphan pid=$($process.ProcessId) name=$($process.Name) error=$($_.Exception.Message)"
        }
    }
}

function Read-FileLinesShared {
    param([string]$Path)

    $stream = $null
    $reader = $null
    try {
        $stream = [System.IO.File]::Open(
            $Path,
            [System.IO.FileMode]::Open,
            [System.IO.FileAccess]::Read,
            [System.IO.FileShare]::ReadWrite
        )
        $reader = [System.IO.StreamReader]::new($stream, [System.Text.Encoding]::UTF8, $true)
        $lines = [System.Collections.Generic.List[string]]::new()
        while ($true) {
            $line = $reader.ReadLine()
            if ($null -eq $line) {
                break
            }
            [void]$lines.Add($line)
        }
        return @($lines.ToArray())
    } finally {
        if ($null -ne $reader) {
            $reader.Dispose()
        } elseif ($null -ne $stream) {
            $stream.Dispose()
        }
    }
}

function Read-LogTail {
    param(
        [string]$Path,
        [int]$Count = 12
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return @()
    }
    try {
        $lines = @(Read-FileLinesShared -Path $Path)
    } catch {
        return @("log_read_error: $($_.Exception.Message)")
    }
    if ($lines.Count -le $Count) {
        return @($lines)
    }
    return @($lines[($lines.Count - $Count)..($lines.Count - 1)])
}

function Read-LastStopReason {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return ""
    }
    try {
        $lines = @(Read-FileLinesShared -Path $Path)
    } catch {
        return ""
    }
    for ($i = $lines.Count - 1; $i -ge 0; $i -= 1) {
        $line = [string]$lines[$i]
        if ($line.StartsWith("stopping:")) {
            return $line.Trim()
        }
    }
    return ""
}

function Get-LogProgressSummary {
    param(
        [object[]]$StdoutTail,
        [object[]]$StderrTail
    )

    $stdoutReadError = @($StdoutTail | Where-Object { ([string]$_).StartsWith("log_read_error:") }).Count -gt 0
    $stderrReadError = @($StderrTail | Where-Object { ([string]$_).StartsWith("log_read_error:") }).Count -gt 0
    $latestLine = ""
    $latestRoundLine = ""
    $latestRound = $null
    $latestEvent = ""
    $latestStage = ""
    $latestStartedRound = $null
    $latestCompletedRound = $null
    $latestDoneRound = $null
    $latestRoundState = "unknown"

    foreach ($rawLine in @($StdoutTail)) {
        $line = [string]$rawLine
        if ($line.Trim().Length -eq 0) {
            continue
        }
        $latestLine = $line
        if ($line -match '^\[round\s+(\d+)\]\s+(.+)$') {
            $round = [int]$Matches[1]
            $rest = [string]$Matches[2]
            $latestRoundLine = $line
            $latestRound = $round
            if ($rest -match '^stage\s+(.+)$') {
                $latestEvent = "stage"
                $latestStage = [string]$Matches[1]
                if ($latestStage -eq "ledger_append:done") {
                    $latestCompletedRound = $round
                }
            } elseif ($rest -match '^case=') {
                $latestEvent = "case"
                $latestStartedRound = $round
            } elseif ($rest -match '^ok\b') {
                $latestEvent = "ok"
                $latestCompletedRound = $round
            } elseif ($rest -match '^failed\b') {
                $latestEvent = "failed"
                $latestCompletedRound = $round
            } elseif ($rest -match '^done\b') {
                $latestEvent = "done"
                $latestDoneRound = $round
            } elseif ($rest -match '^meta\b') {
                $latestEvent = "meta"
            } elseif ($rest -match '^status\b') {
                $latestEvent = "status"
            } else {
                $latestEvent = ($rest -split '\s+')[0]
            }
        }
    }

    $inProgress = $false
    if ($null -ne $latestRound) {
        if ($null -ne $latestCompletedRound -and $latestCompletedRound -ge $latestRound) {
            $latestRoundState = "completed"
        } elseif ($null -ne $latestDoneRound -and $latestDoneRound -ge $latestRound) {
            $latestRoundState = "round_done_waiting_ledger_commit"
        } elseif ($null -eq $latestCompletedRound -or $latestRound -gt $latestCompletedRound) {
            $inProgress = $true
            $latestRoundState = "in_progress"
        }
    }

    $linePreview = $latestLine
    if ($linePreview.Length -gt 240) {
        $linePreview = $linePreview.Substring(0, 240)
    }
    $roundLinePreview = $latestRoundLine
    if ($roundLinePreview.Length -gt 240) {
        $roundLinePreview = $roundLinePreview.Substring(0, 240)
    }

    return [pscustomobject][ordered]@{
        stdout_readable = -not $stdoutReadError
        stderr_readable = -not $stderrReadError
        latest_round = $latestRound
        latest_event = $latestEvent
        latest_stage = $latestStage
        latest_started_round = $latestStartedRound
        latest_completed_round = $latestCompletedRound
        latest_done_round = $latestDoneRound
        latest_round_state = $latestRoundState
        round_in_progress = $inProgress
        latest_line_preview = $linePreview
        latest_round_line_preview = $roundLinePreview
    }
}

function Get-LaunchValidationSummary {
    param([object[]]$StderrTail)

    $launchLine = ""
    foreach ($rawLine in @($StderrTail)) {
        $line = [string]$rawLine
        if ($line -match '\bRunning\b' -or $line -match 'command=powershell\.exe') {
            $launchLine = $line
        }
    }

    $configuredRun = $launchLine.Contains("--require-configured-validation-run") -or $launchLine.Contains("-RequireConfiguredValidationRun")
    $testGateRun = $launchLine.Contains("--require-test-gate-validation-run") -or $launchLine.Contains("-RequireTestGateValidationRun")
    $validationCommand = $launchLine.Contains("--validation-command") -or $launchLine.Contains("-ValidationCommand")
    $useTestGateCommand = $launchLine.Contains("--use-test-gate-validation-command") -or $launchLine.Contains("-UseTestGateValidationCommand")
    $safeTestGateCommand = $launchLine.Contains("--require-safe-test-gate-validation-command") -or $launchLine.Contains("-RequireSafeTestGateValidationCommand")
    $testGatePass = $launchLine.Contains("--require-test-gate-pass") -or $launchLine.Contains("-RequireTestGatePass")

    $mode = "none"
    if ($configuredRun -and $testGateRun) {
        $mode = "mixed"
    } elseif ($configuredRun -or $validationCommand) {
        $mode = "configured"
    } elseif ($testGateRun -or $useTestGateCommand) {
        $mode = "test-gate"
    }

    $nextStep = "start daemon with -EnableConfiguredValidationRun or -EnableTestGateValidationRun when validation execution should be enforced"
    if ($mode -eq "configured") {
        $nextStep = "configured validation execution is requested by the daemon launch command"
    } elseif ($mode -eq "test-gate") {
        $nextStep = "test-gate validation execution is requested by the daemon launch command"
    } elseif ($mode -eq "mixed") {
        $nextStep = "restart with only one validation source; configured and test-gate validation gates are mutually exclusive"
    } elseif ($launchLine.Trim().Length -eq 0) {
        $nextStep = "launch command was not found in stderr tail; inspect daemon stderr log"
    }

    $preview = $launchLine
    if ($preview.Length -gt 360) {
        $preview = $preview.Substring(0, 360)
    }

    return [pscustomobject][ordered]@{
        launch_command_seen = $launchLine.Trim().Length -gt 0
        mode = $mode
        validation_command_present = $validationCommand
        use_test_gate_validation_command = $useTestGateCommand
        require_configured_validation_run = $configuredRun
        require_test_gate_validation_run = $testGateRun
        require_safe_test_gate_validation_command = $safeTestGateCommand
        require_test_gate_pass = $testGatePass
        validation_execution_enforced = $configuredRun -or $testGateRun
        launch_command_preview = $preview
        next_step = $nextStep
    }
}

function Read-LedgerLatestRound {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $null
    }
    try {
        $lines = @(Read-FileLinesShared -Path $Path)
    } catch {
        return $null
    }
    for ($i = $lines.Count - 1; $i -ge 0; $i -= 1) {
        $line = ([string]$lines[$i]).Trim()
        if ($line.Length -eq 0) {
            continue
        }
        try {
            $record = $line | ConvertFrom-Json
        } catch {
            continue
        }
        if (@($record.PSObject.Properties.Name) -contains "round") {
            return [int]$record.round
        }
    }
    return $null
}

function Get-FileFreshness {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [pscustomobject][ordered]@{
            exists = $false
            path = $Path
            length_bytes = 0
            last_write_local = ""
            last_write_utc = ""
            age_seconds = $null
        }
    }
    $item = Get-Item -LiteralPath $Path
    $ageSeconds = [Math]::Max(0, [int][Math]::Floor(([DateTime]::UtcNow - $item.LastWriteTimeUtc).TotalSeconds))
    return [pscustomobject][ordered]@{
        exists = $true
        path = $Path
        length_bytes = [int64]$item.Length
        last_write_local = $item.LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss")
        last_write_utc = $item.LastWriteTimeUtc.ToString("o")
        age_seconds = $ageSeconds
    }
}

function Get-DaemonActivitySummary {
    param(
        [bool]$Running,
        [bool]$StalePidFile,
        [bool]$AdoptedOrphan,
        [object]$LogSummary,
        [object]$StdoutFreshness,
        [object]$LedgerFreshness,
        [object]$LedgerLagRounds,
        [object]$BackendBusyStatus,
        [int]$RoundTimeoutSecs = 900
    )

    $stdoutAge = $StdoutFreshness.age_seconds
    $ledgerAge = $LedgerFreshness.age_seconds
    $state = "unknown"
    $ok = $false
    $reason = "no_round_evidence"
    $nextStep = "inspect daemon stdout/stderr logs"

    if (-not $Running) {
        $state = "not_running"
        $reason = "daemon_process_not_running"
        $nextStep = "start daemon when unattended evolution should run"
    } elseif ($StalePidFile -and -not $AdoptedOrphan) {
        $state = "stale_pid"
        $reason = "pid_file_points_to_missing_process"
        $nextStep = "remove stale pid file or restart daemon"
    } elseif ($AdoptedOrphan -and $null -eq $LogSummary.latest_round) {
        $state = "running_adopted_orphan"
        $ok = $true
        $reason = "matched_live_evolution_process_by_ledger"
        $nextStep = "wait for the adopted evolution-loop process or inspect backend health"
    } elseif ($LogSummary.round_in_progress -eq $true) {
        if ($null -eq $stdoutAge) {
            $state = "in_progress_no_stdout"
            $reason = "round_in_progress_but_stdout_missing"
            $nextStep = "inspect stdout log path and process redirection"
        } elseif ([int]$stdoutAge -le 300) {
            $state = "active"
            $ok = $true
            $reason = "round_in_progress_stdout_recent"
            $nextStep = "wait for current round to finish or inspect log_preview"
        } elseif ($null -ne $BackendBusyStatus -and $BackendBusyStatus.checked -eq $true -and $BackendBusyStatus.busy -eq $true -and [int]$stdoutAge -le $RoundTimeoutSecs) {
            $state = "slow_in_progress"
            $ok = $true
            $reason = "round_in_progress_backend_busy_within_timeout"
            $nextStep = "wait for active backend request to finish before starting or replaying another round"
        } elseif ($null -ne $BackendBusyStatus -and $BackendBusyStatus.checked -eq $true -and $BackendBusyStatus.busy -eq $true) {
            $state = "stale_in_progress"
            $reason = "round_in_progress_backend_busy_past_timeout"
            $nextStep = "inspect backend request timeout, model worker, and daemon stdout"
        } else {
            $state = "stale_in_progress"
            $reason = "round_in_progress_stdout_stale"
            $nextStep = "check backend health, model worker, and daemon stdout"
        }
    } elseif ($LogSummary.latest_round_state -eq "round_done_waiting_ledger_commit") {
        $state = "round_done_waiting_ledger_commit"
        $reason = if ($null -ne $LedgerLagRounds -and [int]$LedgerLagRounds -gt 0) { "stdout_done_marker_seen_waiting_for_ledger_commit" } else { "stdout_done_marker_seen_without_ledger_commit_evidence" }
        if ($null -ne $stdoutAge -and [int]$stdoutAge -le 300) {
            $ok = $true
        }
        $nextStep = "wait for ledger commit for the round marked done in stdout"
    } elseif ($LogSummary.latest_round_state -eq "completed") {
        if ($null -ne $LedgerLagRounds -and [int]$LedgerLagRounds -gt 0) {
            $state = "ledger_lag_after_completion"
            $reason = "latest_round_completed_but_ledger_lag_remains"
            $nextStep = "inspect ledger append and report gate output"
        } else {
            $state = "idle_completed"
            $ok = $true
            $reason = "latest_round_completed_and_ledger_current"
            $nextStep = "wait for next interval or inspect latest ledger round"
        }
    } elseif ($null -ne $LogSummary.latest_round) {
        $state = "round_state_unknown"
        $reason = "latest_round_seen_without_clear_state"
        $nextStep = "inspect round_log_preview"
    }

    return [pscustomobject][ordered]@{
        state = $state
        ok = $ok
        reason = $reason
        next_step = $nextStep
        stdout_age_seconds = $stdoutAge
        ledger_age_seconds = $ledgerAge
        ledger_lag_rounds = $LedgerLagRounds
        backend_busy_checked = if ($null -ne $BackendBusyStatus) { $BackendBusyStatus.checked } else { $false }
        backend_busy = if ($null -ne $BackendBusyStatus) { $BackendBusyStatus.busy } else { $false }
        backend_active_engine_requests = if ($null -ne $BackendBusyStatus) { $BackendBusyStatus.active_engine_requests } else { 0 }
        backend_active_endpoints = if ($null -ne $BackendBusyStatus) { $BackendBusyStatus.active_endpoints } else { @() }
        max_round_timeout_seconds = $RoundTimeoutSecs
    }
}

function Format-DaemonOperatorSummary {
    param(
        [object]$Activity,
        [object]$LogSummary,
        [object]$StdoutFreshness,
        [object]$LedgerFreshness,
        [object]$ActiveRound,
        [object]$LedgerLatestRound,
        [object]$LedgerLagRounds
    )

    $stage = [string]$LogSummary.latest_stage
    if ($stage.Trim().Length -eq 0) {
        $stage = [string]$LogSummary.latest_event
    }
    $stdoutAge = if ($null -ne $StdoutFreshness.age_seconds) { "$($StdoutFreshness.age_seconds)s" } else { "unknown" }
    $ledgerAge = if ($null -ne $LedgerFreshness.age_seconds) { "$($LedgerFreshness.age_seconds)s" } else { "unknown" }
    $active = if ($null -ne $ActiveRound) { [string]$ActiveRound } else { "unknown" }
    $ledger = if ($null -ne $LedgerLatestRound) { [string]$LedgerLatestRound } else { "unknown" }
    $lag = if ($null -ne $LedgerLagRounds) { [string]$LedgerLagRounds } else { "unknown" }

    return "state=$($Activity.state) ok=$($Activity.ok) reason=$($Activity.reason) active_round=$active ledger_round=$ledger lag=$lag stage=$stage stdout_age=$stdoutAge ledger_age=$ledgerAge next_step=$($Activity.next_step)"
}

function Get-DaemonTransitionKind {
    param(
        [string]$ActivityState,
        [string]$LatestRoundState
    )

    if (($ActivityState -eq "not_running" -or $ActivityState -eq "stale_pid") -and $LatestRoundState -eq "in_progress") {
        return "restartable_stale_round"
    }
    if (($ActivityState -eq "active" -or $ActivityState -eq "slow_in_progress") -and $LatestRoundState -eq "in_progress") {
        return "normal_in_progress"
    }
    if ($ActivityState -eq "round_done_waiting_ledger_commit") {
        return "round_done_waiting_ledger_commit"
    }
    if ($ActivityState.StartsWith("stale") -or $ActivityState -eq "in_progress_no_stdout") {
        return "stale_no_activity"
    }
    return $ActivityState
}

function New-DaemonRoundTransitionStatus {
    param(
        [object]$Activity,
        [object]$LogSummary,
        [object]$ActiveRound,
        [object]$LedgerLatestRound,
        [object]$LedgerLagRounds,
        [object]$StdoutFreshness,
        [object]$LedgerFreshness
    )

    return [pscustomobject][ordered]@{
        schema = "daemon_round_transition_status_v1"
        transition_kind = Get-DaemonTransitionKind -ActivityState $Activity.state -LatestRoundState $LogSummary.latest_round_state
        activity_state = $Activity.state
        activity_ok = $Activity.ok
        activity_reason = $Activity.reason
        active_round = $ActiveRound
        ledger_latest_round = $LedgerLatestRound
        ledger_lag_rounds = $LedgerLagRounds
        latest_round_state = $LogSummary.latest_round_state
        latest_done_round = $LogSummary.latest_done_round
        round_in_progress = $LogSummary.round_in_progress
        stdout_age_seconds = $StdoutFreshness.age_seconds
        ledger_age_seconds = $LedgerFreshness.age_seconds
        max_in_progress_stdout_age_seconds = 300
        max_round_timeout_seconds = $Activity.max_round_timeout_seconds
        backend_busy_checked = $Activity.backend_busy_checked
        backend_busy = $Activity.backend_busy
        backend_active_engine_requests = $Activity.backend_active_engine_requests
        backend_active_endpoints = $Activity.backend_active_endpoints
        max_idle_ledger_age_seconds = $null
        read_only = $true
        starts_process = $false
        sends_prompt = $false
    }
}

function New-EvolutionGoalItem {
    param(
        [string]$Kind,
        [string]$IdSuffix,
        [string]$Trigger,
        [int]$Priority,
        [object]$SourceRound,
        [string]$Action,
        [string]$Reason
    )

    $roundText = if ($null -ne $SourceRound) { "r$SourceRound" } else { "unknown" }
    return [pscustomobject][ordered]@{
        goal_id = "evolution-goal-$roundText-$IdSuffix"
        kind = $Kind
        trigger = $Trigger
        priority = $Priority
        source_round = $SourceRound
        action = $Action
        reason = if ($Reason.Length -gt 220) { $Reason.Substring(0, 220) } else { $Reason }
        ready_for_next_round = $true
        releases_repair_factor = $Kind -eq "repair" -or $Kind -eq "splice"
        relabel_required = $Kind -eq "relabel"
    }
}

function New-EvolutionGoalQueue {
    param(
        [string]$Source,
        [object]$Activity,
        [object]$ActiveRound,
        [object]$LedgerLagRounds,
        [bool]$ValidationExecutionOk = $true
    )

    $goals = @()
    $activityState = if ($null -ne $Activity) { [string]$Activity.state } else { "" }
    $activityReason = if ($null -ne $Activity) { [string]$Activity.reason } else { "" }

    if ($activityState.StartsWith("stale") -or $activityState -eq "stale_pid" -or $activityState -eq "in_progress_no_stdout") {
        $goals += New-EvolutionGoalItem -Kind "repair" -IdSuffix "daemon-stale" -Trigger "daemon_activity" -Priority 100 -SourceRound $ActiveRound -Action "release_repair_factor_for_stale_daemon_round" -Reason $activityReason
    }
    if ($activityState -eq "ledger_lag_after_completion") {
        $goals += New-EvolutionGoalItem -Kind "splice" -IdSuffix "ledger-lag-after-completion" -Trigger "daemon_activity" -Priority 94 -SourceRound $ActiveRound -Action "splice_completed_round_terminal_evidence_into_ledger" -Reason $activityReason
    }
    if ($activityState -eq "round_done_waiting_ledger_commit" -or ($null -ne $LedgerLagRounds -and [int]$LedgerLagRounds -gt 0 -and $activityState -notmatch "^slow")) {
        $goals += New-EvolutionGoalItem -Kind "splice" -IdSuffix "done-waiting-ledger" -Trigger "daemon_activity" -Priority 90 -SourceRound $ActiveRound -Action "splice_done_marker_to_ledger_commit_repair" -Reason $activityReason
    }
    if ($activityState -eq "slow_in_progress" -or $activityState -eq "slow_pre_round_activity") {
        $goals += New-EvolutionGoalItem -Kind "fallback" -IdSuffix "slow-daemon" -Trigger "daemon_activity" -Priority 82 -SourceRound $ActiveRound -Action "fallback_runtime_or_pool_route_before_next_round" -Reason $activityReason
    }
    if (-not $ValidationExecutionOk) {
        $goals += New-EvolutionGoalItem -Kind "relabel" -IdSuffix "validation-execution-missing" -Trigger "daemon_validation" -Priority 76 -SourceRound $ActiveRound -Action "relabel_validation_gate_to_repair_target" -Reason "daemon launch command does not enforce validation execution"
    }

    $goalKinds = @($goals | ForEach-Object { [string]$_.kind } | Sort-Object -Unique)
    return [pscustomobject][ordered]@{
        schema = "evolution_goal_queue_v1"
        source = $Source
        read_only = $true
        report_only = $true
        side_effects = $false
        starts_process = $false
        sends_prompt = $false
        queue_len = [int]$goals.Count
        executable_goal_count = [int]@($goals | Where-Object { $_.ready_for_next_round -eq $true }).Count
        goal_kinds = @($goalKinds)
        goals = @($goals)
    }
}

function Daemon-Status {
    param(
        [string]$PidFile,
        [string]$LedgerPath,
        [string]$ReportPath,
        [string]$RemoteChainStatusPath,
        [string]$ModelCacheStatusPath,
        [string]$LogPath,
        [string]$ErrPath,
        [bool]$RequireValidationExecution = $false
    )

    $process = Get-LiveProcess -PidFile $PidFile
    $running = $null -ne $process
    $pidValue = Read-PidFileValue -PidFile $PidFile
    $pidFileExists = Test-Path -LiteralPath $PidFile -PathType Leaf
    $stalePidFile = $pidFileExists -and $null -ne $pidValue -and -not $running
    $orphanProcesses = @()
    if (-not $running) {
        $stalePidValue = 0
        if ($null -ne $pidValue) {
            $stalePidValue = [int]$pidValue
        }
        $orphanProcesses = @(Find-DaemonOrphanProcesses -LedgerPath $LedgerPath -StalePid $stalePidValue)
        if ($orphanProcesses.Count -eq 0 -and $stalePidValue -gt 0) {
            $orphanProcesses = @(Find-DaemonOrphanProcesses -LedgerPath $LedgerPath)
        }
        if ($orphanProcesses.Count -gt 0) {
            $running = $true
        }
    }
    $adoptedOrphan = $orphanProcesses.Count -gt 0
    $adoptedPid = $null
    if ($adoptedOrphan) {
        $preferred = Select-DaemonAdoptionProcess -Processes $orphanProcesses
        if ($null -ne $preferred) {
            $adoptedPid = [int]$preferred.ProcessId
        }
    }
    $orphanPids = @($orphanProcesses | ForEach-Object { [int]$_.ProcessId } | Sort-Object -Unique)
    $stdoutTail = Read-LogTail -Path $LogPath
    $stdoutProgressTail = Read-LogTail -Path $LogPath -Count 240
    $stderrTail = Read-LogTail -Path $ErrPath
    $logSummary = Get-LogProgressSummary -StdoutTail $stdoutProgressTail -StderrTail $stderrTail
    $launchValidation = Get-LaunchValidationSummary -StderrTail $stderrTail
    $ledgerLatestRound = Read-LedgerLatestRound -Path $LedgerPath
    $activeRound = $logSummary.latest_round
    $ledgerLagRounds = $null
    if ($null -ne $activeRound -and $null -ne $ledgerLatestRound) {
        $ledgerLagRounds = [Math]::Max(0, [int]$activeRound - [int]$ledgerLatestRound)
    }
    $stdoutFreshness = Get-FileFreshness -Path $LogPath
    $stderrFreshness = Get-FileFreshness -Path $ErrPath
    $ledgerFreshness = Get-FileFreshness -Path $LedgerPath
    $backendBusyStatus = $null
    if ((-not $SkipBackend) -and $running -and $logSummary.round_in_progress -eq $true -and $null -ne $stdoutFreshness.age_seconds -and [int]$stdoutFreshness.age_seconds -gt 300) {
        $backendBusyStatus = Get-BackendBusyStatus -Backend $Backend
    }
    $activity = Get-DaemonActivitySummary -Running $running -StalePidFile $stalePidFile -AdoptedOrphan $adoptedOrphan -LogSummary $logSummary -StdoutFreshness $stdoutFreshness -LedgerFreshness $ledgerFreshness -LedgerLagRounds $ledgerLagRounds -BackendBusyStatus $backendBusyStatus -RoundTimeoutSecs $TimeoutSecs
    $operatorSummary = Format-DaemonOperatorSummary -Activity $activity -LogSummary $logSummary -StdoutFreshness $stdoutFreshness -LedgerFreshness $ledgerFreshness -ActiveRound $activeRound -LedgerLatestRound $ledgerLatestRound -LedgerLagRounds $ledgerLagRounds
    $transitionStatus = New-DaemonRoundTransitionStatus -Activity $activity -LogSummary $logSummary -ActiveRound $activeRound -LedgerLatestRound $ledgerLatestRound -LedgerLagRounds $ledgerLagRounds -StdoutFreshness $stdoutFreshness -LedgerFreshness $ledgerFreshness
    $validationExecutionOk = (-not $RequireValidationExecution) -or $launchValidation.validation_execution_enforced
    $evolutionGoalQueue = New-EvolutionGoalQueue -Source "daemon" -Activity $activity -ActiveRound $activeRound -LedgerLagRounds $ledgerLagRounds -ValidationExecutionOk:$validationExecutionOk
    return [pscustomobject][ordered]@{
        schema_version = 1
        contract_version = "smartsteam.evolution-loop.daemon.v1"
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        repo = $RepoRoot
        running = $running
        pid = if ($null -ne $process) { $process.Id } elseif ($adoptedOrphan) { $adoptedPid } else { $null }
        pid_source = if ($null -ne $process) { "pid_file" } elseif ($adoptedOrphan) { "adopted_orphan_ledger_match" } else { "none" }
        pid_file_exists = $pidFileExists
        stale_pid_file = $stalePidFile
        stale_pid = if ($stalePidFile) { $pidValue } else { $null }
        adopted_orphan_process_count = $orphanProcesses.Count
        adopted_orphan_pids = $orphanPids
        pid_file = $PidFile
        ledger = $LedgerPath
        ledger_latest_round = $ledgerLatestRound
        active_round = $activeRound
        ledger_lag_rounds = $ledgerLagRounds
        report_json = $ReportPath
        remote_chain_status_json = $RemoteChainStatusPath
        model_cache_status_json = $ModelCacheStatusPath
        stdout_log = $LogPath
        stderr_log = $ErrPath
        stdout_freshness = $stdoutFreshness
        stderr_freshness = $stderrFreshness
        ledger_freshness = $ledgerFreshness
        activity = $activity
        daemon_round_transition_status = $transitionStatus
        operator_summary = $operatorSummary
        last_stop_reason = Read-LastStopReason -Path $LogPath
        validation_execution_required = $RequireValidationExecution
        validation_execution_ok = $validationExecutionOk
        validation_execution_failure = if ($RequireValidationExecution -and -not $launchValidation.validation_execution_enforced) { "daemon launch command does not enforce validation execution" } else { "" }
        evolution_goal_queue_v1 = $evolutionGoalQueue
        launch_validation = $launchValidation
        log_summary = $logSummary
        stdout_tail = $stdoutTail
        stderr_tail = $stderrTail
    }
}

function Quote-CommandArgument {
    param([object]$Value)

    $text = [string]$Value
    if ($text.Length -eq 0) {
        return '""'
    }
    if ($text -notmatch '[\s"]') {
        return $text
    }
    return '"' + ($text -replace '"', '\"') + '"'
}

function Quote-PowerShellLiteral {
    param([object]$Value)

    return "'" + (([string]$Value) -replace "'", "''") + "'"
}

function Quote-WindowsCommandArgument {
    param([object]$Value)

    $text = [string]$Value
    return '"' + ($text -replace '"', '\"') + '"'
}

function Quote-CmdPathArgument {
    param([object]$Value)

    $text = [string]$Value
    return '"' + ($text -replace '"', '""') + '"'
}

function Test-ProcessIdRunning {
    param([object]$PidValue)

    if ($null -eq $PidValue) {
        return $false
    }
    try {
        $pidNumber = [int64]$PidValue
        if ($pidNumber -le 0 -or $pidNumber -gt [int64][int]::MaxValue) {
            return $false
        }
        $process = Get-Process -Id ([int]$pidNumber) -ErrorAction Stop
        return $null -ne $process
    } catch {
        return $false
    }
}

function Remove-StalePoolLeases {
    param([string]$LeaseDir)

    $removed = @()
    if ($LeaseDir.Trim().Length -eq 0 -or -not (Test-Path -LiteralPath $LeaseDir -PathType Container)) {
        return [pscustomobject][ordered]@{
            removed_count = 0
            removed = @()
        }
    }

    $nowUnix = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    foreach ($lease in @(Get-ChildItem -LiteralPath $LeaseDir -Filter "*.lease.json" -File -ErrorAction SilentlyContinue)) {
        try {
            $body = Get-Content -LiteralPath $lease.FullName -Raw
            $json = $body | ConvertFrom-Json
        } catch {
            continue
        }

        $ownerPid = 0
        if ($json.PSObject.Properties.Name -contains "owner_pid") {
            try {
                $ownerPid = [int64]$json.owner_pid
            } catch {
                $ownerPid = 0
            }
        }
        $expiresUnix = 0
        if ($json.PSObject.Properties.Name -contains "expires_unix") {
            try {
                $expiresUnix = [int64]$json.expires_unix
            } catch {
                $expiresUnix = 0
            }
        }

        $reason = ""
        if ($ownerPid -gt 0 -and -not (Test-ProcessIdRunning -PidValue $ownerPid)) {
            $reason = "stale_owner"
        } elseif ($expiresUnix -gt 0 -and $expiresUnix -le $nowUnix) {
            $reason = "expired"
        }

        if ($reason.Trim().Length -eq 0) {
            continue
        }

        try {
            Remove-Item -LiteralPath $lease.FullName -Force
            $removed += [pscustomobject][ordered]@{
                path = $lease.FullName
                owner_pid = $ownerPid
                expires_unix = $expiresUnix
                reason = $reason
            }
        } catch {
            $removed += [pscustomobject][ordered]@{
                path = $lease.FullName
                owner_pid = $ownerPid
                expires_unix = $expiresUnix
                reason = "remove_failed:$($_.Exception.Message)"
            }
        }
    }

    return [pscustomobject][ordered]@{
        removed_count = $removed.Count
        removed = @($removed)
    }
}

function Start-DetachedDaemonProcess {
    param(
        [string]$PowerShellExe,
        [string]$LaunchScript,
        [string]$RepoRoot,
        [string]$StdoutLog,
        [string]$StderrLog
    )

    $daemonCommand = @(
        (Quote-CmdPathArgument $PowerShellExe),
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", (Quote-CmdPathArgument $LaunchScript)
    ) -join " "
    $cmdPayload = "$daemonCommand 1> $(Quote-CmdPathArgument $StdoutLog) 2> $(Quote-CmdPathArgument $StderrLog)"
    $cmdLine = "cmd.exe /d /s /c `"$cmdPayload`""

    try {
        $startup = $null
        try {
            $startup = ([wmiclass]"Win32_ProcessStartup").CreateInstance()
            $startup.ShowWindow = 0
        } catch {
            $startup = $null
        }

        $arguments = @{
            CommandLine = $cmdLine
            CurrentDirectory = $RepoRoot
        }
        if ($null -ne $startup) {
            $arguments.ProcessStartupInformation = $startup
        }
        $result = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments $arguments
        if ($null -ne $result -and [int]$result.ReturnValue -eq 0 -and [int]$result.ProcessId -gt 0) {
            return [pscustomobject][ordered]@{
                process = Get-Process -Id ([int]$result.ProcessId) -ErrorAction SilentlyContinue
                pid = [int]$result.ProcessId
                launch_mode = "cim_cmd_redirect"
                command_line = $cmdLine
                error = ""
            }
        }
        $returnValue = if ($null -ne $result) { [string]$result.ReturnValue } else { "null_result" }
        $cimError = "Win32_Process.Create returned $returnValue"
    } catch {
        $cimError = $_.Exception.Message
    }

    $fallback = Start-Process `
        -FilePath $PowerShellExe `
        -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $LaunchScript) `
        -WorkingDirectory $RepoRoot `
        -WindowStyle Hidden `
        -RedirectStandardOutput $StdoutLog `
        -RedirectStandardError $StderrLog `
        -PassThru
    return [pscustomobject][ordered]@{
        process = $fallback
        pid = [int]$fallback.Id
        launch_mode = "start_process_redirect"
        command_line = "$PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $LaunchScript"
        error = $cimError
    }
}

function Get-NestedValue {
    param(
        [object]$Value,
        [string[]]$Path
    )

    $cursor = $Value
    foreach ($segment in $Path) {
        if ($null -eq $cursor) {
            return $null
        }
        $property = $cursor.PSObject.Properties[$segment]
        if ($null -eq $property) {
            return $null
        }
        $cursor = $property.Value
    }
    return $cursor
}

function Convert-ToPositiveInt {
    param([object]$Value)

    if ($null -eq $Value) {
        return 0
    }
    try {
        $number = [int64]$Value
        if ($number -gt 0 -and $number -le [int64][int]::MaxValue) {
            return [int]$number
        }
    } catch {
        return 0
    }
    return 0
}

function Read-JsonFile {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $null
    }
    try {
        return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    } catch {
        return $null
    }
}

function Normalize-BackendBaseUri {
    param([string]$Backend)

    if ([string]::IsNullOrWhiteSpace($Backend)) {
        return ""
    }
    $base = $Backend.Trim()
    if (-not ($base.StartsWith("http://") -or $base.StartsWith("https://"))) {
        $base = "http://$base"
    }
    return $base.TrimEnd("/")
}

function Get-QualityWorkerContext {
    param([object]$Status)

    $workers = Get-NestedValue -Value $Status -Path @("model_pool", "workers")
    if ($null -eq $workers) {
        $workers = Get-NestedValue -Value $Status -Path @("workers", "items")
    }
    if ($null -eq $workers) {
        return 0
    }
    foreach ($worker in @($workers)) {
        $role = [string](Get-NestedValue -Value $worker -Path @("role"))
        if ($role.Trim().ToLowerInvariant() -ne "quality") {
            continue
        }
        $context = Convert-ToPositiveInt (Get-NestedValue -Value $worker -Path @("context_window"))
        if ($context -le 0) {
            $context = Convert-ToPositiveInt (Get-NestedValue -Value $worker -Path @("default_context_tokens"))
        }
        if ($context -gt 0) {
            return $context
        }
    }
    return 0
}

function Get-BackendModelPoolContext {
    param([string]$Backend)

    $base = Normalize-BackendBaseUri -Backend $Backend
    if ($base.Trim().Length -eq 0) {
        return 0
    }
    try {
        $status = Invoke-RestMethod -Uri "$base/v1/model-pool/status" -TimeoutSec 5
    } catch {
        return 0
    }
    foreach ($path in @(
        @("quality_context_tokens"),
        @("quality_context_required_tokens")
    )) {
        $context = Convert-ToPositiveInt (Get-NestedValue -Value $status -Path $path)
        if ($context -gt 0) {
            return $context
        }
    }
    return Get-QualityWorkerContext -Status $status
}

function Get-BackendRuntimeContext {
    param([string]$Backend)

    $base = Normalize-BackendBaseUri -Backend $Backend
    if ($base.Trim().Length -eq 0) {
        return 0
    }
    $uri = "$base/health"
    try {
        $health = Invoke-RestMethod -Uri $uri -TimeoutSec 8
    } catch {
        return 0
    }
    return Convert-ToPositiveInt (Get-NestedValue -Value $health -Path @("gemma_runtime_context_window"))
}

function Get-BackendBusyStatus {
    param([string]$Backend)

    $base = Normalize-BackendBaseUri -Backend $Backend
    if ($base.Trim().Length -eq 0) {
        return [pscustomobject][ordered]@{
            checked = $false
            busy = $false
            active_engine_requests = 0
            active_business_cycle_stream = $false
            active_request_ids = @()
            active_endpoints = @()
            error = "backend_empty"
        }
    }

    try {
        $health = Invoke-RestMethod -Uri "$base/health" -TimeoutSec 8
    } catch {
        return [pscustomobject][ordered]@{
            checked = $true
            busy = $false
            active_engine_requests = 0
            active_business_cycle_stream = $false
            active_request_ids = @()
            active_endpoints = @()
            error = $_.Exception.Message
        }
    }

    $activeCount = Convert-ToPositiveInt (Get-NestedValue -Value $health -Path @("active_engine_requests"))
    $requestIds = @()
    $endpoints = @()
    foreach ($request in @($health.active_requests)) {
        $requestId = Get-NestedValue -Value $request -Path @("request_id")
        if ($null -ne $requestId) {
            $requestIds += [string]$requestId
        }
        $endpoint = Get-NestedValue -Value $request -Path @("endpoint")
        if ($null -ne $endpoint -and ([string]$endpoint).Trim().Length -gt 0) {
            $endpoints += [string]$endpoint
        }
    }
    $uniqueEndpoints = @($endpoints | Sort-Object -Unique)

    return [pscustomobject][ordered]@{
        checked = $true
        busy = [bool]($activeCount -gt 0 -or [bool](Get-NestedValue -Value $health -Path @("engine_busy")))
        active_engine_requests = $activeCount
        active_business_cycle_stream = @($uniqueEndpoints | Where-Object { $_ -eq "business-cycle-stream" }).Count -gt 0
        active_request_ids = @($requestIds | Sort-Object -Unique)
        active_endpoints = $uniqueEndpoints
        error = ""
    }
}

function Resolve-MinRuntimeContext {
    param(
        [int]$ExplicitMinRuntimeContext,
        [string]$Backend,
        [string]$RemoteChainStatusPath
    )

    if ($ExplicitMinRuntimeContext -gt 0) {
        return [pscustomobject]@{
            Value = $ExplicitMinRuntimeContext
            Source = "explicit"
        }
    }

    $backendModelPoolContext = Get-BackendModelPoolContext -Backend $Backend
    if ($backendModelPoolContext -gt 0) {
        return [pscustomobject]@{
            Value = $backendModelPoolContext
            Source = "backend_model_pool_status"
        }
    }

    $backendContext = Get-BackendRuntimeContext -Backend $Backend
    if ($backendContext -gt 0) {
        return [pscustomobject]@{
            Value = $backendContext
            Source = "backend_health"
        }
    }

    $generatedManifestPath = Join-Path $RepoRoot "target\gemma-chain\apple-model-pool.generated.json"
    $generatedManifest = Read-JsonFile -Path $generatedManifestPath
    foreach ($path in @(
        @("capacity_policy", "quality_required_context_tokens"),
        @("model_pool_route", "quality_context_required_tokens")
    )) {
        $context = Convert-ToPositiveInt (Get-NestedValue -Value $generatedManifest -Path $path)
        if ($context -gt 0) {
            return [pscustomobject]@{
                Value = $context
                Source = "generated_manifest"
            }
        }
    }
    $workerContext = Get-QualityWorkerContext -Status $generatedManifest
    if ($workerContext -gt 0) {
        return [pscustomobject]@{
            Value = $workerContext
            Source = "generated_manifest_quality_worker"
        }
    }

    $remoteStatus = Read-JsonFile -Path $RemoteChainStatusPath
    foreach ($path in @(
        @("model_pool_route", "quality_context_required_tokens"),
        @("model_pool_route", "quality_context_tokens")
    )) {
        $context = Convert-ToPositiveInt (Get-NestedValue -Value $remoteStatus -Path $path)
        if ($context -gt 0) {
            return [pscustomobject]@{
                Value = $context
                Source = "remote_chain_status"
            }
        }
    }
    $remoteWorkerContext = Get-QualityWorkerContext -Status $remoteStatus
    if ($remoteWorkerContext -gt 0) {
        return [pscustomobject]@{
            Value = $remoteWorkerContext
            Source = "remote_chain_status_quality_worker"
        }
    }

    return [pscustomobject]@{
        Value = 262144
        Source = "fallback"
    }
}

$WorkDirPath = Resolve-RepoPath $WorkDir
if ($Ledger.Trim().Length -eq 0) {
    $Ledger = Join-Path $WorkDir "evolution-ledger.jsonl"
}
if ($ReportJson.Trim().Length -eq 0) {
    $ReportJson = Join-Path $WorkDir "report.json"
}
if ($RemoteChainStatusJson.Trim().Length -eq 0) {
    $RemoteChainStatusJson = "target\remote-gemma-chain\status-with-model-cache.json"
}
if ($ModelCacheStatusJson.Trim().Length -eq 0) {
    $ModelCacheStatusJson = "target\remote-gemma-chain\model-cache-status.json"
}
$LedgerPath = Resolve-RepoPath $Ledger
$ReportPath = Resolve-RepoPath $ReportJson
$RemoteChainStatusPath = Resolve-RepoPath $RemoteChainStatusJson
$ModelCacheStatusPath = Resolve-RepoPath $ModelCacheStatusJson
$PidFile = Join-Path $WorkDirPath "evolution-loop.pid"
$StdoutLog = Join-Path $WorkDirPath "evolution-loop.out.log"
$StderrLog = Join-Path $WorkDirPath "evolution-loop.err.log"
$DaemonScriptPath = $MyInvocation.MyCommand.Path
$StartScript = Join-Path $ScriptDir "start-evolution-loop.ps1"
$StatusScript = Join-Path $ScriptDir "status-evolution-loop.ps1"
$PowerShellExe = "powershell.exe"
$PwshCommand = Get-Command "pwsh.exe" -ErrorAction SilentlyContinue
if ($null -ne $PwshCommand -and -not [string]::IsNullOrWhiteSpace([string]$PwshCommand.Source)) {
    $PowerShellExe = [string]$PwshCommand.Source
}
$ResolvedMinRuntimeContext = Resolve-MinRuntimeContext -ExplicitMinRuntimeContext $MinRuntimeContext -Backend $Backend -RemoteChainStatusPath $RemoteChainStatusPath

function Get-DaemonStatusViaSelf {
    param(
        [string]$ScriptPath,
        [string]$PowerShellExe,
        [string]$PidWorkDir,
        [string]$LedgerPath,
        [string]$ReportPath,
        [string]$Backend,
        [string]$RemoteChainStatusPath,
        [string]$ModelCacheStatusPath,
        [int]$MinRuntimeContext
    )

    $args = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $ScriptPath,
        "-JsonStatus",
        "-Backend", $Backend,
        "-Ledger", $LedgerPath,
        "-ReportJson", $ReportPath,
        "-WorkDir", $PidWorkDir,
        "-RemoteChainStatusJson", $RemoteChainStatusPath,
        "-ModelCacheStatusJson", $ModelCacheStatusPath,
        "-MinRuntimeContext", $MinRuntimeContext,
        "-SkipBackend",
        "-SkipRemoteChain"
    )
    try {
        $raw = & $PowerShellExe @args 2>$null
        $text = ($raw | Out-String).Trim()
        if ($text.Length -eq 0) {
            return $null
        }
        $status = $text | ConvertFrom-Json
        return $status.daemon
    } catch {
        return $null
    }
}

if ($Status -or $JsonStatus) {
    $daemon = Daemon-Status -PidFile $PidFile -LedgerPath $LedgerPath -ReportPath $ReportPath -RemoteChainStatusPath $RemoteChainStatusPath -ModelCacheStatusPath $ModelCacheStatusPath -LogPath $StdoutLog -ErrPath $StderrLog -RequireValidationExecution:$RequireValidationExecutionEffective
    $exitCode = if ($FailOnUnhealthy -and ((-not $daemon.activity.ok) -or (-not $daemon.validation_execution_ok))) { 2 } else { 0 }
    if ($JsonStatus) {
        $statusArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $StatusScript, "-Ledger", $LedgerPath, "-ReportJson", $ReportPath, "-Backend", $Backend, "-RemoteChainStatusJson", $RemoteChainStatusPath, "-DaemonWorkDir", $WorkDirPath, "-JsonStatus", "-SkipProcess")
        if ($StrictUnattendedEvolution) {
            $statusArgs += "-StrictUnattendedEvolution"
        }
        if ($SkipBackend) {
            $statusArgs += "-SkipBackend"
        }
        if ($SkipRemoteChain) {
            $statusArgs += "-SkipRemoteChain"
        }
        if ($FailOnUnhealthy) {
            $statusArgs += "-FailOnNotReady"
        }
        $loopStatus = & powershell.exe @statusArgs 2>$null
        $loopExitCode = $LASTEXITCODE
        $loop = $null
        if (($loopStatus | Out-String).Trim().Length -gt 0) {
            $loop = ($loopStatus | Out-String | ConvertFrom-Json)
        }
        if ($FailOnUnhealthy -and $loopExitCode -ne 0) {
            $exitCode = 2
        }
        [pscustomobject][ordered]@{
            daemon = $daemon
            loop = $loop
        } | ConvertTo-Json -Depth 10
    } else {
        Write-Host "SmartSteam evolution-loop daemon"
        Write-Host "read_only=true starts_process=false sends_prompt=false"
        Write-Host "running=$($daemon.running) pid=$($daemon.pid) pid_source=$($daemon.pid_source) stale_pid_file=$($daemon.stale_pid_file) stale_pid=$($daemon.stale_pid) adopted_orphan_count=$($daemon.adopted_orphan_process_count)"
        if ($daemon.adopted_orphan_process_count -gt 0) {
            Write-Host "adopted_orphan_pids=$($daemon.adopted_orphan_pids -join ',')"
        }
        if ($daemon.last_stop_reason.Trim().Length -gt 0) {
            Write-Host "last_stop_reason=$($daemon.last_stop_reason)"
        }
        Write-Host "pid_file=$PidFile"
        Write-Host "ledger=$LedgerPath"
        Write-Host "report_json=$ReportPath"
        Write-Host "remote_chain_status_json=$RemoteChainStatusPath"
        Write-Host "model_cache_status_json=$ModelCacheStatusPath"
        Write-Host "stdout_log=$StdoutLog"
        Write-Host "stderr_log=$StderrLog"
        Write-Host "operator_summary=$($daemon.operator_summary)"
        Write-Host "launch_validation: mode=$($daemon.launch_validation.mode) enforced=$($daemon.launch_validation.validation_execution_enforced) configured_run=$($daemon.launch_validation.require_configured_validation_run) test_gate_run=$($daemon.launch_validation.require_test_gate_validation_run) validation_command=$($daemon.launch_validation.validation_command_present) use_test_gate_command=$($daemon.launch_validation.use_test_gate_validation_command) next_step=$($daemon.launch_validation.next_step)"
        if ($daemon.validation_execution_required) {
            Write-Host "validation_execution_gate: required=$($daemon.validation_execution_required) ok=$($daemon.validation_execution_ok) failure=$($daemon.validation_execution_failure)"
        }
        Write-Host "log_summary: stdout_readable=$($daemon.log_summary.stdout_readable) stderr_readable=$($daemon.log_summary.stderr_readable) latest_round=$($daemon.log_summary.latest_round) latest_event=$($daemon.log_summary.latest_event) latest_stage=$($daemon.log_summary.latest_stage) latest_completed_round=$($daemon.log_summary.latest_completed_round) latest_round_state=$($daemon.log_summary.latest_round_state) round_in_progress=$($daemon.log_summary.round_in_progress)"
        Write-Host "ledger_progress: active_round=$($daemon.active_round) ledger_latest_round=$($daemon.ledger_latest_round) ledger_lag_rounds=$($daemon.ledger_lag_rounds)"
        Write-Host "freshness: stdout_age_secs=$($daemon.stdout_freshness.age_seconds) ledger_age_secs=$($daemon.ledger_freshness.age_seconds) stderr_age_secs=$($daemon.stderr_freshness.age_seconds)"
        Write-Host "activity: state=$($daemon.activity.state) ok=$($daemon.activity.ok) reason=$($daemon.activity.reason) next_step=$($daemon.activity.next_step)"
        Write-Host "daemon_transition: schema=$($daemon.daemon_round_transition_status.schema) kind=$($daemon.daemon_round_transition_status.transition_kind) state=$($daemon.daemon_round_transition_status.activity_state) ok=$($daemon.daemon_round_transition_status.activity_ok) reason=$($daemon.daemon_round_transition_status.activity_reason) active_round=$($daemon.daemon_round_transition_status.active_round) ledger_round=$($daemon.daemon_round_transition_status.ledger_latest_round) lag=$($daemon.daemon_round_transition_status.ledger_lag_rounds) latest_round_state=$($daemon.daemon_round_transition_status.latest_round_state) round_in_progress=$($daemon.daemon_round_transition_status.round_in_progress)"
        if ($daemon.log_summary.latest_line_preview.Trim().Length -gt 0) {
            Write-Host "log_preview=$($daemon.log_summary.latest_line_preview)"
        }
        if ($daemon.log_summary.latest_round_line_preview.Trim().Length -gt 0) {
            Write-Host "round_log_preview=$($daemon.log_summary.latest_round_line_preview)"
        }
        $statusArgs = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $StatusScript, "-Ledger", $LedgerPath, "-ReportJson", $ReportPath, "-Backend", $Backend, "-RemoteChainStatusJson", $RemoteChainStatusPath, "-DaemonWorkDir", $WorkDirPath, "-SkipProcess")
        if ($StrictUnattendedEvolution) {
            $statusArgs += "-StrictUnattendedEvolution"
        }
        if ($SkipBackend) {
            $statusArgs += "-SkipBackend"
        }
        if ($SkipRemoteChain) {
            $statusArgs += "-SkipRemoteChain"
        }
        if ($FailOnUnhealthy) {
            $statusArgs += "-FailOnNotReady"
        }
        & powershell.exe @statusArgs
        if ($FailOnUnhealthy -and $LASTEXITCODE -ne 0) {
            $exitCode = 2
        }
    }
    exit $exitCode
}

if ($Stop) {
    $process = Get-LiveProcess -PidFile $PidFile
    if ($null -eq $process) {
        $stalePid = Read-PidFileValue -PidFile $PidFile
        $stalePidValue = 0
        if ($null -ne $stalePid) {
            $stalePidValue = [int]$stalePid
        }
        $orphans = @(Find-DaemonOrphanProcesses -LedgerPath $LedgerPath -StalePid $stalePidValue)
        $orphanPids = (@($orphans | ForEach-Object { [int]$_.ProcessId } | Sort-Object -Unique) -join ",")
        if ($CheckOnly) {
            Write-Host "check_only=true"
            Write-Host "would_stop_pid="
            Write-Host "stale_pid=$stalePid"
            Write-Host "would_stop_orphan_count=$($orphans.Count)"
            Write-Host "would_stop_orphan_pids=$orphanPids"
            exit 0
        }
        if ($orphans.Count -gt 0) {
            Stop-DaemonMatchedProcesses -Processes $orphans
            if (Test-Path -LiteralPath $PidFile -PathType Leaf) {
                Remove-Item -LiteralPath $PidFile -Force
            }
            Write-Host "daemon_stop: stopped_orphans count=$($orphans.Count) pids=$orphanPids stale_pid=$stalePid"
            exit 0
        }
        if (Test-Path -LiteralPath $PidFile -PathType Leaf) {
            Remove-Item -LiteralPath $PidFile -Force
        }
        Write-Host "daemon_stop: not_running"
        exit 0
    }
    $descendants = @(Get-ChildProcessTree -RootPid $process.Id)
    $treePids = Format-ProcessTreePids -RootPid $process.Id -Descendants $descendants
    if ($CheckOnly) {
        Write-Host "check_only=true"
        Write-Host "would_stop_pid=$($process.Id)"
        Write-Host "would_stop_descendant_count=$($descendants.Count)"
        Write-Host "would_stop_tree_pids=$treePids"
        exit 0
    }
    Stop-DaemonProcessTree -RootProcess $process -Descendants $descendants
    Remove-Item -LiteralPath $PidFile -Force -ErrorAction SilentlyContinue
    Write-Host "daemon_stop: stopped pid=$($process.Id) descendants=$($descendants.Count) tree_pids=$treePids"
    exit 0
}

if ($Start) {
    New-Item -ItemType Directory -Force -Path $WorkDirPath | Out-Null
    $PoolLeaseDirPath = Join-Path $WorkDirPath "pool-leases"
    $existing = Get-LiveProcess -PidFile $PidFile
    $existingOrphans = @()
    $existingAdoptionProcess = $null
    $existingAdoptionPid = $null
    $daemonBeforeStart = $null
    if ($null -eq $existing) {
        $stalePid = Read-PidFileValue -PidFile $PidFile
        $stalePidValue = 0
        if ($null -ne $stalePid) {
            $stalePidValue = [int]$stalePid
        }
        $existingOrphans = @(Find-DaemonOrphanProcesses -LedgerPath $LedgerPath -StalePid $stalePidValue)
        if ($existingOrphans.Count -eq 0 -and $stalePidValue -gt 0) {
            $existingOrphans = @(Find-DaemonOrphanProcesses -LedgerPath $LedgerPath)
        }
        if ($existingOrphans.Count -eq 0) {
            $daemonBeforeStart = Daemon-Status -PidFile $PidFile -LedgerPath $LedgerPath -ReportPath $ReportPath -RemoteChainStatusPath $RemoteChainStatusPath -ModelCacheStatusPath $ModelCacheStatusPath -LogPath $StdoutLog -ErrPath $StderrLog -RequireValidationExecution:$RequireValidationExecutionEffective
            if ($daemonBeforeStart.adopted_orphan_process_count -gt 0) {
                $existingOrphans = @($daemonBeforeStart.adopted_orphan_pids)
                $existingAdoptionPid = $daemonBeforeStart.pid
            }
        }
        if ($existingOrphans.Count -eq 0) {
            $daemonBeforeStart = Get-DaemonStatusViaSelf -ScriptPath $DaemonScriptPath -PowerShellExe $PowerShellExe -PidWorkDir $WorkDirPath -LedgerPath $LedgerPath -ReportPath $ReportPath -Backend $Backend -RemoteChainStatusPath $RemoteChainStatusPath -ModelCacheStatusPath $ModelCacheStatusPath -MinRuntimeContext $ResolvedMinRuntimeContext.Value
            if ($null -ne $daemonBeforeStart -and $daemonBeforeStart.adopted_orphan_process_count -gt 0) {
                $existingOrphans = @($daemonBeforeStart.adopted_orphan_pids)
                $existingAdoptionPid = $daemonBeforeStart.pid
            }
        }
        if ($existingOrphans.Count -gt 0 -and $existingOrphans[0].PSObject.Properties.Name -contains "ProcessId") {
            $existingAdoptionProcess = Select-DaemonAdoptionProcess -Processes $existingOrphans
            if ($null -ne $existingAdoptionProcess) {
                $existingAdoptionPid = [int]$existingAdoptionProcess.ProcessId
            }
        }
    }
    $existingOrphanPids = (@($existingOrphans | ForEach-Object {
        if ($_.PSObject.Properties.Name -contains "ProcessId") {
            [int]$_.ProcessId
        } else {
            [int]$_
        }
    } | Sort-Object -Unique) -join ",")
    $staleLeaseCleanup = if ((-not $CheckOnly) -and $null -eq $existing -and $existingOrphans.Count -eq 0) { Remove-StalePoolLeases -LeaseDir $PoolLeaseDirPath } else { [pscustomobject][ordered]@{ removed_count = 0; removed = @() } }
    $backendBusyStatus = if ($null -eq $existing -and $existingOrphans.Count -eq 0) { Get-BackendBusyStatus -Backend $Backend } else { $null }

    $promptText = $Prompt
    if ($promptText.Trim().Length -eq 0) {
        $promptText = "Check the SmartSteam unattended evolution chain. Return one small improvement and one verifiable evidence item."
    }

    $StartArgs = @(
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", $StartScript,
        "-Backend", $Backend,
        "-Forever",
        "-IntervalSecs", $IntervalSecs,
        "-MaxFailures", $MaxFailures,
        "-MaxTokens", $MaxTokens,
        "-MaxTotalTokens", $MaxTotalTokens,
        "-MaxRuntimeSecs", $MaxRuntimeSecs,
        "-MaxNoFeedbackRounds", $MaxNoFeedbackRounds,
        "-TimeoutSecs", $TimeoutSecs,
        "-Ledger", $LedgerPath,
        "-ReportJson", $ReportPath,
        "-PostRunReportGate",
        "-PostRunContinuationGate",
        "-RemoteChainStatusJson", $RemoteChainStatusPath,
        "-ModelCacheStatusJson", $ModelCacheStatusPath,
        "-RemoteChainGate",
        "-RefreshPoolArtifacts",
        "-PoolBudgetFairnessJson", (Join-Path $WorkDirPath "model-pool-budget-fairness.json"),
        "-PoolRouteTaskKind", "quality",
        "-PoolStageRouteTaskKinds", "summary,router,review,index,test-gate",
        "-PoolStageRouteGate",
        "-ExecutePoolStageCalls",
        "-RequirePoolBudgetPolicy",
        "-PoolAlignmentGate",
        "-RequirePoolRoute",
        "-PoolLeaseDir", $PoolLeaseDirPath,
        "-PoolLeaseBusyPolicy", "skip-low-priority",
        "-MinRuntimeContext", $ResolvedMinRuntimeContext.Value,
        "-ExperienceAuditGate",
        "-StateConsistencyGate",
        "-RequireHelperStageRoles", "summary,router,review,index,test-gate",
        "-RequireLatestHelperStageRoles", "summary,router,review,index,test-gate",
        "-RequireUsefulLatestHelperStageFeedback",
        "-RequireCompleteLatestHelperStageFeedback",
        "-RequireCleanHelperStageFeedback",
        "-RequireFinalJsonPoolStageDispatch",
        "-RequireTestGatePass",
        "-RequireSafeTestGateValidationCommand",
        "-Prompt", $promptText
    )
    if ($RefreshRemoteChainStatus -or (-not $SkipRemoteChain)) {
        $StartArgs += "-RefreshRemoteChainStatus"
    }
    $configuredValidationEnabled = $EnableConfiguredValidationRun -or ((-not $DisableConfiguredValidationRun) -and (-not $EnableTestGateValidationRun))

    if ($EnableTestGateValidationRun) {
        $StartArgs += @(
            "-UseTestGateValidationCommand",
            "-RequireSafeTestGateValidationCommand",
            "-RequireTestGateValidationRun"
        )
    }
    if ($configuredValidationEnabled) {
        $StartArgs += @(
            "-ValidationCommand", $ConfiguredValidationCommand,
            "-ValidationTimeoutSecs", $ValidationTimeoutSecs,
            "-ValidationPhase", "pre",
            "-RequireConfiguredValidationRun"
        )
    }

    $StartProcessArgs = @($StartArgs | ForEach-Object { Quote-CommandArgument $_ })
    $StartCommandLine = $StartProcessArgs -join " "
    $LaunchScript = Join-Path $WorkDirPath "evolution-loop.launch.ps1"
    $StartScriptArgs = @()
    if ($StartArgs.Count -gt 5) {
        $StartScriptArgs = @($StartArgs[5..($StartArgs.Count - 1)])
    }

    if ($CheckOnly) {
        Write-Host "check_only=true"
        Write-Host "starts_process=false"
        Write-Host "sends_prompt=false"
        Write-Host "existing_running=$($null -ne $existing)"
        if ($null -ne $existing) {
            Write-Host "existing_pid=$($existing.Id)"
        }
        Write-Host "existing_orphan_count=$($existingOrphans.Count)"
        if ($existingOrphans.Count -gt 0) {
            Write-Host "existing_orphan_pids=$existingOrphanPids"
        }
        Write-Host "stale_pool_lease_removed_count=$($staleLeaseCleanup.removed_count)"
        if ($staleLeaseCleanup.removed_count -gt 0) {
            Write-Host "stale_pool_lease_removed=$((@($staleLeaseCleanup.removed) | ForEach-Object { "$($_.reason):$($_.path)" }) -join ',')"
        }
        if ($null -ne $backendBusyStatus) {
            Write-Host "backend_busy_checked=$($backendBusyStatus.checked)"
            Write-Host "backend_busy=$($backendBusyStatus.busy)"
            Write-Host "backend_active_engine_requests=$($backendBusyStatus.active_engine_requests)"
            Write-Host "backend_active_endpoints=$($backendBusyStatus.active_endpoints -join ',')"
            Write-Host "backend_active_request_ids=$($backendBusyStatus.active_request_ids -join ',')"
            if ($backendBusyStatus.error.Trim().Length -gt 0) {
                Write-Host "backend_busy_error=$($backendBusyStatus.error)"
            }
        }
        Write-Host "pid_file=$PidFile"
        Write-Host "stdout_log=$StdoutLog"
        Write-Host "stderr_log=$StderrLog"
        Write-Host "min_runtime_context=$($ResolvedMinRuntimeContext.Value)"
        Write-Host "min_runtime_context_source=$($ResolvedMinRuntimeContext.Source)"
        Write-Host "launch_script=$LaunchScript"
        Write-Host "command=$PowerShellExe $StartCommandLine"
        exit 0
    }

    if ($null -ne $existing) {
        Write-Host "daemon_start: already_running pid=$($existing.Id)"
        exit 0
    }
    if ($existingOrphans.Count -gt 0) {
        if ($null -ne $existingAdoptionPid) {
            Set-Content -Encoding ASCII -LiteralPath $PidFile -Value ([string]$existingAdoptionPid)
        }
        Write-Host "daemon_start: already_running_adopted_orphan count=$($existingOrphans.Count) pids=$existingOrphanPids"
        if ($null -ne $existingAdoptionPid) {
            Write-Host "adopted_pid=$existingAdoptionPid"
            Write-Host "pid_file=$PidFile"
        }
        exit 0
    }
    if ($null -ne $backendBusyStatus -and $backendBusyStatus.busy) {
        $daemonBeforeStart = Daemon-Status -PidFile $PidFile -LedgerPath $LedgerPath -ReportPath $ReportPath -RemoteChainStatusPath $RemoteChainStatusPath -ModelCacheStatusPath $ModelCacheStatusPath -LogPath $StdoutLog -ErrPath $StderrLog -RequireValidationExecution:$RequireValidationExecutionEffective
        if ($daemonBeforeStart.adopted_orphan_process_count -eq 0) {
            $daemonViaSelf = Get-DaemonStatusViaSelf -ScriptPath $DaemonScriptPath -PowerShellExe $PowerShellExe -PidWorkDir $WorkDirPath -LedgerPath $LedgerPath -ReportPath $ReportPath -Backend $Backend -RemoteChainStatusPath $RemoteChainStatusPath -ModelCacheStatusPath $ModelCacheStatusPath -MinRuntimeContext $ResolvedMinRuntimeContext.Value
            if ($null -ne $daemonViaSelf) {
                $daemonBeforeStart = $daemonViaSelf
            }
        }
        if ($daemonBeforeStart.adopted_orphan_process_count -gt 0 -and $null -ne $daemonBeforeStart.pid) {
            Set-Content -Encoding ASCII -LiteralPath $PidFile -Value ([string]$daemonBeforeStart.pid)
            Write-Host "daemon_start: adopted_busy_orphan pid=$($daemonBeforeStart.pid) pids=$($daemonBeforeStart.adopted_orphan_pids -join ',')"
            Write-Host "pid_file=$PidFile"
        }
        Write-Host "daemon_start: backend_busy active_engine_requests=$($backendBusyStatus.active_engine_requests) active_endpoints=$($backendBusyStatus.active_endpoints -join ',') active_request_ids=$($backendBusyStatus.active_request_ids -join ',')"
        exit 0
    }

    $launchLines = @(
        '$ErrorActionPreference = "Stop"',
        "Set-Location -LiteralPath $(Quote-PowerShellLiteral $RepoRoot)",
        "`$powershell = $(Quote-PowerShellLiteral $PowerShellExe)",
        "`$script = $(Quote-PowerShellLiteral $StartScript)",
        '$arguments = @('
    )
    for ($index = 0; $index -lt $StartScriptArgs.Count; $index++) {
        $suffix = if ($index -lt ($StartScriptArgs.Count - 1)) { "," } else { "" }
        $launchLines += "    $(Quote-PowerShellLiteral $StartScriptArgs[$index])$suffix"
    }
    $launchLines += @(
        ')',
        'Write-Host "daemon_launch: invoking start-evolution-loop.ps1"',
        'try {',
        '    & $powershell -NoProfile -ExecutionPolicy Bypass -File $script @arguments',
        '    $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }',
        '    Write-Host "daemon_launch: completed exit_code=$exitCode"',
        '    exit $exitCode',
        '} catch {',
        '    Write-Error $_',
        '    exit 1',
        '}'
    )
    Set-Content -Encoding ASCII -LiteralPath $LaunchScript -Value $launchLines

    if ($staleLeaseCleanup.removed_count -gt 0) {
        Write-Host "daemon_start: stale_pool_leases_removed count=$($staleLeaseCleanup.removed_count)"
        foreach ($item in @($staleLeaseCleanup.removed)) {
            Write-Host "stale_pool_lease_removed reason=$($item.reason) owner_pid=$($item.owner_pid) path=$($item.path)"
        }
    }

    $launch = Start-DetachedDaemonProcess -PowerShellExe $PowerShellExe -LaunchScript $LaunchScript -RepoRoot $RepoRoot -StdoutLog $StdoutLog -StderrLog $StderrLog
    $process = if ($null -ne $launch.process) { $launch.process } else { Get-Process -Id $launch.pid -ErrorAction SilentlyContinue }
    Start-Sleep -Seconds 5
    $liveStartedProcess = Get-Process -Id $launch.pid -ErrorAction SilentlyContinue
    if ($null -eq $liveStartedProcess -or ($null -ne $process -and $process.HasExited)) {
        $stdoutPreview = (Read-LogTail -Path $StdoutLog -Count 20 | Out-String).Trim()
        $stderrPreview = (Read-LogTail -Path $StderrLog -Count 20 | Out-String).Trim()
        $exitCodeText = if ($null -ne $process) { [string]$process.ExitCode } else { "unknown" }
        Write-Host "daemon_start: failed_early pid=$($launch.pid) launch_mode=$($launch.launch_mode) exit_code=$exitCodeText"
        if ($launch.error.Trim().Length -gt 0) {
            Write-Host "launch_fallback_error=$($launch.error)"
        }
        if ($stdoutPreview.Length -gt 0) {
            Write-Host "stdout_tail=$stdoutPreview"
        }
        if ($stderrPreview.Length -gt 0) {
            Write-Host "stderr_tail=$stderrPreview"
        }
        exit 1
    }
    Set-Content -Encoding ASCII -LiteralPath $PidFile -Value ([string]$launch.pid)
    Write-Host "daemon_start: started pid=$($launch.pid) launch_mode=$($launch.launch_mode)"
    if ($launch.error.Trim().Length -gt 0) {
        Write-Host "launch_fallback_error=$($launch.error)"
    }
    Write-Host "pid_file=$PidFile"
    Write-Host "ledger=$LedgerPath"
    Write-Host "report_json=$ReportPath"
    Write-Host "stdout_log=$StdoutLog"
    Write-Host "stderr_log=$StderrLog"
    exit 0
}
