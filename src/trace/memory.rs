use super::TRACE_FLOAT_EPSILON;
use super::evolution::require_usize_at_least;
use super::fields::*;
use crate::privacy_redaction::contains_private_or_executable_marker;
pub(super) fn evaluate_trace_memory_feedback(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(memory) = json_object_after_field(line, "memory") else {
        failures.push("memory object is missing or invalid".to_owned());
        return failures;
    };

    let reinforced = extract_json_usize_field(memory, "feedback_reinforced").unwrap_or(0);
    let penalized = extract_json_usize_field(memory, "feedback_penalized").unwrap_or(0);
    let updates = extract_json_usize_field(memory, "feedback_updates").unwrap_or(0);
    let applied = extract_json_usize_field(memory, "feedback_applied").unwrap_or(0);
    let removed = extract_json_usize_field(memory, "feedback_removed").unwrap_or(0);
    let missing = extract_json_usize_field(memory, "feedback_missing").unwrap_or(0);
    let strength_delta =
        extract_json_f32_field(memory, "feedback_strength_delta").unwrap_or(f32::NAN);
    let summaries =
        extract_json_string_array_field(memory, "feedback_update_summaries").unwrap_or_default();

    let expected_updates = reinforced.saturating_add(penalized);
    if updates != expected_updates {
        failures.push(format!(
            "memory feedback_updates {updates} does not match reinforced+penalized {expected_updates}"
        ));
    }
    if summaries.len() != updates {
        failures.push(format!(
            "memory feedback_update_summaries {} does not match feedback_updates {updates}",
            summaries.len()
        ));
    }
    if applied.saturating_add(missing) != updates {
        failures.push(format!(
            "memory feedback applied+missing {} does not match feedback_updates {updates}",
            applied.saturating_add(missing)
        ));
    }
    if removed > applied {
        failures.push(format!(
            "memory feedback_removed {removed} exceeds feedback_applied {applied}"
        ));
    }
    if !strength_delta.is_finite() || strength_delta < 0.0 {
        failures.push(format!(
            "memory feedback_strength_delta {strength_delta:.6} must be finite and >= 0"
        ));
    }

    let summary_applied = summaries
        .iter()
        .filter(|summary| summary.contains("applied=true"))
        .count();
    let summary_missing = summaries
        .iter()
        .filter(|summary| summary.contains("applied=false"))
        .count();
    let summary_removed = summaries
        .iter()
        .filter(|summary| summary.contains("removed=true"))
        .count();
    let summary_reinforced = summaries
        .iter()
        .filter(|summary| summary.starts_with("reinforce#"))
        .count();
    let summary_penalized = summaries
        .iter()
        .filter(|summary| summary.starts_with("penalize#"))
        .count();
    let summary_delta = summaries
        .iter()
        .filter_map(|summary| trace_note_f32(summary, "delta="))
        .map(f32::abs)
        .sum::<f32>();

    if summary_reinforced != reinforced {
        failures.push(format!(
            "memory feedback reinforce summaries {summary_reinforced} do not match feedback_reinforced {reinforced}"
        ));
    }
    if summary_penalized != penalized {
        failures.push(format!(
            "memory feedback penalize summaries {summary_penalized} do not match feedback_penalized {penalized}"
        ));
    }
    if summary_applied != applied {
        failures.push(format!(
            "memory feedback applied summaries {summary_applied} do not match feedback_applied {applied}"
        ));
    }
    if summary_missing != missing {
        failures.push(format!(
            "memory feedback missing summaries {summary_missing} do not match feedback_missing {missing}"
        ));
    }
    if summary_removed != removed {
        failures.push(format!(
            "memory feedback removed summaries {summary_removed} do not match feedback_removed {removed}"
        ));
    }
    if (summary_delta - strength_delta).abs() > 0.000_010 {
        failures.push(format!(
            "memory feedback strength delta summaries {summary_delta:.6} do not match feedback_strength_delta {strength_delta:.6}"
        ));
    }

    failures
}

pub(super) fn evaluate_trace_memory_admission(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(admission) = json_object_after_field(line, "memory_admission") else {
        failures.push("memory_admission object is missing or invalid".to_owned());
        return failures;
    };

    let candidates = extract_json_usize_field(admission, "candidates").unwrap_or(0);
    let ready = extract_json_usize_field(admission, "ready").unwrap_or(0);
    let blocked = extract_json_usize_field(admission, "blocked").unwrap_or(0);
    let admitted = extract_json_usize_field(admission, "admitted").unwrap_or(0);
    let hold = extract_json_usize_field(admission, "hold").unwrap_or(0);
    let reject = extract_json_usize_field(admission, "reject").unwrap_or(0);
    let quarantine = extract_json_usize_field(admission, "quarantine").unwrap_or(0);
    let kinds = extract_json_string_array_field(admission, "kinds").unwrap_or_default();
    let source_semantic = extract_json_usize_field(admission, "source_semantic").unwrap_or(0);
    let source_gist = extract_json_usize_field(admission, "source_gist").unwrap_or(0);
    let source_runtime_kv = extract_json_usize_field(admission, "source_runtime_kv").unwrap_or(0);
    let source_cold = extract_json_usize_field(admission, "source_cold").unwrap_or(0);
    let source_gene_segment =
        extract_json_usize_field(admission, "source_gene_segment").unwrap_or(0);
    let decisions = extract_json_string_array_field(admission, "decisions").unwrap_or_default();
    let summaries =
        extract_json_string_array_field(admission, "candidate_summaries").unwrap_or_default();
    let review_packets = extract_json_usize_field(admission, "review_packets").unwrap_or(0);
    let review_summaries =
        extract_json_string_array_field(admission, "review_packet_summaries").unwrap_or_default();
    let trace_segment_priors =
        extract_json_usize_field(admission, "trace_segment_priors").unwrap_or(0);
    let trace_segment_prior_summaries =
        extract_json_string_array_field(admission, "trace_segment_prior_summaries")
            .unwrap_or_default();
    let ledger_records = extract_json_usize_field(admission, "ledger_records").unwrap_or(0);
    let ledger_authorized = extract_json_usize_field(admission, "ledger_authorized").unwrap_or(0);
    let ledger_applied = extract_json_usize_field(admission, "ledger_applied").unwrap_or(0);
    let ledger_preview_only =
        extract_json_usize_field(admission, "ledger_preview_only").unwrap_or(0);
    let ledger_held = extract_json_usize_field(admission, "ledger_held").unwrap_or(0);
    let ledger_rejected = extract_json_usize_field(admission, "ledger_rejected").unwrap_or(0);
    let ledger_duplicate = extract_json_usize_field(admission, "ledger_duplicate").unwrap_or(0);
    let ledger_decayed = extract_json_usize_field(admission, "ledger_decayed").unwrap_or(0);
    let ledger_merged = extract_json_usize_field(admission, "ledger_merged").unwrap_or(0);
    let ledger_rollback = extract_json_usize_field(admission, "ledger_rollback").unwrap_or(0);
    let ledger_summaries =
        extract_json_string_array_field(admission, "ledger_summaries").unwrap_or_default();
    let read_only = extract_json_bool_field(admission, "read_only");
    let write_allowed = extract_json_bool_field(admission, "write_allowed");
    let applied = extract_json_bool_field(admission, "applied");

    let decision_total = ready
        .saturating_add(hold)
        .saturating_add(reject)
        .saturating_add(quarantine);
    if decision_total != candidates {
        failures.push(format!(
            "memory_admission decisions {decision_total} do not match candidates {candidates}"
        ));
    }
    let expected_blocked = hold.saturating_add(quarantine);
    if blocked != expected_blocked {
        failures.push(format!(
            "memory_admission blocked {blocked} does not match hold+quarantine {expected_blocked}"
        ));
    }
    if admitted > candidates {
        failures.push(format!(
            "memory_admission admitted {admitted} exceeds candidates {candidates}"
        ));
    }
    if summaries.len() != candidates {
        failures.push(format!(
            "memory_admission candidate_summaries {} do not match candidates {candidates}",
            summaries.len()
        ));
    }
    if review_packets != candidates {
        failures.push(format!(
            "memory_admission review_packets {review_packets} do not match candidates {candidates}"
        ));
    }
    if review_summaries.len() != review_packets {
        failures.push(format!(
            "memory_admission review_packet_summaries {} do not match review_packets {review_packets}",
            review_summaries.len()
        ));
    }
    if trace_segment_prior_summaries.len() != trace_segment_priors {
        failures.push(format!(
            "memory_admission trace_segment_prior_summaries {} do not match trace_segment_priors {trace_segment_priors}",
            trace_segment_prior_summaries.len()
        ));
    }
    if ledger_records != candidates {
        failures.push(format!(
            "memory_admission ledger_records {ledger_records} do not match candidates {candidates}"
        ));
    }
    let ledger_decision_total = ledger_preview_only
        .saturating_add(ledger_held)
        .saturating_add(ledger_rejected)
        .saturating_add(ledger_duplicate)
        .saturating_add(ledger_decayed)
        .saturating_add(ledger_merged)
        .saturating_add(ledger_rollback)
        .saturating_add(ledger_authorized);
    if ledger_decision_total != ledger_records {
        failures.push(format!(
            "memory_admission ledger decision counts {ledger_decision_total} do not match ledger_records {ledger_records}"
        ));
    }
    if ledger_summaries.len() != ledger_records {
        failures.push(format!(
            "memory_admission ledger_summaries {} do not match ledger_records {ledger_records}",
            ledger_summaries.len()
        ));
    }
    if ledger_applied > ledger_authorized {
        failures.push(format!(
            "memory_admission ledger_applied {ledger_applied} exceeds ledger_authorized {ledger_authorized}"
        ));
    }
    if candidates > 0 && kinds.is_empty() {
        failures.push("memory_admission candidates require non-empty kinds".to_owned());
    }
    if candidates > 0 && decisions.is_empty() {
        failures.push("memory_admission candidates require non-empty decisions".to_owned());
    }
    if kinds.len() > candidates {
        failures.push(format!(
            "memory_admission kinds {} exceeds candidates {candidates}",
            kinds.len()
        ));
    }
    if decisions.len() > candidates {
        failures.push(format!(
            "memory_admission decisions {} exceeds candidates {candidates}",
            decisions.len()
        ));
    }
    let source_total = source_semantic
        .saturating_add(source_gist)
        .saturating_add(source_runtime_kv)
        .saturating_add(source_cold)
        .saturating_add(source_gene_segment);
    if source_total != candidates {
        failures.push(format!(
            "memory_admission source counts {source_total} do not match candidates {candidates}"
        ));
    }
    if read_only != Some(true) {
        failures.push("memory_admission read_only must be true".to_owned());
    }
    if write_allowed != Some(false) {
        failures.push("memory_admission write_allowed must be false".to_owned());
    }
    if applied != Some(false) {
        failures.push("memory_admission applied must be false".to_owned());
    }
    if read_only == Some(true) && admitted > 0 {
        failures.push("memory_admission read-only preview requires admitted=0".to_owned());
    }
    if read_only == Some(true) && (ledger_authorized > 0 || ledger_applied > 0) {
        failures.push(
            "memory_admission read-only preview requires ledger_authorized=0 and ledger_applied=0"
                .to_owned(),
        );
    }
    if summaries
        .iter()
        .any(|summary| contains_private_or_executable_marker(summary))
    {
        failures.push(
            "memory_admission candidate_summaries must not leak raw prompt or answer payloads"
                .to_owned(),
        );
    }
    for (index, summary) in summaries.iter().enumerate() {
        for marker in [
            "profile=",
            "shadow_state=",
            "drift_state=",
            "source_ids=",
            "expires_after_steps=",
            "score_milli=",
            "drift_gate_domains=",
            "rollback=",
            "source_hash=",
            "privacy=",
            "validation=",
        ] {
            if !summary.contains(marker) {
                failures.push(format!(
                    "memory_admission candidate {index} missing {marker} shadow evidence"
                ));
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
                        "memory_admission candidate {index} missing {domain} drift gate domain"
                    ));
                }
            }
        }
    }
    if review_summaries
        .iter()
        .any(|summary| contains_private_or_executable_marker(summary))
    {
        failures.push(
            "memory_admission review_packet_summaries must not leak raw prompt or answer payloads"
                .to_owned(),
        );
    }
    if ledger_summaries
        .iter()
        .any(|summary| contains_private_or_executable_marker(summary))
    {
        failures.push(
            "memory_admission ledger_summaries must not leak raw prompt or answer payloads"
                .to_owned(),
        );
    }
    if trace_segment_prior_summaries
        .iter()
        .any(|summary| contains_private_or_executable_marker(summary))
    {
        failures.push(
            "memory_admission trace_segment_prior_summaries must not leak raw prompt or answer payloads"
                .to_owned(),
        );
    }
    for (index, summary) in review_summaries.iter().enumerate() {
        for marker in ["approval=", "next=", "rollback="] {
            if !summary.contains(marker) {
                failures.push(format!(
                    "memory_admission review packet {index} missing {marker} evidence"
                ));
            }
        }
        if !summary.contains("write_allowed=false") || !summary.contains("applied=false") {
            failures.push(format!(
                "memory_admission review packet {index} must remain write-disabled and unapplied"
            ));
        }
    }
    for (index, summary) in ledger_summaries.iter().enumerate() {
        for marker in ["rollback=", "source_hash=", "privacy=", "validation="] {
            if !summary.contains(marker) {
                failures.push(format!(
                    "memory_admission ledger record {index} missing {marker} evidence"
                ));
            }
        }
        if summary.contains("authorized=true") || summary.contains("applied=true") {
            failures.push(format!(
                "memory_admission ledger record {index} must remain write-disabled in trace preview"
            ));
        }
    }
    let gene_segment_ledger_metadata = ledger_summaries
        .iter()
        .filter(|summary| gene_segment_ledger_metadata_present(summary))
        .count();
    if source_gene_segment > 0 && gene_segment_ledger_metadata != source_gene_segment {
        failures.push(format!(
            "memory_admission GeneSegment ledger metadata {gene_segment_ledger_metadata} does not match source_gene_segment {source_gene_segment}"
        ));
    }
    for (index, summary) in trace_segment_prior_summaries.iter().enumerate() {
        for marker in [
            "schema=",
            "segment=",
            "candidate=",
            "input_hash=",
            "prompt_digest=",
            "router_decision_digest=",
            "scheduler_phase_ids=",
            "tool_call_ids=",
            "process_reward_milli=",
            "verifier=",
            "final_draft_digest=",
            "source_trace_ids=",
            "rollback=",
            "similarity_milli=",
            "proposed_for_retrieval=",
        ] {
            if !summary.contains(marker) {
                failures.push(format!(
                    "memory_admission trace segment prior {index} missing {marker} evidence"
                ));
            }
        }
        if !summary.contains("read_only=true")
            || !summary.contains("write_allowed=false")
            || !summary.contains("applied=false")
        {
            failures.push(format!(
                "memory_admission trace segment prior {index} must remain preview-only"
            ));
        }
    }

    failures
}

fn gene_segment_ledger_metadata_present(summary: &str) -> bool {
    summary.contains("gene_segment_kv=true")
        && summary.contains("profile=")
        && summary.contains("source=")
        && !summary.contains("source=none")
        && summary.contains("source_hash=")
        && summary.contains("tenant_scope_digest=redaction-digest:")
        && summary.contains("session_scope_digest=redaction-digest:")
}

pub(super) fn evaluate_trace_kv_fusion(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let Some(fusion) = json_object_after_field(line, "kv_fusion") else {
        failures.push("kv_fusion object is missing or invalid".to_owned());
        return failures;
    };

    let candidates = extract_json_usize_field(fusion, "candidates").unwrap_or(0);
    let fused = extract_json_usize_field(fusion, "fused").unwrap_or(0);
    let compressed = extract_json_usize_field(fusion, "compressed").unwrap_or(0);
    let skipped = extract_json_usize_field(fusion, "skipped").unwrap_or(0);
    let held = extract_json_usize_field(fusion, "held").unwrap_or(0);
    let rejected = extract_json_usize_field(fusion, "rejected").unwrap_or(0);
    let approval_blocked = extract_json_usize_field(fusion, "approval_blocked").unwrap_or(0);
    let input_tokens = extract_json_usize_field(fusion, "input_tokens").unwrap_or(0);
    let retained_tokens = extract_json_usize_field(fusion, "retained_tokens").unwrap_or(0);
    let saved_tokens = extract_json_usize_field(fusion, "saved_tokens").unwrap_or(0);
    let min_score = extract_json_f32_field(fusion, "min_score").unwrap_or(f32::NAN);
    let max_score = extract_json_f32_field(fusion, "max_score").unwrap_or(f32::NAN);
    let average_score = extract_json_f32_field(fusion, "average_score").unwrap_or(f32::NAN);
    let score_summaries =
        extract_json_string_array_field(fusion, "score_summaries").unwrap_or_default();
    let read_only = extract_json_bool_field(fusion, "read_only");
    let write_allowed = extract_json_bool_field(fusion, "write_allowed");
    let applied = extract_json_bool_field(fusion, "applied");

    let decision_total = fused
        .saturating_add(compressed)
        .saturating_add(skipped)
        .saturating_add(held)
        .saturating_add(rejected)
        .saturating_add(approval_blocked);
    if decision_total != candidates {
        failures.push(format!(
            "kv_fusion decisions {decision_total} do not match candidates {candidates}"
        ));
    }
    if retained_tokens.saturating_add(saved_tokens) != input_tokens {
        failures.push(format!(
            "kv_fusion retained+saved {} does not match input_tokens {input_tokens}",
            retained_tokens.saturating_add(saved_tokens)
        ));
    }
    if retained_tokens > input_tokens {
        failures.push(format!(
            "kv_fusion retained_tokens {retained_tokens} exceeds input_tokens {input_tokens}"
        ));
    }
    if candidates > 0 && score_summaries.is_empty() {
        failures.push("kv_fusion candidates require score_summaries".to_owned());
    }
    if score_summaries.len() > candidates {
        failures.push(format!(
            "kv_fusion score_summaries {} exceeds candidates {candidates}",
            score_summaries.len()
        ));
    }
    for (name, score) in [
        ("min_score", min_score),
        ("max_score", max_score),
        ("average_score", average_score),
    ] {
        if !score.is_finite() || !(0.0..=1.0).contains(&score) {
            failures.push(format!(
                "kv_fusion {name} {score:.6} must stay within 0.0..=1.0"
            ));
        }
    }
    if candidates > 0 && min_score > average_score {
        failures.push("kv_fusion min_score exceeds average_score".to_owned());
    }
    if candidates > 0 && average_score > max_score {
        failures.push("kv_fusion average_score exceeds max_score".to_owned());
    }
    if read_only != Some(true) {
        failures.push("kv_fusion read_only must be true".to_owned());
    }
    if write_allowed != Some(false) {
        failures.push("kv_fusion write_allowed must be false".to_owned());
    }
    if applied != Some(false) {
        failures.push("kv_fusion applied must be false".to_owned());
    }
    for (index, summary) in score_summaries.iter().enumerate() {
        for marker in [
            "source=",
            "decision=",
            "score=",
            "components=",
            "privacy=",
            "rollback=",
        ] {
            if !summary.contains(marker) {
                failures.push(format!(
                    "kv_fusion score summary {index} missing {marker} evidence"
                ));
            }
        }
        if contains_private_or_executable_marker(summary) {
            failures.push(format!(
                "kv_fusion score summary {index} must not leak raw prompt or answer payloads"
            ));
        }
    }

    failures
}

pub(super) fn evaluate_trace_memory_residency_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-memory-residency-plan-v1\"",
        ),
        ("redacted", "\"redacted\":"),
        ("report_only", "\"report_only\":"),
        ("decision_count", "\"decision_count\":"),
        ("hot", "\"hot\":"),
        ("warm", "\"warm\":"),
        ("cold", "\"cold\":"),
        ("quarantined", "\"quarantined\":"),
        ("retired", "\"retired\":"),
        (
            "protected_rollback_anchors",
            "\"protected_rollback_anchors\":",
        ),
        ("blocked_reasons", "\"blocked_reasons\":"),
        ("token_estimate", "\"token_estimate\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("durable_write_allowed", "\"durable_write_allowed\":"),
        ("applied", "\"applied\":"),
        ("replay_digest", "\"replay_digest\":"),
        (
            "protected_rollback_anchor_digests",
            "\"protected_rollback_anchor_digests\":",
        ),
        ("decision_summaries", "\"decision_summaries\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing memory_residency field {name}"));
        }
    }

    let redacted = extract_json_bool_field(line, "redacted");
    let report_only = extract_json_bool_field(line, "report_only");
    let read_only = extract_json_bool_field(line, "read_only");
    let write_allowed = extract_json_bool_field(line, "write_allowed");
    let durable_write_allowed = extract_json_bool_field(line, "durable_write_allowed");
    let applied = extract_json_bool_field(line, "applied");
    let decision_count = extract_json_usize_field(line, "decision_count").unwrap_or(0);
    let hot = extract_json_usize_field(line, "hot").unwrap_or(0);
    let warm = extract_json_usize_field(line, "warm").unwrap_or(0);
    let cold = extract_json_usize_field(line, "cold").unwrap_or(0);
    let quarantined = extract_json_usize_field(line, "quarantined").unwrap_or(0);
    let retired = extract_json_usize_field(line, "retired").unwrap_or(0);
    let protected = extract_json_usize_field(line, "protected_rollback_anchors").unwrap_or(0);
    let blocked_reasons = extract_json_usize_field(line, "blocked_reasons").unwrap_or(0);
    let token_estimate = extract_json_usize_field(line, "token_estimate").unwrap_or(0);
    let replay_digest = extract_json_string_field(line, "replay_digest").unwrap_or_default();
    let protected_digests =
        extract_json_string_array_field(line, "protected_rollback_anchor_digests")
            .unwrap_or_default();
    let summaries = extract_json_string_array_field(line, "decision_summaries").unwrap_or_default();
    let state_total = hot
        .saturating_add(warm)
        .saturating_add(cold)
        .saturating_add(quarantined)
        .saturating_add(retired);

    if redacted != Some(true) {
        failures.push("memory_residency redacted must be true".to_owned());
    }
    if report_only != Some(true) {
        failures.push("memory_residency report_only must be true".to_owned());
    }
    if read_only != Some(true) {
        failures.push("memory_residency read_only must be true".to_owned());
    }
    if write_allowed != Some(false) {
        failures.push("memory_residency write_allowed must be false".to_owned());
    }
    if durable_write_allowed != Some(false) {
        failures.push("memory_residency durable_write_allowed must be false".to_owned());
    }
    if applied != Some(false) {
        failures.push("memory_residency applied must be false".to_owned());
    }
    if !replay_digest.starts_with("fnv64:") {
        failures
            .push("memory_residency replay_digest must be digest-only fnv64 evidence".to_owned());
    }
    if state_total != decision_count {
        failures.push(format!(
            "memory_residency state counts {state_total} do not match decision_count {decision_count}"
        ));
    }
    if summaries.len() != decision_count {
        failures.push(format!(
            "memory_residency decision_summaries {} do not match decision_count {decision_count}",
            summaries.len()
        ));
    }
    if protected > decision_count {
        failures.push(format!(
            "memory_residency protected_rollback_anchors {protected} exceeds decision_count {decision_count}"
        ));
    }
    if protected_digests.len() != protected {
        failures.push(format!(
            "memory_residency protected_rollback_anchor_digests {} do not match protected_rollback_anchors {protected}",
            protected_digests.len()
        ));
    }
    if decision_count > 0 && token_estimate < decision_count {
        failures.push(format!(
            "memory_residency token_estimate {token_estimate} is smaller than decision_count {decision_count}"
        ));
    }
    if quarantined > 0 && blocked_reasons == 0 {
        failures
            .push("memory_residency quarantined decisions require blocked_reasons > 0".to_owned());
    }
    if line_contains_raw_memory_payload(line) {
        failures
            .push("memory_residency trace must not leak raw private memory payloads".to_owned());
    }

    for (index, digest) in protected_digests.iter().enumerate() {
        if !digest.starts_with("fnv64:") {
            failures.push(format!(
                "memory_residency protected digest {index} must be fnv64 evidence"
            ));
        }
    }
    for (index, summary) in summaries.iter().enumerate() {
        for marker in [
            "id=",
            "state=",
            "score=",
            "tenant=fnv64:",
            "namespace=fnv64:",
            "rollback=fnv64:",
            "protected=",
            "blocked=",
            "tokens=",
        ] {
            if !summary.contains(marker) {
                failures.push(format!(
                    "memory_residency decision summary {index} missing {marker} evidence"
                ));
            }
        }
        if ![
            "state=hot",
            "state=warm",
            "state=cold",
            "state=quarantined",
            "state=retired",
        ]
        .iter()
        .any(|state| summary.contains(state))
        {
            failures.push(format!(
                "memory_residency decision summary {index} has invalid state evidence"
            ));
        }
        if line_contains_raw_memory_payload(summary) {
            failures.push(format!(
                "memory_residency decision summary {index} must not leak raw private memory payloads"
            ));
        }
    }

    failures
}

pub(super) fn evaluate_self_evolving_memory_store_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-evolving-memory-store-v1\"",
        ),
        ("operation", "\"operation\":"),
        ("redacted", "\"redacted\":"),
        ("report_only", "\"report_only\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("durable_write_allowed", "\"durable_write_allowed\":"),
        ("applied", "\"applied\":"),
        ("applied_to_disk", "\"applied_to_disk\":"),
        ("evidence_digest", "\"evidence_digest\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing self_evolving_memory_store field {name}"));
        }
    }

    let operation = extract_json_string_field(line, "operation").unwrap_or_default();
    let redacted = extract_json_bool_field(line, "redacted");
    let report_only = extract_json_bool_field(line, "report_only");
    let read_only = extract_json_bool_field(line, "read_only");
    let write_allowed = extract_json_bool_field(line, "write_allowed");
    let durable_write_allowed = extract_json_bool_field(line, "durable_write_allowed");
    let applied = extract_json_bool_field(line, "applied");
    let applied_to_disk = extract_json_bool_field(line, "applied_to_disk");
    let evidence_digest = extract_json_string_field(line, "evidence_digest").unwrap_or_default();

    if redacted != Some(true) {
        failures.push("self_evolving_memory_store redacted must be true".to_owned());
    }
    if report_only != Some(true) {
        failures.push("self_evolving_memory_store report_only must be true".to_owned());
    }
    if read_only.is_none() {
        failures.push("self_evolving_memory_store read_only must be boolean".to_owned());
    }
    if write_allowed != Some(false) {
        failures.push("self_evolving_memory_store write_allowed must be false".to_owned());
    }
    if durable_write_allowed != Some(false) {
        failures.push("self_evolving_memory_store durable_write_allowed must be false".to_owned());
    }
    if applied != Some(false) {
        failures.push("self_evolving_memory_store applied must be false".to_owned());
    }
    if applied_to_disk != Some(false) {
        failures.push("self_evolving_memory_store applied_to_disk must be false".to_owned());
    }
    if !evidence_digest.starts_with("fnv64:") {
        failures.push("self_evolving_memory_store evidence_digest must be stable fnv64".to_owned());
    }
    if line_contains_raw_memory_payload(line) {
        failures.push(
            "self_evolving_memory_store trace must not contain raw memory payloads".to_owned(),
        );
    }

    match operation.as_str() {
        "retrieval" => evaluate_self_evolving_memory_retrieval_trace(&mut failures, line),
        "maintenance" => evaluate_self_evolving_memory_maintenance_trace(&mut failures, line),
        "consolidation_preview" => {
            evaluate_self_evolving_memory_consolidation_trace(&mut failures, line)
        }
        "ab_evaluation" => evaluate_self_evolving_memory_ab_trace(&mut failures, line),
        "admission_preview" => {
            evaluate_self_evolving_memory_admission_preview_trace(&mut failures, line)
        }
        "" => failures.push("self_evolving_memory_store operation is empty".to_owned()),
        other => failures.push(format!(
            "self_evolving_memory_store operation {other} is not supported"
        )),
    }

    failures
}

fn evaluate_self_evolving_memory_ab_trace(failures: &mut Vec<String>, line: &str) {
    let results = extract_json_usize_field(line, "results").unwrap_or(0);
    let cases = extract_json_usize_field(line, "cases").unwrap_or(0);
    let modes = extract_json_usize_field(line, "modes").unwrap_or(0);
    let languages = extract_json_usize_field(line, "languages").unwrap_or(0);
    let regressions = extract_json_usize_field(line, "regressions").unwrap_or(0);
    let candidate_previews = extract_json_usize_field(line, "candidate_previews").unwrap_or(0);
    let admitted = extract_json_usize_field(line, "admitted_candidates").unwrap_or(0);
    let unsafe_write_rejections =
        extract_json_usize_field(line, "unsafe_write_rejections").unwrap_or(0);
    let compiler_passed = extract_json_usize_field(line, "compiler_passed").unwrap_or(0);
    let tests_passed = extract_json_usize_field(line, "tests_passed").unwrap_or(0);
    let benchmark_passed = extract_json_usize_field(line, "benchmark_passed").unwrap_or(0);

    if results == 0 || cases == 0 || modes == 0 || languages == 0 {
        failures.push("self_evolving_memory_store ab_evaluation coverage is empty".to_owned());
    }
    if results != cases.saturating_mul(modes) {
        failures.push(format!(
            "self_evolving_memory_store ab_evaluation results {results} does not match cases*modes {}",
            cases.saturating_mul(modes)
        ));
    }
    if regressions == 0 {
        failures
            .push("self_evolving_memory_store ab_evaluation must include regressions".to_owned());
    }
    if candidate_previews == 0 {
        failures.push(
            "self_evolving_memory_store ab_evaluation must include preview candidates".to_owned(),
        );
    }
    if admitted != 0 {
        failures.push("self_evolving_memory_store ab_evaluation admitted candidates".to_owned());
    }
    if unsafe_write_rejections == 0 {
        failures
            .push("self_evolving_memory_store ab_evaluation must reject unsafe writes".to_owned());
    }
    if compiler_passed == 0 || tests_passed == 0 || benchmark_passed == 0 {
        failures.push(
            "self_evolving_memory_store ab_evaluation missing validation evidence".to_owned(),
        );
    }
}

fn evaluate_self_evolving_memory_retrieval_trace(failures: &mut Vec<String>, line: &str) {
    let contexts = extract_json_usize_field(line, "contexts").unwrap_or(0);
    let episodes = extract_json_usize_field(line, "episodes").unwrap_or(0);
    let heuristics = extract_json_usize_field(line, "heuristics").unwrap_or(0);
    let tools = extract_json_usize_field(line, "tools").unwrap_or(0);
    let requested_limit = extract_json_usize_field(line, "requested_limit").unwrap_or(0);
    let token_budget = extract_json_usize_field(line, "token_budget").unwrap_or(0);
    let retained_tokens = extract_json_usize_field(line, "retained_tokens").unwrap_or(0);
    let skipped_by_budget = extract_json_usize_field(line, "skipped_by_budget").unwrap_or(0);

    if contexts != episodes.saturating_add(heuristics).saturating_add(tools) {
        failures.push(format!(
            "self_evolving_memory_store retrieval contexts {contexts} does not match episodes+heuristics+tools"
        ));
    }
    if requested_limit == 0 {
        failures
            .push("self_evolving_memory_store retrieval requested_limit must be > 0".to_owned());
    }
    if contexts > requested_limit {
        failures.push(format!(
            "self_evolving_memory_store retrieval contexts {contexts} exceeds requested_limit {requested_limit}"
        ));
    }
    if token_budget == 0 {
        failures.push("self_evolving_memory_store retrieval token_budget must be > 0".to_owned());
    }
    if retained_tokens > token_budget {
        failures.push(format!(
            "self_evolving_memory_store retrieval retained_tokens {retained_tokens} exceeds token_budget {token_budget}"
        ));
    }
    if skipped_by_budget > 0 && retained_tokens >= token_budget {
        return;
    }
    if skipped_by_budget > 0 && contexts == 0 {
        failures.push(
            "self_evolving_memory_store retrieval skipped_by_budget requires retained context"
                .to_owned(),
        );
    }
}

fn evaluate_self_evolving_memory_maintenance_trace(failures: &mut Vec<String>, line: &str) {
    let actions = extract_json_usize_field(line, "maintenance_actions").unwrap_or(0);
    let expected_actions = extract_json_usize_field(line, "decayed_heuristics")
        .unwrap_or(0)
        .saturating_add(extract_json_usize_field(line, "decayed_tool_reliability").unwrap_or(0))
        .saturating_add(extract_json_usize_field(line, "quarantined_heuristics").unwrap_or(0))
        .saturating_add(extract_json_usize_field(line, "merged_duplicate_episodes").unwrap_or(0));

    if actions != expected_actions {
        failures.push(format!(
            "self_evolving_memory_store maintenance actions {actions} does not match component total {expected_actions}"
        ));
    }
}

fn evaluate_self_evolving_memory_consolidation_trace(failures: &mut Vec<String>, line: &str) {
    let actions = extract_json_usize_field(line, "consolidation_actions").unwrap_or(0);
    let expected_actions = extract_json_usize_field(line, "merge_previews")
        .unwrap_or(0)
        .saturating_add(extract_json_usize_field(line, "decay_previews").unwrap_or(0))
        .saturating_add(extract_json_usize_field(line, "tombstone_previews").unwrap_or(0))
        .saturating_add(extract_json_usize_field(line, "merge_rejections").unwrap_or(0));
    let records_before = extract_json_usize_field(line, "records_before").unwrap_or(0);
    let records_after = extract_json_usize_field(line, "records_after_preview").unwrap_or(0);
    let tokens_before = extract_json_usize_field(line, "token_estimate_before").unwrap_or(0);
    let tokens_after = extract_json_usize_field(line, "token_estimate_after_preview").unwrap_or(0);
    let replay_safety = extract_json_usize_field(line, "replay_safety_milli").unwrap_or(0);
    let snapshot_digest = extract_json_string_field(line, "snapshot_digest").unwrap_or_default();
    let plan_digest = extract_json_string_field(line, "plan_digest").unwrap_or_default();

    if actions != expected_actions {
        failures.push(format!(
            "self_evolving_memory_store consolidation actions {actions} does not match component total {expected_actions}"
        ));
    }
    if records_after > records_before {
        failures.push(format!(
            "self_evolving_memory_store consolidation records_after_preview {records_after} exceeds records_before {records_before}"
        ));
    }
    if tokens_after > tokens_before {
        failures.push(format!(
            "self_evolving_memory_store consolidation token_estimate_after_preview {tokens_after} exceeds token_estimate_before {tokens_before}"
        ));
    }
    if replay_safety < 1000 {
        failures.push(
            "self_evolving_memory_store consolidation replay_safety_milli must be 1000".to_owned(),
        );
    }
    if !snapshot_digest.starts_with("fnv64:") || !plan_digest.starts_with("fnv64:") {
        failures.push(
            "self_evolving_memory_store consolidation snapshot and plan digests must be stable fnv64"
                .to_owned(),
        );
    }
}

fn evaluate_self_evolving_memory_admission_preview_trace(failures: &mut Vec<String>, line: &str) {
    let candidates = extract_json_usize_field(line, "candidates").unwrap_or(0);
    let eligible = extract_json_usize_field(line, "eligible").unwrap_or(0);
    let blocked = extract_json_usize_field(line, "blocked").unwrap_or(0);
    let blocked_reasons = extract_json_usize_field(line, "blocked_reasons").unwrap_or(0);
    let read_only = extract_json_bool_field(line, "read_only");

    if eligible.saturating_add(blocked) != candidates {
        failures.push(format!(
            "self_evolving_memory_store admission_preview eligible+blocked {} does not match candidates {candidates}",
            eligible.saturating_add(blocked)
        ));
    }
    if eligible > candidates {
        failures.push(format!(
            "self_evolving_memory_store admission_preview eligible {eligible} exceeds candidates {candidates}"
        ));
    }
    if blocked > 0 && blocked_reasons == 0 {
        failures.push(
            "self_evolving_memory_store admission_preview blocked candidates require reasons"
                .to_owned(),
        );
    }
    if read_only != Some(true) {
        failures
            .push("self_evolving_memory_store admission_preview read_only must be true".to_owned());
    }
}

fn line_contains_raw_memory_payload(line: &str) -> bool {
    contains_private_or_executable_marker(line)
        || ["solution_path", "key_insights"]
            .iter()
            .any(|marker| line.contains(marker))
}

pub(super) fn evaluate_trace_memory_governance(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    if let Some(retention) = json_object_after_field(line, "retention") {
        let stale_after = extract_json_usize_field(retention, "stale_after").unwrap_or(0);
        let decay_rate = extract_json_f32_field(retention, "decay_rate").unwrap_or(f32::NAN);
        let remove_below_strength =
            extract_json_f32_field(retention, "remove_below_strength").unwrap_or(f32::NAN);
        let remove_after_failures =
            extract_json_usize_field(retention, "remove_after_failures").unwrap_or(0);
        let before = extract_json_usize_field(retention, "before").unwrap_or(0);
        let after = extract_json_usize_field(retention, "after").unwrap_or(0);
        let decayed = extract_json_usize_field(retention, "decayed").unwrap_or(0);
        let removed = extract_json_usize_field(retention, "removed").unwrap_or(0);

        if stale_after == 0 {
            failures.push("retention stale_after must be > 0".to_owned());
        }
        if !(0.0..=0.95).contains(&decay_rate) {
            failures.push(format!(
                "retention decay_rate {decay_rate:.6} must stay within 0.0..=0.95"
            ));
        }
        if !(0.0..=3.0).contains(&remove_below_strength) {
            failures.push(format!(
                "retention remove_below_strength {remove_below_strength:.6} must stay within 0.0..=3.0"
            ));
        }
        if remove_after_failures == 0 {
            failures.push("retention remove_after_failures must be > 0".to_owned());
        }
        if decayed > before {
            failures.push(format!(
                "retention decayed {decayed} exceeds before {before}"
            ));
        }
        if removed > before {
            failures.push(format!(
                "retention removed {removed} exceeds before {before}"
            ));
        }
        if after > before {
            failures.push(format!("retention after {after} exceeds before {before}"));
        }
        if after.saturating_add(removed) != before {
            failures.push(format!(
                "retention before {before} does not match after+removed {}",
                after.saturating_add(removed)
            ));
        }
    } else {
        failures.push("retention object is missing or invalid".to_owned());
    }

    if let Some(compaction) = json_object_after_field(line, "memory_compaction") {
        let similarity_threshold =
            extract_json_f32_field(compaction, "similarity_threshold").unwrap_or(f32::NAN);
        let max_candidates = extract_json_usize_field(compaction, "max_candidates").unwrap_or(0);
        let max_merges = extract_json_usize_field(compaction, "max_merges").unwrap_or(0);
        let before = extract_json_usize_field(compaction, "before").unwrap_or(0);
        let after = extract_json_usize_field(compaction, "after").unwrap_or(0);
        let merged = extract_json_usize_field(compaction, "merged").unwrap_or(0);
        let removed = extract_json_usize_field(compaction, "removed").unwrap_or(0);

        if !(0.10..=0.999).contains(&similarity_threshold) {
            failures.push(format!(
                "memory_compaction similarity_threshold {similarity_threshold:.6} must stay within 0.10..=0.999"
            ));
        }
        if merged != removed {
            failures.push(format!(
                "memory_compaction merged {merged} does not match removed {removed}"
            ));
        }
        if merged > max_merges {
            failures.push(format!(
                "memory_compaction merged {merged} exceeds max_merges {max_merges}"
            ));
        }
        if removed > before {
            failures.push(format!(
                "memory_compaction removed {removed} exceeds before {before}"
            ));
        }
        if after > before {
            failures.push(format!(
                "memory_compaction after {after} exceeds before {before}"
            ));
        }
        if after.saturating_add(removed) != before {
            failures.push(format!(
                "memory_compaction before {before} does not match after+removed {}",
                after.saturating_add(removed)
            ));
        }
        if (before < 2 || max_candidates < 2 || max_merges == 0)
            && (merged > 0 || removed > 0 || after != before)
        {
            failures.push(format!(
                    "memory_compaction skipped state requires merged=0 removed=0 after=before, got merged={merged} removed={removed} before={before} after={after}"
                ));
        }
        match json_array_after_field(compaction, "pairs").and_then(json_object_array_items) {
            Some(pairs) => {
                if pairs.len() != merged {
                    failures.push(format!(
                        "memory_compaction pairs {} do not match merged {merged}",
                        pairs.len()
                    ));
                }
                for (index, pair) in pairs.iter().enumerate() {
                    evaluate_memory_compaction_pair(&mut failures, index, pair);
                }
            }
            None => failures.push("memory_compaction pairs array is missing or invalid".to_owned()),
        }
    } else {
        failures.push("memory_compaction object is missing or invalid".to_owned());
    }

    failures
}

fn evaluate_memory_compaction_pair(failures: &mut Vec<String>, index: usize, pair: &str) {
    let primary_id = extract_json_usize_field(pair, "primary_id").unwrap_or(0);
    let removed_id = extract_json_usize_field(pair, "removed_id").unwrap_or(0);
    let similarity = extract_json_f32_field(pair, "similarity").unwrap_or(f32::NAN);
    let namespace = extract_json_string_field(pair, "namespace").unwrap_or_default();
    let primary_vector_dimensions =
        extract_json_usize_field(pair, "primary_vector_dimensions").unwrap_or(0);
    let removed_vector_dimensions =
        extract_json_usize_field(pair, "removed_vector_dimensions").unwrap_or(0);
    let primary_protected = extract_json_bool_field(pair, "primary_protected");
    let removed_protected = extract_json_bool_field(pair, "removed_protected");

    if primary_id == 0 || removed_id == 0 {
        failures.push(format!(
            "memory_compaction pair {index} primary_id and removed_id must be non-zero"
        ));
    }
    if primary_id == removed_id {
        failures.push(format!(
            "memory_compaction pair {index} primary_id must differ from removed_id"
        ));
    }
    if !(0.10..=1.0).contains(&similarity) {
        failures.push(format!(
            "memory_compaction pair {index} similarity {similarity:.6} must stay within 0.10..=1.0"
        ));
    }
    if !namespace_is_safe_for_compaction_evidence(&namespace) {
        failures.push(format!(
            "memory_compaction pair {index} namespace is empty, too broad, or leaks prompt text"
        ));
    }
    if primary_vector_dimensions == 0 || removed_vector_dimensions == 0 {
        failures.push(format!(
            "memory_compaction pair {index} vector dimensions must be non-zero"
        ));
    }
    if primary_protected.is_none() || removed_protected.is_none() {
        failures.push(format!(
            "memory_compaction pair {index} protected fields must be booleans"
        ));
    }
    if removed_protected == Some(true) {
        failures.push(format!(
            "memory_compaction pair {index} must not remove a protected memory"
        ));
    }
}

fn namespace_is_safe_for_compaction_evidence(namespace: &str) -> bool {
    if namespace.is_empty() || namespace.len() > 96 || namespace.contains(" :: ") {
        return false;
    }
    namespace == "semantic"
        || namespace == "gist"
        || (namespace.starts_with("runtime_kv:")
            && namespace
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '-' | '_')))
}

pub(super) fn evaluate_trace_drift(line: &str) -> Vec<String> {
    let mut failures = Vec::new();
    let drift = json_object_after_field(line, "drift");
    let severity = extract_json_string_field(line, "severity");
    let memory_write = extract_json_bool_field(line, "memory_write").unwrap_or(false);
    let runtime_kv_write = extract_json_bool_field(line, "runtime_kv_write").unwrap_or(false);
    let penalize_used_memory =
        extract_json_bool_field(line, "penalize_used_memory").unwrap_or(false);
    let rollback_adaptive = extract_json_bool_field(line, "rollback_adaptive").unwrap_or(false);
    let used_memories = extract_json_usize_field(line, "used").unwrap_or(0);
    let feedback_penalized = extract_json_usize_field(line, "feedback_penalized").unwrap_or(0);
    let runtime_kv_yield_penalties = runtime_kv_yield_penalty_count(line);
    let live_stored_memory = extract_json_bool_field(line, "live_stored_memory").unwrap_or(false);
    let live_stored_gist_memories =
        extract_json_usize_field(line, "live_stored_gist_memories").unwrap_or(0);
    let live_stored_runtime_kv_memories =
        extract_json_usize_field(line, "live_stored_runtime_kv_memories").unwrap_or(0);
    let runtime_kv_stored = extract_json_usize_field(line, "runtime_kv_stored").unwrap_or(0);
    let drift_notes = drift
        .and_then(|drift| extract_json_string_array_field(drift, "notes"))
        .unwrap_or_default();
    let live_router_threshold_delta =
        extract_json_f32_field(line, "live_router_threshold_delta").unwrap_or(0.0);
    let live_hierarchy_weight_delta =
        extract_json_f32_field(line, "live_hierarchy_weight_delta").unwrap_or(0.0);
    let cumulative_drift_rollbacks =
        extract_json_usize_field(line, "cumulative_drift_rollbacks").unwrap_or(0);
    let cumulative_rollback_router_threshold_delta =
        extract_json_f32_field(line, "cumulative_rollback_router_threshold_delta").unwrap_or(0.0);
    let cumulative_rollback_hierarchy_weight_delta =
        extract_json_f32_field(line, "cumulative_rollback_hierarchy_weight_delta").unwrap_or(0.0);

    match severity.as_deref() {
        Some("stable") => {
            if !memory_write {
                failures.push("drift severity stable requires memory_write=true".to_owned());
            }
            if !runtime_kv_write {
                failures.push("drift severity stable requires runtime_kv_write=true".to_owned());
            }
            if penalize_used_memory {
                failures
                    .push("drift severity stable requires penalize_used_memory=false".to_owned());
            }
            if rollback_adaptive {
                failures.push("drift severity stable requires rollback_adaptive=false".to_owned());
            }
        }
        Some("watch") => {
            if !memory_write {
                failures.push("drift severity watch requires memory_write=true".to_owned());
            }
            if penalize_used_memory {
                failures
                    .push("drift severity watch requires penalize_used_memory=false".to_owned());
            }
            if rollback_adaptive {
                failures.push("drift severity watch requires rollback_adaptive=false".to_owned());
            }
        }
        Some("block") => {
            if memory_write {
                failures.push("drift severity block requires memory_write=false".to_owned());
            }
            if runtime_kv_write {
                failures.push("drift severity block requires runtime_kv_write=false".to_owned());
            }
            if rollback_adaptive {
                failures.push("drift severity block requires rollback_adaptive=false".to_owned());
            }
            if used_memories > 0 && !penalize_used_memory {
                failures.push(format!(
                    "drift severity block with used memories {used_memories} requires penalize_used_memory=true"
                ));
            }
        }
        Some("rollback") => {
            if !rollback_adaptive {
                failures.push("drift severity rollback requires rollback_adaptive=true".to_owned());
            }
            if memory_write {
                failures.push("drift severity rollback requires memory_write=false".to_owned());
            }
            if runtime_kv_write {
                failures.push("drift severity rollback requires runtime_kv_write=false".to_owned());
            }
            if used_memories > 0 && !penalize_used_memory {
                failures.push(format!(
                    "drift severity rollback with used memories {used_memories} requires penalize_used_memory=true"
                ));
            }
        }
        Some(other) => failures.push(format!("drift severity {other} is not recognized")),
        None => failures.push("drift severity is missing".to_owned()),
    }

    if runtime_kv_write && !memory_write {
        failures.push("drift runtime_kv_write=true requires memory_write=true".to_owned());
    }
    if penalize_used_memory && used_memories == 0 {
        failures.push("drift penalize_used_memory=true requires used memories > 0".to_owned());
    }
    if feedback_penalized > runtime_kv_yield_penalties && !penalize_used_memory {
        failures.push(format!(
            "memory feedback_penalized {feedback_penalized} exceeds sourced Runtime KV yield penalties {runtime_kv_yield_penalties} without penalize_used_memory=true"
        ));
    }
    if !memory_write
        && (live_stored_memory
            || live_stored_gist_memories > 0
            || live_stored_runtime_kv_memories > 0)
    {
        failures.push(
            "drift memory_write=false forbids live stored semantic/gist/runtime KV memory"
                .to_owned(),
        );
    }

    if drift_notes
        .iter()
        .any(|note| note == "route:fast_path_watch")
    {
        if severity.as_deref() != Some("watch") {
            failures.push("route:fast_path_watch requires drift severity watch".to_owned());
        }
        if !memory_write {
            failures.push("route:fast_path_watch keeps memory_write=true".to_owned());
        }
        if runtime_kv_write {
            failures.push("route:fast_path_watch requires runtime_kv_write=false".to_owned());
        }
        if runtime_kv_stored > 0 || live_stored_runtime_kv_memories > 0 {
            failures.push(format!(
                "route:fast_path_watch forbids runtime KV storage, got runtime_kv_stored={runtime_kv_stored} live_stored_runtime_kv_memories={live_stored_runtime_kv_memories}"
            ));
        }
    }

    if rollback_adaptive {
        if severity.as_deref() != Some("rollback") {
            failures.push("rollback_adaptive=true requires drift severity rollback".to_owned());
        }
        require_usize_at_least(
            &mut failures,
            "cumulative_drift_rollbacks",
            cumulative_drift_rollbacks,
            "rollback_adaptive",
            1,
        );
        if live_router_threshold_delta > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "rollback_adaptive=true requires live_router_threshold_delta=0, got {live_router_threshold_delta:.6}"
            ));
        }
        if live_hierarchy_weight_delta > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "rollback_adaptive=true requires live_hierarchy_weight_delta=0, got {live_hierarchy_weight_delta:.6}"
            ));
        }
        if cumulative_rollback_router_threshold_delta <= TRACE_FLOAT_EPSILON {
            failures.push(
                "rollback_adaptive=true requires cumulative_rollback_router_threshold_delta > 0"
                    .to_owned(),
            );
        }
        if cumulative_rollback_hierarchy_weight_delta <= TRACE_FLOAT_EPSILON {
            failures.push(
                "rollback_adaptive=true requires cumulative_rollback_hierarchy_weight_delta > 0"
                    .to_owned(),
            );
        }
    }

    if cumulative_drift_rollbacks == 0 {
        if cumulative_rollback_router_threshold_delta > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "cumulative_rollback_router_threshold_delta {cumulative_rollback_router_threshold_delta:.6} requires cumulative_drift_rollbacks > 0"
            ));
        }
        if cumulative_rollback_hierarchy_weight_delta > TRACE_FLOAT_EPSILON {
            failures.push(format!(
                "cumulative_rollback_hierarchy_weight_delta {cumulative_rollback_hierarchy_weight_delta:.6} requires cumulative_drift_rollbacks > 0"
            ));
        }
    } else if cumulative_rollback_router_threshold_delta <= TRACE_FLOAT_EPSILON
        || cumulative_rollback_hierarchy_weight_delta <= TRACE_FLOAT_EPSILON
    {
        failures.push(format!(
            "cumulative_drift_rollbacks {cumulative_drift_rollbacks} requires positive rollback router and hierarchy deltas"
        ));
    }

    failures
}

fn runtime_kv_yield_penalty_count(line: &str) -> usize {
    let Some(segment_yield) = json_object_after_field(line, "runtime_diagnostics")
        .and_then(|runtime| extract_json_f32_field(runtime, "runtime_kv_segment_yield"))
    else {
        return 0;
    };
    if !segment_yield.is_finite() || segment_yield >= 0.45 {
        return 0;
    }

    let expected_amount = (0.20 + (1.0 - segment_yield.clamp(0.0, 1.0)) * 0.70).clamp(0.20, 0.90);
    json_object_after_field(line, "memory")
        .and_then(|memory| extract_json_string_array_field(memory, "feedback_update_summaries"))
        .unwrap_or_default()
        .iter()
        .filter(|summary| {
            summary.starts_with("penalize#")
                && trace_note_f32(summary, "amount=")
                    .is_some_and(|amount| (amount - expected_amount).abs() <= TRACE_FLOAT_EPSILON)
        })
        .count()
}
