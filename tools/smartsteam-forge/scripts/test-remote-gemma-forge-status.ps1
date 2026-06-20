param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate SmartSteam remote Gemma Forge status wrapper without SSH, process launch, or prompt sending."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\test-remote-gemma-forge-status.cmd"
    return
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot not found: $RepoRoot"
}

$startScript = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\start-remote-gemma-forge.ps1"
if (-not (Test-Path -LiteralPath $startScript -PathType Leaf)) {
    throw "start-remote-gemma-forge.ps1 not found: $startScript"
}

$workDir = Join-Path $RepoRoot "target\remote-gemma-chain\forge-status-selftest"
if (Test-Path -LiteralPath $workDir) {
    Remove-Item -LiteralPath $workDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $workDir | Out-Null

function Invoke-StatusJson {
    param(
        [int]$LocalModelPort,
        [int]$BackendPort,
        [int]$LabPort,
        [int]$RemoteModelPort,
        [switch]$UseMac32GBModelPool
    )

    $args = @(
        "-RepoRoot", $RepoRoot,
        "-Status",
        "-JsonStatus",
        "-RunDir", $workDir,
        "-LocalModelPort", $LocalModelPort,
        "-BackendPort", $BackendPort,
        "-LabPort", $LabPort,
        "-RemoteModelPort", $RemoteModelPort
    )
    if ($UseMac32GBModelPool) {
        $args += "-UseMac32GBModelPool"
    }

    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $startScript @args 2>&1
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if ($exitCode -ne 0) {
        throw "remote Gemma Forge status failed with exit code ${exitCode}: $text"
    }
    try {
        return $text | ConvertFrom-Json
    } catch {
        throw "remote Gemma Forge status did not return JSON: $($_.Exception.Message); output=$text"
    }
}

function Assert-StatusContract {
    param(
        [object]$Status,
        [string]$Name
    )

    if ($Status.contract_version -ne "smartsteam.remote-gemma-chain.status.v1") {
        throw "$Name contract_version mismatch: $($Status.contract_version)"
    }
    if (-not $Status.read_only -or $Status.starts_process -or $Status.sends_prompt -or $Status.touches_remote) {
        throw "$Name status contract flags are wrong"
    }
    if (-not $Status.remote_probe_skipped) {
        throw "$Name must skip remote probing"
    }
}

$plainStatus = Invoke-StatusJson `
    -LocalModelPort 65511 `
    -BackendPort 65512 `
    -LabPort 65513 `
    -RemoteModelPort 65514
Assert-StatusContract -Status $plainStatus -Name "plain"
if (@($plainStatus.model_pool.required_roles).Count -ne 0) {
    throw "plain status should not require helper roles by default"
}
if ($plainStatus.readiness.ready -ne $false) {
    throw "plain status should be not-ready on closed selftest ports"
}

$mac32Status = Invoke-StatusJson `
    -LocalModelPort 65521 `
    -BackendPort 65522 `
    -LabPort 65523 `
    -RemoteModelPort 65524 `
    -UseMac32GBModelPool
Assert-StatusContract -Status $mac32Status -Name "mac32gb"
$requiredRoles = @($mac32Status.model_pool.required_roles)
$expectedRoles = @("summary", "router", "review", "index", "test-gate")
foreach ($role in $expectedRoles) {
    if ($role -notin $requiredRoles) {
        throw "mac32gb status did not preserve required helper role=$role"
    }
}
if ($mac32Status.model_pool.required_roles_ready -ne $false) {
    throw "mac32gb status should mark required helper roles not ready on closed selftest ports"
}

Write-Host "smartsteam_remote_gemma_forge_status_selftest=PASS"
Write-Host "read_only=$($plainStatus.read_only) starts_process=$($plainStatus.starts_process) sends_prompt=$($plainStatus.sends_prompt) touches_remote=$($plainStatus.touches_remote)"
Write-Host "plain_required_roles=$(@($plainStatus.model_pool.required_roles).Count)"
Write-Host "mac32gb_required_roles=$($requiredRoles -join ',')"
