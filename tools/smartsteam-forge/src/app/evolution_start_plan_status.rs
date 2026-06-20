use super::evolution_candidate_status::{CandidateBacklogSummary, candidate_start_blocked};
use super::evolution_readiness_start_status::ReadinessStartStatus;
use super::evolution_report_gate_status::{ReportGateStatus, evolution_daemon_command};
use super::status_json::{
    bool_value_text, json_bool_field, json_object_field, json_string_array_field,
    json_string_field, json_string_literal, scalar_value,
};

#[cfg(test)]
pub(super) fn unattended_start_plan_json(
    daemon: &str,
    work_dir: &str,
    backend_endpoint: &str,
    candidate_summary: Option<&CandidateBacklogSummary>,
    report_gate_status: &ReportGateStatus,
    remote_chain: Option<&str>,
) -> String {
    unattended_start_plan_json_with_readiness(
        daemon,
        work_dir,
        backend_endpoint,
        candidate_summary,
        report_gate_status,
        remote_chain,
        None,
    )
}

pub(super) fn unattended_start_plan_json_with_readiness(
    daemon: &str,
    work_dir: &str,
    backend_endpoint: &str,
    candidate_summary: Option<&CandidateBacklogSummary>,
    report_gate_status: &ReportGateStatus,
    remote_chain: Option<&str>,
    readiness_status: Option<&ReadinessStartStatus>,
) -> String {
    let plan = UnattendedStartPlan::from_status(
        daemon,
        work_dir,
        backend_endpoint,
        candidate_summary,
        report_gate_status,
        remote_chain,
        readiness_status,
    );

    format!(
        "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"candidate_lifecycle_ready\":{},\"readiness_start_ready\":{},\"readiness_blocks_start\":{},\"readiness_blocking_failures\":{},\"can_start\":{},\"current_state\":{},\"block_reason\":{},\"report_gate_continuation_state\":{},\"report_gate_can_continue_unattended\":{},\"report_gate_blocks_continuation\":{},\"continuation_block_reason\":{},\"remote_runtime_acceleration_ok\":{},\"remote_runtime_acceleration_blocks_start\":false,\"remote_runtime_cpu_or_no_gpu_roles\":{},\"remote_runtime_acceleration_next_step\":{},\"stale_pid_file\":{},\"stale_pid\":{},\"stale_pid_blocks_start\":false,\"stale_pid_cleanup_command\":{},\"check_only_command\":{},\"start_command\":{}}}",
        bool_value_text(plan.candidate_lifecycle_ready),
        bool_value_text(plan.readiness_start_ready),
        bool_value_text(!plan.readiness_start_ready),
        json_string_literal(
            plan.readiness_blocking_failures
                .as_deref()
                .unwrap_or("none")
        ),
        bool_value_text(plan.can_start),
        json_string_literal(plan.current_state()),
        plan.block_reason_json(),
        json_string_literal(plan.report_gate_continuation_state),
        bool_value_text(plan.report_gate_can_continue_unattended),
        bool_value_text(plan.report_gate_blocks_continuation),
        plan.continuation_block_reason_json(),
        plan.acceleration.ok_json(),
        plan.acceleration.roles_json(),
        plan.acceleration.next_step_json(),
        bool_value_text(plan.stale_pid_file),
        plan.stale_pid_json(),
        plan.stale_pid_cleanup_command_json(),
        json_string_literal(&plan.check_only_command),
        json_string_literal(&plan.start_command)
    )
}

#[cfg(test)]
pub(super) fn unattended_start_plan_lines(
    daemon: &str,
    work_dir: &str,
    backend_endpoint: &str,
    candidate_summary: Option<&CandidateBacklogSummary>,
    report_gate_status: &ReportGateStatus,
    remote_chain: Option<&str>,
) -> Vec<String> {
    unattended_start_plan_lines_with_readiness(
        daemon,
        work_dir,
        backend_endpoint,
        candidate_summary,
        report_gate_status,
        remote_chain,
        None,
    )
}

pub(super) fn unattended_start_plan_lines_with_readiness(
    daemon: &str,
    work_dir: &str,
    backend_endpoint: &str,
    candidate_summary: Option<&CandidateBacklogSummary>,
    report_gate_status: &ReportGateStatus,
    remote_chain: Option<&str>,
    readiness_status: Option<&ReadinessStartStatus>,
) -> Vec<String> {
    let plan = UnattendedStartPlan::from_status(
        daemon,
        work_dir,
        backend_endpoint,
        candidate_summary,
        report_gate_status,
        remote_chain,
        readiness_status,
    );

    let mut lines = vec![
        format!(
            "unattended_start_plan can_start={} candidate_lifecycle_ready={} readiness_start_ready={} readiness_blocks_start={} readiness_blocking_failures={} block_reason={} report_gate_continuation_state={} report_gate_can_continue_unattended={} report_gate_blocks_continuation={} continuation_block_reason={} stale_pid_file={} stale_pid={} stale_pid_blocks_start=false current_state={}",
            bool_value_text(plan.can_start),
            bool_value_text(plan.candidate_lifecycle_ready),
            bool_value_text(plan.readiness_start_ready),
            bool_value_text(!plan.readiness_start_ready),
            plan.readiness_blocking_failures
                .as_deref()
                .unwrap_or("none"),
            plan.block_reason.unwrap_or("none"),
            plan.report_gate_continuation_state,
            bool_value_text(plan.report_gate_can_continue_unattended),
            bool_value_text(plan.report_gate_blocks_continuation),
            plan.continuation_block_reason.unwrap_or("none"),
            bool_value_text(plan.stale_pid_file),
            plan.stale_pid_line_value(),
            plan.current_state()
        ),
        format!("unattended_start_check={}", plan.check_only_command),
        format!("unattended_start_command={}", plan.start_command),
    ];
    if plan.acceleration.is_known() {
        lines.push(format!(
            "unattended_start_acceleration ok={} blocks_start=false cpu_or_no_gpu_roles={} next_step={}",
            plan.acceleration.ok_line_value(),
            plan.acceleration.roles_line_value(),
            plan.acceleration.next_step_line_value()
        ));
    }
    if let Some(command) = plan.stale_pid_cleanup_command {
        lines.push(format!("stale_pid_cleanup_command={command}"));
    }
    lines
}

#[cfg(test)]
pub(super) fn unattended_start_next_step(
    daemon: &str,
    work_dir: &str,
    backend_endpoint: &str,
    candidate_summary: Option<&CandidateBacklogSummary>,
    report_gate_status: &ReportGateStatus,
    remote_chain: Option<&str>,
) -> String {
    unattended_start_next_step_with_readiness(
        daemon,
        work_dir,
        backend_endpoint,
        candidate_summary,
        report_gate_status,
        remote_chain,
        None,
    )
}

pub(super) fn unattended_start_next_step_with_readiness(
    daemon: &str,
    work_dir: &str,
    backend_endpoint: &str,
    candidate_summary: Option<&CandidateBacklogSummary>,
    report_gate_status: &ReportGateStatus,
    remote_chain: Option<&str>,
    readiness_status: Option<&ReadinessStartStatus>,
) -> String {
    UnattendedStartPlan::from_status(
        daemon,
        work_dir,
        backend_endpoint,
        candidate_summary,
        report_gate_status,
        remote_chain,
        readiness_status,
    )
    .next_step()
}

struct UnattendedStartPlan<'a> {
    candidate_lifecycle_ready: bool,
    readiness_start_ready: bool,
    readiness_blocking_failures: Option<String>,
    can_start: bool,
    daemon_running: bool,
    block_reason: Option<&'a str>,
    report_gate_continuation_state: &'static str,
    report_gate_can_continue_unattended: bool,
    report_gate_blocks_continuation: bool,
    continuation_block_reason: Option<&'static str>,
    stale_pid_file: bool,
    stale_pid: Option<String>,
    stale_pid_cleanup_command: Option<String>,
    acceleration: RemoteRuntimeAcceleration,
    check_only_command: String,
    start_command: String,
}

impl<'a> UnattendedStartPlan<'a> {
    fn from_status(
        daemon: &str,
        work_dir: &str,
        backend_endpoint: &str,
        candidate_summary: Option<&'a CandidateBacklogSummary>,
        report_gate_status: &ReportGateStatus,
        remote_chain: Option<&str>,
        readiness_status: Option<&ReadinessStartStatus>,
    ) -> Self {
        let candidate_blocked = candidate_start_blocked(candidate_summary);
        let daemon_running = json_bool_field(daemon, "running").unwrap_or(false);
        let report_gate_block_reason = report_gate_status.continuation_block_reason();
        let readiness_start_ready = readiness_status
            .map(ReadinessStartStatus::start_ready)
            .unwrap_or(true);
        let readiness_blocking_failures = readiness_status
            .map(ReadinessStartStatus::start_blocking_failures_text)
            .filter(|value| value != "none");
        let can_start = !candidate_blocked
            && !daemon_running
            && report_gate_block_reason.is_none()
            && readiness_start_ready;
        let block_reason = if candidate_blocked {
            Some("candidate_backlog_not_ready")
        } else if daemon_running {
            Some("already_running")
        } else if !readiness_start_ready {
            Some("readiness_not_ready")
        } else {
            report_gate_block_reason
        };
        let stale_pid_file = json_bool_field(daemon, "stale_pid_file").unwrap_or(false);
        let stale_pid = stale_pid_file.then(|| scalar_value(daemon, "stale_pid"));
        let stale_pid_cleanup_command =
            stale_pid_file.then(|| evolution_daemon_command("-Stop", work_dir, ""));
        let acceleration = RemoteRuntimeAcceleration::from_remote_chain(remote_chain);

        Self {
            candidate_lifecycle_ready: !candidate_blocked,
            readiness_start_ready,
            readiness_blocking_failures,
            can_start,
            daemon_running,
            block_reason,
            report_gate_continuation_state: report_gate_status.continuation_state(),
            report_gate_can_continue_unattended: report_gate_status.can_continue_unattended(),
            report_gate_blocks_continuation: report_gate_block_reason.is_some(),
            continuation_block_reason: report_gate_block_reason,
            stale_pid_file,
            stale_pid,
            stale_pid_cleanup_command,
            acceleration,
            check_only_command: evolution_daemon_command("-StartCheck", work_dir, backend_endpoint),
            start_command: evolution_daemon_command("-Start", work_dir, backend_endpoint),
        }
    }

    fn block_reason_json(&self) -> String {
        self.block_reason
            .map(json_string_literal)
            .unwrap_or_else(|| "null".to_owned())
    }

    fn current_state(&self) -> &'static str {
        if self.daemon_running {
            "running"
        } else if self.can_start {
            "not_running_ready_to_start"
        } else {
            "not_running_blocked"
        }
    }

    fn next_step(&self) -> String {
        if self.daemon_running {
            return "running: monitor JsonStatus; duplicate unattended start is blocked".to_owned();
        }

        if self.can_start {
            return format!(
                "ready_to_start: run StartCheck before Start: {}",
                self.check_only_command
            );
        }

        match self.block_reason {
            Some("candidate_backlog_not_ready") => {
                "blocked: resolve candidate backlog before unattended evolution".to_owned()
            }
            Some("report_gate_not_ready") => {
                "blocked: fix report gate before unattended evolution".to_owned()
            }
            Some("readiness_not_ready") => format!(
                "blocked: fix readiness before unattended evolution ({})",
                self.readiness_blocking_failures
                    .as_deref()
                    .unwrap_or("unknown")
            ),
            Some(reason) => format!("blocked: inspect unattended_start_plan reason={reason}"),
            None => "blocked: inspect unattended_start_plan".to_owned(),
        }
    }

    fn continuation_block_reason_json(&self) -> String {
        self.continuation_block_reason
            .map(json_string_literal)
            .unwrap_or_else(|| "null".to_owned())
    }

    fn stale_pid_json(&self) -> String {
        self.stale_pid
            .as_ref()
            .filter(|value| value.as_str() != "unknown")
            .cloned()
            .unwrap_or_else(|| "null".to_owned())
    }

    fn stale_pid_line_value(&self) -> String {
        self.stale_pid.clone().unwrap_or_else(|| "none".to_owned())
    }

    fn stale_pid_cleanup_command_json(&self) -> String {
        self.stale_pid_cleanup_command
            .as_deref()
            .map(json_string_literal)
            .unwrap_or_else(|| "null".to_owned())
    }
}

#[derive(Default)]
struct RemoteRuntimeAcceleration {
    ok: Option<bool>,
    cpu_or_no_gpu_roles: Vec<String>,
    next_step: Option<String>,
}

impl RemoteRuntimeAcceleration {
    fn from_remote_chain(remote_chain: Option<&str>) -> Self {
        let Some(remote_chain) = remote_chain else {
            return Self::default();
        };
        let Some(remote_runtime) = json_object_field(remote_chain, "remote_runtime") else {
            return Self::default();
        };

        let roles = json_string_array_field(remote_runtime, "cpu_or_no_gpu_roles")
            .unwrap_or_default()
            .into_iter()
            .filter(|role| !role.trim().is_empty())
            .collect::<Vec<_>>();
        let ok = json_bool_field(remote_runtime, "acceleration_ok")
            .or_else(|| (!roles.is_empty()).then_some(false));
        let next_step = json_string_field(remote_runtime, "acceleration_next_step")
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                (ok == Some(false)).then(|| {
                    ".\\tools\\smartsteam-forge\\run-remote-gemma-unattended.cmd -RestartRemote -SkipBuild"
                        .to_owned()
                })
            });

        Self {
            ok,
            cpu_or_no_gpu_roles: roles,
            next_step,
        }
    }

    fn is_known(&self) -> bool {
        self.ok.is_some() || !self.cpu_or_no_gpu_roles.is_empty() || self.next_step.is_some()
    }

    fn ok_json(&self) -> &'static str {
        match self.ok {
            Some(true) => "true",
            Some(false) => "false",
            None => "null",
        }
    }

    fn roles_json(&self) -> String {
        let values = self
            .cpu_or_no_gpu_roles
            .iter()
            .map(|role| json_string_literal(role))
            .collect::<Vec<_>>()
            .join(",");
        format!("[{values}]")
    }

    fn next_step_json(&self) -> String {
        self.next_step
            .as_deref()
            .map(json_string_literal)
            .unwrap_or_else(|| "null".to_owned())
    }

    fn ok_line_value(&self) -> &'static str {
        match self.ok {
            Some(true) => "true",
            Some(false) => "false",
            None => "unknown",
        }
    }

    fn roles_line_value(&self) -> String {
        if self.cpu_or_no_gpu_roles.is_empty() {
            "none".to_owned()
        } else {
            self.cpu_or_no_gpu_roles.join(",")
        }
    }

    fn next_step_line_value(&self) -> &str {
        self.next_step.as_deref().unwrap_or("none")
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::app::evolution_candidate_status::{
        EVOLUTION_CANDIDATES_FILE, read_candidate_backlog_summary,
    };
    use crate::app::evolution_report_gate_status::read_report_gate_status;
    use crate::app::status_json::json_object_field;

    #[test]
    fn start_plan_blocks_duplicate_daemon_start() {
        let daemon = r#"{"running":true,"stale_pid_file":false}"#;
        let status = read_report_gate_status(None);

        let lines = unattended_start_plan_lines(
            daemon,
            "target\\evolution\\daemon",
            "127.0.0.1:7878",
            None,
            &status,
            None,
        )
        .join("\n");
        let json = unattended_start_plan_json(
            daemon,
            "target\\evolution\\daemon",
            "127.0.0.1:7878",
            None,
            &status,
            None,
        );

        assert!(lines.contains("can_start=false"));
        assert!(lines.contains("block_reason=already_running"));
        assert!(lines.contains("current_state=running"));
        assert!(json.contains("\"can_start\":false"));
        assert!(json.contains("\"current_state\":\"running\""));
        assert!(json.contains("\"block_reason\":\"already_running\""));
        assert_eq!(
            unattended_start_next_step(
                daemon,
                "target\\evolution\\daemon",
                "127.0.0.1:7878",
                None,
                &status,
                None
            ),
            "running: monitor JsonStatus; duplicate unattended start is blocked"
        );
    }

    #[test]
    fn start_plan_keeps_stale_pid_as_cleanup_evidence_only() {
        let daemon = r#"{"running":false,"stale_pid_file":true,"stale_pid":4242}"#;
        let status = read_report_gate_status(None);

        let lines = unattended_start_plan_lines(
            daemon,
            "target\\evolution\\daemon",
            "",
            None,
            &status,
            None,
        )
        .join("\n");
        let json = unattended_start_plan_json(
            daemon,
            "target\\evolution\\daemon",
            "",
            None,
            &status,
            None,
        );

        assert!(lines.contains("can_start=true"));
        assert!(lines.contains("current_state=not_running_ready_to_start"));
        assert!(lines.contains("stale_pid_file=true"));
        assert!(lines.contains("stale_pid=4242"));
        assert!(lines.contains("stale_pid_cleanup_command=.\\tools\\smartsteam-forge\\evolution-daemon.cmd -Stop -WorkDir"));
        assert!(json.contains("\"stale_pid_file\":true"));
        assert!(json.contains("\"current_state\":\"not_running_ready_to_start\""));
        assert!(json.contains("\"stale_pid\":4242"));
        assert!(json.contains("\"stale_pid_blocks_start\":false"));
        assert!(json.contains("\"stale_pid_cleanup_command\":\".\\\\tools\\\\smartsteam-forge\\\\evolution-daemon.cmd -Stop -WorkDir"));
        assert!(unattended_start_next_step(
            daemon,
            "target\\evolution\\daemon",
            "",
            None,
            &status,
            None
        )
        .contains("ready_to_start: run StartCheck before Start: .\\tools\\smartsteam-forge\\evolution-daemon.cmd -StartCheck -WorkDir"));
    }

    #[test]
    fn start_plan_prefers_candidate_backlog_block_over_start() {
        let work_dir = temp_work_dir("smartsteam-forge-start-plan-candidate");
        fs::write(
            work_dir.join(EVOLUTION_CANDIDATES_FILE),
            r#"{"schema":"smartsteam.evolution_candidate.v1","candidate_id":"candidate-1","status":"accepted","round":"1","case":"case-1","model":"model-a","answer_preview":"dirty"}"#,
        )
        .unwrap();
        let summary = read_candidate_backlog_summary(&work_dir.join(EVOLUTION_CANDIDATES_FILE))
            .unwrap()
            .unwrap();
        let status = read_report_gate_status(None);

        let json = unattended_start_plan_json(
            r#"{"running":false,"stale_pid_file":false}"#,
            &work_dir.to_string_lossy(),
            "127.0.0.1:7878",
            Some(&summary),
            &status,
            None,
        );
        let _ = fs::remove_dir_all(&work_dir);

        assert!(json.contains("\"candidate_lifecycle_ready\":false"));
        assert!(json.contains("\"can_start\":false"));
        assert!(json.contains("\"current_state\":\"not_running_blocked\""));
        assert!(json.contains("\"block_reason\":\"candidate_backlog_not_ready\""));
        assert_eq!(
            unattended_start_next_step(
                r#"{"running":false,"stale_pid_file":false}"#,
                &work_dir.to_string_lossy(),
                "127.0.0.1:7878",
                Some(&summary),
                &status,
                None
            ),
            "blocked: resolve candidate backlog before unattended evolution"
        );
    }

    #[test]
    fn start_plan_blocks_when_previous_report_gate_is_not_ready() {
        let work_dir = temp_work_dir("smartsteam-forge-start-plan-report-gate");
        let report_path = work_dir.join("report.json");
        fs::write(
            &report_path,
            r#"{
                "report_gate": {"passed": false, "failures": ["model_pool_alignment"]},
                "ledger_gate_report_v1": {"allow_next_round": false, "gate_blocked": true},
                "model_pool_alignment": {"alignment_ok": false}
            }"#,
        )
        .unwrap();
        let report_path_json = report_path.to_string_lossy().replace('\\', "\\\\");
        let loop_status =
            format!(r#"{{"report": {{"path": "{report_path_json}", "exists": true}}}}"#);
        let status = read_report_gate_status(json_object_field(
            &format!(r#"{{"loop":{loop_status}}}"#),
            "loop",
        ));

        let json = unattended_start_plan_json(
            r#"{"running":false,"stale_pid_file":false}"#,
            &work_dir.to_string_lossy(),
            "127.0.0.1:7878",
            None,
            &status,
            None,
        );
        let _ = fs::remove_dir_all(&work_dir);

        assert!(json.contains("\"can_start\":false"));
        assert!(json.contains("\"current_state\":\"not_running_blocked\""));
        assert!(json.contains("\"block_reason\":\"report_gate_not_ready\""));
        assert!(json.contains("\"report_gate_continuation_state\":\"blocked\""));
        assert!(json.contains("\"continuation_block_reason\":\"report_gate_not_ready\""));
        assert_eq!(
            unattended_start_next_step(
                r#"{"running":false,"stale_pid_file":false}"#,
                &work_dir.to_string_lossy(),
                "127.0.0.1:7878",
                None,
                &status,
                None
            ),
            "blocked: fix report gate before unattended evolution"
        );
    }

    #[test]
    fn start_plan_warns_on_remote_runtime_acceleration_without_blocking_start() {
        let status = read_report_gate_status(None);
        let remote_chain = r#"{
            "checked": true,
            "ready": true,
            "remote_runtime": {
                "acceleration_ok": false,
                "cpu_or_no_gpu_roles": ["summary", "review", "test-gate"],
                "acceleration_next_step": ".\\tools\\smartsteam-forge\\run-remote-gemma-unattended.cmd -RestartRemote -SkipBuild"
            }
        }"#;

        let lines = unattended_start_plan_lines(
            r#"{"running":false,"stale_pid_file":false}"#,
            "target\\evolution\\daemon",
            "127.0.0.1:7979",
            None,
            &status,
            Some(remote_chain),
        )
        .join("\n");
        let json = unattended_start_plan_json(
            r#"{"running":false,"stale_pid_file":false}"#,
            "target\\evolution\\daemon",
            "127.0.0.1:7979",
            None,
            &status,
            Some(remote_chain),
        );

        assert!(lines.contains("unattended_start_plan can_start=true"));
        assert!(lines.contains("current_state=not_running_ready_to_start"));
        assert!(lines.contains("unattended_start_acceleration ok=false blocks_start=false"));
        assert!(lines.contains("cpu_or_no_gpu_roles=summary,review,test-gate"));
        assert!(lines.contains(
            "next_step=.\\tools\\smartsteam-forge\\run-remote-gemma-unattended.cmd -RestartRemote -SkipBuild"
        ));
        assert!(json.contains("\"can_start\":true"));
        assert!(json.contains("\"current_state\":\"not_running_ready_to_start\""));
        assert!(json.contains("\"remote_runtime_acceleration_ok\":false"));
        assert!(json.contains("\"remote_runtime_acceleration_blocks_start\":false"));
        assert!(json.contains(
            "\"remote_runtime_cpu_or_no_gpu_roles\":[\"summary\",\"review\",\"test-gate\"]"
        ));
        assert!(json.contains(
            "\"remote_runtime_acceleration_next_step\":\".\\\\tools\\\\smartsteam-forge\\\\run-remote-gemma-unattended.cmd -RestartRemote -SkipBuild\""
        ));
    }

    fn temp_work_dir(prefix: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
