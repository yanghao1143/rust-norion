mod check_only;
mod checks;
mod print;
mod snapshot;

use crate::Args;
use check_only::{gemma_smoke_check_only_report, print_gemma_smoke_check_only_report};
use checks::gemma_business_smoke_preflight_failures_impl;
use print::{
    print_gemma_business_cycle_smoke_preflight_failures_impl,
    print_gemma_business_smoke_preflight_failures_impl,
    print_gemma_model_service_smoke_preflight_failures_impl,
};

pub(crate) fn gemma_business_smoke_preflight_failures(args: &Args) -> Vec<String> {
    gemma_business_smoke_preflight_failures_impl(args)
}

pub(crate) use check_only::GemmaSmokeCheckOnlyReport;

pub(crate) fn gemma_business_smoke_check_only_report(
    args: &Args,
    preflight_failures: &[String],
) -> GemmaSmokeCheckOnlyReport {
    gemma_smoke_check_only_report(args, preflight_failures)
}

pub(crate) fn print_gemma_business_smoke_check_only_report(report: &GemmaSmokeCheckOnlyReport) {
    print_gemma_smoke_check_only_report(report);
}

pub(crate) fn print_gemma_business_smoke_preflight_failures(failures: &[String]) {
    print_gemma_business_smoke_preflight_failures_impl(failures);
}

pub(crate) fn print_gemma_model_service_smoke_preflight_failures(failures: &[String]) {
    print_gemma_model_service_smoke_preflight_failures_impl(failures);
}

pub(crate) fn print_gemma_business_cycle_smoke_preflight_failures(failures: &[String]) {
    print_gemma_business_cycle_smoke_preflight_failures_impl(failures);
}
