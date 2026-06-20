use rust_norion::{
    MemoryCompactionPolicy, MemoryRetentionPolicy, NoironEngine, RecursiveScheduler,
};

use crate::Args;

pub(crate) fn configure_engine(engine: &mut NoironEngine, args: &Args) {
    let hardware_snapshot = args.hardware_snapshot();
    engine.recursive_scheduler = RecursiveScheduler::new(
        args.native_window_tokens,
        args.chunk_tokens,
        args.chunk_overlap_tokens,
        args.merge_fan_in,
    );
    engine.set_auto_replay_limit(args.auto_replay_limit);
    engine.set_hardware_snapshot(hardware_snapshot);
    let governance_plan = engine.hardware_allocator.memory_governance_plan(
        hardware_snapshot,
        engine.memory_retention_policy,
        engine.memory_compaction_policy.clone(),
    );
    engine.set_memory_retention_policy(memory_retention_policy_from_args(
        governance_plan.retention_policy,
        args,
    ));
    engine.set_memory_compaction_policy(memory_compaction_policy_from_args(
        governance_plan.compaction_policy,
        args,
    ));
}

fn memory_retention_policy_from_args(
    mut policy: MemoryRetentionPolicy,
    args: &Args,
) -> MemoryRetentionPolicy {
    if let Some(value) = args.retention_stale_after {
        policy.stale_after = value.max(1);
    }
    if let Some(value) = args.retention_decay_rate {
        policy.decay_rate = value.clamp(0.0, 0.95);
    }
    if let Some(value) = args.retention_remove_below {
        policy.remove_below_strength = value.clamp(0.0, 3.0);
    }
    if let Some(value) = args.retention_remove_after_failures {
        policy.remove_after_failures = value.max(1);
    }

    policy
}

fn memory_compaction_policy_from_args(
    mut policy: MemoryCompactionPolicy,
    args: &Args,
) -> MemoryCompactionPolicy {
    if let Some(value) = args.compaction_similarity_threshold {
        policy.similarity_threshold = value.clamp(0.10, 0.999);
    }
    if let Some(value) = args.compaction_max_candidates {
        policy.max_candidates = value.max(2);
    }
    if let Some(value) = args.compaction_max_merges {
        policy.max_merges = value;
    }

    policy
}
