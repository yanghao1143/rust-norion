param(
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-resource-window.ps1")
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

function Write-ResourceSample {
    param(
        [string]$Dir,
        [datetime]$LastWriteTime,
        [bool]$Unsafe = $false,
        [double]$MemoryGb = 24,
        [bool]$MetalAvailable = $true
    )

    Write-JsonFixture -Path (Join-Path $Dir "remote-resource-status.json") -LastWriteTime $LastWriteTime -Value ([pscustomobject]@{
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        writes_model_weights = $false
        approved_owner_flow = (-not $Unsafe)
        summary = [pscustomobject]@{
            memory_available_gb = $MemoryGb
            metal_available = $MetalAvailable
        }
    })
}

function Invoke-ResourceReader {
    param(
        [string]$RepoRoot,
        [string]$WindowDir,
        [int]$MinSamples = 3,
        [int]$MinSpanMinutes = 10,
        [double]$MinAvailableMemoryGb = 8
    )

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $RepoRoot -WindowDir $WindowDir -MinSamples $MinSamples -MinSpanMinutes $MinSpanMinutes -MinAvailableMemoryGb $MinAvailableMemoryGb -Json
    if ($LASTEXITCODE -ne 0) {
        throw "resource reader exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("gemma-resource-selftest-" + [System.Guid]::NewGuid().ToString("N"))
$missingRoot = Join-Path $root "missing-window"
$windowRoot = Join-Path $root "window"
$unsafeRoot = Join-Path $root "unsafe-window"
$lowRoot = Join-Path $root "low-window"

try {
    New-Item -ItemType Directory -Path $root -Force | Out-Null

    $missing = Invoke-ResourceReader -RepoRoot $root -WindowDir $missingRoot
    Assert-True ($missing.read_only -eq $true) "missing reader must be read-only"
    Assert-True ($missing.starts_process -eq $false) "missing reader must not start processes"
    Assert-True ($missing.sends_prompt -eq $false) "missing reader must not send prompts"
    Assert-True ($missing.touches_remote -eq $false) "missing reader must not touch remote"
    Assert-True ($missing.writes_files -eq $false) "missing reader must not write files"
    Assert-True ($missing.writes_model_weights -eq $false) "missing reader must not write model weights"
    Assert-True ($missing.summary.status -eq "missing_resource_window") "missing resource window status mismatch"
    Assert-True ($missing.summary.resource_window_present -eq $false) "missing window cannot be present"
    Assert-True ($missing.authorization.can_authorize_ssh -eq $false) "missing window cannot authorize ssh"
    Assert-True ($missing.authorization.can_authorize_prompt -eq $false) "missing window cannot authorize prompt"

    $baseTime = Get-Date
    Write-ResourceSample -Dir (Join-Path $windowRoot "sample-001") -LastWriteTime ($baseTime.AddMinutes(-20))
    Write-ResourceSample -Dir (Join-Path $windowRoot "sample-002") -LastWriteTime ($baseTime.AddMinutes(-10))
    Write-ResourceSample -Dir (Join-Path $windowRoot "sample-003") -LastWriteTime $baseTime

    $observed = Invoke-ResourceReader -RepoRoot $root -WindowDir $windowRoot
    Assert-True ($observed.contract_version -eq "smartsteam.remote-gemma-unattended.resource-window.v1") "contract version mismatch"
    Assert-True ($observed.summary.sample_count -eq 3) "resource sample count mismatch"
    Assert-True ($observed.summary.complete_sample_count -eq 3) "resource complete count mismatch"
    Assert-True ($observed.summary.safe_sample_count -eq 3) "resource safe count mismatch"
    Assert-True ($observed.summary.known_headroom_sample_count -eq 3) "resource known headroom count mismatch"
    Assert-True ($observed.summary.ok_headroom_sample_count -eq 3) "resource ok headroom count mismatch"
    Assert-True ($observed.summary.span_seconds -ge 1200) "resource span should cover at least 20 minutes"
    Assert-True ($observed.summary.resource_window_present -eq $true) "resource window should satisfy evidence"
    Assert-True ($observed.summary.can_support_residency_review -eq $true) "resource window should support external review"
    Assert-True ($observed.authorization.can_authorize_daemon -eq $false) "resource evidence alone cannot authorize daemon"
    Assert-True ($observed.authorization.can_authorize_launch -eq $false) "resource evidence alone cannot authorize launch"
    Assert-True ($observed.authorization.can_authorize_prompt -eq $false) "resource evidence alone cannot authorize prompt"
    Assert-True ($observed.authorization.can_authorize_ssh -eq $false) "resource evidence alone cannot authorize ssh"

    Write-ResourceSample -Dir (Join-Path $unsafeRoot "sample-001") -LastWriteTime ($baseTime.AddMinutes(-20))
    Write-ResourceSample -Dir (Join-Path $unsafeRoot "sample-002") -LastWriteTime ($baseTime.AddMinutes(-10)) -Unsafe $true
    Write-ResourceSample -Dir (Join-Path $unsafeRoot "sample-003") -LastWriteTime $baseTime

    $unsafe = Invoke-ResourceReader -RepoRoot $root -WindowDir $unsafeRoot
    Assert-True ($unsafe.summary.sample_count -eq 3) "unsafe resource sample count mismatch"
    Assert-True ($unsafe.summary.safe_sample_count -eq 2) "unsafe resource safe count mismatch"
    Assert-True ($unsafe.summary.status -eq "unsafe_collection_contract") "unsafe resource status mismatch"
    Assert-True ($unsafe.summary.resource_window_present -eq $false) "unsafe resource window cannot be present"

    Write-ResourceSample -Dir (Join-Path $lowRoot "sample-001") -LastWriteTime ($baseTime.AddMinutes(-20))
    Write-ResourceSample -Dir (Join-Path $lowRoot "sample-002") -LastWriteTime ($baseTime.AddMinutes(-10)) -MemoryGb 4
    Write-ResourceSample -Dir (Join-Path $lowRoot "sample-003") -LastWriteTime $baseTime

    $low = Invoke-ResourceReader -RepoRoot $root -WindowDir $lowRoot
    Assert-True ($low.summary.sample_count -eq 3) "low resource sample count mismatch"
    Assert-True ($low.summary.ok_headroom_sample_count -eq 2) "low resource headroom count mismatch"
    Assert-True ($low.summary.status -eq "headroom_below_threshold") "low resource status mismatch"
    Assert-True ($low.authorization.can_authorize_launch -eq $false) "low resource window cannot authorize launch"

    Write-Host "read-remote-resource-window selftest passed"
} finally {
    if (Test-Path -LiteralPath $root) {
        Remove-Item -LiteralPath $root -Recurse -Force
    }
}
