param(
    [string]$RemoteHost = "192.168.10.11",
    [string]$RemoteUser = "xinghuan",
    [string]$IdentityFile = "$env:USERPROFILE\.ssh\smartsteam_mac_ed25519",
    [string]$RemoteModelDir = "/Users/xinghuan/smartsteam-model-box/models",
    [string]$LocalModelDir = "",
    [string]$QualityModelPath = "",
    [string]$HfEndpoint = "https://hf-mirror.com",
    [string]$DownloadManifest = "",
    [string]$HfTokenEnv = "HF_TOKEN",
    [switch]$DownloadMissing,
    [switch]$CheckOnly,
    [switch]$NoCopy,
    [switch]$DryRun,
    [switch]$JsonStatus,
    [string]$OutputJson = ""
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $scriptDir))
if ([string]::IsNullOrWhiteSpace($LocalModelDir)) {
    $LocalModelDir = Join-Path $repoRoot "target\model-cache"
}
if ([string]::IsNullOrWhiteSpace($QualityModelPath)) {
    $QualityModelPath = Join-Path $LocalModelDir "hf-gguf\tvall43\Qwen3.6-14B-A3B-FableVibes-GGUF\Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf"
}

function New-ModelSpec {
    param(
        [string]$Role,
        [string]$LocalPath,
        [string]$RemoteName
    )
    [pscustomobject]@{
        role = $Role
        local_path = $LocalPath
        remote_name = $RemoteName
    }
}

function ConvertTo-MirrorUrl {
    param([string]$Url)
    if ([string]::IsNullOrWhiteSpace($Url)) {
        return $Url
    }
    $endpoint = $HfEndpoint.TrimEnd("/")
    return ($Url -replace "^https://huggingface\.co", $endpoint)
}

function Read-DownloadMap {
    if ([string]::IsNullOrWhiteSpace($DownloadManifest)) {
        return @{}
    }
    if (-not (Test-Path -LiteralPath $DownloadManifest -PathType Leaf)) {
        throw "DownloadManifest not found: $DownloadManifest"
    }
    $raw = Get-Content -LiteralPath $DownloadManifest -Raw
    if ([string]::IsNullOrWhiteSpace($raw)) {
        return @{}
    }
    $json = $raw | ConvertFrom-Json
    $map = @{}
    foreach ($prop in $json.PSObject.Properties) {
        $map[$prop.Name] = [string]$prop.Value
    }
    return $map
}

function Invoke-ModelDownload {
    param(
        [pscustomobject]$Model,
        [hashtable]$DownloadMap
    )

    $url = $null
    if ($DownloadMap.ContainsKey($Model.remote_name)) {
        $url = $DownloadMap[$Model.remote_name]
    } elseif ($DownloadMap.ContainsKey($Model.role)) {
        $url = $DownloadMap[$Model.role]
    }

    if ([string]::IsNullOrWhiteSpace($url)) {
        throw "missing local model and no download URL for $($Model.remote_name); add it to -DownloadManifest"
    }

    $url = ConvertTo-MirrorUrl $url
    $dest = $Model.local_path
    $part = "$dest.part"
    $headers = @{}
    $token = [Environment]::GetEnvironmentVariable($HfTokenEnv)
    if (-not [string]::IsNullOrWhiteSpace($token)) {
        $headers["Authorization"] = "Bearer $token"
    }

    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $dest) | Out-Null
    if ($DryRun) {
        return [pscustomobject]@{ downloaded = $false; dry_run = $true; url = $url; path = $dest }
    }

    if (Test-Path -LiteralPath $part -PathType Leaf) {
        Remove-Item -LiteralPath $part -Force
    }
    Invoke-WebRequest -Uri $url -Headers $headers -OutFile $part
    Move-Item -LiteralPath $part -Destination $dest -Force
    return [pscustomobject]@{ downloaded = $true; dry_run = $false; url = $url; path = $dest }
}

function Invoke-RemoteText {
    param([string]$RemoteCommand)
    $sshArgs = @(
        "-i", $IdentityFile,
        "-o", "BatchMode=yes",
        "-o", "ConnectTimeout=8",
        "$RemoteUser@$RemoteHost",
        $RemoteCommand
    )
    $out = & ssh @sshArgs
    if ($LASTEXITCODE -ne 0) {
        throw "ssh failed with exit code $LASTEXITCODE"
    }
    return ($out -join "`n").Trim()
}

function Get-LocalSha256 {
    param([string]$Path)

    if (Get-Command Get-FileHash -ErrorAction SilentlyContinue) {
        $hash = Get-FileHash -LiteralPath $Path -Algorithm SHA256
        return $hash.Hash.ToLowerInvariant()
    }

    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $sha = [System.Security.Cryptography.SHA256]::Create()
        try {
            $bytes = $sha.ComputeHash($stream)
            return ([System.BitConverter]::ToString($bytes)).Replace("-", "").ToLowerInvariant()
        } finally {
            $sha.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
}

function Get-LocalFileMetadata {
    param([string]$LocalPath)

    if (-not (Test-Path -LiteralPath $LocalPath -PathType Leaf)) {
        return [pscustomobject]@{
            exists = $false
            bytes = $null
            sha256 = $null
            error = ""
        }
    }
    $item = Get-Item -LiteralPath $LocalPath
    return [pscustomobject]@{
        exists = $true
        bytes = [int64]$item.Length
        sha256 = Get-LocalSha256 $item.FullName
        error = ""
    }
}

function Get-RemoteFileMetadata {
    param([string]$RemotePath)
    $escaped = $RemotePath.Replace("'", "'\''")
    $remoteCommand = "if [ -f '$escaped' ]; then bytes=`$(stat -f '%z' '$escaped' 2>/dev/null || stat -c '%s' '$escaped' 2>/dev/null); hash=`$(shasum -a 256 '$escaped' 2>/dev/null | awk '{print `$1}'); if [ -z `"`$hash`" ]; then hash=`$(sha256sum '$escaped' 2>/dev/null | awk '{print `$1}'); fi; printf '%s\t%s\n' `"`$bytes`" `"`$hash`"; else echo __MISSING__; fi"
    try {
        $text = Invoke-RemoteText $remoteCommand
        if ($text -eq "__MISSING__" -or [string]::IsNullOrWhiteSpace($text)) {
            return [pscustomobject]@{
                exists = $false
                bytes = $null
                sha256 = $null
                error = ""
            }
        }
        $parts = $text -split "`t", 2
        $bytes = [int64]$parts[0]
        $sha = if ($parts.Count -gt 1) { $parts[1].Trim().ToLowerInvariant() } else { "" }
        return [pscustomobject]@{
            exists = $true
            bytes = $bytes
            sha256 = if ([string]::IsNullOrWhiteSpace($sha)) { $null } else { $sha }
            error = ""
        }
    } catch {
        return [pscustomobject]@{
            exists = $false
            bytes = $null
            sha256 = $null
            error = $_.Exception.Message
        }
    }
}

function Copy-ModelToRemote {
    param(
        [string]$LocalPath,
        [string]$RemotePath
    )
    if ($DryRun) {
        return
    }
    $target = "${RemoteUser}@${RemoteHost}:$RemotePath"
    $scpArgs = @(
        "-i", $IdentityFile,
        "-o", "BatchMode=yes",
        "-o", "ConnectTimeout=8",
        $LocalPath,
        $target
    )
    & scp @scpArgs
    if ($LASTEXITCODE -ne 0) {
        throw "scp failed with exit code $LASTEXITCODE"
    }
}

$models = @(
    New-ModelSpec "quality" $QualityModelPath "Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf"
    New-ModelSpec "summary" (Join-Path $LocalModelDir "gemma-3-270m-it-qat-Q4_0.gguf") "gemma-3-270m-it-qat-Q4_0.gguf"
    New-ModelSpec "review" (Join-Path $LocalModelDir "gemma-4-E4B-it-Q4_K_M.gguf") "gemma-4-E4B-it-Q4_K_M.gguf"
    New-ModelSpec "router" (Join-Path $LocalModelDir "functiongemma-270m-it-Q4_K_M.gguf") "functiongemma-270m-it-Q4_K_M.gguf"
    New-ModelSpec "index" (Join-Path $LocalModelDir "gemma-4-E2B-it-Q4_K_M.gguf") "gemma-4-E2B-it-Q4_K_M.gguf"
)

$downloadMap = Read-DownloadMap
if (-not $CheckOnly) {
    New-Item -ItemType Directory -Force -Path $LocalModelDir | Out-Null
}

if (-not ($DryRun -or $CheckOnly -or $NoCopy)) {
    $remoteDirEscaped = $RemoteModelDir.Replace("'", "'\''")
    Invoke-RemoteText "mkdir -p '$remoteDirEscaped'" | Out-Null
}

$results = @()
foreach ($model in $models) {
    $download = $null
    $downloadAttempted = $false
    $localMeta = Get-LocalFileMetadata $model.local_path
    if (-not $localMeta.exists) {
        if ($DownloadMissing -and -not $CheckOnly) {
            $downloadAttempted = $true
            $download = Invoke-ModelDownload $model $downloadMap
            $localMeta = Get-LocalFileMetadata $model.local_path
        } elseif (-not ($CheckOnly -or $DryRun)) {
            throw "local model missing: $($model.local_path); rerun with -DownloadMissing and -DownloadManifest"
        }
    }

    $remotePath = "$RemoteModelDir/$($model.remote_name)"
    $remoteMeta = Get-RemoteFileMetadata $remotePath
    if (-not [string]::IsNullOrWhiteSpace($remoteMeta.error) -and -not ($CheckOnly -or $DryRun -or $NoCopy)) {
        throw "remote metadata failed for $remotePath`: $($remoteMeta.error)"
    }

    $sizeMatches = $localMeta.exists -and $remoteMeta.exists -and ([int64]$remoteMeta.bytes -eq [int64]$localMeta.bytes)
    $shaMatches = (
        $localMeta.exists -and
        $remoteMeta.exists -and
        -not [string]::IsNullOrWhiteSpace($localMeta.sha256) -and
        -not [string]::IsNullOrWhiteSpace($remoteMeta.sha256) -and
        ($localMeta.sha256 -eq $remoteMeta.sha256)
    )
    $needsCopy = $localMeta.exists -and (-not $sizeMatches -or -not $shaMatches)
    $copied = $false
    $copySkippedReason = ""
    if ($needsCopy) {
        if ($CheckOnly) {
            $copySkippedReason = "check_only"
        } elseif ($NoCopy) {
            $copySkippedReason = "no_copy"
        } elseif ($DryRun) {
            $copySkippedReason = "dry_run"
        } else {
            Copy-ModelToRemote $model.local_path $remotePath
            $copied = $true
            $remoteMeta = Get-RemoteFileMetadata $remotePath
            if (-not [string]::IsNullOrWhiteSpace($remoteMeta.error)) {
                throw "remote metadata failed after copy for $remotePath`: $($remoteMeta.error)"
            }
            $sizeMatches = $localMeta.exists -and $remoteMeta.exists -and ([int64]$remoteMeta.bytes -eq [int64]$localMeta.bytes)
            $shaMatches = (
                $localMeta.exists -and
                $remoteMeta.exists -and
                -not [string]::IsNullOrWhiteSpace($localMeta.sha256) -and
                -not [string]::IsNullOrWhiteSpace($remoteMeta.sha256) -and
                ($localMeta.sha256 -eq $remoteMeta.sha256)
            )
            $needsCopy = -not ($sizeMatches -and $shaMatches)
        }
    }

    $results += [pscustomobject]@{
        role = $model.role
        name = $model.remote_name
        local_path = $model.local_path
        local_exists = [bool]$localMeta.exists
        local_bytes = $localMeta.bytes
        local_sha256 = $localMeta.sha256
        local_error = $localMeta.error
        remote_path = $remotePath
        remote_exists = [bool]$remoteMeta.exists
        remote_bytes = $remoteMeta.bytes
        remote_sha256 = $remoteMeta.sha256
        remote_error = $remoteMeta.error
        size_matches = [bool]$sizeMatches
        sha256_matches = [bool]$shaMatches
        copy_needed = [bool]$needsCopy
        copy_skipped_reason = $copySkippedReason
        copied = [bool]$copied
        download_attempted = [bool]$downloadAttempted
        downloaded = [bool]($download -and $download.downloaded)
        dry_run = [bool]$DryRun
        check_only = [bool]$CheckOnly
        no_copy = [bool]$NoCopy
        ok = [bool]($localMeta.exists -and $remoteMeta.exists -and $sizeMatches -and $shaMatches)
    }
}

$status = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-model-cache-sync.v1"
    read_only = [bool]$CheckOnly
    starts_process = $false
    sends_prompt = $false
    writes_files = [bool](-not [string]::IsNullOrWhiteSpace($OutputJson))
    copy_allowed = [bool](-not ($CheckOnly -or $NoCopy -or $DryRun))
    download_allowed = [bool]($DownloadMissing -and -not $CheckOnly)
    copies_files = [bool]($results | Where-Object { $_.copied } | Select-Object -First 1)
    downloads_files = [bool]($results | Where-Object { $_.downloaded } | Select-Object -First 1)
    hf_endpoint = $HfEndpoint
    local_model_dir = $LocalModelDir
    remote = [pscustomobject]@{
        host = $RemoteHost
        user = $RemoteUser
        model_dir = $RemoteModelDir
    }
    all_ok = -not ($results | Where-Object { -not $_.ok })
    models = $results
}

if (-not [string]::IsNullOrWhiteSpace($OutputJson)) {
    $outputDir = Split-Path -Parent $OutputJson
    if (-not [string]::IsNullOrWhiteSpace($outputDir)) {
        New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
    }
    $status | ConvertTo-Json -Depth 10 | Set-Content -Encoding UTF8 -LiteralPath $OutputJson
}

if ($JsonStatus) {
    $status | ConvertTo-Json -Depth 10
} else {
    Write-Host "SmartSteam remote model cache sync"
    Write-Host "read_only=$([bool]$CheckOnly) starts_process=false sends_prompt=false writes_files=$($status.writes_files) copy_allowed=$($status.copy_allowed) download_allowed=$($status.download_allowed) copies_files=$($status.copies_files) downloads_files=$($status.downloads_files)"
    Write-Host "hf_endpoint=$HfEndpoint"
    Write-Host "local_model_dir=$LocalModelDir"
    Write-Host "remote=${RemoteUser}@${RemoteHost}:$RemoteModelDir"
    if (-not [string]::IsNullOrWhiteSpace($OutputJson)) {
        Write-Host "output_json=$OutputJson"
    }
    foreach ($result in $results) {
        $action = if ($result.copied) {
            "copied"
        } elseif ($result.copy_needed) {
            "copy-needed/$($result.copy_skipped_reason)"
        } else {
            "already-present"
        }
        Write-Host ("{0}: {1} {2} local_exists={3} remote_exists={4} local={5} remote={6} size_matches={7} sha256_matches={8}" -f $result.role, $result.name, $action, $result.local_exists, $result.remote_exists, $result.local_bytes, $result.remote_bytes, $result.size_matches, $result.sha256_matches)
        if (-not [string]::IsNullOrWhiteSpace($result.remote_error)) {
            Write-Host ("  remote_error={0}" -f $result.remote_error)
        }
    }
    if (-not $status.all_ok) {
        throw "one or more remote model files did not match local size/hash"
    }
    Write-Host "model_cache_sync=PASS"
}
