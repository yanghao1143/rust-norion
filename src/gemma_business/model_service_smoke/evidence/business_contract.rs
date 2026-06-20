mod replay;
mod replay_ledger;
mod state;
mod trace;

use super::field;

#[derive(Debug, Clone, Copy, Default)]
pub(in crate::gemma_business::model_service_smoke) struct BusinessContractEvidence {
    pub(in crate::gemma_business::model_service_smoke) items: u64,
    pub(in crate::gemma_business::model_service_smoke) passed: u64,
    pub(in crate::gemma_business::model_service_smoke) failed: u64,
    pub(in crate::gemma_business::model_service_smoke) missing_signals: u64,
    pub(in crate::gemma_business::model_service_smoke) protocol_leaks: u64,
    pub(in crate::gemma_business::model_service_smoke) substitutions: u64,
    pub(in crate::gemma_business::model_service_smoke) evasive_denials: u64,
    pub(in crate::gemma_business::model_service_smoke) raw_passed: u64,
    pub(in crate::gemma_business::model_service_smoke) raw_failed: u64,
    pub(in crate::gemma_business::model_service_smoke) response_normalized: u64,
    pub(in crate::gemma_business::model_service_smoke) sanitized: u64,
    pub(in crate::gemma_business::model_service_smoke) canonical_fallbacks: u64,
}

impl BusinessContractEvidence {
    pub(in crate::gemma_business::model_service_smoke) fn from_state_body(body: &str) -> Self {
        state::from_body(body)
    }

    pub(in crate::gemma_business::model_service_smoke) fn from_trace_body(body: &str) -> Self {
        trace::from_body(body)
    }

    pub(in crate::gemma_business::model_service_smoke) fn from_replay_body(body: &str) -> Self {
        replay::from_body(body)
    }

    pub(in crate::gemma_business::model_service_smoke) fn from_replay_ledger_body(
        body: &str,
    ) -> Self {
        replay_ledger::from_body(body)
    }

    pub(in crate::gemma_business::model_service_smoke) fn raw_total(self) -> u64 {
        self.raw_passed.saturating_add(self.raw_failed)
    }

    pub(in crate::gemma_business::model_service_smoke) fn normalization_counters_match(
        self,
    ) -> bool {
        self.response_normalized == self.sanitized.saturating_add(self.canonical_fallbacks)
    }
}
