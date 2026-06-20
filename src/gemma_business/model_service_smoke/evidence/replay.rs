mod fields;
mod rust_check;

use super::business_contract::BusinessContractEvidence;
use fields::{
    applied, live_evolution_items, live_evolution_online_reward_feedbacks,
    live_memory_feedback_applied, live_memory_feedback_updates,
};
pub(in crate::gemma_business::model_service_smoke) use rust_check::RustCheckReplayEvidence;

#[derive(Debug, Clone, Copy, Default)]
pub(in crate::gemma_business::model_service_smoke) struct ReplayEvidence {
    pub(in crate::gemma_business::model_service_smoke) applied: u64,
    pub(in crate::gemma_business::model_service_smoke) live_memory_feedback_updates: u64,
    pub(in crate::gemma_business::model_service_smoke) live_memory_feedback_applied: u64,
    pub(in crate::gemma_business::model_service_smoke) live_evolution_items: u64,
    pub(in crate::gemma_business::model_service_smoke) live_evolution_online_reward_feedbacks: u64,
    pub(in crate::gemma_business::model_service_smoke) rust_check: RustCheckReplayEvidence,
    pub(in crate::gemma_business::model_service_smoke) business_contract: BusinessContractEvidence,
}

impl ReplayEvidence {
    pub(in crate::gemma_business::model_service_smoke) fn from_body(body: &str) -> Self {
        Self {
            applied: applied(body),
            live_memory_feedback_updates: live_memory_feedback_updates(body),
            live_memory_feedback_applied: live_memory_feedback_applied(body),
            live_evolution_items: live_evolution_items(body),
            live_evolution_online_reward_feedbacks: live_evolution_online_reward_feedbacks(body),
            rust_check: RustCheckReplayEvidence::from_replay_body(body),
            business_contract: BusinessContractEvidence::from_replay_body(body),
        }
    }

    pub(in crate::gemma_business::model_service_smoke) fn self_improvement_replay_evidence(
        self,
    ) -> bool {
        self.applied > 0 && (self.live_memory_feedback_updates > 0 || self.live_evolution_items > 0)
    }
}
