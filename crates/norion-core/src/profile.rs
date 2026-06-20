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
}
