use crate::app::status_json::{json_bool_field, json_null_field, json_string_field};

use super::json_assert::{require_json_bool, require_json_string};

pub(super) const ROUTE_SMOKE_JSON_SCHEMA: &str = "smartsteam.forge.model_pool_route_smoke.v1";

#[derive(Debug, PartialEq, Eq)]
pub(super) struct RouteSmokeJsonSummary {
    pub(super) task_kind: String,
    pub(super) ok: bool,
    pub(super) route_allowed: Option<bool>,
    pub(super) error: Option<String>,
}

#[cfg(test)]
pub(super) fn validate_route_smoke_json(route_smoke_json: &str) -> Result<(), String> {
    route_smoke_json_summary(route_smoke_json).map(|_| ())
}

pub(super) fn route_smoke_json_summary(
    route_smoke_json: &str,
) -> Result<RouteSmokeJsonSummary, String> {
    require_json_string(
        route_smoke_json,
        "schema",
        ROUTE_SMOKE_JSON_SCHEMA,
        "model pool route smoke JSON schema",
    )?;
    require_json_bool(
        route_smoke_json,
        "read_only",
        true,
        "model pool route smoke JSON read_only",
    )?;
    require_json_bool(
        route_smoke_json,
        "launches_process",
        false,
        "model pool route smoke JSON launches_process",
    )?;
    require_json_bool(
        route_smoke_json,
        "sends_prompt",
        false,
        "model pool route smoke JSON sends_prompt",
    )?;
    let task_kind = json_string_field(route_smoke_json, "task_kind")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "model pool route smoke JSON missing task_kind".to_owned())?;
    let ok = json_bool_field(route_smoke_json, "ok")
        .ok_or_else(|| "model pool route smoke JSON missing ok".to_owned())?;

    let route_allowed = route_allowed_field(route_smoke_json, &task_kind)?;
    let error = error_field(route_smoke_json, ok, &task_kind)?;
    Ok(RouteSmokeJsonSummary {
        task_kind,
        ok,
        route_allowed,
        error,
    })
}

fn route_allowed_field(route_smoke_json: &str, task_kind: &str) -> Result<Option<bool>, String> {
    if let Some(value) = json_bool_field(route_smoke_json, "route_allowed") {
        return Ok(Some(value));
    }
    if json_null_field(route_smoke_json, "route_allowed").is_some() {
        return Ok(None);
    }
    Err(format!(
        "model pool route smoke JSON missing route_allowed for {task_kind}"
    ))
}

fn error_field(
    route_smoke_json: &str,
    ok: bool,
    task_kind: &str,
) -> Result<Option<String>, String> {
    if ok {
        if json_null_field(route_smoke_json, "error").is_some() {
            return Ok(None);
        }
        return Err(format!(
            "model pool route smoke JSON expected null error for successful {task_kind}"
        ));
    }

    json_string_field(route_smoke_json, "error")
        .filter(|value| !value.trim().is_empty())
        .map(Some)
        .ok_or_else(|| format!("model pool route smoke JSON missing error for {task_kind}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_route_smoke_json(ok: bool) -> String {
        format!(
            concat!(
                "{{",
                "\"schema\":\"smartsteam.forge.model_pool_route_smoke.v1\",",
                "\"read_only\":true,",
                "\"launches_process\":false,",
                "\"sends_prompt\":false,",
                "\"task_kind\":\"summary\",",
                "\"ok\":{},",
                "\"route_allowed\":{},",
                "\"error\":{}",
                "}}"
            ),
            if ok { "true" } else { "false" },
            if ok { "true" } else { "null" },
            if ok {
                "null"
            } else {
                "\"summary unavailable\""
            },
        )
    }

    #[test]
    fn route_smoke_json_validation_accepts_success_and_failure_contracts() {
        validate_route_smoke_json(&valid_route_smoke_json(true)).unwrap();
        validate_route_smoke_json(&valid_route_smoke_json(false)).unwrap();
    }

    #[test]
    fn route_smoke_json_validation_rejects_side_effect_flags() {
        let value =
            valid_route_smoke_json(true).replace("\"sends_prompt\":false", "\"sends_prompt\":true");

        assert!(
            validate_route_smoke_json(&value)
                .unwrap_err()
                .contains("route smoke JSON sends_prompt")
        );
    }

    #[test]
    fn route_smoke_json_validation_rejects_missing_nullable_fields() {
        let value = valid_route_smoke_json(true).replace("\"error\":null", "\"other\":null");

        assert!(
            validate_route_smoke_json(&value)
                .unwrap_err()
                .contains("expected null error")
        );
    }

    #[test]
    fn route_smoke_json_validation_rejects_failure_without_error() {
        let value = valid_route_smoke_json(false)
            .replace("\"error\":\"summary unavailable\"", "\"error\":null");

        assert!(
            validate_route_smoke_json(&value)
                .unwrap_err()
                .contains("missing error")
        );
    }

    #[test]
    fn route_smoke_json_summary_projects_pairing_fields() {
        let summary = route_smoke_json_summary(&valid_route_smoke_json(false)).unwrap();

        assert_eq!(
            summary,
            RouteSmokeJsonSummary {
                task_kind: "summary".to_owned(),
                ok: false,
                route_allowed: None,
                error: Some("summary unavailable".to_owned()),
            }
        );
    }
}
