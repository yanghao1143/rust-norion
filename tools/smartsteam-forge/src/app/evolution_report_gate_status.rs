use std::fs;

use super::status_json::{
    bool_value_text, json_bool_field, json_object_field, json_string_array_field,
    json_string_field, json_string_literal, json_top_level_object_field,
};

pub(super) fn render_report_gate_continuation_preflight(status: &str) -> String {
    let loop_status = json_object_field(status, "loop");
    let report_gate_status = read_report_gate_status(loop_status);
    format!(
        "report_gate_preflight read_only=true starts_process=false sends_prompt=false report_exists={} report_read_ok={} continuation_state={} can_continue_unattended={} blocks_continuation={} block_reason={} report_path={}",
        bool_value_text(report_gate_status.report_exists()),
        bool_value_text(report_gate_status.report_read_ok()),
        report_gate_status.continuation_state(),
        bool_value_text(report_gate_status.can_continue_unattended()),
        bool_value_text(report_gate_status.continuation_block_reason().is_some()),
        report_gate_status
            .continuation_block_reason()
            .unwrap_or("none"),
        report_gate_status.report_path()
    )
}

pub(super) fn report_gate_continuation_preflight_ready(preflight: &str) -> bool {
    preflight.lines().any(|line| {
        line.contains("report_gate_preflight ") && line.contains(" blocks_continuation=false ")
    })
}

pub(super) fn read_report_gate_status(loop_status: Option<&str>) -> ReportGateStatus {
    let report = loop_status.and_then(|loop_status| json_object_field(loop_status, "report"));
    let report_path = report
        .and_then(|report| json_string_field(report, "path"))
        .unwrap_or_default();
    let report_exists = report
        .and_then(|report| json_bool_field(report, "exists"))
        .unwrap_or_else(|| !report_path.trim().is_empty() && fs::metadata(&report_path).is_ok());

    let mut status = ReportGateStatus {
        report_path,
        report_exists,
        ..ReportGateStatus::default()
    };

    if !status.report_exists {
        return status;
    }

    match fs::read_to_string(&status.report_path) {
        Ok(report_json) => {
            status.report_read_ok = true;
            status.report_error.clear();
            status.apply_report_json(&report_json);
        }
        Err(error) => {
            status.report_error = error.to_string();
        }
    }

    status
}

pub(super) fn report_gate_preflight_json(
    status: &ReportGateStatus,
    work_dir: &str,
    backend_endpoint: &str,
) -> String {
    let block_reason = status
        .continuation_block_reason()
        .map(json_string_literal)
        .unwrap_or_else(|| "null".to_owned());
    let status_command = evolution_daemon_command("-JsonStatus", work_dir, backend_endpoint);
    let start_check_command = evolution_daemon_command("-StartCheck", work_dir, backend_endpoint);

    format!(
        "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"report_exists\":{},\"report_read_ok\":{},\"continuation_state\":{},\"can_continue_unattended\":{},\"blocks_continuation\":{},\"block_reason\":{},\"repair_hint\":{},\"report_path\":{},\"inspect_status_command\":{},\"start_check_command\":{}}}",
        bool_value_text(status.report_exists()),
        bool_value_text(status.report_read_ok()),
        json_string_literal(status.continuation_state()),
        bool_value_text(status.can_continue_unattended()),
        bool_value_text(status.continuation_block_reason().is_some()),
        block_reason,
        json_string_literal(status.repair_hint()),
        json_string_literal(status.report_path()),
        json_string_literal(&status_command),
        json_string_literal(&start_check_command)
    )
}

pub(super) fn evolution_daemon_command(
    action: &str,
    work_dir: &str,
    backend_endpoint: &str,
) -> String {
    let mut command = format!(
        ".\\tools\\smartsteam-forge\\evolution-daemon.cmd {action} -WorkDir {}",
        powershell_single_quoted(work_dir)
    );
    if !backend_endpoint.trim().is_empty() {
        command.push_str(" -Backend ");
        command.push_str(&powershell_single_quoted(backend_endpoint.trim()));
    }
    command
}

#[derive(Default)]
pub(super) struct ReportGateStatus {
    report_path: String,
    report_exists: bool,
    report_read_ok: bool,
    report_error: String,
    report_gate_passed: Option<bool>,
    strict_report_gate_passed: Option<bool>,
    strict_report_gate_failure_reasons: Vec<String>,
    ledger_gate_allow_next_round: Option<bool>,
    ledger_gate_blocked: Option<bool>,
    continuation_gate_allow_unattended: Option<bool>,
    continuation_gate_blocked: Option<bool>,
    continuation_gate_failure_reasons: Vec<String>,
    model_pool_alignment_ok: Option<bool>,
    model_pool_route_dependency_failures: Vec<String>,
    model_pool_route_blocked_or_failed: Vec<String>,
    model_pool_missing_status_roles: Vec<String>,
    model_pool_missing_status_helper_roles: Vec<String>,
    test_gate_verdict: Option<String>,
    test_gate_validation_command_safety: Option<String>,
}

impl ReportGateStatus {
    pub(super) fn report_path(&self) -> &str {
        &self.report_path
    }

    fn report_exists(&self) -> bool {
        self.report_exists
    }

    fn report_read_ok(&self) -> bool {
        self.report_read_ok
    }

    fn apply_report_json(&mut self, report_json: &str) {
        if let Some(report_gate) = json_top_level_object_field(report_json, "report_gate") {
            self.report_gate_passed = json_bool_field(report_gate, "passed");
        }

        if let Some(strict_gate) = json_top_level_object_field(report_json, "strict_report_gate") {
            self.strict_report_gate_passed = json_bool_field(strict_gate, "passed");
            self.strict_report_gate_failure_reasons =
                json_string_array_field(strict_gate, "failures").unwrap_or_default();
        }

        if let Some(ledger_gate) = json_top_level_object_field(report_json, "ledger_gate_report_v1")
        {
            self.ledger_gate_allow_next_round = json_bool_field(ledger_gate, "allow_next_round");
            self.ledger_gate_blocked = json_bool_field(ledger_gate, "gate_blocked");
        }

        if let Some(continuation_gate) =
            json_top_level_object_field(report_json, "continuation_gate_report_v1")
        {
            self.continuation_gate_allow_unattended =
                json_bool_field(continuation_gate, "allow_unattended_continuation");
            self.continuation_gate_blocked = json_bool_field(continuation_gate, "gate_blocked");
            self.continuation_gate_failure_reasons =
                json_string_array_field(continuation_gate, "failure_reasons").unwrap_or_default();
        }

        if let Some(alignment) = json_top_level_object_field(report_json, "model_pool_alignment") {
            self.model_pool_alignment_ok = json_bool_field(alignment, "alignment_ok");
            self.model_pool_route_dependency_failures =
                json_string_array_field(alignment, "route_dependency_failures").unwrap_or_default();
            self.model_pool_route_blocked_or_failed =
                json_string_array_field(alignment, "route_blocked_or_failed").unwrap_or_default();
            self.model_pool_missing_status_roles =
                json_string_array_field(alignment, "missing_status_roles").unwrap_or_default();
            self.model_pool_missing_status_helper_roles =
                json_string_array_field(alignment, "missing_status_helper_roles")
                    .unwrap_or_default();
        }

        if let Some(test_gate) = json_top_level_object_field(report_json, "test_gate") {
            self.test_gate_verdict = json_string_field(test_gate, "latest_verdict");
            self.test_gate_validation_command_safety =
                json_string_field(test_gate, "latest_validation_command_safety");
        }
    }

    pub(super) fn can_continue_unattended(&self) -> bool {
        if self.continuation_gate_allow_unattended.is_some()
            || self.continuation_gate_blocked.is_some()
        {
            return self.continuation_gate_allow_unattended == Some(true)
                && self.continuation_gate_blocked == Some(false)
                && self.model_pool_alignment_ok == Some(true);
        }
        self.report_gate_passed == Some(true)
            && self.ledger_gate_allow_next_round == Some(true)
            && self.ledger_gate_blocked == Some(false)
            && self.model_pool_alignment_ok == Some(true)
    }

    pub(super) fn continuation_state(&self) -> &'static str {
        if !self.report_exists {
            "no_report"
        } else if !self.report_read_ok {
            "unreadable"
        } else if self.can_continue_unattended() {
            "ready"
        } else {
            "blocked"
        }
    }

    pub(super) fn continuation_block_reason(&self) -> Option<&'static str> {
        if !self.report_exists {
            None
        } else if !self.report_read_ok {
            Some("report_unreadable")
        } else if self.can_continue_unattended() {
            None
        } else {
            Some("report_gate_not_ready")
        }
    }

    pub(super) fn repair_hint(&self) -> &'static str {
        if !self.report_exists {
            "no previous report; first unattended start is allowed"
        } else if !self.report_read_ok {
            "report is unreadable; inspect or regenerate the report before unattended continuation"
        } else if self.can_continue_unattended() {
            "previous report gate is ready; run StartCheck before unattended continuation"
        } else {
            "inspect report_gate_status failures and fix them before unattended continuation"
        }
    }

    pub(super) fn to_json(&self) -> String {
        format!(
            "{{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"report_exists\":{},\"report_path\":{},\"report_read_ok\":{},\"report_error\":{},\"report_gate_passed\":{},\"strict_report_gate_passed\":{},\"strict_report_gate_failure_reasons\":{},\"ledger_gate_allow_next_round\":{},\"ledger_gate_blocked\":{},\"continuation_gate_allow_unattended\":{},\"continuation_gate_blocked\":{},\"continuation_gate_failure_reasons\":{},\"model_pool_alignment_ok\":{},\"model_pool_route_dependency_failures\":{},\"model_pool_route_dependency_failure_count\":{},\"model_pool_route_blocked_or_failed\":{},\"model_pool_missing_status_roles\":{},\"model_pool_missing_status_helper_roles\":{},\"test_gate_verdict\":{},\"test_gate_validation_command_safety\":{},\"can_continue_unattended\":{},\"repair_hint\":{}}}",
            bool_value_text(self.report_exists),
            json_string_literal(&self.report_path),
            bool_value_text(self.report_read_ok),
            json_string_literal(&self.report_error),
            json_optional_bool(self.report_gate_passed),
            json_optional_bool(self.strict_report_gate_passed),
            json_string_array_literal(&self.strict_report_gate_failure_reasons),
            json_optional_bool(self.ledger_gate_allow_next_round),
            json_optional_bool(self.ledger_gate_blocked),
            json_optional_bool(self.continuation_gate_allow_unattended),
            json_optional_bool(self.continuation_gate_blocked),
            json_string_array_literal(&self.continuation_gate_failure_reasons),
            json_optional_bool(self.model_pool_alignment_ok),
            json_string_array_literal(&self.model_pool_route_dependency_failures),
            self.model_pool_route_dependency_failures.len(),
            json_string_array_literal(&self.model_pool_route_blocked_or_failed),
            json_string_array_literal(&self.model_pool_missing_status_roles),
            json_string_array_literal(&self.model_pool_missing_status_helper_roles),
            json_optional_string(self.test_gate_verdict.as_deref()),
            json_optional_string(self.test_gate_validation_command_safety.as_deref()),
            bool_value_text(self.can_continue_unattended()),
            json_string_literal(self.repair_hint())
        )
    }
}

fn json_optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    }
}

fn json_optional_string(value: Option<&str>) -> String {
    value
        .map(json_string_literal)
        .unwrap_or_else(|| "null".to_owned())
}

fn json_string_array_literal(values: &[String]) -> String {
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

fn powershell_single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn continuation_preflight_allows_missing_report_and_blocks_failed_report() {
        let work_dir = temp_work_dir("smartsteam-forge-report-preflight");
        fs::create_dir_all(&work_dir).unwrap();
        let missing_report = work_dir.join("missing-report.json");
        let missing_status = status_with_report(&missing_report.to_string_lossy(), false);

        let missing_preflight = render_report_gate_continuation_preflight(&missing_status);

        assert!(report_gate_continuation_preflight_ready(&missing_preflight));
        assert!(missing_preflight.contains("report_exists=false"));
        assert!(missing_preflight.contains("continuation_state=no_report"));
        assert!(missing_preflight.contains("blocks_continuation=false"));

        let failed_report = work_dir.join("report.json");
        fs::write(
            &failed_report,
            r#"{
                "report_gate": {"passed": false, "failures": ["model_pool_alignment"]},
                "ledger_gate_report_v1": {
                    "allow_next_round": false,
                    "gate_blocked": true
                },
                "model_pool_alignment": {"alignment_ok": false}
            }"#,
        )
        .unwrap();
        let failed_status = status_with_report(&failed_report.to_string_lossy(), true);

        let failed_preflight = render_report_gate_continuation_preflight(&failed_status);
        let _ = fs::remove_dir_all(&work_dir);

        assert!(!report_gate_continuation_preflight_ready(&failed_preflight));
        assert!(failed_preflight.contains("report_exists=true"));
        assert!(failed_preflight.contains("report_read_ok=true"));
        assert!(failed_preflight.contains("continuation_state=blocked"));
        assert!(failed_preflight.contains("blocks_continuation=true"));
        assert!(failed_preflight.contains("block_reason=report_gate_not_ready"));
    }

    #[test]
    fn continuation_preflight_prefers_continuation_gate_when_strict_history_failed() {
        let work_dir = temp_work_dir("smartsteam-forge-continuation-preflight");
        fs::create_dir_all(&work_dir).unwrap();
        let report = work_dir.join("report.json");
        fs::write(
            &report,
            r#"{
                "report_gate": {"passed": false, "failures": ["runtime response failures 1 above maximum 0"]},
                "strict_report_gate": {
                    "passed": false,
                    "failures": ["runtime response failures 1 above maximum 0"]
                },
                "ledger_gate_report_v1": {
                    "allow_next_round": false,
                    "gate_blocked": true
                },
                "continuation_gate_report_v1": {
                    "allow_unattended_continuation": true,
                    "gate_blocked": false,
                    "failure_reasons": [],
                    "strict_report_gate_passed": false
                },
                "model_pool_alignment": {"alignment_ok": true}
            }"#,
        )
        .unwrap();
        let status_json = status_with_report(&report.to_string_lossy(), true);
        let loop_status = json_object_field(&status_json, "loop");
        let report_status = read_report_gate_status(loop_status);

        let preflight = render_report_gate_continuation_preflight(&status_json);
        let status_json = report_status.to_json();
        let _ = fs::remove_dir_all(&work_dir);

        assert!(report_status.can_continue_unattended());
        assert!(report_gate_continuation_preflight_ready(&preflight));
        assert!(preflight.contains("continuation_state=ready"));
        assert!(preflight.contains("blocks_continuation=false"));
        assert!(status_json.contains("\"report_gate_passed\":false"));
        assert!(status_json.contains("\"strict_report_gate_passed\":false"));
        assert!(status_json.contains(
            "\"strict_report_gate_failure_reasons\":[\"runtime response failures 1 above maximum 0\"]"
        ));
        assert!(status_json.contains("\"ledger_gate_allow_next_round\":false"));
        assert!(status_json.contains("\"continuation_gate_allow_unattended\":true"));
        assert!(status_json.contains("\"continuation_gate_blocked\":false"));
        assert!(status_json.contains("\"can_continue_unattended\":true"));
    }

    #[test]
    fn preflight_json_surfaces_repair_hint_and_safe_commands() {
        let work_dir = temp_work_dir("smartsteam-forge-report-preflight-json");
        fs::create_dir_all(&work_dir).unwrap();
        let report = work_dir.join("report.json");
        fs::write(
            &report,
            r#"{
                "report_gate": {"passed": false, "failures": ["test_gate"]},
                "ledger_gate_report_v1": {
                    "allow_next_round": false,
                    "gate_blocked": true
                },
                "model_pool_alignment": {"alignment_ok": false}
            }"#,
        )
        .unwrap();
        let status_json = status_with_report(&report.to_string_lossy(), true);
        let loop_status = json_object_field(&status_json, "loop");
        let report_status = read_report_gate_status(loop_status);

        let preflight_json = report_gate_preflight_json(
            &report_status,
            &work_dir.to_string_lossy(),
            "127.0.0.1:7979",
        );
        let _ = fs::remove_dir_all(&work_dir);

        assert!(preflight_json.contains("\"read_only\":true"));
        assert!(preflight_json.contains("\"starts_process\":false"));
        assert!(preflight_json.contains("\"sends_prompt\":false"));
        assert!(preflight_json.contains("\"continuation_state\":\"blocked\""));
        assert!(preflight_json.contains("\"blocks_continuation\":true"));
        assert!(preflight_json.contains("\"block_reason\":\"report_gate_not_ready\""));
        assert!(preflight_json.contains(
            "\"repair_hint\":\"inspect report_gate_status failures and fix them before unattended continuation\""
        ));
        assert!(preflight_json.contains("evolution-daemon.cmd -JsonStatus -WorkDir"));
        assert!(preflight_json.contains("\"inspect_status_command\":\".\\\\tools\\\\smartsteam-forge\\\\evolution-daemon.cmd -JsonStatus -WorkDir"));
        assert!(preflight_json.contains("evolution-daemon.cmd -StartCheck -WorkDir"));
        assert!(preflight_json.contains("-Backend '127.0.0.1:7979'"));
    }

    fn status_with_report(report_path: &str, exists: bool) -> String {
        format!(
            r#"{{
                "daemon": {{"read_only": true, "starts_process": false, "sends_prompt": false}},
                "loop": {{
                    "read_only": true,
                    "starts_process": false,
                    "sends_prompt": false,
                    "touches_remote": false,
                    "report": {{"path": "{}", "exists": {}}}
                }}
            }}"#,
            report_path.replace('\\', "\\\\"),
            bool_value_text(exists)
        )
    }

    fn temp_work_dir(prefix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
