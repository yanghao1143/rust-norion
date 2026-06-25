param(
    [int]$LimitPerQuery = 20,
    [int]$MinStars = 50,
    [string]$RegistryPath = "docs/outreach/community-registry.yml",
    [string]$OutFile = "",
    [switch]$IncludeGitee,
    [switch]$IncludeProjectRepos
)

$ErrorActionPreference = "Stop"
$OutputEncoding = [System.Text.Encoding]::UTF8
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}
try {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
} catch {
    # Some non-interactive hosts do not expose a writable console.
}

function Read-KnownUrls {
    param([string]$Path)

    $known = New-Object 'System.Collections.Generic.HashSet[string]'
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $known
    }

    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $Path) {
        if ($line -match '^\s+url:\s*(https?://\S+)\s*$') {
            [void]$known.Add($Matches[1].Trim().TrimEnd('/'))
        }
    }
    return $known
}

function ConvertTo-Slug {
    param([string]$Value)
    return ($Value.ToLowerInvariant() -replace '[^a-z0-9]+', '-' -replace '(^-|-$)', '')
}

function Get-FitHint {
    param([string]$Name, [string]$Description)

    $text = "$Name $Description".ToLowerInvariant()
    if ($text -match 'awesome.*(rust|llm|ai)|rust.*(ai|llm|machine learning)|weekly|this week in rust|周刊') {
        return "high"
    }
    if ($text -match 'newsletter' -and $text -match 'curated|digest|weekly|community|roundup') {
        return "high"
    }
    if ($text -match 'agent|memory|runtime|inference|gateway|rag|ml') {
        return "medium"
    }
    return "low"
}

function Get-RecommendedAction {
    param([string]$Name, [string]$Description)

    $text = "$Name $Description".ToLowerInvariant()
    if ($text -match 'weekly|this week in rust|周刊' -or ($text -match 'newsletter' -and $text -match 'curated|digest|community|roundup')) {
        return "inspect weekly/news-tip rules before submitting a release or article update"
    }
    if ($text -match 'awesome|curated|directory|list') {
        return "inspect contribution rules and submit a tailored PR only if maturity/category fit is clear"
    }
    if ($text -match 'community|forum|discussion') {
        return "inspect discussion/category rules before posting a contributor call"
    }
    return "manual review required before any outreach"
}

function Test-CommunitySurface {
    param([string]$Name, [string]$Description)

    $text = "$Name $Description".ToLowerInvariant()
    if ($text -match 'awesome|curated|directory|list|weekly|this week in rust|community|forum|book|learning resource|资源|周刊|社区|教程') {
        return $true
    }
    return $text -match 'newsletter' -and $text -match 'digest|roundup|weekly|curated|community'
}

function Search-GitHubRepos {
    param([string[]]$Queries, [int]$Limit, [int]$MinimumStars)

    $rows = @()
    foreach ($query in $Queries) {
        $json = ""
        try {
            $json = gh search repos $query --limit $Limit --json fullName,description,stargazersCount,url,updatedAt 2>$null
        } catch {
            $global:LASTEXITCODE = 0
            continue
        }
        if ($LASTEXITCODE -ne 0) {
            $global:LASTEXITCODE = 0
            continue
        }
        if ([string]::IsNullOrWhiteSpace($json)) {
            continue
        }
        foreach ($repo in ($json | ConvertFrom-Json)) {
            if ([int]$repo.stargazersCount -lt $MinimumStars) {
                continue
            }
            if (-not $IncludeProjectRepos -and -not (Test-CommunitySurface -Name $repo.fullName -Description $repo.description)) {
                continue
            }
            $rows += [pscustomobject]@{
                platform = "github"
                id = ConvertTo-Slug $repo.fullName
                name = $repo.fullName
                url = $repo.url
                stars = [int]$repo.stargazersCount
                fit = Get-FitHint -Name $repo.fullName -Description $repo.description
                action = Get-RecommendedAction -Name $repo.fullName -Description $repo.description
                description = (($repo.description | Out-String).Trim() -replace '\s+', ' ')
                source_query = $query
            }
        }
    }
    return $rows
}

function Search-GiteeRepos {
    param([string[]]$Queries, [int]$Limit, [int]$MinimumStars)

    $rows = @()
    foreach ($query in $Queries) {
        $uri = "https://gitee.com/api/v5/search/repositories?q=$([uri]::EscapeDataString($query))&sort=stars_count&order=desc&page=1&per_page=$Limit"
        try {
            $items = Invoke-RestMethod -Uri $uri -Method Get -TimeoutSec 20
        } catch {
            Write-Warning "Gitee search failed for '$query': $($_.Exception.GetType().Name)"
            continue
        }
        foreach ($repo in $items) {
            $stars = if ($null -ne $repo.stargazers_count) { [int]$repo.stargazers_count } else { 0 }
            if ($stars -lt $MinimumStars) {
                continue
            }
            $name = if ($repo.full_name) { $repo.full_name } else { $repo.human_name }
            if (-not $IncludeProjectRepos -and -not (Test-CommunitySurface -Name $name -Description $repo.description)) {
                continue
            }
            $rows += [pscustomobject]@{
                platform = "gitee"
                id = ConvertTo-Slug $name
                name = $name
                url = $repo.html_url
                stars = $stars
                fit = Get-FitHint -Name $name -Description $repo.description
                action = Get-RecommendedAction -Name $name -Description $repo.description
                description = (($repo.description | Out-String).Trim() -replace '\s+', ' ')
                source_query = $query
            }
        }
    }
    return $rows
}

$repoRoot = (git rev-parse --show-toplevel).Trim()
Set-Location $repoRoot

$knownUrls = Read-KnownUrls -Path $RegistryPath

$githubQueries = @(
    "awesome rust ai",
    "awesome rust llm",
    "awesome rust ai governance",
    "awesome rust ai tools",
    "local rust ai tools",
    "awesome ai agents",
    "awesome ai agent frameworks",
    "awesome llm agents",
    "awesome agentic ai",
    "awesome llmops",
    "awesome ai sdks",
    "awesome local llm",
    "awesome autonomous agents",
    "awesome multi agent systems",
    "awesome multi-agent systems",
    "awesome agent security",
    "awesome agentic security",
    "awesome ai infrastructure",
    "awesome ai engineering",
    "awesome ai engineering resources",
    "awesome ai tools",
    "awesome genai tools",
    "awesome generative ai tools",
    "awesome ai developer tools",
    "awesome ai safety tools",
    "awesome ai for security",
    "awesome ai guardrails",
    "awesome llm guardrails",
    "awesome agent guardrails",
    "awesome responsible ai tools",
    "awesome prompt injection",
    "awesome prompt injection defenses",
    "awesome ai agent evaluation",
    "awesome agent evaluation",
    "awesome agent benchmark",
    "awesome agentops",
    "awesome ai agentops",
    "awesome rag tools",
    "awesome rag evaluation",
    "awesome llm observability",
    "awesome llm monitoring",
    "awesome llm tracing",
    "awesome llm security",
    "awesome llm agent security",
    "awesome llm self improvement",
    "awesome llm systems",
    "awesome llm evaluation",
    "awesome llm eval",
    "awesome llm benchmark",
    "awesome llm planning",
    "awesome ai gateway",
    "awesome ai workflows",
    "awesome ai automation",
    "awesome software engineering agents",
    "awesome code agents",
    "awesome coding agents",
    "awesome ai coding agents",
    "awesome ai code agents",
    "awesome vibe coding",
    "awesome context engineering",
    "awesome harness engineering",
    "awesome agent harness",
    "awesome agent runtime",
    "awesome agent orchestration",
    "awesome agent memory",
    "awesome ai memory",
    "awesome agent evolution",
    "awesome self evolving agents",
    "awesome efficient agents",
    "awesome llm apps",
    "awesome llm app",
    "awesome llm applications",
    "awesome llm tools",
    "awesome agent tools",
    "awesome agentic evaluation",
    "lifelong llm agent",
    "rust ai weekly",
    "rust machine learning",
    "best of rust machine learning",
    "awesome rust neural network",
    "rust agent framework",
    "rust llm memory",
    "rust inference gateway",
    "rust newsletter"
)

$giteeQueries = @(
    "Rust AI",
    "Rust LLM",
    "Rust 人工智能",
    "Rust 机器学习",
    "Rust Agent",
    "LLMOps",
    "Agent memory",
    "Agent security",
    "LLM security",
    "context engineering",
    "Rust 周刊",
    "awesome Rust"
)

$rows = @(Search-GitHubRepos -Queries $githubQueries -Limit $LimitPerQuery -MinimumStars $MinStars)
if ($IncludeGitee) {
    $rows += @(Search-GiteeRepos -Queries $giteeQueries -Limit $LimitPerQuery -MinimumStars $MinStars)
}

$unique = [ordered]@{}
foreach ($row in $rows) {
    if ([string]::IsNullOrWhiteSpace($row.url)) {
        continue
    }
    $normalized = $row.url.TrimEnd('/')
    if ($knownUrls.Contains($normalized)) {
        continue
    }
    if (-not $unique.Contains($normalized)) {
        $unique[$normalized] = $row
    }
}

$ranked = @($unique.Values | Sort-Object @{ Expression = { if ($_.fit -eq "high") { 0 } elseif ($_.fit -eq "medium") { 1 } else { 2 } } }, @{ Expression = "stars"; Descending = $true })

$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("# Community Candidate Discovery")
$lines.Add("")
$lines.Add("Generated: $((Get-Date).ToUniversalTime().ToString("yyyy-MM-dd HH:mm:ss")) UTC")
$lines.Add("Minimum stars: $MinStars")
$lines.Add("Limit per query: $LimitPerQuery")
$lines.Add("Includes Gitee: $IncludeGitee")
$lines.Add("Includes ordinary project repos: $IncludeProjectRepos")
$lines.Add("")
$lines.Add("This is a discovery list only. Do not post automatically. Inspect each community's contribution rules before outreach.")
$lines.Add("")
$lines.Add("| Fit | Stars | Platform | Candidate | Next action | Source query |")
$lines.Add("| --- | ---: | --- | --- | --- | --- |")
foreach ($row in $ranked) {
    $description = if ([string]::IsNullOrWhiteSpace($row.description)) { "" } else { " - $($row.description)" }
    $candidate = "[$($row.name)]($($row.url))$description"
    $queryText = [string]$row.source_query
    $lines.Add("| $($row.fit) | $($row.stars) | $($row.platform) | $candidate | $($row.action) | " + '`' + $queryText + '`' + " |")
}

if ($ranked.Count -eq 0) {
    $lines.Add("")
    $lines.Add("No new candidates found after filtering known registry URLs.")
}

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

$global:LASTEXITCODE = 0
