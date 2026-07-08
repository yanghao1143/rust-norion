use super::fields::{extract_json_bool_field, extract_json_string_field, extract_json_usize_field};

pub const EXTERNAL_AGENT_LIFECYCLE_TRACE_SCHEMA: &str = "rust-norion-external-agent-lifecycle-v1";
pub const EXTERNAL_AGENT_TARGET_PROJECT_SCOPE: &str = "rust-norion";

pub(super) fn evaluate_external_agent_lifecycle_schema_line(line: &str) -> Vec<String> {
    let mut failures = Vec::new();

    for (name, marker) in [
        (
            "schema",
            "\"schema\":\"rust-norion-external-agent-lifecycle-v1\"",
        ),
        ("report_kind", "\"report_kind\":"),
        ("agents", "\"agents\":"),
        ("target_project_scope", "\"target_project_scope\":"),
        ("evidence_ready", "\"evidence_ready\":"),
        ("project_scoped", "\"project_scoped\":"),
        ("project_scope_mismatch", "\"project_scope_mismatch\":"),
        ("foreign_project", "\"foreign_project\":"),
        ("missing_evidence", "\"missing_evidence\":"),
        ("stale_evidence", "\"stale_evidence\":"),
        ("working", "\"working\":"),
        ("blocked", "\"blocked\":"),
        ("done", "\"done\":"),
        ("idle", "\"idle\":"),
        ("unknown", "\"unknown\":"),
        ("hold_dependent_task", "\"hold_dependent_task\":"),
        (
            "require_operator_attention",
            "\"require_operator_attention\":",
        ),
        ("eligible_to_continue", "\"eligible_to_continue\":"),
        ("observe_only", "\"observe_only\":"),
        ("validation_success", "\"validation_success\":"),
        ("report_only", "\"report_only\":"),
        ("starts_process", "\"starts_process\":"),
        ("sends_prompt", "\"sends_prompt\":"),
        ("writes_memory", "\"writes_memory\":"),
        ("cleanup_required", "\"cleanup_required\":"),
        ("ready", "\"ready\":"),
        ("report_digest", "\"report_digest\":"),
        ("read_only", "\"read_only\":"),
        ("write_allowed", "\"write_allowed\":"),
        ("applied", "\"applied\":"),
    ] {
        if !line.contains(marker) {
            failures.push(format!("missing external_agent_lifecycle field {name}"));
        }
    }

    if extract_json_string_field(line, "schema").as_deref()
        != Some(EXTERNAL_AGENT_LIFECYCLE_TRACE_SCHEMA)
    {
        failures.push("external_agent_lifecycle schema is not supported".to_owned());
    }
    if extract_json_string_field(line, "report_kind").as_deref() != Some("lifecycle_gate") {
        failures.push("external_agent_lifecycle report_kind must be lifecycle_gate".to_owned());
    }

    let agents = extract_json_usize_field(line, "agents").unwrap_or(0);
    let target_project_scope = extract_json_string_field(line, "target_project_scope");
    let evidence_ready = extract_json_usize_field(line, "evidence_ready").unwrap_or(0);
    let project_scoped = extract_json_usize_field(line, "project_scoped").unwrap_or(0);
    let project_scope_mismatch =
        extract_json_usize_field(line, "project_scope_mismatch").unwrap_or(0);
    let foreign_project = extract_json_usize_field(line, "foreign_project").unwrap_or(0);
    let missing_evidence = extract_json_usize_field(line, "missing_evidence").unwrap_or(0);
    let stale_evidence = extract_json_usize_field(line, "stale_evidence").unwrap_or(0);
    let working = extract_json_usize_field(line, "working").unwrap_or(0);
    let blocked = extract_json_usize_field(line, "blocked").unwrap_or(0);
    let done = extract_json_usize_field(line, "done").unwrap_or(0);
    let idle = extract_json_usize_field(line, "idle").unwrap_or(0);
    let unknown = extract_json_usize_field(line, "unknown").unwrap_or(0);
    let hold_dependent_task = extract_json_usize_field(line, "hold_dependent_task").unwrap_or(0);
    let require_operator_attention =
        extract_json_usize_field(line, "require_operator_attention").unwrap_or(0);
    let validation_success = extract_json_usize_field(line, "validation_success").unwrap_or(0);
    let report_only = extract_json_usize_field(line, "report_only").unwrap_or(0);
    let starts_process = extract_json_usize_field(line, "starts_process").unwrap_or(0);
    let sends_prompt = extract_json_usize_field(line, "sends_prompt").unwrap_or(0);
    let writes_memory = extract_json_usize_field(line, "writes_memory").unwrap_or(0);
    let cleanup_required = extract_json_usize_field(line, "cleanup_required").unwrap_or(0);

    if agents == 0 {
        failures.push("external_agent_lifecycle agents must be positive".to_owned());
    }
    if evidence_ready != agents || missing_evidence > 0 || stale_evidence > 0 {
        failures
            .push("external_agent_lifecycle requires fresh evidence for every agent".to_owned());
    }
    if target_project_scope.as_deref() != Some(EXTERNAL_AGENT_TARGET_PROJECT_SCOPE)
        || project_scoped != agents
        || project_scope_mismatch > 0
        || foreign_project > 0
    {
        failures
            .push("external_agent_lifecycle requires project-scoped external agents".to_owned());
    }
    if working + blocked + unknown > 0 {
        failures.push("external_agent_lifecycle has active or unknown agents".to_owned());
    }
    if done + idle != agents {
        failures.push("external_agent_lifecycle done/idle counts must match agents".to_owned());
    }
    if hold_dependent_task + require_operator_attention + cleanup_required > 0 {
        failures.push("external_agent_lifecycle cleanup is still required".to_owned());
    }
    if validation_success > 0 {
        failures.push(
            "external_agent_lifecycle must not treat status as validation success".to_owned(),
        );
    }
    if report_only != agents || starts_process + sends_prompt + writes_memory > 0 {
        failures.push("external_agent_lifecycle must remain report-only".to_owned());
    }
    if !extract_json_bool_field(line, "ready").unwrap_or(false) {
        failures.push("external_agent_lifecycle ready must be true".to_owned());
    }
    if !extract_json_bool_field(line, "read_only").unwrap_or(false)
        || extract_json_bool_field(line, "write_allowed").unwrap_or(true)
        || extract_json_bool_field(line, "applied").unwrap_or(true)
    {
        failures.push("external_agent_lifecycle must be read-only and unapplied".to_owned());
    }
    if !extract_json_string_field(line, "report_digest")
        .unwrap_or_default()
        .starts_with("redaction-digest:")
    {
        failures
            .push("external_agent_lifecycle report_digest must be a redaction digest".to_owned());
    }

    failures
}
