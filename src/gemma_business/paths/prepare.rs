use crate::Args;
use crate::path_utils::ensure_parent_dir;

pub(crate) fn prepare_gemma_business_smoke_paths(args: &Args) -> std::io::Result<()> {
    ensure_parent_dir(&args.memory_path)?;
    ensure_parent_dir(&args.experience_path)?;
    ensure_parent_dir(&args.adaptive_path)?;
    if let Some(trace_path) = &args.trace_path {
        ensure_parent_dir(trace_path)?;
    }
    if let Some(trace_schema_gate_path) = &args.trace_schema_gate_path {
        ensure_parent_dir(trace_schema_gate_path)?;
    }
    Ok(())
}
