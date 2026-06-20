use crate::experience::ExperienceMatch;
use crate::hardware::{HardwarePlan, RuntimeAdapterHint};

use crate::runtime::{
    device::{experience_matches_hardware_plan, parse_runtime_adapter_hint},
    option_f32_display,
};

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeAdapterObservation {
    pub adapter: RuntimeAdapterHint,
    pub score: f32,
    pub reward: f32,
    pub quality: f32,
    pub forward_energy: Option<f32>,
    pub kv_influence: Option<f32>,
    pub experience_id: u64,
}

impl RuntimeAdapterObservation {
    pub fn new(
        adapter: RuntimeAdapterHint,
        score: f32,
        reward: f32,
        quality: f32,
        forward_energy: Option<f32>,
        kv_influence: Option<f32>,
        experience_id: u64,
    ) -> Self {
        Self {
            adapter,
            score: score.clamp(0.0, 1.0),
            reward: reward.clamp(0.0, 1.0),
            quality: quality.clamp(0.0, 1.0),
            forward_energy: forward_energy.filter(|value| value.is_finite()),
            kv_influence: kv_influence.filter(|value| value.is_finite()),
            experience_id,
        }
    }

    pub fn from_experiences(experiences: &[ExperienceMatch], runtime_model_id: &str) -> Vec<Self> {
        let mut observations = experiences
            .iter()
            .filter(|experience| {
                runtime_model_id.is_empty()
                    || experience
                        .runtime_model_id
                        .as_deref()
                        .map(|model_id| model_id == runtime_model_id)
                        .unwrap_or(true)
            })
            .filter_map(|experience| {
                let adapter =
                    parse_runtime_adapter_hint(experience.runtime_selected_adapter.as_deref()?)?;
                let base = experience.score * 0.38
                    + experience.process_reward * 0.34
                    + experience.quality * 0.22;
                let kv_bonus = experience
                    .runtime_kv_influence
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0)
                    * 0.06;
                let energy_penalty = experience
                    .runtime_forward_energy
                    .unwrap_or(0.0)
                    .clamp(0.0, 1.0)
                    * 0.04;
                Some(Self::new(
                    adapter,
                    base + kv_bonus - energy_penalty,
                    experience.process_reward,
                    experience.quality,
                    experience.runtime_forward_energy,
                    experience.runtime_kv_influence,
                    experience.id,
                ))
            })
            .collect::<Vec<_>>();

        observations.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.experience_id.cmp(&right.experience_id))
        });
        observations.truncate(6);
        observations
    }

    pub fn from_experiences_for_hardware(
        experiences: &[ExperienceMatch],
        runtime_model_id: &str,
        hardware_plan: &HardwarePlan,
    ) -> Vec<Self> {
        Self::from_experiences(experiences, runtime_model_id)
            .into_iter()
            .filter(|observation| {
                hardware_plan
                    .execution
                    .adapter_hints
                    .contains(&observation.adapter)
            })
            .filter(|observation| {
                experiences
                    .iter()
                    .find(|experience| experience.id == observation.experience_id)
                    .map(|experience| experience_matches_hardware_plan(experience, hardware_plan))
                    .unwrap_or(false)
            })
            .collect()
    }

    pub fn summary(&self) -> String {
        format!(
            "adapter={} score={:.3} reward={:.3} quality={:.3} forward_energy={} kv_influence={} experience={}",
            self.adapter.as_str(),
            self.score,
            self.reward,
            self.quality,
            option_f32_display(self.forward_energy),
            option_f32_display(self.kv_influence),
            self.experience_id
        )
    }
}
