use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::model_service::response::{
    ModelPoolMetricsSnapshotView, ModelPoolMetricsView, ModelPoolWorkerMetricsView,
};

const MAX_LATENCY_SAMPLES: usize = 256;

static MODEL_POOL_METRICS: OnceLock<Mutex<ModelPoolMetricsState>> = OnceLock::new();

#[derive(Default)]
struct ModelPoolMetricsState {
    total: ModelPoolMetricCounters,
    workers: BTreeMap<String, ModelPoolMetricCounters>,
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
    started_at: Instant,
    finished: bool,
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
    });
}

pub(super) fn begin_worker_call(role: &str) -> ModelPoolCallMetricsGuard {
    with_metrics_state(|state| {
        state.total.in_flight = state.total.in_flight.saturating_add(1);
        let worker = state.workers.entry(role.to_owned()).or_default();
        worker.in_flight = worker.in_flight.saturating_add(1);
    });

    ModelPoolCallMetricsGuard {
        role: role.to_owned(),
        started_at: Instant::now(),
        finished: false,
    }
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

impl ModelPoolCallMetricsGuard {
    pub(super) fn finish(mut self, success: bool) {
        self.finished = true;
        record_worker_call_finish(&self.role, success, self.started_at.elapsed());
    }
}

impl Drop for ModelPoolCallMetricsGuard {
    fn drop(&mut self) {
        if !self.finished {
            record_worker_call_abandoned(&self.role);
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
    use std::sync::{MutexGuard, PoisonError};

    static TEST_METRICS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn test_metrics_guard() -> MutexGuard<'static, ()> {
        TEST_METRICS_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    #[test]
    fn records_route_and_worker_call_metrics() {
        let _guard = test_metrics_guard();
        reset();

        record_route_result(Some("review"), true);
        let call = begin_worker_call("review");
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
        let _guard = test_metrics_guard();
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
}
