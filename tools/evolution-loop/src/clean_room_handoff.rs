use std::fs;
use std::path::Path;

use norion_eval::{CleanRoomContextGate, CleanRoomReportOnlyContextHygieneEvidence};

use crate::json::{
    json_bool_field, json_object_field, json_string, json_string_array, json_string_field,
    json_u64_field, parse_json_string_array,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CleanRoomHandoffSummary {
    pub(crate) memory_startup_admission: Option<MemoryStartupAdmissionSummary>,
    pub(crate) agent_replacement_plan: Option<AgentCleanRoomReplacementPlanSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryStartupAdmissionSummary {
    pub(crate) source_path: String,
    pub(crate) source_json: String,
    pub(crate) read_only_contract: Option<bool>,
    pub(crate) read_only_review_required: Option<bool>,
    pub(crate) index_quality_blocker_count: usize,
    pub(crate) index_quality_warning_count: usize,
    pub(crate) index_operation_count: usize,
    pub(crate) index_refresh_count: usize,
    pub(crate) context_rot_risk_count: usize,
    pub(crate) admission_decision_count: usize,
    pub(crate) admission_accepted_count: usize,
    pub(crate) admission_risk_rejection_count: usize,
    pub(crate) migration_live_store_targeted_count: usize,
    pub(crate) adapter_live_write_count: usize,
    pub(crate) live_write_phase_request_count: usize,
    pub(crate) live_store_mutation_requested: bool,
    pub(crate) store_mutation_count: usize,
    pub(crate) ndkv_write_allowed: bool,
    pub(crate) helper_prose_line_count: usize,
    pub(crate) non_contract_line_count: usize,
    pub(crate) admission_expanded_by_non_contract_evidence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentCleanRoomReplacementPlanSummary {
    pub(crate) source_path: String,
    pub(crate) source_json: String,
    pub(crate) report_only: Option<bool>,
    pub(crate) pure_data_only: Option<bool>,
    pub(crate) side_effects_allowed: Option<bool>,
    pub(crate) starts_thread: Option<bool>,
    pub(crate) sends_message: Option<bool>,
    pub(crate) reads_old_window_payload: Option<bool>,
    pub(crate) original_window_follow_up_assignment_allowed: Option<bool>,
    pub(crate) clean_room_replacement_plan_required: Option<bool>,
    pub(crate) clean_room_replacement_available: Option<bool>,
    pub(crate) replacement_prompt_ready: Option<bool>,
    pub(crate) follow_up_tasks_only_in_replacement_prompt: Option<bool>,
    pub(crate) task_ids: Vec<String>,
    pub(crate) evidence_result_ids: Vec<String>,
    pub(crate) reason_codes: Vec<String>,
}

pub(crate) fn load_status(
    memory_path: Option<&Path>,
    agent_path: Option<&Path>,
) -> Result<Option<CleanRoomHandoffSummary>, String> {
    let memory_startup_admission = memory_path.map(load_memory_startup_admission).transpose()?;
    let agent_replacement_plan = agent_path.map(load_agent_replacement_plan).transpose()?;

    if memory_startup_admission.is_none() && agent_replacement_plan.is_none() {
        Ok(None)
    } else {
        Ok(Some(CleanRoomHandoffSummary {
            memory_startup_admission,
            agent_replacement_plan,
        }))
    }
}

pub(crate) fn option_status_json(status: Option<&CleanRoomHandoffSummary>) -> String {
    match status {
        Some(status) => status.report_json(),
        None => format!(
            "{{\"schema\":\"clean_room_handoff_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_clean_room_handoff\",\"read_only\":true,\"memory_startup_admission\":{},\"agent_clean_room_replacement_plan\":{},\"side_effects\":{}}}",
            missing_memory_json(),
            missing_agent_json(),
            side_effects_json()
        ),
    }
}

impl CleanRoomHandoffSummary {
    fn report_json(&self) -> String {
        format!(
            "{{\"schema\":\"clean_room_handoff_report_v1\",\"consumer_surface\":\"evolution_loop_report_only_clean_room_handoff\",\"read_only\":true,\"memory_startup_admission\":{},\"agent_clean_room_replacement_plan\":{},\"side_effects\":{}}}",
            self.memory_startup_admission
                .as_ref()
                .map(MemoryStartupAdmissionSummary::report_json)
                .unwrap_or_else(missing_memory_json),
            self.agent_replacement_plan
                .as_ref()
                .map(AgentCleanRoomReplacementPlanSummary::report_json)
                .unwrap_or_else(missing_agent_json),
            side_effects_json()
        )
    }
}

impl MemoryStartupAdmissionSummary {
    fn report_json(&self) -> String {
        format!(
            "{{\"loaded\":true,\"source\":\"external_memory_startup_admission_json\",\"source_path\":{},\"source_status\":{},\"evidence_map\":{{\"read_only_contract\":{},\"read_only_review_required\":{},\"index_quality_blocker_count\":{},\"index_quality_warning_count\":{},\"index_operation_count\":{},\"index_refresh_count\":{},\"context_rot_risk_count\":{},\"admission_decision_count\":{},\"admission_accepted_count\":{},\"admission_risk_rejection_count\":{},\"migration_live_store_targeted_count\":{},\"adapter_live_write_count\":{},\"live_write_phase_request_count\":{},\"live_store_mutation_requested\":{},\"store_mutation_count\":{},\"ndkv_write_allowed\":{},\"helper_prose_line_count\":{},\"non_contract_line_count\":{},\"admission_expanded_by_non_contract_evidence\":{}}},\"context_hygiene\":{}}}",
            json_string(&self.source_path),
            source_summary_json(&self.source_json, "memory_startup_admission"),
            option_bool_json(self.read_only_contract),
            option_bool_json(self.read_only_review_required),
            self.index_quality_blocker_count,
            self.index_quality_warning_count,
            self.index_operation_count,
            self.index_refresh_count,
            self.context_rot_risk_count,
            self.admission_decision_count,
            self.admission_accepted_count,
            self.admission_risk_rejection_count,
            self.migration_live_store_targeted_count,
            self.adapter_live_write_count,
            self.live_write_phase_request_count,
            self.live_store_mutation_requested,
            self.store_mutation_count,
            self.ndkv_write_allowed,
            self.helper_prose_line_count,
            self.non_contract_line_count,
            self.admission_expanded_by_non_contract_evidence,
            context_hygiene_json(&self.source_json, &self.source_path, false)
        )
    }
}

impl AgentCleanRoomReplacementPlanSummary {
    fn report_json(&self) -> String {
        format!(
            "{{\"loaded\":true,\"source\":\"external_agent_clean_room_replacement_plan_json\",\"source_path\":{},\"source_plan\":{},\"evidence_map\":{{\"report_only\":{},\"pure_data_only\":{},\"side_effects_allowed\":{},\"starts_thread\":{},\"sends_message\":{},\"reads_old_window_payload\":{},\"original_window_follow_up_assignment_allowed\":{},\"clean_room_replacement_plan_required\":{},\"clean_room_replacement_available\":{},\"replacement_prompt_ready\":{},\"follow_up_tasks_only_in_replacement_prompt\":{},\"replacement_prompt_task_count\":{},\"evidence_result_count\":{},\"reason_code_count\":{},\"task_ids\":{},\"evidence_result_ids\":{},\"reason_codes\":{},\"fresh_clean_room_assignment_exists\":{},\"completed_window_evidence_actionable\":{}}},\"context_hygiene\":{}}}",
            json_string(&self.source_path),
            source_summary_json(&self.source_json, "agent_clean_room_replacement_plan"),
            option_bool_json(self.report_only),
            option_bool_json(self.pure_data_only),
            option_bool_json(self.side_effects_allowed),
            option_bool_json(self.starts_thread),
            option_bool_json(self.sends_message),
            option_bool_json(self.reads_old_window_payload),
            option_bool_json(self.original_window_follow_up_assignment_allowed),
            option_bool_json(self.clean_room_replacement_plan_required),
            option_bool_json(self.clean_room_replacement_available),
            option_bool_json(self.replacement_prompt_ready),
            option_bool_json(self.follow_up_tasks_only_in_replacement_prompt),
            self.task_ids.len(),
            self.evidence_result_ids.len(),
            self.reason_codes.len(),
            json_string_array(&self.task_ids),
            json_string_array(&self.evidence_result_ids),
            json_string_array(&self.reason_codes),
            self.fresh_clean_room_assignment_exists(),
            self.completed_window_evidence_actionable(),
            context_hygiene_json(
                &self.source_json,
                &self.source_path,
                self.fresh_clean_room_assignment_exists()
            )
        )
    }

    fn fresh_clean_room_assignment_exists(&self) -> bool {
        self.clean_room_replacement_available == Some(true)
            && self.replacement_prompt_ready == Some(true)
            && self.follow_up_tasks_only_in_replacement_prompt == Some(true)
            && !self.task_ids.is_empty()
    }

    fn completed_window_evidence_actionable(&self) -> bool {
        self.fresh_clean_room_assignment_exists()
    }
}

fn load_memory_startup_admission(path: &Path) -> Result<MemoryStartupAdmissionSummary, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "read memory startup admission JSON {} failed: {error}",
            path.display()
        )
    })?;
    parse_memory_startup_admission_json(&text, &path.display().to_string())
}

fn load_agent_replacement_plan(
    path: &Path,
) -> Result<AgentCleanRoomReplacementPlanSummary, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "read agent clean-room replacement plan JSON {} failed: {error}",
            path.display()
        )
    })?;
    parse_agent_replacement_plan_json(&text, &path.display().to_string())
}

fn parse_memory_startup_admission_json(
    text: &str,
    source_path: &str,
) -> Result<MemoryStartupAdmissionSummary, String> {
    let root = root_object(text, "memory startup admission JSON")?;
    let status = json_object_field(&root, "memory_startup_admission_status")
        .or_else(|| json_object_field(&root, "memory_startup_admission"))
        .unwrap_or_else(|| root.clone());
    let migration_live_store_targeted_count =
        usize_field(&status, "migration_live_store_targeted_count");
    let adapter_live_write_count = usize_field(&status, "adapter_live_write_count");
    let live_write_phase_request_count = usize_field(&status, "live_write_phase_request_count");
    let store_mutation_count = usize_field(&status, "store_mutation_count");
    let live_store_mutation_requested = json_bool_field(&status, "live_store_mutation_requested")
        .unwrap_or_else(|| {
            migration_live_store_targeted_count > 0
                || adapter_live_write_count > 0
                || live_write_phase_request_count > 0
                || store_mutation_count > 0
        });

    Ok(MemoryStartupAdmissionSummary {
        source_path: source_path.to_owned(),
        source_json: root,
        read_only_contract: json_bool_field(&status, "read_only_contract")
            .or_else(|| json_bool_field(&status, "read_only_contract_holds")),
        read_only_review_required: json_bool_field(&status, "read_only_review_required"),
        index_quality_blocker_count: usize_field(&status, "index_quality_blocker_count"),
        index_quality_warning_count: usize_field(&status, "index_quality_warning_count"),
        index_operation_count: usize_field(&status, "index_operation_count"),
        index_refresh_count: usize_field(&status, "index_refresh_count"),
        context_rot_risk_count: usize_field(&status, "context_rot_risk_count"),
        admission_decision_count: usize_field(&status, "admission_decision_count"),
        admission_accepted_count: usize_field(&status, "admission_accepted_count"),
        admission_risk_rejection_count: usize_field(&status, "admission_risk_rejection_count"),
        migration_live_store_targeted_count,
        adapter_live_write_count,
        live_write_phase_request_count,
        live_store_mutation_requested,
        store_mutation_count,
        ndkv_write_allowed: json_bool_field(&status, "ndkv_write_allowed").unwrap_or(false),
        helper_prose_line_count: usize_field(&status, "helper_prose_line_count"),
        non_contract_line_count: usize_field(&status, "non_contract_line_count"),
        admission_expanded_by_non_contract_evidence: json_bool_field(
            &status,
            "admission_expanded_by_non_contract_evidence",
        )
        .unwrap_or(false),
    })
}

fn parse_agent_replacement_plan_json(
    text: &str,
    source_path: &str,
) -> Result<AgentCleanRoomReplacementPlanSummary, String> {
    let root = root_object(text, "agent clean-room replacement plan JSON")?;
    let plan = json_object_field(&root, "agent_clean_room_replacement_plan")
        .or_else(|| json_object_field(&root, "clean_room_replacement_plan"))
        .unwrap_or_else(|| root.clone());
    let prompt = json_object_field(&plan, "replacement_prompt").unwrap_or_else(|| "{}".to_owned());

    Ok(AgentCleanRoomReplacementPlanSummary {
        source_path: source_path.to_owned(),
        source_json: root,
        report_only: json_bool_field(&plan, "report_only"),
        pure_data_only: json_bool_field(&plan, "pure_data_only"),
        side_effects_allowed: json_bool_field(&plan, "side_effects_allowed"),
        starts_thread: json_bool_field(&plan, "starts_thread"),
        sends_message: json_bool_field(&plan, "sends_message"),
        reads_old_window_payload: json_bool_field(&plan, "reads_old_window_payload"),
        original_window_follow_up_assignment_allowed: json_bool_field(
            &plan,
            "original_window_follow_up_assignment_allowed",
        ),
        clean_room_replacement_plan_required: json_bool_field(
            &plan,
            "clean_room_replacement_plan_required",
        ),
        clean_room_replacement_available: json_bool_field(
            &plan,
            "clean_room_replacement_available",
        ),
        replacement_prompt_ready: json_bool_field(&plan, "replacement_prompt_ready"),
        follow_up_tasks_only_in_replacement_prompt: json_bool_field(
            &plan,
            "follow_up_tasks_only_in_replacement_prompt",
        ),
        task_ids: string_array_field(&prompt, "task_ids"),
        evidence_result_ids: string_array_field(&prompt, "evidence_result_ids"),
        reason_codes: string_array_field(&prompt, "reason_codes"),
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

fn usize_field(body: &str, field: &str) -> usize {
    json_u64_field(body, field)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or_default()
}

fn missing_memory_json() -> String {
    "{\"loaded\":false,\"source\":\"missing\",\"source_path\":null,\"source_status\":null,\"evidence_map\":{\"read_only_contract\":null,\"read_only_review_required\":null,\"index_quality_blocker_count\":0,\"index_quality_warning_count\":0,\"index_operation_count\":0,\"index_refresh_count\":0,\"context_rot_risk_count\":0,\"admission_decision_count\":0,\"admission_accepted_count\":0,\"admission_risk_rejection_count\":0,\"migration_live_store_targeted_count\":0,\"adapter_live_write_count\":0,\"live_write_phase_request_count\":0,\"live_store_mutation_requested\":false,\"store_mutation_count\":0,\"ndkv_write_allowed\":false,\"helper_prose_line_count\":0,\"non_contract_line_count\":0,\"admission_expanded_by_non_contract_evidence\":false}}".to_owned()
}

fn missing_agent_json() -> String {
    "{\"loaded\":false,\"source\":\"missing\",\"source_path\":null,\"source_plan\":null,\"evidence_map\":{\"report_only\":null,\"pure_data_only\":null,\"side_effects_allowed\":null,\"starts_thread\":null,\"sends_message\":null,\"reads_old_window_payload\":null,\"original_window_follow_up_assignment_allowed\":null,\"clean_room_replacement_plan_required\":null,\"clean_room_replacement_available\":null,\"replacement_prompt_ready\":null,\"follow_up_tasks_only_in_replacement_prompt\":null,\"replacement_prompt_task_count\":0,\"evidence_result_count\":0,\"reason_code_count\":0,\"task_ids\":[],\"evidence_result_ids\":[],\"reason_codes\":[],\"fresh_clean_room_assignment_exists\":false,\"completed_window_evidence_actionable\":false},\"context_hygiene\":{\"raw_old_thread_dialog_included\":false,\"raw_old_window_payload_included\":false,\"raw_source_omitted\":true,\"old_thread_dialog_field_count\":0,\"old_window_payload_field_count\":0,\"completed_window_evidence_non_actionable_without_fresh_assignment\":true,\"fresh_clean_room_assignment_exists\":false}}".to_owned()
}

fn side_effects_json() -> &'static str {
    "{\"starts_clean_room_replacement\":false,\"starts_thread\":false,\"sends_message\":false,\"reads_old_window_payload\":false,\"mutates_worker_window_status\":false,\"starts_daemon\":false,\"stops_daemon\":false,\"touches_remote\":false,\"downloads_model\":false,\"warms_model_cache\":false,\"sends_prompt\":false,\"starts_stream\":false,\"replays_prompt\":false,\"expands_memory_admission\":false,\"mutates_memory_store\":false,\"writes_ndkv\":false}"
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
    fn memory_startup_admission_status_is_report_only() {
        let summary = parse_memory_startup_admission_json(
            r#"{
  "schema": "memory_startup_admission_status_v1",
  "memory_startup_admission_status": {
    "read_only_contract": true,
    "read_only_review_required": true,
    "index_quality_blocker_count": 1,
    "index_quality_warning_count": 2,
    "index_operation_count": 3,
    "index_refresh_count": 1,
    "context_rot_risk_count": 2,
    "admission_decision_count": 4,
    "admission_accepted_count": 2,
    "admission_risk_rejection_count": 1,
    "migration_live_store_targeted_count": 0,
    "adapter_live_write_count": 0,
    "live_write_phase_request_count": 0,
    "live_store_mutation_requested": false,
    "store_mutation_count": 0,
    "ndkv_write_allowed": false,
    "helper_prose_line_count": 2,
    "non_contract_line_count": 3,
    "admission_expanded_by_non_contract_evidence": false
  }
}"#,
            "memory.json",
        )
        .unwrap();

        assert_eq!(summary.read_only_contract, Some(true));
        assert_eq!(summary.index_quality_blocker_count, 1);
        assert_eq!(summary.admission_decision_count, 4);
        assert!(!summary.live_store_mutation_requested);
        assert!(!summary.ndkv_write_allowed);
        assert!(!summary.admission_expanded_by_non_contract_evidence);
        let report = CleanRoomHandoffSummary {
            memory_startup_admission: Some(summary),
            agent_replacement_plan: None,
        }
        .report_json();
        assert!(report.contains("\"mutates_memory_store\":false"));
    }

    #[test]
    fn agent_replacement_plan_keeps_prompt_ids_only() {
        let summary = parse_agent_replacement_plan_json(
            r#"{
  "schema": "agent_window_context_clean_room_replacement_plan_v1",
  "report_only": true,
  "pure_data_only": true,
  "side_effects_allowed": false,
  "starts_thread": false,
  "sends_message": false,
  "reads_old_window_payload": false,
  "original_window_follow_up_assignment_allowed": false,
  "clean_room_replacement_plan_required": true,
  "clean_room_replacement_available": true,
  "replacement_prompt_ready": true,
  "follow_up_tasks_only_in_replacement_prompt": true,
  "replacement_prompt": {
    "task_ids": ["R24-clean-room-worker-A"],
    "evidence_result_ids": ["handoff-summary:r23-agent"],
    "reason_codes": ["window_context_polluted", "paused_by_main_window"]
  }
}"#,
            "agent.json",
        )
        .unwrap();

        assert_eq!(summary.report_only, Some(true));
        assert_eq!(summary.side_effects_allowed, Some(false));
        assert_eq!(summary.task_ids, vec!["R24-clean-room-worker-A"]);
        assert_eq!(
            summary.evidence_result_ids,
            vec!["handoff-summary:r23-agent"]
        );
        assert_eq!(summary.reason_codes.len(), 2);
        assert!(summary.report_json().contains("\"starts_thread\":false"));
    }

    #[test]
    fn missing_clean_room_handoff_is_stable_null_surface() {
        let json = option_status_json(None);

        assert!(json.contains("\"schema\":\"clean_room_handoff_report_v1\""));
        assert!(json.contains("\"memory_startup_admission\":{\"loaded\":false"));
        assert!(json.contains("\"agent_clean_room_replacement_plan\":{\"loaded\":false"));
        assert!(json.contains("\"writes_ndkv\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
    }

    #[test]
    fn handoff_report_omits_raw_old_dialog_and_requires_fresh_assignment_for_actionability() {
        let summary = parse_agent_replacement_plan_json(
            r#"{
  "schema": "agent_window_context_clean_room_replacement_plan_v1",
  "report_only": true,
  "pure_data_only": true,
  "side_effects_allowed": false,
  "starts_thread": false,
  "sends_message": false,
  "reads_old_window_payload": false,
  "original_window_follow_up_assignment_allowed": false,
  "clean_room_replacement_plan_required": true,
  "clean_room_replacement_available": true,
  "replacement_prompt_ready": false,
  "follow_up_tasks_only_in_replacement_prompt": true,
  "old_thread_dialog": "USER: reuse the old thread\nASSISTANT: this must be redacted",
  "old_window_payload": {"messages": ["raw old payload must be omitted"]},
  "replacement_prompt": {
    "task_ids": [],
    "evidence_result_ids": ["completed-window-evidence:r29"],
    "reason_codes": ["completed_window_evidence_only"]
  }
}"#,
            "agent-clean-room-replacement-plan-r30.example.json",
        )
        .unwrap();

        let json = CleanRoomHandoffSummary {
            memory_startup_admission: None,
            agent_replacement_plan: Some(summary),
        }
        .report_json();

        assert!(
            json.contains("\"source_plan\":{\"source_kind\":\"agent_clean_room_replacement_plan\"")
        );
        assert!(json.contains("\"raw_source_omitted\":true"));
        assert!(json.contains("\"old_thread_dialog_field_count\":1"));
        assert!(json.contains("\"raw_old_thread_dialog_included\":false"));
        assert!(json.contains("\"fresh_clean_room_assignment_exists\":false"));
        assert!(json.contains("\"completed_window_evidence_actionable\":false"));
        assert!(json.contains("\"clean_room_context\":{\"evidence_count\":1"));
        assert!(json.contains(
            "\"allowed_evidence_labels\":[\"agent-clean-room-replacement-plan-r30.example.json\"]"
        ));
        assert!(json.contains("\"polluted_evidence_labels\":[]"));
        assert!(json.contains("\"completed_window_follow_up_labels\":[]"));
        assert!(json.contains("\"context_hygiene_passed\":true"));
        assert!(json.contains("\"allow_clean_room_eval\":true"));
        assert!(!json.contains("USER: reuse the old thread"));
        assert!(!json.contains("raw old payload must be omitted"));
        assert!(json.contains("\"starts_thread\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
    }
}
