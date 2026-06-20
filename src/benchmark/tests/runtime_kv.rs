use super::*;

#[test]
fn gate_reports_missing_runtime_kv_import() {
    let summary = BenchmarkSummary {
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![BenchmarkCaseResult {
            name: "runtime_import".to_owned(),
            profile: TaskProfile::Coding,
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
            runtime_forward_signal: true,
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
            runtime_kv_exported: 1,
            runtime_kv_stored: 1,
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_adapter_contract_ok: true,
            runtime_adapter_contract_violations: 0,
            runtime_adapter_observations: 0,
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
    gate.min_runtime_kv_import_cases = Some(1);
    gate.min_runtime_kv_imported = Some(2);
    gate.min_runtime_kv_import_device_profiles = Some(1);

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.runtime_kv_import_cases(), 0);
    assert_eq!(summary.total_runtime_kv_imported(), 0);
    assert_eq!(summary.runtime_kv_import_device_profiles(), 0);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_import_cases"))
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_imported"))
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_import_device_profiles"))
    );

    let passing = BenchmarkSummary {
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![BenchmarkCaseResult {
            runtime_kv_imported: 3,
            ..summary.results[0].clone()
        }],
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(passing.runtime_kv_import_cases(), 1);
    assert_eq!(passing.total_runtime_kv_imported(), 3);
    assert_eq!(passing.runtime_kv_import_device_profiles(), 1);
    assert_eq!(passing.runtime_kv_import_devices_csv(), "cpu");
    assert!(passing.summary_line().contains("runtime_kv_import_cases=1"));
    assert!(passing.summary_line().contains("runtime_kv_imported=3"));
    assert!(
        passing
            .summary_line()
            .contains("runtime_kv_import_device_profiles=1")
    );
    assert!(
        passing
            .summary_line()
            .contains("runtime_kv_import_devices=cpu")
    );
}

#[test]
fn gate_reports_missing_runtime_kv_import_device_profile_coverage() {
    let base = BenchmarkCaseResult {
        name: "runtime_import".to_owned(),
        profile: TaskProfile::Coding,
        device: DeviceClass::CpuOnly,
        elapsed_ms: 1,
        quality: 0.7,
        process_reward: 0.7,
        attention_fraction: 0.03,
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
        stored_memories: 1,
        compacted_memories: 0,
        runtime_forward_signal: true,
        runtime_forward_energy_signal: true,
        runtime_kv_influence_signal: true,
        runtime_global_layers: 0,
        runtime_local_window_layers: 0,
        runtime_convolutional_fusion_layers: 0,
        runtime_layer_mode_signal: false,
        runtime_all_layer_modes_signal: false,
        runtime_token_count: 1,
        runtime_uncertainty_token_count: 1,
        runtime_uncertainty_signal: true,
        runtime_kv_imported: 2,
        runtime_kv_exported: 1,
        runtime_kv_stored: 1,
        runtime_selected_adapter: Some("portable-rust".to_owned()),
        runtime_adapter_contract_ok: true,
        runtime_adapter_contract_violations: 0,
        runtime_adapter_observations: 0,
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
        results: vec![base.clone()],
        ..BenchmarkSummary::default()
    };
    let mut gate = BenchmarkGate::default();
    gate.min_runtime_kv_import_device_profiles = Some(2);

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.runtime_kv_import_device_profiles(), 1);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("runtime_kv_import_device_profiles 1 below minimum 2")
            && failure.contains("devices=cpu")
    }));

    let passing = BenchmarkSummary {
        results: vec![
            base.clone(),
            BenchmarkCaseResult {
                device: DeviceClass::IntegratedGpu,
                ..base
            },
        ],
        ..BenchmarkSummary::default()
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(passing.runtime_kv_import_device_profiles(), 2);
    assert_eq!(passing.runtime_kv_import_devices_csv(), "cpu+integrated");
}

#[test]
fn gate_reports_missing_runtime_kv_storage() {
    let summary = BenchmarkSummary {
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![BenchmarkCaseResult {
            name: "runtime_storage".to_owned(),
            profile: TaskProfile::Coding,
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
            runtime_forward_signal: true,
            runtime_forward_energy_signal: true,
            runtime_kv_influence_signal: true,
            runtime_global_layers: 0,
            runtime_local_window_layers: 0,
            runtime_convolutional_fusion_layers: 0,
            runtime_layer_mode_signal: false,
            runtime_all_layer_modes_signal: false,
            runtime_token_count: 1,
            runtime_uncertainty_token_count: 1,
            runtime_uncertainty_signal: true,
            runtime_kv_imported: 1,
            runtime_kv_exported: 2,
            runtime_kv_stored: 0,
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_adapter_contract_ok: true,
            runtime_adapter_contract_violations: 0,
            runtime_adapter_observations: 0,
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
    gate.min_runtime_kv_stored = Some(1);
    gate.min_runtime_kv_stored_device_profiles = Some(1);

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.total_runtime_kv_stored(), 0);
    assert_eq!(summary.runtime_kv_stored_device_profiles(), 0);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_stored"))
    );
    assert!(report.failures.iter().any(|failure| {
        failure.contains("runtime_kv_stored_device_profiles") && failure.contains("devices=none")
    }));

    let passing = BenchmarkSummary {
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![BenchmarkCaseResult {
            runtime_kv_stored: 2,
            ..summary.results[0].clone()
        }],
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(passing.total_runtime_kv_stored(), 2);
    assert_eq!(passing.runtime_kv_stored_device_profiles(), 1);
    assert_eq!(passing.runtime_kv_stored_devices_csv(), "cpu");
    assert!(passing.summary_line().contains("runtime_kv_stored=2"));
    assert!(
        passing
            .summary_line()
            .contains("runtime_kv_stored_device_profiles=1")
    );
    assert!(
        passing
            .summary_line()
            .contains("runtime_kv_stored_devices=cpu")
    );
}

#[test]
fn gate_reports_missing_runtime_kv_stored_device_profile_coverage() {
    let base = BenchmarkCaseResult {
        name: "runtime_storage".to_owned(),
        profile: TaskProfile::Coding,
        device: DeviceClass::CpuOnly,
        elapsed_ms: 1,
        quality: 0.7,
        process_reward: 0.7,
        attention_fraction: 0.03,
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
        stored_memories: 1,
        compacted_memories: 0,
        runtime_forward_signal: true,
        runtime_forward_energy_signal: true,
        runtime_kv_influence_signal: true,
        runtime_global_layers: 0,
        runtime_local_window_layers: 0,
        runtime_convolutional_fusion_layers: 0,
        runtime_layer_mode_signal: false,
        runtime_all_layer_modes_signal: false,
        runtime_token_count: 1,
        runtime_uncertainty_token_count: 1,
        runtime_uncertainty_signal: true,
        runtime_kv_imported: 1,
        runtime_kv_exported: 2,
        runtime_kv_stored: 1,
        runtime_selected_adapter: Some("portable-rust".to_owned()),
        runtime_adapter_contract_ok: true,
        runtime_adapter_contract_violations: 0,
        runtime_adapter_observations: 0,
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
        results: vec![base.clone()],
        ..BenchmarkSummary::default()
    };
    let mut gate = BenchmarkGate::default();
    gate.min_runtime_kv_stored_device_profiles = Some(2);

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.runtime_kv_stored_device_profiles(), 1);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("runtime_kv_stored_device_profiles 1 below minimum 2")
            && failure.contains("devices=cpu")
    }));

    let passing = BenchmarkSummary {
        results: vec![
            base.clone(),
            BenchmarkCaseResult {
                device: DeviceClass::IntegratedGpu,
                ..base
            },
        ],
        ..BenchmarkSummary::default()
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(passing.runtime_kv_stored_device_profiles(), 2);
    assert_eq!(passing.runtime_kv_stored_devices_csv(), "cpu+integrated");
}

#[test]
fn gate_reports_missing_runtime_kv_hold_evidence() {
    let summary = BenchmarkSummary {
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![BenchmarkCaseResult {
            name: "runtime_hold".to_owned(),
            profile: TaskProfile::Coding,
            device: DeviceClass::CpuOnly,
            elapsed_ms: 1,
            quality: 0.7,
            process_reward: 0.7,
            attention_fraction: 0.03,
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
            stored_memories: 1,
            compacted_memories: 0,
            runtime_forward_signal: true,
            runtime_forward_energy_signal: true,
            runtime_kv_influence_signal: true,
            runtime_global_layers: 0,
            runtime_local_window_layers: 0,
            runtime_convolutional_fusion_layers: 0,
            runtime_layer_mode_signal: false,
            runtime_all_layer_modes_signal: false,
            runtime_token_count: 1,
            runtime_uncertainty_token_count: 1,
            runtime_uncertainty_signal: true,
            runtime_kv_imported: 0,
            runtime_kv_exported: 2,
            runtime_kv_stored: 2,
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_adapter_contract_ok: true,
            runtime_adapter_contract_violations: 0,
            runtime_adapter_observations: 0,
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
    gate.min_runtime_kv_hold_cases = Some(1);
    gate.min_runtime_kv_held = Some(1);
    gate.min_runtime_kv_hold_device_profiles = Some(1);

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.runtime_kv_hold_cases(), 0);
    assert_eq!(summary.total_runtime_kv_held(), 0);
    assert_eq!(summary.runtime_kv_hold_device_profiles(), 0);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_hold_cases"))
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_held"))
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_kv_hold_device_profiles"))
    );

    let passing = BenchmarkSummary {
        results: vec![BenchmarkCaseResult {
            runtime_kv_stored: 0,
            drift_severity: DriftSeverity::Watch,
            ..summary.results[0].clone()
        }],
        ..BenchmarkSummary::default()
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(passing.runtime_kv_hold_cases(), 1);
    assert_eq!(passing.total_runtime_kv_held(), 2);
    assert_eq!(passing.runtime_kv_hold_device_profiles(), 1);
    assert_eq!(passing.runtime_kv_hold_devices_csv(), "cpu");
    assert!(passing.summary_line().contains("runtime_kv_hold_cases=1"));
    assert!(passing.summary_line().contains("runtime_kv_held=2"));
    assert!(
        passing
            .summary_line()
            .contains("runtime_kv_hold_device_profiles=1")
    );
    assert!(
        passing
            .summary_line()
            .contains("runtime_kv_hold_devices=cpu")
    );
}

#[test]
fn gate_reports_missing_runtime_kv_hold_device_profile_coverage() {
    let base = BenchmarkCaseResult {
        name: "runtime_hold".to_owned(),
        profile: TaskProfile::Coding,
        device: DeviceClass::CpuOnly,
        elapsed_ms: 1,
        quality: 0.7,
        process_reward: 0.7,
        attention_fraction: 0.03,
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
        stored_memories: 1,
        compacted_memories: 0,
        runtime_forward_signal: true,
        runtime_forward_energy_signal: true,
        runtime_kv_influence_signal: true,
        runtime_global_layers: 0,
        runtime_local_window_layers: 0,
        runtime_convolutional_fusion_layers: 0,
        runtime_layer_mode_signal: false,
        runtime_all_layer_modes_signal: false,
        runtime_token_count: 1,
        runtime_uncertainty_token_count: 1,
        runtime_uncertainty_signal: true,
        runtime_kv_imported: 0,
        runtime_kv_exported: 2,
        runtime_kv_stored: 0,
        runtime_selected_adapter: Some("portable-rust".to_owned()),
        runtime_adapter_contract_ok: true,
        runtime_adapter_contract_violations: 0,
        runtime_adapter_observations: 0,
        runtime_adapter_best_score: None,
        runtime_adapter_best_adapter: None,
        runtime_adapter_selection_mismatches: 0,
        query_embedding_source: "fallback".to_owned(),
        query_embedding_dimensions: 64,
        runtime_embedding_calls: 0,
        fallback_embedding_calls: 1,
        embedding_fallback_used: true,
        drift_severity: DriftSeverity::Watch,
    };
    let summary = BenchmarkSummary {
        results: vec![base.clone()],
        ..BenchmarkSummary::default()
    };
    let mut gate = BenchmarkGate::default();
    gate.min_runtime_kv_hold_device_profiles = Some(2);

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.runtime_kv_hold_device_profiles(), 1);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("runtime_kv_hold_device_profiles 1 below minimum 2")
            && failure.contains("devices=cpu")
    }));

    let passing = BenchmarkSummary {
        results: vec![
            base.clone(),
            BenchmarkCaseResult {
                device: DeviceClass::IntegratedGpu,
                ..base
            },
        ],
        ..BenchmarkSummary::default()
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(passing.runtime_kv_hold_device_profiles(), 2);
    assert_eq!(passing.runtime_kv_hold_devices_csv(), "cpu+integrated");
}
