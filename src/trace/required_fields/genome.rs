use super::{TraceRequiredField, required_field};

pub(super) const GENOME_TRACE_REQUIRED_FIELDS: &[TraceRequiredField] = &[
    required_field("reasoning_genome", "\"reasoning_genome\":{"),
    required_field("reasoning_genome_id", "\"genome_id\":"),
    required_field("reasoning_genome_stable_anchor", "\"stable_anchor_id\":"),
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
    required_field("reasoning_genome_mutation_intents", "\"mutation_intents\":"),
    required_field("reasoning_genome_proposal_ids", "\"proposal_ids\":"),
    required_field("reasoning_genome_read_only", "\"read_only\":"),
    required_field("reasoning_genome_write_allowed", "\"write_allowed\":"),
    required_field("reasoning_genome_mutation_applied", "\"mutation_applied\":"),
    required_field("reasoning_genome_youth_pressure", "\"youth_pressure\":"),
    required_field("reasoning_genome_splice_segments", "\"splice_segments\":"),
    required_field("reasoning_genome_splice_exons", "\"splice_exons\":"),
    required_field("reasoning_genome_splice_introns", "\"splice_introns\":"),
    required_field("reasoning_genome_splice_variants", "\"splice_variants\":"),
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
