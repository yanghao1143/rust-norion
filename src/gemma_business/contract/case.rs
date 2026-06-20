use crate::gemma_business::cases::gemma_model_service_business_case_by_name;
use crate::gemma_business::{GemmaModelServiceBusinessCase, gemma_business_smoke_case};

pub(super) fn business_case_by_name(
    case_name: &str,
) -> Option<&'static GemmaModelServiceBusinessCase> {
    gemma_model_service_business_case_by_name(case_name)
}

pub(super) fn gemma_business_smoke_contract_case() -> GemmaModelServiceBusinessCase {
    gemma_business_smoke_case()
}
