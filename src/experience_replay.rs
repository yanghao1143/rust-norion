use crate::adaptive_state::LiveInferenceEvolution;
use crate::experience::{ExperienceRecord, recursive_runtime_calls_from_notes};
use crate::hierarchy::TaskProfile;
use crate::kv_cache::{MemoryUpdateAction, MemoryUpdateReport};
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

        sort_replay_items(&mut items);
        if limit == 0 {
            items.clear();
        } else if items.len() > limit {
            let overflow = items.iter().skip(limit).cloned().collect::<Vec<_>>();
            let recursive_candidate = overflow
                .iter()
                .find(|item| item.recursive_runtime_calls.is_some())
                .cloned();
            let live_evolution_candidate = overflow
                .iter()
                .find(|item| item.live_evolution.has_evidence())
                .cloned();
            items.truncate(limit);

            if !items
                .iter()
                .any(|item| item.recursive_runtime_calls.is_some())
            {
                if let Some(recursive_item) = recursive_candidate {
                    if let Some((replace_index, _)) = items
                        .iter()
                        .enumerate()
                        .filter(|(_, item)| item.recursive_runtime_calls.is_none())
                        .min_by(|(_, left), (_, right)| {
                            left.priority
                                .partial_cmp(&right.priority)
                                .unwrap_or(std::cmp::Ordering::Equal)
                                .then_with(|| left.experience_id.cmp(&right.experience_id))
                        })
                    {
                        items[replace_index] = recursive_item;
                        sort_replay_items(&mut items);
                    }
                }
            }
            if !items.iter().any(|item| item.live_evolution.has_evidence()) {
                if let Some(live_evolution_item) = live_evolution_candidate {
                    let has_recursive_item = items
                        .iter()
                        .any(|item| item.recursive_runtime_calls.is_some());
                    if let Some((replace_index, _)) = items
                        .iter()
                        .enumerate()
                        .filter(|(_, item)| !item.live_evolution.has_evidence())
                        .filter(|(_, item)| {
                            !has_recursive_item || item.recursive_runtime_calls.is_none()
                        })
                        .min_by(|(_, left), (_, right)| {
                            left.priority
                                .partial_cmp(&right.priority)
                                .unwrap_or(std::cmp::Ordering::Equal)
                                .then_with(|| left.experience_id.cmp(&right.experience_id))
                        })
                    {
                        items[replace_index] = live_evolution_item;
                        sort_replay_items(&mut items);
                    }
                }
            }
        }

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

        let recursive_stats = RecursiveReplayStats::from_notes(&record.process_reward.notes);
        let live_memory_feedback =
            LiveMemoryFeedbackStats::from_notes(&record.process_reward.notes);

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
            live_evolution: record.live_evolution,
            recursive_runtime_calls: recursive_stats
                .and_then(|stats| stats.runtime_calls)
                .or_else(|| recursive_runtime_calls_from_notes(&record.process_reward.notes)),
            recursive_stats,
            live_memory_feedback,
            priority,
            lesson: record.lesson.clone(),
        })
    }
}

fn sort_replay_items(items: &mut [ExperienceReplayItem]) {
    items.sort_by(|left, right| {
        right
            .priority
            .partial_cmp(&left.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.experience_id.cmp(&left.experience_id))
    });
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
    pub live_evolution: LiveInferenceEvolution,
    pub recursive_runtime_calls: Option<usize>,
    pub recursive_stats: Option<RecursiveReplayStats>,
    pub live_memory_feedback: Option<LiveMemoryFeedbackStats>,
    pub priority: f32,
    pub lesson: String,
}

impl ExperienceReplayItem {
    pub fn route_token_count(&self) -> usize {
        (self.route_budget.attention_tokens + self.route_budget.fast_tokens).max(1)
    }

    pub fn recursive_call_pressure(&self) -> f32 {
        recursive_call_pressure(
            self.recursive_runtime_calls,
            self.recursive_stats,
            self.route_token_count(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveMemoryFeedbackStats {
    pub reinforced: usize,
    pub penalized: usize,
    pub reinforcement_amount: f32,
    pub penalty_amount: f32,
    pub applied: usize,
    pub removed: usize,
    pub missing: usize,
    pub strength_delta: f32,
    pub detailed_evidence: bool,
}

impl LiveMemoryFeedbackStats {
    pub fn from_notes(notes: &[String]) -> Option<Self> {
        notes
            .iter()
            .filter(|note| note.starts_with("memory_feedback:"))
            .find_map(|note| {
                let applied = memory_feedback_note_usize(note, "applied=");
                let removed = memory_feedback_note_usize(note, "removed=");
                let missing = memory_feedback_note_usize(note, "missing=");
                let strength_delta = memory_feedback_note_f32(note, "strength_delta=");
                let stats = Self {
                    reinforced: memory_feedback_note_usize(note, "reinforced=").unwrap_or(0),
                    penalized: memory_feedback_note_usize(note, "penalized=").unwrap_or(0),
                    reinforcement_amount: memory_feedback_note_f32(note, "reinforcement_amount=")
                        .unwrap_or(0.0),
                    penalty_amount: memory_feedback_note_f32(note, "penalty_amount=")
                        .unwrap_or(0.0),
                    applied: applied.unwrap_or(0),
                    removed: removed.unwrap_or(0),
                    missing: missing.unwrap_or(0),
                    strength_delta: strength_delta.unwrap_or(0.0),
                    detailed_evidence: applied.is_some()
                        && removed.is_some()
                        && missing.is_some()
                        && strength_delta.is_some(),
                };

                stats.has_updates().then_some(stats)
            })
    }

    pub fn updates(&self) -> usize {
        self.reinforced + self.penalized
    }

    pub fn reinforcement_average(&self) -> Option<f32> {
        (self.reinforced > 0).then(|| self.reinforcement_amount / self.reinforced as f32)
    }

    pub fn penalty_average(&self) -> Option<f32> {
        (self.penalized > 0).then(|| self.penalty_amount / self.penalized as f32)
    }

    pub fn has_detailed_update_evidence(&self) -> bool {
        self.detailed_evidence
            && self.applied.saturating_add(self.missing) == self.updates()
            && self.removed <= self.applied
            && self.strength_delta.is_finite()
            && self.strength_delta >= 0.0
    }

    pub fn applied_ratio(&self) -> Option<f32> {
        (self.has_detailed_update_evidence() && self.updates() > 0)
            .then(|| self.applied as f32 / self.updates() as f32)
    }

    fn has_updates(&self) -> bool {
        self.updates() > 0
            && self.reinforcement_amount.is_finite()
            && self.penalty_amount.is_finite()
    }
}

fn memory_feedback_note_usize(note: &str, key: &str) -> Option<usize> {
    note.split(':')
        .find_map(|part| part.strip_prefix(key))
        .and_then(|value| value.parse::<usize>().ok())
}

fn memory_feedback_note_f32(note: &str, key: &str) -> Option<f32> {
    note.split(':')
        .find_map(|part| part.strip_prefix(key))
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecursiveReplayStats {
    pub chunks: Option<usize>,
    pub merge_rounds: Option<usize>,
    pub waves: Option<usize>,
    pub parallel: Option<usize>,
    pub runtime_calls: Option<usize>,
}

impl RecursiveReplayStats {
    pub fn from_notes(notes: &[String]) -> Option<Self> {
        notes
            .iter()
            .filter(|note| note.starts_with("recursive:"))
            .find_map(|note| {
                let stats = Self {
                    chunks: recursive_note_value(note, "chunks="),
                    merge_rounds: recursive_note_value(note, "merge_rounds="),
                    waves: recursive_note_value(note, "waves="),
                    parallel: recursive_note_value(note, "parallel="),
                    runtime_calls: recursive_note_value(note, "runtime_calls="),
                };

                (stats.chunks.is_some()
                    || stats.merge_rounds.is_some()
                    || stats.waves.is_some()
                    || stats.parallel.is_some()
                    || stats.runtime_calls.is_some())
                .then_some(stats)
            })
    }
}

fn recursive_note_value(note: &str, key: &str) -> Option<usize> {
    note.split(':')
        .find_map(|part| part.strip_prefix(key))
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
}

fn recursive_call_pressure(
    recursive_runtime_calls: Option<usize>,
    recursive_stats: Option<RecursiveReplayStats>,
    token_count: usize,
) -> f32 {
    let Some(calls) = recursive_runtime_calls else {
        return 0.0;
    };

    let expected_calls = recursive_stats
        .and_then(|stats| stats.chunks)
        .unwrap_or_else(|| token_count.max(1))
        .max(1);
    if calls <= expected_calls {
        return 0.0;
    }

    let excess_pressure =
        calls.saturating_sub(expected_calls) as f32 / (expected_calls.max(4) * 3) as f32;
    let wave_pressure = recursive_stats
        .and_then(|stats| stats.waves)
        .map(|waves| (waves.saturating_sub(1) as f32 / 48.0).min(0.10))
        .unwrap_or(0.0);
    let parallel_relief = recursive_stats
        .and_then(|stats| stats.parallel)
        .map(|parallel| ((parallel.saturating_sub(1) as f32) * 0.015).min(0.05))
        .unwrap_or(0.0);

    (excess_pressure + wave_pressure - parallel_relief).clamp(0.0, 0.35)
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
    pub router_updates: usize,
    pub hierarchy_updates: usize,
    pub router_threshold_mutations: usize,
    pub hierarchy_weight_mutations: usize,
    pub router_threshold_delta: f32,
    pub hierarchy_weight_delta: f32,
    pub reinforced: usize,
    pub penalized: usize,
    pub touched_memories: usize,
    pub memory_reinforcements: usize,
    pub memory_penalties: usize,
    pub memory_update_reports: Vec<MemoryUpdateReport>,
    pub applied_memory_updates: usize,
    pub removed_memory_updates: usize,
    pub missing_memory_updates: usize,
    pub memory_strength_delta: f32,
    pub average_reward: f32,
    pub recursive_runtime_items: usize,
    pub recursive_runtime_calls: usize,
    pub average_recursive_call_pressure: f32,
    pub max_recursive_call_pressure: f32,
    pub live_memory_feedback_items: usize,
    pub live_memory_feedback_updates: usize,
    pub live_memory_feedback_reinforcements: usize,
    pub live_memory_feedback_penalties: usize,
    pub live_memory_feedback_detail_items: usize,
    pub live_memory_feedback_applied: usize,
    pub live_memory_feedback_removed: usize,
    pub live_memory_feedback_missing: usize,
    pub live_memory_feedback_strength_delta: f32,
    pub live_evolution_items: usize,
    pub live_evolution_router_threshold_mutations: usize,
    pub live_evolution_hierarchy_weight_mutations: usize,
    pub live_evolution_router_threshold_delta: f32,
    pub live_evolution_hierarchy_weight_delta: f32,
    pub live_evolution_memory_updates: usize,
    pub live_evolution_stored_memory_updates: usize,
    pub live_evolution_reflection_issues: usize,
    pub live_evolution_critical_reflection_issues: usize,
    pub live_evolution_revision_actions: usize,
    pub notes: Vec<String>,
}

impl ExperienceReplayReport {
    pub fn from_plan(plan: &ExperienceReplayPlan) -> Self {
        let average_reward = if plan.items.is_empty() {
            0.0
        } else {
            plan.items.iter().map(|item| item.reward).sum::<f32>() / plan.items.len() as f32
        };
        let recursive_runtime_items = plan
            .items
            .iter()
            .filter(|item| item.recursive_runtime_calls.is_some())
            .count();
        let recursive_runtime_calls = plan
            .items
            .iter()
            .filter_map(|item| item.recursive_runtime_calls)
            .sum();
        let recursive_call_pressure_total = plan
            .items
            .iter()
            .map(ExperienceReplayItem::recursive_call_pressure)
            .sum::<f32>();
        let average_recursive_call_pressure = if plan.items.is_empty() {
            0.0
        } else {
            recursive_call_pressure_total / plan.items.len() as f32
        };
        let max_recursive_call_pressure = plan
            .items
            .iter()
            .map(ExperienceReplayItem::recursive_call_pressure)
            .fold(0.0_f32, f32::max);
        let live_memory_feedback_items = plan
            .items
            .iter()
            .filter(|item| item.live_memory_feedback.is_some())
            .count();
        let live_memory_feedback_reinforcements = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .map(|feedback| feedback.reinforced)
            .sum();
        let live_memory_feedback_penalties = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .map(|feedback| feedback.penalized)
            .sum();
        let live_memory_feedback_updates =
            live_memory_feedback_reinforcements + live_memory_feedback_penalties;
        let live_memory_feedback_detail_items = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .filter(LiveMemoryFeedbackStats::has_detailed_update_evidence)
            .count();
        let live_memory_feedback_applied = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .map(|feedback| feedback.applied)
            .sum();
        let live_memory_feedback_removed = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .map(|feedback| feedback.removed)
            .sum();
        let live_memory_feedback_missing = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .map(|feedback| feedback.missing)
            .sum();
        let live_memory_feedback_strength_delta = plan
            .items
            .iter()
            .filter_map(|item| item.live_memory_feedback)
            .map(|feedback| feedback.strength_delta)
            .sum();
        let live_evolution_items = plan
            .items
            .iter()
            .filter(|item| item.live_evolution.has_evidence())
            .count();
        let live_evolution_router_threshold_mutations = plan
            .items
            .iter()
            .filter(|item| item.live_evolution.router_threshold_delta > 0.000001)
            .count();
        let live_evolution_hierarchy_weight_mutations = plan
            .items
            .iter()
            .filter(|item| item.live_evolution.hierarchy_weight_delta > 0.000001)
            .count();
        let live_evolution_router_threshold_delta = plan
            .items
            .iter()
            .map(|item| item.live_evolution.router_threshold_delta.max(0.0))
            .sum();
        let live_evolution_hierarchy_weight_delta = plan
            .items
            .iter()
            .map(|item| item.live_evolution.hierarchy_weight_delta.max(0.0))
            .sum();
        let live_evolution_memory_updates = plan
            .items
            .iter()
            .map(|item| item.live_evolution.memory_updates())
            .sum();
        let live_evolution_stored_memory_updates = plan
            .items
            .iter()
            .map(|item| item.live_evolution.stored_memory_updates())
            .sum();
        let live_evolution_reflection_issues = plan
            .items
            .iter()
            .map(|item| item.live_evolution.reflection_issues)
            .sum();
        let live_evolution_critical_reflection_issues = plan
            .items
            .iter()
            .map(|item| item.live_evolution.critical_reflection_issues)
            .sum();
        let live_evolution_revision_actions = plan
            .items
            .iter()
            .map(|item| item.live_evolution.revision_actions)
            .sum();

        Self {
            planned: plan.items.len(),
            average_reward,
            recursive_runtime_items,
            recursive_runtime_calls,
            average_recursive_call_pressure,
            max_recursive_call_pressure,
            live_memory_feedback_items,
            live_memory_feedback_updates,
            live_memory_feedback_reinforcements,
            live_memory_feedback_penalties,
            live_memory_feedback_detail_items,
            live_memory_feedback_applied,
            live_memory_feedback_removed,
            live_memory_feedback_missing,
            live_memory_feedback_strength_delta,
            live_evolution_items,
            live_evolution_router_threshold_mutations,
            live_evolution_hierarchy_weight_mutations,
            live_evolution_router_threshold_delta,
            live_evolution_hierarchy_weight_delta,
            live_evolution_memory_updates,
            live_evolution_stored_memory_updates,
            live_evolution_reflection_issues,
            live_evolution_critical_reflection_issues,
            live_evolution_revision_actions,
            ..Self::default()
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "planned={} applied={} router_updates={} hierarchy_updates={} router_threshold_mutations={} hierarchy_weight_mutations={} router_threshold_delta={:.6} hierarchy_weight_delta={:.6} reinforced={} penalized={} touched_memories={} memory_reinforcements={} memory_penalties={} applied_memory_updates={} removed_memory_updates={} missing_memory_updates={} memory_strength_delta={:.6} average_reward={:.3} recursive_runtime_items={} recursive_runtime_calls={} avg_recursive_call_pressure={:.3} max_recursive_call_pressure={:.3} live_memory_feedback_items={} live_memory_feedback_updates={} live_memory_feedback_reinforcements={} live_memory_feedback_penalties={} live_memory_feedback_detail_items={} live_memory_feedback_applied={} live_memory_feedback_removed={} live_memory_feedback_missing={} live_memory_feedback_strength_delta={:.6} live_evolution_items={} live_evolution_router_threshold_mutations={} live_evolution_hierarchy_weight_mutations={} live_evolution_router_threshold_delta={:.6} live_evolution_hierarchy_weight_delta={:.6} live_evolution_memory_updates={} live_evolution_stored_memory_updates={} live_evolution_reflection_issues={} live_evolution_critical_reflection_issues={} live_evolution_revision_actions={}",
            self.planned,
            self.applied,
            self.router_updates,
            self.hierarchy_updates,
            self.router_threshold_mutations,
            self.hierarchy_weight_mutations,
            self.router_threshold_delta,
            self.hierarchy_weight_delta,
            self.reinforced,
            self.penalized,
            self.touched_memories,
            self.memory_reinforcements,
            self.memory_penalties,
            self.applied_memory_updates,
            self.removed_memory_updates,
            self.missing_memory_updates,
            self.memory_strength_delta,
            self.average_reward,
            self.recursive_runtime_items,
            self.recursive_runtime_calls,
            self.average_recursive_call_pressure,
            self.max_recursive_call_pressure,
            self.live_memory_feedback_items,
            self.live_memory_feedback_updates,
            self.live_memory_feedback_reinforcements,
            self.live_memory_feedback_penalties,
            self.live_memory_feedback_detail_items,
            self.live_memory_feedback_applied,
            self.live_memory_feedback_removed,
            self.live_memory_feedback_missing,
            self.live_memory_feedback_strength_delta,
            self.live_evolution_items,
            self.live_evolution_router_threshold_mutations,
            self.live_evolution_hierarchy_weight_mutations,
            self.live_evolution_router_threshold_delta,
            self.live_evolution_hierarchy_weight_delta,
            self.live_evolution_memory_updates,
            self.live_evolution_stored_memory_updates,
            self.live_evolution_reflection_issues,
            self.live_evolution_critical_reflection_issues,
            self.live_evolution_revision_actions
        )
    }

    pub fn record_memory_update(&mut self, update: MemoryUpdateReport) {
        self.touched_memories += 1;
        match update.action {
            MemoryUpdateAction::Reinforce => self.memory_reinforcements += 1,
            MemoryUpdateAction::Penalize => self.memory_penalties += 1,
        }
        if update.was_applied() {
            self.applied_memory_updates += 1;
        } else {
            self.missing_memory_updates += 1;
        }
        if update.removed {
            self.removed_memory_updates += 1;
        }
        self.memory_strength_delta += update.strength_delta.abs();
        self.memory_update_reports.push(update);
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
        assert_eq!(
            reinforced.live_memory_feedback,
            Some(LiveMemoryFeedbackStats {
                reinforced: 2,
                penalized: 0,
                reinforcement_amount: 1.2,
                penalty_amount: 0.0,
                applied: 2,
                removed: 0,
                missing: 0,
                strength_delta: 0.42,
                detailed_evidence: true,
            })
        );
        assert_eq!(
            reinforced.recursive_stats,
            Some(RecursiveReplayStats {
                chunks: Some(4),
                merge_rounds: Some(2),
                waves: Some(2),
                parallel: Some(2),
                runtime_calls: Some(7),
            })
        );
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

    #[test]
    fn planner_keeps_recursive_runtime_sample_when_limit_allows() {
        let planner = ExperienceReplayPlanner::new();
        let mut recursive = record(5, 0.80, RewardAction::Reinforce);
        recursive.profile = TaskProfile::LongDocument;
        recursive.live_evolution = LiveInferenceEvolution::default();
        recursive.process_reward.notes = vec![
            "recursive:chunks=32:merge_rounds=2:waves=8:parallel=2:runtime_calls=96".to_owned(),
        ];
        let mut high_priority = record(1, 0.96, RewardAction::Reinforce);
        high_priority.process_reward.notes.clear();
        high_priority.live_evolution = LiveInferenceEvolution::default();
        let mut second_priority = record(2, 0.95, RewardAction::Reinforce);
        second_priority.process_reward.notes.clear();
        second_priority.live_evolution = LiveInferenceEvolution::default();
        let records = vec![high_priority, second_priority, recursive];

        let plan = planner.plan(&records, 2);

        assert_eq!(plan.items.len(), 2);
        assert!(
            plan.items
                .iter()
                .any(|item| item.recursive_runtime_calls == Some(96))
        );
        assert!(
            plan.items
                .iter()
                .find(|item| item.recursive_runtime_calls == Some(96))
                .unwrap()
                .recursive_call_pressure()
                > 0.0
        );
        assert!(
            plan.items
                .iter()
                .any(|item| item.experience_id == 1 || item.experience_id == 2)
        );
    }

    #[test]
    fn planner_keeps_live_evolution_sample_when_limit_allows() {
        let planner = ExperienceReplayPlanner::new();
        let mut high_priority = record(1, 0.98, RewardAction::Reinforce);
        high_priority.live_evolution = LiveInferenceEvolution::default();
        high_priority.process_reward.notes.clear();
        let mut second_priority = record(2, 0.97, RewardAction::Reinforce);
        second_priority.live_evolution = LiveInferenceEvolution::default();
        second_priority.process_reward.notes.clear();
        let mut live_evolution = record(3, 0.80, RewardAction::Reinforce);
        live_evolution.live_evolution.revision_actions = 1;
        live_evolution.process_reward.notes.clear();
        let records = vec![high_priority, second_priority, live_evolution];

        let plan = planner.plan(&records, 2);

        assert_eq!(plan.items.len(), 2);
        assert!(
            plan.items
                .iter()
                .any(|item| item.live_evolution.has_evidence())
        );
        assert!(plan.items.iter().any(|item| item.experience_id == 1));
    }

    #[test]
    fn planner_does_not_displace_only_recursive_sample_for_live_evolution_at_tiny_limit() {
        let planner = ExperienceReplayPlanner::new();
        let mut recursive = record(5, 0.80, RewardAction::Reinforce);
        recursive.profile = TaskProfile::LongDocument;
        recursive.live_evolution = LiveInferenceEvolution::default();
        recursive.process_reward.notes = vec![
            "recursive:chunks=32:merge_rounds=2:waves=8:parallel=2:runtime_calls=96".to_owned(),
        ];
        let mut high_priority = record(1, 0.98, RewardAction::Reinforce);
        high_priority.live_evolution = LiveInferenceEvolution::default();
        high_priority.process_reward.notes.clear();
        let mut live_evolution = record(3, 0.80, RewardAction::Reinforce);
        live_evolution.live_evolution.revision_actions = 1;
        live_evolution.process_reward.notes.clear();
        let records = vec![high_priority, recursive, live_evolution];

        let plan = planner.plan(&records, 1);

        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].recursive_runtime_calls, Some(96));
        assert!(!plan.items[0].live_evolution.has_evidence());
    }

    #[test]
    fn recursive_pressure_uses_schedule_stats_not_route_token_count() {
        let planner = ExperienceReplayPlanner::new();
        let mut recursive = record(7, 0.88, RewardAction::Reinforce);
        recursive.route_budget.fast_tokens = 2_222;
        recursive.route_budget.attention_tokens = 0;
        recursive.process_reward.notes = vec![
            "recursive:chunks=89:merge_rounds=4:waves=23:parallel=4:runtime_calls=121".to_owned(),
        ];

        let plan = planner.plan(&[recursive], 1);
        let item = &plan.items[0];

        assert_eq!(item.recursive_runtime_calls, Some(121));
        assert_eq!(item.recursive_stats.unwrap().chunks, Some(89));
        assert!(item.route_token_count() > 2_000);
        assert!(item.recursive_call_pressure() > 0.0);
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
                device_profile: Some("cpu".to_owned()),
                primary_lane: Some("cpu-vector".to_owned()),
                fallback_lane: Some("cpu-portable".to_owned()),
                memory_mode: Some("tiered-disk".to_owned()),
                layer_count: 12,
                global_layers: 3,
                local_window_layers: 6,
                convolutional_fusion_layers: 3,
                hidden_size: 128,
                local_window_tokens: 4096,
                forward_energy: Some(0.31),
                kv_influence: Some(0.27),
                imported_kv_blocks: 1,
                exported_kv_blocks: 2,
                hot_kv_precision_bits: Some(8),
                cold_kv_precision_bits: Some(4),
            },
            runtime_token_metrics: Default::default(),
            process_reward: ProcessRewardReport {
                total: reward,
                action,
                components: ProcessRewardComponents::default(),
                notes: vec![
                    "recursive:chunks=4:merge_rounds=2:waves=2:parallel=2:runtime_calls=7"
                        .to_owned(),
                    "memory_feedback:reinforced=2:penalized=0:reinforcement_amount=1.200000:penalty_amount=0.000000:applied=2:removed=0:missing=0:strength_delta=0.420000"
                        .to_owned(),
                ],
            },
            live_evolution: LiveInferenceEvolution {
                router_threshold_delta: 0.02,
                hierarchy_weight_delta: 0.03,
                memory_reinforcements: 2,
                memory_penalties: 0,
                stored_memory: true,
                stored_gist_memories: 1,
                stored_runtime_kv_memories: 1,
                reflection_issues: if action == RewardAction::Penalize { 1 } else { 0 },
                critical_reflection_issues: if action == RewardAction::Penalize { 1 } else { 0 },
                revision_actions: if action == RewardAction::Penalize { 1 } else { 0 },
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
            runtime_token_metrics: input.runtime_token_metrics,
            process_reward: input.process_reward,
            live_evolution: input.live_evolution,
        }
    }

    #[test]
    fn planner_carries_recursive_runtime_calls() {
        let planner = ExperienceReplayPlanner::new();
        let records = vec![record(9, 0.88, RewardAction::Reinforce)];

        let plan = planner.plan(&records, 1);

        assert_eq!(plan.items.len(), 1);
        assert_eq!(plan.items[0].recursive_runtime_calls, Some(7));
        assert_eq!(plan.items[0].recursive_stats.unwrap().chunks, Some(4));
        assert_eq!(plan.items[0].live_evolution.memory_reinforcements, 2);
        assert_eq!(plan.items[0].live_evolution.stored_runtime_kv_memories, 1);
    }

    #[test]
    fn report_summarizes_structured_live_evolution_consumed_by_replay() {
        let planner = ExperienceReplayPlanner::new();
        let reinforced = record(9, 0.88, RewardAction::Reinforce);
        let penalized = record(10, 0.12, RewardAction::Penalize);
        let plan = planner.plan(&[reinforced, penalized], 4);

        let report = ExperienceReplayReport::from_plan(&plan);

        assert_eq!(report.live_evolution_items, 2);
        assert_eq!(report.live_evolution_router_threshold_mutations, 2);
        assert_eq!(report.live_evolution_hierarchy_weight_mutations, 2);
        assert!((report.live_evolution_router_threshold_delta - 0.04).abs() < 0.0001);
        assert!((report.live_evolution_hierarchy_weight_delta - 0.06).abs() < 0.0001);
        assert_eq!(report.live_evolution_memory_updates, 4);
        assert_eq!(report.live_evolution_stored_memory_updates, 6);
        assert_eq!(report.live_evolution_reflection_issues, 1);
        assert_eq!(report.live_evolution_critical_reflection_issues, 1);
        assert_eq!(report.live_evolution_revision_actions, 1);
        assert!(report.summary().contains("live_evolution_items=2"));
        assert!(report.summary().contains("live_evolution_memory_updates=4"));
    }

    #[test]
    fn report_summarizes_recursive_call_pressure() {
        let planner = ExperienceReplayPlanner::new();
        let mut high_cost = record(9, 0.88, RewardAction::Reinforce);
        high_cost.process_reward.notes = vec![
            "recursive:chunks=32:merge_rounds=2:waves=8:parallel=2:runtime_calls=96".to_owned(),
        ];
        let plan = planner.plan(&[high_cost], 1);

        let report = ExperienceReplayReport::from_plan(&plan);

        assert_eq!(report.recursive_runtime_items, 1);
        assert_eq!(report.recursive_runtime_calls, 96);
        assert!(report.average_recursive_call_pressure > 0.0);
        assert_eq!(
            report.average_recursive_call_pressure,
            report.max_recursive_call_pressure
        );
        assert!(report.summary().contains("router_updates=0"));
        assert!(report.summary().contains("hierarchy_updates=0"));
        assert!(report.summary().contains("memory_reinforcements=0"));
        assert!(report.summary().contains("memory_penalties=0"));
        assert!(report.summary().contains("recursive_runtime_calls=96"));
        assert!(report.summary().contains("max_recursive_call_pressure="));
    }

    #[test]
    fn report_summarizes_live_memory_feedback_consumed_by_replay() {
        let planner = ExperienceReplayPlanner::new();
        let reinforced = record(9, 0.88, RewardAction::Reinforce);
        let mut penalized = record(10, 0.12, RewardAction::Penalize);
        penalized.process_reward.notes = vec![
            "memory_feedback:reinforced=0:penalized=3:reinforcement_amount=0.000000:penalty_amount=1.500000:applied=2:removed=1:missing=1:strength_delta=0.660000"
                .to_owned(),
        ];

        let plan = planner.plan(&[reinforced, penalized], 4);
        let report = ExperienceReplayReport::from_plan(&plan);

        assert_eq!(report.live_memory_feedback_items, 2);
        assert_eq!(report.live_memory_feedback_updates, 5);
        assert_eq!(report.live_memory_feedback_reinforcements, 2);
        assert_eq!(report.live_memory_feedback_penalties, 3);
        assert_eq!(report.live_memory_feedback_detail_items, 2);
        assert_eq!(report.live_memory_feedback_applied, 4);
        assert_eq!(report.live_memory_feedback_removed, 1);
        assert_eq!(report.live_memory_feedback_missing, 1);
        assert!((report.live_memory_feedback_strength_delta - 1.08).abs() < 0.0001);
        assert!(report.summary().contains("live_memory_feedback_updates=5"));
        assert!(
            report
                .summary()
                .contains("live_memory_feedback_detail_items=2")
        );
        assert!(report.summary().contains("live_memory_feedback_applied=4"));
    }

    #[test]
    fn live_memory_feedback_stats_keep_legacy_notes_without_detail_evidence() {
        let stats = LiveMemoryFeedbackStats::from_notes(&[
            "memory_feedback:reinforced=1:penalized=0:reinforcement_amount=0.800000:penalty_amount=0.000000"
                .to_owned(),
        ])
        .unwrap();

        assert_eq!(stats.updates(), 1);
        assert_eq!(stats.applied, 0);
        assert!(!stats.has_detailed_update_evidence());
        assert_eq!(stats.applied_ratio(), None);
    }
}
