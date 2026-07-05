use super::*;
use crate::engine::{
    GenerationContext, HeuristicBackend, InferenceBackend, InferenceRequest, NoironEngine,
};
use crate::reasoning_genome::{
    GenomeExpressionInput, ReasoningGene, ReasoningGeneKind, ReasoningGenome,
};
use crate::reflection::{InferenceDraft, ReasoningStep};

#[test]
fn summary_records_reasoning_genome_expression_evidence() {
    let mut engine = NoironEngine::new();
    engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
        DeviceClass::CpuOnly,
        0.25,
        0.0,
        0.35,
        0.15,
    ));
    let mut backend = HeuristicBackend;
    let case = BenchmarkCase::new(
        "reasoning_genome",
        TaskProfile::Coding,
        "Use Rust tests to validate a Noiron reasoning genome chain.",
    );
    let outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    let mut summary = BenchmarkSummary::new();

    summary.record(&case, 5, &outcome);

    assert_eq!(summary.reasoning_genome_expression_cases(), 1);
    assert_eq!(summary.reasoning_genome_expression_device_profiles(), 1);
    assert_eq!(summary.reasoning_genome_splice_cases(), 1);
    assert_eq!(summary.reasoning_genome_splice_device_profiles(), 1);
    assert_eq!(summary.total_reasoning_genome_failures(), 0);
    assert!(summary.genome_evidence().total_genes >= 7);
    assert!(summary.genome_evidence().total_splice_segments >= 1);
    assert!(summary.genome_evidence().total_splice_retained >= 1);
    assert!(summary.genome_evidence().total_splice_input_tokens >= 1);
    assert!(summary.total_reasoning_genome_lifecycle_records() >= 7);
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_expression_cases=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_splice_cases=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_splice_retained=")
    );
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_splice_saved_tokens=")
    );
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_splice_lifecycle_records=")
    );
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_failures=0")
    );
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_repair_payloads=0")
    );
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_regeneration_payloads=0")
    );
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_lifecycle_records=")
    );
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_lifecycle_tombstone_candidates=")
    );
    for marker in [
        "selected_chunks=",
        "skipped_chunks=",
        "kv_reuse=",
        "genome_decisions=",
        "quality_proxy=",
    ] {
        assert!(summary.summary_line().contains(marker), "missing {marker}");
    }

    let report = summary.evaluate(&BenchmarkGate {
        min_reasoning_genome_expression_cases: Some(1),
        min_reasoning_genome_expression_device_profiles: Some(1),
        min_reasoning_genome_splice_cases: Some(1),
        min_reasoning_genome_splice_device_profiles: Some(1),
        ..BenchmarkGate::default()
    });

    assert!(report.passed, "{:?}", report.failures);
}

#[test]
fn summary_gates_reasoning_genome_repair_and_regeneration_payloads() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let case = BenchmarkCase::new(
        "reasoning_genome_payloads",
        TaskProfile::Coding,
        "Force a Reasoning Genome repair payload audit.",
    );
    let mut outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    outcome.reasoning_genome = ReasoningGenome::new(
        "genome:coding:v1",
        TaskProfile::Coding,
        "genome:coding:stable",
        vec![
            ReasoningGene::new(
                "gene:coding:retrieval",
                ReasoningGeneKind::Retrieval,
                "",
                "retrieve useful memory",
            )
            .with_health(12, 0.74, 0.04),
            ReasoningGene::new(
                "gene:coding:safety",
                ReasoningGeneKind::Safety,
                "unsafe drift guard",
                "this safety behavior drifted",
            )
            .with_health(1, 0.20, 0.91),
        ],
    )
    .express(GenomeExpressionInput {
        profile: TaskProfile::Coding,
        quality: 0.42,
        process_reward: 0.38,
        contradiction_count: 1,
        critical_reflection_issue_count: 1,
        revision_action_count: 1,
        used_memories: 0,
        memory_feedback_updates: 0,
        route_attention_fraction: 0.50,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: false,
        genome_mutation_allowed: true,
        drift_rollback: false,
        runtime_kv_hold: false,
    });
    let mut summary = BenchmarkSummary::new();

    summary.record(&case, 5, &outcome);

    assert_eq!(summary.total_reasoning_genome_repair_payloads(), 2);
    assert_eq!(summary.total_reasoning_genome_regeneration_payloads(), 1);
    assert_eq!(summary.total_reasoning_genome_tombstone_candidates(), 1);
    assert_eq!(summary.dna_evolution_reports(), 2);
    assert!(summary.dna_evolution_candidates() >= 2);
    assert!(summary.dna_evolution_candidate_previews() >= 2);
    assert_eq!(
        summary.dna_evolution_candidate_ledger_records(),
        summary.dna_evolution_candidates()
    );
    assert_eq!(
        summary.dna_evolution_candidate_ledger_preview_only(),
        summary.dna_evolution_candidate_ledger_records()
    );
    assert_eq!(summary.dna_evolution_activation_eligible(), 0);
    assert!(summary.dna_evolution_transaction_replays() >= summary.dna_evolution_candidates());
    assert_eq!(summary.dna_evolution_replay_passed(), 2);
    assert_eq!(summary.dna_evolution_validation_passed(), 2);
    assert_eq!(summary.dna_evolution_writer_gate_reports(), 2);
    assert_eq!(summary.dna_evolution_writer_gate_holds(), 2);
    assert_eq!(summary.dna_evolution_writer_gate_ready(), 0);
    assert_eq!(
        summary.dna_evolution_writer_gate_explicit_apply_required(),
        2
    );
    assert_eq!(summary.dna_evolution_writer_gate_durable_write_allowed(), 0);
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_repair_payloads=2")
    );
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_regeneration_payloads=1")
    );
    assert!(
        summary
            .summary_line()
            .contains("reasoning_genome_lifecycle_tombstone_candidates=1")
    );
    assert!(summary.summary_line().contains("dna_evolution_reports=2"));
    assert!(
        summary
            .summary_line()
            .contains("dna_evolution_candidate_ledger_records=")
    );
    assert!(
        summary
            .summary_line()
            .contains("dna_evolution_candidate_ledger_preview_only=")
    );
    assert!(
        summary
            .summary_line()
            .contains("dna_evolution_activation_eligible=0")
    );
    assert!(
        summary
            .summary_line()
            .contains("dna_evolution_writer_gate_reports=2")
    );
    assert!(
        summary
            .summary_line()
            .contains("dna_evolution_writer_gate_durable_write_allowed=0")
    );
    let report = summary.evaluate(&BenchmarkGate {
        min_reasoning_genome_repair_payloads: Some(2),
        min_reasoning_genome_regeneration_payloads: Some(1),
        min_dna_evolution_reports: Some(2),
        min_dna_evolution_candidates: Some(2),
        min_dna_evolution_candidate_previews: Some(2),
        min_dna_evolution_candidate_ledger_records: Some(2),
        min_dna_evolution_candidate_ledger_preview_only: Some(2),
        max_dna_evolution_activation_eligible: Some(0),
        min_dna_evolution_transaction_replays: Some(2),
        min_dna_evolution_replay_passed: Some(2),
        min_dna_evolution_validation_passed: Some(2),
        min_dna_evolution_writer_gate_reports: Some(2),
        min_dna_evolution_writer_gate_holds: Some(2),
        min_dna_evolution_writer_gate_explicit_apply_required: Some(2),
        max_dna_evolution_writer_gate_ready: Some(0),
        max_dna_evolution_writer_gate_durable_write_allowed: Some(0),
        ..BenchmarkGate::default()
    });

    assert!(report.passed, "{:?}", report.failures);
}

#[test]
fn summary_gates_live_reasoning_genome_payloads_from_feedback() {
    struct CriticalBackend;

    impl InferenceBackend for CriticalBackend {
        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            InferenceDraft::new("", vec![ReasoningStep::new("runtime", "empty", 0.0)])
        }
    }

    let mut engine = NoironEngine::new();
    let mut backend = CriticalBackend;
    let case = BenchmarkCase::new(
        "reasoning_genome_live_payloads",
        TaskProfile::Coding,
        "Trigger live Reasoning Genome feedback repair payloads.",
    );
    let outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    let mut summary = BenchmarkSummary::new();

    summary.record(&case, 5, &outcome);

    assert!(summary.total_reasoning_genome_repair_payloads() >= 1);
    assert!(summary.total_reasoning_genome_regeneration_payloads() >= 1);
    let report = summary.evaluate(&BenchmarkGate {
        min_average_quality: 0.0,
        min_average_reward: 0.0,
        max_evolution_drift_rollbacks: None,
        max_evolution_rollback_router_threshold_delta: None,
        max_evolution_rollback_hierarchy_weight_delta: None,
        min_reasoning_genome_repair_payloads: Some(1),
        min_reasoning_genome_regeneration_payloads: Some(1),
        max_drift_rollbacks: None,
        ..BenchmarkGate::default()
    });

    assert!(report.passed, "{:?}", report.failures);
}

#[test]
fn summary_gates_mutation_fixture_and_malignant_recovery_corpus() {
    let mut engine = NoironEngine::new();
    let mut backend = HeuristicBackend;
    let case = BenchmarkCase::new(
        "mutation_fixture_gate",
        TaskProfile::Coding,
        "Validate mutation repair fixtures and malignant recovery drills.",
    );
    let outcome = engine.infer(
        InferenceRequest::new(case.prompt.clone(), case.profile),
        &mut backend,
    );
    let mut summary = BenchmarkSummary::new();

    summary.record(&case, 5, &outcome);

    assert_eq!(summary.mutation_repair_fixtures(), 8);
    assert_eq!(summary.mutation_repair_fixture_kinds(), 8);
    assert!(summary.mutation_repair_candidates() >= 7);
    assert!(summary.mutation_repair_review_packets() >= 8);
    assert_eq!(summary.malignant_gene_recovery_drills(), 6);
    assert_eq!(summary.malignant_gene_quarantines(), 6);
    assert_eq!(summary.malignant_gene_cut_candidates(), 6);
    assert_eq!(summary.malignant_gene_regeneration_candidates(), 6);
    assert_eq!(summary.malignant_gene_failed_replay(), 1);
    assert_eq!(summary.total_reasoning_genome_failures(), 0);
    assert!(
        summary
            .summary_line()
            .contains("mutation_repair_fixtures=8")
    );
    assert!(
        summary
            .summary_line()
            .contains("malignant_gene_recovery_drills=6")
    );

    let report = summary.evaluate(&BenchmarkGate {
        min_mutation_repair_fixtures: Some(8),
        min_mutation_repair_fixture_kinds: Some(8),
        min_mutation_repair_candidates: Some(7),
        min_mutation_repair_review_packets: Some(8),
        min_malignant_gene_recovery_drills: Some(6),
        min_malignant_gene_quarantines: Some(6),
        min_malignant_gene_cut_candidates: Some(6),
        min_malignant_gene_regeneration_candidates: Some(6),
        ..BenchmarkGate::default()
    });

    assert!(report.passed, "{:?}", report.failures);

    let failing = summary.evaluate(&BenchmarkGate {
        min_mutation_repair_fixture_kinds: Some(9),
        min_malignant_gene_regeneration_candidates: Some(7),
        ..BenchmarkGate::default()
    });

    assert!(!failing.passed);
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| failure.contains("mutation_repair_fixture_kinds"))
    );
    assert!(
        failing
            .failures
            .iter()
            .any(|failure| failure.contains("malignant_gene_regeneration_candidates"))
    );
}

#[test]
fn gate_reports_missing_reasoning_genome_and_gene_scissors_coverage() {
    let summary = BenchmarkSummary::new();
    let gate = BenchmarkGate {
        min_reasoning_genome_expression_cases: Some(1),
        min_reasoning_genome_expression_device_profiles: Some(1),
        min_reasoning_genome_splice_cases: Some(1),
        min_reasoning_genome_splice_device_profiles: Some(1),
        min_gene_scissors_proposal_cases: Some(1),
        min_gene_scissors_proposal_device_profiles: Some(1),
        min_reasoning_genome_repair_payloads: Some(1),
        min_reasoning_genome_regeneration_payloads: Some(1),
        min_dna_evolution_reports: Some(1),
        min_dna_evolution_candidates: Some(1),
        min_dna_evolution_candidate_previews: Some(1),
        min_dna_evolution_candidate_ledger_records: Some(1),
        min_dna_evolution_candidate_ledger_preview_only: Some(1),
        min_dna_evolution_transaction_replays: Some(1),
        min_dna_evolution_replay_passed: Some(1),
        min_dna_evolution_validation_passed: Some(1),
        min_dna_evolution_writer_gate_reports: Some(1),
        min_dna_evolution_writer_gate_holds: Some(1),
        min_dna_evolution_writer_gate_explicit_apply_required: Some(1),
        ..BenchmarkGate::default()
    };

    let report = summary.evaluate(&gate);

    assert!(!report.passed);
    for marker in [
        "reasoning_genome_expression_cases",
        "reasoning_genome_expression_device_profiles",
        "reasoning_genome_splice_cases",
        "reasoning_genome_splice_device_profiles",
        "gene_scissors_proposal_cases",
        "gene_scissors_proposal_device_profiles",
        "reasoning_genome_repair_payloads",
        "reasoning_genome_regeneration_payloads",
        "dna_evolution_reports",
        "dna_evolution_candidates",
        "dna_evolution_candidate_previews",
        "dna_evolution_candidate_ledger_records",
        "dna_evolution_candidate_ledger_preview_only",
        "dna_evolution_transaction_replays",
        "dna_evolution_replay_passed",
        "dna_evolution_validation_passed",
        "dna_evolution_writer_gate_reports",
        "dna_evolution_writer_gate_holds",
        "dna_evolution_writer_gate_explicit_apply_required",
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
