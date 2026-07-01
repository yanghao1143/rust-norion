use crate::experience::{
    ExperienceRecord, hygiene_quarantine_candidate_ids, recursive_runtime_calls_from_notes,
};
use crate::process_reward::RewardAction;
use crate::reflection::ReflectionSeverity;

use super::item::{
    ExperienceReplayItem, ExperienceReplayPlan, runtime_kv_budget_pressure,
    runtime_kv_weak_import_pressure,
};
use super::stats::{
    BusinessContractReplayStats, LiveMemoryFeedbackStats, PoolDispatchReplayStats,
    RecursiveReplayStats, RustCheckReplayStats,
};

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
        let quarantine_candidate_ids = hygiene_quarantine_candidate_ids(records);
        let mut items = records
            .iter()
            .filter(|record| !quarantine_candidate_ids.contains(&record.id))
            .filter_map(|record| self.item_for_record(record))
            .collect::<Vec<_>>();

        sort_replay_items(&mut items);
        preserve_signal_coverage(&mut items, limit);

        ExperienceReplayPlan { items }
    }

    fn item_for_record(&self, record: &ExperienceRecord) -> Option<ExperienceReplayItem> {
        let reward = record.process_reward.total.clamp(0.0, 1.0);
        let recursive_stats = RecursiveReplayStats::from_notes(&record.process_reward.notes);
        let live_memory_feedback =
            LiveMemoryFeedbackStats::from_notes(&record.process_reward.notes);
        let rust_check_stats = RustCheckReplayStats::from_notes(&record.process_reward.notes);
        let rust_check_live_memory_feedback = LiveMemoryFeedbackStats::from_notes_for_source(
            &record.process_reward.notes,
            "rust_check",
        );
        let business_contract_stats =
            BusinessContractReplayStats::from_notes(&record.process_reward.notes);
        let pool_dispatch_stats = PoolDispatchReplayStats::from_notes(&record.process_reward.notes);
        let action = if reward >= self.reinforce_threshold {
            RewardAction::Reinforce
        } else if reward <= self.penalize_threshold {
            RewardAction::Penalize
        } else if business_contract_stats.is_some()
            || rust_check_stats.is_some()
            || pool_dispatch_stats.is_some()
        {
            RewardAction::Hold
        } else {
            return None;
        };
        let runtime_kv_budget_pressure = runtime_kv_budget_pressure(&record.runtime_diagnostics);
        let runtime_kv_weak_import_pressure =
            runtime_kv_weak_import_pressure(&record.runtime_diagnostics);
        let priority = replay_priority(
            action,
            reward,
            reflection_issue_priority(record),
            runtime_kv_budget_pressure,
            runtime_kv_weak_import_pressure,
        );
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
            live_evolution: record.live_evolution,
            recursive_runtime_calls: recursive_stats
                .and_then(|stats| stats.runtime_calls)
                .or_else(|| recursive_runtime_calls_from_notes(&record.process_reward.notes)),
            recursive_stats,
            live_memory_feedback,
            rust_check_stats,
            rust_check_live_memory_feedback,
            business_contract_stats,
            pool_dispatch_stats,
            priority,
            lesson: record.lesson.clone(),
        })
    }
}

fn preserve_signal_coverage(items: &mut Vec<ExperienceReplayItem>, limit: usize) {
    if limit == 0 {
        items.clear();
        return;
    }
    if items.len() <= limit {
        return;
    }

    let overflow = items.iter().skip(limit).cloned().collect::<Vec<_>>();
    let recursive_candidate = overflow
        .iter()
        .find(|item| item.recursive_runtime_calls.is_some())
        .cloned();
    let live_evolution_candidate = overflow
        .iter()
        .find(|item| item.live_evolution.has_evidence())
        .cloned();
    let runtime_kv_pressure_candidate = overflow
        .iter()
        .filter(|item| has_runtime_kv_pressure(item))
        .max_by(|left, right| {
            runtime_kv_pressure_score(left)
                .partial_cmp(&runtime_kv_pressure_score(right))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.priority.total_cmp(&right.priority))
        })
        .cloned();
    items.truncate(limit);

    if !items
        .iter()
        .any(|item| item.recursive_runtime_calls.is_some())
        && let Some(recursive_item) = recursive_candidate
    {
        replace_lowest_priority_matching(items, recursive_item, |item| {
            item.recursive_runtime_calls.is_none()
        });
    }
    if !items.iter().any(|item| item.live_evolution.has_evidence())
        && let Some(live_evolution_item) = live_evolution_candidate
    {
        let has_recursive_item = items
            .iter()
            .any(|item| item.recursive_runtime_calls.is_some());
        replace_lowest_priority_matching(items, live_evolution_item, |item| {
            !item.live_evolution.has_evidence()
                && (!has_recursive_item || item.recursive_runtime_calls.is_none())
        });
    }
    if !items.iter().any(has_runtime_kv_pressure)
        && let Some(runtime_kv_pressure_item) = runtime_kv_pressure_candidate
    {
        let has_recursive_item = items
            .iter()
            .any(|item| item.recursive_runtime_calls.is_some());
        let has_live_evolution_item = items.iter().any(|item| item.live_evolution.has_evidence());
        replace_lowest_priority_matching(items, runtime_kv_pressure_item, |item| {
            !has_runtime_kv_pressure(item)
                && (!has_recursive_item || item.recursive_runtime_calls.is_none())
                && (!has_live_evolution_item || !item.live_evolution.has_evidence())
        });
    }
}

fn has_runtime_kv_pressure(item: &ExperienceReplayItem) -> bool {
    runtime_kv_pressure_score(item) > 0.0
}

fn runtime_kv_pressure_score(item: &ExperienceReplayItem) -> f32 {
    item.runtime_kv_budget_pressure()
        .max(item.runtime_kv_weak_import_pressure())
        .clamp(0.0, 1.0)
}

fn replace_lowest_priority_matching(
    items: &mut [ExperienceReplayItem],
    replacement: ExperienceReplayItem,
    predicate: impl Fn(&ExperienceReplayItem) -> bool,
) {
    if let Some((replace_index, _)) = items
        .iter()
        .enumerate()
        .filter(|(_, item)| predicate(item))
        .min_by(|(_, left), (_, right)| {
            left.priority
                .partial_cmp(&right.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.experience_id.cmp(&right.experience_id))
        })
    {
        items[replace_index] = replacement;
        sort_replay_items(items);
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

fn critical_reflection_issue_count(record: &ExperienceRecord) -> usize {
    record
        .reflection_issues
        .iter()
        .filter(|issue| issue.severity == ReflectionSeverity::Critical)
        .count()
}

fn replay_priority(
    action: RewardAction,
    reward: f32,
    reflection_issue_priority: f32,
    runtime_kv_budget_pressure: f32,
    runtime_kv_weak_import_pressure: f32,
) -> f32 {
    let budget_pressure = runtime_kv_budget_pressure.clamp(0.0, 1.0);
    let weak_import_pressure = runtime_kv_weak_import_pressure.clamp(0.0, 1.0);
    match action {
        RewardAction::Reinforce => reward - budget_pressure * 0.10 - weak_import_pressure * 0.08,
        RewardAction::Penalize => {
            1.0 - reward
                + reflection_issue_priority
                + budget_pressure * 0.12
                + weak_import_pressure * 0.10
        }
        RewardAction::Hold => 0.0,
    }
    .clamp(0.0, 1.0)
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
