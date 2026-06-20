use rust_norion::NoironEngine;

use crate::Args;
use crate::gemma_business::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
use crate::gemma_business::local_service::{
    GemmaLocalService, finish_gemma_local_model_service, start_gemma_local_model_service,
};

pub(super) type ModelServiceSmokeService = GemmaLocalService;

pub(super) fn start_gemma_model_service_smoke_service(
    engine: NoironEngine,
    args: &Args,
) -> std::io::Result<ModelServiceSmokeService> {
    start_gemma_local_model_service(
        engine,
        args,
        GEMMA_MODEL_SERVICE_BUSINESS_CASES.len().saturating_mul(2) + 4,
        "Gemma model service smoke requires a command runtime",
    )
}

pub(super) fn finish_gemma_model_service_smoke_service(
    service: ModelServiceSmokeService,
    failures: &mut Vec<String>,
) -> std::io::Result<Args> {
    finish_gemma_local_model_service(
        service,
        "Gemma model service smoke server thread panicked",
        failures,
    )
}
