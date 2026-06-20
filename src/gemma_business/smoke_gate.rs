mod outcome;
mod replay;
mod state;
mod trace;

use rust_norion::InferenceOutcome;

use crate::{Args, option_path_display};
use outcome::run_gemma_business_smoke_outcome_gate;
#[cfg(test)]
pub(crate) use replay::run_gemma_business_smoke_replay;
use replay::run_gemma_business_smoke_replay_gate;
use state::run_gemma_business_smoke_state_gate;
use trace::run_gemma_business_smoke_trace_gate;

pub(crate) fn run_gemma_business_smoke_gates(
    args: &Args,
    outcome: &InferenceOutcome,
) -> std::io::Result<bool> {
    println!("Noiron Gemma business smoke gate");
    let mut passed = true;

    passed &= run_gemma_business_smoke_outcome_gate(outcome);
    passed &= run_gemma_business_smoke_trace_gate(args)?;
    passed &= run_gemma_business_smoke_replay_gate(args)?;
    passed &= run_gemma_business_smoke_state_gate(args)?;

    println!(
        "gemma_business_smoke_gate: passed={} trace_file={} memory_file={} experience_file={} adaptive_file={}",
        passed,
        option_path_display(args.trace_path.as_ref()),
        args.memory_path.display(),
        args.experience_path.display(),
        args.adaptive_path.display()
    );

    Ok(passed)
}
