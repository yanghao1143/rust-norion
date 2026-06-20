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

function Test-UnsafeCommandText {
    param([string]$Command)

    if ([string]::IsNullOrWhiteSpace($Command)) {
        return $true
    }

    return ($Command -match '\bsmoke\b|\bStart\b|\bssh\b|ssh\.exe|plink|Start-Process|forge_cli_prompt|web_lab_prompt|backend_cli_direct_prompt|evolution_loop_prompt_round|model_pool_launch')
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$generatedAt = Get-Date
$generatedAtUtc = $generatedAt.ToUniversalTime()
$readinessScript = Join-Path $PSScriptRoot "read-remote-readiness-contract.ps1"
$snapshotScript = Join-Path $PSScriptRoot "read-remote-unattended-snapshot.ps1"

$readiness = Invoke-JsonScript -ScriptPath $readinessScript -ScriptArgs @("-RepoRoot", $root)
$snapshot = Invoke-JsonScript -ScriptPath $snapshotScript -ScriptArgs @("-RepoRoot", $root)

$evidenceFiles = @($readiness.source_status.snapshot.evidence | ForEach-Object {
    [pscustomobject]@{
        id = $_.id
        path = $_.path
        exists = $_.exists
        fresh = $_.fresh
        age_seconds = $_.age_seconds
        last_write_time_utc = $_.last_write_time_utc
        parse_error = $_.parse_error
    }
})

$workerPorts = @($snapshot.model_pool.workers | ForEach-Object {
    [pscustomobject]@{
        role = $_.role
        port = $_.port
        status = $_.status
        ready = $_.ready
        accelerator = $_.runtime_accelerator
    }
})

$safeCommands = @($readiness.safe_next_read_only_commands | ForEach-Object {
    $unsafeText = Test-UnsafeCommandText -Command ([string]$_.command)
    [pscustomobject]@{
        id = $_.id
        purpose = $_.purpose
        command = $_.command
        read_only = $_.read_only
        starts_process = $_.starts_process
        sends_prompt = $_.sends_prompt
        touches_remote = $_.touches_remote
        writes_files = $_.writes_files
        unsafe_text_match = $unsafeText
    }
})

$unsafeSafeCommandCount = @($safeCommands | Where-Object {
    $_.read_only -ne $true -or
    $_.starts_process -ne $false -or
    $_.sends_prompt -ne $false -or
    $_.touches_remote -ne $false -or
    $_.writes_files -ne $false -or
    $_.unsafe_text_match -eq $true
}).Count

$checklist = @($readiness.evidence_checklist | ForEach-Object {
    $item = $_
    $safeCommand = @($safeCommands | Where-Object { $_.id -eq $item.safe_command_id } | Select-Object -First 1)
    [pscustomobject]@{
        id = $item.id
        gap_id = $item.gap_id
        status = $item.status
        required_evidence = $item.required_evidence
        proof_source = $item.proof_source
        safe_command_id = $item.safe_command_id
        safe_command_resolved = ($safeCommand.Count -eq 1)
        blocks_authorization = $item.blocks_authorization
    }
})

$unresolvedChecklistCount = @($checklist | Where-Object { $_.safe_command_resolved -ne $true }).Count

$consumerStatus = @($readiness.consumer_projection | ForEach-Object {
    [pscustomobject]@{
        id = $_.id
        surface = $_.surface
        entrypoint_kind = $_.entrypoint_kind
        current_allowed = $_.current_allowed
        safe_command_id = $_.safe_command_id
        blocked_by = @($_.blocked_by)
    }
})

$result = [pscustomobject]@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-unattended.residency-gap-report.v1"
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
        readiness = $readiness.contract_version
        snapshot = $snapshot.contract_version
    }
    decision = [pscustomobject]@{
        authorized = $false
        can_support_external_residency_review = $readiness.summary.can_support_external_residency_review
        snapshot_classification = $readiness.summary.snapshot_classification
        evidence_fresh_all = $readiness.summary.evidence_fresh_all
        missing_evidence = @($readiness.summary.missing_evidence)
        pending_external_gates = @($readiness.summary.pending_external_gates)
        reason = "gap_report_is_read_only_projection_and_cannot_authorize_daemon_launch_prompt_or_ssh"
    }
    authorization = [pscustomobject]@{
        can_authorize_daemon = $false
        can_authorize_launch = $false
        can_authorize_prompt = $false
        can_authorize_ssh = $false
        reason = "gap_report_is_read_only_and_fail_closed"
    }
    snapshot_claims = [pscustomobject]@{
        historical_only = $true
        model_cache = [pscustomobject]@{
            all_ok = $snapshot.model_cache.all_ok
            ok_count = $snapshot.model_cache.ok_count
            model_count = $snapshot.model_cache.model_count
            copy_needed_count = $snapshot.model_cache.copy_needed_count
            remote_error_count = $snapshot.model_cache.remote_error_count
        }
        chain = [pscustomobject]@{
            ready = $snapshot.chain.ready
            model_api = $snapshot.chain.model_api
            backend = $snapshot.chain.backend
            web_lab = $snapshot.chain.web_lab
            remote_probe_skipped = $snapshot.chain.remote_probe_skipped
        }
        model_pool = [pscustomobject]@{
            worker_count = $snapshot.model_pool.worker_count
            healthy_worker_count = $snapshot.model_pool.healthy_worker_count
            workers = $workerPorts
        }
        unattended = [pscustomobject]@{
            rounds = $snapshot.unattended.rounds
            success = $snapshot.unattended.success
            failures = $snapshot.unattended.failures
            latest_ledger_round = $snapshot.latest_ledger.round
            latest_ledger_success = $snapshot.latest_ledger.success
        }
    }
    freshness = [pscustomobject]@{
        fresh_minutes = $readiness.summary.fresh_minutes
        max_evidence_age_seconds = $readiness.summary.max_evidence_age_seconds
        evidence_files = $evidenceFiles
    }
    checklist = $checklist
    missing_evidence_actions = $readiness.missing_evidence_actions
    pending_external_gate_actions = $readiness.pending_external_gate_actions
    consumers = $consumerStatus
    safety = [pscustomobject]@{
        reader_contracts_ok = $readiness.summary.reader_contracts_ok
        authorization_fail_closed = $readiness.summary.authorization_fail_closed
        consumer_contract_validated = $readiness.summary.consumer_contract_validated
        consumer_allowed_count = $readiness.summary.consumer_allowed_count
        safe_command_count = $safeCommands.Count
        unsafe_safe_command_count = $unsafeSafeCommandCount
        unresolved_checklist_safe_command_count = $unresolvedChecklistCount
        safe_commands = $safeCommands
    }
}

if ($Json) {
    $result | ConvertTo-Json -Depth 12
    exit 0
}

Write-Host "Gemma remote residency gap report"
Write-Host "read_only=True starts_process=False sends_prompt=False touches_remote=False writes_files=False writes_model_weights=False"
Write-Host "generated_at=$($result.generated_at) generated_at_utc=$($result.generated_at_utc)"
Write-Host "snapshot=$($result.decision.snapshot_classification) evidence_fresh_all=$($result.decision.evidence_fresh_all) max_evidence_age_seconds=$($result.freshness.max_evidence_age_seconds)"
Write-Host "historical cache=$($result.snapshot_claims.model_cache.ok_count)/$($result.snapshot_claims.model_cache.model_count) workers=$($result.snapshot_claims.model_pool.healthy_worker_count)/$($result.snapshot_claims.model_pool.worker_count) unattended=$($result.snapshot_claims.unattended.success)/$($result.snapshot_claims.unattended.rounds)"
Write-Host "can_support_external_residency_review=$($result.decision.can_support_external_residency_review) missing_evidence=$($result.decision.missing_evidence -join ',') pending_external_gates=$($result.decision.pending_external_gates -join ',')"
Write-Host "safety: consumer_allowed_count=$($result.safety.consumer_allowed_count) unsafe_safe_command_count=$($result.safety.unsafe_safe_command_count) unresolved_checklist_safe_command_count=$($result.safety.unresolved_checklist_safe_command_count)"
Write-Host "authorization: daemon=False launch=False prompt=False ssh=False reason=$($result.authorization.reason)"
