use super::*;
use crate::engine::{HeuristicBackend, InferenceRequest, NoironEngine};
use crate::kv_cache::{KvFusionCache, MemoryCompactionPolicy, MemoryRetentionPolicy};

#[test]
fn summary_records_memory_governance_evidence() {
    let mut engine = NoironEngine::new();
    engine.cache = KvFusionCache::with_limits(0.99, 4096);
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.25,
        0.0,
        0.35,
        0.15,
    ));
    engine.set_memory_retention_policy(MemoryRetentionPolicy {
        stale_after: 1,
        decay_rate: 0.50,
        remove_below_strength: 0.15,
        remove_after_failures: 1,
    });
    let compaction_policy = MemoryCompactionPolicy {
        similarity_threshold: 0.90,
        max_candidates: 8,
        max_merges: 2,
    };
    engine.set_memory_compaction_policy(compaction_policy.clone());
    let weak_id =
        engine
            .cache
            .store_or_fuse("benchmark_governance:weak", vec![1.0, 0.0, 0.0, 0.0], 0.05);
    engine.cache.penalize(weak_id, 1.0);
    let stale_unprotected_id = engine.cache.store_or_fuse(
        "benchmark_governance:retention_only",
        vec![0.0, 0.0, 0.0, 0.0],
        0.02,
    );
    engine.cache.penalize(stale_unprotected_id, 0.70);
    for index in 0..8 {
        engine.cache.store_or_fuse(
            format!("benchmark_governance:clock_tick:{index}"),
            vec![0.0, 1.0, index as f32, 0.0],
            0.20,
        );
    }
    let mut backend = HeuristicBackend;
    let case = BenchmarkCase::new(
        "memory_governance",
        TaskProfile::General,
        "Audit Noiron memory governance retention and compaction evidence.",
    );
    let mut compaction_cache = KvFusionCache::with_limits(0.99, 4096);
    compaction_cache.store_or_fuse(
        "benchmark_governance:compact_a",
        vec![0.0, 1.0, 0.0, 0.0],
        0.70,
    );
    compaction_cache.store_or_fuse(
        "benchmark_governance:compact_b",
        vec![0.0, 0.96, 0.28, 0.0],
        0.70,
    );
    let compaction_report = compaction_cache.compact_similar(compaction_policy.clone());
    let mut outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    outcome.memory_compaction_policy = compaction_policy;
    outcome.memory_compaction_report = compaction_report;
    let mut summary = BenchmarkSummary::new();

    summary.record(&case, 5, &outcome);

    assert_eq!(summary.memory_governance_cases(), 1);
    assert_eq!(summary.memory_governance_device_profiles(), 1);
    assert_eq!(
        summary.memory_governance_evidence().failures.len(),
        0,
        "{:?}",
        summary.memory_governance_evidence().failures
    );
    assert_eq!(summary.memory_admission_cases(), 1);
    assert_eq!(summary.memory_admission_device_profiles(), 1);
    assert!(summary.total_memory_admission_candidates() >= 1);
    assert_eq!(summary.total_memory_admission_admitted(), 0);
    assert_eq!(
        summary.total_memory_admission_review_packets(),
        summary.total_memory_admission_candidates()
    );
    assert_eq!(
        summary.total_memory_admission_ledger_records(),
        summary.total_memory_admission_candidates()
    );
    assert_eq!(summary.total_memory_admission_ledger_authorized(), 0);
    assert_eq!(summary.total_memory_admission_ledger_applied(), 0);
    assert_eq!(summary.kv_fusion_cases(), 1);
    assert!(summary.total_kv_fusion_candidates() >= 1);
    assert_eq!(
        summary
            .memory_governance_evidence()
            .kv_fusion_decision_total(),
        summary.total_kv_fusion_candidates()
    );
    assert_eq!(
        summary
            .total_kv_fusion_retained_tokens()
            .saturating_add(summary.total_kv_fusion_saved_tokens()),
        summary.total_kv_fusion_input_tokens()
    );
    assert!(summary.total_kv_fusion_saved_tokens() > 0);
    assert!(summary.total_memory_retention_decayed() >= 1);
    assert!(summary.total_memory_retention_removed() >= 1);
    assert_eq!(summary.total_memory_compaction_pair_evidence(), 1);
    assert_eq!(summary.memory_storage_benchmark_samples(), 1);
    assert!(summary.total_memory_storage_entries_before() >= 1);
    assert!(summary.total_memory_storage_entries_removed() >= 1);
    assert_eq!(summary.memory_retrieval_latency_samples(), 1);
    assert_eq!(summary.total_memory_retrieval_latency_ms(), 5);
    assert_eq!(summary.average_memory_retrieval_latency_ms(), 5);
    assert_eq!(summary.max_memory_retrieval_latency_ms(), 5);
    assert!(summary.summary_line().contains("memory_governance_cases=1"));
    assert!(
        summary
            .summary_line()
            .contains("memory_governance_failures=0")
    );
    assert!(summary.summary_line().contains("memory_admission_cases=1"));
    assert!(summary.summary_line().contains("memory_admission_ready="));
    assert!(summary.summary_line().contains("memory_admission_blocked="));
    assert!(
        summary
            .summary_line()
            .contains("memory_admission_admitted=0")
    );
    assert!(
        summary
            .summary_line()
            .contains("memory_admission_review_packets=")
    );
    assert!(
        summary
            .summary_line()
            .contains("memory_admission_ledger_records=")
    );
    assert!(
        summary
            .summary_line()
            .contains("memory_admission_ledger_authorized=0")
    );
    assert!(summary.summary_line().contains("kv_fusion_cases=1"));
    assert!(summary.summary_line().contains("kv_fusion_candidates="));
    assert!(summary.summary_line().contains("kv_fusion_saved_tokens="));
    assert!(
        summary
            .summary_line()
            .contains("memory_retention_activity_cases=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("memory_compaction_pair_evidence=1")
    );
    assert!(summary.summary_line().contains("memory_storage_samples=1"));
    assert!(
        summary
            .summary_line()
            .contains("memory_storage_entries_removed=")
    );
    assert!(
        summary
            .summary_line()
            .contains("memory_retrieval_latency_samples=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("memory_retained_usefulness_delta_milli=")
    );

    let gate = BenchmarkGate {
        min_memory_governance_cases: Some(1),
        min_memory_governance_device_profiles: Some(1),
        min_kv_fusion_cases: Some(1),
        min_kv_fusion_candidates: Some(1),
        min_kv_fusion_saved_tokens: Some(1),
        min_memory_retention_activity_cases: Some(1),
        min_memory_storage_benchmark_samples: Some(1),
        min_memory_storage_removed_entries: Some(1),
        min_memory_retrieval_latency_samples: Some(1),
        max_memory_retrieval_latency_avg_ms: Some(5),
        ..BenchmarkGate::default()
    };

    let passing = summary.evaluate(&gate);

    assert!(passing.passed, "{:?}", passing.failures);
}

#[test]
fn gate_accepts_memory_governance_activity_evidence() {
    let result = baseline_benchmark_result(
        "memory_governance_activity",
        TaskProfile::General,
        DeviceClass::CpuOnly,
    );
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        improvement_corpus_evidence: BenchmarkImprovementCorpusEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence {
            cases: 2,
            retention_activity_cases: 1,
            compaction_activity_cases: 1,
            total_retention_decayed: 2,
            total_retention_removed: 1,
            total_compaction_merged: 1,
            total_compaction_removed: 1,
            total_compaction_pair_evidence: 1,
            memory_storage_samples: 2,
            memory_storage_entries_before: 8,
            memory_storage_entries_after: 5,
            memory_storage_entries_removed: 3,
            memory_retrieval_latency_samples: 2,
            total_memory_retrieval_latency_ms: 11,
            max_memory_retrieval_latency_ms: 7,
            memory_retained_usefulness_delta_milli: 125,
            memory_retained_usefulness_abs_delta_milli: 175,
            governance_devices: vec![DeviceClass::CpuOnly, DeviceClass::IntegratedGpu],
            retention_activity_devices: vec![DeviceClass::CpuOnly],
            compaction_activity_devices: vec![DeviceClass::IntegratedGpu],
            ..BenchmarkMemoryGovernanceEvidence::default()
        },
        results: vec![
            result.clone(),
            BenchmarkCaseResult {
                device: DeviceClass::IntegratedGpu,
                ..result
            },
        ],
        ..BenchmarkSummary::default()
    };
    let gate = BenchmarkGate {
        min_memory_governance_cases: Some(2),
        min_memory_governance_device_profiles: Some(2),
        min_memory_retention_activity_cases: Some(1),
        min_memory_compaction_activity_cases: Some(1),
        min_memory_storage_benchmark_samples: Some(2),
        min_memory_storage_removed_entries: Some(3),
        min_memory_retrieval_latency_samples: Some(2),
        max_memory_retrieval_latency_avg_ms: Some(6),
        min_memory_retained_usefulness_abs_delta_milli: Some(100),
        ..BenchmarkGate::default()
    };

    let report = summary.evaluate(&gate);

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(summary.memory_governance_device_profiles(), 2);
    assert_eq!(summary.total_memory_retention_decayed(), 2);
    assert_eq!(summary.total_memory_retention_removed(), 1);
    assert_eq!(summary.total_memory_compaction_merged(), 1);
    assert_eq!(summary.total_memory_compaction_removed(), 1);
    assert_eq!(summary.total_memory_compaction_pair_evidence(), 1);
    assert_eq!(summary.memory_storage_benchmark_samples(), 2);
    assert_eq!(summary.total_memory_storage_entries_before(), 8);
    assert_eq!(summary.total_memory_storage_entries_after(), 5);
    assert_eq!(summary.total_memory_storage_entries_removed(), 3);
    assert_eq!(summary.total_memory_storage_reduction_entries(), 3);
    assert_eq!(summary.memory_retrieval_latency_samples(), 2);
    assert_eq!(summary.total_memory_retrieval_latency_ms(), 11);
    assert_eq!(summary.average_memory_retrieval_latency_ms(), 5);
    assert_eq!(summary.max_memory_retrieval_latency_ms(), 7);
    assert_eq!(summary.memory_retained_usefulness_delta_milli(), 125);
    assert_eq!(summary.memory_retained_usefulness_abs_delta_milli(), 175);
    assert!(
        summary
            .summary_line()
            .contains("memory_governance_device_profiles=2")
    );
    assert!(
        summary
            .summary_line()
            .contains("memory_compaction_activity_cases=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("memory_compaction_pair_evidence=1")
    );
}

#[test]
fn gate_reports_missing_memory_governance_coverage() {
    let summary = BenchmarkSummary::new();
    let gate = BenchmarkGate {
        min_memory_governance_cases: Some(1),
        min_memory_governance_device_profiles: Some(1),
        min_kv_fusion_cases: Some(1),
        min_kv_fusion_candidates: Some(1),
        min_kv_fusion_saved_tokens: Some(1),
        min_memory_retention_activity_cases: Some(1),
        min_memory_compaction_activity_cases: Some(1),
        min_memory_storage_benchmark_samples: Some(1),
        min_memory_storage_removed_entries: Some(1),
        min_memory_retrieval_latency_samples: Some(1),
        max_memory_retrieval_latency_avg_ms: Some(0),
        min_memory_retained_usefulness_abs_delta_milli: Some(1),
        ..BenchmarkGate::default()
    };

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    for marker in [
        "memory_governance_cases",
        "memory_governance_device_profiles",
        "kv_fusion_cases",
        "kv_fusion_candidates",
        "kv_fusion_saved_tokens",
        "memory_retention_activity_cases",
        "memory_compaction_activity_cases",
        "memory_storage_benchmark_samples",
        "memory_storage_removed_entries",
        "memory_retrieval_latency_samples",
        "memory_retained_usefulness_abs_delta_milli",
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

#[test]
fn gate_reports_memory_governance_failures() {
    let summary = BenchmarkSummary {
        genome_evidence: BenchmarkGenomeEvidence::default(),
        improvement_corpus_evidence: BenchmarkImprovementCorpusEvidence::default(),
        memory_governance_evidence: BenchmarkMemoryGovernanceEvidence {
            cases: 1,
            failures: vec!["cpu:bad retention stale_after must be > 0".to_owned()],
            ..BenchmarkMemoryGovernanceEvidence::default()
        },
        ..BenchmarkSummary::default()
    };

    let report = summary.evaluate(&BenchmarkGate::default());

    assert!(!report.passed);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.contains("memory_governance_failures"))
    );
}
