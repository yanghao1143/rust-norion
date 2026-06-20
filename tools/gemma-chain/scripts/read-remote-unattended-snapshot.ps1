param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$LocalObservationDir = "",
    [int]$FreshMinutes = 30,
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function ConvertTo-RelativePath {
    param(
        [string]$Root,
        [string]$Path
    )

    $resolvedRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    if ($resolvedPath.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $resolvedPath.Substring($resolvedRoot.Length).TrimStart([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
    }
    return $resolvedPath
}

function Read-JsonFile {
    param([string]$Path)

    $nowUtc = (Get-Date).ToUniversalTime()
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [pscustomobject]@{
            exists = $false
            path = $Path
            last_write_time = $null
            last_write_time_utc = $null
            age_seconds = $null
            parse_error = "missing"
            value = $null
        }
    }

    $item = Get-Item -LiteralPath $Path
    $ageSeconds = [math]::Max(0, [int][math]::Round(($nowUtc - $item.LastWriteTimeUtc).TotalSeconds))
    try {
        $value = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
        $parseError = ""
    } catch {
        $value = $null
        $parseError = $_.Exception.Message
    }

    return [pscustomobject]@{
        exists = $true
        path = $item.FullName
        last_write_time = $item.LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss zzz")
        last_write_time_utc = $item.LastWriteTimeUtc.ToString("o")
        age_seconds = $ageSeconds
        parse_error = $parseError
        value = $value
    }
}

function Read-LedgerTail {
    param([string]$Path)

    $nowUtc = (Get-Date).ToUniversalTime()
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [pscustomobject]@{
            exists = $false
            path = $Path
            last_write_time = $null
            last_write_time_utc = $null
            age_seconds = $null
            line_count = 0
            parse_error = "missing"
            latest = $null
        }
    }

    $item = Get-Item -LiteralPath $Path
    $ageSeconds = [math]::Max(0, [int][math]::Round(($nowUtc - $item.LastWriteTimeUtc).TotalSeconds))
    $lines = @(Get-Content -LiteralPath $Path)
    $latest = $null
    $parseError = ""
    if ($lines.Count -gt 0) {
        try {
            $latest = $lines[-1] | ConvertFrom-Json
        } catch {
            $parseError = $_.Exception.Message
        }
    }

    return [pscustomobject]@{
        exists = $true
        path = $item.FullName
        last_write_time = $item.LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss zzz")
        last_write_time_utc = $item.LastWriteTimeUtc.ToString("o")
        age_seconds = $ageSeconds
        line_count = $lines.Count
        parse_error = $parseError
        latest = $latest
    }
}

function Count-Where {
    param(
        [object[]]$Items,
        [scriptblock]$Predicate
    )

    return @($Items | Where-Object $Predicate).Count
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$freshSeconds = [math]::Max(0, $FreshMinutes * 60)
$modelCachePath = Join-Path $root "target\remote-gemma-chain\model-cache-status.json"
$chainStatusPath = Join-Path $root "target\remote-gemma-chain\status-with-model-cache.json"
$reportPath = Join-Path $root "target\remote-gemma-unattended\evolution-report.json"
$ledgerPath = Join-Path $root "target\remote-gemma-unattended\evolution-ledger.jsonl"

$modelCache = Read-JsonFile -Path $modelCachePath
$chainStatus = Read-JsonFile -Path $chainStatusPath
$report = Read-JsonFile -Path $reportPath
$ledger = Read-LedgerTail -Path $ledgerPath

$localObservationEnabled = -not [string]::IsNullOrWhiteSpace($LocalObservationDir)
$localObservationRoot = $null
$chainObservation = $null
$poolObservation = $null
$bundleObservation = $null
$forgeDaemonObservation = $null
if ($localObservationEnabled) {
    if (Test-Path -LiteralPath $LocalObservationDir -PathType Container) {
        $localObservationRoot = (Resolve-Path -LiteralPath $LocalObservationDir).Path
    } else {
        $localObservationRoot = [System.IO.Path]::GetFullPath($LocalObservationDir)
    }
    $chainObservation = Read-JsonFile -Path (Join-Path $localObservationRoot "chain-status.json")
    $poolObservation = Read-JsonFile -Path (Join-Path $localObservationRoot "pool-status.json")
    $bundleObservation = Read-JsonFile -Path (Join-Path $localObservationRoot "status-bundle.json")
    $forgeDaemonObservation = Read-JsonFile -Path (Join-Path $localObservationRoot "forge-daemon-status.json")
}

$cacheModels = @()
if ($modelCache.value -and $modelCache.value.PSObject.Properties["models"]) {
    $cacheModels = @($modelCache.value.models)
}

$workers = @()
if ($chainStatus.value -and $chainStatus.value.model_pool -and $chainStatus.value.model_pool.PSObject.Properties["workers"]) {
    $workers = @($chainStatus.value.model_pool.workers)
}

$requiredRoles = @()
$missingRequiredRoles = @()
if ($chainStatus.value -and $chainStatus.value.model_pool) {
    if ($chainStatus.value.model_pool.PSObject.Properties["required_roles"]) {
        $requiredRoles = @($chainStatus.value.model_pool.required_roles)
    }
    if ($chainStatus.value.model_pool.PSObject.Properties["missing_required_roles"]) {
        $missingRequiredRoles = @($chainStatus.value.model_pool.missing_required_roles)
    }
}

$latest = $ledger.latest
$snapshot = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.snapshot-summary.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    repo_root = $root
    evidence = [pscustomobject]@{
        model_cache_status = [pscustomobject]@{
            path = ConvertTo-RelativePath -Root $root -Path $modelCachePath
            exists = $modelCache.exists
            last_write_time = $modelCache.last_write_time
            last_write_time_utc = $modelCache.last_write_time_utc
            age_seconds = $modelCache.age_seconds
            fresh = ($modelCache.exists -and $null -ne $modelCache.age_seconds -and $modelCache.age_seconds -le $freshSeconds)
            parse_error = $modelCache.parse_error
        }
        status_with_model_cache = [pscustomobject]@{
            path = ConvertTo-RelativePath -Root $root -Path $chainStatusPath
            exists = $chainStatus.exists
            last_write_time = $chainStatus.last_write_time
            last_write_time_utc = $chainStatus.last_write_time_utc
            age_seconds = $chainStatus.age_seconds
            fresh = ($chainStatus.exists -and $null -ne $chainStatus.age_seconds -and $chainStatus.age_seconds -le $freshSeconds)
            parse_error = $chainStatus.parse_error
        }
        evolution_report = [pscustomobject]@{
            path = ConvertTo-RelativePath -Root $root -Path $reportPath
            exists = $report.exists
            last_write_time = $report.last_write_time
            last_write_time_utc = $report.last_write_time_utc
            age_seconds = $report.age_seconds
            fresh = ($report.exists -and $null -ne $report.age_seconds -and $report.age_seconds -le $freshSeconds)
            parse_error = $report.parse_error
        }
        evolution_ledger = [pscustomobject]@{
            path = ConvertTo-RelativePath -Root $root -Path $ledgerPath
            exists = $ledger.exists
            last_write_time = $ledger.last_write_time
            last_write_time_utc = $ledger.last_write_time_utc
            age_seconds = $ledger.age_seconds
            fresh = ($ledger.exists -and $null -ne $ledger.age_seconds -and $ledger.age_seconds -le $freshSeconds)
            parse_error = $ledger.parse_error
            line_count = $ledger.line_count
        }
    }
    model_cache = [pscustomobject]@{
        all_ok = if ($modelCache.value) { [bool]$modelCache.value.all_ok } else { $false }
        read_only = if ($modelCache.value) { [bool]$modelCache.value.read_only } else { $false }
        starts_process = if ($modelCache.value) { [bool]$modelCache.value.starts_process } else { $false }
        sends_prompt = if ($modelCache.value) { [bool]$modelCache.value.sends_prompt } else { $false }
        writes_files = if ($modelCache.value) { [bool]$modelCache.value.writes_files } else { $false }
        model_count = $cacheModels.Count
        ok_count = Count-Where -Items $cacheModels -Predicate { $_.ok -eq $true }
        copy_needed_count = Count-Where -Items $cacheModels -Predicate { $_.copy_needed -eq $true }
        remote_error_count = Count-Where -Items $cacheModels -Predicate { -not [string]::IsNullOrWhiteSpace([string]$_.remote_error) }
        roles = @($cacheModels | ForEach-Object { $_.role })
    }
    chain = [pscustomobject]@{
        ready = if ($chainStatus.value -and $chainStatus.value.readiness) { [bool]$chainStatus.value.readiness.ready } else { $false }
        read_only = if ($chainStatus.value) { [bool]$chainStatus.value.read_only } else { $false }
        starts_process = if ($chainStatus.value) { [bool]$chainStatus.value.starts_process } else { $false }
        sends_prompt = if ($chainStatus.value) { [bool]$chainStatus.value.sends_prompt } else { $false }
        touches_remote = if ($chainStatus.value) { [bool]$chainStatus.value.touches_remote } else { $false }
        remote_probe_skipped = if ($chainStatus.value) { [bool]$chainStatus.value.remote_probe_skipped } else { $false }
        model_api = if ($chainStatus.value -and $chainStatus.value.readiness) { [bool]$chainStatus.value.readiness.model_api } else { $false }
        backend = if ($chainStatus.value -and $chainStatus.value.readiness) { [bool]$chainStatus.value.readiness.backend } else { $false }
        web_lab = if ($chainStatus.value -and $chainStatus.value.readiness) { [bool]$chainStatus.value.readiness.web_lab } else { $false }
        required_roles = $requiredRoles
        missing_required_roles = $missingRequiredRoles
    }
    model_pool = [pscustomobject]@{
        worker_count = $workers.Count
        healthy_worker_count = Count-Where -Items $workers -Predicate { $_.status -eq "healthy" -or $_.ready -eq $true }
        capacity_recommendation = if ($chainStatus.value -and $chainStatus.value.model_pool -and $chainStatus.value.model_pool.capacity) { $chainStatus.value.model_pool.capacity.recommendation } else { $null }
        workers = @($workers | ForEach-Object {
            [pscustomobject]@{
                role = $_.role
                port = $_.port
                status = $_.status
                ready = $_.ready
                context_window = $_.context_window
                default_max_tokens = $_.default_max_tokens
                runtime_backend = $_.runtime_backend
                runtime_device = $_.runtime_device
                runtime_accelerator = $_.runtime_accelerator
                gpu_layers = $_.gpu_layers
                model_cache_ok = $_.model_cache_ok
            }
        })
    }
    unattended = [pscustomobject]@{
        rounds = if ($report.value) { $report.value.rounds } else { $null }
        success = if ($report.value) { $report.value.success } else { $null }
        failures = if ($report.value) { $report.value.failures } else { $null }
        success_rate = if ($report.value) { $report.value.success_rate } else { $null }
        runtime_tokens_total = if ($report.value -and $report.value.runtime_tokens) { $report.value.runtime_tokens.total } else { $null }
        validation = if ($report.value -and $report.value.validation) { "$($report.value.validation.passed)/$($report.value.validation.checked)" } else { $null }
        self_improve = if ($report.value -and $report.value.self_improve) { "$($report.value.self_improve.passed)/$($report.value.self_improve.checked)" } else { $null }
        report_gate_passed = if ($report.value -and $report.value.report_gate) { [bool]$report.value.report_gate.passed } else { $false }
        recent_failures = if ($report.value -and $report.value.PSObject.Properties["recent_failures"]) { @($report.value.recent_failures).Count } else { $null }
        test_gate_verdict = if ($report.value -and $report.value.test_gate) { $report.value.test_gate.latest_verdict } else { $null }
        test_gate_validation_command = if ($report.value -and $report.value.test_gate) { $report.value.test_gate.latest_validation_command } else { $null }
    }
    latest_ledger = [pscustomobject]@{
        round = if ($latest) { $latest.round } else { $null }
        case = if ($latest) { $latest.case } else { $null }
        success = if ($latest) { $latest.success } else { $null }
        runtime_model = if ($latest) { $latest.runtime_model } else { $null }
        runtime_tokens = if ($latest) { $latest.runtime_tokens } else { $null }
        elapsed_ms = if ($latest) { $latest.elapsed_ms } else { $null }
        validation_checked = if ($latest) { $latest.validation_checked } else { $null }
        validation_passed = if ($latest) { $latest.validation_passed } else { $null }
        validation_command = if ($latest) { $latest.validation_command_preview } else { $null }
        self_improve_passed = if ($latest) { $latest.self_improve_passed } else { $null }
    }
}

if ($localObservationEnabled) {
    $snapshot | Add-Member -MemberType NoteProperty -Name "local_observation" -Value ([pscustomobject]@{
        schema_version = 1
        contract_version = "smartsteam.remote-gemma-unattended.local-observation.v1"
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
        observation_dir = $localObservationRoot
        expected_files = @("chain-status.json", "pool-status.json", "status-bundle.json", "forge-daemon-status.json")
        files = [pscustomobject]@{
            chain_status = [pscustomobject]@{
                path = if ($chainObservation) { ConvertTo-RelativePath -Root $root -Path $chainObservation.path } else { $null }
                exists = if ($chainObservation) { $chainObservation.exists } else { $false }
                parse_error = if ($chainObservation) { $chainObservation.parse_error } else { "not_loaded" }
            }
            pool_status = [pscustomobject]@{
                path = if ($poolObservation) { ConvertTo-RelativePath -Root $root -Path $poolObservation.path } else { $null }
                exists = if ($poolObservation) { $poolObservation.exists } else { $false }
                parse_error = if ($poolObservation) { $poolObservation.parse_error } else { "not_loaded" }
            }
            status_bundle = [pscustomobject]@{
                path = if ($bundleObservation) { ConvertTo-RelativePath -Root $root -Path $bundleObservation.path } else { $null }
                exists = if ($bundleObservation) { $bundleObservation.exists } else { $false }
                parse_error = if ($bundleObservation) { $bundleObservation.parse_error } else { "not_loaded" }
            }
            forge_daemon_status = [pscustomobject]@{
                path = if ($forgeDaemonObservation) { ConvertTo-RelativePath -Root $root -Path $forgeDaemonObservation.path } else { $null }
                exists = if ($forgeDaemonObservation) { $forgeDaemonObservation.exists } else { $false }
                parse_error = if ($forgeDaemonObservation) { $forgeDaemonObservation.parse_error } else { "not_loaded" }
            }
        }
        summary = [pscustomobject]@{
            complete_parse_ok = (
                $chainObservation.exists -and
                [string]::IsNullOrWhiteSpace([string]$chainObservation.parse_error) -and
                $poolObservation.exists -and
                [string]::IsNullOrWhiteSpace([string]$poolObservation.parse_error) -and
                $bundleObservation.exists -and
                [string]::IsNullOrWhiteSpace([string]$bundleObservation.parse_error) -and
                $forgeDaemonObservation.exists -and
                [string]::IsNullOrWhiteSpace([string]$forgeDaemonObservation.parse_error)
            )
            single_sample_only = $true
            window_sample_count = 1
            continuous_window_present = $false
            chain_classification = if ($chainObservation -and $chainObservation.value) { $chainObservation.value.classification } else { $null }
            chain_prompt_ready = if ($chainObservation -and $chainObservation.value) { $chainObservation.value.prompt_ready } else { $null }
            chain_read_only = if ($chainObservation -and $chainObservation.value -and $chainObservation.value.PSObject.Properties["machine_summary"]) { $chainObservation.value.machine_summary.read_only } else { $null }
            pool_launch_allowed = if ($poolObservation -and $poolObservation.value) { $poolObservation.value.launch_allowed } else { $null }
            pool_worker_count = if ($poolObservation -and $poolObservation.value -and $poolObservation.value.PSObject.Properties["capacity"]) { $poolObservation.value.capacity.worker_count } else { $null }
            pool_healthy_worker_count = if ($poolObservation -and $poolObservation.value -and $poolObservation.value.PSObject.Properties["capacity"]) { $poolObservation.value.capacity.healthy_worker_count } else { $null }
            pool_capacity_expansion_allowed = if ($poolObservation -and $poolObservation.value -and $poolObservation.value.PSObject.Properties["capacity"]) { $poolObservation.value.capacity.expansion_allowed } else { $null }
            bundle_read_only = if ($bundleObservation -and $bundleObservation.value) { $bundleObservation.value.read_only } else { $null }
            bundle_sends_prompt = if ($bundleObservation -and $bundleObservation.value) { $bundleObservation.value.sends_prompt } else { $null }
            bundle_launches_process = if ($bundleObservation -and $bundleObservation.value) { $bundleObservation.value.launches_process } else { $null }
            daemon_read_only = if ($forgeDaemonObservation -and $forgeDaemonObservation.value) { $forgeDaemonObservation.value.read_only } else { $null }
            daemon_starts_process = if ($forgeDaemonObservation -and $forgeDaemonObservation.value) { $forgeDaemonObservation.value.starts_process } else { $null }
            daemon_sends_prompt = if ($forgeDaemonObservation -and $forgeDaemonObservation.value) { $forgeDaemonObservation.value.sends_prompt } else { $null }
            daemon_running = if ($forgeDaemonObservation -and $forgeDaemonObservation.value -and $forgeDaemonObservation.value.PSObject.Properties["evolution_status"]) { $forgeDaemonObservation.value.evolution_status.daemon.running } else { $null }
            report_gate_continuation_state = if ($forgeDaemonObservation -and $forgeDaemonObservation.value -and $forgeDaemonObservation.value.PSObject.Properties["report_gate_preflight"]) { $forgeDaemonObservation.value.report_gate_preflight.continuation_state } else { $null }
            unattended_start_plan_can_start = if ($forgeDaemonObservation -and $forgeDaemonObservation.value -and $forgeDaemonObservation.value.PSObject.Properties["unattended_start_plan"]) { $forgeDaemonObservation.value.unattended_start_plan.can_start } else { $null }
        }
        authorization_note = "Local observations are evidence only; this script still does not authorize daemon, launch, SSH, or prompt actions."
    })
}

$evidenceFresh = (
    $snapshot.evidence.model_cache_status.fresh -and
    $snapshot.evidence.status_with_model_cache.fresh -and
    $snapshot.evidence.evolution_report.fresh -and
    $snapshot.evidence.evolution_ledger.fresh
)

$localObservationPresent = ($snapshot.PSObject.Properties["local_observation"] -and $snapshot.local_observation)
$localObservationParseOk = $false
$daemonObservedOnce = $false
$activeDaemonObservedOnce = $false
$promptLaunchGatesObservedOnce = $false
$poolObservedOnce = $false
if ($localObservationPresent) {
    $localObservationParseOk = (
        $snapshot.local_observation.files.chain_status.exists -and
        [string]::IsNullOrWhiteSpace([string]$snapshot.local_observation.files.chain_status.parse_error) -and
        $snapshot.local_observation.files.pool_status.exists -and
        [string]::IsNullOrWhiteSpace([string]$snapshot.local_observation.files.pool_status.parse_error) -and
        $snapshot.local_observation.files.status_bundle.exists -and
        [string]::IsNullOrWhiteSpace([string]$snapshot.local_observation.files.status_bundle.parse_error) -and
        $snapshot.local_observation.files.forge_daemon_status.exists -and
        [string]::IsNullOrWhiteSpace([string]$snapshot.local_observation.files.forge_daemon_status.parse_error)
    )
    $daemonObservedOnce = (
        $localObservationParseOk -and
        $snapshot.local_observation.summary.daemon_read_only -eq $true -and
        $snapshot.local_observation.summary.daemon_starts_process -eq $false -and
        $snapshot.local_observation.summary.daemon_sends_prompt -eq $false -and
        -not [string]::IsNullOrWhiteSpace([string]$snapshot.local_observation.summary.report_gate_continuation_state)
    )
    $activeDaemonObservedOnce = (
        $localObservationParseOk -and
        $null -ne $snapshot.local_observation.summary.daemon_running
    )
    $promptLaunchGatesObservedOnce = (
        $localObservationParseOk -and
        $snapshot.local_observation.summary.bundle_read_only -eq $true -and
        $snapshot.local_observation.summary.bundle_sends_prompt -eq $false -and
        $snapshot.local_observation.summary.bundle_launches_process -eq $false -and
        $null -ne $snapshot.local_observation.summary.chain_prompt_ready -and
        $null -ne $snapshot.local_observation.summary.pool_launch_allowed
    )
    $poolObservedOnce = (
        $localObservationParseOk -and
        $null -ne $snapshot.local_observation.summary.pool_worker_count -and
        $null -ne $snapshot.local_observation.summary.pool_healthy_worker_count
    )
}

$residencyGaps = @()
if (-not $evidenceFresh) {
    $residencyGaps += "fresh_status_snapshot_missing_or_stale"
}
$residencyGaps += "daemon_status_not_rechecked"
$residencyGaps += "active_unattended_daemon_presence_not_verified"
$residencyGaps += "prompt_and_launch_gates_not_rechecked"
$residencyGaps += "continuous_port_health_window_missing"
$residencyGaps += "remote_resource_headroom_window_missing"

$safeNextReadOnlyCommands = @(
    [pscustomobject]@{
        id = "snapshot_summary"
        purpose = "Re-read historical snapshot evidence and freshness metadata."
        command = ".\tools\gemma-chain\scripts\read-remote-unattended-snapshot.ps1 -Json"
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "gemma_chain_status"
        purpose = "Refresh local chain status/gates without sending prompts."
        command = ".\tools\gemma-chain\gemma-chain.cmd chain-status -JsonStatus"
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "gemma_pool_status"
        purpose = "Refresh model-pool status without launching workers."
        command = ".\tools\gemma-chain\gemma-chain.cmd pool-status -JsonStatus"
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "gemma_status_bundle"
        purpose = "Collect local handoff bundle with prompt and launch gates."
        command = ".\tools\gemma-chain\gemma-chain.cmd status-bundle -JsonStatus"
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "forge_daemon_status"
        purpose = "Read Forge unattended daemon status and report gate state."
        command = ".\tools\smartsteam-forge\evolution-daemon.cmd -JsonStatus -WorkDir target\remote-gemma-unattended"
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "forge_daemon_watch_once"
        purpose = "Read one daemon/status sample without starting the daemon."
        command = ".\tools\smartsteam-forge\evolution-daemon.cmd -Watch -Count 1 -IntervalSecs 1 -WorkDir target\remote-gemma-unattended"
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "forge_daemon_start_check"
        purpose = "Dry-run daemon preflight only; do not start unattended evolution."
        command = ".\tools\smartsteam-forge\evolution-daemon.cmd -StartCheck -WorkDir target\remote-gemma-unattended -Backend 127.0.0.1:7979 -MaxTokens 64 -MaxTotalTokens 96 -MaxRuntimeSecs 0 -MaxFailures 1 -MaxNoFeedbackRounds 0 -TimeoutSecs 300"
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "remote_resource_artifact_check"
        purpose = "Check for an already-collected local remote resource/headroom artifact without SSH."
        command = ".\tools\gemma-chain\scripts\read-remote-resource-window.ps1 -Json"
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    },
    [pscustomobject]@{
        id = "observation_window_reader"
        purpose = "Read already-collected local observation-window samples without executing diagnostics."
        command = ".\tools\gemma-chain\scripts\read-remote-observation-window.ps1 -Json"
        read_only = $true
        starts_process = $false
        sends_prompt = $false
        touches_remote = $false
        writes_files = $false
    }
)

$residencyEvidenceChecklist = @(
    [pscustomobject]@{
        id = "fresh_status_snapshot"
        gap_id = "fresh_status_snapshot_missing_or_stale"
        status = if ($evidenceFresh) { "present" } else { "missing_or_stale" }
        required_evidence = "Fresh chain/status/model-cache/evolution evidence inside the selected freshness window."
        proof_source = "evidence.*.fresh and summary.evidence_fresh_all"
        safe_command_id = "snapshot_summary"
        blocks_authorization = $true
    },
    [pscustomobject]@{
        id = "daemon_status"
        gap_id = "daemon_status_not_rechecked"
        status = if ($daemonObservedOnce) { "observed_once_insufficient" } else { "not_rechecked_by_this_script" }
        required_evidence = "Forge daemon status with read_only=true, starts_process=false, sends_prompt=false, and report_gate_continuation_state recorded."
        proof_source = if ($daemonObservedOnce) { "local_observation.forge_daemon_status" } else { "forge_daemon_status or forge_daemon_watch_once output" }
        safe_command_id = "forge_daemon_status"
        blocks_authorization = $true
    },
    [pscustomobject]@{
        id = "active_daemon_presence"
        gap_id = "active_unattended_daemon_presence_not_verified"
        status = if ($activeDaemonObservedOnce) { "observed_once_insufficient" } else { "not_rechecked_by_this_script" }
        required_evidence = "Current unattended daemon presence/running-state evidence to avoid duplicate resident loops."
        proof_source = if ($activeDaemonObservedOnce) { "local_observation.forge_daemon_status" } else { "forge_daemon_status output" }
        safe_command_id = "forge_daemon_status"
        blocks_authorization = $true
    },
    [pscustomobject]@{
        id = "prompt_launch_gates"
        gap_id = "prompt_and_launch_gates_not_rechecked"
        status = if ($promptLaunchGatesObservedOnce) { "observed_once_insufficient" } else { "not_rechecked_by_this_script" }
        required_evidence = "Current prompt and launch gates for evolution_loop_prompt_round and model_pool_launch."
        proof_source = if ($promptLaunchGatesObservedOnce) { "local_observation.chain-status/status-bundle/pool-status" } else { "gemma_status_bundle or gemma_chain_status output" }
        safe_command_id = "gemma_status_bundle"
        blocks_authorization = $true
    },
    [pscustomobject]@{
        id = "continuous_port_health"
        gap_id = "continuous_port_health_window_missing"
        status = if ($poolObservedOnce) { "single_sample_observed_window_missing" } else { "not_collected_by_this_script" }
        required_evidence = "A continuous health window for model API, backend, Web Lab, and pool worker ports 8686-8690."
        proof_source = if ($poolObservedOnce) { "local_observation.pool-status single sample; observation_window_reader still required for a repeated window" } else { "read-remote-observation-window.ps1 output or repeated gemma_pool_status/status-bundle samples saved as local artifacts" }
        safe_command_id = "observation_window_reader"
        blocks_authorization = $true
    },
    [pscustomobject]@{
        id = "remote_resource_headroom"
        gap_id = "remote_resource_headroom_window_missing"
        status = "not_collected_by_this_script"
        required_evidence = "Remote host memory/GPU/Metal headroom over a continuous window, collected by an approved read-only owner flow."
        proof_source = "read-remote-resource-window.ps1 output from already-collected local resource artifacts"
        safe_command_id = "remote_resource_artifact_check"
        blocks_authorization = $true
    }
)

$evidenceFilesPresent = (
    $snapshot.evidence.model_cache_status.exists -and
    $snapshot.evidence.status_with_model_cache.exists -and
    $snapshot.evidence.evolution_report.exists -and
    $snapshot.evidence.evolution_ledger.exists
)

$parseOk = (
    [string]::IsNullOrWhiteSpace($snapshot.evidence.model_cache_status.parse_error) -and
    [string]::IsNullOrWhiteSpace($snapshot.evidence.status_with_model_cache.parse_error) -and
    [string]::IsNullOrWhiteSpace($snapshot.evidence.evolution_report.parse_error) -and
    [string]::IsNullOrWhiteSpace($snapshot.evidence.evolution_ledger.parse_error)
)

$recommendedNextCommandIds = @(
    "snapshot_summary",
    "gemma_chain_status",
    "gemma_pool_status",
    "gemma_status_bundle",
    "forge_daemon_status",
    "forge_daemon_watch_once",
    "forge_daemon_start_check",
    "remote_resource_artifact_check",
    "observation_window_reader"
)

$consumerProjection = @(
    [pscustomobject]@{
        id = "web_lab_prompt"
        surface = "Web Lab"
        entrypoint_kind = "prompt"
        current_allowed = $false
        blocked_by = $residencyGaps
        downstream_sends_prompt = $true
        downstream_launches_process = $false
        downstream_touches_remote = $false
        safe_command_id = "gemma_status_bundle"
        decision = "blocked_by_snapshot_summary"
        reason = "Historical remote Gemma evidence cannot authorize Web Lab prompts."
    },
    [pscustomobject]@{
        id = "forge_cli_prompt"
        surface = "SmartSteam Forge CLI"
        entrypoint_kind = "prompt"
        current_allowed = $false
        blocked_by = $residencyGaps
        downstream_sends_prompt = $true
        downstream_launches_process = $false
        downstream_touches_remote = $false
        safe_command_id = "gemma_status_bundle"
        decision = "blocked_by_snapshot_summary"
        reason = "Historical remote Gemma evidence cannot authorize Forge prompt entrypoints."
    },
    [pscustomobject]@{
        id = "backend_cli_direct_prompt"
        surface = "Backend CLI"
        entrypoint_kind = "prompt"
        current_allowed = $false
        blocked_by = $residencyGaps
        downstream_sends_prompt = $true
        downstream_launches_process = $false
        downstream_touches_remote = $false
        safe_command_id = "gemma_chain_status"
        decision = "blocked_by_snapshot_summary"
        reason = "Historical remote Gemma evidence cannot authorize direct backend prompts."
    },
    [pscustomobject]@{
        id = "evolution_loop_prompt_round"
        surface = "Evolution Loop"
        entrypoint_kind = "prompt"
        current_allowed = $false
        blocked_by = $residencyGaps
        downstream_sends_prompt = $true
        downstream_launches_process = $false
        downstream_touches_remote = $false
        safe_command_id = "gemma_status_bundle"
        decision = "blocked_by_snapshot_summary"
        reason = "Unattended evolution needs fresh gates and daemon state before any prompt round."
    },
    [pscustomobject]@{
        id = "model_pool_launch"
        surface = "Model Pool"
        entrypoint_kind = "launch"
        current_allowed = $false
        blocked_by = $residencyGaps
        downstream_sends_prompt = $false
        downstream_launches_process = $true
        downstream_touches_remote = $false
        safe_command_id = "gemma_pool_status"
        decision = "blocked_by_snapshot_summary"
        reason = "Historical worker health cannot authorize model-pool launch or expansion."
    },
    [pscustomobject]@{
        id = "forge_daemon_residency"
        surface = "SmartSteam Forge Daemon"
        entrypoint_kind = "launch"
        current_allowed = $false
        blocked_by = $residencyGaps
        downstream_sends_prompt = $false
        downstream_launches_process = $true
        downstream_touches_remote = $false
        safe_command_id = "forge_daemon_start_check"
        decision = "blocked_by_snapshot_summary"
        reason = "Resident daemon requires fresh report-gate and duplicate-runner evidence."
    },
    [pscustomobject]@{
        id = "ssh_remote_probe"
        surface = "Remote Host"
        entrypoint_kind = "ssh"
        current_allowed = $false
        blocked_by = $residencyGaps
        downstream_sends_prompt = $false
        downstream_launches_process = $false
        downstream_touches_remote = $true
        safe_command_id = "remote_resource_artifact_check"
        decision = "blocked_by_snapshot_summary"
        reason = "This window is summary-only; use approved read-only owner flow before SSH."
    }
)

$consumerContract = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.consumer-projection.v1"
    projection_field = "consumer_projection"
    fail_closed_default = $true
    allowed_requires_external_gates = $true
    required_fields = @(
        "id",
        "surface",
        "entrypoint_kind",
        "current_allowed",
        "blocked_by",
        "downstream_sends_prompt",
        "downstream_launches_process",
        "downstream_touches_remote",
        "safe_command_id",
        "decision",
        "reason"
    )
    supported_entrypoint_kinds = @("prompt", "launch", "ssh")
    consumer_ids = @($consumerProjection | ForEach-Object { $_.id })
    note = "Snapshot-only consumers must treat missing or unknown fields as blocked and collect fresh read-only evidence before any prompt, launch, SSH, or resident-loop action."
}

$residencyClassification = if (-not $snapshot.model_cache.all_ok) {
    "blocked_model_cache"
} elseif (-not $evidenceFilesPresent -or -not $parseOk) {
    "blocked_evidence_unreadable"
} elseif (-not $evidenceFresh) {
    "blocked_stale_evidence"
} elseif ($residencyGaps.Count -gt 0) {
    "blocked_missing_residency_evidence"
} else {
    "ready_for_external_gate_review"
}

$snapshot | Add-Member -MemberType NoteProperty -Name "summary" -Value ([pscustomobject]@{
    evidence_files_present = $evidenceFilesPresent
    parse_ok = $parseOk
    evidence_fresh_all = $evidenceFresh
    fresh_minutes = $FreshMinutes
    model_cache_all_ok = $snapshot.model_cache.all_ok
    chain_ready_snapshot = $snapshot.chain.ready
    pool_healthy_snapshot = ($snapshot.model_pool.worker_count -gt 0 -and $snapshot.model_pool.worker_count -eq $snapshot.model_pool.healthy_worker_count)
    unattended_success_snapshot = ($snapshot.unattended.failures -eq 0 -and $snapshot.unattended.success -eq $snapshot.unattended.rounds)
    stale_warning = "This summarizes historical files only; regenerate status before authorizing daemon, launch, SSH, or prompt actions."
})

$snapshot | Add-Member -MemberType NoteProperty -Name "authorization" -Value ([pscustomobject]@{
    can_authorize_daemon = $false
    can_authorize_launch = $false
    can_authorize_prompt = $false
    can_authorize_ssh = $false
    reason = "summary_only_fail_closed"
})

$snapshot | Add-Member -MemberType NoteProperty -Name "residency_gaps" -Value $residencyGaps
$snapshot | Add-Member -MemberType NoteProperty -Name "residency_evidence_checklist" -Value $residencyEvidenceChecklist
$snapshot | Add-Member -MemberType NoteProperty -Name "evidence_checklist" -Value $residencyEvidenceChecklist
$snapshot | Add-Member -MemberType NoteProperty -Name "residency_decision" -Value ([pscustomobject]@{
    classification = $residencyClassification
    read_only_evidence_collection_only = $true
    can_proceed_to_resident_loop = $false
    blocked_by = $residencyGaps
    recommended_next_command_ids = $recommendedNextCommandIds
    reason = "Snapshot summary cannot authorize daemon, launch, SSH, or prompt actions."
})
$snapshot | Add-Member -MemberType NoteProperty -Name "consumer_projection" -Value $consumerProjection
$snapshot | Add-Member -MemberType NoteProperty -Name "consumer_contract" -Value $consumerContract
$snapshot | Add-Member -MemberType NoteProperty -Name "safe_next_read_only_commands" -Value $safeNextReadOnlyCommands

if ($Json) {
    $snapshot | ConvertTo-Json -Depth 10
    exit 0
}

Write-Host "Gemma remote unattended snapshot summary"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False"
Write-Host ""
Write-Host "Evidence:"
Write-Host "  model_cache_status     exists=$($snapshot.evidence.model_cache_status.exists) fresh=$($snapshot.evidence.model_cache_status.fresh) age_sec=$($snapshot.evidence.model_cache_status.age_seconds) time=$($snapshot.evidence.model_cache_status.last_write_time) parse_error=$($snapshot.evidence.model_cache_status.parse_error)"
Write-Host "  status_with_model_cache exists=$($snapshot.evidence.status_with_model_cache.exists) fresh=$($snapshot.evidence.status_with_model_cache.fresh) age_sec=$($snapshot.evidence.status_with_model_cache.age_seconds) time=$($snapshot.evidence.status_with_model_cache.last_write_time) parse_error=$($snapshot.evidence.status_with_model_cache.parse_error)"
Write-Host "  evolution_report       exists=$($snapshot.evidence.evolution_report.exists) fresh=$($snapshot.evidence.evolution_report.fresh) age_sec=$($snapshot.evidence.evolution_report.age_seconds) time=$($snapshot.evidence.evolution_report.last_write_time) parse_error=$($snapshot.evidence.evolution_report.parse_error)"
Write-Host "  evolution_ledger       exists=$($snapshot.evidence.evolution_ledger.exists) fresh=$($snapshot.evidence.evolution_ledger.fresh) age_sec=$($snapshot.evidence.evolution_ledger.age_seconds) time=$($snapshot.evidence.evolution_ledger.last_write_time) lines=$($snapshot.evidence.evolution_ledger.line_count) parse_error=$($snapshot.evidence.evolution_ledger.parse_error)"
if ($snapshot.PSObject.Properties["local_observation"]) {
    Write-Host "Local observation: dir=$($snapshot.local_observation.observation_dir)"
    Write-Host "  complete_parse_ok=$($snapshot.local_observation.summary.complete_parse_ok) single_sample_only=$($snapshot.local_observation.summary.single_sample_only) continuous_window_present=$($snapshot.local_observation.summary.continuous_window_present)"
    Write-Host "  chain classification=$($snapshot.local_observation.summary.chain_classification) prompt_ready=$($snapshot.local_observation.summary.chain_prompt_ready)"
    Write-Host "  pool launch_allowed=$($snapshot.local_observation.summary.pool_launch_allowed) workers=$($snapshot.local_observation.summary.pool_healthy_worker_count)/$($snapshot.local_observation.summary.pool_worker_count) expansion_allowed=$($snapshot.local_observation.summary.pool_capacity_expansion_allowed)"
    Write-Host "  bundle read_only=$($snapshot.local_observation.summary.bundle_read_only) sends_prompt=$($snapshot.local_observation.summary.bundle_sends_prompt) launches_process=$($snapshot.local_observation.summary.bundle_launches_process)"
    Write-Host "  daemon read_only=$($snapshot.local_observation.summary.daemon_read_only) running=$($snapshot.local_observation.summary.daemon_running) continuation_state=$($snapshot.local_observation.summary.report_gate_continuation_state) can_start=$($snapshot.local_observation.summary.unattended_start_plan_can_start)"
}
Write-Host ""
Write-Host "Model cache: all_ok=$($snapshot.model_cache.all_ok) ok=$($snapshot.model_cache.ok_count)/$($snapshot.model_cache.model_count) copy_needed=$($snapshot.model_cache.copy_needed_count) remote_errors=$($snapshot.model_cache.remote_error_count) roles=$($snapshot.model_cache.roles -join ',')"
Write-Host "Chain: ready=$($snapshot.chain.ready) model_api=$($snapshot.chain.model_api) backend=$($snapshot.chain.backend) web_lab=$($snapshot.chain.web_lab) required_roles=$($snapshot.chain.required_roles -join ',') missing=$($snapshot.chain.missing_required_roles -join ',')"
Write-Host "Pool: healthy=$($snapshot.model_pool.healthy_worker_count)/$($snapshot.model_pool.worker_count) recommendation=$($snapshot.model_pool.capacity_recommendation)"
foreach ($worker in $snapshot.model_pool.workers) {
    Write-Host "  $($worker.role)@$($worker.port) status=$($worker.status) ready=$($worker.ready) ctx=$($worker.context_window) max=$($worker.default_max_tokens) accel=$($worker.runtime_accelerator) model_cache_ok=$($worker.model_cache_ok)"
}
Write-Host "Unattended: rounds=$($snapshot.unattended.rounds) success=$($snapshot.unattended.success) failures=$($snapshot.unattended.failures) validation=$($snapshot.unattended.validation) self_improve=$($snapshot.unattended.self_improve) report_gate_passed=$($snapshot.unattended.report_gate_passed)"
Write-Host "Latest ledger: round=$($snapshot.latest_ledger.round) case=$($snapshot.latest_ledger.case) success=$($snapshot.latest_ledger.success) model=$($snapshot.latest_ledger.runtime_model) tokens=$($snapshot.latest_ledger.runtime_tokens) validation_passed=$($snapshot.latest_ledger.validation_passed)"
Write-Host "Authorization: daemon=$($snapshot.authorization.can_authorize_daemon) launch=$($snapshot.authorization.can_authorize_launch) prompt=$($snapshot.authorization.can_authorize_prompt) ssh=$($snapshot.authorization.can_authorize_ssh) reason=$($snapshot.authorization.reason)"
Write-Host "Residency decision: classification=$($snapshot.residency_decision.classification) proceed=$($snapshot.residency_decision.can_proceed_to_resident_loop) read_only_evidence_collection_only=$($snapshot.residency_decision.read_only_evidence_collection_only)"
Write-Host "Consumer contract: version=$($snapshot.consumer_contract.contract_version) fail_closed_default=$($snapshot.consumer_contract.fail_closed_default)"
Write-Host "Consumer projection:"
foreach ($consumer in $snapshot.consumer_projection) {
    Write-Host "  $($consumer.id): allowed=$($consumer.current_allowed) kind=$($consumer.entrypoint_kind) safe_command=$($consumer.safe_command_id) reason=$($consumer.reason)"
}
Write-Host "Residency gaps: $($snapshot.residency_gaps -join ',')"
Write-Host "Residency evidence checklist:"
foreach ($item in $snapshot.residency_evidence_checklist) {
    Write-Host "  $($item.id): status=$($item.status) safe_command=$($item.safe_command_id) required=$($item.required_evidence)"
}
Write-Host "Safe next read-only commands:"
foreach ($command in $snapshot.safe_next_read_only_commands) {
    Write-Host "  $($command.id): $($command.command)"
}
Write-Host ""
Write-Host "Warning: $($snapshot.summary.stale_warning)"
