use super::*;

#[test]
fn gate_reports_missing_sparse_filtering_coverage() {
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![BenchmarkCaseResult {
            name: "sparse_filter".to_owned(),
            profile: TaskProfile::LongDocument,
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
            used_memories: 2,
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
    gate.min_sparse_skipped_cases = Some(1);
    gate.min_sparse_skipped_tokens = Some(3);

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("sparse_skipped_cases"))
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("sparse_skipped_tokens"))
    );

    let passing = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![BenchmarkCaseResult {
            sparse_skipped: 2,
            sparse_skipped_tokens: 7,
            ..summary.results[0].clone()
        }],
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(passing.sparse_skipped_cases(), 1);
    assert_eq!(passing.total_sparse_skipped_tokens(), 7);
    assert!(passing.summary_line().contains("sparse_skipped_cases=1"));
    assert!(passing.summary_line().contains("sparse_skipped_tokens=7"));
}

#[test]
fn gate_reports_missing_device_profile_coverage() {
    let base = BenchmarkCaseResult {
        name: "device_coverage".to_owned(),
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
    };
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![base.clone()],
    };
    let mut gate = BenchmarkGate::default();
    gate.min_device_profiles = Some(DeviceClass::explicit_profiles().len());

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.explicit_device_profiles_covered(), 1);
    assert_eq!(
        summary.missing_explicit_device_profiles().len(),
        DeviceClass::explicit_profiles().len() - 1
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("device_profiles"))
    );

    let passing = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: DeviceClass::explicit_profiles()
            .iter()
            .map(|device| BenchmarkCaseResult {
                device: *device,
                ..base.clone()
            })
            .collect(),
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(
        passing.explicit_device_profiles_covered(),
        DeviceClass::explicit_profiles().len()
    );
    assert!(passing.summary_line().contains("device_profiles=12"));
    assert!(passing.summary_line().contains("devices=cpu+integrated"));
}

#[test]
fn gate_reports_missing_recursive_device_profile_coverage() {
    let base = BenchmarkCaseResult {
        name: "recursive_device_coverage".to_owned(),
        profile: TaskProfile::LongDocument,
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
    };
    let mut gate = BenchmarkGate::default();
    gate.min_recursive_device_profiles = Some(DeviceClass::explicit_profiles().len());
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: DeviceClass::explicit_profiles()
            .iter()
            .map(|device| BenchmarkCaseResult {
                device: *device,
                ..base.clone()
            })
            .collect(),
    };

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.explicit_device_profiles_covered(), 12);
    assert_eq!(summary.recursive_device_profiles_covered(), 0);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("recursive_device_profiles"))
    );

    let passing = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: DeviceClass::explicit_profiles()
            .iter()
            .map(|device| BenchmarkCaseResult {
                device: *device,
                requires_recursion: true,
                recursive_chunks: 2,
                recursive_runtime_calls: 3,
                ..base.clone()
            })
            .collect(),
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(passing.recursive_device_profiles_covered(), 12);
    assert!(passing.missing_recursive_device_profiles().is_empty());
    assert!(
        passing
            .summary_line()
            .contains("recursive_device_profiles=12")
    );
    assert!(
        passing
            .summary_line()
            .contains("recursive_devices=cpu+integrated")
    );
}

#[test]
fn gate_reports_drift_failures() {
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![BenchmarkCaseResult {
            name: "drift".to_owned(),
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
            drift_severity: DriftSeverity::Rollback,
        }],
    };
    let report = summary.evaluate(&BenchmarkGate::default());

    assert!(!report.passed);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("drift_rollbacks"))
    );
}
