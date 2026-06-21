use super::*;
use crate::hierarchy::TaskProfile;
use crate::kv_exchange::RuntimeKvBlock;

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
    assert_eq!(expression.repair_payload_count(), 1);
    assert!(
        expression.mutation_plans[0]
            .proposed_label
            .as_deref()
            .is_some_and(|label| label.contains("memory retrieval gene"))
    );
    assert!(
        expression.mutation_plans[0]
            .proposed_tags
            .iter()
            .any(|tag| tag == "youth_renewal")
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
    assert_eq!(expression.regeneration_payload_count(), 1);
    let regenerate = expression
        .mutation_plans
        .iter()
        .find(|plan| plan.intent == GeneScissorsIntent::Regenerate)
        .expect("regeneration plan");
    assert_eq!(
        regenerate.replacement_gene_id.as_deref(),
        Some("gene:test:safety:young")
    );
    assert!(
        regenerate
            .proposed_purpose
            .as_deref()
            .is_some_and(|purpose| purpose.contains("young candidate"))
    );
    assert!(
        regenerate
            .proposed_tags
            .iter()
            .any(|tag| tag == "stable_anchor")
    );
    assert!(expression.is_read_only_preview());
    assert!(
        expression
            .mutation_plans
            .iter()
            .all(|plan| plan.rollback_anchor_id == "genome:test:stable")
    );
}

#[test]
fn quarantined_gene_is_cut_from_expression_until_regeneration_is_validated() {
    let genome = ReasoningGenome::new(
        "genome:test:v1",
        TaskProfile::Coding,
        "genome:test:stable",
        vec![
            ReasoningGene::new(
                "gene:test:unsafe",
                ReasoningGeneKind::Safety,
                "quarantined unsafe strategy",
                "must not be expressed",
            )
            .with_status(ReasoningGeneStatus::Quarantined),
            ReasoningGene::new(
                "gene:test:retrieval",
                ReasoningGeneKind::Retrieval,
                "memory retrieval",
                "safe memory selection",
            ),
        ],
    );

    let expression = genome.express(GenomeExpressionInput {
        profile: TaskProfile::Coding,
        quality: 0.91,
        process_reward: 0.85,
        contradiction_count: 0,
        critical_reflection_issue_count: 0,
        revision_action_count: 0,
        used_memories: 1,
        memory_feedback_updates: 0,
        route_attention_fraction: 0.30,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: true,
        drift_rollback: false,
        runtime_kv_hold: false,
    });

    assert_eq!(expression.expression_gene_count, 2);
    assert_eq!(expression.active_gene_count(), 1);
    assert!(
        !expression
            .active_gene_ids
            .contains(&"gene:test:unsafe".to_owned())
    );
    assert_eq!(expression.scissors_proposal_count(), 0);
    assert!(expression.is_read_only_preview());
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
    assert_eq!(expression.regeneration_payload_count(), 1);
    assert!(expression.is_read_only_preview());
}

#[test]
fn feedback_health_relabels_low_quality_reflection_gene() {
    let input = GenomeExpressionInput {
        profile: TaskProfile::Coding,
        quality: 0.32,
        process_reward: 0.28,
        contradiction_count: 1,
        critical_reflection_issue_count: 0,
        revision_action_count: 2,
        used_memories: 1,
        memory_feedback_updates: 0,
        route_attention_fraction: 0.62,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: true,
        drift_rollback: false,
        runtime_kv_hold: false,
    };

    let expression = ReasoningGenome::default_for_profile(TaskProfile::Coding)
        .with_feedback_health(&input)
        .express(input);

    assert!(expression.aged_gene_count() >= 1);
    assert!(expression.relabel_candidate_count() >= 1);
    assert!(expression.repair_payload_count() >= 1);
    assert!(
        expression
            .mutation_plans
            .iter()
            .any(|plan| plan.target_gene_id == "gene:coding:reflection"
                && plan.intent == GeneScissorsIntent::Relabel
                && plan.has_repair_payload())
    );
    assert!(
        expression
            .active_gene_ids
            .contains(&"gene:coding:reflection".to_owned())
    );
    assert!(expression.is_read_only_preview());
}

#[test]
fn feedback_health_cuts_and_regenerates_critical_safety_gene() {
    let input = GenomeExpressionInput {
        profile: TaskProfile::Coding,
        quality: 0.18,
        process_reward: 0.16,
        contradiction_count: 1,
        critical_reflection_issue_count: 1,
        revision_action_count: 1,
        used_memories: 0,
        memory_feedback_updates: 0,
        route_attention_fraction: 0.78,
        agent_team_collision_free: true,
        toolsmith_gate_passed: true,
        drift_memory_write_allowed: false,
        drift_rollback: false,
        runtime_kv_hold: true,
    };

    let expression = ReasoningGenome::default_for_profile(TaskProfile::Coding)
        .with_feedback_health(&input)
        .express(input);

    let intents = expression.mutation_intents();
    assert_eq!(expression.malignant_gene_count(), 1);
    assert_eq!(expression.regeneration_candidate_count(), 1);
    assert!(
        !expression
            .active_gene_ids
            .contains(&"gene:coding:safety".to_owned())
    );
    assert!(intents.contains(&"quarantine".to_owned()));
    assert!(intents.contains(&"regenerate".to_owned()));
    assert!(intents.contains(&"rollback".to_owned()));
    assert_eq!(expression.regeneration_payload_count(), 1);
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
    assert_eq!(preview.retained_count(), 1);
    assert_eq!(preview.skipped_count(), 1);
    assert_eq!(preview.quarantined_count(), 1);
    assert_eq!(preview.repair_candidate_count(), 0);
    assert!(preview.findings.iter().any(|finding| {
        finding.kind == GeneVariantKind::Drift && finding.segment_id == "segment:private-drift"
    }));
    assert!(preview.findings.iter().any(|finding| {
        finding.kind == GeneVariantKind::Privacy && finding.segment_id == "segment:private-drift"
    }));
    assert!(intents.contains(&"quarantine".to_owned()));
    assert!(intents.contains(&"regenerate".to_owned()));
    assert!(
        preview
            .disposition_summaries()
            .contains(&"retained".to_owned())
    );
    assert!(
        preview
            .disposition_summaries()
            .contains(&"quarantined".to_owned())
    );
    assert!(
        preview
            .segment_reason_summaries(8)
            .iter()
            .any(|summary| summary.contains("disposition=quarantined")
                && summary.contains("findings=drift|privacy"))
    );
    assert!(preview.is_read_only_preview());
}

#[test]
fn dna_splicer_isolates_malformed_chunk_without_poisoning_neighbor_segments() {
    let segments = vec![
        GeneSegment::new(
            "segment:healthy-left",
            TaskProfile::LongDocument,
            GeneSegmentSource::SemanticMemory,
            0,
            64,
        )
        .with_source_hash("sha256:left")
        .with_metadata(
            "healthy left",
            "safe semantic memory chunk",
            "left bounded chunk",
        )
        .with_health(0.90, 0.02, 0.01),
        GeneSegment::new(
            "segment:malformed-middle",
            TaskProfile::LongDocument,
            GeneSegmentSource::RuntimeKv,
            64,
            64,
        )
        .with_source_hash("sha256:middle")
        .with_metadata(
            "malformed middle",
            "bad range should be repaired locally",
            "middle bad chunk",
        )
        .with_schema(false, false)
        .with_health(0.84, 0.03, 0.01),
        GeneSegment::new(
            "segment:healthy-right",
            TaskProfile::LongDocument,
            GeneSegmentSource::GistMemory,
            64,
            128,
        )
        .with_source_hash("sha256:right")
        .with_metadata(
            "healthy right",
            "safe gist memory chunk",
            "right bounded chunk",
        )
        .with_health(0.88, 0.04, 0.01),
    ];

    let preview = DnaSplicer::default().preview(
        TaskProfile::LongDocument,
        "genome:long_document:stable",
        segments,
    );

    assert_eq!(preview.retained_count(), 2);
    assert_eq!(preview.repair_candidate_count(), 1);
    assert_eq!(preview.quarantined_count(), 0);
    assert_eq!(preview.variant_count(), 1);
    assert!(preview.findings.iter().any(|finding| {
        finding.segment_id == "segment:malformed-middle"
            && finding.kind == GeneVariantKind::EmptyRange
    }));
    assert!(preview.findings.iter().any(|finding| {
        finding.segment_id == "segment:malformed-middle" && finding.kind == GeneVariantKind::Schema
    }));
    assert!(preview.segments.iter().any(|segment| {
        segment.segment.id == "segment:healthy-left"
            && segment.disposition == GeneSegmentDisposition::Retained
    }));
    assert!(preview.segments.iter().any(|segment| {
        segment.segment.id == "segment:healthy-right"
            && segment.disposition == GeneSegmentDisposition::Retained
    }));
    assert!(
        preview
            .segment_reason_summaries(8)
            .iter()
            .any(|summary| summary.contains("disposition=repair_candidate")
                && summary.contains("findings=empty_range|schema|kv_shape"))
    );
    assert!(preview.is_read_only_preview());
}

#[test]
fn mut_detector_reports_splice_boundaries_and_kv_shape_variants() {
    let policy = DnaSplicerPolicy {
        max_segment_tokens: 32,
        max_planned_overlap_tokens: 4,
        ..DnaSplicerPolicy::default()
    };
    let detector = MutDetector::new(policy);
    let source_hash = "sha256:prompt-chain";
    let segments = vec![
        GeneSegment::new(
            "segment:left",
            TaskProfile::LongDocument,
            GeneSegmentSource::Prompt,
            0,
            16,
        )
        .with_source_hash(source_hash),
        GeneSegment::new(
            "segment:gap",
            TaskProfile::LongDocument,
            GeneSegmentSource::Prompt,
            24,
            48,
        )
        .with_source_hash(source_hash)
        .with_schema(true, false),
        GeneSegment::new(
            "segment:overlap",
            TaskProfile::LongDocument,
            GeneSegmentSource::Prompt,
            40,
            90,
        )
        .with_source_hash(source_hash),
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
fn runtime_kv_block_gene_segment_carries_shape_evidence_to_splicer() {
    let valid = RuntimeKvBlock::new(1, 0, 16, 20, vec![0.1, 0.2], vec![0.3, 0.4]);
    let malformed = RuntimeKvBlock::new(1, 0, 20, 20, Vec::new(), vec![0.3]);
    let segments = vec![
        GeneSegment::from_runtime_kv_block(
            "segment:runtime-kv-valid",
            TaskProfile::Coding,
            "sha256:valid-kv",
            &valid,
        )
        .with_health(0.88, 0.02, 0.01),
        GeneSegment::from_runtime_kv_block(
            "segment:runtime-kv-malformed",
            TaskProfile::Coding,
            "sha256:bad-kv",
            &malformed,
        )
        .with_health(0.88, 0.02, 0.01),
    ];

    let preview =
        DnaSplicer::default().preview(TaskProfile::Coding, "genome:coding:stable", segments);

    assert_eq!(preview.exon_count(), 1);
    assert_eq!(preview.variant_count(), 1);
    assert!(preview.findings.iter().any(|finding| {
        finding.segment_id == "segment:runtime-kv-malformed"
            && finding.kind == GeneVariantKind::KvShape
    }));
    assert!(preview.is_read_only_preview());
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
    let stale = plans
        .iter()
        .find(|plan| {
            plan.target_gene_id == "segment:stale" && plan.intent == GeneScissorsIntent::Relabel
        })
        .expect("stale relabel plan");
    assert!(stale.has_repair_payload());
    let regenerate = plans
        .iter()
        .find(|plan| {
            plan.target_gene_id == "segment:drift" && plan.intent == GeneScissorsIntent::Regenerate
        })
        .expect("drift regenerate plan");
    assert_eq!(
        regenerate.replacement_gene_id.as_deref(),
        Some("segment:drift:young")
    );
    assert!(regenerate.has_regeneration_payload());
    assert!(
        plans
            .iter()
            .all(|plan| plan.rollback_anchor_id == "genome:general:stable")
    );
    assert!(plans.iter().all(MutationPlan::is_read_only_preview));
}

#[test]
fn dual_chain_schema_round_trips_expression_and_memory_records() {
    let genome = ReasoningGenome::default_for_profile(TaskProfile::Coding);
    let source = DnaGeneSourceEvidence::new(
        DnaGeneEvidenceKind::SyntheticDefault,
        "sha256:genome-default",
        "default profile genome scaffold",
    )
    .with_prompt_digest("prompt-digest:coding")
    .with_privacy_gate();
    let mut chain =
        DnaGeneChain::preview_from_genome(&genome, "tenant:local", "session:roundtrip", source);

    let memory_gene = ReasoningGene::new(
        "gene:coding:memory-tool-reliability",
        ReasoningGeneKind::ToolUse,
        "tool reliability memory",
        "retain validated Toolsmith and runtime adapter reliability evidence",
    )
    .with_tags(["memory_chain", "tool_reliability"])
    .with_health(2, 0.82, 0.08);
    let memory_record = DnaGeneRecord::from_reasoning_gene(
        DnaChainKind::Memory,
        TaskProfile::Coding,
        chain.stable_anchor_id.clone(),
        DnaGeneLineage::new("tenant:local", "session:roundtrip")
            .with_parent("gene:coding:tool-use")
            .with_inheritance(chain.stable_anchor_id.clone(), 1),
        DnaGeneSourceEvidence::new(
            DnaGeneEvidenceKind::ToolReliability,
            "sha256:tool-reliability",
            "sanitized runtime adapter and Toolsmith evidence",
        )
        .with_prompt_digest("prompt-digest:tool"),
        &memory_gene,
    );
    chain.push_memory_record(memory_record);

    let lines = chain.to_kv_lines().expect("valid dual-chain schema");
    let loaded = DnaGeneChain::from_kv_lines(&lines).expect("roundtrip schema");

    assert_eq!(loaded.schema_version, "dna_chain_v1");
    assert_eq!(loaded.express_chain.len(), 7);
    assert_eq!(loaded.memory_chain.len(), 1);
    assert_eq!(loaded.total_gene_count(), 8);
    assert!(loaded.read_only);
    assert!(!loaded.write_allowed);
    assert!(
        loaded
            .express_chain
            .iter()
            .all(|record| record.chain_kind == DnaChainKind::Express)
    );
    assert!(
        loaded
            .memory_chain
            .iter()
            .all(|record| record.chain_kind == DnaChainKind::Memory)
    );
    assert!(
        loaded
            .memory_chain
            .iter()
            .all(|record| record.rollback_anchor_id == "genome:coding:stable")
    );
    assert!(
        loaded
            .memory_chain
            .iter()
            .all(|record| record.operator_approval_required && !record.applied)
    );
}

#[test]
fn dual_chain_schema_rejects_missing_gene_metadata() {
    let genome = ReasoningGenome::default_for_profile(TaskProfile::General);
    let source = DnaGeneSourceEvidence::new(
        DnaGeneEvidenceKind::SyntheticDefault,
        "sha256:general-default",
        "default profile genome scaffold",
    );
    let mut chain =
        DnaGeneChain::preview_from_genome(&genome, "tenant:local", "session:metadata", source);

    chain.express_chain[0].label.clear();
    assert!(matches!(
        chain.validate(),
        Err(DnaGeneSchemaError::LabelMissing { .. })
    ));

    chain.express_chain[0].label = "restored label".to_owned();
    chain.express_chain[0].source_evidence.source_hash.clear();
    assert!(matches!(
        chain.validate(),
        Err(DnaGeneSchemaError::SourceEvidenceMissing { .. })
    ));

    chain.express_chain[0].source_evidence.source_hash = "sha256:restored".to_owned();
    chain.express_chain[0].rollback_anchor_id.clear();
    assert!(matches!(
        chain.validate(),
        Err(DnaGeneSchemaError::RollbackAnchorMissing { .. })
    ));
}

#[test]
fn dual_chain_schema_requires_privacy_gate_before_raw_prompt_marker() {
    let genome = ReasoningGenome::default_for_profile(TaskProfile::Writing);
    let source = DnaGeneSourceEvidence::new(
        DnaGeneEvidenceKind::Reflection,
        "sha256:reflection",
        "reflection evidence with prompt digest only",
    )
    .with_raw_prompt_marker();
    let chain =
        DnaGeneChain::preview_from_genome(&genome, "tenant:local", "session:privacy", source);

    assert!(matches!(
        chain.validate(),
        Err(DnaGeneSchemaError::PrivacyGateRequired { .. })
    ));
}

#[test]
fn dual_chain_schema_rejects_write_enabled_preview_records() {
    let genome = ReasoningGenome::default_for_profile(TaskProfile::LongDocument);
    let source = DnaGeneSourceEvidence::new(
        DnaGeneEvidenceKind::SyntheticDefault,
        "sha256:long-document-default",
        "default long document genome scaffold",
    );
    let mut chain =
        DnaGeneChain::preview_from_genome(&genome, "tenant:local", "session:writes", source);

    chain.express_chain[0].admission_write_authorized = true;
    assert!(matches!(
        chain.validate(),
        Err(DnaGeneSchemaError::WriteGateOpenInPreview)
    ));

    chain.express_chain[0].admission_write_authorized = false;
    chain.express_chain[0].applied = true;
    assert!(matches!(
        chain.validate(),
        Err(DnaGeneSchemaError::AppliedPreviewMutation)
    ));

    chain.express_chain[0].applied = false;
    chain.read_only = false;
    assert!(matches!(
        chain.validate(),
        Err(DnaGeneSchemaError::ReadOnlyPreviewRequired)
    ));
}
