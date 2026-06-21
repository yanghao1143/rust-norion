use super::TRACE_FLOAT_EPSILON;
use super::evolution::require_usize_at_least;
use super::fields::*;
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
    let hold = extract_json_usize_field(admission, "hold").unwrap_or(0);
    let reject = extract_json_usize_field(admission, "reject").unwrap_or(0);
    let quarantine = extract_json_usize_field(admission, "quarantine").unwrap_or(0);
    let kinds = extract_json_string_array_field(admission, "kinds").unwrap_or_default();
    let decisions = extract_json_string_array_field(admission, "decisions").unwrap_or_default();
    let summaries =
        extract_json_string_array_field(admission, "candidate_summaries").unwrap_or_default();
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
    if summaries.len() != candidates {
        failures.push(format!(
            "memory_admission candidate_summaries {} do not match candidates {candidates}",
            summaries.len()
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
    if read_only != Some(true) {
        failures.push("memory_admission read_only must be true".to_owned());
    }
    if write_allowed != Some(false) {
        failures.push("memory_admission write_allowed must be false".to_owned());
    }
    if applied != Some(false) {
        failures.push("memory_admission applied must be false".to_owned());
    }
    if summaries
        .iter()
        .any(|summary| summary.contains("prompt:") || summary.contains("answer:"))
    {
        failures.push(
            "memory_admission candidate_summaries must not leak raw prompt or answer payloads"
                .to_owned(),
        );
    }

    failures
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
    if feedback_penalized > 0 && !penalize_used_memory {
        failures.push(format!(
            "memory feedback_penalized {feedback_penalized} requires penalize_used_memory=true"
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
