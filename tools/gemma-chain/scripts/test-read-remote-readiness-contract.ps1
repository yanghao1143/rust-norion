param(
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-readiness-contract.ps1")
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
    $item = Get-Item -LiteralPath $Path
    $item.LastWriteTime = $LastWriteTime
    $item.LastWriteTimeUtc = $LastWriteTime.ToUniversalTime()
}

function Write-SnapshotFixture {
    param([string]$Root)

    $now = Get-Date
    Write-JsonFixture -Path (Join-Path $Root "target\remote-gemma-chain\model-cache-status.json") -LastWriteTime $now -Value ([pscustomobject]@{
        all_ok = $true
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        writes_files = $true
        models = @(
            [pscustomobject]@{ role = "quality"; ok = $true; copy_needed = $false; remote_error = "" },
            [pscustomobject]@{ role = "summary"; ok = $true; copy_needed = $false; remote_error = "" },
            [pscustomobject]@{ role = "review"; ok = $true; copy_needed = $false; remote_error = "" },
            [pscustomobject]@{ role = "router"; ok = $true; copy_needed = $false; remote_error = "" },
            [pscustomobject]@{ role = "index"; ok = $true; copy_needed = $false; remote_error = "" }
        )
    })

    Write-JsonFixture -Path (Join-Path $Root "target\remote-gemma-chain\status-with-model-cache.json") -LastWriteTime $now -Value ([pscustomobject]@{
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        remote_probe_skipped = $true
        readiness = [pscustomobject]@{
            ready = $true
            model_api = $true
            backend = $true
            web_lab = $true
        }
        model_pool = [pscustomobject]@{
            required_roles = @("summary", "router", "review", "index", "test-gate")
            missing_required_roles = @()
            capacity = [pscustomobject]@{ recommendation = "hold_or_add_optional_test_gate_if_memory_pressure_green" }
            workers = @(
                [pscustomobject]@{ role = "quality"; port = 8686; status = "healthy"; ready = $true },
                [pscustomobject]@{ role = "summary"; port = 8687; status = "healthy"; ready = $true },
                [pscustomobject]@{ role = "review"; port = 8688; status = "healthy"; ready = $true },
                [pscustomobject]@{ role = "router"; port = 8689; status = "healthy"; ready = $true },
                [pscustomobject]@{ role = "test-gate"; port = 8688; status = "healthy"; ready = $true },
                [pscustomobject]@{ role = "index"; port = 8690; status = "healthy"; ready = $true }
            )
        }
    })

    Write-JsonFixture -Path (Join-Path $Root "target\remote-gemma-unattended\evolution-report.json") -LastWriteTime $now -Value ([pscustomobject]@{
        rounds = 4
        success = 4
        failures = 0
        success_rate = 100.0
        runtime_tokens = [pscustomobject]@{ total = 2877 }
        validation = [pscustomobject]@{ passed = 3; checked = 3 }
        self_improve = [pscustomobject]@{ passed = 4; checked = 4 }
        report_gate = [pscustomobject]@{ passed = $true }
        recent_failures = @()
        test_gate = [pscustomobject]@{
            latest_verdict = "pass"
            latest_validation_command = "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"
        }
    })

    $ledgerPath = Join-Path $Root "target\remote-gemma-unattended\evolution-ledger.jsonl"
    $ledgerParent = Split-Path -Parent $ledgerPath
    if (-not (Test-Path -LiteralPath $ledgerParent -PathType Container)) {
        New-Item -ItemType Directory -Path $ledgerParent -Force | Out-Null
    }
    ([pscustomobject]@{
        round = 4
        case = "smartsteam-evolution-loop-0004"
        success = $true
        runtime_model = "google/gemma-4-12B-it"
        runtime_tokens = 853
        elapsed_ms = 69396
        validation_checked = $true
        validation_passed = $true
        validation_command_preview = "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"
        self_improve_passed = $true
    } | ConvertTo-Json -Compress) | Set-Content -LiteralPath $ledgerPath -Encoding UTF8
    $ledgerItem = Get-Item -LiteralPath $ledgerPath
    $ledgerItem.LastWriteTime = $now
    $ledgerItem.LastWriteTimeUtc = $now.ToUniversalTime()
}

function Write-ObservationWindowFixture {
    param([string]$Root)

    $baseTime = Get-Date
    foreach ($index in 1..3) {
        $dir = Join-Path $Root ("target\remote-gemma-observation-window\sample-{0:D3}" -f $index)
        $time = $baseTime.AddMinutes(-30 + ($index * 10))
        Write-JsonFixture -Path (Join-Path $dir "chain-status.json") -LastWriteTime $time -Value ([pscustomobject]@{
            classification = "prompt_ready"
            prompt_ready = $true
            machine_summary = [pscustomobject]@{
                read_only = $true
                sends_prompt = $false
                launches_process = $false
            }
        })
        Write-JsonFixture -Path (Join-Path $dir "pool-status.json") -LastWriteTime $time -Value ([pscustomobject]@{
            launch_allowed = $true
            capacity = [pscustomobject]@{
                worker_count = 6
                healthy_worker_count = 6
                expansion_allowed = $false
            }
        })
        Write-JsonFixture -Path (Join-Path $dir "status-bundle.json") -LastWriteTime $time -Value ([pscustomobject]@{
            read_only = $true
            sends_prompt = $false
            launches_process = $false
        })
        Write-JsonFixture -Path (Join-Path $dir "forge-daemon-status.json") -LastWriteTime $time -Value ([pscustomobject]@{
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
}

function Write-ResourceWindowFixture {
    param([string]$Root)

    $baseTime = Get-Date
    foreach ($index in 1..3) {
        $dir = Join-Path $Root ("target\remote-gemma-resource-window\sample-{0:D3}" -f $index)
        $time = $baseTime.AddMinutes(-30 + ($index * 10))
        Write-JsonFixture -Path (Join-Path $dir "remote-resource-status.json") -LastWriteTime $time -Value ([pscustomobject]@{
            read_only = $true
            starts_process = $false
            sends_prompt = $false
            writes_model_weights = $false
            approved_owner_flow = $true
            summary = [pscustomobject]@{
                memory_available_gb = 24
                metal_available = $true
            }
        })
    }
}

function Invoke-ReadinessContract {
    param([string]$InputRepoRoot)

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $InputRepoRoot -Json
    if ($LASTEXITCODE -ne 0) {
        throw "readiness contract reader exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

$fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("gemma-readiness-selftest-" + [System.Guid]::NewGuid().ToString("N"))

try {
    New-Item -ItemType Directory -Path $fixtureRoot -Force | Out-Null
    Write-SnapshotFixture -Root $fixtureRoot
    Write-ObservationWindowFixture -Root $fixtureRoot
    Write-ResourceWindowFixture -Root $fixtureRoot

    $contract = Invoke-ReadinessContract -InputRepoRoot $fixtureRoot
    Assert-True ($contract.contract_version -eq "smartsteam.remote-gemma-unattended.readiness-contract.v1") "readiness contract version mismatch"
    Assert-True ($contract.read_only -eq $true) "readiness contract must be read-only"
    Assert-True ($contract.starts_process -eq $false) "readiness contract must not start processes"
    Assert-True ($contract.sends_prompt -eq $false) "readiness contract must not send prompts"
    Assert-True ($contract.touches_remote -eq $false) "readiness contract must not touch remote"
    Assert-True ($contract.writes_files -eq $false) "readiness contract must not write files"
    Assert-True ($contract.writes_model_weights -eq $false) "readiness contract must not write model weights"
    Assert-True ($contract.summary.evidence_fresh_all -eq $true) ("fixture evidence should be fresh: repo_root=$($contract.repo_root) fixtureRoot=$fixtureRoot evidence=" + (($contract.source_status.snapshot.evidence | ConvertTo-Json -Compress -Depth 5)))
    Assert-True ($contract.summary.continuous_window_present -eq $true) "fixture observation window should be present"
    Assert-True ($contract.summary.resource_window_present -eq $true) "fixture resource window should be present"
    Assert-True (@($contract.summary.missing_evidence).Count -eq 0) "complete fixture should not have missing evidence"
    Assert-True (@($contract.summary.pending_external_gates | Where-Object { $_ -eq "residency_external_gate" }).Count -eq 1) "complete fixture should still require external residency gate"
    Assert-True (@($contract.pending_external_gate_actions | Where-Object { $_.id -eq "residency_external_gate" }).Count -eq 1) "external residency gate action should be present"
    Assert-True ($contract.summary.can_support_external_residency_review -eq $true) "complete fixture should support external residency review"
    Assert-True ($contract.authorization.can_authorize_daemon -eq $false) "readiness contract must not authorize daemon"
    Assert-True ($contract.authorization.can_authorize_launch -eq $false) "readiness contract must not authorize launch"
    Assert-True ($contract.authorization.can_authorize_prompt -eq $false) "readiness contract must not authorize prompt"
    Assert-True ($contract.authorization.can_authorize_ssh -eq $false) "readiness contract must not authorize ssh"
    Assert-True (@($contract.consumer_projection | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "consumer projection must remain blocked"

    Write-Host "read-remote-readiness-contract selftest passed"
} finally {
    if (Test-Path -LiteralPath $fixtureRoot) {
        Remove-Item -LiteralPath $fixtureRoot -Recurse -Force
    }
}
