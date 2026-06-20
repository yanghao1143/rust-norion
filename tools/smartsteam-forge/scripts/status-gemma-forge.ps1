param(
    [string]$RepoRoot = "D:\rust-norion",
    [string]$RemoteHost = "192.168.10.11",
    [string]$RemoteUser = "xinghuan",
    [string]$RunDir = "",
    [int]$MistralPort = 8686,
    [int]$BackendPort = 7979,
    [int]$LabPort = 8789,
    [switch]$Raw,
    [switch]$NoModelPool,
    [switch]$NoGpu,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$statusForge = Join-Path $scriptDir "status-forge.ps1"

if ($Help) {
    Write-Host "Show SmartSteam Gemma Forge status using the recommended remote-chain ports."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\status-gemma-forge.cmd"
    Write-Host "  .\tools\smartsteam-forge\status-gemma-forge.cmd -Raw"
    Write-Host "  .\tools\smartsteam-forge\status-gemma-forge.cmd -NoGpu"
    Write-Host ""
    Write-Host "Defaults:"
    Write-Host "  model   127.0.0.1:$MistralPort"
    Write-Host "  backend 127.0.0.1:$BackendPort"
    Write-Host "  web lab 127.0.0.1:$LabPort"
    return
}

$statusArgs = @{
    RepoRoot = $RepoRoot
    RemoteHost = $RemoteHost
    RemoteUser = $RemoteUser
    MistralPort = $MistralPort
    BackendPort = $BackendPort
    LabPort = $LabPort
}
if (-not [string]::IsNullOrWhiteSpace($RunDir)) {
    $statusArgs.RunDir = $RunDir
}
if ($Raw) { $statusArgs.Raw = $true }
if ($NoModelPool) { $statusArgs.NoModelPool = $true }
if ($NoGpu) { $statusArgs.NoGpu = $true }

& $statusForge @statusArgs
