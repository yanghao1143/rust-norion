use std::collections::BTreeSet;

use model_pool_advice_core::HELPER_ROLES as MODEL_POOL_SMOKE_TASK_KINDS;

use super::super::advice::validate_model_pool_advice_report;
use super::super::alignment::{
    ManifestStatusAlignmentSummary, ModelPoolSmokeAlignment, manifest_status_alignment_summary,
};
use super::alignment_json::{
    AlignmentJsonSummary, alignment_json_summary, validate_alignment_json,
};
use super::contract::{smoke_contract_json, validate_contract_json};
use super::route_json::{RouteSmokeJsonSummary, route_smoke_json_summary};
use super::sections::{
    bool_text, require_bool_prefix, require_line, require_ordered_sections, require_section_body,
    require_section_lines, require_section_lines_before,
};

pub(super) fn build_model_pool_smoke_report(
    manifest: String,
    status: String,
    advice: String,
    alignment: &ModelPoolSmokeAlignment,
    route_reports: Vec<String>,
) -> String {
    let alignment_ok = bool_text(alignment.alignment_ok());
    let contract_json = smoke_contract_json(alignment_ok);
    let alignment_json = alignment.to_json();
    let alignment = alignment.to_text();
    let mut lines = vec![
        "SmartSteam model pool smoke".to_owned(),
        "read_only=true".to_owned(),
        "launches_process=false".to_owned(),
        "sends_prompt=false".to_owned(),
        format!("smoke_alignment_ok={alignment_ok}"),
        "contract_ok=true".to_owned(),
        "section=contract_json".to_owned(),
        contract_json,
        "section=manifest".to_owned(),
        manifest,
        "section=status".to_owned(),
        status,
        "section=advice".to_owned(),
        advice,
        "section=alignment_json".to_owned(),
        alignment_json,
        "section=alignment".to_owned(),
        alignment,
        "section=routes".to_owned(),
    ];
    lines.extend(route_reports);
    lines.join("\n")
}

pub(in crate::app) fn validate_model_pool_smoke_report(report: &str) -> Result<(), String> {
    let lines = report.lines().collect::<Vec<_>>();
    require_line(&lines, 0, "SmartSteam model pool smoke")?;
    require_line(&lines, 1, "read_only=true")?;
    require_line(&lines, 2, "launches_process=false")?;
    require_line(&lines, 3, "sends_prompt=false")?;
    let smoke_alignment_ok = require_bool_prefix(&lines, 4, "smoke_alignment_ok=")?;
    require_line(&lines, 5, "contract_ok=true")?;
    require_ordered_sections(
        &lines,
        &[
            "section=contract_json",
            "section=manifest",
            "section=status",
            "section=advice",
            "section=alignment_json",
            "section=alignment",
            "section=routes",
        ],
    )?;
    let contract_json = require_section_body(&lines, "section=contract_json")?;
    validate_contract_json(contract_json, smoke_alignment_ok)?;
    let advice_lines =
        require_section_lines_before(&lines, "section=advice", "section=alignment_json")?;
    validate_model_pool_advice_report(&advice_lines.join("\n"))?;
    let alignment_json = require_section_body(&lines, "section=alignment_json")?;
    validate_alignment_json(alignment_json, smoke_alignment_ok)?;
    let alignment_summary = alignment_json_summary(alignment_json)?;
    let manifest_lines =
        require_section_lines_before(&lines, "section=manifest", "section=status")?;
    let status_lines = require_section_lines_before(&lines, "section=status", "section=advice")?;
    let manifest = manifest_lines.join("\n");
    let status = status_lines.join("\n");
    let manifest_status_summary = manifest_status_alignment_summary(&manifest, &status);
    validate_manifest_status_matches_alignment_json(&manifest_status_summary, &alignment_summary)?;
    let alignment_lines = require_section_lines(&lines, "section=alignment")?;
    validate_alignment_text_matches_json(alignment_lines, &alignment_summary)?;
    let route_summaries = validate_route_smoke_json_sections(&lines)?;
    validate_route_smoke_aggregate_matches_alignment_json(&route_summaries, &alignment_summary)?;
    Ok(())
}

fn validate_manifest_status_matches_alignment_json(
    manifest_status: &ManifestStatusAlignmentSummary,
    summary: &AlignmentJsonSummary,
) -> Result<(), String> {
    require_alignment_json_field_match(
        "manifest_roles",
        &manifest_status.manifest_roles,
        &summary.manifest_roles,
    )?;
    require_alignment_json_field_match(
        "status_roles",
        &manifest_status.status_roles,
        &summary.status_roles,
    )?;
    require_alignment_json_field_match(
        "unexpected_manifest_roles",
        &manifest_status.unexpected_manifest_roles,
        &summary.unexpected_manifest_roles,
    )?;
    require_alignment_json_field_match(
        "unexpected_status_roles",
        &manifest_status.unexpected_status_roles,
        &summary.unexpected_status_roles,
    )?;
    require_alignment_json_usize_match(
        "manifest_quality_workers",
        manifest_status.manifest_quality_workers,
        summary.manifest_quality_workers,
    )?;
    require_alignment_json_usize_match(
        "status_quality_workers",
        manifest_status.status_quality_workers,
        summary.status_quality_workers,
    )?;
    require_alignment_json_bool_match(
        "extra_quality_12b_detected",
        manifest_status.extra_quality_12b_detected,
        summary.extra_quality_12b_detected,
    )?;
    require_alignment_json_usize_match(
        "manifest_helper_workers",
        manifest_status.manifest_helper_workers,
        summary.manifest_helper_workers,
    )?;
    require_alignment_json_usize_match(
        "status_helper_workers",
        manifest_status.status_helper_workers,
        summary.status_helper_workers,
    )?;
    require_alignment_json_usize_match(
        "helper_target",
        manifest_status.helper_target,
        summary.helper_target,
    )?;
    require_alignment_json_bool_match(
        "helper_worker_count_aligned",
        manifest_status.helper_worker_count_aligned,
        summary.helper_worker_count_aligned,
    )?;
    require_alignment_json_field_match(
        "missing_manifest_helper_roles",
        &manifest_status.missing_manifest_helper_roles,
        &summary.missing_manifest_helper_roles,
    )?;
    require_alignment_json_field_match(
        "missing_status_helper_roles",
        &manifest_status.missing_status_helper_roles,
        &summary.missing_status_helper_roles,
    )?;
    require_alignment_json_field_match(
        "missing_status_roles",
        &manifest_status.missing_status_roles,
        &summary.missing_status_roles,
    )?;
    require_alignment_json_field_match(
        "unplanned_status_roles",
        &manifest_status.unplanned_status_roles,
        &summary.unplanned_status_roles,
    )
}

fn require_alignment_json_field_match(
    field: &str,
    actual: &[String],
    expected: &[String],
) -> Result<(), String> {
    (actual == expected).then_some(()).ok_or_else(|| {
        format!(
            "model pool smoke alignment JSON {field} mismatch with manifest/status: manifest_status {:?}, json {:?}",
            actual, expected
        )
    })
}

fn require_alignment_json_usize_match(
    field: &str,
    actual: usize,
    expected: usize,
) -> Result<(), String> {
    (actual == expected).then_some(()).ok_or_else(|| {
        format!(
            "model pool smoke alignment JSON {field} mismatch with manifest/status: manifest_status {actual}, json {expected}"
        )
    })
}

fn require_alignment_json_bool_match(
    field: &str,
    actual: bool,
    expected: bool,
) -> Result<(), String> {
    (actual == expected).then_some(()).ok_or_else(|| {
        format!(
            "model pool smoke alignment JSON {field} mismatch with manifest/status: manifest_status {}, json {}",
            bool_text(actual),
            bool_text(expected)
        )
    })
}

fn validate_alignment_text_matches_json(
    lines: &[&str],
    summary: &AlignmentJsonSummary,
) -> Result<(), String> {
    require_alignment_text_line(lines, "model_pool_smoke_alignment")?;
    require_alignment_text_line(
        lines,
        &format!("alignment_ok={}", bool_text(summary.alignment_ok)),
    )?;
    require_alignment_text_line(
        lines,
        &format!("manifest_roles={}", list_text(&summary.manifest_roles)),
    )?;
    require_alignment_text_line(
        lines,
        &format!("status_roles={}", list_text(&summary.status_roles)),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "unexpected_manifest_roles={}",
            list_text(&summary.unexpected_manifest_roles)
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "unexpected_status_roles={}",
            list_text(&summary.unexpected_status_roles)
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "manifest_quality_workers={} status_quality_workers={}",
            summary.manifest_quality_workers, summary.status_quality_workers
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "extra_quality_12b_detected={}",
            bool_text(summary.extra_quality_12b_detected)
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "manifest_helper_workers={} status_helper_workers={} helper_target={}",
            summary.manifest_helper_workers, summary.status_helper_workers, summary.helper_target
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "helper_worker_count_aligned={}",
            bool_text(summary.helper_worker_count_aligned)
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "missing_manifest_helper_roles={}",
            list_text(&summary.missing_manifest_helper_roles)
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "missing_status_helper_roles={}",
            list_text(&summary.missing_status_helper_roles)
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "missing_route_smoke_tasks={}",
            list_text(&summary.missing_route_smoke_tasks)
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "unexpected_route_smoke_tasks={}",
            list_text(&summary.unexpected_route_smoke_tasks)
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "route_smoke_count={} route_smoke_unique_tasks={} route_smoke_target={} route_smoke_count_aligned={}",
            summary.route_smoke_count,
            summary.route_smoke_unique_tasks,
            summary.route_smoke_target,
            bool_text(summary.route_smoke_count_aligned)
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "missing_status_roles={}",
            list_text(&summary.missing_status_roles)
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "unplanned_status_roles={}",
            list_text(&summary.unplanned_status_roles)
        ),
    )?;
    require_alignment_text_line(
        lines,
        &format!(
            "route_blocked_or_failed={}",
            list_text(&summary.route_blocked_or_failed)
        ),
    )
}

fn require_alignment_text_line(lines: &[&str], expected: &str) -> Result<(), String> {
    lines
        .iter()
        .any(|line| *line == expected)
        .then_some(())
        .ok_or_else(|| format!("model pool smoke alignment text missing {expected:?}"))
}

fn list_text(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(",")
    }
}

fn validate_route_smoke_json_sections(
    lines: &[&str],
) -> Result<Vec<RouteSmokeJsonSummary>, String> {
    let mut found_route_smoke = false;
    let mut summaries = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !line.starts_with("route_smoke ") {
            continue;
        }
        found_route_smoke = true;
        let Some(section) = lines.get(index + 1) else {
            return Err(format!(
                "model pool smoke missing section=route_smoke_json after {line}"
            ));
        };
        if *section != "section=route_smoke_json" {
            return Err(format!(
                "model pool smoke missing section=route_smoke_json after {line}"
            ));
        }
        let Some(body) = lines.get(index + 2) else {
            return Err(format!(
                "model pool smoke missing body for section=route_smoke_json after {line}"
            ));
        };
        if body.starts_with("section=") || body.starts_with("route_smoke ") {
            return Err(format!(
                "model pool smoke missing body for section=route_smoke_json after {line}"
            ));
        }
        let summary = route_smoke_json_summary(body)?;
        validate_route_smoke_line_matches_json(line, &summary)?;
        summaries.push(summary);
    }

    for (index, line) in lines.iter().enumerate() {
        if *line != "section=route_smoke_json" {
            continue;
        }
        let previous = index
            .checked_sub(1)
            .and_then(|previous| lines.get(previous));
        if !previous.is_some_and(|line| line.starts_with("route_smoke ")) {
            return Err("model pool smoke found orphan section=route_smoke_json".to_owned());
        }
    }

    found_route_smoke
        .then_some(summaries)
        .ok_or_else(|| "model pool smoke missing route_smoke entries".to_owned())
}

fn validate_route_smoke_aggregate_matches_alignment_json(
    route_summaries: &[RouteSmokeJsonSummary],
    summary: &AlignmentJsonSummary,
) -> Result<(), String> {
    let route_smoke_tasks = route_summaries
        .iter()
        .map(|summary| summary.task_kind.clone())
        .collect::<BTreeSet<_>>();
    let route_blocked_or_failed = route_summaries
        .iter()
        .filter(|route| !route.ok || route.route_allowed != Some(true))
        .map(|route| route.task_kind.clone())
        .collect::<Vec<_>>();
    let route_smoke_count = route_summaries.len();
    let route_smoke_unique_tasks = route_smoke_tasks.len();
    let route_smoke_target = MODEL_POOL_SMOKE_TASK_KINDS.len();
    let route_smoke_count_aligned =
        route_smoke_count == route_smoke_target && route_smoke_unique_tasks == route_smoke_target;
    let missing_route_smoke_tasks = MODEL_POOL_SMOKE_TASK_KINDS
        .iter()
        .filter(|task_kind| !route_smoke_tasks.contains(**task_kind))
        .map(|task_kind| (*task_kind).to_owned())
        .collect::<Vec<_>>();
    let unexpected_route_smoke_tasks = route_smoke_tasks
        .iter()
        .filter(|task_kind| !MODEL_POOL_SMOKE_TASK_KINDS.contains(&task_kind.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    require_route_alignment_json_usize_match(
        "route_smoke_count",
        route_smoke_count,
        summary.route_smoke_count,
    )?;
    require_route_alignment_json_usize_match(
        "route_smoke_unique_tasks",
        route_smoke_unique_tasks,
        summary.route_smoke_unique_tasks,
    )?;
    require_route_alignment_json_usize_match(
        "route_smoke_target",
        route_smoke_target,
        summary.route_smoke_target,
    )?;
    require_route_alignment_json_bool_match(
        "route_smoke_count_aligned",
        route_smoke_count_aligned,
        summary.route_smoke_count_aligned,
    )?;
    require_route_alignment_json_field_match(
        "missing_route_smoke_tasks",
        &missing_route_smoke_tasks,
        &summary.missing_route_smoke_tasks,
    )?;
    require_route_alignment_json_field_match(
        "unexpected_route_smoke_tasks",
        &unexpected_route_smoke_tasks,
        &summary.unexpected_route_smoke_tasks,
    )?;
    require_route_alignment_json_field_match(
        "route_blocked_or_failed",
        &route_blocked_or_failed,
        &summary.route_blocked_or_failed,
    )
}

fn require_route_alignment_json_field_match(
    field: &str,
    actual: &[String],
    expected: &[String],
) -> Result<(), String> {
    (actual == expected).then_some(()).ok_or_else(|| {
        format!(
            "model pool smoke alignment JSON {field} mismatch with route smoke: routes {:?}, json {:?}",
            actual, expected
        )
    })
}

fn require_route_alignment_json_usize_match(
    field: &str,
    actual: usize,
    expected: usize,
) -> Result<(), String> {
    (actual == expected).then_some(()).ok_or_else(|| {
        format!(
            "model pool smoke alignment JSON {field} mismatch with route smoke: routes {actual}, json {expected}"
        )
    })
}

fn require_route_alignment_json_bool_match(
    field: &str,
    actual: bool,
    expected: bool,
) -> Result<(), String> {
    (actual == expected).then_some(()).ok_or_else(|| {
        format!(
            "model pool smoke alignment JSON {field} mismatch with route smoke: routes {}, json {}",
            bool_text(actual),
            bool_text(expected)
        )
    })
}

fn validate_route_smoke_line_matches_json(
    line: &str,
    summary: &RouteSmokeJsonSummary,
) -> Result<(), String> {
    let task_kind = route_smoke_token_value(line, "task_kind=")?;
    if task_kind != summary.task_kind {
        return Err(format!(
            "model pool route smoke JSON task_kind mismatch: line {task_kind:?}, json {:?}",
            summary.task_kind
        ));
    }

    let ok = route_smoke_token_value(line, "ok=").and_then(route_smoke_bool)?;
    if ok != summary.ok {
        return Err(format!(
            "model pool route smoke JSON ok mismatch for {task_kind}: line {}, json {}",
            bool_text(ok),
            bool_text(summary.ok)
        ));
    }

    let route_allowed = route_smoke_token_value(line, "route_allowed=")?;
    let route_allowed_json = route_smoke_option_bool_text(summary.route_allowed);
    if route_allowed != route_allowed_json {
        return Err(format!(
            "model pool route smoke JSON route_allowed mismatch for {task_kind}: line {route_allowed:?}, json {route_allowed_json:?}"
        ));
    }

    let line_error = route_smoke_error_value(line);
    match (summary.ok, line_error, summary.error.as_deref()) {
        (true, None, None) => Ok(()),
        (false, Some(line_error), Some(json_error)) if line_error == json_error => Ok(()),
        (true, _, _) => Err(format!(
            "model pool route smoke JSON expected null error for successful {task_kind}"
        )),
        (false, _, _) => Err(format!(
            "model pool route smoke JSON error mismatch for {task_kind}"
        )),
    }
}

fn route_smoke_token_value<'a>(line: &'a str, prefix: &str) -> Result<&'a str, String> {
    line.split_whitespace()
        .find_map(|token| token.strip_prefix(prefix))
        .ok_or_else(|| format!("model pool route smoke line missing {prefix}"))
}

fn route_smoke_bool(value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!(
            "model pool route smoke line expected bool, got {value:?}"
        )),
    }
}

fn route_smoke_option_bool_text(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}

fn route_smoke_error_value(line: &str) -> Option<&str> {
    line.split_once(" error=").map(|(_, error)| error)
}

#[cfg(test)]
fn valid_report_fixture() -> String {
    vec![
        "SmartSteam model pool smoke".to_owned(),
        "read_only=true".to_owned(),
        "launches_process=false".to_owned(),
        "sends_prompt=false".to_owned(),
        "smoke_alignment_ok=true".to_owned(),
        "contract_ok=true".to_owned(),
        "section=contract_json".to_owned(),
        "{\"schema\":\"smartsteam.forge.model_pool_smoke_contract.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"contract_ok\":true,\"alignment_ok\":true}".to_owned(),
        "section=manifest".to_owned(),
        valid_manifest_fixture(),
        "section=status".to_owned(),
        valid_status_fixture(),
        "section=advice".to_owned(),
        valid_advice_report_fixture(),
        "section=alignment_json".to_owned(),
        valid_alignment_json_fixture(true),
        "section=alignment".to_owned(),
        valid_alignment_text_fixture(true),
        "section=routes".to_owned(),
        "route_smoke task_kind=summary ok=true route_allowed=true".to_owned(),
        "section=route_smoke_json".to_owned(),
        "{\"schema\":\"smartsteam.forge.model_pool_route_smoke.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"summary\",\"ok\":true,\"route_allowed\":true,\"error\":null}".to_owned(),
    ]
    .join("\n")
}

#[cfg(test)]
fn valid_manifest_fixture() -> String {
    [
        "SmartSteam model pool manifest",
        "manifest_worker role=quality port=8686",
        "manifest_worker role=summary port=8687",
    ]
    .join("\n")
}

#[cfg(test)]
fn valid_status_fixture() -> String {
    [
        "SmartSteam model pool status",
        "worker role=quality status=healthy",
        "worker role=summary status=healthy",
    ]
    .join("\n")
}

#[cfg(test)]
fn valid_advice_report_fixture() -> String {
    super::super::advice::model_pool_advice(
        "SmartSteam model pool status\nquality_ready=true\nquality_context_sufficient=true\nquality_context_tokens=262144\nquality_context_required_tokens=262144\ncapacity policy=one_quality_plus_small_helpers expansion_allowed=true recommendation=add_summary_worker_first healthy_helper_worker_count=0 quality_runtime_accelerated=true\nworker role=quality status=healthy ready=true runtime_device=metal runtime_accelerator=metal gpu_layers=99",
    )
}

#[cfg(test)]
fn valid_alignment_json_fixture(alignment_ok: bool) -> String {
    format!(
        concat!(
            "{{",
            "\"schema\":\"smartsteam.forge.model_pool_smoke_alignment.v1\",",
            "\"read_only\":true,",
            "\"launches_process\":false,",
            "\"sends_prompt\":false,",
            "\"alignment_ok\":{},",
            "\"manifest_roles\":[\"quality\",\"summary\"],",
            "\"status_roles\":[\"quality\",\"summary\"],",
            "\"unexpected_manifest_roles\":[],",
            "\"unexpected_status_roles\":[],",
            "\"manifest_quality_workers\":1,",
            "\"status_quality_workers\":1,",
            "\"extra_quality_12b_detected\":false,",
            "\"manifest_helper_workers\":1,",
            "\"status_helper_workers\":1,",
            "\"helper_target\":5,",
            "\"helper_worker_count_aligned\":false,",
            "\"missing_manifest_helper_roles\":[\"router\",\"review\",\"index\",\"test-gate\"],",
            "\"missing_status_helper_roles\":[\"router\",\"review\",\"index\",\"test-gate\"],",
            "\"missing_route_smoke_tasks\":[\"router\",\"review\",\"index\",\"test-gate\"],",
            "\"unexpected_route_smoke_tasks\":[],",
            "\"route_smoke_count\":1,",
            "\"route_smoke_unique_tasks\":1,",
            "\"route_smoke_target\":5,",
            "\"route_smoke_count_aligned\":false,",
            "\"missing_status_roles\":[],",
            "\"unplanned_status_roles\":[],",
            "\"route_blocked_or_failed\":[]",
            "}}"
        ),
        bool_text(alignment_ok)
    )
}

#[cfg(test)]
fn valid_alignment_text_fixture(alignment_ok: bool) -> String {
    [
        "model_pool_smoke_alignment".to_owned(),
        format!("alignment_ok={}", bool_text(alignment_ok)),
        "manifest_roles=quality,summary".to_owned(),
        "status_roles=quality,summary".to_owned(),
        "unexpected_manifest_roles=none".to_owned(),
        "unexpected_status_roles=none".to_owned(),
        "manifest_quality_workers=1 status_quality_workers=1".to_owned(),
        "extra_quality_12b_detected=false".to_owned(),
        "manifest_helper_workers=1 status_helper_workers=1 helper_target=5".to_owned(),
        "helper_worker_count_aligned=false".to_owned(),
        "missing_manifest_helper_roles=router,review,index,test-gate".to_owned(),
        "missing_status_helper_roles=router,review,index,test-gate".to_owned(),
        "missing_route_smoke_tasks=router,review,index,test-gate".to_owned(),
        "unexpected_route_smoke_tasks=none".to_owned(),
        "route_smoke_count=1 route_smoke_unique_tasks=1 route_smoke_target=5 route_smoke_count_aligned=false".to_owned(),
        "missing_status_roles=none".to_owned(),
        "unplanned_status_roles=none".to_owned(),
        "route_blocked_or_failed=none".to_owned(),
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model_pool::alignment::RouteSmokeResult;

    #[test]
    fn report_preserves_header_sections_and_route_payloads() {
        let manifest = "SmartSteam model pool manifest\nmanifest_worker role=quality port=8686";
        let status = "SmartSteam model pool status\nworker role=quality status=healthy";
        let route_results = vec![RouteSmokeResult {
            task_kind: "summary".to_owned(),
            request_ok: true,
            route_allowed: Some(true),
        }];
        let alignment = ModelPoolSmokeAlignment::from_summaries(manifest, status, &route_results);

        let report = build_model_pool_smoke_report(
            manifest.to_owned(),
            status.to_owned(),
            valid_advice_report_fixture(),
            &alignment,
            vec![
                "route_smoke task_kind=summary ok=true route_allowed=true".to_owned(),
                "section=route_smoke_json".to_owned(),
                "{\"schema\":\"smartsteam.forge.model_pool_route_smoke.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"summary\",\"ok\":true,\"route_allowed\":true,\"error\":null}".to_owned(),
                "route body".to_owned(),
            ],
        );
        let lines = report.lines().collect::<Vec<_>>();

        assert_eq!(lines[0], "SmartSteam model pool smoke");
        assert_eq!(lines[1], "read_only=true");
        assert_eq!(lines[2], "launches_process=false");
        assert_eq!(lines[3], "sends_prompt=false");
        assert_eq!(lines[5], "contract_ok=true");
        assert_eq!(lines[6], "section=contract_json");
        assert_eq!(lines[8], "section=manifest");
        assert_eq!(lines[9], "SmartSteam model pool manifest");
        assert!(report.contains("\"schema\":\"smartsteam.forge.model_pool_smoke_contract.v1\""));
        assert!(report.contains("\"contract_ok\":true"));
        assert!(report.contains("section=alignment_json"));
        assert!(report.contains("\"schema\":\"smartsteam.forge.model_pool_smoke_alignment.v1\""));
        assert!(report.contains("section=routes\nroute_smoke task_kind=summary ok=true"));
        assert!(report.contains("section=route_smoke_json"));
        assert!(report.ends_with("route body"));
        validate_model_pool_smoke_report(&report).unwrap();
    }

    #[test]
    fn report_validation_rejects_missing_read_only_header() {
        let report = valid_report_fixture().replacen("read_only=true", "read_only=false", 1);

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("read_only=true")
        );
    }

    #[test]
    fn report_validation_rejects_missing_alignment_schema() {
        let report = valid_report_fixture().replace(
            "\"schema\":\"smartsteam.forge.model_pool_smoke_alignment.v1\",",
            "",
        );

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("alignment JSON schema")
        );
    }

    #[test]
    fn report_validation_rejects_missing_contract_schema() {
        let report = valid_report_fixture().replace(
            "\"schema\":\"smartsteam.forge.model_pool_smoke_contract.v1\",",
            "",
        );

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("contract JSON schema")
        );
    }

    #[test]
    fn report_validation_rejects_failed_contract_header() {
        let report = valid_report_fixture().replacen("contract_ok=true", "contract_ok=false", 1);

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("contract_ok=true")
        );
    }

    #[test]
    fn report_validation_rejects_failed_contract_json() {
        let report =
            valid_report_fixture().replacen("\"contract_ok\":true", "\"contract_ok\":false", 1);

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("contract JSON contract_ok")
        );
    }

    #[test]
    fn report_validation_rejects_alignment_mismatch_between_header_and_contract_json() {
        let report =
            valid_report_fixture().replacen("\"alignment_ok\":true", "\"alignment_ok\":false", 1);

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("contract JSON alignment_ok")
        );
    }

    #[test]
    fn report_validation_rejects_missing_contract_json_body() {
        let report = valid_report_fixture().replacen(
            "section=contract_json\n{\"schema\":\"smartsteam.forge.model_pool_smoke_contract.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"contract_ok\":true,\"alignment_ok\":true}\n",
            "section=contract_json\n",
            1,
        );

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("missing body for section=contract_json")
        );
    }

    #[test]
    fn report_validation_rejects_alignment_json_mismatch_with_header() {
        let alignment_json = valid_alignment_json_fixture(true);
        let report = valid_report_fixture().replacen(
            &alignment_json,
            &valid_alignment_json_fixture(false),
            1,
        );

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("alignment JSON alignment_ok")
        );
    }

    #[test]
    fn report_validation_rejects_missing_alignment_json_body() {
        let alignment_json = valid_alignment_json_fixture(true);
        let report = valid_report_fixture().replacen(
            &format!("section=alignment_json\n{alignment_json}\n"),
            "section=alignment_json\n",
            1,
        );

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("missing body for section=alignment_json")
        );
    }

    #[test]
    fn report_validation_rejects_invalid_embedded_advice_json() {
        let report = valid_report_fixture().replacen(
            "\"schema\":\"smartsteam.forge.model_pool_advice.v1\"",
            "\"schema\":\"wrong.v1\"",
            1,
        );

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("model pool advice JSON schema")
        );
    }

    #[test]
    fn report_validation_rejects_manifest_status_alignment_json_drift() {
        let report = valid_report_fixture().replacen(
            "worker role=summary status=healthy",
            "worker role=router status=healthy",
            1,
        );

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("status_roles mismatch with manifest/status")
        );
    }

    #[test]
    fn report_validation_rejects_route_task_aggregate_alignment_json_drift() {
        let report = valid_report_fixture()
            .replacen("task_kind=summary", "task_kind=router", 1)
            .replacen("\"task_kind\":\"summary\"", "\"task_kind\":\"router\"", 1);

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("missing_route_smoke_tasks mismatch with route smoke")
        );
    }

    #[test]
    fn report_validation_rejects_route_blocked_aggregate_alignment_json_drift() {
        let report = valid_report_fixture()
            .replacen("route_allowed=true", "route_allowed=false", 1)
            .replacen("\"route_allowed\":true", "\"route_allowed\":false", 1);

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("route_blocked_or_failed mismatch with route smoke")
        );
    }

    #[test]
    fn report_validation_rejects_incomplete_alignment_json_topology_fields() {
        let report = valid_report_fixture().replacen("\"route_smoke_count\":1,", "", 1);

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("missing route_smoke_count")
        );
    }

    #[test]
    fn report_validation_rejects_alignment_text_mismatch_with_json() {
        let report = valid_report_fixture().replacen(
            "route_smoke_count=1 route_smoke_unique_tasks=1 route_smoke_target=5 route_smoke_count_aligned=false",
            "route_smoke_count=2 route_smoke_unique_tasks=1 route_smoke_target=5 route_smoke_count_aligned=false",
            1,
        );

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("alignment text missing")
        );
    }

    #[test]
    fn report_validation_rejects_missing_alignment_text_body() {
        let alignment_text = valid_alignment_text_fixture(true);
        let report = valid_report_fixture().replacen(
            &format!("section=alignment\n{alignment_text}\n"),
            "section=alignment\n",
            1,
        );

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("missing body for section=alignment")
        );
    }

    #[test]
    fn report_validation_rejects_missing_route_smoke_json_section() {
        let report = valid_report_fixture().replacen(
            "\nsection=route_smoke_json\n{\"schema\":\"smartsteam.forge.model_pool_route_smoke.v1\",\"read_only\":true,\"launches_process\":false,\"sends_prompt\":false,\"task_kind\":\"summary\",\"ok\":true,\"route_allowed\":true,\"error\":null}",
            "",
            1,
        );

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("missing section=route_smoke_json")
        );
    }

    #[test]
    fn report_validation_rejects_invalid_route_smoke_json_body() {
        let report = valid_report_fixture().replacen(
            "\"schema\":\"smartsteam.forge.model_pool_route_smoke.v1\"",
            "\"schema\":\"wrong.v1\"",
            1,
        );

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("route smoke JSON schema")
        );
    }

    #[test]
    fn report_validation_rejects_route_smoke_task_kind_mismatch() {
        let report = valid_report_fixture().replacen(
            "\"task_kind\":\"summary\"",
            "\"task_kind\":\"review\"",
            1,
        );

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("task_kind mismatch")
        );
    }

    #[test]
    fn report_validation_rejects_route_smoke_allowed_mismatch() {
        let report =
            valid_report_fixture().replacen("\"route_allowed\":true", "\"route_allowed\":false", 1);

        assert!(
            validate_model_pool_smoke_report(&report)
                .unwrap_err()
                .contains("route_allowed mismatch")
        );
    }
}
