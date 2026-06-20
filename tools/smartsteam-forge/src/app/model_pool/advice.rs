use model_pool_advice_core::{
    CAPACITY_POLICY, HELPER_ROLES, HELPER_TARGET_WORKERS, MAX_QUALITY_12B_WORKERS, POLICY,
    RECOMMENDED_LAUNCH_ROLES, missing_helper_roles, model_pool_decision,
};

mod facts;
mod json;
#[cfg(test)]
mod tests;

use facts::facts_from_summary;
use json::model_pool_advice_json;

const UNKNOWN: &str = "unknown";

pub(crate) fn model_pool_advice(status_summary: &str) -> String {
    let facts = facts_from_summary(status_summary);
    let mut lines = vec![
        "SmartSteam Apple model pool advice".to_owned(),
        "read_only=true".to_owned(),
        "launches_process=false".to_owned(),
        "sends_prompt=false".to_owned(),
        format!("policy={POLICY}"),
        format!("capacity_policy={CAPACITY_POLICY}"),
        "avoid_extra_12b=true".to_owned(),
        format!("max_quality_12b_workers={MAX_QUALITY_12B_WORKERS}"),
        format!(
            "recommended_launch_order={}",
            RECOMMENDED_LAUNCH_ROLES.join(",")
        ),
        format!(
            "quality_ready={}",
            option_bool_text(facts.quality_ready).unwrap_or(UNKNOWN)
        ),
        format!(
            "quality_context_sufficient={}",
            option_bool_text(facts.quality_context_sufficient).unwrap_or(UNKNOWN)
        ),
        format!(
            "quality_context_tokens={}",
            facts.quality_context_tokens.as_deref().unwrap_or(UNKNOWN)
        ),
        format!(
            "quality_required_context_tokens={}",
            facts
                .quality_required_context_tokens
                .as_deref()
                .unwrap_or(UNKNOWN)
        ),
        format!(
            "quality_runtime_accelerated={}",
            option_bool_text(facts.quality_runtime_accelerated).unwrap_or(UNKNOWN)
        ),
    ];

    if let Some(recommendation) = facts.capacity_recommendation.as_deref() {
        lines.push(format!("capacity_recommendation={recommendation}"));
    }
    if let Some(expansion_allowed) = facts.expansion_allowed {
        lines.push(format!(
            "capacity_expansion_allowed={}",
            bool_text(expansion_allowed)
        ));
    }
    if let Some(healthy_helper_workers) = facts.healthy_helper_worker_count {
        lines.push(format!(
            "healthy_helper_worker_count={healthy_helper_workers}"
        ));
    }
    if let Some(unknown_runtime_workers) = facts.unknown_runtime_worker_count {
        lines.push(format!(
            "unknown_runtime_worker_count={unknown_runtime_workers}"
        ));
    }
    lines.push(format!(
        "helper_roles=summary:{} router:{} review:{} test-gate:{} index:{}",
        bool_text(facts.has_summary),
        bool_text(facts.has_router),
        bool_text(facts.has_review),
        bool_text(facts.has_test_gate),
        bool_text(facts.has_index)
    ));
    lines.push(format!("expected_helper_roles={}", HELPER_ROLES.join(",")));
    let missing_roles = missing_helper_roles(&facts);
    lines.push(format!(
        "missing_helper_roles={}",
        role_list_text(&missing_roles)
    ));
    lines.push(format!(
        "helper_cpu_or_no_gpu_roles={}",
        string_list_text(&facts.helper_cpu_or_no_gpu_roles)
    ));
    lines.push(format!(
        "parallel_worker_shape=quality:{} helpers_visible:{} helper_target:{}",
        facts.quality_worker_count, facts.helper_worker_count, HELPER_TARGET_WORKERS
    ));
    lines.push(format!(
        "extra_quality_12b_detected={}",
        bool_text(facts.extra_quality_12b_detected())
    ));

    let decision = model_pool_decision(&facts);
    lines.push(format!(
        "safe_to_enable_pool_workers={}",
        bool_text(decision.safe_to_enable_pool_workers)
    ));
    lines.push(format!("next_step={}", decision.next_step));
    lines.push(format!("reason={}", decision.reason));
    lines.push("operator_checks=Activity Monitor GPU History and Memory Pressure must stay healthy before adding workers".to_owned());
    lines.push("section=advice_json".to_owned());
    lines.push(model_pool_advice_json(&facts, &decision));
    lines.join("\n")
}

pub(in crate::app) fn validate_model_pool_advice_report(report: &str) -> Result<(), String> {
    let lines = report.lines().collect::<Vec<_>>();
    require_advice_line(&lines, 0, "SmartSteam Apple model pool advice")?;
    require_advice_line(&lines, 1, "read_only=true")?;
    require_advice_line(&lines, 2, "launches_process=false")?;
    require_advice_line(&lines, 3, "sends_prompt=false")?;
    let advice_json = require_advice_section_body(&lines, "section=advice_json")?;
    let summary = json::model_pool_advice_json_summary(advice_json)?;
    require_advice_text_line(
        &lines,
        &format!(
            "safe_to_enable_pool_workers={}",
            bool_text(summary.safe_to_enable_pool_workers)
        ),
    )?;
    require_advice_text_line(&lines, &format!("next_step={}", summary.next_step))?;
    require_advice_text_line(&lines, &format!("reason={}", summary.reason))?;
    require_advice_text_line(
        &lines,
        &format!(
            "missing_helper_roles={}",
            string_list_text(&summary.missing_helper_roles)
        ),
    )?;
    require_advice_text_line(
        &lines,
        &format!(
            "helper_cpu_or_no_gpu_roles={}",
            string_list_text(&summary.helper_cpu_or_no_gpu_roles)
        ),
    )
}

fn require_advice_line(lines: &[&str], index: usize, expected: &str) -> Result<(), String> {
    match lines.get(index) {
        Some(line) if *line == expected => Ok(()),
        Some(line) => Err(format!(
            "model pool advice line {index} expected {expected:?}, got {line:?}"
        )),
        None => Err(format!(
            "model pool advice missing line {index} expected {expected:?}"
        )),
    }
}

fn require_advice_section_body<'a>(lines: &'a [&str], section: &str) -> Result<&'a str, String> {
    let Some(index) = lines.iter().position(|line| *line == section) else {
        return Err(format!("model pool advice missing {section}"));
    };
    let Some(body) = lines.get(index + 1) else {
        return Err(format!("model pool advice missing body for {section}"));
    };
    if body.starts_with("section=") {
        return Err(format!("model pool advice missing body for {section}"));
    }
    Ok(body)
}

fn require_advice_text_line(lines: &[&str], expected: &str) -> Result<(), String> {
    lines
        .iter()
        .any(|line| *line == expected)
        .then_some(())
        .ok_or_else(|| format!("model pool advice text missing {expected:?}"))
}

fn option_bool_text(value: Option<bool>) -> Option<&'static str> {
    value.map(bool_text)
}

fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn role_list_text(roles: &[&str]) -> String {
    if roles.is_empty() {
        "none".to_owned()
    } else {
        roles.join(",")
    }
}

fn string_list_text(roles: &[String]) -> String {
    if roles.is_empty() {
        "none".to_owned()
    } else {
        roles.join(",")
    }
}
