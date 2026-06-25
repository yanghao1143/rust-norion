use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskProfile {
    General,
    Coding,
    Writing,
    LongDocument,
}

impl FromStr for TaskProfile {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "general" => Ok(Self::General),
            "coding" | "code" | "rust" => Ok(Self::Coding),
            "writing" | "write" => Ok(Self::Writing),
            "long" | "longdoc" | "long-document" | "document" => Ok(Self::LongDocument),
            other => Err(format!("unknown task profile: {other}")),
        }
    }
}

impl TaskProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Coding => "coding",
            Self::Writing => "writing",
            Self::LongDocument => "long_document",
        }
    }
}

impl Default for TaskProfile {
    fn default() -> Self {
        Self::General
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HierarchyWeights {
    pub global: f32,
    pub local: f32,
    pub fusion: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HierarchyWeightFocus {
    Global,
    Local,
    Fusion,
    Balanced,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HierarchyWeightsSummary {
    pub global: f32,
    pub local: f32,
    pub fusion: f32,
    pub total: f32,
    pub dominant: HierarchyWeightFocus,
    pub is_normalized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileHierarchyWeights {
    pub general: HierarchyWeights,
    pub coding: HierarchyWeights,
    pub writing: HierarchyWeights,
    pub long_document: HierarchyWeights,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfileHierarchyWeightsSummary {
    pub general: HierarchyWeightsSummary,
    pub coding: HierarchyWeightsSummary,
    pub writing: HierarchyWeightsSummary,
    pub long_document: HierarchyWeightsSummary,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProfileHierarchyObservations {
    pub general: u64,
    pub coding: u64,
    pub writing: u64,
    pub long_document: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HierarchyAdjustmentFeedback {
    pub profile: TaskProfile,
    pub quality: f32,
    pub perplexity: f32,
    pub contradiction_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HierarchyAdjustmentFeedbackSummary {
    pub profile: TaskProfile,
    pub quality: f32,
    pub perplexity: f32,
    pub contradiction_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskAwareHierarchyAdjustmentReport {
    pub profile: TaskProfile,
    pub feedback: HierarchyAdjustmentFeedbackSummary,
    pub previous: HierarchyWeights,
    pub target: HierarchyWeights,
    pub adjusted: HierarchyWeights,
    pub observations_before: u64,
    pub observations_after: u64,
    pub learning_rate: f32,
    pub can_commit: bool,
    pub requires_repair_first: bool,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HierarchyMutationKind {
    IncreaseGlobal,
    IncreaseLocal,
    IncreaseFusion,
    Stabilize,
    RepairRequired,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskAwareHierarchyMutationRecord {
    pub sequence: u64,
    pub profile: TaskProfile,
    pub kind: HierarchyMutationKind,
    pub previous: HierarchyWeights,
    pub target: HierarchyWeights,
    pub adjusted: HierarchyWeights,
    pub observations_before: u64,
    pub observations_after: u64,
    pub can_commit: bool,
    pub requires_repair_first: bool,
    pub committed: bool,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskAwareHierarchyMutationPlan {
    pub records: Vec<TaskAwareHierarchyMutationRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskAwareHierarchyMutationHistory {
    pub records: Vec<TaskAwareHierarchyMutationRecord>,
    next_sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaskAwareHierarchyAdjustmentPolicy {
    weights: ProfileHierarchyWeights,
    baseline: ProfileHierarchyWeights,
    observations: ProfileHierarchyObservations,
    learning_rate: f32,
}

impl HierarchyWeights {
    pub fn new(global: f32, local: f32, fusion: f32) -> Self {
        let mut weights = Self {
            global,
            local,
            fusion,
        };
        weights.normalize();
        weights
    }

    pub fn for_profile(profile: TaskProfile) -> Self {
        match profile {
            TaskProfile::General => Self::new(0.36, 0.42, 0.22),
            TaskProfile::Coding => Self::new(0.24, 0.58, 0.18),
            TaskProfile::Writing => Self::new(0.56, 0.30, 0.14),
            TaskProfile::LongDocument => Self::new(0.30, 0.22, 0.48),
        }
    }

    pub fn normalize(&mut self) {
        self.global = finite_nonnegative(self.global);
        self.local = finite_nonnegative(self.local);
        self.fusion = finite_nonnegative(self.fusion);

        let total = self.global + self.local + self.fusion;
        if total <= f32::EPSILON {
            self.global = 0.34;
            self.local = 0.33;
            self.fusion = 0.33;
            return;
        }

        self.global /= total;
        self.local /= total;
        self.fusion /= total;
    }

    pub fn blend(self, target: Self, rate: f32) -> Self {
        let rate = rate.clamp(0.0, 1.0);
        Self::new(
            self.global * (1.0 - rate) + target.global * rate,
            self.local * (1.0 - rate) + target.local * rate,
            self.fusion * (1.0 - rate) + target.fusion * rate,
        )
    }

    pub fn total(self) -> f32 {
        self.global + self.local + self.fusion
    }

    pub fn dominant_focus(self) -> HierarchyWeightFocus {
        let max = self.global.max(self.local).max(self.fusion);
        let winners = [
            (HierarchyWeightFocus::Global, self.global),
            (HierarchyWeightFocus::Local, self.local),
            (HierarchyWeightFocus::Fusion, self.fusion),
        ]
        .into_iter()
        .filter(|(_, value)| (*value - max).abs() <= 0.0001)
        .collect::<Vec<_>>();

        if winners.len() == 1 {
            winners[0].0
        } else {
            HierarchyWeightFocus::Balanced
        }
    }

    pub fn summary(self) -> HierarchyWeightsSummary {
        let total = self.total();
        HierarchyWeightsSummary {
            global: self.global,
            local: self.local,
            fusion: self.fusion,
            total,
            dominant: self.dominant_focus(),
            is_normalized: (total - 1.0).abs() <= 0.0001,
        }
    }
}

impl Default for HierarchyWeights {
    fn default() -> Self {
        Self::for_profile(TaskProfile::General)
    }
}

impl ProfileHierarchyWeights {
    pub fn target_defaults() -> Self {
        Self {
            general: HierarchyWeights::for_profile(TaskProfile::General),
            coding: HierarchyWeights::for_profile(TaskProfile::Coding),
            writing: HierarchyWeights::for_profile(TaskProfile::Writing),
            long_document: HierarchyWeights::for_profile(TaskProfile::LongDocument),
        }
    }

    pub fn from_single(weights: HierarchyWeights) -> Self {
        Self {
            general: weights,
            coding: weights,
            writing: weights,
            long_document: weights,
        }
    }

    pub fn get(self, profile: TaskProfile) -> HierarchyWeights {
        match profile {
            TaskProfile::General => self.general,
            TaskProfile::Coding => self.coding,
            TaskProfile::Writing => self.writing,
            TaskProfile::LongDocument => self.long_document,
        }
    }

    pub fn set(&mut self, profile: TaskProfile, weights: HierarchyWeights) {
        match profile {
            TaskProfile::General => self.general = weights,
            TaskProfile::Coding => self.coding = weights,
            TaskProfile::Writing => self.writing = weights,
            TaskProfile::LongDocument => self.long_document = weights,
        }
    }

    pub fn normalize(&mut self) {
        self.general.normalize();
        self.coding.normalize();
        self.writing.normalize();
        self.long_document.normalize();
    }

    pub fn summary(self) -> ProfileHierarchyWeightsSummary {
        ProfileHierarchyWeightsSummary {
            general: self.general.summary(),
            coding: self.coding.summary(),
            writing: self.writing.summary(),
            long_document: self.long_document.summary(),
        }
    }
}

impl Default for ProfileHierarchyWeights {
    fn default() -> Self {
        Self::target_defaults()
    }
}

impl ProfileHierarchyWeightsSummary {
    pub fn all_normalized(self) -> bool {
        self.general.is_normalized
            && self.coding.is_normalized
            && self.writing.is_normalized
            && self.long_document.is_normalized
    }

    pub fn expected_profile_focuses(self) -> bool {
        self.coding.dominant == HierarchyWeightFocus::Local
            && self.writing.dominant == HierarchyWeightFocus::Global
            && self.long_document.dominant == HierarchyWeightFocus::Fusion
    }

    pub fn summary_for(self, profile: TaskProfile) -> HierarchyWeightsSummary {
        match profile {
            TaskProfile::General => self.general,
            TaskProfile::Coding => self.coding,
            TaskProfile::Writing => self.writing,
            TaskProfile::LongDocument => self.long_document,
        }
    }

    pub fn normalized_profile_signal_component_count(self) -> usize {
        usize::from(self.general.is_normalized)
            + usize::from(self.coding.is_normalized)
            + usize::from(self.writing.is_normalized)
            + usize::from(self.long_document.is_normalized)
    }

    pub fn expected_focus_signal_component_count(self) -> usize {
        usize::from(self.coding.dominant == HierarchyWeightFocus::Local)
            + usize::from(self.writing.dominant == HierarchyWeightFocus::Global)
            + usize::from(self.long_document.dominant == HierarchyWeightFocus::Fusion)
    }

    pub fn hierarchy_profile_signal_component_count(self) -> usize {
        self.normalized_profile_signal_component_count()
            .saturating_add(self.expected_focus_signal_component_count())
    }

    pub fn has_hierarchy_profile_signal_components(self) -> bool {
        self.hierarchy_profile_signal_component_count() > 0
    }

    pub fn per_profile_problem_component_count(self) -> usize {
        self.general
            .hierarchy_problem_component_count()
            .saturating_add(self.coding.hierarchy_problem_component_count())
            .saturating_add(self.writing.hierarchy_problem_component_count())
            .saturating_add(self.long_document.hierarchy_problem_component_count())
    }

    pub fn normalized_profile_problem_component_count(self) -> usize {
        usize::from(!self.general.is_normalized)
            + usize::from(!self.coding.is_normalized)
            + usize::from(!self.writing.is_normalized)
            + usize::from(!self.long_document.is_normalized)
    }

    pub fn expected_focus_problem_component_count(self) -> usize {
        usize::from(self.coding.dominant != HierarchyWeightFocus::Local)
            + usize::from(self.writing.dominant != HierarchyWeightFocus::Global)
            + usize::from(self.long_document.dominant != HierarchyWeightFocus::Fusion)
    }

    pub fn hierarchy_profile_problem_component_count(self) -> usize {
        self.per_profile_problem_component_count()
            .saturating_add(self.normalized_profile_problem_component_count())
            .saturating_add(self.expected_focus_problem_component_count())
    }

    pub fn has_hierarchy_profile_problem_components(self) -> bool {
        self.hierarchy_profile_problem_component_count() > 0
    }

    pub fn hierarchy_profile_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .normalized_profile_signal_component_count()
            .saturating_add(self.expected_focus_signal_component_count());
        let expected_problem_count = self
            .per_profile_problem_component_count()
            .saturating_add(self.normalized_profile_problem_component_count())
            .saturating_add(self.expected_focus_problem_component_count());

        self.hierarchy_profile_signal_component_count() == expected_signal_count
            && self.hierarchy_profile_problem_component_count() == expected_problem_count
            && self.all_normalized() == (self.normalized_profile_problem_component_count() == 0)
            && self.expected_profile_focuses()
                == (self.expected_focus_problem_component_count() == 0)
    }

    pub fn hierarchy_profile_shape_is_clean(self) -> bool {
        !self.has_hierarchy_profile_problem_components()
            && self.hierarchy_profile_accounting_is_consistent()
    }

    pub fn can_use_profile_hierarchy_weights(self) -> bool {
        self.hierarchy_profile_shape_is_clean()
    }
}

impl ProfileHierarchyObservations {
    pub fn from_single(observations: u64) -> Self {
        Self {
            general: observations,
            coding: 0,
            writing: 0,
            long_document: 0,
        }
    }

    pub fn get(self, profile: TaskProfile) -> u64 {
        match profile {
            TaskProfile::General => self.general,
            TaskProfile::Coding => self.coding,
            TaskProfile::Writing => self.writing,
            TaskProfile::LongDocument => self.long_document,
        }
    }

    pub fn bump(&mut self, profile: TaskProfile) {
        match profile {
            TaskProfile::General => self.general = self.general.saturating_add(1),
            TaskProfile::Coding => self.coding = self.coding.saturating_add(1),
            TaskProfile::Writing => self.writing = self.writing.saturating_add(1),
            TaskProfile::LongDocument => {
                self.long_document = self.long_document.saturating_add(1);
            }
        }
    }

    pub fn total(self) -> u64 {
        self.general
            .saturating_add(self.coding)
            .saturating_add(self.writing)
            .saturating_add(self.long_document)
    }

    pub fn active_profile_count(self) -> usize {
        [self.general, self.coding, self.writing, self.long_document]
            .into_iter()
            .filter(|observations| *observations > 0)
            .count()
    }

    pub fn profile_observation_signal_component_count(self) -> usize {
        usize::from(self.total() > 0)
            + usize::from(self.general > 0)
            + usize::from(self.coding > 0)
            + usize::from(self.writing > 0)
            + usize::from(self.long_document > 0)
            + usize::from(self.active_profile_count() > 1)
    }

    pub fn has_profile_observation_signal_components(self) -> bool {
        self.profile_observation_signal_component_count() > 0
    }

    pub fn profile_observation_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.total() > 0)
            .saturating_add(usize::from(self.general > 0))
            .saturating_add(usize::from(self.coding > 0))
            .saturating_add(usize::from(self.writing > 0))
            .saturating_add(usize::from(self.long_document > 0))
            .saturating_add(usize::from(self.active_profile_count() > 1));

        self.profile_observation_signal_component_count() == expected_signal_count
    }

    pub fn profile_observation_shape_is_clean(self) -> bool {
        self.profile_observation_accounting_is_consistent()
    }

    pub fn can_use_profile_hierarchy_observations(self) -> bool {
        self.total() > 0 && self.profile_observation_shape_is_clean()
    }
}

impl HierarchyAdjustmentFeedback {
    pub fn new(
        profile: TaskProfile,
        quality: f32,
        perplexity: f32,
        contradiction_count: usize,
    ) -> Self {
        Self {
            profile,
            quality,
            perplexity,
            contradiction_count,
        }
    }

    pub fn feedback_summary(self) -> HierarchyAdjustmentFeedbackSummary {
        HierarchyAdjustmentFeedbackSummary {
            profile: self.profile,
            quality: self.quality,
            perplexity: self.perplexity,
            contradiction_count: self.contradiction_count,
        }
    }
}

impl HierarchyAdjustmentFeedbackSummary {
    pub fn is_low_quality(self) -> bool {
        self.quality < 0.58
    }

    pub fn is_high_quality(self) -> bool {
        self.quality > 0.82 && self.perplexity <= 9.0
    }

    pub fn has_contradictions(self) -> bool {
        self.contradiction_count > 0
    }

    pub fn quality_shape_is_valid(self) -> bool {
        finite_unit(self.quality)
    }

    pub fn perplexity_shape_is_valid(self) -> bool {
        self.perplexity.is_finite() && self.perplexity >= 0.0
    }

    pub fn feedback_signal_component_count(self) -> usize {
        usize::from(self.is_low_quality())
            + usize::from(self.is_high_quality())
            + usize::from(self.has_contradictions())
            + usize::from(self.perplexity > 0.0 && self.perplexity_shape_is_valid())
    }

    pub fn feedback_problem_component_count(self) -> usize {
        usize::from(!self.quality_shape_is_valid()) + usize::from(!self.perplexity_shape_is_valid())
    }

    pub fn feedback_accounting_is_consistent(self) -> bool {
        let expected_signal_count = usize::from(self.is_low_quality())
            .saturating_add(usize::from(self.is_high_quality()))
            .saturating_add(usize::from(self.has_contradictions()))
            .saturating_add(usize::from(
                self.perplexity > 0.0 && self.perplexity_shape_is_valid(),
            ));
        let expected_problem_count = usize::from(!self.quality_shape_is_valid())
            .saturating_add(usize::from(!self.perplexity_shape_is_valid()));

        self.feedback_signal_component_count() == expected_signal_count
            && self.feedback_problem_component_count() == expected_problem_count
    }

    pub fn feedback_shape_is_clean(self) -> bool {
        self.feedback_problem_component_count() == 0 && self.feedback_accounting_is_consistent()
    }

    pub fn can_use_feedback(self) -> bool {
        self.feedback_shape_is_clean()
    }
}

impl TaskAwareHierarchyAdjustmentReport {
    pub fn has_weight_delta(&self) -> bool {
        !hierarchy_weights_close(self.previous, self.adjusted)
    }

    pub fn adjusted_weights_are_clean(&self) -> bool {
        self.previous.summary().can_use_hierarchy_weights()
            && self.target.summary().can_use_hierarchy_weights()
            && self.adjusted.summary().can_use_hierarchy_weights()
    }

    pub fn observation_advanced(&self) -> bool {
        self.observations_after == self.observations_before.saturating_add(1)
    }

    pub fn learning_rate_is_valid(&self) -> bool {
        self.learning_rate.is_finite() && self.learning_rate >= 0.0
    }

    pub fn adjustment_signal_component_count(&self) -> usize {
        usize::from(self.feedback.feedback_signal_component_count() > 0)
            + usize::from(self.has_weight_delta())
            + usize::from(self.adjusted_weights_are_clean())
            + usize::from(self.observation_advanced())
            + usize::from(self.can_commit)
    }

    pub fn adjustment_problem_component_count(&self) -> usize {
        usize::from(!self.feedback.can_use_feedback())
            + usize::from(!self.adjusted_weights_are_clean())
            + usize::from(!self.learning_rate_is_valid())
            + usize::from(!self.observation_advanced())
            + usize::from(self.requires_repair_first && self.can_commit)
    }

    pub fn adjustment_accounting_is_consistent(&self) -> bool {
        let expected_signal_count =
            usize::from(self.feedback.feedback_signal_component_count() > 0)
                .saturating_add(usize::from(self.has_weight_delta()))
                .saturating_add(usize::from(self.adjusted_weights_are_clean()))
                .saturating_add(usize::from(self.observation_advanced()))
                .saturating_add(usize::from(self.can_commit));
        let expected_problem_count = usize::from(!self.feedback.can_use_feedback())
            .saturating_add(usize::from(!self.adjusted_weights_are_clean()))
            .saturating_add(usize::from(!self.learning_rate_is_valid()))
            .saturating_add(usize::from(!self.observation_advanced()))
            .saturating_add(usize::from(self.requires_repair_first && self.can_commit));

        self.adjustment_signal_component_count() == expected_signal_count
            && self.adjustment_problem_component_count() == expected_problem_count
    }

    pub fn adjustment_shape_is_clean(&self) -> bool {
        self.adjustment_problem_component_count() == 0 && self.adjustment_accounting_is_consistent()
    }

    pub fn mutation_kind(&self) -> HierarchyMutationKind {
        if self.requires_repair_first {
            return HierarchyMutationKind::RepairRequired;
        }
        if hierarchy_weights_close(self.previous, self.adjusted) {
            return HierarchyMutationKind::Stabilize;
        }

        let global_delta = self.adjusted.global - self.previous.global;
        let local_delta = self.adjusted.local - self.previous.local;
        let fusion_delta = self.adjusted.fusion - self.previous.fusion;
        let max_delta = global_delta.max(local_delta).max(fusion_delta);

        if max_delta <= 0.0001 {
            HierarchyMutationKind::Stabilize
        } else if float_close(global_delta, max_delta) {
            HierarchyMutationKind::IncreaseGlobal
        } else if float_close(local_delta, max_delta) {
            HierarchyMutationKind::IncreaseLocal
        } else {
            HierarchyMutationKind::IncreaseFusion
        }
    }
}

impl HierarchyMutationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IncreaseGlobal => "increase_global",
            Self::IncreaseLocal => "increase_local",
            Self::IncreaseFusion => "increase_fusion",
            Self::Stabilize => "stabilize",
            Self::RepairRequired => "repair_required",
        }
    }
}

impl TaskAwareHierarchyMutationRecord {
    pub fn from_report(
        sequence: u64,
        report: &TaskAwareHierarchyAdjustmentReport,
        committed: bool,
    ) -> Self {
        Self {
            sequence,
            profile: report.profile,
            kind: report.mutation_kind(),
            previous: report.previous,
            target: report.target,
            adjusted: report.adjusted,
            observations_before: report.observations_before,
            observations_after: report.observations_after,
            can_commit: report.can_commit,
            requires_repair_first: report.requires_repair_first,
            committed: committed && report.can_commit,
            reason_codes: report.reason_codes.clone(),
        }
    }

    pub fn detail_code(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.sequence,
            self.profile.as_str(),
            self.kind.as_str(),
            if self.committed {
                "committed"
            } else {
                "preview"
            }
        )
    }

    pub fn summary_line(&self) -> String {
        format!(
            "task_aware_hierarchy_mutation sequence={} profile={} kind={} can_commit={} repair_first={} committed={} observations_before={} observations_after={} reason_codes={} detail_code={}",
            self.sequence,
            self.profile.as_str(),
            self.kind.as_str(),
            self.can_commit,
            self.requires_repair_first,
            self.committed,
            self.observations_before,
            self.observations_after,
            join_codes(self.reason_codes.clone()),
            self.detail_code(),
        )
    }
}

impl TaskAwareHierarchyMutationPlan {
    pub fn from_reports(reports: &[TaskAwareHierarchyAdjustmentReport]) -> Self {
        let records = reports
            .iter()
            .enumerate()
            .map(|(index, report)| {
                TaskAwareHierarchyMutationRecord::from_report(
                    index.saturating_add(1) as u64,
                    report,
                    false,
                )
            })
            .collect();
        Self { records }
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn persistent_writes_allowed(&self) -> bool {
        false
    }

    pub fn commit_ready_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.can_commit && !record.requires_repair_first)
            .count()
    }

    pub fn repair_required_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.requires_repair_first)
            .count()
    }

    pub fn profile_codes(&self) -> Vec<String> {
        self.records
            .iter()
            .map(|record| record.profile.as_str().to_owned())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn mutation_kind_codes(&self) -> Vec<String> {
        self.records
            .iter()
            .map(|record| record.kind.as_str().to_owned())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        self.records
            .iter()
            .flat_map(|record| record.reason_codes.iter().cloned())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.records
            .iter()
            .map(TaskAwareHierarchyMutationRecord::detail_code)
            .collect()
    }

    pub fn next_commit_candidate(&self) -> Option<&TaskAwareHierarchyMutationRecord> {
        self.records
            .iter()
            .find(|record| record.can_commit && !record.requires_repair_first)
    }

    pub fn requires_operator_review(&self) -> bool {
        !self.is_empty()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "task_aware_hierarchy_mutation_plan empty={} records={} commit_ready={} repair_required={} operator_review_required={} persistent_writes_allowed={} profiles={} mutation_kinds={} reason_codes={} detail_codes={} next_commit={}",
            self.is_empty(),
            self.records.len(),
            self.commit_ready_count(),
            self.repair_required_count(),
            self.requires_operator_review(),
            self.persistent_writes_allowed(),
            join_codes(self.profile_codes()),
            join_codes(self.mutation_kind_codes()),
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
            self.next_commit_candidate()
                .map(TaskAwareHierarchyMutationRecord::detail_code)
                .unwrap_or_else(|| "none".to_owned()),
        )
    }
}

impl TaskAwareHierarchyMutationHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_preview(
        &mut self,
        report: &TaskAwareHierarchyAdjustmentReport,
    ) -> TaskAwareHierarchyMutationRecord {
        self.record_report(report, false)
    }

    pub fn record_committed(
        &mut self,
        report: &TaskAwareHierarchyAdjustmentReport,
    ) -> TaskAwareHierarchyMutationRecord {
        self.record_report(report, true)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn committed_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.committed)
            .count()
    }

    pub fn repair_required_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.requires_repair_first)
            .count()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.records
            .iter()
            .map(TaskAwareHierarchyMutationRecord::detail_code)
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "task_aware_hierarchy_mutation_history records={} committed={} repair_required={} persistent_writes_allowed=false detail_codes={}",
            self.records.len(),
            self.committed_count(),
            self.repair_required_count(),
            join_codes(self.detail_codes()),
        )
    }

    fn record_report(
        &mut self,
        report: &TaskAwareHierarchyAdjustmentReport,
        committed: bool,
    ) -> TaskAwareHierarchyMutationRecord {
        let record =
            TaskAwareHierarchyMutationRecord::from_report(self.next_sequence, report, committed);
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.records.push(record.clone());
        record
    }
}

impl Default for TaskAwareHierarchyMutationHistory {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            next_sequence: 1,
        }
    }
}

impl TaskAwareHierarchyAdjustmentPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_learning_rate(mut self, learning_rate: f32) -> Self {
        self.learning_rate = learning_rate;
        self
    }

    pub fn hierarchy_for(&self, profile: TaskProfile) -> HierarchyWeights {
        self.weights.get(profile)
    }

    pub fn observations(&self) -> ProfileHierarchyObservations {
        self.observations
    }

    pub fn weights(&self) -> ProfileHierarchyWeights {
        self.weights
    }

    pub fn preview_adjustment(
        &self,
        feedback: HierarchyAdjustmentFeedback,
    ) -> TaskAwareHierarchyAdjustmentReport {
        self.plan_adjustment(feedback)
    }

    pub fn preview_mutation_plan(
        &self,
        feedback: &[HierarchyAdjustmentFeedback],
    ) -> TaskAwareHierarchyMutationPlan {
        let reports = feedback
            .iter()
            .copied()
            .map(|feedback| self.plan_adjustment(feedback))
            .collect::<Vec<_>>();
        TaskAwareHierarchyMutationPlan::from_reports(&reports)
    }

    pub fn observe(
        &mut self,
        feedback: HierarchyAdjustmentFeedback,
    ) -> TaskAwareHierarchyAdjustmentReport {
        let report = self.plan_adjustment(feedback);
        if report.can_commit {
            self.weights.set(report.profile, report.adjusted);
            self.observations.bump(report.profile);
        }
        report
    }

    fn plan_adjustment(
        &self,
        feedback: HierarchyAdjustmentFeedback,
    ) -> TaskAwareHierarchyAdjustmentReport {
        let feedback = feedback.feedback_summary();
        let previous = self.weights.get(feedback.profile);
        let observations_before = self.observations.get(feedback.profile);
        let mut reason_codes = vec![format!("profile:{}", feedback.profile.as_str())];
        let mut target = self.baseline.get(feedback.profile);

        if feedback.is_low_quality() {
            reason_codes.push("quality_low".to_owned());
            target = target.blend(low_quality_target(feedback.profile), 0.75);
        } else if feedback.is_high_quality() {
            reason_codes.push("quality_high_stabilize".to_owned());
        } else {
            reason_codes.push("quality_neutral".to_owned());
            target = previous;
        }

        if feedback.has_contradictions() {
            reason_codes.push("contradiction_pressure".to_owned());
            target = target.blend(fusion_pressure_target(target), 0.55);
        }

        if !feedback.can_use_feedback() {
            reason_codes.push("feedback_invalid".to_owned());
        }

        let learning_rate = self.learning_rate.clamp(0.0, 1.0);
        let feedback_pressure = if feedback.has_contradictions() {
            1.0 + (feedback.contradiction_count as f32 * 0.08).min(0.40)
        } else if feedback.is_high_quality() {
            0.45
        } else {
            1.0
        };
        let blend_rate = (learning_rate * feedback_pressure).clamp(0.0, 1.0);
        let adjusted = previous.blend(target, blend_rate);
        let observations_after = observations_before.saturating_add(1);
        let can_commit = feedback.can_use_feedback()
            && self.learning_rate.is_finite()
            && self.learning_rate >= 0.0
            && previous.summary().can_use_hierarchy_weights()
            && target.summary().can_use_hierarchy_weights()
            && adjusted.summary().can_use_hierarchy_weights();
        let requires_repair_first = !can_commit;

        if requires_repair_first {
            reason_codes.push("repair_first".to_owned());
        }

        TaskAwareHierarchyAdjustmentReport {
            profile: feedback.profile,
            feedback,
            previous,
            target,
            adjusted,
            observations_before,
            observations_after,
            learning_rate: self.learning_rate,
            can_commit,
            requires_repair_first,
            reason_codes,
        }
    }
}

impl Default for TaskAwareHierarchyAdjustmentPolicy {
    fn default() -> Self {
        Self {
            weights: ProfileHierarchyWeights::target_defaults(),
            baseline: ProfileHierarchyWeights::target_defaults(),
            observations: ProfileHierarchyObservations::default(),
            learning_rate: 0.12,
        }
    }
}

impl HierarchyWeightsSummary {
    pub fn weights_are_finite(self) -> bool {
        self.global.is_finite()
            && self.local.is_finite()
            && self.fusion.is_finite()
            && self.total.is_finite()
    }

    pub fn weights_are_nonnegative(self) -> bool {
        self.global >= 0.0 && self.local >= 0.0 && self.fusion >= 0.0
    }

    pub fn total_matches_weights(self) -> bool {
        self.weights_are_finite() && float_close(self.total, self.global + self.local + self.fusion)
    }

    pub fn normalized_flag_matches_total(self) -> bool {
        self.weights_are_finite() && self.is_normalized == float_close(self.total, 1.0)
    }

    pub fn dominant_focus_matches_weights(self) -> bool {
        self.weights_are_finite()
            && self.weights_are_nonnegative()
            && self.dominant == dominant_focus_for_values(self.global, self.local, self.fusion)
    }

    pub fn active_weight_signal_component_count(self) -> usize {
        usize::from(finite_positive(self.global))
            + usize::from(finite_positive(self.local))
            + usize::from(finite_positive(self.fusion))
    }

    pub fn focus_signal_component_count(self) -> usize {
        usize::from(self.dominant != HierarchyWeightFocus::Balanced)
            + usize::from(self.dominant_focus_matches_weights())
    }

    pub fn normalization_signal_component_count(self) -> usize {
        usize::from(self.is_normalized && self.normalized_flag_matches_total())
    }

    pub fn hierarchy_signal_component_count(self) -> usize {
        self.active_weight_signal_component_count()
            .saturating_add(self.focus_signal_component_count())
            .saturating_add(self.normalization_signal_component_count())
    }

    pub fn has_hierarchy_signal_components(self) -> bool {
        self.hierarchy_signal_component_count() > 0
    }

    pub fn weight_shape_problem_component_count(self) -> usize {
        usize::from(!self.weights_are_finite())
            + usize::from(!self.weights_are_nonnegative())
            + usize::from(!self.total_matches_weights())
    }

    pub fn focus_problem_component_count(self) -> usize {
        usize::from(!self.dominant_focus_matches_weights())
    }

    pub fn normalization_problem_component_count(self) -> usize {
        usize::from(!self.is_normalized) + usize::from(!self.normalized_flag_matches_total())
    }

    pub fn hierarchy_problem_component_count(self) -> usize {
        self.weight_shape_problem_component_count()
            .saturating_add(self.focus_problem_component_count())
            .saturating_add(self.normalization_problem_component_count())
    }

    pub fn has_hierarchy_problem_components(self) -> bool {
        self.hierarchy_problem_component_count() > 0
    }

    pub fn hierarchy_accounting_is_consistent(self) -> bool {
        let expected_signal_count = self
            .active_weight_signal_component_count()
            .saturating_add(self.focus_signal_component_count())
            .saturating_add(self.normalization_signal_component_count());
        let expected_problem_count = self
            .weight_shape_problem_component_count()
            .saturating_add(self.focus_problem_component_count())
            .saturating_add(self.normalization_problem_component_count());

        self.hierarchy_signal_component_count() == expected_signal_count
            && self.hierarchy_problem_component_count() == expected_problem_count
    }

    pub fn hierarchy_shape_is_clean(self) -> bool {
        !self.has_hierarchy_problem_components() && self.hierarchy_accounting_is_consistent()
    }

    pub fn can_use_hierarchy_weights(self) -> bool {
        self.hierarchy_shape_is_clean()
    }
}

fn finite_positive(value: f32) -> bool {
    value.is_finite() && value > 0.0
}

fn finite_unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn float_close(left: f32, right: f32) -> bool {
    (left - right).abs() <= 0.0001
}

fn dominant_focus_for_values(global: f32, local: f32, fusion: f32) -> HierarchyWeightFocus {
    let max = global.max(local).max(fusion);
    let winners = [
        (HierarchyWeightFocus::Global, global),
        (HierarchyWeightFocus::Local, local),
        (HierarchyWeightFocus::Fusion, fusion),
    ]
    .into_iter()
    .filter(|(_, value)| (*value - max).abs() <= 0.0001)
    .count();

    if winners != 1 {
        return HierarchyWeightFocus::Balanced;
    }

    if float_close(global, max) {
        HierarchyWeightFocus::Global
    } else if float_close(local, max) {
        HierarchyWeightFocus::Local
    } else {
        HierarchyWeightFocus::Fusion
    }
}

fn low_quality_target(profile: TaskProfile) -> HierarchyWeights {
    match profile {
        TaskProfile::General => HierarchyWeights::new(0.32, 0.38, 0.30),
        TaskProfile::Coding => HierarchyWeights::new(0.18, 0.66, 0.16),
        TaskProfile::Writing => HierarchyWeights::new(0.66, 0.22, 0.12),
        TaskProfile::LongDocument => HierarchyWeights::new(0.24, 0.18, 0.58),
    }
}

fn fusion_pressure_target(current: HierarchyWeights) -> HierarchyWeights {
    HierarchyWeights::new(
        current.global * 0.88,
        current.local * 0.82,
        current.fusion + 0.24,
    )
}

fn hierarchy_weights_close(left: HierarchyWeights, right: HierarchyWeights) -> bool {
    float_close(left.global, right.global)
        && float_close(left.local, right.local)
        && float_close(left.fusion, right.fusion)
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hierarchy_weights_normalize() {
        let weights = HierarchyWeights::new(2.0, 1.0, 1.0);
        let summary = weights.summary();

        assert!((weights.global + weights.local + weights.fusion - 1.0).abs() < 0.0001);
        assert!(weights.global > weights.local);
        assert!(summary.is_normalized);
        assert_eq!(summary.dominant, HierarchyWeightFocus::Global);
        assert!((summary.total - 1.0).abs() < 0.0001);
        assert!(summary.weights_are_finite());
        assert!(summary.weights_are_nonnegative());
        assert!(summary.total_matches_weights());
        assert!(summary.normalized_flag_matches_total());
        assert!(summary.dominant_focus_matches_weights());
        assert_eq!(summary.active_weight_signal_component_count(), 3);
        assert_eq!(summary.focus_signal_component_count(), 2);
        assert_eq!(summary.normalization_signal_component_count(), 1);
        assert_eq!(summary.hierarchy_signal_component_count(), 6);
        assert!(summary.has_hierarchy_signal_components());
        assert_eq!(summary.weight_shape_problem_component_count(), 0);
        assert_eq!(summary.focus_problem_component_count(), 0);
        assert_eq!(summary.normalization_problem_component_count(), 0);
        assert_eq!(summary.hierarchy_problem_component_count(), 0);
        assert!(!summary.has_hierarchy_problem_components());
        assert!(summary.hierarchy_accounting_is_consistent());
        assert!(summary.hierarchy_shape_is_clean());
        assert!(summary.can_use_hierarchy_weights());
    }

    #[test]
    fn hierarchy_weights_normalize_sanitizes_nonfinite_inputs() {
        let weights = HierarchyWeights::new(f32::NAN, f32::INFINITY, 2.0);
        let summary = weights.summary();

        assert_eq!(weights.global, 0.0);
        assert_eq!(weights.local, 0.0);
        assert_eq!(weights.fusion, 1.0);
        assert!(summary.weights_are_finite());
        assert!(summary.weights_are_nonnegative());
        assert!(summary.total_matches_weights());
        assert!(summary.normalized_flag_matches_total());
        assert!(summary.dominant_focus_matches_weights());
        assert_eq!(summary.dominant, HierarchyWeightFocus::Fusion);
        assert_eq!(summary.hierarchy_problem_component_count(), 0);
        assert!(summary.hierarchy_shape_is_clean());
        assert!(summary.can_use_hierarchy_weights());

        let fallback = HierarchyWeights::new(f32::NAN, f32::NEG_INFINITY, -1.0).summary();
        assert!(fallback.weights_are_finite());
        assert!(fallback.weights_are_nonnegative());
        assert!(fallback.is_normalized);
        assert!(fallback.total_matches_weights());
        assert!(fallback.hierarchy_shape_is_clean());
    }

    #[test]
    fn hierarchy_weights_blend_preserves_normalized_shape() {
        let coding = HierarchyWeights::for_profile(TaskProfile::Coding);
        let long = HierarchyWeights::for_profile(TaskProfile::LongDocument);

        let blended = coding.blend(long, 0.50);
        let summary = blended.summary();

        assert!(summary.is_normalized);
        assert!(summary.local > 0.0);
        assert!(summary.fusion > coding.fusion);
        assert!(summary.fusion < long.fusion);
    }

    #[test]
    fn profile_hierarchy_weights_summary_reports_expected_focuses() {
        let weights = ProfileHierarchyWeights::target_defaults();
        let summary = weights.summary();

        assert!(summary.all_normalized());
        assert!(summary.expected_profile_focuses());
        assert_eq!(summary.normalized_profile_signal_component_count(), 4);
        assert_eq!(summary.expected_focus_signal_component_count(), 3);
        assert_eq!(summary.hierarchy_profile_signal_component_count(), 7);
        assert!(summary.has_hierarchy_profile_signal_components());
        assert_eq!(summary.per_profile_problem_component_count(), 0);
        assert_eq!(summary.normalized_profile_problem_component_count(), 0);
        assert_eq!(summary.expected_focus_problem_component_count(), 0);
        assert_eq!(summary.hierarchy_profile_problem_component_count(), 0);
        assert!(!summary.has_hierarchy_profile_problem_components());
        assert!(summary.hierarchy_profile_accounting_is_consistent());
        assert!(summary.hierarchy_profile_shape_is_clean());
        assert!(summary.can_use_profile_hierarchy_weights());
        assert_eq!(
            summary.summary_for(TaskProfile::Coding).dominant,
            HierarchyWeightFocus::Local
        );
        assert_eq!(
            summary.summary_for(TaskProfile::Writing).dominant,
            HierarchyWeightFocus::Global
        );
        assert_eq!(
            summary.summary_for(TaskProfile::LongDocument).dominant,
            HierarchyWeightFocus::Fusion
        );
    }

    #[test]
    fn profile_hierarchy_weights_normalize_each_profile() {
        let mut weights = ProfileHierarchyWeights::from_single(HierarchyWeights {
            global: 2.0,
            local: 2.0,
            fusion: 0.0,
        });

        weights.set(
            TaskProfile::LongDocument,
            HierarchyWeights {
                global: 0.0,
                local: 0.0,
                fusion: 0.0,
            },
        );
        weights.normalize();

        let summary = weights.summary();

        assert!(summary.all_normalized());
        assert_eq!(summary.general.dominant, HierarchyWeightFocus::Balanced);
        assert_eq!(summary.long_document.dominant, HierarchyWeightFocus::Global);
        assert!((summary.long_document.total - 1.0).abs() < 0.0001);
        assert_eq!(summary.normalized_profile_problem_component_count(), 0);
        assert_eq!(summary.expected_focus_problem_component_count(), 3);
        assert_eq!(summary.hierarchy_profile_problem_component_count(), 3);
        assert!(summary.has_hierarchy_profile_problem_components());
        assert!(summary.hierarchy_profile_accounting_is_consistent());
        assert!(!summary.hierarchy_profile_shape_is_clean());
        assert!(!summary.can_use_profile_hierarchy_weights());
    }

    #[test]
    fn profile_hierarchy_observations_count_active_profiles() {
        let mut observations = ProfileHierarchyObservations::from_single(2);

        observations.bump(TaskProfile::Coding);
        observations.bump(TaskProfile::Coding);
        observations.bump(TaskProfile::LongDocument);

        assert_eq!(observations.get(TaskProfile::General), 2);
        assert_eq!(observations.get(TaskProfile::Coding), 2);
        assert_eq!(observations.get(TaskProfile::Writing), 0);
        assert_eq!(observations.get(TaskProfile::LongDocument), 1);
        assert_eq!(observations.total(), 5);
        assert_eq!(observations.active_profile_count(), 3);
        assert_eq!(observations.profile_observation_signal_component_count(), 5);
        assert!(observations.has_profile_observation_signal_components());
        assert!(observations.profile_observation_accounting_is_consistent());
        assert!(observations.profile_observation_shape_is_clean());
        assert!(observations.can_use_profile_hierarchy_observations());
    }

    #[test]
    fn hierarchy_weights_summary_counts_public_shape_drift() {
        let summary = HierarchyWeightsSummary {
            global: 0.7,
            local: -0.1,
            fusion: f32::NAN,
            total: 0.7,
            dominant: HierarchyWeightFocus::Fusion,
            is_normalized: true,
        };

        assert!(!summary.weights_are_finite());
        assert!(!summary.weights_are_nonnegative());
        assert!(!summary.total_matches_weights());
        assert!(!summary.normalized_flag_matches_total());
        assert!(!summary.dominant_focus_matches_weights());
        assert_eq!(summary.active_weight_signal_component_count(), 1);
        assert_eq!(summary.focus_signal_component_count(), 1);
        assert_eq!(summary.normalization_signal_component_count(), 0);
        assert_eq!(summary.hierarchy_signal_component_count(), 2);
        assert_eq!(summary.weight_shape_problem_component_count(), 3);
        assert_eq!(summary.focus_problem_component_count(), 1);
        assert_eq!(summary.normalization_problem_component_count(), 1);
        assert_eq!(summary.hierarchy_problem_component_count(), 5);
        assert!(summary.has_hierarchy_problem_components());
        assert!(summary.hierarchy_accounting_is_consistent());
        assert!(!summary.hierarchy_shape_is_clean());
        assert!(!summary.can_use_hierarchy_weights());
    }

    #[test]
    fn profile_hierarchy_weights_summary_counts_profile_shape_drift() {
        let invalid = HierarchyWeightsSummary {
            global: 0.7,
            local: -0.1,
            fusion: f32::NAN,
            total: 0.7,
            dominant: HierarchyWeightFocus::Fusion,
            is_normalized: true,
        };
        let wrong_coding_focus = HierarchyWeightsSummary {
            global: 0.4,
            local: 0.3,
            fusion: 0.3,
            total: 1.0,
            dominant: HierarchyWeightFocus::Global,
            is_normalized: true,
        };
        let wrong_writing_focus = HierarchyWeights::for_profile(TaskProfile::Coding).summary();
        let normalized_flag_drift = HierarchyWeightsSummary {
            global: 0.3,
            local: 0.2,
            fusion: 0.5,
            total: 1.0,
            dominant: HierarchyWeightFocus::Fusion,
            is_normalized: false,
        };

        let summary = ProfileHierarchyWeightsSummary {
            general: invalid,
            coding: wrong_coding_focus,
            writing: wrong_writing_focus,
            long_document: normalized_flag_drift,
        };

        assert!(!summary.all_normalized());
        assert!(!summary.expected_profile_focuses());
        assert_eq!(summary.normalized_profile_signal_component_count(), 3);
        assert_eq!(summary.expected_focus_signal_component_count(), 1);
        assert_eq!(summary.hierarchy_profile_signal_component_count(), 4);
        assert_eq!(summary.per_profile_problem_component_count(), 7);
        assert_eq!(summary.normalized_profile_problem_component_count(), 1);
        assert_eq!(summary.expected_focus_problem_component_count(), 2);
        assert_eq!(summary.hierarchy_profile_problem_component_count(), 10);
        assert!(summary.has_hierarchy_profile_problem_components());
        assert!(summary.hierarchy_profile_accounting_is_consistent());
        assert!(!summary.hierarchy_profile_shape_is_clean());
        assert!(!summary.can_use_profile_hierarchy_weights());
    }

    #[test]
    fn empty_profile_hierarchy_observations_are_noop() {
        let observations = ProfileHierarchyObservations::default();

        assert_eq!(observations.total(), 0);
        assert_eq!(observations.active_profile_count(), 0);
        assert_eq!(observations.profile_observation_signal_component_count(), 0);
        assert!(!observations.has_profile_observation_signal_components());
        assert!(observations.profile_observation_accounting_is_consistent());
        assert!(observations.profile_observation_shape_is_clean());
        assert!(!observations.can_use_profile_hierarchy_observations());
    }

    #[test]
    fn task_aware_hierarchy_adjustment_moves_coding_low_quality_toward_local_focus() {
        let mut policy = TaskAwareHierarchyAdjustmentPolicy::new();
        let previous = policy.hierarchy_for(TaskProfile::Coding);

        let report = policy.observe(HierarchyAdjustmentFeedback::new(
            TaskProfile::Coding,
            0.32,
            18.0,
            0,
        ));

        assert!(report.can_commit);
        assert!(!report.requires_repair_first);
        assert_eq!(report.profile, TaskProfile::Coding);
        assert_eq!(report.observations_before, 0);
        assert_eq!(report.observations_after, 1);
        assert!(report.observation_advanced());
        assert!(report.has_weight_delta());
        assert!(report.adjusted.local > previous.local);
        assert_eq!(
            report.target.summary().dominant,
            HierarchyWeightFocus::Local
        );
        assert!(report.reason_codes.iter().any(|code| code == "quality_low"));
        assert!(
            report
                .reason_codes
                .iter()
                .any(|code| code == "profile:coding")
        );
        assert!(report.adjustment_shape_is_clean());
        assert!(policy.hierarchy_for(TaskProfile::Coding).local > previous.local);
        assert_eq!(policy.observations().get(TaskProfile::Coding), 1);
    }

    #[test]
    fn task_aware_hierarchy_adjustment_raises_fusion_for_long_document_contradictions() {
        let mut policy = TaskAwareHierarchyAdjustmentPolicy::new();
        let previous = policy.hierarchy_for(TaskProfile::LongDocument);

        let report = policy.observe(HierarchyAdjustmentFeedback::new(
            TaskProfile::LongDocument,
            0.72,
            7.0,
            3,
        ));

        assert!(report.can_commit);
        assert!(!report.requires_repair_first);
        assert!(report.target.fusion > previous.fusion);
        assert!(report.adjusted.fusion > previous.fusion);
        assert_eq!(
            report.target.summary().dominant,
            HierarchyWeightFocus::Fusion
        );
        assert!(
            report
                .reason_codes
                .iter()
                .any(|code| code == "contradiction_pressure")
        );
        assert!(report.feedback.has_contradictions());
        assert!(report.adjusted_weights_are_clean());
        assert!(report.adjustment_accounting_is_consistent());
        assert!(policy.hierarchy_for(TaskProfile::LongDocument).fusion > previous.fusion);
        assert_eq!(policy.observations().get(TaskProfile::LongDocument), 1);
    }

    #[test]
    fn task_aware_hierarchy_adjustment_rejects_invalid_feedback_without_mutation() {
        let mut policy = TaskAwareHierarchyAdjustmentPolicy::new();
        let previous = policy.hierarchy_for(TaskProfile::Writing);

        let report = policy.observe(HierarchyAdjustmentFeedback {
            profile: TaskProfile::Writing,
            quality: f32::NAN,
            perplexity: 3.0,
            contradiction_count: 0,
        });

        assert!(!report.can_commit);
        assert!(report.requires_repair_first);
        assert!(!report.feedback.can_use_feedback());
        assert!(
            report
                .reason_codes
                .iter()
                .any(|code| code == "feedback_invalid")
        );
        assert!(
            report
                .reason_codes
                .iter()
                .any(|code| code == "repair_first")
        );
        assert_eq!(policy.hierarchy_for(TaskProfile::Writing), previous);
        assert_eq!(policy.observations().get(TaskProfile::Writing), 0);
        assert_eq!(report.adjustment_problem_component_count(), 1);
        assert!(report.adjustment_accounting_is_consistent());
        assert!(!report.adjustment_shape_is_clean());
    }

    #[test]
    fn mutation_plan_prioritizes_task_aware_hierarchy_candidates_without_writes() {
        let policy = TaskAwareHierarchyAdjustmentPolicy::new();
        let plan = policy.preview_mutation_plan(&[
            HierarchyAdjustmentFeedback::new(TaskProfile::Coding, 0.31, 18.0, 0),
            HierarchyAdjustmentFeedback::new(TaskProfile::LongDocument, 0.70, 8.0, 3),
            HierarchyAdjustmentFeedback {
                profile: TaskProfile::Writing,
                quality: f32::NAN,
                perplexity: 4.0,
                contradiction_count: 0,
            },
        ]);

        assert_eq!(plan.records.len(), 3);
        assert_eq!(plan.commit_ready_count(), 2);
        assert_eq!(plan.repair_required_count(), 1);
        assert!(!plan.persistent_writes_allowed());
        assert!(plan.requires_operator_review());
        assert_eq!(plan.records[0].kind, HierarchyMutationKind::IncreaseLocal);
        assert_eq!(plan.records[1].kind, HierarchyMutationKind::IncreaseFusion);
        assert_eq!(plan.records[2].kind, HierarchyMutationKind::RepairRequired);
        assert_eq!(
            plan.profile_codes(),
            vec![
                "coding".to_owned(),
                "long_document".to_owned(),
                "writing".to_owned()
            ]
        );
        assert_eq!(
            plan.mutation_kind_codes(),
            vec![
                "increase_fusion".to_owned(),
                "increase_local".to_owned(),
                "repair_required".to_owned()
            ]
        );
        assert_eq!(
            plan.next_commit_candidate()
                .map(TaskAwareHierarchyMutationRecord::detail_code),
            Some("1:coding:increase_local:preview".to_owned())
        );
        assert!(plan.reason_codes().contains(&"quality_low".to_owned()));
        assert!(
            plan.reason_codes()
                .contains(&"contradiction_pressure".to_owned())
        );
        assert!(plan.reason_codes().contains(&"feedback_invalid".to_owned()));
        assert_eq!(
            plan.summary_line(),
            "task_aware_hierarchy_mutation_plan empty=false records=3 commit_ready=2 repair_required=1 operator_review_required=true persistent_writes_allowed=false profiles=coding|long_document|writing mutation_kinds=increase_fusion|increase_local|repair_required reason_codes=contradiction_pressure|feedback_invalid|profile:coding|profile:long_document|profile:writing|quality_low|quality_neutral|repair_first detail_codes=1:coding:increase_local:preview|2:long_document:increase_fusion:preview|3:writing:repair_required:preview next_commit=1:coding:increase_local:preview"
        );
    }

    #[test]
    fn mutation_history_records_preview_and_committed_reports() {
        let mut policy = TaskAwareHierarchyAdjustmentPolicy::new();
        let preview = policy.preview_adjustment(HierarchyAdjustmentFeedback::new(
            TaskProfile::Coding,
            0.30,
            20.0,
            0,
        ));
        let committed = policy.observe(HierarchyAdjustmentFeedback::new(
            TaskProfile::LongDocument,
            0.72,
            7.0,
            2,
        ));
        let repair = policy.preview_adjustment(HierarchyAdjustmentFeedback {
            profile: TaskProfile::Writing,
            quality: 0.4,
            perplexity: f32::NEG_INFINITY,
            contradiction_count: 0,
        });
        let mut history = TaskAwareHierarchyMutationHistory::new();

        let preview_record = history.record_preview(&preview);
        let committed_record = history.record_committed(&committed);
        let repair_record = history.record_committed(&repair);

        assert_eq!(preview_record.sequence, 1);
        assert_eq!(preview_record.kind, HierarchyMutationKind::IncreaseLocal);
        assert!(!preview_record.committed);
        assert_eq!(committed_record.sequence, 2);
        assert_eq!(committed_record.kind, HierarchyMutationKind::IncreaseFusion);
        assert!(committed_record.committed);
        assert_eq!(repair_record.sequence, 3);
        assert_eq!(repair_record.kind, HierarchyMutationKind::RepairRequired);
        assert!(!repair_record.committed);
        assert_eq!(history.len(), 3);
        assert_eq!(history.committed_count(), 1);
        assert_eq!(history.repair_required_count(), 1);
        assert_eq!(
            history.detail_codes(),
            vec![
                "1:coding:increase_local:preview".to_owned(),
                "2:long_document:increase_fusion:committed".to_owned(),
                "3:writing:repair_required:preview".to_owned(),
            ]
        );
        assert_eq!(
            history.summary_line(),
            "task_aware_hierarchy_mutation_history records=3 committed=1 repair_required=1 persistent_writes_allowed=false detail_codes=1:coding:increase_local:preview|2:long_document:increase_fusion:committed|3:writing:repair_required:preview"
        );
        assert!(
            committed_record
                .summary_line()
                .contains("profile=long_document kind=increase_fusion")
        );
    }
}
