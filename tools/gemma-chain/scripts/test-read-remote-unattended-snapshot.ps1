param(
    [string]$ScriptPath = (Join-Path $PSScriptRoot "read-remote-unattended-snapshot.ps1")
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
        [int]$Depth = 12
    )

    $parent = Split-Path -Parent $Path
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }

    $Value | ConvertTo-Json -Depth $Depth | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Invoke-Snapshot {
    param(
        [string]$RepoRoot,
        [string[]]$AdditionalArgs = @()
    )

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -RepoRoot $RepoRoot @AdditionalArgs -Json
    if ($LASTEXITCODE -ne 0) {
        throw "snapshot script exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

function New-FixtureRepo {
    $root = Join-Path ([System.IO.Path]::GetTempPath()) ("gemma-snapshot-selftest-" + [System.Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $root -Force | Out-Null
    return $root
}

function Write-HealthyFixture {
    param([string]$Root)

    Write-JsonFixture -Path (Join-Path $Root "target\remote-gemma-chain\model-cache-status.json") -Value ([pscustomobject]@{
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

    Write-JsonFixture -Path (Join-Path $Root "target\remote-gemma-chain\status-with-model-cache.json") -Value ([pscustomobject]@{
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
            capacity = [pscustomobject]@{
                recommendation = "hold_or_add_optional_test_gate_if_memory_pressure_green"
            }
            workers = @(
                [pscustomobject]@{ role = "quality"; port = 8686; status = "healthy"; ready = $true; context_window = 262144; default_max_tokens = 262144; runtime_backend = "llama.cpp"; runtime_device = "metal"; runtime_accelerator = "metal"; gpu_layers = 999; model_cache_ok = $true },
                [pscustomobject]@{ role = "summary"; port = 8687; status = "healthy"; ready = $true; context_window = 8192; default_max_tokens = 768; runtime_backend = "llama.cpp"; runtime_device = "metal"; runtime_accelerator = "metal"; gpu_layers = 999; model_cache_ok = $true },
                [pscustomobject]@{ role = "review"; port = 8688; status = "healthy"; ready = $true; context_window = 4096; default_max_tokens = 1024; runtime_backend = "llama.cpp"; runtime_device = "metal"; runtime_accelerator = "metal"; gpu_layers = 999; model_cache_ok = $true },
                [pscustomobject]@{ role = "router"; port = 8689; status = "healthy"; ready = $true; context_window = 4096; default_max_tokens = 512; runtime_backend = "llama.cpp"; runtime_device = "metal"; runtime_accelerator = "metal"; gpu_layers = 999; model_cache_ok = $true },
                [pscustomobject]@{ role = "test-gate"; port = 8688; status = "healthy"; ready = $true; context_window = 4096; default_max_tokens = 768; runtime_backend = "llama.cpp"; runtime_device = "metal"; runtime_accelerator = "metal"; gpu_layers = 999; model_cache_ok = $true },
                [pscustomobject]@{ role = "index"; port = 8690; status = "healthy"; ready = $true; context_window = 8192; default_max_tokens = 512; runtime_backend = "llama.cpp"; runtime_device = "metal"; runtime_accelerator = "metal"; gpu_layers = 999; model_cache_ok = $true }
            )
        }
    })

    Write-JsonFixture -Path (Join-Path $Root "target\remote-gemma-unattended\evolution-report.json") -Value ([pscustomobject]@{
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
    @(
        ([pscustomobject]@{ round = 1; case = "smartsteam-evolution-loop-0001"; success = $true; runtime_model = "google/gemma-4-12B-it"; runtime_tokens = 669; elapsed_ms = 59307; validation_checked = $false; validation_passed = $null; validation_command_preview = $null; self_improve_passed = $true } | ConvertTo-Json -Compress),
        ([pscustomobject]@{ round = 4; case = "smartsteam-evolution-loop-0004"; success = $true; runtime_model = "google/gemma-4-12B-it"; runtime_tokens = 853; elapsed_ms = 69396; validation_checked = $true; validation_passed = $true; validation_command_preview = "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"; self_improve_passed = $true } | ConvertTo-Json -Compress)
    ) | Set-Content -LiteralPath $ledgerPath -Encoding UTF8
}

function Write-ObservationFixture {
    param([string]$Root)

    $obsRoot = Join-Path $Root "target\remote-gemma-observations"
    New-Item -ItemType Directory -Path $obsRoot -Force | Out-Null

    Write-JsonFixture -Path (Join-Path $obsRoot "chain-status.json") -Value ([pscustomobject]@{
        classification = "prompt_ready"
        prompt_ready = $true
        machine_summary = [pscustomobject]@{
            read_only = $true
            sends_prompt = $false
            launches_process = $false
        }
    })

    Write-JsonFixture -Path (Join-Path $obsRoot "pool-status.json") -Value ([pscustomobject]@{
        launch_allowed = $true
        capacity = [pscustomobject]@{
            worker_count = 6
            healthy_worker_count = 6
            expansion_allowed = $false
        }
    })

    Write-JsonFixture -Path (Join-Path $obsRoot "status-bundle.json") -Value ([pscustomobject]@{
        read_only = $true
        sends_prompt = $false
        launches_process = $false
    })

    Write-JsonFixture -Path (Join-Path $obsRoot "forge-daemon-status.json") -Value ([pscustomobject]@{
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        evolution_status = [pscustomobject]@{
            daemon = [pscustomobject]@{ running = $false }
        }
        report_gate_preflight = [pscustomobject]@{
            continuation_state = "no_report"
        }
        unattended_start_plan = [pscustomobject]@{
            can_start = $true
        }
    })

    return $obsRoot
}

$healthyRoot = New-FixtureRepo
$brokenRoot = New-FixtureRepo

try {
    Write-HealthyFixture -Root $healthyRoot
    $snapshot = Invoke-Snapshot -RepoRoot $healthyRoot

    Assert-True ($snapshot.contract_version -eq "smartsteam.remote-gemma-unattended.snapshot-summary.v1") "contract version mismatch"
    Assert-True ($snapshot.read_only -eq $true) "snapshot must be read_only"
    Assert-True ($snapshot.starts_process -eq $false) "snapshot must not start processes"
    Assert-True ($snapshot.sends_prompt -eq $false) "snapshot must not send prompts"
    Assert-True ($snapshot.touches_remote -eq $false) "snapshot must not touch remote"
    Assert-True ($snapshot.writes_files -eq $false) "snapshot must not write files"
    Assert-True ($snapshot.summary.evidence_files_present -eq $true) "healthy fixture should have all evidence files"
    Assert-True ($snapshot.summary.parse_ok -eq $true) "healthy fixture should parse"
    Assert-True ($snapshot.summary.evidence_fresh_all -eq $true) "healthy fixture should be fresh"
    Assert-True ($snapshot.model_cache.ok_count -eq 5) "healthy fixture should report five ok models"
    Assert-True ($snapshot.model_cache.copy_needed_count -eq 0) "healthy fixture should not need copy"
    Assert-True ($snapshot.model_cache.remote_error_count -eq 0) "healthy fixture should not have remote errors"
    Assert-True ($snapshot.chain.ready -eq $true) "healthy fixture chain should be ready"
    Assert-True ($snapshot.model_pool.worker_count -eq 6) "healthy fixture should have six workers"
    Assert-True ($snapshot.model_pool.healthy_worker_count -eq 6) "healthy fixture should have six healthy workers"
    Assert-True ($snapshot.unattended.validation -eq "3/3") "validation summary mismatch"
    Assert-True ($snapshot.unattended.self_improve -eq "4/4") "self-improve summary mismatch"
    Assert-True ($snapshot.latest_ledger.round -eq 4) "latest ledger round mismatch"
    Assert-True ($snapshot.latest_ledger.validation_passed -eq $true) "latest ledger validation should pass"
    Assert-True ($snapshot.authorization.can_authorize_daemon -eq $false) "summary script must not authorize daemon"
    Assert-True ($snapshot.authorization.can_authorize_launch -eq $false) "summary script must not authorize launch"
    Assert-True ($snapshot.authorization.can_authorize_prompt -eq $false) "summary script must not authorize prompt"
    Assert-True ($snapshot.authorization.can_authorize_ssh -eq $false) "summary script must not authorize ssh"
    Assert-True (@($snapshot.residency_gaps | Where-Object { $_ -eq "daemon_status_not_rechecked" }).Count -eq 1) "residency gaps should include daemon recheck"
    Assert-True (@($snapshot.residency_evidence_checklist).Count -ge 6) "residency checklist should be present"
    Assert-True (@($snapshot.evidence_checklist).Count -eq @($snapshot.residency_evidence_checklist).Count) "evidence_checklist alias should match residency checklist"
    foreach ($gap in @($snapshot.residency_gaps)) {
        Assert-True (@($snapshot.residency_evidence_checklist | Where-Object { $_.gap_id -eq $gap }).Count -eq 1) "gap $gap should have one checklist item"
    }
    Assert-True (@($snapshot.residency_evidence_checklist | Where-Object {
        [string]::IsNullOrWhiteSpace([string]$_.status) -or
        [string]::IsNullOrWhiteSpace([string]$_.required_evidence) -or
        [string]::IsNullOrWhiteSpace([string]$_.proof_source) -or
        [string]::IsNullOrWhiteSpace([string]$_.safe_command_id)
    }).Count -eq 0) "each checklist item should have status, required_evidence, proof_source, and safe_command_id"
    Assert-True (@($snapshot.residency_evidence_checklist | Where-Object { $_.blocks_authorization -ne $true }).Count -eq 0) "checklist items should block authorization until independently proven"
    Assert-True (@($snapshot.safe_next_read_only_commands).Count -ge 8) "safe next commands should be present"
    Assert-True (@($snapshot.safe_next_read_only_commands | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "safe next commands must keep read-only contract"
    Assert-True (@($snapshot.safe_next_read_only_commands | Where-Object { $_.command -match '\bsmoke\b|\bStart\b|\bssh\b|ssh\.exe|plink|Start-Process|forge_cli_prompt|web_lab_prompt|backend_cli_direct_prompt|evolution_loop_prompt_round|model_pool_launch' }).Count -eq 0) "safe next commands must not include prompt, launch, or SSH actions"
    Assert-True (@($snapshot.safe_next_read_only_commands | Where-Object { $_.id -eq "forge_daemon_start_check" -and $_.command -match "-StartCheck" }).Count -eq 1) "safe next commands should include dry-run StartCheck"
    Assert-True (@($snapshot.safe_next_read_only_commands | Where-Object { $_.id -eq "remote_resource_artifact_check" -and $_.command -match "read-remote-resource-window\.ps1 -Json" }).Count -eq 1) "remote resource safe command should use the local resource-window reader"
    foreach ($item in @($snapshot.residency_evidence_checklist)) {
        Assert-True (@($snapshot.safe_next_read_only_commands | Where-Object { $_.id -eq $item.safe_command_id }).Count -eq 1) "checklist safe command $($item.safe_command_id) should exist"
    }
    Assert-True ($snapshot.residency_decision.classification -eq "blocked_missing_residency_evidence") "healthy fixture should still be blocked pending residency evidence"
    Assert-True ($snapshot.residency_decision.can_proceed_to_resident_loop -eq $false) "snapshot must not allow resident loop"
    Assert-True ($snapshot.residency_decision.read_only_evidence_collection_only -eq $true) "snapshot should permit only read-only evidence collection"
    Assert-True (@($snapshot.residency_decision.recommended_next_command_ids).Count -ge 8) "residency decision should expose recommended command ids"
    Assert-True ($snapshot.consumer_contract.contract_version -eq "smartsteam.remote-gemma-unattended.consumer-projection.v1") "consumer contract version mismatch"
    Assert-True ($snapshot.consumer_contract.fail_closed_default -eq $true) "consumer contract must default fail-closed"
    Assert-True ($snapshot.consumer_contract.allowed_requires_external_gates -eq $true) "consumer contract must require external gates"
    Assert-True (@($snapshot.consumer_contract.required_fields | Where-Object { $_ -eq "current_allowed" }).Count -eq 1) "consumer contract should require current_allowed"
    Assert-True (@($snapshot.consumer_contract.required_fields | Where-Object { $_ -eq "safe_command_id" }).Count -eq 1) "consumer contract should require safe_command_id"
    Assert-True (@($snapshot.consumer_projection).Count -ge 7) "consumer projection should include integration surfaces"
    Assert-True (@($snapshot.consumer_projection).Count -eq @($snapshot.consumer_contract.consumer_ids).Count) "consumer contract ids should match projection count"
    Assert-True (@($snapshot.consumer_projection | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "consumer projection must fail closed"
    Assert-True (@($snapshot.consumer_projection | Where-Object { $_.entrypoint_kind -eq "prompt" -and $_.downstream_sends_prompt -ne $true }).Count -eq 0) "prompt consumers should be marked prompt-producing"
    Assert-True (@($snapshot.consumer_projection | Where-Object { $_.entrypoint_kind -eq "launch" -and $_.downstream_launches_process -ne $true }).Count -eq 0) "launch consumers should be marked process-launching"
    Assert-True (@($snapshot.consumer_projection | Where-Object { $_.entrypoint_kind -eq "ssh" -and $_.downstream_touches_remote -ne $true }).Count -eq 0) "ssh consumers should be marked remote-touching"
    foreach ($consumer in @($snapshot.consumer_projection)) {
        Assert-True (@($snapshot.safe_next_read_only_commands | Where-Object { $_.id -eq $consumer.safe_command_id }).Count -eq 1) "consumer safe command $($consumer.safe_command_id) should exist"
        Assert-True (@($snapshot.consumer_contract.consumer_ids | Where-Object { $_ -eq $consumer.id }).Count -eq 1) "consumer contract should list $($consumer.id)"
        foreach ($field in @($snapshot.consumer_contract.required_fields)) {
            Assert-True ($null -ne $consumer.PSObject.Properties[$field]) "consumer $($consumer.id) missing required field $field"
        }
    }
    Assert-True (@($snapshot.consumer_projection | Where-Object { $_.id -eq "web_lab_prompt" -and $_.current_allowed -eq $false }).Count -eq 1) "Web Lab prompt should be blocked"
    Assert-True (@($snapshot.consumer_projection | Where-Object { $_.id -eq "model_pool_launch" -and $_.current_allowed -eq $false }).Count -eq 1) "model pool launch should be blocked"

    $obsRoot = Write-ObservationFixture -Root $healthyRoot
    $observedSnapshot = Invoke-Snapshot -RepoRoot $healthyRoot -AdditionalArgs @("-LocalObservationDir", $obsRoot)
    Assert-True ($observedSnapshot.local_observation.contract_version -eq "smartsteam.remote-gemma-unattended.local-observation.v1") "local observation contract mismatch"
    Assert-True ($observedSnapshot.local_observation.read_only -eq $true) "local observation must be read-only"
    Assert-True ($observedSnapshot.local_observation.starts_process -eq $false) "local observation must not start processes"
    Assert-True ($observedSnapshot.local_observation.sends_prompt -eq $false) "local observation must not send prompts"
    Assert-True ($observedSnapshot.local_observation.summary.complete_parse_ok -eq $true) "local observation should parse all expected files"
    Assert-True ($observedSnapshot.local_observation.summary.single_sample_only -eq $true) "local observation should be marked single sample"
    Assert-True ($observedSnapshot.local_observation.summary.window_sample_count -eq 1) "local observation sample count mismatch"
    Assert-True ($observedSnapshot.local_observation.summary.continuous_window_present -eq $false) "local observation must not claim continuous window"
    Assert-True ($observedSnapshot.local_observation.summary.chain_classification -eq "prompt_ready") "local chain observation mismatch"
    Assert-True ($observedSnapshot.local_observation.summary.pool_worker_count -eq 6) "local pool observation worker count mismatch"
    Assert-True ($observedSnapshot.local_observation.summary.pool_capacity_expansion_allowed -eq $false) "local pool expansion observation mismatch"
    Assert-True ($observedSnapshot.local_observation.summary.bundle_read_only -eq $true) "status bundle observation must be read-only"
    Assert-True ($observedSnapshot.local_observation.summary.daemon_running -eq $false) "daemon observation running mismatch"
    Assert-True ($observedSnapshot.local_observation.summary.report_gate_continuation_state -eq "no_report") "daemon observation report gate mismatch"
    Assert-True (@($observedSnapshot.residency_evidence_checklist | Where-Object { $_.id -eq "daemon_status" -and $_.status -eq "observed_once_insufficient" }).Count -eq 1) "daemon checklist should reflect one observation"
    Assert-True (@($observedSnapshot.residency_evidence_checklist | Where-Object { $_.id -eq "active_daemon_presence" -and $_.status -eq "observed_once_insufficient" }).Count -eq 1) "daemon presence checklist should reflect one observation"
    Assert-True (@($observedSnapshot.residency_evidence_checklist | Where-Object { $_.id -eq "prompt_launch_gates" -and $_.status -eq "observed_once_insufficient" }).Count -eq 1) "prompt/launch checklist should reflect one observation"
    Assert-True (@($observedSnapshot.residency_evidence_checklist | Where-Object { $_.id -eq "continuous_port_health" -and $_.status -eq "single_sample_observed_window_missing" }).Count -eq 1) "port health checklist should require a window"
    Assert-True (@($observedSnapshot.residency_evidence_checklist | Where-Object { $_.id -eq "remote_resource_headroom" -and $_.safe_command_id -eq "remote_resource_artifact_check" }).Count -eq 1) "remote resource checklist should point to local artifact reader"
    Assert-True (@($observedSnapshot.residency_gaps | Where-Object { $_ -eq "daemon_status_not_rechecked" }).Count -eq 1) "one observation should not remove daemon gap"
    Assert-True ($observedSnapshot.authorization.can_authorize_prompt -eq $false) "local observations must not authorize prompt"
    Assert-True (@($observedSnapshot.consumer_projection | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "local observations must keep consumers blocked"

    Write-JsonFixture -Path (Join-Path $brokenRoot "target\remote-gemma-chain\model-cache-status.json") -Value ([pscustomobject]@{ all_ok = $false; models = @() })
    $brokenChainPath = Join-Path $brokenRoot "target\remote-gemma-chain\status-with-model-cache.json"
    $brokenChainParent = Split-Path -Parent $brokenChainPath
    if (-not (Test-Path -LiteralPath $brokenChainParent -PathType Container)) {
        New-Item -ItemType Directory -Path $brokenChainParent -Force | Out-Null
    }
    "{bad json" | Set-Content -LiteralPath $brokenChainPath -Encoding UTF8

    $brokenSnapshot = Invoke-Snapshot -RepoRoot $brokenRoot
    Assert-True ($brokenSnapshot.read_only -eq $true) "broken snapshot should keep read_only contract"
    Assert-True ($brokenSnapshot.starts_process -eq $false) "broken snapshot should not start processes"
    Assert-True ($brokenSnapshot.sends_prompt -eq $false) "broken snapshot should not send prompts"
    Assert-True ($brokenSnapshot.summary.evidence_files_present -eq $false) "broken fixture should report missing evidence"
    Assert-True ($brokenSnapshot.summary.parse_ok -eq $false) "broken fixture should report parse failure"
    Assert-True ($brokenSnapshot.summary.evidence_fresh_all -eq $false) "broken fixture should not be fresh"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$brokenSnapshot.evidence.status_with_model_cache.parse_error)) "broken fixture should expose parse error"
    Assert-True (@($brokenSnapshot.residency_gaps | Where-Object { $_ -eq "fresh_status_snapshot_missing_or_stale" }).Count -eq 1) "broken fixture should require fresh status"
    Assert-True (@($brokenSnapshot.residency_evidence_checklist | Where-Object { $_.gap_id -eq "fresh_status_snapshot_missing_or_stale" -and $_.status -eq "missing_or_stale" }).Count -eq 1) "broken fixture should explain stale/missing evidence"
    Assert-True ($brokenSnapshot.residency_decision.classification -eq "blocked_model_cache") "broken fixture with bad cache should classify model cache block first"
    Assert-True ($brokenSnapshot.consumer_contract.fail_closed_default -eq $true) "broken fixture consumer contract must fail closed"
    Assert-True (@($brokenSnapshot.consumer_projection | Where-Object { $_.current_allowed -ne $false }).Count -eq 0) "broken fixture consumer projection must fail closed"
    Assert-True (@($brokenSnapshot.safe_next_read_only_commands | Where-Object { $_.read_only -ne $true -or $_.starts_process -ne $false -or $_.sends_prompt -ne $false -or $_.touches_remote -ne $false -or $_.writes_files -ne $false }).Count -eq 0) "broken fixture safe commands must keep read-only contract"

    Write-Host "read-remote-unattended-snapshot selftest passed"
} finally {
    if (Test-Path -LiteralPath $healthyRoot) {
        Remove-Item -LiteralPath $healthyRoot -Recurse -Force
    }
    if (Test-Path -LiteralPath $brokenRoot) {
        Remove-Item -LiteralPath $brokenRoot -Recurse -Force
    }
}
