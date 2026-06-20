use std::fs;
use std::path::Path;

use super::body::evaluate_gemma_business_cycle_smoke_report_gate_body;
use super::types::GemmaBusinessCycleSmokeReportGate;
use crate::gemma_business::regression::artifacts::require_gemma_business_cycle_smoke_report_artifacts;

pub fn evaluate_gemma_business_cycle_smoke_report_gate(
    path: &Path,
) -> std::io::Result<GemmaBusinessCycleSmokeReportGate> {
    let body = fs::read_to_string(path)?;
    let mut report = evaluate_gemma_business_cycle_smoke_report_gate_body(&body);
    require_gemma_business_cycle_smoke_report_artifacts(path, &body, &mut report.failures);
    report.passed = report.failures.is_empty();
    Ok(report)
}

pub fn evaluate_gemma_business_regression_gate(
    path: &Path,
) -> std::io::Result<GemmaBusinessCycleSmokeReportGate> {
    evaluate_gemma_business_cycle_smoke_report_gate(path)
}
