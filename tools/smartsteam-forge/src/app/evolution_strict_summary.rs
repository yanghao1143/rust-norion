use std::{
    fs,
    io::{self, Write},
};

use super::evolution_daemon_process::resolve_repo_path;
use super::status_json::{
    bool_value_text, compact_line, json_bool_field, json_number_field, json_object_field,
    json_string_array_field, json_string_field, json_string_literal, json_top_level_bool_field,
    json_top_level_number_field, json_top_level_string_field, scalar_value,
};

const DEFAULT_STRICT_SUMMARY_PATH: &str = "target\\evolution\\strict-status-summary.json";
const STRICT_SUMMARY_CONTRACT: &str = "smartsteam.evolution-loop.strict-status-summary.v1";
const FORGE_STRICT_SUMMARY_SCHEMA: &str = "smartsteam.forge.evolution_strict_summary.v1";

pub fn run_evolution_strict_summary(path: Option<&str>, json_status: bool) -> io::Result<()> {
    let mut stdout = io::stdout();
    run_evolution_strict_summary_to(path, json_status, &mut stdout)
}

pub(in crate::app) fn load_evolution_strict_summary_text(path: Option<&str>) -> io::Result<String> {
    let path = path.unwrap_or(DEFAULT_STRICT_SUMMARY_PATH);
    let resolved = resolve_repo_path(path)?;
    let text = fs::read_to_string(&resolved).map_err(|error| {
        io::Error::other(format!(
            "failed to read strict status summary {}: {error}",
            resolved.display()
        ))
    })?;
    validate_strict_summary_artifact(&text)?;
    render_strict_summary_text(&text, &resolved.display().to_string())
}

fn run_evolution_strict_summary_to<W: Write>(
    path: Option<&str>,
    json_status: bool,
    output: &mut W,
) -> io::Result<()> {
    let path = path.unwrap_or(DEFAULT_STRICT_SUMMARY_PATH);
    let resolved = resolve_repo_path(path)?;
    let text = fs::read_to_string(&resolved).map_err(|error| {
        io::Error::other(format!(
            "failed to read strict status summary {}: {error}",
            resolved.display()
        ))
    })?;
    validate_strict_summary_artifact(&text)?;

    if json_status {
        writeln!(
            output,
            "{}",
            render_strict_summary_json(&text, &resolved.display().to_string())
        )?;
    } else {
        writeln!(
            output,
            "{}",
            render_strict_summary_text(&text, &resolved.display().to_string())?
        )?;
    }
    output.flush()
}

fn validate_strict_summary_artifact(text: &str) -> io::Result<()> {
    match json_top_level_string_field(text, "contract_version").as_deref() {
        Some(STRICT_SUMMARY_CONTRACT) => {}
        Some(actual) => {
            return Err(io::Error::other(format!(
                "strict summary unexpected contract_version={actual}"
            )));
        }
        None => return Err(io::Error::other("strict summary missing contract_version")),
    }
    match json_top_level_bool_field(text, "starts_process") {
        Some(false) => {}
        Some(true) => {
            return Err(io::Error::other(
                "strict summary artifact unexpectedly starts processes",
            ));
        }
        None => return Err(io::Error::other("strict summary missing starts_process")),
    }
    match json_top_level_bool_field(text, "sends_prompt") {
        Some(false) => {}
        Some(true) => {
            return Err(io::Error::other(
                "strict summary artifact unexpectedly sends prompts",
            ));
        }
        None => return Err(io::Error::other("strict summary missing sends_prompt")),
    }
    json_object_field(text, "readiness")
        .ok_or_else(|| io::Error::other("strict summary missing readiness object"))?;
    json_object_field(text, "summary")
        .ok_or_else(|| io::Error::other("strict summary missing summary object"))?;
    Ok(())
}

fn render_strict_summary_json(text: &str, path: &str) -> String {
    let readiness = json_object_field(text, "readiness").unwrap_or("{}");
    let summary = json_object_field(text, "summary").unwrap_or("{}");
    format!(
        "{{\"schema\":{},\"read_only\":true,\"starts_process\":false,\"sends_prompt\":false,\"summary_path\":{},\"snapshot_fresh\":{},\"ready\":{},\"strict_status_ready\":{},\"daemon_state\":{},\"active_round\":{},\"latest_round\":{},\"round_lag\":{},\"latest_case\":{},\"latest_success\":{},\"self_improve_passed\":{},\"validation_passed\":{},\"validation_source\":{},\"validation_status_code\":{},\"test_gate_passed\":{},\"test_gate_verdict\":{},\"test_gate_validation_command_safety\":{},\"helper_stage_role_count\":{},\"helper_stage_contract_complete\":{},\"remote_chain_ready\":{},\"backend_model\":{},\"backend_busy\":{},\"success_rate\":{},\"total_records\":{},\"strict_status_summary\":{}}}",
        json_string_literal(FORGE_STRICT_SUMMARY_SCHEMA),
        json_string_literal(path),
        strict_summary_freshness_json(text),
        json_bool_value(readiness, "ready"),
        json_bool_value(readiness, "strict_status_ready"),
        json_string_value(summary, "daemon_state"),
        json_number_value(summary, "active_round"),
        json_number_value(summary, "latest_round"),
        strict_summary_round_lag_json(summary),
        json_string_value(summary, "latest_case"),
        json_bool_value(summary, "latest_success"),
        json_bool_value(summary, "self_improve_passed"),
        json_bool_value(summary, "validation_passed"),
        json_string_value(summary, "validation_source"),
        json_number_value(summary, "validation_status_code"),
        json_bool_value(summary, "test_gate_passed"),
        json_string_value(summary, "test_gate_verdict"),
        json_string_value(summary, "test_gate_validation_command_safety"),
        json_number_value(summary, "helper_stage_role_count"),
        json_bool_value(summary, "helper_stage_contract_complete"),
        json_bool_value(summary, "remote_chain_ready"),
        json_string_value(summary, "backend_model"),
        json_bool_value(summary, "backend_busy"),
        json_number_value(summary, "success_rate"),
        json_number_value(summary, "total_records"),
        text.trim()
    )
}

fn render_strict_summary_text(text: &str, path: &str) -> io::Result<String> {
    let readiness = json_object_field(text, "readiness")
        .ok_or_else(|| io::Error::other("strict summary missing readiness object"))?;
    let summary = json_object_field(text, "summary")
        .ok_or_else(|| io::Error::other("strict summary missing summary object"))?;
    let failures = json_string_array_field(readiness, "failures")
        .filter(|values| !values.is_empty())
        .map(|values| values.join(","))
        .unwrap_or_else(|| "none".to_owned());
    let helper_roles = json_string_array_field(summary, "helper_stage_roles")
        .filter(|values| !values.is_empty())
        .map(|values| values.join(","))
        .unwrap_or_else(|| "none".to_owned());

    Ok([
        "SmartSteam strict evolution summary".to_owned(),
        "read_only=true starts_process=false sends_prompt=false".to_owned(),
        format!("summary_path={path}"),
        format!(
            "source_snapshot={} snapshot_age_seconds={} max_snapshot_age_seconds={} snapshot_fresh={}",
            string_value(text, "source_snapshot"),
            scalar_value(text, "snapshot_age_seconds"),
            scalar_value(text, "max_snapshot_age_seconds"),
            strict_summary_freshness_text(text)
        ),
        format!(
            "readiness ready={} strict_status_ready={} failures={}",
            bool_value(readiness, "ready"),
            bool_value(readiness, "strict_status_ready"),
            failures
        ),
        format!(
            "daemon state={} active_round={} latest_round={} latest_case={} latest_success={}",
            string_value(summary, "daemon_state"),
            scalar_value(summary, "active_round"),
            scalar_value(summary, "latest_round"),
            string_value(summary, "latest_case"),
            bool_value(summary, "latest_success")
        ),
        format!(
            "quality self_improve_passed={} validation_passed={} validation_source={} validation_status_code={} test_gate={} safe_command={}",
            bool_value(summary, "self_improve_passed"),
            bool_value(summary, "validation_passed"),
            string_value(summary, "validation_source"),
            scalar_value(summary, "validation_status_code"),
            string_value(summary, "test_gate_verdict"),
            string_value(summary, "test_gate_validation_command_safety")
        ),
        format!(
            "helpers roles={} count={} contract_complete={}",
            helper_roles,
            scalar_value(summary, "helper_stage_role_count"),
            bool_value(summary, "helper_stage_contract_complete")
        ),
        format!(
            "remote_chain_ready={} backend_model={} backend_busy={} success_rate={} total_records={}",
            bool_value(summary, "remote_chain_ready"),
            string_value(summary, "backend_model"),
            bool_value(summary, "backend_busy"),
            scalar_value(summary, "success_rate"),
            scalar_value(summary, "total_records")
        ),
        format!("next_step={}", compact_line(&string_value(text, "next_step"), 240)),
    ]
    .join("\n"))
}

fn bool_value(object: &str, field: &str) -> &'static str {
    match json_bool_field(object, field) {
        Some(value) => bool_value_text(value),
        None => "unknown",
    }
}

fn string_value(object: &str, field: &str) -> String {
    json_string_field(object, field).unwrap_or_else(|| "unknown".to_owned())
}

fn json_bool_value(object: &str, field: &str) -> &'static str {
    json_bool_field(object, field).map_or("null", bool_value_text)
}

fn json_number_value(object: &str, field: &str) -> String {
    json_number_field(object, field).unwrap_or_else(|| "null".to_owned())
}

fn json_string_value(object: &str, field: &str) -> String {
    json_string_field(object, field)
        .map(|value| json_string_literal(&value))
        .unwrap_or_else(|| "null".to_owned())
}

fn strict_summary_freshness(text: &str) -> Option<bool> {
    let age = json_top_level_number_field(text, "snapshot_age_seconds")
        .and_then(|value| value.parse::<u64>().ok());
    let max_age = json_top_level_number_field(text, "max_snapshot_age_seconds")
        .and_then(|value| value.parse::<u64>().ok());
    match (age, max_age) {
        (Some(age), Some(max_age)) => Some(age <= max_age),
        _ => None,
    }
}

fn strict_summary_freshness_text(text: &str) -> &'static str {
    strict_summary_freshness(text).map_or("unknown", bool_value_text)
}

fn strict_summary_freshness_json(text: &str) -> &'static str {
    strict_summary_freshness(text).map_or("null", bool_value_text)
}

fn strict_summary_round_lag_json(summary: &str) -> String {
    let active =
        json_number_field(summary, "active_round").and_then(|value| value.parse::<i64>().ok());
    let latest =
        json_number_field(summary, "latest_round").and_then(|value| value.parse::<i64>().ok());
    match (active, latest) {
        (Some(active), Some(latest)) => active.saturating_sub(latest).to_string(),
        _ => "null".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STRICT_SUMMARY: &str = r#"{
        "schema_version": 1,
        "contract_version": "smartsteam.evolution-loop.strict-status-summary.v1",
        "starts_process": false,
        "sends_prompt": false,
        "source_snapshot": "D:\\rust-norion\\target\\evolution\\strict-status.json",
        "snapshot_age_seconds": 1,
        "max_snapshot_age_seconds": 900,
        "readiness": {
            "ready": true,
            "failures": [],
            "strict_status_ready": true
        },
        "summary": {
            "latest_round": 148,
            "active_round": 149,
            "daemon_state": "active",
            "latest_case": "smartsteam-evolution-loop-0148",
            "latest_success": true,
            "feedback_applied": 4,
            "self_improve_passed": true,
            "validation_passed": true,
            "validation_source": "configured",
            "validation_status_code": 0,
            "helper_stage_roles": ["index", "review", "router", "summary", "test-gate"],
            "helper_stage_role_count": 5,
            "helper_stage_contract_complete": true,
            "test_gate_passed": true,
            "test_gate_verdict": "pass",
            "test_gate_validation_command_safety": "safe",
            "remote_chain_ready": true,
            "backend_model": "gemma-4-12b-it-Q8_0.gguf",
            "backend_busy": false,
            "success_rate": 100,
            "total_records": 148
        },
        "next_step": "strict status snapshot is ready"
    }"#;

    #[test]
    fn renders_operator_strict_summary_from_artifact() {
        validate_strict_summary_artifact(STRICT_SUMMARY).unwrap();

        let text = render_strict_summary_text(
            STRICT_SUMMARY,
            "target\\evolution\\strict-status-summary.json",
        )
        .unwrap();

        assert!(text.contains("SmartSteam strict evolution summary"));
        assert!(text.contains("read_only=true starts_process=false sends_prompt=false"));
        assert!(
            text.contains(
                "snapshot_age_seconds=1 max_snapshot_age_seconds=900 snapshot_fresh=true"
            )
        );
        assert!(text.contains("readiness ready=true strict_status_ready=true failures=none"));
        assert!(text.contains(
            "daemon state=active active_round=149 latest_round=148 latest_case=smartsteam-evolution-loop-0148 latest_success=true"
        ));
        assert!(text.contains(
            "helpers roles=index,review,router,summary,test-gate count=5 contract_complete=true"
        ));
        assert!(text.contains(
            "remote_chain_ready=true backend_model=gemma-4-12b-it-Q8_0.gguf backend_busy=false success_rate=100 total_records=148"
        ));
    }

    #[test]
    fn renders_stale_strict_summary_freshness() {
        let stale_summary = STRICT_SUMMARY.replace(
            "\"snapshot_age_seconds\": 1",
            "\"snapshot_age_seconds\": 901",
        );

        let text = render_strict_summary_text(&stale_summary, "summary.json").unwrap();

        assert!(text.contains(
            "snapshot_age_seconds=901 max_snapshot_age_seconds=900 snapshot_fresh=false"
        ));
    }

    #[test]
    fn json_mode_wraps_raw_summary_with_forge_read_only_contract() {
        let text = render_strict_summary_json(STRICT_SUMMARY, "summary.json");

        assert!(text.contains("\"schema\":\"smartsteam.forge.evolution_strict_summary.v1\""));
        assert!(text.contains("\"read_only\":true"));
        assert!(text.contains("\"starts_process\":false"));
        assert!(text.contains("\"sends_prompt\":false"));
        assert!(text.contains("\"summary_path\":\"summary.json\""));
        assert!(text.contains("\"snapshot_fresh\":true"));
        assert!(text.contains("\"ready\":true"));
        assert!(text.contains("\"strict_status_ready\":true"));
        assert!(text.contains("\"daemon_state\":\"active\""));
        assert!(text.contains("\"active_round\":149"));
        assert!(text.contains("\"latest_round\":148"));
        assert!(text.contains("\"round_lag\":1"));
        assert!(text.contains("\"latest_case\":\"smartsteam-evolution-loop-0148\""));
        assert!(text.contains("\"latest_success\":true"));
        assert!(text.contains("\"self_improve_passed\":true"));
        assert!(text.contains("\"validation_passed\":true"));
        assert!(text.contains("\"validation_source\":\"configured\""));
        assert!(text.contains("\"validation_status_code\":0"));
        assert!(text.contains("\"test_gate_passed\":true"));
        assert!(text.contains("\"test_gate_verdict\":\"pass\""));
        assert!(text.contains("\"test_gate_validation_command_safety\":\"safe\""));
        assert!(text.contains("\"helper_stage_role_count\":5"));
        assert!(text.contains("\"helper_stage_contract_complete\":true"));
        assert!(text.contains("\"remote_chain_ready\":true"));
        assert!(text.contains("\"backend_model\":\"gemma-4-12b-it-Q8_0.gguf\""));
        assert!(text.contains("\"backend_busy\":false"));
        assert!(text.contains("\"success_rate\":100"));
        assert!(text.contains("\"total_records\":148"));
        assert!(text.contains("\"strict_status_summary\":{"));
        assert!(text.contains(
            "\"contract_version\": \"smartsteam.evolution-loop.strict-status-summary.v1\""
        ));
    }

    #[test]
    fn json_mode_exposes_stale_summary_freshness() {
        let stale_summary = STRICT_SUMMARY.replace(
            "\"snapshot_age_seconds\": 1",
            "\"snapshot_age_seconds\": 901",
        );

        let text = render_strict_summary_json(&stale_summary, "summary.json");

        assert!(text.contains("\"snapshot_fresh\":false"));
        assert!(text.contains("\"strict_status_summary\":{"));
    }

    #[test]
    fn rejects_summary_that_would_start_processes() {
        let unsafe_summary =
            STRICT_SUMMARY.replace("\"starts_process\": false", "\"starts_process\": true");

        let error = validate_strict_summary_artifact(&unsafe_summary).unwrap_err();

        assert!(error.to_string().contains("starts processes"));
    }
}
