use super::*;

#[test]
fn gate_reports_runtime_adapter_contract_failures() {
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        improvement_corpus_evidence: BenchmarkImprovementCorpusEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        routing_evidence: BenchmarkRoutingEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![
            BenchmarkCaseResult {
                name: "contract_ok".to_owned(),
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
                runtime_kv_imported: 1,
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
            },
            BenchmarkCaseResult {
                name: "contract_bad".to_owned(),
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
                runtime_selected_adapter: Some("cuda".to_owned()),
                runtime_adapter_contract_ok: false,
                runtime_adapter_contract_violations: 1,
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
            },
        ],
    };
    let mut gate = BenchmarkGate::default();
    gate.min_runtime_adapter_contract_cases = Some(2);
    gate.min_runtime_adapter_kinds = Some(2);
    gate.max_runtime_adapter_contract_violations = Some(0);

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.runtime_adapter_contract_cases(), 1);
    assert_eq!(summary.runtime_adapter_kinds(), 1);
    assert_eq!(summary.total_runtime_adapter_contract_violations(), 1);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_adapter_contract_cases"))
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_adapter_kinds"))
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_adapter_contract_violations"))
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_adapter_contract_cases=1")
    );
    assert!(summary.summary_line().contains("runtime_adapter_kinds=1"));
    assert!(
        summary
            .summary_line()
            .contains("runtime_adapter_contract_violations=1")
    );
}

#[test]
fn gate_reports_runtime_adapter_kind_collapse() {
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        improvement_corpus_evidence: BenchmarkImprovementCorpusEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        routing_evidence: BenchmarkRoutingEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![
            BenchmarkCaseResult {
                name: "cpu".to_owned(),
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
            },
            BenchmarkCaseResult {
                name: "gpu".to_owned(),
                device: DeviceClass::DiscreteGpu,
                ..BenchmarkCaseResult {
                    name: "template".to_owned(),
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
                }
            },
        ],
    };
    let mut gate = BenchmarkGate::default();
    gate.min_runtime_adapter_kinds = Some(2);

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.runtime_adapter_kinds(), 1);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_adapter_kinds"))
    );

    let passing = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        improvement_corpus_evidence: BenchmarkImprovementCorpusEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        routing_evidence: BenchmarkRoutingEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        results: vec![
            summary.results[0].clone(),
            BenchmarkCaseResult {
                runtime_selected_adapter: Some("cuda".to_owned()),
                ..summary.results[1].clone()
            },
        ],
        ..summary.clone()
    };

    assert_eq!(passing.runtime_adapter_kinds(), 2);
    assert!(passing.evaluate(&gate).passed);
}

#[test]
fn gate_reports_runtime_adapter_cache_mode_coverage() {
    let summary = BenchmarkSummary {
        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence {
            runtime_adapter_cache_mode_cases: 2,
            adapter_cache_modes: vec!["no_cache".to_owned(), "chunked_cache".to_owned()],
            ..BenchmarkRuntimeDeviceExecutionEvidence::default()
        },
        results: vec![
            baseline_benchmark_result("no_cache", TaskProfile::Coding, DeviceClass::CpuOnly),
            baseline_benchmark_result("chunked_cache", TaskProfile::Coding, DeviceClass::CpuOnly),
        ],
        ..BenchmarkSummary::new()
    };
    let gate = BenchmarkGate {
        min_runtime_adapter_cache_modes: Some(3),
        ..BenchmarkGate::default()
    };

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.runtime_adapter_cache_mode_cases(), 2);
    assert_eq!(summary.runtime_adapter_cache_modes(), 2);
    assert_eq!(
        summary.runtime_adapter_cache_modes_csv(),
        "no_cache+chunked_cache"
    );
    assert!(report.failures.iter().any(|failure| {
        failure.contains("runtime_adapter_cache_modes 2 below minimum 3")
            && failure.contains("modes=no_cache+chunked_cache")
    }));
    assert!(
        summary
            .summary_line()
            .contains("runtime_adapter_cache_mode_cases=2")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_adapter_cache_modes=2")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_adapter_cache_mode_values=no_cache+chunked_cache")
    );

    let passing = BenchmarkSummary {
        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence {
            runtime_adapter_cache_mode_cases: 3,
            adapter_cache_modes: vec![
                "no_cache".to_owned(),
                "chunked_cache".to_owned(),
                "genome_filtered".to_owned(),
            ],
            ..BenchmarkRuntimeDeviceExecutionEvidence::default()
        },
        results: vec![
            baseline_benchmark_result("no_cache", TaskProfile::Coding, DeviceClass::CpuOnly),
            baseline_benchmark_result("chunked_cache", TaskProfile::Coding, DeviceClass::CpuOnly),
            baseline_benchmark_result("genome_filtered", TaskProfile::Coding, DeviceClass::CpuOnly),
        ],
        ..BenchmarkSummary::new()
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(passing.runtime_adapter_cache_mode_cases(), 3);
    assert_eq!(passing.runtime_adapter_cache_modes(), 3);
    assert_eq!(
        passing.runtime_adapter_cache_modes_csv(),
        "no_cache+chunked_cache+genome_filtered"
    );
    assert!(
        passing
            .summary_line()
            .contains("runtime_adapter_cache_mode_values=no_cache+chunked_cache+genome_filtered")
    );
}

#[test]
fn gate_reports_runtime_adapter_stream_trace_and_gate_summary_coverage() {
    let summary = BenchmarkSummary {
        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence {
            runtime_adapter_stream_trace_cases: 1,
            runtime_adapter_stream_gate_summary_cases: 0,
            ..BenchmarkRuntimeDeviceExecutionEvidence::default()
        },
        results: vec![baseline_benchmark_result(
            "stream_trace_only",
            TaskProfile::Coding,
            DeviceClass::CpuOnly,
        )],
        ..BenchmarkSummary::new()
    };
    let gate = BenchmarkGate {
        min_runtime_adapter_stream_trace_cases: Some(2),
        min_runtime_adapter_stream_gate_summary_cases: Some(1),
        ..BenchmarkGate::default()
    };

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.runtime_adapter_stream_trace_cases(), 1);
    assert_eq!(summary.runtime_adapter_stream_gate_summary_cases(), 0);
    assert!(report.failures.iter().any(|failure| {
        failure.contains("runtime_adapter_stream_trace_cases 1 below minimum 2")
    }));
    assert!(report.failures.iter().any(|failure| {
        failure.contains("runtime_adapter_stream_gate_summary_cases 0 below minimum 1")
    }));
    assert!(
        summary
            .summary_line()
            .contains("runtime_adapter_stream_trace_cases=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_adapter_stream_gate_summary_cases=0")
    );

    let passing = BenchmarkSummary {
        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence {
            runtime_adapter_stream_trace_cases: 2,
            runtime_adapter_stream_gate_summary_cases: 2,
            ..BenchmarkRuntimeDeviceExecutionEvidence::default()
        },
        results: vec![
            baseline_benchmark_result("stream_a", TaskProfile::Coding, DeviceClass::CpuOnly),
            baseline_benchmark_result("stream_b", TaskProfile::Coding, DeviceClass::CpuOnly),
        ],
        ..BenchmarkSummary::new()
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(passing.runtime_adapter_stream_trace_cases(), 2);
    assert_eq!(passing.runtime_adapter_stream_gate_summary_cases(), 2);
}

#[test]
fn gate_reports_missing_runtime_adapter_observations() {
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        improvement_corpus_evidence: BenchmarkImprovementCorpusEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        routing_evidence: BenchmarkRoutingEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![BenchmarkCaseResult {
            name: "runtime_adapter_observation".to_owned(),
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
    gate.min_runtime_adapter_observations = Some(1);
    gate.min_runtime_adapter_best_score = Some(0.25);

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.total_runtime_adapter_observations(), 0);
    assert_eq!(summary.max_runtime_adapter_score(), None);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_adapter_observations"))
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_adapter_best_score"))
    );

    let passing = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        improvement_corpus_evidence: BenchmarkImprovementCorpusEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        routing_evidence: BenchmarkRoutingEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![BenchmarkCaseResult {
            runtime_adapter_observations: 2,
            runtime_adapter_best_score: Some(0.51),
            ..summary.results[0].clone()
        }],
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(passing.total_runtime_adapter_observations(), 2);
    assert_eq!(passing.max_runtime_adapter_score(), Some(0.51));
    assert!(
        passing
            .summary_line()
            .contains("runtime_adapter_observations=2")
    );
    assert!(
        passing
            .summary_line()
            .contains("runtime_adapter_best_score=0.510")
    );
}

#[test]
fn gate_reports_runtime_adapter_selection_mismatches() {
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        improvement_corpus_evidence: BenchmarkImprovementCorpusEvidence::default(),
        reflection_evidence: BenchmarkReflectionEvidence::default(),
        live_evolution_evidence: BenchmarkLiveEvolutionEvidence::default(),
        routing_evidence: BenchmarkRoutingEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence::default(),
        embedding_evidence: BenchmarkEmbeddingEvidence::default(),

        runtime_device_execution_evidence: BenchmarkRuntimeDeviceExecutionEvidence::default(),

        runtime_architecture_evidence: BenchmarkRuntimeArchitectureEvidence::default(),
        evolution_ledger: EvolutionLedger::default(),
        results: vec![BenchmarkCaseResult {
            name: "runtime_adapter_selection".to_owned(),
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
            runtime_kv_exported: 1,
            runtime_kv_stored: 1,
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_adapter_contract_ok: true,
            runtime_adapter_contract_violations: 0,
            runtime_adapter_observations: 1,
            runtime_adapter_best_score: Some(0.80),
            runtime_adapter_best_adapter: Some("cpu-simd".to_owned()),
            runtime_adapter_selection_mismatches: 1,
            query_embedding_source: "fallback".to_owned(),
            query_embedding_dimensions: 64,
            runtime_embedding_calls: 0,
            fallback_embedding_calls: 1,
            embedding_fallback_used: true,
            drift_severity: DriftSeverity::Stable,
        }],
    };
    let mut gate = BenchmarkGate::default();
    gate.max_runtime_adapter_selection_mismatches = Some(0);

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    assert_eq!(summary.total_runtime_adapter_selection_mismatches(), 1);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("runtime_adapter_selection_mismatches"))
    );
    assert!(
        summary
            .summary_line()
            .contains("runtime_adapter_selection_mismatches=1")
    );

    let passing = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        improvement_corpus_evidence: BenchmarkImprovementCorpusEvidence::default(),
        results: vec![BenchmarkCaseResult {
            runtime_adapter_best_adapter: Some("portable-rust".to_owned()),
            runtime_adapter_selection_mismatches: 0,
            ..summary.results[0].clone()
        }],
        ..summary
    };
    let passing_report = passing.evaluate(&gate);

    assert!(passing_report.passed, "{:?}", passing_report.failures);
    assert_eq!(passing.total_runtime_adapter_selection_mismatches(), 0);
}
