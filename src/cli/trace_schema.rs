use std::path::Path;

use rust_norion::TraceSchemaGateReport;

pub(crate) fn print_trace_schema_gate_report(path: &Path, report: &TraceSchemaGateReport) {
    println!("Noiron trace schema gate");
    println!("trace_file: {}", path.display());
    println!("{}", report.summary_line());
    for failure in &report.failures {
        println!("trace_schema_gate_failure: {failure}");
    }
}
