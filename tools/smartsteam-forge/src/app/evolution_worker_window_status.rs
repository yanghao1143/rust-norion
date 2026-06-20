use std::collections::BTreeMap;

use super::status_json::{
    bool_value_text, compact_line, json_bool_field, json_object_array_field, json_object_field,
    json_object_keys, json_string_array_field, json_string_field, json_string_literal,
    json_top_level_object_field, json_top_level_string_array_field, json_top_level_string_field,
    scalar_value,
};

pub(super) fn worker_window_status_lines(loop_status: Option<&str>) -> Vec<String> {
    let Some(status) = WorkerWindowStatus::from_loop_status(loop_status) else {
        return Vec::new();
    };

    let mut lines = vec![format!(
        "worker_window_status read_only={} starts_process={} sends_prompt={} starts_clean_room_replacement={} mutates_worker_window_status={} total={} replacement_required={} statuses={}",
        bool_value_text(status.read_only),
        bool_value_text(status.starts_process),
        bool_value_text(status.sends_prompt),
        bool_value_text(status.starts_clean_room_replacement),
        bool_value_text(status.mutates_worker_window_status),
        status.rows.len(),
        status.replacement_required_count(),
        status.status_counts_text()
    )];

    for row in &status.rows {
        lines.push(format!(
            "worker_window id={} status={} paused={} polluted={} stale={} archived={} completed_evidence_only={} clean_room_replacement={} assignment_allowed={} original_window_blocks_assignment={} clean_room_replacement_required={} future_work_requires_fresh_clean_room={} replacement_window_id={} reason_codes={} business_task_ids={} evidence_result_ids={}",
            row.id_line_value(),
            row.status_line_value(),
            bool_value_text(row.paused),
            bool_value_text(row.polluted),
            bool_value_text(row.stale),
            bool_value_text(row.archived),
            bool_value_text(row.completed_evidence_only),
            bool_value_text(row.clean_room_replacement),
            bool_value_text(row.assignment_allowed),
            bool_value_text(row.original_window_blocks_assignment),
            bool_value_text(row.clean_room_replacement_required),
            bool_value_text(row.future_work_requires_fresh_clean_room),
            row.replacement_window_id.as_deref().unwrap_or("none"),
            list_line_value(&row.reason_codes),
            list_line_value(&row.business_task_ids),
            list_line_value(&row.evidence_result_ids)
        ));
    }

    lines
}

pub(super) fn worker_window_status_json(loop_status: Option<&str>) -> String {
    WorkerWindowStatus::from_loop_status(loop_status)
        .unwrap_or_default()
        .to_json()
}

pub(super) fn daemon_round_transition_status_lines(loop_status: Option<&str>) -> Vec<String> {
    let Some(status) = DaemonRoundTransitionStatus::from_loop_status(loop_status) else {
        return Vec::new();
    };

    vec![format!(
        "daemon_round_transition status={} latest_round_state={} round_in_progress={} read_only={} starts_process={} report_only={} observed_round_done={} active_round={} done_round={} ledger_round={} ledger_commit_pending={} ledger_lag_rounds={} starts_daemon={} stops_daemon={} touches_remote={} sends_prompt={} starts_stream={} replays_prompt={} mutates_active_round={} writes_ndkv={} activity_reason={} evidence_ids={} reason_codes={}",
        status.status_line_value(),
        status.latest_round_state_line_value(),
        bool_value_text(status.round_in_progress),
        bool_value_text(status.read_only),
        bool_value_text(status.starts_process),
        bool_value_text(status.report_only),
        bool_value_text(status.observed_round_done),
        status.active_round,
        status.done_round,
        status.ledger_round,
        bool_value_text(status.ledger_commit_pending),
        status.ledger_lag_rounds,
        bool_value_text(status.starts_daemon),
        bool_value_text(status.stops_daemon),
        bool_value_text(status.touches_remote),
        bool_value_text(status.sends_prompt),
        bool_value_text(status.starts_stream),
        bool_value_text(status.replays_prompt),
        bool_value_text(status.mutates_active_round),
        bool_value_text(status.writes_ndkv),
        status.activity_reason_line_value(),
        list_line_value(&status.evidence_ids),
        list_line_value(&status.reason_codes)
    )]
}

pub(super) fn daemon_round_transition_status_json(loop_status: Option<&str>) -> String {
    DaemonRoundTransitionStatus::from_loop_status(loop_status)
        .unwrap_or_default()
        .to_json()
}

pub(super) fn context_hygiene_status_lines(loop_status: Option<&str>) -> Vec<String> {
    let Some(status) = ContextHygieneStatus::from_loop_status(loop_status) else {
        return Vec::new();
    };

    vec![format!(
        "context_hygiene_status read_only={} starts_process={} sends_prompt={} report_only={} completed_window_evidence_non_actionable={} future_work_requires_fresh_clean_room={} reads_old_window_payload={} reason_codes={}",
        bool_value_text(status.read_only),
        bool_value_text(status.starts_process),
        bool_value_text(status.sends_prompt),
        bool_value_text(status.report_only),
        bool_value_text(status.completed_window_evidence_non_actionable),
        bool_value_text(status.future_work_requires_fresh_clean_room),
        bool_value_text(status.reads_old_window_payload),
        list_line_value(&status.reason_codes)
    )]
}

pub(super) fn context_hygiene_status_json(loop_status: Option<&str>) -> String {
    ContextHygieneStatus::from_loop_status(loop_status)
        .unwrap_or_default()
        .to_json()
}

pub(super) fn next_round_decision_status_lines(loop_status: Option<&str>) -> Vec<String> {
    let Some(status) = NextRoundDecisionStatus::from_loop_status(loop_status) else {
        return Vec::new();
    };

    vec![format!(
        "next_round_decision_status decision={} read_only={} starts_process={} sends_prompt={} report_only={} starts_daemon={} stops_daemon={} touches_remote={} starts_stream={} replays_prompt={} writes_ndkv={} active_round={} done_round={} ledger_round={} reason_codes={} evidence_ids={}",
        status.decision_line_value(),
        bool_value_text(status.read_only),
        bool_value_text(status.starts_process),
        bool_value_text(status.sends_prompt),
        bool_value_text(status.report_only),
        bool_value_text(status.starts_daemon),
        bool_value_text(status.stops_daemon),
        bool_value_text(status.touches_remote),
        bool_value_text(status.starts_stream),
        bool_value_text(status.replays_prompt),
        bool_value_text(status.writes_ndkv),
        status.active_round,
        status.done_round,
        status.ledger_round,
        list_line_value(&status.reason_codes),
        list_line_value(&status.evidence_ids)
    )]
}

pub(super) fn next_round_decision_status_json(loop_status: Option<&str>) -> String {
    NextRoundDecisionStatus::from_loop_status(loop_status)
        .unwrap_or_default()
        .to_json()
}

pub(super) fn next_round_downstream_status_consumers_lines(status: Option<&str>) -> Vec<String> {
    let Some(status) = NextRoundDownstreamStatusConsumers::from_status(status) else {
        return Vec::new();
    };

    let mut lines = vec![format!(
        "next_round_downstream_status_consumers read_only={} starts_process={} sends_prompt={} report_only={} side_effects={} starts_daemon={} stops_daemon={} touches_remote={} starts_stream={} replays_prompt={} writes_ndkv={} active_round={} done_round={} ledger_round={} round_id_evidence_active_round={} round_id_evidence_done_round={} round_id_evidence_ledger_round={} round_id_evidence_source_schema={} consumers={} reason_codes={} evidence_ids={}",
        bool_value_text(status.read_only),
        bool_value_text(status.starts_process),
        bool_value_text(status.sends_prompt),
        bool_value_text(status.report_only),
        bool_value_text(status.side_effects),
        bool_value_text(status.starts_daemon),
        bool_value_text(status.stops_daemon),
        bool_value_text(status.touches_remote),
        bool_value_text(status.starts_stream),
        bool_value_text(status.replays_prompt),
        bool_value_text(status.writes_ndkv),
        status.active_round,
        status.done_round,
        status.ledger_round,
        status.round_id_evidence.active_round,
        status.round_id_evidence.done_round,
        status.round_id_evidence.ledger_round,
        status
            .round_id_evidence
            .source_schema
            .as_deref()
            .unwrap_or("unknown"),
        list_line_value(&status.consumer_ids()),
        list_line_value(&status.reason_codes),
        list_line_value(&status.evidence_ids)
    )];

    for consumer in &status.consumers {
        lines.push(format!(
            "next_round_downstream_consumer id={} required={} satisfied={} read_only={} report_only={} side_effects={} starts_process={} sends_prompt={} starts_stream={} replays_prompt={} writes_ndkv={} reason_codes={} evidence_ids={}",
            consumer.id,
            bool_value_text(consumer.required),
            bool_value_text(consumer.satisfied),
            bool_value_text(consumer.read_only),
            bool_value_text(consumer.report_only),
            bool_value_text(consumer.side_effects),
            bool_value_text(consumer.starts_process),
            bool_value_text(consumer.sends_prompt),
            bool_value_text(consumer.starts_stream),
            bool_value_text(consumer.replays_prompt),
            bool_value_text(consumer.writes_ndkv),
            list_line_value(&consumer.reason_codes),
            list_line_value(&consumer.evidence_ids)
        ));
    }

    lines
}

pub(super) fn next_round_downstream_status_consumers_json(status: Option<&str>) -> String {
    NextRoundDownstreamStatusConsumers::from_status(status)
        .unwrap_or_default()
        .to_json()
}

pub(super) fn worker_window_replacement_report_lines(report_json: &str) -> Vec<String> {
    let Some(report) = WorkerWindowReplacementReport::from_report_json(Some(report_json)) else {
        return Vec::new();
    };

    let mut lines = vec![format!(
        "worker_window_replacement_report read_only={} starts_process={} sends_prompt={} status_loaded={} total={} paused={} polluted={} clean_room_replacement={} replacement_required={} blocked_original={} starts_clean_room_replacement={} mutates_worker_window_status={} source={}",
        bool_value_text(report.read_only),
        bool_value_text(report.starts_process),
        bool_value_text(report.sends_prompt),
        bool_value_text(report.status_loaded),
        report.window_count,
        report.paused_count,
        report.polluted_count,
        report.clean_room_replacement_count,
        report.replacement_required_count,
        report.blocked_original_count,
        bool_value_text(report.starts_clean_room_replacement),
        bool_value_text(report.mutates_worker_window_status),
        report.source.as_deref().unwrap_or("unknown")
    )];

    for row in &report.rows {
        lines.push(format!(
            "worker_window_report_source id={} status={} paused={} polluted={} stale={} archived={} completed_evidence_only={} clean_room_replacement={} assignment_allowed={} original_window_blocks_assignment={} clean_room_replacement_required={} future_work_requires_fresh_clean_room={} replacement_window_id={} reason_codes={} business_task_ids={} evidence_result_ids={}",
            row.id_line_value(),
            row.status_line_value(),
            bool_value_text(row.paused),
            bool_value_text(row.polluted),
            bool_value_text(row.stale),
            bool_value_text(row.archived),
            bool_value_text(row.completed_evidence_only),
            bool_value_text(row.clean_room_replacement),
            bool_value_text(row.assignment_allowed),
            bool_value_text(row.original_window_blocks_assignment),
            bool_value_text(row.clean_room_replacement_required),
            bool_value_text(row.future_work_requires_fresh_clean_room),
            row.replacement_window_id.as_deref().unwrap_or("none"),
            list_line_value(&row.reason_codes),
            list_line_value(&row.business_task_ids),
            list_line_value(&row.evidence_result_ids)
        ));
    }

    lines
}

pub(super) fn worker_window_replacement_report_json(report_json: Option<&str>) -> String {
    WorkerWindowReplacementReport::from_report_json(report_json)
        .unwrap_or_default()
        .to_json()
}

struct WorkerWindowStatus {
    read_only: bool,
    starts_process: bool,
    sends_prompt: bool,
    starts_clean_room_replacement: bool,
    mutates_worker_window_status: bool,
    rows: Vec<WorkerWindowRow>,
}

impl Default for WorkerWindowStatus {
    fn default() -> Self {
        Self {
            read_only: true,
            starts_process: false,
            sends_prompt: false,
            starts_clean_room_replacement: false,
            mutates_worker_window_status: false,
            rows: Vec::new(),
        }
    }
}

impl WorkerWindowStatus {
    fn from_loop_status(loop_status: Option<&str>) -> Option<Self> {
        let loop_status = loop_status?;
        let container = json_object_field(loop_status, "worker_window_status")
            .or_else(|| json_object_field(loop_status, "worker_windows"))
            .unwrap_or(loop_status);
        let rows = worker_window_rows(container);
        let has_contract = json_bool_field(container, "read_only").is_some()
            || json_bool_field(container, "starts_clean_room_replacement").is_some()
            || json_bool_field(container, "mutates_worker_window_status").is_some();

        if rows.is_empty() && !has_contract {
            return None;
        }

        Some(Self {
            read_only: json_bool_field(container, "read_only").unwrap_or(true),
            starts_process: json_bool_field(container, "starts_process").unwrap_or(false),
            sends_prompt: json_bool_field(container, "sends_prompt").unwrap_or(false),
            starts_clean_room_replacement: json_bool_field(
                container,
                "starts_clean_room_replacement",
            )
            .unwrap_or(false),
            mutates_worker_window_status: json_bool_field(
                container,
                "mutates_worker_window_status",
            )
            .unwrap_or(false),
            rows,
        })
    }

    fn replacement_required_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.clean_room_replacement_required)
            .count()
    }

    fn status_counts_text(&self) -> String {
        if self.rows.is_empty() {
            return "none".to_owned();
        }

        let mut counts = BTreeMap::<&str, usize>::new();
        for row in &self.rows {
            *counts.entry(row.status_line_value()).or_default() += 1;
        }

        counts
            .into_iter()
            .map(|(status, count)| format!("{status}:{count}"))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn to_json(&self) -> String {
        let rows = self
            .rows
            .iter()
            .map(WorkerWindowRow::to_json)
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "{{\"read_only\":{},\"starts_process\":{},\"sends_prompt\":{},\"starts_clean_room_replacement\":{},\"mutates_worker_window_status\":{},\"total\":{},\"clean_room_replacement_required_count\":{},\"status_counts\":{},\"rows\":[{}]}}",
            bool_value_text(self.read_only),
            bool_value_text(self.starts_process),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.starts_clean_room_replacement),
            bool_value_text(self.mutates_worker_window_status),
            self.rows.len(),
            self.replacement_required_count(),
            json_string_literal(&self.status_counts_text()),
            rows
        )
    }
}

struct WorkerWindowReplacementReport {
    read_only: bool,
    starts_process: bool,
    sends_prompt: bool,
    status_loaded: bool,
    source: Option<String>,
    source_path: Option<String>,
    side_effects_allowed: Option<bool>,
    starts_clean_room_replacement: bool,
    mutates_worker_window_status: bool,
    window_count: String,
    paused_count: String,
    polluted_count: String,
    clean_room_replacement_count: String,
    replacement_required_count: String,
    blocked_original_count: String,
    rows: Vec<WorkerWindowRow>,
}

struct DaemonRoundTransitionStatus {
    read_only: bool,
    starts_process: bool,
    report_only: bool,
    observed_round_done: bool,
    latest_round_state: Option<String>,
    round_in_progress: bool,
    active_round: String,
    done_round: String,
    ledger_round: String,
    ledger_commit_pending: bool,
    ledger_lag_rounds: String,
    status: Option<String>,
    activity_reason: Option<String>,
    evidence_ids: Vec<String>,
    reason_codes: Vec<String>,
    starts_daemon: bool,
    stops_daemon: bool,
    touches_remote: bool,
    sends_prompt: bool,
    starts_stream: bool,
    replays_prompt: bool,
    mutates_active_round: bool,
    writes_ndkv: bool,
}

struct ContextHygieneStatus {
    read_only: bool,
    starts_process: bool,
    sends_prompt: bool,
    report_only: bool,
    completed_window_evidence_non_actionable: bool,
    future_work_requires_fresh_clean_room: bool,
    reads_old_window_payload: bool,
    reason_codes: Vec<String>,
}

struct NextRoundDecisionStatus {
    read_only: bool,
    starts_process: bool,
    sends_prompt: bool,
    report_only: bool,
    decision: Option<String>,
    starts_daemon: bool,
    stops_daemon: bool,
    touches_remote: bool,
    starts_stream: bool,
    replays_prompt: bool,
    writes_ndkv: bool,
    active_round: String,
    done_round: String,
    ledger_round: String,
    reason_codes: Vec<String>,
    evidence_ids: Vec<String>,
}

struct NextRoundDownstreamStatusConsumers {
    read_only: bool,
    starts_process: bool,
    sends_prompt: bool,
    report_only: bool,
    side_effects: bool,
    starts_daemon: bool,
    stops_daemon: bool,
    touches_remote: bool,
    starts_stream: bool,
    replays_prompt: bool,
    writes_ndkv: bool,
    active_round: String,
    done_round: String,
    ledger_round: String,
    round_id_evidence: RoundIdEvidence,
    reason_codes: Vec<String>,
    evidence_ids: Vec<String>,
    consumers: Vec<DownstreamStatusConsumer>,
}

struct RoundIdEvidence {
    active_round: String,
    done_round: String,
    ledger_round: String,
    source_schema: Option<String>,
    transition_kind: Option<String>,
    reason_codes: Vec<String>,
    evidence_ids: Vec<String>,
}

struct DownstreamStatusConsumer {
    id: String,
    required: bool,
    satisfied: bool,
    read_only: bool,
    report_only: bool,
    side_effects: bool,
    starts_process: bool,
    sends_prompt: bool,
    starts_stream: bool,
    replays_prompt: bool,
    writes_ndkv: bool,
    reason_codes: Vec<String>,
    evidence_ids: Vec<String>,
}

impl Default for DaemonRoundTransitionStatus {
    fn default() -> Self {
        Self {
            read_only: true,
            starts_process: false,
            report_only: true,
            observed_round_done: false,
            latest_round_state: None,
            round_in_progress: false,
            active_round: "unknown".to_owned(),
            done_round: "unknown".to_owned(),
            ledger_round: "unknown".to_owned(),
            ledger_commit_pending: false,
            ledger_lag_rounds: "unknown".to_owned(),
            status: None,
            activity_reason: None,
            evidence_ids: Vec::new(),
            reason_codes: Vec::new(),
            starts_daemon: false,
            stops_daemon: false,
            touches_remote: false,
            sends_prompt: false,
            starts_stream: false,
            replays_prompt: false,
            mutates_active_round: false,
            writes_ndkv: false,
        }
    }
}

impl Default for ContextHygieneStatus {
    fn default() -> Self {
        Self {
            read_only: true,
            starts_process: false,
            sends_prompt: false,
            report_only: true,
            completed_window_evidence_non_actionable: false,
            future_work_requires_fresh_clean_room: false,
            reads_old_window_payload: false,
            reason_codes: Vec::new(),
        }
    }
}

impl Default for NextRoundDecisionStatus {
    fn default() -> Self {
        Self {
            read_only: true,
            starts_process: false,
            sends_prompt: false,
            report_only: true,
            decision: None,
            starts_daemon: false,
            stops_daemon: false,
            touches_remote: false,
            starts_stream: false,
            replays_prompt: false,
            writes_ndkv: false,
            active_round: "unknown".to_owned(),
            done_round: "unknown".to_owned(),
            ledger_round: "unknown".to_owned(),
            reason_codes: Vec::new(),
            evidence_ids: Vec::new(),
        }
    }
}

impl Default for NextRoundDownstreamStatusConsumers {
    fn default() -> Self {
        Self {
            read_only: true,
            starts_process: false,
            sends_prompt: false,
            report_only: true,
            side_effects: false,
            starts_daemon: false,
            stops_daemon: false,
            touches_remote: false,
            starts_stream: false,
            replays_prompt: false,
            writes_ndkv: false,
            active_round: "unknown".to_owned(),
            done_round: "unknown".to_owned(),
            ledger_round: "unknown".to_owned(),
            round_id_evidence: RoundIdEvidence::default(),
            reason_codes: Vec::new(),
            evidence_ids: Vec::new(),
            consumers: Vec::new(),
        }
    }
}

impl Default for RoundIdEvidence {
    fn default() -> Self {
        Self {
            active_round: "unknown".to_owned(),
            done_round: "unknown".to_owned(),
            ledger_round: "unknown".to_owned(),
            source_schema: None,
            transition_kind: None,
            reason_codes: Vec::new(),
            evidence_ids: Vec::new(),
        }
    }
}

impl DaemonRoundTransitionStatus {
    fn from_loop_status(loop_status: Option<&str>) -> Option<Self> {
        let loop_status = loop_status?;
        let container = json_object_field(loop_status, "daemon_round_transition_status_v1")
            .or_else(|| json_object_field(loop_status, "daemon_round_transition_status"))
            .or_else(|| json_object_field(loop_status, "daemon_round_transition"))
            .or_else(|| json_object_field(loop_status, "round_done_ledger_commit_pending"))
            .unwrap_or(loop_status);
        let has_transition = json_bool_field(container, "observed_round_done").is_some()
            || json_bool_field(container, "ledger_commit_pending").is_some()
            || json_bool_field(container, "round_in_progress").is_some()
            || string_field_any(container, &["latest_round_state", "round_state"]).is_some()
            || scalar_value_any(container, &["done_round", "latest_done_round"]) != "unknown";
        if !has_transition {
            return None;
        }

        let side_effects = json_object_field(container, "side_effects");
        let transition_kind = string_field_any(container, &["transition_kind"]);
        let status =
            string_field_any(container, &["status", "status_label"]).or(transition_kind.clone());
        let ledger_commit_pending =
            json_bool_field(container, "ledger_commit_pending").unwrap_or(false);
        let explicit_round_in_progress = json_bool_field(container, "round_in_progress");
        let latest_round_state =
            string_field_any(container, &["latest_round_state", "round_state"])
                .or_else(|| normalized_round_state(status.as_deref()))
                .or_else(|| normalized_round_state(transition_kind.as_deref()))
                .or_else(|| {
                    normalized_round_state_from_flags(
                        explicit_round_in_progress,
                        ledger_commit_pending,
                    )
                });
        let observed_round_done =
            json_bool_field(container, "observed_round_done").unwrap_or(ledger_commit_pending);
        let round_in_progress = explicit_round_in_progress
            .unwrap_or_else(|| latest_round_state.as_deref() == Some("round_in_progress"));
        Some(Self {
            read_only: json_bool_field(container, "read_only").unwrap_or(true),
            starts_process: json_bool_field(container, "starts_process").unwrap_or(false),
            report_only: json_bool_field(container, "report_only").unwrap_or(true),
            observed_round_done,
            latest_round_state,
            round_in_progress,
            active_round: scalar_value_any(container, &["active_round", "current_round"]),
            done_round: scalar_value_any(container, &["done_round", "latest_done_round"]),
            ledger_round: scalar_value_any(container, &["ledger_round", "ledger_latest_round"]),
            ledger_commit_pending,
            ledger_lag_rounds: scalar_value(container, "ledger_lag_rounds"),
            status,
            activity_reason: string_field_any(container, &["activity_reason", "reason"]),
            evidence_ids: string_array_field_any(
                container,
                &["evidence_ids", "evidence_result_ids"],
            ),
            reason_codes: string_array_field_any(container, &["reason_codes", "reasons"]),
            starts_daemon: bool_field_in(container, side_effects, "starts_daemon"),
            stops_daemon: bool_field_in(container, side_effects, "stops_daemon"),
            touches_remote: bool_field_in(container, side_effects, "touches_remote"),
            sends_prompt: bool_field_in(container, side_effects, "sends_prompt"),
            starts_stream: bool_field_in(container, side_effects, "starts_stream"),
            replays_prompt: bool_field_in(container, side_effects, "replays_prompt"),
            mutates_active_round: bool_field_in(container, side_effects, "mutates_active_round"),
            writes_ndkv: bool_field_in(container, side_effects, "writes_ndkv"),
        })
    }

    fn status_line_value(&self) -> &str {
        self.status.as_deref().unwrap_or("unknown")
    }

    fn latest_round_state_line_value(&self) -> &str {
        self.latest_round_state.as_deref().unwrap_or("unknown")
    }

    fn activity_reason_line_value(&self) -> &str {
        self.activity_reason.as_deref().unwrap_or("none")
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":{},\"starts_process\":{},\"report_only\":{},\"observed_round_done\":{},\"latest_round_state\":{},\"round_in_progress\":{},\"active_round\":{},\"done_round\":{},\"ledger_round\":{},\"ledger_commit_pending\":{},\"ledger_lag_rounds\":{},\"status\":{},\"activity_reason\":{},\"evidence_ids\":{},\"reason_codes\":{},\"starts_daemon\":{},\"stops_daemon\":{},\"touches_remote\":{},\"sends_prompt\":{},\"starts_stream\":{},\"replays_prompt\":{},\"mutates_active_round\":{},\"writes_ndkv\":{}}}",
            bool_value_text(self.read_only),
            bool_value_text(self.starts_process),
            bool_value_text(self.report_only),
            bool_value_text(self.observed_round_done),
            optional_string_json(self.latest_round_state.as_deref()),
            bool_value_text(self.round_in_progress),
            scalar_json(&self.active_round),
            scalar_json(&self.done_round),
            scalar_json(&self.ledger_round),
            bool_value_text(self.ledger_commit_pending),
            scalar_json(&self.ledger_lag_rounds),
            optional_string_json(self.status.as_deref()),
            optional_string_json(self.activity_reason.as_deref()),
            string_array_json(&self.evidence_ids),
            string_array_json(&self.reason_codes),
            bool_value_text(self.starts_daemon),
            bool_value_text(self.stops_daemon),
            bool_value_text(self.touches_remote),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.starts_stream),
            bool_value_text(self.replays_prompt),
            bool_value_text(self.mutates_active_round),
            bool_value_text(self.writes_ndkv)
        )
    }
}

impl ContextHygieneStatus {
    fn from_loop_status(loop_status: Option<&str>) -> Option<Self> {
        let loop_status = loop_status?;
        let container = json_object_field(loop_status, "context_hygiene_status")
            .or_else(|| json_object_field(loop_status, "context_hygiene"))?;
        let has_status = json_bool_field(container, "completed_window_evidence_non_actionable")
            .is_some()
            || json_bool_field(container, "future_work_requires_fresh_clean_room").is_some()
            || json_bool_field(container, "reads_old_window_payload").is_some();
        if !has_status {
            return None;
        }

        Some(Self {
            read_only: json_bool_field(container, "read_only").unwrap_or(true),
            starts_process: json_bool_field(container, "starts_process").unwrap_or(false),
            sends_prompt: json_bool_field(container, "sends_prompt").unwrap_or(false),
            report_only: json_bool_field(container, "report_only").unwrap_or(true),
            completed_window_evidence_non_actionable: json_bool_field(
                container,
                "completed_window_evidence_non_actionable",
            )
            .unwrap_or(false),
            future_work_requires_fresh_clean_room: json_bool_field(
                container,
                "future_work_requires_fresh_clean_room",
            )
            .unwrap_or(false),
            reads_old_window_payload: json_bool_field(container, "reads_old_window_payload")
                .unwrap_or(false),
            reason_codes: string_array_field_any(container, &["reason_codes", "reasons"]),
        })
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":{},\"starts_process\":{},\"sends_prompt\":{},\"report_only\":{},\"completed_window_evidence_non_actionable\":{},\"future_work_requires_fresh_clean_room\":{},\"reads_old_window_payload\":{},\"reason_codes\":{}}}",
            bool_value_text(self.read_only),
            bool_value_text(self.starts_process),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.report_only),
            bool_value_text(self.completed_window_evidence_non_actionable),
            bool_value_text(self.future_work_requires_fresh_clean_room),
            bool_value_text(self.reads_old_window_payload),
            string_array_json(&self.reason_codes)
        )
    }
}

impl NextRoundDecisionStatus {
    fn from_loop_status(loop_status: Option<&str>) -> Option<Self> {
        let loop_status = loop_status?;
        let container = next_round_decision_container(loop_status)?;
        let decision = string_field_any(
            container,
            &[
                "decision",
                "decision_status",
                "next_round_decision",
                "operator_decision",
                "display_state",
                "status",
                "verdict",
            ],
        )
        .and_then(|decision| normalized_next_round_decision(&decision));
        if decision.is_none()
            && json_bool_field(container, "read_only").is_none()
            && json_bool_field(container, "report_only").is_none()
        {
            return None;
        }

        let side_effects = json_object_field(container, "side_effects");
        Some(Self {
            read_only: json_bool_field(container, "read_only").unwrap_or(true),
            starts_process: bool_field_in_any(
                container,
                side_effects,
                &[
                    "starts_process",
                    "starts_processes",
                    "process_start_allowed",
                ],
            ),
            sends_prompt: bool_field_in_any(
                container,
                side_effects,
                &[
                    "sends_prompt",
                    "replays_prompt",
                    "replays_prompts",
                    "prompt_replay_allowed",
                ],
            ),
            report_only: json_bool_field(container, "report_only").unwrap_or(true),
            decision,
            starts_daemon: bool_field_in(container, side_effects, "starts_daemon"),
            stops_daemon: bool_field_in(container, side_effects, "stops_daemon"),
            touches_remote: bool_field_in_any(
                container,
                side_effects,
                &["touches_remote", "service_call", "remote_mac_call"],
            ),
            starts_stream: bool_field_in_any(
                container,
                side_effects,
                &[
                    "starts_stream",
                    "dispatches_work",
                    "dispatch_work_allowed",
                    "http_sse",
                ],
            ),
            replays_prompt: bool_field_in_any(
                container,
                side_effects,
                &["replays_prompt", "replays_prompts", "prompt_replay_allowed"],
            ),
            writes_ndkv: bool_field_in_any(
                container,
                side_effects,
                &["writes_ndkv", "ndkv_write_allowed", "jsonl_io"],
            ),
            active_round: scalar_value_any(container, &["active_round", "current_round"]),
            done_round: scalar_value_any(container, &["done_round", "latest_done_round"]),
            ledger_round: scalar_value_any(container, &["ledger_round", "ledger_latest_round"]),
            reason_codes: string_list_field_any(
                container,
                &["reason_codes", "reasons", "failure_reasons", "reason_code"],
            ),
            evidence_ids: string_array_field_any(
                container,
                &["evidence_ids", "evidence_result_ids"],
            ),
        })
    }

    fn decision_line_value(&self) -> &str {
        self.decision.as_deref().unwrap_or("unknown")
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":{},\"starts_process\":{},\"sends_prompt\":{},\"report_only\":{},\"decision\":{},\"starts_daemon\":{},\"stops_daemon\":{},\"touches_remote\":{},\"starts_stream\":{},\"replays_prompt\":{},\"writes_ndkv\":{},\"active_round\":{},\"done_round\":{},\"ledger_round\":{},\"reason_codes\":{},\"evidence_ids\":{}}}",
            bool_value_text(self.read_only),
            bool_value_text(self.starts_process),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.report_only),
            optional_string_json(self.decision.as_deref()),
            bool_value_text(self.starts_daemon),
            bool_value_text(self.stops_daemon),
            bool_value_text(self.touches_remote),
            bool_value_text(self.starts_stream),
            bool_value_text(self.replays_prompt),
            bool_value_text(self.writes_ndkv),
            scalar_json(&self.active_round),
            scalar_json(&self.done_round),
            scalar_json(&self.ledger_round),
            string_array_json(&self.reason_codes),
            string_array_json(&self.evidence_ids)
        )
    }
}

impl NextRoundDownstreamStatusConsumers {
    fn from_status(status: Option<&str>) -> Option<Self> {
        let status = status?;
        let container = next_round_downstream_status_consumers_container(status)?;
        let side_effects = json_object_field(container, "side_effects");
        let row_side_effects = json_bool_field(container, "side_effects").unwrap_or(false);
        let consumers = downstream_status_consumers(container);
        let has_status = json_bool_field(container, "read_only").is_some()
            || json_bool_field(container, "report_only").is_some()
            || json_object_field(container, "round_id_evidence").is_some()
            || !consumers.is_empty();

        if !has_status {
            return None;
        }

        let round_id_evidence = RoundIdEvidence::from_container(
            container,
            json_object_field(container, "round_id_evidence"),
        );

        Some(Self {
            read_only: json_bool_field(container, "read_only").unwrap_or(true),
            starts_process: bool_field_in_any(
                container,
                side_effects,
                &[
                    "starts_process",
                    "starts_processes",
                    "process_start_allowed",
                ],
            ),
            sends_prompt: bool_field_in_any(
                container,
                side_effects,
                &[
                    "sends_prompt",
                    "replays_prompt",
                    "replays_prompts",
                    "prompt_replay_allowed",
                ],
            ),
            report_only: json_bool_field(container, "report_only").unwrap_or(true),
            side_effects: row_side_effects
                || bool_field_in_any(
                    container,
                    side_effects,
                    &[
                        "starts_daemon",
                        "stops_daemon",
                        "touches_remote",
                        "starts_stream",
                        "replays_prompt",
                        "writes_ndkv",
                    ],
                ),
            starts_daemon: bool_field_in(container, side_effects, "starts_daemon"),
            stops_daemon: bool_field_in(container, side_effects, "stops_daemon"),
            touches_remote: bool_field_in(container, side_effects, "touches_remote"),
            starts_stream: bool_field_in_any(
                container,
                side_effects,
                &["starts_stream", "dispatches_work", "dispatch_work_allowed"],
            ),
            replays_prompt: bool_field_in_any(
                container,
                side_effects,
                &["replays_prompt", "replays_prompts", "prompt_replay_allowed"],
            ),
            writes_ndkv: bool_field_in_any(
                container,
                side_effects,
                &["writes_ndkv", "ndkv_write_allowed", "jsonl_io"],
            ),
            active_round: scalar_value_any(container, &["active_round", "current_round"]),
            done_round: scalar_value_any(container, &["done_round", "latest_done_round"]),
            ledger_round: scalar_value_any(container, &["ledger_round", "ledger_latest_round"]),
            round_id_evidence,
            reason_codes: string_list_top_level_field_any(
                container,
                &["reason_codes", "reasons", "failure_reasons", "reason_code"],
            ),
            evidence_ids: string_array_top_level_field_any(
                container,
                &["evidence_ids", "evidence_result_ids"],
            ),
            consumers,
        })
    }

    fn consumer_ids(&self) -> Vec<String> {
        self.consumers
            .iter()
            .map(|consumer| consumer.id.clone())
            .collect()
    }

    fn to_json(&self) -> String {
        let consumers = self
            .consumers
            .iter()
            .map(DownstreamStatusConsumer::to_json)
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "{{\"read_only\":{},\"starts_process\":{},\"sends_prompt\":{},\"report_only\":{},\"side_effects\":{},\"starts_daemon\":{},\"stops_daemon\":{},\"touches_remote\":{},\"starts_stream\":{},\"replays_prompt\":{},\"writes_ndkv\":{},\"active_round\":{},\"done_round\":{},\"ledger_round\":{},\"round_id_evidence\":{},\"reason_codes\":{},\"evidence_ids\":{},\"consumers\":[{}]}}",
            bool_value_text(self.read_only),
            bool_value_text(self.starts_process),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.report_only),
            bool_value_text(self.side_effects),
            bool_value_text(self.starts_daemon),
            bool_value_text(self.stops_daemon),
            bool_value_text(self.touches_remote),
            bool_value_text(self.starts_stream),
            bool_value_text(self.replays_prompt),
            bool_value_text(self.writes_ndkv),
            scalar_json(&self.active_round),
            scalar_json(&self.done_round),
            scalar_json(&self.ledger_round),
            self.round_id_evidence.to_json(),
            string_array_json(&self.reason_codes),
            string_array_json(&self.evidence_ids),
            consumers
        )
    }
}

impl RoundIdEvidence {
    fn from_container(container: &str, evidence: Option<&str>) -> Self {
        Self {
            active_round: scalar_value_any_in(
                container,
                evidence,
                &["active_round", "current_round"],
            ),
            done_round: scalar_value_any_in(
                container,
                evidence,
                &["done_round", "latest_done_round"],
            ),
            ledger_round: scalar_value_any_in(
                container,
                evidence,
                &["ledger_round", "ledger_latest_round"],
            ),
            source_schema: evidence.and_then(|value| string_field_any(value, &["source_schema"])),
            transition_kind: evidence
                .and_then(|value| string_field_any(value, &["transition_kind", "status"])),
            reason_codes: string_array_field_any_in(evidence, &["reason_codes", "reasons"]),
            evidence_ids: string_array_field_any_in(
                evidence,
                &["evidence_ids", "evidence_result_ids"],
            ),
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"active_round\":{},\"done_round\":{},\"ledger_round\":{},\"source_schema\":{},\"transition_kind\":{},\"reason_codes\":{},\"evidence_ids\":{}}}",
            scalar_json(&self.active_round),
            scalar_json(&self.done_round),
            scalar_json(&self.ledger_round),
            optional_string_json(self.source_schema.as_deref()),
            optional_string_json(self.transition_kind.as_deref()),
            string_array_json(&self.reason_codes),
            string_array_json(&self.evidence_ids)
        )
    }
}

impl DownstreamStatusConsumer {
    fn from_json(id: String, object: &str) -> Self {
        let side_effects = json_object_field(object, "side_effects");
        let starts_process = bool_field_in_any(
            object,
            side_effects,
            &[
                "starts_process",
                "starts_processes",
                "process_start_allowed",
            ],
        );
        let sends_prompt = bool_field_in_any(
            object,
            side_effects,
            &[
                "sends_prompt",
                "replays_prompt",
                "replays_prompts",
                "prompt_replay_allowed",
            ],
        );
        let starts_stream = bool_field_in_any(
            object,
            side_effects,
            &["starts_stream", "dispatches_work", "dispatch_work_allowed"],
        );
        let replays_prompt = bool_field_in_any(
            object,
            side_effects,
            &["replays_prompt", "replays_prompts", "prompt_replay_allowed"],
        );
        let writes_ndkv = bool_field_in_any(
            object,
            side_effects,
            &["writes_ndkv", "ndkv_write_allowed", "jsonl_io"],
        );
        let side_effect_flag = json_bool_field(object, "side_effects").unwrap_or(false)
            || starts_process
            || sends_prompt
            || starts_stream
            || replays_prompt
            || writes_ndkv;

        Self {
            id: string_field_any(
                object,
                &["consumer_id", "consumer", "surface", "name", "id"],
            )
            .unwrap_or(id),
            required: bool_field_any(object, &["required", "contract_required"]).unwrap_or(true),
            satisfied: bool_field_any(
                object,
                &[
                    "satisfied",
                    "ready",
                    "accepted",
                    "visible",
                    "display_ready",
                    "admission_visible",
                ],
            )
            .unwrap_or(true),
            read_only: json_bool_field(object, "read_only").unwrap_or(true),
            report_only: json_bool_field(object, "report_only").unwrap_or(true),
            side_effects: side_effect_flag,
            starts_process,
            sends_prompt,
            starts_stream,
            replays_prompt,
            writes_ndkv,
            reason_codes: string_list_field_any(
                object,
                &["reason_codes", "reasons", "failure_reasons", "reason_code"],
            ),
            evidence_ids: string_array_field_any(object, &["evidence_ids", "evidence_result_ids"]),
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"id\":{},\"required\":{},\"satisfied\":{},\"read_only\":{},\"starts_process\":{},\"sends_prompt\":{},\"report_only\":{},\"side_effects\":{},\"starts_stream\":{},\"replays_prompt\":{},\"writes_ndkv\":{},\"reason_codes\":{},\"evidence_ids\":{}}}",
            json_string_literal(&self.id),
            bool_value_text(self.required),
            bool_value_text(self.satisfied),
            bool_value_text(self.read_only),
            bool_value_text(self.starts_process),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.report_only),
            bool_value_text(self.side_effects),
            bool_value_text(self.starts_stream),
            bool_value_text(self.replays_prompt),
            bool_value_text(self.writes_ndkv),
            string_array_json(&self.reason_codes),
            string_array_json(&self.evidence_ids)
        )
    }
}

impl Default for WorkerWindowReplacementReport {
    fn default() -> Self {
        Self {
            read_only: true,
            starts_process: false,
            sends_prompt: false,
            status_loaded: false,
            source: None,
            source_path: None,
            side_effects_allowed: None,
            starts_clean_room_replacement: false,
            mutates_worker_window_status: false,
            window_count: "0".to_owned(),
            paused_count: "0".to_owned(),
            polluted_count: "0".to_owned(),
            clean_room_replacement_count: "0".to_owned(),
            replacement_required_count: "0".to_owned(),
            blocked_original_count: "0".to_owned(),
            rows: Vec::new(),
        }
    }
}

impl WorkerWindowReplacementReport {
    fn from_report_json(report_json: Option<&str>) -> Option<Self> {
        let report =
            json_top_level_object_field(report_json?, "worker_window_replacement_report_v1")?;
        let evidence = json_object_field(report, "evidence_map");
        let side_effects = json_object_field(report, "side_effects");
        let source_status = json_object_field(report, "source_status");
        let rows = source_status.map(worker_window_rows).unwrap_or_default();

        Some(Self {
            read_only: json_bool_field(report, "read_only").unwrap_or(true),
            starts_process: false,
            sends_prompt: false,
            status_loaded: json_bool_field(report, "status_loaded").unwrap_or(false),
            source: json_string_field(report, "source"),
            source_path: json_string_field(report, "source_path"),
            side_effects_allowed: evidence
                .and_then(|value| json_bool_field(value, "side_effects_allowed")),
            starts_clean_room_replacement: side_effects
                .and_then(|value| json_bool_field(value, "starts_clean_room_replacement"))
                .unwrap_or(false),
            mutates_worker_window_status: side_effects
                .and_then(|value| json_bool_field(value, "mutates_worker_window_status"))
                .unwrap_or(false),
            window_count: evidence_scalar(evidence, "window_count"),
            paused_count: evidence_scalar(evidence, "paused_count"),
            polluted_count: evidence_scalar(evidence, "polluted_count"),
            clean_room_replacement_count: evidence_scalar(evidence, "clean_room_replacement_count"),
            replacement_required_count: evidence_scalar(evidence, "replacement_required_count"),
            blocked_original_count: evidence_scalar(evidence, "blocked_original_count"),
            rows,
        })
    }

    fn to_json(&self) -> String {
        let rows = self
            .rows
            .iter()
            .map(WorkerWindowRow::to_json)
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "{{\"read_only\":{},\"starts_process\":{},\"sends_prompt\":{},\"status_loaded\":{},\"source\":{},\"source_path\":{},\"side_effects_allowed\":{},\"starts_clean_room_replacement\":{},\"mutates_worker_window_status\":{},\"window_count\":{},\"paused_count\":{},\"polluted_count\":{},\"clean_room_replacement_count\":{},\"replacement_required_count\":{},\"blocked_original_count\":{},\"rows\":[{}]}}",
            bool_value_text(self.read_only),
            bool_value_text(self.starts_process),
            bool_value_text(self.sends_prompt),
            bool_value_text(self.status_loaded),
            optional_string_json(self.source.as_deref()),
            optional_string_json(self.source_path.as_deref()),
            optional_bool_json(self.side_effects_allowed),
            bool_value_text(self.starts_clean_room_replacement),
            bool_value_text(self.mutates_worker_window_status),
            self.window_count,
            self.paused_count,
            self.polluted_count,
            self.clean_room_replacement_count,
            self.replacement_required_count,
            self.blocked_original_count,
            rows
        )
    }
}

#[derive(Default)]
struct WorkerWindowRow {
    id: Option<String>,
    status: Option<String>,
    paused: bool,
    polluted: bool,
    stale: bool,
    archived: bool,
    completed_evidence_only: bool,
    clean_room_replacement: bool,
    assignment_allowed: bool,
    original_window_blocks_assignment: bool,
    clean_room_replacement_required: bool,
    future_work_requires_fresh_clean_room: bool,
    replacement_window_id: Option<String>,
    reason_codes: Vec<String>,
    business_task_ids: Vec<String>,
    evidence_result_ids: Vec<String>,
}

impl WorkerWindowRow {
    fn from_json(object: &str) -> Self {
        let status = string_field_any(object, &["status", "context_status", "window_status"]);
        let paused = bool_field_any(object, &["paused"])
            .unwrap_or_else(|| status.as_deref() == Some("paused"));
        let polluted = bool_field_any(object, &["polluted"])
            .unwrap_or_else(|| status.as_deref() == Some("polluted"));
        let stale = bool_field_any(object, &["stale"])
            .unwrap_or_else(|| status.as_deref() == Some("stale"));
        let archived = bool_field_any(object, &["archived"])
            .unwrap_or_else(|| status.as_deref() == Some("archived"));
        let completed_evidence_only = bool_field_any(
            object,
            &["completed_evidence_only", "completion_evidence_only"],
        )
        .unwrap_or_else(|| status.as_deref() == Some("completed-evidence-only"));
        let clean_room_replacement = bool_field_any(
            object,
            &["clean_room_replacement", "fresh_clean_room_window"],
        )
        .unwrap_or_else(|| status.as_deref() == Some("clean-room-replacement"));
        let default_blocks_assignment =
            paused || polluted || stale || archived || completed_evidence_only;
        let original_window_blocks_assignment = bool_field_any(
            object,
            &[
                "original_window_blocks_assignment",
                "original_window_follow_up_blocked",
                "not_assignable",
            ],
        )
        .unwrap_or(default_blocks_assignment && !clean_room_replacement);
        let assignment_allowed = bool_field_any(object, &["assignment_allowed", "assignable"])
            .unwrap_or(!default_blocks_assignment || clean_room_replacement);
        let clean_room_replacement_required = bool_field_any(
            object,
            &[
                "clean_room_replacement_required",
                "replacement_required",
                "requires_clean_room_replacement",
            ],
        )
        .unwrap_or(original_window_blocks_assignment);
        let future_work_requires_fresh_clean_room = bool_field_any(
            object,
            &[
                "future_work_requires_fresh_clean_room",
                "requires_fresh_clean_room",
            ],
        )
        .unwrap_or(clean_room_replacement_required || original_window_blocks_assignment);

        Self {
            id: string_field_any(object, &["window_id", "id", "worker_window_id"]),
            status,
            paused,
            polluted,
            stale,
            archived,
            completed_evidence_only,
            clean_room_replacement,
            assignment_allowed,
            original_window_blocks_assignment,
            clean_room_replacement_required,
            future_work_requires_fresh_clean_room,
            replacement_window_id: string_field_any(
                object,
                &[
                    "replacement_window_id",
                    "clean_room_replacement_window_id",
                    "replaces_window_id",
                ],
            ),
            reason_codes: string_array_field_any(object, &["reason_codes", "reasons"]),
            business_task_ids: string_array_field_any(object, &["business_task_ids", "task_ids"]),
            evidence_result_ids: string_array_field_any(
                object,
                &["evidence_result_ids", "result_ids"],
            ),
        }
    }

    fn id_line_value(&self) -> &str {
        self.id.as_deref().unwrap_or("unknown")
    }

    fn status_line_value(&self) -> &str {
        self.status.as_deref().unwrap_or("unknown")
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"id\":{},\"status\":{},\"paused\":{},\"polluted\":{},\"stale\":{},\"archived\":{},\"completed_evidence_only\":{},\"clean_room_replacement\":{},\"assignment_allowed\":{},\"original_window_blocks_assignment\":{},\"clean_room_replacement_required\":{},\"future_work_requires_fresh_clean_room\":{},\"replacement_window_id\":{},\"reason_codes\":{},\"business_task_ids\":{},\"evidence_result_ids\":{}}}",
            optional_string_json(self.id.as_deref()),
            optional_string_json(self.status.as_deref()),
            bool_value_text(self.paused),
            bool_value_text(self.polluted),
            bool_value_text(self.stale),
            bool_value_text(self.archived),
            bool_value_text(self.completed_evidence_only),
            bool_value_text(self.clean_room_replacement),
            bool_value_text(self.assignment_allowed),
            bool_value_text(self.original_window_blocks_assignment),
            bool_value_text(self.clean_room_replacement_required),
            bool_value_text(self.future_work_requires_fresh_clean_room),
            optional_string_json(self.replacement_window_id.as_deref()),
            string_array_json(&self.reason_codes),
            string_array_json(&self.business_task_ids),
            string_array_json(&self.evidence_result_ids)
        )
    }
}

fn worker_window_rows(container: &str) -> Vec<WorkerWindowRow> {
    let mut rows = json_object_array_field(container, "worker_windows")
        .or_else(|| json_object_array_field(container, "rows"))
        .or_else(|| json_object_array_field(container, "windows"))
        .unwrap_or_default()
        .into_iter()
        .map(WorkerWindowRow::from_json)
        .collect::<Vec<_>>();

    if let Some(single) = json_object_field(container, "worker_window") {
        rows.push(WorkerWindowRow::from_json(single));
    }

    rows
}

fn next_round_decision_container(status: &str) -> Option<&str> {
    let direct_report = json_top_level_object_field(status, "next_round_decision_report_v1");
    if let Some(report) = direct_report {
        return Some(next_round_decision_report_container(report));
    }

    if let Some(live_status_bundle) = json_top_level_object_field(status, "live_status_bundle") {
        if let Some(live_report) =
            json_object_field(live_status_bundle, "next_round_decision_report_v1")
        {
            return Some(next_round_decision_report_container(live_report));
        }

        let live_decision = json_object_field(live_status_bundle, "next_round_decision_status_v1")
            .or_else(|| json_object_field(live_status_bundle, "next_round_decision_status"))
            .or_else(|| json_object_field(live_status_bundle, "next_round_decision"));
        if live_decision.is_some() {
            return live_decision;
        }
    }

    let direct = json_top_level_object_field(status, "next_round_decision_status_v1")
        .or_else(|| json_top_level_object_field(status, "next_round_decision_status"))
        .or_else(|| json_top_level_object_field(status, "next_round_decision"));
    if direct.is_some() {
        return direct;
    }

    if json_string_field(status, "schema").as_deref() == Some("next_round_decision_report_v1") {
        return Some(next_round_decision_report_container(status));
    }

    json_top_level_object_field(status, "loop").and_then(next_round_decision_container)
}

fn next_round_downstream_status_consumers_container(status: &str) -> Option<&str> {
    let direct = json_top_level_object_field(status, "next_round_downstream_status_consumers_v1")
        .or_else(|| json_top_level_object_field(status, "next_round_downstream_status_consumers"));
    if direct.is_some() {
        return direct;
    }

    if let Some(live_status_bundle) = json_top_level_object_field(status, "live_status_bundle") {
        let live = json_object_field(
            live_status_bundle,
            "next_round_downstream_status_consumers_v1",
        )
        .or_else(|| {
            json_object_field(live_status_bundle, "next_round_downstream_status_consumers")
        });
        if live.is_some() {
            return live;
        }
    }

    if json_string_field(status, "schema").as_deref()
        == Some("next_round_downstream_status_consumers_v1")
    {
        return Some(status);
    }

    json_top_level_object_field(status, "loop")
        .and_then(next_round_downstream_status_consumers_container)
}

fn next_round_decision_report_container(report: &str) -> &str {
    json_object_field(report, "next_round_decision").unwrap_or(report)
}

fn downstream_status_consumers(container: &str) -> Vec<DownstreamStatusConsumer> {
    for field in [
        "consumers",
        "consumer_statuses",
        "consumer_facts",
        "downstream_consumers",
    ] {
        if let Some(consumers) = json_object_field(container, field) {
            let rows = json_object_keys(consumers)
                .into_iter()
                .filter_map(|key| {
                    json_object_field(consumers, &key)
                        .map(|object| DownstreamStatusConsumer::from_json(key, object))
                })
                .collect::<Vec<_>>();
            if !rows.is_empty() {
                return rows;
            }
        }

        if let Some(rows) = json_object_array_field(container, field) {
            let rows = rows
                .into_iter()
                .enumerate()
                .map(|(index, object)| {
                    DownstreamStatusConsumer::from_json(format!("consumer:{index}"), object)
                })
                .collect::<Vec<_>>();
            if !rows.is_empty() {
                return rows;
            }
        }
    }

    Vec::new()
}

fn string_field_any(object: &str, fields: &[&str]) -> Option<String> {
    fields
        .iter()
        .find_map(|field| json_string_field(object, field))
        .map(|value| compact_line(&value, 120))
        .filter(|value| !value.trim().is_empty())
}

fn bool_field_any(object: &str, fields: &[&str]) -> Option<bool> {
    fields
        .iter()
        .find_map(|field| json_bool_field(object, field))
}

fn scalar_value_any(object: &str, fields: &[&str]) -> String {
    fields
        .iter()
        .map(|field| scalar_value(object, field))
        .find(|value| value != "unknown")
        .unwrap_or_else(|| "unknown".to_owned())
}

fn scalar_value_any_in(primary: &str, secondary: Option<&str>, fields: &[&str]) -> String {
    secondary
        .map(|value| scalar_value_any(value, fields))
        .filter(|value| value != "unknown")
        .unwrap_or_else(|| scalar_value_any(primary, fields))
}

fn string_array_field_any(object: &str, fields: &[&str]) -> Vec<String> {
    fields
        .iter()
        .find_map(|field| json_string_array_field(object, field))
        .unwrap_or_default()
        .into_iter()
        .map(|value| compact_line(&value, 120))
        .filter(|value| !value.trim().is_empty())
        .collect()
}

fn string_array_field_any_in(object: Option<&str>, fields: &[&str]) -> Vec<String> {
    object
        .map(|value| string_array_field_any(value, fields))
        .unwrap_or_default()
}

fn string_array_top_level_field_any(object: &str, fields: &[&str]) -> Vec<String> {
    fields
        .iter()
        .find_map(|field| json_top_level_string_array_field(object, field))
        .unwrap_or_default()
        .into_iter()
        .map(|value| compact_line(&value, 120))
        .filter(|value| !value.trim().is_empty())
        .collect()
}

fn string_list_field_any(object: &str, fields: &[&str]) -> Vec<String> {
    fields
        .iter()
        .find_map(|field| {
            json_string_array_field(object, field)
                .or_else(|| json_string_field(object, field).map(|value| vec![value]))
        })
        .unwrap_or_default()
        .into_iter()
        .map(|value| compact_line(&value, 120))
        .filter(|value| !value.trim().is_empty())
        .collect()
}

fn string_list_top_level_field_any(object: &str, fields: &[&str]) -> Vec<String> {
    fields
        .iter()
        .find_map(|field| {
            json_top_level_string_array_field(object, field)
                .or_else(|| json_top_level_string_field(object, field).map(|value| vec![value]))
        })
        .unwrap_or_default()
        .into_iter()
        .map(|value| compact_line(&value, 120))
        .filter(|value| !value.trim().is_empty())
        .collect()
}

fn list_line_value(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
}

fn optional_string_json(value: Option<&str>) -> String {
    value
        .map(json_string_literal)
        .unwrap_or_else(|| "null".to_owned())
}

fn string_array_json(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| json_string_literal(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn scalar_json(value: &str) -> String {
    if value == "unknown" {
        json_string_literal(value)
    } else {
        value.to_owned()
    }
}

fn bool_field_in(primary: &str, secondary: Option<&str>, field: &str) -> bool {
    json_bool_field(primary, field).unwrap_or(false)
        || secondary
            .and_then(|value| json_bool_field(value, field))
            .unwrap_or(false)
}

fn bool_field_in_any(primary: &str, secondary: Option<&str>, fields: &[&str]) -> bool {
    fields
        .iter()
        .any(|field| bool_field_in(primary, secondary, field))
}

fn normalized_round_state(status: Option<&str>) -> Option<String> {
    match status {
        Some("round-done-ledger-commit-pending") => {
            Some("round_done_waiting_ledger_commit".to_owned())
        }
        Some("round-in-progress") => Some("round_in_progress".to_owned()),
        Some("normal_in_progress") => Some("round_in_progress".to_owned()),
        _ => None,
    }
}

fn normalized_round_state_from_flags(
    round_in_progress: Option<bool>,
    ledger_commit_pending: bool,
) -> Option<String> {
    if round_in_progress == Some(true) {
        Some("round_in_progress".to_owned())
    } else if ledger_commit_pending {
        Some("round_done_waiting_ledger_commit".to_owned())
    } else {
        None
    }
}

fn normalized_next_round_decision(decision: &str) -> Option<String> {
    match decision {
        "safe-to-wait"
        | "safe_to_wait"
        | "safe-to-wait/current-round-active"
        | "safe_to_wait_current_round_active" => {
            Some("safe-to-wait/current-round-active".to_owned())
        }
        "safe-to-continue-after-current-round" | "safe_to_continue_after_current_round" => {
            Some("safe-to-continue-after-current-round".to_owned())
        }
        "blocked-operator-attention"
        | "operator-attention-blocked"
        | "operator_attention_blocked" => Some("operator-attention-blocked".to_owned()),
        _ => None,
    }
}

fn evidence_scalar(evidence: Option<&str>, field: &str) -> String {
    evidence
        .map(|evidence| scalar_value(evidence, field))
        .filter(|value| value != "unknown")
        .unwrap_or_else(|| "0".to_owned())
}

fn optional_bool_json(value: Option<bool>) -> &'static str {
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
    fn worker_window_status_surfaces_replacement_without_side_effects() {
        let loop_status = r#"{
            "worker_window_status": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "starts_clean_room_replacement": false,
                "mutates_worker_window_status": false,
                "worker_windows": [
                    {
                        "window_id": "R21-clean-room",
                        "status": "clean",
                        "clean_room_replacement_required": false,
                        "business_task_ids": ["R21-service-cli"]
                    },
                    {
                        "window_id": "R20-eval-paused",
                        "status": "paused",
                        "polluted": true,
                        "stale": false,
                        "completed_evidence_only": true,
                        "clean_room_replacement_required": true,
                        "assignment_allowed": false,
                        "original_window_blocks_assignment": true,
                        "future_work_requires_fresh_clean_room": true,
                        "replacement_window_id": "R21-eval-replacement",
                        "reason_codes": ["polluted_context", "paused_by_owner"],
                        "business_task_ids": ["R20-eval-test"],
                        "evidence_result_ids": ["R21-eval-revalidated"]
                    },
                    {
                        "window_id": "R21-eval-replacement",
                        "status": "clean-room-replacement",
                        "clean_room_replacement": true,
                        "assignment_allowed": true,
                        "business_task_ids": ["R21-eval-test"]
                    }
                ]
            }
        }"#;

        let lines = worker_window_status_lines(Some(loop_status)).join("\n");
        let json = worker_window_status_json(Some(loop_status));

        assert!(lines.contains("worker_window_status read_only=true starts_process=false sends_prompt=false starts_clean_room_replacement=false mutates_worker_window_status=false total=3 replacement_required=1"));
        assert!(lines.contains("statuses=clean:1,clean-room-replacement:1,paused:1"));
        assert!(lines.contains("worker_window id=R20-eval-paused status=paused paused=true polluted=true stale=false archived=false completed_evidence_only=true clean_room_replacement=false assignment_allowed=false original_window_blocks_assignment=true clean_room_replacement_required=true future_work_requires_fresh_clean_room=true replacement_window_id=R21-eval-replacement"));
        assert!(lines.contains("worker_window id=R21-eval-replacement status=clean-room-replacement paused=false polluted=false stale=false archived=false completed_evidence_only=false clean_room_replacement=true assignment_allowed=true original_window_blocks_assignment=false clean_room_replacement_required=false future_work_requires_fresh_clean_room=false"));
        assert!(lines.contains("reason_codes=polluted_context,paused_by_owner"));
        assert!(lines.contains("business_task_ids=R20-eval-test"));
        assert!(lines.contains("evidence_result_ids=R21-eval-revalidated"));
        assert!(json.contains("\"starts_clean_room_replacement\":false"));
        assert!(json.contains("\"mutates_worker_window_status\":false"));
        assert!(json.contains("\"clean_room_replacement_required_count\":1"));
        assert!(json.contains("\"status_counts\":\"clean:1,clean-room-replacement:1,paused:1\""));
        assert!(json.contains("\"clean_room_replacement_required\":true"));
        assert!(json.contains("\"completed_evidence_only\":true"));
        assert!(json.contains("\"assignment_allowed\":false"));
        assert!(json.contains("\"assignment_allowed\":true"));
        assert!(json.contains("\"future_work_requires_fresh_clean_room\":true"));
    }

    #[test]
    fn worker_window_status_defaults_to_empty_read_only_json_when_absent() {
        let json = worker_window_status_json(None);

        assert!(worker_window_status_lines(None).is_empty());
        assert!(json.contains("\"read_only\":true"));
        assert!(json.contains("\"starts_clean_room_replacement\":false"));
        assert!(json.contains("\"total\":0"));
    }

    #[test]
    fn daemon_round_transition_status_surfaces_ledger_pending_without_side_effects() {
        let loop_status = r#"{
            "daemon_round_transition_status": {
                "read_only": true,
                "report_only": true,
                "observed_round_done": true,
                "latest_round_state": "round_done_waiting_ledger_commit",
                "round_in_progress": false,
                "active_round": 333,
                "done_round": 333,
                "ledger_round": 332,
                "ledger_commit_pending": true,
                "ledger_lag_rounds": 1,
                "status_label": "round-done-ledger-commit-pending",
                "activity_reason": "stdout_done_marker_seen_waiting_for_ledger_commit",
                "evidence_ids": ["stdout:round-333:done", "ledger:latest-round-332"],
                "reason_codes": ["round_done_before_ledger_commit"],
                "side_effects": {
                    "starts_daemon": false,
                    "stops_daemon": false,
                    "touches_remote": false,
                    "sends_prompt": false,
                    "starts_stream": false,
                    "replays_prompt": false,
                    "mutates_active_round": false,
                    "writes_ndkv": false
                }
            }
        }"#;

        let lines = daemon_round_transition_status_lines(Some(loop_status)).join("\n");
        let json = daemon_round_transition_status_json(Some(loop_status));

        assert!(lines.contains("daemon_round_transition status=round-done-ledger-commit-pending latest_round_state=round_done_waiting_ledger_commit round_in_progress=false read_only=true starts_process=false report_only=true observed_round_done=true active_round=333 done_round=333 ledger_round=332 ledger_commit_pending=true ledger_lag_rounds=1 starts_daemon=false stops_daemon=false touches_remote=false sends_prompt=false starts_stream=false replays_prompt=false mutates_active_round=false writes_ndkv=false activity_reason=stdout_done_marker_seen_waiting_for_ledger_commit"));
        assert!(lines.contains("evidence_ids=stdout:round-333:done,ledger:latest-round-332"));
        assert!(lines.contains("reason_codes=round_done_before_ledger_commit"));
        assert!(json.contains("\"starts_process\":false"));
        assert!(json.contains("\"latest_round_state\":\"round_done_waiting_ledger_commit\""));
        assert!(json.contains("\"round_in_progress\":false"));
        assert!(json.contains("\"active_round\":333"));
        assert!(json.contains("\"ledger_commit_pending\":true"));
        assert!(json.contains("\"ledger_lag_rounds\":1"));
        assert!(
            json.contains(
                "\"activity_reason\":\"stdout_done_marker_seen_waiting_for_ledger_commit\""
            )
        );
        assert!(json.contains("\"starts_daemon\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
        assert!(json.contains("\"starts_stream\":false"));
        assert!(json.contains("\"writes_ndkv\":false"));
    }

    #[test]
    fn daemon_round_transition_status_surfaces_in_progress_busy_state() {
        let loop_status = r#"{
            "daemon_round_transition_status": {
                "read_only": true,
                "starts_process": false,
                "report_only": true,
                "observed_round_done": false,
                "latest_round_state": "round_in_progress",
                "round_in_progress": true,
                "active_round": 335,
                "done_round": null,
                "ledger_round": 334,
                "ledger_commit_pending": false,
                "ledger_lag_rounds": 1,
                "status": "busy-in-progress",
                "activity_reason": "generate:start",
                "evidence_ids": ["active-round:335", "ledger:latest-round-334"],
                "reason_codes": ["daemon_round_in_progress"],
                "side_effects": {
                    "starts_daemon": false,
                    "stops_daemon": false,
                    "touches_remote": false,
                    "sends_prompt": false,
                    "starts_stream": false,
                    "replays_prompt": false,
                    "mutates_active_round": false,
                    "writes_ndkv": false
                }
            }
        }"#;

        let lines = daemon_round_transition_status_lines(Some(loop_status)).join("\n");
        let json = daemon_round_transition_status_json(Some(loop_status));

        assert!(lines.contains("daemon_round_transition status=busy-in-progress latest_round_state=round_in_progress round_in_progress=true read_only=true starts_process=false report_only=true observed_round_done=false active_round=335 done_round=unknown ledger_round=334 ledger_commit_pending=false ledger_lag_rounds=1"));
        assert!(lines.contains("activity_reason=generate:start"));
        assert!(lines.contains("evidence_ids=active-round:335,ledger:latest-round-334"));
        assert!(lines.contains("reason_codes=daemon_round_in_progress"));
        assert!(json.contains("\"latest_round_state\":\"round_in_progress\""));
        assert!(json.contains("\"round_in_progress\":true"));
        assert!(json.contains("\"observed_round_done\":false"));
        assert!(json.contains("\"ledger_commit_pending\":false"));
        assert!(json.contains("\"activity_reason\":\"generate:start\""));
        assert!(json.contains("\"writes_ndkv\":false"));
    }

    #[test]
    fn daemon_round_transition_status_accepts_service_cli_v1_fields() {
        let loop_status = r#"{
            "latest_done_round": 336,
            "round_in_progress": true,
            "context_hygiene_status": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "report_only": true,
                "completed_window_evidence_non_actionable": true,
                "future_work_requires_fresh_clean_room": true,
                "reads_old_window_payload": false,
                "reason_codes": ["completed_worker_evidence_only"]
            },
            "daemon_round_transition_status_v1": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "report_only": true,
                "transition_kind": "normal_in_progress",
                "active_round": 337,
                "latest_done_round": 336,
                "ledger_latest_round": 336,
                "round_in_progress": true,
                "starts_daemon": false,
                "stops_daemon": false,
                "touches_remote": false,
                "starts_stream": false,
                "replays_prompt": false,
                "mutates_active_round": false,
                "writes_ndkv": false
            }
        }"#;

        let transition_lines = daemon_round_transition_status_lines(Some(loop_status)).join("\n");
        let transition_json = daemon_round_transition_status_json(Some(loop_status));
        let hygiene_lines = context_hygiene_status_lines(Some(loop_status)).join("\n");
        let hygiene_json = context_hygiene_status_json(Some(loop_status));

        assert!(transition_lines.contains("daemon_round_transition status=normal_in_progress latest_round_state=round_in_progress round_in_progress=true read_only=true starts_process=false report_only=true observed_round_done=false active_round=337 done_round=336 ledger_round=336 ledger_commit_pending=false"));
        assert!(transition_lines.contains("starts_daemon=false"));
        assert!(transition_lines.contains("sends_prompt=false"));
        assert!(transition_lines.contains("writes_ndkv=false"));
        assert!(transition_json.contains("\"latest_round_state\":\"round_in_progress\""));
        assert!(transition_json.contains("\"round_in_progress\":true"));
        assert!(transition_json.contains("\"active_round\":337"));
        assert!(transition_json.contains("\"done_round\":336"));
        assert!(transition_json.contains("\"ledger_round\":336"));
        assert!(transition_json.contains("\"status\":\"normal_in_progress\""));
        assert!(hygiene_lines.contains("context_hygiene_status read_only=true starts_process=false sends_prompt=false report_only=true completed_window_evidence_non_actionable=true future_work_requires_fresh_clean_room=true reads_old_window_payload=false"));
        assert!(hygiene_lines.contains("reason_codes=completed_worker_evidence_only"));
        assert!(hygiene_json.contains("\"completed_window_evidence_non_actionable\":true"));
        assert!(hygiene_json.contains("\"starts_process\":false"));
        assert!(hygiene_json.contains("\"sends_prompt\":false"));
    }

    #[test]
    fn next_round_decision_status_surfaces_operator_safe_wait_without_side_effects() {
        let loop_status = r#"{
            "next_round_decision_status_v1": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "report_only": true,
                "decision": "safe-to-wait/current-round-active",
                "active_round": 365,
                "latest_done_round": 364,
                "ledger_latest_round": 364,
                "reason_codes": ["current_round_active"],
                "evidence_ids": ["active-round:365", "ledger:latest-round-364"],
                "side_effects": {
                    "starts_daemon": false,
                    "stops_daemon": false,
                    "touches_remote": false,
                    "sends_prompt": false,
                    "starts_stream": false,
                    "replays_prompt": false,
                    "writes_ndkv": false
                }
            }
        }"#;

        let lines = next_round_decision_status_lines(Some(loop_status)).join("\n");
        let json = next_round_decision_status_json(Some(loop_status));

        assert!(lines.contains("next_round_decision_status decision=safe-to-wait/current-round-active read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=365 done_round=364 ledger_round=364"));
        assert!(lines.contains("reason_codes=current_round_active"));
        assert!(lines.contains("evidence_ids=active-round:365,ledger:latest-round-364"));
        assert!(json.contains("\"decision\":\"safe-to-wait/current-round-active\""));
        assert!(json.contains("\"starts_process\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
        assert!(json.contains("\"starts_daemon\":false"));
        assert!(json.contains("\"writes_ndkv\":false"));
    }

    #[test]
    fn next_round_decision_status_surfaces_nested_side_effects_conservatively() {
        let loop_status = r#"{
            "next_round_decision_status_v1": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "report_only": true,
                "decision": "operator-attention-blocked",
                "starts_stream": false,
                "writes_ndkv": false,
                "side_effects": {
                    "starts_stream": true,
                    "replays_prompt": true,
                    "writes_ndkv": true
                }
            }
        }"#;

        let lines = next_round_decision_status_lines(Some(loop_status)).join("\n");
        let json = next_round_decision_status_json(Some(loop_status));

        assert!(lines.contains("decision=operator-attention-blocked"));
        assert!(lines.contains("starts_stream=true"));
        assert!(lines.contains("replays_prompt=true"));
        assert!(lines.contains("writes_ndkv=true"));
        assert!(json.contains("\"starts_stream\":true"));
        assert!(json.contains("\"replays_prompt\":true"));
        assert!(json.contains("\"writes_ndkv\":true"));
    }

    #[test]
    fn next_round_decision_status_accepts_eval_report_fixture_shape() {
        let report = include_str!("../../fixtures/r36-next-round-decision-report-v1.example.json");

        let lines = next_round_decision_status_lines(Some(report)).join("\n");
        let json = next_round_decision_status_json(Some(report));

        assert!(lines.contains("next_round_decision_status decision=safe-to-continue-after-current-round read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=366 done_round=366 ledger_round=366"));
        assert!(lines.contains("reason_codes=none"));
        assert!(json.contains("\"decision\":\"safe-to-continue-after-current-round\""));
        assert!(json.contains("\"active_round\":366"));
        assert!(json.contains("\"done_round\":366"));
        assert!(json.contains("\"ledger_round\":366"));
        assert!(json.contains("\"starts_process\":false"));
        assert!(json.contains("\"starts_stream\":false"));
        assert!(json.contains("\"writes_ndkv\":false"));
    }

    #[test]
    fn next_round_decision_status_accepts_eval_side_effect_markers_conservatively() {
        let report = r#"{
            "next_round_decision_report_v1": {
                "next_round_decision": {
                    "decision_status": "operator_attention_blocked",
                    "read_only": true,
                    "report_only": true,
                    "process_start_allowed": true,
                    "dispatch_work_allowed": true,
                    "prompt_replay_allowed": true,
                    "ndkv_write_allowed": true,
                    "failure_reasons": ["next-round decision evidence attempted a runtime side effect"]
                }
            }
        }"#;

        let lines = next_round_decision_status_lines(Some(report)).join("\n");
        let json = next_round_decision_status_json(Some(report));

        assert!(lines.contains("decision=operator-attention-blocked"));
        assert!(lines.contains("starts_process=true"));
        assert!(lines.contains("sends_prompt=true"));
        assert!(lines.contains("starts_stream=true"));
        assert!(lines.contains("replays_prompt=true"));
        assert!(lines.contains("writes_ndkv=true"));
        assert!(
            lines.contains(
                "reason_codes=next-round decision evidence attempted a runtime side effect"
            )
        );
        assert!(json.contains("\"starts_process\":true"));
        assert!(json.contains("\"sends_prompt\":true"));
        assert!(json.contains("\"starts_stream\":true"));
        assert!(json.contains("\"replays_prompt\":true"));
        assert!(json.contains("\"writes_ndkv\":true"));
    }

    #[test]
    fn next_round_decision_status_accepts_display_state_fixture_shape() {
        let loop_status = r#"{
            "next_round_decision": {
                "schema": "next_round_decision_evidence_v1",
                "display_state": "blocked-operator-attention",
                "operator_attention_required": true,
                "may_display_unattended_continuation": false,
                "wait_for_current_round": false,
                "continue_after_current_round": false,
                "reason_code": "report_gate_failed_operator_attention_required",
                "side_effects": false,
                "evidence": {
                    "transition_kind": "normal_in_progress",
                    "report_gate_passed": false,
                    "round_in_progress": true
                }
            }
        }"#;

        let lines = next_round_decision_status_lines(Some(loop_status)).join("\n");
        let json = next_round_decision_status_json(Some(loop_status));

        assert!(lines.contains("decision=operator-attention-blocked"));
        assert!(lines.contains("reason_codes=report_gate_failed_operator_attention_required"));
        assert!(json.contains("\"decision\":\"operator-attention-blocked\""));
        assert!(json.contains("\"starts_process\":false"));
    }

    #[test]
    fn next_round_decision_status_accepts_live_status_bundle_fixture_variants() {
        let fixture =
            include_str!("../../fixtures/r37-live-status-bundle-next-round-decision.example.json");
        let safe_status = json_top_level_object_field(fixture, "safe_to_wait_status").unwrap();
        let safe_loop = json_object_field(safe_status, "loop").unwrap();
        let blocked_status =
            json_top_level_object_field(fixture, "blocked_operator_attention_status").unwrap();
        let blocked_loop = json_object_field(blocked_status, "loop").unwrap();

        let safe_lines = next_round_decision_status_lines(Some(safe_loop)).join("\n");
        let safe_json = next_round_decision_status_json(Some(safe_loop));
        let blocked_lines = next_round_decision_status_lines(Some(blocked_loop)).join("\n");
        let blocked_json = next_round_decision_status_json(Some(blocked_loop));

        assert!(safe_lines.contains("next_round_decision_status decision=safe-to-wait/current-round-active read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=369 done_round=368 ledger_round=368"));
        assert!(safe_lines.contains("reason_codes=current_round_active"));
        assert!(safe_lines.contains("evidence_ids=active-round:369,ledger:latest-round-368"));
        assert!(safe_json.contains("\"decision\":\"safe-to-wait/current-round-active\""));
        assert!(safe_json.contains("\"active_round\":369"));
        assert!(safe_json.contains("\"done_round\":368"));
        assert!(safe_json.contains("\"ledger_round\":368"));
        assert!(!safe_json.contains("\"starts_process\":true"));
        assert!(!safe_json.contains("\"writes_ndkv\":true"));

        assert!(blocked_lines.contains("next_round_decision_status decision=operator-attention-blocked read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=368 done_round=368 ledger_round=368"));
        assert!(
            blocked_lines.contains("reason_codes=report_gate_failed_operator_attention_required")
        );
        assert!(blocked_lines.contains("evidence_ids=round:368:report-gate-failed"));
        assert!(blocked_json.contains("\"decision\":\"operator-attention-blocked\""));
        assert!(
            blocked_json
                .contains("\"reason_codes\":[\"report_gate_failed_operator_attention_required\"]")
        );
        assert!(!blocked_json.contains("\"starts_stream\":true"));
        assert!(!blocked_json.contains("\"replays_prompt\":true"));
    }

    #[test]
    fn next_round_decision_status_prefers_live_bundle_report_v1_over_legacy_decision() {
        let fixture = include_str!(
            "../../fixtures/r39-current-next-round-decision-report-v1-status.example.json"
        );
        let current_status = json_top_level_object_field(fixture, "current_status").unwrap();
        let current_loop = json_top_level_object_field(current_status, "loop").unwrap();

        let lines = next_round_decision_status_lines(Some(current_loop)).join("\n");
        let json = next_round_decision_status_json(Some(current_loop));

        assert!(lines.contains("next_round_decision_status decision=safe-to-continue-after-current-round read_only=true starts_process=false sends_prompt=false report_only=true starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=371 done_round=371 ledger_round=370"));
        assert!(lines.contains("reason_codes=latest_done_round_waiting_ledger_commit"));
        assert!(
            lines.contains("evidence_ids=active-round:371,done-round:371,ledger:latest-round-370")
        );
        assert!(!lines.contains("active_round=999"));
        assert!(json.contains("\"decision\":\"safe-to-continue-after-current-round\""));
        assert!(json.contains("\"active_round\":371"));
        assert!(json.contains("\"ledger_round\":370"));
        assert!(!json.contains("\"starts_process\":true"));
        assert!(!json.contains("\"writes_ndkv\":true"));
    }

    #[test]
    fn next_round_downstream_status_consumers_accepts_root_shape() {
        let fixture = include_str!(
            "../../fixtures/r43-next-round-downstream-status-consumers-v1.example.json"
        );
        let status = json_top_level_object_field(fixture, "root_status").unwrap();

        let lines = next_round_downstream_status_consumers_lines(Some(status)).join("\n");
        let json = next_round_downstream_status_consumers_json(Some(status));

        assert!(lines.contains("next_round_downstream_status_consumers read_only=true starts_process=false sends_prompt=false report_only=true side_effects=false starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=373 done_round=372 ledger_round=372 round_id_evidence_active_round=373 round_id_evidence_done_round=372 round_id_evidence_ledger_round=372"));
        assert!(lines.contains("consumers=service_cli_display,forge_operator_display,agent_assignment_acceptance,memory_self_improve_admission_visibility"));
        assert!(lines.contains("reason_codes=downstream_consumers_ready"));
        assert!(lines.contains("evidence_ids=status-root:downstream-consumers"));
        assert!(lines.contains("next_round_downstream_consumer id=forge_operator_display required=true satisfied=true read_only=true report_only=true side_effects=false starts_process=false sends_prompt=false starts_stream=false replays_prompt=false writes_ndkv=false"));
        assert!(lines.contains("next_round_downstream_consumer id=agent_assignment_acceptance required=true satisfied=true"));
        assert!(lines.contains("next_round_downstream_consumer id=memory_self_improve_admission_visibility required=true satisfied=true"));
        assert!(json.contains("\"active_round\":373"));
        assert!(json.contains("\"done_round\":372"));
        assert!(json.contains("\"ledger_round\":372"));
        assert!(json.contains("\"round_id_evidence\":{"));
        assert!(json.contains("\"transition_kind\":\"normal_in_progress\""));
        assert!(json.contains("\"id\":\"service_cli_display\""));
        assert!(!json.contains("\"side_effects\":true"));
        assert!(!json.contains("\"starts_process\":true"));
        assert!(!json.contains("\"sends_prompt\":true"));
        assert!(!json.contains("\"writes_ndkv\":true"));
    }

    #[test]
    fn next_round_downstream_status_consumers_accepts_live_bundle_array_shape() {
        let fixture = include_str!(
            "../../fixtures/r43-next-round-downstream-status-consumers-v1.example.json"
        );
        let status = json_top_level_object_field(fixture, "live_status_bundle_status").unwrap();
        let loop_status = json_top_level_object_field(status, "loop").unwrap();

        let lines = next_round_downstream_status_consumers_lines(Some(loop_status)).join("\n");
        let json = next_round_downstream_status_consumers_json(Some(loop_status));

        assert!(lines.contains("next_round_downstream_status_consumers read_only=true starts_process=false sends_prompt=false report_only=true side_effects=false starts_daemon=false stops_daemon=false touches_remote=false starts_stream=false replays_prompt=false writes_ndkv=false active_round=374 done_round=373 ledger_round=373 round_id_evidence_active_round=374 round_id_evidence_done_round=373 round_id_evidence_ledger_round=373"));
        assert!(lines.contains("consumers=service_cli_display,forge_operator_display,agent_assignment_acceptance,memory_self_improve_admission_visibility"));
        assert!(lines.contains(
            "next_round_downstream_consumer id=service_cli_display required=true satisfied=true"
        ));
        assert!(json.contains("\"transition_kind\":\"round_done_waiting_ledger_commit\""));
        assert!(json.contains("\"active_round\":374"));
        assert!(json.contains("\"done_round\":373"));
        assert!(json.contains("\"ledger_round\":373"));
        assert!(!json.contains("\"active_round\":999"));
        assert!(!json.contains("\"side_effects\":true"));
    }

    #[test]
    fn next_round_decision_status_accepts_all_operator_decisions() {
        for decision in [
            "safe-to-wait/current-round-active",
            "safe-to-continue-after-current-round",
            "operator-attention-blocked",
        ] {
            let loop_status = format!(
                r#"{{
                    "next_round_decision_status": {{
                        "read_only": true,
                        "starts_process": false,
                        "sends_prompt": false,
                        "report_only": true,
                        "decision": "{decision}"
                    }}
                }}"#
            );

            let lines = next_round_decision_status_lines(Some(&loop_status)).join("\n");
            let json = next_round_decision_status_json(Some(&loop_status));

            assert!(lines.contains(&format!("decision={decision}")));
            assert!(json.contains(&format!("\"decision\":\"{decision}\"")));
        }
    }

    #[test]
    fn next_round_decision_status_is_absent_from_text_when_section_absent() {
        let json = next_round_decision_status_json(None);

        assert!(next_round_decision_status_lines(None).is_empty());
        assert!(json.contains("\"read_only\":true"));
        assert!(json.contains("\"starts_process\":false"));
        assert!(json.contains("\"decision\":null"));
    }

    #[test]
    fn worker_window_replacement_report_projects_evolution_loop_report_contract() {
        let report = r#"{
            "worker_window_replacement_report_v1": {
                "schema": "worker_window_replacement_report_v1",
                "consumer_surface": "clean_room_worker_window_replacement_status",
                "read_only": true,
                "status_loaded": true,
                "source": "external_worker_window_status_json",
                "source_path": "docs/runbooks/smartsteam-worker-window-status-r21.example.json",
                "source_status": {
                    "schema": "worker_window_status_v1",
                    "side_effects_allowed": false,
                    "windows": [
                        {
                            "window_id": "r20-eval-test",
                            "status": "paused",
                            "polluted": true,
                            "archived": true,
                            "completed_evidence_only": true,
                            "clean_room_replacement_required": true,
                            "original_window_blocks_assignment": true,
                            "assignment_allowed": false,
                            "future_work_requires_fresh_clean_room": true
                        },
                        {
                            "window_id": "r21-eval-test",
                            "status": "clean-room-replacement",
                            "clean_room_replacement": true,
                            "replaces_window_id": "r20-eval-test",
                            "assignment_allowed": true
                        }
                    ]
                },
                "evidence_map": {
                    "window_count": 2,
                    "paused_count": 1,
                    "polluted_count": 1,
                    "clean_room_replacement_count": 1,
                    "replacement_required_count": 1,
                    "blocked_original_count": 1,
                    "side_effects_allowed": false
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
                    "replays_prompt": false
                }
            }
        }"#;

        let lines = worker_window_replacement_report_lines(report).join("\n");
        let json = worker_window_replacement_report_json(Some(report));

        assert!(lines.contains("worker_window_replacement_report read_only=true starts_process=false sends_prompt=false status_loaded=true total=2 paused=1 polluted=1 clean_room_replacement=1 replacement_required=1 blocked_original=1 starts_clean_room_replacement=false mutates_worker_window_status=false source=external_worker_window_status_json"));
        assert!(lines.contains("worker_window_report_source id=r20-eval-test status=paused paused=true polluted=true stale=false archived=true completed_evidence_only=true clean_room_replacement=false assignment_allowed=false original_window_blocks_assignment=true clean_room_replacement_required=true future_work_requires_fresh_clean_room=true"));
        assert!(lines.contains("worker_window_report_source id=r21-eval-test status=clean-room-replacement paused=false polluted=false stale=false archived=false completed_evidence_only=false clean_room_replacement=true assignment_allowed=true original_window_blocks_assignment=false clean_room_replacement_required=false future_work_requires_fresh_clean_room=false"));
        assert!(json.contains("\"status_loaded\":true"));
        assert!(json.contains("\"side_effects_allowed\":false"));
        assert!(json.contains("\"replacement_required_count\":1"));
        assert!(json.contains("\"archived\":true"));
        assert!(json.contains("\"completed_evidence_only\":true"));
        assert!(json.contains("\"future_work_requires_fresh_clean_room\":true"));
        assert!(json.contains("\"starts_clean_room_replacement\":false"));
        assert!(json.contains("\"mutates_worker_window_status\":false"));
        assert!(json.contains("\"id\":\"r21-eval-test\""));
    }
}
