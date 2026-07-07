use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::json::{
    json_bool_field, json_f64_field, json_object_field, json_string, json_string_array,
    json_string_field, json_u64_field, parse_json_string_array,
};
use crate::routing_rules::{
    QueryFeatures, RouteDecision, capability_snapshot_json, query_features_json,
    route_decision_json,
};
use norion_core::ModelRouteProfile;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RequestOutcome {
    pub(crate) trace_id: String,
    pub(crate) request_id: String,
    pub(crate) task_kind: String,
    pub(crate) skill_tags: Vec<String>,
    pub(crate) query_features: QueryFeatures,
    pub(crate) route_decision: RouteDecision,
    pub(crate) ok: bool,
    pub(crate) error_kind: Option<String>,
    pub(crate) latency_ms: u64,
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) cost_estimate_micro_usd: u64,
    pub(crate) quality_score: Option<f64>,
    pub(crate) reward_placeholder: String,
    pub(crate) reflection_placeholder: String,
    pub(crate) backend_id: Option<String>,
    pub(crate) capability_snapshot: Option<ModelRouteProfile>,
    pub(crate) timestamp_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutcomeSummary {
    pub(crate) total: usize,
    pub(crate) ok: usize,
    pub(crate) failed: usize,
    pub(crate) latest_trace_id: Option<String>,
    pub(crate) latest_request_id: Option<String>,
}

pub(crate) fn append_outcome(path: &Path, outcome: &RequestOutcome) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "create outcome log directory {} failed: {error}",
                parent.display()
            )
        })?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("open outcome log {} failed: {error}", path.display()))?;
    writeln!(file, "{}", outcome_json(outcome))
        .map_err(|error| format!("write outcome log {} failed: {error}", path.display()))
}

pub(crate) fn read_recent_outcomes(
    path: &Path,
    limit: usize,
) -> Result<Vec<RequestOutcome>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "read outcome log {} failed: {error}",
                path.display()
            ));
        }
    };
    let lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .rev()
        .take(limit)
        .collect::<Vec<_>>();
    let mut outcomes = Vec::with_capacity(lines.len());
    for line in lines.into_iter().rev() {
        outcomes.push(parse_outcome(line)?);
    }
    Ok(outcomes)
}

pub(crate) fn dump_recent_summary(path: &Path, limit: usize) -> Result<OutcomeSummary, String> {
    let outcomes = read_recent_outcomes(path, limit)?;
    Ok(summarize_outcomes(&outcomes))
}

pub(crate) fn summarize_outcomes(outcomes: &[RequestOutcome]) -> OutcomeSummary {
    OutcomeSummary {
        total: outcomes.len(),
        ok: outcomes.iter().filter(|outcome| outcome.ok).count(),
        failed: outcomes.iter().filter(|outcome| !outcome.ok).count(),
        latest_trace_id: outcomes.last().map(|outcome| outcome.trace_id.clone()),
        latest_request_id: outcomes.last().map(|outcome| outcome.request_id.clone()),
    }
}

pub(crate) fn outcome_json(outcome: &RequestOutcome) -> String {
    format!(
        "{{\"schema\":\"norion.request_outcome.v1\",\"trace_id\":{},\"request_id\":{},\"task_kind\":{},\"skill_tags\":{},\"query_features\":{},\"strategy\":{},\"chosen_model\":{},\"candidate_count\":{},\"excluded_models\":{},\"route_reason\":{},\"route_decision\":{},\"ok\":{},\"error_kind\":{},\"latency_ms\":{},\"input_tokens\":{},\"output_tokens\":{},\"cost_estimate_micro_usd\":{},\"quality_score\":{},\"reward_placeholder\":{},\"reflection_placeholder\":{},\"backend_id\":{},\"capability_snapshot\":{},\"timestamp_unix\":{},\"trace_mapping\":{}}}",
        json_string(&outcome.trace_id),
        json_string(&outcome.request_id),
        json_string(&outcome.task_kind),
        json_string_array(&outcome.skill_tags),
        query_features_json(&outcome.query_features),
        json_string(&outcome.route_decision.strategy),
        option_json_string(outcome.route_decision.chosen_model.as_deref()),
        outcome.route_decision.candidate_count,
        excluded_models_json_from_decision(&outcome.route_decision),
        json_string(&outcome.route_decision.reason),
        route_decision_json(&outcome.route_decision),
        outcome.ok,
        option_json_string(outcome.error_kind.as_deref()),
        outcome.latency_ms,
        outcome.input_tokens,
        outcome.output_tokens,
        outcome.cost_estimate_micro_usd,
        option_f64_json(outcome.quality_score),
        json_string(&outcome.reward_placeholder),
        json_string(&outcome.reflection_placeholder),
        option_json_string(outcome.backend_id.as_deref()),
        capability_snapshot_json(outcome.capability_snapshot.as_ref()),
        outcome.timestamp_unix,
        trace_mapping_json()
    )
}

fn parse_outcome(line: &str) -> Result<RequestOutcome, String> {
    let query_features = json_object_field(line, "query_features")
        .ok_or_else(|| "outcome missing query_features".to_owned())
        .map(|object| QueryFeatures {
            context_tokens: json_u64_field(&object, "context_tokens").unwrap_or_default(),
            estimated_input_tokens: json_u64_field(&object, "estimated_input_tokens")
                .unwrap_or_default(),
            estimated_output_tokens: json_u64_field(&object, "estimated_output_tokens")
                .unwrap_or_default(),
            max_budget_micro_usd: json_u64_field(&object, "max_budget_micro_usd")
                .unwrap_or_default(),
            required_capabilities: json_object_field(&object, "unused")
                .map(|_| Vec::new())
                .unwrap_or_else(|| {
                    crate::json::json_array_field(&object, "required_capabilities")
                        .map(|array| parse_json_string_array(&array))
                        .unwrap_or_default()
                }),
        })?;
    let route_decision = json_object_field(line, "route_decision")
        .ok_or_else(|| "outcome missing route_decision".to_owned())
        .map(|object| RouteDecision {
            strategy: json_string_field(&object, "strategy").unwrap_or_else(|| "single".to_owned()),
            chosen_model: json_string_field(&object, "chosen_model"),
            backend_id: json_string_field(&object, "backend_id"),
            candidate_count: json_u64_field(&object, "candidate_count").unwrap_or_default()
                as usize,
            candidates: Vec::new(),
            excluded_models: Vec::new(),
            reason: json_string_field(&object, "reason").unwrap_or_default(),
        })?;

    Ok(RequestOutcome {
        trace_id: required_string(line, "trace_id")?,
        request_id: required_string(line, "request_id")?,
        task_kind: required_string(line, "task_kind")?,
        skill_tags: crate::json::json_array_field(line, "skill_tags")
            .map(|array| parse_json_string_array(&array))
            .unwrap_or_default(),
        query_features,
        route_decision,
        ok: json_bool_field(line, "ok").unwrap_or(false),
        error_kind: json_string_field(line, "error_kind"),
        latency_ms: json_u64_field(line, "latency_ms").unwrap_or_default(),
        input_tokens: json_u64_field(line, "input_tokens").unwrap_or_default(),
        output_tokens: json_u64_field(line, "output_tokens").unwrap_or_default(),
        cost_estimate_micro_usd: json_u64_field(line, "cost_estimate_micro_usd")
            .unwrap_or_default(),
        quality_score: json_f64_field(line, "quality_score"),
        reward_placeholder: json_string_field(line, "reward_placeholder")
            .unwrap_or_else(|| "process_reward:pending".to_owned()),
        reflection_placeholder: json_string_field(line, "reflection_placeholder")
            .unwrap_or_else(|| "reflection:pending".to_owned()),
        backend_id: json_string_field(line, "backend_id"),
        capability_snapshot: None,
        timestamp_unix: json_u64_field(line, "timestamp_unix").unwrap_or_default(),
    })
}

fn required_string(line: &str, field: &str) -> Result<String, String> {
    json_string_field(line, field).ok_or_else(|| format!("outcome missing {field}"))
}

fn excluded_models_json_from_decision(decision: &RouteDecision) -> String {
    let items = decision
        .excluded_models
        .iter()
        .map(|excluded| {
            format!(
                "{{\"model_id\":{},\"backend_id\":{},\"reasons\":{}}}",
                json_string(&excluded.model_id),
                json_string(&excluded.backend_id),
                json_string_array(&excluded.reasons)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn trace_mapping_json() -> String {
    "{\"trace\":\"trace_id/request_id/timestamp_unix/backend_id\",\"experience_replay\":\"task_kind/skill_tags/query_features/ok/error_kind/latency_ms/tokens/cost\",\"process_reward\":\"quality_score/reward_placeholder\",\"reflection\":\"reflection_placeholder/route_reason/excluded_models\"}".to_owned()
}

fn option_json_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn option_f64_json(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing_rules::{ExcludedModel, RouteCandidate};

    #[test]
    fn appends_reads_and_summarizes_successful_outcome_jsonl() {
        let path = temp_path("success");
        let _ = fs::remove_file(&path);
        let outcome = outcome(true, None);

        append_outcome(&path, &outcome).unwrap();
        let recent = read_recent_outcomes(&path, 10).unwrap();
        let summary = dump_recent_summary(&path, 10).unwrap();

        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].trace_id, "trace-1");
        assert_eq!(
            recent[0].route_decision.chosen_model.as_deref(),
            Some("fast")
        );
        assert_eq!(summary.total, 1);
        assert_eq!(summary.ok, 1);
        assert_eq!(summary.failed, 0);
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"schema\":\"norion.request_outcome.v1\""));
        assert!(raw.contains("\"trace_mapping\""));
        assert!(raw.contains("\"process_reward\":\"quality_score/reward_placeholder\""));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn records_failed_outcome_with_error_kind_and_route_reason() {
        let outcome = outcome(false, Some("backend_timeout"));
        let json = outcome_json(&outcome);

        assert!(json.contains("\"ok\":false"));
        assert!(json.contains("\"error_kind\":\"backend_timeout\""));
        assert!(json.contains("\"route_reason\":\"route:single:skill_tag:summary\""));
        assert!(json.contains("\"latency_ms\":42"));
        assert!(json.contains("\"input_tokens\":10"));
        assert!(json.contains("\"output_tokens\":20"));
        assert!(json.contains("\"cost_estimate_micro_usd\":7"));
        assert!(json.contains("\"quality_score\":null"));
    }

    #[test]
    fn outcome_json_records_excluded_models_for_budget_health_and_policy() {
        let mut outcome = outcome(true, None);
        outcome.route_decision.excluded_models = vec![
            excluded("slow", "health:unhealthy"),
            excluded("expensive", "budget:estimate:900>500"),
            excluded("denied", "policy:deny_secret"),
        ];
        let json = outcome_json(&outcome);

        assert!(json.contains("\"excluded_models\""));
        assert!(json.contains("health:unhealthy"));
        assert!(json.contains("budget:estimate:900>500"));
        assert!(json.contains("policy:deny_secret"));
    }

    fn outcome(ok: bool, error_kind: Option<&str>) -> RequestOutcome {
        RequestOutcome {
            trace_id: "trace-1".to_owned(),
            request_id: "request-1".to_owned(),
            task_kind: "summary".to_owned(),
            skill_tags: vec!["summary".to_owned()],
            query_features: QueryFeatures {
                context_tokens: 100,
                estimated_input_tokens: 10,
                estimated_output_tokens: 20,
                max_budget_micro_usd: 1000,
                required_capabilities: vec!["text".to_owned()],
            },
            route_decision: RouteDecision {
                strategy: "single".to_owned(),
                chosen_model: Some("fast".to_owned()),
                backend_id: Some("local".to_owned()),
                candidate_count: 1,
                candidates: vec![RouteCandidate {
                    model_id: "fast".to_owned(),
                    backend_id: "local".to_owned(),
                    role: "summary".to_owned(),
                    reasons: vec!["skill_tag:summary".to_owned()],
                }],
                excluded_models: Vec::new(),
                reason: "route:single:skill_tag:summary".to_owned(),
            },
            ok,
            error_kind: error_kind.map(str::to_owned),
            latency_ms: 42,
            input_tokens: 10,
            output_tokens: 20,
            cost_estimate_micro_usd: 7,
            quality_score: None,
            reward_placeholder: "process_reward:pending".to_owned(),
            reflection_placeholder: "reflection:pending".to_owned(),
            backend_id: Some("local".to_owned()),
            capability_snapshot: Some(ModelRouteProfile {
                source_index: 0,
                role: "summary".to_owned(),
                model_profile_id: "fast".to_owned(),
                inference_backend_id: "local".to_owned(),
                model_pool_id: "evolution-loop".to_owned(),
                capabilities: vec!["text".to_owned()],
                max_context_tokens: 1000,
                blocked_reasons: Vec::new(),
                input_cost_per_1k_micro_usd: 1,
                output_cost_per_1k_micro_usd: 1,
                remaining_budget_micro_usd: 1000,
            }),
            timestamp_unix: 123,
        }
    }

    fn excluded(model_id: &str, reason: &str) -> ExcludedModel {
        ExcludedModel {
            model_id: model_id.to_owned(),
            backend_id: "backend".to_owned(),
            reasons: vec![reason.to_owned()],
        }
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "norion-outcome-log-{name}-{}.jsonl",
            std::process::id()
        ))
    }
}
