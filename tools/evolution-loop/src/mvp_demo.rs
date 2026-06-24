use std::fs;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::args::Config;
use crate::json::json_string;
use crate::model_registry as registry;
use crate::outcome_log::{self, OutcomeSummary, RequestOutcome};
use crate::profile_scoring as scoring;
use crate::routing_rules as rules;

const DEMO_SCHEMA: &str = "norion.multi_model_mvp_demo.v1";
static DEMO_OUTCOME_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq)]
struct MvpDemoResult {
    registry_profiles: usize,
    enabled_profiles: usize,
    disabled_profiles: usize,
    rule_selected: String,
    profile_selected: String,
    rule_backend_id: String,
    profile_backend_id: String,
    outcome_summary: OutcomeSummary,
    offline_regression: scoring::OfflineRegressionReport,
    switch_decision: scoring::PolicySwitchDecision,
    profile_explanation_json: String,
}

pub(crate) fn run(config: &Config) -> Result<(), String> {
    let result = build_demo()?;
    println!("{}", render_text_report(&result));
    if let Some(path) = config.report_json_path.as_deref() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "create mvp demo report directory {} failed: {error}",
                    parent.display()
                )
            })?;
        }
        fs::write(path, render_json_report(&result))
            .map_err(|error| format!("write mvp demo report {} failed: {error}", path.display()))?;
    }
    Ok(())
}

fn build_demo() -> Result<MvpDemoResult, String> {
    let registry = registry::ModelRegistry::from_configs(
        registry::parse_typed_config(DEMO_MODEL_CONFIG)?,
        Some("qwen-local-fast,qwen-quality-remote,gpt-5.4"),
    )?;
    let profiles = registry.list();
    let enabled_profiles = registry.list_enabled();
    let routing_profiles = profiles.iter().map(to_routing_profile).collect::<Vec<_>>();
    let routing_registry = rules::ModelRegistry::new(routing_profiles);
    let request = demo_route_request();
    let rule_decision = rules::RuleRouter.route(&routing_registry, &request);
    let rule_selected = rule_decision
        .chosen_model
        .clone()
        .ok_or_else(|| "mvp demo rule router did not select a model".to_owned())?;
    let rule_profile = routing_registry
        .profiles()
        .iter()
        .find(|profile| profile.model_id == rule_selected)
        .cloned()
        .ok_or_else(|| format!("mvp demo missing rule profile: {rule_selected}"))?;
    let quality_profile = routing_registry
        .profiles()
        .iter()
        .find(|profile| profile.model_id == "qwen-quality-remote")
        .cloned()
        .ok_or_else(|| "mvp demo missing qwen-quality-remote profile".to_owned())?;

    let rule_outcome = demo_outcome(
        "mvp-demo-rule",
        &request,
        &rule_decision,
        &rule_profile,
        1800,
        2000,
        0.20,
    );
    let quality_decision = profile_seed_decision(&rule_decision, &quality_profile);
    let quality_outcome = demo_outcome(
        "mvp-demo-profile-seed",
        &request,
        &quality_decision,
        &quality_profile,
        220,
        30,
        0.98,
    );
    let outcome_summary =
        summarize_demo_outcomes(&[rule_outcome.clone(), quality_outcome.clone()])?;

    let mut scorer = scoring::OnlineScorer::new(scoring::ScoringConfig::default());
    for outcome in [&rule_outcome, &quality_outcome] {
        let body = outcome_log::outcome_json(outcome);
        let sample = scoring::OutcomeSample::from_m3_json(&body, "rust-coding")
            .ok_or_else(|| "mvp demo could not parse M3 outcome into M4 sample".to_owned())?;
        scorer.update(sample);
    }
    let candidates = enabled_profiles
        .iter()
        .map(|profile| scoring::CandidateModel::new(profile.id.clone()))
        .collect::<Vec<_>>();
    let profile_decision = scorer
        .route(&candidates, "rust-coding", Some(1.0))
        .ok_or_else(|| "mvp demo profile router did not select a model".to_owned())?;
    let profile_selected = profile_decision.selected_model_id.clone();
    let profile_backend_id = enabled_profiles
        .iter()
        .find(|profile| profile.id == profile_selected)
        .map(|profile| profile.backend_ref.backend_id.clone())
        .unwrap_or_default();
    let profile_explanation_json = scorer.explanation_json(&profile_decision);
    let offline_regression = scoring::OfflineRegressionReport::compare(
        "profile-scoring.v1",
        &scoring::RegressionAggregate {
            quality: 0.20,
            latency_ms: 1800.0,
            cost: 2000.0,
        },
        &scoring::RegressionAggregate {
            quality: 0.98,
            latency_ms: 220.0,
            cost: 30.0,
        },
    );
    let switch_decision = offline_regression.switch_decision(&scoring::ScoringConfig::default());

    Ok(MvpDemoResult {
        registry_profiles: profiles.len(),
        enabled_profiles: enabled_profiles.len(),
        disabled_profiles: profiles.len().saturating_sub(enabled_profiles.len()),
        rule_selected,
        profile_selected,
        rule_backend_id: rule_profile.backend_id,
        profile_backend_id,
        outcome_summary,
        offline_regression,
        switch_decision,
        profile_explanation_json,
    })
}

fn summarize_demo_outcomes(outcomes: &[RequestOutcome]) -> Result<OutcomeSummary, String> {
    let path = demo_outcome_path();
    let _ = fs::remove_file(&path);
    for outcome in outcomes {
        outcome_log::append_outcome(&path, outcome)?;
    }
    let summary = outcome_log::dump_recent_summary(&path, outcomes.len())?;
    let _ = fs::remove_file(path);
    Ok(summary)
}

fn demo_outcome_path() -> PathBuf {
    let nonce = DEMO_OUTCOME_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "rust-norion-mvp-demo-{}-{}-outcomes.jsonl",
        process::id(),
        nonce
    ))
}

fn demo_route_request() -> rules::RouteRequest {
    rules::RouteRequest {
        task_kind: "rust-coding".to_owned(),
        skill_tags: vec!["rust".to_owned(), "coding".to_owned()],
        query_features: rules::QueryFeatures {
            context_tokens: 4096,
            estimated_input_tokens: 1000,
            estimated_output_tokens: 600,
            max_budget_micro_usd: 100_000,
            required_capabilities: vec!["streaming".to_owned(), "cancel".to_owned()],
        },
    }
}

fn demo_outcome(
    request_id: &str,
    request: &rules::RouteRequest,
    route_decision: &rules::RouteDecision,
    profile: &rules::ModelProfile,
    latency_ms: u64,
    cost_estimate_micro_usd: u64,
    quality_score: f64,
) -> RequestOutcome {
    RequestOutcome {
        trace_id: format!("trace-{request_id}"),
        request_id: request_id.to_owned(),
        task_kind: request.task_kind.clone(),
        skill_tags: request.skill_tags.clone(),
        query_features: request.query_features.clone(),
        route_decision: route_decision.clone(),
        ok: true,
        error_kind: None,
        latency_ms,
        input_tokens: request.query_features.estimated_input_tokens,
        output_tokens: request.query_features.estimated_output_tokens,
        cost_estimate_micro_usd,
        quality_score: Some(quality_score),
        reward_placeholder: "mvp_demo_reward_placeholder".to_owned(),
        reflection_placeholder: "mvp_demo_reflection_placeholder".to_owned(),
        backend_id: Some(profile.backend_id.clone()),
        capability_snapshot: Some(profile.clone()),
        timestamp_unix: 1_782_290_000,
    }
}

fn profile_seed_decision(
    rule_decision: &rules::RouteDecision,
    profile: &rules::ModelProfile,
) -> rules::RouteDecision {
    let mut decision = rule_decision.clone();
    decision.strategy = "profile-seed".to_owned();
    decision.chosen_model = Some(profile.model_id.clone());
    decision.backend_id = Some(profile.backend_id.clone());
    decision.reason = format!("profile_seed:selected={}", profile.model_id);
    decision
}

fn to_routing_profile(profile: &registry::ModelProfile) -> rules::ModelProfile {
    let (input_cost, output_cost) = static_cost(profile.cost_tier);
    rules::ModelProfile {
        model_id: profile.id.clone(),
        backend_id: profile.backend_ref.backend_id.clone(),
        skill_tags: profile.skill_tags.clone(),
        capabilities: capability_names(&profile.capabilities),
        max_context_tokens: profile.ctx_window,
        healthy: profile.is_enabled(),
        deny_policy_reasons: profile.policy.deny_reason.clone().into_iter().collect(),
        input_cost_per_1k_micro_usd: input_cost,
        output_cost_per_1k_micro_usd: output_cost,
        remaining_budget_micro_usd: 100_000,
    }
}

fn capability_names(capabilities: &registry::ModelCapabilities) -> Vec<String> {
    let mut names = Vec::new();
    if capabilities.supports_streaming {
        names.push("streaming".to_owned());
    }
    if capabilities.supports_cancel {
        names.push("cancel".to_owned());
    }
    if capabilities.supports_kv_export {
        names.push("kv-export".to_owned());
    }
    if capabilities.supports_local {
        names.push("local".to_owned());
    }
    if capabilities.supports_openai_compat {
        names.push("openai-compatible".to_owned());
    }
    names
}

fn static_cost(tier: registry::CostTier) -> (u64, u64) {
    match tier {
        registry::CostTier::Free => (0, 0),
        registry::CostTier::Low => (4, 8),
        registry::CostTier::Medium => (80, 120),
        registry::CostTier::High => (500, 900),
    }
}

fn render_text_report(result: &MvpDemoResult) -> String {
    let mut lines = Vec::new();
    lines.push(format!("schema={DEMO_SCHEMA}"));
    lines.push(format!(
        "registry profiles={} enabled={} disabled={}",
        result.registry_profiles, result.enabled_profiles, result.disabled_profiles
    ));
    lines.push("rule_vs_profile".to_owned());
    lines.push(
        "strategy\tselected_model\tbackend\tquality\tlatency_ms\tcost_micro_usd\tgate".to_owned(),
    );
    lines.push(format!(
        "rule\t{}\t{}\t{:.2}\t{}\t{}\t{}",
        result.rule_selected, result.rule_backend_id, 0.20, 1800, 2000, "baseline"
    ));
    lines.push(format!(
        "profile\t{}\t{}\t{:.2}\t{}\t{}\t{}",
        result.profile_selected,
        result.profile_backend_id,
        0.98,
        220,
        30,
        if result.switch_decision.allow_switch {
            "offline-regression-pass"
        } else {
            "offline-regression-blocked"
        }
    ));
    lines.push(format!(
        "outcome_summary total={} ok={} failed={} latest_trace_id={}",
        result.outcome_summary.total,
        result.outcome_summary.ok,
        result.outcome_summary.failed,
        result
            .outcome_summary
            .latest_trace_id
            .as_deref()
            .unwrap_or("none")
    ));
    lines.push(format!(
        "offline_regression passed={} quality_delta={:.3} latency_delta_ms={:.1} cost_delta={:.1}",
        result.offline_regression.passed,
        result.offline_regression.quality_delta,
        result.offline_regression.latency_delta_ms,
        result.offline_regression.cost_delta
    ));
    lines.push(format!(
        "policy_switch allow={} reason={}",
        result.switch_decision.allow_switch, result.switch_decision.reason
    ));
    lines.join("\n")
}

fn render_json_report(result: &MvpDemoResult) -> String {
    format!(
        "{{\"schema\":{},\"registry_profiles\":{},\"enabled_profiles\":{},\"disabled_profiles\":{},\"rule_selected\":{},\"profile_selected\":{},\"rule_backend_id\":{},\"profile_backend_id\":{},\"outcome_summary\":{},\"offline_regression\":{},\"policy_switch\":{},\"profile_explanation\":{}}}",
        json_string(DEMO_SCHEMA),
        result.registry_profiles,
        result.enabled_profiles,
        result.disabled_profiles,
        json_string(&result.rule_selected),
        json_string(&result.profile_selected),
        json_string(&result.rule_backend_id),
        json_string(&result.profile_backend_id),
        outcome_summary_json(&result.outcome_summary),
        result.offline_regression.json_report(),
        policy_switch_json(&result.switch_decision),
        result.profile_explanation_json
    )
}

fn outcome_summary_json(summary: &OutcomeSummary) -> String {
    format!(
        "{{\"total\":{},\"ok\":{},\"failed\":{},\"latest_trace_id\":{},\"latest_request_id\":{}}}",
        summary.total,
        summary.ok,
        summary.failed,
        option_json_string(summary.latest_trace_id.as_deref()),
        option_json_string(summary.latest_request_id.as_deref())
    )
}

fn policy_switch_json(decision: &scoring::PolicySwitchDecision) -> String {
    format!(
        "{{\"allow_switch\":{},\"policy_version\":{},\"reason\":{}}}",
        decision.allow_switch,
        json_string(&decision.policy_version),
        json_string(&decision.reason)
    )
}

fn option_json_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

const DEMO_MODEL_CONFIG: &str = r#"
id=qwen-local-fast;provider=local;display_name=Qwen Local Fast;skill_tags=rust,coding,zh;ctx_window=65536;default_max_tokens=4096;cost_tier=low;latency_tier=low;device_class=local-gpu;backend_id=deterministic-local;backend_ref=deterministic://local;supports_streaming=true;supports_cancel=true;supports_kv_export=true;supports_local=true;supports_openai_compat=false;allow_provider_alias=true
id=qwen-quality-remote;provider=newapi;display_name=Qwen Quality Remote;skill_tags=rust,coding,zh;ctx_window=262144;default_max_tokens=8192;cost_tier=high;latency_tier=medium;device_class=remote;backend_id=openai-compatible;backend_ref=https://example.invalid/v1;supports_streaming=true;supports_cancel=true;supports_kv_export=false;supports_local=false;supports_openai_compat=true;allow_provider_alias=true
id=gpt-5.4;provider=openai;display_name=GPT 5.4 Forbidden;skill_tags=rust,coding;ctx_window=262144;default_max_tokens=8192;cost_tier=high;latency_tier=high;device_class=remote;backend_id=blocked;backend_ref=gpt-5.4;supports_streaming=true;supports_cancel=true;supports_kv_export=false;supports_local=false;supports_openai_compat=true;allow_provider_alias=true
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mvp_demo_wires_registry_rule_outcome_and_profile_scoring() {
        let result = build_demo().expect("demo should build");

        assert_eq!(result.registry_profiles, 3);
        assert_eq!(result.enabled_profiles, 2);
        assert_eq!(result.disabled_profiles, 1);
        assert_eq!(result.rule_selected, "qwen-local-fast");
        assert_eq!(result.profile_selected, "qwen-quality-remote");
        assert_eq!(result.outcome_summary.total, 2);
        assert_eq!(result.outcome_summary.ok, 2);
        assert!(result.offline_regression.passed);
        assert!(result.switch_decision.allow_switch);
    }

    #[test]
    fn mvp_demo_reports_rule_vs_profile_table_and_json_schema() {
        let result = build_demo().expect("demo should build");
        let text = render_text_report(&result);
        let json = render_json_report(&result);

        assert!(text.contains("rule_vs_profile"));
        assert!(text.contains("qwen-local-fast"));
        assert!(text.contains("qwen-quality-remote"));
        assert!(json.contains("\"schema\":\"norion.multi_model_mvp_demo.v1\""));
        assert!(json.contains("\"profile_explanation\""));
        assert!(json.contains("\"allow_switch\":true"));
    }
}
