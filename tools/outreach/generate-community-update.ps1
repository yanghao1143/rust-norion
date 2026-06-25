param(
    [int]$SinceDays = 30,
    [int]$MaxCommits = 20,
    [string]$RegistryPath = "docs/outreach/community-registry.yml",
    [string]$TemplateDir = "docs/outreach/templates",
    [string]$OutFile = ""
)

$ErrorActionPreference = "Stop"
$OutputEncoding = [System.Text.Encoding]::UTF8
try {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
} catch {
    # Some non-interactive hosts do not expose a writable console.
}

function Read-RegistryCommunities {
    param([string]$Path)

    $items = @()
    $current = $null
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $Path) {
        if ($line -match '^\s*-\s+id:\s*(.+?)\s*$') {
            if ($null -ne $current) {
                $items += [pscustomobject]$current
            }
            $current = [ordered]@{ id = $Matches[1].Trim() }
            continue
        }
        if ($null -ne $current -and $line -match '^\s+([a-zA-Z_]+):\s*(.*?)\s*$') {
            $current[$Matches[1]] = $Matches[2].Trim()
        }
    }
    if ($null -ne $current) {
        $items += [pscustomobject]$current
    }
    return $items
}

function Read-Template {
    param([string]$Path)
    if (Test-Path -LiteralPath $Path -PathType Leaf) {
        return (Get-Content -Encoding UTF8 -Raw -LiteralPath $Path).Trim()
    }
    return ""
}

$repoRoot = (git rev-parse --show-toplevel).Trim()
Set-Location $repoRoot

$since = (Get-Date).ToUniversalTime().AddDays(-1 * $SinceDays).ToString("yyyy-MM-dd")
$commits = @(git log --since=$since --max-count=$MaxCommits --pretty=format:"- %h %s" --no-merges 2>$null)
if ($commits.Count -eq 0) {
    $commits = @("- No non-merge commits found in the last $SinceDays days.")
} elseif ($commits.Count -ge $MaxCommits) {
    $commits += "- ... capped at $MaxCommits commits; inspect `git log --since=$since --no-merges` for the full list."
}

$communities = @(Read-RegistryCommunities -Path $RegistryPath)
$ready = @(
    $communities |
        Where-Object {
            $_.status -match 'candidate|draft_ready|wait_for_major_update|needs_manual_verification' -and
            $_.status -notmatch '^defer$'
        } |
        Select-Object -First 8
)
$submitted = @($communities | Where-Object { $_.status -eq "submitted" })

$shortEn = Read-Template -Path (Join-Path $TemplateDir "rust-ai-project-short-en.md")
$shortZh = Read-Template -Path (Join-Path $TemplateDir "rust-ai-project-short-zh.md")
$updateEn = Read-Template -Path (Join-Path $TemplateDir "iteration-update-en.md")
$updateZh = Read-Template -Path (Join-Path $TemplateDir "iteration-update-zh.md")

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("# rust-norion Community Outreach Draft")
$lines.Add("")
$lines.Add("Generated: $((Get-Date).ToUniversalTime().ToString("yyyy-MM-dd HH:mm:ss")) UTC")
$lines.Add("Window: last $SinceDays days, since $since UTC")
$lines.Add("Commit cap: $MaxCommits")
$lines.Add("")
$lines.Add("## Recent Repository Changes")
$lines.Add("")
$lines.AddRange([string[]]$commits)
$lines.Add("")
$lines.Add("## Already Submitted")
$lines.Add("")
if ($submitted.Count -eq 0) {
    $lines.Add("- None recorded yet.")
} else {
    foreach ($item in $submitted) {
        $proof = if ($item.PSObject.Properties.Name -contains "proof") { $item.proof } else { "" }
        $lines.Add("- $($item.id): $proof")
    }
}
$lines.Add("")
$lines.Add("## Candidate Channels For The Next Manual Cycle")
$lines.Add("")
if ($ready.Count -eq 0) {
    $lines.Add("- No candidate channels found. Review `$RegistryPath`.")
} else {
    foreach ($item in $ready) {
        $lines.Add("- $($item.id) [$($item.platform) / $($item.kind)] status=$($item.status)")
        $lines.Add("  - URL: $($item.url)")
        $lines.Add("  - Next action: $($item.next_action)")
    }
}
$lines.Add("")
$lines.Add("## English Short Pitch")
$lines.Add("")
$lines.Add($shortEn)
$lines.Add("")
$lines.Add("## Chinese Short Pitch")
$lines.Add("")
$lines.Add($shortZh)
$lines.Add("")
$lines.Add("## English Update Template")
$lines.Add("")
$lines.Add($updateEn)
$lines.Add("")
$lines.Add("## Chinese Update Template")
$lines.Add("")
$lines.Add($updateZh)
$lines.Add("")
$lines.Add("## Manual Submission Rules")
$lines.Add("")
$lines.Add("- Tailor the post to the selected community.")
$lines.Add("- Submit only where project self-submission, weekly tips, resource-list PRs, or discussions are allowed.")
$lines.Add("- Do not bulk-post identical recruitment text.")
$lines.Add("- Record every proof URL in `docs/outreach/community-registry.yml`.")

$output = ($lines -join [Environment]::NewLine)

if (-not [string]::IsNullOrWhiteSpace($OutFile)) {
    $parent = Split-Path -Parent $OutFile
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    Set-Content -Encoding UTF8 -LiteralPath $OutFile -Value $output
} else {
    Write-Output $output
}
