#![allow(dead_code)]

use std::collections::BTreeMap;

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
    pub(crate) error_kind: Option<String>,
    pub(crate) latency_ms: Option<f64>,
    pub(crate) cost: Option<f64>,
    pub(crate) quality_hint: Option<f64>,
    pub(crate) cache_hit: bool,
    pub(crate) drift_detected: bool,
    pub(crate) timestamp_unix: Option<u64>,
}

impl OutcomeSample {
    pub(crate) fn from_m3_json(body: &str, default_skill_tag: &str) -> Option<Self> {
        let model_id = json_string_field(body, "chosen_model")
            .or_else(|| json_string_field(body, "model_id"))
            .or_else(|| json_string_field(body, "runtime_model"))
            .or_else(|| json_string_field(body, "selected_role"))?;
        let skill_tag = json_string_field(body, "skill_tag")
            .or_else(|| json_string_field(body, "task_kind"))
            .unwrap_or_else(|| default_skill_tag.to_owned());
        let success = json_bool_field(body, "success")
            .or_else(|| json_bool_field(body, "ok"))
            .or_else(|| json_bool_field(body, "passed"))
            .unwrap_or(false);
        let latency_ms = json_f64_field(body, "latency_ms")
            .or_else(|| json_f64_field(body, "elapsed_ms"))
            .or_else(|| json_u64_field(body, "elapsed_ms").map(|value| value as f64));
        let cost = json_f64_field(body, "cost")
            .or_else(|| json_f64_field(body, "cost_hint"))
            .or_else(|| json_u64_field(body, "cost_estimate_micro_usd").map(|value| value as f64))
            .or_else(|| json_u64_field(body, "runtime_tokens").map(|tokens| tokens as f64));
        let quality_hint = json_f64_field(body, "quality_hint")
            .or_else(|| json_f64_field(body, "quality_score"))
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
            error_kind: json_string_field(body, "error_kind"),
            latency_ms,
            cost,
            quality_hint,
            cache_hit,
            drift_detected,
            timestamp_unix: json_u64_field(body, "timestamp_unix"),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScoringConfig {
    pub(crate) version: String,
    pub(crate) rollback_hint: String,
    pub(crate) circuit_breaker: CircuitBreakerConfig,
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
            circuit_breaker: CircuitBreakerConfig::default(),
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
    pub(crate) circuit_state: String,
    pub(crate) circuit_reason: Option<String>,
    pub(crate) probe_allowed: bool,
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OfflineReplayAggregate {
    pub(crate) sample_count: usize,
    pub(crate) ok_count: usize,
    pub(crate) failure_count: usize,
    pub(crate) drift_count: usize,
    pub(crate) raw_quality: f64,
    pub(crate) quality: f64,
    pub(crate) latency_ms: f64,
    pub(crate) cost: f64,
    pub(crate) drift_penalty: f64,
}

impl OfflineReplayAggregate {
    fn empty() -> Self {
        Self {
            sample_count: 0,
            ok_count: 0,
            failure_count: 0,
            drift_count: 0,
            raw_quality: 0.0,
            quality: 0.0,
            latency_ms: 0.0,
            cost: 0.0,
            drift_penalty: 0.0,
        }
    }

    fn from_samples(samples: &[OutcomeSample]) -> Self {
        if samples.is_empty() {
            return Self::empty();
        }
        let sample_count = samples.len();
        let ok_count = samples.iter().filter(|sample| sample.success).count();
        let failure_count = sample_count.saturating_sub(ok_count);
        let drift_count = samples
            .iter()
            .filter(|sample| sample.drift_detected)
            .count();
        let raw_quality = samples
            .iter()
            .map(|sample| {
                sample
                    .quality_hint
                    .unwrap_or(if sample.success { 0.6 } else { 0.0 })
            })
            .sum::<f64>()
            / sample_count as f64;
        let latency_ms = average_option(samples.iter().filter_map(|sample| sample.latency_ms));
        let cost = average_option(samples.iter().filter_map(|sample| sample.cost));
        let drift_penalty = (drift_count as f64 / sample_count as f64) * 0.10;
        let quality = clamp01(raw_quality - drift_penalty);
        Self {
            sample_count,
            ok_count,
            failure_count,
            drift_count,
            raw_quality,
            quality,
            latency_ms,
            cost,
            drift_penalty,
        }
    }

    fn regression_aggregate(&self) -> RegressionAggregate {
        RegressionAggregate {
            quality: self.quality,
            latency_ms: self.latency_ms,
            cost: self.cost,
        }
    }

    fn json_report(&self) -> String {
        format!(
            "{{\"sample_count\":{},\"ok_count\":{},\"failure_count\":{},\"drift_count\":{},\"raw_quality\":{:.6},\"quality\":{:.6},\"latency_ms\":{:.3},\"cost\":{:.6},\"drift_penalty\":{:.6}}}",
            self.sample_count,
            self.ok_count,
            self.failure_count,
            self.drift_count,
            self.raw_quality,
            self.quality,
            self.latency_ms,
            self.cost,
            self.drift_penalty
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OfflineReplayReport {
    pub(crate) schema: String,
    pub(crate) version: String,
    pub(crate) candidate_policy: String,
    pub(crate) total_records: usize,
    pub(crate) ignored_records: usize,
    pub(crate) min_samples_per_group: usize,
    pub(crate) rule_baseline: OfflineReplayAggregate,
    pub(crate) profile_candidate: OfflineReplayAggregate,
    pub(crate) offline_regression: OfflineRegressionReport,
    pub(crate) switch_decision: PolicySwitchDecision,
}

impl OfflineReplayReport {
    pub(crate) fn from_outcome_jsonl(
        text: &str,
        min_samples_per_group: usize,
        config: &ScoringConfig,
    ) -> Self {
        let mut rule_samples = Vec::new();
        let mut profile_samples = Vec::new();
        let mut total_records = 0usize;
        let mut ignored_records = 0usize;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            total_records += 1;
            let Some(strategy) = replay_strategy(line) else {
                ignored_records += 1;
                continue;
            };
            let Some(sample) = OutcomeSample::from_m3_json(line, "general") else {
                ignored_records += 1;
                continue;
            };
            let strategy = strategy.to_ascii_lowercase();
            if strategy == "single" || strategy.contains("rule") {
                rule_samples.push(sample);
            } else if strategy.contains("profile") {
                profile_samples.push(sample);
            } else {
                ignored_records += 1;
            }
        }
        let rule_baseline = OfflineReplayAggregate::from_samples(&rule_samples);
        let profile_candidate = OfflineReplayAggregate::from_samples(&profile_samples);
        let mut offline_regression = OfflineRegressionReport::compare(
            config.version.clone(),
            &rule_baseline.regression_aggregate(),
            &profile_candidate.regression_aggregate(),
        );
        if rule_baseline.sample_count < min_samples_per_group
            || profile_candidate.sample_count < min_samples_per_group
        {
            offline_regression.passed = false;
            offline_regression.blocked_reason = Some(format!(
                "offline_regression_blocked insufficient_samples rule={} profile={} min={}",
                rule_baseline.sample_count, profile_candidate.sample_count, min_samples_per_group
            ));
        }
        let switch_decision = offline_regression.switch_decision(config);
        Self {
            schema: "norion.profile_routing_offline_replay_report.v1".to_owned(),
            version: SCORING_VERSION.to_owned(),
            candidate_policy: config.version.clone(),
            total_records,
            ignored_records,
            min_samples_per_group,
            rule_baseline,
            profile_candidate,
            offline_regression,
            switch_decision,
        }
    }

    pub(crate) fn json_report(&self) -> String {
        format!(
            "{{\"schema\":{},\"version\":{},\"candidate_policy\":{},\"total_records\":{},\"ignored_records\":{},\"min_samples_per_group\":{},\"rule_baseline\":{},\"profile_candidate\":{},\"offline_regression\":{},\"policy_switch\":{}}}",
            json_string(&self.schema),
            json_string(&self.version),
            json_string(&self.candidate_policy),
            self.total_records,
            self.ignored_records,
            self.min_samples_per_group,
            self.rule_baseline.json_report(),
            self.profile_candidate.json_report(),
            self.offline_regression.json_report(),
            policy_switch_json(&self.switch_decision)
        )
    }
}

pub(crate) fn option_offline_replay_json(summary: Option<&OfflineReplayReport>) -> String {
    summary
        .map(OfflineReplayReport::json_report)
        .unwrap_or_else(|| "null".to_owned())
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CircuitBreakerConfig {
    pub(crate) enabled: bool,
    pub(crate) failure_threshold: u64,
    pub(crate) cooldown_seconds: u64,
    pub(crate) quality_drop_threshold: f64,
    pub(crate) budget_overrun_threshold: f64,
    pub(crate) severe_budget_multiplier: f64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            failure_threshold: 3,
            cooldown_seconds: 300,
            quality_drop_threshold: 0.35,
            budget_overrun_threshold: 10_000.0,
            severe_budget_multiplier: 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreakerState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Open => "open",
            Self::HalfOpen => "half_open",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CircuitBreakerDecision {
    pub(crate) state: CircuitBreakerState,
    pub(crate) selectable: bool,
    pub(crate) probe_allowed: bool,
    pub(crate) reason: Option<String>,
}

impl CircuitBreakerDecision {
    fn closed() -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            selectable: true,
            probe_allowed: false,
            reason: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CircuitBreakerProfileReport {
    pub(crate) model_id: String,
    pub(crate) skill_tag: String,
    pub(crate) state: CircuitBreakerState,
    pub(crate) consecutive_degraded: u64,
    pub(crate) total_degraded: u64,
    pub(crate) total_recovered: u64,
    pub(crate) last_reason: Option<String>,
    pub(crate) last_sample_unix: Option<u64>,
    pub(crate) opened_at_unix: Option<u64>,
    pub(crate) opened_until_unix: Option<u64>,
    pub(crate) probe_allowed: bool,
}

impl CircuitBreakerProfileReport {
    fn json_report(&self) -> String {
        format!(
            "{{\"model_id\":{},\"skill_tag\":{},\"state\":{},\"consecutive_degraded\":{},\"total_degraded\":{},\"total_recovered\":{},\"last_reason\":{},\"last_sample_unix\":{},\"opened_at_unix\":{},\"opened_until_unix\":{},\"probe_allowed\":{}}}",
            json_string(&self.model_id),
            json_string(&self.skill_tag),
            json_string(self.state.as_str()),
            self.consecutive_degraded,
            self.total_degraded,
            self.total_recovered,
            option_str_json(self.last_reason.as_deref()),
            option_u64_json(self.last_sample_unix),
            option_u64_json(self.opened_at_unix),
            option_u64_json(self.opened_until_unix),
            self.probe_allowed
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CircuitBreakerEntry {
    model_id: String,
    skill_tag: String,
    state: CircuitBreakerState,
    consecutive_degraded: u64,
    total_degraded: u64,
    total_recovered: u64,
    last_reason: Option<String>,
    last_sample_unix: Option<u64>,
    opened_at_unix: Option<u64>,
    opened_until_unix: Option<u64>,
}

impl CircuitBreakerEntry {
    fn new(model_id: impl Into<String>, skill_tag: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            skill_tag: skill_tag.into(),
            state: CircuitBreakerState::Closed,
            consecutive_degraded: 0,
            total_degraded: 0,
            total_recovered: 0,
            last_reason: None,
            last_sample_unix: None,
            opened_at_unix: None,
            opened_until_unix: None,
        }
    }

    fn state_at(&self, now_unix: u64) -> CircuitBreakerState {
        if self.state == CircuitBreakerState::Open
            && self
                .opened_until_unix
                .is_some_and(|opened_until| now_unix >= opened_until)
        {
            CircuitBreakerState::HalfOpen
        } else {
            self.state
        }
    }

    fn open(&mut self, now_unix: u64, reason: String, config: &CircuitBreakerConfig) {
        self.state = CircuitBreakerState::Open;
        self.opened_at_unix = Some(now_unix);
        self.opened_until_unix = Some(now_unix.saturating_add(config.cooldown_seconds));
        self.last_reason = Some(reason);
    }

    fn close(&mut self) {
        self.state = CircuitBreakerState::Closed;
        self.consecutive_degraded = 0;
        self.opened_at_unix = None;
        self.opened_until_unix = None;
        self.last_reason = None;
    }

    fn snapshot(&self, now_unix: u64) -> CircuitBreakerProfileReport {
        let state = self.state_at(now_unix);
        CircuitBreakerProfileReport {
            model_id: self.model_id.clone(),
            skill_tag: self.skill_tag.clone(),
            state,
            consecutive_degraded: self.consecutive_degraded,
            total_degraded: self.total_degraded,
            total_recovered: self.total_recovered,
            last_reason: self.last_reason.clone(),
            last_sample_unix: self.last_sample_unix,
            opened_at_unix: self.opened_at_unix,
            opened_until_unix: self.opened_until_unix,
            probe_allowed: state == CircuitBreakerState::HalfOpen,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProfileCircuitBreaker {
    config: CircuitBreakerConfig,
    entries: BTreeMap<(String, String), CircuitBreakerEntry>,
}

impl ProfileCircuitBreaker {
    pub(crate) fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            entries: BTreeMap::new(),
        }
    }

    pub(crate) fn observe(&mut self, sample: &OutcomeSample, observed_unix: u64) {
        if !self.config.enabled {
            return;
        }
        let key = (sample.model_id.clone(), sample.skill_tag.clone());
        let entry = self
            .entries
            .entry(key)
            .or_insert_with(|| CircuitBreakerEntry::new(&sample.model_id, &sample.skill_tag));
        let state_before = entry.state_at(observed_unix);
        entry.state = state_before;
        entry.last_sample_unix = Some(observed_unix);

        if let Some(reason) = degradation_reason(sample, &self.config) {
            entry.consecutive_degraded = entry.consecutive_degraded.saturating_add(1);
            entry.total_degraded = entry.total_degraded.saturating_add(1);
            entry.last_reason = Some(reason.clone());
            if state_before == CircuitBreakerState::HalfOpen
                || entry.consecutive_degraded >= self.config.failure_threshold
                || severe_degradation(sample, &reason, &self.config)
            {
                entry.open(observed_unix, reason, &self.config);
            }
        } else if state_before == CircuitBreakerState::HalfOpen
            || state_before == CircuitBreakerState::Closed
        {
            if state_before == CircuitBreakerState::HalfOpen {
                entry.total_recovered = entry.total_recovered.saturating_add(1);
            }
            entry.close();
            entry.last_sample_unix = Some(observed_unix);
        } else {
            entry.consecutive_degraded = 0;
        }
    }

    pub(crate) fn decision(
        &self,
        model_id: &str,
        skill_tag: &str,
        now_unix: u64,
    ) -> CircuitBreakerDecision {
        if !self.config.enabled {
            return CircuitBreakerDecision::closed();
        }
        let Some(entry) = self
            .entries
            .get(&(model_id.to_owned(), skill_tag.to_owned()))
        else {
            return CircuitBreakerDecision::closed();
        };
        let snapshot = entry.snapshot(now_unix);
        match snapshot.state {
            CircuitBreakerState::Closed => CircuitBreakerDecision::closed(),
            CircuitBreakerState::Open => CircuitBreakerDecision {
                state: CircuitBreakerState::Open,
                selectable: false,
                probe_allowed: false,
                reason: Some(format!(
                    "circuit_open reason={} opened_until_unix={}",
                    snapshot.last_reason.as_deref().unwrap_or("degraded"),
                    snapshot.opened_until_unix.unwrap_or(0)
                )),
            },
            CircuitBreakerState::HalfOpen => CircuitBreakerDecision {
                state: CircuitBreakerState::HalfOpen,
                selectable: false,
                probe_allowed: true,
                reason: Some(format!(
                    "circuit_half_open_probe reason={} opened_until_unix={}",
                    snapshot.last_reason.as_deref().unwrap_or("degraded"),
                    snapshot.opened_until_unix.unwrap_or(0)
                )),
            },
        }
    }

    pub(crate) fn snapshots(&self, now_unix: u64) -> Vec<CircuitBreakerProfileReport> {
        self.entries
            .values()
            .map(|entry| entry.snapshot(now_unix))
            .collect()
    }
}

impl Default for ProfileCircuitBreaker {
    fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CircuitBreakerReport {
    pub(crate) schema: String,
    pub(crate) version: String,
    pub(crate) total_records: usize,
    pub(crate) ignored_records: usize,
    pub(crate) profile_count: usize,
    pub(crate) open_count: usize,
    pub(crate) half_open_count: usize,
    pub(crate) rollback_required: bool,
    pub(crate) rollback_reason: Option<String>,
    pub(crate) fallback_strategy: String,
    pub(crate) profiles: Vec<CircuitBreakerProfileReport>,
}

impl CircuitBreakerReport {
    pub(crate) fn from_outcome_jsonl(text: &str, config: &CircuitBreakerConfig) -> Self {
        let mut breaker = ProfileCircuitBreaker::new(config.clone());
        let mut total_records = 0usize;
        let mut ignored_records = 0usize;
        let mut observed_clock = 0u64;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            total_records += 1;
            let Some(sample) = OutcomeSample::from_m3_json(line, "general") else {
                ignored_records += 1;
                continue;
            };
            observed_clock = observed_clock.saturating_add(1);
            let observed_unix = sample.timestamp_unix.unwrap_or(observed_clock);
            observed_clock = observed_clock.max(observed_unix);
            breaker.observe(&sample, observed_unix);
        }
        let profiles = breaker.snapshots(observed_clock);
        let open_count = profiles
            .iter()
            .filter(|profile| profile.state == CircuitBreakerState::Open)
            .count();
        let half_open_count = profiles
            .iter()
            .filter(|profile| profile.state == CircuitBreakerState::HalfOpen)
            .count();
        let rollback_required = open_count > 0 || half_open_count > 0;
        let rollback_reason = rollback_required.then(|| {
            let degraded = profiles
                .iter()
                .filter(|profile| profile.state != CircuitBreakerState::Closed)
                .map(|profile| {
                    format!(
                        "{}:{}:{}",
                        profile.model_id,
                        profile.skill_tag,
                        profile.last_reason.as_deref().unwrap_or("degraded")
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("profile_circuit_breaker_degraded profiles={degraded}")
        });
        Self {
            schema: "norion.profile_circuit_breaker_report.v1".to_owned(),
            version: SCORING_VERSION.to_owned(),
            total_records,
            ignored_records,
            profile_count: profiles.len(),
            open_count,
            half_open_count,
            rollback_required,
            rollback_reason,
            fallback_strategy: if rollback_required {
                "rule-routing|quality-only".to_owned()
            } else {
                "none".to_owned()
            },
            profiles,
        }
    }

    pub(crate) fn json_report(&self) -> String {
        let profiles = self
            .profiles
            .iter()
            .map(CircuitBreakerProfileReport::json_report)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"schema\":{},\"version\":{},\"total_records\":{},\"ignored_records\":{},\"profile_count\":{},\"open_count\":{},\"half_open_count\":{},\"rollback_required\":{},\"rollback_reason\":{},\"fallback_strategy\":{},\"profiles\":[{}]}}",
            json_string(&self.schema),
            json_string(&self.version),
            self.total_records,
            self.ignored_records,
            self.profile_count,
            self.open_count,
            self.half_open_count,
            self.rollback_required,
            option_str_json(self.rollback_reason.as_deref()),
            json_string(&self.fallback_strategy),
            profiles
        )
    }
}

pub(crate) fn option_circuit_breaker_json(report: Option<&CircuitBreakerReport>) -> String {
    report
        .map(CircuitBreakerReport::json_report)
        .unwrap_or_else(|| "null".to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RegressionAggregate {
    pub(crate) quality: f64,
    pub(crate) latency_ms: f64,
    pub(crate) cost: f64,
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
    circuit_breaker: ProfileCircuitBreaker,
    exploration_remaining: u64,
    observed_clock_unix: u64,
}

impl OnlineScorer {
    pub(crate) fn new(config: ScoringConfig) -> Self {
        let exploration_remaining = config.epsilon_budget;
        let circuit_breaker = ProfileCircuitBreaker::new(config.circuit_breaker.clone());
        Self {
            config,
            profiles: BTreeMap::new(),
            circuit_breaker,
            exploration_remaining,
            observed_clock_unix: 0,
        }
    }

    pub(crate) fn update(&mut self, sample: OutcomeSample) -> &ModelProfileScore {
        let key = (sample.model_id.clone(), sample.skill_tag.clone());
        self.observed_clock_unix = self.observed_clock_unix.saturating_add(1);
        let observed_unix = sample.timestamp_unix.unwrap_or(self.observed_clock_unix);
        self.observed_clock_unix = self.observed_clock_unix.max(observed_unix);
        self.circuit_breaker.observe(&sample, observed_unix);
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
        let circuit = self
            .circuit_breaker
            .decision(model_id, skill_tag, self.observed_clock_unix);
        let circuit_penalty = if circuit.selectable { score } else { 0.0 };
        let circuit_reason = circuit.reason.clone();
        let circuit_state = circuit.state.as_str().to_owned();
        let probe_allowed = circuit.probe_allowed;
        RoutingScore {
            model_id: model_id.to_owned(),
            skill_tag: skill_tag.to_owned(),
            score: circuit_penalty,
            explore: false,
            circuit_state,
            circuit_reason,
            probe_allowed,
            reason: format!(
                "version={} success_rate={:.3} reliability={:.3} quality_hint={:.3} latency_component={:.3} cost_component={:.3} cache_hit_rate={:.3} drift_penalty={:.3} failure_streak={} circuit_state={} circuit_reason={} score={:.3}",
                self.config.version,
                profile.success_rate,
                profile.reliability,
                profile.quality_hint,
                latency_component,
                cost_component,
                profile.cache_hit_rate,
                profile.drift_penalty,
                profile.recent_failure_streak,
                circuit.state.as_str(),
                circuit.reason.as_deref().unwrap_or("none"),
                circuit_penalty
            ),
        }
    }

    pub(crate) fn route(
        &mut self,
        candidates: &[CandidateModel],
        skill_tag: &str,
        exploration_roll: Option<f64>,
    ) -> Option<RouteDecision> {
        let mut scores = candidates
            .iter()
            .map(|candidate| self.score_candidate(&candidate.model_id, skill_tag))
            .collect::<Vec<_>>();
        scores.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.model_id.cmp(&right.model_id))
        });
        let explore_requested = self.exploration_requested(exploration_roll.unwrap_or(1.0));
        let explore_index = if explore_requested {
            scores
                .iter()
                .enumerate()
                .find(|(_, score)| score.probe_allowed)
                .map(|(index, _)| index)
                .or_else(|| {
                    scores
                        .iter()
                        .enumerate()
                        .skip(1)
                        .find(|(_, score)| score.circuit_state == "closed")
                        .map(|(index, _)| index)
                })
                .or_else(|| {
                    scores
                        .iter()
                        .enumerate()
                        .find(|(_, score)| score.circuit_state == "closed")
                        .map(|(index, _)| index)
                })
        } else {
            None
        };
        let selected_index = if let Some(index) = explore_index {
            self.exploration_remaining = self.exploration_remaining.saturating_sub(1);
            index
        } else {
            scores
                .iter()
                .enumerate()
                .find(|(_, score)| score.circuit_state == "closed")
                .map(|(index, _)| index)?
        };
        let selected = scores.get_mut(selected_index)?;
        selected.explore = explore_index.is_some();
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

    fn exploration_requested(&self, exploration_roll: f64) -> bool {
        if !self.config.exploration_enabled
            || self.config.epsilon <= 0.0
            || self.exploration_remaining == 0
        {
            return false;
        }
        if exploration_roll <= self.config.epsilon {
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateModel {
    pub(crate) model_id: String,
}

impl CandidateModel {
    pub(crate) fn new(model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
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

fn replay_strategy(line: &str) -> Option<String> {
    json_string_field(line, "strategy").or_else(|| {
        json_object_field(line, "route_decision")
            .and_then(|route| json_string_field(&route, "strategy"))
    })
}

fn degradation_reason(sample: &OutcomeSample, config: &CircuitBreakerConfig) -> Option<String> {
    if !sample.success {
        let error = sample
            .error_kind
            .as_deref()
            .unwrap_or("protocol_error")
            .to_ascii_lowercase();
        if error.contains("timeout") || error.contains("timed out") {
            return Some("timeout".to_owned());
        }
        if error.contains("validation") || error.contains("test") {
            return Some("validation_error".to_owned());
        }
        if error.contains("protocol")
            || error.contains("json")
            || error.contains("parse")
            || error.contains("stream")
        {
            return Some("protocol_error".to_owned());
        }
        return Some("protocol_error".to_owned());
    }
    if sample
        .cost
        .is_some_and(|cost| cost >= config.budget_overrun_threshold)
    {
        return Some("budget_overrun".to_owned());
    }
    if sample
        .quality_hint
        .is_some_and(|quality| quality <= config.quality_drop_threshold)
    {
        return Some("quality_drop".to_owned());
    }
    None
}

fn severe_degradation(sample: &OutcomeSample, reason: &str, config: &CircuitBreakerConfig) -> bool {
    reason == "validation_error"
        || sample.cost.is_some_and(|cost| {
            cost >= config.budget_overrun_threshold * config.severe_budget_multiplier.max(1.0)
        })
        || sample.quality_hint.is_some_and(|quality| quality <= 0.05)
}

fn average_option(values: impl Iterator<Item = f64>) -> f64 {
    let mut total = 0.0;
    let mut count = 0usize;
    for value in values.filter(|value| value.is_finite()) {
        total += value;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
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

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn policy_switch_json(decision: &PolicySwitchDecision) -> String {
    format!(
        "{{\"allow_switch\":{},\"policy_version\":{},\"reason\":{}}}",
        decision.allow_switch,
        json_string(&decision.policy_version),
        json_string(&decision.reason)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(model_id: &str, skill_tag: &str, success: bool) -> OutcomeSample {
        OutcomeSample {
            model_id: model_id.to_owned(),
            skill_tag: skill_tag.to_owned(),
            success,
            error_kind: None,
            latency_ms: Some(1_000.0),
            cost: Some(100.0),
            quality_hint: Some(if success { 0.9 } else { 0.1 }),
            cache_hit: false,
            drift_detected: false,
            timestamp_unix: None,
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
        let candidates = [CandidateModel::new("best"), CandidateModel::new("alt")];

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
    fn circuit_breaker_routes_away_from_open_profile() {
        let config = ScoringConfig {
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 2,
                cooldown_seconds: 100,
                ..CircuitBreakerConfig::default()
            },
            ..ScoringConfig::default()
        };
        let mut scorer = OnlineScorer::new(config);
        let mut failing = sample("degraded", "review", false);
        failing.error_kind = Some("timeout".to_owned());
        failing.timestamp_unix = Some(10);
        scorer.update(failing.clone());
        failing.timestamp_unix = Some(11);
        scorer.update(failing);
        let mut healthy = sample("healthy", "review", true);
        healthy.timestamp_unix = Some(12);
        scorer.update(healthy);

        let decision = scorer
            .route(
                &[
                    CandidateModel::new("degraded"),
                    CandidateModel::new("healthy"),
                ],
                "review",
                Some(1.0),
            )
            .unwrap();
        let degraded_score = decision
            .scores
            .iter()
            .find(|score| score.model_id == "degraded")
            .unwrap();

        assert_eq!(decision.selected_model_id, "healthy");
        assert_eq!(degraded_score.circuit_state, "open");
        assert!(degraded_score.reason.contains("timeout"));
    }

    #[test]
    fn circuit_breaker_half_open_uses_bounded_exploration_probe() {
        let config = ScoringConfig {
            exploration_enabled: true,
            epsilon: 1.0,
            epsilon_budget: 1,
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 1,
                cooldown_seconds: 10,
                ..CircuitBreakerConfig::default()
            },
            ..ScoringConfig::default()
        };
        let mut scorer = OnlineScorer::new(config);
        let mut failing = sample("degraded", "review", false);
        failing.error_kind = Some("protocol_json_parse".to_owned());
        failing.timestamp_unix = Some(10);
        scorer.update(failing);
        let mut healthy = sample("healthy", "review", true);
        healthy.timestamp_unix = Some(21);
        scorer.update(healthy);

        let decision = scorer
            .route(
                &[
                    CandidateModel::new("degraded"),
                    CandidateModel::new("healthy"),
                ],
                "review",
                Some(0.0),
            )
            .unwrap();

        assert_eq!(decision.selected_model_id, "degraded");
        assert!(decision.selected_by_exploration);
        assert_eq!(scorer.exploration_remaining(), 0);
        assert!(
            decision
                .scores
                .iter()
                .any(|score| score.model_id == "degraded"
                    && score.circuit_state == "half_open"
                    && score.probe_allowed)
        );
    }

    #[test]
    fn circuit_breaker_successful_probe_closes_and_recovers() {
        let config = ScoringConfig {
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 1,
                cooldown_seconds: 5,
                ..CircuitBreakerConfig::default()
            },
            ..ScoringConfig::default()
        };
        let mut breaker = ProfileCircuitBreaker::new(config.circuit_breaker.clone());
        let mut failing = sample("probe", "summary", false);
        failing.error_kind = Some("timeout".to_owned());
        breaker.observe(&failing, 10);
        let mut recovered = sample("probe", "summary", true);
        recovered.timestamp_unix = Some(16);
        breaker.observe(&recovered, 16);

        let decision = breaker.decision("probe", "summary", 16);
        let snapshot = breaker.snapshots(16).pop().unwrap();

        assert_eq!(decision.state, CircuitBreakerState::Closed);
        assert!(decision.selectable);
        assert_eq!(snapshot.total_recovered, 1);
    }

    #[test]
    fn circuit_breaker_report_marks_rollback_and_fallback_strategy() {
        let report = CircuitBreakerReport::from_outcome_jsonl(
            "{\"strategy\":\"profile-scoring.v1\",\"chosen_model\":\"bad\",\"task_kind\":\"review\",\"ok\":false,\"error_kind\":\"timeout\",\"timestamp_unix\":10}\n\
{\"strategy\":\"profile-scoring.v1\",\"chosen_model\":\"bad\",\"task_kind\":\"review\",\"ok\":false,\"error_kind\":\"timeout\",\"timestamp_unix\":11}\n\
{\"strategy\":\"profile-scoring.v1\",\"chosen_model\":\"bad\",\"task_kind\":\"review\",\"ok\":false,\"error_kind\":\"timeout\",\"timestamp_unix\":12}\n\
{\"strategy\":\"profile-scoring.v1\",\"chosen_model\":\"good\",\"task_kind\":\"review\",\"ok\":true,\"quality_score\":0.92,\"timestamp_unix\":12}\n",
            &CircuitBreakerConfig::default(),
        );
        let json = report.json_report();

        assert_eq!(report.total_records, 4);
        assert_eq!(report.open_count, 1);
        assert!(report.rollback_required);
        assert_eq!(report.fallback_strategy, "rule-routing|quality-only");
        assert!(json.contains("\"state\":\"open\""));
        assert!(json.contains("\"last_reason\":\"timeout\""));
        assert!(json.contains("\"rollback_required\":true"));
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
    fn offline_replay_allows_profile_switch_from_outcome_log() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            &replay_log(
                &[(true, 0.70, 1400, 600), (true, 0.72, 1300, 580)],
                &[(true, 0.84, 900, 620, false), (true, 0.86, 850, 610, false)],
            ),
            2,
            &ScoringConfig::default(),
        );

        assert_eq!(report.total_records, 4);
        assert_eq!(report.rule_baseline.sample_count, 2);
        assert_eq!(report.profile_candidate.sample_count, 2);
        assert!(report.offline_regression.passed);
        assert!(report.switch_decision.allow_switch);
        assert!(report.json_report().contains("\"allow_switch\":true"));
    }

    #[test]
    fn offline_replay_blocks_quality_regression() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            &replay_log(&[(true, 0.90, 1000, 100)], &[(true, 0.80, 900, 90, false)]),
            1,
            &ScoringConfig::default(),
        );

        assert!(!report.offline_regression.passed);
        assert!(!report.switch_decision.allow_switch);
        assert!(report.switch_decision.reason.contains("quality_delta"));
    }

    #[test]
    fn offline_replay_blocks_latency_regression() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            &replay_log(
                &[(true, 0.90, 1000, 100)],
                &[(true, 0.90, 1300, 120, false)],
            ),
            1,
            &ScoringConfig::default(),
        );

        assert!(!report.offline_regression.passed);
        assert!(report.switch_decision.reason.contains("latency_delta_ms"));
    }

    #[test]
    fn offline_replay_blocks_cost_regression() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            &replay_log(
                &[(true, 0.90, 1000, 100)],
                &[(true, 0.90, 1100, 140, false)],
            ),
            1,
            &ScoringConfig::default(),
        );

        assert!(!report.offline_regression.passed);
        assert!(report.switch_decision.reason.contains("cost_delta"));
    }

    #[test]
    fn offline_replay_blocks_insufficient_samples_and_reports_drift() {
        let report = OfflineReplayReport::from_outcome_jsonl(
            &replay_log(&[(true, 0.90, 1000, 100)], &[(true, 0.95, 800, 80, true)]),
            2,
            &ScoringConfig::default(),
        );

        assert_eq!(report.profile_candidate.drift_count, 1);
        assert!(report.profile_candidate.drift_penalty > 0.0);
        assert!(!report.switch_decision.allow_switch);
        assert!(
            report
                .switch_decision
                .reason
                .contains("insufficient_samples")
        );
        assert!(report.json_report().contains("\"drift_penalty\""));
    }

    #[test]
    fn explanation_json_contains_version_and_candidate_reasons() {
        let mut scorer = OnlineScorer::new(ScoringConfig::default());
        scorer.update(sample("quality", "code", true));
        let decision = scorer
            .route(&[CandidateModel::new("quality")], "code", None)
            .unwrap();
        let explanation = scorer.explanation_json(&decision);

        assert!(explanation.contains(SCORING_VERSION));
        assert!(explanation.contains("candidate_reasons"));
        assert!(explanation.contains("profile_route_best"));
    }

    fn replay_log(
        rule: &[(bool, f64, u64, u64)],
        profile: &[(bool, f64, u64, u64, bool)],
    ) -> String {
        let mut lines = Vec::new();
        for (index, (ok, quality, latency, cost)) in rule.iter().enumerate() {
            lines.push(format!(
                "{{\"strategy\":\"single\",\"chosen_model\":\"rule-{index}\",\"task_kind\":\"review\",\"ok\":{},\"latency_ms\":{},\"cost_estimate_micro_usd\":{},\"quality_score\":{:.3}}}",
                ok, latency, cost, quality
            ));
        }
        for (index, (ok, quality, latency, cost, drift)) in profile.iter().enumerate() {
            lines.push(format!(
                "{{\"strategy\":\"profile-scoring.v1\",\"chosen_model\":\"profile-{index}\",\"task_kind\":\"review\",\"ok\":{},\"latency_ms\":{},\"cost_estimate_micro_usd\":{},\"quality_score\":{:.3},\"drift_detected\":{}}}",
                ok, latency, cost, quality, drift
            ));
        }
        lines.join("\n")
    }
}
