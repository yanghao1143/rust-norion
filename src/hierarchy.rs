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
    learning_rate: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct HierarchyState {
    pub current: HierarchyWeights,
}

impl Default for HierarchyController {
    fn default() -> Self {
        Self {
            current: HierarchyWeights::default(),
            learning_rate: 0.22,
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
        }
    }

    pub fn restore_state(&mut self, state: HierarchyState) {
        self.current = state.current;
        self.current.normalize();
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
        let target = Self::target_for_profile(profile);
        self.current = self.current.blend(target, self.learning_rate);
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
        self.current = self.current.blend(target, self.learning_rate);
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
}
