param(
    [string]$RepoRoot = "D:\rust-norion",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Validate SmartSteam evolution-loop status command without backend calls or prompt sending."
    exit 0
}

$statusScript = Join-Path $RepoRoot "tools\evolution-loop\status-evolution-loop.ps1"
if (-not (Test-Path -LiteralPath $statusScript -PathType Leaf)) {
    throw "status-evolution-loop.ps1 not found: $statusScript"
}
$strictStatusCmd = Join-Path $RepoRoot "tools\evolution-loop\strict-status-evolution-loop.cmd"
if (-not (Test-Path -LiteralPath $strictStatusCmd -PathType Leaf)) {
    throw "strict-status-evolution-loop.cmd not found: $strictStatusCmd"
}
$strictSnapshotCmd = Join-Path $RepoRoot "tools\evolution-loop\snapshot-strict-status-evolution-loop.cmd"
if (-not (Test-Path -LiteralPath $strictSnapshotCmd -PathType Leaf)) {
    throw "snapshot-strict-status-evolution-loop.cmd not found: $strictSnapshotCmd"
}
$verifyStrictSnapshotScript = Join-Path $RepoRoot "tools\evolution-loop\verify-strict-status-snapshot.ps1"
if (-not (Test-Path -LiteralPath $verifyStrictSnapshotScript -PathType Leaf)) {
    throw "verify-strict-status-snapshot.ps1 not found: $verifyStrictSnapshotScript"
}
$publishStrictSummaryScript = Join-Path $RepoRoot "tools\evolution-loop\publish-strict-status-summary.ps1"
if (-not (Test-Path -LiteralPath $publishStrictSummaryScript -PathType Leaf)) {
    throw "publish-strict-status-summary.ps1 not found: $publishStrictSummaryScript"
}
$verifyStrictSummaryScript = Join-Path $RepoRoot "tools\evolution-loop\verify-strict-status-summary.ps1"
if (-not (Test-Path -LiteralPath $verifyStrictSummaryScript -PathType Leaf)) {
    throw "verify-strict-status-summary.ps1 not found: $verifyStrictSummaryScript"
}
$refreshStrictArtifactsScript = Join-Path $RepoRoot "tools\evolution-loop\refresh-strict-status-artifacts.ps1"
if (-not (Test-Path -LiteralPath $refreshStrictArtifactsScript -PathType Leaf)) {
    throw "refresh-strict-status-artifacts.ps1 not found: $refreshStrictArtifactsScript"
}
$daemonScript = Join-Path $RepoRoot "tools\evolution-loop\daemon-evolution-loop.ps1"
if (-not (Test-Path -LiteralPath $daemonScript -PathType Leaf)) {
    throw "daemon-evolution-loop.ps1 not found: $daemonScript"
}
$transitionConsumerFixture = Join-Path $RepoRoot "tools\evolution-loop\fixtures\daemon-round-transition-status-v1.consumer.example.json"
if (-not (Test-Path -LiteralPath $transitionConsumerFixture -PathType Leaf)) {
    throw "transition consumer fixture not found: $transitionConsumerFixture"
}
$nextRoundDecisionFixture = Join-Path $RepoRoot "tools\evolution-loop\fixtures\next-round-decision-evidence-v1.report.example.json"
if (-not (Test-Path -LiteralPath $nextRoundDecisionFixture -PathType Leaf)) {
    throw "next-round decision fixture not found: $nextRoundDecisionFixture"
}
$nextRoundDecisionReportFixture = Join-Path $RepoRoot "tools\evolution-loop\fixtures\next-round-decision-report-v1.report.example.json"
if (-not (Test-Path -LiteralPath $nextRoundDecisionReportFixture -PathType Leaf)) {
    throw "next-round decision report fixture not found: $nextRoundDecisionReportFixture"
}
$nextRoundDownstreamConsumersFixture = Join-Path $RepoRoot "tools\evolution-loop\fixtures\next-round-downstream-status-consumers-v1.report.example.json"
if (-not (Test-Path -LiteralPath $nextRoundDownstreamConsumersFixture -PathType Leaf)) {
    throw "next-round downstream consumers fixture not found: $nextRoundDownstreamConsumersFixture"
}

function Assert-DaemonRoundTransitionConsumerStatus {
    param(
        [object]$Status,
        [string]$Name,
        [string]$ExpectedKind,
        [bool]$ExpectedRoundInProgress
    )

    if ($null -eq $Status) {
        throw "$Name transition status missing"
    }
    if ($Status.schema -ne "daemon_round_transition_status_v1") {
        throw "$Name transition status schema is not daemon_round_transition_status_v1"
    }
    if ($Status.transition_kind -ne $ExpectedKind) {
        throw "$Name transition kind expected $ExpectedKind but got $($Status.transition_kind)"
    }
    if ($Status.read_only -ne $true -or $Status.starts_process -ne $false -or $Status.sends_prompt -ne $false) {
        throw "$Name transition status broke report-only contract"
    }
    if ($Status.round_in_progress -ne $ExpectedRoundInProgress) {
        throw "$Name transition round_in_progress expected $ExpectedRoundInProgress but got $($Status.round_in_progress)"
    }
}

function Assert-DaemonRoundTransitionConsumerFixture {
    param([string]$Path)

    $fixture = Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
    if ($fixture.schema -ne "daemon_round_transition_status_v1.consumer_fixture") {
        throw "transition consumer fixture schema mismatch"
    }
    if ($fixture.read_only -ne $true -or $fixture.starts_process -ne $false -or $fixture.sends_prompt -ne $false) {
        throw "transition consumer fixture broke report-only top-level contract"
    }
    if ($fixture.consumer_contract.status_json_path -ne "daemon.daemon_round_transition_status") {
        throw "transition consumer fixture status json path changed"
    }
    if ($fixture.consumer_contract.daemon_json_path -ne "daemon.daemon_round_transition_status") {
        throw "transition consumer fixture daemon json path changed"
    }
    if ($fixture.consumer_contract.log_prose_required -ne $false -or $fixture.consumer_contract.operator_summary_required -ne $false) {
        throw "transition consumer fixture requires prose scraping"
    }

    $seenKinds = @{}
    foreach ($example in @($fixture.examples)) {
        $status = $example.daemon.daemon_round_transition_status
        $expectedRoundInProgress = $status.transition_kind -eq "normal_in_progress"
        Assert-DaemonRoundTransitionConsumerStatus -Status $status -Name "fixture:$($example.name)" -ExpectedKind $status.transition_kind -ExpectedRoundInProgress $expectedRoundInProgress
        $seenKinds[$status.transition_kind] = $true
    }
    foreach ($requiredKind in @("normal_in_progress", "round_done_waiting_ledger_commit")) {
        if (-not $seenKinds.ContainsKey($requiredKind)) {
            throw "transition consumer fixture missing $requiredKind"
        }
    }
}

function Assert-NextRoundDecisionReportV1 {
    param(
        [object]$Report,
        [object]$Decision,
        [string]$Name
    )

    if ($null -eq $Report) {
        throw "$Name missing next_round_decision_report_v1"
    }
    if ($Report.schema -ne "next_round_decision_report_v1") {
        throw "$Name report schema is not next_round_decision_report_v1"
    }
    if ($Report.source_schema -ne "next_round_decision_evidence_v1") {
        throw "$Name report source schema is not next_round_decision_evidence_v1"
    }
    if ($Report.read_only -ne $true -or $Report.report_only -ne $true -or $Report.side_effects -ne $false) {
        throw "$Name report broke read-only/report-only side-effect contract"
    }
    if ($Report.starts_process -ne $false -or $Report.sends_prompt -ne $false) {
        throw "$Name report introduced process or prompt side effects"
    }
    if ($Report.changes_daemon_loop_behavior -ne $false -or $Report.changes_prompt_content -ne $false -or $Report.changes_report_gate_stop_semantics -ne $false) {
        throw "$Name report changed daemon loop, prompt, or report gate boundaries"
    }
    if ($Report.changes_runtime_calls -ne $false -or $Report.changes_model_pool_behavior -ne $false) {
        throw "$Name report changed runtime or model pool boundaries"
    }
    if ($null -eq $Decision) {
        throw "$Name missing source decision for report comparison"
    }
    foreach ($field in @("display_state", "safe_to_wait_current_round_active", "safe_to_continue_after_current_round", "operator_attention_blocked", "operator_attention_required", "may_display_unattended_continuation", "wait_for_current_round", "continue_after_current_round", "reason_code")) {
        if ($Report.$field -ne $Decision.$field) {
            throw "$Name report field $field diverged from next_round_decision"
        }
    }
    if ($Report.evidence.transition_kind -ne $Decision.evidence.transition_kind -or $Report.evidence.report_gate_passed -ne $Decision.evidence.report_gate_passed) {
        throw "$Name report evidence diverged from next_round_decision evidence"
    }
    if ($Report.next_round_decision.schema -ne $Decision.schema -or $Report.next_round_decision.display_state -ne $Decision.display_state) {
        throw "$Name report did not preserve nested next_round_decision evidence"
    }
}

function Get-ExpectedDownstreamDecisionStatus {
    param([object]$Report)

    switch ([string]$Report.display_state) {
        "safe-to-wait" { return "safe_to_wait_current_round_active" }
        "safe-to-continue-after-current-round" { return "safe_to_continue_after_current_round" }
        default { return "operator_attention_blocked" }
    }
}

function Assert-NextRoundDownstreamStatusConsumersV1 {
    param(
        [object]$Projection,
        [object]$Report,
        [string]$Name,
        [object]$DaemonRoundTransitionStatus = $null
    )

    if ($null -eq $Projection) {
        throw "$Name missing next_round_downstream_status_consumers_v1"
    }
    if ($Projection.schema -ne "next_round_downstream_status_consumers_v1") {
        throw "$Name downstream schema is not next_round_downstream_status_consumers_v1"
    }
    if ($Projection.source_schema -ne "next_round_decision_report_v1") {
        throw "$Name downstream source schema is not next_round_decision_report_v1"
    }
    if ($Projection.read_only -ne $true -or $Projection.report_only -ne $true -or $Projection.side_effects -ne $false) {
        throw "$Name downstream projection broke read-only/report-only side-effect contract"
    }
    if ($Projection.starts_process -ne $false -or $Projection.sends_prompt -ne $false) {
        throw "$Name downstream projection introduced process or prompt side effects"
    }
    if ($Projection.changes_daemon_loop_behavior -ne $false -or $Projection.changes_prompt_content -ne $false -or $Projection.changes_report_gate_stop_semantics -ne $false) {
        throw "$Name downstream projection changed daemon loop, prompt, or report gate boundaries"
    }
    if ($Projection.changes_runtime_calls -ne $false -or $Projection.changes_model_pool_behavior -ne $false) {
        throw "$Name downstream projection changed runtime or model pool boundaries"
    }
    if ($Projection.normalized_facts -ne $true) {
        throw "$Name downstream projection did not mark normalized facts"
    }
    foreach ($consumer in @("service_cli_display_status", "forge_operator_display", "agent_assignment_acceptance", "memory_self_improve_admission_visibility")) {
        if ($Projection.consumers.$consumer -ne $true) {
            throw "$Name downstream projection missing consumer fact $consumer"
        }
    }
    if ($null -eq $Report) {
        throw "$Name missing source next_round_decision_report_v1"
    }

    $downstream = $Projection.next_round_downstream
    if ($null -eq $downstream) {
        throw "$Name missing next_round_downstream facts"
    }
    $expectedSourceStatus = Get-ExpectedDownstreamDecisionStatus -Report $Report
    $expectedEffectiveStatus = if ($Report.operator_attention_required -eq $true) { "operator_attention_blocked" } else { $expectedSourceStatus }
    if ($downstream.source_decision_status -ne $expectedSourceStatus) {
        throw "$Name downstream source decision expected $expectedSourceStatus but got $($downstream.source_decision_status)"
    }
    if ($downstream.effective_decision_status -ne $expectedEffectiveStatus) {
        throw "$Name downstream effective decision expected $expectedEffectiveStatus but got $($downstream.effective_decision_status)"
    }

    $expectedService = "display_operator_attention"
    $expectedForge = "forge_operator_attention"
    $expectedAgent = "reject_until_operator_attention"
    $expectedMemory = "visible_operator_attention_blocked"
    if ($expectedEffectiveStatus -eq "safe_to_wait_current_round_active") {
        $expectedService = "display_safe_to_wait_current_round"
        $expectedForge = "forge_safe_to_wait"
        $expectedAgent = "defer_until_current_round_completes"
        $expectedMemory = "visible_waiting_current_round"
    } elseif ($expectedEffectiveStatus -eq "safe_to_continue_after_current_round") {
        $expectedService = "display_safe_to_continue"
        $expectedForge = "forge_safe_to_continue"
        $expectedAgent = "accept_next_round_assignment"
        $expectedMemory = "visible_admission_safe"
    }
    if ($downstream.service_cli_display_status -ne $expectedService -or $downstream.forge_operator_display_status -ne $expectedForge) {
        throw "$Name downstream display consumer facts diverged from effective decision"
    }
    if ($downstream.agent_assignment_acceptance -ne $expectedAgent -or $downstream.memory_self_improve_admission_visibility -ne $expectedMemory) {
        throw "$Name downstream agent/memory consumer facts diverged from effective decision"
    }
    if ($downstream.read_only -ne $Report.read_only -or $downstream.report_only -ne $Report.report_only) {
        throw "$Name downstream read/report flags diverged from source report"
    }
    if ($downstream.no_side_effects -ne $true -or $downstream.dispatch_work_allowed -ne $false -or $downstream.prompt_replay_allowed -ne $false -or $downstream.process_start_allowed -ne $false -or $downstream.memory_write_allowed -ne $false -or $downstream.ndkv_write_allowed -ne $false) {
        throw "$Name downstream projection allowed a side-effect capability"
    }
    if ($downstream.current_round_active -ne $Report.safe_to_wait_current_round_active) {
        throw "$Name downstream current_round_active diverged from safe_to_wait_current_round_active"
    }
    if ($downstream.live_status_display_state -ne $Report.display_state) {
        throw "$Name downstream display state diverged from source report"
    }
    $roundEvidence = $downstream.round_id_evidence
    if ($null -eq $roundEvidence) {
        throw "$Name downstream missing round-id evidence"
    }
    if ($roundEvidence.read_only -ne $true -or $roundEvidence.report_only -ne $true -or $roundEvidence.side_effects -ne $false -or $roundEvidence.starts_process -ne $false -or $roundEvidence.sends_prompt -ne $false) {
        throw "$Name downstream round-id evidence broke read-only/report-only side-effect contract"
    }
    if ($null -ne $DaemonRoundTransitionStatus) {
        if ($roundEvidence.source_schema -ne "daemon_round_transition_status_v1" -or $roundEvidence.source_path -ne "daemon.daemon_round_transition_status") {
            throw "$Name downstream round-id evidence did not cite daemon transition status"
        }
        if ($roundEvidence.has_round_id_evidence -ne $true) {
            throw "$Name downstream round-id evidence did not mark round identifiers present"
        }
        foreach ($field in @("active_round", "ledger_latest_round", "latest_done_round")) {
            if ($downstream.$field -ne $DaemonRoundTransitionStatus.$field) {
                throw "$Name downstream $field did not mirror daemon transition status"
            }
            if ($roundEvidence.$field -ne $DaemonRoundTransitionStatus.$field) {
                throw "$Name downstream round-id evidence $field did not mirror daemon transition status"
            }
        }
        foreach ($field in @("ledger_lag_rounds", "transition_kind", "latest_round_state", "round_in_progress")) {
            if ($roundEvidence.$field -ne $DaemonRoundTransitionStatus.$field) {
                throw "$Name downstream round-id evidence $field did not mirror daemon transition status"
            }
        }
    } else {
        if ($roundEvidence.has_round_id_evidence -ne $false -or $null -ne $roundEvidence.source_schema -or $null -ne $roundEvidence.source_path) {
            throw "$Name downstream round-id evidence should stay empty without daemon transition status"
        }
        foreach ($field in @("active_round", "ledger_latest_round", "latest_done_round")) {
            if ($null -ne $downstream.$field -or $null -ne $roundEvidence.$field) {
                throw "$Name downstream $field should stay null without daemon transition status"
            }
        }
    }
    if ($downstream.readiness_can_schedule_next_round -ne ($expectedEffectiveStatus -eq "safe_to_continue_after_current_round")) {
        throw "$Name downstream readiness_can_schedule_next_round mismatch"
    }
    if ($Projection.next_round_decision_report_v1.schema -ne $Report.schema -or $Projection.next_round_decision_report_v1.display_state -ne $Report.display_state) {
        throw "$Name downstream projection did not preserve nested source report"
    }
}

function Assert-NextRoundDecisionFixture {
    param(
        [string]$Path,
        [string]$TransitionFixturePath
    )

    $fixture = Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
    if ($fixture.schema -ne "next_round_decision_evidence_surface_v1.report_fixture") {
        throw "next-round decision fixture schema mismatch"
    }
    if ($fixture.report_only -ne $true -or $fixture.read_only -ne $true -or $fixture.side_effects -ne $false) {
        throw "next-round decision fixture broke report-only/read-only contract"
    }
    if ($fixture.starts_process -ne $false -or $fixture.sends_prompt -ne $false) {
        throw "next-round decision fixture introduced process or prompt side effects"
    }
    if ($fixture.changes_daemon_loop_behavior -ne $false -or $fixture.changes_prompt_content -ne $false -or $fixture.changes_report_gate_stop_semantics -ne $false) {
        throw "next-round decision fixture changed daemon loop, prompt, or report gate boundaries"
    }
    if ($fixture.changes_runtime_calls -ne $false -or $fixture.changes_model_pool_behavior -ne $false) {
        throw "next-round decision fixture changed runtime or model pool boundaries"
    }
    if ($fixture.consumes.transition_fixture_schema -ne "daemon_round_transition_status_v1.consumer_fixture") {
        throw "next-round decision fixture no longer consumes transition fixture contract"
    }
    if ($fixture.consumes.transition_status_path -ne "live_status_bundle.daemon.daemon_round_transition_status") {
        throw "next-round decision fixture transition status path changed"
    }
    if ($fixture.consumes.report_gate_path -ne "live_status_bundle.report_gate") {
        throw "next-round decision fixture report gate path changed"
    }
    if ($fixture.consumes.requires_log_prose -ne $false -or $fixture.consumes.requires_operator_summary -ne $false) {
        throw "next-round decision fixture requires prose scraping"
    }

    $transitionFixture = Get-Content -Raw -LiteralPath $TransitionFixturePath | ConvertFrom-Json
    $transitionExamples = @{}
    foreach ($example in @($transitionFixture.examples)) {
        $transitionExamples[$example.name] = $example.daemon.daemon_round_transition_status
    }

    $seenStates = @{}
    $reportGatePassedExamples = 0
    foreach ($example in @($fixture.examples)) {
        $status = $example.live_status_bundle.daemon.daemon_round_transition_status
        $decision = $example.next_round_decision
        $report = $example.next_round_decision_report_v1
        $sourceName = [string]$example.input_refs.transition_fixture_example
        if (-not $transitionExamples.ContainsKey($sourceName)) {
            throw "next-round decision example $($example.name) references missing transition fixture example $sourceName"
        }
        if ($status.schema -ne $transitionExamples[$sourceName].schema -or $status.transition_kind -ne $transitionExamples[$sourceName].transition_kind) {
            throw "next-round decision example $($example.name) drifted from transition fixture kind"
        }
        if ($decision.schema -ne "next_round_decision_evidence_v1" -or $decision.side_effects -ne $false) {
            throw "next-round decision example $($example.name) broke decision schema or side_effects=false"
        }
        if ($decision.read_only -ne $true -or $decision.report_only -ne $true -or $decision.starts_process -eq $true -or $decision.sends_prompt -eq $true) {
            throw "next-round decision example $($example.name) broke read-only/report-only process contract"
        }
        if ($decision.evidence.transition_kind -ne $status.transition_kind) {
            throw "next-round decision example $($example.name) evidence transition_kind mismatch"
        }
        if ($decision.evidence.report_gate_passed -ne $example.live_status_bundle.report_gate.passed) {
            throw "next-round decision example $($example.name) evidence report gate mismatch"
        }
        if ($null -ne $report) {
            Assert-NextRoundDecisionReportV1 -Report $report -Decision $decision -Name "fixture:$($example.name)"
        }
        if ($null -ne $example.live_status_bundle.next_round_decision_report_v1) {
            Assert-NextRoundDecisionReportV1 -Report $example.live_status_bundle.next_round_decision_report_v1 -Decision $decision -Name "fixture-live-bundle:$($example.name)"
        }
        if ($null -ne $example.live_status_bundle.next_round_decision) {
            if ($example.live_status_bundle.next_round_decision.schema -ne $decision.schema -or $example.live_status_bundle.next_round_decision.display_state -ne $decision.display_state) {
                throw "next-round decision example $($example.name) live bundle did not preserve next_round_decision"
            }
        }
        if ($example.live_status_bundle.report_gate.passed -eq $true) {
            $reportGatePassedExamples++
        }

        switch ($decision.display_state) {
            "safe-to-wait" {
                if ($status.transition_kind -ne "normal_in_progress" -or $status.round_in_progress -ne $true -or $example.live_status_bundle.report_gate.passed -ne $true) {
                    throw "safe-to-wait example $($example.name) lacks active busy normal_in_progress passed-gate evidence"
                }
                if ($decision.wait_for_current_round -ne $true -or $decision.operator_attention_required -ne $false) {
                    throw "safe-to-wait example $($example.name) has wrong operator decision flags"
                }
                if ($decision.safe_to_wait_current_round_active -ne $true -or $decision.safe_to_continue_after_current_round -ne $false -or $decision.operator_attention_blocked -ne $false) {
                    throw "safe-to-wait example $($example.name) has wrong pure decision booleans"
                }
            }
            "safe-to-continue-after-current-round" {
                $safeToContinueKinds = @("round_done_waiting_ledger_commit", "post_round_activity", "idle_completed")
                if (($safeToContinueKinds -notcontains $status.transition_kind) -or $status.round_in_progress -ne $false -or $example.live_status_bundle.report_gate.passed -ne $true) {
                    throw "safe-to-continue example $($example.name) lacks completed/post-round passed-gate evidence"
                }
                if ($decision.continue_after_current_round -ne $true -or $decision.operator_attention_required -ne $false) {
                    throw "safe-to-continue example $($example.name) has wrong operator decision flags"
                }
                if ($decision.safe_to_wait_current_round_active -ne $false -or $decision.safe_to_continue_after_current_round -ne $true -or $decision.operator_attention_blocked -ne $false) {
                    throw "safe-to-continue example $($example.name) has wrong pure decision booleans"
                }
            }
            "blocked-operator-attention" {
                if ($decision.operator_attention_required -ne $true -or $decision.may_display_unattended_continuation -ne $false) {
                    throw "blocked example $($example.name) is not blocked for operator attention"
                }
                if ($decision.safe_to_wait_current_round_active -ne $false -or $decision.safe_to_continue_after_current_round -ne $false -or $decision.operator_attention_blocked -ne $true) {
                    throw "blocked example $($example.name) has wrong pure decision booleans"
                }
            }
            default {
                throw "unknown next-round decision display state $($decision.display_state)"
            }
        }
        $seenStates[$decision.display_state] = $true
    }
    foreach ($requiredState in @("safe-to-wait", "safe-to-continue-after-current-round", "blocked-operator-attention")) {
        if (-not $seenStates.ContainsKey($requiredState)) {
            throw "next-round decision fixture missing $requiredState"
        }
    }
    if ($reportGatePassedExamples -lt 2) {
        throw "next-round decision fixture does not cover report gate passed safe states"
    }
}

function Assert-NextRoundDecisionReportFixture {
    param([string]$Path)

    $fixture = Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
    if ($fixture.schema -ne "next_round_decision_report_v1.report_fixture") {
        throw "next-round decision report fixture schema mismatch"
    }
    if ($fixture.report_only -ne $true -or $fixture.read_only -ne $true -or $fixture.side_effects -ne $false) {
        throw "next-round decision report fixture broke report-only/read-only contract"
    }
    if ($fixture.starts_process -ne $false -or $fixture.sends_prompt -ne $false) {
        throw "next-round decision report fixture introduced process or prompt side effects"
    }
    if ($fixture.consumes.decision_path -ne "next_round_decision" -or $fixture.emits.status_path -ne "next_round_decision_report_v1") {
        throw "next-round decision report fixture paths changed"
    }

    $seenStates = @{}
    foreach ($example in @($fixture.examples)) {
        $decision = $example.next_round_decision
        $report = $example.next_round_decision_report_v1
        Assert-NextRoundDecisionReportV1 -Report $report -Decision $decision -Name "report-fixture:$($example.name)"
        Assert-NextRoundDecisionReportV1 -Report $example.live_status_bundle.next_round_decision_report_v1 -Decision $decision -Name "report-fixture-live-bundle:$($example.name)"
        if ($example.live_status_bundle.next_round_decision.schema -ne $decision.schema -or $example.live_status_bundle.next_round_decision.display_state -ne $decision.display_state) {
            throw "next-round decision report fixture $($example.name) live bundle did not preserve next_round_decision"
        }
        $seenStates[$report.display_state] = $true
    }
    foreach ($requiredState in @("safe-to-wait", "safe-to-continue-after-current-round", "blocked-operator-attention")) {
        if (-not $seenStates.ContainsKey($requiredState)) {
            throw "next-round decision report fixture missing $requiredState"
        }
    }
}

function Assert-NextRoundDownstreamConsumersFixture {
    param([string]$Path)

    $fixture = Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
    if ($fixture.schema -ne "next_round_downstream_status_consumers_v1.report_fixture") {
        throw "next-round downstream consumers fixture schema mismatch"
    }
    if ($fixture.report_only -ne $true -or $fixture.read_only -ne $true -or $fixture.side_effects -ne $false) {
        throw "next-round downstream consumers fixture broke report-only/read-only contract"
    }
    if ($fixture.starts_process -ne $false -or $fixture.sends_prompt -ne $false) {
        throw "next-round downstream consumers fixture introduced process or prompt side effects"
    }
    if ($fixture.consumes.decision_report_path -ne "next_round_decision_report_v1" -or $fixture.emits.status_path -ne "next_round_downstream_status_consumers_v1") {
        throw "next-round downstream consumers fixture paths changed"
    }
    if ($fixture.consumes.daemon_round_transition_status_path -ne "daemon.daemon_round_transition_status") {
        throw "next-round downstream consumers fixture daemon transition path changed"
    }
    foreach ($consumer in @("service_cli_display_status", "forge_operator_display", "agent_assignment_acceptance", "memory_self_improve_admission_visibility")) {
        if (@($fixture.consumers) -notcontains $consumer) {
            throw "next-round downstream consumers fixture missing consumer $consumer"
        }
    }

    $seenStates = @{}
    foreach ($example in @($fixture.examples)) {
        $report = $example.next_round_decision_report_v1
        $projection = $example.next_round_downstream_status_consumers_v1
        $transition = if ($null -ne $example.daemon_round_transition_status_v1) { $example.daemon_round_transition_status_v1 } else { $null }
        if ($null -eq $transition) {
            throw "next-round downstream consumers fixture missing daemon transition status for $($example.name)"
        }
        Assert-NextRoundDownstreamStatusConsumersV1 -Projection $projection -Report $report -Name "downstream-fixture:$($example.name)" -DaemonRoundTransitionStatus $transition
        $seenStates[$report.display_state] = $true
    }
    foreach ($requiredState in @("safe-to-wait", "safe-to-continue-after-current-round", "blocked-operator-attention")) {
        if (-not $seenStates.ContainsKey($requiredState)) {
            throw "next-round downstream consumers fixture missing $requiredState"
        }
    }
}

Assert-DaemonRoundTransitionConsumerFixture -Path $transitionConsumerFixture
Assert-NextRoundDecisionFixture -Path $nextRoundDecisionFixture -TransitionFixturePath $transitionConsumerFixture
Assert-NextRoundDecisionReportFixture -Path $nextRoundDecisionReportFixture
Assert-NextRoundDownstreamConsumersFixture -Path $nextRoundDownstreamConsumersFixture

$testDir = Join-Path $RepoRoot "tools\evolution-loop\target\evolution\status-selftest"
New-Item -ItemType Directory -Force -Path $testDir | Out-Null
$ledger = Join-Path $testDir "ledger.jsonl"
Set-Content -Encoding ASCII -LiteralPath $ledger -Value @(
    '{"round":1,"case":"status-selftest-0001","success":true,"runtime_tokens":8,"elapsed_ms":100,"feedback_applied":2,"self_improve_passed":true}',
    '{"round":2,"case":"status-selftest-0002","success":true,"runtime_tokens":13,"elapsed_ms":200,"feedback_applied":3,"self_improve_passed":true}'
)
$remoteStatus = Join-Path $testDir "remote-chain-status.json"
Set-Content -Encoding ASCII -LiteralPath $remoteStatus -Value (@{
    schema_version = 1
    contract_version = "smartsteam.remote-gemma-chain.status.v1"
    read_only = $true
    sends_prompt = $false
    starts_process = $false
    touches_remote = $false
    readiness = @{
        ready = $true
        model_api = $true
        backend = $true
        web_lab = $true
        quality_worker = $true
    }
    model_pool = @{
        worker_count = 6
        healthy_worker_count = 6
    }
    model_cache = @{
        path = "target\remote-gemma-chain\model-cache-status.json"
        all_ok = $true
        ok_count = 5
        model_count = 5
        remote_error_count = 0
    }
    remote_runtime = @{
        probed = $true
        touches_remote = $true
        worker_count = 6
        cpu_or_no_gpu_count = 3
        workers = @(
            @{ role = "quality"; port = 8686; gpu_layers = "999"; device = "default"; cpu_or_no_gpu = $false; backend_metadata_may_differ = $false },
            @{ role = "summary"; port = 8687; gpu_layers = "0"; device = "none"; cpu_or_no_gpu = $true; backend_metadata_may_differ = $true },
            @{ role = "review"; port = 8688; gpu_layers = "0"; device = "none"; cpu_or_no_gpu = $true; backend_metadata_may_differ = $true },
            @{ role = "test-gate"; port = 8688; gpu_layers = "0"; device = "none"; cpu_or_no_gpu = $true; backend_metadata_may_differ = $true }
        )
        error = ""
    }
} | ConvertTo-Json -Depth 10)

$jsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus -StrictLedgerHygiene
if ($LASTEXITCODE -ne 0) {
    throw "status json command failed with exit code $LASTEXITCODE"
}
$status = ($jsonText | Out-String | ConvertFrom-Json)
if ($status.read_only -ne $true) {
    throw "status read_only contract failed"
}
if ($status.starts_process -ne $false) {
    throw "status starts_process contract failed"
}
if ($status.sends_prompt -ne $false) {
    throw "status sends_prompt contract failed"
}
if ($status.ledger.total_records -ne 2) {
    throw "expected 2 ledger records"
}
if ($status.ledger.feedback_applied_total -ne 5) {
    throw "expected feedback total 5"
}
if ($status.readiness.ready -ne $true) {
    throw "expected ready status"
}
if ($status.next_round_decision.schema -ne "next_round_decision_evidence_v1" -or $status.next_round_decision.read_only -ne $true -or $status.next_round_decision.report_only -ne $true -or $status.next_round_decision.side_effects -ne $false) {
    throw "status did not expose read-only next-round decision contract"
}
if ($status.next_round_decision.display_state -ne "blocked-operator-attention" -or $status.next_round_decision.operator_attention_blocked -ne $true) {
    throw "status without report-gate evidence should conservatively expose operator-attention-blocked"
}
Assert-NextRoundDecisionReportV1 -Report $status.next_round_decision_report_v1 -Decision $status.next_round_decision -Name "status-json"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $status.next_round_downstream_status_consumers_v1 -Report $status.next_round_decision_report_v1 -Name "status-json"
if ($status.live_status_bundle.schema -ne "live_status_bundle_v1" -or $status.live_status_bundle.read_only -ne $true -or $status.live_status_bundle.report_only -ne $true -or $status.live_status_bundle.side_effects -ne $false) {
    throw "status did not expose read-only live status bundle contract"
}
Assert-NextRoundDecisionReportV1 -Report $status.live_status_bundle.next_round_decision_report_v1 -Decision $status.next_round_decision -Name "status-json-live-bundle"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $status.live_status_bundle.next_round_downstream_status_consumers_v1 -Report $status.live_status_bundle.next_round_decision_report_v1 -Name "status-json-live-bundle"

$readyFailOnNotReadyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus -StrictLedgerHygiene -FailOnNotReady
if ($LASTEXITCODE -ne 0) {
    throw "ready status with FailOnNotReady should exit 0, got $LASTEXITCODE"
}
$readyFailOnNotReady = ($readyFailOnNotReadyText | Out-String | ConvertFrom-Json)
if ($readyFailOnNotReady.readiness.ready -ne $true) {
    throw "ready status with FailOnNotReady should stay ready"
}
if ($readyFailOnNotReady.read_only -ne $true -or $readyFailOnNotReady.starts_process -ne $false -or $readyFailOnNotReady.sends_prompt -ne $false) {
    throw "ready status with FailOnNotReady broke read-only contract"
}

$report = Join-Path $testDir "report.json"
Set-Content -Encoding ASCII -LiteralPath $report -Value (@{
    rounds = 1
    success = 1
    failures = 0
    success_rate = 100.0
    report_gate = @{
        passed = $false
        failures = @("stale fixture gate failure")
    }
    self_improve_proposal_acceptance_summary_v1 = @{
        schema = "self_improve_proposal_acceptance_summary_v1"
        evidence_backed_business_improvement_count = 0
        advisory_only_count = 2
        require_repair_count = 0
        accepted_without_business_evidence_count = 0
        prompt_guidance = @{
            should_convert_advisory_to_evidence_backed_business_improvement = $true
            should_repair_unvalidated_or_unaccepted_proposals = $false
            requires_checked_passed_validation_and_accepted_memory_admission = $true
        }
        action_plan = @{
            action_required = $true
            primary_action = "convert_advisory_to_evidence_backed_business_improvement"
            actions = @(
                "convert_advisory_to_evidence_backed_business_improvement",
                "require_checked_passed_validation_and_accepted_memory_admission"
            )
            requires_checked_passed_validation_and_accepted_memory_admission = $true
        }
        action_assignment = @{
            action_required = $true
            primary_action = "convert_advisory_to_evidence_backed_business_improvement"
            actions = @(
                "convert_advisory_to_evidence_backed_business_improvement",
                "require_checked_passed_validation_and_accepted_memory_admission"
            )
            target_count = 2
            requires_checked_passed_validation_and_accepted_memory_admission = $true
            targets = @(
                @{
                    proposal_id = "self-improve-r392-helper-contract"
                    missing_requirements = @(
                        "accepted_memory_admission",
                        "evidence_backed_business_improvement"
                    )
                }
            )
        }
    }
    self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1 = @{
        target_count = 2
        request_count = 2
        approval_request_item_count = 2
        approval_decision_item_count = 2
        review_packet_item_count = 2
        ready_review_packet_count = 2
        pending_approval_count = 2
        blocked_count = 0
        first_review_packet_item_id = "self-improve-r392-helper-contract"
        approval_review_packet_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_usefulness_report_v1 = @{
        target_count = 2
        projected_report_count = 2
        accepted_memory_admission_count = 0
        quarantined_candidate_count = 2
        review_packet_item_count = 2
        useful_reflection_item_count = 2
        pending_operator_approval_count = 2
        blocked_count = 0
        wasted_compute_guard_count = 2
        adapter_safe_count = 2
        first_reflection_item_id = "self-improve-r392-helper-contract"
        reflection_usefulness_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_dedupe_cluster_report_v1 = @{
        target_count = 2
        useful_reflection_item_count = 2
        reflection_cluster_count = 1
        duplicate_cluster_count = 1
        duplicate_reflection_item_count = 1
        wasted_compute_guard_count = 2
        pending_operator_approval_count = 2
        adapter_safe_count = 2
        first_cluster_id = "memory-reflection-dedupe:selfimprovehelpercontract:fnv1a64-0000000000001111"
        reflection_dedupe_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_reuse_plan_report_v1 = @{
        target_count = 2
        reflection_cluster_count = 1
        plan_item_count = 1
        ready_reuse_plan_count = 1
        duplicate_cluster_count = 1
        duplicate_reflection_item_count = 1
        projected_saved_reflection_count = 1
        first_plan_item_id = "memory-reflection-dedupe:selfimprovehelpercontract:fnv1a64-0000000000001111"
        reflection_reuse_plan_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_reuse_preflight_report_v1 = @{
        target_count = 2
        plan_item_count = 1
        ready_reuse_plan_count = 1
        preflight_item_count = 1
        preflight_passed_item_count = 1
        blocked_item_count = 0
        duplicate_cluster_count = 1
        duplicate_reflection_item_count = 1
        projected_saved_reflection_count = 1
        projected_model_call_skip_count = 1
        first_preflight_item_id = "memory-reflection-dedupe:selfimprovehelpercontract:fnv1a64-0000000000001111"
        reuse_preflight_passed = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        model_call_skip_authorized = $false
        reflection_reuse_execution_authorized = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_reuse_lookup_preview_report_v1 = @{
        read_only = $true
        report_only = $true
        candidate_only = $true
        target_count = 2
        preflight_item_count = 1
        lookup_preview_item_count = 1
        ready_lookup_preview_count = 1
        blocked_item_count = 0
        duplicate_cluster_count = 1
        duplicate_reflection_item_count = 1
        projected_saved_reflection_count = 1
        projected_model_call_skip_count = 1
        first_lookup_key = "memory-reflection-reuse-lookup:memoryreflectiondedupe:fnv1a64-0000000000002222"
        lookup_preview_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        model_call_skip_authorized = $false
        reflection_reuse_execution_authorized = $false
        memory_lookup_performed = $false
        lookup_hit_assumed = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_reuse_lookup_approval_request_report_v1 = @{
        read_only = $true
        report_only = $true
        candidate_only = $true
        target_count = 2
        preflight_item_count = 1
        lookup_preview_item_count = 1
        ready_lookup_preview_count = 1
        approval_request_item_count = 1
        ready_approval_request_count = 1
        requested_lookup_approval_count = 1
        blocked_item_count = 0
        approval_token_present_count = 1
        rejection_token_present_count = 1
        duplicate_cluster_count = 1
        duplicate_reflection_item_count = 1
        projected_saved_reflection_count = 1
        projected_model_call_skip_count = 1
        first_approval_request_id = "memory-reflection-reuse-lookup-approval:memoryreflectiondedupe:fnv1a64-0000000000003333"
        lookup_approval_request_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        model_call_skip_authorized = $false
        reflection_reuse_execution_authorized = $false
        memory_lookup_performed = $false
        lookup_hit_assumed = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_report_v1 = @{
        read_only = $true
        report_only = $true
        candidate_only = $true
        target_count = 2
        preflight_item_count = 1
        lookup_preview_item_count = 1
        ready_lookup_preview_count = 1
        approval_request_item_count = 1
        ready_approval_request_count = 1
        approval_decision_preview_item_count = 1
        ready_approval_decision_preview_count = 1
        approved_lookup_execution_count = 0
        pending_approval_count = 1
        blocked_item_count = 0
        approval_token_present_count = 1
        rejection_token_present_count = 1
        duplicate_cluster_count = 1
        duplicate_reflection_item_count = 1
        projected_saved_reflection_count = 1
        projected_model_call_skip_count = 1
        first_approval_decision_preview_id = "memory-reflection-reuse-lookup-approval-decision-preview:memoryreflectiondedupe:fnv1a64-0000000000004444"
        lookup_approval_decision_preview_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        model_call_skip_authorized = $false
        reflection_reuse_execution_authorized = $false
        memory_lookup_performed = $false
        lookup_hit_assumed = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_report_v1 = @{
        read_only = $true
        report_only = $true
        candidate_only = $true
        target_count = 2
        preflight_item_count = 1
        lookup_preview_item_count = 1
        ready_lookup_preview_count = 1
        approval_request_item_count = 1
        ready_approval_request_count = 1
        approval_decision_preview_item_count = 1
        ready_approval_decision_preview_count = 1
        token_intake_preview_item_count = 1
        ready_token_intake_preview_count = 1
        pending_operator_token_count = 1
        blocked_item_count = 0
        approval_token_present_count = 1
        rejection_token_present_count = 1
        duplicate_cluster_count = 1
        duplicate_reflection_item_count = 1
        projected_saved_reflection_count = 1
        projected_model_call_skip_count = 1
        first_token_intake_preview_id = "memory-reflection-reuse-lookup-approval-token-intake-preview:memoryreflectiondedupe:fnv1a64-0000000000005555"
        lookup_approval_token_intake_preview_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        model_call_skip_authorized = $false
        reflection_reuse_execution_authorized = $false
        memory_lookup_performed = $false
        lookup_hit_assumed = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_report_v1 = @{
        read_only = $true
        report_only = $true
        candidate_only = $true
        target_count = 2
        preflight_item_count = 1
        lookup_preview_item_count = 1
        ready_lookup_preview_count = 1
        approval_request_item_count = 1
        ready_approval_request_count = 1
        approval_decision_preview_item_count = 1
        ready_approval_decision_preview_count = 1
        token_intake_preview_item_count = 1
        ready_token_intake_preview_count = 1
        token_intake_decision_preview_item_count = 1
        ready_token_intake_decision_preview_count = 1
        pending_operator_token_count = 1
        approved_lookup_execution_count = 0
        blocked_item_count = 0
        approval_token_present_count = 1
        rejection_token_present_count = 1
        duplicate_cluster_count = 1
        duplicate_reflection_item_count = 1
        projected_saved_reflection_count = 1
        projected_model_call_skip_count = 1
        first_token_intake_decision_preview_id = "memory-reflection-reuse-lookup-approval-token-intake-decision-preview:memoryreflectiondedupe:fnv1a64-0000000000006666"
        lookup_approval_token_intake_decision_preview_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        model_call_skip_authorized = $false
        reflection_reuse_execution_authorized = $false
        memory_lookup_performed = $false
        lookup_hit_assumed = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_report_v1 = @{
        read_only = $true
        report_only = $true
        candidate_only = $true
        target_count = 2
        token_intake_decision_preview_item_count = 1
        ready_token_intake_decision_preview_count = 1
        token_decision_record_preview_item_count = 1
        ready_token_decision_record_preview_count = 1
        pending_operator_token_count = 1
        approved_lookup_execution_count = 0
        blocked_item_count = 0
        approval_token_present_count = 1
        rejection_token_present_count = 1
        duplicate_cluster_count = 1
        duplicate_reflection_item_count = 1
        projected_saved_reflection_count = 1
        projected_model_call_skip_count = 1
        first_token_decision_record_preview_id = "memory-reflection-reuse-lookup-approval-token-decision-record-preview:memoryreflectiondedupe:fnv1a64-0000000000007777"
        lookup_approval_token_decision_record_preview_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        model_call_skip_authorized = $false
        reflection_reuse_execution_authorized = $false
        memory_lookup_performed = $false
        lookup_hit_assumed = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_report_v1 = @{
        read_only = $true
        report_only = $true
        candidate_only = $true
        target_count = 2
        token_decision_record_preview_item_count = 1
        ready_token_decision_record_preview_count = 1
        token_decision_record_request_item_count = 1
        ready_token_decision_record_request_count = 1
        requested_token_decision_record_count = 1
        pending_operator_token_count = 1
        approved_lookup_execution_count = 0
        blocked_item_count = 0
        approval_token_present_count = 1
        rejection_token_present_count = 1
        duplicate_cluster_count = 1
        duplicate_reflection_item_count = 1
        projected_saved_reflection_count = 1
        projected_model_call_skip_count = 1
        first_token_decision_record_request_id = "memory-reflection-reuse-lookup-approval-token-decision-record-request:memoryreflectiondedupe:fnv1a64-0000000000008888"
        lookup_approval_token_decision_record_request_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        model_call_skip_authorized = $false
        reflection_reuse_execution_authorized = $false
        memory_lookup_performed = $false
        lookup_hit_assumed = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_report_v1 = @{
        read_only = $true
        report_only = $true
        candidate_only = $true
        target_count = 2
        token_decision_record_request_item_count = 1
        ready_token_decision_record_request_count = 1
        token_decision_record_review_packet_item_count = 1
        ready_token_decision_record_review_packet_count = 1
        requested_token_decision_record_count = 1
        pending_operator_token_count = 1
        approved_lookup_execution_count = 0
        blocked_item_count = 0
        approval_token_present_count = 1
        rejection_token_present_count = 1
        duplicate_cluster_count = 1
        duplicate_reflection_item_count = 1
        projected_saved_reflection_count = 1
        projected_model_call_skip_count = 1
        first_token_decision_record_review_packet_id = "memory-reflection-reuse-lookup-approval-token-decision-record-review-packet:memoryreflectiondedupe:fnv1a64-0000000000009999"
        lookup_approval_token_decision_record_review_packet_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        model_call_skip_authorized = $false
        reflection_reuse_execution_authorized = $false
        memory_lookup_performed = $false
        lookup_hit_assumed = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_report_v1 = @{
        read_only = $true
        report_only = $true
        candidate_only = $true
        target_count = 2
        token_decision_record_review_packet_item_count = 1
        ready_token_decision_record_review_packet_count = 1
        token_decision_record_review_packet_decision_preview_item_count = 1
        ready_token_decision_record_review_packet_decision_preview_count = 1
        requested_token_decision_record_count = 1
        pending_operator_token_count = 1
        approved_lookup_execution_count = 0
        blocked_item_count = 0
        approval_token_present_count = 1
        rejection_token_present_count = 1
        duplicate_cluster_count = 1
        duplicate_reflection_item_count = 1
        projected_saved_reflection_count = 1
        projected_model_call_skip_count = 1
        first_token_decision_record_review_packet_decision_preview_id = "memory-reflection-reuse-lookup-approval-token-decision-record-review-packet-decision-preview:memoryreflectiondedupe:fnv1a64-000000000000aaaa"
        lookup_approval_token_decision_record_review_packet_decision_preview_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        model_call_skip_authorized = $false
        reflection_reuse_execution_authorized = $false
        memory_lookup_performed = $false
        lookup_hit_assumed = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    self_improve_proposal_memory_admission_operator_approval_token_intake_preview_report_v1 = @{
        target_count = 2
        review_packet_item_count = 2
        useful_reflection_item_count = 2
        intake_item_count = 2
        ready_intake_count = 2
        pending_operator_token_count = 2
        blocked_count = 0
        approval_token_present_count = 2
        rejection_token_present_count = 2
        first_intake_item_id = "self-improve-r392-helper-contract"
        approval_token_intake_ready = $true
        explicit_operator_approval_required = $true
        validation_required = $true
        rollback_required = $true
        commit_allowed = $false
        admission_write_authorized = $false
        failure_reasons = @()
        auto_apply = $false
        memory_store_write_allowed = $false
        ndkv_write_allowed = $false
    }
    remote_chain = @{
        remote_runtime = @{
            probed = $false
            acceleration_ok = $false
        }
    }
} | ConvertTo-Json -Depth 10)
$reportJsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -ReportJson $report -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus -StrictLedgerHygiene
if ($LASTEXITCODE -ne 0) {
    throw "status report json command failed with exit code $LASTEXITCODE"
}
$reportStatus = ($reportJsonText | Out-String | ConvertFrom-Json)
if ($reportStatus.report.exists -ne $true) {
    throw "report fixture was not reported as existing"
}
if ($reportStatus.report.rounds -ne 1 -or $reportStatus.report.ledger_lag_rounds -ne 1 -or $reportStatus.report.stale -ne $true) {
    throw "report freshness fields were not exposed"
}
if ($reportStatus.report.success -ne 1 -or $reportStatus.report.failures -ne 0 -or [double]$reportStatus.report.success_rate -ne 100.0) {
    throw "report summary fields were not exposed"
}
if ($reportStatus.report.report_gate_passed -ne $false -or $reportStatus.report.report_gate_failure_count -ne 1) {
    throw "report gate fields were not exposed"
}
if ($reportStatus.report.self_improve_proposal_acceptance_summary_source -ne "self_improve_proposal_acceptance_summary_v1") {
    throw "report self-improve proposal top-level summary source was not exposed"
}
if ($reportStatus.report.self_improve_proposal_business_count -ne 0 -or $reportStatus.report.self_improve_proposal_advisory_count -ne 2 -or $reportStatus.report.self_improve_proposal_repair_count -ne 0) {
    throw "report self-improve proposal summary counts were not exposed"
}
if ($reportStatus.report.self_improve_proposal_convert_advisory_to_business_evidence -ne $true -or $reportStatus.report.self_improve_proposal_repair_unvalidated_or_unaccepted -ne $false -or $reportStatus.report.self_improve_proposal_requires_validation_and_memory_admission -ne $true) {
    throw "report self-improve proposal prompt guidance was not exposed"
}
if ($reportStatus.report.self_improve_proposal_action_required -ne $true -or $reportStatus.report.self_improve_proposal_primary_action -ne "convert_advisory_to_evidence_backed_business_improvement" -or $reportStatus.report.self_improve_proposal_action_plan_requires_validation_and_memory_admission -ne $true) {
    throw "report self-improve proposal action plan was not exposed"
}
$reportProposalActions = @($reportStatus.report.self_improve_proposal_actions)
if ($reportProposalActions.Count -ne 2 -or $reportProposalActions[0] -ne "convert_advisory_to_evidence_backed_business_improvement" -or $reportProposalActions[1] -ne "require_checked_passed_validation_and_accepted_memory_admission") {
    throw "report self-improve proposal action list was not exposed"
}
if ($reportStatus.report.self_improve_proposal_action_assignment_target_count -ne 2 -or $reportStatus.report.self_improve_proposal_action_assignment_first_target -ne "self-improve-r392-helper-contract") {
    throw "report self-improve proposal action assignment target metadata was not exposed"
}
$reportProposalFirstMissing = @($reportStatus.report.self_improve_proposal_action_assignment_first_missing_requirements)
if ($reportProposalFirstMissing.Count -ne 2 -or $reportProposalFirstMissing[0] -ne "accepted_memory_admission" -or $reportProposalFirstMissing[1] -ne "evidence_backed_business_improvement") {
    throw "report self-improve proposal action assignment missing requirements were not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_admission_commit_approval_review_packet_source -ne "self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1" -or $reportStatus.report.self_improve_proposal_memory_admission_commit_approval_review_packet_item_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_admission_commit_approval_review_packet_ready_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_admission_commit_approval_review_packet_pending_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_admission_commit_approval_review_packet_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_admission_commit_approval_review_packet_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_admission_commit_approval_review_packet_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_admission_commit_approval_review_packet_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_admission_commit_approval_review_packet_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal commit approval review packet was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_reflection_usefulness_source -ne "self_improve_proposal_memory_reflection_usefulness_report_v1" -or $reportStatus.report.self_improve_proposal_memory_reflection_usefulness_useful_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_reflection_usefulness_pending_operator_approval_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_reflection_usefulness_wasted_compute_guard_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_reflection_usefulness_adapter_safe_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_reflection_usefulness_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_usefulness_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_usefulness_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_usefulness_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_usefulness_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection usefulness was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_source -ne "self_improve_proposal_memory_reflection_dedupe_cluster_report_v1" -or $reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_useful_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_duplicate_cluster_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_duplicate_reflection_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_wasted_compute_guard_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_pending_operator_approval_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_adapter_safe_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_dedupe_cluster_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection dedupe cluster was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_reflection_reuse_plan_source -ne "self_improve_proposal_memory_reflection_reuse_plan_report_v1" -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_plan_cluster_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_plan_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_plan_ready_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_plan_duplicate_cluster_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_plan_duplicate_reflection_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_plan_projected_saved_reflection_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_plan_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_plan_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_plan_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_plan_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_plan_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection reuse plan was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_source -ne "self_improve_proposal_memory_reflection_reuse_preflight_report_v1" -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_plan_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_passed_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_blocked_item_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_projected_model_call_skip_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_passed -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_model_call_skip_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_execution_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_preflight_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection reuse preflight was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_source -ne "self_improve_proposal_memory_reflection_reuse_lookup_preview_report_v1" -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_preflight_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_ready_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_blocked_item_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_projected_model_call_skip_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_model_call_skip_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_execution_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_memory_lookup_performed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_lookup_hit_assumed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_read_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_report_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_candidate_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_auto_apply -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_preview_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection reuse lookup preview was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_source -ne "self_improve_proposal_memory_reflection_reuse_lookup_approval_request_report_v1" -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_preflight_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_lookup_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_ready_lookup_preview_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_ready_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_requested_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_blocked_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_projected_model_call_skip_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_model_call_skip_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_execution_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_memory_lookup_performed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_lookup_hit_assumed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_read_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_report_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_candidate_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_auto_apply -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_request_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection reuse lookup approval request was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_source -ne "self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_report_v1" -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_preflight_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_lookup_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ready_lookup_preview_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_approval_request_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ready_approval_request_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ready_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_approved_lookup_execution_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_pending_approval_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_blocked_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_projected_model_call_skip_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_model_call_skip_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_execution_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_memory_lookup_performed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_lookup_hit_assumed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_read_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_report_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_candidate_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_auto_apply -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection reuse lookup approval decision preview was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_source -ne "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_report_v1" -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_preflight_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_lookup_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready_lookup_preview_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_approval_request_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready_approval_request_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_approval_decision_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready_approval_decision_preview_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_pending_operator_token_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_blocked_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_projected_model_call_skip_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_model_call_skip_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_execution_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_memory_lookup_performed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_lookup_hit_assumed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_read_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_report_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_candidate_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_auto_apply -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection reuse lookup approval token intake preview was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_source -ne "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_report_v1" -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_preflight_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_lookup_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_lookup_preview_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_approval_request_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_approval_request_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_approval_decision_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_approval_decision_preview_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_token_intake_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_token_intake_preview_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_pending_operator_token_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_approved_lookup_execution_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_blocked_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_projected_model_call_skip_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_model_call_skip_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_execution_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_memory_lookup_performed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_lookup_hit_assumed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_read_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_report_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_candidate_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_auto_apply -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection reuse lookup approval token intake decision preview was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_source -ne "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_report_v1" -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_target_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_token_intake_decision_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_ready_token_intake_decision_preview_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_ready_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_pending_operator_token_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_approved_lookup_execution_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_blocked_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_projected_model_call_skip_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_model_call_skip_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_execution_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_memory_lookup_performed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_lookup_hit_assumed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_read_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_report_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_candidate_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_auto_apply -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection reuse lookup approval token decision record preview was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_source -ne "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_report_v1" -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_target_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_token_decision_record_preview_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_ready_token_decision_record_preview_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_ready_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_requested_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_pending_operator_token_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_approved_lookup_execution_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_blocked_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_projected_model_call_skip_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_model_call_skip_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_execution_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_memory_lookup_performed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_lookup_hit_assumed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_read_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_report_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_candidate_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_auto_apply -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection reuse lookup approval token decision record request was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_source -ne "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_report_v1" -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_target_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_token_decision_record_request_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_ready_token_decision_record_request_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_item_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_ready_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_requested_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_pending_operator_token_count -ne 1 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_approved_lookup_execution_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_blocked_count -ne 0 -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_first_item -ne "memory-reflection-reuse-lookup-approval-token-decision-record-review-packet:memoryreflectiondedupe:fnv1a64-0000000000009999" -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_validation_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_rollback_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_admission_write_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_model_call_skip_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_execution_authorized -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_memory_lookup_performed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_lookup_hit_assumed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_read_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_report_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_candidate_only -ne $true -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_auto_apply -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection reuse lookup approval token decision record review packet was not exposed"
}
$reviewPacketDecisionPreview = $reportStatus.report
if ($reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_source -ne "self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_report_v1" -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_target_count -ne 2 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_token_decision_record_review_packet_item_count -ne 1 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_ready_token_decision_record_review_packet_count -ne 1 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_item_count -ne 1 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_ready_count -ne 1 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_requested_count -ne 1 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_pending_operator_token_count -ne 1 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_approved_lookup_execution_count -ne 0 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_blocked_count -ne 0 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_approval_token_present_count -ne 1 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_rejection_token_present_count -ne 1 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_duplicate_cluster_count -ne 1 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_duplicate_reflection_item_count -ne 1 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_projected_saved_reflection_count -ne 1 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_projected_model_call_skip_count -ne 1 -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_first_item -ne "memory-reflection-reuse-lookup-approval-token-decision-record-review-packet-decision-preview:memoryreflectiondedupe:fnv1a64-000000000000aaaa" -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_ready -ne $true -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_explicit_operator_approval_required -ne $true -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_validation_required -ne $true -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_rollback_required -ne $true -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_commit_allowed -ne $false -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_admission_write_authorized -ne $false -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_model_call_skip_authorized -ne $false -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_execution_authorized -ne $false -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_memory_lookup_performed -ne $false -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_lookup_hit_assumed -ne $false -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_read_only -ne $true -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_report_only -ne $true -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_candidate_only -ne $true -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_auto_apply -ne $false -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_memory_store_write_allowed -ne $false -or $reviewPacketDecisionPreview.self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory reflection reuse lookup approval token decision record review packet decision preview was not exposed"
}
if ($reportStatus.report.self_improve_proposal_memory_approval_token_intake_source -ne "self_improve_proposal_memory_admission_operator_approval_token_intake_preview_report_v1" -or $reportStatus.report.self_improve_proposal_memory_approval_token_intake_item_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_approval_token_intake_ready_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_approval_token_intake_pending_operator_token_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_approval_token_intake_approval_token_present_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_approval_token_intake_rejection_token_present_count -ne 2 -or $reportStatus.report.self_improve_proposal_memory_approval_token_intake_ready -ne $true -or $reportStatus.report.self_improve_proposal_memory_approval_token_intake_explicit_operator_approval_required -ne $true -or $reportStatus.report.self_improve_proposal_memory_approval_token_intake_commit_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_approval_token_intake_memory_store_write_allowed -ne $false -or $reportStatus.report.self_improve_proposal_memory_approval_token_intake_ndkv_write_allowed -ne $false) {
    throw "report self-improve proposal memory approval token intake preview was not exposed"
}
if ($reportStatus.next_round_decision.display_state -ne "blocked-operator-attention" -or $reportStatus.next_round_decision.operator_attention_blocked -ne $true -or $reportStatus.next_round_decision.reason_code -ne "report_gate_failed_operator_attention_required") {
    throw "failed report gate did not surface blocked next-round decision"
}
Assert-NextRoundDecisionReportV1 -Report $reportStatus.next_round_decision_report_v1 -Decision $reportStatus.next_round_decision -Name "failed-report-status"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $reportStatus.next_round_downstream_status_consumers_v1 -Report $reportStatus.next_round_decision_report_v1 -Name "failed-report-status"
if ($reportStatus.report.remote_runtime_probed -ne $false -or $reportStatus.report.remote_runtime_acceleration_ok -ne $false) {
    throw "report remote runtime fields were not exposed"
}

$reportHumanText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -ReportJson $report -SkipBackend -SkipRemoteChain -SkipDaemon
if ($LASTEXITCODE -ne 0) {
    throw "status report human command failed with exit code $LASTEXITCODE"
}
$reportHuman = ($reportHumanText | Out-String)
if ($reportHuman -notmatch "report: exists=True") {
    throw "human status did not summarize report existence"
}
if ($reportHuman -notmatch "rounds=1" -or $reportHuman -notmatch "ledger_lag=1" -or $reportHuman -notmatch "stale=True") {
    throw "human status did not summarize report freshness"
}
if ($reportHuman -notmatch "gate_passed=False" -or $reportHuman -notmatch "gate_failures=1") {
    throw "human status did not summarize report gate"
}
if ($reportHuman -notmatch "remote_runtime_probed=False" -or $reportHuman -notmatch "remote_runtime_acceleration_ok=False") {
    throw "human status did not summarize report remote runtime"
}
if ($reportHuman -notmatch "report_self_improve_proposal_acceptance_summary_v1: source=self_improve_proposal_acceptance_summary_v1" -or $reportHuman -notmatch "advisory_only=2" -or $reportHuman -notmatch "convert_advisory_to_business_evidence=True" -or $reportHuman -notmatch "repair_unvalidated_or_unaccepted=False" -or $reportHuman -notmatch "requires_validation_and_memory_admission=True" -or $reportHuman -notmatch "action_required=True" -or $reportHuman -notmatch "primary_action=convert_advisory_to_evidence_backed_business_improvement" -or $reportHuman -notmatch "actions=convert_advisory_to_evidence_backed_business_improvement,require_checked_passed_validation_and_accepted_memory_admission" -or $reportHuman -notmatch "action_plan_requires_validation_and_memory_admission=True" -or $reportHuman -notmatch "action_assignment_targets=2" -or $reportHuman -notmatch "action_assignment_first_target=self-improve-r392-helper-contract" -or $reportHuman -notmatch "action_assignment_first_missing=accepted_memory_admission,evidence_backed_business_improvement") {
    throw "human status did not summarize self-improve proposal prompt guidance"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1: source=self_improve_proposal_memory_admission_commit_approval_review_packet_report_v1" -or $reportHuman -notmatch "review_packet_items=2" -or $reportHuman -notmatch "ready=2" -or $reportHuman -notmatch "pending=2" -or $reportHuman -notmatch "approval_review_packet_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve approval review packet"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_usefulness_report_v1: source=self_improve_proposal_memory_reflection_usefulness_report_v1" -or $reportHuman -notmatch "useful=2" -or $reportHuman -notmatch "pending_operator_approval=2" -or $reportHuman -notmatch "wasted_compute_guard=2" -or $reportHuman -notmatch "adapter_safe=2" -or $reportHuman -notmatch "reflection_usefulness_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection usefulness"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_dedupe_cluster_report_v1: source=self_improve_proposal_memory_reflection_dedupe_cluster_report_v1" -or $reportHuman -notmatch "clusters=1" -or $reportHuman -notmatch "duplicate_clusters=1" -or $reportHuman -notmatch "duplicate_reflections=1" -or $reportHuman -notmatch "wasted_compute_guard=2" -or $reportHuman -notmatch "adapter_safe=2" -or $reportHuman -notmatch "reflection_dedupe_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection dedupe cluster"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_reuse_plan_report_v1: source=self_improve_proposal_memory_reflection_reuse_plan_report_v1" -or $reportHuman -notmatch "plan_items=1" -or $reportHuman -notmatch "ready=1" -or $reportHuman -notmatch "duplicate_clusters=1" -or $reportHuman -notmatch "duplicate_reflections=1" -or $reportHuman -notmatch "projected_saved_reflections=1" -or $reportHuman -notmatch "reflection_reuse_plan_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection reuse plan"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_reuse_preflight_report_v1: source=self_improve_proposal_memory_reflection_reuse_preflight_report_v1" -or $reportHuman -notmatch "preflight_items=1" -or $reportHuman -notmatch "passed=1" -or $reportHuman -notmatch "blocked=0" -or $reportHuman -notmatch "projected_model_call_skips=1" -or $reportHuman -notmatch "reuse_preflight_passed=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "model_call_skip_authorized=False" -or $reportHuman -notmatch "reflection_reuse_execution_authorized=False" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection reuse preflight"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_reuse_lookup_preview_report_v1: source=self_improve_proposal_memory_reflection_reuse_lookup_preview_report_v1" -or $reportHuman -notmatch "lookup_items=1" -or $reportHuman -notmatch "ready=1" -or $reportHuman -notmatch "blocked=0" -or $reportHuman -notmatch "projected_model_call_skips=1" -or $reportHuman -notmatch "lookup_preview_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "model_call_skip_authorized=False" -or $reportHuman -notmatch "reflection_reuse_execution_authorized=False" -or $reportHuman -notmatch "memory_lookup_performed=False" -or $reportHuman -notmatch "lookup_hit_assumed=False" -or $reportHuman -notmatch "read_only=True" -or $reportHuman -notmatch "report_only=True" -or $reportHuman -notmatch "candidate_only=True" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection reuse lookup preview"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_request_report_v1: source=self_improve_proposal_memory_reflection_reuse_lookup_approval_request_report_v1" -or $reportHuman -notmatch "approval_requests=1" -or $reportHuman -notmatch "ready_requests=1" -or $reportHuman -notmatch "requested=1" -or $reportHuman -notmatch "blocked=0" -or $reportHuman -notmatch "projected_model_call_skips=1" -or $reportHuman -notmatch "lookup_approval_request_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "model_call_skip_authorized=False" -or $reportHuman -notmatch "reflection_reuse_execution_authorized=False" -or $reportHuman -notmatch "memory_lookup_performed=False" -or $reportHuman -notmatch "lookup_hit_assumed=False" -or $reportHuman -notmatch "read_only=True" -or $reportHuman -notmatch "report_only=True" -or $reportHuman -notmatch "candidate_only=True" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection reuse lookup approval request"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_report_v1: source=self_improve_proposal_memory_reflection_reuse_lookup_approval_decision_preview_report_v1" -or $reportHuman -notmatch "decision_previews=1" -or $reportHuman -notmatch "ready_decision_previews=1" -or $reportHuman -notmatch "approved_lookup_executions=0" -or $reportHuman -notmatch "pending=1" -or $reportHuman -notmatch "blocked=0" -or $reportHuman -notmatch "projected_model_call_skips=1" -or $reportHuman -notmatch "lookup_approval_decision_preview_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "model_call_skip_authorized=False" -or $reportHuman -notmatch "reflection_reuse_execution_authorized=False" -or $reportHuman -notmatch "memory_lookup_performed=False" -or $reportHuman -notmatch "lookup_hit_assumed=False" -or $reportHuman -notmatch "read_only=True" -or $reportHuman -notmatch "report_only=True" -or $reportHuman -notmatch "candidate_only=True" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection reuse lookup approval decision preview"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_report_v1: source=self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_preview_report_v1" -or $reportHuman -notmatch "token_intake_previews=1" -or $reportHuman -notmatch "ready_token_intake_previews=1" -or $reportHuman -notmatch "pending_operator_tokens=1" -or $reportHuman -notmatch "blocked=0" -or $reportHuman -notmatch "projected_model_call_skips=1" -or $reportHuman -notmatch "lookup_approval_token_intake_preview_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "model_call_skip_authorized=False" -or $reportHuman -notmatch "reflection_reuse_execution_authorized=False" -or $reportHuman -notmatch "memory_lookup_performed=False" -or $reportHuman -notmatch "lookup_hit_assumed=False" -or $reportHuman -notmatch "read_only=True" -or $reportHuman -notmatch "report_only=True" -or $reportHuman -notmatch "candidate_only=True" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection reuse lookup approval token intake preview"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_report_v1: source=self_improve_proposal_memory_reflection_reuse_lookup_approval_token_intake_decision_preview_report_v1" -or $reportHuman -notmatch "token_intake_decision_previews=1" -or $reportHuman -notmatch "ready_token_intake_decision_previews=1" -or $reportHuman -notmatch "approved_lookup_executions=0" -or $reportHuman -notmatch "pending_operator_tokens=1" -or $reportHuman -notmatch "blocked=0" -or $reportHuman -notmatch "projected_model_call_skips=1" -or $reportHuman -notmatch "lookup_approval_token_intake_decision_preview_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "model_call_skip_authorized=False" -or $reportHuman -notmatch "reflection_reuse_execution_authorized=False" -or $reportHuman -notmatch "memory_lookup_performed=False" -or $reportHuman -notmatch "lookup_hit_assumed=False" -or $reportHuman -notmatch "read_only=True" -or $reportHuman -notmatch "report_only=True" -or $reportHuman -notmatch "candidate_only=True" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection reuse lookup approval token intake decision preview"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_report_v1: source=self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_preview_report_v1" -or $reportHuman -notmatch "token_decision_record_previews=1" -or $reportHuman -notmatch "ready_token_decision_record_previews=1" -or $reportHuman -notmatch "approved_lookup_executions=0" -or $reportHuman -notmatch "pending_operator_tokens=1" -or $reportHuman -notmatch "blocked=0" -or $reportHuman -notmatch "projected_model_call_skips=1" -or $reportHuman -notmatch "lookup_approval_token_decision_record_preview_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "model_call_skip_authorized=False" -or $reportHuman -notmatch "reflection_reuse_execution_authorized=False" -or $reportHuman -notmatch "memory_lookup_performed=False" -or $reportHuman -notmatch "lookup_hit_assumed=False" -or $reportHuman -notmatch "read_only=True" -or $reportHuman -notmatch "report_only=True" -or $reportHuman -notmatch "candidate_only=True" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection reuse lookup approval token decision record preview"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_report_v1: source=self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_request_report_v1" -or $reportHuman -notmatch "token_decision_record_requests=1" -or $reportHuman -notmatch "ready_token_decision_record_requests=1" -or $reportHuman -notmatch "requested_token_decision_records=1" -or $reportHuman -notmatch "approved_lookup_executions=0" -or $reportHuman -notmatch "pending_operator_tokens=1" -or $reportHuman -notmatch "blocked=0" -or $reportHuman -notmatch "projected_model_call_skips=1" -or $reportHuman -notmatch "lookup_approval_token_decision_record_request_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "model_call_skip_authorized=False" -or $reportHuman -notmatch "reflection_reuse_execution_authorized=False" -or $reportHuman -notmatch "memory_lookup_performed=False" -or $reportHuman -notmatch "lookup_hit_assumed=False" -or $reportHuman -notmatch "read_only=True" -or $reportHuman -notmatch "report_only=True" -or $reportHuman -notmatch "candidate_only=True" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection reuse lookup approval token decision record request"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_report_v1: source=self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_report_v1" -or $reportHuman -notmatch "token_decision_record_review_packets=1" -or $reportHuman -notmatch "ready_token_decision_record_review_packets=1" -or $reportHuman -notmatch "requested_token_decision_records=1" -or $reportHuman -notmatch "approved_lookup_executions=0" -or $reportHuman -notmatch "pending_operator_tokens=1" -or $reportHuman -notmatch "blocked=0" -or $reportHuman -notmatch "lookup_approval_token_decision_record_review_packet_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "validation_required=True" -or $reportHuman -notmatch "rollback_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "admission_write_authorized=False" -or $reportHuman -notmatch "model_call_skip_authorized=False" -or $reportHuman -notmatch "reflection_reuse_execution_authorized=False" -or $reportHuman -notmatch "memory_lookup_performed=False" -or $reportHuman -notmatch "lookup_hit_assumed=False" -or $reportHuman -notmatch "read_only=True" -or $reportHuman -notmatch "report_only=True" -or $reportHuman -notmatch "candidate_only=True" -or $reportHuman -notmatch "auto_apply=False" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection reuse lookup approval token decision record review packet"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_report_v1: source=self_improve_proposal_memory_reflection_reuse_lookup_approval_token_decision_record_review_packet_decision_preview_report_v1" -or $reportHuman -notmatch "review_packet_decision_previews=1" -or $reportHuman -notmatch "ready_review_packet_decision_previews=1" -or $reportHuman -notmatch "requested_token_decision_records=1" -or $reportHuman -notmatch "approved_lookup_executions=0" -or $reportHuman -notmatch "pending_operator_tokens=1" -or $reportHuman -notmatch "blocked=0" -or $reportHuman -notmatch "lookup_approval_token_decision_record_review_packet_decision_preview_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "validation_required=True" -or $reportHuman -notmatch "rollback_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "admission_write_authorized=False" -or $reportHuman -notmatch "model_call_skip_authorized=False" -or $reportHuman -notmatch "reflection_reuse_execution_authorized=False" -or $reportHuman -notmatch "memory_lookup_performed=False" -or $reportHuman -notmatch "lookup_hit_assumed=False" -or $reportHuman -notmatch "read_only=True" -or $reportHuman -notmatch "report_only=True" -or $reportHuman -notmatch "candidate_only=True" -or $reportHuman -notmatch "auto_apply=False" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve memory reflection reuse lookup approval token decision record review packet decision preview"
}
if ($reportHuman -notmatch "report_self_improve_proposal_memory_admission_operator_approval_token_intake_preview_report_v1: source=self_improve_proposal_memory_admission_operator_approval_token_intake_preview_report_v1" -or $reportHuman -notmatch "intake_items=2" -or $reportHuman -notmatch "ready=2" -or $reportHuman -notmatch "pending_operator_tokens=2" -or $reportHuman -notmatch "approval_tokens=2" -or $reportHuman -notmatch "rejection_tokens=2" -or $reportHuman -notmatch "approval_token_intake_ready=True" -or $reportHuman -notmatch "explicit_operator_approval_required=True" -or $reportHuman -notmatch "commit_allowed=False" -or $reportHuman -notmatch "memory_store_write_allowed=False" -or $reportHuman -notmatch "ndkv_write_allowed=False") {
    throw "human status did not summarize self-improve approval token intake preview"
}
if ($reportHuman -notmatch "next_round_decision: schema=next_round_decision_evidence_v1 display_state=blocked-operator-attention") {
    throw "human status did not summarize blocked next-round decision"
}
if ($reportHuman -notmatch "next_round_decision_report_v1: schema=next_round_decision_report_v1 display_state=blocked-operator-attention") {
    throw "human status did not summarize blocked next-round decision report v1"
}
if ($reportHuman -notmatch "next_round_downstream_status_consumers_v1: schema=next_round_downstream_status_consumers_v1 effective_decision_status=operator_attention_blocked") {
    throw "human status did not summarize blocked downstream next-round consumers"
}

$legacyProposalReport = Join-Path $testDir "legacy-proposal-report.json"
Set-Content -Encoding ASCII -LiteralPath $legacyProposalReport -Value (@{
    rounds = 2
    success = 2
    failures = 0
    success_rate = 100.0
    report_gate = @{
        passed = $true
        failures = @()
    }
    self_improve_proposal_artifact_v1 = @{
        proposals = @(
            @{
                business_improvement_acceptance = @{
                    evidence_backed_business_improvement = $false
                    advisory_only = $true
                    require_repair = $false
                    memory_admission_accepted = $false
                }
            },
            @{
                business_improvement_acceptance = @{
                    evidence_backed_business_improvement = $false
                    advisory_only = $false
                    require_repair = $true
                    memory_admission_accepted = $true
                }
            }
        )
    }
} | ConvertTo-Json -Depth 10)
$legacyProposalText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -ReportJson $legacyProposalReport -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "legacy proposal report status command failed with exit code $LASTEXITCODE"
}
$legacyProposalStatus = ($legacyProposalText | Out-String | ConvertFrom-Json)
if ($legacyProposalStatus.report.self_improve_proposal_acceptance_summary_source -ne "self_improve_proposal_artifact_v1") {
    throw "legacy proposal report did not use artifact fallback source"
}
if ($legacyProposalStatus.report.self_improve_proposal_business_count -ne 0 -or $legacyProposalStatus.report.self_improve_proposal_advisory_count -ne 1 -or $legacyProposalStatus.report.self_improve_proposal_repair_count -ne 1 -or $legacyProposalStatus.report.self_improve_proposal_accepted_without_business_evidence_count -ne 1) {
    throw "legacy proposal report fallback counts were not derived"
}
if ($legacyProposalStatus.report.self_improve_proposal_convert_advisory_to_business_evidence -ne $true -or $legacyProposalStatus.report.self_improve_proposal_repair_unvalidated_or_unaccepted -ne $true -or $legacyProposalStatus.report.self_improve_proposal_requires_validation_and_memory_admission -ne $true) {
    throw "legacy proposal report fallback prompt guidance was not derived"
}
if ($legacyProposalStatus.report.self_improve_proposal_action_required -ne $true -or $legacyProposalStatus.report.self_improve_proposal_primary_action -ne "convert_advisory_to_evidence_backed_business_improvement" -or $legacyProposalStatus.report.self_improve_proposal_action_plan_requires_validation_and_memory_admission -ne $true) {
    throw "legacy proposal report fallback action plan was not derived"
}
$legacyProposalActions = @($legacyProposalStatus.report.self_improve_proposal_actions)
if ($legacyProposalActions.Count -ne 3 -or $legacyProposalActions[0] -ne "convert_advisory_to_evidence_backed_business_improvement" -or $legacyProposalActions[1] -ne "repair_unvalidated_or_unaccepted_proposals" -or $legacyProposalActions[2] -ne "require_checked_passed_validation_and_accepted_memory_admission") {
    throw "legacy proposal report fallback action list was not derived"
}

$passedReport = Join-Path $testDir "passed-report.json"
Set-Content -Encoding ASCII -LiteralPath $passedReport -Value (@{
    rounds = 2
    success = 2
    failures = 0
    success_rate = 100.0
    report_gate = @{
        passed = $true
        failures = @()
    }
} | ConvertTo-Json -Depth 10)

$selfImproveText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus -RequireLatestSelfImprove -FailOnNotReady
if ($LASTEXITCODE -ne 0) {
    throw "ledger with latest self-improve evidence should exit 0, got $LASTEXITCODE"
}
$selfImproveStatus = ($selfImproveText | Out-String | ConvertFrom-Json)
if ($selfImproveStatus.readiness.ready -ne $true) {
    throw "ledger with latest self-improve evidence should be ready"
}
if ($selfImproveStatus.ledger.latest.feedback_applied -le 0 -or $selfImproveStatus.ledger.latest.self_improve_passed -ne $true) {
    throw "latest self-improve evidence was not exposed"
}

$missingSelfImproveLedger = Join-Path $testDir "missing-self-improve-ledger.jsonl"
Set-Content -Encoding ASCII -LiteralPath $missingSelfImproveLedger -Value @(
    '{"round":1,"case":"status-selftest-self-improve-missing-0001","success":true,"runtime_tokens":8,"elapsed_ms":100,"feedback_applied":2,"self_improve_passed":true}',
    '{"round":2,"case":"status-selftest-self-improve-missing-0002","success":true,"runtime_tokens":13,"elapsed_ms":200,"feedback_applied":0,"self_improve_passed":false}'
)
$missingSelfImproveText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $missingSelfImproveLedger -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus -RequireLatestSelfImprove -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "missing latest self-improve evidence should exit nonzero"
}
$missingSelfImproveStatus = ($missingSelfImproveText | Out-String | ConvertFrom-Json)
if ($missingSelfImproveStatus.readiness.ready -ne $false) {
    throw "missing latest self-improve evidence should be not-ready"
}
if ((@($missingSelfImproveStatus.readiness.failures) -join ",") -notmatch "latest_self_improve_missing") {
    throw "missing latest self-improve evidence did not report latest_self_improve_missing"
}

$helperStageLedger = Join-Path $testDir "helper-stage-ledger.jsonl"
Set-Content -Encoding ASCII -LiteralPath $helperStageLedger -Value @(
    '{"round":1,"case":"status-selftest-helper-stage-0001","success":true,"runtime_tokens":8,"elapsed_ms":100,"feedback_applied":2,"self_improve_passed":true,"helper_stage_feedback_by_role":{"summary":["task_kind=summary preview=memory_update: keep"],"router":["task_kind=router preview=route_intent: index"],"review":["task_kind=review preview=change_request: tune"],"index":["task_kind=index preview=clean_gist: keep"],"test-gate":["task_kind=test-gate preview=verdict: pass / validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"]},"helper_stage_contract_by_role":{"summary":{"fields":{"memory_update":"keep"}},"router":{"fields":{"route_intent":"index"}},"review":{"fields":{"change_request":"tune"}},"index":{"fields":{"clean_gist":"keep","tags":"role=index;case=status-selftest-helper-stage-0001;round=1;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123","dependency_link":"review.change_request","source_origin":"review.change_request","validation_timestamp":"1781770123","retention":"keep"}},"test-gate":{"fields":{"verdict":"pass","validation_command":"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"}}}}'
)
$helperStageText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $helperStageLedger -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus -RequireLatestHelperStageRoles "summary,router,review,index,test-gate" -RequireLatestHelperStageContracts -RequireLatestTestGatePass -RequireLatestSafeTestGateValidationCommand -FailOnNotReady
if ($LASTEXITCODE -ne 0) {
    throw "ledger with latest helper stage evidence should exit 0, got $LASTEXITCODE"
}
$helperStageStatus = ($helperStageText | Out-String | ConvertFrom-Json)
if ($helperStageStatus.readiness.ready -ne $true) {
    throw "ledger with latest helper stage evidence should be ready"
}
$helperRoles = @($helperStageStatus.ledger.latest.helper_stage_roles)
if (($helperRoles -join ",") -ne "index,review,router,summary,test-gate") {
    throw "helper stage roles were not exposed: $($helperRoles -join ',')"
}
if ($helperStageStatus.ledger.latest.helper_stage_role_count -ne 5 -or $helperStageStatus.ledger.latest.helper_stage_feedback_total -ne 5) {
    throw "helper stage role or feedback counts were not exposed"
}
$helperContractRoles = @($helperStageStatus.ledger.latest.helper_stage_contract_roles)
if (($helperContractRoles -join ",") -ne "index,review,router,summary,test-gate") {
    throw "helper stage contract roles were not exposed: $($helperContractRoles -join ',')"
}
if ($helperStageStatus.ledger.latest.helper_stage_contract_complete -ne $true) {
    throw "helper stage contract completeness was not exposed"
}
$helperContractCompleteRoles = @($helperStageStatus.ledger.latest.helper_stage_contract_complete_roles)
if (($helperContractCompleteRoles -join ",") -ne "index,review,router,summary,test-gate") {
    throw "helper stage complete contract roles were not exposed: $($helperContractCompleteRoles -join ',')"
}
if (@($helperStageStatus.ledger.latest.helper_stage_contract_incomplete_roles).Count -ne 0) {
    throw "helper stage complete fixture should not report incomplete contract roles"
}
if ($helperStageStatus.ledger.latest.test_gate_verdict -ne "pass" -or $helperStageStatus.ledger.latest.test_gate_passed -ne $true) {
    throw "test-gate pass evidence was not exposed"
}
if ($helperStageStatus.ledger.latest.test_gate_validation_command_safety -ne "safe") {
    throw "test-gate validation command safety was not exposed"
}

$missingHelperStageLedger = Join-Path $testDir "missing-helper-stage-ledger.jsonl"
Set-Content -Encoding ASCII -LiteralPath $missingHelperStageLedger -Value @(
    '{"round":1,"case":"status-selftest-helper-stage-missing-0001","success":true,"runtime_tokens":8,"elapsed_ms":100,"feedback_applied":2,"self_improve_passed":true,"helper_stage_feedback_by_role":{"summary":["task_kind=summary preview=memory_update: keep"],"router":["task_kind=router preview=route_intent: index"]}}'
)
$missingHelperStageText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $missingHelperStageLedger -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus -RequireLatestHelperStageRoles "summary,router,review,index,test-gate" -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "missing latest helper stage evidence should exit nonzero"
}
$missingHelperStageStatus = ($missingHelperStageText | Out-String | ConvertFrom-Json)
if ($missingHelperStageStatus.readiness.ready -ne $false) {
    throw "missing latest helper stage evidence should be not-ready"
}
if ((@($missingHelperStageStatus.readiness.failures) -join ",") -notmatch "latest_helper_stage_roles_missing") {
    throw "missing latest helper stage evidence did not report latest_helper_stage_roles_missing"
}

$incompleteHelperContractLedger = Join-Path $testDir "incomplete-helper-contract-ledger.jsonl"
Set-Content -Encoding ASCII -LiteralPath $incompleteHelperContractLedger -Value @(
    '{"round":1,"case":"status-selftest-helper-contract-incomplete-0001","success":true,"runtime_tokens":8,"elapsed_ms":100,"feedback_applied":2,"self_improve_passed":true,"helper_stage_feedback_by_role":{"summary":["task_kind=summary preview=memory_update: keep"],"router":["task_kind=router preview=route_intent: index"],"review":["task_kind=review preview=risk: intermittent"],"index":["task_kind=index preview=clean_gist: keep"],"test-gate":["task_kind=test-gate preview=verdict: pass"]},"helper_stage_contract_by_role":{"summary":{"matched_markers":["memory_update","next_context","duplicate_guard"],"expected_markers":["memory_update","next_context","duplicate_guard"]},"router":{"matched_markers":["route_intent","tool_call","preflight"],"expected_markers":["route_intent","tool_call","preflight"]},"review":{"matched_markers":["risk"],"expected_markers":["risk","change_request","verification"]},"index":{"matched_markers":["clean_gist","tags","retention"],"expected_markers":["clean_gist","tags","dependency_link","source_origin","validation_timestamp","retention"]},"test-gate":{"matched_markers":["verdict","validation_command","failure_kind"],"expected_markers":["verdict","validation_command","failure_kind"]}}}'
)
$incompleteHelperContractText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $incompleteHelperContractLedger -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus -RequireLatestHelperStageRoles "summary,router,review,index,test-gate" -RequireLatestHelperStageContracts -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "incomplete latest helper contract should exit nonzero"
}
$incompleteHelperContractStatus = ($incompleteHelperContractText | Out-String | ConvertFrom-Json)
if ($incompleteHelperContractStatus.readiness.ready -ne $false) {
    throw "incomplete latest helper contract should be not-ready"
}
if ((@($incompleteHelperContractStatus.readiness.failures) -join ",") -notmatch "latest_helper_stage_contract_incomplete") {
    throw "incomplete latest helper contract did not report latest_helper_stage_contract_incomplete"
}
$incompleteContractRoles = @($incompleteHelperContractStatus.ledger.latest.helper_stage_contract_incomplete_roles)
if (($incompleteContractRoles -join ",") -ne "index,review") {
    throw "incomplete helper contract roles were not exposed: $($incompleteContractRoles -join ',')"
}

$failingTestGateLedger = Join-Path $testDir "failing-test-gate-ledger.jsonl"
Set-Content -Encoding ASCII -LiteralPath $failingTestGateLedger -Value @(
    '{"round":1,"case":"status-selftest-test-gate-fail-0001","success":true,"runtime_tokens":8,"elapsed_ms":100,"feedback_applied":2,"self_improve_passed":true,"helper_stage_feedback_by_role":{"test-gate":["task_kind=test-gate preview=verdict: fail"]},"helper_stage_contract_by_role":{"test-gate":{"fields":{"verdict":"fail","validation_command":"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"},"matched_markers":["verdict","validation_command","failure_kind"],"expected_markers":["verdict","validation_command","failure_kind"]}}}'
)
$failingTestGateText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $failingTestGateLedger -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus -RequireLatestTestGatePass -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "failing latest test-gate verdict should exit nonzero"
}
$failingTestGateStatus = ($failingTestGateText | Out-String | ConvertFrom-Json)
if ((@($failingTestGateStatus.readiness.failures) -join ",") -notmatch "latest_test_gate_not_pass") {
    throw "failing latest test-gate verdict did not report latest_test_gate_not_pass"
}
if ($failingTestGateStatus.ledger.latest.test_gate_passed -ne $false) {
    throw "failing latest test-gate verdict should expose test_gate_passed=false"
}

$unsafeTestGateLedger = Join-Path $testDir "unsafe-test-gate-ledger.jsonl"
Set-Content -Encoding ASCII -LiteralPath $unsafeTestGateLedger -Value @(
    '{"round":1,"case":"status-selftest-test-gate-unsafe-0001","success":true,"runtime_tokens":8,"elapsed_ms":100,"feedback_applied":2,"self_improve_passed":true,"helper_stage_feedback_by_role":{"test-gate":["task_kind=test-gate preview=verdict: pass"]},"helper_stage_contract_by_role":{"test-gate":{"fields":{"verdict":"pass","validation_command":"cargo test; Remove-Item target"},"matched_markers":["verdict","validation_command","failure_kind"],"expected_markers":["verdict","validation_command","failure_kind"]}}}'
)
$unsafeTestGateText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $unsafeTestGateLedger -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus -RequireLatestSafeTestGateValidationCommand -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "unsafe latest test-gate validation command should exit nonzero"
}
$unsafeTestGateStatus = ($unsafeTestGateText | Out-String | ConvertFrom-Json)
if ((@($unsafeTestGateStatus.readiness.failures) -join ",") -notmatch "latest_test_gate_validation_command_not_safe") {
    throw "unsafe latest test-gate validation command did not report latest_test_gate_validation_command_not_safe"
}
if ($unsafeTestGateStatus.ledger.latest.test_gate_validation_command_safety -ne "unsafe") {
    throw "unsafe latest test-gate validation command should expose unsafe safety"
}

$validatedLedger = Join-Path $testDir "validated-ledger.jsonl"
Set-Content -Encoding ASCII -LiteralPath $validatedLedger -Value @(
    '{"round":1,"case":"status-selftest-validation-0001","success":true,"runtime_tokens":8,"elapsed_ms":100,"feedback_applied":2,"self_improve_passed":true,"validation_checked":true,"validation_passed":true,"validation_command_source":"configured","validation_command_safety":"explicit","validation_status_code":0,"validation_elapsed_ms":1234}',
    '{"round":2,"case":"status-selftest-validation-0002","success":true,"runtime_tokens":13,"elapsed_ms":200,"feedback_applied":3,"self_improve_passed":true,"validation_checked":true,"validation_passed":true,"validation_command_source":"configured","validation_command_safety":"explicit","validation_status_code":0,"validation_elapsed_ms":2345}'
)
$validatedText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $validatedLedger -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus -RequireLatestConfiguredValidationRun -FailOnNotReady
if ($LASTEXITCODE -ne 0) {
    throw "validated ledger with RequireLatestConfiguredValidationRun should exit 0, got $LASTEXITCODE"
}
$validatedStatus = ($validatedText | Out-String | ConvertFrom-Json)
if ($validatedStatus.readiness.ready -ne $true) {
    throw "validated ledger should be ready"
}
if ($validatedStatus.ledger.latest.validation_checked -ne $true -or $validatedStatus.ledger.latest.validation_passed -ne $true) {
    throw "validated ledger latest validation booleans were not exposed"
}
if ($validatedStatus.ledger.latest.validation_command_source -ne "configured") {
    throw "validated ledger latest validation source was not exposed"
}
if ($validatedStatus.ledger.latest.validation_status_code -ne 0) {
    throw "validated ledger latest validation status code was not exposed"
}

$missingValidatedLedger = Join-Path $testDir "missing-validated-ledger.jsonl"
Set-Content -Encoding ASCII -LiteralPath $missingValidatedLedger -Value @(
    '{"round":1,"case":"status-selftest-validation-missing-0001","success":true,"runtime_tokens":8,"elapsed_ms":100,"feedback_applied":2,"self_improve_passed":true,"validation_checked":true,"validation_passed":true,"validation_command_source":"configured","validation_status_code":0}',
    '{"round":2,"case":"status-selftest-validation-missing-0002","success":true,"runtime_tokens":13,"elapsed_ms":200,"feedback_applied":3,"self_improve_passed":true}'
)
$missingValidatedText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $missingValidatedLedger -SkipBackend -SkipRemoteChain -SkipDaemon -JsonStatus -RequireLatestConfiguredValidationRun -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "missing latest configured validation should exit nonzero"
}
$missingValidatedStatus = ($missingValidatedText | Out-String | ConvertFrom-Json)
if ($missingValidatedStatus.readiness.ready -ne $false) {
    throw "missing latest configured validation should be not-ready"
}
if ((@($missingValidatedStatus.readiness.failures) -join ",") -notmatch "latest_configured_validation_missing") {
    throw "missing latest configured validation did not report latest_configured_validation_missing"
}

$remoteJsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipProcess -SkipDaemon -RemoteChainStatusJson $remoteStatus -JsonStatus -StrictLedgerHygiene
if ($LASTEXITCODE -ne 0) {
    throw "status remote json command failed with exit code $LASTEXITCODE"
}
$remoteStatusResult = ($remoteJsonText | Out-String | ConvertFrom-Json)
if ($remoteStatusResult.remote_chain.ready -ne $true) {
    throw "remote chain fixture was not reported ready"
}
if ($remoteStatusResult.remote_chain.model_cache_all_ok -ne $true) {
    throw "remote chain model_cache_all_ok was not exposed"
}
if ($remoteStatusResult.remote_chain.model_cache_ok_count -ne 5 -or $remoteStatusResult.remote_chain.model_cache_model_count -ne 5) {
    throw "remote chain model_cache counts were not exposed"
}
if ($remoteStatusResult.remote_chain.model_cache_remote_error_count -ne 0) {
    throw "remote chain model_cache remote error count was not exposed"
}
if ($remoteStatusResult.remote_chain.remote_runtime.probed -ne $true) {
    throw "remote chain runtime probe flag was not exposed"
}
if ($remoteStatusResult.remote_chain.remote_runtime.worker_count -ne 6) {
    throw "remote chain runtime worker count was not exposed"
}
if ($remoteStatusResult.remote_chain.remote_runtime.cpu_or_no_gpu_count -ne 3) {
    throw "remote chain runtime cpu/no-gpu count was not exposed"
}
$cpuRoles = @($remoteStatusResult.remote_chain.remote_runtime.cpu_or_no_gpu_roles)
if (($cpuRoles -join ",") -ne "summary,review,test-gate") {
    throw "remote chain runtime cpu/no-gpu roles were not exposed: $($cpuRoles -join ',')"
}
$metadataRoles = @($remoteStatusResult.remote_chain.remote_runtime.backend_metadata_may_differ_roles)
if (($metadataRoles -join ",") -ne "summary,review,test-gate") {
    throw "remote chain runtime metadata-differ roles were not exposed: $($metadataRoles -join ',')"
}
if ($remoteStatusResult.remote_chain.remote_runtime.acceleration_ok -ne $false) {
    throw "remote chain runtime acceleration_ok should be false for cpu/no-gpu roles"
}
if ([string]$remoteStatusResult.remote_chain.remote_runtime.acceleration_next_step -notmatch "run-remote-gemma-unattended\.cmd -RestartRemote -SkipBuild") {
    throw "remote chain runtime acceleration next_step was not exposed"
}

$humanText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipDaemon
if ($LASTEXITCODE -ne 0) {
    throw "status human command failed with exit code $LASTEXITCODE"
}
$human = ($humanText | Out-String)
if ($human -notmatch "sends_prompt=false") {
    throw "human status did not print sends_prompt=false"
}
if ($human -notmatch "ledger_records: total=2") {
    throw "human status did not summarize ledger records"
}

$humanRemoteText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipProcess -SkipDaemon -RemoteChainStatusJson $remoteStatus
if ($LASTEXITCODE -ne 0) {
    throw "status human remote command failed with exit code $LASTEXITCODE"
}
$humanRemote = ($humanRemoteText | Out-String)
if ($humanRemote -notmatch "model_cache_ok=5/5") {
    throw "human status did not summarize remote model_cache counts"
}
if ($humanRemote -notmatch "model_cache_all_ok=True") {
    throw "human status did not summarize remote model_cache all_ok"
}
if ($humanRemote -notmatch "remote_runtime_probed=True") {
    throw "human status did not summarize remote runtime probe state"
}
if ($humanRemote -notmatch "remote_runtime_cpu_or_no_gpu=3") {
    throw "human status did not summarize remote runtime cpu/no-gpu count"
}
if ($humanRemote -notmatch "remote_runtime_cpu_or_no_gpu_roles=summary,review,test-gate") {
    throw "human status did not summarize remote runtime cpu/no-gpu roles"
}
if ($humanRemote -notmatch "remote_runtime_acceleration_ok=False") {
    throw "human status did not summarize remote runtime acceleration state"
}
if ($humanRemote -notmatch "run-remote-gemma-unattended\.cmd -RestartRemote -SkipBuild") {
    throw "human status did not summarize remote runtime acceleration next step"
}

$daemonDir = Join-Path $testDir "daemon"
New-Item -ItemType Directory -Force -Path $daemonDir | Out-Null
Set-Content -Encoding ASCII -LiteralPath (Join-Path $daemonDir "evolution-loop.pid") -Value ([string]$PID)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $daemonDir "evolution-ledger.jsonl") -Value '{"round":2,"case":"status-selftest-0002","success":true,"runtime_tokens":13,"elapsed_ms":200,"feedback_applied":2,"self_improve_passed":true}'
Set-Content -Encoding ASCII -LiteralPath (Join-Path $daemonDir "evolution-loop.out.log") -Value @(
    "[round 1] case=status-selftest-0001",
    "[round 1] stage ledger_append:done",
    "[round 1] ok runtime_tokens=8 elapsed_ms=100",
    "[round 2] case=status-selftest-0002",
    "[round 2] stage generate:start"
)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $daemonDir "evolution-loop.err.log") -Value 'Running tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --validation-command cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --require-configured-validation-run'

$daemonJsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -ReportJson $passedReport -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "status daemon json command failed with exit code $LASTEXITCODE"
}
$daemonStatus = ($daemonJsonText | Out-String | ConvertFrom-Json)
if ($daemonStatus.daemon.checked -ne $true) {
    throw "daemon status was not checked"
}
if ($daemonStatus.daemon.running -ne $true) {
    throw "daemon status did not detect running fixture process"
}
if ($daemonStatus.daemon.activity_state -ne "active") {
    throw "daemon status did not classify active fixture"
}
if ($daemonStatus.daemon.activity_reason -ne "round_in_progress_stdout_recent") {
    throw "daemon status did not expose active activity reason"
}
if ($daemonStatus.daemon.daemon_round_transition_status.schema -ne "daemon_round_transition_status_v1") {
    throw "daemon status did not expose transition status schema"
}
if ($daemonStatus.daemon.daemon_round_transition_status.transition_kind -ne "normal_in_progress") {
    throw "daemon status did not expose normal_in_progress transition kind"
}
if ($daemonStatus.daemon.daemon_round_transition_status.activity_reason -ne "round_in_progress_stdout_recent") {
    throw "daemon status transition did not expose active reason"
}
if ($daemonStatus.daemon.daemon_round_transition_status.read_only -ne $true -or $daemonStatus.daemon.daemon_round_transition_status.starts_process -ne $false -or $daemonStatus.daemon.daemon_round_transition_status.sends_prompt -ne $false) {
    throw "daemon status transition broke report-only contract"
}
Assert-DaemonRoundTransitionConsumerStatus -Status $daemonStatus.daemon.daemon_round_transition_status -Name "status-json:normal_in_progress" -ExpectedKind "normal_in_progress" -ExpectedRoundInProgress $true
if ($daemonStatus.daemon.ledger_lag_rounds -ne 0) {
    throw "daemon status reported unexpected fixture ledger lag"
}
if ([string]$daemonStatus.daemon.operator_summary -notmatch "state=active") {
    throw "daemon status did not expose operator summary"
}
if ([string]$daemonStatus.next_step -notmatch "daemon active: wait for current round") {
    throw "status next_step did not prioritize active daemon"
}
if ($daemonStatus.next_round_decision.display_state -ne "safe-to-wait" -or $daemonStatus.next_round_decision.safe_to_wait_current_round_active -ne $true -or $daemonStatus.next_round_decision.safe_to_continue_after_current_round -ne $false -or $daemonStatus.next_round_decision.operator_attention_blocked -ne $false) {
    throw "active daemon with passed report gate did not expose safe-to-wait next-round decision"
}
Assert-NextRoundDecisionReportV1 -Report $daemonStatus.next_round_decision_report_v1 -Decision $daemonStatus.next_round_decision -Name "active-daemon-status"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $daemonStatus.next_round_downstream_status_consumers_v1 -Report $daemonStatus.next_round_decision_report_v1 -Name "active-daemon-status" -DaemonRoundTransitionStatus $daemonStatus.daemon.daemon_round_transition_status
if ($daemonStatus.live_status_bundle.daemon.daemon_round_transition_status.transition_kind -ne "normal_in_progress" -or $daemonStatus.live_status_bundle.report_gate.passed -ne $true) {
    throw "active daemon live status bundle did not carry transition and report-gate evidence"
}
Assert-NextRoundDecisionReportV1 -Report $daemonStatus.live_status_bundle.next_round_decision_report_v1 -Decision $daemonStatus.next_round_decision -Name "active-daemon-live-bundle"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $daemonStatus.live_status_bundle.next_round_downstream_status_consumers_v1 -Report $daemonStatus.live_status_bundle.next_round_decision_report_v1 -Name "active-daemon-live-bundle" -DaemonRoundTransitionStatus $daemonStatus.live_status_bundle.daemon.daemon_round_transition_status

Set-Content -Encoding ASCII -LiteralPath (Join-Path $daemonDir "report.json") -Value (@{
    rounds = 2
    success = 2
    failures = 0
    success_rate = 100.0
    report_gate = @{
        passed = $true
        failures = @()
    }
} | ConvertTo-Json -Depth 10)
$daemonAutoReportJsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "status daemon auto report json command failed with exit code $LASTEXITCODE"
}
$daemonAutoReportStatus = ($daemonAutoReportJsonText | Out-String | ConvertFrom-Json)
if ($daemonAutoReportStatus.ledger_source -ne "daemon_auto" -or $daemonAutoReportStatus.daemon_ledger_auto_selected -ne $true) {
    throw "daemon auto report status did not auto-select daemon ledger"
}
if ($daemonAutoReportStatus.report_source -ne "daemon_auto" -or $daemonAutoReportStatus.daemon_report_auto_selected -ne $true) {
    throw "daemon auto report status did not auto-select daemon report"
}
if ($daemonAutoReportStatus.report.exists -ne $true -or $daemonAutoReportStatus.report.path -ne (Join-Path $daemonDir "report.json")) {
    throw "daemon auto report status did not expose daemon report path"
}
if ($daemonAutoReportStatus.live_status_bundle.report_gate.passed -ne $true -or $daemonAutoReportStatus.next_round_decision.reason_code -ne "active_round_in_progress_wait_for_completion") {
    throw "daemon auto report status did not use report-gate evidence for next-round decision"
}

$strictActiveDaemonDir = Join-Path $testDir "daemon-strict-active-ledger-gate"
New-Item -ItemType Directory -Force -Path $strictActiveDaemonDir | Out-Null
$strictActiveLatest = [ordered]@{
    round = 377
    case = "status-selftest-strict-active-ledger-gate-0377"
    success = $true
    runtime_tokens = 13
    elapsed_ms = 200
    feedback_applied = 3
    self_improve_passed = $true
    validation_checked = $true
    validation_passed = $true
    validation_command_source = "configured"
    validation_command_safety = "safe"
    validation_status_code = 0
    helper_stage_feedback_by_role = [ordered]@{
        summary = @("task_kind=summary preview=memory_update: keep")
        router = @("task_kind=router preview=route_intent: index")
        review = @("task_kind=review preview=change_request: tune")
        index = @("task_kind=index preview=clean_gist: keep")
        "test-gate" = @("task_kind=test-gate preview=verdict: pass / validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml")
    }
    helper_stage_contract_by_role = [ordered]@{
        summary = @{ fields = @{ memory_update = "keep" } }
        router = @{ fields = @{ route_intent = "index" } }
        review = @{ fields = @{ change_request = "tune" } }
        index = @{ fields = @{ clean_gist = "keep"; tags = "role=index;case=status-selftest-strict-active-ledger-gate-0377;round=377;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123"; dependency_link = "review.change_request"; source_origin = "review.change_request"; validation_timestamp = "1781770123"; retention = "keep" } }
        "test-gate" = @{ fields = @{ verdict = "pass"; validation_command = "cargo test -q --manifest-path tools/evolution-loop/Cargo.toml" } }
    }
}
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictActiveDaemonDir "evolution-loop.pid") -Value ([string]$PID)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictActiveDaemonDir "evolution-ledger.jsonl") -Value ($strictActiveLatest | ConvertTo-Json -Depth 12 -Compress)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictActiveDaemonDir "evolution-loop.out.log") -Value @(
    "[round 377] case=status-selftest-strict-active-ledger-gate-0377",
    "[round 377] stage ledger_append:done",
    "[round 377] ok runtime_tokens=13 elapsed_ms=200",
    "[round 377] done [DONE]",
    "[round 378] case=status-selftest-strict-active-ledger-gate-0378",
    "[round 378] stage generate:start"
)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictActiveDaemonDir "evolution-loop.err.log") -Value 'Running tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --validation-command cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --require-configured-validation-run'

$strictActiveText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $strictActiveDaemonDir -StrictUnattendedEvolution -FailOnNotReady -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "strict active daemon ledger-derived report gate should be ready, got exit code $LASTEXITCODE"
}
$strictActive = ($strictActiveText | Out-String | ConvertFrom-Json)
if ($strictActive.report.exists -ne $false -or $strictActive.live_status_bundle.report_gate.source -ne "strict_unattended_ledger_latest") {
    throw "strict active daemon did not use ledger-derived report gate when report JSON was absent"
}
if ($strictActive.live_status_bundle.report_gate.passed -ne $true -or $strictActive.live_status_bundle.report_gate.source_round -ne 377) {
    throw "strict active daemon ledger-derived report gate did not preserve pass evidence"
}
if ($strictActive.readiness.ready -ne $true) {
    throw "strict active daemon with ledger-derived report gate should remain ready"
}
if ($strictActive.daemon.daemon_round_transition_status.transition_kind -ne "normal_in_progress" -or $strictActive.daemon.ledger_lag_rounds -ne 1) {
    throw "strict active daemon fixture did not reproduce normal_in_progress with ledger lag 1"
}
if ($strictActive.next_round_decision.display_state -ne "safe-to-wait" -or $strictActive.next_round_decision.operator_attention_blocked -ne $false -or $strictActive.next_round_decision.reason_code -ne "active_round_in_progress_wait_for_completion") {
    throw "strict active daemon with ledger-derived report gate was incorrectly operator-attention blocked"
}
if ($strictActive.next_round_downstream_status_consumers_v1.next_round_downstream.effective_decision_status -ne "safe_to_wait_current_round_active") {
    throw "strict active daemon downstream consumers did not receive safe-to-wait status"
}
if ($strictActive.next_round_downstream_status_consumers_v1.next_round_downstream.round_id_evidence.active_round -ne 378 -or $strictActive.next_round_downstream_status_consumers_v1.next_round_downstream.round_id_evidence.ledger_latest_round -ne 377 -or $strictActive.next_round_downstream_status_consumers_v1.next_round_downstream.round_id_evidence.latest_done_round -ne 377) {
    throw "strict active daemon downstream round-id evidence did not preserve active/completed round ids"
}
Assert-NextRoundDecisionReportV1 -Report $strictActive.next_round_decision_report_v1 -Decision $strictActive.next_round_decision -Name "strict-active-ledger-gate-status"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $strictActive.next_round_downstream_status_consumers_v1 -Report $strictActive.next_round_decision_report_v1 -Name "strict-active-ledger-gate-status" -DaemonRoundTransitionStatus $strictActive.daemon.daemon_round_transition_status

$busyHealthBody = '{"ok":true,"readiness_ok":false,"safe_device_ok":true,"engine_busy":true,"active_engine_requests":1,"gemma_runtime_reachable":true,"gemma_runtime_model":"Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf"}'
$busyHealthJson = Join-Path $testDir "busy-backend-health.json"
Set-Content -Encoding ASCII -LiteralPath $busyHealthJson -Value $busyHealthBody
$busyDaemonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -BackendHealthJsonPath $busyHealthJson -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -ne 0) {
    throw "active daemon with busy backend should stay ready, got exit code $LASTEXITCODE"
}
$busyDaemonStatus = ($busyDaemonText | Out-String | ConvertFrom-Json)
if ($busyDaemonStatus.readiness.ready -ne $true) {
    throw "active daemon with busy backend was incorrectly marked not ready"
}
if ($busyDaemonStatus.readiness.backend_busy_during_active_daemon -ne $true) {
    throw "active daemon busy backend evidence flag was not exposed"
}
if ((@($busyDaemonStatus.readiness.failures) -join ",") -match "backend_not_ready") {
    throw "active daemon busy backend should not report backend_not_ready"
}
if ([string]$busyDaemonStatus.next_step -notmatch "daemon active: wait for current round") {
    throw "active daemon busy backend did not preserve daemon-prioritized next_step"
}

$ignoredLedger = Join-Path $testDir "ignored-invalid-ledger.jsonl"
Set-Content -Encoding ASCII -LiteralPath $ignoredLedger -Value @(
    '{"round":1,"case":"ignored-invalid-0001","success":true}',
    '{this is not json}'
)
$daemonLedgerJsonText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ignoredLedger -UseDaemonLedger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "status daemon ledger json command failed with exit code $LASTEXITCODE"
}
$daemonLedgerStatus = ($daemonLedgerJsonText | Out-String | ConvertFrom-Json)
if ($daemonLedgerStatus.ledger_source -ne "daemon") {
    throw "daemon ledger status did not expose ledger_source=daemon"
}
if ($daemonLedgerStatus.ledger.path -ne (Join-Path $daemonDir "evolution-ledger.jsonl")) {
    throw "daemon ledger status did not use daemon ledger path"
}
if ($daemonLedgerStatus.ledger.invalid_records -ne 0) {
    throw "daemon ledger status should ignore invalid explicit ledger when UseDaemonLedger is set"
}
if ($daemonLedgerStatus.ledger.feedback_applied_total -ne 2) {
    throw "daemon ledger status did not summarize daemon ledger feedback"
}

$strictDaemonDir = Join-Path $testDir "daemon-strict"
New-Item -ItemType Directory -Force -Path $strictDaemonDir | Out-Null
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictDaemonDir "evolution-loop.pid") -Value ([string]$PID)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictDaemonDir "evolution-ledger.jsonl") -Value '{"round":3,"case":"status-selftest-strict-0003","success":true,"runtime_tokens":21,"elapsed_ms":300,"feedback_applied":4,"self_improve_passed":true,"validation_checked":true,"validation_passed":true,"validation_command_source":"configured","validation_command_safety":"explicit","validation_status_code":0,"validation_elapsed_ms":321,"helper_stage_feedback_by_role":{"summary":["task_kind=summary preview=memory_update: keep"],"router":["task_kind=router preview=route_intent: index"],"review":["task_kind=review preview=change_request: tune"],"index":["task_kind=index preview=clean_gist: keep"],"test-gate":["task_kind=test-gate preview=verdict: pass / validation_command: cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"]},"helper_stage_contract_by_role":{"summary":{"fields":{"memory_update":"keep"}},"router":{"fields":{"route_intent":"index"}},"review":{"fields":{"change_request":"tune"}},"index":{"fields":{"clean_gist":"keep","tags":"role=index;case=status-selftest-strict-0003;round=3;primary=present;final_json=present;dependency=review.change_request;source_origin=review.change_request;validation_timestamp=1781770123","dependency_link":"review.change_request","source_origin":"review.change_request","validation_timestamp":"1781770123","retention":"keep"}},"test-gate":{"fields":{"verdict":"pass","validation_command":"cargo test -q --manifest-path tools/evolution-loop/Cargo.toml"}}}}'
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictDaemonDir "evolution-loop.out.log") -Value @(
    "[round 3] case=status-selftest-strict-0003",
    "[round 3] stage generate:start"
)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictDaemonDir "evolution-loop.err.log") -Value 'Running tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --validation-command cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --require-configured-validation-run'
$strictText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ignoredLedger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $strictDaemonDir -StrictUnattendedEvolution -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -ne 0) {
    throw "strict unattended evolution status should pass, got $LASTEXITCODE"
}
$strictStatus = ($strictText | Out-String | ConvertFrom-Json)
if ($strictStatus.strict_unattended_evolution -ne $true) {
    throw "strict unattended evolution profile flag was not exposed"
}
if ($strictStatus.ledger_source -ne "daemon") {
    throw "strict unattended evolution should imply daemon ledger"
}
if ($strictStatus.readiness.ready -ne $true) {
    throw "strict unattended evolution fixture should be ready"
}
if ($strictStatus.daemon.max_in_progress_stdout_age_seconds -ne 900 -or $strictStatus.daemon.max_idle_ledger_age_seconds -ne 900) {
    throw "strict unattended evolution default freshness thresholds were not exposed"
}
if (($strictStatus.ledger.latest.helper_stage_roles -join ",") -ne "index,review,router,summary,test-gate") {
    throw "strict unattended evolution did not expose required helper roles"
}

$strictCmdText = & $strictStatusCmd -RepoRoot $RepoRoot -Ledger $ignoredLedger -SkipBackend -SkipRemoteChain -DaemonWorkDir $strictDaemonDir
if ($LASTEXITCODE -ne 0) {
    throw "strict status cmd wrapper should pass, got $LASTEXITCODE"
}
$strictCmdStatus = ($strictCmdText | Out-String | ConvertFrom-Json)
if ($strictCmdStatus.strict_unattended_evolution -ne $true -or $strictCmdStatus.ledger_source -ne "daemon") {
    throw "strict status cmd wrapper did not enable strict daemon ledger profile"
}
if ($strictCmdStatus.readiness.ready -ne $true) {
    throw "strict status cmd wrapper fixture should be ready"
}

$strictSnapshotPath = Join-Path $testDir "strict-status-snapshot.json"
$strictSnapshotText = & $strictSnapshotCmd $strictSnapshotPath -RepoRoot $RepoRoot -Ledger $ignoredLedger -SkipBackend -SkipRemoteChain -DaemonWorkDir $strictDaemonDir
if ($LASTEXITCODE -ne 0) {
    throw "strict status snapshot wrapper should pass, got $LASTEXITCODE"
}
if (($strictSnapshotText | Out-String) -notmatch "strict_status_snapshot=") {
    throw "strict status snapshot wrapper did not print output path"
}
if (-not (Test-Path -LiteralPath $strictSnapshotPath -PathType Leaf)) {
    throw "strict status snapshot wrapper did not write JSON file"
}
$strictSnapshotStatus = Get-Content -LiteralPath $strictSnapshotPath -Raw | ConvertFrom-Json
if ($strictSnapshotStatus.strict_unattended_evolution -ne $true -or $strictSnapshotStatus.ledger_source -ne "daemon") {
    throw "strict status snapshot wrapper did not write strict daemon ledger status"
}
if ($strictSnapshotStatus.readiness.ready -ne $true) {
    throw "strict status snapshot wrapper fixture should be ready"
}
$strictSnapshotTransition = $strictSnapshotStatus.daemon.daemon_round_transition_status
Assert-NextRoundDecisionReportV1 -Report $strictSnapshotStatus.next_round_decision_report_v1 -Decision $strictSnapshotStatus.next_round_decision -Name "strict-snapshot-status"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $strictSnapshotStatus.next_round_downstream_status_consumers_v1 -Report $strictSnapshotStatus.next_round_decision_report_v1 -Name "strict-snapshot-status" -DaemonRoundTransitionStatus $strictSnapshotTransition

$strictSnapshotVerifyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $verifyStrictSnapshotScript -RepoRoot $RepoRoot -SnapshotJson $strictSnapshotPath -MaxSnapshotAgeSeconds 3600 -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -ne 0) {
    throw "strict status snapshot verifier should pass, got $LASTEXITCODE"
}
$strictSnapshotVerify = ($strictSnapshotVerifyText | Out-String | ConvertFrom-Json)
if ($strictSnapshotVerify.readiness.ready -ne $true -or $strictSnapshotVerify.strict_unattended_evolution -ne $true -or $strictSnapshotVerify.ledger_source -ne "daemon") {
    throw "strict status snapshot verifier did not expose ready strict daemon snapshot"
}
if ($strictSnapshotVerify.latest_round -ne 3 -or $strictSnapshotVerify.daemon_state -ne "active") {
    throw "strict status snapshot verifier did not summarize latest round and daemon state"
}
if ($strictSnapshotVerify.summary.latest_case -ne "status-selftest-strict-0003") {
    throw "strict status snapshot verifier did not expose latest case summary"
}
if ($strictSnapshotVerify.summary.self_improve_passed -ne $true -or $strictSnapshotVerify.summary.validation_passed -ne $true) {
    throw "strict status snapshot verifier did not expose self-improve and validation summary"
}
if (($strictSnapshotVerify.summary.helper_stage_roles -join ",") -ne "index,review,router,summary,test-gate") {
    throw "strict status snapshot verifier did not expose helper role summary"
}
if ($strictSnapshotVerify.summary.helper_stage_contract_complete -ne $true -or $strictSnapshotVerify.summary.test_gate_passed -ne $true) {
    throw "strict status snapshot verifier did not expose helper contract and test-gate summary"
}
if ($strictSnapshotVerify.summary.test_gate_validation_command_safety -ne "safe") {
    throw "strict status snapshot verifier did not expose test-gate validation safety summary"
}
Assert-NextRoundDecisionReportV1 -Report $strictSnapshotVerify.next_round_decision_report_v1 -Decision $strictSnapshotVerify.next_round_decision -Name "strict-snapshot-verify"
Assert-NextRoundDecisionReportV1 -Report $strictSnapshotVerify.summary.next_round_decision_report_v1 -Decision $strictSnapshotVerify.next_round_decision -Name "strict-snapshot-verify-summary"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $strictSnapshotVerify.next_round_downstream_status_consumers_v1 -Report $strictSnapshotVerify.next_round_decision_report_v1 -Name "strict-snapshot-verify" -DaemonRoundTransitionStatus $strictSnapshotTransition
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $strictSnapshotVerify.summary.next_round_downstream_status_consumers_v1 -Report $strictSnapshotVerify.summary.next_round_decision_report_v1 -Name "strict-snapshot-verify-summary" -DaemonRoundTransitionStatus $strictSnapshotTransition

$strictSummaryPath = Join-Path $testDir "strict-status-summary.json"
$strictSummaryText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $publishStrictSummaryScript -RepoRoot $RepoRoot -SnapshotJson $strictSnapshotPath -OutJson $strictSummaryPath -MaxSnapshotAgeSeconds 3600 -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -ne 0) {
    throw "strict status summary publisher should pass, got $LASTEXITCODE"
}
if (-not (Test-Path -LiteralPath $strictSummaryPath -PathType Leaf)) {
    throw "strict status summary publisher did not write JSON file"
}
$strictSummaryStatus = ($strictSummaryText | Out-String | ConvertFrom-Json)
$strictSummaryFile = Get-Content -LiteralPath $strictSummaryPath -Raw | ConvertFrom-Json
if ($strictSummaryStatus.readiness.ready -ne $true -or $strictSummaryFile.readiness.ready -ne $true) {
    throw "strict status summary publisher should expose ready status"
}
if ($strictSummaryFile.summary.latest_round -ne 3 -or $strictSummaryFile.summary.latest_case -ne "status-selftest-strict-0003") {
    throw "strict status summary publisher did not preserve latest round/case"
}
if (($strictSummaryFile.summary.helper_stage_roles -join ",") -ne "index,review,router,summary,test-gate") {
    throw "strict status summary publisher did not preserve helper roles"
}
if ($strictSummaryFile.summary.test_gate_passed -ne $true -or $strictSummaryFile.summary.test_gate_validation_command_safety -ne "safe") {
    throw "strict status summary publisher did not preserve test-gate summary"
}
Assert-NextRoundDecisionReportV1 -Report $strictSummaryFile.summary.next_round_decision_report_v1 -Decision $strictSnapshotStatus.next_round_decision -Name "strict-summary-file"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $strictSummaryFile.summary.next_round_downstream_status_consumers_v1 -Report $strictSummaryFile.summary.next_round_decision_report_v1 -Name "strict-summary-file" -DaemonRoundTransitionStatus $strictSnapshotTransition

$strictSummaryVerifyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $verifyStrictSummaryScript -RepoRoot $RepoRoot -SummaryJson $strictSummaryPath -MaxSummaryAgeSeconds 3600 -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -ne 0) {
    throw "strict status summary verifier should pass, got $LASTEXITCODE"
}
$strictSummaryVerify = ($strictSummaryVerifyText | Out-String | ConvertFrom-Json)
if ($strictSummaryVerify.readiness.ready -ne $true -or $strictSummaryVerify.summary.latest_round -ne 3) {
    throw "strict status summary verifier did not expose ready summary"
}
if (($strictSummaryVerify.summary.helper_stage_roles -join ",") -ne "index,review,router,summary,test-gate") {
    throw "strict status summary verifier did not expose helper roles"
}
if ($strictSummaryVerify.summary.test_gate_passed -ne $true -or $strictSummaryVerify.summary.test_gate_validation_command_safety -ne "safe") {
    throw "strict status summary verifier did not expose test-gate safety"
}
Assert-NextRoundDecisionReportV1 -Report $strictSummaryVerify.summary.next_round_decision_report_v1 -Decision $strictSnapshotStatus.next_round_decision -Name "strict-summary-verify"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $strictSummaryVerify.summary.next_round_downstream_status_consumers_v1 -Report $strictSummaryVerify.summary.next_round_decision_report_v1 -Name "strict-summary-verify" -DaemonRoundTransitionStatus $strictSnapshotTransition

$staleStrictSummaryPath = Join-Path $testDir "strict-status-summary-stale.json"
Copy-Item -LiteralPath $strictSummaryPath -Destination $staleStrictSummaryPath -Force
(Get-Item -LiteralPath $staleStrictSummaryPath).LastWriteTime = (Get-Date).AddMinutes(-20)
$staleStrictSummaryVerifyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $verifyStrictSummaryScript -RepoRoot $RepoRoot -SummaryJson $staleStrictSummaryPath -MaxSummaryAgeSeconds 60 -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "stale strict status summary verifier should exit nonzero"
}
$staleStrictSummaryVerify = ($staleStrictSummaryVerifyText | Out-String | ConvertFrom-Json)
if ($staleStrictSummaryVerify.readiness.ready -ne $false) {
    throw "stale strict status summary verifier should be not-ready"
}
if ((@($staleStrictSummaryVerify.readiness.failures) -join ",") -notmatch "summary_stale") {
    throw "stale strict status summary verifier did not report summary_stale"
}

$refreshSnapshotPath = Join-Path $testDir "strict-refresh-status.json"
$refreshSummaryPath = Join-Path $testDir "strict-refresh-summary.json"
$refreshText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $refreshStrictArtifactsScript -RepoRoot $RepoRoot -SnapshotJson $refreshSnapshotPath -SummaryJson $refreshSummaryPath -Ledger $ignoredLedger -DaemonWorkDir $strictDaemonDir -SkipBackend -SkipRemoteChain -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -ne 0) {
    throw "strict status artifacts refresh should pass, got $LASTEXITCODE"
}
$refresh = ($refreshText | Out-String | ConvertFrom-Json)
if ($refresh.readiness.ready -ne $true) {
    throw "strict status artifacts refresh should be ready"
}
if (-not (Test-Path -LiteralPath $refreshSnapshotPath -PathType Leaf) -or -not (Test-Path -LiteralPath $refreshSummaryPath -PathType Leaf)) {
    throw "strict status artifacts refresh did not write both artifacts"
}
$refreshSummary = Get-Content -LiteralPath $refreshSummaryPath -Raw | ConvertFrom-Json
if ($refreshSummary.summary.latest_round -ne 3 -or $refreshSummary.summary.test_gate_validation_command_safety -ne "safe") {
    throw "strict status artifacts refresh did not publish compact summary"
}
if ([string]$refreshSummary.summary.next_round_decision_display_state -eq "" -or $null -eq $refreshSummary.summary.operator_attention_blocked) {
    throw "strict status artifacts refresh did not publish next-round decision summary"
}
Assert-NextRoundDecisionReportV1 -Report $refreshSummary.summary.next_round_decision_report_v1 -Decision $strictSnapshotStatus.next_round_decision -Name "strict-refresh-summary"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $refreshSummary.summary.next_round_downstream_status_consumers_v1 -Report $refreshSummary.summary.next_round_decision_report_v1 -Name "strict-refresh-summary" -DaemonRoundTransitionStatus $strictSnapshotTransition

$staleStrictSnapshotPath = Join-Path $testDir "strict-status-snapshot-stale.json"
Copy-Item -LiteralPath $strictSnapshotPath -Destination $staleStrictSnapshotPath -Force
(Get-Item -LiteralPath $staleStrictSnapshotPath).LastWriteTime = (Get-Date).AddMinutes(-20)
$staleStrictSnapshotVerifyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $verifyStrictSnapshotScript -RepoRoot $RepoRoot -SnapshotJson $staleStrictSnapshotPath -MaxSnapshotAgeSeconds 60 -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "stale strict status snapshot verifier should exit nonzero"
}
$staleStrictSnapshotVerify = ($staleStrictSnapshotVerifyText | Out-String | ConvertFrom-Json)
if ($staleStrictSnapshotVerify.readiness.ready -ne $false) {
    throw "stale strict status snapshot verifier should be not-ready"
}
if ((@($staleStrictSnapshotVerify.readiness.failures) -join ",") -notmatch "snapshot_stale") {
    throw "stale strict status snapshot verifier did not report snapshot_stale"
}

$strictMissingDir = Join-Path $testDir "daemon-strict-missing"
New-Item -ItemType Directory -Force -Path $strictMissingDir | Out-Null
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictMissingDir "evolution-loop.pid") -Value ([string]$PID)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictMissingDir "evolution-ledger.jsonl") -Value '{"round":4,"case":"status-selftest-strict-missing-0004","success":true,"runtime_tokens":21,"elapsed_ms":300,"feedback_applied":4,"self_improve_passed":true,"validation_checked":true,"validation_passed":true,"validation_command_source":"configured","validation_command_safety":"explicit","validation_status_code":0,"validation_elapsed_ms":321}'
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictMissingDir "evolution-loop.out.log") -Value @(
    "[round 4] case=status-selftest-strict-missing-0004",
    "[round 4] stage generate:start"
)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $strictMissingDir "evolution-loop.err.log") -Value 'Running tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --validation-command cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --require-configured-validation-run'
$strictMissingText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ignoredLedger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $strictMissingDir -StrictUnattendedEvolution -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "strict unattended evolution missing helper/test-gate evidence should exit nonzero"
}
$strictMissingStatus = ($strictMissingText | Out-String | ConvertFrom-Json)
if ($strictMissingStatus.readiness.ready -ne $false) {
    throw "strict unattended evolution missing helper/test-gate evidence should be not-ready"
}
$strictMissingFailures = @($strictMissingStatus.readiness.failures) -join ","
if ($strictMissingFailures -notmatch "latest_helper_stage_roles_missing" -or $strictMissingFailures -notmatch "latest_helper_stage_contract_incomplete" -or $strictMissingFailures -notmatch "latest_test_gate_not_pass" -or $strictMissingFailures -notmatch "latest_test_gate_validation_command_not_safe") {
    throw "strict unattended evolution missing helper/test-gate evidence did not report expected failures: $strictMissingFailures"
}

$daemonNativeText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $daemonScript -JsonStatus -WorkDir $daemonDir -Ledger (Join-Path $daemonDir "evolution-ledger.jsonl") -SkipBackend -SkipRemoteChain 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "daemon native json command failed with exit code $LASTEXITCODE"
}
$daemonNative = ($daemonNativeText | Out-String | ConvertFrom-Json)
if ($daemonNative.daemon.active_round -ne $daemonStatus.daemon.active_round) {
    throw "status daemon snapshot active round diverged from daemon wrapper"
}
if ($daemonNative.daemon.ledger_latest_round -ne $daemonStatus.daemon.ledger_latest_round) {
    throw "status daemon snapshot ledger round diverged from daemon wrapper"
}
if ($daemonNative.daemon.ledger_lag_rounds -ne $daemonStatus.daemon.ledger_lag_rounds) {
    throw "status daemon snapshot ledger lag diverged from daemon wrapper"
}
if ($daemonNative.daemon.activity.state -ne $daemonStatus.daemon.activity_state) {
    throw "status daemon snapshot activity state diverged from daemon wrapper"
}

$daemonHumanText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir
if ($LASTEXITCODE -ne 0) {
    throw "status daemon human command failed with exit code $LASTEXITCODE"
}
$daemonHuman = ($daemonHumanText | Out-String)
if ($daemonHuman -notmatch "daemon: running=True state=active") {
    throw "human status did not print daemon operator summary"
}
if ($daemonHuman -notmatch "daemon_transition: schema=daemon_round_transition_status_v1 kind=normal_in_progress") {
    throw "human status did not print daemon transition summary"
}
if ($daemonHuman -notmatch "next_step: daemon active: wait for current round") {
    throw "human status did not print daemon-prioritized next_step"
}

$daemonHealthyText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir -RequireDaemonHealthy -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "status daemon healthy gate command failed with exit code $LASTEXITCODE"
}
$daemonHealthy = ($daemonHealthyText | Out-String | ConvertFrom-Json)
if ($daemonHealthy.readiness.ready -ne $true) {
    throw "daemon healthy gate should pass for active fixture"
}

$daemonValidationText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir -RequireDaemonValidationExecution -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "status daemon validation execution gate command failed with exit code $LASTEXITCODE"
}
$daemonValidation = ($daemonValidationText | Out-String | ConvertFrom-Json)
if ($daemonValidation.readiness.ready -ne $true) {
    throw "daemon validation execution gate should pass for configured launch fixture"
}
if ($daemonValidation.daemon.launch_validation.mode -ne "configured") {
    throw "daemon validation execution gate did not parse configured launch mode"
}
if ($daemonValidation.daemon.launch_validation.validation_execution_enforced -ne $true) {
    throw "daemon validation execution gate did not expose enforced validation"
}
if ($daemonValidation.daemon.validation_execution_ok -ne $true) {
    throw "daemon validation execution gate should be ok for configured launch fixture"
}

$daemonValidationHumanText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir -RequireDaemonValidationExecution
if ($LASTEXITCODE -ne 0) {
    throw "status daemon validation execution human command failed with exit code $LASTEXITCODE"
}
$daemonValidationHuman = ($daemonValidationHumanText | Out-String)
if ($daemonValidationHuman -notmatch "launch_validation: mode=configured enforced=True") {
    throw "human status did not print configured launch validation summary"
}
if ($daemonValidationHuman -notmatch "daemon_validation_execution_gate: required=True ok=True") {
    throw "human status did not print daemon validation execution gate"
}

Set-Content -Encoding ASCII -LiteralPath (Join-Path $daemonDir "evolution-loop.err.log") -Value 'Running tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --require-test-gate-pass'
$daemonValidationMissingText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir -RequireDaemonValidationExecution -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "daemon without validation execution and FailOnNotReady should exit nonzero"
}
$daemonValidationMissing = ($daemonValidationMissingText | Out-String | ConvertFrom-Json)
if ($daemonValidationMissing.readiness.ready -ne $false) {
    throw "daemon without validation execution should be not-ready"
}
if ((@($daemonValidationMissing.readiness.failures) -join ",") -notmatch "daemon_validation_execution_missing") {
    throw "daemon validation execution gate did not report daemon_validation_execution_missing"
}
if ($daemonValidationMissing.daemon.launch_validation.validation_execution_enforced -ne $false) {
    throw "daemon validation execution missing fixture should not expose enforced validation"
}
if ([string]$daemonValidationMissing.daemon.validation_execution_failure -notmatch "does not enforce validation execution") {
    throw "daemon validation execution missing fixture did not expose failure detail"
}

(Get-Item -LiteralPath (Join-Path $daemonDir "evolution-loop.out.log")).LastWriteTime = (Get-Date).AddMinutes(-20)
$daemonRelaxedFreshnessText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir -RequireDaemonHealthy -MaxDaemonInProgressStdoutAgeSeconds 2000 -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "status daemon relaxed freshness gate command failed with exit code $LASTEXITCODE"
}
$daemonRelaxedFreshness = ($daemonRelaxedFreshnessText | Out-String | ConvertFrom-Json)
if ($daemonRelaxedFreshness.readiness.ready -ne $true) {
    throw "relaxed daemon in-progress stdout freshness gate should pass"
}
if ($daemonRelaxedFreshness.daemon.activity_state -ne "active") {
    throw "relaxed daemon in-progress stdout freshness gate should keep fixture active"
}
if ($daemonRelaxedFreshness.daemon.max_in_progress_stdout_age_seconds -ne 2000) {
    throw "relaxed daemon in-progress stdout freshness threshold was not exposed"
}

$daemonStaleText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir -RequireDaemonHealthy -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "status daemon stale gate command failed with exit code $LASTEXITCODE"
}
$daemonStale = ($daemonStaleText | Out-String | ConvertFrom-Json)
if ($daemonStale.daemon.activity_state -ne "stale_in_progress") {
    throw "daemon stale gate fixture did not become stale_in_progress"
}
if ($daemonStale.daemon.activity_reason -ne "round_in_progress_stdout_stale") {
    throw "daemon stale gate did not expose stale reason"
}
if ($daemonStale.daemon.daemon_round_transition_status.transition_kind -ne "stale_no_activity") {
    throw "daemon stale gate did not expose stale_no_activity transition kind"
}
if ($daemonStale.daemon.daemon_round_transition_status.activity_reason -ne "round_in_progress_stdout_stale") {
    throw "daemon stale transition did not expose stale reason"
}
if ($daemonStale.readiness.ready -ne $false) {
    throw "daemon healthy gate should fail for stale fixture"
}
$daemonStaleFailures = @($daemonStale.readiness.failures)
if (($daemonStaleFailures -join ",") -notmatch "daemon_not_healthy") {
    throw "daemon healthy gate did not report daemon_not_healthy"
}

$daemonStaleFailText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir -RequireDaemonHealthy -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "stale daemon with FailOnNotReady should exit nonzero"
}
$daemonStaleFail = ($daemonStaleFailText | Out-String | ConvertFrom-Json)
if ($daemonStaleFail.readiness.ready -ne $false) {
    throw "stale daemon with FailOnNotReady should still print not-ready JSON"
}
if ((@($daemonStaleFail.readiness.failures) -join ",") -notmatch "daemon_not_healthy") {
    throw "stale daemon with FailOnNotReady did not print daemon_not_healthy"
}
if ($daemonStaleFail.read_only -ne $true -or $daemonStaleFail.starts_process -ne $false -or $daemonStaleFail.sends_prompt -ne $false) {
    throw "stale daemon with FailOnNotReady broke read-only contract"
}

$daemonStaleFailHumanText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $daemonDir -RequireDaemonHealthy -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "stale daemon human status with FailOnNotReady should exit nonzero"
}
$daemonStaleFailHuman = ($daemonStaleFailHumanText | Out-String)
if ($daemonStaleFailHuman -notmatch "read_only=true starts_process=false sends_prompt=false") {
    throw "stale daemon human status with FailOnNotReady did not print read-only contract"
}
if ($daemonStaleFailHuman -notmatch "status: ready=False failures=daemon_not_healthy") {
    throw "stale daemon human status with FailOnNotReady did not print readiness failure"
}

$idleDaemonDir = Join-Path $testDir "daemon-idle"
New-Item -ItemType Directory -Force -Path $idleDaemonDir | Out-Null
Set-Content -Encoding ASCII -LiteralPath (Join-Path $idleDaemonDir "evolution-loop.pid") -Value ([string]$PID)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $idleDaemonDir "evolution-ledger.jsonl") -Value '{"round":3,"case":"status-selftest-idle-0003","success":true,"runtime_tokens":21,"elapsed_ms":300,"feedback_applied":3,"self_improve_passed":true}'
Set-Content -Encoding ASCII -LiteralPath (Join-Path $idleDaemonDir "evolution-loop.out.log") -Value @(
    "[round 3] case=status-selftest-idle-0003",
    "[round 3] stage ledger_append:done",
    "[round 3] ok runtime_tokens=21 elapsed_ms=300"
)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $idleDaemonDir "evolution-loop.err.log") -Value 'Running tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --validation-command cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --require-configured-validation-run'
(Get-Item -LiteralPath (Join-Path $idleDaemonDir "evolution-ledger.jsonl")).LastWriteTime = (Get-Date).AddMinutes(-20)

$idleDefaultText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -ReportJson $passedReport -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $idleDaemonDir -RequireDaemonHealthy -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "idle daemon default freshness gate command failed with exit code $LASTEXITCODE"
}
$idleDefault = ($idleDefaultText | Out-String | ConvertFrom-Json)
if ($idleDefault.readiness.ready -ne $true -or $idleDefault.daemon.activity_state -ne "idle_completed") {
    throw "idle daemon default freshness should stay ready when idle ledger threshold is disabled"
}
if ($idleDefault.next_round_decision.display_state -ne "safe-to-continue-after-current-round" -or $idleDefault.next_round_decision.reason_code -ne "idle_completed_report_gate_passed_ready_for_next_round" -or $idleDefault.next_round_decision.operator_attention_blocked -ne $false) {
    throw "idle daemon with passed report gate should be safe to continue without operator attention"
}

$postRoundDaemonDir = Join-Path $testDir "daemon-post-round"
New-Item -ItemType Directory -Force -Path $postRoundDaemonDir | Out-Null
Set-Content -Encoding ASCII -LiteralPath (Join-Path $postRoundDaemonDir "evolution-loop.pid") -Value ([string]$PID)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $postRoundDaemonDir "evolution-ledger.jsonl") -Value '{"round":4,"case":"status-selftest-post-round-0004","success":true,"runtime_tokens":21,"elapsed_ms":300,"feedback_applied":3,"self_improve_passed":true}'
Set-Content -Encoding ASCII -LiteralPath (Join-Path $postRoundDaemonDir "evolution-loop.out.log") -Value @(
    "[round 4] case=status-selftest-post-round-0004",
    "[round 4] stage ledger_append:done",
    "[round 4] ok runtime_tokens=21 elapsed_ms=300",
    "remote_chain_gate: passed ready:true",
    "experience_audit_gate: start endpoint=/v1/experience-cleanup-audit limit=25 timeout_secs=300"
)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $postRoundDaemonDir "evolution-loop.err.log") -Value 'Running tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --validation-command cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --require-configured-validation-run'

$postRoundText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -ReportJson $passedReport -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $postRoundDaemonDir -RequireDaemonHealthy -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "post-round activity daemon status command failed with exit code $LASTEXITCODE"
}
$postRound = ($postRoundText | Out-String | ConvertFrom-Json)
if ($postRound.readiness.ready -ne $true -or $postRound.daemon.activity_state -ne "post_round_activity") {
    throw "post-round gate activity should be healthy but not reported as idle_completed"
}
if ($postRound.daemon.post_round_activity -ne $true) {
    throw "post-round gate activity flag was not exposed"
}
if ([string]$postRound.daemon.post_round_activity_line_preview -notmatch "experience_audit_gate: start") {
    throw "post-round gate activity preview did not preserve latest gate line"
}
if ($postRound.next_round_decision.display_state -ne "safe-to-continue-after-current-round" -or $postRound.next_round_decision.reason_code -ne "post_round_activity_report_gate_passed_ready_for_next_round" -or $postRound.next_round_decision.operator_attention_blocked -ne $false) {
    throw "post-round activity with passed report gate should not be operator-attention blocked"
}

$doneLagDaemonDir = Join-Path $testDir "daemon-done-ledger-lag"
New-Item -ItemType Directory -Force -Path $doneLagDaemonDir | Out-Null
Set-Content -Encoding ASCII -LiteralPath (Join-Path $doneLagDaemonDir "evolution-loop.pid") -Value ([string]$PID)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $doneLagDaemonDir "evolution-ledger.jsonl") -Value '{"round":5,"case":"status-selftest-done-lag-0005","success":true,"runtime_tokens":21,"elapsed_ms":300,"feedback_applied":3,"self_improve_passed":true}'
Set-Content -Encoding ASCII -LiteralPath (Join-Path $doneLagDaemonDir "evolution-loop.out.log") -Value @(
    "[round 5] case=status-selftest-done-lag-0005",
    "[round 5] stage ledger_append:done",
    "[round 5] ok runtime_tokens=21 elapsed_ms=300",
    "[round 6] case=status-selftest-done-lag-0006",
    "[round 6] stage report_gate:done",
    "[round 6] done [DONE]"
)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $doneLagDaemonDir "evolution-loop.err.log") -Value 'Running tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --validation-command cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --require-configured-validation-run'

$doneLagText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -ReportJson $passedReport -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $doneLagDaemonDir -RequireDaemonHealthy -JsonStatus
if ($LASTEXITCODE -ne 0) {
    throw "done-marker ledger-lag daemon status command failed with exit code $LASTEXITCODE"
}
$doneLag = ($doneLagText | Out-String | ConvertFrom-Json)
if ($doneLag.daemon.latest_round_state -ne "round_done_waiting_ledger_commit") {
    throw "done-marker ledger lag should not be reported as latest_round_state=in_progress"
}
if ($doneLag.daemon.activity_state -ne "round_done_waiting_ledger_commit") {
    throw "done-marker ledger lag did not expose waiting-for-ledger activity state"
}
if ($doneLag.daemon.ledger_lag_rounds -ne 1 -or $doneLag.daemon.latest_done_round -ne 6) {
    throw "done-marker ledger lag did not expose lag and latest_done_round"
}
if ($doneLag.daemon.round_in_progress -ne $false) {
    throw "done-marker ledger lag should not expose round_in_progress=true"
}
if ($doneLag.daemon.activity_reason -ne "stdout_done_marker_seen_waiting_for_ledger_commit") {
    throw "done-marker ledger lag did not expose activity reason"
}
if ($doneLag.daemon.daemon_round_transition_status.transition_kind -ne "round_done_waiting_ledger_commit") {
    throw "done-marker ledger lag did not expose transition kind"
}
if ($doneLag.daemon.daemon_round_transition_status.latest_done_round -ne 6 -or $doneLag.daemon.daemon_round_transition_status.round_in_progress -ne $false) {
    throw "done-marker ledger lag transition did not expose done round and non-progress state"
}
Assert-DaemonRoundTransitionConsumerStatus -Status $doneLag.daemon.daemon_round_transition_status -Name "status-json:round_done_waiting_ledger_commit" -ExpectedKind "round_done_waiting_ledger_commit" -ExpectedRoundInProgress $false
if ($doneLag.next_round_decision.display_state -ne "safe-to-continue-after-current-round" -or $doneLag.next_round_decision.safe_to_wait_current_round_active -ne $false -or $doneLag.next_round_decision.safe_to_continue_after_current_round -ne $true -or $doneLag.next_round_decision.operator_attention_blocked -ne $false) {
    throw "done-marker ledger lag with passed report gate did not expose safe-to-continue next-round decision"
}
Assert-NextRoundDecisionReportV1 -Report $doneLag.next_round_decision_report_v1 -Decision $doneLag.next_round_decision -Name "done-lag-status"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $doneLag.next_round_downstream_status_consumers_v1 -Report $doneLag.next_round_decision_report_v1 -Name "done-lag-status" -DaemonRoundTransitionStatus $doneLag.daemon.daemon_round_transition_status
if ($doneLag.live_status_bundle.daemon.daemon_round_transition_status.transition_kind -ne "round_done_waiting_ledger_commit" -or $doneLag.live_status_bundle.report_gate.passed -ne $true) {
    throw "done-marker live status bundle did not carry transition and report-gate evidence"
}
Assert-NextRoundDecisionReportV1 -Report $doneLag.live_status_bundle.next_round_decision_report_v1 -Decision $doneLag.next_round_decision -Name "done-lag-live-bundle"
Assert-NextRoundDownstreamStatusConsumersV1 -Projection $doneLag.live_status_bundle.next_round_downstream_status_consumers_v1 -Report $doneLag.live_status_bundle.next_round_decision_report_v1 -Name "done-lag-live-bundle" -DaemonRoundTransitionStatus $doneLag.live_status_bundle.daemon.daemon_round_transition_status

$stalePostRoundDaemonDir = Join-Path $testDir "daemon-post-round-stale"
New-Item -ItemType Directory -Force -Path $stalePostRoundDaemonDir | Out-Null
Set-Content -Encoding ASCII -LiteralPath (Join-Path $stalePostRoundDaemonDir "evolution-loop.pid") -Value ([string]$PID)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $stalePostRoundDaemonDir "evolution-ledger.jsonl") -Value '{"round":5,"case":"status-selftest-post-round-stale-0005","success":true,"runtime_tokens":21,"elapsed_ms":300,"feedback_applied":3,"self_improve_passed":true}'
Set-Content -Encoding ASCII -LiteralPath (Join-Path $stalePostRoundDaemonDir "evolution-loop.out.log") -Value @(
    "[round 5] case=status-selftest-post-round-stale-0005",
    "[round 5] stage ledger_append:done",
    "[round 5] ok runtime_tokens=21 elapsed_ms=300",
    "experience_audit_gate: start endpoint=/v1/experience-cleanup-audit limit=25 timeout_secs=300"
)
Set-Content -Encoding ASCII -LiteralPath (Join-Path $stalePostRoundDaemonDir "evolution-loop.err.log") -Value 'Running tools\evolution-loop\target\debug\evolution-loop.exe --backend 127.0.0.1:7979 --validation-command cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --require-configured-validation-run'
(Get-Item -LiteralPath (Join-Path $stalePostRoundDaemonDir "evolution-loop.out.log")).LastWriteTime = (Get-Date).AddMinutes(-20)

$stalePostRoundText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $stalePostRoundDaemonDir -RequireDaemonHealthy -MaxDaemonInProgressStdoutAgeSeconds 60 -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "stale post-round activity with FailOnNotReady should exit nonzero"
}
$stalePostRound = ($stalePostRoundText | Out-String | ConvertFrom-Json)
if ($stalePostRound.daemon.activity_state -ne "stale_post_round_activity") {
    throw "stale post-round activity did not become stale_post_round_activity"
}
if ($stalePostRound.readiness.ready -ne $false) {
    throw "stale post-round activity should be not-ready"
}
if ((@($stalePostRound.readiness.failures) -join ",") -notmatch "daemon_not_healthy") {
    throw "stale post-round activity did not report daemon_not_healthy"
}

$idleStaleText = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $statusScript -RepoRoot $RepoRoot -Ledger $ledger -SkipBackend -SkipRemoteChain -SkipProcess -DaemonWorkDir $idleDaemonDir -RequireDaemonHealthy -MaxDaemonIdleLedgerAgeSeconds 60 -JsonStatus -FailOnNotReady
if ($LASTEXITCODE -eq 0) {
    throw "idle daemon stale ledger with FailOnNotReady should exit nonzero"
}
$idleStale = ($idleStaleText | Out-String | ConvertFrom-Json)
if ($idleStale.daemon.activity_state -ne "stale_idle_completed") {
    throw "idle daemon stale ledger did not become stale_idle_completed"
}
if ($idleStale.daemon.daemon_round_transition_status.transition_kind -ne "stale_no_activity") {
    throw "idle daemon stale ledger did not expose stale_no_activity transition kind"
}
if ($idleStale.readiness.ready -ne $false) {
    throw "idle daemon stale ledger should be not-ready"
}
if ((@($idleStale.readiness.failures) -join ",") -notmatch "daemon_not_healthy") {
    throw "idle daemon stale ledger did not report daemon_not_healthy"
}
if ($idleStale.daemon.max_idle_ledger_age_seconds -ne 60) {
    throw "idle daemon stale ledger threshold was not exposed"
}

Write-Host "evolution_loop_status_selftest=PASS"
Write-Host "read_only=true"
Write-Host "starts_process=false"
Write-Host "sends_prompt=false"
