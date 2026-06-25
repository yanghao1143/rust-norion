use super::components::action_for_total;
use super::types::{ProcessRewardComponents, ProcessRewardInput};

pub(super) fn reward_notes(
    input: &ProcessRewardInput,
    components: ProcessRewardComponents,
    total: f32,
) -> Vec<String> {
    let mut notes = Vec::new();

    if components.route >= 0.75 {
        notes.push("route:efficient_for_quality".to_owned());
    } else if components.route <= 0.40 {
        notes.push("route:under_allocated_attention".to_owned());
    }

    if components.memory >= 0.72 {
        notes.push("memory:useful_reuse_or_gist".to_owned());
    } else if input.used_memories > 0 && input.contradiction_count > 0 {
        notes.push("memory:reuse_needs_penalty".to_owned());
    }

    if input.critical_reflection_issue_count > 0 {
        notes.push(format!(
            "reflection:critical_issues={}",
            input.critical_reflection_issue_count
        ));
    } else if input.reflection_issue_count > 0 {
        notes.push(format!(
            "reflection:issues={}:actions={}",
            input.reflection_issue_count, input.revision_action_count
        ));
    }

    if input.recursive_schedule.requires_recursion {
        notes.push(format!(
            "recursive:chunks={}:merge_rounds={}:waves={}:parallel={}:runtime_calls={}",
            input.recursive_schedule.chunk_count(),
            input.recursive_schedule.merge_round_count(),
            input.recursive_schedule.execution_wave_count(),
            input.recursive_schedule.max_parallel_chunks,
            input.recursive_runtime_calls
        ));
    }
    if input.recursive_runtime_calls > input.recursive_schedule.chunk_count().max(1) * 2 {
        notes.push(format!(
            "latency:recursive_runtime_calls={}",
            input.recursive_runtime_calls
        ));
    }

    if components.admission <= 0.35 {
        notes.push("admission:stored_low_quality_memory".to_owned());
    }
    if input.stored_runtime_kv_memories > 0 {
        notes.push(format!(
            "runtime_kv:stored={}",
            input.stored_runtime_kv_memories
        ));
    }
    if input.runtime_kv_segment_count() > 0 {
        notes.push(format!(
            "runtime_kv_segments:included={}:skipped={}:rejected={}:total={}",
            input.runtime_kv_segments_included,
            input.runtime_kv_segments_skipped,
            input.runtime_kv_segments_rejected,
            input.runtime_kv_segment_count()
        ));
    }
    notes.extend(input.toolsmith_plan.reward_notes());
    notes.extend(input.agent_team_plan.reward_notes());

    notes.push(format!(
        "total:{total:.3}:{}",
        action_for_total(total).as_str()
    ));
    notes
}
