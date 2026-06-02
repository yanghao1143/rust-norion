use std::io;
use std::path::Path;

use crate::disk_kv::DiskKvStore;
use crate::experience_replay::ExperienceReplayReport;
use crate::hierarchy::{
    HierarchyState, HierarchyWeights, ProfileHierarchyObservations, ProfileHierarchyWeights,
};
use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};
use crate::router::{ProfileObservations, ProfileThresholds, RouterState};
use crate::tiered_cache::{MemoryPlacement, MemoryTier, TieredCachePlan};

#[derive(Debug, Clone)]
pub struct AdaptiveState {
    pub router: RouterState,
    pub hierarchy: HierarchyState,
    pub tier_plan: TieredCachePlan,
    pub memory_retention_policy: MemoryRetentionPolicy,
    pub memory_compaction_policy: MemoryCompactionPolicy,
    pub evolution_ledger: EvolutionLedger,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LiveInferenceEvolution {
    pub router_threshold_delta: f32,
    pub hierarchy_weight_delta: f32,
    pub online_reward_feedbacks: usize,
    pub online_reward_reinforcements: usize,
    pub online_reward_penalties: usize,
    pub online_reward_strength: f32,
    pub online_reward_reinforcement_strength: f32,
    pub online_reward_penalty_strength: f32,
    pub memory_reinforcements: usize,
    pub memory_penalties: usize,
    pub stored_memory: bool,
    pub stored_gist_memories: usize,
    pub stored_runtime_kv_memories: usize,
    pub reflection_issues: usize,
    pub critical_reflection_issues: usize,
    pub revision_actions: usize,
}

impl LiveInferenceEvolution {
    pub fn memory_updates(self) -> usize {
        self.memory_reinforcements
            .saturating_add(self.memory_penalties)
    }

    pub fn stored_memory_updates(self) -> usize {
        usize::from(self.stored_memory)
            .saturating_add(self.stored_gist_memories)
            .saturating_add(self.stored_runtime_kv_memories)
    }

    pub fn has_evidence(self) -> bool {
        self.router_threshold_delta > 0.000001
            || self.hierarchy_weight_delta > 0.000001
            || self.online_reward_feedbacks > 0
            || nonnegative_f32(self.online_reward_strength) > 0.000001
            || self.memory_updates() > 0
            || self.stored_memory_updates() > 0
            || self.reflection_issues > 0
            || self.critical_reflection_issues > 0
            || self.revision_actions > 0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EvolutionLedger {
    pub live_inference_runs: u64,
    pub live_router_threshold_mutations: u64,
    pub live_hierarchy_weight_mutations: u64,
    pub live_router_threshold_delta: f32,
    pub live_hierarchy_weight_delta: f32,
    pub live_online_reward_feedbacks: u64,
    pub live_online_reward_reinforcements: u64,
    pub live_online_reward_penalties: u64,
    pub live_online_reward_strength: f32,
    pub live_online_reward_reinforcement_strength: f32,
    pub live_online_reward_penalty_strength: f32,
    pub live_memory_reinforcements: u64,
    pub live_memory_penalties: u64,
    pub live_stored_memories: u64,
    pub live_stored_gist_memories: u64,
    pub live_stored_runtime_kv_memories: u64,
    pub live_reflection_issues: u64,
    pub live_critical_reflection_issues: u64,
    pub live_revision_actions: u64,
    pub replay_runs: u64,
    pub replay_items: u64,
    pub router_threshold_mutations: u64,
    pub hierarchy_weight_mutations: u64,
    pub router_threshold_delta: f32,
    pub hierarchy_weight_delta: f32,
    pub memory_reinforcements: u64,
    pub memory_penalties: u64,
    pub replay_live_memory_feedback_items: u64,
    pub replay_live_memory_feedback_reinforcements: u64,
    pub replay_live_memory_feedback_penalties: u64,
    pub replay_live_memory_feedback_detail_items: u64,
    pub replay_live_memory_feedback_applied: u64,
    pub replay_live_memory_feedback_removed: u64,
    pub replay_live_memory_feedback_missing: u64,
    pub replay_live_memory_feedback_strength_delta: f32,
    pub replay_live_evolution_items: u64,
    pub replay_live_evolution_router_threshold_mutations: u64,
    pub replay_live_evolution_hierarchy_weight_mutations: u64,
    pub replay_live_evolution_router_threshold_delta: f32,
    pub replay_live_evolution_hierarchy_weight_delta: f32,
    pub replay_live_evolution_online_reward_feedbacks: u64,
    pub replay_live_evolution_online_reward_reinforcements: u64,
    pub replay_live_evolution_online_reward_penalties: u64,
    pub replay_live_evolution_online_reward_strength: f32,
    pub replay_live_evolution_online_reward_reinforcement_strength: f32,
    pub replay_live_evolution_online_reward_penalty_strength: f32,
    pub replay_live_evolution_memory_updates: u64,
    pub replay_live_evolution_stored_memory_updates: u64,
    pub replay_live_evolution_reflection_issues: u64,
    pub replay_live_evolution_critical_reflection_issues: u64,
    pub replay_live_evolution_revision_actions: u64,
    pub recursive_replay_items: u64,
    pub recursive_runtime_calls: u64,
    pub drift_rollbacks: u64,
    pub rollback_router_threshold_delta: f32,
    pub rollback_hierarchy_weight_delta: f32,
}

impl EvolutionLedger {
    pub fn record_live_inference(&mut self, report: LiveInferenceEvolution) {
        self.live_inference_runs = self.live_inference_runs.saturating_add(1);
        if report.router_threshold_delta > 0.000001 {
            self.live_router_threshold_mutations =
                self.live_router_threshold_mutations.saturating_add(1);
            self.live_router_threshold_delta += report.router_threshold_delta;
        }
        if report.hierarchy_weight_delta > 0.000001 {
            self.live_hierarchy_weight_mutations =
                self.live_hierarchy_weight_mutations.saturating_add(1);
            self.live_hierarchy_weight_delta += report.hierarchy_weight_delta;
        }
        self.live_online_reward_feedbacks = self
            .live_online_reward_feedbacks
            .saturating_add(report.online_reward_feedbacks as u64);
        self.live_online_reward_reinforcements = self
            .live_online_reward_reinforcements
            .saturating_add(report.online_reward_reinforcements as u64);
        self.live_online_reward_penalties = self
            .live_online_reward_penalties
            .saturating_add(report.online_reward_penalties as u64);
        self.live_online_reward_strength += nonnegative_f32(report.online_reward_strength);
        self.live_online_reward_reinforcement_strength +=
            nonnegative_f32(report.online_reward_reinforcement_strength);
        self.live_online_reward_penalty_strength +=
            nonnegative_f32(report.online_reward_penalty_strength);
        self.live_memory_reinforcements = self
            .live_memory_reinforcements
            .saturating_add(report.memory_reinforcements as u64);
        self.live_memory_penalties = self
            .live_memory_penalties
            .saturating_add(report.memory_penalties as u64);
        self.live_stored_memories = self
            .live_stored_memories
            .saturating_add(u64::from(report.stored_memory));
        self.live_stored_gist_memories = self
            .live_stored_gist_memories
            .saturating_add(report.stored_gist_memories as u64);
        self.live_stored_runtime_kv_memories = self
            .live_stored_runtime_kv_memories
            .saturating_add(report.stored_runtime_kv_memories as u64);
        self.live_reflection_issues = self
            .live_reflection_issues
            .saturating_add(report.reflection_issues as u64);
        self.live_critical_reflection_issues = self
            .live_critical_reflection_issues
            .saturating_add(report.critical_reflection_issues as u64);
        self.live_revision_actions = self
            .live_revision_actions
            .saturating_add(report.revision_actions as u64);
    }

    pub fn record_replay(&mut self, report: &ExperienceReplayReport) {
        if report.applied == 0 {
            return;
        }

        self.replay_runs = self.replay_runs.saturating_add(1);
        self.replay_items = self.replay_items.saturating_add(report.applied as u64);
        self.router_threshold_mutations = self
            .router_threshold_mutations
            .saturating_add(report.router_threshold_mutations as u64);
        self.hierarchy_weight_mutations = self
            .hierarchy_weight_mutations
            .saturating_add(report.hierarchy_weight_mutations as u64);
        self.router_threshold_delta += report.router_threshold_delta;
        self.hierarchy_weight_delta += report.hierarchy_weight_delta;
        self.memory_reinforcements = self
            .memory_reinforcements
            .saturating_add(report.memory_reinforcements as u64);
        self.memory_penalties = self
            .memory_penalties
            .saturating_add(report.memory_penalties as u64);
        self.replay_live_memory_feedback_items = self
            .replay_live_memory_feedback_items
            .saturating_add(report.live_memory_feedback_items as u64);
        self.replay_live_memory_feedback_reinforcements = self
            .replay_live_memory_feedback_reinforcements
            .saturating_add(report.live_memory_feedback_reinforcements as u64);
        self.replay_live_memory_feedback_penalties = self
            .replay_live_memory_feedback_penalties
            .saturating_add(report.live_memory_feedback_penalties as u64);
        self.replay_live_memory_feedback_detail_items = self
            .replay_live_memory_feedback_detail_items
            .saturating_add(report.live_memory_feedback_detail_items as u64);
        self.replay_live_memory_feedback_applied = self
            .replay_live_memory_feedback_applied
            .saturating_add(report.live_memory_feedback_applied as u64);
        self.replay_live_memory_feedback_removed = self
            .replay_live_memory_feedback_removed
            .saturating_add(report.live_memory_feedback_removed as u64);
        self.replay_live_memory_feedback_missing = self
            .replay_live_memory_feedback_missing
            .saturating_add(report.live_memory_feedback_missing as u64);
        self.replay_live_memory_feedback_strength_delta +=
            report.live_memory_feedback_strength_delta.max(0.0);
        self.replay_live_evolution_items = self
            .replay_live_evolution_items
            .saturating_add(report.live_evolution_items as u64);
        self.replay_live_evolution_router_threshold_mutations = self
            .replay_live_evolution_router_threshold_mutations
            .saturating_add(report.live_evolution_router_threshold_mutations as u64);
        self.replay_live_evolution_hierarchy_weight_mutations = self
            .replay_live_evolution_hierarchy_weight_mutations
            .saturating_add(report.live_evolution_hierarchy_weight_mutations as u64);
        self.replay_live_evolution_router_threshold_delta +=
            report.live_evolution_router_threshold_delta.max(0.0);
        self.replay_live_evolution_hierarchy_weight_delta +=
            report.live_evolution_hierarchy_weight_delta.max(0.0);
        self.replay_live_evolution_online_reward_feedbacks = self
            .replay_live_evolution_online_reward_feedbacks
            .saturating_add(report.live_evolution_online_reward_feedbacks as u64);
        self.replay_live_evolution_online_reward_reinforcements = self
            .replay_live_evolution_online_reward_reinforcements
            .saturating_add(report.live_evolution_online_reward_reinforcements as u64);
        self.replay_live_evolution_online_reward_penalties = self
            .replay_live_evolution_online_reward_penalties
            .saturating_add(report.live_evolution_online_reward_penalties as u64);
        self.replay_live_evolution_online_reward_strength +=
            nonnegative_f32(report.live_evolution_online_reward_strength);
        self.replay_live_evolution_online_reward_reinforcement_strength +=
            nonnegative_f32(report.live_evolution_online_reward_reinforcement_strength);
        self.replay_live_evolution_online_reward_penalty_strength +=
            nonnegative_f32(report.live_evolution_online_reward_penalty_strength);
        self.replay_live_evolution_memory_updates = self
            .replay_live_evolution_memory_updates
            .saturating_add(report.live_evolution_memory_updates as u64);
        self.replay_live_evolution_stored_memory_updates = self
            .replay_live_evolution_stored_memory_updates
            .saturating_add(report.live_evolution_stored_memory_updates as u64);
        self.replay_live_evolution_reflection_issues = self
            .replay_live_evolution_reflection_issues
            .saturating_add(report.live_evolution_reflection_issues as u64);
        self.replay_live_evolution_critical_reflection_issues = self
            .replay_live_evolution_critical_reflection_issues
            .saturating_add(report.live_evolution_critical_reflection_issues as u64);
        self.replay_live_evolution_revision_actions = self
            .replay_live_evolution_revision_actions
            .saturating_add(report.live_evolution_revision_actions as u64);
        self.recursive_replay_items = self
            .recursive_replay_items
            .saturating_add(report.recursive_runtime_items as u64);
        self.recursive_runtime_calls = self
            .recursive_runtime_calls
            .saturating_add(report.recursive_runtime_calls as u64);
    }

    pub fn memory_updates(self) -> u64 {
        self.memory_reinforcements
            .saturating_add(self.memory_penalties)
    }

    pub fn live_memory_updates(self) -> u64 {
        self.live_memory_reinforcements
            .saturating_add(self.live_memory_penalties)
    }

    pub fn live_stored_memory_updates(self) -> u64 {
        self.live_stored_memories
            .saturating_add(self.live_stored_gist_memories)
            .saturating_add(self.live_stored_runtime_kv_memories)
    }

    pub fn replay_live_memory_feedback_updates(self) -> u64 {
        self.replay_live_memory_feedback_reinforcements
            .saturating_add(self.replay_live_memory_feedback_penalties)
    }

    pub fn replay_live_memory_feedback_detail_updates(self) -> u64 {
        self.replay_live_memory_feedback_applied
            .saturating_add(self.replay_live_memory_feedback_missing)
    }

    pub fn record_drift_rollback(
        &mut self,
        router_threshold_delta: f32,
        hierarchy_weight_delta: f32,
    ) {
        self.drift_rollbacks = self.drift_rollbacks.saturating_add(1);
        self.rollback_router_threshold_delta += router_threshold_delta.max(0.0);
        self.rollback_hierarchy_weight_delta += hierarchy_weight_delta.max(0.0);
    }

    pub fn summary_line(self) -> String {
        format!(
            "evolution: live_inference_runs={} live_router_threshold_mutations={} live_hierarchy_weight_mutations={} live_router_threshold_delta={:.6} live_hierarchy_weight_delta={:.6} live_online_reward_feedbacks={} live_online_reward_reinforcements={} live_online_reward_penalties={} live_online_reward_strength={:.6} live_online_reward_reinforcement_strength={:.6} live_online_reward_penalty_strength={:.6} live_memory_updates={} live_stored_memory_updates={} live_reflection_issues={} live_critical_reflection_issues={} live_revision_actions={} replay_runs={} replay_items={} router_threshold_mutations={} hierarchy_weight_mutations={} router_threshold_delta={:.6} hierarchy_weight_delta={:.6} memory_updates={} replay_live_memory_feedback_items={} replay_live_memory_feedback_updates={} replay_live_memory_feedback_reinforcements={} replay_live_memory_feedback_penalties={} replay_live_memory_feedback_detail_items={} replay_live_memory_feedback_applied={} replay_live_memory_feedback_removed={} replay_live_memory_feedback_missing={} replay_live_memory_feedback_strength_delta={:.6} replay_live_evolution_items={} replay_live_evolution_router_threshold_mutations={} replay_live_evolution_hierarchy_weight_mutations={} replay_live_evolution_router_threshold_delta={:.6} replay_live_evolution_hierarchy_weight_delta={:.6} replay_live_evolution_online_reward_feedbacks={} replay_live_evolution_online_reward_reinforcements={} replay_live_evolution_online_reward_penalties={} replay_live_evolution_online_reward_strength={:.6} replay_live_evolution_online_reward_reinforcement_strength={:.6} replay_live_evolution_online_reward_penalty_strength={:.6} replay_live_evolution_memory_updates={} replay_live_evolution_stored_memory_updates={} replay_live_evolution_reflection_issues={} replay_live_evolution_critical_reflection_issues={} replay_live_evolution_revision_actions={} recursive_replay_items={} recursive_runtime_calls={} drift_rollbacks={} rollback_router_threshold_delta={:.6} rollback_hierarchy_weight_delta={:.6}",
            self.live_inference_runs,
            self.live_router_threshold_mutations,
            self.live_hierarchy_weight_mutations,
            self.live_router_threshold_delta,
            self.live_hierarchy_weight_delta,
            self.live_online_reward_feedbacks,
            self.live_online_reward_reinforcements,
            self.live_online_reward_penalties,
            self.live_online_reward_strength,
            self.live_online_reward_reinforcement_strength,
            self.live_online_reward_penalty_strength,
            self.live_memory_updates(),
            self.live_stored_memory_updates(),
            self.live_reflection_issues,
            self.live_critical_reflection_issues,
            self.live_revision_actions,
            self.replay_runs,
            self.replay_items,
            self.router_threshold_mutations,
            self.hierarchy_weight_mutations,
            self.router_threshold_delta,
            self.hierarchy_weight_delta,
            self.memory_updates(),
            self.replay_live_memory_feedback_items,
            self.replay_live_memory_feedback_updates(),
            self.replay_live_memory_feedback_reinforcements,
            self.replay_live_memory_feedback_penalties,
            self.replay_live_memory_feedback_detail_items,
            self.replay_live_memory_feedback_applied,
            self.replay_live_memory_feedback_removed,
            self.replay_live_memory_feedback_missing,
            self.replay_live_memory_feedback_strength_delta,
            self.replay_live_evolution_items,
            self.replay_live_evolution_router_threshold_mutations,
            self.replay_live_evolution_hierarchy_weight_mutations,
            self.replay_live_evolution_router_threshold_delta,
            self.replay_live_evolution_hierarchy_weight_delta,
            self.replay_live_evolution_online_reward_feedbacks,
            self.replay_live_evolution_online_reward_reinforcements,
            self.replay_live_evolution_online_reward_penalties,
            self.replay_live_evolution_online_reward_strength,
            self.replay_live_evolution_online_reward_reinforcement_strength,
            self.replay_live_evolution_online_reward_penalty_strength,
            self.replay_live_evolution_memory_updates,
            self.replay_live_evolution_stored_memory_updates,
            self.replay_live_evolution_reflection_issues,
            self.replay_live_evolution_critical_reflection_issues,
            self.replay_live_evolution_revision_actions,
            self.recursive_replay_items,
            self.recursive_runtime_calls,
            self.drift_rollbacks,
            self.rollback_router_threshold_delta,
            self.rollback_hierarchy_weight_delta
        )
    }
}

impl AdaptiveState {
    pub fn save_to_disk_kv(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut store = DiskKvStore::open(path)?;
        store.put(
            "adaptive/router",
            format!(
                "{:.6}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}",
                self.router.threshold,
                self.router.observations,
                self.router.profile_thresholds.general,
                self.router.profile_thresholds.coding,
                self.router.profile_thresholds.writing,
                self.router.profile_thresholds.long_document,
                self.router.profile_observations.general,
                self.router.profile_observations.coding,
                self.router.profile_observations.writing,
                self.router.profile_observations.long_document
            )
            .as_bytes(),
        )?;
        store.put(
            "adaptive/hierarchy",
            serialize_hierarchy_state(self.hierarchy).as_bytes(),
        )?;
        store.put(
            "adaptive/tier_plan",
            serialize_tier_plan(&self.tier_plan).as_bytes(),
        )?;
        store.put(
            "adaptive/memory_retention",
            serialize_memory_retention_policy(self.memory_retention_policy).as_bytes(),
        )?;
        store.put(
            "adaptive/memory_compaction",
            serialize_memory_compaction_policy(&self.memory_compaction_policy).as_bytes(),
        )?;
        store.put(
            "adaptive/evolution_ledger",
            serialize_evolution_ledger(self.evolution_ledger).as_bytes(),
        )?;
        store.compact()
    }

    pub fn load_from_disk_kv(path: impl AsRef<Path>) -> io::Result<Option<Self>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(None);
        }

        let store = DiskKvStore::open(path)?;
        let Some(router_bytes) = store.get("adaptive/router")? else {
            return Ok(None);
        };
        let Some(hierarchy_bytes) = store.get("adaptive/hierarchy")? else {
            return Ok(None);
        };
        let Some(router) = parse_router_state(&String::from_utf8_lossy(&router_bytes)) else {
            return Ok(None);
        };
        let Some(hierarchy) = parse_hierarchy_state(&String::from_utf8_lossy(&hierarchy_bytes))
        else {
            return Ok(None);
        };

        let tier_plan = if let Some(tier_bytes) = store.get("adaptive/tier_plan")? {
            parse_tier_plan(&String::from_utf8_lossy(&tier_bytes))
        } else {
            TieredCachePlan::default()
        };
        let memory_retention_policy =
            if let Some(retention_bytes) = store.get("adaptive/memory_retention")? {
                parse_memory_retention_policy(&String::from_utf8_lossy(&retention_bytes))
                    .unwrap_or_default()
            } else {
                MemoryRetentionPolicy::default()
            };
        let memory_compaction_policy =
            if let Some(compaction_bytes) = store.get("adaptive/memory_compaction")? {
                parse_memory_compaction_policy(&String::from_utf8_lossy(&compaction_bytes))
                    .unwrap_or_default()
            } else {
                MemoryCompactionPolicy::default()
            };
        let evolution_ledger = if let Some(ledger_bytes) = store.get("adaptive/evolution_ledger")? {
            parse_evolution_ledger(&String::from_utf8_lossy(&ledger_bytes)).unwrap_or_default()
        } else {
            EvolutionLedger::default()
        };

        Ok(Some(Self {
            router,
            hierarchy,
            tier_plan,
            memory_retention_policy,
            memory_compaction_policy,
            evolution_ledger,
        }))
    }
}

fn parse_router_state(value: &str) -> Option<RouterState> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 2 && fields.len() != 10 {
        return None;
    }

    let threshold = fields[0].parse::<f32>().ok()?;
    let observations = fields[1].parse::<u64>().ok()?;
    let profile_thresholds = if fields.len() == 10 {
        ProfileThresholds {
            general: fields[2].parse::<f32>().ok()?,
            coding: fields[3].parse::<f32>().ok()?,
            writing: fields[4].parse::<f32>().ok()?,
            long_document: fields[5].parse::<f32>().ok()?,
        }
    } else {
        ProfileThresholds::from_single(threshold)
    };
    let profile_observations = if fields.len() == 10 {
        ProfileObservations {
            general: fields[6].parse::<u64>().ok()?,
            coding: fields[7].parse::<u64>().ok()?,
            writing: fields[8].parse::<u64>().ok()?,
            long_document: fields[9].parse::<u64>().ok()?,
        }
    } else {
        ProfileObservations::from_single(observations)
    };

    Some(RouterState {
        threshold,
        observations,
        profile_thresholds,
        profile_observations,
    })
}

fn parse_hierarchy_state(value: &str) -> Option<HierarchyState> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 3 && fields.len() != 19 {
        return None;
    }

    let current = HierarchyWeights::new(
        fields[0].parse::<f32>().ok()?,
        fields[1].parse::<f32>().ok()?,
        fields[2].parse::<f32>().ok()?,
    );
    let profile_weights = if fields.len() == 19 {
        ProfileHierarchyWeights {
            general: parse_hierarchy_weights(&fields[3..6])?,
            coding: parse_hierarchy_weights(&fields[6..9])?,
            writing: parse_hierarchy_weights(&fields[9..12])?,
            long_document: parse_hierarchy_weights(&fields[12..15])?,
        }
    } else {
        ProfileHierarchyWeights::from_single(current)
    };
    let profile_observations = if fields.len() == 19 {
        ProfileHierarchyObservations {
            general: fields[15].parse::<u64>().ok()?,
            coding: fields[16].parse::<u64>().ok()?,
            writing: fields[17].parse::<u64>().ok()?,
            long_document: fields[18].parse::<u64>().ok()?,
        }
    } else {
        ProfileHierarchyObservations::default()
    };

    Some(HierarchyState {
        current,
        profile_weights,
        profile_observations,
    })
}

fn serialize_hierarchy_state(state: HierarchyState) -> String {
    format!(
        "{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        state.current.global,
        state.current.local,
        state.current.convolution,
        serialize_hierarchy_weights(state.profile_weights.general),
        serialize_hierarchy_weights(state.profile_weights.coding),
        serialize_hierarchy_weights(state.profile_weights.writing),
        serialize_hierarchy_weights(state.profile_weights.long_document),
        state.profile_observations.general,
        state.profile_observations.coding,
        state.profile_observations.writing,
        state.profile_observations.long_document
    )
}

fn serialize_hierarchy_weights(weights: HierarchyWeights) -> String {
    format!(
        "{:.6}\t{:.6}\t{:.6}",
        weights.global, weights.local, weights.convolution
    )
}

fn parse_hierarchy_weights(fields: &[&str]) -> Option<HierarchyWeights> {
    if fields.len() != 3 {
        return None;
    }

    Some(HierarchyWeights::new(
        fields[0].parse::<f32>().ok()?,
        fields[1].parse::<f32>().ok()?,
        fields[2].parse::<f32>().ok()?,
    ))
}

fn serialize_tier_plan(plan: &TieredCachePlan) -> String {
    plan.placements()
        .iter()
        .map(|placement| {
            format!(
                "{}\t{}\t{:.6}\t{}",
                placement.id,
                placement.tier.as_str(),
                placement.score,
                escape_field(&placement.reason)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_tier_plan(value: &str) -> TieredCachePlan {
    let placements = value
        .lines()
        .filter_map(parse_memory_placement)
        .collect::<Vec<_>>();
    TieredCachePlan::new(placements)
}

fn serialize_memory_retention_policy(policy: MemoryRetentionPolicy) -> String {
    format!(
        "{}\t{:.6}\t{:.6}\t{}",
        policy.stale_after,
        policy.decay_rate,
        policy.remove_below_strength,
        policy.remove_after_failures
    )
}

fn parse_memory_retention_policy(value: &str) -> Option<MemoryRetentionPolicy> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 4 {
        return None;
    }

    Some(MemoryRetentionPolicy {
        stale_after: fields[0].parse::<u64>().ok()?.max(1),
        decay_rate: fields[1].parse::<f32>().ok()?.clamp(0.0, 0.95),
        remove_below_strength: fields[2].parse::<f32>().ok()?.clamp(0.0, 3.0),
        remove_after_failures: fields[3].parse::<u64>().ok()?.max(1),
    })
}

fn serialize_memory_compaction_policy(policy: &MemoryCompactionPolicy) -> String {
    format!(
        "{:.6}\t{}\t{}",
        policy.similarity_threshold, policy.max_candidates, policy.max_merges
    )
}

fn parse_memory_compaction_policy(value: &str) -> Option<MemoryCompactionPolicy> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 3 {
        return None;
    }

    Some(MemoryCompactionPolicy {
        similarity_threshold: fields[0].parse::<f32>().ok()?.clamp(0.10, 0.999),
        max_candidates: fields[1].parse::<usize>().ok()?.max(2),
        max_merges: fields[2].parse::<usize>().ok()?,
    })
}

fn serialize_evolution_ledger(ledger: EvolutionLedger) -> String {
    [
        ledger.live_inference_runs.to_string(),
        ledger.live_router_threshold_mutations.to_string(),
        ledger.live_hierarchy_weight_mutations.to_string(),
        format!("{:.6}", ledger.live_router_threshold_delta),
        format!("{:.6}", ledger.live_hierarchy_weight_delta),
        ledger.live_online_reward_feedbacks.to_string(),
        ledger.live_online_reward_reinforcements.to_string(),
        ledger.live_online_reward_penalties.to_string(),
        ledger.live_memory_reinforcements.to_string(),
        ledger.live_memory_penalties.to_string(),
        ledger.live_stored_memories.to_string(),
        ledger.live_stored_gist_memories.to_string(),
        ledger.live_stored_runtime_kv_memories.to_string(),
        ledger.live_reflection_issues.to_string(),
        ledger.live_critical_reflection_issues.to_string(),
        ledger.live_revision_actions.to_string(),
        ledger.replay_runs.to_string(),
        ledger.replay_items.to_string(),
        ledger.router_threshold_mutations.to_string(),
        ledger.hierarchy_weight_mutations.to_string(),
        format!("{:.6}", ledger.router_threshold_delta),
        format!("{:.6}", ledger.hierarchy_weight_delta),
        ledger.memory_reinforcements.to_string(),
        ledger.memory_penalties.to_string(),
        ledger.replay_live_memory_feedback_items.to_string(),
        ledger
            .replay_live_memory_feedback_reinforcements
            .to_string(),
        ledger.replay_live_memory_feedback_penalties.to_string(),
        ledger.replay_live_memory_feedback_detail_items.to_string(),
        ledger.replay_live_memory_feedback_applied.to_string(),
        ledger.replay_live_memory_feedback_removed.to_string(),
        ledger.replay_live_memory_feedback_missing.to_string(),
        format!("{:.6}", ledger.replay_live_memory_feedback_strength_delta),
        ledger.replay_live_evolution_items.to_string(),
        ledger
            .replay_live_evolution_router_threshold_mutations
            .to_string(),
        ledger
            .replay_live_evolution_hierarchy_weight_mutations
            .to_string(),
        format!("{:.6}", ledger.replay_live_evolution_router_threshold_delta),
        format!("{:.6}", ledger.replay_live_evolution_hierarchy_weight_delta),
        ledger
            .replay_live_evolution_online_reward_feedbacks
            .to_string(),
        ledger
            .replay_live_evolution_online_reward_reinforcements
            .to_string(),
        ledger
            .replay_live_evolution_online_reward_penalties
            .to_string(),
        ledger.replay_live_evolution_memory_updates.to_string(),
        ledger
            .replay_live_evolution_stored_memory_updates
            .to_string(),
        ledger.replay_live_evolution_reflection_issues.to_string(),
        ledger
            .replay_live_evolution_critical_reflection_issues
            .to_string(),
        ledger.replay_live_evolution_revision_actions.to_string(),
        ledger.recursive_replay_items.to_string(),
        ledger.recursive_runtime_calls.to_string(),
        ledger.drift_rollbacks.to_string(),
        format!("{:.6}", ledger.rollback_router_threshold_delta),
        format!("{:.6}", ledger.rollback_hierarchy_weight_delta),
        format!("{:.6}", ledger.live_online_reward_strength),
        format!("{:.6}", ledger.live_online_reward_reinforcement_strength),
        format!("{:.6}", ledger.live_online_reward_penalty_strength),
        format!("{:.6}", ledger.replay_live_evolution_online_reward_strength),
        format!(
            "{:.6}",
            ledger.replay_live_evolution_online_reward_reinforcement_strength
        ),
        format!(
            "{:.6}",
            ledger.replay_live_evolution_online_reward_penalty_strength
        ),
    ]
    .join("\t")
}

fn parse_evolution_ledger(value: &str) -> Option<EvolutionLedger> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 10
        && fields.len() != 13
        && fields.len() != 16
        && fields.len() != 29
        && fields.len() != 34
        && fields.len() != 44
        && fields.len() != 50
        && fields.len() != 56
    {
        return None;
    }

    if fields.len() == 50 || fields.len() == 56 {
        let has_online_reward_strength = fields.len() == 56;
        return Some(EvolutionLedger {
            live_inference_runs: fields[0].parse::<u64>().ok()?,
            live_router_threshold_mutations: fields[1].parse::<u64>().ok()?,
            live_hierarchy_weight_mutations: fields[2].parse::<u64>().ok()?,
            live_router_threshold_delta: parse_nonnegative_f32(fields[3])?,
            live_hierarchy_weight_delta: parse_nonnegative_f32(fields[4])?,
            live_online_reward_feedbacks: fields[5].parse::<u64>().ok()?,
            live_online_reward_reinforcements: fields[6].parse::<u64>().ok()?,
            live_online_reward_penalties: fields[7].parse::<u64>().ok()?,
            live_online_reward_strength: if has_online_reward_strength {
                parse_nonnegative_f32(fields[50])?
            } else {
                0.0
            },
            live_online_reward_reinforcement_strength: if has_online_reward_strength {
                parse_nonnegative_f32(fields[51])?
            } else {
                0.0
            },
            live_online_reward_penalty_strength: if has_online_reward_strength {
                parse_nonnegative_f32(fields[52])?
            } else {
                0.0
            },
            live_memory_reinforcements: fields[8].parse::<u64>().ok()?,
            live_memory_penalties: fields[9].parse::<u64>().ok()?,
            live_stored_memories: fields[10].parse::<u64>().ok()?,
            live_stored_gist_memories: fields[11].parse::<u64>().ok()?,
            live_stored_runtime_kv_memories: fields[12].parse::<u64>().ok()?,
            live_reflection_issues: fields[13].parse::<u64>().ok()?,
            live_critical_reflection_issues: fields[14].parse::<u64>().ok()?,
            live_revision_actions: fields[15].parse::<u64>().ok()?,
            replay_runs: fields[16].parse::<u64>().ok()?,
            replay_items: fields[17].parse::<u64>().ok()?,
            router_threshold_mutations: fields[18].parse::<u64>().ok()?,
            hierarchy_weight_mutations: fields[19].parse::<u64>().ok()?,
            router_threshold_delta: parse_nonnegative_f32(fields[20])?,
            hierarchy_weight_delta: parse_nonnegative_f32(fields[21])?,
            memory_reinforcements: fields[22].parse::<u64>().ok()?,
            memory_penalties: fields[23].parse::<u64>().ok()?,
            replay_live_memory_feedback_items: fields[24].parse::<u64>().ok()?,
            replay_live_memory_feedback_reinforcements: fields[25].parse::<u64>().ok()?,
            replay_live_memory_feedback_penalties: fields[26].parse::<u64>().ok()?,
            replay_live_memory_feedback_detail_items: fields[27].parse::<u64>().ok()?,
            replay_live_memory_feedback_applied: fields[28].parse::<u64>().ok()?,
            replay_live_memory_feedback_removed: fields[29].parse::<u64>().ok()?,
            replay_live_memory_feedback_missing: fields[30].parse::<u64>().ok()?,
            replay_live_memory_feedback_strength_delta: parse_nonnegative_f32(fields[31])?,
            replay_live_evolution_items: fields[32].parse::<u64>().ok()?,
            replay_live_evolution_router_threshold_mutations: fields[33].parse::<u64>().ok()?,
            replay_live_evolution_hierarchy_weight_mutations: fields[34].parse::<u64>().ok()?,
            replay_live_evolution_router_threshold_delta: parse_nonnegative_f32(fields[35])?,
            replay_live_evolution_hierarchy_weight_delta: parse_nonnegative_f32(fields[36])?,
            replay_live_evolution_online_reward_feedbacks: fields[37].parse::<u64>().ok()?,
            replay_live_evolution_online_reward_reinforcements: fields[38].parse::<u64>().ok()?,
            replay_live_evolution_online_reward_penalties: fields[39].parse::<u64>().ok()?,
            replay_live_evolution_online_reward_strength: if has_online_reward_strength {
                parse_nonnegative_f32(fields[53])?
            } else {
                0.0
            },
            replay_live_evolution_online_reward_reinforcement_strength:
                if has_online_reward_strength {
                    parse_nonnegative_f32(fields[54])?
                } else {
                    0.0
                },
            replay_live_evolution_online_reward_penalty_strength: if has_online_reward_strength {
                parse_nonnegative_f32(fields[55])?
            } else {
                0.0
            },
            replay_live_evolution_memory_updates: fields[40].parse::<u64>().ok()?,
            replay_live_evolution_stored_memory_updates: fields[41].parse::<u64>().ok()?,
            replay_live_evolution_reflection_issues: fields[42].parse::<u64>().ok()?,
            replay_live_evolution_critical_reflection_issues: fields[43].parse::<u64>().ok()?,
            replay_live_evolution_revision_actions: fields[44].parse::<u64>().ok()?,
            recursive_replay_items: fields[45].parse::<u64>().ok()?,
            recursive_runtime_calls: fields[46].parse::<u64>().ok()?,
            drift_rollbacks: fields[47].parse::<u64>().ok()?,
            rollback_router_threshold_delta: parse_nonnegative_f32(fields[48])?,
            rollback_hierarchy_weight_delta: parse_nonnegative_f32(fields[49])?,
        });
    }

    if fields.len() == 29 || fields.len() == 34 || fields.len() == 44 {
        let has_replay_live_memory_feedback_detail = fields.len() >= 34;
        let has_replay_live_evolution = fields.len() == 44;
        let replay_live_evolution_index = if has_replay_live_evolution {
            Some(29)
        } else {
            None
        };
        let recursive_replay_index = if has_replay_live_evolution {
            39
        } else if has_replay_live_memory_feedback_detail {
            29
        } else {
            24
        };
        return Some(EvolutionLedger {
            live_inference_runs: fields[0].parse::<u64>().ok()?,
            live_router_threshold_mutations: fields[1].parse::<u64>().ok()?,
            live_hierarchy_weight_mutations: fields[2].parse::<u64>().ok()?,
            live_router_threshold_delta: parse_nonnegative_f32(fields[3])?,
            live_hierarchy_weight_delta: parse_nonnegative_f32(fields[4])?,
            live_online_reward_feedbacks: 0,
            live_online_reward_reinforcements: 0,
            live_online_reward_penalties: 0,
            live_online_reward_strength: 0.0,
            live_online_reward_reinforcement_strength: 0.0,
            live_online_reward_penalty_strength: 0.0,
            live_memory_reinforcements: fields[5].parse::<u64>().ok()?,
            live_memory_penalties: fields[6].parse::<u64>().ok()?,
            live_stored_memories: fields[7].parse::<u64>().ok()?,
            live_stored_gist_memories: fields[8].parse::<u64>().ok()?,
            live_stored_runtime_kv_memories: fields[9].parse::<u64>().ok()?,
            live_reflection_issues: fields[10].parse::<u64>().ok()?,
            live_critical_reflection_issues: fields[11].parse::<u64>().ok()?,
            live_revision_actions: fields[12].parse::<u64>().ok()?,
            replay_runs: fields[13].parse::<u64>().ok()?,
            replay_items: fields[14].parse::<u64>().ok()?,
            router_threshold_mutations: fields[15].parse::<u64>().ok()?,
            hierarchy_weight_mutations: fields[16].parse::<u64>().ok()?,
            router_threshold_delta: parse_nonnegative_f32(fields[17])?,
            hierarchy_weight_delta: parse_nonnegative_f32(fields[18])?,
            memory_reinforcements: fields[19].parse::<u64>().ok()?,
            memory_penalties: fields[20].parse::<u64>().ok()?,
            replay_live_memory_feedback_items: fields[21].parse::<u64>().ok()?,
            replay_live_memory_feedback_reinforcements: fields[22].parse::<u64>().ok()?,
            replay_live_memory_feedback_penalties: fields[23].parse::<u64>().ok()?,
            replay_live_memory_feedback_detail_items: if has_replay_live_memory_feedback_detail {
                fields[24].parse::<u64>().ok()?
            } else {
                0
            },
            replay_live_memory_feedback_applied: if has_replay_live_memory_feedback_detail {
                fields[25].parse::<u64>().ok()?
            } else {
                0
            },
            replay_live_memory_feedback_removed: if has_replay_live_memory_feedback_detail {
                fields[26].parse::<u64>().ok()?
            } else {
                0
            },
            replay_live_memory_feedback_missing: if has_replay_live_memory_feedback_detail {
                fields[27].parse::<u64>().ok()?
            } else {
                0
            },
            replay_live_memory_feedback_strength_delta: if has_replay_live_memory_feedback_detail {
                fields[28].parse::<f32>().ok()?.max(0.0)
            } else {
                0.0
            },
            replay_live_evolution_items: replay_live_evolution_index
                .and_then(|index| fields[index].parse::<u64>().ok())
                .unwrap_or(0),
            replay_live_evolution_router_threshold_mutations: replay_live_evolution_index
                .and_then(|index| fields[index + 1].parse::<u64>().ok())
                .unwrap_or(0),
            replay_live_evolution_hierarchy_weight_mutations: replay_live_evolution_index
                .and_then(|index| fields[index + 2].parse::<u64>().ok())
                .unwrap_or(0),
            replay_live_evolution_router_threshold_delta: replay_live_evolution_index
                .and_then(|index| fields[index + 3].parse::<f32>().ok())
                .unwrap_or(0.0)
                .max(0.0),
            replay_live_evolution_hierarchy_weight_delta: replay_live_evolution_index
                .and_then(|index| fields[index + 4].parse::<f32>().ok())
                .unwrap_or(0.0)
                .max(0.0),
            replay_live_evolution_online_reward_feedbacks: 0,
            replay_live_evolution_online_reward_reinforcements: 0,
            replay_live_evolution_online_reward_penalties: 0,
            replay_live_evolution_online_reward_strength: 0.0,
            replay_live_evolution_online_reward_reinforcement_strength: 0.0,
            replay_live_evolution_online_reward_penalty_strength: 0.0,
            replay_live_evolution_memory_updates: replay_live_evolution_index
                .and_then(|index| fields[index + 5].parse::<u64>().ok())
                .unwrap_or(0),
            replay_live_evolution_stored_memory_updates: replay_live_evolution_index
                .and_then(|index| fields[index + 6].parse::<u64>().ok())
                .unwrap_or(0),
            replay_live_evolution_reflection_issues: replay_live_evolution_index
                .and_then(|index| fields[index + 7].parse::<u64>().ok())
                .unwrap_or(0),
            replay_live_evolution_critical_reflection_issues: replay_live_evolution_index
                .and_then(|index| fields[index + 8].parse::<u64>().ok())
                .unwrap_or(0),
            replay_live_evolution_revision_actions: replay_live_evolution_index
                .and_then(|index| fields[index + 9].parse::<u64>().ok())
                .unwrap_or(0),
            recursive_replay_items: fields[recursive_replay_index].parse::<u64>().ok()?,
            recursive_runtime_calls: fields[recursive_replay_index + 1].parse::<u64>().ok()?,
            drift_rollbacks: fields[recursive_replay_index + 2].parse::<u64>().ok()?,
            rollback_router_threshold_delta: fields[recursive_replay_index + 3]
                .parse::<f32>()
                .ok()?
                .max(0.0),
            rollback_hierarchy_weight_delta: fields[recursive_replay_index + 4]
                .parse::<f32>()
                .ok()?
                .max(0.0),
        });
    }

    let (
        replay_live_memory_feedback_items,
        replay_live_memory_feedback_reinforcements,
        replay_live_memory_feedback_penalties,
        recursive_replay_index,
    ) = if fields.len() == 16 {
        (
            fields[8].parse::<u64>().ok()?,
            fields[9].parse::<u64>().ok()?,
            fields[10].parse::<u64>().ok()?,
            11,
        )
    } else {
        (0, 0, 0, 8)
    };

    let rollback_index = recursive_replay_index + 2;

    Some(EvolutionLedger {
        live_inference_runs: 0,
        live_router_threshold_mutations: 0,
        live_hierarchy_weight_mutations: 0,
        live_router_threshold_delta: 0.0,
        live_hierarchy_weight_delta: 0.0,
        live_online_reward_feedbacks: 0,
        live_online_reward_reinforcements: 0,
        live_online_reward_penalties: 0,
        live_online_reward_strength: 0.0,
        live_online_reward_reinforcement_strength: 0.0,
        live_online_reward_penalty_strength: 0.0,
        live_memory_reinforcements: 0,
        live_memory_penalties: 0,
        live_stored_memories: 0,
        live_stored_gist_memories: 0,
        live_stored_runtime_kv_memories: 0,
        live_reflection_issues: 0,
        live_critical_reflection_issues: 0,
        live_revision_actions: 0,
        replay_runs: fields[0].parse::<u64>().ok()?,
        replay_items: fields[1].parse::<u64>().ok()?,
        router_threshold_mutations: fields[2].parse::<u64>().ok()?,
        hierarchy_weight_mutations: fields[3].parse::<u64>().ok()?,
        router_threshold_delta: parse_nonnegative_f32(fields[4])?,
        hierarchy_weight_delta: parse_nonnegative_f32(fields[5])?,
        memory_reinforcements: fields[6].parse::<u64>().ok()?,
        memory_penalties: fields[7].parse::<u64>().ok()?,
        replay_live_memory_feedback_items,
        replay_live_memory_feedback_reinforcements,
        replay_live_memory_feedback_penalties,
        replay_live_memory_feedback_detail_items: 0,
        replay_live_memory_feedback_applied: 0,
        replay_live_memory_feedback_removed: 0,
        replay_live_memory_feedback_missing: 0,
        replay_live_memory_feedback_strength_delta: 0.0,
        replay_live_evolution_items: 0,
        replay_live_evolution_router_threshold_mutations: 0,
        replay_live_evolution_hierarchy_weight_mutations: 0,
        replay_live_evolution_router_threshold_delta: 0.0,
        replay_live_evolution_hierarchy_weight_delta: 0.0,
        replay_live_evolution_online_reward_feedbacks: 0,
        replay_live_evolution_online_reward_reinforcements: 0,
        replay_live_evolution_online_reward_penalties: 0,
        replay_live_evolution_online_reward_strength: 0.0,
        replay_live_evolution_online_reward_reinforcement_strength: 0.0,
        replay_live_evolution_online_reward_penalty_strength: 0.0,
        replay_live_evolution_memory_updates: 0,
        replay_live_evolution_stored_memory_updates: 0,
        replay_live_evolution_reflection_issues: 0,
        replay_live_evolution_critical_reflection_issues: 0,
        replay_live_evolution_revision_actions: 0,
        recursive_replay_items: fields[recursive_replay_index].parse::<u64>().ok()?,
        recursive_runtime_calls: fields[recursive_replay_index + 1].parse::<u64>().ok()?,
        drift_rollbacks: fields
            .get(rollback_index)
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0),
        rollback_router_threshold_delta: fields
            .get(rollback_index + 1)
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(0.0)
            .max(0.0),
        rollback_hierarchy_weight_delta: fields
            .get(rollback_index + 2)
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(0.0)
            .max(0.0),
    })
}

fn parse_nonnegative_f32(value: &str) -> Option<f32> {
    value
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
        .map(|value| value.max(0.0))
}

fn nonnegative_f32(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn parse_memory_placement(value: &str) -> Option<MemoryPlacement> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 4 {
        return None;
    }

    Some(MemoryPlacement {
        id: fields[0].parse::<u64>().ok()?,
        tier: fields[1].parse::<MemoryTier>().ok()?,
        score: fields[2].parse::<f32>().ok()?,
        reason: unescape_field(fields[3]),
    })
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn unescape_field(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn adaptive_state_roundtrips_through_disk_kv() {
        let path = temp_path("adaptive-state");
        let state = AdaptiveState {
            router: RouterState {
                threshold: 0.61,
                observations: 17,
                profile_thresholds: ProfileThresholds {
                    general: 0.61,
                    coding: 0.49,
                    writing: 0.66,
                    long_document: 0.42,
                },
                profile_observations: ProfileObservations {
                    general: 8,
                    coding: 5,
                    writing: 3,
                    long_document: 1,
                },
            },
            hierarchy: HierarchyState {
                current: HierarchyWeights::new(0.2, 0.6, 0.2),
                profile_weights: ProfileHierarchyWeights {
                    general: HierarchyWeights::new(0.36, 0.42, 0.22),
                    coding: HierarchyWeights::new(0.18, 0.68, 0.14),
                    writing: HierarchyWeights::new(0.60, 0.26, 0.14),
                    long_document: HierarchyWeights::new(0.24, 0.18, 0.58),
                },
                profile_observations: ProfileHierarchyObservations {
                    general: 2,
                    coding: 7,
                    writing: 5,
                    long_document: 3,
                },
            },
            tier_plan: TieredCachePlan::new(vec![MemoryPlacement {
                id: 7,
                tier: MemoryTier::WarmRam,
                score: 0.42,
                reason: "warm\tstate".to_owned(),
            }]),
            memory_retention_policy: MemoryRetentionPolicy {
                stale_after: 11,
                decay_rate: 0.12,
                remove_below_strength: 0.08,
                remove_after_failures: 7,
            },
            memory_compaction_policy: MemoryCompactionPolicy {
                similarity_threshold: 0.91,
                max_candidates: 64,
                max_merges: 4,
            },
            evolution_ledger: EvolutionLedger {
                live_inference_runs: 11,
                live_router_threshold_mutations: 8,
                live_hierarchy_weight_mutations: 6,
                live_router_threshold_delta: 0.19,
                live_hierarchy_weight_delta: 0.13,
                live_online_reward_feedbacks: 6,
                live_online_reward_reinforcements: 4,
                live_online_reward_penalties: 2,
                live_online_reward_strength: 3.25,
                live_online_reward_reinforcement_strength: 2.15,
                live_online_reward_penalty_strength: 1.10,
                live_memory_reinforcements: 9,
                live_memory_penalties: 4,
                live_stored_memories: 3,
                live_stored_gist_memories: 5,
                live_stored_runtime_kv_memories: 2,
                live_reflection_issues: 7,
                live_critical_reflection_issues: 1,
                live_revision_actions: 10,
                replay_runs: 3,
                replay_items: 9,
                router_threshold_mutations: 5,
                hierarchy_weight_mutations: 7,
                router_threshold_delta: 0.42,
                hierarchy_weight_delta: 0.21,
                memory_reinforcements: 4,
                memory_penalties: 2,
                replay_live_memory_feedback_items: 3,
                replay_live_memory_feedback_reinforcements: 5,
                replay_live_memory_feedback_penalties: 1,
                replay_live_memory_feedback_detail_items: 2,
                replay_live_memory_feedback_applied: 4,
                replay_live_memory_feedback_removed: 1,
                replay_live_memory_feedback_missing: 1,
                replay_live_memory_feedback_strength_delta: 0.72,
                replay_live_evolution_items: 4,
                replay_live_evolution_router_threshold_mutations: 2,
                replay_live_evolution_hierarchy_weight_mutations: 1,
                replay_live_evolution_router_threshold_delta: 0.08,
                replay_live_evolution_hierarchy_weight_delta: 0.05,
                replay_live_evolution_online_reward_feedbacks: 3,
                replay_live_evolution_online_reward_reinforcements: 2,
                replay_live_evolution_online_reward_penalties: 1,
                replay_live_evolution_online_reward_strength: 1.75,
                replay_live_evolution_online_reward_reinforcement_strength: 1.20,
                replay_live_evolution_online_reward_penalty_strength: 0.55,
                replay_live_evolution_memory_updates: 6,
                replay_live_evolution_stored_memory_updates: 3,
                replay_live_evolution_reflection_issues: 5,
                replay_live_evolution_critical_reflection_issues: 1,
                replay_live_evolution_revision_actions: 4,
                recursive_replay_items: 1,
                recursive_runtime_calls: 8,
                drift_rollbacks: 2,
                rollback_router_threshold_delta: 0.11,
                rollback_hierarchy_weight_delta: 0.09,
            },
        };

        state.save_to_disk_kv(&path).unwrap();
        let loaded = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();

        assert!((loaded.router.threshold - 0.61).abs() < 0.0001);
        assert_eq!(loaded.router.observations, 17);
        assert!((loaded.router.profile_thresholds.coding - 0.49).abs() < 0.0001);
        assert_eq!(loaded.router.profile_observations.writing, 3);
        assert!((loaded.hierarchy.current.local - 0.6).abs() < 0.0001);
        assert!((loaded.hierarchy.profile_weights.coding.local - 0.68).abs() < 0.0001);
        assert_eq!(loaded.hierarchy.profile_observations.long_document, 3);
        let placement = loaded.tier_plan.placement_for(7).unwrap();
        assert_eq!(placement.tier, MemoryTier::WarmRam);
        assert_eq!(placement.reason, "warm\tstate");
        assert_eq!(loaded.memory_retention_policy.stale_after, 11);
        assert!((loaded.memory_retention_policy.decay_rate - 0.12).abs() < 0.0001);
        assert!((loaded.memory_retention_policy.remove_below_strength - 0.08).abs() < 0.0001);
        assert_eq!(loaded.memory_retention_policy.remove_after_failures, 7);
        assert!((loaded.memory_compaction_policy.similarity_threshold - 0.91).abs() < 0.0001);
        assert_eq!(loaded.memory_compaction_policy.max_candidates, 64);
        assert_eq!(loaded.memory_compaction_policy.max_merges, 4);
        assert_eq!(loaded.evolution_ledger.replay_runs, 3);
        assert_eq!(loaded.evolution_ledger.live_inference_runs, 11);
        assert_eq!(loaded.evolution_ledger.live_router_threshold_mutations, 8);
        assert_eq!(loaded.evolution_ledger.live_hierarchy_weight_mutations, 6);
        assert!((loaded.evolution_ledger.live_router_threshold_delta - 0.19).abs() < 0.0001);
        assert!((loaded.evolution_ledger.live_hierarchy_weight_delta - 0.13).abs() < 0.0001);
        assert_eq!(loaded.evolution_ledger.live_online_reward_feedbacks, 6);
        assert_eq!(loaded.evolution_ledger.live_online_reward_reinforcements, 4);
        assert_eq!(loaded.evolution_ledger.live_online_reward_penalties, 2);
        assert!((loaded.evolution_ledger.live_online_reward_strength - 3.25).abs() < 0.0001);
        assert!(
            (loaded
                .evolution_ledger
                .live_online_reward_reinforcement_strength
                - 2.15)
                .abs()
                < 0.0001
        );
        assert!(
            (loaded.evolution_ledger.live_online_reward_penalty_strength - 1.10).abs() < 0.0001
        );
        assert_eq!(loaded.evolution_ledger.live_memory_updates(), 13);
        assert_eq!(loaded.evolution_ledger.live_stored_memory_updates(), 10);
        assert_eq!(loaded.evolution_ledger.live_reflection_issues, 7);
        assert_eq!(loaded.evolution_ledger.live_critical_reflection_issues, 1);
        assert_eq!(loaded.evolution_ledger.live_revision_actions, 10);
        assert_eq!(loaded.evolution_ledger.replay_items, 9);
        assert_eq!(loaded.evolution_ledger.router_threshold_mutations, 5);
        assert_eq!(loaded.evolution_ledger.hierarchy_weight_mutations, 7);
        assert!((loaded.evolution_ledger.router_threshold_delta - 0.42).abs() < 0.0001);
        assert!((loaded.evolution_ledger.hierarchy_weight_delta - 0.21).abs() < 0.0001);
        assert_eq!(loaded.evolution_ledger.memory_updates(), 6);
        assert_eq!(loaded.evolution_ledger.replay_live_memory_feedback_items, 3);
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_memory_feedback_updates(),
            6
        );
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_memory_feedback_reinforcements,
            5
        );
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_memory_feedback_penalties,
            1
        );
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_memory_feedback_detail_items,
            2
        );
        assert_eq!(
            loaded.evolution_ledger.replay_live_memory_feedback_applied,
            4
        );
        assert_eq!(
            loaded.evolution_ledger.replay_live_memory_feedback_removed,
            1
        );
        assert_eq!(
            loaded.evolution_ledger.replay_live_memory_feedback_missing,
            1
        );
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_memory_feedback_detail_updates(),
            5
        );
        assert!(
            (loaded
                .evolution_ledger
                .replay_live_memory_feedback_strength_delta
                - 0.72)
                .abs()
                < 0.0001
        );
        assert_eq!(loaded.evolution_ledger.replay_live_evolution_items, 4);
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_evolution_router_threshold_mutations,
            2
        );
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_evolution_hierarchy_weight_mutations,
            1
        );
        assert!(
            (loaded
                .evolution_ledger
                .replay_live_evolution_router_threshold_delta
                - 0.08)
                .abs()
                < 0.0001
        );
        assert!(
            (loaded
                .evolution_ledger
                .replay_live_evolution_hierarchy_weight_delta
                - 0.05)
                .abs()
                < 0.0001
        );
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_evolution_online_reward_feedbacks,
            3
        );
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_evolution_online_reward_reinforcements,
            2
        );
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_evolution_online_reward_penalties,
            1
        );
        assert!(
            (loaded
                .evolution_ledger
                .replay_live_evolution_online_reward_strength
                - 1.75)
                .abs()
                < 0.0001
        );
        assert!(
            (loaded
                .evolution_ledger
                .replay_live_evolution_online_reward_reinforcement_strength
                - 1.20)
                .abs()
                < 0.0001
        );
        assert!(
            (loaded
                .evolution_ledger
                .replay_live_evolution_online_reward_penalty_strength
                - 0.55)
                .abs()
                < 0.0001
        );
        assert_eq!(
            loaded.evolution_ledger.replay_live_evolution_memory_updates,
            6
        );
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_evolution_stored_memory_updates,
            3
        );
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_evolution_reflection_issues,
            5
        );
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_evolution_critical_reflection_issues,
            1
        );
        assert_eq!(
            loaded
                .evolution_ledger
                .replay_live_evolution_revision_actions,
            4
        );
        assert_eq!(loaded.evolution_ledger.recursive_replay_items, 1);
        assert_eq!(loaded.evolution_ledger.recursive_runtime_calls, 8);
        assert_eq!(loaded.evolution_ledger.drift_rollbacks, 2);
        assert!((loaded.evolution_ledger.rollback_router_threshold_delta - 0.11).abs() < 0.0001);
        assert!((loaded.evolution_ledger.rollback_hierarchy_weight_delta - 0.09).abs() < 0.0001);
        cleanup(path);
    }

    #[test]
    fn evolution_ledger_loads_legacy_without_rollback_fields() {
        let legacy = "3\t9\t5\t7\t0.420000\t0.210000\t4\t2\t1\t8";
        let ledger = parse_evolution_ledger(legacy).unwrap();

        assert_eq!(ledger.replay_runs, 3);
        assert_eq!(ledger.memory_updates(), 6);
        assert_eq!(ledger.replay_live_memory_feedback_items, 0);
        assert_eq!(ledger.replay_live_memory_feedback_updates(), 0);
        assert_eq!(ledger.replay_live_memory_feedback_detail_items, 0);
        assert_eq!(ledger.replay_live_memory_feedback_detail_updates(), 0);
        assert_eq!(ledger.live_online_reward_strength, 0.0);
        assert_eq!(ledger.live_online_reward_reinforcement_strength, 0.0);
        assert_eq!(ledger.live_online_reward_penalty_strength, 0.0);
        assert_eq!(ledger.replay_live_evolution_online_reward_strength, 0.0);
        assert_eq!(
            ledger.replay_live_evolution_online_reward_reinforcement_strength,
            0.0
        );
        assert_eq!(
            ledger.replay_live_evolution_online_reward_penalty_strength,
            0.0
        );
        assert_eq!(ledger.recursive_runtime_calls, 8);
        assert_eq!(ledger.drift_rollbacks, 0);
        assert_eq!(ledger.rollback_router_threshold_delta, 0.0);
        assert_eq!(ledger.rollback_hierarchy_weight_delta, 0.0);
    }

    #[test]
    fn adaptive_state_loads_legacy_files_without_memory_policies() {
        let path = temp_path("adaptive-state-legacy");
        {
            let mut store = DiskKvStore::open(&path).unwrap();
            store.put("adaptive/router", b"0.610000\t17").unwrap();
            store
                .put("adaptive/hierarchy", b"0.200000\t0.600000\t0.200000")
                .unwrap();
            store.compact().unwrap();
        }

        let loaded = AdaptiveState::load_from_disk_kv(&path).unwrap().unwrap();

        assert!((loaded.router.threshold - 0.61).abs() < 0.0001);
        assert_eq!(loaded.router.observations, 17);
        assert!((loaded.hierarchy.current.local - 0.6).abs() < 0.0001);
        assert_eq!(
            loaded.memory_retention_policy.stale_after,
            MemoryRetentionPolicy::default().stale_after
        );
        assert!(
            (loaded.memory_compaction_policy.similarity_threshold
                - MemoryCompactionPolicy::default().similarity_threshold)
                .abs()
                < 0.0001
        );
        assert_eq!(loaded.evolution_ledger, EvolutionLedger::default());
        cleanup(path);
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-norion-{label}-{}-{nanos}.ndkv",
            std::process::id()
        ))
    }

    fn cleanup(path: std::path::PathBuf) {
        let _ = fs::remove_file(path);
    }
}
