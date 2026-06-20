param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
    [string]$ObservationWindowDir = "target\remote-gemma-observation-window",
    [string]$ResourceWindowDir = "target\remote-gemma-resource-window",
    [int]$MinSamples = 3,
    [int]$MinSpanMinutes = 10,
    [double]$MinAvailableMemoryGb = 8,
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Invoke-JsonScript {
    param(
        [string]$ScriptPath,
        [string[]]$ScriptArgs = @()
    )

    $jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath @ScriptArgs -Json
    if ($LASTEXITCODE -ne 0) {
        throw "$ScriptPath exited with $LASTEXITCODE"
    }
    return ($jsonText | ConvertFrom-Json)
}

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

function Get-SafeCommand {
    param(
        [object[]]$Commands,
        [string]$Id
    )

    $command = @($Commands | Where-Object { $_.id -eq $Id } | Select-Object -First 1)
    if ($command.Count -eq 0) {
        return $null
    }

    return [pscustomobject]@{
        id = $command[0].id
        purpose = $command[0].purpose
        command = $command[0].command
        read_only = $command[0].read_only
        starts_process = $command[0].starts_process
        sends_prompt = $command[0].sends_prompt
        touches_remote = $command[0].touches_remote
        writes_files = $command[0].writes_files
    }
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$generatedAt = Get-Date
$generatedAtUtc = $generatedAt.ToUniversalTime()
$gapReportScript = Join-Path $PSScriptRoot "read-remote-residency-gap-report.ps1"
$gapReport = Invoke-JsonScript -ScriptPath $gapReportScript -ScriptArgs @("-RepoRoot", $root)
$safeCommands = @($gapReport.safety.safe_commands)
$resolvedObservationDir = Resolve-InputPath -Root $root -Path $ObservationWindowDir
$resolvedResourceDir = Resolve-InputPath -Root $root -Path $ResourceWindowDir

$observationSourceCommands = @(
    Get-SafeCommand -Commands $safeCommands -Id "gemma_chain_status"
    Get-SafeCommand -Commands $safeCommands -Id "gemma_pool_status"
    Get-SafeCommand -Commands $safeCommands -Id "gemma_status_bundle"
    Get-SafeCommand -Commands $safeCommands -Id "forge_daemon_status"
) | Where-Object { $null -ne $_ }

$resourceSourceCommands = @(
    Get-SafeCommand -Commands $safeCommands -Id "remote_resource_artifact_check"
) | Where-Object { $null -ne $_ }

$snapshotSourceCommands = @(
    Get-SafeCommand -Commands $safeCommands -Id "snapshot_summary"
    Get-SafeCommand -Commands $safeCommands -Id "gemma_chain_status"
    Get-SafeCommand -Commands $safeCommands -Id "gemma_pool_status"
    Get-SafeCommand -Commands $safeCommands -Id "gemma_status_bundle"
    Get-SafeCommand -Commands $safeCommands -Id "forge_daemon_status"
) | Where-Object { $null -ne $_ }

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.evidence-package-plan.v1"
    generated_at = $generatedAt.ToString("yyyy-MM-dd HH:mm:ss zzz")
    generated_at_utc = $generatedAtUtc.ToString("o")
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    source_contracts = [pscustomobject]@{
        residency_gap_report = $gapReport.contract_version
    }
    current_state = [pscustomobject]@{
        snapshot_classification = $gapReport.decision.snapshot_classification
        evidence_fresh_all = $gapReport.decision.evidence_fresh_all
        can_support_external_residency_review = $gapReport.decision.can_support_external_residency_review
        missing_evidence = @($gapReport.decision.missing_evidence)
        pending_external_gates = @($gapReport.decision.pending_external_gates)
        authorization = $gapReport.authorization
    }
    plan = [pscustomobject]@{
        operator_boundary = [pscustomobject]@{
            this_script_collects_evidence = $false
            this_script_writes_artifacts = $false
            approved_owner_flow_required_to_write_artifacts = $true
            ssh_requires_explicit_user_authorization = $true
            prompt_launch_or_daemon_start_requires_separate_gate = $true
        }
        snapshot_refresh_package = [pscustomobject]@{
            purpose = "Produce fresh local evidence for chain/model-cache/unattended status before external residency review."
            required_outputs = @(
                "target\remote-gemma-chain\model-cache-status.json",
                "target\remote-gemma-chain\status-with-model-cache.json",
                "target\remote-gemma-unattended\evolution-report.json",
                "target\remote-gemma-unattended\evolution-ledger.jsonl"
            )
            source_command_ids = @($snapshotSourceCommands | ForEach-Object { $_.id })
            source_commands = $snapshotSourceCommands
            acceptance = [pscustomobject]@{
                evidence_fresh_all = $true
                parse_ok = $true
                authorization_still_false = $true
                historical_snapshot_still_not_authorization = $true
            }
        }
        observation_window_package = [pscustomobject]@{
            purpose = "Prove model API/backend/Web Lab and worker health over a continuous local artifact window."
            window_dir = ConvertTo-RelativePath -Root $root -Path $resolvedObservationDir
            sample_dir_pattern = "sample-001, sample-002, sample-003, ..."
            requirements = [pscustomobject]@{
                min_samples = $MinSamples
                min_span_minutes = $MinSpanMinutes
                min_healthy_workers = 6
            }
            required_files_per_sample = @(
                [pscustomobject]@{ file = "chain-status.json"; source_command_id = "gemma_chain_status"; required_fields = @("classification or prompt_ready/ready", "machine_summary.read_only=true", "machine_summary.sends_prompt=false", "machine_summary.launches_process=false") },
                [pscustomobject]@{ file = "pool-status.json"; source_command_id = "gemma_pool_status"; required_fields = @("capacity.worker_count", "capacity.healthy_worker_count", "capacity.expansion_allowed") },
                [pscustomobject]@{ file = "status-bundle.json"; source_command_id = "gemma_status_bundle"; required_fields = @("read_only=true", "sends_prompt=false", "launches_process=false") },
                [pscustomobject]@{ file = "forge-daemon-status.json"; source_command_id = "forge_daemon_status"; required_fields = @("read_only=true", "starts_process=false", "sends_prompt=false", "evolution_status.daemon.running", "report_gate_preflight.continuation_state") }
            )
            source_command_ids = @($observationSourceCommands | ForEach-Object { $_.id })
            source_commands = $observationSourceCommands
            verifier_command = ".\tools\gemma-chain\scripts\read-remote-observation-window.ps1 -Json"
            acceptance = [pscustomobject]@{
                summary_status = "window_observed_external_gate_required"
                continuous_window_present = $true
                authorization_still_false = $true
            }
        }
        resource_window_package = [pscustomobject]@{
            purpose = "Prove remote host memory and Metal/GPU headroom over a continuous local artifact window."
            window_dir = ConvertTo-RelativePath -Root $root -Path $resolvedResourceDir
            sample_dir_pattern = "sample-001, sample-002, sample-003, ..."
            requirements = [pscustomobject]@{
                min_samples = $MinSamples
                min_span_minutes = $MinSpanMinutes
                min_available_memory_gb = $MinAvailableMemoryGb
                approved_owner_flow_required = $true
            }
            accepted_file_names = @("remote-resource-status.json", "resource-status.json", "resource-headroom.json")
            required_fields_per_sample = @(
                "read_only=true",
                "starts_process=false",
                "sends_prompt=false",
                "writes_model_weights=false",
                "approved_owner_flow=true",
                "summary.memory_available_gb or memory.available_gb or headroom.memory_available_gb",
                "summary.metal_available or summary.gpu_available or metal.available or gpu.available"
            )
            source_command_ids = @($resourceSourceCommands | ForEach-Object { $_.id })
            source_commands = $resourceSourceCommands
            verifier_command = ".\tools\gemma-chain\scripts\read-remote-resource-window.ps1 -Json"
            acceptance = [pscustomobject]@{
                summary_status = "resource_window_observed_external_gate_required"
                resource_window_present = $true
                authorization_still_false = $true
            }
        }
    }
    safety = [pscustomobject]@{
        source_safe_command_count = $safeCommands.Count
        source_unsafe_safe_command_count = $gapReport.safety.unsafe_safe_command_count
        source_unresolved_checklist_safe_command_count = $gapReport.safety.unresolved_checklist_safe_command_count
        forbidden_without_explicit_user_authorization = @("ssh", "model start", "model stop", "prompt", "daemon start", "model weight writes")
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "evidence_package_plan_is_read_only_and_cannot_authorize_actions"
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 12
    exit 0
}

Write-Host "Gemma remote evidence package plan"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "generated_at=$($result.generated_at) generated_at_utc=$($result.generated_at_utc)"
Write-Host "current_state snapshot=$($result.current_state.snapshot_classification) evidence_fresh_all=$($result.current_state.evidence_fresh_all) missing_evidence=$($result.current_state.missing_evidence -join ',') pending_external_gates=$($result.current_state.pending_external_gates -join ',')"
Write-Host "observation_window dir=$($result.plan.observation_window_package.window_dir) min_samples=$($result.plan.observation_window_package.requirements.min_samples) min_span_minutes=$($result.plan.observation_window_package.requirements.min_span_minutes)"
Write-Host "resource_window dir=$($result.plan.resource_window_package.window_dir) min_samples=$($result.plan.resource_window_package.requirements.min_samples) min_span_minutes=$($result.plan.resource_window_package.requirements.min_span_minutes) min_memory_gb=$($result.plan.resource_window_package.requirements.min_available_memory_gb)"
Write-Host "safety: source_safe_command_count=$($result.safety.source_safe_command_count) source_unsafe_safe_command_count=$($result.safety.source_unsafe_safe_command_count) source_unresolved_checklist_safe_command_count=$($result.safety.source_unresolved_checklist_safe_command_count)"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
