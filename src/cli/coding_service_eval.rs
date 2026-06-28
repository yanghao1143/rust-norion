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
    for path in coding_service_eval_trace_output_paths(args)
        .into_iter()
        .flatten()
    {
        append_coding_service_eval_readiness_trace_jsonl(path, &report)?;
    }
    Ok(report.passed())
}

pub(crate) fn run_coding_service_eval_runner_cli(args: &Args) -> io::Result<bool> {
    let report = default_coding_service_eval_runner_report();
    print_coding_service_eval_runner_report(&report);
    for path in coding_service_eval_trace_output_paths(args)
        .into_iter()
        .flatten()
    {
        append_coding_service_eval_runner_trace_jsonl(path, &report)?;
    }
    Ok(report.passed())
}

fn coding_service_eval_trace_output_paths(args: &Args) -> [Option<&Path>; 2] {
    [
        args.trace_path.as_deref(),
        args.trace_schema_gate_path
            .as_deref()
            .filter(|gate_path| args.trace_path.as_deref() != Some(*gate_path)),
    ]
}

fn print_coding_service_eval_readiness_report(report: &CodingServiceEvalReadinessReport) {
    println!("Noiron coding service eval readiness");
    println!("{}", report.summary_line());
    println!(
        "coding_service_eval_benchmark_feed kind=readiness requests={} completed=0 rust_validation_checked=0 compile_checked=0 unit_test_checked=0 evidence_packets={} read_only={} write_allowed={} applied={}",
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
        "coding_service_eval_benchmark_feed kind=runner requests={} completed={} rust_validation_checked={} compile_checked={} unit_test_checked={} evidence_packets={} read_only={} write_allowed={} applied={}",
        report.plan_count,
        report.completed_count,
        report.rust_validation_checked_count,
        compile_checked,
        unit_test_checked,
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
