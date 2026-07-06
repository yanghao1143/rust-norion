use super::*;
use crate::kv_quant::QuantizationBits;

fn approved_experience_reuse_digest() -> Option<String> {
    Some("redaction-digest:approved-experience".to_owned())
}

#[test]
fn kv_quant_benchmark_default_gate_passes() {
    let summary = KvQuantBenchmarkSummary::run_default();
    let report = summary.evaluate(&KvQuantBenchmarkGate::default());

    assert_eq!(summary.len(), 6);
    assert!(summary.max_abs_error_for(QuantizationBits::Four) > 0.0);
    assert!(summary.max_abs_error_for(QuantizationBits::Eight) > 0.0);
    assert!(report.passed, "{:?}", report.failures);
    assert!(summary.summary_line().contains("kv_quant_benchmark"));
    assert!(report.summary_line().contains("passed=true"));
}

#[test]
fn kv_quant_gate_reports_accuracy_and_compression_failures() {
    let mut summary = KvQuantBenchmarkSummary::default();
    summary.record("wide", QuantizationBits::Four, &[-1.0, 0.0, 1.0]);
    let gate = KvQuantBenchmarkGate {
        max_four_bit_abs_error: 0.0,
        max_four_bit_mean_error: 0.0,
        max_four_bit_compression_ratio: 0.01,
        max_eight_bit_abs_error: 1.0,
        max_eight_bit_mean_error: 1.0,
        max_eight_bit_compression_ratio: 1.0,
        max_total_elapsed_us: None,
    };

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("q4_max_abs_error"))
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("q4_compression_ratio"))
    );
}

#[test]
fn persistent_roundtrip_report_requires_reuse_and_runtime_kv_import() {
    let report = PersistentRoundtripReport::evaluate(PersistentRoundtripInput {
        first_stored_memory: true,
        first_runtime_kv_stored: 1,
        first_runtime_kv_namespace_preserved: true,
        first_disk_kv_reopen_verified: true,
        second_used_memories: 2,
        second_used_runtime_kv_memory: true,
        second_used_experiences: 1,
        second_approved_experience_reuse_digest: approved_experience_reuse_digest(),
        second_imported_runtime_kv_blocks: 2,
        second_imported_runtime_kv_from_namespace: true,
        second_runtime_kv_disk_rehydrated: true,
        second_kvswap_boundary_verified: true,
        second_runtime_adapter_observations: 1,
        second_runtime_adapter_best_score: Some(0.84),
        second_runtime_adapter_best_adapter: Some("portable-rust".to_owned()),
        second_runtime_selected_adapter: Some("portable-rust".to_owned()),
        second_compute_budget_saved_tokens: 32,
        second_compute_budget_avoided_tokens: 48,
        second_compute_budget_kv_lookups_skipped: 1,
        second_compute_budget_anchor_count: 1,
        second_compute_budget_anchors_preserved: true,
        second_compute_budget_anchors_preserved_count: 1,
        second_quality: 0.82,
        first_drift_severity: DriftSeverity::Watch,
        second_drift_severity: DriftSeverity::Stable,
    });

    assert!(report.passed);
    assert_eq!(
        report.negative_gate_evidence,
        issue30_roundtrip_negative_gate_evidence()
    );
    assert!(report.negative_gate_evidence.passed());
    assert!(report.summary_line().contains("passed=true"));
    assert!(
        report
            .summary_line()
            .contains("second_runtime_adapter_observations=1")
    );
    assert!(
        report
            .summary_line()
            .contains("second_approved_experience_reuse_digest=redaction-digest:")
    );
    assert!(
        report
            .summary_line()
            .contains("second_imported_runtime_kv_from_namespace=true")
    );
    assert!(
        report
            .summary_line()
            .contains("second_compute_budget_avoided_tokens=48")
    );
    for marker in [
        "negative_unauthorized_write_allowed=false",
        "negative_polluted_evidence_blocked=true",
        "negative_polluted_evidence_quarantined=true",
        "negative_bad_candidate_held_or_rolled_back=true",
        "negative_bad_candidate_digest=redaction-digest:",
        "negative_bad_candidate_decision=hold_then_rollback",
        "negative_rollback_anchor_present=true",
        "negative_rollback_anchor_evidence_id=issue-30-roundtrip-negative-gate-hold",
        "negative_rollback_anchor_digest=redaction-digest:",
        "negative_tenant_scope_write_denied=true",
        "negative_tenant_scope_mode=local_single_user_preview",
        "negative_tenant_scope_actor=fnv64:",
        "negative_tenant_scope_target=fnv64:",
        "negative_tenant_scope_denial_lane=self_evolving_memory",
        "negative_tenant_scope_denial_reason=cross_tenant_scope_rejected",
        "negative_single_tenant_preview=true",
        "negative_provenance_license_redaction_passed=true",
        "negative_digest_only=true",
        "first_disk_kv_reopen_verified=true",
        "second_runtime_kv_disk_rehydrated=true",
        "second_kvswap_boundary_verified=true",
    ] {
        assert!(report.summary_line().contains(marker), "{marker}");
    }

    let failed = PersistentRoundtripReport::evaluate(PersistentRoundtripInput {
        first_stored_memory: false,
        first_runtime_kv_stored: 0,
        first_runtime_kv_namespace_preserved: false,
        first_disk_kv_reopen_verified: false,
        second_used_memories: 0,
        second_used_runtime_kv_memory: false,
        second_used_experiences: 0,
        second_approved_experience_reuse_digest: None,
        second_imported_runtime_kv_blocks: 0,
        second_imported_runtime_kv_from_namespace: false,
        second_runtime_kv_disk_rehydrated: false,
        second_kvswap_boundary_verified: false,
        second_runtime_adapter_observations: 0,
        second_runtime_adapter_best_score: None,
        second_runtime_adapter_best_adapter: None,
        second_runtime_selected_adapter: None,
        second_compute_budget_saved_tokens: 0,
        second_compute_budget_avoided_tokens: 0,
        second_compute_budget_kv_lookups_skipped: 0,
        second_compute_budget_anchor_count: 0,
        second_compute_budget_anchors_preserved: false,
        second_compute_budget_anchors_preserved_count: 0,
        second_quality: 0.2,
        first_drift_severity: DriftSeverity::Stable,
        second_drift_severity: DriftSeverity::Block,
    });

    assert!(!failed.passed);
    assert!(failed.failures.len() >= 7);
    assert!(
        failed
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_kv namespace"))
    );
    assert!(
        failed
            .failures
            .iter()
            .any(|failure| failure.contains("persisted runtime KV memory"))
    );
    assert!(
        failed
            .failures
            .iter()
            .any(|failure| failure.contains("disk KV files"))
    );
    assert!(
        failed
            .failures
            .iter()
            .any(|failure| failure.contains("kvswap boundary"))
    );
    assert!(
        failed
            .failures
            .iter()
            .any(|failure| failure.contains("adapter observations"))
    );
    assert!(
        failed
            .failures
            .iter()
            .any(|failure| failure.contains("best runtime adapter observation"))
    );
    assert!(
        failed
            .failures
            .iter()
            .any(|failure| failure.contains("approved experience reuse"))
    );
    assert!(
        failed
            .failures
            .iter()
            .any(|failure| failure.contains("compute budget avoided tokens"))
    );
}

#[test]
fn issue30_roundtrip_negative_gate_evidence_fails_closed() {
    let evidence = issue30_roundtrip_negative_gate_evidence();

    assert!(evidence.passed());
    assert!(!evidence.unauthorized_write_allowed);
    assert!(evidence.polluted_evidence_blocked);
    assert!(evidence.polluted_evidence_quarantined);
    assert!(evidence.bad_candidate_held_or_rolled_back);
    assert!(evidence.bad_candidate_bound());
    assert!(
        evidence
            .bad_candidate_digest
            .starts_with("redaction-digest:")
    );
    assert_eq!(evidence.bad_candidate_decision, "hold_then_rollback");
    assert!(evidence.rollback_anchor_present);
    assert!(evidence.rollback_anchor_bound());
    assert_eq!(
        evidence.rollback_anchor_evidence_id,
        "issue-30-roundtrip-negative-gate-hold"
    );
    assert!(
        evidence
            .rollback_anchor_digest
            .starts_with("redaction-digest:")
    );
    assert!(evidence.tenant_scope_write_denied);
    assert!(evidence.single_tenant_preview);
    assert!(evidence.provenance_license_redaction_passed);
    assert!(evidence.digest_only);
    assert!(evidence.failure_reasons().is_empty());
}

#[test]
fn issue30_kvswap_boundary_fixture_is_ready() {
    assert!(issue30_kvswap_boundary_verified());
}

#[test]
fn persistent_roundtrip_report_requires_observed_adapter_to_drive_second_runtime() {
    let report = PersistentRoundtripReport::evaluate(PersistentRoundtripInput {
        first_stored_memory: true,
        first_runtime_kv_stored: 1,
        first_runtime_kv_namespace_preserved: true,
        first_disk_kv_reopen_verified: true,
        second_used_memories: 2,
        second_used_runtime_kv_memory: true,
        second_used_experiences: 1,
        second_approved_experience_reuse_digest: approved_experience_reuse_digest(),
        second_imported_runtime_kv_blocks: 2,
        second_imported_runtime_kv_from_namespace: true,
        second_runtime_kv_disk_rehydrated: true,
        second_kvswap_boundary_verified: true,
        second_runtime_adapter_observations: 1,
        second_runtime_adapter_best_score: Some(0.80),
        second_runtime_adapter_best_adapter: Some("cpu-simd".to_owned()),
        second_runtime_selected_adapter: Some("portable-rust".to_owned()),
        second_compute_budget_saved_tokens: 32,
        second_compute_budget_avoided_tokens: 48,
        second_compute_budget_kv_lookups_skipped: 1,
        second_compute_budget_anchor_count: 1,
        second_compute_budget_anchors_preserved: true,
        second_compute_budget_anchors_preserved_count: 1,
        second_quality: 0.82,
        first_drift_severity: DriftSeverity::Stable,
        second_drift_severity: DriftSeverity::Stable,
    });

    assert!(!report.passed);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("selected adapter portable-rust"))
    );
    assert!(
        report
            .summary_line()
            .contains("second_runtime_adapter_best_adapter=cpu-simd")
    );
    assert!(
        report
            .summary_line()
            .contains("second_runtime_selected_adapter=portable-rust")
    );
}

#[test]
fn persistent_roundtrip_report_drops_untrusted_adapter_labels() {
    let report = PersistentRoundtripReport::evaluate(PersistentRoundtripInput {
        first_stored_memory: true,
        first_runtime_kv_stored: 1,
        first_runtime_kv_namespace_preserved: true,
        first_disk_kv_reopen_verified: true,
        second_used_memories: 2,
        second_used_runtime_kv_memory: true,
        second_used_experiences: 1,
        second_approved_experience_reuse_digest: approved_experience_reuse_digest(),
        second_imported_runtime_kv_blocks: 2,
        second_imported_runtime_kv_from_namespace: true,
        second_runtime_kv_disk_rehydrated: true,
        second_kvswap_boundary_verified: true,
        second_runtime_adapter_observations: 1,
        second_runtime_adapter_best_score: Some(0.80),
        second_runtime_adapter_best_adapter: Some("unknown-best secret=sk-best".to_owned()),
        second_runtime_selected_adapter: Some("unknown-selected secret=sk-selected".to_owned()),
        second_compute_budget_saved_tokens: 32,
        second_compute_budget_avoided_tokens: 48,
        second_compute_budget_kv_lookups_skipped: 1,
        second_compute_budget_anchor_count: 1,
        second_compute_budget_anchors_preserved: true,
        second_compute_budget_anchors_preserved_count: 1,
        second_quality: 0.82,
        first_drift_severity: DriftSeverity::Stable,
        second_drift_severity: DriftSeverity::Stable,
    });
    let summary_line = report.summary_line();

    assert!(!report.passed);
    assert_eq!(report.second_runtime_adapter_best_adapter, None);
    assert_eq!(report.second_runtime_selected_adapter, None);
    assert!(summary_line.contains("second_runtime_adapter_best_adapter=none"));
    assert!(summary_line.contains("second_runtime_selected_adapter=none"));
    for marker in [
        "unknown-best",
        "unknown-selected",
        "secret=",
        "sk-best",
        "sk-selected",
    ] {
        assert!(!summary_line.contains(marker), "{summary_line}");
        assert!(
            !report
                .failures
                .iter()
                .any(|failure| failure.contains(marker))
        );
    }
}

#[test]
fn issue30_problem_hypothesis_evidence_is_digest_only() {
    let line = issue30_problem_hypothesis_evidence_line();

    for marker in [
        "issue377_problem_finding_present=true",
        "issue377_problem_finding_id=redaction-digest:",
        "issue377_hypothesis_candidate_present=true",
        "issue377_hypothesis_candidate_id=redaction-digest:",
        "issue377_problem_hypothesis_link=redaction-digest:",
        "issue377_admission_decision=preview_only",
        "issue377_predicament_signal_present=true",
        "issue377_predicament_id=redaction-digest:",
        "issue377_predicament_progress_delta=0",
        "issue377_predicament_repeat_count=2",
        "issue377_predicament_evidence_gap_count=0",
        "issue377_predicament_action_novelty=0",
        "issue377_predicament_stuck=true",
        "issue377_self_trigger_stage=preview_only",
        "issue377_evolution_apply_allowed=false",
        "issue377_experiment_plan_present=true",
        "issue377_experiment_plan_id=redaction-digest:",
        "issue377_experiment_plan_mode=preview_only",
        "issue377_evidence_bundle_present=true",
        "issue377_evidence_bundle_id=redaction-digest:",
        "issue377_evidence_bundle_refs_digest_only=true",
        "issue377_experiment_decision=promote_for_approval",
        "issue377_experiment_runner_allowed=false",
        "issue377_experiment_apply_allowed=false",
        "issue377_mutation_candidate_emitter_present=true",
        "issue377_mutation_candidate_emitter_id=redaction-digest:",
        "issue377_mutation_candidate_id=redaction-digest:",
        "issue377_mutation_candidate_evidence_digest=redaction-digest:",
        "issue377_mutation_candidate_rollback_anchor=redaction-digest:",
        "issue377_mutation_candidate_requested_write_scope=reasoning_genome_preview",
        "issue377_mutation_candidate_kind=mutation_plan_preview",
        "issue377_mutation_candidate_preview_only=true",
        "issue377_mutation_candidate_refs_digest_only=true",
        "issue377_mutation_candidate_writer_gate_preflight=hold",
        "issue377_mutation_candidate_write_allowed=false",
        "issue377_mutation_candidate_applied=false",
        "issue377_mutation_candidate_apply_allowed=false",
        "issue377_mutation_candidate_manual_review_required=true",
        "issue377_manual_approval_binding_present=true",
        "issue377_manual_approval_candidate_id=redaction-digest:",
        "issue377_manual_approval_evidence_digest=redaction-digest:",
        "issue377_manual_approval_rollback_anchor=redaction-digest:",
        "issue377_manual_approval_requested_write_scope=reasoning_genome_preview",
        "issue377_manual_approval_ref=redaction-digest:",
        "issue377_manual_approval_expiration=1970-01-01T00:00:00Z",
        "issue377_manual_approval_apply_allowed=false",
        "issue377_manual_approval_applied=false",
    ] {
        assert!(line.contains(marker), "{marker}");
    }
    assert!(!crate::privacy_redaction::contains_private_or_executable_marker(&line));
}

#[test]
fn issue30_entry_chain_evidence_is_digest_only() {
    let line = issue30_entry_chain_evidence_line();

    for marker in [
        "issue30_environment_pressure_present=true",
        "issue30_pollution_event_id=redaction-digest:",
        "issue385_self_ontology_body_present=true",
        "issue385_body_state_id=redaction-digest:",
        "issue385_pheromone_signal_marker_present=true",
        "issue385_pheromone_signal_marker_id=redaction-digest:",
        "issue385_pheromone_signal_surface=digest_marker",
        "issue385_pheromone_signal_digest_gate_allowed=true",
        "issue385_pheromone_signal_preview_only=true",
        "issue375_pre_reasoning_genome_isa_present=true",
        "issue375_reasoning_frame_id=redaction-digest:",
        "issue375_reasoning_frame_environment_signals_present=true",
        "issue375_reasoning_frame_allowed_observations=repo_issue_terminal_runtime_state",
        "issue375_reasoning_frame_action_vocab=observe_inspect_compare_summarize_verify_quarantine",
        "issue375_reasoning_frame_suppressed_capabilities=write_process_browser_network_memory_genome_runtime",
        "issue375_reasoning_frame_risk_limits=preview_only_digest_only",
        "issue375_expression_vm_side_effect=read_only",
        "issue375_genome_isa_apply_allowed=false",
        "issue30_backend_action=deterministic_runtime_kv_roundtrip",
        "issue4_dna_candidate_ledger_present=true",
        "issue4_dna_candidate_ledger_schema=dna_evolution_candidate_ledger_v1",
        "issue4_dna_candidate_ledger_records=1",
        "issue4_dna_candidate_ledger_candidate_count=1",
        "issue4_dna_candidate_ledger_candidate_only=true",
        "issue4_dna_candidate_ledger_digest=redaction-digest:",
        "issue4_dna_candidate_ledger_raw_records_allowed=false",
        "issue4_dna_candidate_ledger_write_allowed=false",
        "issue4_dna_candidate_ledger_applied=false",
        "issue4_dna_candidate_ledger_preview_source=entry_chain_dna_evolution_controller",
        "issue243_active_control_knobs=routing|context_anchor|suppression|checkpoint|memory_maintenance",
        "issue243_evidence_digest=redaction-digest:",
        "issue243_policy_version=control_expression_gate_v1",
        "issue243_decision_reason=no_weight_runtime_control_preview",
        "issue243_control_expression_profile_selected=1",
        "issue243_context_anchor_promoted=1",
        "issue243_suppression_gate_triggered=1",
        "issue243_checkpoint_repair_requested=1",
        "issue243_checkpoint_rejected=1",
        "issue243_memory_refresh_candidate=1",
        "issue243_memory_tombstone_candidate=1",
        "issue243_control_expression_preview_admission=1",
        "issue243_write_allowed=false",
        "issue243_applied=false",
        "issue243_operator_approval_required=true",
        "issue379_control_candidate_preview_only=true",
        "issue379_action_vocab_mask_preview=true",
        "issue379_signal_saliency_bias_preview=true",
        "issue379_zero_beat_primitive_decision_present=true",
        "issue379_primitive_authority=preview_only",
        "issue379_primitive_side_effect=read_only",
        "issue379_primitive_reversibility=rollback_required",
        "issue379_primitive_evidence=digest_only",
        "issue379_primitive_uncertainty=hold_on_gap",
        "issue379_primitive_attention=focus_or_mask_preview",
        "issue379_zero_beat_output=action_vocab_mask_and_signal_saliency_bias",
        "issue379_generation_bias_apply_allowed=false",
        "issue493_tool_organ_registry_present=true",
        "issue493_tool_organ_registry_id=redaction-digest:",
        "issue493_tool_organ_registry_preview_only=true",
        "issue493_tool_organ_registry_side_effect=read_only",
        "issue493_tool_organ_registry_apply_allowed=false",
        "issue493_tool_organ_capability_matrix_digest=redaction-digest:",
        "issue493_preview_bundle_protocol=bundle_v1",
        "issue493_preview_bundle_digest=redaction-digest:",
        "issue493_preview_bundle_refs_digest_only=true",
        "issue493_preview_bundle_raw_artifacts_allowed=false",
        "issue493_tool_install_allowed=false",
        "issue493_tool_execution_allowed=false",
        "bio_epigenetic_expression_marker_present=true",
        "bio_epigenetic_expression_marker_id=redaction-digest:",
        "bio_mrna_cache_candidate_digest=redaction-digest:",
        "bio_expression_cache_protocol=mrna_preview_v1",
        "bio_expression_cache_key_digest=redaction-digest:",
        "bio_hot_path_observation_window=100",
        "bio_hot_path_min_success_rate=0.98",
        "bio_gate_relaxation_allowed=false",
        "bio_cache_materialization_allowed=false",
        "bio_raw_payload_or_kv_cached=false",
        "bio_negative_evidence_overrides=true",
        "issue501_telomere_state_present=true",
        "issue501_remaining_tokens=0",
        "issue501_remaining_steps=0",
        "issue501_remaining_messages=0",
        "issue501_repair_streak_count=2",
        "issue501_loop_risk_signal_count=",
        "issue501_senescent=true",
        "issue501_apoptosis_required=true",
        "issue501_new_external_call_allowed=false",
        "issue501_new_file_write_allowed=false",
        "issue501_new_memory_write_allowed=false",
        "issue501_new_adaptive_state_write_allowed=false",
        "issue501_memory_promotion_allowed=false",
        "issue501_genome_mutation_allowed=false",
        "issue501_takeover_packet_digest=redaction-digest:",
        "issue501_rollback_anchor_digest=redaction-digest:",
        "issue501_handoff_next_owner=scheduler",
        "issue501_raw_payload_present=false",
        "issue501_preview_side_effect_allowed=false",
        "issue502_pheromone_blackboard_present=true",
        "issue502_signal_count=",
        "issue502_ranked_action_count=",
        "issue502_top_signal_kind=repair_first",
        "issue502_top_action=repair_review",
        "issue502_blackboard_digest=redaction-digest:",
        "issue502_source_digest=redaction-digest:",
        "issue502_payload_digest=redaction-digest:",
        "issue502_raw_payload_present=false",
        "issue502_side_effect_allowed=false",
        "issue502_ttl_decay_present=true",
        "issue502_conflict_routes_to_repair=true",
        "issue502_ranked_actions_from_state_only=true",
        "issue509_quorum_sensing_present=true",
        "issue509_decision_id=redaction-digest:",
        "issue509_quorum_report_digest=redaction-digest:",
        "issue509_risk_class=irreversible",
        "issue509_required_quorum_milli=700",
        "issue509_evaluator_count=3",
        "issue509_independent_model_count=3",
        "issue509_independent_lane_count=3",
        "issue509_approve_signal_count=2",
        "issue509_reject_signal_count=1",
        "issue509_abstain_signal_count=0",
        "issue509_approval_concentration_milli=666",
        "issue509_conflict_count=1",
        "issue509_quorum_reached=false",
        "issue509_apply_allowed=false",
        "issue509_raw_evaluator_payload_present=false",
        "issue509_duplicate_sources_count_once=true",
        "issue509_conflict_routes_to_repair=true",
        "issue509_writer_gate_bypass_allowed=false",
    ] {
        assert!(line.contains(marker), "{marker}");
    }
    assert!(!crate::privacy_redaction::contains_private_or_executable_marker(&line));
}

#[test]
fn pheromone_signal_marker_evidence_is_digest_only() {
    let line = issue30_entry_chain_evidence_line();

    for marker in [
        "issue385_self_ontology_body_present=true",
        "issue385_body_state_id=redaction-digest:",
        "issue385_pheromone_signal_marker_present=true",
        "issue385_pheromone_signal_marker_id=redaction-digest:",
        "issue385_pheromone_signal_surface=digest_marker",
        "issue385_pheromone_signal_digest_gate_allowed=true",
        "issue385_pheromone_signal_preview_only=true",
    ] {
        assert!(line.contains(marker), "{marker}");
    }
    assert!(!crate::privacy_redaction::contains_private_or_executable_marker(&line));
}

#[test]
fn persistent_roundtrip_matrix_requires_every_explicit_device_to_pass() {
    let passing_report = PersistentRoundtripReport::evaluate(PersistentRoundtripInput {
        first_stored_memory: true,
        first_runtime_kv_stored: 1,
        first_runtime_kv_namespace_preserved: true,
        first_disk_kv_reopen_verified: true,
        second_used_memories: 2,
        second_used_runtime_kv_memory: true,
        second_used_experiences: 1,
        second_approved_experience_reuse_digest: approved_experience_reuse_digest(),
        second_imported_runtime_kv_blocks: 1,
        second_imported_runtime_kv_from_namespace: true,
        second_runtime_kv_disk_rehydrated: true,
        second_kvswap_boundary_verified: true,
        second_runtime_adapter_observations: 1,
        second_runtime_adapter_best_score: Some(0.72),
        second_runtime_adapter_best_adapter: Some("portable-rust".to_owned()),
        second_runtime_selected_adapter: Some("portable-rust".to_owned()),
        second_compute_budget_saved_tokens: 32,
        second_compute_budget_avoided_tokens: 48,
        second_compute_budget_kv_lookups_skipped: 1,
        second_compute_budget_anchor_count: 1,
        second_compute_budget_anchors_preserved: true,
        second_compute_budget_anchors_preserved_count: 1,
        second_quality: 0.80,
        first_drift_severity: DriftSeverity::Stable,
        second_drift_severity: DriftSeverity::Stable,
    });
    let complete = PersistentRoundtripMatrixReport::evaluate(
        DeviceClass::explicit_profiles()
            .iter()
            .copied()
            .map(|device| PersistentRoundtripDeviceReport {
                device,
                report: passing_report.clone(),
            })
            .collect(),
    );

    assert!(complete.passed, "{:?}", complete.failures);
    assert_eq!(
        complete.covered_devices(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(complete.missing_devices().is_empty());
    assert_eq!(
        complete.second_compute_budget_saved_tokens(),
        32 * DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        complete.second_compute_budget_avoided_tokens(),
        48 * DeviceClass::explicit_profiles().len()
    );
    assert_eq!(
        complete.second_compute_budget_kv_lookups_skipped(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(complete.device_reports.iter().all(|device_report| {
        device_report.report.negative_gate_evidence.passed()
            && device_report
                .report
                .summary_line()
                .contains("negative_tenant_scope_write_denied=true")
            && device_report
                .report
                .summary_line()
                .contains("negative_tenant_scope_denial_reason=cross_tenant_scope_rejected")
            && device_report
                .report
                .summary_line()
                .contains("negative_digest_only=true")
    }));
    assert!(
        complete
            .summary_line()
            .contains("persistent_roundtrip_matrix: passed=true")
    );
    assert!(complete.summary_line().contains(&format!(
        "second_compute_budget_avoided_tokens={}",
        48 * DeviceClass::explicit_profiles().len()
    )));

    let failed_report = PersistentRoundtripReport::evaluate(PersistentRoundtripInput {
        first_stored_memory: true,
        first_runtime_kv_stored: 1,
        first_runtime_kv_namespace_preserved: true,
        first_disk_kv_reopen_verified: true,
        second_used_memories: 1,
        second_used_runtime_kv_memory: false,
        second_used_experiences: 1,
        second_approved_experience_reuse_digest: approved_experience_reuse_digest(),
        second_imported_runtime_kv_blocks: 1,
        second_imported_runtime_kv_from_namespace: false,
        second_runtime_kv_disk_rehydrated: false,
        second_kvswap_boundary_verified: true,
        second_runtime_adapter_observations: 1,
        second_runtime_adapter_best_score: Some(0.72),
        second_runtime_adapter_best_adapter: Some("portable-rust".to_owned()),
        second_runtime_selected_adapter: Some("portable-rust".to_owned()),
        second_compute_budget_saved_tokens: 32,
        second_compute_budget_avoided_tokens: 48,
        second_compute_budget_kv_lookups_skipped: 1,
        second_compute_budget_anchor_count: 1,
        second_compute_budget_anchors_preserved: true,
        second_compute_budget_anchors_preserved_count: 1,
        second_quality: 0.80,
        first_drift_severity: DriftSeverity::Stable,
        second_drift_severity: DriftSeverity::Stable,
    });
    let incomplete = PersistentRoundtripMatrixReport::evaluate(vec![
        PersistentRoundtripDeviceReport {
            device: DeviceClass::CpuOnly,
            report: passing_report,
        },
        PersistentRoundtripDeviceReport {
            device: DeviceClass::IntegratedGpu,
            report: failed_report,
        },
    ]);

    assert!(!incomplete.passed);
    assert_eq!(incomplete.covered_devices(), 2);
    assert_eq!(
        incomplete.missing_devices().len(),
        DeviceClass::explicit_profiles().len() - 2
    );
    assert_eq!(
        incomplete.failed_devices(),
        vec![DeviceClass::IntegratedGpu]
    );
    assert!(
        incomplete
            .failures
            .iter()
            .any(|failure| failure.contains("missing="))
    );
    assert!(
        incomplete
            .failures
            .iter()
            .any(|failure| failure.contains("integrated"))
    );
}
