mod print;

use crate::Args;
use crate::cli::state::{print_state_inspection_gate_report, run_state_inspection};
use crate::gemma_business::state_gate::gemma_business_smoke_state_gate;

use print::print_gemma_business_smoke_state_summary;

pub(super) fn run_gemma_business_smoke_state_gate(args: &Args) -> std::io::Result<bool> {
    let inspection = run_state_inspection(args)?;
    print_gemma_business_smoke_state_summary(&inspection);
    let gate = gemma_business_smoke_state_gate(args);
    let gate_report = inspection.evaluate(&gate);
    print_state_inspection_gate_report(&gate_report);
    Ok(gate_report.passed())
}
