use super::json::{
    json_array_field, json_bool_field, json_number_field, json_object_field, json_object_items,
    json_string_field,
};

const DEFAULT_REPAIR_LIMIT: usize = 3;

pub(crate) fn experience_repair_summary(body: &str) -> Result<String, String> {
    ensure_ok(body, "experience repair")?;
    let mut lines = vec!["Noiron experience repair dry-run".to_owned()];
    push_field_line(
        &mut lines,
        "experience_file",
        json_string_field(body, "experience_file"),
    );
    push_field_line(
        &mut lines,
        "applied",
        json_bool_field(body, "applied").map(|value| value.to_string()),
    );
    push_field_line(
        &mut lines,
        "backup_file",
        json_string_field(body, "backup_file"),
    );

    if let Some(plan) = json_object_field(body, "plan") {
        lines.push(repair_plan_line(plan));
        if let Some(projection) = json_object_field(plan, "projected_hygiene_after_repair") {
            lines.push(projected_hygiene_line(projection));
        }
        push_listed_repairs(&mut lines, plan);
    } else {
        lines.push("plan=none".to_owned());
    }

    Ok(lines.join("\n"))
}

fn ensure_ok(body: &str, label: &str) -> Result<(), String> {
    if json_bool_field(body, "ok") == Some(false) {
        let error = json_string_field(body, "error").unwrap_or_else(|| "unknown".to_owned());
        return Err(format!("{label} failed: {error}"));
    }
    Ok(())
}

fn repair_plan_line(plan: &str) -> String {
    let total = json_number_field(plan, "total_records").unwrap_or_else(|| "unknown".to_owned());
    let legacy =
        json_number_field(plan, "legacy_metadata_lessons").unwrap_or_else(|| "unknown".to_owned());
    let repairable = json_number_field(plan, "repairable_legacy_metadata_lessons")
        .unwrap_or_else(|| "unknown".to_owned());
    let remaining = json_number_field(plan, "remaining_legacy_metadata_lessons_after_repair")
        .unwrap_or_else(|| "unknown".to_owned());
    let remaining_watch = json_number_field(plan, "remaining_watch_after_repair")
        .unwrap_or_else(|| "unknown".to_owned());
    let remaining_quarantine =
        json_number_field(plan, "remaining_quarantine_candidates_after_repair")
            .unwrap_or_else(|| "unknown".to_owned());
    let index_noisy =
        json_number_field(plan, "index_noisy_records").unwrap_or_else(|| "unknown".to_owned());
    let index_duplicates =
        json_number_field(plan, "index_duplicate_outputs").unwrap_or_else(|| "unknown".to_owned());
    let repairable_index =
        json_number_field(plan, "repairable_index_records").unwrap_or_else(|| "unknown".to_owned());
    let skipped_quarantine = json_number_field(plan, "skipped_quarantine_candidates")
        .unwrap_or_else(|| "unknown".to_owned());
    let skipped_missing = json_number_field(plan, "skipped_missing_clean_gist")
        .unwrap_or_else(|| "unknown".to_owned());

    format!(
        "plan: total_records={total} legacy_metadata_lessons={legacy} repairable_legacy_metadata_lessons={repairable} index_noisy_records={index_noisy} index_duplicate_outputs={index_duplicates} repairable_index_records={repairable_index} remaining_legacy_metadata_lessons_after_repair={remaining} remaining_watch_after_repair={remaining_watch} remaining_quarantine_candidates_after_repair={remaining_quarantine} skipped_quarantine_candidates={skipped_quarantine} skipped_missing_clean_gist={skipped_missing}"
    )
}

fn projected_hygiene_line(projection: &str) -> String {
    let findings =
        json_number_field(projection, "findings").unwrap_or_else(|| "unknown".to_owned());
    let watch = json_number_field(projection, "watch").unwrap_or_else(|| "unknown".to_owned());
    let quarantine = json_number_field(projection, "quarantine_candidates")
        .unwrap_or_else(|| "unknown".to_owned());
    let legacy = json_number_field(projection, "legacy_metadata_lessons")
        .unwrap_or_else(|| "unknown".to_owned());
    let missing = json_number_field(projection, "legacy_metadata_without_clean_gist")
        .unwrap_or_else(|| "unknown".to_owned());
    let index_quality = json_number_field(projection, "index_quality_score")
        .unwrap_or_else(|| "unknown".to_owned());
    let index_noisy = json_number_field(projection, "index_noisy_records")
        .unwrap_or_else(|| "unknown".to_owned());
    let index_duplicates = json_number_field(projection, "index_duplicate_outputs")
        .unwrap_or_else(|| "unknown".to_owned());
    let index_ready = json_bool_field(projection, "index_retrieval_ready")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned());
    let index_risk =
        json_string_field(projection, "index_risk_level").unwrap_or_else(|| "unknown".to_owned());

    format!(
        "projected_hygiene_after_repair: findings={findings} watch={watch} quarantine_candidates={quarantine} legacy_metadata_lessons={legacy} legacy_metadata_without_clean_gist={missing} index_quality_score={index_quality} index_noisy_records={index_noisy} index_duplicate_outputs={index_duplicates} index_retrieval_ready={index_ready} index_risk_level={index_risk}"
    )
}

fn push_listed_repairs(lines: &mut Vec<String>, plan: &str) {
    let Some(repairs) = json_array_field(plan, "listed_repairs") else {
        return;
    };
    let items = json_object_items(repairs);
    if items.is_empty() {
        return;
    }
    lines.push(format!("listed_repairs={}", items.len()));
    for item in items.into_iter().take(DEFAULT_REPAIR_LIMIT) {
        let id = json_number_field(item, "experience_id").unwrap_or_else(|| "unknown".to_owned());
        let action = json_string_field(item, "action").unwrap_or_else(|| "unknown".to_owned());
        let proposed = compact_preview(
            &json_string_field(item, "proposed_lesson_preview")
                .unwrap_or_else(|| "unknown".to_owned()),
        );
        let gist = compact_preview(
            &json_string_field(item, "source_gist_preview").unwrap_or_else(|| "unknown".to_owned()),
        );
        lines.push(format!(
            "repair id={id} action={action} proposed={proposed} source_gist={gist}"
        ));
    }
}

fn push_field_line(lines: &mut Vec<String>, name: &str, value: Option<String>) {
    if let Some(value) = value {
        lines.push(format!("{name}={value}"));
    }
}

fn compact_preview(value: &str) -> String {
    const MAX_CHARS: usize = 160;
    let compacted = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut preview = compacted.chars().take(MAX_CHARS).collect::<String>();
    if compacted.chars().count() > MAX_CHARS {
        preview.push_str("...");
    }
    preview
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_experience_repair_dry_run() {
        let summary = experience_repair_summary(
            "{\"ok\":true,\"request_id\":5,\"experience_file\":\"noiron-experience.ndkv\",\"applied\":false,\"backup_file\":null,\"plan\":{\"total_records\":863,\"legacy_metadata_lessons\":860,\"repairable_legacy_metadata_lessons\":828,\"index_noisy_records\":2,\"index_duplicate_outputs\":1,\"repairable_index_records\":1,\"remaining_legacy_metadata_lessons_after_repair\":32,\"remaining_watch_after_repair\":28,\"remaining_quarantine_candidates_after_repair\":4,\"skipped_quarantine_candidates\":4,\"skipped_missing_clean_gist\":28,\"projected_hygiene_after_repair\":{\"total_records\":863,\"findings\":32,\"watch\":28,\"quarantine_candidates\":4,\"legacy_metadata_lessons\":32,\"legacy_metadata_without_clean_gist\":29,\"index_quality_score\":0.88,\"index_noisy_records\":0,\"index_duplicate_outputs\":0,\"index_retrieval_ready\":true,\"index_risk_level\":\"watch\"},\"listed_repairs\":[{\"experience_id\":21,\"action\":\"reuse_response\",\"source\":\"clean_gist\",\"old_lesson_preview\":\"accepted_pattern quality=0.9\",\"proposed_lesson_preview\":\"reuse_response: Rust loop\",\"source_gist_preview\":\"Rust loop\"}]}}",
        )
        .unwrap();

        assert!(summary.contains("Noiron experience repair dry-run"));
        assert!(summary.contains("experience_file=noiron-experience.ndkv"));
        assert!(summary.contains("applied=false"));
        assert!(summary.contains("repairable_legacy_metadata_lessons=828"));
        assert!(summary.contains("index_noisy_records=2"));
        assert!(summary.contains("index_duplicate_outputs=1"));
        assert!(summary.contains("repairable_index_records=1"));
        assert!(summary.contains("remaining_legacy_metadata_lessons_after_repair=32"));
        assert!(summary.contains("projected_hygiene_after_repair: findings=32"));
        assert!(summary.contains("index_quality_score=0.88"));
        assert!(summary.contains("index_retrieval_ready=true"));
        assert!(summary.contains("index_risk_level=watch"));
        assert!(summary.contains("repair id=21"));
        assert!(summary.contains("reuse_response: Rust loop"));
    }

    #[test]
    fn compacts_long_repair_previews() {
        let long_preview = "reuse_response: ".to_owned() + &"Rust loop ".repeat(80);
        let body = format!(
            "{{\"ok\":true,\"experience_file\":\"noiron-experience.ndkv\",\"applied\":false,\"plan\":{{\"total_records\":1,\"legacy_metadata_lessons\":1,\"repairable_legacy_metadata_lessons\":1,\"remaining_legacy_metadata_lessons_after_repair\":0,\"remaining_watch_after_repair\":0,\"remaining_quarantine_candidates_after_repair\":0,\"skipped_quarantine_candidates\":0,\"skipped_missing_clean_gist\":0,\"listed_repairs\":[{{\"experience_id\":21,\"action\":\"reuse_response\",\"proposed_lesson_preview\":\"{long_preview}\",\"source_gist_preview\":\"{long_preview}\"}}]}}}}"
        );

        let summary = experience_repair_summary(&body).unwrap();

        assert!(summary.contains("repair id=21"));
        assert!(summary.contains("..."));
        assert!(summary.lines().all(|line| line.len() < 420));
    }

    #[test]
    fn repair_summary_ignores_plan_key_inside_string_values() {
        let summary = experience_repair_summary(
            r#"{"ok":true,"note":"\"plan\":{\"total_records\":999,\"listed_repairs\":[{\"experience_id\":999}]},","experience_file":"noiron-experience.ndkv","applied":false,"plan":{"total_records":2,"legacy_metadata_lessons":1,"repairable_legacy_metadata_lessons":1,"remaining_legacy_metadata_lessons_after_repair":0,"remaining_watch_after_repair":0,"remaining_quarantine_candidates_after_repair":0,"skipped_quarantine_candidates":0,"skipped_missing_clean_gist":0,"listed_repairs":[]}}"#,
        )
        .unwrap();

        assert!(summary.contains("experience_file=noiron-experience.ndkv"));
        assert!(summary.contains("plan: total_records=2"));
        assert!(!summary.contains("999"));
        assert!(!summary.contains("repair id=999"));
    }
}
