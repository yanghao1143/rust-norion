use std::path::Path;

mod names;
mod scan;

use crate::Args;

use super::run_dir::gemma_smoke_base_dir;
use scan::gemma_smoke_generated_run_dirs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GemmaSmokeRetentionReport {
    pub(crate) before: usize,
    pub(crate) kept: usize,
    pub(crate) removed: usize,
}

pub(crate) fn prune_gemma_smoke_runs(args: &Args) -> std::io::Result<()> {
    if args.gemma_smoke_keep_runs == 0 {
        return Ok(());
    }
    let Some(base_dir) = gemma_smoke_base_dir(args) else {
        return Ok(());
    };
    let report = prune_gemma_smoke_run_dirs(Path::new(base_dir), args.gemma_smoke_keep_runs)?;
    println!(
        "gemma_smoke_retention: base={} keep_runs={} before={} kept={} removed={}",
        base_dir, args.gemma_smoke_keep_runs, report.before, report.kept, report.removed
    );
    Ok(())
}

pub(crate) fn prune_gemma_smoke_run_dirs(
    base_dir: &Path,
    keep_runs: usize,
) -> std::io::Result<GemmaSmokeRetentionReport> {
    let Some(parent) = base_dir.parent() else {
        return Ok(empty_retention_report());
    };
    let Some(base_name) = base_dir.file_name().and_then(|name| name.to_str()) else {
        return Ok(empty_retention_report());
    };
    if keep_runs == 0 || !parent.exists() {
        return Ok(empty_retention_report());
    }

    let run_dirs = gemma_smoke_generated_run_dirs(parent, base_name)?;
    let before = run_dirs.len();
    let mut removed = 0usize;
    for path in run_dirs.iter().skip(keep_runs) {
        if path.parent() == Some(parent) {
            std::fs::remove_dir_all(path)?;
            removed += 1;
        }
    }

    Ok(GemmaSmokeRetentionReport {
        before,
        kept: before.saturating_sub(removed),
        removed,
    })
}

fn empty_retention_report() -> GemmaSmokeRetentionReport {
    GemmaSmokeRetentionReport {
        before: 0,
        kept: 0,
        removed: 0,
    }
}
