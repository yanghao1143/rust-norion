param(
    [string]$RepoRoot = "",
    [string]$SummaryJson = "target\evolution\strict-status-summary.json",
    [int]$MaxSummaryAgeSeconds = 900,
    [string]$RequiredHelperStageRoles = "summary,router,review,index,test-gate",
    [switch]$JsonStatus,
    [switch]$FailOnNotReady,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if ($RepoRoot.Trim().Length -eq 0) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
}

if ($Help) {
    Write-Host "Verify the compact SmartSteam strict unattended evolution status summary artifact."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\evolution-loop\verify-strict-status-summary.cmd [-JsonStatus] [-FailOnNotReady]"
    Write-Host "  .\tools\evolution-loop\verify-strict-status-summary.cmd [-SummaryJson PATH] [-MaxSummaryAgeSeconds 900]"
    Write-Host ""
    Write-Host "Contracts:"
    Write-Host "  read_only=true"
    Write-Host "  starts_process=false"
    Write-Host "  sends_prompt=false"
    exit 0
}

function Resolve-RepoPath {
    param([string]$Path)

    if ($Path.Trim().Length -eq 0) {
        return ""
    }
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return Join-Path $RepoRoot $Path
}

function Has-Property {
    param(
        [object]$Object,
        [string]$Name
    )

    return $null -ne $Object -and $Object.PSObject.Properties.Name -contains $Name
}

function Parse-CommaList {
    param([string]$Value)

    $items = @()
    foreach ($item in $Value.Split(",")) {
        $trimmed = $item.Trim()
        if ($trimmed.Length -gt 0 -and $items -notcontains $trimmed) {
            $items += $trimmed
        }
    }
    return $items
}

$SummaryPath = Resolve-RepoPath $SummaryJson
$exists = Test-Path -LiteralPath $SummaryPath -PathType Leaf
$ageSeconds = $null
$artifact = $null
$parseError = ""
if ($exists) {
    $item = Get-Item -LiteralPath $SummaryPath
    $ageSeconds = [int][Math]::Max(0, ((Get-Date) - $item.LastWriteTime).TotalSeconds)
    try {
        $artifact = Get-Content -LiteralPath $SummaryPath -Raw | ConvertFrom-Json
    } catch {
        $parseError = $_.Exception.Message
    }
}

$summary = if ($null -ne $artifact -and (Has-Property -Object $artifact -Name "summary")) { $artifact.summary } else { $null }
$artifactReady = if ($null -ne $artifact -and (Has-Property -Object $artifact -Name "readiness") -and (Has-Property -Object $artifact.readiness -Name "ready")) { [bool]$artifact.readiness.ready } else { $false }
$contractVersion = if ($null -ne $artifact -and (Has-Property -Object $artifact -Name "contract_version")) { [string]$artifact.contract_version } else { "" }
$startsProcess = if ($null -ne $artifact -and (Has-Property -Object $artifact -Name "starts_process")) { [bool]$artifact.starts_process } else { $true }
$sendsPrompt = if ($null -ne $artifact -and (Has-Property -Object $artifact -Name "sends_prompt")) { [bool]$artifact.sends_prompt } else { $true }

$helperRoles = if ($null -ne $summary -and (Has-Property -Object $summary -Name "helper_stage_roles")) { @($summary.helper_stage_roles | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }) } else { @() }
$requiredRoles = Parse-CommaList -Value $RequiredHelperStageRoles
$missingRoles = @()
foreach ($role in $requiredRoles) {
    if ($helperRoles -notcontains $role) {
        $missingRoles += $role
    }
}

$failures = @()
if (-not $exists) {
    $failures += "summary_missing"
}
if ($parseError.Trim().Length -gt 0) {
    $failures += "summary_invalid"
}
if ($exists -and $null -ne $ageSeconds -and [int]$MaxSummaryAgeSeconds -gt 0 -and $ageSeconds -gt [int]$MaxSummaryAgeSeconds) {
    $failures += "summary_stale"
}
if ($null -ne $artifact -and $contractVersion -ne "smartsteam.evolution-loop.strict-status-summary.v1") {
    $failures += "summary_contract_mismatch"
}
if ($null -ne $artifact -and ($startsProcess -or $sendsPrompt)) {
    $failures += "summary_contract_not_read_only"
}
if ($null -ne $artifact -and -not $artifactReady) {
    $failures += "summary_not_ready"
}
if ($null -eq $summary) {
    $failures += "summary_payload_missing"
} else {
    if (-not [bool]$summary.self_improve_passed) {
        $failures += "summary_self_improve_missing"
    }
    if (-not [bool]$summary.validation_passed -or [string]$summary.validation_source -ne "configured" -or $summary.validation_status_code -ne 0) {
        $failures += "summary_validation_missing"
    }
    if ($missingRoles.Count -gt 0) {
        $failures += "summary_helper_roles_missing"
    }
    if (-not [bool]$summary.helper_stage_contract_complete) {
        $failures += "summary_helper_contract_incomplete"
    }
    if (-not [bool]$summary.test_gate_passed -or [string]$summary.test_gate_verdict -ne "pass") {
        $failures += "summary_test_gate_not_pass"
    }
    if ([string]$summary.test_gate_validation_command_safety -ne "safe") {
        $failures += "summary_test_gate_validation_command_not_safe"
    }
}

$verification = [pscustomobject][ordered]@{
    schema_version = 1
    contract_version = "smartsteam.evolution-loop.strict-status-summary-verification.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    summary_artifact = [pscustomobject][ordered]@{
        path = $SummaryPath
        exists = $exists
        age_seconds = $ageSeconds
        max_age_seconds = [int]$MaxSummaryAgeSeconds
        parse_error = $parseError
        contract_version = $contractVersion
    }
    readiness = [pscustomobject][ordered]@{
        ready = $failures.Count -eq 0
        failures = @($failures | Sort-Object -Unique)
        summary_ready = $artifactReady
    }
    missing_helper_stage_roles = @($missingRoles)
    summary = $summary
    next_step = if ($failures.Count -eq 0) { "strict status summary is ready" } else { "refresh strict status artifacts or inspect summary failures" }
}

$exitCode = if ($FailOnNotReady -and -not $verification.readiness.ready) { 2 } else { 0 }
if ($JsonStatus) {
    $verification | ConvertTo-Json -Depth 8
    exit $exitCode
}

Write-Host "SmartSteam strict status summary verification"
Write-Host "read_only=true starts_process=false sends_prompt=false"
Write-Host "summary=$SummaryPath exists=$exists age_seconds=$ageSeconds max_age_seconds=$MaxSummaryAgeSeconds"
Write-Host "ready=$($verification.readiness.ready) failures=$($verification.readiness.failures -join ',') latest_round=$($summary.latest_round) active_round=$($summary.active_round) daemon_state=$($summary.daemon_state) next_round_decision=$($summary.next_round_decision_display_state) helper_roles=$($helperRoles -join ',') test_gate=$($summary.test_gate_verdict)"
Write-Host "next_step: $($verification.next_step)"
exit $exitCode
