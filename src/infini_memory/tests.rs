use super::*;
use crate::kv_cache::{MemoryEntry, MemoryMatch};

#[test]
fn active_matches_fill_local_window() {
    let planner = InfiniMemoryPlanner::with_limits(1, 4);
    let entries = vec![entry(1, "rust router", 0.7), entry(2, "long memory", 1.3)];
    let matches = vec![memory_match(1, "rust router", 0.9, 0.7)];

    let plan = planner.plan(&entries, &matches);

    assert_eq!(plan.local_window().len(), 1);
    assert_eq!(plan.local_window()[0].id, 1);
    assert_eq!(plan.counts().local_window, 1);
}

#[test]
fn strong_non_active_memory_goes_global() {
    let planner = InfiniMemoryPlanner::with_limits(1, 4);
    let entries = vec![
        entry(1, "current query", 0.8),
        entry(2, "durable global memory", 2.2),
    ];
    let matches = vec![memory_match(1, "current query", 0.9, 0.8)];

    let plan = planner.plan(&entries, &matches);

    assert!(plan.global_memory().iter().any(|item| item.id == 2));
}

#[test]
fn weak_memory_is_sparse_skipped() {
    let planner = InfiniMemoryPlanner::with_limits(1, 4);
    let mut weak = entry(9, "weak stale memory", 0.02);
    weak.hits = 0;
    weak.failures = 3;
    weak.last_score = 0.0;
    weak.last_access = 0;
    let entries = vec![weak];
    let plan = planner.plan(&entries, &[]);

    assert!(plan.global_memory().is_empty());
    assert_eq!(plan.skipped()[0].id, 9);
}

#[test]
fn redundant_global_memory_is_skipped() {
    let planner = InfiniMemoryPlanner {
        redundancy_threshold: 0.5,
        ..InfiniMemoryPlanner::with_limits(1, 4)
    };
    let entries = vec![
        entry(1, "rust router adaptive memory", 2.0),
        entry(2, "rust router adaptive memory copy", 1.9),
    ];
    let plan = planner.plan(&entries, &[]);

    assert_eq!(plan.global_memory().len(), 1);
    assert_eq!(plan.skipped().len(), 1);
    assert!(
        plan.skipped()
            .iter()
            .any(|item| item.reason.contains("redundant"))
    );
}

#[test]
fn local_token_budget_skips_overflow() {
    let planner = InfiniMemoryPlanner::with_limits(4, 4).with_token_budgets(3, 32);
    let entries = vec![
        entry(1, "short local", 0.8),
        entry(2, "very long local memory key", 0.8),
    ];
    let matches = vec![
        memory_match(1, "short local", 0.9, 0.8),
        memory_match(2, "very long local memory key", 0.88, 0.8),
    ];

    let plan = planner.plan(&entries, &matches);

    assert_eq!(plan.counts().local_tokens, 2);
    assert!(
        plan.skipped()
            .iter()
            .any(|item| item.reason.contains("local_token_budget"))
    );
}

#[test]
fn global_token_budget_skips_overflow() {
    let planner = InfiniMemoryPlanner::with_limits(1, 4).with_token_budgets(16, 3);
    let entries = vec![
        entry(1, "compact global", 2.2),
        entry(2, "very long durable global memory", 2.1),
    ];

    let plan = planner.plan(&entries, &[]);

    assert_eq!(plan.counts().global_tokens, 2);
    assert!(
        plan.skipped()
            .iter()
            .any(|item| item.reason.contains("global_token_budget"))
    );
}

fn entry(id: u64, key: &str, strength: f32) -> MemoryEntry {
    MemoryEntry {
        id,
        key: key.to_owned(),
        vector: vec![id as f32, strength],
        strength,
        hits: 2,
        failures: 0,
        last_score: 0.9,
        created_at: id,
        last_access: id + 1,
    }
}

fn memory_match(id: u64, key: &str, similarity: f32, strength: f32) -> MemoryMatch {
    MemoryMatch {
        id,
        key: key.to_owned(),
        similarity,
        strength,
        vector: vec![similarity, strength],
    }
}
