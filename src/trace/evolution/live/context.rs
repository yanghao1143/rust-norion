use super::super::super::fields::{
    extract_json_bool_field, extract_json_f32_field, extract_json_string_array_field,
    extract_json_usize_field,
};

#[derive(Debug, Clone)]
pub(super) struct LiveEvolutionTrace {
    pub(super) live_inference_recorded: bool,
    pub(super) adaptive: LiveAdaptiveTrace,
    pub(super) memory: LiveMemoryTrace,
    pub(super) stored: LiveStoredMemoryTrace,
    pub(super) reflection: LiveReflectionTrace,
    pub(super) online_reward: OnlineRewardTrace,
    pub(super) cumulative: CumulativeLiveEvolutionTrace,
    pub(super) has_online_reward_note: bool,
}

impl LiveEvolutionTrace {
    pub(super) fn from_line(line: &str) -> Self {
        Self {
            live_inference_recorded: extract_json_bool_field(line, "live_inference_recorded")
                .unwrap_or(false),
            adaptive: LiveAdaptiveTrace::from_line(line),
            memory: LiveMemoryTrace::from_line(line),
            stored: LiveStoredMemoryTrace::from_line(line),
            reflection: LiveReflectionTrace::from_line(line),
            online_reward: OnlineRewardTrace::from_line(line, "live_online_reward"),
            cumulative: CumulativeLiveEvolutionTrace::from_line(line),
            has_online_reward_note: line.contains("online_reward_feedback:"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct LiveAdaptiveTrace {
    pub(super) router_threshold_delta: f32,
    pub(super) hierarchy_weight_delta: f32,
    pub(super) cumulative_router_threshold_delta: f32,
    pub(super) cumulative_hierarchy_weight_delta: f32,
    pub(super) cumulative_router_threshold_mutations: usize,
    pub(super) cumulative_hierarchy_weight_mutations: usize,
}

impl LiveAdaptiveTrace {
    fn from_line(line: &str) -> Self {
        Self {
            router_threshold_delta: extract_json_f32_field(line, "live_router_threshold_delta")
                .unwrap_or(0.0),
            hierarchy_weight_delta: extract_json_f32_field(line, "live_hierarchy_weight_delta")
                .unwrap_or(0.0),
            cumulative_router_threshold_delta: extract_json_f32_field(
                line,
                "cumulative_live_router_threshold_delta",
            )
            .unwrap_or(0.0),
            cumulative_hierarchy_weight_delta: extract_json_f32_field(
                line,
                "cumulative_live_hierarchy_weight_delta",
            )
            .unwrap_or(0.0),
            cumulative_router_threshold_mutations: extract_json_usize_field(
                line,
                "cumulative_live_router_threshold_mutations",
            )
            .unwrap_or(0),
            cumulative_hierarchy_weight_mutations: extract_json_usize_field(
                line,
                "cumulative_live_hierarchy_weight_mutations",
            )
            .unwrap_or(0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct LiveMemoryTrace {
    pub(super) reinforcements: usize,
    pub(super) penalties: usize,
    pub(super) updates: usize,
    pub(super) feedback_reinforced: usize,
    pub(super) feedback_penalized: usize,
}

impl LiveMemoryTrace {
    fn from_line(line: &str) -> Self {
        Self {
            reinforcements: extract_json_usize_field(line, "live_memory_reinforcements")
                .unwrap_or(0),
            penalties: extract_json_usize_field(line, "live_memory_penalties").unwrap_or(0),
            updates: extract_json_usize_field(line, "live_memory_updates").unwrap_or(0),
            feedback_reinforced: extract_json_usize_field(line, "feedback_reinforced").unwrap_or(0),
            feedback_penalized: extract_json_usize_field(line, "feedback_penalized").unwrap_or(0),
        }
    }

    pub(super) fn expected_updates(self) -> usize {
        self.reinforcements.saturating_add(self.penalties)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct LiveStoredMemoryTrace {
    pub(super) memory: usize,
    pub(super) gist_memories: usize,
    pub(super) runtime_kv_memories: usize,
    pub(super) updates: usize,
    pub(super) gist_stored: usize,
    pub(super) runtime_kv_stored: usize,
}

impl LiveStoredMemoryTrace {
    fn from_line(line: &str) -> Self {
        Self {
            memory: usize::from(
                extract_json_bool_field(line, "live_stored_memory").unwrap_or(false),
            ),
            gist_memories: extract_json_usize_field(line, "live_stored_gist_memories").unwrap_or(0),
            runtime_kv_memories: extract_json_usize_field(line, "live_stored_runtime_kv_memories")
                .unwrap_or(0),
            updates: extract_json_usize_field(line, "live_stored_memory_updates").unwrap_or(0),
            gist_stored: extract_json_usize_field(line, "gist_stored").unwrap_or(0),
            runtime_kv_stored: extract_json_usize_field(line, "runtime_kv_stored").unwrap_or(0),
        }
    }

    pub(super) fn expected_updates(self) -> usize {
        self.memory
            .saturating_add(self.gist_memories)
            .saturating_add(self.runtime_kv_memories)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct LiveReflectionTrace {
    pub(super) issues: usize,
    pub(super) live_issues: usize,
    pub(super) critical_issues: usize,
    pub(super) live_critical_issues: usize,
    pub(super) revision_actions: usize,
    pub(super) live_revision_actions: usize,
}

impl LiveReflectionTrace {
    fn from_line(line: &str) -> Self {
        Self {
            issues: extract_json_usize_field(line, "issues").unwrap_or(0),
            live_issues: extract_json_usize_field(line, "live_reflection_issues").unwrap_or(0),
            critical_issues: extract_json_usize_field(line, "critical_issues").unwrap_or(0),
            live_critical_issues: extract_json_usize_field(line, "live_critical_reflection_issues")
                .unwrap_or(0),
            revision_actions: extract_json_string_array_field(line, "revision_actions")
                .map(|actions| actions.len())
                .unwrap_or(0),
            live_revision_actions: extract_json_usize_field(line, "live_revision_actions")
                .unwrap_or(0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct OnlineRewardTrace {
    pub(super) feedbacks: usize,
    pub(super) reinforcements: usize,
    pub(super) penalties: usize,
    pub(super) strength: f32,
    pub(super) reinforcement_strength: f32,
    pub(super) penalty_strength: f32,
}

impl OnlineRewardTrace {
    fn from_line(line: &str, prefix: &str) -> Self {
        Self {
            feedbacks: extract_json_usize_field(line, &format!("{prefix}_feedbacks")).unwrap_or(0),
            reinforcements: extract_json_usize_field(line, &format!("{prefix}_reinforcements"))
                .unwrap_or(0),
            penalties: extract_json_usize_field(line, &format!("{prefix}_penalties")).unwrap_or(0),
            strength: extract_json_f32_field(line, &format!("{prefix}_strength")).unwrap_or(0.0),
            reinforcement_strength: extract_json_f32_field(
                line,
                &format!("{prefix}_reinforcement_strength"),
            )
            .unwrap_or(0.0),
            penalty_strength: extract_json_f32_field(line, &format!("{prefix}_penalty_strength"))
                .unwrap_or(0.0),
        }
    }

    pub(super) fn expected_feedbacks(self) -> usize {
        self.reinforcements.saturating_add(self.penalties)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct CumulativeLiveEvolutionTrace {
    pub(super) inference_runs: usize,
    pub(super) online_reward: OnlineRewardTrace,
    pub(super) memory: CumulativeLiveMemoryTrace,
    pub(super) stored: CumulativeLiveStoredMemoryTrace,
    pub(super) reflection: CumulativeLiveReflectionTrace,
}

impl CumulativeLiveEvolutionTrace {
    fn from_line(line: &str) -> Self {
        Self {
            inference_runs: extract_json_usize_field(line, "live_inference_runs").unwrap_or(0),
            online_reward: OnlineRewardTrace::from_line(line, "cumulative_live_online_reward"),
            memory: CumulativeLiveMemoryTrace::from_line(line),
            stored: CumulativeLiveStoredMemoryTrace::from_line(line),
            reflection: CumulativeLiveReflectionTrace::from_line(line),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct CumulativeLiveMemoryTrace {
    pub(super) reinforcements: usize,
    pub(super) penalties: usize,
    pub(super) updates: usize,
}

impl CumulativeLiveMemoryTrace {
    fn from_line(line: &str) -> Self {
        Self {
            reinforcements: extract_json_usize_field(line, "cumulative_live_memory_reinforcements")
                .unwrap_or(0),
            penalties: extract_json_usize_field(line, "cumulative_live_memory_penalties")
                .unwrap_or(0),
            updates: extract_json_usize_field(line, "cumulative_live_memory_updates").unwrap_or(0),
        }
    }

    pub(super) fn expected_updates(self) -> usize {
        self.reinforcements.saturating_add(self.penalties)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct CumulativeLiveStoredMemoryTrace {
    pub(super) memories: usize,
    pub(super) gist_memories: usize,
    pub(super) runtime_kv_memories: usize,
    pub(super) updates: usize,
}

impl CumulativeLiveStoredMemoryTrace {
    fn from_line(line: &str) -> Self {
        Self {
            memories: extract_json_usize_field(line, "cumulative_live_stored_memories")
                .unwrap_or(0),
            gist_memories: extract_json_usize_field(line, "cumulative_live_stored_gist_memories")
                .unwrap_or(0),
            runtime_kv_memories: extract_json_usize_field(
                line,
                "cumulative_live_stored_runtime_kv_memories",
            )
            .unwrap_or(0),
            updates: extract_json_usize_field(line, "cumulative_live_stored_memory_updates")
                .unwrap_or(0),
        }
    }

    pub(super) fn expected_updates(self) -> usize {
        self.memories
            .saturating_add(self.gist_memories)
            .saturating_add(self.runtime_kv_memories)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct CumulativeLiveReflectionTrace {
    pub(super) issues: usize,
    pub(super) critical_issues: usize,
    pub(super) revision_actions: usize,
}

impl CumulativeLiveReflectionTrace {
    fn from_line(line: &str) -> Self {
        Self {
            issues: extract_json_usize_field(line, "cumulative_live_reflection_issues")
                .unwrap_or(0),
            critical_issues: extract_json_usize_field(
                line,
                "cumulative_live_critical_reflection_issues",
            )
            .unwrap_or(0),
            revision_actions: extract_json_usize_field(line, "cumulative_live_revision_actions")
                .unwrap_or(0),
        }
    }
}
