use std::str::FromStr;

use crate::router::GenerationMetrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            "general" | "通用" => Ok(Self::General),
            "coding" | "code" | "rust" | "代码" | "编程" => Ok(Self::Coding),
            "writing" | "write" | "小说" | "写作" => Ok(Self::Writing),
            "long" | "longdoc" | "long-document" | "document" | "长文档" => {
                Ok(Self::LongDocument)
            }
            other => Err(format!("unknown task profile: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HierarchyWeights {
    pub global: f32,
    pub local: f32,
    pub convolution: f32,
}

impl HierarchyWeights {
    pub fn new(global: f32, local: f32, convolution: f32) -> Self {
        let mut weights = Self {
            global,
            local,
            convolution,
        };
        weights.normalize();
        weights
    }

    pub fn normalize(&mut self) {
        self.global = self.global.max(0.0);
        self.local = self.local.max(0.0);
        self.convolution = self.convolution.max(0.0);

        let sum = self.global + self.local + self.convolution;
        if sum <= f32::EPSILON {
            self.global = 0.34;
            self.local = 0.33;
            self.convolution = 0.33;
            return;
        }

        self.global /= sum;
        self.local /= sum;
        self.convolution /= sum;
    }

    pub fn blend(self, target: Self, rate: f32) -> Self {
        let rate = rate.clamp(0.0, 1.0);
        Self::new(
            self.global * (1.0 - rate) + target.global * rate,
            self.local * (1.0 - rate) + target.local * rate,
            self.convolution * (1.0 - rate) + target.convolution * rate,
        )
    }
}

impl Default for HierarchyWeights {
    fn default() -> Self {
        Self::new(0.36, 0.42, 0.22)
    }
}

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

#[derive(Debug, Clone, Copy)]
pub struct ProfileHierarchyWeights {
    pub general: HierarchyWeights,
    pub coding: HierarchyWeights,
    pub writing: HierarchyWeights,
    pub long_document: HierarchyWeights,
}

impl ProfileHierarchyWeights {
    pub fn target_defaults() -> Self {
        Self {
            general: HierarchyController::target_for_profile(TaskProfile::General),
            coding: HierarchyController::target_for_profile(TaskProfile::Coding),
            writing: HierarchyController::target_for_profile(TaskProfile::Writing),
            long_document: HierarchyController::target_for_profile(TaskProfile::LongDocument),
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
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProfileHierarchyObservations {
    pub general: u64,
    pub coding: u64,
    pub writing: u64,
    pub long_document: u64,
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
        match profile {
            TaskProfile::General => HierarchyWeights::new(0.36, 0.42, 0.22),
            TaskProfile::Coding => HierarchyWeights::new(0.24, 0.58, 0.18),
            TaskProfile::Writing => HierarchyWeights::new(0.56, 0.30, 0.14),
            TaskProfile::LongDocument => HierarchyWeights::new(0.30, 0.22, 0.48),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coding_profile_prefers_local_attention() {
        let target = HierarchyController::target_for_profile(TaskProfile::Coding);

        assert!(target.local > target.global);
        assert!(target.local > target.convolution);
    }

    #[test]
    fn weights_are_normalized() {
        let weights = HierarchyWeights::new(10.0, 5.0, 1.0);
        let sum = weights.global + weights.local + weights.convolution;

        assert!((sum - 1.0).abs() < 0.0001);
    }

    #[test]
    fn observations_update_only_selected_profile_weights() {
        let mut controller = HierarchyController::new();
        let coding_before = controller.state().profile_weights.get(TaskProfile::Coding);
        let writing_before = controller.state().profile_weights.get(TaskProfile::Writing);

        controller.observe(
            TaskProfile::Writing,
            GenerationMetrics {
                perplexity: 30.0,
                semantic_consistency: 0.2,
                contradiction_count: 2,
                token_count: 32,
            },
        );

        let state = controller.state();
        let coding_after = state.profile_weights.get(TaskProfile::Coding);
        let writing_after = state.profile_weights.get(TaskProfile::Writing);
        assert!((coding_after.local - coding_before.local).abs() < 0.0001);
        assert!(writing_after.global > writing_before.global);
        assert_eq!(state.profile_observations.get(TaskProfile::Writing), 1);
        assert_eq!(state.profile_observations.get(TaskProfile::Coding), 0);
    }

    #[test]
    fn adapt_to_profile_uses_profile_specific_learned_weights() {
        let mut controller = HierarchyController::new();
        controller.observe(
            TaskProfile::LongDocument,
            GenerationMetrics {
                perplexity: 32.0,
                semantic_consistency: 0.2,
                contradiction_count: 1,
                token_count: 64,
            },
        );
        let learned_long = controller
            .state()
            .profile_weights
            .get(TaskProfile::LongDocument);

        let adapted_coding = controller.adapt_to_profile(TaskProfile::Coding);
        let adapted_long = controller.adapt_to_profile(TaskProfile::LongDocument);

        assert!(adapted_coding.local > adapted_coding.convolution);
        assert!(learned_long.convolution > adapted_coding.convolution);
        assert!(adapted_long.convolution > adapted_coding.convolution);
    }
}
