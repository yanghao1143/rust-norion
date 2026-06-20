mod artifacts;
mod case_flow;
mod evidence;
mod health;
mod print;
mod service;

use rust_norion::NoironEngine;

use super::GEMMA_MODEL_SERVICE_BUSINESS_CASES;
use crate::Args;
use artifacts::{BusinessCycleSmokeArtifacts, write_gemma_business_cycle_smoke_artifacts};
use case_flow::run_business_cycle_cases;
use evidence::BusinessCycleSmokeEvidence;
use health::{fetch_business_cycle_health, require_business_cycle_health_ok};
use print::{BusinessCycleSmokePrintReport, print_gemma_business_cycle_smoke_report};
use service::{
    finish_gemma_business_cycle_smoke_service, start_gemma_business_cycle_smoke_service,
};

pub(crate) fn run_gemma_business_cycle_smoke(
    engine: NoironEngine,
    args: &Args,
) -> std::io::Result<bool> {
    println!("Noiron Gemma business-cycle smoke gate");
    let service = start_gemma_business_cycle_smoke_service(engine, args)?;
    let bind = service.bind.clone();

    let mut failures = Vec::new();
    let health = fetch_business_cycle_health(&bind, &mut failures);
    let case_results = run_business_cycle_cases(&bind, &mut failures);

    let service_args = finish_gemma_business_cycle_smoke_service(service, &mut failures)?;

    let evidence = BusinessCycleSmokeEvidence::from_run(&health, &case_results);
    require_business_cycle_health_ok(evidence.health_body, &mut failures);
    let artifacts = write_gemma_business_cycle_smoke_artifacts(BusinessCycleSmokeArtifacts {
        passed: failures.is_empty(),
        bind: &bind,
        service_args: &service_args,
        health_body: evidence.health_body,
        final_cycle_body: evidence.final_cycle_body,
        case_results: &case_results,
        failures: &failures,
        metrics: &evidence.metrics,
    })?;

    print_gemma_business_cycle_smoke_report(BusinessCycleSmokePrintReport {
        passed: failures.is_empty(),
        bind: &bind,
        health_body: evidence.health_body,
        case_results: &case_results,
        failures: &failures,
        service_args: &service_args,
        report_path: artifacts.report_path.as_ref(),
        runtime_token_count: evidence.metrics.runtime_token_count,
        feedback_applied: evidence.metrics.feedback_applied,
        rust_check_feedback_applied: evidence.metrics.rust_check_feedback_applied,
        checked_trace_lines: evidence.metrics.checked_trace_lines,
        passed_cases: evidence.metrics.passed_cases,
        expected_case_count: GEMMA_MODEL_SERVICE_BUSINESS_CASES.len(),
        final_cycle_body: evidence.final_cycle_body,
    });

    Ok(failures.is_empty())
}
