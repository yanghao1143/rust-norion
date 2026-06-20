mod business_contract;
mod inspect;
mod replay;

pub(in crate::gemma_business::model_service_smoke) use business_contract::BusinessContractEvidence;
pub(super) use inspect::InspectEvidence;
pub(super) use replay::ReplayEvidence;

use crate::gemma_business::response_json::response_u64_field;

fn field(body: &str, name: &str) -> u64 {
    response_u64_field(body, name)
}
