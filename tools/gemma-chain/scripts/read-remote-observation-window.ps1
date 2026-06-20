param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$WindowDir = "target\remote-gemma-observation-window",
    [int]$MinSamples = 3,
    [int]$MinSpanMinutes = 10,
    [int]$MinHealthyWorkers = 6,
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$expectedFiles = @(
    "chain-status.json",
    "pool-status.json",
    "status-bundle.json",
    "forge-daemon-status.json"
)

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

function Resolve-InputPath {
    param(
        [string]$Root,
        [string]$Path
    )

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }
    return [System.IO.Path]::GetFullPath((Join-Path $Root $Path))
}

function Read-JsonFile {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [pscustomobject]@{
            exists = $false
            path = $Path
            last_write_time = $null
            last_write_time_utc = $null
            parse_error = "missing"
            value = $null
        }
    }

    $item = Get-Item -LiteralPath $Path
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
        last_write_time_utc = $item.LastWriteTimeUtc
        parse_error = $parseError
        value = $value
    }
}

function Test-HasAnyExpectedFile {
    param([string]$Dir)

    foreach ($name in $expectedFiles) {
        if (Test-Path -LiteralPath (Join-Path $Dir $name) -PathType Leaf) {
            return $true
        }
    }
    return $false
}

function Get-FirstPropertyValue {
    param(
        [object]$Object,
        [string[]]$Names
    )

    if ($null -eq $Object) {
        return $null
    }

    foreach ($name in $Names) {
        if ($Object.PSObject.Properties[$name]) {
            return $Object.$name
        }
    }
    return $null
}

function Get-NestedFirstValue {
    param(
        [object]$Object,
        [string[][]]$Paths
    )

    foreach ($path in $Paths) {
        $cursor = $Object
        $found = $true
        foreach ($part in $path) {
            if ($null -eq $cursor -or -not $cursor.PSObject.Properties[$part]) {
                $found = $false
                break
            }
            $cursor = $cursor.$part
        }
        if ($found) {
            return $cursor
        }
    }
    return $null
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$resolvedWindowDir = Resolve-InputPath -Root $root -Path $WindowDir
$sampleDirs = @()

if (Test-Path -LiteralPath $resolvedWindowDir -PathType Container) {
    if (Test-HasAnyExpectedFile -Dir $resolvedWindowDir) {
        $sampleDirs += (Get-Item -LiteralPath $resolvedWindowDir)
    }

    $sampleDirs += @(Get-ChildItem -LiteralPath $resolvedWindowDir -Directory | Where-Object {
        Test-HasAnyExpectedFile -Dir $_.FullName
    } | Sort-Object Name)
}

$sampleDirs = @($sampleDirs | Sort-Object FullName -Unique)
$samples = @()

foreach ($dir in $sampleDirs) {
    $chain = Read-JsonFile -Path (Join-Path $dir.FullName "chain-status.json")
    $pool = Read-JsonFile -Path (Join-Path $dir.FullName "pool-status.json")
    $bundle = Read-JsonFile -Path (Join-Path $dir.FullName "status-bundle.json")
    $daemon = Read-JsonFile -Path (Join-Path $dir.FullName "forge-daemon-status.json")
    $files = @($chain, $pool, $bundle, $daemon)
    $completeParseOk = (@($files | Where-Object { $_.exists -ne $true -or -not [string]::IsNullOrWhiteSpace([string]$_.parse_error) }).Count -eq 0)

    $chainReadOnly = if ($chain.value -and $chain.value.PSObject.Properties["machine_summary"]) { $chain.value.machine_summary.read_only } elseif ($chain.value -and $chain.value.PSObject.Properties["read_only"]) { $chain.value.read_only } else { $null }
    $chainSendsPrompt = if ($chain.value -and $chain.value.PSObject.Properties["machine_summary"]) { $chain.value.machine_summary.sends_prompt } elseif ($chain.value -and $chain.value.PSObject.Properties["sends_prompt"]) { $chain.value.sends_prompt } else { $null }
    $chainLaunchesProcess = if ($chain.value -and $chain.value.PSObject.Properties["machine_summary"]) { $chain.value.machine_summary.launches_process } elseif ($chain.value -and $chain.value.PSObject.Properties["launches_process"]) { $chain.value.launches_process } else { $null }

    $bundleReadOnly = if ($bundle.value) { $bundle.value.read_only } else { $null }
    $bundleSendsPrompt = if ($bundle.value) { $bundle.value.sends_prompt } else { $null }
    $bundleLaunchesProcess = if ($bundle.value) { $bundle.value.launches_process } else { $null }

    $daemonReadOnly = if ($daemon.value) { $daemon.value.read_only } else { $null }
    $daemonStartsProcess = if ($daemon.value) { $daemon.value.starts_process } else { $null }
    $daemonSendsPrompt = if ($daemon.value) { $daemon.value.sends_prompt } else { $null }
    $chainClassification = if ($chain.value) { $chain.value.classification } else { $null }
    $chainPromptReady = if ($chain.value) { Get-FirstPropertyValue -Object $chain.value -Names @("prompt_ready", "ready") } else { $null }
    $chainModelApi = if ($chain.value) { Get-NestedFirstValue -Object $chain.value -Paths @(@("readiness", "model_api"), @("ports", "model_api"), @("model_api")) } else { $null }
    $chainBackend = if ($chain.value) { Get-NestedFirstValue -Object $chain.value -Paths @(@("readiness", "backend"), @("ports", "backend"), @("backend")) } else { $null }
    $chainWebLab = if ($chain.value) { Get-NestedFirstValue -Object $chain.value -Paths @(@("readiness", "web_lab"), @("ports", "web_lab"), @("web_lab")) } else { $null }
    $poolWorkerCount = if ($pool.value -and $pool.value.PSObject.Properties["capacity"]) { $pool.value.capacity.worker_count } elseif ($pool.value -and $pool.value.PSObject.Properties["worker_count"]) { $pool.value.worker_count } else { $null }
    $poolHealthyWorkerCount = if ($pool.value -and $pool.value.PSObject.Properties["capacity"]) { $pool.value.capacity.healthy_worker_count } elseif ($pool.value -and $pool.value.PSObject.Properties["healthy_worker_count"]) { $pool.value.healthy_worker_count } else { $null }
    $poolCapacityExpansionAllowed = if ($pool.value -and $pool.value.PSObject.Properties["capacity"]) { $pool.value.capacity.expansion_allowed } else { $null }
    $chainHealthKnown = (
        $null -ne $chainPromptReady -or
        -not [string]::IsNullOrWhiteSpace([string]$chainClassification) -or
        ($null -ne $chainModelApi -and $null -ne $chainBackend -and $null -ne $chainWebLab)
    )
    $chainHealthOk = (
        ($chainPromptReady -eq $true) -or
        ([string]$chainClassification -eq "prompt_ready") -or
        ($chainModelApi -eq $true -and $chainBackend -eq $true -and $chainWebLab -eq $true)
    )
    $poolHealthKnown = ($null -ne $poolWorkerCount -and $null -ne $poolHealthyWorkerCount)
    $poolHealthOk = (
        $poolHealthKnown -and
        [int]$poolWorkerCount -ge $MinHealthyWorkers -and
        [int]$poolHealthyWorkerCount -ge $MinHealthyWorkers -and
        [int]$poolHealthyWorkerCount -eq [int]$poolWorkerCount
    )
    $healthKnown = ($chainHealthKnown -and $poolHealthKnown)
    $healthOk = ($chainHealthOk -and $poolHealthOk)

    $safeContractOk = (
        $completeParseOk -and
        $chainReadOnly -eq $true -and
        $chainSendsPrompt -eq $false -and
        $chainLaunchesProcess -eq $false -and
        $bundleReadOnly -eq $true -and
        $bundleSendsPrompt -eq $false -and
        $bundleLaunchesProcess -eq $false -and
        $daemonReadOnly -eq $true -and
        $daemonStartsProcess -eq $false -and
        $daemonSendsPrompt -eq $false
    )

    $sampleTimes = @($files | Where-Object { $null -ne $_.last_write_time_utc } | ForEach-Object { $_.last_write_time_utc })
    $sampleTimeUtc = if ($sampleTimes.Count -gt 0) { ($sampleTimes | Sort-Object | Select-Object -Last 1).ToString("o") } else { $null }

    $samples += [pscustomobject]@{
        id = $dir.Name
        path = ConvertTo-RelativePath -Root $root -Path $dir.FullName
        sample_time_utc = $sampleTimeUtc
        complete_parse_ok = $completeParseOk
        safe_contract_ok = $safeContractOk
        health_known = $healthKnown
        health_ok = $healthOk
        files = [pscustomobject]@{
            chain_status = [pscustomobject]@{
                path = ConvertTo-RelativePath -Root $root -Path $chain.path
                exists = $chain.exists
                parse_error = $chain.parse_error
            }
            pool_status = [pscustomobject]@{
                path = ConvertTo-RelativePath -Root $root -Path $pool.path
                exists = $pool.exists
                parse_error = $pool.parse_error
            }
            status_bundle = [pscustomobject]@{
                path = ConvertTo-RelativePath -Root $root -Path $bundle.path
                exists = $bundle.exists
                parse_error = $bundle.parse_error
            }
            forge_daemon_status = [pscustomobject]@{
                path = ConvertTo-RelativePath -Root $root -Path $daemon.path
                exists = $daemon.exists
                parse_error = $daemon.parse_error
            }
        }
        extracted = [pscustomobject]@{
            chain_classification = $chainClassification
            chain_prompt_ready = $chainPromptReady
            chain_model_api = $chainModelApi
            chain_backend = $chainBackend
            chain_web_lab = $chainWebLab
            chain_health_known = $chainHealthKnown
            chain_health_ok = $chainHealthOk
            chain_read_only = $chainReadOnly
            chain_sends_prompt = $chainSendsPrompt
            chain_launches_process = $chainLaunchesProcess
            pool_launch_allowed = if ($pool.value) { $pool.value.launch_allowed } else { $null }
            pool_worker_count = $poolWorkerCount
            pool_healthy_worker_count = $poolHealthyWorkerCount
            pool_capacity_expansion_allowed = $poolCapacityExpansionAllowed
            pool_health_known = $poolHealthKnown
            pool_health_ok = $poolHealthOk
            bundle_read_only = $bundleReadOnly
            bundle_sends_prompt = $bundleSendsPrompt
            bundle_launches_process = $bundleLaunchesProcess
            daemon_read_only = $daemonReadOnly
            daemon_starts_process = $daemonStartsProcess
            daemon_sends_prompt = $daemonSendsPrompt
            daemon_running = if ($daemon.value -and $daemon.value.PSObject.Properties["evolution_status"]) { $daemon.value.evolution_status.daemon.running } else { $null }
            report_gate_continuation_state = if ($daemon.value -and $daemon.value.PSObject.Properties["report_gate_preflight"]) { $daemon.value.report_gate_preflight.continuation_state } else { $null }
        }
    }
}

$sampleTimesParsed = @($samples | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.sample_time_utc) } | ForEach-Object { [datetime]$_.sample_time_utc } | Sort-Object)
$spanSeconds = if ($sampleTimesParsed.Count -ge 2) { [int][math]::Round((($sampleTimesParsed | Select-Object -Last 1) - ($sampleTimesParsed | Select-Object -First 1)).TotalSeconds) } else { 0 }
$completeSampleCount = @($samples | Where-Object { $_.complete_parse_ok -eq $true }).Count
$safeSampleCount = @($samples | Where-Object { $_.safe_contract_ok -eq $true }).Count
$knownHealthSampleCount = @($samples | Where-Object { $_.health_known -eq $true }).Count
$okHealthSampleCount = @($samples | Where-Object { $_.health_ok -eq $true }).Count
$continuousWindowPresent = (
    $samples.Count -ge $MinSamples -and
    $completeSampleCount -eq $samples.Count -and
    $safeSampleCount -eq $samples.Count -and
    $knownHealthSampleCount -eq $samples.Count -and
    $okHealthSampleCount -eq $samples.Count -and
    $spanSeconds -ge ($MinSpanMinutes * 60)
)

$status = if ($samples.Count -eq 0) {
    "missing_window"
} elseif ($samples.Count -lt $MinSamples) {
    "insufficient_samples"
} elseif ($completeSampleCount -ne $samples.Count) {
    "incomplete_or_unparseable_samples"
} elseif ($safeSampleCount -ne $samples.Count) {
    "unsafe_sample_contract"
} elseif ($knownHealthSampleCount -ne $samples.Count) {
    "health_unknown"
} elseif ($okHealthSampleCount -ne $samples.Count) {
    "health_not_ready"
} elseif ($spanSeconds -lt ($MinSpanMinutes * 60)) {
    "insufficient_span"
} else {
    "window_observed_external_gate_required"
}

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.observation-window.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    repo_root = $root
    window_dir = ConvertTo-RelativePath -Root $root -Path $resolvedWindowDir
    expected_files_per_sample = $expectedFiles
    requirements = [pscustomobject]@{
        min_samples = $MinSamples
        min_span_minutes = $MinSpanMinutes
        min_healthy_workers = $MinHealthyWorkers
    }
    summary = [pscustomobject]@{
        status = $status
        sample_count = $samples.Count
        complete_sample_count = $completeSampleCount
        safe_sample_count = $safeSampleCount
        known_health_sample_count = $knownHealthSampleCount
        ok_health_sample_count = $okHealthSampleCount
        span_seconds = $spanSeconds
        continuous_window_present = $continuousWindowPresent
        can_support_residency_review = $continuousWindowPresent
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "window_reader_is_evidence_only_fail_closed"
    }
    samples = $samples
    note = "This script only reads already-collected local observation samples. It never runs model, SSH, prompt, launch, or daemon commands."
}

if ($Json) {
    $result | ConvertTo-Json -Depth 12
    exit 0
}

Write-Host "Gemma remote observation window"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False"
Write-Host "window_dir=$($result.window_dir)"
Write-Host "status=$($result.summary.status) samples=$($result.summary.sample_count) complete=$($result.summary.complete_sample_count) safe=$($result.summary.safe_sample_count) health_ok=$($result.summary.ok_health_sample_count) span_sec=$($result.summary.span_seconds) continuous_window_present=$($result.summary.continuous_window_present)"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
foreach ($sample in $result.samples) {
    Write-Host "  $($sample.id): complete=$($sample.complete_parse_ok) safe=$($sample.safe_contract_ok) health_ok=$($sample.health_ok) time=$($sample.sample_time_utc) chain=$($sample.extracted.chain_classification) workers=$($sample.extracted.pool_healthy_worker_count)/$($sample.extracted.pool_worker_count) daemon_running=$($sample.extracted.daemon_running)"
}
Write-Host "Note: $($result.note)"
