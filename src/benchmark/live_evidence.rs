use crate::engine::InferenceOutcome;
use crate::hardware::DeviceClass;

use super::{explicit_device_count, push_unique_device};

const BENCHMARK_FLOAT_EPSILON: f32 = 0.000_001;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkLiveEvolutionEvidence {
    pub(super) inference_devices: Vec<DeviceClass>,
    pub(super) router_threshold_mutation_devices: Vec<DeviceClass>,
    pub(super) hierarchy_weight_mutation_devices: Vec<DeviceClass>,
    pub(super) online_reward_devices: Vec<DeviceClass>,
    pub(super) online_reward_strength_devices: Vec<DeviceClass>,
    pub(super) memory_update_devices: Vec<DeviceClass>,
    pub(super) stored_memory_update_devices: Vec<DeviceClass>,
    pub(super) reflection_issue_devices: Vec<DeviceClass>,
    pub(super) critical_reflection_issue_devices: Vec<DeviceClass>,
    pub(super) revision_action_devices: Vec<DeviceClass>,
    pub(super) replay_live_evolution_devices: Vec<DeviceClass>,
    pub(super) replay_live_evolution_online_reward_devices: Vec<DeviceClass>,
    pub(super) replay_live_evolution_online_reward_strength_devices: Vec<DeviceClass>,
    pub(super) replay_live_evolution_memory_update_devices: Vec<DeviceClass>,
    pub(super) replay_live_evolution_critical_reflection_issue_devices: Vec<DeviceClass>,
    pub(super) replay_live_evolution_revision_action_devices: Vec<DeviceClass>,
}

impl BenchmarkLiveEvolutionEvidence {
    pub(super) fn record(&mut self, outcome: &InferenceOutcome) {
        let device = outcome.hardware_plan.device;
        let live = outcome.live_evolution;

        push_unique_device(&mut self.inference_devices, device);
        if live.router_threshold_delta > 0.000001 {
            push_unique_device(&mut self.router_threshold_mutation_devices, device);
        }
        if live.hierarchy_weight_delta > 0.000001 {
            push_unique_device(&mut self.hierarchy_weight_mutation_devices, device);
        }
        if live.online_reward_feedbacks > 0
            && live.online_reward_feedbacks
                == live
                    .online_reward_reinforcements
                    .saturating_add(live.online_reward_penalties)
        {
            push_unique_device(&mut self.online_reward_devices, device);
        }
        if online_reward_strength_is_consistent(
            live.online_reward_feedbacks,
            live.online_reward_reinforcements,
            live.online_reward_penalties,
            live.online_reward_strength,
            live.online_reward_reinforcement_strength,
            live.online_reward_penalty_strength,
        ) {
            push_unique_device(&mut self.online_reward_strength_devices, device);
        }
        if live.memory_reinforcements > 0 || live.memory_penalties > 0 {
            push_unique_device(&mut self.memory_update_devices, device);
        }
        if live.stored_memory
            || live.stored_gist_memories > 0
            || live.stored_runtime_kv_memories > 0
        {
            push_unique_device(&mut self.stored_memory_update_devices, device);
        }
        if live.reflection_issues > 0 {
            push_unique_device(&mut self.reflection_issue_devices, device);
        }
        if live.critical_reflection_issues > 0 {
            push_unique_device(&mut self.critical_reflection_issue_devices, device);
        }
        if live.revision_actions > 0 {
            push_unique_device(&mut self.revision_action_devices, device);
        }
        if let Some(replay) = outcome.auto_replay_report.as_ref() {
            if replay.live_evolution_items > 0 {
                push_unique_device(&mut self.replay_live_evolution_devices, device);
            }
            if replay.live_evolution_online_reward_feedbacks > 0
                && replay.live_evolution_online_reward_feedbacks
                    == replay
                        .live_evolution_online_reward_reinforcements
                        .saturating_add(replay.live_evolution_online_reward_penalties)
            {
                push_unique_device(
                    &mut self.replay_live_evolution_online_reward_devices,
                    device,
                );
            }
            if online_reward_strength_is_consistent(
                replay.live_evolution_online_reward_feedbacks,
                replay.live_evolution_online_reward_reinforcements,
                replay.live_evolution_online_reward_penalties,
                replay.live_evolution_online_reward_strength,
                replay.live_evolution_online_reward_reinforcement_strength,
                replay.live_evolution_online_reward_penalty_strength,
            ) {
                push_unique_device(
                    &mut self.replay_live_evolution_online_reward_strength_devices,
                    device,
                );
            }
            if replay.live_evolution_memory_updates > 0 {
                push_unique_device(
                    &mut self.replay_live_evolution_memory_update_devices,
                    device,
                );
            }
            if replay.live_evolution_critical_reflection_issues > 0 {
                push_unique_device(
                    &mut self.replay_live_evolution_critical_reflection_issue_devices,
                    device,
                );
            }
            if replay.live_evolution_revision_actions > 0 {
                push_unique_device(
                    &mut self.replay_live_evolution_revision_action_devices,
                    device,
                );
            }
        }
    }

    pub fn inference_device_profiles(&self) -> usize {
        explicit_device_count(&self.inference_devices)
    }

    pub fn router_threshold_mutation_device_profiles(&self) -> usize {
        explicit_device_count(&self.router_threshold_mutation_devices)
    }

    pub fn hierarchy_weight_mutation_device_profiles(&self) -> usize {
        explicit_device_count(&self.hierarchy_weight_mutation_devices)
    }

    pub fn online_reward_device_profiles(&self) -> usize {
        explicit_device_count(&self.online_reward_devices)
    }

    pub fn online_reward_strength_device_profiles(&self) -> usize {
        explicit_device_count(&self.online_reward_strength_devices)
    }

    pub fn memory_update_device_profiles(&self) -> usize {
        explicit_device_count(&self.memory_update_devices)
    }

    pub fn stored_memory_update_device_profiles(&self) -> usize {
        explicit_device_count(&self.stored_memory_update_devices)
    }

    pub fn reflection_issue_device_profiles(&self) -> usize {
        explicit_device_count(&self.reflection_issue_devices)
    }

    pub fn critical_reflection_issue_device_profiles(&self) -> usize {
        explicit_device_count(&self.critical_reflection_issue_devices)
    }

    pub fn revision_action_device_profiles(&self) -> usize {
        explicit_device_count(&self.revision_action_devices)
    }

    pub fn replay_live_evolution_device_profiles(&self) -> usize {
        explicit_device_count(&self.replay_live_evolution_devices)
    }

    pub fn replay_live_evolution_online_reward_device_profiles(&self) -> usize {
        explicit_device_count(&self.replay_live_evolution_online_reward_devices)
    }

    pub fn replay_live_evolution_online_reward_strength_device_profiles(&self) -> usize {
        explicit_device_count(&self.replay_live_evolution_online_reward_strength_devices)
    }

    pub fn replay_live_evolution_memory_update_device_profiles(&self) -> usize {
        explicit_device_count(&self.replay_live_evolution_memory_update_devices)
    }

    pub fn replay_live_evolution_critical_reflection_issue_device_profiles(&self) -> usize {
        explicit_device_count(&self.replay_live_evolution_critical_reflection_issue_devices)
    }

    pub fn replay_live_evolution_revision_action_device_profiles(&self) -> usize {
        explicit_device_count(&self.replay_live_evolution_revision_action_devices)
    }
}

fn online_reward_strength_is_consistent(
    feedbacks: usize,
    reinforcements: usize,
    penalties: usize,
    total: f32,
    reinforcement: f32,
    penalty: f32,
) -> bool {
    let has_reinforcement_strength = reinforcement > BENCHMARK_FLOAT_EPSILON;
    let has_penalty_strength = penalty > BENCHMARK_FLOAT_EPSILON;
    total.is_finite()
        && reinforcement.is_finite()
        && penalty.is_finite()
        && feedbacks > 0
        && feedbacks == reinforcements.saturating_add(penalties)
        && total > BENCHMARK_FLOAT_EPSILON
        && reinforcement >= 0.0
        && penalty >= 0.0
        && (!has_reinforcement_strength || reinforcements > 0)
        && (!has_penalty_strength || penalties > 0)
        && (total - (reinforcement + penalty)).abs() <= BENCHMARK_FLOAT_EPSILON
}
