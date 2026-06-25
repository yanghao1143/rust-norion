param(
    [string]$RegistryPath = "docs/outreach/community-registry.yml",
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

function Add-CommunityRows {
    param(
        [System.Collections.Generic.List[string]]$Lines,
        [string]$Heading,
        [object[]]$Items,
        [switch]$IncludeProof
    )

    $Lines.Add("## $Heading")
    $Lines.Add("")
    if ($Items.Count -eq 0) {
        $Lines.Add("- None.")
        $Lines.Add("")
        return
    }

    foreach ($item in $Items) {
        $proof = ""
        if ($IncludeProof -and ($item.PSObject.Properties.Name -contains "proof")) {
            $proof = " proof=$($item.proof)"
        }
        $Lines.Add("- $($item.id) [$($item.platform) / $($item.kind)] fit=$($item.fit) status=$($item.status)$proof")
        $Lines.Add("  - URL: $($item.url)")
        $Lines.Add("  - Next action: $($item.next_action)")
    }
    $Lines.Add("")
}

$repoRoot = (git rev-parse --show-toplevel).Trim()
Set-Location $repoRoot

$communities = @(Read-RegistryCommunities -Path $RegistryPath)
$submitted = @($communities | Where-Object { $_.status -eq "submitted" })
$readyIssue = @($communities | Where-Object { $_.status -eq "candidate_issue_or_pr" -or $_.status -eq "candidate_manual_review" })
$readyPr = @($communities | Where-Object { $_.status -eq "candidate_pr" })
$manualLogin = @($communities | Where-Object { $_.status -match '^draft_ready|needs_manual_verification' })
$waiting = @($communities | Where-Object { $_.status -match '^wait_for_major_update$|^defer$' })
$iterationUpdates = @(
    $communities |
        Where-Object {
            $_.status -notmatch '^defer$' -and (
                $_.kind -match 'weekly|forum_topic|article|project_request|weekly_project_list' -or
                $_.status -eq 'wait_for_major_update' -or
                $_.next_action -match 'update|release|article'
            )
        }
)

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("# rust-norion Community Outreach Queue")
$lines.Add("")
$lines.Add("Generated: $((Get-Date).ToUniversalTime().ToString('yyyy-MM-dd HH:mm:ss')) UTC")
$lines.Add("Registry: $RegistryPath")
$lines.Add("")
$lines.Add("Summary:")
$lines.Add("")
$lines.Add("- total=$($communities.Count)")
$lines.Add("- submitted=$($submitted.Count)")
$lines.Add("- issue_or_manual_candidates=$($readyIssue.Count)")
$lines.Add("- pr_candidates=$($readyPr.Count)")
$lines.Add("- manual_login_or_verification=$($manualLogin.Count)")
$lines.Add("- deferred_or_waiting=$($waiting.Count)")
$lines.Add("- iteration_update_candidates=$($iterationUpdates.Count)")
$lines.Add("")
$lines.Add("Rules:")
$lines.Add("")
$lines.Add("- Do not bulk-post identical text.")
$lines.Add("- Prefer a PR when the target repository explicitly asks for PRs.")
$lines.Add("- Use an issue only when suggestions/questions are allowed or category fit is uncertain.")
$lines.Add("- Treat Gitee mirrors as discovery leads unless an active contribution path is verified.")
$lines.Add("")

Add-CommunityRows -Lines $lines -Heading "Already Submitted" -Items $submitted -IncludeProof
Add-CommunityRows -Lines $lines -Heading "Ready For Issue Or Manual Review" -Items $readyIssue
Add-CommunityRows -Lines $lines -Heading "Ready For Focused PR" -Items $readyPr
Add-CommunityRows -Lines $lines -Heading "Ready For Iteration Updates" -Items $iterationUpdates
Add-CommunityRows -Lines $lines -Heading "Manual Login Or Verification Required" -Items $manualLogin
Add-CommunityRows -Lines $lines -Heading "Deferred Or Waiting" -Items $waiting

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
