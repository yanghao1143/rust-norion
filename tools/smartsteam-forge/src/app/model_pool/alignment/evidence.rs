use std::collections::BTreeSet;

use super::topology::{
    helper_line_count, missing_helper_roles, missing_smoke_tasks, role_line_count,
    roles_from_lines, unexpected_roles, unexpected_smoke_tasks,
};
use super::{MODEL_POOL_SMOKE_TASK_KINDS, RouteSmokeResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ModelPoolSmokeEvidence {
    pub(super) manifest_roles: BTreeSet<String>,
    pub(super) status_roles: BTreeSet<String>,
    pub(super) unexpected_manifest_roles: Vec<String>,
    pub(super) unexpected_status_roles: Vec<String>,
    pub(super) manifest_quality_workers: usize,
    pub(super) status_quality_workers: usize,
    pub(super) extra_quality_12b_detected: bool,
    pub(super) manifest_helper_workers: usize,
    pub(super) status_helper_workers: usize,
    pub(super) helper_target: usize,
    pub(super) helper_worker_count_aligned: bool,
    pub(super) missing_manifest_helper_roles: Vec<String>,
    pub(super) missing_status_helper_roles: Vec<String>,
    pub(super) missing_route_smoke_tasks: Vec<String>,
    pub(super) unexpected_route_smoke_tasks: Vec<String>,
    pub(super) route_smoke_count: usize,
    pub(super) route_smoke_unique_tasks: usize,
    pub(super) route_smoke_count_aligned: bool,
    pub(super) missing_status_roles: Vec<String>,
    pub(super) unplanned_status_roles: Vec<String>,
    pub(super) route_blocked_or_failed: Vec<String>,
}

impl ModelPoolSmokeEvidence {
    pub(super) fn from_summaries(
        manifest: &str,
        status: &str,
        route_results: &[RouteSmokeResult],
    ) -> Self {
        let manifest_roles = roles_from_lines(manifest, "manifest_worker ");
        let status_roles = roles_from_lines(status, "worker ");
        let unexpected_manifest_roles = unexpected_roles(&manifest_roles);
        let unexpected_status_roles = unexpected_roles(&status_roles);
        let missing_status_roles = manifest_roles
            .difference(&status_roles)
            .cloned()
            .collect::<Vec<_>>();
        let unplanned_status_roles = status_roles
            .difference(&manifest_roles)
            .cloned()
            .collect::<Vec<_>>();
        let route_blocked_or_failed = route_results
            .iter()
            .filter(|result| !result.request_ok || result.route_allowed != Some(true))
            .map(|result| result.task_kind.clone())
            .collect::<Vec<_>>();
        let route_smoke_tasks = route_results
            .iter()
            .map(|result| result.task_kind.clone())
            .collect::<BTreeSet<_>>();
        let helper_target = MODEL_POOL_SMOKE_TASK_KINDS.len();
        let missing_route_smoke_tasks = missing_smoke_tasks(&route_smoke_tasks);
        let unexpected_route_smoke_tasks = unexpected_smoke_tasks(&route_smoke_tasks);
        let route_smoke_count = route_results.len();
        let route_smoke_unique_tasks = route_smoke_tasks.len();
        let route_smoke_count_aligned =
            route_smoke_count == helper_target && route_smoke_unique_tasks == helper_target;
        let manifest_quality_workers = role_line_count(manifest, "manifest_worker ", "quality");
        let status_quality_workers = role_line_count(status, "worker ", "quality");
        let extra_quality_12b_detected = manifest_quality_workers > 1 || status_quality_workers > 1;
        let manifest_helper_workers = helper_line_count(manifest, "manifest_worker ");
        let status_helper_workers = helper_line_count(status, "worker ");
        let helper_worker_count_aligned =
            manifest_helper_workers == helper_target && status_helper_workers == helper_target;
        let missing_manifest_helper_roles = missing_helper_roles(&manifest_roles);
        let missing_status_helper_roles = missing_helper_roles(&status_roles);

        Self {
            manifest_roles,
            status_roles,
            unexpected_manifest_roles,
            unexpected_status_roles,
            manifest_quality_workers,
            status_quality_workers,
            extra_quality_12b_detected,
            manifest_helper_workers,
            status_helper_workers,
            helper_target,
            helper_worker_count_aligned,
            missing_manifest_helper_roles,
            missing_status_helper_roles,
            missing_route_smoke_tasks,
            unexpected_route_smoke_tasks,
            route_smoke_count,
            route_smoke_unique_tasks,
            route_smoke_count_aligned,
            missing_status_roles,
            unplanned_status_roles,
            route_blocked_or_failed,
        }
    }

    pub(super) fn alignment_ok(&self) -> bool {
        !self.manifest_roles.is_empty()
            && self.unexpected_manifest_roles.is_empty()
            && self.unexpected_status_roles.is_empty()
            && self.missing_status_roles.is_empty()
            && self.unplanned_status_roles.is_empty()
            && self.missing_manifest_helper_roles.is_empty()
            && self.missing_status_helper_roles.is_empty()
            && self.missing_route_smoke_tasks.is_empty()
            && self.unexpected_route_smoke_tasks.is_empty()
            && self.route_smoke_count_aligned
            && self.helper_worker_count_aligned
            && self.route_blocked_or_failed.is_empty()
            && self.manifest_quality_workers == 1
            && self.status_quality_workers == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_manifest() -> &'static str {
        concat!(
            "SmartSteam model pool manifest\n",
            "manifest_worker role=quality port=8686\n",
            "manifest_worker role=summary port=8687\n",
            "manifest_worker role=router port=8689\n",
            "manifest_worker role=review port=8688\n",
            "manifest_worker role=index port=8690\n",
            "manifest_worker role=test-gate port=8688"
        )
    }

    fn full_status() -> &'static str {
        concat!(
            "SmartSteam model pool status\n",
            "worker role=quality status=healthy\n",
            "worker role=summary status=healthy\n",
            "worker role=router status=healthy\n",
            "worker role=review status=healthy\n",
            "worker role=index status=healthy\n",
            "worker role=test-gate status=healthy"
        )
    }

    fn full_routes() -> Vec<RouteSmokeResult> {
        MODEL_POOL_SMOKE_TASK_KINDS
            .iter()
            .map(|task_kind| RouteSmokeResult {
                task_kind: (*task_kind).to_owned(),
                request_ok: true,
                route_allowed: Some(true),
            })
            .collect()
    }

    #[test]
    fn evidence_accepts_complete_quality_and_helper_topology() {
        let evidence =
            ModelPoolSmokeEvidence::from_summaries(full_manifest(), full_status(), &full_routes());

        assert!(evidence.alignment_ok());
        assert_eq!(evidence.manifest_quality_workers, 1);
        assert_eq!(evidence.status_quality_workers, 1);
        assert_eq!(evidence.manifest_helper_workers, evidence.helper_target);
        assert_eq!(evidence.status_helper_workers, evidence.helper_target);
        assert!(evidence.route_smoke_count_aligned);
        assert!(evidence.route_blocked_or_failed.is_empty());
    }

    #[test]
    fn evidence_rejects_blocked_routes_and_missing_status_roles() {
        let routes = vec![
            RouteSmokeResult {
                task_kind: "summary".to_owned(),
                request_ok: true,
                route_allowed: Some(true),
            },
            RouteSmokeResult {
                task_kind: "review".to_owned(),
                request_ok: true,
                route_allowed: Some(false),
            },
        ];
        let evidence = ModelPoolSmokeEvidence::from_summaries(
            "SmartSteam model pool manifest\nmanifest_worker role=quality port=8686\nmanifest_worker role=summary port=8687\nmanifest_worker role=review port=8688",
            "SmartSteam model pool status\nworker role=quality status=healthy\nworker role=summary status=healthy",
            &routes,
        );

        assert!(!evidence.alignment_ok());
        assert_eq!(evidence.missing_status_roles, vec!["review"]);
        assert_eq!(evidence.route_blocked_or_failed, vec!["review"]);
        assert_eq!(
            evidence.missing_route_smoke_tasks,
            vec!["router", "index", "test-gate"]
        );
    }
}
