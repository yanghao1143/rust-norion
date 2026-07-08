use std::io;
use std::path::Path;

use rust_norion::{
    CodingServiceEvalReadinessReport, CodingServiceEvalRunnerReport,
    append_coding_service_eval_readiness_trace_jsonl,
    append_coding_service_eval_runner_trace_jsonl, default_coding_service_eval_readiness_report,
    default_coding_service_eval_runner_report,
};

use crate::Args;

pub(crate) fn run_coding_service_eval_readiness_cli(args: &Args) -> io::Result<bool> {
    let report = default_coding_service_eval_readiness_report();
    print_coding_service_eval_readiness_report(&report);
    if let Some(path) = coding_service_eval_trace_output_path(args) {
        append_coding_service_eval_readiness_trace_jsonl(path, &report)?;
    }
    Ok(report.passed())
}

pub(crate) fn run_coding_service_eval_runner_cli(args: &Args) -> io::Result<bool> {
    let report = default_coding_service_eval_runner_report();
    print_coding_service_eval_runner_report(&report);
    if let Some(path) = coding_service_eval_trace_output_path(args) {
        append_coding_service_eval_runner_trace_jsonl(path, &report)?;
    }
    Ok(report.passed())
}

fn coding_service_eval_trace_output_path(args: &Args) -> Option<&Path> {
    args.trace_path
        .as_deref()
        .or(args.trace_schema_gate_path.as_deref())
}

fn print_coding_service_eval_readiness_report(report: &CodingServiceEvalReadinessReport) {
    println!("Noiron coding service eval readiness");
    println!("{}", report.summary_line());
    println!(
        "coding_service_eval_benchmark_feed kind=readiness requests={} completed=0 rust_validation_checked=0 compile_checked=0 unit_test_checked=0 benchmark_checked=0 benchmark_passed=0 evidence_packets={} read_only={} write_allowed={} applied={}",
        report.request_plan_count,
        report.request_evidence_packets.len(),
        report.read_only,
        report.write_allowed,
        report.applied
    );
    for failure in &report.corpus_validation_failures {
        println!("coding_service_eval_failure: {failure}");
    }
    for capability in &report.missing_capabilities {
        println!("coding_service_eval_missing_capability: {capability}");
    }
}

fn print_coding_service_eval_runner_report(report: &CodingServiceEvalRunnerReport) {
    let compile_checked = report
        .run_records
        .iter()
        .filter(|record| record.compile_checked)
        .count();
    let unit_test_checked = report
        .run_records
        .iter()
        .filter(|record| record.unit_test_checked)
        .count();

    println!("Noiron coding service eval runner");
    println!("{}", report.summary_line());
    println!(
        "coding_service_eval_benchmark_feed kind=runner requests={} completed={} rust_validation_checked={} compile_checked={} unit_test_checked={} benchmark_checked={} benchmark_passed={} evidence_packets={} read_only={} write_allowed={} applied={}",
        report.plan_count,
        report.completed_count,
        report.rust_validation_checked_count,
        compile_checked,
        unit_test_checked,
        report.benchmark_checked_count,
        report.benchmark_passed_count,
        report.evidence_packets.len(),
        report.read_only,
        report.write_allowed,
        report.applied
    );
    for record in report
        .run_records
        .iter()
        .filter(|record| !record.passed_runner_contract())
    {
        println!(
            "coding_service_eval_runner_failure: fixture={} profile={} state={} evidence_redacted={}",
            record.fixture_id,
            record.profile.as_str(),
            record.final_state_label,
            record.evidence_is_redacted()
        );
    }
}
