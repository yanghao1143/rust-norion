use std::fs;

use crate::args::Config;
use crate::json::json_string;
use crate::model_registry;
use crate::outcome_log::{RequestOutcome, outcome_json};
use crate::profile_scoring::{
    CandidateModel, OfflineReplayReport, OnlineScorer, OutcomeSample, ScoringConfig,
};
use crate::routing_rules::{
    ModelProfile, ModelRegistry, QueryFeatures, RouteDecision, RouteRequest, RuleRouter,
};

pub(crate) fn run(config: &Config) -> Result<(), String> {
    let report = build_report()?;
    println!("{}", report.table());
    if let Some(path) = &config.report_json_path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("create {} failed: {error}", parent.display()))?;
        }
        fs::write(path, report.json())
            .map_err(|error| format!("write {} failed: {error}", path.display()))?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct MvpDemoReport {
    registry_profiles: usize,
    rule_model: String,
    profile_model: String,
    replay: OfflineReplayReport,
}

impl MvpDemoReport {
    fn table(&self) -> String {
        [
            "| policy | model | samples | quality | latency_ms | cost | allow_switch |",
            "| --- | --- | ---: | ---: | ---: | ---: | --- |",
            &format!(
                "| rule-routing | {} | {} | {:.3} | {:.0} | {:.0} | baseline |",
                self.rule_model,
                self.replay.baseline.samples,
                self.replay.baseline.quality_avg,
                self.replay.baseline.latency_avg_ms,
                self.replay.baseline.cost_avg
            ),
            &format!(
                "| profile-routing | {} | {} | {:.3} | {:.0} | {:.0} | {} |",
                self.profile_model,
                self.replay.candidate.samples,
                self.replay.candidate.quality_avg,
                self.replay.candidate.latency_avg_ms,
                self.replay.candidate.cost_avg,
                self.replay.allow_switch
            ),
        ]
        .join("\n")
    }

    fn json(&self) -> String {
        format!(
            "{{\"schema\":\"norion.mvp_demo.v1\",\"registry_profiles\":{},\"rule_model\":{},\"profile_model\":{},\"offline_replay\":{}}}\n",
            self.registry_profiles,
            json_string(&self.rule_model),
            json_string(&self.profile_model),
            self.replay.json_report()
        )
    }
}

fn build_report() -> Result<MvpDemoReport, String> {
    let m1_registry = model_registry::default_model_registry()?;
    let routing_registry = routing_registry();
    let request = RouteRequest {
        task_kind: "review".to_owned(),
        skill_tags: vec!["review".to_owned()],
        query_features: QueryFeatures {
            context_tokens: 1024,
            estimated_input_tokens: 500,
            estimated_output_tokens: 250,
            max_budget_micro_usd: 1_000,
            required_capabilities: vec!["text".to_owned()],
        },
    };
    let rule_decision = RuleRouter.route(&routing_registry, &request);
    let rule_model = rule_decision
        .chosen_model
        .clone()
        .ok_or_else(|| "mvp demo rule routing selected no model".to_owned())?;

    let mut scorer = OnlineScorer::new(ScoringConfig::default());
    for sample in profile_samples() {
        scorer.update(sample);
    }
    let profile_decision = scorer
        .route(
            &[
                CandidateModel::new(rule_model.clone()),
                CandidateModel::new("profile-quality"),
            ],
            "review",
            None,
        )
        .ok_or_else(|| "mvp demo profile routing selected no model".to_owned())?;

    let profile_model = profile_decision.selected_model_id;
    let jsonl = [
        outcome_json(&outcome(
            &rule_decision,
            "rule-routing",
            &rule_model,
            0,
            0.80,
            1000,
            100,
        )),
        outcome_json(&outcome(
            &rule_decision,
            "rule-routing",
            &rule_model,
            1,
            0.82,
            1100,
            120,
        )),
        outcome_json(&outcome(
            &profile_route_decision(&profile_model),
            "profile-routing",
            &profile_model,
            2,
            0.84,
            900,
            90,
        )),
        outcome_json(&outcome(
            &profile_route_decision(&profile_model),
            "profile-routing",
            &profile_model,
            3,
            0.85,
            920,
            95,
        )),
    ]
    .join("\n");
    let replay = OfflineReplayReport::from_outcome_jsonl(
        "mvp-demo://inline-outcomes",
        &jsonl,
        2,
        &ScoringConfig::default(),
    );

    Ok(MvpDemoReport {
        registry_profiles: m1_registry.list_enabled().len(),
        rule_model,
        profile_model,
        replay,
    })
}

fn routing_registry() -> ModelRegistry {
    ModelRegistry::new(vec![
        profile("rule-fast", "deterministic", 10, 10),
        profile("profile-quality", "newapi-pool", 60, 60),
    ])
}

fn profile(model_id: &str, backend_id: &str, input_cost: u64, output_cost: u64) -> ModelProfile {
    ModelProfile {
        model_id: model_id.to_owned(),
        backend_id: backend_id.to_owned(),
        skill_tags: vec!["review".to_owned()],
        capabilities: vec!["text".to_owned()],
        max_context_tokens: 32_768,
        healthy: true,
        deny_policy_reasons: Vec::new(),
        input_cost_per_1k_micro_usd: input_cost,
        output_cost_per_1k_micro_usd: output_cost,
        remaining_budget_micro_usd: 10_000,
    }
}

fn profile_samples() -> Vec<OutcomeSample> {
    vec![
        sample("rule-fast", 0.80, 1000.0, 100.0),
        sample("profile-quality", 0.86, 850.0, 80.0),
        sample("profile-quality", 0.88, 820.0, 75.0),
    ]
}

fn sample(model_id: &str, quality: f64, latency_ms: f64, cost: f64) -> OutcomeSample {
    OutcomeSample {
        model_id: model_id.to_owned(),
        skill_tag: "review".to_owned(),
        success: true,
        latency_ms: Some(latency_ms),
        cost: Some(cost),
        quality_hint: Some(quality),
        cache_hit: false,
        drift_detected: false,
    }
}

fn profile_route_decision(model: &str) -> RouteDecision {
    RouteDecision {
        strategy: "profile-routing".to_owned(),
        chosen_model: Some(model.to_owned()),
        backend_id: Some("newapi-pool".to_owned()),
        candidate_count: 2,
        candidates: Vec::new(),
        excluded_models: Vec::new(),
        reason: "profile_route_best selected=profile-quality".to_owned(),
    }
}

fn outcome(
    route_decision: &RouteDecision,
    strategy: &str,
    model: &str,
    index: u64,
    quality: f64,
    latency_ms: u64,
    cost: u64,
) -> RequestOutcome {
    let mut route_decision = route_decision.clone();
    route_decision.strategy = strategy.to_owned();
    route_decision.chosen_model = Some(model.to_owned());
    RequestOutcome {
        trace_id: format!("mvp-demo-trace-{index}"),
        request_id: format!("mvp-demo-request-{index}"),
        task_kind: "review".to_owned(),
        skill_tags: vec!["review".to_owned()],
        query_features: QueryFeatures {
            context_tokens: 1024,
            estimated_input_tokens: 500,
            estimated_output_tokens: 250,
            max_budget_micro_usd: 1_000,
            required_capabilities: vec!["text".to_owned()],
        },
        route_decision,
        ok: true,
        error_kind: None,
        latency_ms,
        input_tokens: 500,
        output_tokens: 250,
        cost_estimate_micro_usd: cost,
        quality_score: Some(quality),
        reward_placeholder: "process_reward:mvp_demo".to_owned(),
        reflection_placeholder: "reflection:mvp_demo".to_owned(),
        backend_id: Some("demo-backend".to_owned()),
        capability_snapshot: None,
        timestamp_unix: 1_700_000_000 + index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mvp_demo_outputs_rule_vs_profile_switch_evidence() {
        let report = build_report().unwrap();
        let table = report.table();
        let json = report.json();

        assert_eq!(report.registry_profiles, 2);
        assert_eq!(report.rule_model, "rule-fast");
        assert_eq!(report.profile_model, "profile-quality");
        assert!(report.replay.allow_switch);
        assert!(table.contains("rule-routing"));
        assert!(table.contains("profile-routing"));
        assert!(json.contains("\"schema\":\"norion.mvp_demo.v1\""));
        assert!(json.contains("\"allow_switch\":true"));
    }
}
