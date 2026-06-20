mod case_flow;
mod evidence;
mod gate;
mod print;
mod requests;
mod responses;
mod service;

use rust_norion::NoironEngine;

use case_flow::run_model_service_business_cases;
use gate::{ModelServiceSmokeGateInputs, push_model_service_smoke_failures};
use print::{ModelServiceSmokeReport, print_model_service_smoke_report};
use requests::run_model_service_smoke_followup_requests;
use responses::ModelServiceSmokeResponses;
use service::{finish_gemma_model_service_smoke_service, start_gemma_model_service_smoke_service};

use crate::Args;
use crate::model_service::http::wait_for_model_service_http_response;

pub(crate) fn run_gemma_model_service_smoke(
    engine: NoironEngine,
    args: &Args,
) -> std::io::Result<bool> {
    println!("Noiron Gemma model service smoke gate");
    let service = start_gemma_model_service_smoke_service(engine, args)?;
    let bind = service.bind.clone();

    let mut failures = Vec::new();
    let health = wait_for_model_service_http_response(&bind, "GET", "/health", None)?;
    let case_run = run_model_service_business_cases(&bind, &mut failures)?;
    let followup = run_model_service_smoke_followup_requests(&bind)?;

    let service_args = finish_gemma_model_service_smoke_service(service, &mut failures)?;

    let responses = ModelServiceSmokeResponses::from_followup(&health, &followup);

    push_model_service_smoke_failures(
        ModelServiceSmokeGateInputs {
            health_body: responses.health_body,
            self_improve_body: responses.self_improve_body,
            inspect_body: responses.inspect_body,
            case_run: &case_run,
            replay: &responses.replay,
            inspect: &responses.inspect,
        },
        &mut failures,
    );

    print_model_service_smoke_report(ModelServiceSmokeReport {
        bind: &bind,
        service_args: &service_args,
        failures: &failures,
        health_body: responses.health_body,
        self_improve_body: responses.self_improve_body,
        inspect_body: responses.inspect_body,
        case_run: &case_run,
        replay: &responses.replay,
        inspect: &responses.inspect,
    });

    Ok(failures.is_empty())
}
