param(
    [string]$RepoRoot = "D:\rust-norion",
    [string]$RemoteModel = "/Users/xinghuan/smartsteam-model-box/models/Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf",
    [string]$RemoteSmallModel = "/Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate SmartSteam remote model-pool guard rules without SSH, process launch, or prompt sending."
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  .\tools\smartsteam-forge\test-remote-model-pool-guards.cmd"
    Write-Host ""
    Write-Host "Checks:"
    Write-Host "  - helper workers using the quality 12B model are rejected by default"
    Write-Host "  - helper workers whose model names look 12B+ are rejected by default"
    Write-Host "  - small helper models pass CheckOnly"
    Write-Host "  - -AllowLargePoolWorkerModels explicitly disables the guard for stress tests"
    return
}

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot not found: $RepoRoot"
}

$startScript = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\start-remote-gemma-forge.ps1"
if (-not (Test-Path -LiteralPath $startScript -PathType Leaf)) {
    throw "start-remote-gemma-forge.ps1 not found: $startScript"
}
$chainScript = Join-Path $RepoRoot "tools\smartsteam-forge\scripts\start-remote-gemma-chain.ps1"
if (-not (Test-Path -LiteralPath $chainScript -PathType Leaf)) {
    throw "start-remote-gemma-chain.ps1 not found: $chainScript"
}

function Invoke-GuardCase {
    param(
        [string]$Name,
        [string[]]$ArgumentList,
        [bool]$ShouldPass,
        [string]$ExpectedPattern = "",
        [string[]]$ExpectedOutputPatterns = @()
    )

    Write-Host ""
    Write-Host "guard_case=$Name"
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $startScript @ArgumentList 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }

    if ($ShouldPass) {
        if ($exitCode -ne 0) {
            throw "guard case '$Name' expected success, got exit code $exitCode"
        }
        if ($text -notmatch "SmartSteam remote Gemma Forge preflight: PASS") {
            throw "guard case '$Name' did not report CheckOnly PASS"
        }
        foreach ($pattern in $ExpectedOutputPatterns) {
            if ($text -notmatch $pattern) {
                throw "guard case '$Name' did not contain expected output pattern '$pattern'"
            }
        }
    } else {
        if ($exitCode -eq 0) {
            throw "guard case '$Name' expected failure, got success"
        }
        if (-not [string]::IsNullOrWhiteSpace($ExpectedPattern) -and $text -notmatch $ExpectedPattern) {
            throw "guard case '$Name' did not contain expected pattern '$ExpectedPattern'"
        }
    }
    Write-Host "guard_case_result=$Name PASS"
}

function Invoke-ChainGuardCase {
    param(
        [string]$Name,
        [string[]]$ArgumentList,
        [bool]$ShouldPass = $true,
        [string]$ExpectedPattern = "",
        [string[]]$ExpectedOutputPatterns = @()
    )

    Write-Host ""
    Write-Host "guard_case=$Name"
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $chainScript @ArgumentList 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    $text = ($output | ForEach-Object { $_.ToString() }) -join "`n"
    if (-not [string]::IsNullOrWhiteSpace($text)) {
        Write-Host $text.TrimEnd()
    }
    if ($ShouldPass) {
        if ($exitCode -ne 0) {
            throw "guard case '$Name' expected success, got exit code $exitCode"
        }
        if ($text -notmatch "SmartSteam remote Gemma chain preflight: PASS") {
            throw "guard case '$Name' did not report chain CheckOnly PASS"
        }
        foreach ($pattern in $ExpectedOutputPatterns) {
            if ($text -notmatch $pattern) {
                throw "guard case '$Name' did not contain expected output pattern '$pattern'"
            }
        }
    } else {
        if ($exitCode -eq 0) {
            throw "guard case '$Name' expected failure, got success"
        }
        if (-not [string]::IsNullOrWhiteSpace($ExpectedPattern) -and $text -notmatch $ExpectedPattern) {
            throw "guard case '$Name' did not contain expected pattern '$ExpectedPattern'"
        }
    }
    Write-Host "guard_case_result=$Name PASS"
}

$commonArgs = @(
    "-RepoRoot", $RepoRoot,
    "-RemoteModel", $RemoteModel,
    "-CheckOnly",
    "-NoForge",
    "-EnablePoolWorkers"
)

$generatedManifestPath = Join-Path $RepoRoot "target\gemma-chain\apple-model-pool.generated.json"

function New-GuardManifest {
    param(
        [string]$Path,
        [bool]$IncludeAdvice,
        [int]$ShapeQuality = 1,
        [string]$CpuHelperRole = ""
    )

    $workers = @(
        [pscustomobject]@{ role = "quality"; port = 8686; base_url = "http://127.0.0.1:8686"; default_context_tokens = 262144; default_max_tokens = 262144; low_priority = $false; runtime_backend = "llama.cpp"; runtime_device = "metal"; runtime_accelerator = "metal"; gpu_layers = 999 },
        [pscustomobject]@{ role = "summary"; port = 8687; base_url = "http://127.0.0.1:8687"; default_context_tokens = 8192; default_max_tokens = 768; low_priority = $true; runtime_backend = "llama.cpp"; runtime_device = "metal"; runtime_accelerator = "metal"; gpu_layers = 999 },
        [pscustomobject]@{ role = "router"; port = 8689; base_url = "http://127.0.0.1:8689"; default_context_tokens = 4096; default_max_tokens = 512; low_priority = $true; runtime_backend = "llama.cpp"; runtime_device = "metal"; runtime_accelerator = "metal"; gpu_layers = 999 },
        [pscustomobject]@{ role = "review"; port = 8688; base_url = "http://127.0.0.1:8688"; default_context_tokens = 8192; default_max_tokens = 1536; low_priority = $true; runtime_backend = "llama.cpp"; runtime_device = "metal"; runtime_accelerator = "metal"; gpu_layers = 999 },
        [pscustomobject]@{ role = "index"; port = 8690; base_url = "http://127.0.0.1:8690"; default_context_tokens = 4096; default_max_tokens = 512; low_priority = $true; runtime_backend = "llama.cpp"; runtime_device = "metal"; runtime_accelerator = "metal"; gpu_layers = 999 },
        [pscustomobject]@{ role = "test-gate"; port = 8688; base_url = "http://127.0.0.1:8688"; default_context_tokens = 4096; default_max_tokens = 1536; low_priority = $true; runtime_backend = "llama.cpp"; runtime_device = "metal"; runtime_accelerator = "metal"; gpu_layers = 999 }
    )
    if (-not [string]::IsNullOrWhiteSpace($CpuHelperRole)) {
        foreach ($worker in $workers) {
            if ([string]$worker.role -eq $CpuHelperRole) {
                $worker.runtime_device = "cpu"
                $worker.runtime_accelerator = "accelerate"
                $worker.gpu_layers = 0
            }
        }
    }
    $manifest = [pscustomobject]@{
        schema_version = 1
        contract_version = "gemma-chain.v1"
        read_only = $true
        sends_prompt = $false
        launches_process = $false
        manifest_kind = "rust-norion.model-pool"
        workers = $workers
    }
    if ($IncludeAdvice) {
        $manifest | Add-Member -NotePropertyName advice -NotePropertyValue ([pscustomobject]@{
            decision_source = "model-pool-advice-core"
            policy = "one_quality_12b_plus_small_helpers"
            safe_to_enable_pool_workers = $true
            next_step = "add_summary_worker_first"
            reason = "quality_chain_ready_no_helpers_visible"
            kind = "busy"
            extra_quality_12b_detected = $false
            avoid_extra_12b = $true
            max_quality_12b_workers = 1
            quality_worker_count = 1
            helper_worker_count = 5
            helper_target_worker_count = 5
            helper_roles = @("summary", "router", "review", "index", "test-gate")
            worker_shape = [pscustomobject]@{
                quality = $ShapeQuality
                helpers_visible = 5
                helper_target = 5
            }
        }) -Force
    }
    $parent = Split-Path -Parent $Path
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    Set-Content -LiteralPath $Path -Encoding utf8 -Value ($manifest | ConvertTo-Json -Depth 10)
}

$guardTempDir = Join-Path $RepoRoot "target\smartsteam-forge-guard-tests"
$missingAdviceManifest = Join-Path $guardTempDir "missing-advice-model-pool.json"
$badWorkerShapeManifest = Join-Path $guardTempDir "bad-worker-shape-model-pool.json"
$cpuHelperManifest = Join-Path $guardTempDir "cpu-helper-model-pool.json"
$cpuIndexManifest = Join-Path $guardTempDir "cpu-index-model-pool.json"
$chainDefaultRepoRoot = Join-Path $guardTempDir "chain-default-repo"
$chainDefaultManifest = Join-Path $chainDefaultRepoRoot "target\gemma-chain\apple-model-pool.generated.json"
New-GuardManifest -Path $missingAdviceManifest -IncludeAdvice $false
New-GuardManifest -Path $badWorkerShapeManifest -IncludeAdvice $true -ShapeQuality 2
New-GuardManifest -Path $cpuHelperManifest -IncludeAdvice $true -CpuHelperRole "review"
New-GuardManifest -Path $cpuIndexManifest -IncludeAdvice $true -CpuHelperRole "index"
New-GuardManifest -Path $chainDefaultManifest -IncludeAdvice $true

Invoke-GuardCase `
    -Name "missing_manifest_advice_rejected" `
    -ArgumentList ($commonArgs + @("-ModelPoolManifest", $missingAdviceManifest, "-RemoteSmallModel", $RemoteSmallModel)) `
    -ShouldPass $false `
    -ExpectedPattern "requires advice"

Invoke-GuardCase `
    -Name "bad_worker_shape_rejected" `
    -ArgumentList ($commonArgs + @("-ModelPoolManifest", $badWorkerShapeManifest, "-RemoteSmallModel", $RemoteSmallModel)) `
    -ShouldPass $false `
    -ExpectedPattern "worker_shape\.quality"

Invoke-ChainGuardCase `
    -Name "direct_chain_cpu_helper_manifest_rejected" `
    -ArgumentList @(
        "-RepoRoot", $RepoRoot,
        "-RemoteModel", $RemoteModel,
        "-RemoteSmallModel", $RemoteSmallModel,
        "-EnablePoolWorkers",
        "-ModelPoolManifest", $cpuHelperManifest,
        "-CheckOnly"
    ) `
    -ShouldPass $false `
    -ExpectedPattern "worker role=review must be Metal accelerated"

Invoke-ChainGuardCase `
    -Name "direct_chain_cpu_index_manifest_rejected" `
    -ArgumentList @(
        "-RepoRoot", $RepoRoot,
        "-RemoteModel", $RemoteModel,
        "-RemoteSmallModel", $RemoteSmallModel,
        "-EnablePoolWorkers",
        "-ModelPoolManifest", $cpuIndexManifest,
        "-CheckOnly"
    ) `
    -ShouldPass $false `
    -ExpectedPattern "worker role=index must be Metal accelerated"

Invoke-GuardCase `
    -Name "same_quality_12b_rejected" `
    -ArgumentList ($commonArgs + @("-RemoteSmallModel", $RemoteModel)) `
    -ShouldPass $false `
    -ExpectedPattern "same model as quality|rejects helper model paths"

Invoke-GuardCase `
    -Name "large_helper_name_rejected" `
    -ArgumentList ($commonArgs + @("-RemoteSmallModel", "/Users/xinghuan/smartsteam-model-box/models/gemma-13b-helper-Q4.gguf")) `
    -ShouldPass $false `
    -ExpectedPattern "rejects helper model paths"

Invoke-GuardCase `
    -Name "small_helper_allowed" `
    -ArgumentList ($commonArgs + @("-RemoteSmallModel", $RemoteSmallModel)) `
    -ShouldPass $true `
    -ExpectedOutputPatterns @(
        "model_pool_advice_source=model-pool-advice-core",
        "model_pool_safe_to_enable_pool_workers=True",
        "model_pool_next_step=run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls",
        "model_pool_advice_reason=full_helper_pool_visible",
        "model_pool_extra_quality_12b_detected=False",
        "model_pool_worker_shape=quality:1 helpers_visible:5 helper_target:5"
    )

Invoke-GuardCase `
    -Name "mac32gb_preset_uses_role_models" `
    -ArgumentList ($commonArgs + @("-UseMac32GBModelPool")) `
    -ShouldPass $true `
    -ExpectedOutputPatterns @(
        "mac32gb_model_pool_preset=True",
        "context_tokens=65536",
        "default_max_tokens=4096",
        "helper_start_command=.*-UseMac32GBModelPool",
        "pool_worker_summary_model=/Users/xinghuan/smartsteam-model-box/models/gemma-3-270m-it-qat-Q4_0.gguf",
        "pool_worker_router_model=/Users/xinghuan/smartsteam-model-box/models/functiongemma-270m-it-Q4_K_M.gguf",
        "pool_worker_review_model=/Users/xinghuan/smartsteam-model-box/models/gemma-4-E4B-it-Q4_K_M.gguf",
        "pool_worker_index_model=/Users/xinghuan/smartsteam-model-box/models/gemma-4-E2B-it-Q4_K_M.gguf",
        "pool_worker_test-gate_model=/Users/xinghuan/smartsteam-model-box/models/gemma-4-E4B-it-Q4_K_M.gguf",
        "pool_worker_summary_gpu_layers=999",
        "pool_worker_review_gpu_layers=999",
        "pool_worker_index_gpu_layers=999",
        "pool_worker_test-gate_gpu_layers=999",
        "pool_worker_test-gate_default_max_tokens=1536",
        "pool_worker_review_device=",
        "pool_worker_index_device=",
        "pool_worker_test-gate_device="
    )

Invoke-GuardCase `
    -Name "explicit_large_helper_override_allowed" `
    -ArgumentList ($commonArgs + @("-RemoteSmallModel", $RemoteModel, "-AllowLargePoolWorkerModels")) `
    -ShouldPass $true `
    -ExpectedOutputPatterns @(
        "model_pool_advice_source=model-pool-advice-core",
        "model_pool_safe_to_enable_pool_workers=True",
        "model_pool_extra_quality_12b_detected=False"
    )

Invoke-ChainGuardCase `
    -Name "direct_chain_auto_uses_generated_manifest" `
    -ArgumentList @(
        "-RepoRoot", $chainDefaultRepoRoot,
        "-RemoteModel", $RemoteModel,
        "-RemoteSmallModel", $RemoteSmallModel,
        "-EnablePoolWorkers",
        "-CheckOnly"
    ) `
    -ExpectedOutputPatterns @(
        "auto_model_pool_manifest=.*apple-model-pool\.generated\.json",
        "model_pool_manifest=.*apple-model-pool\.generated\.json",
        "model_pool_manifest_runtime_metadata=present",
        "pool_worker_roles=summary,router,review,index,test-gate",
        "existing_worker_mismatch_policy=fail_without_restart_remote",
        "pool_worker_summary_launch_flags=ngl:999 device:default",
        "pool_worker_review_launch_flags=ngl:999 device:default",
        "pool_worker_test-gate_launch_flags=ngl:999 device:default",
        "touches_remote=false",
        "starts_process=false",
        "sends_prompt=false"
    )

$chainScriptText = Get-Content -LiteralPath $chainScript -Raw
foreach ($pattern in @(
    "launch flags mismatch",
    "rerun with -RestartRemote",
    "ensure_existing_worker_matches",
    "__EXPECTED_DEVICE_MODE__",
    "GetTempFileName",
    "ssh.exe -i",
    "sh -s",
    "`$blockingCpuOrNoGpuWorkers = @(`$cpuOrNoGpuWorkers)"
)) {
    if ($chainScriptText -notmatch [regex]::Escape($pattern)) {
        throw "start-remote-gemma-chain.ps1 is missing existing-worker launch flag guard pattern: $pattern"
    }
}
Write-Host "existing_worker_launch_flag_guard=present"

$restoreOutput = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $startScript `
    -RepoRoot $RepoRoot `
    -RemoteModel $RemoteModel `
    -UseMac32GBModelPool `
    -CheckOnly `
    -NoForge 2>&1
if ($LASTEXITCODE -ne 0) {
    $restoreText = ($restoreOutput | ForEach-Object { $_.ToString() }) -join "`n"
    throw "failed to restore Mac32GB generated manifest after guard tests: $restoreText"
}
Write-Host "model_pool_generated_manifest_restored=mac32gb context_tokens=65536"

Write-Host ""
Write-Host "remote_model_pool_guard_selftest=PASS"
Write-Host "touches_remote=false"
Write-Host "starts_process=false"
Write-Host "sends_prompt=false"
