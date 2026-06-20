param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate SmartSteam remote Gemma experience cleanup command assembly without writing state."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\test-remote-gemma-experience-cleanup.cmd"
    return
}

$script = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\clean-remote-gemma-experience.ps1"
if (-not (Test-Path -LiteralPath $script -PathType Leaf)) {
    throw "clean-remote-gemma-experience.ps1 not found: $script"
}

function Invoke-CleanupCase {
    param(
        [string]$Name,
        [string[]]$ArgumentList
    )
    Write-Host ""
    Write-Host "cleanup_case=$Name"
    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $script @ArgumentList 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($exitCode -ne 0) {
        throw "cleanup case '$Name' failed with exit code $exitCode"
    }
    return $text
}

function Assert-Contains {
    param(
        [string]$Name,
        [string]$Text,
        [string]$Pattern
    )
    if ($Text -notmatch [regex]::Escape($Pattern)) {
        throw "cleanup case '$Name' did not contain expected text: $Pattern"
    }
}

function Assert-NotContains {
    param(
        [string]$Name,
        [string]$Text,
        [string]$Pattern
    )
    if ($Text -match [regex]::Escape($Pattern)) {
        throw "cleanup case '$Name' unexpectedly contained text: $Pattern"
    }
}

$testRunDir = Join-Path $RepoRoot "target\smartsteam-experience-cleanup-tests\remote-chain"
$readOnlyText = Invoke-CleanupCase -Name "read_only_check_only" -ArgumentList @(
    "-RepoRoot", $RepoRoot,
    "-RunDir", $testRunDir,
    "-Limit", "7",
    "-CheckOnly"
)
Assert-Contains -Name "read_only_check_only" -Text $readOnlyText -Pattern "SmartSteam remote Gemma experience cleanup preflight: PASS"
Assert-Contains -Name "read_only_check_only" -Text $readOnlyText -Pattern "mode:       read-only"
Assert-Contains -Name "read_only_check_only" -Text $readOnlyText -Pattern "--experience-cleanup-audit"
Assert-Contains -Name "read_only_check_only" -Text $readOnlyText -Pattern "--experience-hygiene-quarantine"
Assert-Contains -Name "read_only_check_only" -Text $readOnlyText -Pattern "--experience-repair"
Assert-Contains -Name "read_only_check_only" -Text $readOnlyText -Pattern "--inspect-state"
Assert-Contains -Name "read_only_check_only" -Text $readOnlyText -Pattern "writes_experience_state=false"
Assert-Contains -Name "read_only_check_only" -Text $readOnlyText -Pattern "reloads_backend=false"
Assert-Contains -Name "read_only_check_only" -Text $readOnlyText -Pattern "touches_remote=false"
Assert-Contains -Name "read_only_check_only" -Text $readOnlyText -Pattern "starts_process=false"
Assert-Contains -Name "read_only_check_only" -Text $readOnlyText -Pattern "sends_prompt=false"
Assert-NotContains -Name "read_only_check_only" -Text $readOnlyText -Pattern "--experience-hygiene-apply"
Assert-NotContains -Name "read_only_check_only" -Text $readOnlyText -Pattern "--experience-repair-apply"
Write-Host "cleanup_case_result=read_only_check_only PASS"

$applyText = Invoke-CleanupCase -Name "apply_check_only_plans_reload" -ArgumentList @(
    "-RepoRoot", $RepoRoot,
    "-RunDir", $testRunDir,
    "-Limit", "7",
    "-ApplyQuarantine",
    "-ApplyRepair",
    "-SkipBuild",
    "-CheckOnly"
)
Assert-Contains -Name "apply_check_only_plans_reload" -Text $applyText -Pattern "mode:       apply"
Assert-Contains -Name "apply_check_only_plans_reload" -Text $applyText -Pattern "--experience-hygiene-apply"
Assert-Contains -Name "apply_check_only_plans_reload" -Text $applyText -Pattern "--experience-repair-apply"
Assert-Contains -Name "apply_check_only_plans_reload" -Text $applyText -Pattern "reload_command:"
Assert-Contains -Name "apply_check_only_plans_reload" -Text $applyText -Pattern "writes_experience_state=would_write"
Assert-Contains -Name "apply_check_only_plans_reload" -Text $applyText -Pattern "reloads_backend=would_reload"
Assert-Contains -Name "apply_check_only_plans_reload" -Text $applyText -Pattern "touches_remote=false"
Assert-Contains -Name "apply_check_only_plans_reload" -Text $applyText -Pattern "starts_process=false"
Assert-Contains -Name "apply_check_only_plans_reload" -Text $applyText -Pattern "sends_prompt=false"
Write-Host "cleanup_case_result=apply_check_only_plans_reload PASS"

Write-Host ""
Write-Host "remote_gemma_experience_cleanup_selftest=PASS"
Write-Host "writes_experience_state=false"
Write-Host "touches_remote=false"
Write-Host "starts_process=false"
Write-Host "sends_prompt=false"
