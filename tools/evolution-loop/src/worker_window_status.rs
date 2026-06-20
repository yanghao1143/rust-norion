use std::fs;
use std::path::Path;

use crate::json::{
    json_array_field, json_bool_field, json_object_field, json_string, json_string_field,
    parse_json_object_array,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkerWindowStatusSummary {
    pub(crate) source_path: String,
    pub(crate) source_status_json: String,
    pub(crate) window_count: usize,
    pub(crate) paused_count: usize,
    pub(crate) polluted_count: usize,
    pub(crate) clean_room_replacement_count: usize,
    pub(crate) replacement_required_count: usize,
    pub(crate) blocked_original_count: usize,
    pub(crate) side_effects_allowed: Option<bool>,
}

pub(crate) fn load_status(
    path: Option<&Path>,
) -> Result<Option<WorkerWindowStatusSummary>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "read worker window status JSON {} failed: {error}",
            path.display()
        )
    })?;
    parse_status_json(&text, &path.display().to_string()).map(Some)
}

pub(crate) fn option_status_json(status: Option<&WorkerWindowStatusSummary>) -> String {
    match status {
        Some(status) => status.report_json(),
        None => {
            "{\"schema\":\"worker_window_replacement_report_v1\",\"consumer_surface\":\"clean_room_worker_window_replacement_status\",\"read_only\":true,\"status_loaded\":false,\"source\":\"missing\",\"source_path\":null,\"source_status\":null,\"evidence_map\":{\"window_count\":0,\"paused_count\":0,\"polluted_count\":0,\"clean_room_replacement_count\":0,\"replacement_required_count\":0,\"blocked_original_count\":0,\"side_effects_allowed\":null},\"side_effects\":{\"starts_clean_room_replacement\":false,\"mutates_worker_window_status\":false,\"starts_daemon\":false,\"stops_daemon\":false,\"touches_remote\":false,\"downloads_model\":false,\"warms_model_cache\":false,\"sends_prompt\":false,\"starts_stream\":false,\"replays_prompt\":false}}".to_owned()
        }
    }
}

impl WorkerWindowStatusSummary {
    fn report_json(&self) -> String {
        format!(
            "{{\"schema\":\"worker_window_replacement_report_v1\",\"consumer_surface\":\"clean_room_worker_window_replacement_status\",\"read_only\":true,\"status_loaded\":true,\"source\":\"external_worker_window_status_json\",\"source_path\":{},\"source_status\":{},\"evidence_map\":{{\"window_count\":{},\"paused_count\":{},\"polluted_count\":{},\"clean_room_replacement_count\":{},\"replacement_required_count\":{},\"blocked_original_count\":{},\"side_effects_allowed\":{}}},\"side_effects\":{{\"starts_clean_room_replacement\":false,\"mutates_worker_window_status\":false,\"starts_daemon\":false,\"stops_daemon\":false,\"touches_remote\":false,\"downloads_model\":false,\"warms_model_cache\":false,\"sends_prompt\":false,\"starts_stream\":false,\"replays_prompt\":false}}}}",
            json_string(&self.source_path),
            self.source_status_json,
            self.window_count,
            self.paused_count,
            self.polluted_count,
            self.clean_room_replacement_count,
            self.replacement_required_count,
            self.blocked_original_count,
            option_bool_json(self.side_effects_allowed)
        )
    }
}

fn parse_status_json(text: &str, source_path: &str) -> Result<WorkerWindowStatusSummary, String> {
    let wrapped = format!("{{\"root\":{}}}", text.trim());
    let source_status_json = json_object_field(&wrapped, "root")
        .ok_or_else(|| "worker window status JSON must be an object".to_owned())?;
    let windows = json_array_field(&source_status_json, "windows")
        .map(|array| parse_json_object_array(&array))
        .unwrap_or_default();

    let mut paused_count = 0usize;
    let mut polluted_count = 0usize;
    let mut clean_room_replacement_count = 0usize;
    let mut replacement_required_count = 0usize;
    let mut blocked_original_count = 0usize;

    for window in &windows {
        let status = json_string_field(window, "status").unwrap_or_default();
        if status == "paused" {
            paused_count += 1;
        }
        if status == "polluted" || json_bool_field(window, "polluted") == Some(true) {
            polluted_count += 1;
        }
        if status == "clean-room-replacement"
            || json_bool_field(window, "clean_room_replacement") == Some(true)
        {
            clean_room_replacement_count += 1;
        }
        if json_bool_field(window, "clean_room_replacement_required") == Some(true) {
            replacement_required_count += 1;
        }
        if json_bool_field(window, "original_window_blocks_assignment") == Some(true)
            || json_bool_field(window, "assignment_allowed") == Some(false)
        {
            blocked_original_count += 1;
        }
    }

    Ok(WorkerWindowStatusSummary {
        source_path: source_path.to_owned(),
        source_status_json,
        window_count: windows.len(),
        paused_count,
        polluted_count,
        clean_room_replacement_count,
        replacement_required_count,
        blocked_original_count,
        side_effects_allowed: json_bool_field(text, "side_effects_allowed"),
    })
}

fn option_bool_json(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_window_status_counts_paused_polluted_and_replacements() {
        let summary = parse_status_json(
            r#"{
  "schema": "worker_window_status_v1",
  "side_effects_allowed": false,
  "windows": [
    {
      "window_id": "r20-eval-test",
      "status": "paused",
      "polluted": true,
      "clean_room_replacement_required": true,
      "original_window_blocks_assignment": true
    },
    {
      "window_id": "r21-eval-test",
      "status": "clean-room-replacement",
      "clean_room_replacement": true,
      "assignment_allowed": true
    },
    {
      "window_id": "r21-service-cli",
      "status": "clean-room-replacement",
      "assignment_allowed": true
    }
  ]
}"#,
            "worker-window-status.json",
        )
        .unwrap();

        assert_eq!(summary.window_count, 3);
        assert_eq!(summary.paused_count, 1);
        assert_eq!(summary.polluted_count, 1);
        assert_eq!(summary.clean_room_replacement_count, 2);
        assert_eq!(summary.replacement_required_count, 1);
        assert_eq!(summary.blocked_original_count, 1);
        assert_eq!(summary.side_effects_allowed, Some(false));

        let json = summary.report_json();
        assert!(json.contains("\"schema\":\"worker_window_replacement_report_v1\""));
        assert!(json.contains("\"status_loaded\":true"));
        assert!(json.contains("\"paused_count\":1"));
        assert!(json.contains("\"polluted_count\":1"));
        assert!(json.contains("\"clean_room_replacement_count\":2"));
        assert!(json.contains("\"starts_clean_room_replacement\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
    }

    #[test]
    fn missing_worker_window_status_is_report_only_null_surface() {
        let json = option_status_json(None);

        assert!(json.contains("\"status_loaded\":false"));
        assert!(json.contains("\"source_status\":null"));
        assert!(json.contains("\"side_effects_allowed\":null"));
        assert!(json.contains("\"mutates_worker_window_status\":false"));
    }
}
