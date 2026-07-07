use super::*;
use crate::hierarchy::TaskProfile;
use crate::kv_exchange::RuntimeKvBlock;
use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};

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
        genome_mutation_allowed: true,
        drift_rollback: false,
        runtime_kv_hold: false,
    });

    assert_eq!(expression.expression_gene_count, 7);
    assert_eq!(expression.active_gene_count(), 7);
    assert_eq!(expression.aged_gene_count(), 0);
    assert_eq!(expression.malignant_gene_count(), 0);
    assert_eq!(expression.lifecycle_record_count(), 7);
    assert_eq!(expression.tombstone_candidate_count(), 0);
    assert!(
        expression
            .lifecycle_action_summaries()
            .contains(&"keep".to_owned())
    );
    assert!(expression.lifecycle_source_evidence_count() >= 7);
    assert!(expression.is_read_only_preview());
    assert_eq!(expression.scissors_proposal_count(), 0);
}

#[test]
fn stable_expression_emits_digest_only_epigenetic_marker() {
    let expression =
        ReasoningGenome::default_for_profile(TaskProfile::Coding).express(GenomeExpressionInput {
            profile: TaskProfile::Coding,
            quality: 0.99,
            process_reward: 0.99,
            contradiction_count: 0,
            critical_reflection_issue_count: 0,
            revision_action_count: 0,
            used_memories: 2,
            memory_feedback_updates: 1,
            route_attention_fraction: 0.42,
            agent_team_collision_free: true,
            toolsmith_gate_passed: true,
            drift_memory_write_allowed: true,
            genome_mutation_allowed: true,
            drift_rollback: false,
            runtime_kv_hold: false,
        });

    let marker = expression
        .epigenetic_expression_cache_marker()
        .expect("stable expression marker");

    assert!(marker.marker_id.starts_with("redaction-digest:"));
    assert!(
        marker
            .cache_candidate_digest
            .starts_with("redaction-digest:")
    );
    assert!(marker.cache_key_digest.starts_with("redaction-digest:"));
    assert_eq!(marker.observation_window, 100);
    assert_eq!(marker.min_success_rate_milli, 980);
    for value in [
        marker.marker_id,
        marker.cache_candidate_digest,
        marker.cache_key_digest,
    ] {
        assert!(!contains_private_or_executable_marker(&value));
    }
}

#[test]
fn negative_expression_evidence_blocks_epigenetic_marker() {
    let expression =
        ReasoningGenome::default_for_profile(TaskProfile::Coding).express(GenomeExpressionInput {
            profile: TaskProfile::Coding,
            quality: 0.99,
            process_reward: 0.99,
            contradiction_count: 1,
            critical_reflection_issue_count: 0,
            revision_action_count: 0,
            used_memories: 2,
            memory_feedback_updates: 1,
            route_attention_fraction: 0.42,
            agent_team_collision_free: true,
            toolsmith_gate_passed: true,
            drift_memory_write_allowed: true,
            genome_mutation_allowed: true,
            drift_rollback: false,
            runtime_kv_hold: false,
        });

    assert!(expression.epigenetic_expression_cache_marker().is_none());
}

#[test]
fn reasoning_frame_preview_generates_issue375_evidence() {
    let frame = ReasoningFrame::issue375_preview("redaction-digest:body-state");

    assert_eq!(frame.genome_isa.name, "PreReasoningGenomeIsa");
    assert!(frame.genome_isa.opcodes.contains(&GenomeOpcode::Verify));
    assert!(frame.efficiency_snapshot.is_none());
    assert!(frame.validate_preview().is_ok());
    assert!(
        frame
            .evidence_requirements
            .contains(&ReasoningFrameEvidenceRequirement::DigestOnlyFrameId)
    );
    assert!(
        frame
            .validation_requirements
            .contains(&ReasoningFrameValidationRequirement::NoApply)
    );

    let fields = frame.issue375_evidence_fields();

    assert!(fields.contains("issue375_pre_reasoning_genome_isa_present=true"));
    assert!(fields.contains("issue375_reasoning_frame_id=redaction-digest:"));
    assert!(fields.contains(
        "issue375_reasoning_frame_allowed_observations=repo_issue_terminal_runtime_state"
    ));
    assert!(fields.contains(
        "issue375_reasoning_frame_action_vocab=observe_inspect_compare_summarize_verify_quarantine"
    ));
    assert!(
        fields.contains(
            "issue375_reasoning_frame_suppressed_capabilities=write_process_browser_network_memory_genome_runtime"
        )
    );
    assert!(fields.contains("issue375_expression_vm_side_effect=read_only"));
    assert!(fields.contains("issue375_genome_isa_apply_allowed=false"));
}

#[test]
fn reasoning_frame_efficiency_snapshot_stays_preview_only() {
    let snapshot = ReasoningFrameEfficiencySnapshot::preview(
        3, 1, 2, 4, "normal", 128, 96, 32, 12, 0.87, 0.91,
    );
    let frame = ReasoningFrame::issue375_preview("redaction-digest:body-state")
        .with_efficiency_snapshot(snapshot.clone());

    assert!(snapshot.has_feedback_signal());
    assert_eq!(snapshot.quality_milli, 870);
    assert_eq!(snapshot.process_reward_milli, 910);
    assert!(frame.validate_preview().is_ok());

    let mut write_enabled = frame.clone();
    write_enabled
        .efficiency_snapshot
        .as_mut()
        .expect("snapshot")
        .write_allowed = true;
    assert_eq!(
        write_enabled.validate_preview(),
        Err(ReasoningFrameValidationError::EfficiencySnapshotNotPreviewOnly)
    );

    let mut applied = frame;
    applied
        .efficiency_snapshot
        .as_mut()
        .expect("snapshot")
        .applied = true;
    assert_eq!(
        applied.validate_preview(),
        Err(ReasoningFrameValidationError::EfficiencySnapshotNotPreviewOnly)
    );
}

#[test]
fn reasoning_frame_preview_rejects_write_apply_and_file_capability() {
    let mut frame = ReasoningFrame::issue375_preview("redaction-digest:body-state");

    frame.write_allowed = true;
    assert_eq!(
        frame.validate_preview(),
        Err(ReasoningFrameValidationError::WriteAllowed)
    );

    frame.write_allowed = false;
    frame.genome_isa.apply_allowed = true;
    assert_eq!(
        frame.validate_preview(),
        Err(ReasoningFrameValidationError::GenomeIsaApplyAllowed)
    );

    frame.genome_isa.apply_allowed = false;
    frame
        .granted_capabilities
        .push(ReasoningFrameCapability::FileWrite);
    assert_eq!(
        frame.validate_preview(),
        Err(ReasoningFrameValidationError::ForbiddenCapabilityGranted(
            ReasoningFrameCapability::FileWrite
        ))
    );
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
        genome_mutation_allowed: true,
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
    assert_eq!(
        expression.mutation_plans[0].validation_status,
        GeneValidationStatus::Pending
    );
    assert!(expression.mutation_plans[0].has_source_evidence());
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
    assert_eq!(expression.lifecycle_record_count(), 1);
    assert_eq!(
        expression.lifecycle_records[0].action,
        GeneLifecycleAction::Relabel
    );
    assert_eq!(
        expression.lifecycle_records[0].validation_status,
        GeneValidationStatus::Pending
    );
    assert!(expression.lifecycle_records[0].decay_score > 0.0);
    assert!(
        expression.lifecycle_records[0]
            .last_confirmed_purpose
            .contains("retrieve useful memory")
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
        genome_mutation_allowed: true,
        drift_rollback: true,
        runtime_kv_hold: true,
    });

    let intents = expression.mutation_intents();
    assert_eq!(expression.malignant_gene_count(), 1);
    assert_eq!(expression.regeneration_candidate_count(), 1);
    assert!(intents.contains(&"quarantine".to_owned()));
    assert!(intents.contains(&"regenerate".to_owned()));
    assert!(intents.contains(&"rollback".to_owned()));
    assert!(intents.contains(&"cut".to_owned()));
    assert!(expression.youth_pressure > 0.50);
    assert_eq!(expression.regeneration_payload_count(), 1);
    assert_eq!(expression.tombstone_candidate_count(), 1);
    assert!(
        expression
            .lifecycle_action_summaries()
            .contains(&"cut".to_owned())
    );
    assert!(
        expression
            .lifecycle_records
            .iter()
            .any(|record| record.is_tombstone_candidate()
                && record.rollback_anchor_id == "genome:test:stable"
                && record.validation_status == GeneValidationStatus::Pending)
    );
    let regenerate = expression
        .mutation_plans
        .iter()
        .find(|plan| plan.intent == GeneScissorsIntent::Regenerate)
        .expect("regeneration plan");
    assert!(regenerate.has_source_evidence());
    assert_eq!(regenerate.validation_status, GeneValidationStatus::Pending);
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
        genome_mutation_allowed: true,
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
    assert_eq!(expression.lifecycle_record_count(), 2);
    assert_eq!(expression.tombstone_candidate_count(), 1);
    assert!(
        expression
            .lifecycle_records
            .iter()
            .any(|record| record.gene_id == "gene:test:unsafe"
                && record.action == GeneLifecycleAction::Cut
                && record.is_tombstone_candidate())
    );
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
        genome_mutation_allowed: true,
        drift_rollback: true,
        runtime_kv_hold: false,
    });

    let intents = expression.mutation_intents();
    assert_eq!(expression.malignant_gene_count(), 1);
    assert_eq!(expression.regeneration_candidate_count(), 1);
    assert!(intents.contains(&"quarantine".to_owned()));
    assert!(intents.contains(&"regenerate".to_owned()));
    assert!(intents.contains(&"rollback".to_owned()));
    assert!(intents.contains(&"cut".to_owned()));
    assert_eq!(expression.regeneration_payload_count(), 1);
    assert_eq!(expression.tombstone_candidate_count(), 1);
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
        genome_mutation_allowed: true,
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
        genome_mutation_allowed: true,
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
    assert!(intents.contains(&"cut".to_owned()));
    assert_eq!(expression.regeneration_payload_count(), 1);
    assert_eq!(expression.tombstone_candidate_count(), 1);
    assert!(expression.is_read_only_preview());
}

#[test]
fn regeneration_uses_stable_anchor_and_high_fitness_sibling_evidence() {
    let genome = ReasoningGenome::new(
        "genome:test:v1",
        TaskProfile::Coding,
        "genome:test:stable",
        vec![
            ReasoningGene::new(
                "gene:test:safety",
                ReasoningGeneKind::Safety,
                "unsafe drift guard",
                "this safety behavior drifted",
            )
            .with_health(3, 0.18, 0.93),
            ReasoningGene::new(
                "gene:test:retrieval",
                ReasoningGeneKind::Retrieval,
                "healthy retrieval",
                "validated retrieval sibling",
            )
            .with_health(1, 0.92, 0.04),
        ],
    );

    let expression = genome.express(GenomeExpressionInput {
        profile: TaskProfile::Coding,
        quality: 0.40,
        process_reward: 0.35,
        contradiction_count: 1,
        critical_reflection_issue_count: 0,
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

    let regenerate = expression
        .mutation_plans
        .iter()
        .find(|plan| plan.intent == GeneScissorsIntent::Regenerate)
        .expect("regeneration plan");

    assert!(
        regenerate
            .source_gene_ids
            .contains(&"genome:test:stable".to_owned())
    );
    assert!(
        regenerate
            .source_gene_ids
            .contains(&"gene:test:retrieval".to_owned())
    );
    assert!(regenerate.source_evidence.iter().any(|evidence| {
        evidence.kind == GeneLifecycleSourceKind::HighFitnessSibling
            && evidence.source_id == "gene:test:retrieval"
    }));
    assert!(expression.lifecycle_records.iter().any(|record| {
        record.action == GeneLifecycleAction::Regenerate
            && record.source_evidence.iter().any(|evidence| {
                evidence.kind == GeneLifecycleSourceKind::StableAnchor
                    && evidence.source_id == "genome:test:stable"
            })
    }));
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
fn lineage_audit_export_cuts_bad_segment_without_poisoning_neighbors() {
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

    let packet = DnaLineageAuditPacket::from_splice_preview(&preview);
    let left = packet
        .nodes_for_source("segment:healthy-left")
        .into_iter()
        .find(|node| node.kind == DnaLineageAuditNodeKind::OriginalSegment)
        .expect("left segment node");
    let middle = packet
        .nodes_for_source("segment:malformed-middle")
        .into_iter()
        .find(|node| node.kind == DnaLineageAuditNodeKind::OriginalSegment)
        .expect("middle segment node");
    let right = packet
        .nodes_for_source("segment:healthy-right")
        .into_iter()
        .find(|node| node.kind == DnaLineageAuditNodeKind::OriginalSegment)
        .expect("right segment node");

    assert_eq!(left.gate_status, "retained");
    assert_eq!(right.gate_status, "retained");
    assert_eq!(middle.gate_status, "repair_candidate");
    assert!(middle.reason_codes.contains(&"empty_range".to_owned()));
    assert!(middle.reason_codes.contains(&"schema".to_owned()));
    assert!(middle.reason_codes.contains(&"kv_shape".to_owned()));

    let left_id = left.id.clone();
    let middle_id = middle.id.clone();
    let right_id = right.id.clone();
    assert!(!packet.edges.iter().any(|edge| {
        (edge.parent_id == middle_id && (edge.child_id == left_id || edge.child_id == right_id))
            || (edge.child_id == middle_id
                && (edge.parent_id == left_id || edge.parent_id == right_id))
    }));

    let json = packet.to_redacted_json();
    assert!(json.contains("\"before_digest\""));
    assert!(json.contains("\"after_digest\""));
    assert!(json.contains("\"reason_codes\""));
    assert!(json.contains("\"gate_status\""));
    assert!(json.contains("\"rollback_anchor_id\""));
    assert!(!json.contains("bad range should be repaired locally"));
    assert!(!json.contains("middle bad chunk"));
    assert!(packet.exports_are_redacted());
}

#[test]
fn lineage_audit_export_distinguishes_preview_and_applied_repairs() {
    let segments = vec![
        GeneSegment::new(
            "segment:private-drift",
            TaskProfile::Coding,
            GeneSegmentSource::RuntimeKv,
            0,
            24,
        )
        .with_source_hash("sha256:private-drift")
        .with_metadata(
            "private runtime drift",
            "must be quarantined before reuse",
            "runtime drift",
        )
        .with_health(0.80, 0.80, 0.50),
    ];
    let preview =
        DnaSplicer::default().preview(TaskProfile::Coding, "genome:coding:stable", segments);
    let preview_packet = DnaLineageAuditPacket::from_splice_preview(&preview);
    let preview_json = preview_packet.to_redacted_json();

    assert!(preview_json.contains("\"repair_state\":\"preview_only\""));
    assert!(!preview_json.contains("\"repair_state\":\"approved_applied\""));

    let mut applied_preview = preview.clone();
    let regenerate = applied_preview
        .mutation_plans
        .iter_mut()
        .find(|plan| plan.intent == GeneScissorsIntent::Regenerate)
        .expect("regeneration plan");
    regenerate.validation_status = GeneValidationStatus::Passed;
    regenerate.admission_write_authorized = true;
    regenerate.applied = true;
    applied_preview.read_only = false;
    applied_preview.write_allowed = true;
    applied_preview.applied = true;

    let applied_packet = DnaLineageAuditPacket::from_splice_preview(&applied_preview);
    let applied_json = applied_packet.to_redacted_json();

    assert!(applied_json.contains("\"repair_state\":\"approved_applied\""));
    assert!(applied_json.contains("\"kind\":\"approved_repair\""));
    assert!(applied_json.contains("\"gate_status\":\"passed\""));
    assert!(applied_packet.exports_are_redacted());
}

#[test]
fn lineage_audit_export_redacts_private_and_executable_payload_markers() {
    let unsafe_segment_id =
        "prompt: secret=abc api_key=xyz curl http://bad.example powershell rm -rf";
    let segments = vec![
        GeneSegment::new(
            unsafe_segment_id,
            TaskProfile::Coding,
            GeneSegmentSource::Prompt,
            0,
            32,
        )
        .with_source_hash("secret=raw-source-hash")
        .with_metadata(
            "answer: private key material",
            "curl http://bad.example should never be exported",
            "powershell rm -rf marker",
        )
        .with_health(0.80, 0.92, 0.95),
    ];
    let preview =
        DnaSplicer::default().preview(TaskProfile::Coding, "genome:coding:stable", segments);

    let packet = DnaLineageAuditPacket::from_splice_preview(&preview);
    let json = packet.to_redacted_json();
    let markdown = packet.to_redacted_markdown();

    assert!(json.contains("redacted-ref:audit-digest:"));
    assert!(markdown.contains("redacted-ref:audit-digest:"));
    for marker in [
        "prompt:",
        "answer:",
        "secret=",
        "api_key",
        "private key",
        "curl ",
        "powershell",
        "rm ",
    ] {
        assert!(
            !json.to_ascii_lowercase().contains(marker),
            "json leaked marker {marker}: {json}"
        );
        assert!(
            !markdown.to_ascii_lowercase().contains(marker),
            "markdown leaked marker {marker}: {markdown}"
        );
    }
    assert!(packet.exports_are_redacted());
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
fn mut_detector_relabels_aged_segment_with_last_confirmed_purpose() {
    let detector = MutDetector::default();
    let segments = vec![
        GeneSegment::new(
            "segment:aged-heuristic",
            TaskProfile::Coding,
            GeneSegmentSource::SemanticMemory,
            0,
            24,
        )
        .with_source_hash("sha256:aged")
        .with_metadata(
            "compiler repair heuristic",
            "preserve validated Rust compiler repair strategy",
            "bounded compiler repair memory",
        )
        .with_last_confirmed_purpose("validated Rust compiler repair strategy")
        .with_age(12)
        .with_health(0.82, 0.04, 0.01),
    ];

    let findings = detector.detect(&segments);

    assert!(segments[0].decay_score() > 0.0);
    assert!(findings.iter().any(|finding| {
        finding.segment_id == "segment:aged-heuristic"
            && finding.kind == GeneVariantKind::StaleLabel
            && finding.suggested_intent == GeneScissorsIntent::Relabel
    }));
}

#[test]
fn mut_detector_reports_contradiction_and_low_fitness_repetition() {
    let detector = MutDetector::default();
    let segments = vec![
        GeneSegment::new(
            "segment:conflict-rule",
            TaskProfile::Coding,
            GeneSegmentSource::ToolOutput,
            0,
            12,
        )
        .with_source_hash("sha256:conflict")
        .with_metadata(
            "conflicting tool rule",
            "contradict previous Rust validation evidence",
            "tool rule conflict",
        )
        .with_health(0.72, 0.04, 0.01),
        GeneSegment::new(
            "segment:weak-repeat-a",
            TaskProfile::Coding,
            GeneSegmentSource::SemanticMemory,
            12,
            24,
        )
        .with_source_hash("sha256:weak-a")
        .with_metadata(
            "weak repeated heuristic",
            "same stale heuristic appeared with low reward",
            "weak repeated heuristic",
        )
        .with_health(0.20, 0.03, 0.01),
        GeneSegment::new(
            "segment:weak-repeat-b",
            TaskProfile::Coding,
            GeneSegmentSource::SemanticMemory,
            24,
            36,
        )
        .with_source_hash("sha256:weak-b")
        .with_metadata(
            "weak repeated heuristic",
            "same stale heuristic appeared with low reward",
            "weak repeated heuristic duplicate",
        )
        .with_health(0.18, 0.03, 0.01),
    ];

    let findings = detector.detect(&segments);

    assert!(findings.iter().any(|finding| {
        finding.segment_id == "segment:conflict-rule"
            && finding.kind == GeneVariantKind::Contradiction
    }));
    assert!(findings.iter().any(|finding| {
        finding.segment_id == "segment:weak-repeat-b"
            && finding.kind == GeneVariantKind::LowFitnessRepetition
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
fn gene_scissors_lifecycle_tracks_quarantine_and_repair_candidates_without_writes() {
    let segments = vec![
        GeneSegment::new(
            "segment:private-drift",
            TaskProfile::Coding,
            GeneSegmentSource::RuntimeKv,
            0,
            24,
        )
        .with_source_hash("sha256:private-drift")
        .with_metadata(
            "private runtime drift",
            "must be quarantined before reuse",
            "runtime drift",
        )
        .with_health(0.80, 0.80, 0.50),
        GeneSegment::new(
            "segment:stale-repair",
            TaskProfile::Coding,
            GeneSegmentSource::GenomeLedger,
            24,
            48,
        )
        .with_source_hash("sha256:stale-repair")
        .with_metadata("", "", "stale repair candidate")
        .with_health(0.64, 0.05, 0.01),
    ];

    let preview =
        DnaSplicer::default().preview(TaskProfile::Coding, "genome:coding:stable", segments);

    assert_eq!(preview.lifecycle_record_count(), 2);
    assert_eq!(preview.quarantined_lifecycle_count(), 1);
    assert!(
        preview
            .lifecycle_state_summaries()
            .contains(&"quarantined".to_owned())
    );
    assert!(
        preview
            .control_lifecycle_state_summaries()
            .contains(&"quarantined".to_owned())
    );
    assert!(
        preview
            .lifecycle_state_summaries()
            .contains(&"repair_candidate".to_owned())
    );
    assert!(
        preview
            .control_lifecycle_state_summaries()
            .contains(&"repaired_candidate".to_owned())
    );
    let quarantine = preview
        .lifecycle_records
        .iter()
        .find(|record| record.target_segment_id == "segment:private-drift")
        .expect("quarantine lifecycle");
    assert_eq!(quarantine.state, GeneScissorsLifecycleState::Quarantined);
    assert_eq!(quarantine.state.control_lifecycle_state(), "quarantined");
    assert_eq!(
        quarantine.validation_status,
        GeneScissorsValidationStatus::Pending
    );
    assert!(quarantine.confidence >= 0.90);
    assert_eq!(quarantine.rollback_anchor_id, "genome:coding:stable");
    assert_eq!(quarantine.reason_code, "drift");
    assert_eq!(quarantine.source_digest, "sha256:private-drift");
    assert_eq!(
        quarantine.parent_lineage,
        "genome:coding:stable:segment:private-drift"
    );
    assert_eq!(quarantine.affected_scope, "runtime_kv:0..24");
    assert_eq!(
        quarantine.readmission_gate,
        "hold_until_verifier_and_operator_approval"
    );
    assert!(quarantine.operator_approval_required);
    assert_eq!(
        quarantine.stable_anchor_sources,
        vec!["genome:coding:stable".to_owned()]
    );
    assert!(!quarantine.admission_write_authorized);
    assert!(!quarantine.applied);
    assert!(quarantine.summary().contains("write_allowed=false"));
    assert!(
        quarantine
            .summary()
            .contains("control_lifecycle_state=quarantined")
    );
    assert!(quarantine.summary().contains("source_digest_present=true"));
    assert!(quarantine.summary().contains("affected_scope_present=true"));
    assert!(
        quarantine
            .summary()
            .contains("operator_approval_required=true")
    );
    assert!(
        preview.lifecycle_summaries(4).iter().all(|summary| {
            summary.contains("profile=Coding")
                && summary.contains("shadow_state=")
                && summary.contains("drift_state=")
                && summary.contains("source_ids=")
                && summary.contains("expires_after_steps=")
                && summary.contains("score_milli=")
                && summary.contains("drift_gate_domains=golden_fixture:pending|routing_behavior:pending|memory_hygiene:pending|privacy:pending|trace_schema:pending")
                && summary.contains("rollback=redaction-digest:")
                && summary.contains("write_allowed=false")
                && summary.contains("applied=false")
        })
    );
    assert!(!contains_private_or_executable_marker(
        &quarantine.summary()
    ));
    assert!(preview.is_read_only_preview());
}

#[test]
fn failed_gene_scissors_validation_holds_or_rejects_without_mutation_write() {
    let repair = GeneScissorsLifecycleRecord {
        id: "gene_scissors:repair".to_owned(),
        target_segment_id: "segment:repair".to_owned(),
        finding_ids: vec!["finding:segment:repair:schema".to_owned()],
        finding_kinds: vec![GeneVariantKind::Schema],
        mutation_plan_ids: vec!["mutation:segment:repair:repair".to_owned()],
        state: GeneScissorsLifecycleState::RepairCandidate,
        validation_status: GeneScissorsValidationStatus::Pending,
        confidence: 0.74,
        reason_code: "schema".to_owned(),
        source_digest: "sha256:repair".to_owned(),
        parent_lineage: "genome:coding:stable:segment:repair".to_owned(),
        rollback_anchor_id: "genome:coding:stable".to_owned(),
        affected_scope: "genome_ledger:0..8".to_owned(),
        readmission_gate: "hold_until_verifier_and_operator_approval".to_owned(),
        operator_approval_required: true,
        stable_anchor_sources: vec!["genome:coding:stable".to_owned()],
        next_action: "validate_repair_candidate".to_owned(),
        admission_write_authorized: false,
        applied: false,
    }
    .with_validation_status(GeneScissorsValidationStatus::Failed);
    let quarantine = GeneScissorsLifecycleRecord {
        id: "gene_scissors:quarantine".to_owned(),
        target_segment_id: "segment:quarantine".to_owned(),
        finding_ids: vec!["finding:segment:quarantine:privacy".to_owned()],
        finding_kinds: vec![GeneVariantKind::Privacy],
        mutation_plan_ids: vec!["mutation:segment:quarantine:quarantine".to_owned()],
        state: GeneScissorsLifecycleState::Quarantined,
        validation_status: GeneScissorsValidationStatus::Pending,
        confidence: 0.92,
        reason_code: "privacy".to_owned(),
        source_digest: "sha256:quarantine".to_owned(),
        parent_lineage: "genome:coding:stable:segment:quarantine".to_owned(),
        rollback_anchor_id: "genome:coding:stable".to_owned(),
        affected_scope: "runtime_kv:0..8".to_owned(),
        readmission_gate: "hold_until_verifier_and_operator_approval".to_owned(),
        operator_approval_required: true,
        stable_anchor_sources: vec!["genome:coding:stable".to_owned()],
        next_action: "keep_isolated_generate_stable_anchor_replacement".to_owned(),
        admission_write_authorized: false,
        applied: false,
    }
    .with_validation_status(GeneScissorsValidationStatus::Failed);
    let cut_preview = GeneScissorsLifecycleRecord {
        id: "gene_scissors:cut".to_owned(),
        target_segment_id: "segment:cut".to_owned(),
        finding_ids: vec!["finding:segment:cut:privacy".to_owned()],
        finding_kinds: vec![GeneVariantKind::Privacy],
        mutation_plan_ids: vec!["mutation:segment:cut:quarantine".to_owned()],
        state: GeneScissorsLifecycleState::Quarantined,
        validation_status: GeneScissorsValidationStatus::Passed,
        confidence: 0.92,
        reason_code: "privacy".to_owned(),
        source_digest: "sha256:cut".to_owned(),
        parent_lineage: "genome:coding:stable:segment:cut".to_owned(),
        rollback_anchor_id: "genome:coding:stable".to_owned(),
        affected_scope: "runtime_kv:0..8".to_owned(),
        readmission_gate: "hold_until_verifier_and_operator_approval".to_owned(),
        operator_approval_required: true,
        stable_anchor_sources: vec!["genome:coding:stable".to_owned()],
        next_action: "await_operator_approval_before_apply".to_owned(),
        admission_write_authorized: false,
        applied: false,
    }
    .with_cut_preview();

    assert_eq!(repair.state, GeneScissorsLifecycleState::Held);
    assert_eq!(quarantine.state, GeneScissorsLifecycleState::Rejected);
    assert_eq!(cut_preview.state, GeneScissorsLifecycleState::Cut);
    assert_eq!(
        cut_preview.state.control_lifecycle_state(),
        "tombstone_preview"
    );
    assert!(repair.next_action.contains("hold"));
    assert!(quarantine.next_action.contains("reject"));
    assert!(cut_preview.next_action.contains("operator_approval"));
    assert!(
        cut_preview
            .summary()
            .contains("control_lifecycle_state=tombstone_preview")
    );
    assert!(repair.operator_approval_required);
    assert_eq!(
        cut_preview.readmission_gate,
        "hold_until_verifier_and_operator_approval"
    );
    assert!(repair.is_read_only_preview());
    assert!(quarantine.is_read_only_preview());
    assert!(cut_preview.is_read_only_preview());
}

#[test]
fn gene_scissors_transaction_journal_records_cut_quarantine_and_regenerate_previews() {
    let preview = sample_quarantine_splice_preview();
    let journal = GeneScissorsTransactionJournal::from_splice_preview(&preview);
    let report = journal.replay();

    assert_eq!(
        journal.schema_version,
        GENE_SCISSORS_TRANSACTION_SCHEMA_VERSION
    );
    assert_eq!(journal.transactions.len(), 3);
    assert_eq!(report.quarantine_count, 1);
    assert_eq!(report.cut_preview_count, 1);
    assert_eq!(report.regenerate_preview_count, 1);
    assert!(report.passed_preview_gate());
    assert!(
        report
            .active_expression_excluded_segments
            .contains(&"segment:private-drift".to_owned()),
        "{:?}",
        report.active_expression_excluded_segments
    );
    assert!(
        report
            .forensic_copy_digests
            .iter()
            .all(|digest| digest.starts_with("redaction-digest:"))
    );
    assert!(journal.is_read_only_preview());
    assert!(journal.exports_are_redacted());

    let lines = journal.to_journal_lines();
    let loaded =
        GeneScissorsTransactionJournal::from_journal_lines(&lines).expect("journal roundtrip");
    assert_eq!(loaded.to_journal_lines(), lines);
    assert_eq!(loaded.stable_anchor_id, journal.stable_anchor_id);
    assert_eq!(loaded.replay().transaction_count, 3);

    let rollback = MutationPlan::preview(
        "mutation:segment:rollback-drift:rollback",
        GeneScissorsIntent::Rollback,
        "segment:rollback-drift",
        "rollback drift uses a distinct stable anchor",
        "restore from stable anchor without rewriting the journal anchor",
        "rollback:segment:drift",
    )
    .with_sources(["gene:stable:trusted"]);
    let journal = GeneScissorsTransactionJournal::from_mutation_plans(
        TaskProfile::Coding,
        "gene:stable:trusted",
        &[rollback],
    );
    let loaded = GeneScissorsTransactionJournal::from_journal_lines(&journal.to_journal_lines())
        .expect("distinct rollback anchor journal roundtrip");
    assert_eq!(loaded.stable_anchor_id, journal.stable_anchor_id);
    assert_ne!(
        loaded.stable_anchor_id,
        loaded.transactions[0].rollback_anchor_id
    );
}

#[test]
fn gene_scissors_transaction_journal_suppresses_duplicate_transactions() {
    let preview = sample_quarantine_splice_preview();
    let mut journal = GeneScissorsTransactionJournal::from_splice_preview(&preview);
    let transaction = journal.transactions[0].clone();

    assert!(!journal.append(transaction.clone()));
    assert!(!journal.append(transaction));
    let report = journal.replay();

    assert_eq!(report.transaction_count, 3);
    assert_eq!(report.duplicate_suppressed_count, 1);
    assert_eq!(journal.duplicate_transaction_ids.len(), 1);
}

#[test]
fn gene_scissors_transaction_journal_records_rollback_preview() {
    let rollback = MutationPlan::preview(
        "mutation:segment:rollback:rollback",
        GeneScissorsIntent::Rollback,
        "segment:rollback",
        "runtime drift rollback requires stable anchor replay",
        "restore the stable segment before any durable mutation is admitted",
        "genome:coding:stable",
    )
    .with_sources(["genome:coding:stable"]);
    let journal = GeneScissorsTransactionJournal::from_mutation_plans(
        TaskProfile::Coding,
        "genome:coding:stable",
        &[rollback],
    );
    let report = journal.replay();
    let transaction = journal.transactions.first().expect("rollback transaction");

    assert_eq!(
        transaction.state,
        GeneScissorsTransactionState::RollbackPreview
    );
    assert_eq!(report.rollback_preview_count, 1);
    assert!(
        report
            .active_expression_excluded_segments
            .contains(&"segment:rollback".to_owned())
    );
    assert_eq!(transaction.validation_status, GeneValidationStatus::Pending);
    assert!(!transaction.write_allowed);
    assert!(!transaction.applied);
}

#[test]
fn gene_scissors_transaction_journal_links_regeneration_child_lineage() {
    let preview = sample_quarantine_splice_preview();
    let journal = GeneScissorsTransactionJournal::from_splice_preview(&preview);
    let regenerate = journal
        .transactions
        .iter()
        .find(|transaction| transaction.state == GeneScissorsTransactionState::RegeneratePreview)
        .expect("regeneration transaction");

    assert_eq!(
        regenerate.replacement_segment_id.as_deref(),
        Some("segment:private-drift:young")
    );
    assert_eq!(
        regenerate.lineage_parent_id.as_deref(),
        Some("segment:private-drift")
    );
    assert!(
        regenerate
            .child_lineage_id
            .as_deref()
            .is_some_and(|lineage| lineage.starts_with("redaction-digest:"))
    );
    assert_eq!(regenerate.child_generation, 1);
    assert!(!regenerate.active_expression_allowed);
    assert!(!regenerate.memory_admission_allowed);
}

#[test]
fn gene_scissors_transaction_journal_redacts_payload_markers_from_exports() {
    let bad_target = "prompt: secret=raw hidden reasoning";
    let cut = MutationPlan::preview(
        "mutation:prompt:secret:cut",
        GeneScissorsIntent::Cut,
        bad_target,
        "prompt: secret=raw hidden reasoning should be held as digest-only evidence",
        "curl http://bad.example must never enter trace output",
        "genome:coding:stable",
    )
    .with_sources([bad_target]);
    let journal = GeneScissorsTransactionJournal::from_mutation_plans(
        TaskProfile::Coding,
        "genome:coding:stable",
        &[cut],
    );

    assert!(journal.exports_are_redacted());
    for line in journal
        .to_journal_lines()
        .into_iter()
        .chain(journal.to_redacted_trace_lines())
    {
        assert!(!line.contains("prompt:"));
        assert!(!line.contains("secret="));
        assert!(!line.contains("hidden reasoning"));
        assert!(!line.contains("curl "));
        assert!(!contains_private_or_executable_marker(&line), "{line}");
        assert!(line.contains("redaction-digest:"));
    }
}

fn sample_quarantine_splice_preview() -> DnaSplicePreview {
    let segments = vec![
        GeneSegment::new(
            "segment:private-drift",
            TaskProfile::Coding,
            GeneSegmentSource::RuntimeKv,
            0,
            32,
        )
        .with_source_hash("sha256:private-drift")
        .with_metadata(
            "private runtime drift",
            "must be quarantined before reuse",
            "runtime drift",
        )
        .with_kv_residency(GeneKvResidency::Sink)
        .with_health(0.80, 0.80, 0.50),
    ];

    DnaSplicer::default().preview(TaskProfile::Coding, "genome:coding:stable", segments)
}

#[test]
fn mutation_fixture_corpus_classifies_all_expected_categories() {
    let report = MutationRepairFixtureCorpus::default().evaluate();

    assert!(report.passed(), "{:?}", report.failures);
    assert_eq!(report.results.len(), 8);
    assert!(report.preview_only);
    assert!(report.total_repair_candidate_count >= 7);
    assert!(report.total_review_packet_line_count >= report.results.len());
    assert!(report.missing_fixture_kinds.is_empty());
    assert!(
        report
            .summary()
            .contains("mutation_fixture_corpus passed=true")
    );

    let insertion = report
        .result_for_kind(MutationFixtureKind::Insertion)
        .expect("insertion fixture");
    assert!(insertion.has_finding_kind(GeneVariantKind::Insertion));
    assert_eq!(
        insertion.mutated_disposition,
        Some(GeneSegmentDisposition::RepairCandidate)
    );
    assert!(insertion.protected_segments_retained);

    let deletion = report
        .result_for_kind(MutationFixtureKind::Deletion)
        .expect("deletion fixture");
    assert!(deletion.has_finding_kind(GeneVariantKind::Deletion));
    assert_eq!(
        deletion.lifecycle_state,
        Some(GeneScissorsLifecycleState::RepairCandidate)
    );

    let truncation = report
        .result_for_kind(MutationFixtureKind::Truncation)
        .expect("truncation fixture");
    assert!(truncation.has_finding_kind(GeneVariantKind::Truncation));

    let schema_drift = report
        .result_for_kind(MutationFixtureKind::SchemaDrift)
        .expect("schema drift fixture");
    assert!(schema_drift.has_finding_kind(GeneVariantKind::Schema));
    assert!(schema_drift.has_finding_kind(GeneVariantKind::Drift));
    assert_eq!(
        schema_drift.mutated_disposition,
        Some(GeneSegmentDisposition::Quarantined)
    );

    let contradiction = report
        .result_for_kind(MutationFixtureKind::ContradictoryPolicy)
        .expect("contradictory policy fixture");
    assert!(contradiction.has_finding_kind(GeneVariantKind::Contradiction));

    let stale = report
        .result_for_kind(MutationFixtureKind::StaleLabel)
        .expect("stale label fixture");
    assert!(stale.has_finding_kind(GeneVariantKind::StaleLabel));
}

#[test]
fn mutation_fixture_malicious_payload_stays_inert_and_quarantined() {
    let report = MutationRepairFixtureCorpus::default().evaluate();
    let malicious = report
        .result_for_kind(MutationFixtureKind::MaliciousInstruction)
        .expect("malicious fixture");

    assert!(malicious.passed(), "{:?}", malicious.failures);
    assert!(malicious.has_finding_kind(GeneVariantKind::Drift));
    assert!(malicious.has_finding_kind(GeneVariantKind::Privacy));
    assert_eq!(
        malicious.mutated_disposition,
        Some(GeneSegmentDisposition::Quarantined)
    );
    assert_eq!(
        malicious.lifecycle_state,
        Some(GeneScissorsLifecycleState::Quarantined)
    );
    assert!(malicious.protected_segments_retained);
    assert_eq!(
        malicious.protected_segment_summaries,
        vec![
            "segment:malicious-express:genome_ledger:retained".to_owned(),
            "segment:malicious-memory:semantic_memory:retained".to_owned(),
        ]
    );
    assert!(malicious.payload_digest.starts_with("fixture-digest:"));
    assert!(malicious.sanitized_payload_summary.contains("digest-only"));
    assert!(
        !malicious
            .review_packet_lines
            .iter()
            .any(|line| contains_executable_payload_marker(line))
    );
}

#[test]
fn mutation_fixture_repair_candidates_have_digests_and_preview_gates() {
    let report = MutationRepairFixtureCorpus::default().evaluate();

    assert!(report.passed(), "{:?}", report.failures);
    for result in report
        .results
        .iter()
        .filter(|result| result.mutated_segment_id.is_some())
    {
        assert!(
            result
                .before_digest
                .as_deref()
                .is_some_and(|digest| digest.starts_with("fixture-digest:")),
            "missing before digest for {}",
            result.fixture_id
        );
        assert!(
            !result.repair_candidates.is_empty(),
            "missing repair candidates for {}",
            result.fixture_id
        );
        for candidate in &result.repair_candidates {
            assert!(candidate.before_digest.starts_with("fixture-digest:"));
            assert!(candidate.after_digest.starts_with("fixture-digest:"));
            assert_ne!(candidate.before_digest, candidate.after_digest);
            assert_eq!(candidate.rollback_anchor_id, "genome:fixture:stable");
            assert!(!candidate.validation_gates.is_empty());
            assert_eq!(candidate.validation_status, GeneValidationStatus::Pending);
            assert!(candidate.preview_only);
            assert!(!candidate.admission_write_authorized);
            assert!(!candidate.applied);
        }
        assert!(
            result
                .review_packet_lines
                .iter()
                .any(|line| line.contains("mutation_fixture_repair"))
        );
    }
}

#[test]
fn mutation_fixture_gate_fails_when_required_coverage_is_missing() {
    let mut corpus = MutationRepairFixtureCorpus::default();
    corpus
        .fixtures
        .retain(|fixture| fixture.kind != MutationFixtureKind::MaliciousInstruction);

    let report = corpus.evaluate();
    let gate = report.gate_report();

    assert!(!report.passed());
    assert!(!gate.passed);
    assert!(
        gate.missing_fixture_kinds
            .contains(&MutationFixtureKind::MaliciousInstruction)
    );
    assert!(gate.failures.iter().any(|failure| {
        failure.contains("mutation_fixture_coverage_missing:malicious_instruction")
    }));
}

#[test]
fn malignant_gene_recovery_drills_cover_all_poisoning_categories() {
    let report = MalignantGeneRecoveryDrillCorpus::default().evaluate();

    assert!(report.passed(), "{:?}", report.failures);
    assert_eq!(report.results.len(), 6);
    assert_eq!(report.missing_fixture_kinds, Vec::new());
    assert_eq!(report.quarantined_count, 6);
    assert_eq!(report.cut_candidate_count, 6);
    assert_eq!(report.regeneration_candidate_count, 6);
    assert_eq!(report.failed_replay_count, 1);
    assert!(report.preview_only);
    assert!(
        report
            .summary()
            .contains("malignant_gene_recovery_drills passed=true")
    );
}

#[test]
fn malignant_gene_recovery_quarantines_cuts_and_regenerates_without_promotion() {
    let report = MalignantGeneRecoveryDrillCorpus::default().evaluate();

    for result in &report.results {
        assert_eq!(result.classification, "malignant_quarantined");
        assert!(result.confidence >= 0.90, "{:?}", result);
        assert!(result.quarantine_plan_present, "{:?}", result);
        assert!(result.cut_candidate_present, "{:?}", result);
        assert!(result.regeneration_candidate_present, "{:?}", result);
        assert!(result.tombstone_id.is_some(), "{:?}", result);
        assert_eq!(result.rollback_anchor_id, "genome:malignant-drill:stable");
        assert!(result.preview_only, "{:?}", result);
        assert!(
            !result.evidence_packet_lines.iter().any(|line| {
                line.contains("write_allowed=true") || line.contains("applied=true")
            })
        );
    }
}

#[test]
fn malignant_gene_recovery_regenerates_from_anchor_not_bad_payload() {
    let report = MalignantGeneRecoveryDrillCorpus::default().evaluate();

    for result in &report.results {
        assert!(
            result
                .trusted_regeneration_sources
                .contains(&"genome:malignant-drill:stable".to_owned()),
            "{:?}",
            result.trusted_regeneration_sources
        );
        assert!(
            !result.copied_bad_payload_source,
            "{} copied target into regeneration sources {:?}",
            result.fixture_id, result.trusted_regeneration_sources
        );
        assert!(
            !result
                .trusted_regeneration_sources
                .contains(&result.target_segment_id),
            "{} target leaked into regeneration sources",
            result.fixture_id
        );
    }
}

#[test]
fn malignant_gene_recovery_failed_delete_attempt_keeps_hold_reasons() {
    let report = MalignantGeneRecoveryDrillCorpus::default().evaluate();
    let delete_attempt = report
        .result_for_kind(MalignantGeneDrillKind::IrreversibleDeleteAttempt)
        .expect("irreversible-delete drill");

    assert_eq!(
        delete_attempt.validation_status,
        GeneValidationStatus::Failed
    );
    assert_eq!(delete_attempt.approval_decision, "rejected_hold");
    for reason in [
        "destructive_intent_blocked",
        "replay_validation_failed",
        "operator_approval_required",
        "preview_only",
    ] {
        assert!(
            delete_attempt.hold_reasons.contains(&reason.to_owned()),
            "{:?}",
            delete_attempt.hold_reasons
        );
    }
}

#[test]
fn malignant_gene_recovery_evidence_packets_are_redacted() {
    let report = MalignantGeneRecoveryDrillCorpus::default().evaluate();

    for result in &report.results {
        assert_eq!(result.redaction_status, "redacted");
        assert!(result.payload_digest.starts_with("redaction-digest:"));
        assert!(result.protected_segments_retained);
        for line in &result.evidence_packet_lines {
            assert!(
                !contains_private_or_executable_marker(line),
                "{} leaked private marker in {line}",
                result.fixture_id
            );
            assert!(line.contains("redaction_status=redacted"));
            assert!(
                line.contains("payload_digest=redaction-digest:")
                    || line.contains("summary=digest-only")
            );
        }
    }
}

#[test]
fn gene_purpose_relabel_accepts_fresh_preview_without_mutating_current_record() {
    let current = sample_gene_purpose_record();
    let evidence = sample_gene_purpose_evidence();
    let current_label = current.label.clone();

    let proposal = GenePurposeRelabelValidator::default().validate(&current, &evidence);

    assert!(proposal.accepted(), "{:?}", proposal.reason_codes);
    assert_eq!(
        proposal.decision,
        GenePurposeRelabelDecision::AcceptedPreview
    );
    assert_eq!(proposal.validation_status, GeneValidationStatus::Pending);
    assert!(proposal.preview_only);
    assert!(proposal.approval_required);
    assert!(!proposal.write_allowed);
    assert!(!proposal.applied);
    assert!(proposal.proposed_record.is_preview_only());
    assert_eq!(current.label, current_label);
    assert_eq!(current.freshness, GenePurposeFreshness::Fresh);
    assert!(current.stable_id.starts_with("redaction-digest:"));
    assert!(current.provenance_digest.starts_with("redaction-digest:"));
    assert!(current.purpose_digest.starts_with("redaction-digest:"));

    let first_line = proposal.proposed_record.to_kv_line();
    let second_line = proposal.proposed_record.to_kv_line();
    assert_eq!(first_line, second_line);
    assert!(first_line.contains(GENE_PURPOSE_ONTOLOGY_VERSION));
    assert!(first_line.contains("redaction-digest:"));
    assert!(!contains_private_or_executable_marker(
        &proposal.summary_line()
    ));
}

#[test]
fn gene_purpose_relabel_accepts_aged_low_fitness_preview() {
    let aged_gene = ReasoningGene::new(
        "gene:test:aging-routing",
        ReasoningGeneKind::Routing,
        "routing threshold tuner",
        "keeps adaptive attention thresholds bounded by task and hardware evidence",
    )
    .with_tags(["routing", "threshold"])
    .with_health(9, 0.50, 0.18);
    let current = GenePurposeRecord::from_reasoning_gene(
        TaskProfile::Coding,
        "tenant:local",
        GenePurposeEvidenceClass::HealthMetadata,
        "genome:test:stable",
        &aged_gene,
    );
    let evidence =
        sample_gene_purpose_evidence().with_health(GenePurposeFreshness::Aging, 0.48, 0.62);

    let proposal = GenePurposeRelabelValidator::default().validate(&current, &evidence);

    assert!(proposal.accepted(), "{:?}", proposal.reason_codes);
    assert_eq!(current.freshness, GenePurposeFreshness::Aging);
    assert_eq!(
        proposal.proposed_record.freshness,
        GenePurposeFreshness::Aging
    );
    assert!(proposal.proposed_record.is_preview_only());
    for tag in ["purpose_ontology", "relabel", "preview_only"] {
        assert!(
            proposal.proposed_tags.contains(&tag.to_owned()),
            "{:?}",
            proposal.proposed_tags
        );
    }
}

#[test]
fn gene_purpose_relabel_quarantines_conflicting_labels() {
    let current = sample_gene_purpose_record();
    let evidence = GenePurposeRelabelEvidence::new(
        GenePurposeEvidenceClass::Reflection,
        stable_redaction_digest(["evidence", "conflict"]),
        "digest-only reflection found a contradiction",
        "conflicting memory router",
        "contradict the current purpose with a mutually exclusive runtime policy",
        "genome:test:stable",
    );

    let proposal = GenePurposeRelabelValidator::default().validate(&current, &evidence);

    assert!(proposal.quarantined());
    assert_eq!(proposal.validation_status, GeneValidationStatus::Failed);
    assert!(
        proposal
            .reason_codes
            .contains(&"conflicting_relabel".to_owned()),
        "{:?}",
        proposal.reason_codes
    );
    assert!(
        proposal
            .reason_codes
            .contains(&"contradictory_label".to_owned()),
        "{:?}",
        proposal.reason_codes
    );
}

#[test]
fn gene_purpose_relabel_quarantines_missing_rollback_anchor() {
    let mut current = sample_gene_purpose_record();
    current.rollback_anchor_id.clear();
    let evidence = sample_gene_purpose_evidence();

    let proposal = GenePurposeRelabelValidator::default().validate(&current, &evidence);

    assert!(proposal.quarantined());
    assert_eq!(proposal.validation_status, GeneValidationStatus::Failed);
    assert!(
        proposal
            .reason_codes
            .contains(&"missing_rollback_anchor".to_owned()),
        "{:?}",
        proposal.reason_codes
    );
}

#[test]
fn gene_purpose_relabel_quarantines_stale_evidence() {
    let current = sample_gene_purpose_record();
    let evidence =
        sample_gene_purpose_evidence().with_health(GenePurposeFreshness::Stale, 0.64, 0.70);

    let proposal = GenePurposeRelabelValidator::default().validate(&current, &evidence);

    assert!(proposal.quarantined());
    assert_eq!(proposal.validation_status, GeneValidationStatus::Failed);
    assert!(
        proposal.reason_codes.contains(&"stale_evidence".to_owned()),
        "{:?}",
        proposal.reason_codes
    );
}

#[test]
fn gene_purpose_relabel_redacts_private_payloads_from_preview_record() {
    let current = sample_gene_purpose_record();
    let evidence = GenePurposeRelabelEvidence::new(
        GenePurposeEvidenceClass::Reflection,
        stable_redaction_digest(["evidence", "private"]),
        "prompt: keep secret=raw in digest only",
        "prompt: private routing note",
        "secret=raw should never enter a preview purpose record",
        "genome:test:stable",
    )
    .with_tags(["secret=raw"])
    .without_privacy_check();

    let proposal = GenePurposeRelabelValidator::default().validate(&current, &evidence);

    assert!(proposal.quarantined());
    assert_eq!(proposal.validation_status, GeneValidationStatus::Failed);
    for reason in ["privacy_gate_missing", "private_payload_marker"] {
        assert!(
            proposal.reason_codes.contains(&reason.to_owned()),
            "{:?}",
            proposal.reason_codes
        );
    }
    let summary = proposal.summary_line();
    let kv_line = proposal.proposed_record.to_kv_line();
    assert!(summary.contains("redaction-digest:"));
    assert!(kv_line.contains("redaction-digest:"));
    assert!(!summary.contains("prompt:"));
    assert!(!summary.contains("secret="));
    assert!(!kv_line.contains("prompt:"));
    assert!(!kv_line.contains("secret="));
    assert!(!contains_private_or_executable_marker(&summary));
    assert!(!contains_private_or_executable_marker(&kv_line));
}

fn sample_gene_purpose_record() -> GenePurposeRecord {
    let gene = ReasoningGene::new(
        "gene:test:retrieval-purpose",
        ReasoningGeneKind::Retrieval,
        "memory retrieval controller",
        "select useful semantic, gist, and runtime KV memory with bounded evidence",
    )
    .with_tags(["retrieval", "memory_chain"])
    .with_health(2, 0.82, 0.06);

    GenePurposeRecord::from_reasoning_gene(
        TaskProfile::Coding,
        "local_scope",
        GenePurposeEvidenceClass::Reflection,
        "genome:test:stable",
        &gene,
    )
}

fn sample_gene_purpose_evidence() -> GenePurposeRelabelEvidence {
    GenePurposeRelabelEvidence::new(
        GenePurposeEvidenceClass::Reflection,
        stable_redaction_digest(["evidence", "purpose", "retrieval"]),
        "digest-only reflection confirms retrieval purpose",
        "bounded memory retrieval selector",
        "selects validated memory and runtime KV evidence for coding tasks",
        "genome:test:stable",
    )
    .with_tags(["memory", "coding", "bounded_kv"])
}

fn contains_executable_payload_marker(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    [
        "rm ",
        "curl ",
        "wget ",
        "powershell",
        "cmd.exe",
        "sudo ",
        "api_key",
        "private key",
        "secret=",
    ]
    .iter()
    .any(|marker| line.contains(marker))
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

    assert_eq!(loaded.schema_version, "dna_chain_v2");
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
            .all(|record| record.decay_score > 0.0 && record.decay_score <= 1.0)
    );
    assert!(
        loaded
            .memory_chain
            .iter()
            .all(|record| record.operator_approval_required && !record.applied)
    );
}

#[test]
fn replication_proofread_review_gates_copy_and_fork_fixtures_preview_only() {
    let genome = ReasoningGenome::default_for_profile(TaskProfile::Coding);
    let source = DnaGeneSourceEvidence::new(
        DnaGeneEvidenceKind::SyntheticDefault,
        "sha256:genome-default",
        "default profile genome scaffold",
    );
    let chain =
        DnaGeneChain::preview_from_genome(&genome, "tenant:local", "session:proofread", source);
    let task_gene = chain.express_chain.first().expect("task gene");

    let exact = task_gene.replication_proofread_review(
        task_gene.source_evidence.source_hash.clone(),
        "tenant:local",
        "exact_copy",
        ["gene_kind", "purpose", "rollback_anchor"],
        ["gene_kind", "purpose", "rollback_anchor"],
        0,
    );
    assert_eq!(
        exact.repair_action,
        ReplicationRepairAction::AcceptExactCopy
    );
    assert!(exact.mismatch_fields.is_empty());

    let trace_prior = ReplicationProofreadReview::from_input(ReplicationProofreadInput::new(
        "trace_segment:runtime-prior",
        "sha256:trace-source",
        "sha256:trace-copy",
        "lineage:trace-parent",
        "scope:trace-replay",
        "scoped_mutation",
        ["trace_digest", "replay_weight"],
        ["trace_digest", "replay_weight"],
        -1,
    ));
    assert_eq!(
        trace_prior.repair_action,
        ReplicationRepairAction::HoldMinorMismatch
    );
    assert_eq!(trace_prior.mutation_budget_delta, -1);
    assert!(
        trace_prior
            .mismatch_fields
            .contains(&"copy_digest".to_owned())
    );

    let missing_source = ReplicationProofreadReview::from_input(ReplicationProofreadInput::new(
        "memory_candidate:lesson",
        "",
        "sha256:copy",
        "lineage:memory-parent",
        "scope:memory",
        "exact_copy",
        ["source_digest", "content_digest"],
        ["source_digest", "content_digest"],
        0,
    ));
    assert_eq!(
        missing_source.repair_action,
        ReplicationRepairAction::RejectMajorMismatch
    );

    let scope_mismatch = task_gene.replication_proofread_review(
        task_gene.source_evidence.source_hash.clone(),
        "tenant:other",
        "exact_copy",
        ["gene_kind"],
        ["gene_kind"],
        0,
    );
    assert_eq!(
        scope_mismatch.repair_action,
        ReplicationRepairAction::RejectMajorMismatch
    );
    assert!(
        scope_mismatch
            .mismatch_fields
            .contains(&"target_scope".to_owned())
    );

    let model_cell = ReplicationProofreadReview::from_input(ReplicationProofreadInput::new(
        "model_cell:quality-worker",
        "sha256:model-policy-a",
        "sha256:model-policy-b",
        "lineage:model-pool",
        "scope:model-cell",
        "",
        ["route_policy", "adapter_digest"],
        ["route_policy", "adapter_digest"],
        0,
    ));
    assert_eq!(
        model_cell.repair_action,
        ReplicationRepairAction::QuarantineUnexplainedDrift
    );

    let handoff = ReplicationProofreadReview::from_input(ReplicationProofreadInput::new(
        "prompt: secret=raw handoff",
        "sha256:handoff-a",
        "sha256:handoff-b",
        "lineage:agent-window",
        "scope:handoff-lesson",
        "",
        ["packet_digest", "provenance_digest"],
        ["packet_digest", "provenance_digest"],
        0,
    ));
    assert_eq!(
        handoff.repair_action,
        ReplicationRepairAction::QuarantineUnexplainedDrift
    );

    let transaction =
        GeneScissorsTransactionJournal::from_splice_preview(&sample_quarantine_splice_preview())
            .transactions
            .into_iter()
            .find(|transaction| {
                transaction.state == GeneScissorsTransactionState::RegeneratePreview
            })
            .expect("regeneration transaction");
    let gene_candidate = transaction.replication_proofread_review(
        "segment:private-drift:young",
        ["before_digest", "after_digest"],
        ["before_digest", "after_digest"],
        -1,
    );
    assert_eq!(
        gene_candidate.repair_action,
        ReplicationRepairAction::HoldMinorMismatch
    );

    for review in [
        &exact,
        &trace_prior,
        &missing_source,
        &scope_mismatch,
        &model_cell,
        &handoff,
        &gene_candidate,
    ] {
        assert!(review.is_preview_only());
        assert!(!review.can_authorize_write());
        assert!(!review.write_allowed);
        assert!(!review.applied);
        assert!(!contains_private_or_executable_marker(
            &review.summary_line()
        ));
    }
}

#[test]
fn lineage_audit_from_dual_chain_tracks_express_memory_parent_edges() {
    let genome = ReasoningGenome::default_for_profile(TaskProfile::Coding);
    let source = DnaGeneSourceEvidence::new(
        DnaGeneEvidenceKind::SyntheticDefault,
        "sha256:genome-default",
        "prompt: secret=raw answer: hidden source summary must stay out of audit exports",
    );
    let mut chain =
        DnaGeneChain::preview_from_genome(&genome, "tenant:local", "session:audit", source);

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
        DnaGeneLineage::new("tenant:local", "session:audit")
            .with_parent("gene:coding:tool-use")
            .with_inheritance(chain.stable_anchor_id.clone(), 1),
        DnaGeneSourceEvidence::new(
            DnaGeneEvidenceKind::ToolReliability,
            "sha256:tool-reliability",
            "answer: private key should not leave digest-only audit",
        ),
        &memory_gene,
    );
    chain.push_memory_record(memory_record);

    let packet = DnaLineageAuditPacket::from_gene_chain(&chain);
    let memory_node = packet
        .nodes_for_source("gene:coding:memory-tool-reliability")
        .into_iter()
        .find(|node| node.kind == DnaLineageAuditNodeKind::GeneRecord)
        .expect("memory-chain gene node");

    assert_eq!(memory_node.chain_kind.as_deref(), Some("memory_chain"));
    assert_eq!(memory_node.gate_status, "preview_only");
    assert!(
        memory_node
            .reason_codes
            .contains(&"tool_reliability".to_owned())
    );
    assert!(packet.edges.iter().any(|edge| {
        edge.parent_id == "gene:gene:coding:tool-use"
            && edge.child_id == memory_node.id
            && edge.relation == "parent_gene"
    }));
    let json = packet.to_redacted_json();
    assert!(!json.contains("prompt:"));
    assert!(!json.contains("answer:"));
    assert!(!json.contains("secret="));
    assert!(packet.exports_are_redacted());
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
