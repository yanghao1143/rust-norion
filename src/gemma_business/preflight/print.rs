pub(super) fn print_gemma_business_smoke_preflight_failures_impl(failures: &[String]) {
    println!("Noiron Gemma business smoke gate");
    println!(
        "gemma_business_smoke_preflight: passed=false failures={}",
        failures.len()
    );
    for failure in failures {
        println!("gemma_business_smoke_preflight_failure: {failure}");
    }
}

pub(super) fn print_gemma_model_service_smoke_preflight_failures_impl(failures: &[String]) {
    println!("Noiron Gemma model service smoke gate");
    println!(
        "gemma_model_service_smoke_preflight: passed=false failures={}",
        failures.len()
    );
    for failure in failures {
        println!("gemma_model_service_smoke_preflight_failure: {failure}");
    }
}

pub(super) fn print_gemma_business_cycle_smoke_preflight_failures_impl(failures: &[String]) {
    println!("Noiron Gemma business-cycle smoke gate");
    println!(
        "gemma_business_cycle_smoke_preflight: passed=false failures={}",
        failures.len()
    );
    for failure in failures {
        println!("gemma_business_cycle_smoke_preflight_failure: {failure}");
    }
}
