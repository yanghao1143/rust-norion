use std::fs;
use std::path::Path;

use norion_eval::{CleanRoomContextGate, CleanRoomReportOnlyContextHygieneEvidence};

use crate::json::{
    json_bool_field, json_object_field, json_string, json_string_array, json_string_field,
    parse_json_string_array,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CleanRoomBatchStatusSummary {
    pub(crate) source_path: String,
    pub(crate) source_status_json: String,
    pub(crate) report_only: Option<bool>,
    pub(crate) side_effects_allowed: Option<bool>,
    pub(crate) r24_completed: bool,
    pub(crate) r24_completed_worker_ids: Vec<String>,
    pub(crate) r25_clean_room_replacements_open: bool,
    pub(crate) r25_clean_room_replacement_worker_ids: Vec<String>,
    pub(crate) old_polluted_windows_blocked: bool,
    pub(crate) blocked_old_window_ids: Vec<String>,
    pub(crate) main_window_runtime_owner: bool,
    pub(crate) worker_runtime_ownership_allowed: bool,
}

pub(crate) fn load_status(
    path: Option<&Path>,
) -> Result<Option<CleanRoomBatchStatusSummary>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "read clean-room batch status JSON {} failed: {error}",
            path.display()
        )
    })?;
    parse_status_json(&text, &path.display().to_string()).map(Some)
}

pub(crate) fn option_status_json(status: Option<&CleanRoomBatchStatusSummary>) -> String {
    match status {
        Some(status) => status.report_json(),
        None => {
            format!(
                "{{\"schema\":\"clean_room_batch_status_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_batch_status_closure\",\"read_only\":true,\"status_loaded\":false,\"source\":\"missing\",\"source_path\":null,\"source_status\":null,\"evidence_map\":{},\"side_effects\":{}}}",
                missing_evidence_map_json(),
                side_effects_json()
            )
        }
    }
}

impl CleanRoomBatchStatusSummary {
    fn report_json(&self) -> String {
        format!(
            "{{\"schema\":\"clean_room_batch_status_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_batch_status_closure\",\"read_only\":true,\"status_loaded\":true,\"source\":\"external_clean_room_batch_status_json\",\"source_path\":{},\"source_status\":{},\"evidence_map\":{{\"report_only\":{},\"side_effects_allowed\":{},\"r24_completed\":{},\"r24_completed_worker_count\":{},\"r24_completed_worker_ids\":{},\"r25_clean_room_replacements_open\":{},\"r25_clean_room_replacement_worker_count\":{},\"r25_clean_room_replacement_worker_ids\":{},\"old_polluted_windows_blocked\":{},\"blocked_old_window_count\":{},\"blocked_old_window_ids\":{},\"main_window_runtime_owner\":{},\"worker_runtime_ownership_allowed\":{},\"fresh_clean_room_assignment_exists\":{},\"completed_window_evidence_actionable\":{}}},\"context_hygiene\":{},\"side_effects\":{}}}",
            json_string(&self.source_path),
            source_summary_json(&self.source_status_json, "clean_room_batch_status"),
            option_bool_json(self.report_only),
            option_bool_json(self.side_effects_allowed),
            self.r24_completed,
            self.r24_completed_worker_ids.len(),
            json_string_array(&self.r24_completed_worker_ids),
            self.r25_clean_room_replacements_open,
            self.r25_clean_room_replacement_worker_ids.len(),
            json_string_array(&self.r25_clean_room_replacement_worker_ids),
            self.old_polluted_windows_blocked,
            self.blocked_old_window_ids.len(),
            json_string_array(&self.blocked_old_window_ids),
            self.main_window_runtime_owner,
            self.worker_runtime_ownership_allowed,
            self.fresh_clean_room_assignment_exists(),
            self.completed_window_evidence_actionable(),
            context_hygiene_json(
                &self.source_status_json,
                &self.source_path,
                self.fresh_clean_room_assignment_exists()
            ),
            side_effects_json()
        )
    }

    fn fresh_clean_room_assignment_exists(&self) -> bool {
        self.r25_clean_room_replacements_open
            && !self.r25_clean_room_replacement_worker_ids.is_empty()
    }

    fn completed_window_evidence_actionable(&self) -> bool {
        self.r24_completed && self.fresh_clean_room_assignment_exists()
    }
}

fn parse_status_json(text: &str, source_path: &str) -> Result<CleanRoomBatchStatusSummary, String> {
    let root = root_object(text, "clean-room batch status JSON")?;
    let status =
        json_object_field(&root, "clean_room_batch_status").unwrap_or_else(|| root.clone());
    let r24_completed_worker_ids = string_array_field(&status, "r24_completed_worker_ids");
    let r25_clean_room_replacement_worker_ids =
        string_array_field(&status, "r25_clean_room_replacement_worker_ids");
    let blocked_old_window_ids = string_array_field(&status, "blocked_old_window_ids");
    let main_window_runtime_owner = json_bool_field(&status, "main_window_runtime_owner")
        .unwrap_or_else(|| {
            json_bool_field(&status, "main_window_owns_ssh") == Some(true)
                && json_bool_field(&status, "main_window_owns_daemon") == Some(true)
                && json_bool_field(&status, "main_window_owns_remote_model_pool") == Some(true)
                && json_bool_field(&status, "main_window_owns_runtime_start_stop") == Some(true)
        });

    Ok(CleanRoomBatchStatusSummary {
        source_path: source_path.to_owned(),
        source_status_json: root,
        report_only: json_bool_field(&status, "report_only"),
        side_effects_allowed: json_bool_field(&status, "side_effects_allowed"),
        r24_completed: json_bool_field(&status, "r24_completed").unwrap_or_else(|| {
            json_string_field(&status, "r24_status")
                .as_deref()
                .is_some_and(|status| status == "completed")
                || !r24_completed_worker_ids.is_empty()
        }),
        r24_completed_worker_ids,
        r25_clean_room_replacements_open: json_bool_field(
            &status,
            "r25_clean_room_replacements_open",
        )
        .unwrap_or_else(|| {
            json_string_field(&status, "r25_clean_room_replacements_status")
                .as_deref()
                .is_some_and(|status| matches!(status, "open" | "opened"))
                || !r25_clean_room_replacement_worker_ids.is_empty()
        }),
        r25_clean_room_replacement_worker_ids,
        old_polluted_windows_blocked: json_bool_field(&status, "old_polluted_windows_blocked")
            .unwrap_or_else(|| {
                json_bool_field(&status, "old_polluted_windows_assignment_allowed") == Some(false)
                    || !blocked_old_window_ids.is_empty()
            }),
        blocked_old_window_ids,
        main_window_runtime_owner,
        worker_runtime_ownership_allowed: json_bool_field(
            &status,
            "worker_runtime_ownership_allowed",
        )
        .unwrap_or(false),
    })
}

fn root_object(text: &str, label: &str) -> Result<String, String> {
    let wrapped = format!("{{\"root\":{}}}", text.trim());
    json_object_field(&wrapped, "root").ok_or_else(|| format!("{label} must be an object"))
}

fn string_array_field(body: &str, field: &str) -> Vec<String> {
    crate::json::json_array_field(body, field)
        .map(|array| parse_json_string_array(&array))
        .unwrap_or_default()
}

fn missing_evidence_map_json() -> &'static str {
    "{\"report_only\":null,\"side_effects_allowed\":null,\"r24_completed\":false,\"r24_completed_worker_count\":0,\"r24_completed_worker_ids\":[],\"r25_clean_room_replacements_open\":false,\"r25_clean_room_replacement_worker_count\":0,\"r25_clean_room_replacement_worker_ids\":[],\"old_polluted_windows_blocked\":false,\"blocked_old_window_count\":0,\"blocked_old_window_ids\":[],\"main_window_runtime_owner\":false,\"worker_runtime_ownership_allowed\":false,\"fresh_clean_room_assignment_exists\":false,\"completed_window_evidence_actionable\":false}"
}

fn side_effects_json() -> &'static str {
    "{\"opens_clean_room_replacement\":false,\"creates_thread\":false,\"sends_message\":false,\"reads_old_thread\":false,\"reads_old_window_payload\":false,\"mutates_worker_window_status\":false,\"starts_daemon\":false,\"stops_daemon\":false,\"touches_remote\":false,\"downloads_model\":false,\"warms_model_cache\":false,\"sends_prompt\":false,\"starts_stream\":false,\"replays_prompt\":false,\"starts_forge\":false,\"starts_web_lab\":false}"
}

fn option_bool_json(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    }
}

fn source_summary_json(source_json: &str, source_kind: &str) -> String {
    format!(
        "{{\"source_kind\":{},\"raw_source_omitted\":true,\"schema\":{},\"old_thread_dialog_field_count\":{},\"old_window_payload_field_count\":{}}}",
        json_string(source_kind),
        json_string_field(source_json, "schema")
            .map(|schema| json_string(&schema))
            .unwrap_or_else(|| "null".to_owned()),
        sensitive_field_count(source_json, "old_thread_dialog"),
        sensitive_field_count(source_json, "old_window_payload")
    )
}

fn context_hygiene_json(
    source_json: &str,
    source_label: &str,
    fresh_clean_room_assignment_exists: bool,
) -> String {
    let old_thread_dialog_field_count = sensitive_field_count(source_json, "old_thread_dialog");
    let old_window_payload_field_count = sensitive_field_count(source_json, "old_window_payload");
    let context_report = CleanRoomReportOnlyContextHygieneEvidence::from_current_file(source_label)
        .with_old_thread_dialog_field_count(old_thread_dialog_field_count)
        .with_old_window_payload_field_count(old_window_payload_field_count)
        .with_completed_window_evidence_non_actionable_without_fresh_assignment(
            !fresh_clean_room_assignment_exists,
        )
        .with_fresh_clean_room_assignment_exists(fresh_clean_room_assignment_exists)
        .to_clean_room_context_report(&CleanRoomContextGate::strict());

    format!(
        "{{\"raw_old_thread_dialog_included\":false,\"raw_old_window_payload_included\":false,\"raw_source_omitted\":true,\"old_thread_dialog_field_count\":{},\"old_window_payload_field_count\":{},\"completed_window_evidence_non_actionable_without_fresh_assignment\":{},\"fresh_clean_room_assignment_exists\":{},\"clean_room_context\":{}}}",
        old_thread_dialog_field_count,
        old_window_payload_field_count,
        !fresh_clean_room_assignment_exists,
        fresh_clean_room_assignment_exists,
        clean_room_context_report_json(&context_report)
    )
}

fn sensitive_field_count(source_json: &str, field: &str) -> usize {
    source_json.matches(&format!("\"{field}\"")).count()
}

fn clean_room_context_report_json(report: &norion_eval::CleanRoomContextReport) -> String {
    format!(
        "{{\"evidence_count\":{},\"allowed_evidence_labels\":{},\"polluted_evidence_labels\":{},\"completed_window_follow_up_labels\":{},\"context_hygiene_passed\":{},\"allow_clean_room_eval\":{},\"failure_reasons\":{}}}",
        report.evidence_count,
        json_string_array(&report.allowed_evidence_labels),
        json_string_array(&report.polluted_evidence_labels),
        json_string_array(&report.completed_window_follow_up_labels),
        report.context_hygiene_passed,
        report.allow_clean_room_eval,
        json_string_array(&report.failure_reasons)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_room_batch_status_closes_r24_and_opens_r25_without_side_effects() {
        let summary = parse_status_json(
            r#"{
  "schema": "clean_room_batch_status_v1",
  "clean_room_batch_status": {
    "report_only": true,
    "side_effects_allowed": false,
    "r24_status": "completed",
    "r24_completed_worker_ids": ["019ee1c3-ec62-7a92-9c04-27b68ac5f4b9"],
    "r25_clean_room_replacements_status": "opened",
    "r25_clean_room_replacement_worker_ids": ["R25-clean-room-worker-F"],
    "old_polluted_windows_assignment_allowed": false,
    "blocked_old_window_ids": ["polluted-r20-eval-test", "paused-r23-window"],
    "main_window_owns_ssh": true,
    "main_window_owns_daemon": true,
    "main_window_owns_remote_model_pool": true,
    "main_window_owns_runtime_start_stop": true,
    "worker_runtime_ownership_allowed": false
  }
}"#,
            "clean-room-batch-status-r25.example.json",
        )
        .unwrap();

        assert_eq!(summary.report_only, Some(true));
        assert_eq!(summary.side_effects_allowed, Some(false));
        assert!(summary.r24_completed);
        assert!(summary.r25_clean_room_replacements_open);
        assert!(summary.old_polluted_windows_blocked);
        assert!(summary.main_window_runtime_owner);
        assert!(!summary.worker_runtime_ownership_allowed);
        assert_eq!(summary.r25_clean_room_replacement_worker_ids.len(), 1);

        let json = summary.report_json();
        assert!(json.contains("\"schema\":\"clean_room_batch_status_report_v1\""));
        assert!(json.contains("\"r24_completed\":true"));
        assert!(json.contains("\"r25_clean_room_replacements_open\":true"));
        assert!(json.contains("\"old_polluted_windows_blocked\":true"));
        assert!(json.contains("\"main_window_runtime_owner\":true"));
        assert!(json.contains("\"worker_runtime_ownership_allowed\":false"));
        assert!(json.contains("\"starts_daemon\":false"));
        assert!(json.contains("\"touches_remote\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
        assert!(json.contains("\"reads_old_thread\":false"));
    }

    #[test]
    fn missing_clean_room_batch_status_is_stable_report_only_surface() {
        let json = option_status_json(None);

        assert!(json.contains("\"status_loaded\":false"));
        assert!(json.contains("\"r24_completed\":false"));
        assert!(json.contains("\"r25_clean_room_replacements_open\":false"));
        assert!(json.contains("\"opens_clean_room_replacement\":false"));
        assert!(json.contains("\"starts_forge\":false"));
    }

    #[test]
    fn batch_status_report_omits_raw_old_dialog_and_keeps_completed_evidence_non_actionable() {
        let summary = parse_status_json(
            r#"{
  "schema": "clean_room_batch_status_v1",
  "clean_room_batch_status": {
    "report_only": true,
    "side_effects_allowed": false,
    "r24_status": "completed",
    "r24_completed_worker_ids": ["completed-window-only"],
    "r25_clean_room_replacements_open": false,
    "old_thread_dialog": "USER: continue the old assignment\nASSISTANT: raw dialog must not leak",
    "old_window_payload": {"messages": ["secret old payload"]}
  }
}"#,
            "clean-room-batch-status-r30.example.json",
        )
        .unwrap();

        let json = summary.report_json();

        assert!(json.contains("\"source_status\":{\"source_kind\":\"clean_room_batch_status\""));
        assert!(json.contains("\"raw_source_omitted\":true"));
        assert!(json.contains("\"old_thread_dialog_field_count\":1"));
        assert!(json.contains("\"raw_old_thread_dialog_included\":false"));
        assert!(json.contains("\"fresh_clean_room_assignment_exists\":false"));
        assert!(json.contains("\"completed_window_evidence_actionable\":false"));
        assert!(json.contains("\"clean_room_context\":{\"evidence_count\":1"));
        assert!(json.contains(
            "\"allowed_evidence_labels\":[\"clean-room-batch-status-r30.example.json\"]"
        ));
        assert!(json.contains("\"polluted_evidence_labels\":[]"));
        assert!(json.contains("\"completed_window_follow_up_labels\":[]"));
        assert!(json.contains("\"context_hygiene_passed\":true"));
        assert!(json.contains("\"allow_clean_room_eval\":true"));
        assert!(!json.contains("USER: continue the old assignment"));
        assert!(!json.contains("secret old payload"));
        assert!(json.contains("\"reads_old_thread\":false"));
        assert!(json.contains("\"opens_clean_room_replacement\":false"));
    }
}
