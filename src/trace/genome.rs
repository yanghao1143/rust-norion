use super::fields::*;

pub(super) fn evaluate_trace_reasoning_genome(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(genome) = json_object_after_field(line, "reasoning_genome") else {
        failures.push("reasoning_genome object is missing or invalid".to_owned());
        return failures;
    };

    let genome_id = extract_json_string_field(genome, "genome_id").unwrap_or_default();
    let stable_anchor_id =
        extract_json_string_field(genome, "stable_anchor_id").unwrap_or_default();
    let gene_count = extract_json_usize_field(genome, "gene_count").unwrap_or(0);
    let active_genes = extract_json_usize_field(genome, "active_genes").unwrap_or(0);
    let aged_genes = extract_json_usize_field(genome, "aged_genes").unwrap_or(0);
    let malignant_genes = extract_json_usize_field(genome, "malignant_genes").unwrap_or(0);
    let relabel_candidates = extract_json_usize_field(genome, "relabel_candidates").unwrap_or(0);
    let regeneration_candidates =
        extract_json_usize_field(genome, "regeneration_candidates").unwrap_or(0);
    let gene_scissors_proposals =
        extract_json_usize_field(genome, "gene_scissors_proposals").unwrap_or(0);
    let mutation_intents =
        extract_json_string_array_field(genome, "mutation_intents").unwrap_or_default();
    let proposal_ids = extract_json_string_array_field(genome, "proposal_ids").unwrap_or_default();
    let read_only = extract_json_bool_field(genome, "read_only");
    let write_allowed = extract_json_bool_field(genome, "write_allowed");
    let mutation_applied = extract_json_bool_field(genome, "mutation_applied");
    let youth_pressure = extract_json_f32_field(genome, "youth_pressure").unwrap_or(f32::NAN);

    if genome_id.trim().is_empty() || !genome_id.starts_with("genome:") {
        failures.push("reasoning_genome genome_id must be a non-empty genome: id".to_owned());
    }
    if stable_anchor_id.trim().is_empty() || !stable_anchor_id.contains(":stable") {
        failures.push(
            "reasoning_genome stable_anchor_id must name a stable rollback anchor".to_owned(),
        );
    }
    if gene_count == 0 {
        failures.push("reasoning_genome gene_count must be > 0".to_owned());
    }
    if active_genes > gene_count {
        failures.push(format!(
            "reasoning_genome active_genes {active_genes} exceeds gene_count {gene_count}"
        ));
    }
    if aged_genes > gene_count || malignant_genes > gene_count {
        failures.push(format!(
            "reasoning_genome aged/malignant counts exceed gene_count {gene_count}"
        ));
    }
    if relabel_candidates < aged_genes && aged_genes > 0 {
        failures.push(format!(
            "reasoning_genome aged_genes {aged_genes} require relabel_candidates >= aged_genes"
        ));
    }
    if regeneration_candidates < malignant_genes && malignant_genes > 0 {
        failures.push(format!(
            "reasoning_genome malignant_genes {malignant_genes} require regeneration_candidates >= malignant_genes"
        ));
    }
    if proposal_ids.len() != gene_scissors_proposals {
        failures.push(format!(
            "reasoning_genome proposal_ids {} do not match gene_scissors_proposals {gene_scissors_proposals}",
            proposal_ids.len()
        ));
    }
    if gene_scissors_proposals > 0 && mutation_intents.is_empty() {
        failures
            .push("reasoning_genome gene_scissors_proposals require mutation_intents".to_owned());
    }
    if malignant_genes > 0 {
        require_intent(&mut failures, &mutation_intents, "quarantine");
        require_intent(&mut failures, &mutation_intents, "regenerate");
    }
    if aged_genes > 0 {
        require_intent(&mut failures, &mutation_intents, "relabel");
    }
    if read_only != Some(true) {
        failures.push("reasoning_genome read_only must be true".to_owned());
    }
    if write_allowed != Some(false) {
        failures.push("reasoning_genome write_allowed must be false".to_owned());
    }
    if mutation_applied != Some(false) {
        failures.push("reasoning_genome mutation_applied must be false".to_owned());
    }
    if !(0.0..=1.0).contains(&youth_pressure) {
        failures.push(format!(
            "reasoning_genome youth_pressure {youth_pressure:.6} must stay within 0.0..=1.0"
        ));
    }

    failures
}

fn require_intent(failures: &mut Vec<String>, mutation_intents: &[String], expected: &str) {
    if !mutation_intents.iter().any(|intent| intent == expected) {
        failures.push(format!(
            "reasoning_genome mutation_intents must include {expected}"
        ));
    }
}
