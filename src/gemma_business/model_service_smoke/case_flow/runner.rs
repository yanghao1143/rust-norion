use std::io;

use super::feedback::apply_generated_feedback;
use super::generate::run_generate;
use super::record::record_model_service_case_result;
use super::rust_check::run_rust_check_feedback;
use super::types::ModelServiceCaseRun;
use crate::gemma_business::{GEMMA_MODEL_SERVICE_BUSINESS_CASES, GemmaModelServiceBusinessCase};

pub(super) fn run_model_service_business_cases(
    bind: &str,
    failures: &mut Vec<String>,
) -> io::Result<ModelServiceCaseRun> {
    let mut run = ModelServiceCaseRun::default();
    for business_case in &GEMMA_MODEL_SERVICE_BUSINESS_CASES {
        run_model_service_business_case(bind, business_case, failures, &mut run)?;
    }
    Ok(run)
}

fn run_model_service_business_case(
    bind: &str,
    business_case: &GemmaModelServiceBusinessCase,
    failures: &mut Vec<String>,
    run: &mut ModelServiceCaseRun,
) -> io::Result<()> {
    let generate = run_generate(bind, business_case, failures)?;
    let feedback_ok = apply_generated_feedback(
        bind,
        business_case,
        generate.experience_id,
        generate.feedback_memory_ids.len(),
        failures,
    )?;

    let rust_check = run_rust_check_feedback(
        bind,
        business_case,
        generate.experience_id,
        generate.feedback_memory_ids.len(),
        failures,
    )?;
    record_model_service_case_result(
        business_case,
        generate,
        feedback_ok,
        rust_check,
        failures,
        run,
    );
    Ok(())
}
