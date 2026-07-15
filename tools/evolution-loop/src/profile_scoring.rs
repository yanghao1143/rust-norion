use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::json::{
    json_bool_field, json_f64_field, json_object_field, json_string, json_string_array,
    json_string_field, json_u64_field,
};

pub(crate) const SCORING_VERSION: &str = "profile-scoring.v1";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelProfileScore {
    pub(crate) model_id: String,
    pub(crate) skill_tag: String,
    pub(crate) observations: u64,
    pub(crate) success_rate: f64,
    pub(crate) reliability: f64,
    pub(crate) latency_ewma_ms: Option<f64>,
    pub(crate) cost_ewma: Option<f64>,
    pub(crate) quality_hint: f64,
    pub(crate) cache_hit_rate: f64,
    pub(crate) recent_failure_streak: u64,
    pub(crate) drift_penalty: f64,
}

impl ModelProfileScore {
    pub(crate) fn new(model_id: impl Into<String>, skill_tag: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            skill_tag: skill_tag.into(),
            observations: 0,
            success_rate: 0.5,
            reliability: 0.5,
            latency_ewma_ms: None,
            cost_ewma: None,
            quality_hint: 0.5,
            cache_hit_rate: 0.0,
            recent_failure_streak: 0,
            drift_penalty: 0.0,
        }
    }

    pub(crate) fn update(&mut self, sample: &OutcomeSample, config: &ScoringConfig) {
        self.observations = self.observations.saturating_add(1);
        self.success_rate = bounded_ewma(
            self.success_rate,
            if sample.success { 1.0 } else { 0.0 },
            config.success_alpha,
        );
        self.reliability = bounded_ewma(
            self.reliability,
            if sample.success { 1.0 } else { 0.0 },
            config.reliability_alpha,
        );
        if let Some(latency_ms) = sample.latency_ms {
            self.latency_ewma_ms = Some(option_ewma(
                self.latency_ewma_ms,
                latency_ms,
                config.latency_alpha,
            ));
        }
        if let Some(cost) = sample.cost {
            self.cost_ewma = Some(option_ewma(self.cost_ewma, cost, config.cost_alpha));
        }
        if let Some(quality) = sample.quality_hint {
            self.quality_hint = bounded_ewma(self.quality_hint, quality, config.quality_alpha);
        } else if sample.success {
            self.quality_hint = bounded_ewma(self.quality_hint, 0.6, config.quality_alpha * 0.5);
        } else {
            self.quality_hint = bounded_ewma(self.quality_hint, 0.2, config.quality_alpha);
        }
        self.cache_hit_rate = bounded_ewma(
            self.cache_hit_rate,
            if sample.cache_hit { 1.0 } else { 0.0 },
            config.cache_alpha,
        );
        if sample.success {
            self.recent_failure_streak = 0;
        } else {
            self.recent_failure_streak = self.recent_failure_streak.saturating_add(1);
        }
        self.drift_penalty = bounded_ewma(
            self.drift_penalty,
            if sample.drift_detected { 1.0 } else { 0.0 },
            config.drift_alpha,
        );
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutcomeSample {
    pub(crate) model_id: String,
    pub(crate) skill_tag: String,
    pub(crate) success: bool,
    pub(crate) latency_ms: Option<f64>,
    pub(crate) cost: Option<f64>,
    pub(crate) quality_hint: Option<f64>,
    pub(crate) cache_hit: bool,
    pub(crate) drift_detected: bool,
}

impl OutcomeSample {
    pub(crate) fn from_m3_json(body: &str, default_skill_tag: &str) -> Option<Self> {
        let model_id = json_string_field(body, "model_id")
            .or_else(|| json_string_field(body, "runtime_model"))
            .or_else(|| json_string_field(body, "selected_role"))?;
        let skill_tag = json_string_field(body, "skill_tag")
            .or_else(|| json_string_field(body, "task_kind"))
            .unwrap_or_else(|| default_skill_tag.to_owned());
        let success = json_bool_field(body, "success")
            .or_else(|| json_bool_field(body, "passed"))
            .unwrap_or(false);
        let latency_ms = json_f64_field(body, "latency_ms")
            .or_else(|| json_f64_field(body, "elapsed_ms"))
            .or_else(|| json_u64_field(body, "elapsed_ms").map(|value| value as f64));
        let cost = json_f64_field(body, "cost")
            .or_else(|| json_f64_field(body, "cost_hint"))
            .or_else(|| json_u64_field(body, "runtime_tokens").map(|tokens| tokens as f64));
        let quality_hint = json_f64_field(body, "quality_hint")
            .or_else(|| json_f64_field(body, "reward_hint"))
            .map(clamp01);
        let cache_hit = json_bool_field(body, "cache_hit").unwrap_or(false);
        let drift_detected = json_bool_field(body, "drift_detected")
            .or_else(|| json_bool_field(body, "profile_drift"))
            .unwrap_or(false);

        Some(Self {
            model_id,
            skill_tag,
            success,
            latency_ms,
            cost,
            quality_hint,
            cache_hit,
            drift_detected,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScoringConfig {
    pub(crate) version: String,
    pub(crate) rollback_hint: String,
    pub(crate) success_alpha: f64,
    pub(crate) reliability_alpha: f64,
    pub(crate) latency_alpha: f64,
    pub(crate) cost_alpha: f64,
    pub(crate) quality_alpha: f64,
    pub(crate) cache_alpha: f64,
    pub(crate) drift_alpha: f64,
    pub(crate) success_weight: f64,
    pub(crate) reliability_weight: f64,
    pub(crate) quality_weight: f64,
    pub(crate) latency_weight: f64,
    pub(crate) cost_weight: f64,
    pub(crate) cache_hit_weight: f64,
    pub(crate) drift_penalty_weight: f64,
    pub(crate) failure_streak_weight: f64,
    pub(crate) epsilon: f64,
    pub(crate) epsilon_budget: u64,
    pub(crate) exploration_enabled: bool,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            version: SCORING_VERSION.to_owned(),
            rollback_hint: "disable profile routing and fall back to rule routing".to_owned(),
            success_alpha: 0.30,
            reliability_alpha: 0.20,
            latency_alpha: 0.25,
            cost_alpha: 0.25,
            quality_alpha: 0.25,
            cache_alpha: 0.20,
            drift_alpha: 0.35,
            success_weight: 0.28,
            reliability_weight: 0.20,
            quality_weight: 0.24,
            latency_weight: 0.10,
            cost_weight: 0.08,
            cache_hit_weight: 0.06,
            drift_penalty_weight: 0.18,
            failure_streak_weight: 0.04,
            epsilon: 0.03,
            epsilon_budget: 0,
            exploration_enabled: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RoutingScore {
    pub(crate) model_id: String,
    pub(crate) skill_tag: String,
    pub(crate) score: f64,
    pub(crate) explore: bool,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteDecision {
    pub(crate) selected_model_id: String,
    pub(crate) selected_skill_tag: String,
    pub(crate) selected_by_exploration: bool,
    pub(crate) scores: Vec<RoutingScore>,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OfflineRegressionReport {
    pub(crate) version: String,
    pub(crate) candidate_policy: String,
    pub(crate) quality_delta: f64,
    pub(crate) latency_delta_ms: f64,
    pub(crate) cost_delta: f64,
    pub(crate) passed: bool,
    pub(crate) blocked_reason: Option<String>,
}

impl OfflineRegressionReport {
    pub(crate) fn compare(
        candidate_policy: impl Into<String>,
        rule_baseline: &RegressionAggregate,
        profile_candidate: &RegressionAggregate,
    ) -> Self {
        let quality_delta = profile_candidate.quality - rule_baseline.quality;
        let latency_delta_ms = profile_candidate.latency_ms - rule_baseline.latency_ms;
        let cost_delta = profile_candidate.cost - rule_baseline.cost;
        let passed = quality_delta >= -0.000_001 && (latency_delta_ms <= 0.0 || cost_delta <= 0.0);
        let blocked_reason = if passed {
            None
        } else {
            Some(format!(
                "offline_regression_blocked quality_delta={quality_delta:.3} latency_delta_ms={latency_delta_ms:.1} cost_delta={cost_delta:.3}"
            ))
        };
        Self {
            version: SCORING_VERSION.to_owned(),
            candidate_policy: candidate_policy.into(),
            quality_delta,
            latency_delta_ms,
            cost_delta,
            passed,
            blocked_reason,
        }
    }

    pub(crate) fn switch_decision(&self, config: &ScoringConfig) -> PolicySwitchDecision {
        if self.passed {
            PolicySwitchDecision {
                allow_switch: true,
                policy_version: config.version.clone(),
                reason: format!(
                    "offline_regression_passed policy={} quality_delta={:.3} latency_delta_ms={:.1} cost_delta={:.3} rollback_hint={}",
                    self.candidate_policy,
                    self.quality_delta,
                    self.latency_delta_ms,
                    self.cost_delta,
                    config.rollback_hint
                ),
            }
        } else {
            PolicySwitchDecision {
                allow_switch: false,
                policy_version: config.version.clone(),
                reason: self.blocked_reason.clone().unwrap_or_else(|| {
                    format!(
                        "offline_regression_blocked rollback_hint={}",
                        config.rollback_hint
                    )
                }),
            }
        }
    }

    pub(crate) fn json_report(&self) -> String {
        format!(
            "{{\"version\":{},\"candidate_policy\":{},\"quality_delta\":{:.6},\"latency_delta_ms\":{:.3},\"cost_delta\":{:.6},\"passed\":{},\"blocked_reason\":{}}}",
            json_string(&self.version),
            json_string(&self.candidate_policy),
            self.quality_delta,
            self.latency_delta_ms,
            self.cost_delta,
            self.passed,
            option_str_json(self.blocked_reason.as_deref())
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RegressionAggregate {
    pub(crate) quality: f64,
    pub(crate) latency_ms: f64,
    pub(crate) cost: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OfflineReplayAggregate {
    pub(crate) policy: String,
    pub(crate) samples: usize,
    pub(crate) successes: usize,
    pub(crate) failures: usize,
    pub(crate) quality_avg: f64,
    pub(crate) latency_avg_ms: f64,
    pub(crate) cost_avg: f64,
    pub(crate) failure_rate: f64,
    pub(crate) drift_penalty: f64,
}

impl OfflineReplayAggregate {
    fn empty(policy: &str) -> Self {
        Self {
            policy: policy.to_owned(),
            samples: 0,
            successes: 0,
            failures: 0,
            quality_avg: 0.0,
            latency_avg_ms: 0.0,
            cost_avg: 0.0,
            failure_rate: 1.0,
            drift_penalty: 1.0,
        }
    }

    fn from_records(policy: &str, records: &[ReplayOutcomeRecord]) -> Self {
        if records.is_empty() {
            return Self::empty(policy);
        }
        let samples = records.len();
        let successes = records.iter().filter(|record| record.success).count();
        let failures = samples.saturating_sub(successes);
        let quality_avg = records
            .iter()
            .map(ReplayOutcomeRecord::quality_value)
            .sum::<f64>()
            / samples as f64;
        let latency_avg_ms =
            records.iter().map(|record| record.latency_ms).sum::<f64>() / samples as f64;
        let cost_avg = records.iter().map(|record| record.cost).sum::<f64>() / samples as f64;
        let failure_rate = failures as f64 / samples as f64;
        let drift_penalty = records
            .iter()
            .filter(|record| record.drift_detected)
            .count() as f64
            / samples as f64;
        Self {
            policy: policy.to_owned(),
            samples,
            successes,
            failures,
            quality_avg,
            latency_avg_ms,
            cost_avg,
            failure_rate,
            drift_penalty,
        }
    }

    pub(crate) fn json_report(&self) -> String {
        format!(
            "{{\"policy\":{},\"samples\":{},\"successes\":{},\"failures\":{},\"quality_avg\":{:.6},\"latency_avg_ms\":{:.3},\"cost_avg\":{:.6},\"failure_rate\":{:.6},\"drift_penalty\":{:.6}}}",
            json_string(&self.policy),
            self.samples,
            self.successes,
            self.failures,
            self.quality_avg,
            self.latency_avg_ms,
            self.cost_avg,
            self.failure_rate,
            self.drift_penalty
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OfflineReplayReport {
    pub(crate) version: String,
    pub(crate) source_path: String,
    pub(crate) min_samples: usize,
    pub(crate) baseline: OfflineReplayAggregate,
    pub(crate) candidate: OfflineReplayAggregate,
    pub(crate) quality_delta: f64,
    pub(crate) latency_delta_ms: f64,
    pub(crate) cost_delta: f64,
    pub(crate) failure_rate_delta: f64,
    pub(crate) drift_penalty_delta: f64,
    pub(crate) allow_switch: bool,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) rollback_hint: String,
}

impl OfflineReplayReport {
    pub(crate) fn from_outcome_jsonl(
        source_path: impl Into<String>,
        outcome_jsonl: &str,
        min_samples: usize,
        config: &ScoringConfig,
    ) -> Self {
        let min_samples = min_samples.max(1);
        let mut baseline_records = Vec::new();
        let mut candidate_records = Vec::new();
        for line in outcome_jsonl
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            let Some(record) = ReplayOutcomeRecord::from_json_line(line) else {
                continue;
            };
            match record.policy {
                ReplayPolicy::Baseline => baseline_records.push(record),
                ReplayPolicy::Candidate => candidate_records.push(record),
            }
        }
        let baseline = OfflineReplayAggregate::from_records("rule-routing", &baseline_records);
        let candidate = OfflineReplayAggregate::from_records("profile-routing", &candidate_records);
        Self::compare(source_path, min_samples, baseline, candidate, config)
    }

    fn compare(
        source_path: impl Into<String>,
        min_samples: usize,
        baseline: OfflineReplayAggregate,
        candidate: OfflineReplayAggregate,
        config: &ScoringConfig,
    ) -> Self {
        let quality_delta = candidate.quality_avg - baseline.quality_avg;
        let latency_delta_ms = candidate.latency_avg_ms - baseline.latency_avg_ms;
        let cost_delta = candidate.cost_avg - baseline.cost_avg;
        let failure_rate_delta = candidate.failure_rate - baseline.failure_rate;
        let drift_penalty_delta = candidate.drift_penalty - baseline.drift_penalty;
        let blocked_reason = replay_blocked_reason(
            &baseline,
            &candidate,
            min_samples,
            quality_delta,
            latency_delta_ms,
            cost_delta,
            failure_rate_delta,
            drift_penalty_delta,
            &config.rollback_hint,
        );
        Self {
            version: SCORING_VERSION.to_owned(),
            source_path: source_path.into(),
            min_samples,
            baseline,
            candidate,
            quality_delta,
            latency_delta_ms,
            cost_delta,
            failure_rate_delta,
            drift_penalty_delta,
            allow_switch: blocked_reason.is_none(),
            blocked_reason,
            rollback_hint: config.rollback_hint.clone(),
        }
    }

    pub(crate) fn json_report(&self) -> String {
        format!(
            "{{\"version\":{},\"source_path\":{},\"min_samples\":{},\"baseline\":{},\"candidate\":{},\"deltas\":{{\"quality_delta\":{:.6},\"latency_delta_ms\":{:.3},\"cost_delta\":{:.6},\"failure_rate_delta\":{:.6},\"drift_penalty_delta\":{:.6}}},\"allow_switch\":{},\"blocked_reason\":{},\"rollback_hint\":{}}}",
            json_string(&self.version),
            json_string(&self.source_path),
            self.min_samples,
            self.baseline.json_report(),
            self.candidate.json_report(),
            self.quality_delta,
            self.latency_delta_ms,
            self.cost_delta,
            self.failure_rate_delta,
            self.drift_penalty_delta,
            self.allow_switch,
            option_str_json(self.blocked_reason.as_deref()),
            json_string(&self.rollback_hint)
        )
    }
}

pub(crate) fn load_offline_replay_report(
    path: Option<&Path>,
    min_samples: usize,
    config: &ScoringConfig,
) -> Result<Option<OfflineReplayReport>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "read profile outcome log {} failed: {error}",
            path.display()
        )
    })?;
    Ok(Some(OfflineReplayReport::from_outcome_jsonl(
        path.display().to_string(),
        &text,
        min_samples,
        config,
    )))
}

pub(crate) fn option_offline_replay_json(report: Option<&OfflineReplayReport>) -> String {
    report
        .map(OfflineReplayReport::json_report)
        .unwrap_or_else(|| "{}".to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayPolicy {
    Baseline,
    Candidate,
}

#[derive(Debug, Clone, PartialEq)]
struct ReplayOutcomeRecord {
    policy: ReplayPolicy,
    success: bool,
    latency_ms: f64,
    cost: f64,
    quality_score: Option<f64>,
    drift_detected: bool,
}

impl ReplayOutcomeRecord {
    fn from_json_line(line: &str) -> Option<Self> {
        let strategy = json_string_field(line, "strategy").or_else(|| {
            json_object_field(line, "route_decision")
                .and_then(|route| json_string_field(&route, "strategy"))
        })?;
        let policy = replay_policy(&strategy)?;
        let success = json_bool_field(line, "ok")
            .or_else(|| json_bool_field(line, "success"))
            .unwrap_or(false);
        let latency_ms = json_f64_field(line, "latency_ms")
            .or_else(|| json_u64_field(line, "latency_ms").map(|value| value as f64))
            .or_else(|| json_f64_field(line, "elapsed_ms"))
            .or_else(|| json_u64_field(line, "elapsed_ms").map(|value| value as f64));
        let cost = json_f64_field(line, "cost")
            .or_else(|| json_f64_field(line, "cost_estimate_micro_usd"))
            .or_else(|| json_u64_field(line, "cost_estimate_micro_usd").map(|value| value as f64));
        let quality_score = json_f64_field(line, "quality_score")
            .or_else(|| json_f64_field(line, "quality_hint"))
            .map(clamp01);
        let drift_detected = json_bool_field(line, "drift_detected")
            .or_else(|| json_bool_field(line, "profile_drift"))
            .or_else(|| legacy_drift_evidence(line));
        if success
            && (latency_ms.is_none()
                || cost.is_none()
                || quality_score.is_none()
                || drift_detected.is_none())
        {
            return None;
        }
        Some(Self {
            policy,
            success,
            latency_ms: latency_ms.unwrap_or_default().max(0.0),
            cost: cost.unwrap_or_default().max(0.0),
            quality_score,
            drift_detected: drift_detected.unwrap_or(false),
        })
    }

    fn quality_value(&self) -> f64 {
        self.quality_score
            .unwrap_or(if self.success { 1.0 } else { 0.0 })
    }
}

fn legacy_drift_evidence(line: &str) -> Option<bool> {
    let mut found = false;
    let mut drift_detected = false;
    for field in ["reward_placeholder", "reflection_placeholder", "error_kind"] {
        if let Some(value) = json_string_field(line, field) {
            found = true;
            drift_detected |= value.to_ascii_lowercase().contains("drift");
        }
    }
    found.then_some(drift_detected)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PolicySwitchDecision {
    pub(crate) allow_switch: bool,
    pub(crate) policy_version: String,
    pub(crate) reason: String,
}

#[derive(Debug, Clone)]
pub(crate) struct OnlineScorer {
    config: ScoringConfig,
    profiles: BTreeMap<(String, String), ModelProfileScore>,
    exploration_remaining: u64,
}

impl OnlineScorer {
    pub(crate) fn new(config: ScoringConfig) -> Self {
        let exploration_remaining = config.epsilon_budget;
        Self {
            config,
            profiles: BTreeMap::new(),
            exploration_remaining,
        }
    }

    pub(crate) fn update(&mut self, sample: OutcomeSample) -> &ModelProfileScore {
        let key = (sample.model_id.clone(), sample.skill_tag.clone());
        let profile = self
            .profiles
            .entry(key)
            .or_insert_with(|| ModelProfileScore::new(&sample.model_id, &sample.skill_tag));
        profile.update(&sample, &self.config);
        profile
    }

    pub(crate) fn profile(&self, model_id: &str, skill_tag: &str) -> Option<&ModelProfileScore> {
        self.profiles
            .get(&(model_id.to_owned(), skill_tag.to_owned()))
    }

    pub(crate) fn score_candidate(&self, model_id: &str, skill_tag: &str) -> RoutingScore {
        let profile = self.profile(model_id, skill_tag);
        let default_profile;
        let profile = match profile {
            Some(profile) => profile,
            None => {
                default_profile = ModelProfileScore::new(model_id, skill_tag);
                &default_profile
            }
        };
        let latency_component = inverse_penalty(profile.latency_ewma_ms, 5_000.0);
        let cost_component = inverse_penalty(profile.cost_ewma, 10_000.0);
        let failure_penalty =
            (profile.recent_failure_streak.min(5) as f64) * self.config.failure_streak_weight;
        let positive = profile.success_rate * self.config.success_weight
            + profile.reliability * self.config.reliability_weight
            + profile.quality_hint * self.config.quality_weight
            + latency_component * self.config.latency_weight
            + cost_component * self.config.cost_weight
            + profile.cache_hit_rate * self.config.cache_hit_weight;
        let negative = profile.drift_penalty * self.config.drift_penalty_weight + failure_penalty;
        let score = clamp01(positive - negative);
        RoutingScore {
            model_id: model_id.to_owned(),
            skill_tag: skill_tag.to_owned(),
            score,
            explore: false,
            reason: format!(
                "version={} success_rate={:.3} reliability={:.3} quality_hint={:.3} latency_component={:.3} cost_component={:.3} cache_hit_rate={:.3} drift_penalty={:.3} failure_streak={} score={:.3}",
                self.config.version,
                profile.success_rate,
                profile.reliability,
                profile.quality_hint,
                latency_component,
                cost_component,
                profile.cache_hit_rate,
                profile.drift_penalty,
                profile.recent_failure_streak,
                score
            ),
        }
    }

    pub(crate) fn route(
        &mut self,
        candidates: &[String],
        skill_tag: &str,
        exploration_roll: Option<f64>,
    ) -> Option<RouteDecision> {
        let mut scores = candidates
            .iter()
            .map(|model_id| self.score_candidate(model_id, skill_tag))
            .collect::<Vec<_>>();
        scores.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.model_id.cmp(&right.model_id))
        });
        let selected_index = if self.should_explore(exploration_roll.unwrap_or(1.0)) {
            scores
                .iter()
                .enumerate()
                .skip(1)
                .min_by(|(_, left), (_, right)| left.model_id.cmp(&right.model_id))
                .map(|(index, _)| index)
                .unwrap_or(0)
        } else {
            0
        };
        let selected = scores.get_mut(selected_index)?;
        selected.explore = selected_index != 0;
        let selected_model_id = selected.model_id.clone();
        let selected_skill_tag = selected.skill_tag.clone();
        let selected_by_exploration = selected.explore;
        let reason = if selected_by_exploration {
            format!(
                "profile_route_explore remaining_budget={} selected={} reason={}",
                self.exploration_remaining, selected_model_id, selected.reason
            )
        } else {
            format!(
                "profile_route_best selected={} reason={}",
                selected_model_id, selected.reason
            )
        };
        Some(RouteDecision {
            selected_model_id,
            selected_skill_tag,
            selected_by_exploration,
            scores,
            reason,
        })
    }

    pub(crate) fn explanation_json(&self, decision: &RouteDecision) -> String {
        let reasons = decision
            .scores
            .iter()
            .map(|score| score.reason.clone())
            .collect::<Vec<_>>();
        format!(
            "{{\"version\":{},\"selected_model_id\":{},\"selected_skill_tag\":{},\"selected_by_exploration\":{},\"reason\":{},\"candidate_reasons\":{}}}",
            json_string(&self.config.version),
            json_string(&decision.selected_model_id),
            json_string(&decision.selected_skill_tag),
            decision.selected_by_exploration,
            json_string(&decision.reason),
            json_string_array(&reasons)
        )
    }

    pub(crate) fn exploration_remaining(&self) -> u64 {
        self.exploration_remaining
    }

    fn should_explore(&mut self, exploration_roll: f64) -> bool {
        if !self.config.exploration_enabled
            || self.config.epsilon <= 0.0
            || self.exploration_remaining == 0
        {
            return false;
        }
        if exploration_roll <= self.config.epsilon {
            self.exploration_remaining = self.exploration_remaining.saturating_sub(1);
            true
        } else {
            false
        }
    }
}

fn bounded_ewma(previous: f64, next: f64, alpha: f64) -> f64 {
    clamp01(previous * (1.0 - clamp01(alpha)) + clamp01(next) * clamp01(alpha))
}

fn option_ewma(previous: Option<f64>, next: f64, alpha: f64) -> f64 {
    let next = next.max(0.0);
    previous
        .map(|previous| previous * (1.0 - clamp01(alpha)) + next * clamp01(alpha))
        .unwrap_or(next)
}

fn inverse_penalty(value: Option<f64>, scale: f64) -> f64 {
    let Some(value) = value else {
        return 0.5;
    };
    clamp01(1.0 / (1.0 + value.max(0.0) / scale.max(1.0)))
}

fn clamp01(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    value.clamp(0.0, 1.0)
}

fn option_str_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn replay_policy(strategy: &str) -> Option<ReplayPolicy> {
    let normalized = strategy.trim().to_ascii_lowercase();
    if normalized.contains("profile") || normalized.contains("candidate") {
        Some(ReplayPolicy::Candidate)
    } else if normalized.contains("rule")
        || normalized.contains("baseline")
        || normalized == "single"
        || normalized == "rule-routing"
    {
        Some(ReplayPolicy::Baseline)
    } else {
        None
    }
}

fn replay_blocked_reason(
    baseline: &OfflineReplayAggregate,
    candidate: &OfflineReplayAggregate,
    min_samples: usize,
    quality_delta: f64,
    latency_delta_ms: f64,
    cost_delta: f64,
    failure_rate_delta: f64,
    drift_penalty_delta: f64,
    rollback_hint: &str,
) -> Option<String> {
    if baseline.samples < min_samples || candidate.samples < min_samples {
        return Some(format!(
            "offline_replay_insufficient_samples baseline_samples={} candidate_samples={} min_samples={} rollback_hint={rollback_hint}",
            baseline.samples, candidate.samples, min_samples
        ));
    }
    if quality_delta < -0.000_001 {
        return Some(format!(
            "offline_replay_quality_regression quality_delta={quality_delta:.3} rollback_hint={rollback_hint}"
        ));
    }
    if latency_delta_ms > 0.000_001 {
        return Some(format!(
            "offline_replay_latency_regression latency_delta_ms={latency_delta_ms:.1} rollback_hint={rollback_hint}"
        ));
    }
    if cost_delta > 0.000_001 {
        return Some(format!(
            "offline_replay_cost_regression cost_delta={cost_delta:.3} rollback_hint={rollback_hint}"
        ));
    }
    if failure_rate_delta > 0.000_001 {
        return Some(format!(
            "offline_replay_failure_regression failure_rate_delta={failure_rate_delta:.3} rollback_hint={rollback_hint}"
        ));
    }
    if drift_penalty_delta > 0.000_001 {
        return Some(format!(
            "offline_replay_drift_regression drift_penalty_delta={drift_penalty_delta:.3} rollback_hint={rollback_hint}"
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(model_id: &str, skill_tag: &str, success: bool) -> OutcomeSample {
        OutcomeSample {
            model_id: model_id.to_owned(),
            skill_tag: skill_tag.to_owned(),
            success,
            latency_ms: Some(1_000.0),
            cost: Some(100.0),
            quality_hint: Some(if success { 0.9 } else { 0.1 }),
            cache_hit: false,
            drift_detected: false,
        }
    }

    #[test]
    fn successful_outcomes_raise_profile_score() {
        let mut scorer = OnlineScorer::new(ScoringConfig::default());
        let before = scorer.score_candidate("fast-code", "code").score;

        scorer.update(sample("fast-code", "code", true));
        scorer.update(sample("fast-code", "code", true));
        let after = scorer.score_candidate("fast-code", "code").score;

        assert!(after > before, "after={after} before={before}");
        assert!(
            scorer
                .score_candidate("fast-code", "code")
                .reason
                .contains("success_rate")
        );
    }

    #[test]
    fn failures_and_drift_lower_profile_score() {
        let mut scorer = OnlineScorer::new(ScoringConfig::default());
        scorer.update(sample("reviewer", "review", true));
        let before = scorer.score_candidate("reviewer", "review").score;
        let mut failed = sample("reviewer", "review", false);
        failed.drift_detected = true;

        scorer.update(failed.clone());
        scorer.update(failed);
        let after = scorer.score_candidate("reviewer", "review").score;
        let profile = scorer.profile("reviewer", "review").unwrap();

        assert!(after < before, "after={after} before={before}");
        assert_eq!(profile.recent_failure_streak, 2);
        assert!(profile.drift_penalty > 0.0);
    }

    #[test]
    fn latency_and_cost_use_ewma_and_penalize_slow_expensive_models() {
        let mut scorer = OnlineScorer::new(ScoringConfig::default());
        let mut fast = sample("fast", "summary", true);
        fast.latency_ms = Some(500.0);
        fast.cost = Some(50.0);
        let mut slow = sample("slow", "summary", true);
        slow.latency_ms = Some(9_000.0);
        slow.cost = Some(20_000.0);

        scorer.update(fast);
        scorer.update(slow);
        let fast_score = scorer.score_candidate("fast", "summary").score;
        let slow_score = scorer.score_candidate("slow", "summary").score;

        assert_eq!(
            scorer.profile("fast", "summary").unwrap().latency_ewma_ms,
            Some(500.0)
        );
        assert!(fast_score > slow_score);
    }

    #[test]
    fn cache_hit_bonus_can_break_otherwise_equal_candidates() {
        let mut scorer = OnlineScorer::new(ScoringConfig::default());
        let cached = OutcomeSample {
            cache_hit: true,
            ..sample("cached", "routing", true)
        };
        scorer.update(cached);
        scorer.update(sample("uncached", "routing", true));

        let cached_score = scorer.score_candidate("cached", "routing").score;
        let uncached_score = scorer.score_candidate("uncached", "routing").score;

        assert!(cached_score > uncached_score);
    }

    #[test]
    fn epsilon_exploration_has_budget_and_can_be_disabled() {
        let config = ScoringConfig {
            exploration_enabled: true,
            epsilon: 1.0,
            epsilon_budget: 1,
            ..ScoringConfig::default()
        };
        let mut scorer = OnlineScorer::new(config);
        scorer.update(sample("best", "code", true));
        scorer.update(sample("alt", "code", false));
        let candidates = ["best".to_owned(), "alt".to_owned()];

        let first = scorer.route(&candidates, "code", Some(0.0)).unwrap();
        let second = scorer.route(&candidates, "code", Some(0.0)).unwrap();

        assert_eq!(first.selected_model_id, "alt");
        assert!(first.selected_by_exploration);
        assert_eq!(scorer.exploration_remaining(), 0);
        assert_eq!(second.selected_model_id, "best");
        assert!(!second.selected_by_exploration);
    }

    #[test]
    fn parses_m3_outcome_json_into_sample() {
        let sample = OutcomeSample::from_m3_json(
            r#"{"runtime_model":"quality","task_kind":"review","success":true,"elapsed_ms":1200,"runtime_tokens":444,"quality_hint":0.82,"cache_hit":true}"#,
            "summary",
        )
        .unwrap();

        assert_eq!(sample.model_id, "quality");
        assert_eq!(sample.skill_tag, "review");
        assert!(sample.success);
        assert_eq!(sample.latency_ms, Some(1200.0));
        assert_eq!(sample.cost, Some(444.0));
        assert_eq!(sample.quality_hint, Some(0.82));
        assert!(sample.cache_hit);
    }

    #[test]
    fn offline_regression_blocks_policy_switch_when_quality_drops_without_tradeoff() {
        let report = OfflineRegressionReport::compare(
            "profile-route",
            &RegressionAggregate {
                quality: 0.90,
                latency_ms: 1_000.0,
                cost: 100.0,
            },
            &RegressionAggregate {
                quality: 0.85,
                latency_ms: 1_100.0,
                cost: 120.0,
            },
        );
        let decision = report.switch_decision(&ScoringConfig::default());

        assert!(!report.passed);
        assert!(!decision.allow_switch);
        assert!(decision.reason.contains("offline_regression_blocked"));
        assert!(report.json_report().contains("\"passed\":false"));
    }

    #[test]
    fn offline_regression_allows_policy_switch_with_quality_preserved_and_better_latency() {
        let config = ScoringConfig::default();
        let report = OfflineRegressionReport::compare(
            "profile-route",
            &RegressionAggregate {
                quality: 0.80,
                latency_ms: 1_000.0,
                cost: 100.0,
            },
            &RegressionAggregate {
                quality: 0.80,
                latency_ms: 900.0,
                cost: 110.0,
            },
        );
        let decision = report.switch_decision(&config);

        assert!(report.passed);
        assert!(decision.allow_switch);
        assert_eq!(decision.policy_version, SCORING_VERSION);
        assert!(decision.reason.contains(&config.rollback_hint));
    }

    #[test]
    fn offline_replay_allows_profile_switch_when_candidate_preserves_quality_and_costs_less() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            "fixture.jsonl",
            &fixture_jsonl(&[
                ("rule-baseline", true, 0.80, 1_000, 100, false),
                ("rule-baseline", true, 0.82, 1_100, 120, false),
                ("profile-candidate", true, 0.83, 900, 90, false),
                ("profile-candidate", true, 0.84, 950, 95, false),
            ]),
            2,
            &ScoringConfig::default(),
        );

        assert!(report.allow_switch);
        assert!(report.quality_delta > 0.0);
        assert!(report.latency_delta_ms < 0.0);
        assert!(report.cost_delta < 0.0);
        assert!(report.json_report().contains("\"allow_switch\":true"));
        assert!(report.json_report().contains("\"rollback_hint\""));
    }

    #[test]
    fn offline_replay_blocks_quality_regression() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            "fixture.jsonl",
            &fixture_jsonl(&[
                ("rule-baseline", true, 0.90, 1_000, 100, false),
                ("rule-baseline", true, 0.88, 1_000, 100, false),
                ("profile-candidate", true, 0.70, 900, 90, false),
                ("profile-candidate", true, 0.72, 900, 90, false),
            ]),
            2,
            &ScoringConfig::default(),
        );

        assert!(!report.allow_switch);
        assert!(report.quality_delta < 0.0);
        assert!(
            report
                .blocked_reason
                .as_deref()
                .unwrap()
                .contains("quality_regression")
        );
    }

    #[test]
    fn offline_replay_blocks_latency_regression() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            "fixture.jsonl",
            &fixture_jsonl(&[
                ("rule-baseline", true, 0.80, 1_000, 100, false),
                ("rule-baseline", true, 0.80, 1_000, 100, false),
                ("profile-candidate", true, 0.80, 1_400, 90, false),
                ("profile-candidate", true, 0.80, 1_300, 90, false),
            ]),
            2,
            &ScoringConfig::default(),
        );

        assert!(!report.allow_switch);
        assert!(report.latency_delta_ms > 0.0);
        assert!(
            report
                .blocked_reason
                .as_deref()
                .unwrap()
                .contains("latency_regression")
        );
    }

    #[test]
    fn offline_replay_blocks_cost_regression() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            "fixture.jsonl",
            &fixture_jsonl(&[
                ("rule-baseline", true, 0.80, 1_000, 100, false),
                ("rule-baseline", true, 0.80, 1_000, 100, false),
                ("profile-candidate", true, 0.80, 900, 140, false),
                ("profile-candidate", true, 0.80, 900, 130, false),
            ]),
            2,
            &ScoringConfig::default(),
        );

        assert!(!report.allow_switch);
        assert!(report.cost_delta > 0.0);
        assert!(
            report
                .blocked_reason
                .as_deref()
                .unwrap()
                .contains("cost_regression")
        );
    }

    #[test]
    fn offline_replay_blocks_insufficient_samples() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            "fixture.jsonl",
            &fixture_jsonl(&[
                ("rule-baseline", true, 0.80, 1_000, 100, false),
                ("profile-candidate", true, 0.90, 800, 80, false),
            ]),
            2,
            &ScoringConfig::default(),
        );

        assert!(!report.allow_switch);
        assert!(
            report
                .blocked_reason
                .as_deref()
                .unwrap()
                .contains("insufficient_samples")
        );
    }

    #[test]
    fn offline_replay_skips_records_without_latency_evidence() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            "fixture.jsonl",
            concat!(
                "{\"strategy\":\"single\",\"success\":true,\"cost\":100,\"quality_score\":0.90,\"drift_detected\":false}\n",
                "{\"strategy\":\"profile-candidate\",\"success\":true,\"elapsed_ms\":800,\"cost_estimate_micro_usd\":80,\"quality_score\":0.90,\"drift_detected\":false}\n",
            ),
            1,
            &ScoringConfig::default(),
        );

        assert_eq!(report.baseline.samples, 0);
        assert_eq!(report.candidate.samples, 1);
        assert!(!report.allow_switch);
        assert!(
            report
                .blocked_reason
                .as_deref()
                .unwrap()
                .contains("insufficient_samples")
        );
    }

    #[test]
    fn offline_replay_skips_successful_candidate_with_incomplete_metric_evidence() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            "fixture.jsonl",
            concat!(
                "{\"strategy\":\"single\",\"success\":true,\"elapsed_ms\":1000,\"cost\":100,\"quality_score\":0.90,\"drift_detected\":false}\n",
                "{\"strategy\":\"profile-candidate\",\"success\":true,\"elapsed_ms\":800,\"cost\":80,\"drift_detected\":false}\n",
                "{\"strategy\":\"profile-candidate\",\"success\":true,\"elapsed_ms\":800,\"quality_score\":0.90,\"drift_detected\":false}\n",
                "{\"strategy\":\"profile-candidate\",\"success\":true,\"elapsed_ms\":800,\"cost\":80,\"quality_score\":0.90}\n",
            ),
            1,
            &ScoringConfig::default(),
        );

        assert_eq!(report.baseline.samples, 1);
        assert_eq!(report.candidate.samples, 0);
        assert!(!report.allow_switch);
        assert!(
            report
                .blocked_reason
                .as_deref()
                .unwrap()
                .contains("insufficient_samples")
        );
    }

    #[test]
    fn offline_replay_accepts_legacy_v1_placeholder_drift_evidence() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            "legacy-outcomes.jsonl",
            "{\"schema\":\"norion.request_outcome.v1\",\"strategy\":\"single\",\"ok\":true,\"latency_ms\":1000,\"cost_estimate_micro_usd\":100,\"quality_score\":0.80,\"reward_placeholder\":\"process_reward:pending\",\"reflection_placeholder\":\"reflection:pending\"}\n",
            1,
            &ScoringConfig::default(),
        );

        assert_eq!(report.baseline.samples, 1);
        assert_eq!(report.baseline.drift_penalty, 0.0);
        assert_eq!(report.candidate.samples, 0);
        assert!(!report.allow_switch);
    }

    #[test]
    fn explanation_json_contains_version_and_candidate_reasons() {
        let mut scorer = OnlineScorer::new(ScoringConfig::default());
        scorer.update(sample("quality", "code", true));
        let decision = scorer.route(&["quality".to_owned()], "code", None).unwrap();
        let explanation = scorer.explanation_json(&decision);

        assert!(explanation.contains(SCORING_VERSION));
        assert!(explanation.contains("candidate_reasons"));
        assert!(explanation.contains("profile_route_best"));
    }

    fn fixture_jsonl(records: &[(&str, bool, f64, u64, u64, bool)]) -> String {
        records
            .iter()
            .enumerate()
            .map(|(index, (strategy, ok, quality, latency, cost, drift))| {
                format!(
                    "{{\"schema\":\"norion.request_outcome.v1\",\"trace_id\":\"trace-{index}\",\"request_id\":\"request-{index}\",\"task_kind\":\"review\",\"skill_tags\":[\"review\"],\"strategy\":\"{strategy}\",\"route_decision\":{{\"strategy\":\"{strategy}\",\"chosen_model\":\"model-{index}\",\"candidate_count\":2,\"reason\":\"fixture\"}},\"ok\":{ok},\"latency_ms\":{latency},\"cost_estimate_micro_usd\":{cost},\"quality_score\":{quality},\"drift_detected\":{drift}}}"
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
