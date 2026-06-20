param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path,
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

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$generatedAt = Get-Date
$generatedAtUtc = $generatedAt.ToUniversalTime()
$snapshotScript = Join-Path $PSScriptRoot "read-remote-unattended-snapshot.ps1"
$observationScript = Join-Path $PSScriptRoot "read-remote-observation-window.ps1"
$resourceScript = Join-Path $PSScriptRoot "read-remote-resource-window.ps1"
$consumerContractScript = Join-Path $PSScriptRoot "test-remote-consumer-contract.ps1"

$snapshot = Invoke-JsonScript -ScriptPath $snapshotScript -ScriptArgs @("-RepoRoot", $root)
$observation = Invoke-JsonScript -ScriptPath $observationScript -ScriptArgs @("-RepoRoot", $root)
$resource = Invoke-JsonScript -ScriptPath $resourceScript -ScriptArgs @("-RepoRoot", $root)
$consumerContract = Invoke-JsonScript -ScriptPath $consumerContractScript -ScriptArgs @("-RepoRoot", $root)

$readerContractsOk = (
    $snapshot.read_only -eq $true -and
    $snapshot.starts_process -eq $false -and
    $snapshot.sends_prompt -eq $false -and
    $snapshot.touches_remote -eq $false -and
    $snapshot.writes_files -eq $false -and
    $observation.read_only -eq $true -and
    $observation.starts_process -eq $false -and
    $observation.sends_prompt -eq $false -and
    $observation.touches_remote -eq $false -and
    $observation.writes_files -eq $false -and
    $resource.read_only -eq $true -and
    $resource.starts_process -eq $false -and
    $resource.sends_prompt -eq $false -and
    $resource.touches_remote -eq $false -and
    $resource.writes_files -eq $false -and
    $resource.writes_model_weights -eq $false -and
    $consumerContract.read_only -eq $true -and
    $consumerContract.starts_process -eq $false -and
    $consumerContract.sends_prompt -eq $false -and
    $consumerContract.touches_remote -eq $false -and
    $consumerContract.writes_files -eq $false
)

$authorizationFailClosed = (
    $snapshot.authorization.can_authorize_daemon -eq $false -and
    $snapshot.authorization.can_authorize_launch -eq $false -and
    $snapshot.authorization.can_authorize_prompt -eq $false -and
    $snapshot.authorization.can_authorize_ssh -eq $false -and
    $observation.authorization.can_authorize_daemon -eq $false -and
    $observation.authorization.can_authorize_launch -eq $false -and
    $observation.authorization.can_authorize_prompt -eq $false -and
    $observation.authorization.can_authorize_ssh -eq $false -and
    $resource.authorization.can_authorize_daemon -eq $false -and
    $resource.authorization.can_authorize_launch -eq $false -and
    $resource.authorization.can_authorize_prompt -eq $false -and
    $resource.authorization.can_authorize_ssh -eq $false -and
    $consumerContract.authorization.can_authorize_daemon -eq $false -and
    $consumerContract.authorization.can_authorize_launch -eq $false -and
    $consumerContract.authorization.can_authorize_prompt -eq $false -and
    $consumerContract.authorization.can_authorize_ssh -eq $false
)

$missingEvidence = @()
if ($snapshot.summary.evidence_fresh_all -ne $true) {
    $missingEvidence += "fresh_snapshot"
}
if ($observation.summary.continuous_window_present -ne $true) {
    $missingEvidence += "continuous_port_worker_window"
}
if ($resource.summary.resource_window_present -ne $true) {
    $missingEvidence += "remote_resource_headroom_window"
}
if ($consumerContract.summary.invalid_safe_command_count -ne 0 -or $consumerContract.summary.consumer_allowed_count -ne 0) {
    $missingEvidence += "consumer_contract_fail_closed_review"
}

$pendingExternalGates = @()
if ($snapshot.residency_decision.can_proceed_to_resident_loop -ne $true) {
    $pendingExternalGates += "residency_external_gate"
}

function Get-SafeCommandById {
    param([string]$Id)

    $command = @($snapshot.safe_next_read_only_commands | Where-Object { $_.id -eq $Id } | Select-Object -First 1)
    if ($command.Count -eq 0) {
        return $null
    }
    return $command[0]
}

function New-MissingEvidenceAction {
    param(
        [string]$Id,
        [string]$Status,
        [object[]]$ChecklistItems,
        [string]$Description,
        [string[]]$AdditionalSafeCommandIds = @()
    )

    $safeCommandIds = @(
        $ChecklistItems | ForEach-Object { $_.safe_command_id } | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }
        $AdditionalSafeCommandIds
    ) | Select-Object -Unique
    $safeCommands = @($safeCommandIds | ForEach-Object {
        $command = Get-SafeCommandById -Id $_
        if ($null -ne $command) {
            [pscustomobject]@{
                id = $command.id
                purpose = $command.purpose
                command = $command.command
                read_only = $command.read_only
                starts_process = $command.starts_process
                sends_prompt = $command.sends_prompt
                touches_remote = $command.touches_remote
                writes_files = $command.writes_files
            }
        }
    })

    return [pscustomobject]@{
        id = $Id
        status = $Status
        description = $Description
        checklist_ids = @($ChecklistItems | ForEach-Object { $_.id })
        gap_ids = @($ChecklistItems | ForEach-Object { $_.gap_id })
        safe_command_ids = @($safeCommandIds)
        safe_commands = $safeCommands
        blocks_authorization = $true
    }
}

$checklist = @($snapshot.evidence_checklist)
$missingEvidenceActions = @()
$pendingExternalGateActions = @()
if (@($missingEvidence | Where-Object { $_ -eq "fresh_snapshot" }).Count -gt 0) {
    $missingEvidenceActions += New-MissingEvidenceAction -Id "fresh_snapshot" -Status "missing_or_stale" -ChecklistItems @($checklist | Where-Object { $_.id -eq "fresh_status_snapshot" }) -Description "Historical snapshot evidence is stale or missing; re-read snapshot summary and collect fresh status evidence before any external gate."
}
if (@($pendingExternalGates | Where-Object { $_ -eq "residency_external_gate" }).Count -gt 0) {
    $pendingExternalGateActions += New-MissingEvidenceAction -Id "residency_external_gate" -Status "external_gate_required" -ChecklistItems @($checklist | Where-Object { $_.id -in @("daemon_status", "active_daemon_presence", "prompt_launch_gates") }) -AdditionalSafeCommandIds @("forge_daemon_watch_once", "forge_daemon_start_check", "gemma_chain_status", "gemma_pool_status") -Description "Resident loop still requires daemon status, duplicate-runner evidence, and prompt/launch gates from read-only checks before any user-authorized action."
}
if (@($missingEvidence | Where-Object { $_ -eq "continuous_port_worker_window" }).Count -gt 0) {
    $missingEvidenceActions += New-MissingEvidenceAction -Id "continuous_port_worker_window" -Status $observation.summary.status -ChecklistItems @($checklist | Where-Object { $_.id -eq "continuous_port_health" }) -Description "Continuous local observation-window artifact for model API/backend/Web Lab and worker health is missing or insufficient."
}
if (@($missingEvidence | Where-Object { $_ -eq "remote_resource_headroom_window" }).Count -gt 0) {
    $missingEvidenceActions += New-MissingEvidenceAction -Id "remote_resource_headroom_window" -Status $resource.summary.status -ChecklistItems @($checklist | Where-Object { $_.id -eq "remote_resource_headroom" }) -Description "Approved read-only resource/headroom artifact window for remote memory and Metal/GPU is missing or insufficient."
}
if (@($missingEvidence | Where-Object { $_ -eq "consumer_contract_fail_closed_review" }).Count -gt 0) {
    $missingEvidenceActions += [pscustomobject]@{
        id = "consumer_contract_fail_closed_review"
        status = "contract_review_required"
        description = "Consumer projection or safe command contract no longer matches fail-closed expectations; inspect consumer contract selftest before integration."
        checklist_ids = @()
        gap_ids = @()
        safe_command_ids = @("snapshot_summary")
        safe_commands = @(Get-SafeCommandById -Id "snapshot_summary")
        blocks_authorization = $true
    }
}

$canSupportExternalReview = (
    $readerContractsOk -and
    $authorizationFailClosed -and
    $snapshot.summary.evidence_fresh_all -eq $true -and
    $observation.summary.continuous_window_present -eq $true -and
    $resource.summary.resource_window_present -eq $true -and
    $consumerContract.summary.invalid_safe_command_count -eq 0 -and
    $consumerContract.summary.consumer_allowed_count -eq 0 -and
    $missingEvidence.Count -eq 0
)

$evidenceEntries = @(
    $snapshot.evidence.PSObject.Properties | ForEach-Object {
        [pscustomobject]@{
            id = $_.Name
            path = $_.Value.path
            exists = $_.Value.exists
            fresh = $_.Value.fresh
            age_seconds = $_.Value.age_seconds
            last_write_time_utc = $_.Value.last_write_time_utc
            parse_error = $_.Value.parse_error
        }
    }
)
$evidenceAgeValues = @($evidenceEntries | Where-Object { $null -ne $_.age_seconds } | ForEach-Object { [int]$_.age_seconds })
$maxEvidenceAgeSeconds = if ($evidenceAgeValues.Count -gt 0) { ($evidenceAgeValues | Measure-Object -Maximum).Maximum } else { $null }

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.readiness-contract.v1"
    generated_at = $generatedAt.ToString("yyyy-MM-dd HH:mm:ss zzz")
    generated_at_utc = $generatedAtUtc.ToString("o")
    read_only = $true
    starts_process = $false
    sends_prompt = $false
    touches_remote = $false
    writes_files = $false
    writes_model_weights = $false
    repo_root = $root
    summary = [pscustomobject]@{
        reader_contracts_ok = $readerContractsOk
        authorization_fail_closed = $authorizationFailClosed
        generated_at_utc = $generatedAtUtc.ToString("o")
        fresh_minutes = $snapshot.summary.fresh_minutes
        max_evidence_age_seconds = $maxEvidenceAgeSeconds
        snapshot_classification = $snapshot.residency_decision.classification
        evidence_fresh_all = $snapshot.summary.evidence_fresh_all
        observation_window_status = $observation.summary.status
        continuous_window_present = $observation.summary.continuous_window_present
        resource_window_status = $resource.summary.status
        resource_window_present = $resource.summary.resource_window_present
        consumer_contract_version = $consumerContract.summary.consumer_contract_version
        consumer_contract_validated = ($consumerContract.summary.invalid_safe_command_count -eq 0)
        consumer_allowed_count = $consumerContract.summary.consumer_allowed_count
        unsafe_safe_command_count = $consumerContract.summary.invalid_safe_command_count
        can_support_external_residency_review = $canSupportExternalReview
        missing_evidence = $missingEvidence
        missing_evidence_action_count = $missingEvidenceActions.Count
        pending_external_gates = $pendingExternalGates
        pending_external_gate_action_count = $pendingExternalGateActions.Count
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "readiness_contract_is_read_only_and_fail_closed"
    }
    consumer_projection = $snapshot.consumer_projection
    evidence_checklist = $snapshot.evidence_checklist
    missing_evidence_actions = $missingEvidenceActions
    pending_external_gate_actions = $pendingExternalGateActions
    safe_next_read_only_commands = $snapshot.safe_next_read_only_commands
    source_status = [pscustomobject]@{
        snapshot = [pscustomobject]@{
            contract_version = $snapshot.contract_version
            classification = $snapshot.residency_decision.classification
            evidence_fresh_all = $snapshot.summary.evidence_fresh_all
            fresh_minutes = $snapshot.summary.fresh_minutes
            max_evidence_age_seconds = $maxEvidenceAgeSeconds
            evidence = $evidenceEntries
        }
        observation_window = [pscustomobject]@{
            contract_version = $observation.contract_version
            status = $observation.summary.status
            sample_count = $observation.summary.sample_count
            continuous_window_present = $observation.summary.continuous_window_present
        }
        resource_window = [pscustomobject]@{
            contract_version = $resource.contract_version
            status = $resource.summary.status
            sample_count = $resource.summary.sample_count
            resource_window_present = $resource.summary.resource_window_present
        }
        consumer_contract = [pscustomobject]@{
            contract_version = $consumerContract.contract_version
            consumer_contract_version = $consumerContract.summary.consumer_contract_version
            consumer_count = $consumerContract.summary.consumer_count
            consumer_allowed_count = $consumerContract.summary.consumer_allowed_count
            invalid_safe_command_count = $consumerContract.summary.invalid_safe_command_count
        }
    }
    source_contracts = [pscustomobject]@{
        snapshot = $snapshot.contract_version
        observation_window = $observation.contract_version
        resource_window = $resource.contract_version
        consumer_contract_selftest = $consumerContract.contract_version
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 10
    exit 0
}

Write-Host "Gemma remote readiness contract"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "generated_at=$($result.generated_at) generated_at_utc=$($result.generated_at_utc) max_evidence_age_seconds=$($result.summary.max_evidence_age_seconds) fresh_minutes=$($result.summary.fresh_minutes)"
Write-Host "snapshot=$($result.summary.snapshot_classification) evidence_fresh_all=$($result.summary.evidence_fresh_all)"
Write-Host "observation_window=$($result.summary.observation_window_status) continuous_window_present=$($result.summary.continuous_window_present)"
Write-Host "resource_window=$($result.summary.resource_window_status) resource_window_present=$($result.summary.resource_window_present)"
Write-Host "consumer_contract_validated=$($result.summary.consumer_contract_validated) consumer_allowed_count=$($result.summary.consumer_allowed_count) unsafe_safe_command_count=$($result.summary.unsafe_safe_command_count)"
Write-Host "can_support_external_residency_review=$($result.summary.can_support_external_residency_review) missing_evidence=$($result.summary.missing_evidence -join ',') missing_evidence_actions=$($result.summary.missing_evidence_action_count) pending_external_gates=$($result.summary.pending_external_gates -join ',')"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
