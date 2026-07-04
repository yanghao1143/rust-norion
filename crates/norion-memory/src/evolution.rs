use std::collections::BTreeSet;

use crate::{
    ExperienceIndexQualityGate, KvSwapBoundaryReadiness, MemoryAdapter, MemoryAdapterCapability,
    MemoryAdapterDescriptor, MemoryAdapterHealth, MemoryCompactionPlan, MemoryResult,
    MemoryRetentionPlan, ReplayApplyReport, ReplayReport,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MemoryEvolutionLedger {
    pub replay_runs: u64,
    pub replay_items: u64,
    pub replay_reinforcements: u64,
    pub replay_penalties: u64,
    pub replay_memory_updates: u64,
    pub replay_memory_missing: u64,
    pub replay_invalid_memory_ids: u64,
    pub recursive_runtime_items: u64,
    pub live_memory_feedback_items: u64,
    pub context_rot_items: u64,
    pub retention_decays: u64,
    pub retention_removals: u64,
    pub compaction_merges: u64,
    pub compaction_removals: u64,
    pub external_feedback_batches: u64,
    pub external_feedback_applied: u64,
    pub external_feedback_missing: u64,
    pub external_feedback_removed: u64,
    pub external_feedback_strength_delta: f32,
    pub drift_rollbacks: u64,
    pub index_quality_blockers: u64,
    pub index_quality_warnings: u64,
    pub kvswap_boundary_blockers: u64,
    pub kvswap_boundary_warnings: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryHygienePressure {
    pub score: u64,
    pub index_quality_blockers: u64,
    pub index_quality_warnings: u64,
    pub kvswap_boundary_blockers: u64,
    pub kvswap_boundary_warnings: u64,
    pub context_rot_items: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MemoryAutophagyRecallSignals {
    pub rejected_context_count: u64,
    pub duplicate_reject_count: u64,
    pub missing_kv_count: u64,
    pub unsafe_sidecar_reject_count: u64,
}

impl MemoryAutophagyRecallSignals {
    pub fn noise_score(&self) -> u64 {
        self.rejected_context_count
            .saturating_add(self.duplicate_reject_count.saturating_mul(2))
            .saturating_add(self.missing_kv_count)
            .saturating_add(self.unsafe_sidecar_reject_count.saturating_mul(5))
    }

    pub fn active_recall_prune_candidates(&self) -> usize {
        self.rejected_context_count
            .saturating_add(self.duplicate_reject_count)
            .saturating_add(self.missing_kv_count)
            .saturating_add(self.unsafe_sidecar_reject_count) as usize
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if self.rejected_context_count > 0 {
            codes.insert("recall_rejected_context".to_owned());
        }
        if self.duplicate_reject_count > 0 {
            codes.insert("recall_duplicate".to_owned());
        }
        if self.missing_kv_count > 0 {
            codes.insert("recall_missing_kv".to_owned());
        }
        if self.unsafe_sidecar_reject_count > 0 {
            codes.insert("recall_unsafe_sidecar".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if self.rejected_context_count > 0 {
            codes.insert(format!("recall_rejected:{}", self.rejected_context_count));
        }
        if self.duplicate_reject_count > 0 {
            codes.insert(format!("recall_duplicate:{}", self.duplicate_reject_count));
        }
        if self.missing_kv_count > 0 {
            codes.insert(format!("recall_missing_kv:{}", self.missing_kv_count));
        }
        if self.unsafe_sidecar_reject_count > 0 {
            codes.insert(format!(
                "recall_unsafe_sidecar:{}",
                self.unsafe_sidecar_reject_count
            ));
        }
        codes.into_iter().collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryAutophagyPlan {
    pub context_pressure_score: u64,
    pub retrieval_noise_score: u64,
    pub stale_decay_candidates: usize,
    pub duplicate_merge_candidates: usize,
    pub gist_recomposition_candidates: usize,
    pub active_recall_prune_candidates: usize,
    pub quarantine_candidates: usize,
    live_delete_allowed: bool,
    durable_mutation_allowed: bool,
    pub reason_codes: Vec<String>,
    pub detail_codes: Vec<String>,
}

impl MemoryAutophagyPlan {
    fn from_counts(
        context_pressure_score: u64,
        retrieval_noise_score: u64,
        stale_decay_candidates: usize,
        duplicate_merge_candidates: usize,
        active_recall_prune_candidates: usize,
        quarantine_candidates: usize,
        reason_codes: BTreeSet<String>,
        detail_codes: BTreeSet<String>,
    ) -> Self {
        let gist_recomposition_candidates =
            stale_decay_candidates.saturating_add(duplicate_merge_candidates);
        let mut reason_codes = reason_codes;
        if stale_decay_candidates > 0 {
            reason_codes.insert("recycle_preview".to_owned());
        }
        if duplicate_merge_candidates > 0 {
            reason_codes.insert("gist_recomposition_preview".to_owned());
        }
        if active_recall_prune_candidates > 0 {
            reason_codes.insert("active_recall_prune_preview".to_owned());
        }
        if quarantine_candidates > 0 {
            reason_codes.insert("quarantine_preview".to_owned());
        }
        if reason_codes.is_empty() {
            reason_codes.insert("clean".to_owned());
        }

        Self {
            context_pressure_score,
            retrieval_noise_score,
            stale_decay_candidates,
            duplicate_merge_candidates,
            gist_recomposition_candidates,
            active_recall_prune_candidates,
            quarantine_candidates,
            live_delete_allowed: false,
            durable_mutation_allowed: false,
            reason_codes: reason_codes.into_iter().collect(),
            detail_codes: detail_codes.into_iter().collect(),
        }
    }

    pub fn from_signals(
        retention: &MemoryRetentionPlan,
        compaction: &MemoryCompactionPlan,
        hygiene: &MemoryHygienePressure,
        recall: &MemoryAutophagyRecallSignals,
    ) -> Self {
        let stale_decay_candidates = retention.decays.len();
        let duplicate_merge_candidates = compaction.merges.len();
        let active_recall_prune_candidates = recall.active_recall_prune_candidates();
        let quarantine_candidates = retention
            .removals
            .len()
            .saturating_add(hygiene.blocker_count() as usize)
            .saturating_add(recall.unsafe_sidecar_reject_count as usize);
        let mut reason_codes = BTreeSet::new();
        let mut detail_codes = BTreeSet::new();

        reason_codes.extend(retention.reason_codes());
        reason_codes.extend(compaction.reason_codes());
        reason_codes.extend(hygiene.reason_codes());
        reason_codes.extend(recall.reason_codes());
        detail_codes.extend(retention.detail_codes());
        detail_codes.extend(compaction.detail_codes());
        detail_codes.extend(hygiene.detail_codes());
        detail_codes.extend(recall.detail_codes());

        Self::from_counts(
            hygiene.score,
            recall.noise_score(),
            stale_decay_candidates,
            duplicate_merge_candidates,
            active_recall_prune_candidates,
            quarantine_candidates,
            reason_codes,
            detail_codes,
        )
    }

    pub fn preview_only(&self) -> bool {
        !self.live_delete_allowed && !self.durable_mutation_allowed
    }

    pub fn live_delete_allowed(&self) -> bool {
        self.live_delete_allowed
    }

    pub fn durable_mutation_allowed(&self) -> bool {
        self.durable_mutation_allowed
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_autophagy_preview context_pressure_score={} retrieval_noise_score={} stale_decay_candidates={} duplicate_merge_candidates={} gist_recomposition_candidates={} active_recall_prune_candidates={} quarantine_candidates={} live_delete_allowed={} durable_mutation_allowed={} reason_codes={}",
            self.context_pressure_score,
            self.retrieval_noise_score,
            self.stale_decay_candidates,
            self.duplicate_merge_candidates,
            self.gist_recomposition_candidates,
            self.active_recall_prune_candidates,
            self.quarantine_candidates,
            self.live_delete_allowed,
            self.durable_mutation_allowed,
            join_codes(self.reason_codes.clone()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryHygieneActionLane {
    pub lane_code: String,
    pub priority_code: String,
    pub score: u64,
    pub item_count: u64,
}

impl MemoryHygieneActionLane {
    fn new(lane_code: &str, priority_code: &str, score: u64, item_count: u64) -> Self {
        Self {
            lane_code: lane_code.to_owned(),
            priority_code: priority_code.to_owned(),
            score,
            item_count,
        }
    }

    pub fn detail_code(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.lane_code, self.priority_code, self.score, self.item_count
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_hygiene_action_lane lane={} priority={} score={} items={} detail_code={}",
            self.lane_code,
            self.priority_code,
            self.score,
            self.item_count,
            self.detail_code()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryHygieneWorkItem {
    pub lane_code: String,
    pub priority_code: String,
    pub score: u64,
    pub item_count: u64,
    pub operator_review_required: bool,
    pub isolation_recommended: bool,
}

impl MemoryHygieneWorkItem {
    fn from_lane(
        lane: &MemoryHygieneActionLane,
        operator_review_required: bool,
        isolation_recommended: bool,
    ) -> Self {
        Self {
            lane_code: lane.lane_code.clone(),
            priority_code: lane.priority_code.clone(),
            score: lane.score,
            item_count: lane.item_count,
            operator_review_required,
            isolation_recommended,
        }
    }

    pub fn dispatch_code(&self) -> String {
        let review_code = if self.operator_review_required {
            "operator_review"
        } else {
            "auto"
        };
        let isolation_code = if self.isolation_recommended {
            "isolated"
        } else {
            "shared"
        };
        format!(
            "dispatch:{}:{}:{}:{}:{}:{}",
            review_code,
            isolation_code,
            self.lane_code,
            self.priority_code,
            self.score,
            self.item_count
        )
    }

    pub fn detail_code(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.lane_code, self.priority_code, self.score, self.item_count
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_hygiene_work_item lane={} priority={} score={} items={} operator_review={} isolation={} dispatch_code={} detail_code={}",
            self.lane_code,
            self.priority_code,
            self.score,
            self.item_count,
            self.operator_review_required,
            self.isolation_recommended,
            self.dispatch_code(),
            self.detail_code()
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryHygieneWorkQueue {
    pub clean: bool,
    pub total_score: u64,
    pub item_count: usize,
    pub operator_review_count: usize,
    pub isolation_count: usize,
    pub next_dispatch_code: String,
    pub lane_codes: Vec<String>,
    pub priority_codes: Vec<String>,
    pub dispatch_codes: Vec<String>,
    pub detail_codes: Vec<String>,
}

impl MemoryHygieneWorkQueue {
    pub fn from_plan(plan: &MemoryHygieneWorkPlan) -> Self {
        let items = plan.work_items();
        let mut lane_codes = items
            .iter()
            .map(|item| item.lane_code.clone())
            .collect::<Vec<_>>();
        let mut priority_codes = items
            .iter()
            .map(|item| item.priority_code.clone())
            .collect::<Vec<_>>();
        let mut dispatch_codes = if items.is_empty() {
            vec![plan.next_dispatch_code()]
        } else {
            items
                .iter()
                .map(MemoryHygieneWorkItem::dispatch_code)
                .collect::<Vec<_>>()
        };
        let mut detail_codes = items
            .iter()
            .map(MemoryHygieneWorkItem::detail_code)
            .collect::<Vec<_>>();

        lane_codes.sort();
        lane_codes.dedup();
        priority_codes.sort();
        priority_codes.dedup();
        dispatch_codes.sort();
        dispatch_codes.dedup();
        detail_codes.sort();
        detail_codes.dedup();

        Self {
            clean: plan.clean,
            total_score: plan.total_score,
            item_count: items.len(),
            operator_review_count: items
                .iter()
                .filter(|item| item.operator_review_required)
                .count(),
            isolation_count: items
                .iter()
                .filter(|item| item.isolation_recommended)
                .count(),
            next_dispatch_code: plan.next_dispatch_code(),
            lane_codes,
            priority_codes,
            dispatch_codes,
            detail_codes,
        }
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if self.clean {
            codes.insert("clean".to_owned());
        }
        if self.item_count > 0 {
            codes.insert("items_present".to_owned());
        }
        if self.operator_review_count > 0 {
            codes.insert("operator_review_required".to_owned());
        }
        if self.isolation_count > 0 {
            codes.insert("isolation_recommended".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_hygiene_work_queue clean={} total_score={} items={} operator_review={} isolation={} next_dispatch={} lanes={} priorities={} dispatch_codes={} detail_codes={} reason_codes={}",
            self.clean,
            self.total_score,
            self.item_count,
            self.operator_review_count,
            self.isolation_count,
            self.next_dispatch_code,
            join_codes(self.lane_codes.clone()),
            join_codes(self.priority_codes.clone()),
            join_codes(self.dispatch_codes.clone()),
            join_codes(self.detail_codes.clone()),
            join_codes(self.reason_codes()),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryHygieneWorkPlan {
    pub clean: bool,
    pub total_score: u64,
    pub lane_count: usize,
    pub next_action_code: String,
    pub operator_review_required: bool,
    pub isolation_recommended: bool,
    pub action_lane_codes: Vec<String>,
    pub action_lane_detail_codes: Vec<String>,
}

impl MemoryHygieneWorkPlan {
    pub fn from_pressure(pressure: &MemoryHygienePressure) -> Self {
        let lanes = pressure.action_lanes();
        let next_action_code = lanes
            .first()
            .map(|lane| lane.lane_code.clone())
            .unwrap_or_else(|| "none".to_owned());
        let operator_review_required = lanes.iter().any(|lane| lane.priority_code == "quarantine");
        let isolation_recommended = lanes
            .first()
            .is_some_and(|lane| lane.priority_code == "quarantine");

        Self {
            clean: pressure.is_clean(),
            total_score: pressure.score,
            lane_count: lanes.len(),
            next_action_code,
            operator_review_required,
            isolation_recommended,
            action_lane_codes: lanes.iter().map(|lane| lane.lane_code.clone()).collect(),
            action_lane_detail_codes: lanes
                .iter()
                .map(MemoryHygieneActionLane::detail_code)
                .collect(),
        }
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if self.clean {
            codes.insert("clean".to_owned());
        }
        if self.lane_count > 0 {
            codes.insert("lanes_present".to_owned());
        }
        if self.operator_review_required {
            codes.insert("operator_review_required".to_owned());
        }
        if self.isolation_recommended {
            codes.insert("isolation_recommended".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        codes.insert(format!("next_action:{}", self.next_action_code));
        codes.insert(format!("lane_count:{}", self.lane_count));
        codes.insert(format!("total_score:{}", self.total_score));
        codes.extend(
            self.action_lane_detail_codes
                .iter()
                .map(|detail| format!("lane:{detail}")),
        );
        codes.into_iter().collect()
    }

    pub fn work_items(&self) -> Vec<MemoryHygieneWorkItem> {
        if self.clean || self.action_lane_codes.is_empty() {
            return Vec::new();
        }

        self.action_lane_detail_codes
            .iter()
            .filter_map(|detail| {
                let mut parts = detail.split(':');
                let lane_code = parts.next()?;
                let priority_code = parts.next()?;
                let score = parts.next()?.parse::<u64>().ok()?;
                let item_count = parts.next()?.parse::<u64>().ok()?;
                if parts.next().is_some() {
                    return None;
                }
                let lane =
                    MemoryHygieneActionLane::new(lane_code, priority_code, score, item_count);
                Some(MemoryHygieneWorkItem::from_lane(
                    &lane,
                    self.operator_review_required,
                    self.isolation_recommended,
                ))
            })
            .collect()
    }

    pub fn dispatch_codes(&self) -> Vec<String> {
        let work_items = self.work_items();
        if work_items.is_empty() {
            return vec!["dispatch:clean:auto:shared:none".to_owned()];
        }

        work_items
            .into_iter()
            .map(|item| item.dispatch_code())
            .collect()
    }

    pub fn next_dispatch_code(&self) -> String {
        self.dispatch_codes()
            .into_iter()
            .next()
            .unwrap_or_else(|| "dispatch:clean:auto:shared:none".to_owned())
    }

    pub fn work_queue(&self) -> MemoryHygieneWorkQueue {
        MemoryHygieneWorkQueue::from_plan(self)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_hygiene_work_plan clean={} total_score={} lanes={} next_action={} operator_review={} isolation={} action_lanes={} action_lane_details={} dispatch_next={} dispatch_codes={} reason_codes={} detail_codes={}",
            self.clean,
            self.total_score,
            self.lane_count,
            self.next_action_code,
            self.operator_review_required,
            self.isolation_recommended,
            join_codes(self.action_lane_codes.clone()),
            join_codes(self.action_lane_detail_codes.clone()),
            self.next_dispatch_code(),
            join_codes(self.dispatch_codes()),
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }
}

impl MemoryHygienePressure {
    pub fn from_ledger(ledger: &MemoryEvolutionLedger) -> Self {
        Self::from_counts(
            ledger.index_quality_blockers,
            ledger.index_quality_warnings,
            ledger.kvswap_boundary_blockers,
            ledger.kvswap_boundary_warnings,
            ledger.context_rot_items,
        )
    }

    pub fn from_quality_gate_and_boundary(
        gate: &ExperienceIndexQualityGate,
        boundary: Option<&KvSwapBoundaryReadiness>,
    ) -> Self {
        let (kvswap_boundary_blockers, kvswap_boundary_warnings) = boundary
            .map(|readiness| {
                (
                    readiness.blocker_count as u64,
                    readiness.warning_count as u64,
                )
            })
            .unwrap_or_default();
        Self::from_counts(
            gate.blocker_count as u64,
            gate.warning_count as u64,
            kvswap_boundary_blockers,
            kvswap_boundary_warnings,
            0,
        )
    }

    fn from_counts(
        index_quality_blockers: u64,
        index_quality_warnings: u64,
        kvswap_boundary_blockers: u64,
        kvswap_boundary_warnings: u64,
        context_rot_items: u64,
    ) -> Self {
        let score = index_quality_blockers
            .saturating_add(kvswap_boundary_blockers)
            .saturating_mul(100)
            .saturating_add(
                index_quality_warnings
                    .saturating_add(kvswap_boundary_warnings)
                    .saturating_mul(10),
            )
            .saturating_add(context_rot_items.saturating_mul(5));

        Self {
            score,
            index_quality_blockers,
            index_quality_warnings,
            kvswap_boundary_blockers,
            kvswap_boundary_warnings,
            context_rot_items,
        }
    }

    pub fn is_clean(&self) -> bool {
        self.score == 0
    }

    pub fn blocker_count(&self) -> u64 {
        self.index_quality_blockers
            .saturating_add(self.kvswap_boundary_blockers)
    }

    pub fn warning_count(&self) -> u64 {
        self.index_quality_warnings
            .saturating_add(self.kvswap_boundary_warnings)
    }

    pub fn priority_code(&self) -> &'static str {
        if self.blocker_count() > 0 || self.context_rot_items > 8 {
            "quarantine"
        } else if self.warning_count() > 0 || self.context_rot_items > 0 {
            "repair"
        } else {
            "clean"
        }
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if self.index_quality_blockers > 0 {
            codes.insert("index_quality_blocker".to_owned());
        }
        if self.index_quality_warnings > 0 {
            codes.insert("index_quality_warning".to_owned());
        }
        if self.kvswap_boundary_blockers > 0 {
            codes.insert("kvswap_boundary_blocker".to_owned());
        }
        if self.kvswap_boundary_warnings > 0 {
            codes.insert("kvswap_boundary_warning".to_owned());
        }
        if self.context_rot_items > 0 {
            codes.insert("context_rot_pressure".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if self.index_quality_blockers > 0 {
            codes.insert(format!(
                "index_quality_blockers:{}",
                self.index_quality_blockers
            ));
        }
        if self.index_quality_warnings > 0 {
            codes.insert(format!(
                "index_quality_warnings:{}",
                self.index_quality_warnings
            ));
        }
        if self.kvswap_boundary_blockers > 0 {
            codes.insert(format!(
                "kvswap_boundary_blockers:{}",
                self.kvswap_boundary_blockers
            ));
        }
        if self.kvswap_boundary_warnings > 0 {
            codes.insert(format!(
                "kvswap_boundary_warnings:{}",
                self.kvswap_boundary_warnings
            ));
        }
        if self.context_rot_items > 0 {
            codes.insert(format!("context_rot_items:{}", self.context_rot_items));
        }
        codes.into_iter().collect()
    }

    pub fn action_lanes(&self) -> Vec<MemoryHygieneActionLane> {
        let mut lanes = Vec::new();
        let index_items = self
            .index_quality_blockers
            .saturating_add(self.index_quality_warnings);
        if index_items > 0 {
            lanes.push(MemoryHygieneActionLane::new(
                "experience_index_rebuild",
                if self.index_quality_blockers > 0 {
                    "quarantine"
                } else {
                    "repair"
                },
                self.index_quality_blockers
                    .saturating_mul(100)
                    .saturating_add(self.index_quality_warnings.saturating_mul(10)),
                index_items,
            ));
        }

        let kvswap_items = self
            .kvswap_boundary_blockers
            .saturating_add(self.kvswap_boundary_warnings);
        if kvswap_items > 0 {
            lanes.push(MemoryHygieneActionLane::new(
                "kvswap_boundary_repair",
                if self.kvswap_boundary_blockers > 0 {
                    "quarantine"
                } else {
                    "repair"
                },
                self.kvswap_boundary_blockers
                    .saturating_mul(100)
                    .saturating_add(self.kvswap_boundary_warnings.saturating_mul(10)),
                kvswap_items,
            ));
        }

        if self.context_rot_items > 0 {
            lanes.push(MemoryHygieneActionLane::new(
                "context_rot_review",
                if self.context_rot_items > 8 {
                    "quarantine"
                } else {
                    "repair"
                },
                self.context_rot_items.saturating_mul(5),
                self.context_rot_items,
            ));
        }

        lanes.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.lane_code.cmp(&right.lane_code))
        });
        lanes
    }

    pub fn action_lane_codes(&self) -> Vec<String> {
        self.action_lanes()
            .into_iter()
            .map(|lane| lane.lane_code)
            .collect()
    }

    pub fn action_lane_detail_codes(&self) -> Vec<String> {
        self.action_lanes()
            .into_iter()
            .map(|lane| lane.detail_code())
            .collect()
    }

    pub fn work_plan(&self) -> MemoryHygieneWorkPlan {
        MemoryHygieneWorkPlan::from_pressure(self)
    }

    pub fn summary_line(&self) -> String {
        let work_plan = self.work_plan();
        format!(
            "memory_hygiene_pressure clean={} priority={} score={} blockers={} warnings={} context_rot_items={} index_quality_blockers={} index_quality_warnings={} kvswap_boundary_blockers={} kvswap_boundary_warnings={} action_lanes={} action_lane_details={} work_next_action={} work_operator_review={} work_isolation={} reason_codes={} detail_codes={}",
            self.is_clean(),
            self.priority_code(),
            self.score,
            self.blocker_count(),
            self.warning_count(),
            self.context_rot_items,
            self.index_quality_blockers,
            self.index_quality_warnings,
            self.kvswap_boundary_blockers,
            self.kvswap_boundary_warnings,
            join_codes(self.action_lane_codes()),
            join_codes(self.action_lane_detail_codes()),
            work_plan.next_action_code,
            work_plan.operator_review_required,
            work_plan.isolation_recommended,
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }
}

impl MemoryEvolutionLedger {
    pub fn record_replay_report(&mut self, report: &ReplayReport) {
        if report.planned == 0 {
            return;
        }
        self.replay_runs = self.replay_runs.saturating_add(1);
        self.replay_items = self.replay_items.saturating_add(report.planned as u64);
        self.replay_reinforcements = self
            .replay_reinforcements
            .saturating_add(report.reinforced as u64);
        self.replay_penalties = self
            .replay_penalties
            .saturating_add(report.penalized as u64);
        self.replay_memory_updates = self
            .replay_memory_updates
            .saturating_add(report.memory_reinforcements as u64)
            .saturating_add(report.memory_penalties as u64);
        self.recursive_runtime_items = self
            .recursive_runtime_items
            .saturating_add(report.recursive_runtime_items as u64);
        self.live_memory_feedback_items = self
            .live_memory_feedback_items
            .saturating_add(report.live_memory_feedback_items as u64);
        self.context_rot_items = self
            .context_rot_items
            .saturating_add(report.context_rot_items as u64);
        self.external_feedback_applied = self
            .external_feedback_applied
            .saturating_add(report.feedback_applied as u64);
        self.external_feedback_missing = self
            .external_feedback_missing
            .saturating_add(report.feedback_missing as u64);
        self.external_feedback_removed = self
            .external_feedback_removed
            .saturating_add(report.feedback_removed as u64);
        self.external_feedback_strength_delta += report.feedback_strength_delta.abs();
    }

    pub fn record_replay_apply_report(&mut self, report: &ReplayApplyReport) {
        self.replay_memory_updates = self
            .replay_memory_updates
            .saturating_add(report.strength_updates() as u64);
        self.replay_memory_missing = self
            .replay_memory_missing
            .saturating_add(report.missing as u64);
        self.replay_invalid_memory_ids = self
            .replay_invalid_memory_ids
            .saturating_add(report.invalid_memory_ids.len() as u64);
    }

    pub fn record_retention_plan(&mut self, plan: &MemoryRetentionPlan) {
        self.retention_decays = self
            .retention_decays
            .saturating_add(plan.decays.len() as u64);
        self.retention_removals = self
            .retention_removals
            .saturating_add(plan.removals.len() as u64);
    }

    pub fn record_compaction_plan(&mut self, plan: &MemoryCompactionPlan) {
        self.compaction_merges = self
            .compaction_merges
            .saturating_add(plan.merges.len() as u64);
        self.compaction_removals = self
            .compaction_removals
            .saturating_add(plan.removed_ids.len() as u64);
    }

    pub fn record_external_feedback(
        &mut self,
        applied: usize,
        missing: usize,
        removed: usize,
        strength_delta: f32,
    ) {
        if applied == 0 && missing == 0 && removed == 0 {
            return;
        }
        self.external_feedback_batches = self.external_feedback_batches.saturating_add(1);
        self.external_feedback_applied = self
            .external_feedback_applied
            .saturating_add(applied as u64);
        self.external_feedback_missing = self
            .external_feedback_missing
            .saturating_add(missing as u64);
        self.external_feedback_removed = self
            .external_feedback_removed
            .saturating_add(removed as u64);
        if strength_delta.is_finite() {
            self.external_feedback_strength_delta += strength_delta.abs();
        }
    }

    pub fn record_drift_rollback(&mut self) {
        self.drift_rollbacks = self.drift_rollbacks.saturating_add(1);
    }

    pub fn record_index_quality_gate(&mut self, gate: &ExperienceIndexQualityGate) {
        self.index_quality_blockers = self
            .index_quality_blockers
            .saturating_add(gate.blocker_count as u64);
        self.index_quality_warnings = self
            .index_quality_warnings
            .saturating_add(gate.warning_count as u64);
    }

    pub fn record_kvswap_boundary_readiness(&mut self, readiness: &KvSwapBoundaryReadiness) {
        self.kvswap_boundary_blockers = self
            .kvswap_boundary_blockers
            .saturating_add(readiness.blocker_count as u64);
        self.kvswap_boundary_warnings = self
            .kvswap_boundary_warnings
            .saturating_add(readiness.warning_count as u64);
    }

    pub fn replay_evidence_checklist_detail(&self) -> String {
        format!(
            "replay_runs={} replay_items={} replay_updates={}",
            self.replay_runs, self.replay_items, self.replay_memory_updates
        )
    }

    pub fn maintenance_actions(&self) -> u64 {
        self.retention_decays
            .saturating_add(self.retention_removals)
            .saturating_add(self.compaction_merges)
            .saturating_add(self.compaction_removals)
    }

    pub fn missing_memory_update_ratio(&self) -> f32 {
        let failed = self
            .replay_memory_missing
            .saturating_add(self.replay_invalid_memory_ids)
            .saturating_add(self.external_feedback_missing);
        let total = self
            .replay_memory_updates
            .saturating_add(failed)
            .saturating_add(self.external_feedback_applied);
        if total == 0 {
            0.0
        } else {
            failed as f32 / total as f32
        }
    }

    pub fn hygiene_pressure(&self) -> MemoryHygienePressure {
        MemoryHygienePressure::from_ledger(self)
    }

    pub fn autophagy_plan(&self) -> MemoryAutophagyPlan {
        let hygiene_pressure = self.hygiene_pressure();
        let recall = MemoryAutophagyRecallSignals {
            rejected_context_count: self.replay_memory_missing,
            duplicate_reject_count: 0,
            missing_kv_count: self
                .replay_invalid_memory_ids
                .saturating_add(self.external_feedback_missing),
            unsafe_sidecar_reject_count: 0,
        };
        let active_recall_prune_candidates = recall
            .active_recall_prune_candidates()
            .saturating_add(self.context_rot_items as usize);
        let retrieval_noise_score = recall
            .noise_score()
            .saturating_add(self.context_rot_items.saturating_mul(5));
        let quarantine_candidates = (self.retention_removals as usize)
            .saturating_add(hygiene_pressure.blocker_count() as usize);
        let mut reason_codes = BTreeSet::new();
        let mut detail_codes = BTreeSet::new();

        reason_codes.extend(self.reason_codes());
        reason_codes.extend(hygiene_pressure.reason_codes());
        reason_codes.extend(recall.reason_codes());
        detail_codes.extend(hygiene_pressure.detail_codes());
        detail_codes.extend(recall.detail_codes());

        MemoryAutophagyPlan::from_counts(
            hygiene_pressure.score,
            retrieval_noise_score,
            self.retention_decays as usize,
            self.compaction_merges as usize,
            active_recall_prune_candidates,
            quarantine_candidates,
            reason_codes,
            detail_codes,
        )
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        if self.replay_runs > 0 {
            codes.insert("replay_evidence".to_owned());
        }
        if self.replay_memory_updates > 0 {
            codes.insert("replay_memory_update".to_owned());
        }
        if self.replay_memory_missing > 0 {
            codes.insert("replay_missing_memory".to_owned());
        }
        if self.replay_invalid_memory_ids > 0 {
            codes.insert("invalid_memory_id".to_owned());
        }
        if self.recursive_runtime_items > 0 {
            codes.insert("recursive_runtime".to_owned());
        }
        if self.live_memory_feedback_items > 0 {
            codes.insert("live_memory_feedback".to_owned());
        }
        if self.context_rot_items > 0 {
            codes.insert("context_rot".to_owned());
        }
        if self.retention_decays > 0 {
            codes.insert("retention_decay".to_owned());
        }
        if self.retention_removals > 0 {
            codes.insert("retention_removal".to_owned());
        }
        if self.compaction_merges > 0 {
            codes.insert("compaction_merge".to_owned());
        }
        if self.compaction_removals > 0 {
            codes.insert("compaction_removal".to_owned());
        }
        if self.external_feedback_applied > 0 {
            codes.insert("external_feedback_applied".to_owned());
        }
        if self.external_feedback_missing > 0 {
            codes.insert("external_feedback_missing".to_owned());
        }
        if self.external_feedback_removed > 0 {
            codes.insert("external_feedback_removed".to_owned());
        }
        if self.drift_rollbacks > 0 {
            codes.insert("drift_rollback".to_owned());
        }
        if self.index_quality_blockers > 0 {
            codes.insert("index_quality_blocker".to_owned());
        }
        if self.index_quality_warnings > 0 {
            codes.insert("index_quality_warning".to_owned());
        }
        if self.kvswap_boundary_blockers > 0 {
            codes.insert("kvswap_boundary_blocker".to_owned());
        }
        if self.kvswap_boundary_warnings > 0 {
            codes.insert("kvswap_boundary_warning".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn summary_line(&self) -> String {
        let hygiene_pressure = self.hygiene_pressure();
        let hygiene_work_plan = hygiene_pressure.work_plan();
        let autophagy_plan = self.autophagy_plan();
        format!(
            "memory_evolution replay_runs={} replay_items={} replay_updates={} replay_missing={} invalid_memory_ids={} context_rot_items={} live_feedback_items={} retention_decays={} retention_removals={} compaction_merges={} compaction_removals={} external_applied={} external_missing={} drift_rollbacks={} index_quality_blockers={} index_quality_warnings={} kvswap_boundary_blockers={} kvswap_boundary_warnings={} hygiene_pressure_score={} hygiene_pressure_priority={} hygiene_pressure_action_lanes={} hygiene_pressure_action_lane_details={} hygiene_work_next_action={} hygiene_work_operator_review={} hygiene_work_isolation={} autophagy_context_pressure_score={} autophagy_retrieval_noise_score={} autophagy_stale_decay_candidates={} autophagy_duplicate_merge_candidates={} autophagy_gist_recomposition_candidates={} autophagy_active_recall_prune_candidates={} autophagy_quarantine_candidates={} autophagy_live_delete_allowed={} autophagy_durable_mutation_allowed={} hygiene_pressure_reason_codes={} hygiene_pressure_detail_codes={} reason_codes={}",
            self.replay_runs,
            self.replay_items,
            self.replay_memory_updates,
            self.replay_memory_missing,
            self.replay_invalid_memory_ids,
            self.context_rot_items,
            self.live_memory_feedback_items,
            self.retention_decays,
            self.retention_removals,
            self.compaction_merges,
            self.compaction_removals,
            self.external_feedback_applied,
            self.external_feedback_missing,
            self.drift_rollbacks,
            self.index_quality_blockers,
            self.index_quality_warnings,
            self.kvswap_boundary_blockers,
            self.kvswap_boundary_warnings,
            hygiene_pressure.score,
            hygiene_pressure.priority_code(),
            join_codes(hygiene_pressure.action_lane_codes()),
            join_codes(hygiene_pressure.action_lane_detail_codes()),
            hygiene_work_plan.next_action_code,
            hygiene_work_plan.operator_review_required,
            hygiene_work_plan.isolation_recommended,
            autophagy_plan.context_pressure_score,
            autophagy_plan.retrieval_noise_score,
            autophagy_plan.stale_decay_candidates,
            autophagy_plan.duplicate_merge_candidates,
            autophagy_plan.gist_recomposition_candidates,
            autophagy_plan.active_recall_prune_candidates,
            autophagy_plan.quarantine_candidates,
            autophagy_plan.live_delete_allowed,
            autophagy_plan.durable_mutation_allowed,
            join_codes(hygiene_pressure.reason_codes()),
            join_codes(hygiene_pressure.detail_codes()),
            join_codes(self.reason_codes()),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryEvolutionPolicy {
    pub max_missing_memory_update_ratio: f32,
    pub max_context_rot_items: u64,
    pub max_drift_rollbacks: u64,
    pub require_replay_before_live_write: bool,
}

impl Default for MemoryEvolutionPolicy {
    fn default() -> Self {
        Self {
            max_missing_memory_update_ratio: 0.35,
            max_context_rot_items: 8,
            max_drift_rollbacks: 0,
            require_replay_before_live_write: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryEvolutionAssessment {
    pub allow_isolated_write: bool,
    pub rollback_recommended: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

impl MemoryEvolutionAssessment {
    pub fn requires_operator_review(&self) -> bool {
        !self.allow_isolated_write || self.rollback_recommended || !self.warnings.is_empty()
    }

    pub fn blocker_codes(&self) -> Vec<String> {
        metric_codes(&self.blockers)
    }

    pub fn warning_codes(&self) -> Vec<String> {
        metric_codes(&self.warnings)
    }

    pub fn blocker_detail_codes(&self) -> Vec<String> {
        metric_detail_codes(&self.blockers)
    }

    pub fn warning_detail_codes(&self) -> Vec<String> {
        metric_detail_codes(&self.warnings)
    }

    pub fn checklist_detail(&self) -> String {
        format!(
            "evolution_blockers={} warnings={} blocker_codes={} warning_codes={}",
            self.blockers.len(),
            self.warnings.len(),
            join_codes(self.blocker_codes()),
            join_codes(self.warning_codes())
        )
    }
}

fn metric_codes(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|item| item.split_once('=').map_or(item.as_str(), |(code, _)| code))
        .filter(|code| !code.is_empty())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn metric_detail_codes(items: &[String]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| {
            let item = item.trim();
            if item.is_empty() {
                None
            } else if let Some((code, value)) = item.split_once('=') {
                Some(format!("{code}:{}", metric_value_code(value)))
            } else {
                Some(item.to_owned())
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn metric_value_code(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
        .collect()
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

pub trait MemoryEvolutionGate {
    fn assess(&self, ledger: &MemoryEvolutionLedger) -> MemoryEvolutionAssessment;
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DefaultMemoryEvolutionGate {
    pub policy: MemoryEvolutionPolicy,
}

impl MemoryAdapter for DefaultMemoryEvolutionGate {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_memory_evolution_gate",
            vec![MemoryAdapterCapability::MemoryEvolution],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(None))
    }
}

impl MemoryEvolutionGate for DefaultMemoryEvolutionGate {
    fn assess(&self, ledger: &MemoryEvolutionLedger) -> MemoryEvolutionAssessment {
        let mut blockers = Vec::new();
        let mut warnings = Vec::new();
        let missing_ratio = ledger.missing_memory_update_ratio();

        if self.policy.require_replay_before_live_write && ledger.replay_runs == 0 {
            blockers.push("missing_replay_evidence".to_owned());
        }
        if missing_ratio > self.policy.max_missing_memory_update_ratio {
            blockers.push(format!("memory_update_missing_ratio={missing_ratio:.3}"));
        }
        if ledger.context_rot_items > self.policy.max_context_rot_items {
            warnings.push(format!("context_rot_items={}", ledger.context_rot_items));
        }
        let rollback_recommended = ledger.drift_rollbacks > self.policy.max_drift_rollbacks;
        if rollback_recommended {
            warnings.push(format!("drift_rollbacks={}", ledger.drift_rollbacks));
        }
        if ledger.maintenance_actions() > ledger.replay_items.saturating_mul(2).max(8) {
            warnings.push(format!(
                "maintenance_actions={}",
                ledger.maintenance_actions()
            ));
        }
        if ledger.index_quality_blockers > 0 {
            warnings.push(format!(
                "index_quality_blockers={}",
                ledger.index_quality_blockers
            ));
        }
        if ledger.index_quality_warnings > 0 {
            warnings.push(format!(
                "index_quality_warnings={}",
                ledger.index_quality_warnings
            ));
        }
        if ledger.kvswap_boundary_blockers > 0 {
            warnings.push(format!(
                "kvswap_boundary_blockers={}",
                ledger.kvswap_boundary_blockers
            ));
        }
        if ledger.kvswap_boundary_warnings > 0 {
            warnings.push(format!(
                "kvswap_boundary_warnings={}",
                ledger.kvswap_boundary_warnings
            ));
        }
        let hygiene_pressure = ledger.hygiene_pressure();
        if hygiene_pressure.blocker_count() > 0 || hygiene_pressure.warning_count() > 0 {
            warnings.push(format!(
                "memory_hygiene_pressure={}",
                hygiene_pressure.score
            ));
        }
        warnings.sort();
        warnings.dedup();

        MemoryEvolutionAssessment {
            allow_isolated_write: blockers.is_empty(),
            rollback_recommended,
            blockers,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DefaultExperienceGovernance, ExperienceEnvelope, ExperienceGovernance, KvSwapBoundaryAudit,
        MemoryCompactionMerge, MemoryDecay, MemoryRetentionRemoval, ReplayAction,
        ReplayApplyReport, ReplayItem, ReplayMemoryUpdate, ReplayPlan, ReplaySignal,
    };

    #[test]
    fn evolution_ledger_records_replay_and_apply_reports() {
        let plan = ReplayPlan {
            items: vec![
                ReplayItem {
                    experience_id: "good".to_owned(),
                    action: ReplayAction::Reinforce,
                    reward: 0.9,
                    quality: 0.9,
                    priority: 0.9,
                    lesson: "lesson".to_owned(),
                    memory_updates: vec![ReplayMemoryUpdate {
                        memory_id: "1".to_owned(),
                        source_experience_id: "good".to_owned(),
                        action: ReplayAction::Reinforce,
                        amount: 0.9,
                    }],
                    feedback: None,
                    signals: vec![ReplaySignal::RecursiveRuntime],
                },
                ReplayItem {
                    experience_id: "rot".to_owned(),
                    action: ReplayAction::Penalize,
                    reward: 0.1,
                    quality: 0.1,
                    priority: 1.0,
                    lesson: "avoid rot".to_owned(),
                    memory_updates: Vec::new(),
                    feedback: None,
                    signals: vec![ReplaySignal::ContextRot],
                },
            ],
        };
        let replay = ReplayReport::from_plan(&plan);
        let apply = ReplayApplyReport {
            requested: 2,
            applied: 1,
            reinforced: 1,
            missing: 1,
            invalid_memory_ids: vec!["bad".to_owned()],
            ..ReplayApplyReport::default()
        };

        let mut ledger = MemoryEvolutionLedger::default();
        ledger.record_replay_report(&replay);
        ledger.record_replay_apply_report(&apply);

        assert_eq!(ledger.replay_runs, 1);
        assert_eq!(ledger.replay_items, 2);
        assert_eq!(ledger.replay_memory_updates, 2);
        assert_eq!(ledger.replay_memory_missing, 1);
        assert_eq!(ledger.replay_invalid_memory_ids, 1);
        assert_eq!(ledger.context_rot_items, 1);
        assert_eq!(ledger.recursive_runtime_items, 1);
        assert_eq!(
            ledger.replay_evidence_checklist_detail(),
            "replay_runs=1 replay_items=2 replay_updates=2"
        );
        assert_eq!(
            ledger.reason_codes(),
            vec![
                "context_rot".to_owned(),
                "invalid_memory_id".to_owned(),
                "recursive_runtime".to_owned(),
                "replay_evidence".to_owned(),
                "replay_memory_update".to_owned(),
                "replay_missing_memory".to_owned()
            ]
        );
        assert_eq!(
            ledger.summary_line(),
            "memory_evolution replay_runs=1 replay_items=2 replay_updates=2 replay_missing=1 invalid_memory_ids=1 context_rot_items=1 live_feedback_items=0 retention_decays=0 retention_removals=0 compaction_merges=0 compaction_removals=0 external_applied=0 external_missing=0 drift_rollbacks=0 index_quality_blockers=0 index_quality_warnings=0 kvswap_boundary_blockers=0 kvswap_boundary_warnings=0 hygiene_pressure_score=5 hygiene_pressure_priority=repair hygiene_pressure_action_lanes=context_rot_review hygiene_pressure_action_lane_details=context_rot_review:repair:5:1 hygiene_work_next_action=context_rot_review hygiene_work_operator_review=false hygiene_work_isolation=false autophagy_context_pressure_score=5 autophagy_retrieval_noise_score=7 autophagy_stale_decay_candidates=0 autophagy_duplicate_merge_candidates=0 autophagy_gist_recomposition_candidates=0 autophagy_active_recall_prune_candidates=3 autophagy_quarantine_candidates=0 autophagy_live_delete_allowed=false autophagy_durable_mutation_allowed=false hygiene_pressure_reason_codes=context_rot_pressure hygiene_pressure_detail_codes=context_rot_items:1 reason_codes=context_rot|invalid_memory_id|recursive_runtime|replay_evidence|replay_memory_update|replay_missing_memory"
        );
    }

    #[test]
    fn evolution_ledger_records_index_quality_and_kvswap_boundary_risks() {
        let records = vec![
            ExperienceEnvelope::new("same-a", "Prompt A", "Lesson A")
                .with_clean_gist("A clean duplicate summary with enough signal."),
            ExperienceEnvelope::new("same-b", " prompt   a ", " lesson a ")
                .with_clean_gist("A clean duplicate summary with enough signal."),
            ExperienceEnvelope::new(
                "legacy",
                "A normal Rust runtime prompt",
                "accepted_pattern quality=0.91 overlap=0.44 max_severity=watch",
            ),
        ];
        let governance = DefaultExperienceGovernance::default();
        let report = governance.assess(&records);
        let plan = governance.rebuild_plan(&records);
        let quality_gate = report.quality_gate(&plan);
        let boundary = KvSwapBoundaryAudit {
            overlapping_hot_cold_ids: vec!["hot/cold".to_owned()],
            stale_metadata_ids: vec!["stale".to_owned()],
            ..KvSwapBoundaryAudit::default()
        };

        let mut ledger = MemoryEvolutionLedger::default();
        ledger.record_index_quality_gate(&quality_gate);
        ledger.record_kvswap_boundary_readiness(&boundary.readiness());

        assert_eq!(ledger.index_quality_blockers, 1);
        assert_eq!(ledger.index_quality_warnings, 2);
        assert_eq!(ledger.kvswap_boundary_blockers, 1);
        assert_eq!(ledger.kvswap_boundary_warnings, 1);
        assert_eq!(
            ledger.reason_codes(),
            vec![
                "index_quality_blocker".to_owned(),
                "index_quality_warning".to_owned(),
                "kvswap_boundary_blocker".to_owned(),
                "kvswap_boundary_warning".to_owned(),
            ]
        );
        let pressure = ledger.hygiene_pressure();
        assert_eq!(pressure.score, 230);
        assert_eq!(pressure.priority_code(), "quarantine");
        assert_eq!(
            pressure.reason_codes(),
            vec![
                "index_quality_blocker".to_owned(),
                "index_quality_warning".to_owned(),
                "kvswap_boundary_blocker".to_owned(),
                "kvswap_boundary_warning".to_owned(),
            ]
        );
        assert_eq!(
            pressure.action_lane_codes(),
            vec![
                "experience_index_rebuild".to_owned(),
                "kvswap_boundary_repair".to_owned()
            ]
        );
        assert_eq!(
            pressure.action_lane_detail_codes(),
            vec![
                "experience_index_rebuild:quarantine:120:3".to_owned(),
                "kvswap_boundary_repair:quarantine:110:2".to_owned()
            ]
        );
        let work_plan = pressure.work_plan();
        assert_eq!(work_plan.next_action_code, "experience_index_rebuild");
        assert!(work_plan.operator_review_required);
        assert!(work_plan.isolation_recommended);
        assert_eq!(
            work_plan.dispatch_codes(),
            vec![
                "dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3"
                    .to_owned(),
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:110:2"
                    .to_owned()
            ]
        );
        assert_eq!(
            work_plan.next_dispatch_code(),
            "dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3"
        );
        assert_eq!(
            work_plan.work_items(),
            vec![
                MemoryHygieneWorkItem {
                    lane_code: "experience_index_rebuild".to_owned(),
                    priority_code: "quarantine".to_owned(),
                    score: 120,
                    item_count: 3,
                    operator_review_required: true,
                    isolation_recommended: true,
                },
                MemoryHygieneWorkItem {
                    lane_code: "kvswap_boundary_repair".to_owned(),
                    priority_code: "quarantine".to_owned(),
                    score: 110,
                    item_count: 2,
                    operator_review_required: true,
                    isolation_recommended: true,
                }
            ]
        );
        assert_eq!(
            work_plan.work_items()[0].summary_line(),
            "memory_hygiene_work_item lane=experience_index_rebuild priority=quarantine score=120 items=3 operator_review=true isolation=true dispatch_code=dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3 detail_code=experience_index_rebuild:quarantine:120:3"
        );
        let work_queue = work_plan.work_queue();
        assert_eq!(work_queue.clean, false);
        assert_eq!(work_queue.total_score, 230);
        assert_eq!(work_queue.item_count, 2);
        assert_eq!(work_queue.operator_review_count, 2);
        assert_eq!(work_queue.isolation_count, 2);
        assert_eq!(
            work_queue.next_dispatch_code,
            "dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3"
        );
        assert_eq!(
            work_queue.lane_codes,
            vec![
                "experience_index_rebuild".to_owned(),
                "kvswap_boundary_repair".to_owned()
            ]
        );
        assert_eq!(work_queue.priority_codes, vec!["quarantine".to_owned()]);
        assert_eq!(
            work_queue.dispatch_codes,
            vec![
                "dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3"
                    .to_owned(),
                "dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:110:2"
                    .to_owned()
            ]
        );
        assert_eq!(
            work_queue.reason_codes(),
            vec![
                "isolation_recommended".to_owned(),
                "items_present".to_owned(),
                "operator_review_required".to_owned()
            ]
        );
        assert_eq!(
            work_queue.summary_line(),
            "memory_hygiene_work_queue clean=false total_score=230 items=2 operator_review=2 isolation=2 next_dispatch=dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3 lanes=experience_index_rebuild|kvswap_boundary_repair priorities=quarantine dispatch_codes=dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3|dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:110:2 detail_codes=experience_index_rebuild:quarantine:120:3|kvswap_boundary_repair:quarantine:110:2 reason_codes=isolation_recommended|items_present|operator_review_required"
        );
        assert_eq!(
            work_plan.detail_codes(),
            vec![
                "lane:experience_index_rebuild:quarantine:120:3".to_owned(),
                "lane:kvswap_boundary_repair:quarantine:110:2".to_owned(),
                "lane_count:2".to_owned(),
                "next_action:experience_index_rebuild".to_owned(),
                "total_score:230".to_owned()
            ]
        );
        assert!(work_plan.summary_line().contains("dispatch_next=dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3"));
        assert!(work_plan.summary_line().contains("dispatch_codes=dispatch:operator_review:isolated:experience_index_rebuild:quarantine:120:3|dispatch:operator_review:isolated:kvswap_boundary_repair:quarantine:110:2"));
        assert_eq!(
            pressure.summary_line(),
            "memory_hygiene_pressure clean=false priority=quarantine score=230 blockers=2 warnings=3 context_rot_items=0 index_quality_blockers=1 index_quality_warnings=2 kvswap_boundary_blockers=1 kvswap_boundary_warnings=1 action_lanes=experience_index_rebuild|kvswap_boundary_repair action_lane_details=experience_index_rebuild:quarantine:120:3|kvswap_boundary_repair:quarantine:110:2 work_next_action=experience_index_rebuild work_operator_review=true work_isolation=true reason_codes=index_quality_blocker|index_quality_warning|kvswap_boundary_blocker|kvswap_boundary_warning detail_codes=index_quality_blockers:1|index_quality_warnings:2|kvswap_boundary_blockers:1|kvswap_boundary_warnings:1"
        );
    }

    #[test]
    fn hygiene_work_plan_dispatch_codes_have_clean_fallback() {
        let plan = MemoryHygienePressure::default().work_plan();

        assert!(plan.clean);
        assert_eq!(plan.next_action_code, "none");
        assert_eq!(
            plan.dispatch_codes(),
            vec!["dispatch:clean:auto:shared:none".to_owned()]
        );
        assert!(plan.work_items().is_empty());
        assert_eq!(
            plan.next_dispatch_code(),
            "dispatch:clean:auto:shared:none".to_owned()
        );
        assert!(
            plan.summary_line()
                .contains("dispatch_next=dispatch:clean:auto:shared:none")
        );
        let queue = plan.work_queue();
        assert!(queue.clean);
        assert_eq!(queue.item_count, 0);
        assert_eq!(queue.operator_review_count, 0);
        assert_eq!(queue.isolation_count, 0);
        assert_eq!(
            queue.dispatch_codes,
            vec!["dispatch:clean:auto:shared:none".to_owned()]
        );
        assert_eq!(queue.reason_codes(), vec!["clean".to_owned()]);
        assert_eq!(
            queue.summary_line(),
            "memory_hygiene_work_queue clean=true total_score=0 items=0 operator_review=0 isolation=0 next_dispatch=dispatch:clean:auto:shared:none lanes=none priorities=none dispatch_codes=dispatch:clean:auto:shared:none detail_codes=none reason_codes=clean"
        );
    }

    #[test]
    fn hygiene_pressure_can_project_quality_gate_without_kvswap_boundary() {
        let records = vec![ExperienceEnvelope::new(
            "legacy",
            "A normal Rust runtime prompt",
            "accepted_pattern quality=0.91 overlap=0.44 max_severity=watch",
        )];
        let governance = DefaultExperienceGovernance::default();
        let report = governance.assess(&records);
        let plan = governance.rebuild_plan(&records);
        let pressure = MemoryHygienePressure::from_quality_gate_and_boundary(
            &report.quality_gate(&plan),
            None,
        );

        assert_eq!(pressure.score, 20);
        assert_eq!(pressure.priority_code(), "repair");
        assert_eq!(pressure.blocker_count(), 0);
        assert_eq!(pressure.warning_count(), 2);
        assert_eq!(
            pressure.detail_codes(),
            vec!["index_quality_warnings:2".to_owned()]
        );
    }

    #[test]
    fn evolution_ledger_records_retention_compaction_and_external_feedback() {
        let mut ledger = MemoryEvolutionLedger::default();
        ledger.record_retention_plan(&MemoryRetentionPlan {
            before: 2,
            after_estimate: 1,
            decays: vec![MemoryDecay {
                id: "stale".to_owned(),
                idle_ticks: 9,
                strength_before: 0.8,
                strength_after: 0.6,
                reason: "stale_decay".to_owned(),
            }],
            removals: vec![MemoryRetentionRemoval {
                id: "bad".to_owned(),
                reason: "weak_stale".to_owned(),
            }],
        });
        ledger.record_compaction_plan(&MemoryCompactionPlan {
            before: 2,
            after_estimate: 1,
            merges: vec![MemoryCompactionMerge {
                primary_id: "a".to_owned(),
                removed_id: "b".to_owned(),
                similarity: 0.99,
                reason: "same_namespace_high_similarity".to_owned(),
            }],
            removed_ids: vec!["b".to_owned()],
            skipped_reason: None,
        });
        ledger.record_external_feedback(2, 1, 1, -0.5);

        assert_eq!(ledger.retention_decays, 1);
        assert_eq!(ledger.retention_removals, 1);
        assert_eq!(ledger.compaction_merges, 1);
        assert_eq!(ledger.compaction_removals, 1);
        assert_eq!(ledger.external_feedback_batches, 1);
        assert_eq!(ledger.external_feedback_applied, 2);
        assert_eq!(ledger.external_feedback_missing, 1);
        assert!((ledger.external_feedback_strength_delta - 0.5).abs() < f32::EPSILON);
        let autophagy = ledger.autophagy_plan();
        assert_eq!(autophagy.stale_decay_candidates, 1);
        assert_eq!(autophagy.duplicate_merge_candidates, 1);
        assert_eq!(autophagy.gist_recomposition_candidates, 2);
        assert_eq!(autophagy.quarantine_candidates, 1);
        assert!(autophagy.preview_only());
        assert_eq!(
            ledger.reason_codes(),
            vec![
                "compaction_merge".to_owned(),
                "compaction_removal".to_owned(),
                "external_feedback_applied".to_owned(),
                "external_feedback_missing".to_owned(),
                "external_feedback_removed".to_owned(),
                "retention_decay".to_owned(),
                "retention_removal".to_owned()
            ]
        );
    }

    #[test]
    fn autophagy_plan_combines_pressure_without_live_delete() {
        let retention = MemoryRetentionPlan {
            before: 3,
            after_estimate: 1,
            decays: vec![MemoryDecay {
                id: "stale-low-hit".to_owned(),
                idle_ticks: 12,
                strength_before: 0.22,
                strength_after: 0.11,
                reason: "stale_decay".to_owned(),
            }],
            removals: vec![MemoryRetentionRemoval {
                id: "failed-memory".to_owned(),
                reason: "weak_stale".to_owned(),
            }],
        };
        let compaction = MemoryCompactionPlan {
            before: 3,
            after_estimate: 2,
            merges: vec![MemoryCompactionMerge {
                primary_id: "dup-primary".to_owned(),
                removed_id: "dup-removed".to_owned(),
                similarity: 0.98,
                reason: "same_namespace_high_similarity".to_owned(),
            }],
            removed_ids: vec!["dup-removed".to_owned()],
            skipped_reason: None,
        };
        let hygiene = MemoryHygienePressure {
            score: 115,
            index_quality_blockers: 1,
            index_quality_warnings: 1,
            kvswap_boundary_blockers: 0,
            kvswap_boundary_warnings: 0,
            context_rot_items: 1,
        };
        let recall = MemoryAutophagyRecallSignals {
            rejected_context_count: 2,
            duplicate_reject_count: 1,
            missing_kv_count: 1,
            unsafe_sidecar_reject_count: 1,
        };

        let plan = MemoryAutophagyPlan::from_signals(&retention, &compaction, &hygiene, &recall);

        assert_eq!(plan.context_pressure_score, 115);
        assert_eq!(plan.retrieval_noise_score, 10);
        assert_eq!(plan.stale_decay_candidates, 1);
        assert_eq!(plan.duplicate_merge_candidates, 1);
        assert_eq!(plan.gist_recomposition_candidates, 2);
        assert_eq!(plan.active_recall_prune_candidates, 5);
        assert_eq!(plan.quarantine_candidates, 3);
        assert!(plan.preview_only());
        assert!(plan.reason_codes.contains(&"recycle_preview".to_owned()));
        assert!(
            plan.reason_codes
                .contains(&"gist_recomposition_preview".to_owned())
        );
        assert!(
            plan.reason_codes
                .contains(&"active_recall_prune_preview".to_owned())
        );
        assert!(plan.reason_codes.contains(&"quarantine_preview".to_owned()));
        assert!(plan.detail_codes.iter().any(|code| {
            code.starts_with("decay:stale_decay:")
                || code.starts_with("merge:same_namespace_high_similarity:")
        }));
        let summary = plan.summary_line();
        assert!(summary.contains("memory_autophagy_preview"));
        assert!(summary.contains("live_delete_allowed=false"));
        assert!(summary.contains("durable_mutation_allowed=false"));
        assert!(summary.contains("stale_decay_candidates=1"));
        assert!(summary.contains("duplicate_merge_candidates=1"));
        assert!(!summary.contains("stale-low-hit"));
        assert!(!summary.contains("dup-primary"));
    }

    #[test]
    fn autophagy_plan_reports_clean_pressure_as_preview_only_noop() {
        let retention = MemoryRetentionPlan::default();
        let compaction = MemoryCompactionPlan::default();
        let hygiene = MemoryHygienePressure::default();
        let recall = MemoryAutophagyRecallSignals::default();

        let plan = MemoryAutophagyPlan::from_signals(&retention, &compaction, &hygiene, &recall);

        assert_eq!(plan.context_pressure_score, 0);
        assert_eq!(plan.retrieval_noise_score, 0);
        assert_eq!(plan.stale_decay_candidates, 0);
        assert_eq!(plan.duplicate_merge_candidates, 0);
        assert_eq!(plan.gist_recomposition_candidates, 0);
        assert_eq!(plan.active_recall_prune_candidates, 0);
        assert_eq!(plan.quarantine_candidates, 0);
        assert_eq!(plan.reason_codes, vec!["clean".to_owned()]);
        assert!(plan.detail_codes.is_empty());
        assert!(plan.preview_only());
        assert_eq!(
            plan.summary_line(),
            "memory_autophagy_preview context_pressure_score=0 retrieval_noise_score=0 stale_decay_candidates=0 duplicate_merge_candidates=0 gist_recomposition_candidates=0 active_recall_prune_candidates=0 quarantine_candidates=0 live_delete_allowed=false durable_mutation_allowed=false reason_codes=clean"
        );
    }

    #[test]
    fn evolution_gate_blocks_missing_replay_and_high_missing_ratio() {
        let ledger = MemoryEvolutionLedger {
            replay_memory_updates: 1,
            replay_memory_missing: 4,
            ..MemoryEvolutionLedger::default()
        };

        let assessment = DefaultMemoryEvolutionGate::default().assess(&ledger);

        assert!(!assessment.allow_isolated_write);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|blocker| blocker == "missing_replay_evidence")
        );
        assert!(
            assessment
                .blockers
                .iter()
                .any(|blocker| blocker.starts_with("memory_update_missing_ratio="))
        );
        assert_eq!(
            assessment.blocker_codes(),
            vec![
                "memory_update_missing_ratio".to_owned(),
                "missing_replay_evidence".to_owned()
            ]
        );
        assert_eq!(
            assessment.blocker_detail_codes(),
            vec![
                "memory_update_missing_ratio:0.800".to_owned(),
                "missing_replay_evidence".to_owned()
            ]
        );
    }

    #[test]
    fn evolution_gate_warns_on_context_rot_and_drift_rollbacks() {
        let ledger = MemoryEvolutionLedger {
            replay_runs: 1,
            replay_items: 2,
            replay_memory_updates: 4,
            context_rot_items: 9,
            drift_rollbacks: 1,
            index_quality_blockers: 2,
            kvswap_boundary_warnings: 1,
            ..MemoryEvolutionLedger::default()
        };

        let assessment = DefaultMemoryEvolutionGate::default().assess(&ledger);

        assert!(assessment.allow_isolated_write);
        assert!(assessment.rollback_recommended);
        assert!(
            assessment
                .warnings
                .iter()
                .any(|warning| warning == "context_rot_items=9")
        );
        assert!(
            assessment
                .warnings
                .iter()
                .any(|warning| warning == "drift_rollbacks=1")
        );
        assert!(
            assessment
                .warnings
                .iter()
                .any(|warning| warning == "index_quality_blockers=2")
        );
        assert!(
            assessment
                .warnings
                .iter()
                .any(|warning| warning == "kvswap_boundary_warnings=1")
        );
        assert_eq!(
            assessment.warning_codes(),
            vec![
                "context_rot_items".to_owned(),
                "drift_rollbacks".to_owned(),
                "index_quality_blockers".to_owned(),
                "kvswap_boundary_warnings".to_owned(),
                "memory_hygiene_pressure".to_owned()
            ]
        );
        assert_eq!(
            assessment.warning_detail_codes(),
            vec![
                "context_rot_items:9".to_owned(),
                "drift_rollbacks:1".to_owned(),
                "index_quality_blockers:2".to_owned(),
                "kvswap_boundary_warnings:1".to_owned(),
                "memory_hygiene_pressure:255".to_owned()
            ]
        );
    }

    #[test]
    fn evolution_gate_is_read_only_adapter() {
        let gate = DefaultMemoryEvolutionGate::default();
        let descriptor = gate.descriptor();

        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::MemoryEvolution)
        );
        assert!(gate.health().unwrap().ready);
    }
}
