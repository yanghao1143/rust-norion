use rust_norion::evaluate_trace_schema_jsonl;

use crate::Args;
use crate::cli::trace_schema::print_trace_schema_gate_report;

pub(super) fn run_gemma_business_smoke_trace_gate(args: &Args) -> std::io::Result<bool> {
    let Some(trace_schema_gate_path) = &args.trace_schema_gate_path else {
        println!("gemma_business_smoke_trace_failure: trace schema gate path missing");
        return Ok(false);
    };
    let report = evaluate_trace_schema_jsonl(trace_schema_gate_path)?;
    print_trace_schema_gate_report(trace_schema_gate_path, &report);
    Ok(report.passed)
}
