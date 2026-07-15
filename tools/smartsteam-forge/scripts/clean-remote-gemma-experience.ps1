param(
    [string]$RepoRoot = "D:\rust-norion",
    [string]$RunDir = "",
    [string]$MemoryPath = "",
    [string]$ExperiencePath = "",
    [string]$AdaptivePath = "",
    [int]$Limit = 25,
    [switch]$ApplyQuarantine,
    [switch]$ApplyRepair,
    [switch]$NoReload,
    [switch]$SkipBuild,
    [int]$BackendPort = 7979,
    [int]$LocalModelPort = 8686,
    [switch]$CheckOnly,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Audit and optionally clean the remote Gemma chain experience store."
    Write-Host ""
    Write-Host "Default mode is read-only: audit + quarantine dry-run + repair dry-run + strict inspect gate."
    Write-Host "Apply mode requires explicit -ApplyQuarantine and/or -ApplyRepair. After any apply,"
    Write-Host "the script reloads only the local rust-norion backend unless -NoReload is passed."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\clean-remote-gemma-experience.cmd -CheckOnly"
    Write-Host "  .\tools\smartsteam-forge\clean-remote-gemma-experience.cmd"
    Write-Host "  .\tools\smartsteam-forge\clean-remote-gemma-experience.cmd -ApplyQuarantine"
    Write-Host "  .\tools\smartsteam-forge\clean-remote-gemma-experience.cmd -ApplyQuarantine -ApplyRepair"
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

function Invoke-Step {
    param(
        [string]$Label,
        [string]$Command,
        [object[]]$ArgumentList
    )

    Write-Host ""
    Write-Host "$Label"
    Write-Host "  $Command $(Join-Args -Items $ArgumentList)"
    & $Command @ArgumentList
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE"
    }
}

$RepoRoot = Resolve-RepoRoot $RepoRoot
if ([string]::IsNullOrWhiteSpace($RunDir)) {
    $RunDir = Join-Path $RepoRoot "target\remote-gemma-chain"
}
if ([string]::IsNullOrWhiteSpace($ExperiencePath)) {
    $ExperiencePath = Join-Path $RunDir "state\experience.ndkv"
}
if ([string]::IsNullOrWhiteSpace($MemoryPath)) {
    $MemoryPath = Join-Path $RunDir "state\memory.ndkv"
}
if ([string]::IsNullOrWhiteSpace($AdaptivePath)) {
    $AdaptivePath = Join-Path $RunDir "state\adaptive.ndkv"
}
$MemoryPath = [System.IO.Path]::GetFullPath($MemoryPath)
$ExperiencePath = [System.IO.Path]::GetFullPath($ExperiencePath)
$AdaptivePath = [System.IO.Path]::GetFullPath($AdaptivePath)
$cleanupDir = Join-Path $RepoRoot "target\experience-cleanup-audit"
New-Item -ItemType Directory -Force -Path $cleanupDir | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$backupPath = Join-Path $cleanupDir "experience.ndkv.quarantine-backup.$stamp.ndkv"
$quarantinePath = Join-Path $cleanupDir "experience.ndkv.quarantine.$stamp.ndkv"
$repairBackupPath = Join-Path $cleanupDir "experience.ndkv.repair-backup.$stamp.ndkv"
$cargo = "cargo"
$reloadScript = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\reload-remote-gemma-backend.ps1"

$auditArgs = @(
    "run", "--",
    "--memory", $MemoryPath,
    "--experience", $ExperiencePath,
    "--adaptive", $AdaptivePath,
    "--experience-cleanup-audit",
    "--experience-cleanup-audit-limit", $Limit
)
$quarantineDryRunArgs = @(
    "run", "--",
    "--memory", $MemoryPath,
    "--experience", $ExperiencePath,
    "--adaptive", $AdaptivePath,
    "--experience-hygiene-quarantine",
    "--experience-hygiene-limit", $Limit
)
$quarantineApplyArgs = @(
    "run", "--",
    "--memory", $MemoryPath,
    "--experience", $ExperiencePath,
    "--adaptive", $AdaptivePath,
    "--experience-hygiene-apply",
    "--experience-hygiene-limit", $Limit,
    "--experience-hygiene-backup-path", $backupPath,
    "--experience-hygiene-quarantine-path", $quarantinePath
)
$repairDryRunArgs = @(
    "run", "--",
    "--memory", $MemoryPath,
    "--experience", $ExperiencePath,
    "--adaptive", $AdaptivePath,
    "--experience-repair",
    "--experience-repair-limit", $Limit
)
$repairApplyArgs = @(
    "run", "--",
    "--memory", $MemoryPath,
    "--experience", $ExperiencePath,
    "--adaptive", $AdaptivePath,
    "--experience-repair-apply",
    "--experience-repair-limit", $Limit,
    "--experience-repair-backup-path", $repairBackupPath
)
$inspectArgs = @(
    "run", "--",
    "--memory", $MemoryPath,
    "--experience", $ExperiencePath,
    "--adaptive", $AdaptivePath,
    "--inspect-state",
    "--inspect-gate",
    "--inspect-limit", $Limit,
    "--inspect-max-experience-hygiene-quarantine-candidates", 0,
    "--inspect-max-experience-repairable-legacy-metadata-lessons", 0,
    "--inspect-max-experience-repairable-index-records", 0,
    "--inspect-max-experience-repair-projected-legacy-metadata-lessons", 0,
    "--inspect-max-experience-repair-skipped-missing-clean-gist", 0,
    "--inspect-max-experience-index-overlong-without-clean-gist", 0,
    "--inspect-max-experience-index-noisy-records", 0,
    "--inspect-max-experience-index-noise-penalty", 0,
    "--inspect-min-experience-index-quality-score", 0.92,
    "--inspect-require-experience-index-retrieval-ready"
)

Write-Host "SmartSteam remote Gemma experience cleanup"
Write-Host "repo:       $RepoRoot"
Write-Host "run_dir:    $RunDir"
Write-Host "memory:     $MemoryPath"
Write-Host "experience: $ExperiencePath"
Write-Host "adaptive:   $AdaptivePath"
Write-Host "limit:      $Limit"
Write-Host "mode:       $(if ($ApplyQuarantine -or $ApplyRepair) { 'apply' } else { 'read-only' })"
Write-Host "reload:     $(if (($ApplyQuarantine -or $ApplyRepair) -and -not $NoReload) { 'after_apply' } else { 'disabled_or_not_needed' })"

if ($CheckOnly) {
    Write-Host ""
    Write-Host "audit_command: $cargo $(Join-Args -Items $auditArgs)"
    Write-Host "quarantine_dry_run_command: $cargo $(Join-Args -Items $quarantineDryRunArgs)"
    if ($ApplyQuarantine) {
        Write-Host "quarantine_apply_command: $cargo $(Join-Args -Items $quarantineApplyArgs)"
    }
    Write-Host "repair_dry_run_command: $cargo $(Join-Args -Items $repairDryRunArgs)"
    if ($ApplyRepair) {
        Write-Host "repair_apply_command: $cargo $(Join-Args -Items $repairApplyArgs)"
    }
    Write-Host "inspect_command: $cargo $(Join-Args -Items $inspectArgs)"
    if (($ApplyQuarantine -or $ApplyRepair) -and -not $NoReload) {
        Write-Host "reload_command: $reloadScript -RepoRoot $RepoRoot -RunDir $RunDir -BackendPort $BackendPort -LocalModelPort $LocalModelPort -SkipBuild"
    }
    if ($ApplyQuarantine -or $ApplyRepair) {
        Write-Host "stop_command: $reloadScript -RepoRoot $RepoRoot -RunDir $RunDir -BackendPort $BackendPort -LocalModelPort $LocalModelPort -SkipBuild -StopOnly"
    }
    Write-Host ""
    Write-Host "SmartSteam remote Gemma experience cleanup preflight: PASS"
    Write-Host "check_only=true"
    Write-Host "writes_experience_state=$(if ($ApplyQuarantine -or $ApplyRepair) { 'would_write' } else { 'false' })"
    Write-Host "reloads_backend=$(if (($ApplyQuarantine -or $ApplyRepair) -and -not $NoReload) { 'would_reload' } else { 'false' })"
    Write-Host "touches_remote=false"
    Write-Host "starts_process=false"
    Write-Host "sends_prompt=false"
    return
}

if ($ApplyQuarantine -or $ApplyRepair) {
    if (-not (Test-Path -LiteralPath $reloadScript -PathType Leaf)) {
        throw "reload script not found: $reloadScript"
    }
    Invoke-Step -Label "stop backend before atomic experience apply" -Command "powershell.exe" -ArgumentList @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $reloadScript,
        "-RepoRoot", $RepoRoot,
        "-RunDir", $RunDir,
        "-BackendPort", $BackendPort,
        "-LocalModelPort", $LocalModelPort,
        "-SkipBuild",
        "-StopOnly"
    )
}

Push-Location $RepoRoot
try {
    Invoke-Step -Label "cleanup audit" -Command $cargo -ArgumentList $auditArgs
    Invoke-Step -Label "quarantine dry-run" -Command $cargo -ArgumentList $quarantineDryRunArgs
    if ($ApplyQuarantine) {
        Invoke-Step -Label "quarantine apply" -Command $cargo -ArgumentList $quarantineApplyArgs
    }
    Invoke-Step -Label "repair dry-run" -Command $cargo -ArgumentList $repairDryRunArgs
    if ($ApplyRepair) {
        Invoke-Step -Label "repair apply" -Command $cargo -ArgumentList $repairApplyArgs
    }
    Invoke-Step -Label "strict inspect gate" -Command $cargo -ArgumentList $inspectArgs
} finally {
    Pop-Location
}

if (($ApplyQuarantine -or $ApplyRepair) -and -not $NoReload) {
    if (-not (Test-Path -LiteralPath $reloadScript -PathType Leaf)) {
        throw "reload script not found: $reloadScript"
    }
    $reloadArgs = @(
        "-RepoRoot", $RepoRoot,
        "-RunDir", $RunDir,
        "-BackendPort", $BackendPort,
        "-LocalModelPort", $LocalModelPort
    )
    if ($SkipBuild) {
        $reloadArgs += "-SkipBuild"
    }
    Invoke-Step -Label "backend reload after experience apply" -Command "powershell.exe" -ArgumentList (@(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $reloadScript
    ) + $reloadArgs)
}

Write-Host ""
Write-Host "remote_gemma_experience_cleanup=PASS"
Write-Host "writes_experience_state=$(if ($ApplyQuarantine -or $ApplyRepair) { 'true' } else { 'false' })"
Write-Host "reloads_backend=$(if (($ApplyQuarantine -or $ApplyRepair) -and -not $NoReload) { 'true' } else { 'false' })"
Write-Host "touches_remote=false"
Write-Host "sends_prompt=false"
