use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::model_service::json::{
    json_bool_field, json_string_field, json_u64_field, service_json_string,
};
use crate::model_service::response::{
    ModelPoolMetricsSnapshotView, ModelPoolMetricsView, ModelPoolWorkerMetricsView,
    ModelPoolWorkerQuarantineView, ModelPoolWorkerView, model_pool_worker_id,
};
use crate::path_utils::ensure_parent_dir;

use super::config::WorkerSpec;

const MAX_LATENCY_SAMPLES: usize = 256;
const OUTCOMES_PATH_ENV: &str = "NORION_MODEL_POOL_OUTCOMES_PATH";
const COOLDOWN_SECS_ENV: &str = "NORION_MODEL_POOL_FAILURE_COOLDOWN_SECS";
#[cfg(not(test))]
const DEFAULT_OUTCOMES_PATH: &str = "target/evolution/model-pool-worker-outcomes.jsonl";
const DEFAULT_COOLDOWN_SECS: u64 = 60;

static MODEL_POOL_METRICS: OnceLock<Mutex<ModelPoolMetricsState>> = OnceLock::new();
#[cfg(test)]
static TEST_METRICS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Default)]
struct ModelPoolMetricsState {
    total: ModelPoolMetricCounters,
    workers: BTreeMap<String, ModelPoolMetricCounters>,
    worker_outcomes: BTreeMap<String, WorkerOutcome>,
    half_open_worker_ids: BTreeSet<String>,
}

#[derive(Default)]
struct ModelPoolMetricCounters {
    route_count: u64,
    selected_count: u64,
    blocked_count: u64,
    in_flight: u64,
    success_count: u64,
    failure_count: u64,
    latency_total_ms: u128,
    latency_sample_count: u64,
    latency_samples_ms: VecDeque<u64>,
}

pub(super) struct ModelPoolCallMetricsGuard {
    role: String,
    worker_id: String,
    isolation: WorkerIsolationConfig,
    half_open_state_key: Option<String>,
    started_at: Instant,
    finished: bool,
}

#[derive(Debug, Clone)]
pub(super) struct WorkerIsolationConfig {
    outcomes_path: PathBuf,
    cooldown_secs: u64,
}

#[derive(Debug, Clone)]
struct WorkerOutcome {
    consecutive_failures: u64,
    observed_unix: u64,
    reason: Option<String>,
    persisted: bool,
}

impl WorkerIsolationConfig {
    pub(super) fn from_env() -> Self {
        Self {
            outcomes_path: std::env::var(OUTCOMES_PATH_ENV)
                .ok()
                .filter(|path| !path.trim().is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(default_outcomes_path),
            cooldown_secs: std::env::var(COOLDOWN_SECS_ENV)
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(DEFAULT_COOLDOWN_SECS)
                .max(1),
        }
    }

    #[cfg(test)]
    pub(super) fn new(outcomes_path: PathBuf, cooldown_secs: u64) -> Self {
        Self {
            outcomes_path,
            cooldown_secs: cooldown_secs.max(1),
        }
    }
}

fn default_outcomes_path() -> PathBuf {
    #[cfg(test)]
    {
        std::env::temp_dir().join(format!(
            "rust-norion-model-pool-worker-outcomes-{}.jsonl",
            std::process::id()
        ))
    }
    #[cfg(not(test))]
    {
        PathBuf::from(DEFAULT_OUTCOMES_PATH)
    }
}

pub(super) fn record_route_result(selected_role: Option<&str>, route_allowed: bool) {
    with_metrics_state(|state| {
        state.total.route_count = state.total.route_count.saturating_add(1);
        if route_allowed {
            state.total.selected_count = state.total.selected_count.saturating_add(1);
        } else {
            state.total.blocked_count = state.total.blocked_count.saturating_add(1);
        }

        if let Some(role) = selected_role {
            let worker = state.workers.entry(role.to_owned()).or_default();
            worker.route_count = worker.route_count.saturating_add(1);
            if route_allowed {
                worker.selected_count = worker.selected_count.saturating_add(1);
            } else {
                worker.blocked_count = worker.blocked_count.saturating_add(1);
            }
        }
    })
}

pub(super) fn try_begin_worker_call(
    worker: &ModelPoolWorkerView,
    isolation: &WorkerIsolationConfig,
    now_unix: u64,
) -> Option<ModelPoolCallMetricsGuard> {
    let worker_id = model_pool_worker_id(&worker.base_url);
    let state_key = worker_outcome_state_key(isolation, &worker_id);
    let half_open_state_key = with_metrics_state(|state| {
        let outcome = state.worker_outcomes.get(&state_key);
        if outcome.is_some_and(|outcome| worker_outcome_is_active(outcome, isolation, now_unix)) {
            return None;
        }
        let half_open = outcome.is_some_and(|outcome| outcome.consecutive_failures > 0);
        if half_open && !state.half_open_worker_ids.insert(state_key.clone()) {
            return None;
        }
        state.total.in_flight = state.total.in_flight.saturating_add(1);
        let counters = state.workers.entry(worker.role.clone()).or_default();
        counters.in_flight = counters.in_flight.saturating_add(1);
        Some(half_open.then_some(state_key))
    })?;

    Some(ModelPoolCallMetricsGuard {
        role: worker.role.clone(),
        worker_id,
        isolation: isolation.clone(),
        half_open_state_key,
        started_at: Instant::now(),
        finished: false,
    })
}

#[cfg(test)]
pub(super) fn begin_worker_call(
    worker: &ModelPoolWorkerView,
    isolation: &WorkerIsolationConfig,
) -> ModelPoolCallMetricsGuard {
    try_begin_worker_call(worker, isolation, unix_now()).expect("worker should accept test call")
}

pub(super) fn worker_quarantines(
    specs: &[WorkerSpec],
    isolation: &WorkerIsolationConfig,
    now_unix: u64,
) -> BTreeMap<String, ModelPoolWorkerQuarantineView> {
    let disk_outcomes = read_worker_outcomes(&isolation.outcomes_path);
    with_metrics_state(|state| {
        for (worker_id, outcome) in disk_outcomes {
            let state_key = worker_outcome_state_key(isolation, &worker_id);
            let should_replace = state
                .worker_outcomes
                .get(&state_key)
                .is_none_or(|current| outcome.observed_unix > current.observed_unix);
            if should_replace {
                state.worker_outcomes.insert(state_key, outcome);
            }
        }

        specs
            .iter()
            .filter_map(|spec| {
                let worker_id = model_pool_worker_id(&spec.base_url);
                let outcome = state
                    .worker_outcomes
                    .get(&worker_outcome_state_key(isolation, &worker_id))?;
                let retry_after_unix = outcome
                    .observed_unix
                    .saturating_add(isolation.cooldown_secs);
                worker_outcome_is_active(outcome, isolation, now_unix).then(|| {
                    (
                        worker_id,
                        ModelPoolWorkerQuarantineView {
                            consecutive_failures: outcome.consecutive_failures,
                            retry_after_unix,
                            reason: outcome
                                .reason
                                .clone()
                                .unwrap_or_else(|| "worker_failure".to_owned()),
                            persisted: outcome.persisted,
                        },
                    )
                })
            })
            .collect()
    })
}

fn worker_outcome_is_active(
    outcome: &WorkerOutcome,
    isolation: &WorkerIsolationConfig,
    now_unix: u64,
) -> bool {
    outcome.consecutive_failures > 0
        && now_unix
            < outcome
                .observed_unix
                .saturating_add(isolation.cooldown_secs)
}

pub(super) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(super) fn snapshot() -> ModelPoolMetricsSnapshotView {
    with_metrics_state(|state| ModelPoolMetricsSnapshotView {
        route_metrics: state.total.view(),
        worker_metrics: state
            .workers
            .iter()
            .map(|(role, metrics)| ModelPoolWorkerMetricsView {
                role: role.clone(),
                metrics: metrics.view(),
            })
            .collect(),
    })
}

#[cfg(test)]
pub(super) fn reset() {
    with_metrics_state(|state| {
        *state = ModelPoolMetricsState::default();
    });
}

#[cfg(test)]
pub(super) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    TEST_METRICS_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl ModelPoolCallMetricsGuard {
    #[cfg(test)]
    pub(super) fn finish(mut self, success: bool) {
        self.finished = true;
        record_worker_call_finish(&self.role, success, self.started_at.elapsed());
        release_half_open_claim(self.half_open_state_key.as_deref());
    }

    pub(super) fn finish_with_reason_at(
        mut self,
        success: bool,
        reason: Option<&str>,
        observed_unix: u64,
    ) -> bool {
        self.finished = true;
        record_worker_call_finish(&self.role, success, self.started_at.elapsed());
        if success || reason.is_some() {
            let persisted = record_worker_outcome(
                &self.isolation,
                &self.worker_id,
                &self.role,
                success,
                reason,
                observed_unix,
            );
            release_half_open_claim(self.half_open_state_key.as_deref());
            return persisted;
        }
        release_half_open_claim(self.half_open_state_key.as_deref());
        false
    }
}

impl Drop for ModelPoolCallMetricsGuard {
    fn drop(&mut self) {
        if !self.finished {
            record_worker_call_abandoned(&self.role);
            release_half_open_claim(self.half_open_state_key.as_deref());
        }
    }
}

impl ModelPoolMetricCounters {
    fn view(&self) -> ModelPoolMetricsView {
        ModelPoolMetricsView {
            route_count: self.route_count,
            selected_count: self.selected_count,
            blocked_count: self.blocked_count,
            in_flight: self.in_flight,
            queued_count: 0,
            lease_wait_ms: Some(0),
            lease_wait_p95_ms: Some(0),
            success_count: self.success_count,
            failure_count: self.failure_count,
            avg_latency_ms: average_latency_ms(self.latency_total_ms, self.latency_sample_count),
            latency_p50_ms: percentile_latency_ms(&self.latency_samples_ms, 50),
            latency_p95_ms: percentile_latency_ms(&self.latency_samples_ms, 95),
        }
    }
}

fn record_worker_call_finish(role: &str, success: bool, latency: Duration) {
    with_metrics_state(|state| {
        record_call_finish_on_counters(&mut state.total, success, latency);
        let worker = state.workers.entry(role.to_owned()).or_default();
        record_call_finish_on_counters(worker, success, latency);
    });
}

fn record_worker_call_abandoned(role: &str) {
    with_metrics_state(|state| {
        state.total.in_flight = state.total.in_flight.saturating_sub(1);
        let worker = state.workers.entry(role.to_owned()).or_default();
        worker.in_flight = worker.in_flight.saturating_sub(1);
    });
}

fn release_half_open_claim(state_key: Option<&str>) {
    let Some(state_key) = state_key else {
        return;
    };
    with_metrics_state(|state| {
        state.half_open_worker_ids.remove(state_key);
    });
}

fn record_worker_outcome(
    isolation: &WorkerIsolationConfig,
    worker_id: &str,
    role: &str,
    success: bool,
    reason: Option<&str>,
    observed_unix: u64,
) -> bool {
    with_metrics_state(|state| {
        let state_key = worker_outcome_state_key(isolation, worker_id);
        let previous_failures = state
            .worker_outcomes
            .get(&state_key)
            .map(|outcome| outcome.consecutive_failures)
            .unwrap_or(0);
        if success && previous_failures == 0 {
            return false;
        }
        let consecutive_failures = if success {
            0
        } else {
            previous_failures.saturating_add(1)
        };
        let persisted = persist_worker_outcome(
            &isolation.outcomes_path,
            worker_id,
            role,
            success,
            consecutive_failures,
            reason,
            observed_unix,
        )
        .map(|()| true)
        .unwrap_or_else(|error| {
            eprintln!(
                "model_pool_worker_outcome_persist_failed path={} error={error}",
                isolation.outcomes_path.display()
            );
            false
        });
        state.worker_outcomes.insert(
            state_key,
            WorkerOutcome {
                consecutive_failures,
                observed_unix,
                reason: reason.map(str::to_owned),
                persisted,
            },
        );
        persisted
    })
}

fn persist_worker_outcome(
    path: &Path,
    worker_id: &str,
    role: &str,
    success: bool,
    consecutive_failures: u64,
    reason: Option<&str>,
    observed_unix: u64,
) -> std::io::Result<()> {
    ensure_parent_dir(path)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(
        file,
        "{{\"observed_unix\":{},\"worker_id\":{},\"role\":{},\"ok\":{},\"consecutive_failures\":{},\"reason\":{}}}",
        observed_unix,
        service_json_string(worker_id),
        service_json_string(role),
        success,
        consecutive_failures,
        reason
            .map(service_json_string)
            .unwrap_or_else(|| "null".to_owned())
    )
}

fn read_worker_outcomes(path: &Path) -> BTreeMap<String, WorkerOutcome> {
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let mut outcomes = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('{') || !line.ends_with('}') {
            continue;
        }
        let Some(worker_id) = json_string_field(line, "worker_id") else {
            continue;
        };
        let Some(success) = json_bool_field(line, "ok") else {
            continue;
        };
        let Some(observed_unix) = json_u64_field(line, "observed_unix") else {
            continue;
        };
        let Some(recorded_failures) = json_u64_field(line, "consecutive_failures") else {
            continue;
        };
        let consecutive_failures = if success { 0 } else { recorded_failures.max(1) };
        outcomes.insert(
            worker_id,
            WorkerOutcome {
                consecutive_failures,
                observed_unix,
                reason: json_string_field(line, "reason"),
                persisted: true,
            },
        );
    }
    outcomes
}

fn worker_outcome_state_key(isolation: &WorkerIsolationConfig, worker_id: &str) -> String {
    format!("{}\0{worker_id}", isolation.outcomes_path.to_string_lossy())
}

fn record_call_finish_on_counters(
    counters: &mut ModelPoolMetricCounters,
    success: bool,
    latency: Duration,
) {
    counters.in_flight = counters.in_flight.saturating_sub(1);
    if success {
        counters.success_count = counters.success_count.saturating_add(1);
    } else {
        counters.failure_count = counters.failure_count.saturating_add(1);
    }
    counters.latency_total_ms = counters
        .latency_total_ms
        .saturating_add(latency.as_millis());
    counters.latency_sample_count = counters.latency_sample_count.saturating_add(1);
    record_latency_sample(counters, latency);
}

fn average_latency_ms(total_ms: u128, samples: u64) -> Option<u64> {
    if samples == 0 {
        return None;
    }
    let average = total_ms / u128::from(samples);
    Some(average.min(u128::from(u64::MAX)) as u64)
}

fn record_latency_sample(counters: &mut ModelPoolMetricCounters, latency: Duration) {
    let latency_ms = latency.as_millis().min(u128::from(u64::MAX)) as u64;
    if counters.latency_samples_ms.len() == MAX_LATENCY_SAMPLES {
        counters.latency_samples_ms.pop_front();
    }
    counters.latency_samples_ms.push_back(latency_ms);
}

fn percentile_latency_ms(samples: &VecDeque<u64>, percentile: usize) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.iter().copied().collect::<Vec<_>>();
    sorted.sort_unstable();
    let rank = percentile
        .saturating_mul(sorted.len())
        .div_ceil(100)
        .saturating_sub(1)
        .min(sorted.len().saturating_sub(1));
    sorted.get(rank).copied()
}

fn with_metrics_state<R>(action: impl FnOnce(&mut ModelPoolMetricsState) -> R) -> R {
    let metrics = MODEL_POOL_METRICS.get_or_init(|| Mutex::new(ModelPoolMetricsState::default()));
    match metrics.lock() {
        Ok(mut state) => action(&mut state),
        Err(poisoned) => {
            let mut state = poisoned.into_inner();
            action(&mut state)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_route_and_worker_call_metrics() {
        let _guard = test_guard();
        reset();
        let isolation = WorkerIsolationConfig::new(
            std::env::temp_dir().join(format!(
                "rust-norion-model-pool-metrics-{}.jsonl",
                std::process::id()
            )),
            60,
        );
        let worker = ModelPoolWorkerView {
            role: "review".to_owned(),
            port: 8688,
            base_url: "http://127.0.0.1:8688".to_owned(),
            enabled_by_default: true,
            model_class: "test".to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: 8192,
            default_max_tokens: 1536,
            low_priority: true,
            reachable: true,
            model: Some("test".to_owned()),
            context_window: Some(8192),
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
            input_cost_per_1k_micro_usd: None,
            output_cost_per_1k_micro_usd: None,
            remaining_budget_micro_usd: None,
            error: None,
            quarantine: None,
        };

        record_route_result(Some("review"), true);
        let call = begin_worker_call(&worker, &isolation);
        call.finish(true);

        let snapshot = snapshot();

        assert_eq!(snapshot.route_metrics.route_count, 1);
        assert_eq!(snapshot.route_metrics.selected_count, 1);
        assert_eq!(snapshot.route_metrics.success_count, 1);
        assert_eq!(snapshot.route_metrics.in_flight, 0);
        assert_eq!(snapshot.route_metrics.queued_count, 0);
        assert_eq!(snapshot.route_metrics.lease_wait_ms, Some(0));
        assert_eq!(snapshot.route_metrics.lease_wait_p95_ms, Some(0));
        assert!(snapshot.route_metrics.latency_p50_ms.is_some());
        assert!(snapshot.route_metrics.latency_p95_ms.is_some());
        let review = snapshot
            .worker_metrics
            .iter()
            .find(|metrics| metrics.role == "review")
            .expect("review metrics should be present");
        assert_eq!(review.metrics.route_count, 1);
        assert_eq!(review.metrics.selected_count, 1);
        assert_eq!(review.metrics.success_count, 1);
        assert_eq!(review.metrics.queued_count, 0);
        assert_eq!(review.metrics.lease_wait_ms, Some(0));
        assert_eq!(review.metrics.lease_wait_p95_ms, Some(0));
        assert!(review.metrics.latency_p50_ms.is_some());
        assert!(review.metrics.latency_p95_ms.is_some());
    }

    #[test]
    fn records_blocked_routes_without_worker_selection() {
        let _guard = test_guard();
        reset();

        record_route_result(None, false);

        let snapshot = snapshot();

        assert_eq!(snapshot.route_metrics.route_count, 1);
        assert_eq!(snapshot.route_metrics.blocked_count, 1);
        assert!(snapshot.worker_metrics.is_empty());
    }

    #[test]
    fn computes_latency_percentiles_from_bounded_samples() {
        let mut counters = ModelPoolMetricCounters::default();
        for latency_ms in [10, 20, 100] {
            record_call_finish_on_counters(&mut counters, true, Duration::from_millis(latency_ms));
        }

        let view = counters.view();

        assert_eq!(view.success_count, 3);
        assert_eq!(view.avg_latency_ms, Some(43));
        assert_eq!(view.latency_p50_ms, Some(20));
        assert_eq!(view.latency_p95_ms, Some(100));
        assert_eq!(view.queued_count, 0);
        assert_eq!(view.lease_wait_ms, Some(0));
        assert_eq!(view.lease_wait_p95_ms, Some(0));
    }

    #[test]
    fn outcome_reader_ignores_truncated_records() {
        let path = std::env::temp_dir().join(format!(
            "rust-norion-model-pool-truncated-{}.jsonl",
            std::process::id()
        ));
        fs::write(
            &path,
            concat!(
                "{\"observed_unix\":10,\"worker_id\":\"model-pool.v1:http://127.0.0.1:8688\",\"role\":\"review\",\"ok\":false,\"consecutive_failures\":1,\"reason\":\"transport\"}\n",
                "{\"observed_unix\":11,\"worker_id\":\"model-pool.v1:http://127.0.0.1:8689\",\"role\":\"router\",\"ok\":false"
            ),
        )
        .unwrap();

        let outcomes = read_worker_outcomes(&path);
        let _ = fs::remove_file(path);

        assert_eq!(outcomes.len(), 1);
        assert!(outcomes.contains_key("model-pool.v1:http://127.0.0.1:8688"));
    }
}
