mod evaluate;

use rust_norion::{ExperienceReplayReport, NoironEngine};

use crate::cli::state::ensure_runtime_state_write_window_clean;
use crate::engine_config::configure_engine;
use crate::Args;
use evaluate::evaluate_gemma_business_smoke_replay_report;

pub(super) fn run_gemma_business_smoke_replay_gate(args: &Args) -> std::io::Result<bool> {
    let replay_report = run_gemma_business_smoke_replay(args)?;
    println!("gemma_business_smoke_replay: {}", replay_report.summary());
    Ok(evaluate_gemma_business_smoke_replay_report(&replay_report))
}

pub(crate) fn run_gemma_business_smoke_replay(
    args: &Args,
) -> std::io::Result<ExperienceReplayReport> {
    ensure_runtime_state_write_window_clean(args)?;
    let mut engine = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    configure_engine(&mut engine, args);
    let report = engine.replay_experience(args.replay_limit.max(1));
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    Ok(report)
}
