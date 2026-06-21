use super::*;

#[test]
fn gate_reports_missing_evolution_ledger_coverage() {
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger {
            replay_rust_check_failed: 1,
            ..EvolutionLedger::default()
        },
        results: vec![BenchmarkCaseResult {
            name: "evolution_ledger".to_owned(),
            profile: TaskProfile::LongDocument,
            device: DeviceClass::CpuOnly,
            elapsed_ms: 1,
            quality: 0.9,
            process_reward: 0.9,
            attention_fraction: 0.5,
            requires_recursion: true,
            recursive_chunks: 4,
            recursive_waves: 2,
            recursive_runtime_calls: 7,
            auto_replay_applied: 0,
            auto_replay_router_updates: 0,
            auto_replay_hierarchy_updates: 0,
            auto_replay_router_threshold_mutations: 0,
            auto_replay_hierarchy_weight_mutations: 0,
            auto_replay_router_threshold_delta: 0.0,
            auto_replay_hierarchy_weight_delta: 0.0,
            auto_replay_memory_reinforcements: 0,
            auto_replay_memory_penalties: 0,
            auto_replay_live_memory_feedback_items: 0,
            auto_replay_live_memory_feedback_updates: 0,
            auto_replay_live_memory_feedback_reinforcements: 0,
            auto_replay_live_memory_feedback_penalties: 0,
            auto_replay_live_memory_feedback_detail_items: 0,
            auto_replay_live_memory_feedback_applied: 0,
            auto_replay_live_memory_feedback_removed: 0,
            auto_replay_live_memory_feedback_missing: 0,
            auto_replay_live_memory_feedback_strength_delta: 0.0,
            auto_replay_recursive_runtime_items: 0,
            auto_replay_recursive_runtime_calls: 0,
            auto_replay_avg_recursive_call_pressure: 0.0,
            auto_replay_max_recursive_call_pressure: 0.0,
            used_memories: 0,
            infini_local_window: 1,
            infini_global_memory: 1,
            sparse_skipped: 0,
            sparse_skipped_tokens: 0,
            stored_memories: 0,
            compacted_memories: 0,
            runtime_forward_signal: false,
            runtime_forward_energy_signal: false,
            runtime_kv_influence_signal: false,
            runtime_global_layers: 0,
            runtime_local_window_layers: 0,
            runtime_convolutional_fusion_layers: 0,
            runtime_layer_mode_signal: false,
            runtime_all_layer_modes_signal: false,
            runtime_token_count: 0,
            runtime_uncertainty_token_count: 0,
            runtime_uncertainty_signal: false,
            runtime_kv_imported: 0,
            runtime_kv_exported: 0,
            runtime_kv_stored: 0,
            runtime_adapter_observations: 0,
            runtime_selected_adapter: None,
            runtime_adapter_contract_ok: false,
            runtime_adapter_contract_violations: 0,
            runtime_adapter_best_score: None,
            runtime_adapter_best_adapter: None,
            runtime_adapter_selection_mismatches: 0,
            query_embedding_source: "fallback".to_owned(),
            query_embedding_dimensions: 64,
            runtime_embedding_calls: 0,
            fallback_embedding_calls: 1,
            embedding_fallback_used: true,
            drift_severity: DriftSeverity::Stable,
        }],
    };
    let mut gate = BenchmarkGate::default();
    gate.min_evolution_live_inference_runs = Some(1);
    gate.min_evolution_live_router_threshold_mutations = Some(1);
    gate.min_evolution_live_hierarchy_weight_mutations = Some(1);
    gate.min_evolution_live_router_threshold_delta = Some(0.01);
    gate.min_evolution_live_hierarchy_weight_delta = Some(0.01);
    gate.min_evolution_live_online_reward_feedbacks = Some(1);
    gate.min_evolution_live_online_reward_reinforcements = Some(1);
    gate.min_evolution_live_online_reward_penalties = Some(1);
    gate.min_evolution_live_online_reward_strength = Some(1.0);
    gate.min_evolution_live_online_reward_reinforcement_strength = Some(0.6);
    gate.min_evolution_live_online_reward_penalty_strength = Some(0.4);
    gate.min_evolution_live_memory_updates = Some(1);
    gate.min_evolution_live_stored_memory_updates = Some(1);
    gate.min_evolution_live_reflection_issues = Some(1);
    gate.min_evolution_live_critical_reflection_issues = Some(1);
    gate.min_evolution_live_revision_actions = Some(1);
    gate.min_evolution_replay_runs = Some(1);
    gate.min_evolution_replay_items = Some(2);
    gate.min_evolution_router_threshold_mutations = Some(3);
    gate.min_evolution_hierarchy_weight_mutations = Some(4);
    gate.min_evolution_router_threshold_delta = Some(0.05);
    gate.min_evolution_hierarchy_weight_delta = Some(0.06);
    gate.min_evolution_memory_updates = Some(5);
    gate.min_evolution_replay_live_memory_feedback_updates = Some(3);
    gate.min_evolution_replay_live_memory_feedback_detail_items = Some(2);
    gate.min_evolution_replay_live_memory_feedback_applied = Some(4);
    gate.min_evolution_replay_live_memory_feedback_strength_delta = Some(0.42);
    gate.min_evolution_replay_rust_check_items = Some(1);
    gate.min_evolution_replay_rust_check_passed = Some(1);
    gate.max_evolution_replay_rust_check_failed = Some(0);
    gate.min_evolution_replay_rust_check_live_memory_feedback_updates = Some(2);
    gate.min_evolution_replay_rust_check_live_memory_feedback_applied = Some(2);
    gate.min_evolution_replay_rust_check_live_memory_feedback_strength_delta = Some(0.36);
    gate.min_evolution_replay_live_evolution_items = Some(2);
    gate.min_evolution_replay_live_evolution_online_reward_feedbacks = Some(2);
    gate.min_evolution_replay_live_evolution_online_reward_reinforcements = Some(1);
    gate.min_evolution_replay_live_evolution_online_reward_penalties = Some(1);
    gate.min_evolution_replay_live_evolution_online_reward_strength = Some(1.0);
    gate.min_evolution_replay_live_evolution_online_reward_reinforcement_strength = Some(0.6);
    gate.min_evolution_replay_live_evolution_online_reward_penalty_strength = Some(0.4);
    gate.min_evolution_replay_live_evolution_memory_updates = Some(3);
    gate.min_evolution_replay_live_evolution_stored_memory_updates = Some(2);
    gate.min_evolution_replay_live_evolution_reflection_issues = Some(2);
    gate.min_evolution_replay_live_evolution_critical_reflection_issues = Some(1);
    gate.min_evolution_replay_live_evolution_revision_actions = Some(2);
    gate.min_evolution_recursive_replay_items = Some(6);
    gate.min_evolution_recursive_runtime_calls = Some(7);
    gate.max_evolution_drift_rollbacks = Some(0);
    gate.max_evolution_rollback_router_threshold_delta = Some(0.0);
    gate.max_evolution_rollback_hierarchy_weight_delta = Some(0.0);

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    for marker in [
        "evolution_live_inference_runs",
        "evolution_live_router_threshold_mutations",
        "evolution_live_hierarchy_weight_mutations",
        "evolution_live_router_threshold_delta",
        "evolution_live_hierarchy_weight_delta",
        "evolution_live_online_reward_feedbacks",
        "evolution_live_online_reward_reinforcements",
        "evolution_live_online_reward_penalties",
        "evolution_live_online_reward_strength",
        "evolution_live_online_reward_reinforcement_strength",
        "evolution_live_online_reward_penalty_strength",
        "evolution_live_memory_updates",
        "evolution_live_stored_memory_updates",
        "evolution_live_reflection_issues",
        "evolution_live_critical_reflection_issues",
        "evolution_live_revision_actions",
        "evolution_replay_runs",
        "evolution_replay_items",
        "evolution_router_threshold_mutations",
        "evolution_hierarchy_weight_mutations",
        "evolution_router_threshold_delta",
        "evolution_hierarchy_weight_delta",
        "evolution_memory_updates",
        "evolution_replay_live_memory_feedback_updates",
        "evolution_replay_live_memory_feedback_detail_items",
        "evolution_replay_live_memory_feedback_applied",
        "evolution_replay_live_memory_feedback_strength_delta",
        "evolution_replay_rust_check_items",
        "evolution_replay_rust_check_passed",
        "evolution_replay_rust_check_failed",
        "evolution_replay_rust_check_live_memory_feedback_updates",
        "evolution_replay_rust_check_live_memory_feedback_applied",
        "evolution_replay_rust_check_live_memory_feedback_strength_delta",
        "evolution_replay_live_evolution_items",
        "evolution_replay_live_evolution_online_reward_feedbacks",
        "evolution_replay_live_evolution_online_reward_reinforcements",
        "evolution_replay_live_evolution_online_reward_penalties",
        "evolution_replay_live_evolution_online_reward_strength",
        "evolution_replay_live_evolution_online_reward_reinforcement_strength",
        "evolution_replay_live_evolution_online_reward_penalty_strength",
        "evolution_replay_live_evolution_memory_updates",
        "evolution_replay_live_evolution_stored_memory_updates",
        "evolution_replay_live_evolution_reflection_issues",
        "evolution_replay_live_evolution_critical_reflection_issues",
        "evolution_replay_live_evolution_revision_actions",
        "evolution_recursive_replay_items",
        "evolution_recursive_runtime_calls",
    ] {
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains(marker)),
            "missing failure marker {marker}: {:?}",
            report.failures
        );
    }

    let passing = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger {
            live_inference_runs: 8,
            live_router_threshold_mutations: 2,
            live_hierarchy_weight_mutations: 1,
            live_router_threshold_delta: 0.07,
            live_hierarchy_weight_delta: 0.04,
            live_online_reward_feedbacks: 3,
            live_online_reward_reinforcements: 2,
            live_online_reward_penalties: 1,
            live_online_reward_strength: 1.80,
            live_online_reward_reinforcement_strength: 1.20,
            live_online_reward_penalty_strength: 0.60,
            live_memory_reinforcements: 3,
            live_memory_penalties: 2,
            live_stored_memories: 1,
            live_stored_gist_memories: 2,
            live_stored_runtime_kv_memories: 1,
            live_reflection_issues: 4,
            live_critical_reflection_issues: 1,
            live_revision_actions: 5,
            replay_runs: 1,
            replay_items: 2,
            router_threshold_mutations: 3,
            hierarchy_weight_mutations: 4,
            router_threshold_delta: 0.05,
            hierarchy_weight_delta: 0.06,
            memory_reinforcements: 5,
            memory_penalties: 0,
            replay_live_memory_feedback_items: 2,
            replay_live_memory_feedback_reinforcements: 2,
            replay_live_memory_feedback_penalties: 1,
            replay_live_memory_feedback_detail_items: 2,
            replay_live_memory_feedback_applied: 4,
            replay_live_memory_feedback_removed: 1,
            replay_live_memory_feedback_missing: 1,
            replay_live_memory_feedback_strength_delta: 0.42,
            replay_rust_check_items: 1,
            replay_rust_check_passed: 1,
            replay_rust_check_failed: 0,
            replay_rust_check_diagnostic_chars: 0,
            replay_rust_check_live_memory_feedback_items: 1,
            replay_rust_check_live_memory_feedback_updates: 2,
            replay_rust_check_live_memory_feedback_applied: 2,
            replay_rust_check_live_memory_feedback_strength_delta: 0.36,
            replay_business_contract_items: 3,
            replay_business_contract_passed: 3,
            replay_business_contract_failed: 0,
            replay_business_contract_raw_passed: 1,
            replay_business_contract_raw_failed: 2,
            replay_business_contract_response_normalized: 2,
            replay_business_contract_sanitized: 0,
            replay_business_contract_canonical_fallbacks: 2,
            replay_live_evolution_items: 2,
            replay_live_evolution_router_threshold_mutations: 1,
            replay_live_evolution_hierarchy_weight_mutations: 1,
            replay_live_evolution_router_threshold_delta: 0.04,
            replay_live_evolution_hierarchy_weight_delta: 0.03,
            replay_live_evolution_online_reward_feedbacks: 2,
            replay_live_evolution_online_reward_reinforcements: 1,
            replay_live_evolution_online_reward_penalties: 1,
            replay_live_evolution_online_reward_strength: 1.30,
            replay_live_evolution_online_reward_reinforcement_strength: 0.80,
            replay_live_evolution_online_reward_penalty_strength: 0.50,
            replay_live_evolution_memory_updates: 3,
            replay_live_evolution_stored_memory_updates: 2,
            replay_live_evolution_reflection_issues: 2,
            replay_live_evolution_critical_reflection_issues: 1,
            replay_live_evolution_revision_actions: 2,
            recursive_replay_items: 6,
            recursive_runtime_calls: 7,
            drift_rollbacks: 0,
            rollback_router_threshold_delta: 0.0,
            rollback_hierarchy_weight_delta: 0.0,
            external_feedbacks: 2,
            external_feedback_reinforcements: 3,
            external_feedback_penalties: 1,
            external_feedback_memory_updates: 4,
            external_feedback_removed: 1,
            external_feedback_missing: 2,
            external_feedback_strength_delta: 0.31,
        },
        results: summary.results.clone(),
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    let summary_line = passing.summary_line();
    assert!(summary_line.contains("evolution_live_online_reward_strength=1.800000"));
    assert!(summary_line.contains("evolution_live_online_reward_reinforcement_strength=1.200000"));
    assert!(summary_line.contains("evolution_live_online_reward_penalty_strength=0.600000"));
    assert!(
        summary_line.contains("evolution_replay_live_evolution_online_reward_strength=1.300000")
    );
    assert!(
        summary_line.contains(
            "evolution_replay_live_evolution_online_reward_reinforcement_strength=0.800000"
        )
    );
    assert!(
        summary_line
            .contains("evolution_replay_live_evolution_online_reward_penalty_strength=0.500000")
    );
    assert_eq!(passing.evolution_ledger().replay_runs, 1);
    assert_eq!(passing.evolution_ledger().live_inference_runs, 8);
    assert_eq!(passing.evolution_ledger().live_online_reward_feedbacks, 3);
    assert_eq!(
        passing
            .evolution_ledger()
            .replay_live_evolution_online_reward_feedbacks,
        2
    );
    assert_eq!(passing.evolution_ledger().live_memory_updates(), 5);
    assert_eq!(passing.evolution_ledger().live_stored_memory_updates(), 4);
    assert_eq!(passing.evolution_ledger().memory_updates(), 5);
    assert_eq!(
        passing
            .evolution_ledger()
            .replay_live_memory_feedback_updates(),
        3
    );
    assert!(passing.summary_line().contains("evolution_replay_runs=1"));
    assert!(
        passing
            .summary_line()
            .contains("evolution_live_inference_runs=8")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_live_online_reward_feedbacks=3")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_live_memory_updates=5")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_live_stored_memory_updates=4")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_external_feedbacks=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_external_feedback_memory_updates=4")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_external_feedback_strength_delta=0.310000")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_memory_feedback_updates=3")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_memory_feedback_detail_items=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_memory_feedback_applied=4")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_memory_feedback_strength_delta=0.420000")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_rust_check_items=1")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_rust_check_passed=1")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_rust_check_live_memory_feedback_updates=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_rust_check_live_memory_feedback_strength_delta=0.360000")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_evolution_items=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_evolution_online_reward_feedbacks=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_evolution_memory_updates=3")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_evolution_stored_memory_updates=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_evolution_reflection_issues=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_evolution_critical_reflection_issues=1")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_replay_live_evolution_revision_actions=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_router_threshold_delta=0.050000")
    );
    assert!(
        passing
            .summary_line()
            .contains("evolution_recursive_runtime_calls=7")
    );
}

#[test]
fn gate_reports_evolution_ledger_drift_rollback_failures() {
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger {
            drift_rollbacks: 2,
            rollback_router_threshold_delta: 0.03,
            rollback_hierarchy_weight_delta: 0.04,
            ..EvolutionLedger::default()
        },
        results: vec![BenchmarkCaseResult {
            name: "evolution_rollback_audit".to_owned(),
            profile: TaskProfile::General,
            device: DeviceClass::CpuOnly,
            elapsed_ms: 1,
            quality: 0.9,
            process_reward: 0.9,
            attention_fraction: 0.5,
            requires_recursion: false,
            recursive_chunks: 1,
            recursive_waves: 1,
            recursive_runtime_calls: 1,
            auto_replay_applied: 0,
            auto_replay_router_updates: 0,
            auto_replay_hierarchy_updates: 0,
            auto_replay_router_threshold_mutations: 0,
            auto_replay_hierarchy_weight_mutations: 0,
            auto_replay_router_threshold_delta: 0.0,
            auto_replay_hierarchy_weight_delta: 0.0,
            auto_replay_memory_reinforcements: 0,
            auto_replay_memory_penalties: 0,
            auto_replay_live_memory_feedback_items: 0,
            auto_replay_live_memory_feedback_updates: 0,
            auto_replay_live_memory_feedback_reinforcements: 0,
            auto_replay_live_memory_feedback_penalties: 0,
            auto_replay_live_memory_feedback_detail_items: 0,
            auto_replay_live_memory_feedback_applied: 0,
            auto_replay_live_memory_feedback_removed: 0,
            auto_replay_live_memory_feedback_missing: 0,
            auto_replay_live_memory_feedback_strength_delta: 0.0,
            auto_replay_recursive_runtime_items: 0,
            auto_replay_recursive_runtime_calls: 0,
            auto_replay_avg_recursive_call_pressure: 0.0,
            auto_replay_max_recursive_call_pressure: 0.0,
            used_memories: 0,
            infini_local_window: 0,
            infini_global_memory: 0,
            sparse_skipped: 0,
            sparse_skipped_tokens: 0,
            stored_memories: 0,
            compacted_memories: 0,
            runtime_forward_signal: false,
            runtime_forward_energy_signal: false,
            runtime_kv_influence_signal: false,
            runtime_global_layers: 0,
            runtime_local_window_layers: 0,
            runtime_convolutional_fusion_layers: 0,
            runtime_layer_mode_signal: false,
            runtime_all_layer_modes_signal: false,
            runtime_token_count: 0,
            runtime_uncertainty_token_count: 0,
            runtime_uncertainty_signal: false,
            runtime_kv_imported: 0,
            runtime_kv_exported: 0,
            runtime_kv_stored: 0,
            runtime_adapter_observations: 0,
            runtime_selected_adapter: None,
            runtime_adapter_contract_ok: false,
            runtime_adapter_contract_violations: 0,
            runtime_adapter_best_score: None,
            runtime_adapter_best_adapter: None,
            runtime_adapter_selection_mismatches: 0,
            query_embedding_source: "fallback".to_owned(),
            query_embedding_dimensions: 64,
            runtime_embedding_calls: 0,
            fallback_embedding_calls: 1,
            embedding_fallback_used: true,
            drift_severity: DriftSeverity::Stable,
        }],
    };

    let report = summary.evaluate(&BenchmarkGate::default());

    assert!(!report.passed);
    for marker in [
        "evolution_drift_rollbacks",
        "evolution_rollback_router_threshold_delta",
        "evolution_rollback_hierarchy_weight_delta",
    ] {
        assert!(
            report
                .failures
                .iter()
                .any(|failure| failure.contains(marker)),
            "missing failure marker {marker}: {:?}",
            report.failures
        );
    }
}
