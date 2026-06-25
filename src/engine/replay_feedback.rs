use crate::drift::{DriftReport, DriftSeverity};
use crate::experience_replay::ExperienceReplayItem;
use crate::process_reward::{ProcessRewardReport, RewardAction};
use crate::reflection::ReflectionReport;
use crate::router::GenerationMetrics;

use super::MemoryFeedbackReport;

pub(super) fn replay_memory_update_amount(item: &ExperienceReplayItem) -> f32 {
    match item.action {
        RewardAction::Reinforce => replay_reinforcement_amount(item),
        RewardAction::Penalize => replay_penalty_amount(item),
        RewardAction::Hold => 0.0,
    }
}

pub(super) fn replay_reinforcement_amount(item: &ExperienceReplayItem) -> f32 {
    let reflection_drag = item.reflection_issue_count as f32 * 0.03
        + item.critical_reflection_issue_count as f32 * 0.16
        + item.revision_action_count as f32 * 0.02;
    let runtime_bonus = runtime_kv_influence_bonus(item);
    let runtime_segment_bonus = runtime_kv_segment_reinforcement_signal(item);
    let live_feedback_bonus = item
        .live_memory_feedback
        .and_then(|feedback| feedback.reinforcement_average())
        .map(|average| average.clamp(0.0, 1.0) * 0.08)
        .unwrap_or(0.0);
    let live_penalty_drag = item
        .live_memory_feedback
        .and_then(|feedback| feedback.penalty_average())
        .map(|average| average.clamp(0.0, 1.0) * 0.12)
        .unwrap_or(0.0);
    let live_evolution_bonus = replay_live_evolution_reinforcement_bonus(item);
    (item.reward
        + runtime_bonus
        + runtime_segment_bonus
        + live_feedback_bonus
        + live_evolution_bonus
        - reflection_drag
        - live_penalty_drag
        - item.recursive_call_pressure() * 0.25)
        .clamp(0.05, 1.0)
}

pub(super) fn replay_penalty_amount(item: &ExperienceReplayItem) -> f32 {
    let reflection_pressure = item.reflection_issue_count as f32 * 0.04
        + item.critical_reflection_issue_count as f32 * 0.18
        + item.revision_action_count as f32 * 0.03;
    let live_penalty_pressure = item
        .live_memory_feedback
        .and_then(|feedback| feedback.penalty_average())
        .map(|average| average.clamp(0.0, 1.0) * 0.18)
        .unwrap_or(0.0);
    let live_evolution_pressure = replay_live_evolution_penalty_pressure(item);
    let runtime_segment_pressure = runtime_kv_segment_penalty_pressure(item);
    (1.0 - item.reward
        + reflection_pressure
        + live_penalty_pressure
        + live_evolution_pressure
        + runtime_segment_pressure
        + item.recursive_call_pressure() * 0.20)
        .clamp(0.05, 1.0)
}

fn replay_live_evolution_reinforcement_bonus(item: &ExperienceReplayItem) -> f32 {
    let live = item.live_evolution;
    let mutation_bonus =
        ((live.router_threshold_delta + live.hierarchy_weight_delta).clamp(0.0, 0.25)) * 0.16;
    let memory_bonus = (live.memory_reinforcements as f32 * 0.018).min(0.06);
    let stored_bonus = (live.stored_memory_updates() as f32 * 0.012).min(0.05);
    let online_reward_bonus =
        nonnegative_f32(live.online_reward_reinforcement_strength).clamp(0.0, 1.0) * 0.05;
    let reflection_drag =
        (live.reflection_issues as f32 * 0.010 + live.revision_actions as f32 * 0.008).min(0.05);

    (mutation_bonus + memory_bonus + stored_bonus + online_reward_bonus - reflection_drag)
        .clamp(0.0, 0.14)
}

fn replay_live_evolution_penalty_pressure(item: &ExperienceReplayItem) -> f32 {
    let live = item.live_evolution;
    let reflection_pressure = (live.reflection_issues as f32 * 0.015
        + live.critical_reflection_issues as f32 * 0.055
        + live.revision_actions as f32 * 0.018)
        .min(0.16);
    let memory_penalty_pressure = (live.memory_penalties as f32 * 0.024).min(0.08);
    let online_reward_pressure =
        nonnegative_f32(live.online_reward_penalty_strength).clamp(0.0, 1.0) * 0.07;
    let stored_memory_drag = (live.stored_memory_updates() as f32 * 0.006).min(0.04);

    (reflection_pressure + memory_penalty_pressure + online_reward_pressure - stored_memory_drag)
        .clamp(0.0, 0.22)
}

fn nonnegative_f32(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn runtime_kv_influence_bonus(item: &ExperienceReplayItem) -> f32 {
    item.runtime_diagnostics
        .kv_influence
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 1.0) * 0.10)
        .unwrap_or(0.0)
}

fn runtime_kv_segment_reinforcement_signal(item: &ExperienceReplayItem) -> f32 {
    let diagnostics = &item.runtime_diagnostics;
    let total = diagnostics.runtime_kv_segment_count();
    if total == 0 {
        return 0.0;
    }

    let total = total as f32;
    let included = diagnostics.runtime_kv_segments_included as f32 / total;
    let skipped = diagnostics.runtime_kv_segments_skipped as f32 / total;
    let rejected = diagnostics.runtime_kv_segments_rejected as f32 / total;
    (included * 0.06 - skipped * 0.015 - rejected * 0.06).clamp(-0.06, 0.06)
}

fn runtime_kv_segment_penalty_pressure(item: &ExperienceReplayItem) -> f32 {
    let diagnostics = &item.runtime_diagnostics;
    let total = diagnostics.runtime_kv_segment_count();
    if total == 0 {
        return 0.0;
    }

    let total = total as f32;
    let included = diagnostics.runtime_kv_segments_included as f32 / total;
    let skipped = diagnostics.runtime_kv_segments_skipped as f32 / total;
    let rejected = diagnostics.runtime_kv_segments_rejected as f32 / total;
    (rejected * 0.10 + skipped * 0.025 - included * 0.04).clamp(0.0, 0.10)
}

pub(super) fn memory_feedback_note(report: &MemoryFeedbackReport) -> Option<String> {
    (report.total_updates() > 0).then(|| {
        format!(
            "memory_feedback:reinforced={}:penalized={}:reinforcement_amount={:.6}:penalty_amount={:.6}:applied={}:removed={}:missing={}:strength_delta={:.6}",
            report.reinforced,
            report.penalized,
            report.reinforcement_amount,
            report.penalty_amount,
            report.applied_updates(),
            report.removed_updates(),
            report.missing_updates(),
            report.strength_delta()
        )
    })
}

pub(super) fn used_memory_reinforcement_amount(report: &ReflectionReport) -> f32 {
    (report.quality - report.revision_actions.len() as f32 * 0.02).clamp(0.05, 1.0)
}

pub(super) fn used_memory_penalty_amount(
    report: &ReflectionReport,
    drift_report: &DriftReport,
    metrics: GenerationMetrics,
) -> f32 {
    let severity_pressure = match drift_report.severity {
        DriftSeverity::Stable => 0.05,
        DriftSeverity::Watch => 0.12,
        DriftSeverity::Block => 0.38,
        DriftSeverity::Rollback => 0.62,
    };
    let reflection_pressure = report.contradictions.len() as f32 * 0.12
        + report.critical_issue_count() as f32 * 0.18
        + report.revision_actions.len() as f32 * 0.03;
    let metric_pressure = metrics.contradiction_count as f32 * 0.10
        + ((metrics.perplexity - 24.0).max(0.0) / 48.0).min(0.20)
        + (1.0 - metrics.semantic_consistency.clamp(0.0, 1.0)) * 0.10;

    (1.0 - report.quality + severity_pressure + reflection_pressure + metric_pressure)
        .clamp(0.05, 1.0)
}

pub(super) fn replay_metrics(item: &ExperienceReplayItem) -> GenerationMetrics {
    let token_count = item.route_token_count();
    let recursive_call_pressure = item.recursive_call_pressure();
    match item.action {
        RewardAction::Reinforce => GenerationMetrics {
            perplexity: (6.0
                + (1.0 - item.reward) * 8.0
                + item.stream_windows as f32 * 0.03
                + recursive_call_pressure * 14.0)
                .clamp(3.0, 24.0),
            semantic_consistency: (item.quality.max(item.reward) - recursive_call_pressure * 0.18)
                .clamp(0.0, 1.0),
            contradiction_count: item.contradiction_count
                + usize::from(recursive_call_pressure >= 0.18 && item.reward < 0.90),
            token_count,
        },
        RewardAction::Penalize => GenerationMetrics {
            perplexity: (18.0
                + (1.0 - item.reward) * 18.0
                + item.stream_windows as f32 * 0.05
                + recursive_call_pressure * 18.0)
                .clamp(12.0, 56.0),
            semantic_consistency: (item.quality.min(item.reward) - recursive_call_pressure * 0.12)
                .clamp(0.0, 1.0),
            contradiction_count: item
                .contradiction_count
                .max(item.critical_reflection_issue_count)
                .max(1),
            token_count,
        },
        RewardAction::Hold => GenerationMetrics {
            perplexity: 10.0,
            semantic_consistency: item.quality.clamp(0.0, 1.0),
            contradiction_count: item
                .contradiction_count
                .max(item.critical_reflection_issue_count),
            token_count,
        },
    }
}

pub(super) fn process_reward_feedback_metrics(
    report: &ProcessRewardReport,
    base: GenerationMetrics,
    reflection: &ReflectionReport,
    drift_report: &DriftReport,
) -> Option<GenerationMetrics> {
    if drift_report.rollback_adaptive {
        return None;
    }

    let strength = process_reward_feedback_strength(report);

    match report.action {
        RewardAction::Reinforce
            if reflection.critical_issue_count() == 0
                && reflection.contradictions.is_empty()
                && reflection.issues.len() <= 1 =>
        {
            let perplexity_scale = 0.88 - strength * 0.22;
            let semantic_floor = 0.76 + strength * 0.12;
            let semantic_ceiling = 0.92 + strength * 0.06;
            Some(GenerationMetrics {
                perplexity: (base.perplexity * perplexity_scale).clamp(3.0, 12.0),
                semantic_consistency: base
                    .semantic_consistency
                    .max(reflection.quality)
                    .max(report.total)
                    .clamp(semantic_floor, semantic_ceiling),
                contradiction_count: 0,
                token_count: base.token_count,
            })
        }
        RewardAction::Penalize => {
            let perplexity_pressure = 8.0 + strength * 24.0;
            let semantic_ceiling = 0.52 - strength * 0.24;
            let contradiction_floor = 1 + (strength >= 0.75) as usize;
            Some(GenerationMetrics {
                perplexity: (base.perplexity + perplexity_pressure).clamp(12.0, 64.0),
                semantic_consistency: base
                    .semantic_consistency
                    .min(reflection.quality)
                    .min(report.total)
                    .clamp(0.0, semantic_ceiling),
                contradiction_count: base
                    .contradiction_count
                    .max(reflection.critical_issue_count())
                    .max(contradiction_floor),
                token_count: base.token_count,
            })
        }
        RewardAction::Hold => None,
        RewardAction::Reinforce => None,
    }
}

pub(super) fn process_reward_feedback_strength(report: &ProcessRewardReport) -> f32 {
    const MIN_FEEDBACK_STRENGTH: f32 = 0.35;
    const REINFORCE_THRESHOLD: f32 = 0.72;
    const PENALIZE_THRESHOLD: f32 = 0.42;

    match report.action {
        RewardAction::Reinforce => {
            let range = 1.0 - REINFORCE_THRESHOLD;
            let normalized = (report.total - REINFORCE_THRESHOLD) / range;
            normalized.clamp(MIN_FEEDBACK_STRENGTH, 1.0)
        }
        RewardAction::Penalize => {
            let normalized = (PENALIZE_THRESHOLD - report.total) / PENALIZE_THRESHOLD;
            normalized.clamp(MIN_FEEDBACK_STRENGTH, 1.0)
        }
        RewardAction::Hold => 0.0,
    }
}

pub(super) fn process_reward_feedback_note(
    report: &ProcessRewardReport,
    metrics: GenerationMetrics,
) -> String {
    format!(
        "online_reward_feedback:action={}:strength={:.3}:perplexity={:.3}:semantic={:.3}:contradictions={}",
        report.action.as_str(),
        process_reward_feedback_strength(report),
        metrics.perplexity,
        metrics.semantic_consistency,
        metrics.contradiction_count
    )
}
