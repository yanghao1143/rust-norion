use super::status_json::{
    bool_value_text, compact_line, json_bool_field, json_object_field, json_string_array_field,
    json_string_field, json_string_literal, json_top_level_object_field, scalar_value,
};

pub(super) fn clean_room_handoff_report_lines(report_json: &str) -> Vec<String> {
    let Some(report) = CleanRoomHandoffReport::from_report_json(Some(report_json)) else {
        return Vec::new();
    };

    vec![
        format!(
            "clean_room_handoff_report read_only={} starts_process={} sends_prompt={} status_loaded={} report_only={} safe={} source={} memory_admission_safe={} no_live_write={} no_ndkv_write={} agent_replacement_plan_required={} replacement_prompt_ready={} starts_clean_room_replacement={} mutates_worker_window_status={} mutates_memory_store={} writes_ndkv={}",
            bool_value_text(report.read_only),
            bool_value_text(report.starts_process),
            bool_value_text(report.sends_prompt),
            bool_value_text(report.status_loaded),
            bool_value_text(report.report_only),
            bool_value_text(report.safe),
            report.source.as_deref().unwrap_or("unknown"),
            bool_value_text(report.memory_admission.safe),
            bool_value_text(report.memory_admission.no_live_write),
            bool_value_text(report.memory_admission.no_ndkv_write),
            bool_value_text(
                report
                    .agent_replacement
                    .clean_room_replacement_plan_required
            ),
            bool_value_text(report.agent_replacement.replacement_prompt_ready),
            bool_value_text(report.side_effects.starts_clean_room_replacement),
            bool_value_text(report.side_effects.mutates_worker_window_status),
            bool_value_text(report.side_effects.mutates_memory_store),
            bool_value_text(report.side_effects.writes_ndkv)
        ),
        format!(
            "clean_room_handoff_memory status_loaded={} read_only_contract={} admission_decisions={} admission_accepted={} admission_risk_rejections={} store_mutations={} helper_prose_lines={} non_contract_lines={} admission_expanded_by_non_contract={}",
            bool_value_text(report.memory_admission.status_loaded),
            bool_value_text(report.memory_admission.read_only_contract),
            report.memory_admission.admission_decision_count,
            report.memory_admission.admission_accepted_count,
            report.memory_admission.admission_risk_rejection_count,
            report.memory_admission.store_mutation_count,
            report.memory_admission.helper_prose_line_count,
            report.memory_admission.non_contract_line_count,
            bool_value_text(
                report
                    .memory_admission
                    .admission_expanded_by_non_contract_evidence
            )
        ),
        format!(
            "clean_room_handoff_agent status_loaded={} report_only={} pure_data_only={} side_effects_allowed={} reads_old_window_payload={} starts_thread={} sends_message={} prompt_tasks={} prompt_evidence_results={} prompt_reason_codes={}",
            bool_value_text(report.agent_replacement.status_loaded),
            bool_value_text(report.agent_replacement.report_only),
            bool_value_text(report.agent_replacement.pure_data_only),
            bool_value_text(report.agent_replacement.side_effects_allowed),
            bool_value_text(report.agent_replacement.reads_old_window_payload),
            bool_value_text(report.agent_replacement.starts_thread),
            bool_value_text(report.agent_replacement.sends_message),
            report.agent_replacement.prompt_task_count,
            report.agent_replacement.prompt_evidence_result_count,
            report.agent_replacement.prompt_reason_code_count
        ),
    ]
}

pub(super) fn clean_room_handoff_report_json(report_json: Option<&str>) -> String {
    CleanRoomHandoffReport::from_report_json(report_json)
        .unwrap_or_default()
        .to_json()
}

struct CleanRoomHandoffReport {
    read_only: bool,
    starts_process: bool,
    sends_prompt: bool,
    status_loaded: bool,
    report_only: bool,
    source: Option<String>,
    memory_admission: HandoffMemoryAdmission,
    agent_replacement: HandoffAgentReplacement,
    side_effects: HandoffSideEffects,
    safe: bool,
}

impl Default for CleanRoomHandoffReport {
    fn default() -> Self {
        Self {
            read_only: true,
            starts_process: false,
            sends_prompt: false,
            status_loaded: false,
            report_only: true,
            source: None,
            memory_admission: HandoffMemoryAdmission::default(),
            agent_replacement: HandoffAgentReplacement::default(),
            side_effects: HandoffSideEffects::default(),
            safe: true,
        }
    }
}

impl CleanRoomHandoffReport {
    fn from_report_json(report_json: Option<&str>) -> Option<Self> {
        let report = json_top_level_object_field(report_json?, "clean_room_handoff_report_v1")?;
        let evidence = json_object_field(report, "evidence_map");
        let side_effects = HandoffSideEffects::from_json(json_object_field(report, "side_effects"));
        let memory = HandoffMemoryAdmission::from_report(report, evidence);
        let agent = HandoffAgentReplacement::from_report(report, evidence);
        let read_only = json_bool_field(report, "read_only").unwrap_or(true);
        let starts_process = json_bool_field(report, "starts_process").unwrap_or(false);
        let sends_prompt =
            json_bool_field(report, "sends_prompt").unwrap_or(side_effects.sends_prompt);
        let report_only = json_bool_field(report, "report_only").unwrap_or(true);
        let safe = read_only
            && !starts_process
            && !sends_prompt
            && report_only
            && memory.safe
            && agent.safe
            && side_effects.safe();

        Some(Self {
            read_only,
            starts_process,
            sends_prompt,
            status_loaded: true,
            report_only,
            source: json_string_field(report, "source").map(|value| compact_line(&value, 160)),
            memory_admission: memory,
            agent_replacement: agent,
            side_effects,
            safe,
        })
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":{},\"starts_process\":{},\"sends_prompt\":{},\"status_loaded\":{},\"report_only\":{},\"safe\":{},\"source\":{},\"memory_admission\":{},\"agent_replacement\":{},\"side_effects\":{}}}",
            bool_value_text(self.read_only),
            bool_value_text(self.starts_process),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.status_loaded),
            bool_value_text(self.report_only),
            bool_value_text(self.safe),
            optional_string_json(self.source.as_deref()),
            self.memory_admission.to_json(),
            self.agent_replacement.to_json(),
            self.side_effects.to_json()
        )
    }
}

struct HandoffMemoryAdmission {
    status_loaded: bool,
    read_only_contract: bool,
    admission_decision_count: String,
    admission_accepted_count: String,
    admission_risk_rejection_count: String,
    live_store_mutation_requested: bool,
    store_mutation_count: String,
    ndkv_write_allowed: bool,
    helper_prose_line_count: String,
    non_contract_line_count: String,
    admission_expanded_by_non_contract_evidence: bool,
    no_live_write: bool,
    no_ndkv_write: bool,
    safe: bool,
}

impl Default for HandoffMemoryAdmission {
    fn default() -> Self {
        Self {
            status_loaded: false,
            read_only_contract: true,
            admission_decision_count: "0".to_owned(),
            admission_accepted_count: "0".to_owned(),
            admission_risk_rejection_count: "0".to_owned(),
            live_store_mutation_requested: false,
            store_mutation_count: "0".to_owned(),
            ndkv_write_allowed: false,
            helper_prose_line_count: "0".to_owned(),
            non_contract_line_count: "0".to_owned(),
            admission_expanded_by_non_contract_evidence: false,
            no_live_write: true,
            no_ndkv_write: true,
            safe: true,
        }
    }
}

impl HandoffMemoryAdmission {
    fn from_report(report: &str, evidence: Option<&str>) -> Self {
        let memory = json_object_field(report, "source_memory_startup_admission_status")
            .or_else(|| json_object_field(report, "memory_startup_admission_status"))
            .or_else(|| json_object_field(report, "source_memory_status"));
        let status_loaded = memory.is_some()
            || evidence
                .map(|value| scalar_value(value, "admission_decision_count") != "unknown")
                .unwrap_or(false);
        let live_store_mutation_requested = bool_from(memory, "live_store_mutation_requested")
            .or_else(|| bool_from(evidence, "live_store_mutation_requested"))
            .unwrap_or(false);
        let store_mutation_count = scalar_from(memory, evidence, "store_mutation_count");
        let ndkv_write_allowed = bool_from(memory, "ndkv_write_allowed")
            .or_else(|| bool_from(evidence, "ndkv_write_allowed"))
            .unwrap_or(false);
        let admission_expanded_by_non_contract_evidence =
            bool_from(memory, "admission_expanded_by_non_contract_evidence")
                .or_else(|| bool_from(evidence, "admission_expanded_by_non_contract_evidence"))
                .unwrap_or(false);
        let no_live_write = !live_store_mutation_requested && store_mutation_count == "0";
        let no_ndkv_write = !ndkv_write_allowed;
        let read_only_contract = bool_from(memory, "read_only_contract")
            .or_else(|| bool_from(evidence, "memory_read_only_contract"))
            .unwrap_or(true);
        let safe = read_only_contract
            && no_live_write
            && no_ndkv_write
            && !admission_expanded_by_non_contract_evidence;

        Self {
            status_loaded,
            read_only_contract,
            admission_decision_count: scalar_from(memory, evidence, "admission_decision_count"),
            admission_accepted_count: scalar_from(memory, evidence, "admission_accepted_count"),
            admission_risk_rejection_count: scalar_from(
                memory,
                evidence,
                "admission_risk_rejection_count",
            ),
            live_store_mutation_requested,
            store_mutation_count,
            ndkv_write_allowed,
            helper_prose_line_count: scalar_from(memory, evidence, "helper_prose_line_count"),
            non_contract_line_count: scalar_from(memory, evidence, "non_contract_line_count"),
            admission_expanded_by_non_contract_evidence,
            no_live_write,
            no_ndkv_write,
            safe,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"status_loaded\":{},\"safe\":{},\"read_only_contract\":{},\"admission_decision_count\":{},\"admission_accepted_count\":{},\"admission_risk_rejection_count\":{},\"live_store_mutation_requested\":{},\"store_mutation_count\":{},\"ndkv_write_allowed\":{},\"helper_prose_line_count\":{},\"non_contract_line_count\":{},\"admission_expanded_by_non_contract_evidence\":{},\"no_live_write\":{},\"no_ndkv_write\":{}}}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.safe),
            bool_value_text(self.read_only_contract),
            self.admission_decision_count,
            self.admission_accepted_count,
            self.admission_risk_rejection_count,
            bool_value_text(self.live_store_mutation_requested),
            self.store_mutation_count,
            bool_value_text(self.ndkv_write_allowed),
            self.helper_prose_line_count,
            self.non_contract_line_count,
            bool_value_text(self.admission_expanded_by_non_contract_evidence),
            bool_value_text(self.no_live_write),
            bool_value_text(self.no_ndkv_write)
        )
    }
}

struct HandoffAgentReplacement {
    status_loaded: bool,
    report_only: bool,
    pure_data_only: bool,
    side_effects_allowed: bool,
    reads_old_window_payload: bool,
    starts_thread: bool,
    sends_message: bool,
    clean_room_replacement_plan_required: bool,
    clean_room_replacement_available: bool,
    replacement_prompt_ready: bool,
    prompt_task_count: String,
    prompt_evidence_result_count: String,
    prompt_reason_code_count: String,
    safe: bool,
}

impl Default for HandoffAgentReplacement {
    fn default() -> Self {
        Self {
            status_loaded: false,
            report_only: true,
            pure_data_only: true,
            side_effects_allowed: false,
            reads_old_window_payload: false,
            starts_thread: false,
            sends_message: false,
            clean_room_replacement_plan_required: false,
            clean_room_replacement_available: false,
            replacement_prompt_ready: false,
            prompt_task_count: "0".to_owned(),
            prompt_evidence_result_count: "0".to_owned(),
            prompt_reason_code_count: "0".to_owned(),
            safe: true,
        }
    }
}

impl HandoffAgentReplacement {
    fn from_report(report: &str, evidence: Option<&str>) -> Self {
        let agent = json_object_field(report, "source_agent_clean_room_replacement_plan")
            .or_else(|| json_object_field(report, "agent_clean_room_replacement_plan"))
            .or_else(|| json_object_field(report, "source_agent_replacement_plan"));
        let prompt = agent.and_then(|value| json_object_field(value, "replacement_prompt"));
        let prompt_task_count = count_or_scalar(prompt, evidence, "task_ids", "prompt_task_count");
        let prompt_evidence_result_count = count_or_scalar(
            prompt,
            evidence,
            "evidence_result_ids",
            "prompt_evidence_result_count",
        );
        let prompt_reason_code_count =
            count_or_scalar(prompt, evidence, "reason_codes", "prompt_reason_code_count");
        let starts_thread = bool_from(agent, "starts_thread")
            .or_else(|| bool_from(evidence, "starts_thread"))
            .unwrap_or(false);
        let sends_message = bool_from(agent, "sends_message")
            .or_else(|| bool_from(evidence, "sends_message"))
            .unwrap_or(false);
        let reads_old_window_payload = bool_from(agent, "reads_old_window_payload")
            .or_else(|| bool_from(evidence, "reads_old_window_payload"))
            .unwrap_or(false);
        let side_effects_allowed = bool_from(agent, "side_effects_allowed")
            .or_else(|| bool_from(evidence, "side_effects_allowed"))
            .unwrap_or(false);
        let report_only = bool_from(agent, "report_only").unwrap_or(true);
        let pure_data_only = bool_from(agent, "pure_data_only").unwrap_or(true);
        let clean_room_replacement_plan_required =
            bool_from(agent, "clean_room_replacement_plan_required")
                .or_else(|| bool_from(evidence, "clean_room_replacement_plan_required"))
                .unwrap_or(false);
        let clean_room_replacement_available = bool_from(agent, "clean_room_replacement_available")
            .or_else(|| bool_from(evidence, "clean_room_replacement_available"))
            .unwrap_or(false);
        let replacement_prompt_ready = bool_from(agent, "replacement_prompt_ready")
            .or_else(|| bool_from(evidence, "replacement_prompt_ready"))
            .unwrap_or(false);
        let status_loaded = agent.is_some()
            || clean_room_replacement_plan_required
            || clean_room_replacement_available
            || replacement_prompt_ready;
        let safe = report_only
            && pure_data_only
            && !side_effects_allowed
            && !reads_old_window_payload
            && !starts_thread
            && !sends_message;

        Self {
            status_loaded,
            report_only,
            pure_data_only,
            side_effects_allowed,
            reads_old_window_payload,
            starts_thread,
            sends_message,
            clean_room_replacement_plan_required,
            clean_room_replacement_available,
            replacement_prompt_ready,
            prompt_task_count,
            prompt_evidence_result_count,
            prompt_reason_code_count,
            safe,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"status_loaded\":{},\"safe\":{},\"report_only\":{},\"pure_data_only\":{},\"side_effects_allowed\":{},\"reads_old_window_payload\":{},\"starts_thread\":{},\"sends_message\":{},\"clean_room_replacement_plan_required\":{},\"clean_room_replacement_available\":{},\"replacement_prompt_ready\":{},\"prompt_task_count\":{},\"prompt_evidence_result_count\":{},\"prompt_reason_code_count\":{}}}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.safe),
            bool_value_text(self.report_only),
            bool_value_text(self.pure_data_only),
            bool_value_text(self.side_effects_allowed),
            bool_value_text(self.reads_old_window_payload),
            bool_value_text(self.starts_thread),
            bool_value_text(self.sends_message),
            bool_value_text(self.clean_room_replacement_plan_required),
            bool_value_text(self.clean_room_replacement_available),
            bool_value_text(self.replacement_prompt_ready),
            self.prompt_task_count,
            self.prompt_evidence_result_count,
            self.prompt_reason_code_count
        )
    }
}

#[derive(Default)]
struct HandoffSideEffects {
    starts_clean_room_replacement: bool,
    mutates_worker_window_status: bool,
    starts_daemon: bool,
    stops_daemon: bool,
    touches_remote: bool,
    downloads_model: bool,
    warms_model_cache: bool,
    sends_prompt: bool,
    starts_stream: bool,
    replays_prompt: bool,
    starts_thread: bool,
    sends_message: bool,
    mutates_memory_store: bool,
    writes_ndkv: bool,
}

impl HandoffSideEffects {
    fn from_json(side_effects: Option<&str>) -> Self {
        Self {
            starts_clean_room_replacement: bool_from(side_effects, "starts_clean_room_replacement")
                .unwrap_or(false),
            mutates_worker_window_status: bool_from(side_effects, "mutates_worker_window_status")
                .unwrap_or(false),
            starts_daemon: bool_from(side_effects, "starts_daemon").unwrap_or(false),
            stops_daemon: bool_from(side_effects, "stops_daemon").unwrap_or(false),
            touches_remote: bool_from(side_effects, "touches_remote").unwrap_or(false),
            downloads_model: bool_from(side_effects, "downloads_model").unwrap_or(false),
            warms_model_cache: bool_from(side_effects, "warms_model_cache").unwrap_or(false),
            sends_prompt: bool_from(side_effects, "sends_prompt").unwrap_or(false),
            starts_stream: bool_from(side_effects, "starts_stream").unwrap_or(false),
            replays_prompt: bool_from(side_effects, "replays_prompt").unwrap_or(false),
            starts_thread: bool_from(side_effects, "starts_thread").unwrap_or(false),
            sends_message: bool_from(side_effects, "sends_message").unwrap_or(false),
            mutates_memory_store: bool_from(side_effects, "mutates_memory_store").unwrap_or(false),
            writes_ndkv: bool_from(side_effects, "writes_ndkv").unwrap_or(false),
        }
    }

    fn safe(&self) -> bool {
        !self.starts_clean_room_replacement
            && !self.mutates_worker_window_status
            && !self.starts_daemon
            && !self.stops_daemon
            && !self.touches_remote
            && !self.downloads_model
            && !self.warms_model_cache
            && !self.sends_prompt
            && !self.starts_stream
            && !self.replays_prompt
            && !self.starts_thread
            && !self.sends_message
            && !self.mutates_memory_store
            && !self.writes_ndkv
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"starts_clean_room_replacement\":{},\"mutates_worker_window_status\":{},\"starts_daemon\":{},\"stops_daemon\":{},\"touches_remote\":{},\"downloads_model\":{},\"warms_model_cache\":{},\"sends_prompt\":{},\"starts_stream\":{},\"replays_prompt\":{},\"starts_thread\":{},\"sends_message\":{},\"mutates_memory_store\":{},\"writes_ndkv\":{}}}",
            bool_value_text(self.starts_clean_room_replacement),
            bool_value_text(self.mutates_worker_window_status),
            bool_value_text(self.starts_daemon),
            bool_value_text(self.stops_daemon),
            bool_value_text(self.touches_remote),
            bool_value_text(self.downloads_model),
            bool_value_text(self.warms_model_cache),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.starts_stream),
            bool_value_text(self.replays_prompt),
            bool_value_text(self.starts_thread),
            bool_value_text(self.sends_message),
            bool_value_text(self.mutates_memory_store),
            bool_value_text(self.writes_ndkv)
        )
    }
}

fn bool_from(object: Option<&str>, field: &str) -> Option<bool> {
    object.and_then(|value| json_bool_field(value, field))
}

fn scalar_from(primary: Option<&str>, fallback: Option<&str>, field: &str) -> String {
    primary
        .map(|value| scalar_value(value, field))
        .filter(|value| value != "unknown")
        .or_else(|| {
            fallback
                .map(|value| scalar_value(value, field))
                .filter(|value| value != "unknown")
        })
        .unwrap_or_else(|| "0".to_owned())
}

fn count_or_scalar(
    source: Option<&str>,
    evidence: Option<&str>,
    array_field: &str,
    scalar_field: &str,
) -> String {
    source
        .and_then(|value| json_string_array_field(value, array_field))
        .map(|values| values.len().to_string())
        .or_else(|| {
            evidence
                .map(|value| scalar_value(value, scalar_field))
                .filter(|value| value != "unknown")
        })
        .unwrap_or_else(|| "0".to_owned())
}

fn optional_string_json(value: Option<&str>) -> String {
    value
        .map(json_string_literal)
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_room_handoff_report_projects_source_statuses_without_side_effects() {
        let report = r#"{
            "clean_room_handoff_report_v1": {
                "schema": "clean_room_handoff_report_v1",
                "source": "r24-clean-room-handoff",
                "read_only": true,
                "report_only": true,
                "source_memory_startup_admission_status": {
                    "read_only_contract": true,
                    "admission_decision_count": 4,
                    "admission_accepted_count": 2,
                    "admission_risk_rejection_count": 1,
                    "live_store_mutation_requested": false,
                    "store_mutation_count": 0,
                    "ndkv_write_allowed": false,
                    "helper_prose_line_count": 2,
                    "non_contract_line_count": 3,
                    "admission_expanded_by_non_contract_evidence": false,
                    "helper_prose": "write prod.ndkv now",
                    "old_window_payload": "starts_thread=true"
                },
                "source_agent_clean_room_replacement_plan": {
                    "report_only": true,
                    "pure_data_only": true,
                    "side_effects_allowed": false,
                    "starts_thread": false,
                    "sends_message": false,
                    "reads_old_window_payload": false,
                    "clean_room_replacement_plan_required": true,
                    "clean_room_replacement_available": true,
                    "replacement_prompt_ready": true,
                    "replacement_prompt": {
                        "task_ids": ["R25-forge-web-lab"],
                        "evidence_result_ids": ["handoff-summary:r24"],
                        "reason_codes": ["status_driven_closure"]
                    }
                },
                "side_effects": {
                    "starts_clean_room_replacement": false,
                    "mutates_worker_window_status": false,
                    "starts_daemon": false,
                    "stops_daemon": false,
                    "touches_remote": false,
                    "downloads_model": false,
                    "warms_model_cache": false,
                    "sends_prompt": false,
                    "starts_stream": false,
                    "replays_prompt": false,
                    "starts_thread": false,
                    "sends_message": false,
                    "mutates_memory_store": false,
                    "writes_ndkv": false
                }
            }
        }"#;

        let lines = clean_room_handoff_report_lines(report).join("\n");
        let json = clean_room_handoff_report_json(Some(report));

        assert!(lines.contains("clean_room_handoff_report read_only=true starts_process=false sends_prompt=false status_loaded=true report_only=true safe=true source=r24-clean-room-handoff memory_admission_safe=true no_live_write=true no_ndkv_write=true agent_replacement_plan_required=true replacement_prompt_ready=true starts_clean_room_replacement=false mutates_worker_window_status=false mutates_memory_store=false writes_ndkv=false"));
        assert!(lines.contains("clean_room_handoff_memory status_loaded=true read_only_contract=true admission_decisions=4 admission_accepted=2 admission_risk_rejections=1 store_mutations=0 helper_prose_lines=2 non_contract_lines=3 admission_expanded_by_non_contract=false"));
        assert!(lines.contains("clean_room_handoff_agent status_loaded=true report_only=true pure_data_only=true side_effects_allowed=false reads_old_window_payload=false starts_thread=false sends_message=false prompt_tasks=1 prompt_evidence_results=1 prompt_reason_codes=1"));
        assert!(json.contains("\"status_loaded\":true"));
        assert!(json.contains("\"safe\":true"));
        assert!(json.contains("\"prompt_task_count\":1"));
        assert!(json.contains("\"writes_ndkv\":false"));
        assert!(!json.contains("prod.ndkv"));
        assert!(!json.contains("starts_thread=true"));
        assert!(!json.contains("sends_message=true"));
    }

    #[test]
    fn clean_room_handoff_report_defaults_to_safe_empty_json_when_absent() {
        let json = clean_room_handoff_report_json(None);

        assert!(clean_room_handoff_report_lines("{}").is_empty());
        assert!(json.contains("\"read_only\":true"));
        assert!(json.contains("\"status_loaded\":false"));
        assert!(json.contains("\"safe\":true"));
        assert!(json.contains("\"starts_daemon\":false"));
        assert!(json.contains("\"writes_ndkv\":false"));
    }
}
