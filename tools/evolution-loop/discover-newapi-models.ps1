param(
    [string]$OutputJson = "target\evolution\newapi-model-discovery.json",
    [string]$MatrixMarkdown = "target\evolution\newapi-model-matrix.md",
    [switch]$Probe,
    [int]$MaxProbeModels = 24,
    [int]$ProbeTimeoutSecs = 45,
    [int]$ProbeMaxTokens = 24,
    [string]$ProbePrompt = "Reply with exactly: norion-newapi-probe-ok",
    [switch]$SelfTest,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Discover NewAPI models and write a repeatable candidate matrix."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\evolution-loop\discover-newapi-models.cmd"
    Write-Host "  .\tools\evolution-loop\discover-newapi-models.cmd -Probe -MaxProbeModels 12"
    Write-Host "  .\tools\evolution-loop\discover-newapi-models.cmd -SelfTest"
    Write-Host ""
    Write-Host "Environment:"
    Write-Host "  NORION_NEWAPI_BASE_URL   OpenAI-compatible NewAPI base URL"
    Write-Host "  NORION_NEWAPI_API_KEY    API key used only in the Authorization header"
    Write-Host ""
    Write-Host "Policy:"
    Write-Host "  GPT-5 and GPT-5-or-newer model ids are always excluded."
    return
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)
Set-Location $RepoRoot

function Resolve-ArtifactPath {
    param([string]$Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }
    return (Join-Path $RepoRoot $Path)
}

function New-NewApiUrl {
    param(
        [string]$BaseUrl,
        [string]$Path
    )

    $trimmed = $BaseUrl.Trim().TrimEnd("/")
    if ($trimmed.EndsWith("/v1")) {
        return "$trimmed/$Path"
    }
    return "$trimmed/v1/$Path"
}

function Get-NewApiHeaders {
    $key = [string]$env:NORION_NEWAPI_API_KEY
    if ([string]::IsNullOrWhiteSpace($key)) {
        throw "NORION_NEWAPI_API_KEY is required unless -SelfTest is used"
    }
    return @{
        Authorization = "Bearer $key"
    }
}

function Test-ForbiddenGpt5OrHigher {
    param([string]$ModelId)

    $normalized = $ModelId.Trim().ToLowerInvariant()
    if ($normalized -match '(^|[^a-z0-9])(?:chat)?gpt[-_\. ]?(?<major>\d+)(?:[^\d]|$)') {
        return ([int]$Matches["major"] -ge 5)
    }
    return $false
}

function Get-ModelRoles {
    param([string]$ModelId)

    $normalized = $ModelId.Trim().ToLowerInvariant()
    if ($normalized -match 'qwen3\.5-397b-a17b') {
        return @("heavy_reasoning", "coding")
    }
    if ($normalized -match 'qwen3-next-80b-a3b-instruct') {
        return @("heavy_reasoning")
    }

    $roles = @()
    if (
        $normalized -match '(mini|flash|lite|small|haiku|router|review|fast|turbo|270m)' -or
        $normalized -match '(^|[^0-9])(?:8b|7b|4b|3b|1\.5b|1b)([^0-9]|$)'
    ) {
        $roles += "fast_router_reviewer"
    }
    if ($normalized -match '(reason|thinking|deepseek-r1|r1|(^|[^a-z0-9])o[134]([^a-z0-9]|$)|opus|large|70b|72b|80b|120b|397b|405b|671b)') {
        $roles += "heavy_reasoning"
    }
    if ($normalized -match '(code|coder|coding|codestral|devstral|program|qwen.*coder|claude.*sonnet)') {
        $roles += "coding"
    }
    if ($roles.Count -eq 0) {
        $roles += "fallback"
    }
    return @($roles | Select-Object -Unique)
}

function Get-ManualFailureReasons {
    param([string]$ModelId)

    $normalized = $ModelId.Trim().ToLowerInvariant()
    $reasons = @()
    if ($normalized -match 'moonshotai/kimi-k2\.6') {
        $reasons += "manual_evidence_unstable_repetitive_output"
    }
    return $reasons
}

function Get-ManualEvidenceNotes {
    param([string]$ModelId)

    $normalized = $ModelId.Trim().ToLowerInvariant()
    $notes = @()
    if ($normalized -match 'qwen3\.5-397b-a17b') {
        $notes += "2026-06-24: present in /v1/models; short chat-completion usable. Exact-ok probe returned anomalous tool text in about 2904ms, but evolution-loop three-line contract prompt succeeded in about 15879ms with risk/change_request/verification output. Rust coding probe succeeded in about 6274ms with clean add_one code. Mark usable for heavy_reasoning/coding, not fast router default."
    }
    if ($normalized -match 'qwen3-next-80b-a3b-instruct') {
        $notes += "2026-06-24: evolution-loop three-line contract prompt succeeded in about 3417ms and was stable in the observed run."
    }
    if ($normalized -match 'moonshotai/kimi-k2\.6') {
        $notes += "2026-06-24: observed around 40717ms with repeated Destruct output; downgrade and mark unstable until a later probe disproves it."
    }
    return $notes
}

function Get-PrimaryTier {
    param([string[]]$Roles)

    foreach ($tier in @("coding", "heavy_reasoning", "fast_router_reviewer", "fallback")) {
        if ($Roles -contains $tier) {
            return $tier
        }
    }
    return "fallback"
}

function Get-ModelIdsFromResponse {
    param($Response)

    $items = @()
    if ($null -ne $Response.data) {
        $items = @($Response.data)
    } elseif ($null -ne $Response.models) {
        $items = @($Response.models)
    } elseif ($Response -is [array]) {
        $items = @($Response)
    } else {
        $items = @($Response)
    }

    $ids = @()
    foreach ($item in $items) {
        if ($item -is [string]) {
            $ids += $item
            continue
        }
        foreach ($property in @("id", "model", "name")) {
            if ($null -ne $item.$property -and -not [string]::IsNullOrWhiteSpace([string]$item.$property)) {
                $ids += [string]$item.$property
                break
            }
        }
    }
    return @($ids | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique)
}

function Invoke-ModelList {
    $baseUrl = [string]$env:NORION_NEWAPI_BASE_URL
    if ([string]::IsNullOrWhiteSpace($baseUrl)) {
        throw "NORION_NEWAPI_BASE_URL is required unless -SelfTest is used"
    }
    $uri = New-NewApiUrl -BaseUrl $baseUrl -Path "models"
    $response = Invoke-RestMethod -Method Get -Uri $uri -Headers (Get-NewApiHeaders) -TimeoutSec 45
    return Get-ModelIdsFromResponse -Response $response
}

function Invoke-ModelProbe {
    param(
        [string]$ModelId,
        [int]$TimeoutSecs,
        [int]$MaxTokens,
        [string]$Prompt
    )

    $baseUrl = [string]$env:NORION_NEWAPI_BASE_URL
    $uri = New-NewApiUrl -BaseUrl $baseUrl -Path "chat/completions"
    $body = @{
        model = $ModelId
        messages = @(
            @{ role = "system"; content = "You are a terse health probe." },
            @{ role = "user"; content = $Prompt }
        )
        temperature = 0
        max_tokens = $MaxTokens
        stream = $false
    } | ConvertTo-Json -Depth 8

    $started = Get-Date
    try {
        $response = Invoke-RestMethod -Method Post -Uri $uri -Headers (Get-NewApiHeaders) -ContentType "application/json" -Body $body -TimeoutSec $TimeoutSecs
        $elapsedMs = [int]((Get-Date) - $started).TotalMilliseconds
        $content = ""
        if ($null -ne $response.choices -and $response.choices.Count -gt 0) {
            if ($null -ne $response.choices[0].message.content) {
                $content = [string]$response.choices[0].message.content
            } elseif ($null -ne $response.choices[0].text) {
                $content = [string]$response.choices[0].text
            }
        }
        return [pscustomobject]@{
            ok = $true
            elapsed_ms = $elapsedMs
            response_chars = $content.Length
            error = $null
        }
    } catch {
        $elapsedMs = [int]((Get-Date) - $started).TotalMilliseconds
        return [pscustomobject]@{
            ok = $false
            elapsed_ms = $elapsedMs
            response_chars = 0
            error = $_.Exception.Message
        }
    }
}

function New-DiscoveryReport {
    param(
        [string[]]$ModelIds,
        [bool]$RunProbe,
        [int]$MaxProbeCount,
        [int]$TimeoutSecs,
        [int]$MaxTokens,
        [string]$Prompt
    )

    $records = @()
    foreach ($id in $ModelIds) {
        $forbidden = Test-ForbiddenGpt5OrHigher -ModelId $id
        $roles = if ($forbidden) { @() } else { Get-ModelRoles -ModelId $id }
        $reasons = @()
        if ($forbidden) {
            $reasons += "policy_excluded_gpt5_or_higher"
        }
        $reasons += Get-ManualFailureReasons -ModelId $id
        $records += [pscustomobject]@{
            model = $id
            allowed_by_policy = (-not $forbidden)
            candidate_roles = $roles
            primary_tier = if ($forbidden) { "excluded" } else { Get-PrimaryTier -Roles $roles }
            evidence_notes = @(Get-ManualEvidenceNotes -ModelId $id)
            probe = [pscustomobject]@{
                requested = $false
                ok = $null
                elapsed_ms = $null
                response_chars = $null
                error = $null
            }
            failure_reasons = $reasons
        }
    }

    $probeable = @($records | Where-Object { $_.allowed_by_policy } | Select-Object -First $MaxProbeCount)
    if ($RunProbe) {
        foreach ($record in $probeable) {
            $probeResult = Invoke-ModelProbe -ModelId $record.model -TimeoutSecs $TimeoutSecs -MaxTokens $MaxTokens -Prompt $Prompt
            $record.probe = [pscustomobject]@{
                requested = $true
                ok = $probeResult.ok
                elapsed_ms = $probeResult.elapsed_ms
                response_chars = $probeResult.response_chars
                error = $probeResult.error
            }
            if (-not $probeResult.ok) {
                $record.failure_reasons += "probe_failed"
            }
        }
    }

    $candidateRecords = @($records | Where-Object {
        $_.allowed_by_policy -and @($_.failure_reasons).Count -eq 0 -and ($_.probe.requested -ne $true -or $_.probe.ok -eq $true)
    })
    $failedRecords = @($records | Where-Object {
        (-not $_.allowed_by_policy) -or @($_.failure_reasons).Count -gt 0 -or ($_.probe.requested -eq $true -and $_.probe.ok -ne $true)
    })

    $tiers = [ordered]@{
        fast_router_reviewer = @($candidateRecords | Where-Object { $_.candidate_roles -contains "fast_router_reviewer" } | Select-Object -ExpandProperty model)
        heavy_reasoning = @($candidateRecords | Where-Object { $_.candidate_roles -contains "heavy_reasoning" } | Select-Object -ExpandProperty model)
        coding = @($candidateRecords | Where-Object { $_.candidate_roles -contains "coding" } | Select-Object -ExpandProperty model)
        fallback = @($candidateRecords | Where-Object { $_.candidate_roles -contains "fallback" } | Select-Object -ExpandProperty model)
    }

    return [pscustomobject]@{
        schema = "norion.newapi_model_discovery.v1"
        generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
        source = [pscustomobject]@{
            base_url_env = "NORION_NEWAPI_BASE_URL"
            api_key_env = "NORION_NEWAPI_API_KEY"
            api_key_written = $false
            model_list_endpoint = "/v1/models"
            probe_endpoint = "/v1/chat/completions"
        }
        policy = [pscustomobject]@{
            excluded_family = "gpt-5-or-higher"
            note = "Models matching GPT major version 5 or newer are excluded before probing."
        }
        probe = [pscustomobject]@{
            requested = $RunProbe
            max_probe_models = $MaxProbeCount
            timeout_secs = $TimeoutSecs
            max_tokens = $MaxTokens
        }
        counts = [pscustomobject]@{
            discovered = $records.Count
            candidates = $candidateRecords.Count
            failed_or_excluded = $failedRecords.Count
        }
        tiers = $tiers
        models = $records
        failed_models = @($failedRecords | ForEach-Object {
            [pscustomobject]@{
                model = $_.model
                reasons = $_.failure_reasons
                probe_error = $_.probe.error
            }
        })
    }
}

function Write-DiscoveryMarkdown {
    param(
        $Report,
        [string]$Path
    )

    $lines = @()
    $lines += "# NewAPI Model Discovery Matrix"
    $lines += ""
    $lines += "- Schema: $($Report.schema)"
    $lines += "- Generated UTC: $($Report.generated_at_utc)"
    $lines += "- API key written: $($Report.source.api_key_written)"
    $lines += "- GPT-5/GPT-5-or-newer policy: excluded before probing"
    $lines += "- Probe requested: $($Report.probe.requested)"
    $lines += ""
    $lines += "## Candidate Tiers"
    $lines += ""
    $lines += "| Tier | Models |"
    $lines += "| --- | --- |"
    foreach ($tier in @("fast_router_reviewer", "heavy_reasoning", "coding", "fallback")) {
        $models = @($Report.tiers.$tier)
        $value = if ($models.Count -gt 0) { ($models -join ", ") } else { "(none)" }
        $lines += "| $tier | $value |"
    }
    $lines += ""
    $lines += "## Failed Or Excluded Models"
    $lines += ""
    $lines += "| Model | Reasons | Probe Error |"
    $lines += "| --- | --- | --- |"
    foreach ($failed in @($Report.failed_models)) {
        $reasonText = @($failed.reasons) -join ", "
        $errorText = if ([string]::IsNullOrWhiteSpace([string]$failed.probe_error)) { "" } else { ([string]$failed.probe_error).Replace("|", "/") }
        $lines += "| $($failed.model) | $reasonText | $errorText |"
    }
    if (@($Report.failed_models).Count -eq 0) {
        $lines += "| (none) |  |  |"
    }
    $lines += ""
    $lines += "## Manual Evidence Notes"
    $lines += ""
    foreach ($model in @($Report.models | Where-Object { @($_.evidence_notes).Count -gt 0 })) {
        foreach ($note in @($model.evidence_notes)) {
            $lines += "- $($model.model): $note"
        }
    }

    $parent = Split-Path -Parent $Path
    if ($parent -and $parent.Trim().Length -gt 0) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    Set-Content -LiteralPath $Path -Encoding ASCII -Value ($lines -join "`n")
}

function Write-DiscoveryJson {
    param(
        $Report,
        [string]$Path
    )

    $parent = Split-Path -Parent $Path
    if ($parent -and $parent.Trim().Length -gt 0) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    Set-Content -LiteralPath $Path -Encoding ASCII -Value ($Report | ConvertTo-Json -Depth 12)
}

if ($SelfTest) {
    $fixtureModels = @(
        "gpt-4.1-mini",
        "qwen2.5-coder-32b-instruct",
        "deepseek-r1",
        "claude-3-5-haiku",
        "moonshot-v1-8k",
        "qwen/qwen3.5-397b-a17b",
        "qwen/qwen3-next-80b-a3b-instruct",
        "moonshotai/kimi-k2.6",
        "codestral-latest",
        "gpt-5",
        "openai/gpt-5.3",
        "gpt-6-preview"
    )
    $report = New-DiscoveryReport -ModelIds $fixtureModels -RunProbe:$false -MaxProbeCount 24 -TimeoutSecs 1 -MaxTokens 8 -Prompt "selftest"

    foreach ($forbidden in @("gpt-5", "openai/gpt-5.3", "gpt-6-preview")) {
        $match = @($report.failed_models | Where-Object { $_.model -eq $forbidden -and $_.reasons -contains "policy_excluded_gpt5_or_higher" })
        if ($match.Count -ne 1) {
            throw "selftest failed: forbidden model was not excluded: $forbidden"
        }
    }
    foreach ($tierCheck in @(
        @{ tier = "fast_router_reviewer"; model = "gpt-4.1-mini" },
        @{ tier = "heavy_reasoning"; model = "deepseek-r1" },
        @{ tier = "coding"; model = "qwen2.5-coder-32b-instruct" },
        @{ tier = "fallback"; model = "moonshot-v1-8k" },
        @{ tier = "heavy_reasoning"; model = "qwen/qwen3.5-397b-a17b" },
        @{ tier = "coding"; model = "qwen/qwen3.5-397b-a17b" },
        @{ tier = "heavy_reasoning"; model = "qwen/qwen3-next-80b-a3b-instruct" }
    )) {
        if (@($report.tiers.($tierCheck.tier)) -notcontains $tierCheck.model) {
            throw "selftest failed: $($tierCheck.model) missing from $($tierCheck.tier)"
        }
    }
    if (@($report.tiers.fast_router_reviewer) -contains "qwen/qwen3.5-397b-a17b") {
        throw "selftest failed: qwen/qwen3.5-397b-a17b must not be a fast router default"
    }
    $kimiFailure = @($report.failed_models | Where-Object {
        $_.model -eq "moonshotai/kimi-k2.6" -and $_.reasons -contains "manual_evidence_unstable_repetitive_output"
    })
    if ($kimiFailure.Count -ne 1) {
        throw "selftest failed: moonshotai/kimi-k2.6 must be marked unstable"
    }

    $jsonPath = Resolve-ArtifactPath -Path $OutputJson
    $markdownPath = Resolve-ArtifactPath -Path $MatrixMarkdown
    Write-DiscoveryJson -Report $report -Path $jsonPath
    Write-DiscoveryMarkdown -Report $report -Path $markdownPath
    Write-Host "newapi_model_discovery_selftest=PASS"
    Write-Host "output_json=$OutputJson"
    Write-Host "matrix_markdown=$MatrixMarkdown"
    Write-Host "touches_network=false"
    Write-Host "sends_prompt=false"
    Write-Host "api_key_written=false"
    exit 0
}

$modelIds = Invoke-ModelList
$report = New-DiscoveryReport -ModelIds $modelIds -RunProbe:$Probe.IsPresent -MaxProbeCount $MaxProbeModels -TimeoutSecs $ProbeTimeoutSecs -MaxTokens $ProbeMaxTokens -Prompt $ProbePrompt

$jsonOutputPath = Resolve-ArtifactPath -Path $OutputJson
$markdownOutputPath = Resolve-ArtifactPath -Path $MatrixMarkdown
Write-DiscoveryJson -Report $report -Path $jsonOutputPath
Write-DiscoveryMarkdown -Report $report -Path $markdownOutputPath

Write-Host "newapi_model_discovery=PASS"
Write-Host "base_url_env=NORION_NEWAPI_BASE_URL set=$(-not [string]::IsNullOrWhiteSpace([string]$env:NORION_NEWAPI_BASE_URL))"
Write-Host "api_key_env=NORION_NEWAPI_API_KEY set=$(-not [string]::IsNullOrWhiteSpace([string]$env:NORION_NEWAPI_API_KEY))"
Write-Host "api_key_written=false"
Write-Host "discovered=$($report.counts.discovered)"
Write-Host "candidates=$($report.counts.candidates)"
Write-Host "failed_or_excluded=$($report.counts.failed_or_excluded)"
Write-Host "output_json=$OutputJson"
Write-Host "matrix_markdown=$MatrixMarkdown"
