param(
    [string]$RepoRoot = "D:\rust-norion",
    [string]$RemoteHost = "192.168.10.11",
    [string]$RemoteUser = "xinghuan",
    [string]$IdentityFile = "$env:USERPROFILE\.ssh\smartsteam_mac_ed25519",
    [string]$RemoteRoot = "/Users/xinghuan/smartsteam-model-box",
    [int]$RemoteModelPort = 8686,
    [int]$LocalModelPort = 8686,
    [string]$RunDir = "",
    [switch]$KeepRemoteModel,
    [switch]$KeepPoolWorkers,
    [switch]$DryRun,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Stop the remote Gemma chain started by start-remote-gemma-chain.cmd."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\stop-remote-gemma-chain.cmd -DryRun"
    Write-Host "  .\tools\smartsteam-forge\stop-remote-gemma-chain.cmd"
    Write-Host "  .\tools\smartsteam-forge\stop-remote-gemma-chain.cmd -KeepRemoteModel"
    Write-Host "  .\tools\smartsteam-forge\stop-remote-gemma-chain.cmd -KeepPoolWorkers"
    Write-Host ""
    Write-Host "Notes:"
    Write-Host "  -KeepRemoteModel stops only local tunnel/backend/Web Lab and leaves all remote llama-server workers running."
    Write-Host "  -KeepPoolWorkers leaves remote summary/router/review/index/test-gate workers running while stopping the quality worker."
    Write-Host "  PID files are checked against process name/command line before stopping."
    return
}

if ([string]::IsNullOrWhiteSpace($RunDir)) {
    $RunDir = Join-Path $RepoRoot "target\remote-gemma-chain"
}

function Stop-PidFile {
    param(
        [string]$Path,
        [string]$ExpectedName,
        [string[]]$ExpectedCommandText
    )

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }

    $raw = (Get-Content -LiteralPath $Path -ErrorAction SilentlyContinue | Select-Object -First 1)
    [int]$processId = 0
    if (-not [int]::TryParse($raw, [ref]$processId)) {
        Remove-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
        return
    }

    $process = Get-CimInstance Win32_Process -Filter "ProcessId = $processId" -ErrorAction SilentlyContinue
    if ($null -ne $process) {
        $nameMatches = [string]::IsNullOrWhiteSpace($ExpectedName) -or $process.Name -eq $ExpectedName
        $commandMatches = $true
        foreach ($expectedText in @($ExpectedCommandText)) {
            if (-not [string]::IsNullOrWhiteSpace($expectedText) -and -not ([string]$process.CommandLine).Contains($expectedText)) {
                $commandMatches = $false
                break
            }
        }
        if (-not ($nameMatches -and $commandMatches)) {
            throw "refusing to stop pid $processId from $Path because it no longer looks like this remote Gemma chain process"
        }
        if ($DryRun) {
            Write-Host "would stop local pid $processId from $Path"
            return
        }
        Write-Host "stopping local pid $processId from $Path"
        Stop-Process -Id $processId -Force
    }
    if (-not $DryRun) {
        Remove-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
    }
}

Stop-PidFile (Join-Path $RunDir "rustgpt-lab.pid") "rustgpt-lab.exe" "rustgpt-lab"
Stop-PidFile (Join-Path $RunDir "rust-norion.pid") "rust-norion.exe" "remote-gemma-chain"
$expectedForward = "$LocalModelPort`:127.0.0.1:$RemoteModelPort"
$expectedTarget = "$RemoteUser@$RemoteHost"
Stop-PidFile (Join-Path $RunDir "ssh-tunnel.pid") "ssh.exe" @("-L", $expectedForward, $expectedTarget)
foreach ($worker in @(
    [pscustomobject]@{ Role = "summary"; Port = 8687 },
    [pscustomobject]@{ Role = "review"; Port = 8688 },
    [pscustomobject]@{ Role = "router"; Port = 8689 },
    [pscustomobject]@{ Role = "test-gate"; Port = 8688 },
    [pscustomobject]@{ Role = "index"; Port = 8690 },
    [pscustomobject]@{ Role = "spare"; Port = 8690 }
)) {
    $workerForward = "$($worker.Port)`:127.0.0.1:$($worker.Port)"
    Stop-PidFile (Join-Path $RunDir "ssh-tunnel-$($worker.Role).pid") "ssh.exe" @("-L", $workerForward, $expectedTarget)
}

if (-not $KeepRemoteModel) {
    $target = "$RemoteUser@$RemoteHost"
    $remoteCommand = @'
set -eu
stop_pid_file() {
  LABEL="$1"
  PID="$2"
  if [ ! -f "$PID" ]; then
    return 0
  fi
  OLD_PID="$(cat "$PID" 2>/dev/null || true)"
  if [ -n "$OLD_PID" ] && kill -0 "$OLD_PID" 2>/dev/null; then
    if [ "__DRY_RUN__" = "1" ]; then
      echo "would stop remote $LABEL pid $OLD_PID"
      return 0
    fi
    echo "stopping remote $LABEL pid $OLD_PID"
    kill "$OLD_PID" 2>/dev/null || true
    sleep 1
    if kill -0 "$OLD_PID" 2>/dev/null; then
      kill -9 "$OLD_PID" 2>/dev/null || true
    fi
  fi
  if [ "__DRY_RUN__" != "1" ]; then
    rm -f "$PID"
  fi
}
stop_pid_file "quality llama-server" "__REMOTE_ROOT__/llama-server.pid"
if [ "__KEEP_POOL_WORKERS__" != "1" ]; then
  for ROLE in summary router review index test-gate spare; do
    stop_pid_file "$ROLE worker" "__REMOTE_ROOT__/llama-server-$ROLE.pid"
  done
fi
'@
    $remoteCommand = $remoteCommand.Replace("__REMOTE_ROOT__", $RemoteRoot)
    $remoteCommand = $remoteCommand.Replace("__DRY_RUN__", $(if ($DryRun) { "1" } else { "0" }))
    $remoteCommand = $remoteCommand.Replace("__KEEP_POOL_WORKERS__", $(if ($KeepPoolWorkers) { "1" } else { "0" }))
    & ssh.exe -i $IdentityFile -o BatchMode=yes $target $remoteCommand
    if ($LASTEXITCODE -ne 0) {
        throw "remote ssh command failed with exit code $LASTEXITCODE"
    }
}

if ($DryRun) {
    Write-Host "remote Gemma chain dry-run complete"
} else {
    Write-Host "remote Gemma chain stopped"
}
