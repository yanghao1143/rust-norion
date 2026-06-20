param(
    [switch]$Start,
    [switch]$Stop,
    [switch]$Status,
    [switch]$JsonStatus,
    [switch]$Candidates,
    [switch]$CandidateList,
    [switch]$CandidateGate,
    [switch]$CandidatesSave,
    [string]$CandidateMark = "",
    [string]$CandidateApplyCheck = "",
    [string]$CandidateValidate = "",
    [string]$CandidateValidationCommand = "",
    [string]$CandidateValidationStatus = "",
    [string]$CandidateStatus = "",
    [string]$CandidateNote = "",
    [switch]$Watch,
    [switch]$CheckOnly,
    [switch]$StartCheck,
    [switch]$JsonStartCheck,
    [switch]$StopCheck,
    [string]$WorkDir = "target\evolution\daemon",
    [string]$Backend = "",
    [string]$Prompt = "",
    [int]$CandidatesLimit = 5,
    [string]$CandidatesBacklog = "",
    [int]$IntervalSecs = 5,
    [int]$MaxTokens = -1,
    [int]$MaxTotalTokens = -1,
    [int]$MaxRuntimeSecs = -1,
    [int]$MaxFailures = -1,
    [int]$MaxNoFeedbackRounds = -1,
    [int]$TimeoutSecs = -1,
    [int]$Count = 0,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ForgeDir = Split-Path -Parent $ScriptDir
$RepoRoot = Split-Path -Parent (Split-Path -Parent $ForgeDir)
$ForgeManifest = Join-Path $ForgeDir "Cargo.toml"

if ($Help) {
    Write-Host "Control the SmartSteam Forge unattended evolution daemon."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -Status"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -Start -Backend 127.0.0.1:7979"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -StartCheck -Backend 127.0.0.1:7979"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -JsonStartCheck -Backend 127.0.0.1:7979"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -StartCheck -MaxTotalTokens 96 -MaxTokens 64"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -Stop"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -Candidates -CandidatesLimit 5"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -Candidates -CandidatesSave"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -CandidateList -CandidateStatus accepted"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -CandidateGate"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -CandidateApplyCheck <id|next>"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -CandidateValidate <id> -CandidateValidationCommand <cmd> -CandidateValidationStatus 0"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -CandidateMark <id> -CandidateStatus accepted"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -Watch -IntervalSecs 5"
    Write-Host "  .\tools\smartsteam-forge\evolution-daemon.cmd -Watch -Count 3"
    Write-Host ""
    Write-Host "Defaults:"
    Write-Host "  WorkDir: target\evolution\daemon"
    Write-Host "  Status, JsonStatus, and Candidates are read-only and do not send prompts."
    Write-Host "  Start runs candidate lifecycle preflight, then launches a budgeted daemon and sends prompts by design."
    Write-Host "  Start/StartCheck can override budgets with MaxTokens, MaxTotalTokens, MaxRuntimeSecs, MaxFailures, MaxNoFeedbackRounds, TimeoutSecs, and IntervalSecs."
    Write-Host "  JsonStartCheck is also a safe dry-run and returns machine-readable JSON with the budgeted command preview."
    Write-Host "  StartCheck/StopCheck are safe dry-runs."
    return
}

if ($StartCheck) {
    $Start = $true
    $CheckOnly = $true
}
if ($JsonStartCheck) {
    $Start = $true
    $CheckOnly = $true
}
if ($StopCheck) {
    $Stop = $true
    $CheckOnly = $true
}
if (($CandidatesSave -or -not [string]::IsNullOrWhiteSpace($CandidatesBacklog)) -and -not $Start -and -not $Stop -and -not $Status -and -not $JsonStatus -and -not $Watch -and [string]::IsNullOrWhiteSpace($CandidateMark) -and [string]::IsNullOrWhiteSpace($CandidateApplyCheck) -and [string]::IsNullOrWhiteSpace($CandidateValidate) -and -not $CandidateList -and -not $CandidateGate) {
    $Candidates = $true
}
if (-not $Start -and -not $Stop -and -not $Status -and -not $JsonStatus -and -not $Candidates -and -not $CandidateList -and -not $CandidateGate -and [string]::IsNullOrWhiteSpace($CandidateMark) -and [string]::IsNullOrWhiteSpace($CandidateApplyCheck) -and [string]::IsNullOrWhiteSpace($CandidateValidate) -and -not $Watch) {
    $Status = $true
}

$actions = 0
if ($Start) { $actions += 1 }
if ($Stop) { $actions += 1 }
if ($Status -or $JsonStatus) { $actions += 1 }
if ($Candidates) { $actions += 1 }
if ($CandidateList) { $actions += 1 }
if ($CandidateGate) { $actions += 1 }
if (-not [string]::IsNullOrWhiteSpace($CandidateMark)) { $actions += 1 }
if (-not [string]::IsNullOrWhiteSpace($CandidateApplyCheck)) { $actions += 1 }
if (-not [string]::IsNullOrWhiteSpace($CandidateValidate)) { $actions += 1 }
if ($Watch) { $actions += 1 }
if ($actions -gt 1) {
    throw "choose only one daemon action: Start, Stop, Status, JsonStatus, Candidates, CandidateList, CandidateGate, CandidateApplyCheck, CandidateValidate, CandidateMark, or Watch"
}
if (-not [string]::IsNullOrWhiteSpace($CandidateMark) -and [string]::IsNullOrWhiteSpace($CandidateStatus)) {
    throw "CandidateStatus is required with CandidateMark"
}
if (-not [string]::IsNullOrWhiteSpace($CandidateValidate) -and [string]::IsNullOrWhiteSpace($CandidateValidationCommand)) {
    throw "CandidateValidationCommand is required with CandidateValidate"
}
if (-not [string]::IsNullOrWhiteSpace($CandidateValidate) -and [string]::IsNullOrWhiteSpace($CandidateValidationStatus)) {
    throw "CandidateValidationStatus is required with CandidateValidate"
}
if ($CandidatesLimit -le 0) {
    throw "CandidatesLimit must be positive"
}
if ($IntervalSecs -le 0) {
    throw "IntervalSecs must be positive"
}
if ($Count -lt 0) {
    throw "Count must be zero or positive"
}

if (-not (Test-Path -LiteralPath $ForgeManifest -PathType Leaf)) {
    throw "SmartSteam Forge manifest not found: $ForgeManifest"
}

$forgeArgs = @(
    "run",
    "-q",
    "--manifest-path", $ForgeManifest,
    "--"
)

if ($Start) {
    if ($JsonStartCheck) {
        $forgeArgs += "--evolution-start-check-json"
    } else {
        $forgeArgs += "--evolution-start"
    }
} elseif ($Stop) {
    $forgeArgs += "--evolution-stop"
} elseif ($Watch) {
    $forgeArgs += @("--evolution-watch", ([string]$IntervalSecs))
    if ($Count -gt 0) {
        $forgeArgs += @("--evolution-watch-count", ([string]$Count))
    }
} elseif ($Candidates) {
    $forgeArgs += @("--evolution-candidates", "--evolution-candidates-limit", ([string]$CandidatesLimit))
    if ($CandidatesSave) {
        $forgeArgs += "--evolution-candidates-save"
    }
    if (-not [string]::IsNullOrWhiteSpace($CandidatesBacklog)) {
        $forgeArgs += @("--evolution-candidates-backlog", $CandidatesBacklog)
    }
} elseif ($CandidateList) {
    $forgeArgs += @("--evolution-candidate-list", "--evolution-candidates-limit", ([string]$CandidatesLimit))
    if (-not [string]::IsNullOrWhiteSpace($CandidateStatus)) {
        $forgeArgs += @("--evolution-candidate-status", $CandidateStatus)
    }
    if (-not [string]::IsNullOrWhiteSpace($CandidatesBacklog)) {
        $forgeArgs += @("--evolution-candidates-backlog", $CandidatesBacklog)
    }
} elseif ($CandidateGate) {
    $forgeArgs += "--evolution-candidate-gate"
    if (-not [string]::IsNullOrWhiteSpace($CandidatesBacklog)) {
        $forgeArgs += @("--evolution-candidates-backlog", $CandidatesBacklog)
    }
} elseif (-not [string]::IsNullOrWhiteSpace($CandidateApplyCheck)) {
    $forgeArgs += @("--evolution-candidate-apply-check", $CandidateApplyCheck)
    if (-not [string]::IsNullOrWhiteSpace($CandidatesBacklog)) {
        $forgeArgs += @("--evolution-candidates-backlog", $CandidatesBacklog)
    }
} elseif (-not [string]::IsNullOrWhiteSpace($CandidateValidate)) {
    $forgeArgs += @(
        "--evolution-candidate-validate", $CandidateValidate,
        "--evolution-candidate-validation-command", $CandidateValidationCommand,
        "--evolution-candidate-validation-status", $CandidateValidationStatus
    )
    if (-not [string]::IsNullOrWhiteSpace($CandidateNote)) {
        $forgeArgs += @("--evolution-candidate-note", $CandidateNote)
    }
    if (-not [string]::IsNullOrWhiteSpace($CandidatesBacklog)) {
        $forgeArgs += @("--evolution-candidates-backlog", $CandidatesBacklog)
    }
} elseif (-not [string]::IsNullOrWhiteSpace($CandidateMark)) {
    $forgeArgs += @(
        "--evolution-candidate-mark", $CandidateMark,
        "--evolution-candidate-status", $CandidateStatus
    )
    if (-not [string]::IsNullOrWhiteSpace($CandidateNote)) {
        $forgeArgs += @("--evolution-candidate-note", $CandidateNote)
    }
    if (-not [string]::IsNullOrWhiteSpace($CandidatesBacklog)) {
        $forgeArgs += @("--evolution-candidates-backlog", $CandidatesBacklog)
    }
} elseif ($JsonStatus) {
    $forgeArgs += "--evolution-status-json"
} else {
    $forgeArgs += "--evolution-status"
}

$forgeArgs += @("--evolution-work-dir", $WorkDir)

if ($CheckOnly) {
    $forgeArgs += "--evolution-check-only"
}
if (($Start -or $Status -or $JsonStatus -or $Watch) -and -not [string]::IsNullOrWhiteSpace($Backend)) {
    $forgeArgs += @("--backend", $Backend)
}
if ($Start -and -not [string]::IsNullOrWhiteSpace($Prompt)) {
    $forgeArgs += @("--prompt", $Prompt)
}
if ($Start -and -not [string]::IsNullOrWhiteSpace($CandidatesBacklog)) {
    $forgeArgs += @("--evolution-candidates-backlog", $CandidatesBacklog)
}
if ($Start -and $PSBoundParameters.ContainsKey("IntervalSecs")) {
    $forgeArgs += @("--evolution-interval-secs", ([string]$IntervalSecs))
}
if ($Start -and $PSBoundParameters.ContainsKey("MaxTokens")) {
    $forgeArgs += @("--evolution-max-tokens", ([string]$MaxTokens))
}
if ($Start -and $PSBoundParameters.ContainsKey("MaxTotalTokens")) {
    $forgeArgs += @("--evolution-max-total-tokens", ([string]$MaxTotalTokens))
}
if ($Start -and $PSBoundParameters.ContainsKey("MaxRuntimeSecs")) {
    $forgeArgs += @("--evolution-max-runtime-secs", ([string]$MaxRuntimeSecs))
}
if ($Start -and $PSBoundParameters.ContainsKey("MaxFailures")) {
    $forgeArgs += @("--evolution-max-failures", ([string]$MaxFailures))
}
if ($Start -and $PSBoundParameters.ContainsKey("MaxNoFeedbackRounds")) {
    $forgeArgs += @("--evolution-max-no-feedback-rounds", ([string]$MaxNoFeedbackRounds))
}
if ($Start -and $PSBoundParameters.ContainsKey("TimeoutSecs")) {
    $forgeArgs += @("--evolution-timeout-secs", ([string]$TimeoutSecs))
}

function Quote-ProcessArgument {
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

Push-Location $RepoRoot
try {
    $cargoArgLine = ($forgeArgs | ForEach-Object { Quote-ProcessArgument $_ }) -join " "
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = "cargo.exe"
    $startInfo.Arguments = $cargoArgLine
    $startInfo.WorkingDirectory = $RepoRoot
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.CreateNoWindow = $true
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void]$process.Start()
    $stdoutText = $process.StandardOutput.ReadToEnd()
    $stderrText = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    $exitCode = $process.ExitCode
    $text = @($stdoutText, $stderrText) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    exit $exitCode
} finally {
    Pop-Location
}
