use std::cmp::Ordering;

use crate::engine::NoironEngine;
use crate::hierarchy::{
    HierarchyWeights, ProfileHierarchyObservations, ProfileHierarchyWeights, TaskProfile,
};
use crate::process_reward::RewardAction;
use crate::router::{ProfileObservations, ProfileThresholds};
use crate::tiered_cache::TierCounts;

#[derive(Debug, Clone)]
pub struct StateMemorySummary {
    pub id: u64,
    pub key: String,
    pub strength: f32,
    pub hits: u64,
    pub failures: u64,
    pub last_score: f32,
}

#[derive(Debug, Clone)]
pub struct StateExperienceSummary {
    pub id: u64,
    pub profile: TaskProfile,
    pub quality: f32,
    pub process_reward: f32,
    pub reward_action: RewardAction,
    pub reflection_issues: usize,
    pub critical_reflection_issues: usize,
    pub revision_actions: usize,
    pub lesson: String,
}

#[derive(Debug, Clone)]
pub struct StateInspectionReport {
    pub memory_count: usize,
    pub experience_count: usize,
    pub router_threshold: f32,
    pub router_observations: u64,
    pub profile_thresholds: ProfileThresholds,
    pub profile_observations: ProfileObservations,
    pub hierarchy: HierarchyWeights,
    pub profile_hierarchy_weights: ProfileHierarchyWeights,
    pub profile_hierarchy_observations: ProfileHierarchyObservations,
    pub tier_counts: TierCounts,
    pub top_memories: Vec<StateMemorySummary>,
    pub top_experiences: Vec<StateExperienceSummary>,
}

impl StateInspectionReport {
    pub fn from_engine(engine: &NoironEngine, limit: usize) -> Self {
        let limit = limit.max(1);
        let adaptive_state = engine.adaptive_state();
        let mut top_memories = engine
            .cache
            .entries()
            .iter()
            .map(|entry| {
                let value_score =
                    entry.strength + entry.hits as f32 * 0.04 - entry.failures as f32 * 0.10;
                (value_score, entry)
            })
            .collect::<Vec<_>>();
        top_memories.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.1.id.cmp(&right.1.id))
        });

        let top_memories = top_memories
            .into_iter()
            .take(limit)
            .map(|(_, entry)| StateMemorySummary {
                id: entry.id,
                key: compact(&entry.key, 120),
                strength: entry.strength,
                hits: entry.hits,
                failures: entry.failures,
                last_score: entry.last_score,
            })
            .collect::<Vec<_>>();

        let mut top_experiences = engine.experience.records().iter().collect::<Vec<_>>();
        top_experiences.sort_by(|left, right| {
            right
                .process_reward
                .total
                .partial_cmp(&left.process_reward.total)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    right
                        .quality
                        .partial_cmp(&left.quality)
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| left.id.cmp(&right.id))
        });

        let top_experiences = top_experiences
            .into_iter()
            .take(limit)
            .map(|record| StateExperienceSummary {
                id: record.id,
                profile: record.profile,
                quality: record.quality,
                process_reward: record.process_reward.total,
                reward_action: record.process_reward.action,
                reflection_issues: record.reflection_issues.len(),
                critical_reflection_issues: record
                    .reflection_issues
                    .iter()
                    .filter(|issue| {
                        issue.severity == crate::reflection::ReflectionSeverity::Critical
                    })
                    .count(),
                revision_actions: record.revision_actions.len(),
                lesson: compact(&record.lesson, 160),
            })
            .collect::<Vec<_>>();

        Self {
            memory_count: engine.cache.len(),
            experience_count: engine.experience.len(),
            router_threshold: adaptive_state.router.threshold,
            router_observations: adaptive_state.router.observations,
            profile_thresholds: adaptive_state.router.profile_thresholds,
            profile_observations: adaptive_state.router.profile_observations,
            hierarchy: adaptive_state.hierarchy.current,
            profile_hierarchy_weights: adaptive_state.hierarchy.profile_weights,
            profile_hierarchy_observations: adaptive_state.hierarchy.profile_observations,
            tier_counts: adaptive_state.tier_plan.counts(),
            top_memories,
            top_experiences,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "state: memories={} experiences={} router_threshold={:.3} router_observations={} profile_thresholds=(general:{:.3},coding:{:.3},writing:{:.3},long:{:.3}) hierarchy=({:.2},{:.2},{:.2}) profile_hierarchy_local=(general:{:.2},coding:{:.2},writing:{:.2},long:{:.2}) tiers=({},{},{})",
            self.memory_count,
            self.experience_count,
            self.router_threshold,
            self.router_observations,
            self.profile_thresholds.general,
            self.profile_thresholds.coding,
            self.profile_thresholds.writing,
            self.profile_thresholds.long_document,
            self.hierarchy.global,
            self.hierarchy.local,
            self.hierarchy.convolution,
            self.profile_hierarchy_weights.general.local,
            self.profile_hierarchy_weights.coding.local,
            self.profile_hierarchy_weights.writing.local,
            self.profile_hierarchy_weights.long_document.local,
            self.tier_counts.hot_gpu,
            self.tier_counts.warm_ram,
            self.tier_counts.cold_disk
        )
    }
}

fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::NoironEngine;
    use crate::experience::ExperienceInput;
    use crate::hierarchy::{HierarchyWeights, TaskProfile};
    use crate::process_reward::{ProcessRewardComponents, ProcessRewardReport, RewardAction};
    use crate::reflection::{ReflectionIssue, ReflectionSeverity};
    use crate::router::RouteBudget;

    #[test]
    fn inspection_report_summarizes_memory_experience_and_adaptive_state() {
        let mut engine = NoironEngine::new();
        let memory_id =
            engine
                .cache
                .store_or_fuse("inspectable reinforced memory", vec![1.0, 0.0, 0.0], 0.9);
        engine.cache.reinforce(memory_id, 0.8);
        engine.experience.record(ExperienceInput {
            prompt: "inspect state".to_owned(),
            profile: TaskProfile::Coding,
            lesson: "state inspection should expose learned control decisions".to_owned(),
            quality: 0.91,
            contradictions: Vec::new(),
            reflection_issues: vec![ReflectionIssue::new(
                "needs_grounding",
                ReflectionSeverity::Warning,
                "inspect warning",
            )],
            revision_actions: vec!["increase_prompt_grounding".to_owned()],
            stored_memory_id: Some(memory_id),
            router_threshold_after: 0.62,
            stream_windows: 2,
            route_budget: RouteBudget {
                threshold: 0.62,
                attention_tokens: 2,
                fast_tokens: 2,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            used_memory_ids: vec![memory_id],
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            process_reward: ProcessRewardReport {
                total: 0.88,
                action: RewardAction::Reinforce,
                components: ProcessRewardComponents::default(),
                notes: Vec::new(),
            },
        });

        let report = StateInspectionReport::from_engine(&engine, 3);

        assert_eq!(report.memory_count, 1);
        assert_eq!(report.experience_count, 1);
        assert_eq!(report.top_memories[0].id, memory_id);
        assert!(report.top_memories[0].key.contains("inspectable"));
        assert_eq!(
            report.top_experiences[0].reward_action,
            RewardAction::Reinforce
        );
        assert_eq!(report.top_experiences[0].reflection_issues, 1);
        assert_eq!(report.top_experiences[0].revision_actions, 1);
        assert!(report.summary_line().contains("memories=1"));
    }
}
