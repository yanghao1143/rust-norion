use super::fields::*;
use crate::privacy_redaction::contains_private_or_executable_marker;
use crate::reasoning_genome::{
    DNA_EVOLUTION_APPLY_PLAN_SCHEMA_VERSION, DNA_EVOLUTION_APPLY_PLAN_TRACE_SCHEMA,
    DNA_EVOLUTION_CANDIDATE_LEDGER_SCHEMA_VERSION, DNA_EVOLUTION_CONTROLLER_SCHEMA_VERSION,
};
use crate::writer_gate::UNIFIED_WRITER_GATE_SCHEMA_VERSION;

pub(super) fn evaluate_dna_evolution_controller_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        ("schema", "\"schema\":"),
        ("generation_id", "\"generation_id\":"),
        ("parent_anchors", "\"parent_anchors\":"),
        ("stable_anchor", "\"stable_anchor\":"),
        ("profile", "\"profile\":"),
        ("candidate_count", "\"candidate_count\":"),
        ("candidate_preview", "\"candidate_preview\":"),
        ("hold", "\"hold\":"),
        ("reject", "\"reject\":"),
        ("rollback", "\"rollback\":"),
        ("activation_eligible", "\"activation_eligible\":"),
        ("fitness_delta_summary", "\"fitness_delta_summary\":"),
        ("validation_status", "\"validation_status\":"),
        ("approval_status", "\"approval_status\":"),
        ("transaction_replay", "\"transaction_replay\":"),
        ("candidate_ledger", "\"candidate_ledger\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("raw_payload_included", "\"raw_payload_included\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing dna_evolution_controller field {name}"));
        }
    }

    match extract_json_string_field(line, "schema") {
        Some(value) if value == DNA_EVOLUTION_CONTROLLER_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "dna_evolution_controller schema {value} is not supported"
        )),
        None => failures.push("dna_evolution_controller schema missing".to_owned()),
    }

    let generation_id = extract_json_string_field(line, "generation_id").unwrap_or_default();
    if generation_id.trim().is_empty() {
        failures.push("dna_evolution_controller generation_id is empty".to_owned());
    }
    if extract_json_string_array_field(line, "parent_anchors")
        .unwrap_or_default()
        .is_empty()
    {
        failures.push("dna_evolution_controller parent_anchors must be non-empty".to_owned());
    }
    if extract_json_string_field(line, "stable_anchor")
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        failures.push("dna_evolution_controller stable_anchor is empty".to_owned());
    }

    let candidates = extract_json_usize_field(line, "candidate_count").unwrap_or(0);
    let decisions = extract_json_usize_field(line, "candidate_preview")
        .unwrap_or(0)
        .saturating_add(extract_json_usize_field(line, "hold").unwrap_or(0))
        .saturating_add(extract_json_usize_field(line, "reject").unwrap_or(0))
        .saturating_add(extract_json_usize_field(line, "rollback").unwrap_or(0));
    if candidates == 0 {
        failures.push("dna_evolution_controller candidate_count must be nonzero".to_owned());
    }
    if decisions != candidates {
        failures.push(
            "dna_evolution_controller decision counts do not match candidate_count".to_owned(),
        );
    }
    let activation_eligible =
        extract_json_usize_field(line, "activation_eligible").unwrap_or(usize::MAX);
    if activation_eligible > candidates {
        failures.push(
            "dna_evolution_controller activation_eligible exceeds candidate_count".to_owned(),
        );
    }

    let validation = extract_json_string_field(line, "validation_status").unwrap_or_default();
    if !matches!(validation.as_str(), "missing" | "passed" | "failed") {
        failures.push(format!(
            "dna_evolution_controller validation_status {validation} is not supported"
        ));
    }
    let approval = extract_json_string_field(line, "approval_status").unwrap_or_default();
    if !matches!(approval.as_str(), "pending" | "approved" | "rejected") {
        failures.push(format!(
            "dna_evolution_controller approval_status {approval} is not supported"
        ));
    }

    let Some(replay) = json_object_after_field(line, "transaction_replay") else {
        failures.push("dna_evolution_controller transaction_replay object missing".to_owned());
        return failures;
    };
    let replay_count = extract_json_usize_field(replay, "count").unwrap_or(0);
    if candidates > 0 && replay_count == 0 {
        failures
            .push("dna_evolution_controller transaction_replay count must be nonzero".to_owned());
    }
    if extract_json_bool_field(replay, "passed").is_none() {
        failures.push("dna_evolution_controller transaction_replay passed missing".to_owned());
    }
    if extract_json_usize_field(replay, "blocked").is_none() {
        failures.push("dna_evolution_controller transaction_replay blocked missing".to_owned());
    }

    let Some(candidate_ledger) = json_object_after_field(line, "candidate_ledger") else {
        failures.push("dna_evolution_controller candidate_ledger object missing".to_owned());
        return failures;
    };
    if candidate_ledger.contains("\"records\":[")
        || candidate_ledger.contains("\"record_lines\":[")
        || candidate_ledger.contains("\"ledger_lines\":[")
    {
        failures.push(
            "dna_evolution_controller candidate_ledger must expose records as count/digest only"
                .to_owned(),
        );
    }
    match extract_json_string_field(candidate_ledger, "schema") {
        Some(value) if value == DNA_EVOLUTION_CANDIDATE_LEDGER_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "dna_evolution_controller candidate_ledger schema {value} is not supported"
        )),
        None => {
            failures.push("dna_evolution_controller candidate_ledger schema missing".to_owned())
        }
    }
    let ledger_records = extract_json_usize_field(candidate_ledger, "records").unwrap_or(0);
    if ledger_records != candidates {
        failures.push(format!(
            "dna_evolution_controller candidate_ledger records {ledger_records} do not match candidate_count {candidates}"
        ));
    }
    if extract_json_bool_field(candidate_ledger, "candidate_only") != Some(true) {
        failures.push(
            "dna_evolution_controller candidate_ledger candidate_only must be true".to_owned(),
        );
    }
    let ledger_digest = extract_json_string_field(candidate_ledger, "digest").unwrap_or_default();
    if !ledger_digest.starts_with("redaction-digest:")
        || contains_private_or_executable_marker(&ledger_digest)
    {
        failures.push(
            "dna_evolution_controller candidate_ledger digest must be redaction digest".to_owned(),
        );
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "dna_evolution_controller",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "dna_evolution_controller",
    );
    require_bool(
        &mut failures,
        line,
        "applied",
        false,
        "dna_evolution_controller",
    );
    require_bool(
        &mut failures,
        line,
        "raw_payload_included",
        false,
        "dna_evolution_controller",
    );

    let fitness_summary =
        extract_json_string_field(line, "fitness_delta_summary").unwrap_or_default();
    if fitness_summary.trim().is_empty() {
        failures.push("dna_evolution_controller fitness_delta_summary is empty".to_owned());
    }
    if contains_private_or_executable_marker(&fitness_summary)
        || contains_private_or_executable_marker(&generation_id)
    {
        failures.push("dna_evolution_controller trace leaked private marker".to_owned());
    }

    failures
}

pub(super) fn evaluate_dna_evolution_apply_plan_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-dna-evolution-apply-plan-v1\"",
        ),
        ("plan_schema", "\"plan_schema\":"),
        ("controller_schema", "\"controller_schema\":"),
        ("writer_gate_schema", "\"writer_gate_schema\":"),
        ("decision", "\"decision\":"),
        ("writer_gate_decision", "\"writer_gate_decision\":"),
        ("generation_id", "\"generation_id\":"),
        ("candidates", "\"candidates\":"),
        ("ready_candidates", "\"ready_candidates\":"),
        ("held_candidates", "\"held_candidates\":"),
        ("rejected_candidates", "\"rejected_candidates\":"),
        ("reason_code_count", "\"reason_code_count\":"),
        ("explicit_apply_required", "\"explicit_apply_required\":"),
        ("candidate_digest", "\"candidate_digest\":"),
        ("apply_plan_digest", "\"apply_plan_digest\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("summary", "\"summary\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing dna_evolution_apply_plan field {name}"));
        }
    }

    if line.contains("\"records\":[") || line.contains("\"record_lines\":[") {
        failures.push(
            "dna_evolution_apply_plan must expose candidate records as count/digest only"
                .to_owned(),
        );
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "dna_evolution_apply_plan",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "dna_evolution_apply_plan",
    );
    require_bool(
        &mut failures,
        line,
        "applied",
        false,
        "dna_evolution_apply_plan",
    );

    let candidates = extract_json_usize_field(line, "candidates").unwrap_or(0);
    let ready = extract_json_usize_field(line, "ready_candidates").unwrap_or(0);
    let held = extract_json_usize_field(line, "held_candidates").unwrap_or(0);
    let rejected = extract_json_usize_field(line, "rejected_candidates").unwrap_or(0);
    let explicit_apply_required =
        extract_json_bool_field(line, "explicit_apply_required").unwrap_or(false);
    if candidates == 0 {
        failures.push("dna_evolution_apply_plan candidates must be nonzero".to_owned());
    }
    if ready.saturating_add(held).saturating_add(rejected) != candidates {
        failures.push(
            "dna_evolution_apply_plan decision candidate counts do not match candidates".to_owned(),
        );
    }
    if ready > 0 && !explicit_apply_required {
        failures.push(
            "dna_evolution_apply_plan ready candidates must require explicit apply".to_owned(),
        );
    }

    let decision = extract_json_string_field(line, "decision").unwrap_or_default();
    if ready > 0 && held == 0 && rejected == 0 {
        if decision != "ready_for_explicit_apply" {
            failures.push(format!(
                "dna_evolution_apply_plan decision {decision} does not match ready counters"
            ));
        }
    } else if rejected == candidates {
        if decision != "rejected" {
            failures.push(format!(
                "dna_evolution_apply_plan decision {decision} does not match rejected counters"
            ));
        }
    } else if !matches!(
        decision.as_str(),
        "held_for_writer_gate" | "held_for_candidate_state"
    ) {
        failures.push(format!(
            "dna_evolution_apply_plan decision {decision} is not a supported hold decision"
        ));
    }

    match extract_json_string_field(line, "schema") {
        Some(value) if value == DNA_EVOLUTION_APPLY_PLAN_TRACE_SCHEMA => {}
        Some(value) => failures.push(format!(
            "dna_evolution_apply_plan schema {value} is not supported"
        )),
        None => failures.push("dna_evolution_apply_plan schema missing".to_owned()),
    }
    match extract_json_string_field(line, "plan_schema") {
        Some(value) if value == DNA_EVOLUTION_APPLY_PLAN_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "dna_evolution_apply_plan plan_schema {value} is not supported"
        )),
        None => failures.push("dna_evolution_apply_plan plan_schema missing".to_owned()),
    }
    match extract_json_string_field(line, "controller_schema") {
        Some(value) if value == DNA_EVOLUTION_CONTROLLER_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "dna_evolution_apply_plan controller_schema {value} is not supported"
        )),
        None => failures.push("dna_evolution_apply_plan controller_schema missing".to_owned()),
    }
    match extract_json_string_field(line, "writer_gate_schema") {
        Some(value) if value == UNIFIED_WRITER_GATE_SCHEMA_VERSION => {}
        Some(value) => failures.push(format!(
            "dna_evolution_apply_plan writer_gate_schema {value} is not supported"
        )),
        None => failures.push("dna_evolution_apply_plan writer_gate_schema missing".to_owned()),
    }

    for field in ["candidate_digest", "apply_plan_digest"] {
        let value = extract_json_string_field(line, field).unwrap_or_default();
        if !value.starts_with("redaction-digest:") {
            failures.push(format!(
                "dna_evolution_apply_plan {field} must be redaction digest"
            ));
        }
        if contains_private_or_executable_marker(&value) {
            failures.push(format!(
                "dna_evolution_apply_plan {field} leaked private marker"
            ));
        }
    }
    let summary = extract_json_string_field(line, "summary").unwrap_or_default();
    if contains_private_or_executable_marker(&summary) {
        failures.push("dna_evolution_apply_plan summary leaked private marker".to_owned());
    }

    failures
}

pub(super) fn evaluate_trace_reasoning_genome(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(genome) = json_object_after_field(line, "reasoning_genome") else {
        failures.push("reasoning_genome object is missing or invalid".to_owned());
        return failures;
    };

    let genome_id = extract_json_string_field(genome, "genome_id").unwrap_or_default();
    let stable_anchor_id =
        extract_json_string_field(genome, "stable_anchor_id").unwrap_or_default();
    let generation_before_field = extract_json_usize_field(genome, "generation_before");
    let generation_after_field = extract_json_usize_field(genome, "generation_after");
    let generation_before = generation_before_field.unwrap_or(0);
    let generation_after = generation_after_field.unwrap_or(0);
    let active_genome_id_after =
        extract_json_string_field(genome, "active_genome_id_after").unwrap_or_default();
    let reasoning_frame_id =
        extract_json_string_field(genome, "reasoning_frame_id").unwrap_or_default();
    let reasoning_frame_valid = extract_json_bool_field(genome, "reasoning_frame_valid");
    let task_gene_decision =
        extract_json_string_field(genome, "task_gene_decision").unwrap_or_default();
    let task_skill_decision =
        extract_json_string_field(genome, "task_skill_decision").unwrap_or_default();
    let writer_gate_decision =
        extract_json_string_field(genome, "writer_gate_decision").unwrap_or_default();
    let apply_plan_decision =
        extract_json_string_field(genome, "apply_plan_decision").unwrap_or_default();
    let mutation_count = extract_json_usize_field(genome, "mutation_count").unwrap_or(0);
    let rollback_applied = extract_json_bool_field(genome, "rollback_applied");
    let chain_records = extract_json_usize_field(genome, "chain_records").unwrap_or(0);
    let lineage_scope_digests =
        extract_json_string_array_field(genome, "lineage_scope_digests").unwrap_or_default();
    let mixed_lineage = extract_json_bool_field(genome, "mixed_lineage");
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
    let lifecycle_records = extract_json_usize_field(genome, "lifecycle_records").unwrap_or(0);
    let lifecycle_actions =
        extract_json_string_array_field(genome, "lifecycle_actions").unwrap_or_default();
    let lifecycle_summaries =
        extract_json_string_array_field(genome, "lifecycle_summaries").unwrap_or_default();
    let lifecycle_tombstone_candidates =
        extract_json_usize_field(genome, "lifecycle_tombstone_candidates").unwrap_or(0);
    let lifecycle_pending_validations =
        extract_json_usize_field(genome, "lifecycle_pending_validations").unwrap_or(0);
    let lifecycle_source_evidence =
        extract_json_usize_field(genome, "lifecycle_source_evidence").unwrap_or(0);
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
    let splice_lifecycle_records =
        extract_json_usize_field(genome, "splice_lifecycle_records").unwrap_or(0);
    let splice_lifecycle_states =
        extract_json_string_array_field(genome, "splice_lifecycle_states").unwrap_or_default();
    let splice_lifecycle_summaries =
        extract_json_string_array_field(genome, "splice_lifecycle_summaries").unwrap_or_default();
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
    if generation_before_field.is_none() || generation_after_field.is_none() {
        failures.push("reasoning_genome generation fields are missing".to_owned());
    }
    if active_genome_id_after.trim().is_empty() || !active_genome_id_after.starts_with("genome:") {
        failures.push("reasoning_genome active_genome_id_after must be a genome: id".to_owned());
    }
    if !reasoning_frame_id.starts_with("redaction-digest:") || reasoning_frame_valid != Some(true) {
        failures.push("reasoning_genome ReasoningFrame must be valid and digest-only".to_owned());
    }
    if task_gene_decision.trim().is_empty() || task_skill_decision.trim().is_empty() {
        failures.push("reasoning_genome task gene decisions are missing".to_owned());
    }
    if gene_count == 0 {
        failures.push("reasoning_genome gene_count must be > 0".to_owned());
    }
    if chain_records < gene_count {
        failures.push(format!(
            "reasoning_genome chain_records {chain_records} below gene_count {gene_count}"
        ));
    }
    if chain_records > 0 && lineage_scope_digests.is_empty() {
        failures.push("reasoning_genome chain_records require lineage_scope_digests".to_owned());
    }
    for digest in &lineage_scope_digests {
        if !digest.starts_with("redaction-digest:") || contains_private_or_executable_marker(digest)
        {
            failures.push("reasoning_genome lineage_scope_digests must be redacted".to_owned());
            break;
        }
    }
    if chain_records > 0 && mixed_lineage != Some(false) {
        failures.push("reasoning_genome mixed_lineage must be false".to_owned());
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
    match mutation_applied {
        Some(true) => {
            if writer_gate_decision != "ready_for_explicit_apply"
                || apply_plan_decision != "ready_for_explicit_apply"
            {
                failures.push(
                    "reasoning_genome applied mutation requires ready writer/apply gates"
                        .to_owned(),
                );
            }
            if generation_after <= generation_before {
                failures
                    .push("reasoning_genome applied mutation must advance generation".to_owned());
            }
            if mutation_count == 0 {
                failures.push("reasoning_genome applied mutation count must be > 0".to_owned());
            }
        }
        Some(false) => {
            if generation_after != generation_before {
                failures.push(
                    "reasoning_genome unapplied mutation must not advance generation".to_owned(),
                );
            }
            if rollback_applied == Some(true) {
                failures.push(
                    "reasoning_genome rollback_applied requires mutation_applied=true".to_owned(),
                );
            }
        }
        None => failures.push("reasoning_genome mutation_applied is missing".to_owned()),
    }
    if !(0.0..=1.0).contains(&youth_pressure) {
        failures.push(format!(
            "reasoning_genome youth_pressure {youth_pressure:.6} must stay within 0.0..=1.0"
        ));
    }
    if lifecycle_records < gene_count {
        failures.push(format!(
            "reasoning_genome lifecycle_records {lifecycle_records} below gene_count {gene_count}"
        ));
    }
    if lifecycle_records > 0 && lifecycle_actions.is_empty() {
        failures.push("reasoning_genome lifecycle_records require lifecycle_actions".to_owned());
    }
    if lifecycle_records > 0 && lifecycle_summaries.is_empty() {
        failures.push("reasoning_genome lifecycle_records require lifecycle_summaries".to_owned());
    }
    if lifecycle_source_evidence < lifecycle_records {
        failures.push(format!(
            "reasoning_genome lifecycle_source_evidence {lifecycle_source_evidence} below lifecycle_records {lifecycle_records}"
        ));
    }
    if lifecycle_pending_validations > gene_scissors_proposals + lifecycle_records {
        failures.push(format!(
            "reasoning_genome lifecycle_pending_validations {lifecycle_pending_validations} exceed proposal/lifecycle evidence"
        ));
    }
    if lifecycle_tombstone_candidates < malignant_genes && malignant_genes > 0 {
        failures.push(format!(
            "reasoning_genome malignant_genes {malignant_genes} require lifecycle_tombstone_candidates >= malignant_genes"
        ));
    }
    if active_genes > 0 {
        require_lifecycle_action(&mut failures, &lifecycle_actions, "keep");
    }
    if aged_genes > 0 {
        require_lifecycle_action(&mut failures, &lifecycle_actions, "relabel");
    }
    if malignant_genes > 0 {
        require_lifecycle_action(&mut failures, &lifecycle_actions, "regenerate");
        require_lifecycle_action(&mut failures, &lifecycle_actions, "cut");
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
    if splice_findings > 0 && splice_lifecycle_records == 0 {
        failures.push("reasoning_genome splice_findings require lifecycle records".to_owned());
    }
    if splice_lifecycle_records > splice_findings {
        failures.push(format!(
            "reasoning_genome splice_lifecycle_records {splice_lifecycle_records} exceed splice_findings {splice_findings}"
        ));
    }
    if splice_lifecycle_records > 0 && splice_lifecycle_states.is_empty() {
        failures.push(
            "reasoning_genome splice_lifecycle_records require splice_lifecycle_states".to_owned(),
        );
    }
    if splice_lifecycle_records > 0 && splice_lifecycle_summaries.is_empty() {
        failures.push(
            "reasoning_genome splice_lifecycle_records require splice_lifecycle_summaries"
                .to_owned(),
        );
    }
    if splice_quarantined > 0 {
        require_splice_lifecycle_state(&mut failures, &splice_lifecycle_states, "quarantined");
    }
    if splice_repair_candidates > 0 {
        require_splice_lifecycle_state(&mut failures, &splice_lifecycle_states, "repair_candidate");
    }
    for summary in &splice_lifecycle_summaries {
        if contains_raw_payload_marker(summary) {
            failures
                .push("reasoning_genome splice_lifecycle_summaries must stay sanitized".to_owned());
            break;
        }
        if summary.contains("write_allowed=true") || summary.contains("applied=true") {
            failures
                .push("reasoning_genome splice_lifecycle_summaries must stay read-only".to_owned());
            break;
        }
        for marker in [
            "profile=",
            "shadow_state=",
            "drift_state=",
            "source_ids=",
            "expires_after_steps=",
            "score_milli=",
            "drift_gate_domains=",
            "rollback=",
        ] {
            if !summary.contains(marker) {
                failures.push(format!(
                    "reasoning_genome splice_lifecycle_summaries missing {marker} shadow evidence"
                ));
                break;
            }
        }
        if summary.contains("drift_gate_domains=") {
            for domain in [
                "golden_fixture:",
                "routing_behavior:",
                "memory_hygiene:",
                "privacy:",
                "trace_schema:",
            ] {
                if !summary.contains(domain) {
                    failures.push(format!(
                        "reasoning_genome splice_lifecycle_summaries missing {domain} drift gate domain"
                    ));
                    break;
                }
            }
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

fn require_lifecycle_action(failures: &mut Vec<String>, actions: &[String], expected: &str) {
    if !actions.iter().any(|action| action == expected) {
        failures.push(format!(
            "reasoning_genome lifecycle_actions must include {expected}"
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

fn require_splice_lifecycle_state(failures: &mut Vec<String>, states: &[String], expected: &str) {
    if !states.iter().any(|state| state == expected) {
        failures.push(format!(
            "reasoning_genome splice_lifecycle_states must include {expected}"
        ));
    }
}

fn require_bool(failures: &mut Vec<String>, line: &str, field: &str, expected: bool, label: &str) {
    match extract_json_bool_field(line, field) {
        Some(actual) if actual == expected => {}
        Some(actual) => failures.push(format!("{label} {field}={actual} expected {expected}")),
        None => failures.push(format!("{label} {field} missing")),
    }
}

fn contains_raw_payload_marker(summary: &str) -> bool {
    contains_private_or_executable_marker(summary)
        || summary.contains("label=")
        || summary.contains("purpose=")
        || summary.contains("gist=")
}
