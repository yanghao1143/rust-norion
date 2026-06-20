mod checks;
mod feedback;
mod generate;
mod record;
mod runner;
mod rust_check;
mod types;

use std::io;

pub(super) use types::ModelServiceCaseRun;

pub(super) fn run_model_service_business_cases(
    bind: &str,
    failures: &mut Vec<String>,
) -> io::Result<ModelServiceCaseRun> {
    runner::run_model_service_business_cases(bind, failures)
}
