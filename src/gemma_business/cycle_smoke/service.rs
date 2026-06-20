use rust_norion::NoironEngine;

use crate::Args;
use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
use crate::gemma_business::local_service::{
    GemmaLocalService, finish_gemma_local_model_service, start_gemma_local_model_service,
};

pub(super) type BusinessCycleSmokeService = GemmaLocalService;

pub(super) fn start_gemma_business_cycle_smoke_service(
    engine: NoironEngine,
    args: &Args,
) -> std::io::Result<BusinessCycleSmokeService> {
    start_gemma_local_model_service(
        engine,
        args,
        GEMMA_MODEL_SERVICE_BUSINESS_CASES.len() + 1,
        "Gemma business-cycle smoke requires a command runtime",
    )
}

pub(super) fn finish_gemma_business_cycle_smoke_service(
    service: BusinessCycleSmokeService,
    failures: &mut Vec<String>,
) -> std::io::Result<Args> {
    finish_gemma_local_model_service(
        service,
        "Gemma business-cycle smoke server thread panicked",
        failures,
    )
}
