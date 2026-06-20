use crate::app::status_json::{json_bool_field, json_number_field, json_string_array_field};

use super::json_assert::{bool_text, require_json_bool, require_json_string};

const ALIGNMENT_JSON_SCHEMA: &str = "smartsteam.forge.model_pool_smoke_alignment.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AlignmentJsonSummary {
    pub(super) alignment_ok: bool,
    pub(super) manifest_roles: Vec<String>,
    pub(super) status_roles: Vec<String>,
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
    pub(super) route_smoke_target: usize,
    pub(super) route_smoke_count_aligned: bool,
    pub(super) missing_status_roles: Vec<String>,
    pub(super) unplanned_status_roles: Vec<String>,
    pub(super) route_blocked_or_failed: Vec<String>,
}

pub(super) fn validate_alignment_json(
    alignment_json: &str,
    smoke_alignment_ok: bool,
) -> Result<(), String> {
    let summary = alignment_json_summary(alignment_json)?;
    if summary.alignment_ok != smoke_alignment_ok {
        return Err(format!(
            "model pool smoke alignment JSON alignment_ok expected {}, got {}",
            bool_text(smoke_alignment_ok),
            bool_text(summary.alignment_ok)
        ));
    }
    Ok(())
}

pub(super) fn alignment_json_summary(alignment_json: &str) -> Result<AlignmentJsonSummary, String> {
    require_json_string(
        alignment_json,
        "schema",
        ALIGNMENT_JSON_SCHEMA,
        "model pool smoke alignment JSON schema",
    )?;
    require_json_bool(
        alignment_json,
        "read_only",
        true,
        "model pool smoke alignment JSON read_only",
    )?;
    require_json_bool(
        alignment_json,
        "launches_process",
        false,
        "model pool smoke alignment JSON launches_process",
    )?;
    require_json_bool(
        alignment_json,
        "sends_prompt",
        false,
        "model pool smoke alignment JSON sends_prompt",
    )?;
    Ok(AlignmentJsonSummary {
        alignment_ok: required_bool(alignment_json, "alignment_ok")?,
        manifest_roles: required_string_array(alignment_json, "manifest_roles")?,
        status_roles: required_string_array(alignment_json, "status_roles")?,
        unexpected_manifest_roles: required_string_array(
            alignment_json,
            "unexpected_manifest_roles",
        )?,
        unexpected_status_roles: required_string_array(alignment_json, "unexpected_status_roles")?,
        manifest_quality_workers: required_usize(alignment_json, "manifest_quality_workers")?,
        status_quality_workers: required_usize(alignment_json, "status_quality_workers")?,
        extra_quality_12b_detected: required_bool(alignment_json, "extra_quality_12b_detected")?,
        manifest_helper_workers: required_usize(alignment_json, "manifest_helper_workers")?,
        status_helper_workers: required_usize(alignment_json, "status_helper_workers")?,
        helper_target: required_usize(alignment_json, "helper_target")?,
        helper_worker_count_aligned: required_bool(alignment_json, "helper_worker_count_aligned")?,
        missing_manifest_helper_roles: required_string_array(
            alignment_json,
            "missing_manifest_helper_roles",
        )?,
        missing_status_helper_roles: required_string_array(
            alignment_json,
            "missing_status_helper_roles",
        )?,
        missing_route_smoke_tasks: required_string_array(
            alignment_json,
            "missing_route_smoke_tasks",
        )?,
        unexpected_route_smoke_tasks: required_string_array(
            alignment_json,
            "unexpected_route_smoke_tasks",
        )?,
        route_smoke_count: required_usize(alignment_json, "route_smoke_count")?,
        route_smoke_unique_tasks: required_usize(alignment_json, "route_smoke_unique_tasks")?,
        route_smoke_target: required_usize(alignment_json, "route_smoke_target")?,
        route_smoke_count_aligned: required_bool(alignment_json, "route_smoke_count_aligned")?,
        missing_status_roles: required_string_array(alignment_json, "missing_status_roles")?,
        unplanned_status_roles: required_string_array(alignment_json, "unplanned_status_roles")?,
        route_blocked_or_failed: required_string_array(alignment_json, "route_blocked_or_failed")?,
    })
}

fn required_bool(alignment_json: &str, field: &str) -> Result<bool, String> {
    json_bool_field(alignment_json, field)
        .ok_or_else(|| format!("model pool smoke alignment JSON missing {field}"))
}

fn required_string_array(alignment_json: &str, field: &str) -> Result<Vec<String>, String> {
    json_string_array_field(alignment_json, field)
        .ok_or_else(|| format!("model pool smoke alignment JSON missing {field}"))
}

fn required_usize(alignment_json: &str, field: &str) -> Result<usize, String> {
    let raw = json_number_field(alignment_json, field)
        .ok_or_else(|| format!("model pool smoke alignment JSON missing {field}"))?;
    raw.parse::<usize>().map_err(|_| {
        format!("model pool smoke alignment JSON expected usize for {field}, got {raw:?}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alignment_json(alignment_ok: bool) -> String {
        format!(
            concat!(
                "{{",
                "\"schema\":\"smartsteam.forge.model_pool_smoke_alignment.v1\",",
                "\"read_only\":true,",
                "\"launches_process\":false,",
                "\"sends_prompt\":false,",
                "\"alignment_ok\":{},",
                "\"manifest_roles\":[\"quality\",\"summary\"],",
                "\"status_roles\":[\"quality\"],",
                "\"unexpected_manifest_roles\":[],",
                "\"unexpected_status_roles\":[\"extra\"],",
                "\"manifest_quality_workers\":1,",
                "\"status_quality_workers\":1,",
                "\"extra_quality_12b_detected\":false,",
                "\"manifest_helper_workers\":1,",
                "\"status_helper_workers\":0,",
                "\"helper_target\":5,",
                "\"helper_worker_count_aligned\":false,",
                "\"missing_manifest_helper_roles\":[\"router\"],",
                "\"missing_status_helper_roles\":[\"summary\",\"router\"],",
                "\"missing_route_smoke_tasks\":[\"review\"],",
                "\"unexpected_route_smoke_tasks\":[],",
                "\"route_smoke_count\":4,",
                "\"route_smoke_unique_tasks\":4,",
                "\"route_smoke_target\":5,",
                "\"route_smoke_count_aligned\":false,",
                "\"missing_status_roles\":[\"summary\"],",
                "\"unplanned_status_roles\":[\"extra\"],",
                "\"route_blocked_or_failed\":[\"review\"]",
                "}}"
            ),
            bool_text(alignment_ok)
        )
    }

    #[test]
    fn alignment_json_validation_accepts_matching_contract_fields() {
        validate_alignment_json(&alignment_json(false), false).unwrap();
    }

    #[test]
    fn alignment_json_validation_rejects_side_effect_flags() {
        let value = alignment_json(true).replace("\"sends_prompt\":false", "\"sends_prompt\":true");

        assert!(
            validate_alignment_json(&value, true)
                .unwrap_err()
                .contains("alignment JSON sends_prompt")
        );
    }

    #[test]
    fn alignment_json_validation_rejects_alignment_mismatch() {
        assert!(
            validate_alignment_json(&alignment_json(false), true)
                .unwrap_err()
                .contains("alignment JSON alignment_ok")
        );
    }

    #[test]
    fn alignment_json_summary_projects_topology_fields() {
        let summary = alignment_json_summary(&alignment_json(false)).unwrap();

        assert_eq!(
            summary,
            AlignmentJsonSummary {
                alignment_ok: false,
                manifest_roles: vec!["quality".to_owned(), "summary".to_owned()],
                status_roles: vec!["quality".to_owned()],
                unexpected_manifest_roles: Vec::new(),
                unexpected_status_roles: vec!["extra".to_owned()],
                manifest_quality_workers: 1,
                status_quality_workers: 1,
                extra_quality_12b_detected: false,
                manifest_helper_workers: 1,
                status_helper_workers: 0,
                helper_target: 5,
                helper_worker_count_aligned: false,
                missing_manifest_helper_roles: vec!["router".to_owned()],
                missing_status_helper_roles: vec!["summary".to_owned(), "router".to_owned()],
                missing_route_smoke_tasks: vec!["review".to_owned()],
                unexpected_route_smoke_tasks: Vec::new(),
                route_smoke_count: 4,
                route_smoke_unique_tasks: 4,
                route_smoke_target: 5,
                route_smoke_count_aligned: false,
                missing_status_roles: vec!["summary".to_owned()],
                unplanned_status_roles: vec!["extra".to_owned()],
                route_blocked_or_failed: vec!["review".to_owned()],
            }
        );
    }

    #[test]
    fn alignment_json_summary_rejects_missing_topology_fields() {
        let value = alignment_json(true).replace("\"route_smoke_count\":4,", "");

        assert!(
            alignment_json_summary(&value)
                .unwrap_err()
                .contains("missing route_smoke_count")
        );
    }
}
