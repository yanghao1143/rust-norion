param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$WindowDir = "target\remote-gemma-resource-window",
    [int]$MinSamples = 3,
    [int]$MinSpanMinutes = 10,
    [double]$MinAvailableMemoryGb = 8,
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$resourceFileNames = @(
    "remote-resource-status.json",
    "resource-status.json",
    "resource-headroom.json"
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

function Get-MemoryGb {
    param([object]$Value)

    $gb = Get-NestedFirstValue -Object $Value -Paths @(
        @("summary", "memory_available_gb"),
        @("summary", "available_memory_gb"),
        @("memory", "available_gb"),
        @("memory", "free_gb"),
        @("headroom", "memory_available_gb"),
        @("headroom", "available_memory_gb")
    )
    if ($null -ne $gb) {
        return [double]$gb
    }

    $bytes = Get-NestedFirstValue -Object $Value -Paths @(
        @("summary", "memory_available_bytes"),
        @("summary", "available_memory_bytes"),
        @("memory", "available_bytes"),
        @("memory", "free_bytes"),
        @("headroom", "memory_available_bytes"),
        @("headroom", "available_memory_bytes")
    )
    if ($null -ne $bytes) {
        return [math]::Round(([double]$bytes / 1GB), 3)
    }

    return $null
}

function Get-MetalAvailable {
    param([object]$Value)

    return Get-NestedFirstValue -Object $Value -Paths @(
        @("summary", "metal_available"),
        @("summary", "gpu_available"),
        @("metal", "available"),
        @("gpu", "available"),
        @("accelerator", "available"),
        @("headroom", "metal_available")
    )
}

function Test-HasResourceFile {
    param([string]$Dir)

    foreach ($name in $resourceFileNames) {
        if (Test-Path -LiteralPath (Join-Path $Dir $name) -PathType Leaf) {
            return $true
        }
    }
    return $false
}

function Find-ResourceFile {
    param([string]$Dir)

    foreach ($name in $resourceFileNames) {
        $path = Join-Path $Dir $name
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            return $path
        }
    }
    return (Join-Path $Dir $resourceFileNames[0])
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$resolvedWindowDir = Resolve-InputPath -Root $root -Path $WindowDir
$sampleDirs = @()

if (Test-Path -LiteralPath $resolvedWindowDir -PathType Container) {
    if (Test-HasResourceFile -Dir $resolvedWindowDir) {
        $sampleDirs += (Get-Item -LiteralPath $resolvedWindowDir)
    }

    $sampleDirs += @(Get-ChildItem -LiteralPath $resolvedWindowDir -Directory | Where-Object {
        Test-HasResourceFile -Dir $_.FullName
    } | Sort-Object Name)
}

$sampleDirs = @($sampleDirs | Sort-Object FullName -Unique)
$samples = @()

foreach ($dir in $sampleDirs) {
    $resource = Read-JsonFile -Path (Find-ResourceFile -Dir $dir.FullName)
    $value = $resource.value
    $readOnly = Get-FirstPropertyValue -Object $value -Names @("read_only", "collection_read_only")
    $startsProcess = Get-FirstPropertyValue -Object $value -Names @("starts_process", "collection_starts_process")
    $sendsPrompt = Get-FirstPropertyValue -Object $value -Names @("sends_prompt", "collection_sends_prompt")
    $writesModelWeights = Get-FirstPropertyValue -Object $value -Names @("writes_model_weights", "collection_writes_model_weights")
    $approvedOwnerFlow = Get-FirstPropertyValue -Object $value -Names @("approved_owner_flow", "collected_by_approved_owner_flow")
    $memoryGb = if ($value) { Get-MemoryGb -Value $value } else { $null }
    $metalAvailable = if ($value) { Get-MetalAvailable -Value $value } else { $null }

    $completeParseOk = ($resource.exists -eq $true -and [string]::IsNullOrWhiteSpace([string]$resource.parse_error))
    $safeContractOk = (
        $completeParseOk -and
        $readOnly -eq $true -and
        $startsProcess -eq $false -and
        $sendsPrompt -eq $false -and
        $writesModelWeights -ne $true -and
        $approvedOwnerFlow -eq $true
    )
    $headroomKnown = ($null -ne $memoryGb -and $null -ne $metalAvailable)
    $headroomOk = ($headroomKnown -and [double]$memoryGb -ge $MinAvailableMemoryGb -and $metalAvailable -eq $true)

    $samples += [pscustomobject]@{
        id = $dir.Name
        path = ConvertTo-RelativePath -Root $root -Path $dir.FullName
        sample_time_utc = if ($resource.last_write_time_utc) { $resource.last_write_time_utc.ToString("o") } else { $null }
        complete_parse_ok = $completeParseOk
        safe_contract_ok = $safeContractOk
        headroom_known = $headroomKnown
        headroom_ok = $headroomOk
        file = [pscustomobject]@{
            path = ConvertTo-RelativePath -Root $root -Path $resource.path
            exists = $resource.exists
            parse_error = $resource.parse_error
        }
        extracted = [pscustomobject]@{
            read_only = $readOnly
            starts_process = $startsProcess
            sends_prompt = $sendsPrompt
            writes_model_weights = $writesModelWeights
            approved_owner_flow = $approvedOwnerFlow
            memory_available_gb = $memoryGb
            min_available_memory_gb = $MinAvailableMemoryGb
            metal_available = $metalAvailable
        }
    }
}

$sampleTimesParsed = @($samples | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.sample_time_utc) } | ForEach-Object { [datetime]$_.sample_time_utc } | Sort-Object)
$spanSeconds = if ($sampleTimesParsed.Count -ge 2) { [int][math]::Round((($sampleTimesParsed | Select-Object -Last 1) - ($sampleTimesParsed | Select-Object -First 1)).TotalSeconds) } else { 0 }
$completeSampleCount = @($samples | Where-Object { $_.complete_parse_ok -eq $true }).Count
$safeSampleCount = @($samples | Where-Object { $_.safe_contract_ok -eq $true }).Count
$knownHeadroomSampleCount = @($samples | Where-Object { $_.headroom_known -eq $true }).Count
$okHeadroomSampleCount = @($samples | Where-Object { $_.headroom_ok -eq $true }).Count
$resourceWindowPresent = (
    $samples.Count -ge $MinSamples -and
    $completeSampleCount -eq $samples.Count -and
    $safeSampleCount -eq $samples.Count -and
    $knownHeadroomSampleCount -eq $samples.Count -and
    $okHeadroomSampleCount -eq $samples.Count -and
    $spanSeconds -ge ($MinSpanMinutes * 60)
)

$status = if ($samples.Count -eq 0) {
    "missing_resource_window"
} elseif ($samples.Count -lt $MinSamples) {
    "insufficient_samples"
} elseif ($completeSampleCount -ne $samples.Count) {
    "incomplete_or_unparseable_samples"
} elseif ($safeSampleCount -ne $samples.Count) {
    "unsafe_collection_contract"
} elseif ($knownHeadroomSampleCount -ne $samples.Count) {
    "headroom_unknown"
} elseif ($okHeadroomSampleCount -ne $samples.Count) {
    "headroom_below_threshold"
} elseif ($spanSeconds -lt ($MinSpanMinutes * 60)) {
    "insufficient_span"
} else {
    "resource_window_observed_external_gate_required"
}

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.resource-window.v1"
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    window_dir = ConvertTo-RelativePath -Root $root -Path $resolvedWindowDir
    accepted_file_names = $resourceFileNames
    requirements = [pscustomobject]@{
        min_samples = $MinSamples
        min_span_minutes = $MinSpanMinutes
        min_available_memory_gb = $MinAvailableMemoryGb
        approved_owner_flow_required = $true
    }
    summary = [pscustomobject]@{
        status = $status
        sample_count = $samples.Count
        complete_sample_count = $completeSampleCount
        safe_sample_count = $safeSampleCount
        known_headroom_sample_count = $knownHeadroomSampleCount
        ok_headroom_sample_count = $okHeadroomSampleCount
        span_seconds = $spanSeconds
        resource_window_present = $resourceWindowPresent
        can_support_residency_review = $resourceWindowPresent
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "resource_window_reader_is_evidence_only_fail_closed"
    }
    samples = $samples
    note = "This script only reads already-collected local remote-resource artifacts. It never SSHes, runs model commands, sends prompts, launches processes, or writes model weights."
}

if ($Json) {
    $result | ConvertTo-Json -Depth 12
    exit 0
}

Write-Host "Gemma remote resource window"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "window_dir=$($result.window_dir)"
Write-Host "status=$($result.summary.status) samples=$($result.summary.sample_count) complete=$($result.summary.complete_sample_count) safe=$($result.summary.safe_sample_count) headroom_ok=$($result.summary.ok_headroom_sample_count) span_sec=$($result.summary.span_seconds) resource_window_present=$($result.summary.resource_window_present)"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
foreach ($sample in $result.samples) {
    Write-Host "  $($sample.id): complete=$($sample.complete_parse_ok) safe=$($sample.safe_contract_ok) headroom_ok=$($sample.headroom_ok) memory_gb=$($sample.extracted.memory_available_gb) metal=$($sample.extracted.metal_available) approved_owner_flow=$($sample.extracted.approved_owner_flow)"
}
Write-Host "Note: $($result.note)"
