use rust_norion::NoironEngine;

use super::note::model_service_business_contract_note;
use crate::gemma_business::audit::{
    GemmaModelServiceAnswerAudit, GemmaModelServiceBusinessNormalization,
};

pub(super) fn annotate_model_service_business_contract_experience(
    engine: &mut NoironEngine,
    experience_id: u64,
    case_name: &str,
    audit: &GemmaModelServiceAnswerAudit,
    normalization: &GemmaModelServiceBusinessNormalization,
) -> bool {
    let Some(record) = engine.experience.record_mut(experience_id) else {
        return false;
    };
    record.process_reward.notes.insert(
        0,
        model_service_business_contract_note(case_name, audit, normalization),
    );
    true
}
