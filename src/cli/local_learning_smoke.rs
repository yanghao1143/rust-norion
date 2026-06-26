use std::io;
use std::path::PathBuf;

use rust_norion::{
    LocalTransformerRuntime, NoironEngine, RuntimeBackend, TraceSchemaGateReport,
    evaluate_trace_schema_jsonl,
};

use crate::Args;
use crate::cli::trace_schema::print_trace_schema_gate_report;
use crate::engine_config::configure_engine;
use crate::inference_runner::run_timed_inference_with_options;
use crate::model_service::types::TimedOutcome;

const LOCAL_LEARNING_SMOKE_CASE: &str = "local_learning_smoke";
const DEFAULT_LOCAL_LEARNING_SMOKE_MAX_TOKENS: usize = 32;

pub(crate) fn run_local_learning_smoke_cli(args: &Args) -> io::Result<bool> {
    let mut engine = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    configure_engine(&mut engine, args);

    let runtime = LocalTransformerRuntime::with_manifest(args.runtime_manifest());
    let max_tokens = args
        .max_tokens
        .unwrap_or(DEFAULT_LOCAL_LEARNING_SMOKE_MAX_TOKENS);
    let mut backend = RuntimeBackend::new(runtime).with_max_tokens(max_tokens);
    let trace_output_path = local_learning_smoke_trace_output_path(args);
    let timed = run_timed_inference_with_options(
        &mut engine,
        &mut backend,
        args.prompt.clone(),
        args.profile,
        Some(max_tokens),
        trace_output_path,
        Some(LOCAL_LEARNING_SMOKE_CASE),
    )?;

    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;

    let trace_gate_report = if let Some(path) = &args.trace_schema_gate_path {
        let report = evaluate_trace_schema_jsonl(path)?;
        print_trace_schema_gate_report(path, &report);
        Some(report)
    } else {
        None
    };
    let passed = local_learning_smoke_passed(&timed, trace_gate_report.as_ref());
    print_local_learning_smoke_summary(args, &timed, trace_gate_report.as_ref(), passed);
    Ok(passed)
}

fn local_learning_smoke_trace_output_path(args: &Args) -> Option<&PathBuf> {
    args.trace_path
        .as_ref()
        .or(args.trace_schema_gate_path.as_ref())
}

fn local_learning_smoke_passed(
    timed: &TimedOutcome,
    trace_gate_report: Option<&TraceSchemaGateReport>,
) -> bool {
    timed.outcome.runtime_token_metrics.token_count > 0
        && timed.outcome.experience_id > 0
        && trace_gate_report
            .map(|report| report.passed && report.checked_lines > 0)
            .unwrap_or(true)
}

fn print_local_learning_smoke_summary(
    args: &Args,
    timed: &TimedOutcome,
    trace_gate_report: Option<&TraceSchemaGateReport>,
    passed: bool,
) {
    let outcome = &timed.outcome;
    println!("Noiron local learning smoke");
    println!(
        "local_learning_smoke: passed={} elapsed_ms={} trace_gate_passed={}",
        passed,
        timed.elapsed_ms,
        trace_gate_report
            .map(|report| report.passed)
            .unwrap_or(true)
    );
    println!("memory_file: {}", args.memory_path.display());
    println!("experience_file: {}", args.experience_path.display());
    println!("adaptive_file: {}", args.adaptive_path.display());
    if let Some(path) = local_learning_smoke_trace_output_path(args) {
        println!("trace_file: {}", path.display());
    }
    println!(
        "runtime: tokens={} exported_kv={} stored_runtime_kv={}",
        outcome.runtime_token_metrics.token_count,
        outcome.exported_runtime_kv_blocks,
        outcome.stored_runtime_kv_memory_ids.len()
    );
    println!(
        "memory: stored={} gist_stored={} experience_id={} used_memories={} used_experiences={}",
        outcome.stored_memory_id.is_some(),
        outcome.stored_gist_memory_ids.len(),
        outcome.experience_id,
        outcome.used_memories.len(),
        outcome.used_experiences.len()
    );
    println!(
        "live_evolution: memory_updates={} stored_updates={} reflection_issues={} revision_actions={}",
        outcome.live_evolution.memory_updates(),
        outcome.live_evolution.stored_memory_updates(),
        outcome.live_evolution.reflection_issues,
        outcome.live_evolution.revision_actions
    );
    println!(
        "evolution_ledger: live_runs={} live_memory_updates={} live_stored_runtime_kv={} replay_runs={}",
        outcome.evolution_ledger.live_inference_runs,
        outcome.evolution_ledger.live_memory_updates(),
        outcome.evolution_ledger.live_stored_runtime_kv_memories,
        outcome.evolution_ledger.replay_runs
    );
}
