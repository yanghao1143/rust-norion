use crate::experience::{ExperienceRecord, recursive_runtime_calls_from_notes};
use crate::hierarchy::TaskProfile;
use crate::process_reward::RewardAction;
use crate::reflection::{ReflectionSeverity, RuntimeDiagnostics};
use crate::router::RouteBudget;

#[derive(Debug, Clone)]
pub struct ExperienceReplayPlanner {
    reinforce_threshold: f32,
    penalize_threshold: f32,
}

impl Default for ExperienceReplayPlanner {
    fn default() -> Self {
        Self {
            reinforce_threshold: 0.72,
            penalize_threshold: 0.42,
        }
    }
}

impl ExperienceReplayPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn plan(&self, records: &[ExperienceRecord], limit: usize) -> ExperienceReplayPlan {
        let mut items = records
            .iter()
            .filter_map(|record| self.item_for_record(record))
            .collect::<Vec<_>>();

        items.sort_by(|left, right| {
            right
                .priority
                .partial_cmp(&left.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.experience_id.cmp(&left.experience_id))
        });
        items.truncate(limit);

        ExperienceReplayPlan { items }
    }

    fn item_for_record(&self, record: &ExperienceRecord) -> Option<ExperienceReplayItem> {
        let reward = record.process_reward.total.clamp(0.0, 1.0);
        let action = if reward >= self.reinforce_threshold {
            RewardAction::Reinforce
        } else if reward <= self.penalize_threshold {
            RewardAction::Penalize
        } else {
            return None;
        };
        let priority = match action {
            RewardAction::Reinforce => reward,
            RewardAction::Penalize => 1.0 - reward + reflection_issue_priority(record),
            RewardAction::Hold => 0.0,
        }
        .clamp(0.0, 1.0);
        let mut memory_ids = record
            .used_memory_ids
            .iter()
            .copied()
            .chain(record.stored_memory_id)
            .chain(record.gist_memory_ids.iter().copied())
            .chain(record.stored_runtime_kv_memory_ids.iter().copied())
            .collect::<Vec<_>>();
        memory_ids.sort_unstable();
        memory_ids.dedup();

        Some(ExperienceReplayItem {
            experience_id: record.id,
            profile: record.profile,
            action,
            reward,
            quality: record.quality,
            contradiction_count: record
                .contradictions
                .len()
                .max(critical_reflection_issue_count(record)),
            reflection_issue_count: record.reflection_issues.len(),
            critical_reflection_issue_count: critical_reflection_issue_count(record),
            revision_action_count: record.revision_actions.len(),
            stream_windows: record.stream_windows,
            route_budget: record.route_budget,
            memory_ids,
            runtime_diagnostics: record.runtime_diagnostics.clone(),
            recursive_runtime_calls: recursive_runtime_calls_from_notes(
                &record.process_reward.notes,
            ),
            priority,
            lesson: record.lesson.clone(),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExperienceReplayPlan {
    pub items: Vec<ExperienceReplayItem>,
}

impl ExperienceReplayPlan {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct ExperienceReplayItem {
    pub experience_id: u64,
    pub profile: TaskProfile,
    pub action: RewardAction,
    pub reward: f32,
    pub quality: f32,
    pub contradiction_count: usize,
    pub reflection_issue_count: usize,
    pub critical_reflection_issue_count: usize,
    pub revision_action_count: usize,
    pub stream_windows: usize,
    pub route_budget: RouteBudget,
    pub memory_ids: Vec<u64>,
    pub runtime_diagnostics: RuntimeDiagnostics,
    pub recursive_runtime_calls: Option<usize>,
    pub priority: f32,
    pub lesson: String,
}

fn critical_reflection_issue_count(record: &ExperienceRecord) -> usize {
    record
        .reflection_issues
        .iter()
        .filter(|issue| issue.severity == ReflectionSeverity::Critical)
        .count()
}

fn reflection_issue_priority(record: &ExperienceRecord) -> f32 {
    record
        .reflection_issues
        .iter()
        .map(|issue| match issue.severity {
            ReflectionSeverity::Info => 0.01,
            ReflectionSeverity::Warning => 0.04,
            ReflectionSeverity::Critical => 0.12,
        })
        .sum::<f32>()
        .min(0.28)
}

#[derive(Debug, Clone, Default)]
pub struct ExperienceReplayReport {
    pub planned: usize,
    pub applied: usize,
    pub reinforced: usize,
    pub penalized: usize,
    pub touched_memories: usize,
    pub average_reward: f32,
    pub notes: Vec<String>,
}

impl ExperienceReplayReport {
    pub fn from_plan(plan: &ExperienceReplayPlan) -> Self {
        let average_reward = if plan.items.is_empty() {
            0.0
        } else {
            plan.items.iter().map(|item| item.reward).sum::<f32>() / plan.items.len() as f32
        };

        Self {
            planned: plan.items.len(),
            average_reward,
            ..Self::default()
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "planned={} applied={} reinforced={} penalized={} touched_memories={} average_reward={:.3}",
            self.planned,
            self.applied,
            self.reinforced,
            self.penalized,
            self.touched_memories,
            self.average_reward
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experience::ExperienceInput;
    use crate::hierarchy::HierarchyWeights;
    use crate::process_reward::{ProcessRewardComponents, ProcessRewardReport};
    use crate::reflection::ReflectionIssue;

    #[test]
    fn planner_selects_reinforce_and_penalize_records() {
        let planner = ExperienceReplayPlanner::new();
        let records = vec![
            record(1, 0.90, RewardAction::Reinforce),
            record(2, 0.50, RewardAction::Hold),
            record(3, 0.20, RewardAction::Penalize),
        ];

        let plan = planner.plan(&records, 8);

        assert_eq!(plan.items.len(), 2);
        assert!(
            plan.items
                .iter()
                .any(|item| item.action == RewardAction::Reinforce)
        );
        let reinforced = plan
            .items
            .iter()
            .find(|item| item.action == RewardAction::Reinforce)
            .unwrap();
        assert!(reinforced.memory_ids.contains(&1));
        assert!(reinforced.memory_ids.contains(&11));
        assert!(reinforced.memory_ids.contains(&21));
        assert!(reinforced.memory_ids.contains(&31));
        assert_eq!(
            reinforced.runtime_diagnostics.model_id.as_deref(),
            Some("replay-runtime")
        );
        assert_eq!(reinforced.runtime_diagnostics.forward_energy, Some(0.31));
        assert!(
            plan.items
                .iter()
                .any(|item| item.action == RewardAction::Penalize)
        );
        assert!(!plan.items.iter().any(|item| item.experience_id == 2));
        let penalized = plan
            .items
            .iter()
            .find(|item| item.action == RewardAction::Penalize)
            .unwrap();
        assert_eq!(penalized.critical_reflection_issue_count, 1);
        assert_eq!(penalized.revision_action_count, 1);
    }

    #[test]
    fn planner_honors_limit_and_priority() {
        let planner = ExperienceReplayPlanner::new();
        let records = vec![
            record(1, 0.73, RewardAction::Reinforce),
            record(2, 0.95, RewardAction::Reinforce),
            record(3, 0.01, RewardAction::Penalize),
        ];

        let plan = planner.plan(&records, 1);

        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].experience_id, 3);
    }

    fn record(id: u64, reward: f32, action: RewardAction) -> ExperienceRecord {
        let input = ExperienceInput {
            prompt: "replay prompt".to_owned(),
            profile: TaskProfile::Coding,
            lesson: "replay lesson".to_owned(),
            quality: reward,
            contradictions: if action == RewardAction::Penalize {
                vec!["bad".to_owned()]
            } else {
                Vec::new()
            },
            reflection_issues: if action == RewardAction::Penalize {
                vec![ReflectionIssue::new(
                    "bad",
                    ReflectionSeverity::Critical,
                    "bad replay issue",
                )]
            } else {
                Vec::new()
            },
            revision_actions: if action == RewardAction::Penalize {
                vec!["review_bad_replay".to_owned()]
            } else {
                Vec::new()
            },
            stored_memory_id: Some(id),
            router_threshold_after: 0.5,
            stream_windows: 2,
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 2,
                fast_tokens: 2,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            used_memory_ids: vec![id + 20],
            gist_records: Vec::new(),
            gist_memory_ids: vec![id + 10],
            stored_runtime_kv_memory_ids: vec![id + 30],
            runtime_diagnostics: RuntimeDiagnostics {
                model_id: Some("replay-runtime".to_owned()),
                selected_adapter: Some("portable-rust".to_owned()),
                layer_count: 12,
                hidden_size: 128,
                local_window_tokens: 4096,
                forward_energy: Some(0.31),
                kv_influence: Some(0.27),
                imported_kv_blocks: 1,
                exported_kv_blocks: 2,
            },
            process_reward: ProcessRewardReport {
                total: reward,
                action,
                components: ProcessRewardComponents::default(),
                notes: vec![
                    "recursive:chunks=4:merge_rounds=2:waves=2:parallel=2:runtime_calls=7"
                        .to_owned(),
                ],
            },
        };

        ExperienceRecord {
            id,
            prompt: input.prompt,
            profile: input.profile,
            lesson: input.lesson,
            quality: input.quality,
            contradictions: input.contradictions,
            reflection_issues: input.reflection_issues,
            revision_actions: input.revision_actions,
            stored_memory_id: input.stored_memory_id,
            router_threshold_after: input.router_threshold_after,
            stream_windows: input.stream_windows,
            route_budget: input.route_budget,
            hierarchy: input.hierarchy,
            used_memory_ids: input.used_memory_ids,
            gist_records: input.gist_records,
            gist_memory_ids: input.gist_memory_ids,
            stored_runtime_kv_memory_ids: input.stored_runtime_kv_memory_ids,
            runtime_diagnostics: input.runtime_diagnostics,
            process_reward: input.process_reward,
        }
    }

    #[test]
    fn planner_carries_recursive_runtime_calls() {
        let planner = ExperienceReplayPlanner::new();
        let records = vec![record(9, 0.88, RewardAction::Reinforce)];

        let plan = planner.plan(&records, 1);

        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].recursive_runtime_calls, Some(7));
    }
}
