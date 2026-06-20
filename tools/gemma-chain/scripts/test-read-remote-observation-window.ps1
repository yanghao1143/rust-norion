param(
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-observation-window.ps1")
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Assert-True {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

function Write-JsonFixture {
    param(
        [string]$Path,
        [object]$Value,
        [datetime]$LastWriteTime
    )

    $parent = Split-Path -Parent $Path
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }

    $Value | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $Path -Encoding UTF8
    (Get-Item -LiteralPath $Path).LastWriteTime = $LastWriteTime
}

function Write-Sample {
    param(
        [string]$Dir,
        [datetime]$LastWriteTime,
        [bool]$UnsafeBundle = $false,
        [bool]$PromptReady = $true,
        [int]$WorkerCount = 6,
        [int]$HealthyWorkerCount = 6
    )

    Write-JsonFixture -Path (Join-Path $Dir "chain-status.json") -LastWriteTime $LastWriteTime -Value ([pscustomobject]@{
        classification = if ($PromptReady) { "prompt_ready" } else { "not_ready" }
        prompt_ready = $PromptReady
        machine_summary = [pscustomobject]@{
            read_only = $true
            sends_prompt = $false
            launches_process = $false
        }
    })

    Write-JsonFixture -Path (Join-Path $Dir "pool-status.json") -LastWriteTime $LastWriteTime -Value ([pscustomobject]@{
        launch_allowed = $true
        capacity = [pscustomobject]@{
            worker_count = $WorkerCount
            healthy_worker_count = $HealthyWorkerCount
            expansion_allowed = $false
        }
    })

    Write-JsonFixture -Path (Join-Path $Dir "status-bundle.json") -LastWriteTime $LastWriteTime -Value ([pscustomobject]@{
        read_only = $true
        sends_prompt = $UnsafeBundle
        launches_process = $false
    })

    Write-JsonFixture -Path (Join-Path $Dir "forge-daemon-status.json") -LastWriteTime $LastWriteTime -Value ([pscustomobject]@{
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        evolution_status = [pscustomobject]@{
            daemon = [pscustomobject]@{ running = $false }
        }
        report_gate_preflight = [pscustomobject]@{
            continuation_state = "no_report"
        }
    })
}

function Invoke-WindowReader {
    param(
        [string]$RepoRoot,
        [string]$WindowDir,
        [int]$MinSamples = 3,
        [int]$MinSpanMinutes = 10
    )

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $RepoRoot -WindowDir $WindowDir -MinSamples $MinSamples -MinSpanMinutes $MinSpanMinutes -Json
    if ($LASTEXITCODE -ne 0) {
        throw "window reader exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("gemma-window-selftest-" + [System.Guid]::NewGuid().ToString("N"))
$missingRoot = Join-Path $root "missing-window"
$windowRoot = Join-Path $root "window"
$unsafeRoot = Join-Path $root "unsafe-window"
$unhealthyRoot = Join-Path $root "unhealthy-window"

try {
    New-Item -ItemType Directory -Path $root -Force | Out-Null

    $missing = Invoke-WindowReader -RepoRoot $root -WindowDir $missingRoot
    Assert-True ($missing.read_only -eq $true) "missing reader must be read-only"
    Assert-True ($missing.starts_process -eq $false) "missing reader must not start processes"
    Assert-True ($missing.sends_prompt -eq $false) "missing reader must not send prompts"
    Assert-True ($missing.touches_remote -eq $false) "missing reader must not touch remote"
    Assert-True ($missing.writes_files -eq $false) "missing reader must not write files"
    Assert-True ($missing.summary.status -eq "missing_window") "missing window status mismatch"
    Assert-True ($missing.summary.continuous_window_present -eq $false) "missing window cannot be continuous"
    Assert-True ($missing.authorization.can_authorize_daemon -eq $false) "missing window cannot authorize daemon"
    Assert-True ($missing.authorization.can_authorize_prompt -eq $false) "missing window cannot authorize prompt"
    Assert-True ($missing.authorization.can_authorize_ssh -eq $false) "missing window cannot authorize ssh"

    $baseTime = Get-Date
    Write-Sample -Dir (Join-Path $windowRoot "sample-001") -LastWriteTime ($baseTime.AddMinutes(-20))
    Write-Sample -Dir (Join-Path $windowRoot "sample-002") -LastWriteTime ($baseTime.AddMinutes(-10))
    Write-Sample -Dir (Join-Path $windowRoot "sample-003") -LastWriteTime $baseTime

    $observed = Invoke-WindowReader -RepoRoot $root -WindowDir $windowRoot
    Assert-True ($observed.contract_version -eq "smartsteam.remote-gemma-unattended.observation-window.v1") "contract version mismatch"
    Assert-True ($observed.summary.sample_count -eq 3) "window sample count mismatch"
    Assert-True ($observed.summary.complete_sample_count -eq 3) "window complete count mismatch"
    Assert-True ($observed.summary.safe_sample_count -eq 3) "window safe count mismatch"
    Assert-True ($observed.summary.known_health_sample_count -eq 3) "window known health count mismatch"
    Assert-True ($observed.summary.ok_health_sample_count -eq 3) "window ok health count mismatch"
    Assert-True ($observed.summary.span_seconds -ge 1200) "window span should cover at least 20 minutes"
    Assert-True ($observed.summary.continuous_window_present -eq $true) "window should satisfy continuous evidence"
    Assert-True ($observed.summary.can_support_residency_review -eq $true) "window should support external review"
    Assert-True ($observed.authorization.can_authorize_daemon -eq $false) "window evidence alone cannot authorize daemon"
    Assert-True ($observed.authorization.can_authorize_launch -eq $false) "window evidence alone cannot authorize launch"
    Assert-True ($observed.authorization.can_authorize_prompt -eq $false) "window evidence alone cannot authorize prompt"
    Assert-True ($observed.authorization.can_authorize_ssh -eq $false) "window evidence alone cannot authorize ssh"

    Write-Sample -Dir (Join-Path $unsafeRoot "sample-001") -LastWriteTime ($baseTime.AddMinutes(-20))
    Write-Sample -Dir (Join-Path $unsafeRoot "sample-002") -LastWriteTime ($baseTime.AddMinutes(-10)) -UnsafeBundle $true
    Write-Sample -Dir (Join-Path $unsafeRoot "sample-003") -LastWriteTime $baseTime

    $unsafe = Invoke-WindowReader -RepoRoot $root -WindowDir $unsafeRoot
    Assert-True ($unsafe.summary.sample_count -eq 3) "unsafe sample count mismatch"
    Assert-True ($unsafe.summary.safe_sample_count -eq 2) "unsafe safe count mismatch"
    Assert-True ($unsafe.summary.status -eq "unsafe_sample_contract") "unsafe status mismatch"
    Assert-True ($unsafe.summary.continuous_window_present -eq $false) "unsafe window cannot be continuous"
    Assert-True ($unsafe.authorization.can_authorize_prompt -eq $false) "unsafe window cannot authorize prompt"

    Write-Sample -Dir (Join-Path $unhealthyRoot "sample-001") -LastWriteTime ($baseTime.AddMinutes(-20))
    Write-Sample -Dir (Join-Path $unhealthyRoot "sample-002") -LastWriteTime ($baseTime.AddMinutes(-10)) -HealthyWorkerCount 5
    Write-Sample -Dir (Join-Path $unhealthyRoot "sample-003") -LastWriteTime $baseTime

    $unhealthy = Invoke-WindowReader -RepoRoot $root -WindowDir $unhealthyRoot
    Assert-True ($unhealthy.summary.sample_count -eq 3) "unhealthy sample count mismatch"
    Assert-True ($unhealthy.summary.safe_sample_count -eq 3) "unhealthy safe count mismatch"
    Assert-True ($unhealthy.summary.known_health_sample_count -eq 3) "unhealthy known health count mismatch"
    Assert-True ($unhealthy.summary.ok_health_sample_count -eq 2) "unhealthy ok health count mismatch"
    Assert-True ($unhealthy.summary.status -eq "health_not_ready") "unhealthy status mismatch"
    Assert-True ($unhealthy.summary.continuous_window_present -eq $false) "unhealthy window cannot be continuous"
    Assert-True ($unhealthy.authorization.can_authorize_launch -eq $false) "unhealthy window cannot authorize launch"

    Write-Host "read-remote-observation-window selftest passed"
} finally {
    if (Test-Path -LiteralPath $root) {
        Remove-Item -LiteralPath $root -Recurse -Force
    }
}
