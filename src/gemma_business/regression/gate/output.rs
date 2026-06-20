use std::path::Path;

use super::types::GemmaBusinessCycleSmokeReportGate;
use crate::gemma_business::regression::print::print_gemma_business_report_gate;

pub fn print_gemma_business_cycle_smoke_report_gate(
    path: &Path,
    report: &GemmaBusinessCycleSmokeReportGate,
) {
    print_gemma_business_report_gate(
        "gemma_business_cycle_smoke_report_gate",
        "gemma_business_cycle_smoke_report_gate_failure",
        path,
        report,
    );
}

pub fn print_gemma_business_regression_gate(
    path: &Path,
    report: &GemmaBusinessCycleSmokeReportGate,
) {
    print_gemma_business_report_gate(
        "gemma_business_regression_gate",
        "gemma_business_regression_gate_failure",
        path,
        report,
    );
}
