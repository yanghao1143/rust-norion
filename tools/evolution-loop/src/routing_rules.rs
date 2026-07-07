use std::collections::BTreeSet;

use crate::json::{json_string, json_string_array};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelProfile {
    pub(crate) model_id: String,
    pub(crate) backend_id: String,
    pub(crate) skill_tags: Vec<String>,
    pub(crate) capabilities: Vec<String>,
    pub(crate) max_context_tokens: u64,
    pub(crate) healthy: bool,
    pub(crate) deny_policy_reasons: Vec<String>,
    pub(crate) input_cost_per_1k_micro_usd: u64,
    pub(crate) output_cost_per_1k_micro_usd: u64,
    pub(crate) remaining_budget_micro_usd: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QueryFeatures {
    pub(crate) context_tokens: u64,
    pub(crate) estimated_input_tokens: u64,
    pub(crate) estimated_output_tokens: u64,
    pub(crate) max_budget_micro_usd: u64,
    pub(crate) required_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteRequest {
    pub(crate) task_kind: String,
    pub(crate) skill_tags: Vec<String>,
    pub(crate) query_features: QueryFeatures,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExcludedModel {
    pub(crate) model_id: String,
    pub(crate) backend_id: String,
    pub(crate) reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteCandidate {
    pub(crate) model_id: String,
    pub(crate) backend_id: String,
    pub(crate) reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteDecision {
    pub(crate) strategy: String,
    pub(crate) chosen_model: Option<String>,
    pub(crate) backend_id: Option<String>,
    pub(crate) candidate_count: usize,
    pub(crate) candidates: Vec<RouteCandidate>,
    pub(crate) excluded_models: Vec<ExcludedModel>,
    pub(crate) reason: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct RuleRouter;

impl RuleRouter {
    pub(crate) fn route(&self, profiles: &[ModelProfile], request: &RouteRequest) -> RouteDecision {
        let required = request
            .query_features
            .required_capabilities
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let requested_tags = request
            .skill_tags
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();

        let mut candidates = Vec::new();
        let mut excluded_models = Vec::new();

        for profile in profiles {
            let mut excluded_reasons = Vec::new();
            if !profile.healthy {
                excluded_reasons.push("health:unhealthy".to_owned());
            }
            if request.query_features.context_tokens > profile.max_context_tokens {
                excluded_reasons.push(format!(
                    "context:{}>{}",
                    request.query_features.context_tokens, profile.max_context_tokens
                ));
            }
            let missing_capabilities = missing_values(&required, &profile.capabilities);
            if !missing_capabilities.is_empty() {
                excluded_reasons.push(format!(
                    "capability:missing:{}",
                    missing_capabilities.join("|")
                ));
            }
            let estimated_cost = estimated_cost_micro_usd(profile, &request.query_features);
            if estimated_cost > request.query_features.max_budget_micro_usd {
                excluded_reasons.push(format!(
                    "budget:estimate:{}>{}",
                    estimated_cost, request.query_features.max_budget_micro_usd
                ));
            }
            if estimated_cost > profile.remaining_budget_micro_usd {
                excluded_reasons.push(format!(
                    "budget:remaining:{}>{}",
                    estimated_cost, profile.remaining_budget_micro_usd
                ));
            }
            excluded_reasons.extend(
                profile
                    .deny_policy_reasons
                    .iter()
                    .map(|reason| format!("policy:{reason}")),
            );

            if !excluded_reasons.is_empty() {
                excluded_models.push(ExcludedModel {
                    model_id: profile.model_id.clone(),
                    backend_id: profile.backend_id.clone(),
                    reasons: excluded_reasons,
                });
                continue;
            }

            let matched_tags = matching_values(&requested_tags, &profile.skill_tags);
            let mut reasons = Vec::new();
            if matched_tags.is_empty() {
                reasons.push("skill_tag:no_match".to_owned());
            } else {
                reasons.push(format!("skill_tag:{}", matched_tags.join("|")));
            }
            reasons.push(format!("cost_estimate_micro_usd:{estimated_cost}"));
            reasons.push(format!("context_capacity:{}", profile.max_context_tokens));
            candidates.push(RouteCandidate {
                model_id: profile.model_id.clone(),
                backend_id: profile.backend_id.clone(),
                reasons,
            });
        }

        candidates.sort_by(|left, right| {
            candidate_score(right, request, profiles)
                .cmp(&candidate_score(left, request, profiles))
                .then_with(|| left.model_id.cmp(&right.model_id))
        });

        let chosen = candidates.first();
        RouteDecision {
            strategy: "single".to_owned(),
            chosen_model: chosen.map(|candidate| candidate.model_id.clone()),
            backend_id: chosen.map(|candidate| candidate.backend_id.clone()),
            candidate_count: candidates.len(),
            candidates: candidates.clone(),
            excluded_models,
            reason: chosen
                .map(|candidate| format!("route:single:{}", candidate.reasons.join(";")))
                .unwrap_or_else(|| "route:none:no_eligible_model".to_owned()),
        }
    }
}

pub(crate) fn route_decision_json(decision: &RouteDecision) -> String {
    format!(
        "{{\"strategy\":{},\"chosen_model\":{},\"backend_id\":{},\"candidate_count\":{},\"candidates\":{},\"excluded_models\":{},\"reason\":{}}}",
        json_string(&decision.strategy),
        option_json_string(decision.chosen_model.as_deref()),
        option_json_string(decision.backend_id.as_deref()),
        decision.candidate_count,
        route_candidates_json(&decision.candidates),
        excluded_models_json(&decision.excluded_models),
        json_string(&decision.reason)
    )
}

pub(crate) fn query_features_json(features: &QueryFeatures) -> String {
    format!(
        "{{\"context_tokens\":{},\"estimated_input_tokens\":{},\"estimated_output_tokens\":{},\"max_budget_micro_usd\":{},\"required_capabilities\":{}}}",
        features.context_tokens,
        features.estimated_input_tokens,
        features.estimated_output_tokens,
        features.max_budget_micro_usd,
        json_string_array(&features.required_capabilities)
    )
}

pub(crate) fn capability_snapshot_json(profile: Option<&ModelProfile>) -> String {
    match profile {
        Some(profile) => format!(
            "{{\"model_id\":{},\"backend_id\":{},\"skill_tags\":{},\"capabilities\":{},\"max_context_tokens\":{},\"healthy\":{},\"input_cost_per_1k_micro_usd\":{},\"output_cost_per_1k_micro_usd\":{},\"remaining_budget_micro_usd\":{}}}",
            json_string(&profile.model_id),
            json_string(&profile.backend_id),
            json_string_array(&profile.skill_tags),
            json_string_array(&profile.capabilities),
            profile.max_context_tokens,
            profile.healthy,
            profile.input_cost_per_1k_micro_usd,
            profile.output_cost_per_1k_micro_usd,
            profile.remaining_budget_micro_usd
        ),
        None => "null".to_owned(),
    }
}

fn candidate_score(
    candidate: &RouteCandidate,
    request: &RouteRequest,
    profiles: &[ModelProfile],
) -> i128 {
    let Some(profile) = profiles
        .iter()
        .find(|profile| profile.model_id == candidate.model_id)
    else {
        return i128::MIN;
    };
    let requested_tags = request
        .skill_tags
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let matched_tags = matching_values(&requested_tags, &profile.skill_tags).len() as i128;
    let cost = estimated_cost_micro_usd(profile, &request.query_features) as i128;
    matched_tags * 1_000_000 - cost
}

fn estimated_cost_micro_usd(profile: &ModelProfile, features: &QueryFeatures) -> u64 {
    let input = features
        .estimated_input_tokens
        .saturating_mul(profile.input_cost_per_1k_micro_usd)
        / 1000;
    let output = features
        .estimated_output_tokens
        .saturating_mul(profile.output_cost_per_1k_micro_usd)
        / 1000;
    input.saturating_add(output)
}

fn missing_values(required: &BTreeSet<&str>, available: &[String]) -> Vec<String> {
    let available = available
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    required
        .iter()
        .filter(|value| !available.contains(**value))
        .map(|value| (*value).to_owned())
        .collect()
}

fn matching_values(requested: &BTreeSet<&str>, available: &[String]) -> Vec<String> {
    let available = available
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    requested
        .iter()
        .filter(|value| available.contains(**value))
        .map(|value| (*value).to_owned())
        .collect()
}

fn route_candidates_json(candidates: &[RouteCandidate]) -> String {
    let items = candidates
        .iter()
        .map(|candidate| {
            format!(
                "{{\"model_id\":{},\"backend_id\":{},\"reasons\":{}}}",
                json_string(&candidate.model_id),
                json_string(&candidate.backend_id),
                json_string_array(&candidate.reasons)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn excluded_models_json(excluded_models: &[ExcludedModel]) -> String {
    let items = excluded_models
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

fn option_json_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_single_profile_from_registry_by_skill_and_cost() {
        let profiles = vec![
            profile(
                "slow",
                "remote",
                &["review"],
                true,
                16_000,
                80,
                80,
                10_000,
                &[],
            ),
            profile(
                "fast",
                "local",
                &["summary"],
                true,
                8_000,
                10,
                10,
                10_000,
                &[],
            ),
        ];
        let request = request("summary", &["summary"], 1000, 5000);

        let decision = RuleRouter.route(&profiles, &request);

        assert_eq!(decision.strategy, "single");
        assert_eq!(decision.chosen_model.as_deref(), Some("fast"));
        assert_eq!(decision.backend_id.as_deref(), Some("local"));
        assert_eq!(decision.candidate_count, 2);
        assert!(decision.reason.contains("skill_tag:summary"));
    }

    #[test]
    fn excludes_unhealthy_over_budget_policy_denied_and_missing_capability_profiles() {
        let profiles = vec![
            profile(
                "unhealthy",
                "b1",
                &["summary"],
                false,
                8_000,
                1,
                1,
                10_000,
                &[],
            ),
            profile(
                "too-expensive",
                "b2",
                &["summary"],
                true,
                8_000,
                9000,
                9000,
                10_000,
                &[],
            ),
            profile(
                "policy-denied",
                "b3",
                &["summary"],
                true,
                8_000,
                1,
                1,
                10_000,
                &["deny_secret"],
            ),
            profile(
                "missing-cap",
                "b4",
                &["summary"],
                true,
                8_000,
                1,
                1,
                10_000,
                &[],
            ),
            profile("winner", "b5", &["summary"], true, 8_000, 1, 1, 10_000, &[]),
        ];
        let mut request = request("summary", &["summary"], 1000, 500);
        request.query_features.required_capabilities = vec!["tool-use".to_owned()];
        let mut profiles = profiles;
        profiles[3].capabilities = vec!["text".to_owned()];
        profiles[4].capabilities = vec!["text".to_owned(), "tool-use".to_owned()];

        let decision = RuleRouter.route(&profiles, &request);

        assert_eq!(decision.chosen_model.as_deref(), Some("winner"));
        assert_eq!(decision.excluded_models.len(), 4);
        assert!(excluded_reason(&decision, "unhealthy", "health:unhealthy"));
        assert!(excluded_reason(
            &decision,
            "too-expensive",
            "budget:estimate"
        ));
        assert!(excluded_reason(
            &decision,
            "policy-denied",
            "policy:deny_secret"
        ));
        assert!(excluded_reason(
            &decision,
            "missing-cap",
            "capability:missing:tool-use"
        ));
    }

    fn profile(
        model_id: &str,
        backend_id: &str,
        skill_tags: &[&str],
        healthy: bool,
        max_context_tokens: u64,
        input_cost: u64,
        output_cost: u64,
        remaining_budget: u64,
        deny_policy_reasons: &[&str],
    ) -> ModelProfile {
        ModelProfile {
            model_id: model_id.to_owned(),
            backend_id: backend_id.to_owned(),
            skill_tags: skill_tags.iter().map(|tag| (*tag).to_owned()).collect(),
            capabilities: vec!["text".to_owned()],
            max_context_tokens,
            healthy,
            deny_policy_reasons: deny_policy_reasons
                .iter()
                .map(|reason| (*reason).to_owned())
                .collect(),
            input_cost_per_1k_micro_usd: input_cost,
            output_cost_per_1k_micro_usd: output_cost,
            remaining_budget_micro_usd: remaining_budget,
        }
    }

    fn request(
        task_kind: &str,
        skill_tags: &[&str],
        context_tokens: u64,
        max_budget_micro_usd: u64,
    ) -> RouteRequest {
        RouteRequest {
            task_kind: task_kind.to_owned(),
            skill_tags: skill_tags.iter().map(|tag| (*tag).to_owned()).collect(),
            query_features: QueryFeatures {
                context_tokens,
                estimated_input_tokens: 100,
                estimated_output_tokens: 100,
                max_budget_micro_usd,
                required_capabilities: vec!["text".to_owned()],
            },
        }
    }

    fn excluded_reason(decision: &RouteDecision, model_id: &str, reason: &str) -> bool {
        decision.excluded_models.iter().any(|excluded| {
            excluded.model_id == model_id
                && excluded
                    .reasons
                    .iter()
                    .any(|candidate| candidate.contains(reason))
        })
    }
}
