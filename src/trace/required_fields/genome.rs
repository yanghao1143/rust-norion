use super::{TraceRequiredField, required_field};

pub(super) const GENOME_TRACE_REQUIRED_FIELDS: &[TraceRequiredField] = &[
    required_field("reasoning_genome", "\"reasoning_genome\":{"),
    required_field("reasoning_genome_id", "\"genome_id\":"),
    required_field("reasoning_genome_stable_anchor", "\"stable_anchor_id\":"),
    required_field("reasoning_genome_strategy", "\"strategy\":"),
    required_field(
        "reasoning_genome_strategy_genome_id",
        "\"strategy_genome_id\":",
    ),
    required_field(
        "reasoning_genome_strategy_gene_count",
        "\"strategy_gene_count\":",
    ),
    required_field(
        "reasoning_genome_generation_before",
        "\"generation_before\":",
    ),
    required_field("reasoning_genome_generation_after", "\"generation_after\":"),
    required_field(
        "reasoning_genome_active_genome_id_after",
        "\"active_genome_id_after\":",
    ),
    required_field(
        "reasoning_genome_reasoning_frame_id",
        "\"reasoning_frame_id\":",
    ),
    required_field(
        "reasoning_genome_reasoning_frame_valid",
        "\"reasoning_frame_valid\":",
    ),
    required_field(
        "reasoning_genome_reasoning_frame_vm_executed",
        "\"reasoning_frame_vm_executed\":",
    ),
    required_field(
        "reasoning_genome_reasoning_frame_opcode_count",
        "\"reasoning_frame_opcode_count\":",
    ),
    required_field(
        "reasoning_genome_reasoning_frame_opcodes",
        "\"reasoning_frame_opcodes\":",
    ),
    required_field(
        "reasoning_genome_reasoning_frame_routing_bias",
        "\"reasoning_frame_routing_bias\":",
    ),
    required_field(
        "reasoning_genome_reasoning_frame_memory_policy",
        "\"reasoning_frame_memory_policy\":",
    ),
    required_field(
        "reasoning_genome_reasoning_frame_mutation_previews",
        "\"reasoning_frame_mutation_previews\":",
    ),
    required_field(
        "reasoning_genome_task_gene_decision",
        "\"task_gene_decision\":",
    ),
    required_field(
        "reasoning_genome_task_skill_decision",
        "\"task_skill_decision\":",
    ),
    required_field(
        "reasoning_genome_writer_gate_decision",
        "\"writer_gate_decision\":",
    ),
    required_field(
        "reasoning_genome_apply_plan_decision",
        "\"apply_plan_decision\":",
    ),
    required_field("reasoning_genome_mutation_count", "\"mutation_count\":"),
    required_field(
        "reasoning_genome_dual_chain_committed",
        "\"dual_chain_committed\":",
    ),
    required_field(
        "reasoning_genome_express_chain_records",
        "\"express_chain_records\":",
    ),
    required_field(
        "reasoning_genome_memory_chain_records",
        "\"memory_chain_records\":",
    ),
    required_field("reasoning_genome_rollback_applied", "\"rollback_applied\":"),
    required_field("reasoning_genome_chain_records", "\"chain_records\":"),
    required_field(
        "reasoning_genome_lineage_scope_digests",
        "\"lineage_scope_digests\":",
    ),
    required_field("reasoning_genome_mixed_lineage", "\"mixed_lineage\":"),
    required_field("reasoning_genome_gene_count", "\"gene_count\":"),
    required_field("reasoning_genome_active_genes", "\"active_genes\":"),
    required_field("reasoning_genome_aged_genes", "\"aged_genes\":"),
    required_field("reasoning_genome_malignant_genes", "\"malignant_genes\":"),
    required_field(
        "reasoning_genome_relabel_candidates",
        "\"relabel_candidates\":",
    ),
    required_field(
        "reasoning_genome_regeneration_candidates",
        "\"regeneration_candidates\":",
    ),
    required_field(
        "reasoning_genome_gene_scissors_proposals",
        "\"gene_scissors_proposals\":",
    ),
    required_field("reasoning_genome_repair_payloads", "\"repair_payloads\":"),
    required_field(
        "reasoning_genome_regeneration_payloads",
        "\"regeneration_payloads\":",
    ),
    required_field("reasoning_genome_mutation_intents", "\"mutation_intents\":"),
    required_field("reasoning_genome_proposal_ids", "\"proposal_ids\":"),
    required_field("reasoning_genome_read_only", "\"read_only\":"),
    required_field("reasoning_genome_write_allowed", "\"write_allowed\":"),
    required_field("reasoning_genome_mutation_applied", "\"mutation_applied\":"),
    required_field("reasoning_genome_youth_pressure", "\"youth_pressure\":"),
    required_field(
        "reasoning_genome_lifecycle_records",
        "\"lifecycle_records\":",
    ),
    required_field(
        "reasoning_genome_lifecycle_actions",
        "\"lifecycle_actions\":",
    ),
    required_field(
        "reasoning_genome_lifecycle_summaries",
        "\"lifecycle_summaries\":",
    ),
    required_field(
        "reasoning_genome_lifecycle_tombstone_candidates",
        "\"lifecycle_tombstone_candidates\":",
    ),
    required_field(
        "reasoning_genome_lifecycle_pending_validations",
        "\"lifecycle_pending_validations\":",
    ),
    required_field(
        "reasoning_genome_lifecycle_source_evidence",
        "\"lifecycle_source_evidence\":",
    ),
    required_field("reasoning_genome_splice_segments", "\"splice_segments\":"),
    required_field("reasoning_genome_splice_exons", "\"splice_exons\":"),
    required_field("reasoning_genome_splice_introns", "\"splice_introns\":"),
    required_field("reasoning_genome_splice_variants", "\"splice_variants\":"),
    required_field("reasoning_genome_splice_retained", "\"splice_retained\":"),
    required_field("reasoning_genome_splice_skipped", "\"splice_skipped\":"),
    required_field(
        "reasoning_genome_splice_quarantined",
        "\"splice_quarantined\":",
    ),
    required_field(
        "reasoning_genome_splice_repair_candidates",
        "\"splice_repair_candidates\":",
    ),
    required_field(
        "reasoning_genome_splice_dispositions",
        "\"splice_dispositions\":",
    ),
    required_field(
        "reasoning_genome_splice_reason_summaries",
        "\"splice_reason_summaries\":",
    ),
    required_field(
        "reasoning_genome_splice_lifecycle_records",
        "\"splice_lifecycle_records\":",
    ),
    required_field(
        "reasoning_genome_splice_lifecycle_states",
        "\"splice_lifecycle_states\":",
    ),
    required_field(
        "reasoning_genome_splice_lifecycle_summaries",
        "\"splice_lifecycle_summaries\":",
    ),
    required_field("reasoning_genome_splice_findings", "\"splice_findings\":"),
    required_field(
        "reasoning_genome_splice_finding_kinds",
        "\"splice_finding_kinds\":",
    ),
    required_field(
        "reasoning_genome_splice_mutation_intents",
        "\"splice_mutation_intents\":",
    ),
    required_field("reasoning_genome_splice_proposals", "\"splice_proposals\":"),
    required_field(
        "reasoning_genome_splice_proposal_ids",
        "\"splice_proposal_ids\":",
    ),
    required_field("reasoning_genome_splice_read_only", "\"splice_read_only\":"),
    required_field(
        "reasoning_genome_splice_write_allowed",
        "\"splice_write_allowed\":",
    ),
    required_field("reasoning_genome_splice_applied", "\"splice_applied\":"),
];
