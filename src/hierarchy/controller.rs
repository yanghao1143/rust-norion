use crate::router::GenerationMetrics;

use super::profile::TaskProfile;
use super::profile_state::{ProfileHierarchyObservations, ProfileHierarchyWeights};
use super::weights::HierarchyWeights;

#[derive(Debug, Clone)]
pub struct HierarchyController {
    current: HierarchyWeights,
    profile_weights: ProfileHierarchyWeights,
    learning_rate: f32,
    profile_observations: ProfileHierarchyObservations,
}

#[derive(Debug, Clone, Copy)]
pub struct HierarchyState {
    pub current: HierarchyWeights,
    pub profile_weights: ProfileHierarchyWeights,
    pub profile_observations: ProfileHierarchyObservations,
}

impl Default for HierarchyController {
    fn default() -> Self {
        Self {
            current: HierarchyWeights::default(),
            profile_weights: ProfileHierarchyWeights::target_defaults(),
            learning_rate: 0.22,
            profile_observations: ProfileHierarchyObservations::default(),
        }
    }
}

impl HierarchyController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> HierarchyWeights {
        self.current
    }

    pub fn state(&self) -> HierarchyState {
        HierarchyState {
            current: self.current,
            profile_weights: self.profile_weights,
            profile_observations: self.profile_observations,
        }
    }

    pub fn restore_state(&mut self, state: HierarchyState) {
        self.current = state.current;
        self.current.normalize();
        self.profile_weights = state.profile_weights;
        self.profile_weights.normalize();
        self.profile_observations = state.profile_observations;
    }

    pub fn target_for_profile(profile: TaskProfile) -> HierarchyWeights {
        target_for_profile(profile)
    }

    pub fn adapt_to_profile(&mut self, profile: TaskProfile) -> HierarchyWeights {
        let learned = self.profile_weights.get(profile);
        let target = Self::target_for_profile(profile);
        let adapted = learned.blend(target, self.learning_rate * 0.25);
        self.profile_weights.set(profile, adapted);
        self.current = adapted;
        self.current
    }

    pub fn observe(
        &mut self,
        profile: TaskProfile,
        metrics: GenerationMetrics,
    ) -> HierarchyWeights {
        let mut target = Self::target_for_profile(profile);
        let quality = metrics.quality_score();

        if quality < 0.55 {
            match profile {
                TaskProfile::Coding => target.local += 0.12,
                TaskProfile::Writing => target.global += 0.12,
                TaskProfile::LongDocument => target.convolution += 0.12,
                TaskProfile::General => target.global += 0.06,
            }
        } else if quality > 0.84 {
            target.convolution += 0.05;
        }

        target.normalize();
        let learned = self
            .profile_weights
            .get(profile)
            .blend(target, self.learning_rate);
        self.profile_weights.set(profile, learned);
        self.profile_observations.bump(profile);
        self.current = learned;
        self.current
    }
}

pub(crate) fn target_for_profile(profile: TaskProfile) -> HierarchyWeights {
    match profile {
        TaskProfile::General => HierarchyWeights::new(0.36, 0.42, 0.22),
        TaskProfile::Coding => HierarchyWeights::new(0.24, 0.58, 0.18),
        TaskProfile::Writing => HierarchyWeights::new(0.56, 0.30, 0.14),
        TaskProfile::LongDocument => HierarchyWeights::new(0.30, 0.22, 0.48),
    }
}
