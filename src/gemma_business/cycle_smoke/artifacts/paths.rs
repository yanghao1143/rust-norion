use std::path::PathBuf;

use crate::Args;

use crate::gemma_business::GEMMA_BUSINESS_CYCLE_SMOKE_REPORT_FILE;

pub(super) struct BusinessCycleSmokeArtifactPathSet {
    pub(super) response_path: Option<PathBuf>,
    pub(super) report_path: Option<PathBuf>,
}

pub(super) fn business_cycle_smoke_artifact_paths(
    service_args: &Args,
) -> BusinessCycleSmokeArtifactPathSet {
    let run_dir = service_args
        .trace_path
        .as_ref()
        .and_then(|trace_path| trace_path.parent().map(|path| path.to_path_buf()));
    BusinessCycleSmokeArtifactPathSet {
        response_path: run_dir
            .as_ref()
            .map(|run_dir| run_dir.join("business-cycle.response.json")),
        report_path: run_dir
            .as_ref()
            .map(|run_dir| run_dir.join(GEMMA_BUSINESS_CYCLE_SMOKE_REPORT_FILE)),
    }
}
