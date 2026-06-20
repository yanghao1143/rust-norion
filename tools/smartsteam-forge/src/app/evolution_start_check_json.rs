use super::evolution_daemon_args::EvolutionDaemonStartOptions;
use super::evolution_readiness_start_status::{ReadinessStartStatus, readiness_start_gate_json};
use super::evolution_start_command_preview::EvolutionStartCommandPreview;
use super::status_json::{
    bool_value_text, json_bool_field, json_object_field, json_string_field, json_string_literal,
};

pub(super) struct StartCheckJsonInput<'a> {
    pub(super) status: &'a str,
    pub(super) work_dir: &'a str,
    pub(super) backend: Option<&'a str>,
    pub(super) candidate: StartCheckCandidateSnapshot<'a>,
    pub(super) report: StartCheckReportSnapshot<'a>,
    pub(super) readiness: StartCheckReadinessSnapshot<'a>,
    pub(super) command_preview: &'a EvolutionStartCommandPreview,
    pub(super) start_options: EvolutionDaemonStartOptions,
}

pub(super) struct StartCheckCandidateSnapshot<'a> {
    pub(super) preflight: &'a str,
    pub(super) preflight_ready: bool,
    pub(super) backlog_json: &'a str,
    pub(super) start_gate_json: &'a str,
}

pub(super) struct StartCheckReportSnapshot<'a> {
    pub(super) preflight: &'a str,
    pub(super) preflight_ready: bool,
    pub(super) status_json: &'a str,
    pub(super) start_gate_json: &'a str,
}

pub(super) struct StartCheckReadinessSnapshot<'a> {
    pub(super) preflight: &'a str,
    pub(super) preflight_ready: bool,
}

pub(super) fn render_start_check_json(input: StartCheckJsonInput<'_>) -> String {
    let daemon = json_object_field(input.status, "daemon").unwrap_or("{}");
    let status_backend_endpoint = json_object_field(input.status, "loop").and_then(|loop_status| {
        json_string_field(loop_status, "backend_endpoint").filter(|value| !value.trim().is_empty())
    });
    let daemon_running = json_bool_field(daemon, "running").unwrap_or(false);
    let block_reasons = start_check_block_reasons(
        daemon_running,
        input.candidate.preflight_ready,
        input.report.preflight_ready,
        input.readiness.preflight_ready,
    );
    let can_start = block_reasons.is_empty();
    let readiness_gate_json =
        readiness_start_gate_json(&ReadinessStartStatus::from_status(input.status));

    format!(
        "{{\"schema\":\"smartsteam.forge.evolution_start_check.v1\",\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"check_only\":true,\"action\":\"start\",\"work_dir\":{},\"backend\":{},\"effective_backend\":{},\"status_backend_endpoint\":{},\"candidate_preflight_ready\":{},\"report_gate_preflight_ready\":{},\"readiness_preflight_ready\":{},\"candidate_backlog\":{},\"report_gate_status\":{},\"daemon_start_gate\":{},\"report_gate_start_gate\":{},\"readiness_start_gate\":{},\"daemon_running\":{},\"can_start\":{},\"current_state\":{},\"block_reasons\":{},\"candidate_preflight\":{},\"report_gate_preflight\":{},\"readiness_preflight\":{},\"command_preview\":{},\"command_output\":{},\"preview_source\":\"rust_pure_preview\",\"min_runtime_context\":{},\"min_runtime_context_source\":{},\"budget_overrides\":{}}}",
        json_string_literal(input.work_dir),
        json_optional_string(input.backend),
        json_string_literal(&input.command_preview.effective_backend),
        json_optional_string(status_backend_endpoint.as_deref()),
        bool_value_text(input.candidate.preflight_ready),
        bool_value_text(input.report.preflight_ready),
        bool_value_text(input.readiness.preflight_ready),
        input.candidate.backlog_json,
        input.report.status_json,
        input.candidate.start_gate_json,
        input.report.start_gate_json,
        readiness_gate_json,
        bool_value_text(daemon_running),
        bool_value_text(can_start),
        json_string_literal(current_state(daemon_running, can_start)),
        json_string_array(&block_reasons),
        json_string_literal(input.candidate.preflight.trim()),
        json_string_literal(input.report.preflight.trim()),
        json_string_literal(input.readiness.preflight.trim()),
        json_string_literal(&input.command_preview.command),
        json_string_literal(input.command_preview.command_output.trim()),
        input.command_preview.min_runtime_context,
        json_string_literal(input.command_preview.min_runtime_context_source),
        start_options_json(input.start_options)
    )
}

fn start_check_block_reasons(
    daemon_running: bool,
    candidate_preflight_ready: bool,
    report_gate_preflight_ready: bool,
    readiness_preflight_ready: bool,
) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if daemon_running {
        reasons.push("already_running");
    }
    if !candidate_preflight_ready {
        reasons.push("candidate_backlog_not_ready");
    }
    if !report_gate_preflight_ready {
        reasons.push("report_gate_not_ready");
    }
    if !readiness_preflight_ready {
        reasons.push("readiness_not_ready");
    }
    reasons
}

fn current_state(daemon_running: bool, can_start: bool) -> &'static str {
    if daemon_running {
        "running"
    } else if can_start {
        "not_running_ready_to_start"
    } else {
        "not_running_blocked"
    }
}

fn start_options_json(options: EvolutionDaemonStartOptions) -> String {
    format!(
        "{{\"interval_secs\":{},\"max_tokens\":{},\"max_total_tokens\":{},\"max_runtime_secs\":{},\"max_failures\":{},\"max_no_feedback_rounds\":{},\"timeout_secs\":{}}}",
        json_optional_u64(options.interval_secs),
        json_optional_u64(options.max_tokens),
        json_optional_u64(options.max_total_tokens),
        json_optional_u64(options.max_runtime_secs),
        json_optional_u64(options.max_failures),
        json_optional_u64(options.max_no_feedback_rounds),
        json_optional_u64(options.timeout_secs)
    )
}

fn json_optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn json_optional_string(value: Option<&str>) -> String {
    value
        .map(json_string_literal)
        .unwrap_or_else(|| "null".to_owned())
}

fn json_string_array(values: &[&str]) -> String {
    let items = values
        .iter()
        .map(|value| json_string_literal(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::evolution_status_contract::validate_read_only_start_check;

    #[test]
    fn start_check_json_surfaces_budgeted_command_preview() {
        let status = r#"{
            "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false, "running": false},
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "backend_endpoint": "127.0.0.1:7979",
                "readiness": {"ready": true, "failures": []}
            }
        }"#;
        let command_preview = EvolutionStartCommandPreview {
            command: "powershell.exe -NoProfile -File start.ps1 -Backend 127.0.0.1:7979 -MaxTokens 64 -MaxTotalTokens 96"
                .to_owned(),
            command_output: "check_only=true\nstarts_process=false\nsends_prompt=false\nmin_runtime_context=65536\nmin_runtime_context_source=status_model_pool\ncommand=powershell.exe -NoProfile -File start.ps1 -Backend 127.0.0.1:7979 -MaxTokens 64 -MaxTotalTokens 96".to_owned(),
            effective_backend: "127.0.0.1:7979".to_owned(),
            min_runtime_context: 65536,
            min_runtime_context_source: "status_model_pool",
        };

        let json = render_start_check_json(StartCheckJsonInput {
            status,
            work_dir: "target\\evolution\\daemon",
            backend: Some("127.0.0.1:7979"),
            candidate: StartCheckCandidateSnapshot {
                preflight: "candidate_preflight ready=true accepted_pending=0",
                preflight_ready: true,
                backlog_json: "{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"path\":\"target\\\\evolution\\\\daemon\\\\evolution-candidates.jsonl\",\"exists\":false,\"total\":0,\"validation_ready\":true}",
                start_gate_json: "{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"candidate_lifecycle_ready\":true,\"blocks_unattended_start\":false,\"accepted_pending\":0,\"implemented_unvalidated\":0,\"implemented_failed\":0,\"invalid\":0}",
            },
            report: StartCheckReportSnapshot {
                preflight: "report_gate_preflight blocks_continuation=false",
                preflight_ready: true,
                status_json: "{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"report_exists\":false,\"can_continue_unattended\":false}",
                start_gate_json: "{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"report_exists\":false,\"blocks_continuation\":false}",
            },
            readiness: StartCheckReadinessSnapshot {
                preflight: "readiness_preflight status_ready=true start_ready=true blocks_start=false failures=none start_blocking_failures=none",
                preflight_ready: true,
            },
            command_preview: &command_preview,
            start_options: EvolutionDaemonStartOptions {
                interval_secs: Some(1),
                max_tokens: Some(64),
                max_total_tokens: Some(96),
                max_runtime_secs: Some(0),
                max_failures: Some(1),
                max_no_feedback_rounds: Some(0),
                timeout_secs: Some(300),
            },
        });

        validate_read_only_start_check(&json).unwrap();
        assert!(json.contains("\"schema\":\"smartsteam.forge.evolution_start_check.v1\""));
        assert!(json.contains("\"read_only\":true"));
        assert!(json.contains("\"starts_process\":false"));
        assert!(json.contains("\"sends_prompt\":false"));
        assert!(json.contains("\"check_only\":true"));
        assert!(json.contains("\"preview_source\":\"rust_pure_preview\""));
        assert!(json.contains("\"effective_backend\":\"127.0.0.1:7979\""));
        assert!(json.contains("\"status_backend_endpoint\":\"127.0.0.1:7979\""));
        assert!(json.contains("\"candidate_preflight_ready\":true"));
        assert!(json.contains("\"report_gate_preflight_ready\":true"));
        assert!(json.contains("\"readiness_preflight_ready\":true"));
        assert!(json.contains("\"candidate_backlog\":{"));
        assert!(json.contains("\"validation_ready\":true"));
        assert!(json.contains("\"report_gate_status\":{"));
        assert!(json.contains("\"can_continue_unattended\":false"));
        assert!(json.contains("\"daemon_start_gate\":{"));
        assert!(json.contains("\"candidate_lifecycle_ready\":true"));
        assert!(json.contains("\"blocks_unattended_start\":false"));
        assert!(json.contains("\"report_gate_start_gate\":{"));
        assert!(json.contains("\"report_exists\":false"));
        assert!(json.contains("\"blocks_continuation\":false"));
        assert!(json.contains("\"readiness_start_gate\":{"));
        assert!(json.contains("\"status_ready\":true"));
        assert!(json.contains("\"start_ready\":true"));
        assert!(json.contains("\"blocks_start\":false"));
        assert!(json.contains("\"start_blocking_failures\":[]"));
        assert!(json.contains("\"daemon_running\":false"));
        assert!(json.contains("\"can_start\":true"));
        assert!(json.contains("\"current_state\":\"not_running_ready_to_start\""));
        assert!(json.contains("\"block_reasons\":[]"));
        assert!(json.contains("\"command_preview\":\"powershell.exe -NoProfile -File start.ps1 -Backend 127.0.0.1:7979 -MaxTokens 64 -MaxTotalTokens 96\""));
        assert!(json.contains("\"min_runtime_context\":65536"));
        assert!(json.contains("\"min_runtime_context_source\":\"status_model_pool\""));
        assert!(json.contains("\"readiness_preflight\":\"readiness_preflight status_ready=true start_ready=true blocks_start=false failures=none start_blocking_failures=none\""));
        assert!(json.contains("\"max_tokens\":64"));
        assert!(json.contains("\"max_total_tokens\":96"));
        assert!(json.contains("\"max_runtime_secs\":0"));
        assert!(json.contains("\"max_no_feedback_rounds\":0"));
    }

    #[test]
    fn start_check_json_blocks_when_preflights_or_preview_are_missing() {
        let status = r#"{
            "daemon": {"read_only": true, "starts_process": false, "sends_prompt": false, "running": true},
            "loop": {
                "read_only": true,
                "starts_process": false,
                "sends_prompt": false,
                "readiness": {
                    "ready": false,
                    "failures": ["backend_not_ready", "ledger_missing"]
                }
            }
        }"#;

        let command_preview = EvolutionStartCommandPreview {
            command: "powershell.exe -NoProfile -File start.ps1 -Backend 127.0.0.1:7979"
                .to_owned(),
            command_output:
                "check_only=true\nstarts_process=false\nsends_prompt=false\nmin_runtime_context=262144\nmin_runtime_context_source=fallback\ncommand=powershell.exe -NoProfile -File start.ps1 -Backend 127.0.0.1:7979"
                    .to_owned(),
            effective_backend: "127.0.0.1:7979".to_owned(),
            min_runtime_context: 262144,
            min_runtime_context_source: "fallback",
        };

        let json = render_start_check_json(StartCheckJsonInput {
            status,
            work_dir: "target\\evolution\\daemon",
            backend: None,
            candidate: StartCheckCandidateSnapshot {
                preflight: "candidate_preflight ready=false accepted_pending=1",
                preflight_ready: false,
                backlog_json: "{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"path\":\"target\\\\evolution\\\\daemon\\\\evolution-candidates.jsonl\",\"exists\":true,\"total\":1,\"validation_ready\":false}",
                start_gate_json: "{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"candidate_lifecycle_ready\":false,\"blocks_unattended_start\":true,\"accepted_pending\":1,\"implemented_unvalidated\":0,\"implemented_failed\":0,\"invalid\":0}",
            },
            report: StartCheckReportSnapshot {
                preflight: "report_gate_preflight blocks_continuation=true",
                preflight_ready: false,
                status_json: "{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"report_exists\":true,\"can_continue_unattended\":false}",
                start_gate_json: "{\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"report_exists\":true,\"blocks_continuation\":true,\"block_reason\":\"report_gate_not_ready\"}",
            },
            readiness: StartCheckReadinessSnapshot {
                preflight: "readiness_preflight status_ready=false start_ready=false blocks_start=true failures=backend_not_ready start_blocking_failures=backend_not_ready",
                preflight_ready: false,
            },
            command_preview: &command_preview,
            start_options: EvolutionDaemonStartOptions::default(),
        });

        validate_read_only_start_check(&json).unwrap();
        assert!(json.contains("\"daemon_running\":true"));
        assert!(json.contains("\"can_start\":false"));
        assert!(json.contains("\"current_state\":\"running\""));
        assert!(json.contains(
            "\"block_reasons\":[\"already_running\",\"candidate_backlog_not_ready\",\"report_gate_not_ready\",\"readiness_not_ready\"]"
        ));
        assert!(json.contains("\"candidate_backlog\":{"));
        assert!(json.contains("\"validation_ready\":false"));
        assert!(json.contains("\"report_gate_status\":{"));
        assert!(json.contains("\"can_continue_unattended\":false"));
        assert!(json.contains("\"daemon_start_gate\":{"));
        assert!(json.contains("\"candidate_lifecycle_ready\":false"));
        assert!(json.contains("\"blocks_unattended_start\":true"));
        assert!(json.contains("\"report_gate_start_gate\":{"));
        assert!(json.contains("\"report_exists\":true"));
        assert!(json.contains("\"blocks_continuation\":true"));
        assert!(json.contains("\"block_reason\":\"report_gate_not_ready\""));
        assert!(json.contains("\"readiness_preflight_ready\":false"));
        assert!(json.contains("\"readiness_start_gate\":{"));
        assert!(json.contains("\"status_ready\":false"));
        assert!(json.contains("\"start_ready\":false"));
        assert!(json.contains("\"blocks_start\":true"));
        assert!(json.contains("\"failures\":[\"backend_not_ready\",\"ledger_missing\"]"));
        assert!(json.contains("\"start_blocking_failures\":[\"backend_not_ready\"]"));
        assert!(json.contains("\"block_reason\":\"readiness_not_ready\""));
        assert!(json.contains(
            "\"command_preview\":\"powershell.exe -NoProfile -File start.ps1 -Backend 127.0.0.1:7979\""
        ));
        assert!(json.contains("\"backend\":null"));
        assert!(json.contains("\"status_backend_endpoint\":null"));
    }
}
