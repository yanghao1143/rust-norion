param(
    [string]$RegistryPath = "docs/outreach/community-registry.yml"
)

$ErrorActionPreference = "Stop"

function Read-Registry {
    param([string]$Path)

    $result = @{
        templates = [ordered]@{}
        communities = @()
    }

    $section = ""
    $current = $null
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $Path) {
        if ($line -match '^templates:\s*$') {
            $section = "templates"
            continue
        }
        if ($line -match '^communities:\s*$') {
            $section = "communities"
            continue
        }

        if ($section -eq "templates" -and $line -match '^\s+([a-zA-Z_]+):\s*(.+?)\s*$') {
            $result.templates[$Matches[1]] = $Matches[2].Trim()
            continue
        }

        if ($section -eq "communities") {
            if ($line -match '^\s*-\s+id:\s*(.+?)\s*$') {
                if ($null -ne $current) {
                    $result.communities += [pscustomobject]$current
                }
                $current = [ordered]@{ id = $Matches[1].Trim() }
                continue
            }
            if ($null -ne $current -and $line -match '^\s+([a-zA-Z_]+):\s*(.*?)\s*$') {
                $current[$Matches[1]] = $Matches[2].Trim()
            }
        }
    }

    if ($null -ne $current) {
        $result.communities += [pscustomobject]$current
    }

    return $result
}

function Add-Failure {
    param(
        [System.Collections.Generic.List[string]]$Failures,
        [string]$Message
    )
    $Failures.Add($Message) | Out-Null
}

$repoRoot = (git rev-parse --show-toplevel).Trim()
Set-Location $repoRoot

if (-not (Test-Path -LiteralPath $RegistryPath -PathType Leaf)) {
    throw "Registry not found: $RegistryPath"
}

$allowedStatuses = @(
    "submitted",
    "candidate_pr",
    "candidate_issue_or_pr",
    "candidate_manual_review",
    "draft_ready_manual_login_required",
    "draft_ready_github_oauth_required",
    "draft_ready_manual_publish",
    "defer",
    "wait_for_major_update",
    "needs_manual_verification"
)

$requiredFields = @("id", "platform", "kind", "url", "audience", "fit", "status", "next_action", "notes")
$registry = Read-Registry -Path $RegistryPath
$failures = New-Object 'System.Collections.Generic.List[string]'

foreach ($name in @("short_zh", "short_en", "update_zh", "update_en")) {
    if (-not $registry.templates.Contains($name)) {
        Add-Failure $failures "Missing template entry: $name"
        continue
    }
    $path = [string]$registry.templates[$name]
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        Add-Failure $failures "Template path does not exist: $name -> $path"
    }
}

if ($registry.communities.Count -eq 0) {
    Add-Failure $failures "No communities found in registry."
}

$seenIds = @{}
$seenUrls = @{}
foreach ($community in $registry.communities) {
    foreach ($field in $requiredFields) {
        if (-not ($community.PSObject.Properties.Name -contains $field) -or [string]::IsNullOrWhiteSpace([string]$community.$field)) {
            Add-Failure $failures "Community '$($community.id)' missing required field: $field"
        }
    }

    if ($community.PSObject.Properties.Name -contains "id") {
        if ($seenIds.ContainsKey($community.id)) {
            Add-Failure $failures "Duplicate community id: $($community.id)"
        } else {
            $seenIds[$community.id] = $true
        }
    }

    if ($community.PSObject.Properties.Name -contains "url") {
        $normalizedUrl = ([string]$community.url).TrimEnd("/")
        if ($seenUrls.ContainsKey($normalizedUrl)) {
            Add-Failure $failures "Duplicate community url: $normalizedUrl"
        } else {
            $seenUrls[$normalizedUrl] = $true
        }
        if ($normalizedUrl -notmatch '^https?://') {
            Add-Failure $failures "Community '$($community.id)' has non-http URL: $normalizedUrl"
        }
    }

    if ($community.PSObject.Properties.Name -contains "status") {
        if ($allowedStatuses -notcontains $community.status) {
            Add-Failure $failures "Community '$($community.id)' has unsupported status: $($community.status)"
        }
        if ($community.status -eq "submitted") {
            if (-not ($community.PSObject.Properties.Name -contains "proof") -or [string]::IsNullOrWhiteSpace([string]$community.proof)) {
                Add-Failure $failures "Submitted community '$($community.id)' is missing proof URL."
            } elseif ([string]$community.proof -notmatch '^https?://') {
                Add-Failure $failures "Submitted community '$($community.id)' has non-http proof: $($community.proof)"
            }
        }
    }
}

if ($failures.Count -gt 0) {
    foreach ($failure in $failures) {
        Write-Error $failure
    }
    exit 1
}

Write-Host "community_registry_validation=PASS communities=$($registry.communities.Count) templates=$($registry.templates.Count)"
