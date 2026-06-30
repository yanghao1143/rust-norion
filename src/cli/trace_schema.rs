use std::path::Path;

use rust_norion::TraceSchemaGateReport;

pub(crate) fn print_trace_schema_gate_report(path: &Path, report: &TraceSchemaGateReport) {
    println!("Noiron trace schema gate");
    println!("trace_file: {}", path.display());
    println!("{}", report.summary_line());
    println!("{}", runtime_closed_loop_summary_line(report));
    for failure in &report.failures {
        println!("trace_schema_gate_failure: {failure}");
    }
}

fn runtime_closed_loop_summary_line(report: &TraceSchemaGateReport) -> String {
    format!(
        "runtime_closed_loop_counters adaptive_routing_events={} adaptive_routing_saved_tokens={} task_hierarchy_events={} task_hierarchy_compute_reduction_milli={} compute_budget_events={} compute_budget_avoided_tokens={} compute_budget_kv_lookups_skipped={} memory_admission_events={} memory_admission_ledger_records={} memory_admission_ledger_preview_only={} kv_fusion_events={} kv_fusion_saved_tokens={}",
        report.adaptive_routing_events,
        report.adaptive_routing_saved_tokens,
        report.task_hierarchy_events,
        report.task_hierarchy_compute_reduction_milli,
        report.compute_budget_events,
        report.compute_budget_avoided_tokens,
        report.compute_budget_kv_lookups_skipped,
        report.memory_admission_events,
        report.memory_admission_ledger_records,
        report.memory_admission_ledger_preview_only,
        report.kv_fusion_events,
        report.kv_fusion_saved_tokens
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_closed_loop_summary_line_groups_trace_gate_counters() {
        let report = TraceSchemaGateReport {
            adaptive_routing_events: 2,
            adaptive_routing_saved_tokens: 192,
            task_hierarchy_events: 1,
            task_hierarchy_compute_reduction_milli: 280,
            compute_budget_events: 3,
            compute_budget_avoided_tokens: 233,
            compute_budget_kv_lookups_skipped: 4,
            memory_admission_events: 1,
            memory_admission_ledger_records: 3,
            memory_admission_ledger_preview_only: 1,
            kv_fusion_events: 1,
            kv_fusion_saved_tokens: 100,
            ..TraceSchemaGateReport::default()
        };

        let line = runtime_closed_loop_summary_line(&report);

        assert!(line.starts_with("runtime_closed_loop_counters "));
        assert!(line.contains("adaptive_routing_events=2"));
        assert!(line.contains("compute_budget_avoided_tokens=233"));
        assert!(line.contains("compute_budget_kv_lookups_skipped=4"));
        assert!(line.contains("memory_admission_ledger_records=3"));
        assert!(line.contains("kv_fusion_saved_tokens=100"));
    }
}
