use std::collections::BTreeSet;

use model_pool_advice_core::HELPER_ROLES as MODEL_POOL_SMOKE_TASK_KINDS;

mod evidence;
mod projection;
mod render;
#[cfg(test)]
mod tests;
mod topology;

use evidence::ModelPoolSmokeEvidence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteSmokeResult {
    pub(crate) task_kind: String,
    pub(crate) request_ok: bool,
    pub(crate) route_allowed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelPoolSmokeAlignment {
    alignment_ok: bool,
    manifest_roles: BTreeSet<String>,
    status_roles: BTreeSet<String>,
    unexpected_manifest_roles: Vec<String>,
    unexpected_status_roles: Vec<String>,
    pub(crate) manifest_quality_workers: usize,
    pub(crate) status_quality_workers: usize,
    extra_quality_12b_detected: bool,
    pub(crate) manifest_helper_workers: usize,
    pub(crate) status_helper_workers: usize,
    helper_target: usize,
    helper_worker_count_aligned: bool,
    pub(crate) missing_manifest_helper_roles: Vec<String>,
    pub(crate) missing_status_helper_roles: Vec<String>,
    missing_route_smoke_tasks: Vec<String>,
    unexpected_route_smoke_tasks: Vec<String>,
    pub(crate) route_smoke_count: usize,
    pub(crate) route_smoke_unique_tasks: usize,
    route_smoke_target: usize,
    route_smoke_count_aligned: bool,
    missing_status_roles: Vec<String>,
    unplanned_status_roles: Vec<String>,
    pub(crate) route_blocked_or_failed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManifestStatusAlignmentSummary {
    pub(crate) manifest_roles: Vec<String>,
    pub(crate) status_roles: Vec<String>,
    pub(crate) unexpected_manifest_roles: Vec<String>,
    pub(crate) unexpected_status_roles: Vec<String>,
    pub(crate) manifest_quality_workers: usize,
    pub(crate) status_quality_workers: usize,
    pub(crate) extra_quality_12b_detected: bool,
    pub(crate) manifest_helper_workers: usize,
    pub(crate) status_helper_workers: usize,
    pub(crate) helper_target: usize,
    pub(crate) helper_worker_count_aligned: bool,
    pub(crate) missing_manifest_helper_roles: Vec<String>,
    pub(crate) missing_status_helper_roles: Vec<String>,
    pub(crate) missing_status_roles: Vec<String>,
    pub(crate) unplanned_status_roles: Vec<String>,
}

impl ModelPoolSmokeAlignment {
    pub(crate) fn from_summaries(
        manifest: &str,
        status: &str,
        route_results: &[RouteSmokeResult],
    ) -> Self {
        let evidence = ModelPoolSmokeEvidence::from_summaries(manifest, status, route_results);
        let alignment_ok = evidence.alignment_ok();

        Self {
            alignment_ok,
            manifest_roles: evidence.manifest_roles,
            status_roles: evidence.status_roles,
            unexpected_manifest_roles: evidence.unexpected_manifest_roles,
            unexpected_status_roles: evidence.unexpected_status_roles,
            manifest_quality_workers: evidence.manifest_quality_workers,
            status_quality_workers: evidence.status_quality_workers,
            extra_quality_12b_detected: evidence.extra_quality_12b_detected,
            manifest_helper_workers: evidence.manifest_helper_workers,
            status_helper_workers: evidence.status_helper_workers,
            helper_target: evidence.helper_target,
            helper_worker_count_aligned: evidence.helper_worker_count_aligned,
            missing_manifest_helper_roles: evidence.missing_manifest_helper_roles,
            missing_status_helper_roles: evidence.missing_status_helper_roles,
            missing_route_smoke_tasks: evidence.missing_route_smoke_tasks,
            unexpected_route_smoke_tasks: evidence.unexpected_route_smoke_tasks,
            route_smoke_count: evidence.route_smoke_count,
            route_smoke_unique_tasks: evidence.route_smoke_unique_tasks,
            route_smoke_target: evidence.helper_target,
            route_smoke_count_aligned: evidence.route_smoke_count_aligned,
            missing_status_roles: evidence.missing_status_roles,
            unplanned_status_roles: evidence.unplanned_status_roles,
            route_blocked_or_failed: evidence.route_blocked_or_failed,
        }
    }
}

pub(crate) fn manifest_status_alignment_summary(
    manifest: &str,
    status: &str,
) -> ManifestStatusAlignmentSummary {
    let evidence = ModelPoolSmokeEvidence::from_summaries(manifest, status, &[]);

    ManifestStatusAlignmentSummary {
        manifest_roles: evidence.manifest_roles.into_iter().collect(),
        status_roles: evidence.status_roles.into_iter().collect(),
        unexpected_manifest_roles: evidence.unexpected_manifest_roles,
        unexpected_status_roles: evidence.unexpected_status_roles,
        manifest_quality_workers: evidence.manifest_quality_workers,
        status_quality_workers: evidence.status_quality_workers,
        extra_quality_12b_detected: evidence.extra_quality_12b_detected,
        manifest_helper_workers: evidence.manifest_helper_workers,
        status_helper_workers: evidence.status_helper_workers,
        helper_target: evidence.helper_target,
        helper_worker_count_aligned: evidence.helper_worker_count_aligned,
        missing_manifest_helper_roles: evidence.missing_manifest_helper_roles,
        missing_status_helper_roles: evidence.missing_status_helper_roles,
        missing_status_roles: evidence.missing_status_roles,
        unplanned_status_roles: evidence.unplanned_status_roles,
    }
}

#[cfg(test)]
pub(crate) fn model_pool_smoke_alignment(
    manifest: &str,
    status: &str,
    route_results: &[RouteSmokeResult],
) -> String {
    ModelPoolSmokeAlignment::from_summaries(manifest, status, route_results).to_text()
}
