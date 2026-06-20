use std::collections::BTreeSet;

use crate::{
    ExperienceEnvelope, LongTermMemory, MemoryAdapter, MemoryAdapterCapability,
    MemoryAdapterDescriptor, MemoryAdapterHealth, MemoryResult, MemoryScope, clamp01,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayAction {
    Reinforce,
    Penalize,
    Hold,
}

impl ReplayAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reinforce => "reinforce",
            Self::Penalize => "penalize",
            Self::Hold => "hold",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReplaySignal {
    RecursiveRuntime,
    LiveMemoryFeedback,
    RustCheck,
    BusinessContract,
    ContextRot,
}

impl ReplaySignal {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RecursiveRuntime => "recursive_runtime",
            Self::LiveMemoryFeedback => "live_memory_feedback",
            Self::RustCheck => "rust_check",
            Self::BusinessContract => "business_contract",
            Self::ContextRot => "context_rot",
        }
    }

    fn priority_bonus(self) -> f32 {
        match self {
            Self::RecursiveRuntime => 0.08,
            Self::LiveMemoryFeedback => 0.12,
            Self::RustCheck => 0.05,
            Self::BusinessContract => 0.04,
            Self::ContextRot => 0.16,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ReplayFeedbackStats {
    pub reinforced: usize,
    pub penalized: usize,
    pub reinforcement_amount: f32,
    pub penalty_amount: f32,
    pub applied: usize,
    pub removed: usize,
    pub missing: usize,
    pub strength_delta: f32,
}

impl ReplayFeedbackStats {
    pub fn updates(self) -> usize {
        self.reinforced.saturating_add(self.penalized)
    }

    pub fn from_notes(notes: &[String]) -> Option<Self> {
        notes
            .iter()
            .filter(|note| note.starts_with("memory_feedback:"))
            .find_map(|note| Self::from_note(note))
    }

    fn from_note(note: &str) -> Option<Self> {
        let stats = Self {
            reinforced: note_usize(note, "reinforced=").unwrap_or(0),
            penalized: note_usize(note, "penalized=").unwrap_or(0),
            reinforcement_amount: note_f32(note, "reinforcement_amount=").unwrap_or(0.0),
            penalty_amount: note_f32(note, "penalty_amount=").unwrap_or(0.0),
            applied: note_usize(note, "applied=").unwrap_or(0),
            removed: note_usize(note, "removed=").unwrap_or(0),
            missing: note_usize(note, "missing=").unwrap_or(0),
            strength_delta: note_f32(note, "strength_delta=").unwrap_or(0.0),
        };
        (stats.updates() > 0).then_some(stats)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayCandidate {
    pub id: String,
    pub lesson: String,
    pub reward: f32,
    pub quality: f32,
    pub scope: MemoryScope,
    pub memory_ids: Vec<String>,
    pub signals: Vec<ReplaySignal>,
    pub notes: Vec<String>,
}

impl ReplayCandidate {
    pub fn new(id: impl Into<String>, lesson: impl Into<String>, reward: f32) -> Self {
        Self {
            id: id.into(),
            lesson: lesson.into(),
            reward: clamp01(reward),
            quality: clamp01(reward),
            scope: MemoryScope::default(),
            memory_ids: Vec::new(),
            signals: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn from_experience(envelope: &ExperienceEnvelope) -> Self {
        let mut candidate = Self::new(&envelope.id, &envelope.lesson, envelope.quality)
            .with_scope(envelope.scope.clone());
        candidate.signals = envelope
            .tags
            .iter()
            .filter_map(|tag| signal_from_tag(tag))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        candidate
    }

    pub fn with_scope(mut self, scope: MemoryScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = clamp01(quality);
        self
    }

    pub fn with_memory_ids(mut self, memory_ids: Vec<String>) -> Self {
        self.memory_ids = memory_ids;
        self.memory_ids.sort();
        self.memory_ids.dedup();
        self
    }

    pub fn with_signals(mut self, signals: Vec<ReplaySignal>) -> Self {
        self.signals = signals
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        self
    }

    pub fn with_notes(mut self, notes: Vec<String>) -> Self {
        self.notes = notes;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayMemoryUpdate {
    pub memory_id: String,
    pub source_experience_id: String,
    pub action: ReplayAction,
    pub amount: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayItem {
    pub experience_id: String,
    pub action: ReplayAction,
    pub reward: f32,
    pub quality: f32,
    pub priority: f32,
    pub lesson: String,
    pub memory_updates: Vec<ReplayMemoryUpdate>,
    pub feedback: Option<ReplayFeedbackStats>,
    pub signals: Vec<ReplaySignal>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReplayPlan {
    pub items: Vec<ReplayItem>,
}

impl ReplayPlan {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReplayReport {
    pub planned: usize,
    pub reinforced: usize,
    pub penalized: usize,
    pub held: usize,
    pub touched_memories: usize,
    pub memory_reinforcements: usize,
    pub memory_penalties: usize,
    pub feedback_items: usize,
    pub feedback_updates: usize,
    pub feedback_applied: usize,
    pub feedback_removed: usize,
    pub feedback_missing: usize,
    pub feedback_strength_delta: f32,
    pub average_reward: f32,
    pub recursive_runtime_items: usize,
    pub live_memory_feedback_items: usize,
    pub rust_check_items: usize,
    pub business_contract_items: usize,
    pub context_rot_items: usize,
    pub detail_codes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReplayApplyReport {
    pub requested: usize,
    pub applied: usize,
    pub reinforced: usize,
    pub penalized: usize,
    pub missing: usize,
    pub missing_memory_ids: Vec<String>,
    pub invalid_memory_ids: Vec<String>,
}

impl ReplayApplyReport {
    pub fn strength_updates(&self) -> usize {
        self.reinforced.saturating_add(self.penalized)
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.missing_memory_ids
            .iter()
            .map(|id| format!("missing_memory:{}", hex_id(id)))
            .chain(
                self.invalid_memory_ids
                    .iter()
                    .map(|id| format!("invalid_memory_id:{}", hex_id(id))),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

pub fn apply_replay_updates_to_long_term<M: LongTermMemory>(
    memory: &mut M,
    plan: &ReplayPlan,
) -> MemoryResult<ReplayApplyReport> {
    let mut report = ReplayApplyReport::default();
    for update in plan
        .items
        .iter()
        .flat_map(|item| item.memory_updates.iter())
    {
        report.requested += 1;
        let Ok(id) = update.memory_id.parse::<u64>() else {
            report.invalid_memory_ids.push(update.memory_id.clone());
            continue;
        };
        let applied = match update.action {
            ReplayAction::Reinforce => {
                let applied = memory.reinforce(id, update.amount)?;
                report.reinforced += usize::from(applied);
                applied
            }
            ReplayAction::Penalize => {
                let applied = memory.penalize(id, update.amount)?;
                report.penalized += usize::from(applied);
                applied
            }
            ReplayAction::Hold => false,
        };
        if applied {
            report.applied += 1;
        } else {
            report.missing += 1;
            report.missing_memory_ids.push(update.memory_id.clone());
        }
    }
    Ok(report)
}

impl ReplayReport {
    pub fn from_plan(plan: &ReplayPlan) -> Self {
        let mut report = Self {
            planned: plan.items.len(),
            ..Self::default()
        };
        let mut reward_total = 0.0;
        let mut touched = BTreeSet::new();
        let mut detail_codes = BTreeSet::new();

        for item in &plan.items {
            reward_total += item.reward;
            detail_codes.insert(format!(
                "item:{}:{}",
                item.action.as_str(),
                hex_id(&item.experience_id)
            ));
            match item.action {
                ReplayAction::Reinforce => report.reinforced += 1,
                ReplayAction::Penalize => report.penalized += 1,
                ReplayAction::Hold => report.held += 1,
            }
            for update in &item.memory_updates {
                touched.insert(update.memory_id.clone());
                detail_codes.insert(format!(
                    "memory_update:{}:{}:{}",
                    update.action.as_str(),
                    hex_id(&update.memory_id),
                    hex_id(&update.source_experience_id)
                ));
                match update.action {
                    ReplayAction::Reinforce => report.memory_reinforcements += 1,
                    ReplayAction::Penalize => report.memory_penalties += 1,
                    ReplayAction::Hold => {}
                }
            }
            if let Some(feedback) = item.feedback {
                detail_codes.insert(format!(
                    "feedback:live_memory_feedback:{}",
                    hex_id(&item.experience_id)
                ));
                if feedback.missing > 0 {
                    detail_codes
                        .insert(format!("feedback_missing:{}", hex_id(&item.experience_id)));
                }
                report.feedback_items += 1;
                report.feedback_updates += feedback.updates();
                report.feedback_applied += feedback.applied;
                report.feedback_removed += feedback.removed;
                report.feedback_missing += feedback.missing;
                report.feedback_strength_delta += feedback.strength_delta;
            }
            report.recursive_runtime_items +=
                usize::from(has_signal(item, ReplaySignal::RecursiveRuntime));
            report.live_memory_feedback_items +=
                usize::from(has_signal(item, ReplaySignal::LiveMemoryFeedback));
            report.rust_check_items += usize::from(has_signal(item, ReplaySignal::RustCheck));
            report.business_contract_items +=
                usize::from(has_signal(item, ReplaySignal::BusinessContract));
            report.context_rot_items += usize::from(has_signal(item, ReplaySignal::ContextRot));
            for signal in &item.signals {
                detail_codes.insert(format!(
                    "signal:{}:{}",
                    signal.as_str(),
                    hex_id(&item.experience_id)
                ));
            }
        }

        report.touched_memories = touched.len();
        if report.planned > 0 {
            report.average_reward = reward_total / report.planned as f32;
        }
        report.detail_codes = detail_codes.into_iter().collect();
        report
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if self.reinforced > 0 {
            codes.insert("action_reinforce".to_owned());
        }
        if self.penalized > 0 {
            codes.insert("action_penalize".to_owned());
        }
        if self.held > 0 {
            codes.insert("action_hold".to_owned());
        }
        if self.memory_reinforcements > 0 {
            codes.insert("memory_reinforcement".to_owned());
        }
        if self.memory_penalties > 0 {
            codes.insert("memory_penalty".to_owned());
        }
        if self.feedback_items > 0 {
            codes.insert("live_feedback".to_owned());
        }
        if self.feedback_missing > 0 {
            codes.insert("feedback_missing_memory".to_owned());
        }
        if self.recursive_runtime_items > 0 {
            codes.insert(ReplaySignal::RecursiveRuntime.as_str().to_owned());
        }
        if self.live_memory_feedback_items > 0 {
            codes.insert(ReplaySignal::LiveMemoryFeedback.as_str().to_owned());
        }
        if self.rust_check_items > 0 {
            codes.insert(ReplaySignal::RustCheck.as_str().to_owned());
        }
        if self.business_contract_items > 0 {
            codes.insert(ReplaySignal::BusinessContract.as_str().to_owned());
        }
        if self.context_rot_items > 0 {
            codes.insert(ReplaySignal::ContextRot.as_str().to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.detail_codes.clone()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_replay planned={} reinforced={} penalized={} held={} touched_memories={} memory_reinforcements={} memory_penalties={} feedback_items={} feedback_updates={} feedback_applied={} feedback_removed={} feedback_missing={} average_reward={:.3} recursive_runtime={} live_memory_feedback={} rust_check={} business_contract={} context_rot={} reason_codes={} detail_codes={}",
            self.planned,
            self.reinforced,
            self.penalized,
            self.held,
            self.touched_memories,
            self.memory_reinforcements,
            self.memory_penalties,
            self.feedback_items,
            self.feedback_updates,
            self.feedback_applied,
            self.feedback_removed,
            self.feedback_missing,
            self.average_reward,
            self.recursive_runtime_items,
            self.live_memory_feedback_items,
            self.rust_check_items,
            self.business_contract_items,
            self.context_rot_items,
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

fn hex_id(id: &str) -> String {
    id.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

pub trait ExperienceReplayPlanner {
    fn plan(
        &self,
        candidates: &[ReplayCandidate],
        scope: Option<&MemoryScope>,
        limit: usize,
    ) -> ReplayPlan;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefaultExperienceReplayPlanner {
    pub reinforce_threshold: f32,
    pub penalize_threshold: f32,
}

impl Default for DefaultExperienceReplayPlanner {
    fn default() -> Self {
        Self {
            reinforce_threshold: 0.72,
            penalize_threshold: 0.42,
        }
    }
}

impl MemoryAdapter for DefaultExperienceReplayPlanner {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_experience_replay_planner",
            vec![MemoryAdapterCapability::ExperienceReplay],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(None))
    }
}

impl ExperienceReplayPlanner for DefaultExperienceReplayPlanner {
    fn plan(
        &self,
        candidates: &[ReplayCandidate],
        scope: Option<&MemoryScope>,
        limit: usize,
    ) -> ReplayPlan {
        if limit == 0 {
            return ReplayPlan::default();
        }
        let mut items = candidates
            .iter()
            .filter(|candidate| {
                scope
                    .and_then(|scope| scope.same_task_as(&candidate.scope))
                    .unwrap_or(true)
            })
            .filter_map(|candidate| self.item_for_candidate(candidate))
            .collect::<Vec<_>>();

        sort_replay_items(&mut items);
        preserve_signal_coverage(&mut items, limit);
        items.truncate(limit);
        ReplayPlan { items }
    }
}

impl DefaultExperienceReplayPlanner {
    fn item_for_candidate(&self, candidate: &ReplayCandidate) -> Option<ReplayItem> {
        let action = if candidate.reward >= self.reinforce_threshold {
            ReplayAction::Reinforce
        } else if candidate.reward <= self.penalize_threshold {
            ReplayAction::Penalize
        } else if candidate.signals.contains(&ReplaySignal::BusinessContract) {
            ReplayAction::Hold
        } else {
            return None;
        };
        let signal_bonus = candidate
            .signals
            .iter()
            .map(|signal| signal.priority_bonus())
            .sum::<f32>()
            .min(0.28);
        let priority = match action {
            ReplayAction::Reinforce => candidate.reward + signal_bonus,
            ReplayAction::Penalize => 1.0 - candidate.reward + signal_bonus,
            ReplayAction::Hold => 0.1 + signal_bonus,
        }
        .clamp(0.0, 1.0);
        let amount = match action {
            ReplayAction::Reinforce => candidate.reward,
            ReplayAction::Penalize => 1.0 - candidate.reward,
            ReplayAction::Hold => 0.0,
        };
        let memory_updates = candidate
            .memory_ids
            .iter()
            .filter(|_| action != ReplayAction::Hold)
            .map(|memory_id| ReplayMemoryUpdate {
                memory_id: memory_id.clone(),
                source_experience_id: candidate.id.clone(),
                action,
                amount,
            })
            .collect::<Vec<_>>();
        let feedback = ReplayFeedbackStats::from_notes(&candidate.notes);

        Some(ReplayItem {
            experience_id: candidate.id.clone(),
            action,
            reward: candidate.reward,
            quality: candidate.quality,
            priority,
            lesson: candidate.lesson.clone(),
            memory_updates,
            feedback,
            signals: candidate.signals.clone(),
        })
    }
}

fn preserve_signal_coverage(items: &mut Vec<ReplayItem>, limit: usize) {
    if items.len() <= limit {
        return;
    }
    let overflow = items.iter().skip(limit).cloned().collect::<Vec<_>>();
    items.truncate(limit);
    for signal in [
        ReplaySignal::RecursiveRuntime,
        ReplaySignal::LiveMemoryFeedback,
        ReplaySignal::ContextRot,
    ] {
        if items.iter().any(|item| has_signal(item, signal)) {
            continue;
        }
        let Some(candidate) = overflow
            .iter()
            .find(|item| has_signal(item, signal))
            .cloned()
        else {
            continue;
        };
        if let Some((replace_index, _)) = items
            .iter()
            .enumerate()
            .filter(|(_, item)| !has_signal(item, signal))
            .min_by(|(_, left), (_, right)| {
                left.priority
                    .partial_cmp(&right.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left.experience_id.cmp(&right.experience_id))
            })
        {
            items[replace_index] = candidate;
            sort_replay_items(items);
        }
    }
}

fn sort_replay_items(items: &mut [ReplayItem]) {
    items.sort_by(|left, right| {
        right
            .priority
            .partial_cmp(&left.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.experience_id.cmp(&left.experience_id))
    });
}

fn has_signal(item: &ReplayItem, signal: ReplaySignal) -> bool {
    item.signals.contains(&signal)
}

fn signal_from_tag(tag: &str) -> Option<ReplaySignal> {
    match tag {
        "recursive_runtime" | "recursive" => Some(ReplaySignal::RecursiveRuntime),
        "live_memory_feedback" | "memory_feedback" => Some(ReplaySignal::LiveMemoryFeedback),
        "rust_check" => Some(ReplaySignal::RustCheck),
        "business_contract" => Some(ReplaySignal::BusinessContract),
        "context_rot" => Some(ReplaySignal::ContextRot),
        _ => None,
    }
}

fn note_usize(note: &str, key: &str) -> Option<usize> {
    note.split(':')
        .find_map(|part| part.strip_prefix(key))
        .and_then(|value| value.parse::<usize>().ok())
}

fn note_f32(note: &str, key: &str) -> Option<f32> {
    note.split(':')
        .find_map(|part| part.strip_prefix(key))
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InMemoryLongTermMemory, MemoryDocumentInput};

    #[test]
    fn replay_planner_selects_reinforce_penalize_and_hold_items() {
        let candidates = vec![
            ReplayCandidate::new("good", "use the adapter", 0.9)
                .with_memory_ids(vec!["m1".to_owned()]),
            ReplayCandidate::new("bad", "avoid polluted context", 0.1)
                .with_memory_ids(vec!["m2".to_owned()])
                .with_signals(vec![ReplaySignal::ContextRot]),
            ReplayCandidate::new("audit", "business audit", 0.6)
                .with_signals(vec![ReplaySignal::BusinessContract]),
            ReplayCandidate::new("skip", "neutral", 0.6),
        ];

        let plan = DefaultExperienceReplayPlanner::default().plan(&candidates, None, 10);
        assert_eq!(plan.items.len(), 3);
        assert!(
            plan.items
                .iter()
                .any(|item| item.experience_id == "good" && item.action == ReplayAction::Reinforce)
        );
        assert!(
            plan.items
                .iter()
                .any(|item| item.experience_id == "bad" && item.action == ReplayAction::Penalize)
        );
        assert!(
            plan.items
                .iter()
                .any(|item| item.experience_id == "audit" && item.action == ReplayAction::Hold)
        );
    }

    #[test]
    fn replay_planner_filters_cross_task_candidates_but_keeps_global() {
        let candidates = vec![
            ReplayCandidate::new("runtime", "runtime lesson", 0.9)
                .with_scope(MemoryScope::for_task("runtime")),
            ReplayCandidate::new("global", "global lesson", 0.8),
            ReplayCandidate::new("ops", "ops lesson", 0.95)
                .with_scope(MemoryScope::for_task("ops")),
        ];

        let plan = DefaultExperienceReplayPlanner::default().plan(
            &candidates,
            Some(&MemoryScope::for_task("runtime")),
            10,
        );
        let ids = plan
            .items
            .iter()
            .map(|item| item.experience_id.as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"runtime"));
        assert!(ids.contains(&"global"));
        assert!(!ids.contains(&"ops"));
    }

    #[test]
    fn replay_planner_preserves_live_feedback_signal_when_limited() {
        let candidates = vec![
            ReplayCandidate::new("top", "top", 0.99),
            ReplayCandidate::new("second", "second", 0.98),
            ReplayCandidate::new("feedback", "feedback", 0.73)
                .with_signals(vec![ReplaySignal::LiveMemoryFeedback])
                .with_notes(vec![
                    "memory_feedback:reinforced=2:penalized=1:reinforcement_amount=0.4:penalty_amount=0.2:applied=2:removed=0:missing=1:strength_delta=0.3"
                        .to_owned(),
                ]),
        ];

        let plan = DefaultExperienceReplayPlanner::default().plan(&candidates, None, 2);
        assert!(
            plan.items
                .iter()
                .any(|item| item.experience_id == "feedback")
        );
        let report = ReplayReport::from_plan(&plan);
        assert_eq!(report.feedback_items, 1);
        assert_eq!(report.feedback_updates, 3);
        assert_eq!(report.feedback_missing, 1);
        assert!((report.feedback_strength_delta - 0.3).abs() < f32::EPSILON);
        assert_eq!(
            report.reason_codes(),
            vec![
                "action_reinforce".to_owned(),
                "feedback_missing_memory".to_owned(),
                "live_feedback".to_owned(),
                "live_memory_feedback".to_owned()
            ]
        );
        assert_eq!(
            report.detail_codes(),
            vec![
                "feedback:live_memory_feedback:666565646261636b".to_owned(),
                "feedback_missing:666565646261636b".to_owned(),
                "item:reinforce:666565646261636b".to_owned(),
                "item:reinforce:746f70".to_owned(),
                "signal:live_memory_feedback:666565646261636b".to_owned()
            ]
        );
    }

    #[test]
    fn replay_report_summarizes_memory_updates_and_signals() {
        let candidates = vec![
            ReplayCandidate::new("reinforce", "good", 0.8)
                .with_memory_ids(vec!["m1".to_owned(), "m2".to_owned()])
                .with_signals(vec![ReplaySignal::RustCheck]),
            ReplayCandidate::new("penalize", "bad", 0.2)
                .with_memory_ids(vec!["m2".to_owned()])
                .with_signals(vec![ReplaySignal::ContextRot]),
        ];

        let plan = DefaultExperienceReplayPlanner::default().plan(&candidates, None, 10);
        let report = ReplayReport::from_plan(&plan);
        assert_eq!(report.planned, 2);
        assert_eq!(report.reinforced, 1);
        assert_eq!(report.penalized, 1);
        assert_eq!(report.touched_memories, 2);
        assert_eq!(report.memory_reinforcements, 2);
        assert_eq!(report.memory_penalties, 1);
        assert_eq!(report.rust_check_items, 1);
        assert_eq!(report.context_rot_items, 1);
        assert_eq!(
            report.summary_line(),
            "memory_replay planned=2 reinforced=1 penalized=1 held=0 touched_memories=2 memory_reinforcements=2 memory_penalties=1 feedback_items=0 feedback_updates=0 feedback_applied=0 feedback_removed=0 feedback_missing=0 average_reward=0.500 recursive_runtime=0 live_memory_feedback=0 rust_check=1 business_contract=0 context_rot=1 reason_codes=action_penalize|action_reinforce|context_rot|memory_penalty|memory_reinforcement|rust_check detail_codes=item:penalize:70656e616c697a65|item:reinforce:7265696e666f726365|memory_update:penalize:6d32:70656e616c697a65|memory_update:reinforce:6d31:7265696e666f726365|memory_update:reinforce:6d32:7265696e666f726365|signal:context_rot:70656e616c697a65|signal:rust_check:7265696e666f726365"
        );
        assert_eq!(
            report.reason_codes(),
            vec![
                "action_penalize".to_owned(),
                "action_reinforce".to_owned(),
                "context_rot".to_owned(),
                "memory_penalty".to_owned(),
                "memory_reinforcement".to_owned(),
                "rust_check".to_owned()
            ]
        );
        assert_eq!(
            report.detail_codes(),
            vec![
                "item:penalize:70656e616c697a65".to_owned(),
                "item:reinforce:7265696e666f726365".to_owned(),
                "memory_update:penalize:6d32:70656e616c697a65".to_owned(),
                "memory_update:reinforce:6d31:7265696e666f726365".to_owned(),
                "memory_update:reinforce:6d32:7265696e666f726365".to_owned(),
                "signal:context_rot:70656e616c697a65".to_owned(),
                "signal:rust_check:7265696e666f726365".to_owned()
            ]
        );
    }

    #[test]
    fn replay_candidate_projects_from_experience_tags() {
        let envelope = ExperienceEnvelope::new("42", "prompt", "lesson")
            .with_quality(0.88)
            .with_tags(vec!["rust_check".to_owned(), "context_rot".to_owned()])
            .with_scope(MemoryScope::for_task("runtime"));

        let candidate = ReplayCandidate::from_experience(&envelope);
        assert_eq!(candidate.id, "42");
        assert_eq!(candidate.reward, 0.88);
        assert_eq!(candidate.scope.task_id.as_deref(), Some("runtime"));
        assert!(candidate.signals.contains(&ReplaySignal::RustCheck));
        assert!(candidate.signals.contains(&ReplaySignal::ContextRot));
    }

    #[test]
    fn replay_planner_is_read_only_adapter() {
        let descriptor = DefaultExperienceReplayPlanner::default().descriptor();
        assert_eq!(descriptor.name, "default_experience_replay_planner");
        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::ExperienceReplay)
        );
    }

    #[test]
    fn replay_updates_apply_to_long_term_memory_strength() {
        let mut memory = InMemoryLongTermMemory::new();
        let reinforce_id = memory
            .remember(MemoryDocumentInput::new("strong memory", vec![1.0]).with_strength(0.4))
            .unwrap();
        let penalize_id = memory
            .remember(MemoryDocumentInput::new("weak memory", vec![0.0]).with_strength(0.8))
            .unwrap();
        let candidates = vec![
            ReplayCandidate::new("reinforce", "good", 0.9)
                .with_memory_ids(vec![reinforce_id.to_string()]),
            ReplayCandidate::new("penalize", "bad", 0.2).with_memory_ids(vec![
                penalize_id.to_string(),
                "999999".to_owned(),
                "not-a-number".to_owned(),
            ]),
        ];
        let plan = DefaultExperienceReplayPlanner::default().plan(&candidates, None, 10);

        let report = apply_replay_updates_to_long_term(&mut memory, &plan).unwrap();
        assert_eq!(report.requested, 4);
        assert_eq!(report.applied, 2);
        assert_eq!(report.reinforced, 1);
        assert_eq!(report.penalized, 1);
        assert_eq!(report.missing, 1);
        assert_eq!(report.missing_memory_ids, vec!["999999".to_owned()]);
        assert_eq!(report.invalid_memory_ids, vec!["not-a-number".to_owned()]);
        assert_eq!(
            report.detail_codes(),
            vec![
                "invalid_memory_id:6e6f742d612d6e756d626572".to_owned(),
                "missing_memory:393939393939".to_owned()
            ]
        );
        assert!(memory.get(reinforce_id).unwrap().unwrap().strength > 0.4);
        assert!(memory.get(penalize_id).unwrap().unwrap().strength < 0.8);
    }
}
