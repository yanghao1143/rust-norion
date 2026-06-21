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
    let repair_payloads = extract_json_usize_field(genome, "repair_payloads").unwrap_or(0);
    let regeneration_payloads =
        extract_json_usize_field(genome, "regeneration_payloads").unwrap_or(0);
    let mutation_intents =
        extract_json_string_array_field(genome, "mutation_intents").unwrap_or_default();
    let proposal_ids = extract_json_string_array_field(genome, "proposal_ids").unwrap_or_default();
    let read_only = extract_json_bool_field(genome, "read_only");
    let write_allowed = extract_json_bool_field(genome, "write_allowed");
    let mutation_applied = extract_json_bool_field(genome, "mutation_applied");
    let youth_pressure = extract_json_f32_field(genome, "youth_pressure").unwrap_or(f32::NAN);
    let splice_segments = extract_json_usize_field(genome, "splice_segments").unwrap_or(0);
    let splice_exons = extract_json_usize_field(genome, "splice_exons").unwrap_or(0);
    let splice_introns = extract_json_usize_field(genome, "splice_introns").unwrap_or(0);
    let splice_variants = extract_json_usize_field(genome, "splice_variants").unwrap_or(0);
    let splice_retained = extract_json_usize_field(genome, "splice_retained").unwrap_or(0);
    let splice_skipped = extract_json_usize_field(genome, "splice_skipped").unwrap_or(0);
    let splice_quarantined = extract_json_usize_field(genome, "splice_quarantined").unwrap_or(0);
    let splice_repair_candidates =
        extract_json_usize_field(genome, "splice_repair_candidates").unwrap_or(0);
    let splice_dispositions =
        extract_json_string_array_field(genome, "splice_dispositions").unwrap_or_default();
    let splice_reason_summaries =
        extract_json_string_array_field(genome, "splice_reason_summaries").unwrap_or_default();
    let splice_findings = extract_json_usize_field(genome, "splice_findings").unwrap_or(0);
    let splice_finding_kinds =
        extract_json_string_array_field(genome, "splice_finding_kinds").unwrap_or_default();
    let splice_mutation_intents =
        extract_json_string_array_field(genome, "splice_mutation_intents").unwrap_or_default();
    let splice_proposals = extract_json_usize_field(genome, "splice_proposals").unwrap_or(0);
    let splice_proposal_ids =
        extract_json_string_array_field(genome, "splice_proposal_ids").unwrap_or_default();
    let splice_read_only = extract_json_bool_field(genome, "splice_read_only");
    let splice_write_allowed = extract_json_bool_field(genome, "splice_write_allowed");
    let splice_applied = extract_json_bool_field(genome, "splice_applied");

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
    if repair_payloads > gene_scissors_proposals {
        failures.push(format!(
            "reasoning_genome repair_payloads {repair_payloads} exceed gene_scissors_proposals {gene_scissors_proposals}"
        ));
    }
    if relabel_candidates > 0 && repair_payloads < relabel_candidates {
        failures.push(format!(
            "reasoning_genome relabel_candidates {relabel_candidates} require repair_payloads >= relabel_candidates"
        ));
    }
    if malignant_genes > 0 && regeneration_payloads < malignant_genes {
        failures.push(format!(
            "reasoning_genome malignant_genes {malignant_genes} require regeneration_payloads >= malignant_genes"
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
    if splice_segments != splice_exons + splice_introns + splice_variants {
        failures.push(format!(
            "reasoning_genome splice_segments {splice_segments} does not match exons+introns+variants {}",
            splice_exons + splice_introns + splice_variants
        ));
    }
    if splice_segments
        != splice_retained + splice_skipped + splice_quarantined + splice_repair_candidates
    {
        failures.push(format!(
            "reasoning_genome splice_segments {splice_segments} does not match disposition counts {}",
            splice_retained + splice_skipped + splice_quarantined + splice_repair_candidates
        ));
    }
    if splice_retained != splice_exons {
        failures.push(format!(
            "reasoning_genome splice_retained {splice_retained} must match splice_exons {splice_exons}"
        ));
    }
    if splice_skipped != splice_introns {
        failures.push(format!(
            "reasoning_genome splice_skipped {splice_skipped} must match splice_introns {splice_introns}"
        ));
    }
    if splice_variants != splice_quarantined + splice_repair_candidates {
        failures.push(format!(
            "reasoning_genome splice_variants {splice_variants} must match quarantined+repair_candidates {}",
            splice_quarantined + splice_repair_candidates
        ));
    }
    if splice_segments > 0 && splice_dispositions.is_empty() {
        failures.push("reasoning_genome splice_segments require splice_dispositions".to_owned());
    }
    if splice_segments > 0 && splice_reason_summaries.is_empty() {
        failures
            .push("reasoning_genome splice_segments require splice_reason_summaries".to_owned());
    }
    if splice_quarantined > 0 {
        require_splice_disposition(&mut failures, &splice_dispositions, "quarantined");
    }
    if splice_repair_candidates > 0 {
        require_splice_disposition(&mut failures, &splice_dispositions, "repair_candidate");
    }
    for summary in &splice_reason_summaries {
        if contains_raw_payload_marker(summary) {
            failures
                .push("reasoning_genome splice_reason_summaries must stay sanitized".to_owned());
            break;
        }
    }
    if splice_variants > 0 && splice_findings == 0 {
        failures.push("reasoning_genome splice_variants require splice_findings".to_owned());
    }
    if splice_findings > 0 && splice_finding_kinds.is_empty() {
        failures.push("reasoning_genome splice_findings require splice_finding_kinds".to_owned());
    }
    if splice_proposal_ids.len() != splice_proposals {
        failures.push(format!(
            "reasoning_genome splice_proposal_ids {} do not match splice_proposals {splice_proposals}",
            splice_proposal_ids.len()
        ));
    }
    if splice_findings > 0 && splice_proposals == 0 {
        failures.push("reasoning_genome splice_findings require splice_proposals".to_owned());
    }
    if splice_proposals > 0 && splice_mutation_intents.is_empty() {
        failures
            .push("reasoning_genome splice_proposals require splice_mutation_intents".to_owned());
    }
    if splice_variants > 0 {
        if splice_finding_kinds
            .iter()
            .any(|kind| matches!(kind.as_str(), "drift" | "privacy"))
        {
            require_intent(&mut failures, &splice_mutation_intents, "quarantine");
            require_intent(&mut failures, &splice_mutation_intents, "regenerate");
        }
    }
    if splice_read_only != Some(true) {
        failures.push("reasoning_genome splice_read_only must be true".to_owned());
    }
    if splice_write_allowed != Some(false) {
        failures.push("reasoning_genome splice_write_allowed must be false".to_owned());
    }
    if splice_applied != Some(false) {
        failures.push("reasoning_genome splice_applied must be false".to_owned());
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

fn require_splice_disposition(failures: &mut Vec<String>, dispositions: &[String], expected: &str) {
    if !dispositions
        .iter()
        .any(|disposition| disposition == expected)
    {
        failures.push(format!(
            "reasoning_genome splice_dispositions must include {expected}"
        ));
    }
}

fn contains_raw_payload_marker(summary: &str) -> bool {
    let lower = summary.to_ascii_lowercase();
    lower.contains("prompt:")
        || lower.contains("answer:")
        || lower.contains("label=")
        || lower.contains("purpose=")
        || lower.contains("gist=")
}
