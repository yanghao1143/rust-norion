mod fields;
mod replay;

use super::business_contract::BusinessContractEvidence;
use crate::gemma_business::smoke_report::GemmaModelServiceRuntimeAudit;
use fields::{
    evolution_external_feedback_memory_updates, evolution_external_feedbacks, runtime_tokens,
    rust_check_experiences, rust_check_passed,
};
use replay::InspectReplayEvidence;

#[derive(Debug, Clone, Copy, Default)]
pub(in crate::gemma_business::model_service_smoke) struct InspectEvidence {
    pub(in crate::gemma_business::model_service_smoke) runtime_audit: GemmaModelServiceRuntimeAudit,
    pub(in crate::gemma_business::model_service_smoke) runtime_tokens: u64,
    pub(in crate::gemma_business::model_service_smoke) evolution_external_feedbacks: u64,
    pub(in crate::gemma_business::model_service_smoke) evolution_external_feedback_memory_updates:
        u64,
    pub(in crate::gemma_business::model_service_smoke) rust_check_passed: u64,
    pub(in crate::gemma_business::model_service_smoke) rust_check_experiences: u64,
    pub(in crate::gemma_business::model_service_smoke) evolution_replay_rust_check_items: u64,
    pub(in crate::gemma_business::model_service_smoke) evolution_replay_rust_check_passed: u64,
    pub(in crate::gemma_business::model_service_smoke) evolution_replay_runs: u64,
    pub(in crate::gemma_business::model_service_smoke) evolution_replay_items: u64,
    pub(in crate::gemma_business::model_service_smoke) business_contract_state:
        BusinessContractEvidence,
    pub(in crate::gemma_business::model_service_smoke) business_contract_trace:
        BusinessContractEvidence,
    pub(in crate::gemma_business::model_service_smoke) business_contract_replay_ledger:
        BusinessContractEvidence,
}

impl InspectEvidence {
    pub(in crate::gemma_business::model_service_smoke) fn from_body(body: &str) -> Self {
        let replay = InspectReplayEvidence::from_body(body);
        Self {
            runtime_audit: GemmaModelServiceRuntimeAudit::from_inspect_body(body),
            runtime_tokens: runtime_tokens(body),
            evolution_external_feedbacks: evolution_external_feedbacks(body),
            evolution_external_feedback_memory_updates: evolution_external_feedback_memory_updates(
                body,
            ),
            rust_check_passed: rust_check_passed(body),
            rust_check_experiences: rust_check_experiences(body),
            evolution_replay_rust_check_items: replay.rust_check_items,
            evolution_replay_rust_check_passed: replay.rust_check_passed,
            evolution_replay_runs: replay.runs,
            evolution_replay_items: replay.items,
            business_contract_state: BusinessContractEvidence::from_state_body(body),
            business_contract_trace: BusinessContractEvidence::from_trace_body(body),
            business_contract_replay_ledger: BusinessContractEvidence::from_replay_ledger_body(
                body,
            ),
        }
    }
}
