use super::status_json::{
    bool_value_text, compact_line, json_bool_field, json_object_field, json_string_array_field,
    json_string_field, json_string_literal, scalar_value,
};

pub(super) fn unified_status_lines(
    status: &str,
    worker_window_replacement_report_json: &str,
    clean_room_handoff_report_json: &str,
    self_improve_proposal_panel_json: &str,
    helper_stage_repair_panel_json: &str,
) -> Vec<String> {
    let view = UnifiedStatusView::from_status(
        status,
        worker_window_replacement_report_json,
        clean_room_handoff_report_json,
        self_improve_proposal_panel_json,
        helper_stage_repair_panel_json,
    );
    vec![
        format!(
            "unified_status read_only={} starts_process={} sends_prompt={} starts_daemon={} stops_daemon={} touches_remote={} downloads_model={} warms_model_cache={} starts_stream={} replays_prompt={} daemon_healthy={} supervisor_healthy={} model_pool_healthy={} worker_replacement_required={} memory_admission_safe={} no_live_write={} no_ndkv_write={} clean_room_handoff_loaded={} clean_room_handoff_safe={} self_improve_proposal_loaded={} self_improve_proposal_safe={} helper_stage_repair_loaded={} helper_stage_repair_safe={} helper_stage_repair_required={}",
            bool_value_text(view.read_only),
            bool_value_text(view.starts_process),
            bool_value_text(view.sends_prompt),
            bool_value_text(view.starts_daemon),
            bool_value_text(view.stops_daemon),
            bool_value_text(view.touches_remote),
            bool_value_text(view.downloads_model),
            bool_value_text(view.warms_model_cache),
            bool_value_text(view.starts_stream),
            bool_value_text(view.replays_prompt),
            bool_value_text(view.daemon.daemon_healthy),
            bool_value_text(view.daemon.supervisor_healthy),
            bool_value_text(view.model_pool.healthy),
            bool_value_text(view.worker_window_replacement.replacement_required),
            bool_value_text(view.memory_startup_admission.safe),
            bool_value_text(view.memory_startup_admission.no_live_write),
            bool_value_text(view.memory_startup_admission.no_ndkv_write),
            bool_value_text(view.clean_room_handoff.status_loaded),
            bool_value_text(view.clean_room_handoff.safe),
            bool_value_text(view.self_improve_proposal.status_loaded),
            bool_value_text(view.self_improve_proposal.safe),
            bool_value_text(view.helper_stage_repair.status_loaded),
            bool_value_text(view.helper_stage_repair.safe),
            bool_value_text(view.helper_stage_repair.repair_required)
        ),
        format!(
            "unified_daemon running={} pid={} supervisor_running={} supervisor_healthy={}",
            bool_value_text(view.daemon.running),
            view.daemon.pid.as_deref().unwrap_or("unknown"),
            optional_bool_line(view.daemon.supervisor_running),
            bool_value_text(view.daemon.supervisor_healthy)
        ),
        format!(
            "unified_model_pool available={} launch_allowed={} workers={}/{} reason={}",
            bool_value_text(view.model_pool.available),
            bool_value_text(view.model_pool.launch_allowed),
            view.model_pool.healthy_worker_count,
            view.model_pool.worker_count,
            view.model_pool.reason
        ),
        format!(
            "unified_worker_replacement status_loaded={} replacement_required={} replacement_required_count={} starts_clean_room_replacement={} mutates_worker_window_status={}",
            bool_value_text(view.worker_window_replacement.status_loaded),
            bool_value_text(view.worker_window_replacement.replacement_required),
            view.worker_window_replacement.replacement_required_count,
            bool_value_text(view.worker_window_replacement.starts_clean_room_replacement),
            bool_value_text(view.worker_window_replacement.mutates_worker_window_status)
        ),
        format!(
            "unified_memory_startup_admission status_loaded={} safe={} read_only_contract={} admission_decisions={} admission_accepted={} admission_risk_rejections={} live_store_mutation_requested={} store_mutations={} ndkv_write_allowed={} helper_prose_lines={} non_contract_lines={} admission_expanded_by_non_contract={}",
            bool_value_text(view.memory_startup_admission.status_loaded),
            bool_value_text(view.memory_startup_admission.safe),
            bool_value_text(view.memory_startup_admission.read_only_contract),
            view.memory_startup_admission.admission_decision_count,
            view.memory_startup_admission.admission_accepted_count,
            view.memory_startup_admission.admission_risk_rejection_count,
            bool_value_text(view.memory_startup_admission.live_store_mutation_requested),
            view.memory_startup_admission.store_mutation_count,
            bool_value_text(view.memory_startup_admission.ndkv_write_allowed),
            view.memory_startup_admission.helper_prose_line_count,
            view.memory_startup_admission.non_contract_line_count,
            bool_value_text(
                view.memory_startup_admission
                    .admission_expanded_by_non_contract_evidence
            )
        ),
        format!(
            "unified_clean_room_handoff status_loaded={} safe={} report_only={} memory_admission_safe={} agent_replacement_safe={} starts_clean_room_replacement={} mutates_worker_window_status={} mutates_memory_store={} writes_ndkv={}",
            bool_value_text(view.clean_room_handoff.status_loaded),
            bool_value_text(view.clean_room_handoff.safe),
            bool_value_text(view.clean_room_handoff.report_only),
            bool_value_text(view.clean_room_handoff.memory_admission_safe),
            bool_value_text(view.clean_room_handoff.agent_replacement_safe),
            bool_value_text(view.clean_room_handoff.starts_clean_room_replacement),
            bool_value_text(view.clean_room_handoff.mutates_worker_window_status),
            bool_value_text(view.clean_room_handoff.mutates_memory_store),
            bool_value_text(view.clean_room_handoff.writes_ndkv)
        ),
        format!(
            "unified_self_improve_proposal status_loaded={} safe={} report_only={} candidate={} validated={} admitted={} quarantined={} promoted={} repair_required={} starts_daemon={} starts_stream={} replays_prompt={} sends_prompt={} guidance_loaded={} convert_advisory_to_business_evidence={} repair_unvalidated_or_unaccepted={} requires_validation_and_memory_admission={} action_plan_loaded={} action_required={} primary_action={} actions={} action_plan_requires_validation_and_memory_admission={} action_assignment_loaded={} action_assignment_targets={} action_assignment_first_target={} action_assignment_first_round={} action_assignment_first_evidence_ids={} action_assignment_first_memory_admission={} action_assignment_first_validation_checked={} action_assignment_first_validation_passed={} action_assignment_first_memory_accepted={} action_assignment_first_business_evidence={} action_assignment_first_advisory_only={} action_assignment_first_require_repair={} action_assignment_first_missing={}",
            bool_value_text(view.self_improve_proposal.status_loaded),
            bool_value_text(view.self_improve_proposal.safe),
            bool_value_text(view.self_improve_proposal.report_only),
            view.self_improve_proposal.candidate_count,
            view.self_improve_proposal.validated_count,
            view.self_improve_proposal.admitted_count,
            view.self_improve_proposal.quarantined_count,
            view.self_improve_proposal.promoted_count,
            view.self_improve_proposal.repair_required_count,
            bool_value_text(view.self_improve_proposal.starts_daemon),
            bool_value_text(view.self_improve_proposal.starts_stream),
            bool_value_text(view.self_improve_proposal.replays_prompt),
            bool_value_text(view.self_improve_proposal.sends_prompt),
            bool_value_text(view.self_improve_proposal.prompt_guidance.status_loaded),
            bool_value_text(
                view.self_improve_proposal
                    .prompt_guidance
                    .convert_advisory_to_business_evidence
            ),
            bool_value_text(
                view.self_improve_proposal
                    .prompt_guidance
                    .repair_unvalidated_or_unaccepted
            ),
            bool_value_text(
                view.self_improve_proposal
                    .prompt_guidance
                    .requires_validation_and_memory_admission
            ),
            bool_value_text(view.self_improve_proposal.action_plan.status_loaded),
            bool_value_text(view.self_improve_proposal.action_plan.action_required),
            view.self_improve_proposal.action_plan.primary_action,
            list_value(&view.self_improve_proposal.action_plan.actions),
            bool_value_text(
                view.self_improve_proposal
                    .action_plan
                    .requires_validation_and_memory_admission
            ),
            bool_value_text(view.self_improve_proposal.action_assignment.status_loaded),
            view.self_improve_proposal.action_assignment.target_count,
            view.self_improve_proposal.action_assignment.first_target,
            view.self_improve_proposal
                .action_assignment
                .first_source_round,
            list_value(
                &view
                    .self_improve_proposal
                    .action_assignment
                    .first_evidence_ids
            ),
            view.self_improve_proposal
                .action_assignment
                .first_memory_admission_decision,
            bool_value_text(
                view.self_improve_proposal
                    .action_assignment
                    .first_validation_checked
            ),
            bool_value_text(
                view.self_improve_proposal
                    .action_assignment
                    .first_validation_passed
            ),
            bool_value_text(
                view.self_improve_proposal
                    .action_assignment
                    .first_memory_admission_accepted
            ),
            bool_value_text(
                view.self_improve_proposal
                    .action_assignment
                    .first_evidence_backed_business_improvement
            ),
            bool_value_text(
                view.self_improve_proposal
                    .action_assignment
                    .first_advisory_only
            ),
            bool_value_text(
                view.self_improve_proposal
                    .action_assignment
                    .first_require_repair
            ),
            list_value(
                &view
                    .self_improve_proposal
                    .action_assignment
                    .first_missing_requirements
            )
        ),
        format!(
            "unified_helper_stage_repair status_loaded={} safe={} report_only={} repair_required={} proposals={} incomplete_roles={} missing_helper_role_repair_required={} missing_helper_role_repair_proposals={} missing_helper_roles={} roles={} starts_daemon={} starts_forge={} starts_web_lab={} calls_model={} starts_stream={} replays_prompt={} sends_prompt={} auto_apply={}",
            bool_value_text(view.helper_stage_repair.status_loaded),
            bool_value_text(view.helper_stage_repair.safe),
            bool_value_text(view.helper_stage_repair.report_only),
            bool_value_text(view.helper_stage_repair.repair_required),
            view.helper_stage_repair.proposal_count,
            view.helper_stage_repair.incomplete_role_count,
            bool_value_text(view.helper_stage_repair.missing_helper_role_repair_required),
            view.helper_stage_repair
                .missing_helper_role_repair_proposal_count,
            list_value(&view.helper_stage_repair.missing_helper_roles),
            list_value(&view.helper_stage_repair.roles),
            bool_value_text(view.helper_stage_repair.starts_daemon),
            bool_value_text(view.helper_stage_repair.starts_forge),
            bool_value_text(view.helper_stage_repair.starts_web_lab),
            bool_value_text(view.helper_stage_repair.calls_model),
            bool_value_text(view.helper_stage_repair.starts_stream),
            bool_value_text(view.helper_stage_repair.replays_prompt),
            bool_value_text(view.helper_stage_repair.sends_prompt),
            bool_value_text(view.helper_stage_repair.auto_apply)
        ),
    ]
}

pub(super) fn unified_status_json(
    status: &str,
    worker_window_replacement_report_json: &str,
    clean_room_handoff_report_json: &str,
    self_improve_proposal_panel_json: &str,
    helper_stage_repair_panel_json: &str,
) -> String {
    UnifiedStatusView::from_status(
        status,
        worker_window_replacement_report_json,
        clean_room_handoff_report_json,
        self_improve_proposal_panel_json,
        helper_stage_repair_panel_json,
    )
    .to_json()
}

struct UnifiedStatusView {
    read_only: bool,
    starts_process: bool,
    sends_prompt: bool,
    starts_daemon: bool,
    stops_daemon: bool,
    touches_remote: bool,
    downloads_model: bool,
    warms_model_cache: bool,
    starts_stream: bool,
    replays_prompt: bool,
    daemon: UnifiedDaemonStatus,
    model_pool: UnifiedModelPoolStatus,
    worker_window_replacement: UnifiedWorkerWindowReplacementStatus,
    memory_startup_admission: UnifiedMemoryStartupAdmissionStatus,
    clean_room_handoff: UnifiedCleanRoomHandoffStatus,
    self_improve_proposal: UnifiedSelfImproveProposalStatus,
    helper_stage_repair: UnifiedHelperStageRepairStatus,
}

impl UnifiedStatusView {
    fn from_status(
        status: &str,
        worker_window_replacement_report_json: &str,
        clean_room_handoff_report_json: &str,
        self_improve_proposal_panel_json: &str,
        helper_stage_repair_panel_json: &str,
    ) -> Self {
        let daemon = json_object_field(status, "daemon");
        let loop_status = json_object_field(status, "loop");
        let model_pool = loop_status.and_then(|value| json_object_field(value, "model_pool"));
        let memory = json_object_field(status, "memory_startup_admission_status");

        Self {
            read_only: true,
            starts_process: false,
            sends_prompt: false,
            starts_daemon: false,
            stops_daemon: false,
            touches_remote: false,
            downloads_model: false,
            warms_model_cache: false,
            starts_stream: false,
            replays_prompt: false,
            daemon: UnifiedDaemonStatus::from_parts(daemon, loop_status),
            model_pool: UnifiedModelPoolStatus::from_json(model_pool),
            worker_window_replacement: UnifiedWorkerWindowReplacementStatus::from_json(
                worker_window_replacement_report_json,
            ),
            memory_startup_admission: UnifiedMemoryStartupAdmissionStatus::from_json(memory),
            clean_room_handoff: UnifiedCleanRoomHandoffStatus::from_json(
                clean_room_handoff_report_json,
            ),
            self_improve_proposal: UnifiedSelfImproveProposalStatus::from_json(
                self_improve_proposal_panel_json,
            ),
            helper_stage_repair: UnifiedHelperStageRepairStatus::from_json(
                helper_stage_repair_panel_json,
            ),
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":{},\"starts_process\":{},\"sends_prompt\":{},\"starts_daemon\":{},\"stops_daemon\":{},\"touches_remote\":{},\"downloads_model\":{},\"warms_model_cache\":{},\"starts_stream\":{},\"replays_prompt\":{},\"daemon_healthy\":{},\"supervisor_healthy\":{},\"model_pool_healthy\":{},\"worker_replacement_required\":{},\"memory_admission_safe\":{},\"no_live_write\":{},\"no_ndkv_write\":{},\"clean_room_handoff_loaded\":{},\"clean_room_handoff_safe\":{},\"self_improve_proposal_loaded\":{},\"self_improve_proposal_safe\":{},\"helper_stage_repair_loaded\":{},\"helper_stage_repair_safe\":{},\"helper_stage_repair_required\":{},\"daemon\":{},\"model_pool\":{},\"worker_window_replacement\":{},\"memory_startup_admission\":{},\"clean_room_handoff\":{},\"self_improve_proposal\":{},\"helper_stage_repair\":{}}}",
            bool_value_text(self.read_only),
            bool_value_text(self.starts_process),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.starts_daemon),
            bool_value_text(self.stops_daemon),
            bool_value_text(self.touches_remote),
            bool_value_text(self.downloads_model),
            bool_value_text(self.warms_model_cache),
            bool_value_text(self.starts_stream),
            bool_value_text(self.replays_prompt),
            bool_value_text(self.daemon.daemon_healthy),
            bool_value_text(self.daemon.supervisor_healthy),
            bool_value_text(self.model_pool.healthy),
            bool_value_text(self.worker_window_replacement.replacement_required),
            bool_value_text(self.memory_startup_admission.safe),
            bool_value_text(self.memory_startup_admission.no_live_write),
            bool_value_text(self.memory_startup_admission.no_ndkv_write),
            bool_value_text(self.clean_room_handoff.status_loaded),
            bool_value_text(self.clean_room_handoff.safe),
            bool_value_text(self.self_improve_proposal.status_loaded),
            bool_value_text(self.self_improve_proposal.safe),
            bool_value_text(self.helper_stage_repair.status_loaded),
            bool_value_text(self.helper_stage_repair.safe),
            bool_value_text(self.helper_stage_repair.repair_required),
            self.daemon.to_json(),
            self.model_pool.to_json(),
            self.worker_window_replacement.to_json(),
            self.memory_startup_admission.to_json(),
            self.clean_room_handoff.to_json(),
            self.self_improve_proposal.to_json(),
            self.helper_stage_repair.to_json()
        )
    }
}

struct UnifiedDaemonStatus {
    running: bool,
    pid: Option<String>,
    supervisor_running: Option<bool>,
    supervisor_healthy: bool,
    daemon_healthy: bool,
}

impl UnifiedDaemonStatus {
    fn from_parts(daemon: Option<&str>, loop_status: Option<&str>) -> Self {
        let running = daemon
            .and_then(|value| json_bool_field(value, "running"))
            .unwrap_or(false);
        let pid = daemon
            .map(|value| scalar_value(value, "pid"))
            .filter(|value| value != "unknown" && value != "null");
        let supervisor = loop_status.and_then(|value| json_object_field(value, "supervisor"));
        let supervisor_running = supervisor.and_then(|value| json_bool_field(value, "running"));
        let supervisor_healthy = supervisor
            .and_then(|value| json_bool_field(value, "healthy"))
            .unwrap_or(supervisor_running.unwrap_or(false));

        Self {
            running,
            pid,
            supervisor_running,
            supervisor_healthy,
            daemon_healthy: running,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"running\":{},\"pid\":{},\"daemon_healthy\":{},\"supervisor_running\":{},\"supervisor_healthy\":{}}}",
            bool_value_text(self.running),
            self.pid
                .as_deref()
                .map(json_string_literal)
                .unwrap_or_else(|| "null".to_owned()),
            bool_value_text(self.daemon_healthy),
            optional_bool_json(self.supervisor_running),
            bool_value_text(self.supervisor_healthy)
        )
    }
}

struct UnifiedModelPoolStatus {
    available: bool,
    launch_allowed: bool,
    worker_count: String,
    healthy_worker_count: String,
    reason: String,
    healthy: bool,
}

impl UnifiedModelPoolStatus {
    fn from_json(model_pool: Option<&str>) -> Self {
        let available = model_pool
            .and_then(|value| json_bool_field(value, "available"))
            .unwrap_or(false);
        let launch_allowed = model_pool
            .and_then(|value| json_bool_field(value, "launch_allowed"))
            .unwrap_or(false);
        let worker_count = model_pool
            .map(|value| scalar_value(value, "worker_count"))
            .filter(|value| value != "unknown")
            .unwrap_or_else(|| "0".to_owned());
        let healthy_worker_count = model_pool
            .map(|value| scalar_value(value, "healthy_worker_count"))
            .filter(|value| value != "unknown")
            .unwrap_or_else(|| "0".to_owned());
        let reason = model_pool
            .and_then(|value| json_string_field(value, "reason"))
            .unwrap_or_else(|| "unknown".to_owned());
        let healthy = available && launch_allowed && worker_count == healthy_worker_count;

        Self {
            available,
            launch_allowed,
            worker_count,
            healthy_worker_count,
            reason: compact_line(&reason, 160),
            healthy,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"available\":{},\"launch_allowed\":{},\"healthy\":{},\"worker_count\":{},\"healthy_worker_count\":{},\"reason\":{}}}",
            bool_value_text(self.available),
            bool_value_text(self.launch_allowed),
            bool_value_text(self.healthy),
            self.worker_count,
            self.healthy_worker_count,
            json_string_literal(&self.reason)
        )
    }
}

struct UnifiedWorkerWindowReplacementStatus {
    status_loaded: bool,
    replacement_required: bool,
    replacement_required_count: String,
    starts_clean_room_replacement: bool,
    mutates_worker_window_status: bool,
}

impl UnifiedWorkerWindowReplacementStatus {
    fn from_json(report: &str) -> Self {
        let status_loaded = json_bool_field(report, "status_loaded").unwrap_or(false);
        let replacement_required_count = scalar_value(report, "replacement_required_count");
        let replacement_required_count = if replacement_required_count == "unknown" {
            "0".to_owned()
        } else {
            replacement_required_count
        };
        let replacement_required = replacement_required_count != "0";
        Self {
            status_loaded,
            replacement_required,
            replacement_required_count,
            starts_clean_room_replacement: json_bool_field(report, "starts_clean_room_replacement")
                .unwrap_or(false),
            mutates_worker_window_status: json_bool_field(report, "mutates_worker_window_status")
                .unwrap_or(false),
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"status_loaded\":{},\"replacement_required\":{},\"replacement_required_count\":{},\"starts_clean_room_replacement\":{},\"mutates_worker_window_status\":{}}}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.replacement_required),
            self.replacement_required_count,
            bool_value_text(self.starts_clean_room_replacement),
            bool_value_text(self.mutates_worker_window_status)
        )
    }
}

struct UnifiedMemoryStartupAdmissionStatus {
    status_loaded: bool,
    read_only_contract: bool,
    read_only_review_required: bool,
    index_quality_blocker_count: String,
    index_quality_warning_count: String,
    index_operation_count: String,
    index_refresh_count: String,
    context_rot_risk_count: String,
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

impl UnifiedMemoryStartupAdmissionStatus {
    fn from_json(memory: Option<&str>) -> Self {
        let status_loaded = memory.is_some();
        let read_only_contract = memory
            .and_then(|value| json_bool_field(value, "read_only_contract"))
            .unwrap_or(true);
        let live_store_mutation_requested = memory
            .and_then(|value| json_bool_field(value, "live_store_mutation_requested"))
            .unwrap_or(false);
        let ndkv_write_allowed = memory
            .and_then(|value| json_bool_field(value, "ndkv_write_allowed"))
            .unwrap_or(false);
        let admission_expanded_by_non_contract_evidence = memory
            .and_then(|value| json_bool_field(value, "admission_expanded_by_non_contract_evidence"))
            .unwrap_or(false);
        let store_mutation_count = scalar_or_zero(memory, "store_mutation_count");
        let no_live_write = !live_store_mutation_requested && store_mutation_count == "0";
        let no_ndkv_write = !ndkv_write_allowed;
        let safe = read_only_contract
            && no_live_write
            && no_ndkv_write
            && !admission_expanded_by_non_contract_evidence;

        Self {
            status_loaded,
            read_only_contract,
            read_only_review_required: memory
                .and_then(|value| json_bool_field(value, "read_only_review_required"))
                .unwrap_or(false),
            index_quality_blocker_count: scalar_or_zero(memory, "index_quality_blocker_count"),
            index_quality_warning_count: scalar_or_zero(memory, "index_quality_warning_count"),
            index_operation_count: scalar_or_zero(memory, "index_operation_count"),
            index_refresh_count: scalar_or_zero(memory, "index_refresh_count"),
            context_rot_risk_count: scalar_or_zero(memory, "context_rot_risk_count"),
            admission_decision_count: scalar_or_zero(memory, "admission_decision_count"),
            admission_accepted_count: scalar_or_zero(memory, "admission_accepted_count"),
            admission_risk_rejection_count: scalar_or_zero(
                memory,
                "admission_risk_rejection_count",
            ),
            live_store_mutation_requested,
            store_mutation_count,
            ndkv_write_allowed,
            helper_prose_line_count: scalar_or_zero(memory, "helper_prose_line_count"),
            non_contract_line_count: scalar_or_zero(memory, "non_contract_line_count"),
            admission_expanded_by_non_contract_evidence,
            no_live_write,
            no_ndkv_write,
            safe,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"status_loaded\":{},\"safe\":{},\"read_only_contract\":{},\"read_only_review_required\":{},\"index_quality_blocker_count\":{},\"index_quality_warning_count\":{},\"index_operation_count\":{},\"index_refresh_count\":{},\"context_rot_risk_count\":{},\"admission_decision_count\":{},\"admission_accepted_count\":{},\"admission_risk_rejection_count\":{},\"live_store_mutation_requested\":{},\"store_mutation_count\":{},\"ndkv_write_allowed\":{},\"helper_prose_line_count\":{},\"non_contract_line_count\":{},\"admission_expanded_by_non_contract_evidence\":{},\"no_live_write\":{},\"no_ndkv_write\":{}}}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.safe),
            bool_value_text(self.read_only_contract),
            bool_value_text(self.read_only_review_required),
            self.index_quality_blocker_count,
            self.index_quality_warning_count,
            self.index_operation_count,
            self.index_refresh_count,
            self.context_rot_risk_count,
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

struct UnifiedCleanRoomHandoffStatus {
    status_loaded: bool,
    report_only: bool,
    safe: bool,
    memory_admission_safe: bool,
    agent_replacement_safe: bool,
    starts_clean_room_replacement: bool,
    mutates_worker_window_status: bool,
    mutates_memory_store: bool,
    writes_ndkv: bool,
}

impl UnifiedCleanRoomHandoffStatus {
    fn from_json(report: &str) -> Self {
        let status_loaded = json_bool_field(report, "status_loaded").unwrap_or(false);
        let report_only = json_bool_field(report, "report_only").unwrap_or(true);
        let memory = json_object_field(report, "memory_admission");
        let agent = json_object_field(report, "agent_replacement");
        let side_effects = json_object_field(report, "side_effects");
        let starts_clean_room_replacement = side_effects
            .and_then(|value| json_bool_field(value, "starts_clean_room_replacement"))
            .unwrap_or(false);
        let mutates_worker_window_status = side_effects
            .and_then(|value| json_bool_field(value, "mutates_worker_window_status"))
            .unwrap_or(false);
        let mutates_memory_store = side_effects
            .and_then(|value| json_bool_field(value, "mutates_memory_store"))
            .unwrap_or(false);
        let writes_ndkv = side_effects
            .and_then(|value| json_bool_field(value, "writes_ndkv"))
            .unwrap_or(false);
        let memory_admission_safe = memory
            .and_then(|value| json_bool_field(value, "safe"))
            .unwrap_or(true);
        let agent_replacement_safe = agent
            .and_then(|value| json_bool_field(value, "safe"))
            .unwrap_or(true);
        let safe = json_bool_field(report, "safe").unwrap_or(
            report_only
                && memory_admission_safe
                && agent_replacement_safe
                && !starts_clean_room_replacement
                && !mutates_worker_window_status
                && !mutates_memory_store
                && !writes_ndkv,
        );

        Self {
            status_loaded,
            report_only,
            safe,
            memory_admission_safe,
            agent_replacement_safe,
            starts_clean_room_replacement,
            mutates_worker_window_status,
            mutates_memory_store,
            writes_ndkv,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"status_loaded\":{},\"report_only\":{},\"safe\":{},\"memory_admission_safe\":{},\"agent_replacement_safe\":{},\"starts_clean_room_replacement\":{},\"mutates_worker_window_status\":{},\"mutates_memory_store\":{},\"writes_ndkv\":{}}}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.report_only),
            bool_value_text(self.safe),
            bool_value_text(self.memory_admission_safe),
            bool_value_text(self.agent_replacement_safe),
            bool_value_text(self.starts_clean_room_replacement),
            bool_value_text(self.mutates_worker_window_status),
            bool_value_text(self.mutates_memory_store),
            bool_value_text(self.writes_ndkv)
        )
    }
}

struct UnifiedSelfImproveProposalStatus {
    status_loaded: bool,
    report_only: bool,
    safe: bool,
    candidate_count: String,
    validated_count: String,
    admitted_count: String,
    quarantined_count: String,
    promoted_count: String,
    repair_required_count: String,
    starts_daemon: bool,
    starts_stream: bool,
    replays_prompt: bool,
    sends_prompt: bool,
    prompt_guidance: UnifiedSelfImproveProposalGuidance,
    action_plan: UnifiedSelfImproveProposalActionPlan,
    action_assignment: UnifiedSelfImproveProposalActionAssignment,
}

impl UnifiedSelfImproveProposalStatus {
    fn from_json(report: &str) -> Self {
        let status_loaded = json_bool_field(report, "status_loaded").unwrap_or(false);
        let report_only = json_bool_field(report, "report_only").unwrap_or(true);
        let base_safe = json_bool_field(report, "safe").unwrap_or(true);
        let side_effects = json_object_field(report, "side_effects");
        let starts_daemon = side_effects
            .and_then(|value| json_bool_field(value, "starts_daemon"))
            .unwrap_or(false);
        let starts_stream = side_effects
            .and_then(|value| json_bool_field(value, "starts_stream"))
            .unwrap_or(false);
        let replays_prompt = side_effects
            .and_then(|value| json_bool_field(value, "replays_prompt"))
            .unwrap_or(false);
        let sends_prompt = json_bool_field(report, "sends_prompt").unwrap_or(false)
            || side_effects
                .and_then(|value| json_bool_field(value, "sends_prompt"))
                .unwrap_or(false);
        let prompt_guidance = UnifiedSelfImproveProposalGuidance::from_json(json_object_field(
            report,
            "prompt_guidance",
        ));
        let action_plan = UnifiedSelfImproveProposalActionPlan::from_json(json_object_field(
            report,
            "action_plan",
        ));
        let action_assignment = UnifiedSelfImproveProposalActionAssignment::from_json(
            json_object_field(report, "action_assignment"),
        );
        let safe = base_safe && prompt_guidance.safe && action_plan.safe && action_assignment.safe;

        Self {
            status_loaded,
            report_only,
            safe,
            candidate_count: scalar_or_zero(Some(report), "candidate_count"),
            validated_count: scalar_or_zero(Some(report), "validated_count"),
            admitted_count: scalar_or_zero(Some(report), "admitted_count"),
            quarantined_count: scalar_or_zero(Some(report), "quarantined_count"),
            promoted_count: scalar_or_zero(Some(report), "promoted_count"),
            repair_required_count: scalar_or_zero(Some(report), "repair_required_count"),
            starts_daemon,
            starts_stream,
            replays_prompt,
            sends_prompt,
            prompt_guidance,
            action_plan,
            action_assignment,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":{},\"status_loaded\":{},\"report_only\":{},\"safe\":{},\"candidate_count\":{},\"validated_count\":{},\"admitted_count\":{},\"quarantined_count\":{},\"promoted_count\":{},\"repair_required_count\":{},\"starts_daemon\":{},\"starts_stream\":{},\"replays_prompt\":{},\"prompt_guidance\":{},\"action_plan\":{},\"action_assignment\":{}}}",
            bool_value_text(self.sends_prompt),
            bool_value_text(self.status_loaded),
            bool_value_text(self.report_only),
            bool_value_text(self.safe),
            self.candidate_count,
            self.validated_count,
            self.admitted_count,
            self.quarantined_count,
            self.promoted_count,
            self.repair_required_count,
            bool_value_text(self.starts_daemon),
            bool_value_text(self.starts_stream),
            bool_value_text(self.replays_prompt),
            self.prompt_guidance.to_json(),
            self.action_plan.to_json(),
            self.action_assignment.to_json()
        )
    }
}

struct UnifiedSelfImproveProposalGuidance {
    status_loaded: bool,
    safe: bool,
    convert_advisory_to_business_evidence: bool,
    repair_unvalidated_or_unaccepted: bool,
    requires_validation_and_memory_admission: bool,
}

impl UnifiedSelfImproveProposalGuidance {
    fn from_json(guidance: Option<&str>) -> Self {
        let Some(guidance) = guidance else {
            return Self {
                status_loaded: false,
                safe: true,
                convert_advisory_to_business_evidence: false,
                repair_unvalidated_or_unaccepted: false,
                requires_validation_and_memory_admission: false,
            };
        };
        let read_only = json_bool_field(guidance, "read_only").unwrap_or(true);
        let starts_process = json_bool_field(guidance, "starts_process").unwrap_or(false);
        let sends_prompt = json_bool_field(guidance, "sends_prompt").unwrap_or(false);
        let report_only = json_bool_field(guidance, "report_only").unwrap_or(true);
        let explicit_safe = json_bool_field(guidance, "safe").unwrap_or(true);

        Self {
            status_loaded: json_bool_field(guidance, "status_loaded").unwrap_or(true),
            safe: read_only && !starts_process && !sends_prompt && report_only && explicit_safe,
            convert_advisory_to_business_evidence: json_bool_field(
                guidance,
                "convert_advisory_to_business_evidence",
            )
            .unwrap_or(false),
            repair_unvalidated_or_unaccepted: json_bool_field(
                guidance,
                "repair_unvalidated_or_unaccepted",
            )
            .unwrap_or(false),
            requires_validation_and_memory_admission: json_bool_field(
                guidance,
                "requires_validation_and_memory_admission",
            )
            .unwrap_or(false),
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"status_loaded\":{},\"report_only\":true,\"safe\":{},\"convert_advisory_to_business_evidence\":{},\"repair_unvalidated_or_unaccepted\":{},\"requires_validation_and_memory_admission\":{}}}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.safe),
            bool_value_text(self.convert_advisory_to_business_evidence),
            bool_value_text(self.repair_unvalidated_or_unaccepted),
            bool_value_text(self.requires_validation_and_memory_admission)
        )
    }
}

struct UnifiedSelfImproveProposalActionPlan {
    status_loaded: bool,
    safe: bool,
    action_required: bool,
    primary_action: String,
    actions: Vec<String>,
    requires_validation_and_memory_admission: bool,
    auto_apply: bool,
}

impl UnifiedSelfImproveProposalActionPlan {
    fn from_json(action_plan: Option<&str>) -> Self {
        let Some(action_plan) = action_plan else {
            return Self {
                status_loaded: false,
                safe: true,
                action_required: false,
                primary_action: "none".to_owned(),
                actions: Vec::new(),
                requires_validation_and_memory_admission: false,
                auto_apply: false,
            };
        };
        let actions = json_string_array_field(action_plan, "actions").unwrap_or_default();
        let read_only = json_bool_field(action_plan, "read_only").unwrap_or(true);
        let starts_process = json_bool_field(action_plan, "starts_process").unwrap_or(false);
        let sends_prompt = json_bool_field(action_plan, "sends_prompt").unwrap_or(false);
        let report_only = json_bool_field(action_plan, "report_only").unwrap_or(true);
        let explicit_safe = json_bool_field(action_plan, "safe").unwrap_or(true);
        let auto_apply = json_bool_field(action_plan, "auto_apply").unwrap_or(false);

        Self {
            status_loaded: json_bool_field(action_plan, "status_loaded").unwrap_or(true),
            safe: read_only
                && !starts_process
                && !sends_prompt
                && report_only
                && explicit_safe
                && !auto_apply,
            action_required: json_bool_field(action_plan, "action_required")
                .unwrap_or(!actions.is_empty()),
            primary_action: json_string_field(action_plan, "primary_action").unwrap_or_else(|| {
                actions
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "none".to_owned())
            }),
            actions,
            requires_validation_and_memory_admission: json_bool_field(
                action_plan,
                "requires_validation_and_memory_admission",
            )
            .unwrap_or(false),
            auto_apply,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"status_loaded\":{},\"report_only\":true,\"safe\":{},\"action_required\":{},\"primary_action\":{},\"actions\":{},\"requires_validation_and_memory_admission\":{},\"auto_apply\":{}}}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.safe),
            bool_value_text(self.action_required),
            json_string_literal(&self.primary_action),
            string_array_json(&self.actions),
            bool_value_text(self.requires_validation_and_memory_admission),
            bool_value_text(self.auto_apply)
        )
    }
}

struct UnifiedSelfImproveProposalActionAssignment {
    status_loaded: bool,
    safe: bool,
    action_required: bool,
    primary_action: String,
    actions: Vec<String>,
    target_count: String,
    first_target: String,
    first_source_round: String,
    first_evidence_ids: Vec<String>,
    first_memory_admission_decision: String,
    first_validation_checked: bool,
    first_validation_passed: bool,
    first_memory_admission_accepted: bool,
    first_evidence_backed_business_improvement: bool,
    first_advisory_only: bool,
    first_require_repair: bool,
    first_missing_requirements: Vec<String>,
    requires_validation_and_memory_admission: bool,
    auto_apply: bool,
}

impl UnifiedSelfImproveProposalActionAssignment {
    fn from_json(action_assignment: Option<&str>) -> Self {
        let Some(action_assignment) = action_assignment else {
            return Self {
                status_loaded: false,
                safe: true,
                action_required: false,
                primary_action: "none".to_owned(),
                actions: Vec::new(),
                target_count: "0".to_owned(),
                first_target: "none".to_owned(),
                first_source_round: "null".to_owned(),
                first_evidence_ids: Vec::new(),
                first_memory_admission_decision: "unknown".to_owned(),
                first_validation_checked: false,
                first_validation_passed: false,
                first_memory_admission_accepted: false,
                first_evidence_backed_business_improvement: false,
                first_advisory_only: false,
                first_require_repair: false,
                first_missing_requirements: Vec::new(),
                requires_validation_and_memory_admission: false,
                auto_apply: false,
            };
        };
        let actions = json_string_array_field(action_assignment, "actions").unwrap_or_default();
        let target_count = scalar_or_zero(Some(action_assignment), "target_count");
        let first_source_round = scalar_value(action_assignment, "first_source_round");
        let first_source_round = if first_source_round == "unknown" {
            "null".to_owned()
        } else {
            first_source_round
        };
        let read_only = json_bool_field(action_assignment, "read_only").unwrap_or(true);
        let starts_process = json_bool_field(action_assignment, "starts_process").unwrap_or(false);
        let sends_prompt = json_bool_field(action_assignment, "sends_prompt").unwrap_or(false);
        let report_only = json_bool_field(action_assignment, "report_only").unwrap_or(true);
        let explicit_safe = json_bool_field(action_assignment, "safe").unwrap_or(true);
        let auto_apply = json_bool_field(action_assignment, "auto_apply").unwrap_or(false);

        Self {
            status_loaded: json_bool_field(action_assignment, "status_loaded").unwrap_or(true),
            safe: read_only
                && !starts_process
                && !sends_prompt
                && report_only
                && explicit_safe
                && !auto_apply,
            action_required: json_bool_field(action_assignment, "action_required")
                .unwrap_or(target_count != "0"),
            primary_action: json_string_field(action_assignment, "primary_action")
                .or_else(|| actions.first().cloned())
                .unwrap_or_else(|| "none".to_owned()),
            actions,
            target_count,
            first_target: json_string_field(action_assignment, "first_target")
                .unwrap_or_else(|| "none".to_owned()),
            first_source_round,
            first_evidence_ids: json_string_array_field(action_assignment, "first_evidence_ids")
                .unwrap_or_default(),
            first_memory_admission_decision: json_string_field(
                action_assignment,
                "first_memory_admission_decision",
            )
            .unwrap_or_else(|| "unknown".to_owned()),
            first_validation_checked: json_bool_field(
                action_assignment,
                "first_validation_checked",
            )
            .unwrap_or(false),
            first_validation_passed: json_bool_field(action_assignment, "first_validation_passed")
                .unwrap_or(false),
            first_memory_admission_accepted: json_bool_field(
                action_assignment,
                "first_memory_admission_accepted",
            )
            .unwrap_or(false),
            first_evidence_backed_business_improvement: json_bool_field(
                action_assignment,
                "first_evidence_backed_business_improvement",
            )
            .unwrap_or(false),
            first_advisory_only: json_bool_field(action_assignment, "first_advisory_only")
                .unwrap_or(false),
            first_require_repair: json_bool_field(action_assignment, "first_require_repair")
                .unwrap_or(false),
            first_missing_requirements: json_string_array_field(
                action_assignment,
                "first_missing_requirements",
            )
            .unwrap_or_default(),
            requires_validation_and_memory_admission: json_bool_field(
                action_assignment,
                "requires_validation_and_memory_admission",
            )
            .unwrap_or(false),
            auto_apply,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"status_loaded\":{},\"report_only\":true,\"safe\":{},\"action_required\":{},\"primary_action\":{},\"actions\":{},\"target_count\":{},\"first_target\":{},\"first_source_round\":{},\"first_evidence_ids\":{},\"first_memory_admission_decision\":{},\"first_validation_checked\":{},\"first_validation_passed\":{},\"first_memory_admission_accepted\":{},\"first_evidence_backed_business_improvement\":{},\"first_advisory_only\":{},\"first_require_repair\":{},\"first_missing_requirements\":{},\"requires_validation_and_memory_admission\":{},\"auto_apply\":{}}}",
            bool_value_text(self.status_loaded),
            bool_value_text(self.safe),
            bool_value_text(self.action_required),
            json_string_literal(&self.primary_action),
            string_array_json(&self.actions),
            self.target_count,
            json_string_literal(&self.first_target),
            self.first_source_round,
            string_array_json(&self.first_evidence_ids),
            json_string_literal(&self.first_memory_admission_decision),
            bool_value_text(self.first_validation_checked),
            bool_value_text(self.first_validation_passed),
            bool_value_text(self.first_memory_admission_accepted),
            bool_value_text(self.first_evidence_backed_business_improvement),
            bool_value_text(self.first_advisory_only),
            bool_value_text(self.first_require_repair),
            string_array_json(&self.first_missing_requirements),
            bool_value_text(self.requires_validation_and_memory_admission),
            bool_value_text(self.auto_apply)
        )
    }
}

struct UnifiedHelperStageRepairStatus {
    status_loaded: bool,
    report_only: bool,
    safe: bool,
    repair_required: bool,
    proposal_count: String,
    incomplete_role_count: String,
    missing_helper_role_repair_required: bool,
    missing_helper_role_repair_proposal_count: String,
    missing_helper_roles: Vec<String>,
    roles: Vec<String>,
    starts_daemon: bool,
    starts_forge: bool,
    starts_web_lab: bool,
    calls_model: bool,
    starts_stream: bool,
    replays_prompt: bool,
    sends_prompt: bool,
    auto_apply: bool,
    writes_ndkv: bool,
    mutates_memory_store: bool,
}

impl UnifiedHelperStageRepairStatus {
    fn from_json(report: &str) -> Self {
        let status_loaded = json_bool_field(report, "status_loaded").unwrap_or(false);
        let report_only = json_bool_field(report, "report_only").unwrap_or(true);
        let side_effects = json_object_field(report, "side_effects");
        let starts_daemon = side_effects
            .and_then(|value| json_bool_field(value, "starts_daemon"))
            .unwrap_or(false);
        let starts_forge = side_effects
            .and_then(|value| json_bool_field(value, "starts_forge"))
            .unwrap_or(false);
        let starts_web_lab = side_effects
            .and_then(|value| json_bool_field(value, "starts_web_lab"))
            .unwrap_or(false);
        let calls_model = side_effects
            .and_then(|value| json_bool_field(value, "calls_model"))
            .unwrap_or(false);
        let starts_stream = side_effects
            .and_then(|value| json_bool_field(value, "starts_stream"))
            .unwrap_or(false);
        let replays_prompt = side_effects
            .and_then(|value| json_bool_field(value, "replays_prompt"))
            .unwrap_or(false);
        let side_effect_sends_prompt = side_effects
            .and_then(|value| json_bool_field(value, "sends_prompt"))
            .unwrap_or(false);
        let sends_prompt =
            json_bool_field(report, "sends_prompt").unwrap_or(false) || side_effect_sends_prompt;
        let writes_ndkv = side_effects
            .and_then(|value| json_bool_field(value, "writes_ndkv"))
            .unwrap_or(false);
        let mutates_memory_store = side_effects
            .and_then(|value| json_bool_field(value, "mutates_memory_store"))
            .unwrap_or(false);
        let auto_apply = json_bool_field(report, "auto_apply").unwrap_or(false);
        let proposal_count = scalar_or_zero(Some(report), "proposal_count");
        let repair_required =
            json_bool_field(report, "repair_required").unwrap_or(proposal_count != "0");
        let missing_helper_role_repair_proposal_count = scalar_or_zero_from_fields(
            Some(report),
            &[
                "missing_helper_role_repair_proposal_count",
                "missing_helper_role_count",
            ],
        );
        let missing_helper_role_repair_required =
            json_bool_field(report, "missing_helper_role_repair_required").unwrap_or(
                missing_helper_role_repair_proposal_count
                    .parse::<usize>()
                    .is_ok_and(|count| count > 0),
            );
        let safe = json_bool_field(report, "safe").unwrap_or(
            report_only
                && !starts_daemon
                && !starts_forge
                && !starts_web_lab
                && !calls_model
                && !starts_stream
                && !replays_prompt
                && !sends_prompt
                && !writes_ndkv
                && !mutates_memory_store
                && !auto_apply,
        );

        Self {
            status_loaded,
            report_only,
            safe,
            repair_required,
            proposal_count,
            incomplete_role_count: scalar_or_zero(Some(report), "incomplete_role_count"),
            missing_helper_role_repair_required,
            missing_helper_role_repair_proposal_count,
            missing_helper_roles: json_string_array_field(report, "missing_helper_roles")
                .unwrap_or_default(),
            roles: json_string_array_field(report, "roles").unwrap_or_default(),
            starts_daemon,
            starts_forge,
            starts_web_lab,
            calls_model,
            starts_stream,
            replays_prompt,
            sends_prompt,
            auto_apply,
            writes_ndkv,
            mutates_memory_store,
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":{},\"status_loaded\":{},\"report_only\":{},\"safe\":{},\"repair_required\":{},\"proposal_count\":{},\"incomplete_role_count\":{},\"missing_helper_role_repair_required\":{},\"missing_helper_role_repair_proposal_count\":{},\"missing_helper_roles\":{},\"roles\":{},\"starts_daemon\":{},\"starts_forge\":{},\"starts_web_lab\":{},\"calls_model\":{},\"starts_stream\":{},\"replays_prompt\":{},\"auto_apply\":{},\"writes_ndkv\":{},\"mutates_memory_store\":{}}}",
            bool_value_text(self.sends_prompt),
            bool_value_text(self.status_loaded),
            bool_value_text(self.report_only),
            bool_value_text(self.safe),
            bool_value_text(self.repair_required),
            self.proposal_count,
            self.incomplete_role_count,
            bool_value_text(self.missing_helper_role_repair_required),
            self.missing_helper_role_repair_proposal_count,
            string_array_json(&self.missing_helper_roles),
            string_array_json(&self.roles),
            bool_value_text(self.starts_daemon),
            bool_value_text(self.starts_forge),
            bool_value_text(self.starts_web_lab),
            bool_value_text(self.calls_model),
            bool_value_text(self.starts_stream),
            bool_value_text(self.replays_prompt),
            bool_value_text(self.auto_apply),
            bool_value_text(self.writes_ndkv),
            bool_value_text(self.mutates_memory_store)
        )
    }
}

fn scalar_or_zero(object: Option<&str>, field: &str) -> String {
    object
        .map(|value| scalar_value(value, field))
        .filter(|value| value != "unknown")
        .unwrap_or_else(|| "0".to_owned())
}

fn scalar_or_zero_from_fields(object: Option<&str>, fields: &[&str]) -> String {
    fields
        .iter()
        .find_map(|field| {
            object
                .map(|value| scalar_value(value, field))
                .filter(|value| value != "unknown")
        })
        .unwrap_or_else(|| "0".to_owned())
}

fn list_value(values: &[String]) -> String {
    values
        .is_empty()
        .then(|| "none".to_owned())
        .unwrap_or_else(|| values.join(","))
}

fn string_array_json(values: &[String]) -> String {
    let mut out = String::from("[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&json_string_literal(value));
    }
    out.push(']');
    out
}

fn optional_bool_json(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    }
}

fn optional_bool_line(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKER_REPLACEMENT: &str = r#"{
        "read_only": true,
        "starts_process": false,
        "sends_prompt": false,
        "status_loaded": true,
        "replacement_required_count": 1,
        "starts_clean_room_replacement": false,
        "mutates_worker_window_status": false
    }"#;

    const CLEAN_ROOM_HANDOFF: &str = r#"{
        "read_only": true,
        "starts_process": false,
        "sends_prompt": false,
        "status_loaded": true,
        "report_only": true,
        "safe": true,
        "memory_admission": {"safe": true},
        "agent_replacement": {"safe": true},
        "side_effects": {
            "starts_clean_room_replacement": false,
            "mutates_worker_window_status": false,
            "mutates_memory_store": false,
            "writes_ndkv": false
        }
    }"#;

    const SELF_IMPROVE_PROPOSAL: &str = r#"{
        "read_only": true,
        "starts_process": false,
        "sends_prompt": false,
        "status_loaded": true,
        "report_only": true,
        "safe": true,
        "candidate_count": 2,
        "validated_count": 1,
        "admitted_count": 1,
        "quarantined_count": 1,
        "promoted_count": 1,
        "repair_required_count": 1,
        "prompt_guidance": {
            "read_only": true,
            "starts_process": false,
            "sends_prompt": false,
            "status_loaded": true,
            "report_only": true,
            "safe": true,
            "convert_advisory_to_business_evidence": true,
            "repair_unvalidated_or_unaccepted": false,
            "requires_validation_and_memory_admission": true
        },
        "action_plan": {
            "read_only": true,
            "starts_process": false,
            "sends_prompt": false,
            "status_loaded": true,
            "report_only": true,
            "safe": true,
            "action_required": true,
            "primary_action": "convert_advisory_to_evidence_backed_business_improvement",
            "actions": [
                "convert_advisory_to_evidence_backed_business_improvement",
                "require_checked_passed_validation_and_accepted_memory_admission"
            ],
            "requires_validation_and_memory_admission": true,
            "auto_apply": false
        },
        "action_assignment": {
            "read_only": true,
            "starts_process": false,
            "sends_prompt": false,
            "status_loaded": true,
            "report_only": true,
            "safe": true,
            "action_required": true,
            "primary_action": "convert_advisory_to_evidence_backed_business_improvement",
            "actions": [
                "convert_advisory_to_evidence_backed_business_improvement",
                "require_checked_passed_validation_and_accepted_memory_admission"
            ],
            "target_count": 2,
            "first_target": "self-improve-r385-helper_contract-modifythereviewstagesval",
            "first_source_round": 385,
            "first_evidence_ids": [
                "ledger.round.385.helper_stage_contract.review.change_request"
            ],
            "first_memory_admission_decision": "quarantined",
            "first_validation_checked": true,
            "first_validation_passed": true,
            "first_memory_admission_accepted": false,
            "first_evidence_backed_business_improvement": false,
            "first_advisory_only": true,
            "first_require_repair": false,
            "first_missing_requirements": [
                "accepted_memory_admission",
                "evidence_backed_business_improvement"
            ],
            "requires_validation_and_memory_admission": true,
            "auto_apply": false
        },
        "side_effects": {
            "starts_daemon": false,
            "sends_prompt": false,
            "starts_stream": false,
            "replays_prompt": false
        }
    }"#;

    const HELPER_STAGE_REPAIR: &str = r#"{
        "read_only": true,
        "starts_process": false,
        "sends_prompt": false,
        "status_loaded": true,
        "report_only": true,
        "safe": true,
        "repair_required": true,
        "proposal_count": 3,
        "incomplete_role_count": 2,
        "missing_helper_role_repair_required": true,
        "missing_helper_role_repair_proposal_count": 1,
        "missing_helper_roles": ["router"],
        "roles": ["router", "review", "test-gate"],
        "auto_apply": false,
        "side_effects": {
            "starts_daemon": false,
            "starts_forge": false,
            "starts_web_lab": false,
            "calls_model": false,
            "sends_prompt": false,
            "starts_stream": false,
            "replays_prompt": false,
            "writes_ndkv": false,
            "mutates_memory_store": false
        }
    }"#;

    #[test]
    fn unified_status_combines_daemon_pool_worker_and_memory_without_side_effects() {
        let status = r#"{
            "daemon": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "running": true,
                "pid": 224392
            },
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "touches_remote": false,
                "supervisor": {"running": true, "pid": 210792, "healthy": true},
                "model_pool": {
                    "available": true,
                    "launch_allowed": true,
                    "worker_count": 6,
                    "healthy_worker_count": 6,
                    "reason": "ready"
                }
            },
            "memory_startup_admission_status": {
                "read_only_contract": true,
                "read_only_review_required": false,
                "index_quality_blocker_count": 0,
                "index_quality_warning_count": 1,
                "index_operation_count": 2,
                "index_refresh_count": 1,
                "context_rot_risk_count": 1,
                "admission_decision_count": 3,
                "admission_accepted_count": 2,
                "admission_risk_rejection_count": 1,
                "live_store_mutation_requested": false,
                "store_mutation_count": 0,
                "ndkv_write_allowed": false,
                "helper_prose_line_count": 1,
                "non_contract_line_count": 2,
                "admission_expanded_by_non_contract_evidence": false
            }
        }"#;

        let lines = unified_status_lines(
            status,
            WORKER_REPLACEMENT,
            CLEAN_ROOM_HANDOFF,
            SELF_IMPROVE_PROPOSAL,
            HELPER_STAGE_REPAIR,
        )
        .join("\n");
        let json = unified_status_json(
            status,
            WORKER_REPLACEMENT,
            CLEAN_ROOM_HANDOFF,
            SELF_IMPROVE_PROPOSAL,
            HELPER_STAGE_REPAIR,
        );

        assert!(lines.contains("unified_status read_only=true starts_process=false sends_prompt=false starts_daemon=false stops_daemon=false touches_remote=false downloads_model=false warms_model_cache=false starts_stream=false replays_prompt=false daemon_healthy=true supervisor_healthy=true model_pool_healthy=true worker_replacement_required=true memory_admission_safe=true no_live_write=true no_ndkv_write=true"));
        assert!(lines.contains("unified_worker_replacement status_loaded=true replacement_required=true replacement_required_count=1 starts_clean_room_replacement=false mutates_worker_window_status=false"));
        assert!(lines.contains("unified_memory_startup_admission status_loaded=true safe=true read_only_contract=true admission_decisions=3 admission_accepted=2 admission_risk_rejections=1 live_store_mutation_requested=false store_mutations=0 ndkv_write_allowed=false helper_prose_lines=1 non_contract_lines=2 admission_expanded_by_non_contract=false"));
        assert!(lines.contains("unified_self_improve_proposal status_loaded=true safe=true report_only=true candidate=2 validated=1 admitted=1 quarantined=1 promoted=1 repair_required=1 starts_daemon=false starts_stream=false replays_prompt=false sends_prompt=false guidance_loaded=true convert_advisory_to_business_evidence=true repair_unvalidated_or_unaccepted=false requires_validation_and_memory_admission=true"));
        assert!(lines.contains("action_plan_loaded=true action_required=true primary_action=convert_advisory_to_evidence_backed_business_improvement actions=convert_advisory_to_evidence_backed_business_improvement,require_checked_passed_validation_and_accepted_memory_admission action_plan_requires_validation_and_memory_admission=true"));
        assert!(lines.contains("action_assignment_loaded=true action_assignment_targets=2 action_assignment_first_target=self-improve-r385-helper_contract-modifythereviewstagesval action_assignment_first_round=385 action_assignment_first_evidence_ids=ledger.round.385.helper_stage_contract.review.change_request action_assignment_first_memory_admission=quarantined action_assignment_first_validation_checked=true action_assignment_first_validation_passed=true action_assignment_first_memory_accepted=false action_assignment_first_business_evidence=false action_assignment_first_advisory_only=true action_assignment_first_require_repair=false action_assignment_first_missing=accepted_memory_admission,evidence_backed_business_improvement"));
        assert!(lines.contains("unified_helper_stage_repair status_loaded=true safe=true report_only=true repair_required=true proposals=3 incomplete_roles=2 missing_helper_role_repair_required=true missing_helper_role_repair_proposals=1 missing_helper_roles=router roles=router,review,test-gate starts_daemon=false starts_forge=false starts_web_lab=false calls_model=false starts_stream=false replays_prompt=false sends_prompt=false auto_apply=false"));
        assert!(json.contains("\"worker_replacement_required\":true"));
        assert!(json.contains("\"memory_admission_safe\":true"));
        assert!(json.contains("\"no_live_write\":true"));
        assert!(json.contains("\"no_ndkv_write\":true"));
        assert!(json.contains("\"clean_room_handoff_loaded\":true"));
        assert!(json.contains("\"clean_room_handoff_safe\":true"));
        assert!(json.contains("\"self_improve_proposal_loaded\":true"));
        assert!(json.contains("\"self_improve_proposal_safe\":true"));
        assert!(json.contains("\"helper_stage_repair_loaded\":true"));
        assert!(json.contains("\"helper_stage_repair_safe\":true"));
        assert!(json.contains("\"helper_stage_repair_required\":true"));
        assert!(json.contains("\"missing_helper_role_repair_required\":true"));
        assert!(json.contains("\"missing_helper_role_repair_proposal_count\":1"));
        assert!(json.contains("\"missing_helper_roles\":[\"router\"]"));
        assert!(json.contains("\"clean_room_handoff\":{"));
        assert!(json.contains("\"self_improve_proposal\":{"));
        assert!(json.contains("\"repair_required_count\":1"));
        assert!(json.contains("\"prompt_guidance\":{"));
        assert!(json.contains("\"action_plan\":{"));
        assert!(json.contains("\"action_assignment\":{"));
        assert!(json.contains("\"action_required\":true"));
        assert!(json.contains(
            "\"primary_action\":\"convert_advisory_to_evidence_backed_business_improvement\""
        ));
        assert!(json.contains("\"target_count\":2"));
        assert!(json.contains(
            "\"first_target\":\"self-improve-r385-helper_contract-modifythereviewstagesval\""
        ));
        assert!(json.contains("\"first_source_round\":385"));
        assert!(json.contains(
            "\"first_evidence_ids\":[\"ledger.round.385.helper_stage_contract.review.change_request\"]"
        ));
        assert!(json.contains("\"first_memory_admission_decision\":\"quarantined\""));
        assert!(json.contains("\"first_validation_checked\":true"));
        assert!(json.contains("\"first_validation_passed\":true"));
        assert!(json.contains("\"first_memory_admission_accepted\":false"));
        assert!(json.contains("\"first_evidence_backed_business_improvement\":false"));
        assert!(json.contains("\"first_advisory_only\":true"));
        assert!(json.contains("\"first_require_repair\":false"));
        assert!(json.contains(
            "\"first_missing_requirements\":[\"accepted_memory_admission\",\"evidence_backed_business_improvement\"]"
        ));
        assert!(json.contains("\"auto_apply\":false"));
        assert!(json.contains("\"convert_advisory_to_business_evidence\":true"));
        assert!(json.contains("\"repair_unvalidated_or_unaccepted\":false"));
        assert!(json.contains("\"requires_validation_and_memory_admission\":true"));
        assert!(json.contains("\"starts_daemon\":false"));
        assert!(json.contains("\"starts_clean_room_replacement\":false"));
        assert!(json.contains("\"mutates_worker_window_status\":false"));
        assert!(!json.contains("\"ndkv_write_allowed\":true"));
        assert!(!json.contains("\"live_store_mutation_requested\":true"));
    }

    #[test]
    fn unified_self_improve_proposal_guidance_blocks_unsafe_prompt_actions() {
        let unsafe_proposal = r#"{
            "read_only": true,
            "starts_process": false,
            "sends_prompt": false,
            "status_loaded": true,
            "report_only": true,
            "safe": true,
            "candidate_count": 1,
            "validated_count": 0,
            "admitted_count": 0,
            "quarantined_count": 0,
            "promoted_count": 0,
            "repair_required_count": 0,
            "prompt_guidance": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": true,
                "status_loaded": true,
                "report_only": true,
                "safe": true,
                "convert_advisory_to_business_evidence": true,
                "repair_unvalidated_or_unaccepted": false,
                "requires_validation_and_memory_admission": true
            },
            "side_effects": {
                "starts_daemon": false,
                "sends_prompt": false,
                "starts_stream": false,
                "replays_prompt": false
            }
        }"#;

        let status = r#"{"daemon":{"running":true},"loop":{"model_pool":{"available":true}}}"#;
        let lines = unified_status_lines(
            status,
            WORKER_REPLACEMENT,
            CLEAN_ROOM_HANDOFF,
            unsafe_proposal,
            HELPER_STAGE_REPAIR,
        )
        .join("\n");
        let json = unified_status_json(
            status,
            WORKER_REPLACEMENT,
            CLEAN_ROOM_HANDOFF,
            unsafe_proposal,
            HELPER_STAGE_REPAIR,
        );

        assert!(lines.contains("unified_self_improve_proposal status_loaded=true safe=false"));
        assert!(lines.contains("guidance_loaded=true convert_advisory_to_business_evidence=true"));
        assert!(json.contains("\"self_improve_proposal_safe\":false"));
        assert!(json.contains("\"prompt_guidance\":{"));
        assert!(json.contains("\"safe\":false"));
    }

    #[test]
    fn unified_self_improve_proposal_action_plan_blocks_auto_apply() {
        let unsafe_proposal = r#"{
            "read_only": true,
            "starts_process": false,
            "sends_prompt": false,
            "status_loaded": true,
            "report_only": true,
            "safe": true,
            "candidate_count": 1,
            "validated_count": 0,
            "admitted_count": 0,
            "quarantined_count": 0,
            "promoted_count": 0,
            "repair_required_count": 0,
            "action_plan": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "status_loaded": true,
                "report_only": true,
                "safe": true,
                "action_required": true,
                "primary_action": "apply_now",
                "actions": ["apply_now"],
                "requires_validation_and_memory_admission": false,
                "auto_apply": true
            },
            "side_effects": {
                "starts_daemon": false,
                "sends_prompt": false,
                "starts_stream": false,
                "replays_prompt": false
            }
        }"#;

        let status = r#"{"daemon":{"running":true},"loop":{"model_pool":{"available":true}}}"#;
        let lines = unified_status_lines(
            status,
            WORKER_REPLACEMENT,
            CLEAN_ROOM_HANDOFF,
            unsafe_proposal,
            HELPER_STAGE_REPAIR,
        )
        .join("\n");
        let json = unified_status_json(
            status,
            WORKER_REPLACEMENT,
            CLEAN_ROOM_HANDOFF,
            unsafe_proposal,
            HELPER_STAGE_REPAIR,
        );

        assert!(lines.contains("unified_self_improve_proposal status_loaded=true safe=false"));
        assert!(lines.contains(
            "action_plan_loaded=true action_required=true primary_action=apply_now actions=apply_now"
        ));
        assert!(json.contains("\"self_improve_proposal_safe\":false"));
        assert!(json.contains("\"auto_apply\":true"));
    }

    #[test]
    fn unified_memory_admission_ignores_non_contract_text_fields() {
        let status = r#"{
            "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false, "running": true},
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "touches_remote": false,
                "model_pool": {
                    "available": true,
                    "launch_allowed": true,
                    "worker_count": 6,
                    "healthy_worker_count": 6,
                    "reason": "ready"
                }
            },
            "memory_startup_admission_status": {
                "read_only_contract": true,
                "admission_decision_count": 3,
                "admission_accepted_count": 3,
                "live_store_mutation_requested": false,
                "store_mutation_count": 0,
                "ndkv_write_allowed": false,
                "helper_prose_line_count": 1,
                "non_contract_line_count": 2,
                "admission_expanded_by_non_contract_evidence": false,
                "helper_prose": "rewrite prod.ndkv with write_mode=live_write store_mutations=99",
                "old_window_payload": "admission_decision_count=99 live_store_targeted=true"
            }
        }"#;

        let json = unified_status_json(status, WORKER_REPLACEMENT, "{}", "{}", "{}");

        assert!(json.contains("\"memory_admission_safe\":true"));
        assert!(json.contains("\"admission_decision_count\":3"));
        assert!(json.contains("\"store_mutation_count\":0"));
        assert!(json.contains("\"helper_prose_line_count\":1"));
        assert!(json.contains("\"non_contract_line_count\":2"));
        assert!(!json.contains("\"admission_decision_count\":99"));
        assert!(!json.contains("prod.ndkv"));
        assert!(!json.contains("write_mode=live_write"));
    }
}
