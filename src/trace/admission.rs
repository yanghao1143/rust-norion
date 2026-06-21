use super::fields::{
    extract_json_bool_field, extract_json_string_array_field, extract_json_string_field,
    extract_json_usize_field, extract_last_json_string_array_field, json_array_after_field,
    json_object_after_field, json_object_array_items,
};

pub(super) fn evaluate_self_evolution_admission_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-evolution-admission-v1\"",
        ),
        ("candidate_id", "\"candidate_id\":"),
        ("read_only", "\"read_only\":"),
        ("report_only", "\"report_only\":"),
        ("preview_only", "\"preview_only\":"),
        ("policy_valid", "\"policy_valid\":"),
        (
            "admitted_for_human_review",
            "\"admitted_for_human_review\":",
        ),
        ("human_approval_required", "\"human_approval_required\":"),
        ("review_packet", "\"review_packet\":"),
        ("rust_check", "\"rust_check\":"),
        ("validation", "\"validation\":"),
        ("benchmark_gate", "\"benchmark_gate\":"),
        ("rollback", "\"rollback\":"),
        ("adaptive_preview", "\"adaptive_preview\":"),
        ("writes", "\"writes\":"),
        ("blocked_reasons", "\"blocked_reasons\":"),
        ("telemetry", "\"telemetry\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing self_evolution_admission field {name}"));
        }
    }

    let candidate_id = extract_json_string_field(line, "candidate_id").unwrap_or_default();
    if candidate_id.trim().is_empty() {
        failures.push("self_evolution_admission candidate_id is empty".to_owned());
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "self_evolution_admission",
    );
    require_bool(
        &mut failures,
        line,
        "report_only",
        true,
        "self_evolution_admission",
    );
    require_bool(
        &mut failures,
        line,
        "preview_only",
        true,
        "self_evolution_admission",
    );
    require_bool(
        &mut failures,
        line,
        "human_approval_required",
        true,
        "self_evolution_admission",
    );
    require_bool(
        &mut failures,
        line,
        "policy_valid",
        true,
        "self_evolution_admission",
    );

    let admitted_for_human_review = extract_json_bool_field(line, "admitted_for_human_review");
    let blocked_reasons =
        extract_last_json_string_array_field(line, "blocked_reasons").unwrap_or_default();
    match admitted_for_human_review {
        Some(true) if !blocked_reasons.is_empty() => failures.push(
            "self_evolution_admission admitted review packet must not have blocked reasons"
                .to_owned(),
        ),
        Some(false) if blocked_reasons.is_empty() => failures.push(
            "self_evolution_admission blocked review packet requires blocked reasons".to_owned(),
        ),
        Some(_) => {}
        None => failures
            .push("self_evolution_admission admitted_for_human_review must be boolean".to_owned()),
    }

    evaluate_rust_check(&mut failures, line);
    evaluate_validation(&mut failures, line, admitted_for_human_review);
    evaluate_benchmark_gate(&mut failures, line);
    evaluate_review_packet(&mut failures, line, admitted_for_human_review);
    evaluate_rollback(&mut failures, line);
    evaluate_adaptive_preview(&mut failures, line, admitted_for_human_review);
    evaluate_writes(&mut failures, line);
    evaluate_telemetry(&mut failures, line);

    failures
}

pub(super) fn evaluate_self_evolution_experiment_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-evolution-experiment-v1\"",
        ),
        ("sequence", "\"sequence\":"),
        ("experiment_id", "\"experiment_id\":"),
        ("candidate_id", "\"candidate_id\":"),
        ("decision", "\"decision\":"),
        ("repeated_experiment", "\"repeated_experiment\":"),
        ("conflicting_evidence", "\"conflicting_evidence\":"),
        ("rollback_required", "\"rollback_required\":"),
        ("rollback_replayable", "\"rollback_replayable\":"),
        ("human_approval_required", "\"human_approval_required\":"),
        ("active_candidate", "\"active_candidate\":"),
        ("read_only", "\"read_only\":"),
        ("report_only", "\"report_only\":"),
        ("preview_only", "\"preview_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("evidence_ids", "\"evidence_ids\":"),
        ("rollback_anchor_ids", "\"rollback_anchor_ids\":"),
        ("blocked_reasons", "\"blocked_reasons\":"),
        ("content_digest", "\"content_digest\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing self_evolution_experiment field {name}"));
        }
    }

    let sequence = extract_json_usize_field(line, "sequence").unwrap_or(0);
    if sequence == 0 {
        failures.push("self_evolution_experiment sequence must be positive".to_owned());
    }
    for field in ["experiment_id", "candidate_id", "content_digest"] {
        let value = extract_json_string_field(line, field).unwrap_or_default();
        if value.trim().is_empty() {
            failures.push(format!("self_evolution_experiment {field} is empty"));
        }
    }
    let content_digest = extract_json_string_field(line, "content_digest").unwrap_or_default();
    if !content_digest.starts_with("fnv64:") {
        failures.push("self_evolution_experiment content_digest must be stable fnv64".to_owned());
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "self_evolution_experiment",
    );
    require_bool(
        &mut failures,
        line,
        "report_only",
        true,
        "self_evolution_experiment",
    );
    require_bool(
        &mut failures,
        line,
        "preview_only",
        true,
        "self_evolution_experiment",
    );
    require_bool(
        &mut failures,
        line,
        "human_approval_required",
        true,
        "self_evolution_experiment",
    );
    require_bool(
        &mut failures,
        line,
        "active_candidate",
        false,
        "self_evolution_experiment",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "self_evolution_experiment",
    );
    require_bool(
        &mut failures,
        line,
        "applied",
        false,
        "self_evolution_experiment",
    );

    let decision = extract_json_string_field(line, "decision").unwrap_or_default();
    let rollback_required = extract_json_bool_field(line, "rollback_required");
    let rollback_replayable = extract_json_bool_field(line, "rollback_replayable");
    let conflicting_evidence = extract_json_bool_field(line, "conflicting_evidence");
    let evidence_ids = extract_json_string_array_field(line, "evidence_ids").unwrap_or_default();
    let rollback_anchor_ids =
        extract_json_string_array_field(line, "rollback_anchor_ids").unwrap_or_default();
    let blocked_reasons =
        extract_json_string_array_field(line, "blocked_reasons").unwrap_or_default();

    for (field, values) in [
        ("evidence_ids", &evidence_ids),
        ("rollback_anchor_ids", &rollback_anchor_ids),
        ("blocked_reasons", &blocked_reasons),
    ] {
        if values.iter().any(|value| value.trim().is_empty()) {
            failures.push(format!(
                "self_evolution_experiment {field} contains empty item"
            ));
        }
    }
    if evidence_ids.is_empty() {
        failures.push("self_evolution_experiment requires evidence_ids".to_owned());
    }
    if rollback_anchor_ids.is_empty() {
        failures.push("self_evolution_experiment requires rollback_anchor_ids".to_owned());
    }

    match decision.as_str() {
        "admit_for_human_review" => {
            if !blocked_reasons.is_empty() {
                failures.push(
                    "self_evolution_experiment admitted record must not have blocked reasons"
                        .to_owned(),
                );
            }
            if rollback_required != Some(false) {
                failures.push(
                    "self_evolution_experiment admitted record must not require rollback"
                        .to_owned(),
                );
            }
        }
        "hold" => {
            if blocked_reasons.is_empty() && conflicting_evidence != Some(true) {
                failures.push(
                    "self_evolution_experiment hold requires blocked reasons or conflict"
                        .to_owned(),
                );
            }
            if rollback_required != Some(false) {
                failures
                    .push("self_evolution_experiment hold must not require rollback".to_owned());
            }
        }
        "reject" => {
            if blocked_reasons.is_empty() {
                failures
                    .push("self_evolution_experiment reject requires blocked reasons".to_owned());
            }
            if rollback_required != Some(false) {
                failures
                    .push("self_evolution_experiment reject must not require rollback".to_owned());
            }
        }
        "rollback" => {
            if rollback_required != Some(true) {
                failures
                    .push("self_evolution_experiment rollback must require rollback".to_owned());
            }
            if rollback_replayable != Some(true) {
                failures.push("self_evolution_experiment rollback must be replayable".to_owned());
            }
            if blocked_reasons.is_empty() {
                failures
                    .push("self_evolution_experiment rollback requires blocked reasons".to_owned());
            }
        }
        _ => failures.push(format!(
            "self_evolution_experiment decision {decision} is not supported"
        )),
    }

    failures
}

pub(super) fn evaluate_self_evolution_rollback_replay_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-evolution-rollback-replay-plan-v1\"",
        ),
        ("item_count", "\"item_count\":"),
        ("replayable", "\"replayable\":"),
        ("blocked", "\"blocked\":"),
        ("all_replayable", "\"all_replayable\":"),
        ("active_candidates", "\"active_candidates\":"),
        ("item_write_allowed", "\"item_write_allowed\":"),
        ("item_applied", "\"item_applied\":"),
        ("read_only", "\"read_only\":"),
        ("report_only", "\"report_only\":"),
        ("preview_only", "\"preview_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("rollback_anchor_ids", "\"rollback_anchor_ids\":"),
        ("evidence_ids", "\"evidence_ids\":"),
        ("items", "\"items\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!(
                "missing self_evolution_rollback_replay field {name}"
            ));
        }
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "self_evolution_rollback_replay",
    );
    require_bool(
        &mut failures,
        line,
        "report_only",
        true,
        "self_evolution_rollback_replay",
    );
    require_bool(
        &mut failures,
        line,
        "preview_only",
        true,
        "self_evolution_rollback_replay",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "self_evolution_rollback_replay",
    );
    require_bool(
        &mut failures,
        line,
        "applied",
        false,
        "self_evolution_rollback_replay",
    );

    let item_count = extract_json_usize_field(line, "item_count");
    let replayable = extract_json_usize_field(line, "replayable");
    let blocked = extract_json_usize_field(line, "blocked");
    let all_replayable = extract_json_bool_field(line, "all_replayable");
    let active_candidates = extract_json_usize_field(line, "active_candidates");
    let item_write_allowed = extract_json_usize_field(line, "item_write_allowed");
    let item_applied = extract_json_usize_field(line, "item_applied");
    let rollback_anchor_ids =
        extract_json_string_array_field(line, "rollback_anchor_ids").unwrap_or_default();
    let evidence_ids = extract_json_string_array_field(line, "evidence_ids").unwrap_or_default();

    for (field, values) in [
        ("rollback_anchor_ids", &rollback_anchor_ids),
        ("evidence_ids", &evidence_ids),
    ] {
        if values.iter().any(|value| value.trim().is_empty()) {
            failures.push(format!(
                "self_evolution_rollback_replay {field} contains empty item"
            ));
        }
    }

    match (item_count, replayable, blocked) {
        (Some(items), Some(replayable), Some(blocked))
            if replayable.saturating_add(blocked) != items =>
        {
            failures.push(format!(
                "self_evolution_rollback_replay replayable+blocked {} does not match item_count {items}",
                replayable.saturating_add(blocked)
            ));
        }
        (Some(_), Some(_), Some(_)) => {}
        _ => failures.push("self_evolution_rollback_replay count fields are incomplete".to_owned()),
    }

    match (all_replayable, blocked) {
        (Some(true), Some(0)) | (Some(false), Some(_)) => {}
        (Some(true), Some(blocked)) => failures.push(format!(
            "self_evolution_rollback_replay all_replayable=true but blocked={blocked}"
        )),
        (Some(_), None) => {
            failures.push("self_evolution_rollback_replay blocked count is missing".to_owned())
        }
        (None, _) => {
            failures.push("self_evolution_rollback_replay all_replayable is missing".to_owned())
        }
    }

    if replayable.unwrap_or(0) > 0 {
        if rollback_anchor_ids.is_empty() {
            failures.push(
                "self_evolution_rollback_replay replayable plan requires rollback_anchor_ids"
                    .to_owned(),
            );
        }
        if evidence_ids.is_empty() {
            failures.push(
                "self_evolution_rollback_replay replayable plan requires evidence_ids".to_owned(),
            );
        }
    }

    let item_objects = match json_array_after_field(line, "items") {
        Some(items) => match json_object_array_items(items) {
            Some(items) => items,
            None => {
                failures
                    .push("self_evolution_rollback_replay items must be object array".to_owned());
                Vec::new()
            }
        },
        None => {
            failures.push("self_evolution_rollback_replay items array is missing".to_owned());
            Vec::new()
        }
    };

    if let Some(item_count) = item_count {
        if item_objects.len() != item_count {
            failures.push(format!(
                "self_evolution_rollback_replay items length {} does not match item_count {item_count}",
                item_objects.len()
            ));
        }
    }

    let mut actual_replayable = 0usize;
    let mut actual_blocked = 0usize;
    let mut actual_active_candidates = 0usize;
    let mut actual_item_write_allowed = 0usize;
    let mut actual_item_applied = 0usize;

    for item in item_objects {
        evaluate_self_evolution_rollback_replay_item(item, &mut failures);
        let item_replayable = extract_json_bool_field(item, "replayable").unwrap_or(false);
        actual_replayable = actual_replayable.saturating_add(usize::from(item_replayable));
        actual_blocked = actual_blocked.saturating_add(usize::from(!item_replayable));
        actual_active_candidates = actual_active_candidates.saturating_add(usize::from(
            extract_json_bool_field(item, "active_candidate").unwrap_or(false),
        ));
        actual_item_write_allowed = actual_item_write_allowed.saturating_add(usize::from(
            extract_json_bool_field(item, "write_allowed").unwrap_or(false),
        ));
        actual_item_applied = actual_item_applied.saturating_add(usize::from(
            extract_json_bool_field(item, "applied").unwrap_or(false),
        ));
    }

    for (field, expected, actual) in [
        ("replayable", replayable, actual_replayable),
        ("blocked", blocked, actual_blocked),
        (
            "active_candidates",
            active_candidates,
            actual_active_candidates,
        ),
        (
            "item_write_allowed",
            item_write_allowed,
            actual_item_write_allowed,
        ),
        ("item_applied", item_applied, actual_item_applied),
    ] {
        if let Some(expected) = expected {
            if expected != actual {
                failures.push(format!(
                    "self_evolution_rollback_replay {field}={expected} does not match item total {actual}"
                ));
            }
        }
    }

    failures
}

pub(super) fn evaluate_self_evolution_rollback_replay_gate_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-evolution-rollback-replay-gate-v1\"",
        ),
        ("decision", "\"decision\":"),
        (
            "admitted_for_human_review",
            "\"admitted_for_human_review\":",
        ),
        ("human_approval_required", "\"human_approval_required\":"),
        ("review_packet", "\"review_packet\":"),
        ("item_count", "\"item_count\":"),
        ("replayable", "\"replayable\":"),
        ("blocked", "\"blocked\":"),
        ("all_replayable", "\"all_replayable\":"),
        ("active_candidates", "\"active_candidates\":"),
        ("item_write_allowed", "\"item_write_allowed\":"),
        ("item_applied", "\"item_applied\":"),
        ("read_only", "\"read_only\":"),
        ("report_only", "\"report_only\":"),
        ("preview_only", "\"preview_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("plan_read_only", "\"plan_read_only\":"),
        ("plan_report_only", "\"plan_report_only\":"),
        ("plan_preview_only", "\"plan_preview_only\":"),
        ("plan_write_allowed", "\"plan_write_allowed\":"),
        ("plan_applied", "\"plan_applied\":"),
        ("rollback_anchor_ids", "\"rollback_anchor_ids\":"),
        ("evidence_ids", "\"evidence_ids\":"),
        ("blocked_reasons", "\"blocked_reasons\":"),
        ("content_digest", "\"content_digest\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!(
                "missing self_evolution_rollback_replay_gate field {name}"
            ));
        }
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "self_evolution_rollback_replay_gate",
    );
    require_bool(
        &mut failures,
        line,
        "report_only",
        true,
        "self_evolution_rollback_replay_gate",
    );
    require_bool(
        &mut failures,
        line,
        "preview_only",
        true,
        "self_evolution_rollback_replay_gate",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "self_evolution_rollback_replay_gate",
    );
    require_bool(
        &mut failures,
        line,
        "applied",
        false,
        "self_evolution_rollback_replay_gate",
    );
    require_bool(
        &mut failures,
        line,
        "human_approval_required",
        true,
        "self_evolution_rollback_replay_gate",
    );

    let decision = extract_json_string_field(line, "decision").unwrap_or_default();
    let admitted_for_human_review =
        extract_json_bool_field(line, "admitted_for_human_review").unwrap_or(false);
    let item_count = extract_json_usize_field(line, "item_count");
    let replayable = extract_json_usize_field(line, "replayable");
    let blocked = extract_json_usize_field(line, "blocked");
    let all_replayable = extract_json_bool_field(line, "all_replayable");
    let active_candidates = extract_json_usize_field(line, "active_candidates").unwrap_or(0);
    let item_write_allowed = extract_json_usize_field(line, "item_write_allowed").unwrap_or(0);
    let item_applied = extract_json_usize_field(line, "item_applied").unwrap_or(0);
    let plan_read_only = extract_json_bool_field(line, "plan_read_only").unwrap_or(false);
    let plan_report_only = extract_json_bool_field(line, "plan_report_only").unwrap_or(false);
    let plan_preview_only = extract_json_bool_field(line, "plan_preview_only").unwrap_or(false);
    let plan_write_allowed = extract_json_bool_field(line, "plan_write_allowed").unwrap_or(false);
    let plan_applied = extract_json_bool_field(line, "plan_applied").unwrap_or(false);
    let rollback_anchor_ids =
        extract_json_string_array_field(line, "rollback_anchor_ids").unwrap_or_default();
    let evidence_ids = extract_json_string_array_field(line, "evidence_ids").unwrap_or_default();
    let blocked_reasons =
        extract_json_string_array_field(line, "blocked_reasons").unwrap_or_default();
    let content_digest = extract_json_string_field(line, "content_digest").unwrap_or_default();
    evaluate_rollback_replay_gate_review_packet(&mut failures, line, admitted_for_human_review);

    if !content_digest.starts_with("fnv64:") {
        failures.push(
            "self_evolution_rollback_replay_gate content_digest must be stable fnv64".to_owned(),
        );
    }

    for (field, values) in [
        ("rollback_anchor_ids", &rollback_anchor_ids),
        ("evidence_ids", &evidence_ids),
        ("blocked_reasons", &blocked_reasons),
    ] {
        if values.iter().any(|value| value.trim().is_empty()) {
            failures.push(format!(
                "self_evolution_rollback_replay_gate {field} contains empty item"
            ));
        }
    }

    match (item_count, replayable, blocked) {
        (Some(items), Some(replayable), Some(blocked))
            if replayable.saturating_add(blocked) != items =>
        {
            failures.push(format!(
                "self_evolution_rollback_replay_gate replayable+blocked {} does not match item_count {items}",
                replayable.saturating_add(blocked)
            ));
        }
        (Some(_), Some(_), Some(_)) => {}
        _ => failures
            .push("self_evolution_rollback_replay_gate count fields are incomplete".to_owned()),
    }

    match (all_replayable, blocked) {
        (Some(true), Some(0)) | (Some(false), Some(_)) => {}
        (Some(true), Some(blocked)) => failures.push(format!(
            "self_evolution_rollback_replay_gate all_replayable=true but blocked={blocked}"
        )),
        (Some(_), None) => {
            failures.push("self_evolution_rollback_replay_gate blocked count is missing".to_owned())
        }
        (None, _) => failures
            .push("self_evolution_rollback_replay_gate all_replayable is missing".to_owned()),
    }

    match decision.as_str() {
        "admit_for_human_review" => {
            if !admitted_for_human_review {
                failures.push(
                    "self_evolution_rollback_replay_gate admitted decision requires admitted_for_human_review=true"
                        .to_owned(),
                );
            }
            if item_count.unwrap_or(0) == 0 {
                failures
                    .push("self_evolution_rollback_replay_gate admitted plan is empty".to_owned());
            }
            if blocked.unwrap_or(0) > 0 || !all_replayable.unwrap_or(false) {
                failures.push(
                    "self_evolution_rollback_replay_gate admitted plan must be all replayable"
                        .to_owned(),
                );
            }
            if rollback_anchor_ids.is_empty() {
                failures.push(
                    "self_evolution_rollback_replay_gate admitted plan requires rollback_anchor_ids"
                        .to_owned(),
                );
            }
            if evidence_ids.is_empty() {
                failures.push(
                    "self_evolution_rollback_replay_gate admitted plan requires evidence_ids"
                        .to_owned(),
                );
            }
            if active_candidates > 0 {
                failures.push(
                    "self_evolution_rollback_replay_gate admitted plan has active candidates"
                        .to_owned(),
                );
            }
            if item_write_allowed > 0 || item_applied > 0 {
                failures.push(
                    "self_evolution_rollback_replay_gate admitted plan has item write/applied flags"
                        .to_owned(),
                );
            }
            if !plan_read_only || !plan_report_only || !plan_preview_only {
                failures.push(
                    "self_evolution_rollback_replay_gate admitted plan must stay read/report/preview only"
                        .to_owned(),
                );
            }
            if plan_write_allowed || plan_applied {
                failures.push(
                    "self_evolution_rollback_replay_gate admitted plan must not write or apply"
                        .to_owned(),
                );
            }
            if !blocked_reasons.is_empty() {
                failures.push(
                    "self_evolution_rollback_replay_gate admitted plan must not have blocked reasons"
                        .to_owned(),
                );
            }
        }
        "hold" => {
            if admitted_for_human_review {
                failures.push(
                    "self_evolution_rollback_replay_gate hold requires admitted_for_human_review=false"
                        .to_owned(),
                );
            }
            if blocked_reasons.is_empty() {
                failures.push(
                    "self_evolution_rollback_replay_gate hold requires blocked reasons".to_owned(),
                );
            }
        }
        _ => failures.push(format!(
            "self_evolution_rollback_replay_gate decision {decision} is not supported"
        )),
    }

    if replayable.unwrap_or(0) > 0 {
        if rollback_anchor_ids.is_empty() {
            failures.push(
                "self_evolution_rollback_replay_gate replayable plan requires rollback_anchor_ids"
                    .to_owned(),
            );
        }
        if evidence_ids.is_empty() {
            failures.push(
                "self_evolution_rollback_replay_gate replayable plan requires evidence_ids"
                    .to_owned(),
            );
        }
    }

    failures
}

pub(super) fn evaluate_self_evolution_operator_approval_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-evolution-operator-approval-v1\"",
        ),
        ("decision", "\"decision\":"),
        ("operator_approved", "\"operator_approved\":"),
        ("operator_digest", "\"operator_digest\":"),
        ("approval_ticket_digest", "\"approval_ticket_digest\":"),
        (
            "approved_review_packet_count",
            "\"approved_review_packet_count\":",
        ),
        ("approved_evidence_count", "\"approved_evidence_count\":"),
        (
            "approved_rollback_anchor_count",
            "\"approved_rollback_anchor_count\":",
        ),
        (
            "approved_content_digest_count",
            "\"approved_content_digest_count\":",
        ),
        (
            "approved_source_report_schema_count",
            "\"approved_source_report_schema_count\":",
        ),
        ("approved_refs_digest", "\"approved_refs_digest\":"),
        ("approval_reason_digest", "\"approval_reason_digest\":"),
        (
            "approval_attestation_digest",
            "\"approval_attestation_digest\":",
        ),
        ("read_only", "\"read_only\":"),
        ("report_only", "\"report_only\":"),
        ("preview_only", "\"preview_only\":"),
        ("activation_write_allowed", "\"activation_write_allowed\":"),
        ("active_candidate", "\"active_candidate\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("blocked_reasons_count", "\"blocked_reasons_count\":"),
        ("blocked_reasons_digest", "\"blocked_reasons_digest\":"),
        ("content_digest", "\"content_digest\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!(
                "missing self_evolution_operator_approval field {name}"
            ));
        }
    }

    for raw_field in ["operator_id", "approval_ticket_id", "approval_reason"] {
        if line.contains(&format!("\"{raw_field}\":")) {
            failures.push(format!(
                "self_evolution_operator_approval must not expose raw {raw_field}"
            ));
        }
    }
    for raw_array in [
        "approved_review_packet_ids",
        "approved_evidence_ids",
        "approved_rollback_anchor_ids",
        "approved_content_digests",
        "approved_source_report_schemas",
        "blocked_reasons",
    ] {
        if line.contains(&format!("\"{raw_array}\":[")) {
            failures.push(format!(
                "self_evolution_operator_approval must expose {raw_array} as count/digest only"
            ));
        }
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "self_evolution_operator_approval",
    );
    require_bool(
        &mut failures,
        line,
        "report_only",
        true,
        "self_evolution_operator_approval",
    );
    require_bool(
        &mut failures,
        line,
        "preview_only",
        true,
        "self_evolution_operator_approval",
    );
    require_bool(
        &mut failures,
        line,
        "activation_write_allowed",
        false,
        "self_evolution_operator_approval",
    );
    require_bool(
        &mut failures,
        line,
        "active_candidate",
        false,
        "self_evolution_operator_approval",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "self_evolution_operator_approval",
    );
    require_bool(
        &mut failures,
        line,
        "applied",
        false,
        "self_evolution_operator_approval",
    );

    let decision = extract_json_string_field(line, "decision").unwrap_or_default();
    let operator_approved = extract_json_bool_field(line, "operator_approved");
    let blocked_reasons_count =
        extract_json_usize_field(line, "blocked_reasons_count").unwrap_or(0);

    for field in [
        "operator_digest",
        "approval_ticket_digest",
        "approved_refs_digest",
        "approval_reason_digest",
        "approval_attestation_digest",
        "blocked_reasons_digest",
        "content_digest",
    ] {
        let value = extract_json_string_field(line, field).unwrap_or_default();
        if !value.starts_with("fnv64:") {
            failures.push(format!(
                "self_evolution_operator_approval {field} must be stable fnv64"
            ));
        }
    }

    let approved_review_packet_count =
        extract_json_usize_field(line, "approved_review_packet_count").unwrap_or(0);
    let approved_evidence_count =
        extract_json_usize_field(line, "approved_evidence_count").unwrap_or(0);
    let approved_rollback_anchor_count =
        extract_json_usize_field(line, "approved_rollback_anchor_count").unwrap_or(0);
    let approved_content_digest_count =
        extract_json_usize_field(line, "approved_content_digest_count").unwrap_or(0);
    let approved_source_report_schema_count =
        extract_json_usize_field(line, "approved_source_report_schema_count").unwrap_or(0);

    match decision.as_str() {
        "approved" => {
            if operator_approved != Some(true) {
                failures.push(
                    "self_evolution_operator_approval approved decision requires operator_approved=true"
                        .to_owned(),
                );
            }
            if blocked_reasons_count != 0 {
                failures.push(
                    "self_evolution_operator_approval approved decision must not have blocked reasons"
                        .to_owned(),
                );
            }
            for (field, count) in [
                ("approved_review_packet_count", approved_review_packet_count),
                ("approved_evidence_count", approved_evidence_count),
                (
                    "approved_rollback_anchor_count",
                    approved_rollback_anchor_count,
                ),
                (
                    "approved_content_digest_count",
                    approved_content_digest_count,
                ),
                (
                    "approved_source_report_schema_count",
                    approved_source_report_schema_count,
                ),
            ] {
                if count == 0 {
                    failures.push(format!(
                        "self_evolution_operator_approval approved decision requires {field}>0"
                    ));
                }
            }
        }
        "hold" => {
            if operator_approved != Some(false) {
                failures.push(
                    "self_evolution_operator_approval hold decision requires operator_approved=false"
                        .to_owned(),
                );
            }
            if blocked_reasons_count == 0 {
                failures.push(
                    "self_evolution_operator_approval hold decision requires blocked reasons"
                        .to_owned(),
                );
            }
        }
        _ => failures.push(format!(
            "self_evolution_operator_approval decision {decision} is not supported"
        )),
    }

    failures
}

pub(super) fn evaluate_self_evolution_rollback_replay_apply_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-self-evolution-rollback-replay-apply-v1\"",
        ),
        ("decision", "\"decision\":"),
        ("ready_for_operator_apply", "\"ready_for_operator_apply\":"),
        ("explicit_apply_required", "\"explicit_apply_required\":"),
        (
            "rollback_gate_admitted_for_human_review",
            "\"rollback_gate_admitted_for_human_review\":",
        ),
        ("operator_approved", "\"operator_approved\":"),
        ("item_count", "\"item_count\":"),
        ("replayable", "\"replayable\":"),
        ("blocked", "\"blocked\":"),
        ("review_packet_count", "\"review_packet_count\":"),
        ("evidence_id_count", "\"evidence_id_count\":"),
        ("rollback_anchor_count", "\"rollback_anchor_count\":"),
        ("content_digest_count", "\"content_digest_count\":"),
        (
            "source_report_schema_count",
            "\"source_report_schema_count\":",
        ),
        ("read_only", "\"read_only\":"),
        ("report_only", "\"report_only\":"),
        ("preview_only", "\"preview_only\":"),
        ("activation_write_allowed", "\"activation_write_allowed\":"),
        ("active_candidate", "\"active_candidate\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("blocked_reasons_count", "\"blocked_reasons_count\":"),
        ("blocked_reasons_digest", "\"blocked_reasons_digest\":"),
        ("content_digest", "\"content_digest\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!(
                "missing self_evolution_rollback_replay_apply field {name}"
            ));
        }
    }

    for raw_array in [
        "approval_review_packet_ids",
        "evidence_ids",
        "rollback_anchor_ids",
        "content_digests",
        "source_report_schemas",
        "blocked_reasons",
    ] {
        if line.contains(&format!("\"{raw_array}\":[")) {
            failures.push(format!(
                "self_evolution_rollback_replay_apply must expose {raw_array} as count/digest only"
            ));
        }
    }

    require_bool(
        &mut failures,
        line,
        "read_only",
        true,
        "self_evolution_rollback_replay_apply",
    );
    require_bool(
        &mut failures,
        line,
        "report_only",
        true,
        "self_evolution_rollback_replay_apply",
    );
    require_bool(
        &mut failures,
        line,
        "preview_only",
        true,
        "self_evolution_rollback_replay_apply",
    );
    require_bool(
        &mut failures,
        line,
        "explicit_apply_required",
        true,
        "self_evolution_rollback_replay_apply",
    );
    require_bool(
        &mut failures,
        line,
        "activation_write_allowed",
        false,
        "self_evolution_rollback_replay_apply",
    );
    require_bool(
        &mut failures,
        line,
        "active_candidate",
        false,
        "self_evolution_rollback_replay_apply",
    );
    require_bool(
        &mut failures,
        line,
        "write_allowed",
        false,
        "self_evolution_rollback_replay_apply",
    );
    require_bool(
        &mut failures,
        line,
        "applied",
        false,
        "self_evolution_rollback_replay_apply",
    );

    for field in ["blocked_reasons_digest", "content_digest"] {
        let value = extract_json_string_field(line, field).unwrap_or_default();
        if !value.starts_with("fnv64:") {
            failures.push(format!(
                "self_evolution_rollback_replay_apply {field} must be stable fnv64"
            ));
        }
    }

    let decision = extract_json_string_field(line, "decision").unwrap_or_default();
    let ready_for_operator_apply =
        extract_json_bool_field(line, "ready_for_operator_apply").unwrap_or(false);
    let rollback_gate_admitted =
        extract_json_bool_field(line, "rollback_gate_admitted_for_human_review").unwrap_or(false);
    let operator_approved = extract_json_bool_field(line, "operator_approved").unwrap_or(false);
    let item_count = extract_json_usize_field(line, "item_count").unwrap_or(0);
    let replayable = extract_json_usize_field(line, "replayable").unwrap_or(0);
    let blocked = extract_json_usize_field(line, "blocked").unwrap_or(0);
    let review_packet_count = extract_json_usize_field(line, "review_packet_count").unwrap_or(0);
    let evidence_id_count = extract_json_usize_field(line, "evidence_id_count").unwrap_or(0);
    let rollback_anchor_count =
        extract_json_usize_field(line, "rollback_anchor_count").unwrap_or(0);
    let content_digest_count = extract_json_usize_field(line, "content_digest_count").unwrap_or(0);
    let source_report_schema_count =
        extract_json_usize_field(line, "source_report_schema_count").unwrap_or(0);
    let blocked_reasons_count =
        extract_json_usize_field(line, "blocked_reasons_count").unwrap_or(0);

    if replayable.saturating_add(blocked) != item_count {
        failures.push(format!(
            "self_evolution_rollback_replay_apply replayable+blocked {} does not match item_count {item_count}",
            replayable.saturating_add(blocked)
        ));
    }

    match decision.as_str() {
        "ready_for_operator_apply" => {
            if !ready_for_operator_apply {
                failures.push(
                    "self_evolution_rollback_replay_apply ready decision requires ready_for_operator_apply=true"
                        .to_owned(),
                );
            }
            if !rollback_gate_admitted || !operator_approved {
                failures.push(
                    "self_evolution_rollback_replay_apply ready decision requires admitted gate and approved operator"
                        .to_owned(),
                );
            }
            if item_count == 0 || replayable != item_count || blocked != 0 {
                failures.push(
                    "self_evolution_rollback_replay_apply ready decision requires a non-empty all-replayable plan"
                        .to_owned(),
                );
            }
            for (field, count) in [
                ("review_packet_count", review_packet_count),
                ("evidence_id_count", evidence_id_count),
                ("rollback_anchor_count", rollback_anchor_count),
                ("content_digest_count", content_digest_count),
                ("source_report_schema_count", source_report_schema_count),
            ] {
                if count == 0 {
                    failures.push(format!(
                        "self_evolution_rollback_replay_apply ready decision requires {field}>0"
                    ));
                }
            }
            if blocked_reasons_count != 0 {
                failures.push(
                    "self_evolution_rollback_replay_apply ready decision must not have blocked reasons"
                        .to_owned(),
                );
            }
        }
        "hold" => {
            if ready_for_operator_apply {
                failures.push(
                    "self_evolution_rollback_replay_apply hold decision requires ready_for_operator_apply=false"
                        .to_owned(),
                );
            }
            if blocked_reasons_count == 0 {
                failures.push(
                    "self_evolution_rollback_replay_apply hold decision requires blocked reasons"
                        .to_owned(),
                );
            }
        }
        _ => failures.push(format!(
            "self_evolution_rollback_replay_apply decision {decision} is not supported"
        )),
    }

    failures
}

fn evaluate_self_evolution_rollback_replay_item(item: &str, failures: &mut Vec<String>) {
    for (name, marker) in [
        ("sequence", "\"sequence\":"),
        ("experiment_id", "\"experiment_id\":"),
        ("candidate_id", "\"candidate_id\":"),
        ("decision", "\"decision\":"),
        ("rollback_required", "\"rollback_required\":"),
        ("rollback_replayable", "\"rollback_replayable\":"),
        ("replayable", "\"replayable\":"),
        ("active_candidate", "\"active_candidate\":"),
        ("read_only", "\"read_only\":"),
        ("report_only", "\"report_only\":"),
        ("preview_only", "\"preview_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
        ("evidence_ids", "\"evidence_ids\":"),
        ("rollback_anchor_ids", "\"rollback_anchor_ids\":"),
        ("blocked_reasons", "\"blocked_reasons\":"),
        ("content_digest", "\"content_digest\":"),
    ] {
        if !item.contains(marker) {
            failures.push(format!(
                "missing self_evolution_rollback_replay item field {name}"
            ));
        }
    }

    if extract_json_usize_field(item, "sequence").unwrap_or(0) == 0 {
        failures.push("self_evolution_rollback_replay item sequence must be positive".to_owned());
    }
    for field in ["experiment_id", "candidate_id", "content_digest"] {
        let value = extract_json_string_field(item, field).unwrap_or_default();
        if value.trim().is_empty() {
            failures.push(format!(
                "self_evolution_rollback_replay item {field} is empty"
            ));
        }
    }
    let content_digest = extract_json_string_field(item, "content_digest").unwrap_or_default();
    if !content_digest.starts_with("fnv64:") {
        failures.push(
            "self_evolution_rollback_replay item content_digest must be stable fnv64".to_owned(),
        );
    }

    let replayable = extract_json_bool_field(item, "replayable");
    let blocked_reasons =
        extract_json_string_array_field(item, "blocked_reasons").unwrap_or_default();
    let evidence_ids = extract_json_string_array_field(item, "evidence_ids").unwrap_or_default();
    let rollback_anchor_ids =
        extract_json_string_array_field(item, "rollback_anchor_ids").unwrap_or_default();

    for (field, values) in [
        ("evidence_ids", &evidence_ids),
        ("rollback_anchor_ids", &rollback_anchor_ids),
        ("blocked_reasons", &blocked_reasons),
    ] {
        if values.iter().any(|value| value.trim().is_empty()) {
            failures.push(format!(
                "self_evolution_rollback_replay item {field} contains empty item"
            ));
        }
    }

    if replayable == Some(true) {
        if extract_json_string_field(item, "decision").as_deref() != Some("rollback") {
            failures.push(
                "self_evolution_rollback_replay replayable item decision must be rollback"
                    .to_owned(),
            );
        }
        require_bool(
            failures,
            item,
            "rollback_required",
            true,
            "self_evolution_rollback_replay replayable item",
        );
        require_bool(
            failures,
            item,
            "rollback_replayable",
            true,
            "self_evolution_rollback_replay replayable item",
        );
        require_bool(
            failures,
            item,
            "active_candidate",
            false,
            "self_evolution_rollback_replay replayable item",
        );
        require_bool(
            failures,
            item,
            "read_only",
            true,
            "self_evolution_rollback_replay replayable item",
        );
        require_bool(
            failures,
            item,
            "report_only",
            true,
            "self_evolution_rollback_replay replayable item",
        );
        require_bool(
            failures,
            item,
            "preview_only",
            true,
            "self_evolution_rollback_replay replayable item",
        );
        require_bool(
            failures,
            item,
            "write_allowed",
            false,
            "self_evolution_rollback_replay replayable item",
        );
        require_bool(
            failures,
            item,
            "applied",
            false,
            "self_evolution_rollback_replay replayable item",
        );
        if evidence_ids.is_empty() {
            failures.push(
                "self_evolution_rollback_replay replayable item requires evidence_ids".to_owned(),
            );
        }
        if rollback_anchor_ids.is_empty() {
            failures.push(
                "self_evolution_rollback_replay replayable item requires rollback_anchor_ids"
                    .to_owned(),
            );
        }
        if !blocked_reasons.is_empty() {
            failures.push(
                "self_evolution_rollback_replay replayable item must not have blocked reasons"
                    .to_owned(),
            );
        }
    } else if blocked_reasons.is_empty() {
        failures.push(
            "self_evolution_rollback_replay blocked item requires blocked reasons".to_owned(),
        );
    }
}

fn evaluate_review_packet(
    failures: &mut Vec<String>,
    line: &str,
    admitted_for_human_review: Option<bool>,
) {
    let Some(review_packet) = json_object_after_field(line, "review_packet") else {
        failures.push("self_evolution_admission review_packet object is missing".to_owned());
        return;
    };

    require_bool(
        failures,
        review_packet,
        "read_only",
        true,
        "self_evolution_admission review_packet",
    );
    require_bool(
        failures,
        review_packet,
        "approval_tokens_included",
        false,
        "self_evolution_admission review_packet",
    );

    let approval_review_packet_ids =
        require_string_array(failures, review_packet, "approval_review_packet_ids");
    let evidence_ids = require_string_array(failures, review_packet, "evidence_ids");
    let rollback_anchor_ids = require_string_array(failures, review_packet, "rollback_anchor_ids");
    let content_digests = require_string_array(failures, review_packet, "content_digests");
    let source_report_schemas =
        require_string_array(failures, review_packet, "source_report_schemas");

    require_count(
        failures,
        review_packet,
        "approval_review_packet_count",
        approval_review_packet_ids.len(),
    );
    require_count(
        failures,
        review_packet,
        "evidence_count",
        evidence_ids.len(),
    );
    require_count(
        failures,
        review_packet,
        "rollback_anchor_count",
        rollback_anchor_ids.len(),
    );
    require_count(
        failures,
        review_packet,
        "content_digest_count",
        content_digests.len(),
    );
    require_count(
        failures,
        review_packet,
        "source_report_schema_count",
        source_report_schemas.len(),
    );

    if admitted_for_human_review == Some(true) {
        if approval_review_packet_ids.is_empty() {
            failures.push(
                "self_evolution_admission admitted packet requires review packet ids".to_owned(),
            );
        }
        if evidence_ids.is_empty() {
            failures.push(
                "self_evolution_admission admitted packet requires review evidence ids".to_owned(),
            );
        }
        if content_digests.is_empty() {
            failures.push(
                "self_evolution_admission admitted packet requires review content digests"
                    .to_owned(),
            );
        }
        if source_report_schemas.is_empty() {
            failures.push(
                "self_evolution_admission admitted packet requires review source schemas"
                    .to_owned(),
            );
        }
    }
}

fn evaluate_rollback_replay_gate_review_packet(
    failures: &mut Vec<String>,
    line: &str,
    admitted_for_human_review: bool,
) {
    let Some(review_packet) = json_object_after_field(line, "review_packet") else {
        failures
            .push("self_evolution_rollback_replay_gate review_packet object is missing".to_owned());
        return;
    };

    require_bool(
        failures,
        review_packet,
        "read_only",
        true,
        "self_evolution_rollback_replay_gate review_packet",
    );
    require_bool(
        failures,
        review_packet,
        "approval_tokens_included",
        false,
        "self_evolution_rollback_replay_gate review_packet",
    );

    let approval_review_packet_ids =
        require_string_array(failures, review_packet, "approval_review_packet_ids");
    let evidence_ids = require_string_array(failures, review_packet, "evidence_ids");
    let rollback_anchor_ids = require_string_array(failures, review_packet, "rollback_anchor_ids");
    let content_digests = require_string_array(failures, review_packet, "content_digests");
    let source_report_schemas =
        require_string_array(failures, review_packet, "source_report_schemas");

    require_count(
        failures,
        review_packet,
        "approval_review_packet_count",
        approval_review_packet_ids.len(),
    );
    require_count(
        failures,
        review_packet,
        "evidence_count",
        evidence_ids.len(),
    );
    require_count(
        failures,
        review_packet,
        "rollback_anchor_count",
        rollback_anchor_ids.len(),
    );
    require_count(
        failures,
        review_packet,
        "content_digest_count",
        content_digests.len(),
    );
    require_count(
        failures,
        review_packet,
        "source_report_schema_count",
        source_report_schemas.len(),
    );

    for (field, values) in [
        ("approval_review_packet_ids", &approval_review_packet_ids),
        ("evidence_ids", &evidence_ids),
        ("rollback_anchor_ids", &rollback_anchor_ids),
        ("content_digests", &content_digests),
        ("source_report_schemas", &source_report_schemas),
    ] {
        if values.iter().any(|value| value.trim().is_empty()) {
            failures.push(format!(
                "self_evolution_rollback_replay_gate review_packet {field} contains empty item"
            ));
        }
    }

    if approval_review_packet_ids.is_empty() {
        failures.push(
            "self_evolution_rollback_replay_gate review_packet requires packet ids".to_owned(),
        );
    }
    if content_digests.is_empty() {
        failures.push(
            "self_evolution_rollback_replay_gate review_packet requires content digests".to_owned(),
        );
    }
    if source_report_schemas.is_empty() {
        failures.push(
            "self_evolution_rollback_replay_gate review_packet requires source schemas".to_owned(),
        );
    }
    if admitted_for_human_review {
        if evidence_ids.is_empty() {
            failures.push(
                "self_evolution_rollback_replay_gate admitted review packet requires evidence ids"
                    .to_owned(),
            );
        }
        if rollback_anchor_ids.is_empty() {
            failures.push(
                "self_evolution_rollback_replay_gate admitted review packet requires rollback anchors"
                    .to_owned(),
            );
        }
    }
}

fn evaluate_rust_check(failures: &mut Vec<String>, line: &str) {
    let Some(rust_check) = json_object_after_field(line, "rust_check") else {
        failures.push("self_evolution_admission rust_check object is missing".to_owned());
        return;
    };
    let items = extract_json_usize_field(rust_check, "items");
    let passed = extract_json_usize_field(rust_check, "passed");
    let failed = extract_json_usize_field(rust_check, "failed");
    let validation_passed = extract_json_bool_field(rust_check, "validation_passed");

    if items.is_none() || passed.is_none() || failed.is_none() || validation_passed.is_none() {
        failures.push("self_evolution_admission rust_check fields are incomplete".to_owned());
    }
    if let (Some(items), Some(passed), Some(failed)) = (items, passed, failed) {
        if passed.saturating_add(failed) > items {
            failures.push(format!(
                "self_evolution_admission rust_check passed+failed {} exceeds items {items}",
                passed.saturating_add(failed)
            ));
        }
        if validation_passed == Some(true) && (items == 0 || passed == 0 || failed > 0) {
            failures.push(
                "self_evolution_admission rust_validation_passed requires passed checks and no failures"
                    .to_owned(),
            );
        }
    }
}

fn evaluate_validation(
    failures: &mut Vec<String>,
    line: &str,
    admitted_for_human_review: Option<bool>,
) {
    let Some(validation) = json_object_after_field(line, "validation") else {
        failures.push("self_evolution_admission validation object is missing".to_owned());
        return;
    };
    let validation_passed = extract_json_bool_field(validation, "passed");
    if validation_passed.is_none() {
        failures.push("self_evolution_admission validation passed is missing".to_owned());
    }

    for lane in ["compiler", "tests", "benchmarks", "experiments"] {
        evaluate_validation_lane(failures, validation, lane, admitted_for_human_review);
    }

    if admitted_for_human_review == Some(true) && validation_passed != Some(true) {
        failures
            .push("self_evolution_admission admitted packet requires validation passed".to_owned());
    }
}

fn evaluate_validation_lane(
    failures: &mut Vec<String>,
    validation: &str,
    lane: &str,
    admitted_for_human_review: Option<bool>,
) {
    let Some(lane_object) = json_object_after_field(validation, lane) else {
        failures.push(format!(
            "self_evolution_admission validation {lane} object is missing"
        ));
        return;
    };
    let items = extract_json_usize_field(lane_object, "items");
    let passed = extract_json_usize_field(lane_object, "passed");
    let failed = extract_json_usize_field(lane_object, "failed");
    let validation_passed = extract_json_bool_field(lane_object, "validation_passed");

    if items.is_none() || passed.is_none() || failed.is_none() || validation_passed.is_none() {
        failures.push(format!(
            "self_evolution_admission validation {lane} fields are incomplete"
        ));
    }
    if let (Some(items), Some(passed), Some(failed)) = (items, passed, failed) {
        if passed.saturating_add(failed) > items {
            failures.push(format!(
                "self_evolution_admission validation {lane} passed+failed {} exceeds items {items}",
                passed.saturating_add(failed)
            ));
        }
        if admitted_for_human_review == Some(true)
            && (items == 0 || passed == 0 || failed > 0 || validation_passed != Some(true))
        {
            failures.push(format!(
                "self_evolution_admission admitted packet requires passed {lane} validation"
            ));
        }
    }
}

fn evaluate_benchmark_gate(failures: &mut Vec<String>, line: &str) {
    let Some(benchmark_gate) = json_object_after_field(line, "benchmark_gate") else {
        failures.push("self_evolution_admission benchmark_gate object is missing".to_owned());
        return;
    };
    let passed = extract_json_bool_field(benchmark_gate, "passed");
    let failures_array =
        extract_json_string_array_field(benchmark_gate, "failures").unwrap_or_default();
    match passed {
        Some(true) if !failures_array.is_empty() => failures.push(
            "self_evolution_admission benchmark_gate passed=true must not include failures"
                .to_owned(),
        ),
        Some(false) if failures_array.is_empty() => failures.push(
            "self_evolution_admission benchmark_gate passed=false requires failures".to_owned(),
        ),
        Some(_) => {}
        None => {
            failures.push("self_evolution_admission benchmark_gate passed is missing".to_owned())
        }
    }
}

fn evaluate_rollback(failures: &mut Vec<String>, line: &str) {
    let Some(rollback) = json_object_after_field(line, "rollback") else {
        failures.push("self_evolution_admission rollback object is missing".to_owned());
        return;
    };
    if extract_json_bool_field(rollback, "budget_clean").is_none() {
        failures.push("self_evolution_admission rollback budget_clean is missing".to_owned());
    }
    if extract_json_usize_field(rollback, "drift_rollbacks").is_none() {
        failures.push("self_evolution_admission rollback drift_rollbacks is missing".to_owned());
    }
}

fn evaluate_adaptive_preview(
    failures: &mut Vec<String>,
    line: &str,
    admitted_for_human_review: Option<bool>,
) {
    let Some(adaptive_preview) = json_object_after_field(line, "adaptive_preview") else {
        failures.push("self_evolution_admission adaptive_preview object is missing".to_owned());
        return;
    };
    let read_only = require_bool_value(
        failures,
        adaptive_preview,
        "read_only",
        "self_evolution_admission adaptive_preview",
    );
    let report_only = require_bool_value(
        failures,
        adaptive_preview,
        "report_only",
        "self_evolution_admission adaptive_preview",
    );
    let preview_only = require_bool_value(
        failures,
        adaptive_preview,
        "preview_only",
        "self_evolution_admission adaptive_preview",
    );
    let write_allowed = require_bool_value(
        failures,
        adaptive_preview,
        "write_allowed",
        "self_evolution_admission adaptive_preview",
    );
    let applied = require_bool_value(
        failures,
        adaptive_preview,
        "applied",
        "self_evolution_admission adaptive_preview",
    );

    let evidence_present = extract_json_bool_field(adaptive_preview, "evidence_present");
    let source_count = extract_json_usize_field(adaptive_preview, "source_count").unwrap_or(0);
    if evidence_present == Some(true) && source_count == 0 {
        failures.push(
            "self_evolution_admission adaptive_preview evidence requires source_count".to_owned(),
        );
    }
    if admitted_for_human_review == Some(true) && evidence_present != Some(true) {
        failures.push(
            "self_evolution_admission admitted packet requires adaptive preview evidence"
                .to_owned(),
        );
    }
    if admitted_for_human_review == Some(true) {
        if read_only != Some(true) {
            failures.push(
                "self_evolution_admission admitted packet requires adaptive_preview read_only=true"
                    .to_owned(),
            );
        }
        if report_only != Some(true) {
            failures.push(
                "self_evolution_admission admitted packet requires adaptive_preview report_only=true"
                    .to_owned(),
            );
        }
        if preview_only != Some(true) {
            failures.push(
                "self_evolution_admission admitted packet requires adaptive_preview preview_only=true"
                    .to_owned(),
            );
        }
        if write_allowed != Some(false) {
            failures.push(
                "self_evolution_admission admitted packet requires adaptive_preview write_allowed=false"
                    .to_owned(),
            );
        }
        if applied != Some(false) {
            failures.push(
                "self_evolution_admission admitted packet requires adaptive_preview applied=false"
                    .to_owned(),
            );
        }
    }
}

fn evaluate_writes(failures: &mut Vec<String>, line: &str) {
    let Some(writes) = json_object_after_field(line, "writes") else {
        failures.push("self_evolution_admission writes object is missing".to_owned());
        return;
    };
    for field in [
        "mutation_allowed",
        "memory_store_allowed",
        "ndkv_allowed",
        "model_weight_allowed",
        "git_allowed",
    ] {
        require_bool(
            failures,
            writes,
            field,
            false,
            "self_evolution_admission writes",
        );
    }
}

fn evaluate_telemetry(failures: &mut Vec<String>, line: &str) {
    let telemetry = extract_json_string_array_field(line, "telemetry").unwrap_or_default();
    if !telemetry
        .iter()
        .any(|entry| entry == "self_evolution_admission=true")
    {
        failures.push(
            "self_evolution_admission telemetry must include self_evolution_admission=true"
                .to_owned(),
        );
    }
}

fn require_bool(
    failures: &mut Vec<String>,
    object: &str,
    field: &str,
    expected: bool,
    context: &str,
) {
    match extract_json_bool_field(object, field) {
        Some(actual) if actual == expected => {}
        Some(actual) => failures.push(format!(
            "{context} {field}={actual} does not match required {expected}"
        )),
        None => failures.push(format!("{context} {field} is missing")),
    }
}

fn require_bool_value(
    failures: &mut Vec<String>,
    object: &str,
    field: &str,
    context: &str,
) -> Option<bool> {
    match extract_json_bool_field(object, field) {
        Some(value) => Some(value),
        None => {
            failures.push(format!("{context} {field} is missing"));
            None
        }
    }
}

fn require_string_array(failures: &mut Vec<String>, object: &str, field: &str) -> Vec<String> {
    match extract_json_string_array_field(object, field) {
        Some(items) => {
            if items.iter().any(|item| item.trim().is_empty()) {
                failures.push(format!(
                    "self_evolution_admission review_packet {field} contains empty item"
                ));
            }
            items
        }
        None => {
            failures.push(format!(
                "self_evolution_admission review_packet {field} is missing"
            ));
            Vec::new()
        }
    }
}

fn require_count(failures: &mut Vec<String>, object: &str, field: &str, expected: usize) {
    match extract_json_usize_field(object, field) {
        Some(actual) if actual == expected => {}
        Some(actual) => failures.push(format!(
            "self_evolution_admission review_packet {field}={actual} does not match array length {expected}"
        )),
        None => failures.push(format!(
            "self_evolution_admission review_packet {field} is missing"
        )),
    }
}
