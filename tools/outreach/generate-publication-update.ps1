param(
    [int]$SinceDays = 7,
    [int]$MaxCommits = 30,
    [string]$BaseRef = "",
    [string]$OutDir = "publication-update"
)

$ErrorActionPreference = "Stop"
$OutputEncoding = [System.Text.Encoding]::UTF8
try {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
} catch {
    # Non-interactive hosts can expose a read-only console.
}

function Add-Lines {
    param(
        [System.Collections.Generic.List[string]]$Lines,
        [string[]]$Values
    )
    foreach ($value in $Values) {
        $Lines.Add($value)
    }
}

function Select-Category {
    param([string]$Text)

    if ($Text -match '(?i)dna|genome|gene|splice|scissor|mutation|evolution|rejuvenation|reasoning') {
        return "dna_rgc"
    }
    if ($Text -match '(?i)runtime|inference|model|router|profile|backend|pool') {
        return "runtime"
    }
    if ($Text -match '(?i)test|bench|trace|gate|validation|rollback|evidence|safety|privacy') {
        return "evidence"
    }
    if ($Text -match '(?i)doc|paper|readme|outreach|release|citation|zenodo|osf|sciencedb|openi') {
        return "publication"
    }
    return "other"
}

$repoRoot = (git rev-parse --show-toplevel).Trim()
Set-Location $repoRoot

$generatedUtc = (Get-Date).ToUniversalTime().ToString("yyyy-MM-dd HH:mm:ss")
$sinceDate = (Get-Date).ToUniversalTime().AddDays(-1 * $SinceDays).ToString("yyyy-MM-dd")

if (-not [string]::IsNullOrWhiteSpace($BaseRef)) {
    $commitRange = "$BaseRef..HEAD"
    $commitLines = @(git log $commitRange --max-count=$MaxCommits --pretty=format:"%h`t%s" --no-merges 2>$null)
    $changedFiles = @(git diff --name-only $commitRange 2>$null | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique)
    $windowLabel = "changes since $BaseRef"
} else {
    $commitLines = @(git log --since=$sinceDate --max-count=$MaxCommits --pretty=format:"%h`t%s" --no-merges 2>$null)
    $changedFiles = @(git log --since=$sinceDate --name-only --pretty=format: 2>$null | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique)
    $windowLabel = "last $SinceDays days, since $sinceDate UTC"
}

if ($commitLines.Count -eq 0) {
    $commitLines = @("none`tNo non-merge commits found for this update window.")
}

$categoryMap = [ordered]@{
    dna_rgc = New-Object System.Collections.Generic.List[string]
    runtime = New-Object System.Collections.Generic.List[string]
    evidence = New-Object System.Collections.Generic.List[string]
    publication = New-Object System.Collections.Generic.List[string]
    other = New-Object System.Collections.Generic.List[string]
}

foreach ($line in $commitLines) {
    $parts = $line -split "`t", 2
    $hash = $parts[0]
    $subject = if ($parts.Count -gt 1) { $parts[1] } else { $line }
    $category = Select-Category -Text $subject
    $categoryMap[$category].Add("- $hash $subject")
}

foreach ($file in $changedFiles) {
    $category = Select-Category -Text $file
    if ($categoryMap[$category].Count -lt 10) {
        $categoryMap[$category].Add("- file: $file")
    }
}

$links = [ordered]@{
    GitHub = "https://github.com/yanghao1143/rust-norion"
    Release = "https://github.com/yanghao1143/rust-norion/releases/tag/rgc-v0.1.0"
    Zenodo = "https://doi.org/10.5281/zenodo.20901489"
    OSF = "https://osf.io/cybdm/"
    ScienceDB = "https://doi.org/10.57760/sciencedb.41287"
    CSTR = "https://cstr.cn/31253.11.sciencedb.41287"
    OpenI = "https://openi.pcl.ac.cn/asd8841315/rust-norion"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$datasetEn = New-Object System.Collections.Generic.List[string]
Add-Lines $datasetEn @(
    "# rust-norion Publication Update Packet",
    "",
    "Generated: $generatedUtc UTC",
    "Window: $windowLabel",
    "",
    "## Dataset Description",
    "",
    "This update records the current rust-norion publication and reproducibility materials for the Reasoning Genome Chain technical report. It summarizes recent repository changes, the files that changed, reproducibility entry points, and the DNA-inspired inference-control work that should be reflected in ScienceDB, OSF, release notes, and community posts. The packet is metadata-only: it must not include real API keys, private prompts, raw traces, model weights, or unredacted logs.",
    "",
    "## DNA / Reasoning Genome Chain Highlights",
    ""
)
if ($categoryMap.dna_rgc.Count -eq 0) {
    $datasetEn.Add("- No DNA/RGC-specific commits were found in this window; keep the prior archive description unchanged unless a reviewer needs a status refresh.")
} else {
    Add-Lines $datasetEn ([string[]]$categoryMap.dna_rgc)
}
Add-Lines $datasetEn @(
    "",
    "## Runtime And Control-Layer Changes",
    ""
)
if ($categoryMap.runtime.Count -eq 0) {
    $datasetEn.Add("- No runtime-control commits were found in this window.")
} else {
    Add-Lines $datasetEn ([string[]]$categoryMap.runtime)
}
Add-Lines $datasetEn @(
    "",
    "## Evidence, Validation, And Safety Changes",
    ""
)
if ($categoryMap.evidence.Count -eq 0) {
    $datasetEn.Add("- No validation or safety commits were found in this window.")
} else {
    Add-Lines $datasetEn ([string[]]$categoryMap.evidence)
}
Add-Lines $datasetEn @(
    "",
    "## Reproducibility Checklist",
    "",
    "- Review the changed files listed in `manifest.json` before updating public archives.",
    "- Run `cargo check -q --workspace` and the focused tests relevant to the touched modules before claiming new validation evidence.",
    "- Attach regenerated manuscript, ZIP package, benchmark logs, or trace summaries only after confirming they contain no secrets or private raw payloads.",
    "",
    "## Stable Links",
    "",
    "- GitHub: $($links.GitHub)",
    "- Release: $($links.Release)",
    "- Zenodo DOI: $($links.Zenodo)",
    "- OSF archive: $($links.OSF)",
    "- ScienceDB DOI: $($links.ScienceDB)",
    "- ScienceDB CSTR: $($links.CSTR)",
    "- OpenI project: $($links.OpenI)"
)

$datasetZh = New-Object System.Collections.Generic.List[string]
Add-Lines $datasetZh @(
    "# rust-norion 数据集描述更新",
    "",
    "生成时间：$generatedUtc UTC",
    "更新窗口：$windowLabel",
    "",
    "## 可直接用于数据集描述",
    "",
    "本次更新归档 rust-norion Reasoning Genome Chain 技术报告及复现材料的最新状态，覆盖近期代码变更、文件清单、复现入口和 DNA 启发推理控制层的新进展。数据集描述应同步写清四件事：版本变化、上传文件、可复现材料、DNA/RGC 最新成果。该更新包只记录公开元数据，不包含真实 API key、私有 prompt、原始 trace、模型权重或未脱敏日志。",
    "",
    "## DNA / 推理基因链亮点",
    ""
)
if ($categoryMap.dna_rgc.Count -eq 0) {
    $datasetZh.Add("- 本窗口没有识别到 DNA/RGC 专项提交；除非需要状态刷新，否则沿用上一版数据集核心描述。")
} else {
    Add-Lines $datasetZh ([string[]]$categoryMap.dna_rgc)
}
Add-Lines $datasetZh @(
    "",
    "## 运行时与控制层变化",
    ""
)
if ($categoryMap.runtime.Count -eq 0) {
    $datasetZh.Add("- 本窗口没有识别到运行时控制层专项提交。")
} else {
    Add-Lines $datasetZh ([string[]]$categoryMap.runtime)
}
Add-Lines $datasetZh @(
    "",
    "## 证据、验证与安全变化",
    ""
)
if ($categoryMap.evidence.Count -eq 0) {
    $datasetZh.Add("- 本窗口没有识别到验证或安全专项提交。")
} else {
    Add-Lines $datasetZh ([string[]]$categoryMap.evidence)
}
Add-Lines $datasetZh @(
    "",
    "## 发布链接",
    "",
    "- GitHub：$($links.GitHub)",
    "- Release：$($links.Release)",
    "- Zenodo DOI：$($links.Zenodo)",
    "- OSF：$($links.OSF)",
    "- ScienceDB DOI：$($links.ScienceDB)",
    "- ScienceDB CSTR：$($links.CSTR)",
    "- OpenI：$($links.OpenI)"
)

$platformZh = New-Object System.Collections.Generic.List[string]
Add-Lines $platformZh @(
    "# rust-norion 对外更新稿",
    "",
    "rust-norion 正在把 AI 推理外层做成可审计、可回滚、可复现的 Rust 控制层。Reasoning Genome Chain 把记忆、路由、反思、工具调用、证据门禁和自进化准入表示为可测试的推理基因，而不是把系统能力藏在 prompt 和临时脚本里。",
    "",
    "本次更新重点：",
    ""
)
foreach ($bucket in @("dna_rgc", "runtime", "evidence", "publication")) {
    foreach ($item in $categoryMap[$bucket] | Select-Object -First 3) {
        $platformZh.Add($item)
    }
}
Add-Lines $platformZh @(
    "",
    "贡献者方向：Rust 控制层、Agent memory、runtime adapter、trace/schema gate、benchmark、中文/英文文档和复现脚本。",
    "",
    "链接：$($links.GitHub)",
    "可引用版本：$($links.Zenodo)"
)

$platformEn = New-Object System.Collections.Generic.List[string]
Add-Lines $platformEn @(
    "# rust-norion External Update",
    "",
    "rust-norion is a Rust prototype for auditable AI inference control. The Reasoning Genome Chain represents memory, routing, reflection, tool dispatch, evidence gates, rollback, and self-evolution admission as testable strategy records instead of burying control behavior in prompts and ad hoc scripts.",
    "",
    "Recent update highlights:",
    ""
)
foreach ($bucket in @("dna_rgc", "runtime", "evidence", "publication")) {
    foreach ($item in $categoryMap[$bucket] | Select-Object -First 3) {
        $platformEn.Add($item)
    }
}
Add-Lines $platformEn @(
    "",
    "Open contributor lanes: Rust control layer, agent memory, runtime adapters, trace/schema gates, benchmarks, bilingual docs, and reproducibility scripts.",
    "",
    "Repository: $($links.GitHub)",
    "Citable release: $($links.Zenodo)"
)

Set-Content -Encoding UTF8 -LiteralPath (Join-Path $OutDir "dataset-description.md") -Value ($datasetEn -join [Environment]::NewLine)
Set-Content -Encoding UTF8 -LiteralPath (Join-Path $OutDir "dataset-description.zh.md") -Value ($datasetZh -join [Environment]::NewLine)
Set-Content -Encoding UTF8 -LiteralPath (Join-Path $OutDir "platform-update.zh.md") -Value ($platformZh -join [Environment]::NewLine)
Set-Content -Encoding UTF8 -LiteralPath (Join-Path $OutDir "platform-update.en.md") -Value ($platformEn -join [Environment]::NewLine)

$manifest = [pscustomobject]@{
    generated_utc = $generatedUtc
    window = $windowLabel
    since_days = $SinceDays
    base_ref = $BaseRef
    max_commits = $MaxCommits
    commit_count = $commitLines.Count
    changed_file_count = $changedFiles.Count
    changed_files_sample = @($changedFiles | Select-Object -First 300)
    categories = [ordered]@{
        dna_rgc = @($categoryMap.dna_rgc)
        runtime = @($categoryMap.runtime)
        evidence = @($categoryMap.evidence)
        publication = @($categoryMap.publication)
        other = @($categoryMap.other)
    }
    links = $links
}

$manifest | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 -LiteralPath (Join-Path $OutDir "manifest.json")

Write-Output "Generated publication update packet in $OutDir"
