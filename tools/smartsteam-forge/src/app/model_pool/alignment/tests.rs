use super::*;

#[test]
fn reports_manifest_status_and_route_mismatches() {
    let manifest = "SmartSteam model pool manifest\nmanifest_worker role=quality port=8686\nmanifest_worker role=summary port=8687\nmanifest_worker role=review port=8688";
    let status = "SmartSteam model pool status\nworker role=quality status=healthy\nworker role=summary status=healthy\nworker role=extra status=healthy";
    let route_results = vec![
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

    let alignment = model_pool_smoke_alignment(manifest, status, &route_results);

    assert!(alignment.contains("alignment_ok=false"));
    assert!(alignment.contains("manifest_roles=quality,review,summary"));
    assert!(alignment.contains("status_roles=extra,quality,summary"));
    assert!(alignment.contains("unexpected_manifest_roles=none"));
    assert!(alignment.contains("unexpected_status_roles=extra"));
    assert!(alignment.contains("extra_quality_12b_detected=false"));
    assert!(alignment.contains("helper_worker_count_aligned=false"));
    assert!(alignment.contains("missing_manifest_helper_roles=router,index,test-gate"));
    assert!(alignment.contains("missing_status_helper_roles=router,review,index,test-gate"));
    assert!(alignment.contains("missing_route_smoke_tasks=router,index,test-gate"));
    assert!(alignment.contains("unexpected_route_smoke_tasks=none"));
    assert!(alignment.contains("missing_status_roles=review"));
    assert!(alignment.contains("unplanned_status_roles=extra"));
    assert!(alignment.contains("route_blocked_or_failed=review"));
}

#[test]
fn requires_full_helper_roles_even_when_routes_pass() {
    let manifest = "SmartSteam model pool manifest\nmanifest_worker role=quality port=8686\nmanifest_worker role=summary port=8687";
    let status = "SmartSteam model pool status\nworker role=quality status=healthy\nworker role=summary status=healthy";
    let route_results = MODEL_POOL_SMOKE_TASK_KINDS
        .iter()
        .map(|task_kind| RouteSmokeResult {
            task_kind: (*task_kind).to_owned(),
            request_ok: true,
            route_allowed: Some(true),
        })
        .collect::<Vec<_>>();

    let alignment = model_pool_smoke_alignment(manifest, status, &route_results);

    assert!(alignment.contains("alignment_ok=false"));
    assert!(alignment.contains("unexpected_manifest_roles=none"));
    assert!(alignment.contains("unexpected_status_roles=none"));
    assert!(alignment.contains("manifest_quality_workers=1 status_quality_workers=1"));
    assert!(alignment.contains("route_blocked_or_failed=none"));
    assert!(alignment.contains("helper_worker_count_aligned=false"));
    assert!(alignment.contains("missing_route_smoke_tasks=none"));
    assert!(alignment.contains("unexpected_route_smoke_tasks=none"));
    assert!(alignment.contains("missing_manifest_helper_roles=router,review,index,test-gate"));
    assert!(alignment.contains("missing_status_helper_roles=router,review,index,test-gate"));
}

#[test]
fn requires_all_route_smoke_tasks() {
    let manifest = concat!(
        "SmartSteam model pool manifest\n",
        "manifest_worker role=quality port=8686\n",
        "manifest_worker role=summary port=8687\n",
        "manifest_worker role=router port=8689\n",
        "manifest_worker role=review port=8688\n",
        "manifest_worker role=index port=8690\n",
        "manifest_worker role=test-gate port=8688"
    );
    let status = concat!(
        "SmartSteam model pool status\n",
        "worker role=quality status=healthy\n",
        "worker role=summary status=healthy\n",
        "worker role=router status=healthy\n",
        "worker role=review status=healthy\n",
        "worker role=index status=healthy\n",
        "worker role=test-gate status=healthy"
    );
    let route_results = vec![RouteSmokeResult {
        task_kind: "summary".to_owned(),
        request_ok: true,
        route_allowed: Some(true),
    }];

    let alignment = model_pool_smoke_alignment(manifest, status, &route_results);

    assert!(alignment.contains("alignment_ok=false"));
    assert!(alignment.contains("unexpected_manifest_roles=none"));
    assert!(alignment.contains("unexpected_status_roles=none"));
    assert!(alignment.contains("missing_manifest_helper_roles=none"));
    assert!(alignment.contains("missing_status_helper_roles=none"));
    assert!(alignment.contains("missing_route_smoke_tasks=router,review,index,test-gate"));
    assert!(alignment.contains("unexpected_route_smoke_tasks=none"));
    assert!(alignment.contains("route_blocked_or_failed=none"));
}

#[test]
fn rejects_duplicate_helper_workers() {
    let manifest = concat!(
        "SmartSteam model pool manifest\n",
        "manifest_worker role=quality port=8686\n",
        "manifest_worker role=summary port=8687\n",
        "manifest_worker role=summary port=8697\n",
        "manifest_worker role=router port=8689\n",
        "manifest_worker role=review port=8688\n",
        "manifest_worker role=index port=8690\n",
        "manifest_worker role=test-gate port=8688"
    );
    let status = concat!(
        "SmartSteam model pool status\n",
        "worker role=quality status=healthy\n",
        "worker role=summary status=healthy\n",
        "worker role=summary status=healthy\n",
        "worker role=router status=healthy\n",
        "worker role=review status=healthy\n",
        "worker role=index status=healthy\n",
        "worker role=test-gate status=healthy"
    );
    let route_results = MODEL_POOL_SMOKE_TASK_KINDS
        .iter()
        .map(|task_kind| RouteSmokeResult {
            task_kind: (*task_kind).to_owned(),
            request_ok: true,
            route_allowed: Some(true),
        })
        .collect::<Vec<_>>();

    let alignment = model_pool_smoke_alignment(manifest, status, &route_results);

    assert!(alignment.contains("alignment_ok=false"));
    assert!(alignment.contains("unexpected_manifest_roles=none"));
    assert!(alignment.contains("unexpected_status_roles=none"));
    assert!(alignment.contains("manifest_helper_workers=6 status_helper_workers=6"));
    assert!(alignment.contains("helper_worker_count_aligned=false"));
    assert!(alignment.contains("missing_manifest_helper_roles=none"));
    assert!(alignment.contains("missing_status_helper_roles=none"));
    assert!(alignment.contains("missing_route_smoke_tasks=none"));
    assert!(alignment.contains("unexpected_route_smoke_tasks=none"));
    assert!(alignment.contains("route_blocked_or_failed=none"));
}

#[test]
fn rejects_unknown_roles_even_when_manifest_and_status_match() {
    let manifest = concat!(
        "SmartSteam model pool manifest\n",
        "manifest_worker role=quality port=8686\n",
        "manifest_worker role=summary port=8687\n",
        "manifest_worker role=router port=8689\n",
        "manifest_worker role=review port=8688\n",
        "manifest_worker role=index port=8690\n",
        "manifest_worker role=test-gate port=8688\n",
        "manifest_worker role=explore port=8691"
    );
    let status = concat!(
        "SmartSteam model pool status\n",
        "worker role=quality status=healthy\n",
        "worker role=summary status=healthy\n",
        "worker role=router status=healthy\n",
        "worker role=review status=healthy\n",
        "worker role=index status=healthy\n",
        "worker role=test-gate status=healthy\n",
        "worker role=explore status=healthy"
    );
    let route_results = MODEL_POOL_SMOKE_TASK_KINDS
        .iter()
        .map(|task_kind| RouteSmokeResult {
            task_kind: (*task_kind).to_owned(),
            request_ok: true,
            route_allowed: Some(true),
        })
        .collect::<Vec<_>>();

    let alignment = model_pool_smoke_alignment(manifest, status, &route_results);

    assert!(alignment.contains("alignment_ok=false"));
    assert!(alignment.contains("unexpected_manifest_roles=explore"));
    assert!(alignment.contains("unexpected_status_roles=explore"));
    assert!(alignment.contains("missing_status_roles=none"));
    assert!(alignment.contains("unplanned_status_roles=none"));
    assert!(alignment.contains("missing_route_smoke_tasks=none"));
    assert!(alignment.contains("unexpected_route_smoke_tasks=none"));
    assert!(alignment.contains("route_blocked_or_failed=none"));
}

#[test]
fn rejects_unexpected_route_smoke_tasks() {
    let manifest = full_helper_manifest();
    let status = full_helper_status();
    let mut route_results = full_allowed_route_results();
    route_results.push(RouteSmokeResult {
        task_kind: "explore".to_owned(),
        request_ok: true,
        route_allowed: Some(true),
    });

    let alignment = model_pool_smoke_alignment(manifest, status, &route_results);

    assert!(alignment.contains("alignment_ok=false"));
    assert!(alignment.contains("unexpected_manifest_roles=none"));
    assert!(alignment.contains("unexpected_status_roles=none"));
    assert!(alignment.contains("missing_manifest_helper_roles=none"));
    assert!(alignment.contains("missing_status_helper_roles=none"));
    assert!(alignment.contains("missing_route_smoke_tasks=none"));
    assert!(alignment.contains("unexpected_route_smoke_tasks=explore"));
    assert!(alignment.contains(
        "route_smoke_count=6 route_smoke_unique_tasks=6 route_smoke_target=5 route_smoke_count_aligned=false"
    ));
    assert!(alignment.contains("route_blocked_or_failed=none"));
}

#[test]
fn rejects_duplicate_route_smoke_tasks() {
    let manifest = full_helper_manifest();
    let status = full_helper_status();
    let mut route_results = full_allowed_route_results();
    route_results.push(RouteSmokeResult {
        task_kind: "summary".to_owned(),
        request_ok: true,
        route_allowed: Some(true),
    });

    let alignment = model_pool_smoke_alignment(manifest, status, &route_results);

    assert!(alignment.contains("alignment_ok=false"));
    assert!(alignment.contains("missing_route_smoke_tasks=none"));
    assert!(alignment.contains("unexpected_route_smoke_tasks=none"));
    assert!(alignment.contains(
        "route_smoke_count=6 route_smoke_unique_tasks=5 route_smoke_target=5 route_smoke_count_aligned=false"
    ));
    assert!(alignment.contains("route_blocked_or_failed=none"));
}

#[test]
fn accepts_complete_one_quality_plus_helpers_topology() {
    let alignment = model_pool_smoke_alignment(
        full_helper_manifest(),
        full_helper_status(),
        &full_allowed_route_results(),
    );

    assert!(alignment.contains("alignment_ok=true"));
    assert!(alignment.contains("manifest_roles=index,quality,review,router,summary,test-gate"));
    assert!(alignment.contains("status_roles=index,quality,review,router,summary,test-gate"));
    assert!(alignment.contains("unexpected_manifest_roles=none"));
    assert!(alignment.contains("unexpected_status_roles=none"));
    assert!(alignment.contains("manifest_quality_workers=1 status_quality_workers=1"));
    assert!(alignment.contains("extra_quality_12b_detected=false"));
    assert!(alignment.contains("manifest_helper_workers=5 status_helper_workers=5"));
    assert!(alignment.contains("helper_worker_count_aligned=true"));
    assert!(alignment.contains("missing_manifest_helper_roles=none"));
    assert!(alignment.contains("missing_status_helper_roles=none"));
    assert!(alignment.contains("missing_route_smoke_tasks=none"));
    assert!(alignment.contains("unexpected_route_smoke_tasks=none"));
    assert!(alignment.contains(
        "route_smoke_count=5 route_smoke_unique_tasks=5 route_smoke_target=5 route_smoke_count_aligned=true"
    ));
    assert!(alignment.contains("missing_status_roles=none"));
    assert!(alignment.contains("unplanned_status_roles=none"));
    assert!(alignment.contains("route_blocked_or_failed=none"));
}

#[test]
fn exposes_machine_readable_fields() {
    let alignment = ModelPoolSmokeAlignment::from_summaries(
        full_helper_manifest(),
        full_helper_status(),
        &full_allowed_route_results(),
    );

    assert!(alignment.alignment_ok());
    assert_eq!(alignment.manifest_quality_workers, 1);
    assert_eq!(alignment.status_quality_workers, 1);
    assert_eq!(alignment.manifest_helper_workers, 5);
    assert_eq!(alignment.status_helper_workers, 5);
    assert_eq!(alignment.route_smoke_count, 5);
    assert_eq!(alignment.route_smoke_unique_tasks, 5);
    assert!(alignment.missing_manifest_helper_roles.is_empty());
    assert!(alignment.missing_status_helper_roles.is_empty());
    assert!(alignment.route_blocked_or_failed.is_empty());
    assert!(alignment.to_text().contains("alignment_ok=true"));
    assert!(
        alignment
            .to_json()
            .contains("\"schema\":\"smartsteam.forge.model_pool_smoke_alignment.v1\"")
    );
    assert!(alignment.to_json().contains("\"alignment_ok\":true"));
    assert!(alignment.to_json().contains(
        "\"manifest_roles\":[\"index\",\"quality\",\"review\",\"router\",\"summary\",\"test-gate\"]"
    ));
    assert!(
        alignment
            .to_json()
            .contains("\"manifest_quality_workers\":1")
    );
    assert!(
        alignment
            .to_json()
            .contains("\"manifest_helper_workers\":5")
    );
    assert!(
        alignment
            .to_json()
            .contains("\"route_smoke_count_aligned\":true")
    );
    assert!(
        alignment
            .to_json()
            .contains("\"route_blocked_or_failed\":[]")
    );
}

#[test]
fn flags_extra_quality_12b_workers() {
    let manifest = "SmartSteam model pool manifest\nmanifest_worker role=quality port=8686\nmanifest_worker role=quality port=8696\nmanifest_worker role=summary port=8687";
    let status = "SmartSteam model pool status\nworker role=quality status=healthy\nworker role=quality status=healthy\nworker role=summary status=healthy";
    let route_results = vec![RouteSmokeResult {
        task_kind: "summary".to_owned(),
        request_ok: true,
        route_allowed: Some(true),
    }];

    let alignment = model_pool_smoke_alignment(manifest, status, &route_results);

    assert!(alignment.contains("alignment_ok=false"));
    assert!(alignment.contains("manifest_quality_workers=2 status_quality_workers=2"));
    assert!(alignment.contains("extra_quality_12b_detected=true"));
}

fn full_helper_manifest() -> &'static str {
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

fn full_helper_status() -> &'static str {
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

fn full_allowed_route_results() -> Vec<RouteSmokeResult> {
    MODEL_POOL_SMOKE_TASK_KINDS
        .iter()
        .map(|task_kind| RouteSmokeResult {
            task_kind: (*task_kind).to_owned(),
            request_ok: true,
            route_allowed: Some(true),
        })
        .collect()
}
