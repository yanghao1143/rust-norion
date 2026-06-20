use std::path::PathBuf;

mod metrics;
mod paths;
mod write;

use crate::Args;
use crate::gemma_business::smoke_report::GemmaBusinessCycleCaseResult;

pub(super) use metrics::BusinessCycleSmokeMetrics;
use paths::business_cycle_smoke_artifact_paths;
use write::{write_business_cycle_report_artifact, write_business_cycle_response_artifact};

pub(super) struct BusinessCycleSmokeArtifactPaths {
    pub(super) report_path: Option<PathBuf>,
}

pub(super) struct BusinessCycleSmokeArtifacts<'a> {
    pub(super) passed: bool,
    pub(super) bind: &'a str,
    pub(super) service_args: &'a Args,
    pub(super) health_body: &'a str,
    pub(super) final_cycle_body: &'a str,
    pub(super) case_results: &'a [GemmaBusinessCycleCaseResult],
    pub(super) failures: &'a [String],
    pub(super) metrics: &'a BusinessCycleSmokeMetrics,
}

pub(super) fn write_gemma_business_cycle_smoke_artifacts(
    artifacts: BusinessCycleSmokeArtifacts<'_>,
) -> std::io::Result<BusinessCycleSmokeArtifactPaths> {
    let paths = business_cycle_smoke_artifact_paths(artifacts.service_args);

    write_business_cycle_response_artifact(paths.response_path.as_ref(), &artifacts)?;
    write_business_cycle_report_artifact(
        paths.response_path.as_ref(),
        &paths.report_path,
        &artifacts,
    )?;

    Ok(BusinessCycleSmokeArtifactPaths {
        report_path: paths.report_path,
    })
}
