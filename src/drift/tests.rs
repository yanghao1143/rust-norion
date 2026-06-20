use super::*;
use crate::router::{GenerationMetrics, RouteBudget};

#[test]
fn stable_high_quality_output_can_write_memory_and_runtime_kv() {
    let report = DriftGuard::new().evaluate(input(0.86, 0, 8.0, 0.72, 1));

    assert_eq!(report.severity, DriftSeverity::Watch);
    assert!(report.allow_memory_write);
    assert!(report.allow_runtime_kv_write);
    assert!(!report.rollback_adaptive);
}

#[test]
fn contradiction_blocks_memory_without_rollback() {
    let report = DriftGuard::new().evaluate(input(0.72, 1, 9.0, 0.72, 1));

    assert_eq!(report.severity, DriftSeverity::Block);
    assert!(!report.allow_memory_write);
    assert!(!report.allow_runtime_kv_write);
    assert!(!report.rollback_adaptive);
}

#[test]
fn severe_low_quality_rolls_back_adaptive_state() {
    let report = DriftGuard::new().evaluate(input(0.22, 0, 10.0, 0.20, 0));

    assert_eq!(report.severity, DriftSeverity::Rollback);
    assert!(!report.allow_memory_write);
    assert!(report.rollback_adaptive);
}

#[test]
fn fast_path_watch_holds_runtime_kv_but_keeps_memory_write() {
    let mut input = input(0.68, 0, 9.0, 0.70, 1);
    input.route_budget.attention_tokens = 0;
    input.route_budget.fast_tokens = 12;
    input.route_budget.attention_fraction = 0.0;

    let report = DriftGuard::new().evaluate(input);

    assert_eq!(report.severity, DriftSeverity::Watch);
    assert!(report.allow_memory_write);
    assert!(!report.allow_runtime_kv_write);
    assert!(
        report
            .notes
            .iter()
            .any(|note| note == "route:fast_path_watch")
    );
}

fn input(
    quality: f32,
    contradiction_count: usize,
    perplexity: f32,
    semantic_consistency: f32,
    exported_runtime_kv_blocks: usize,
) -> DriftInput {
    DriftInput {
        quality,
        contradiction_count,
        metrics: GenerationMetrics {
            perplexity,
            semantic_consistency,
            contradiction_count,
            token_count: 32,
        },
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        used_memories: 1,
        exported_runtime_kv_blocks,
        stream_windows: 2,
    }
}
