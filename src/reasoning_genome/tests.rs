use super::*;
use crate::hierarchy::TaskProfile;

#[test]
fn default_genome_expresses_read_only_profile_genes() {
    let genome = ReasoningGenome::default_for_profile(TaskProfile::Coding);
    let expression = genome.express(GenomeExpressionInput {
        profile: TaskProfile::Coding,
        quality: 0.92,
        process_reward: 0.88,
        contradiction_count: 0,
        critical_reflection_issue_count: 0,
        revision_action_count: 0,
        used_memories: 2,
        memory_feedback_updates: 1,
        route_attention_fraction: 0.42,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: true,
        drift_rollback: false,
        runtime_kv_hold: false,
    });

    assert_eq!(expression.expression_gene_count, 7);
    assert_eq!(expression.active_gene_count(), 7);
    assert_eq!(expression.aged_gene_count(), 0);
    assert_eq!(expression.malignant_gene_count(), 0);
    assert!(expression.is_read_only_preview());
    assert_eq!(expression.scissors_proposal_count(), 0);
}

#[test]
fn aging_gene_gets_relabel_plan_without_writes() {
    let genome = ReasoningGenome::new(
        "genome:test:v1",
        TaskProfile::General,
        "genome:test:stable",
        vec![
            ReasoningGene::new(
                "gene:test:retrieval",
                ReasoningGeneKind::Retrieval,
                "",
                "retrieve useful memory",
            )
            .with_health(12, 0.72, 0.05),
        ],
    );

    let expression = genome.express(GenomeExpressionInput {
        profile: TaskProfile::General,
        quality: 0.80,
        process_reward: 0.72,
        contradiction_count: 0,
        critical_reflection_issue_count: 0,
        revision_action_count: 0,
        used_memories: 0,
        memory_feedback_updates: 0,
        route_attention_fraction: 0.20,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: true,
        drift_rollback: false,
        runtime_kv_hold: false,
    });

    assert_eq!(expression.aged_gene_count(), 1);
    assert_eq!(expression.relabel_candidate_count(), 1);
    assert_eq!(expression.scissors_proposal_count(), 1);
    assert_eq!(
        expression.mutation_plans[0].intent,
        GeneScissorsIntent::Relabel
    );
    assert!(expression.is_read_only_preview());
}

#[test]
fn malignant_gene_is_quarantined_and_regenerated_from_stable_anchor() {
    let genome = ReasoningGenome::new(
        "genome:test:v1",
        TaskProfile::Coding,
        "genome:test:stable",
        vec![
            ReasoningGene::new(
                "gene:test:safety",
                ReasoningGeneKind::Safety,
                "unsafe memory admission",
                "this gene drifted",
            )
            .with_health(2, 0.12, 0.91),
        ],
    );

    let expression = genome.express(GenomeExpressionInput {
        profile: TaskProfile::Coding,
        quality: 0.30,
        process_reward: 0.20,
        contradiction_count: 2,
        critical_reflection_issue_count: 1,
        revision_action_count: 1,
        used_memories: 1,
        memory_feedback_updates: 1,
        route_attention_fraction: 0.80,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: false,
        drift_rollback: true,
        runtime_kv_hold: true,
    });

    let intents = expression.mutation_intents();
    assert_eq!(expression.malignant_gene_count(), 1);
    assert_eq!(expression.regeneration_candidate_count(), 1);
    assert!(intents.contains(&"quarantine".to_owned()));
    assert!(intents.contains(&"regenerate".to_owned()));
    assert!(intents.contains(&"rollback".to_owned()));
    assert!(expression.youth_pressure > 0.50);
    assert!(expression.is_read_only_preview());
    assert!(
        expression
            .mutation_plans
            .iter()
            .all(|plan| plan.rollback_anchor_id == "genome:test:stable")
    );
}

#[test]
fn rollback_pressure_quarantines_and_regenerates_active_safety_gene() {
    let genome = ReasoningGenome::default_for_profile(TaskProfile::Coding);

    let expression = genome.express(GenomeExpressionInput {
        profile: TaskProfile::Coding,
        quality: 0.74,
        process_reward: 0.61,
        contradiction_count: 1,
        critical_reflection_issue_count: 0,
        revision_action_count: 1,
        used_memories: 2,
        memory_feedback_updates: 1,
        route_attention_fraction: 0.55,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: false,
        drift_rollback: true,
        runtime_kv_hold: false,
    });

    let intents = expression.mutation_intents();
    assert_eq!(expression.malignant_gene_count(), 1);
    assert_eq!(expression.regeneration_candidate_count(), 1);
    assert!(intents.contains(&"quarantine".to_owned()));
    assert!(intents.contains(&"regenerate".to_owned()));
    assert!(intents.contains(&"rollback".to_owned()));
    assert!(expression.is_read_only_preview());
}

#[test]
fn dna_splicer_classifies_exons_introns_and_variants_without_writes() {
    let segments = vec![
        GeneSegment::new(
            "segment:good-retrieval",
            TaskProfile::Coding,
            GeneSegmentSource::SemanticMemory,
            0,
            64,
        )
        .with_source_hash("sha256:good")
        .with_metadata(
            "compiler evidence",
            "carry Rust compiler feedback into retrieval posture",
            "bounded compiler feedback",
        )
        .with_kv_residency(GeneKvResidency::HotRecent)
        .with_health(0.91, 0.04, 0.0),
        GeneSegment::new(
            "segment:weak-context",
            TaskProfile::Coding,
            GeneSegmentSource::GistMemory,
            64,
            96,
        )
        .with_source_hash("sha256:weak")
        .with_metadata(
            "low value gist",
            "kept as cold evidence unless retrieval quality improves",
            "low value gist",
        )
        .with_kv_residency(GeneKvResidency::ColdEvidence)
        .with_health(0.22, 0.08, 0.01),
        GeneSegment::new(
            "segment:private-drift",
            TaskProfile::Coding,
            GeneSegmentSource::RuntimeKv,
            96,
            128,
        )
        .with_source_hash("sha256:private")
        .with_metadata(
            "drifting runtime kv",
            "must be isolated before KV import",
            "runtime KV drift",
        )
        .with_kv_residency(GeneKvResidency::Sink)
        .with_health(0.78, 0.72, 0.61),
    ];

    let preview =
        DnaSplicer::default().preview(TaskProfile::Coding, "genome:coding:stable", segments);

    let intents = preview.mutation_intents();
    assert_eq!(preview.exon_count(), 1);
    assert_eq!(preview.intron_count(), 1);
    assert_eq!(preview.variant_count(), 1);
    assert!(preview.findings.iter().any(|finding| {
        finding.kind == GeneVariantKind::Drift && finding.segment_id == "segment:private-drift"
    }));
    assert!(preview.findings.iter().any(|finding| {
        finding.kind == GeneVariantKind::Privacy && finding.segment_id == "segment:private-drift"
    }));
    assert!(intents.contains(&"quarantine".to_owned()));
    assert!(intents.contains(&"regenerate".to_owned()));
    assert!(preview.is_read_only_preview());
}

#[test]
fn mut_detector_reports_splice_boundaries_and_kv_shape_variants() {
    let policy = DnaSplicerPolicy {
        max_segment_tokens: 32,
        ..DnaSplicerPolicy::default()
    };
    let detector = MutDetector::new(policy);
    let segments = vec![
        GeneSegment::new(
            "segment:left",
            TaskProfile::LongDocument,
            GeneSegmentSource::Prompt,
            0,
            16,
        )
        .with_source_hash("sha256:left"),
        GeneSegment::new(
            "segment:gap",
            TaskProfile::LongDocument,
            GeneSegmentSource::Prompt,
            24,
            48,
        )
        .with_source_hash("sha256:gap")
        .with_schema(true, false),
        GeneSegment::new(
            "segment:overlap",
            TaskProfile::LongDocument,
            GeneSegmentSource::Prompt,
            40,
            90,
        )
        .with_source_hash("sha256:overlap"),
    ];

    let findings = detector.detect(&segments);

    assert!(findings.iter().any(|finding| {
        finding.kind == GeneVariantKind::Deletion && finding.segment_id == "segment:gap"
    }));
    assert!(findings.iter().any(|finding| {
        finding.kind == GeneVariantKind::Insertion && finding.segment_id == "segment:overlap"
    }));
    assert!(findings.iter().any(|finding| {
        finding.kind == GeneVariantKind::KvShape && finding.segment_id == "segment:gap"
    }));
    assert!(findings.iter().any(|finding| {
        finding.kind == GeneVariantKind::Truncation && finding.segment_id == "segment:overlap"
    }));
}

#[test]
fn mut_fixer_maps_stale_label_to_relabel_and_drift_to_quarantine_regenerate() {
    let detector = MutDetector::default();
    let segments = vec![
        GeneSegment::new(
            "segment:stale",
            TaskProfile::General,
            GeneSegmentSource::GenomeLedger,
            0,
            16,
        )
        .with_source_hash("sha256:stale")
        .with_metadata("", "", "stale segment metadata"),
        GeneSegment::new(
            "segment:drift",
            TaskProfile::General,
            GeneSegmentSource::RuntimeKv,
            16,
            32,
        )
        .with_source_hash("sha256:drift")
        .with_health(0.82, 0.88, 0.03),
    ];
    let findings = detector.detect(&segments);

    let plans = MutFixer.mutation_plans(&findings, "genome:general:stable");
    let intents = plans
        .iter()
        .map(|plan| (plan.target_gene_id.as_str(), plan.intent))
        .collect::<Vec<_>>();

    assert!(intents.contains(&("segment:stale", GeneScissorsIntent::Relabel)));
    assert!(intents.contains(&("segment:drift", GeneScissorsIntent::Quarantine)));
    assert!(intents.contains(&("segment:drift", GeneScissorsIntent::Regenerate)));
    assert!(
        plans
            .iter()
            .all(|plan| plan.rollback_anchor_id == "genome:general:stable")
    );
    assert!(plans.iter().all(MutationPlan::is_read_only_preview));
}
