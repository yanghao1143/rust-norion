use crate::app::status_json::json_string_literal;
#[cfg(test)]
use crate::app::status_json::{
    require_json_bool_equals, require_json_string_equals, required_json_string,
};

const SMOKE_ERROR_JSON_SCHEMA: &str = "smartsteam.forge.model_pool_smoke_error.v1";

#[derive(Debug, PartialEq, Eq)]
#[cfg(test)]
pub(super) struct SmokeErrorJsonSummary {
    pub(super) contract_ok: bool,
    pub(super) error: String,
    pub(super) user_message: String,
}

pub(super) fn smoke_error_status(error: &str) -> String {
    [
        format!("model pool smoke contract_ok=false error={error}"),
        "section=smoke_error_json".to_owned(),
        smoke_error_json(error),
    ]
    .join("\n")
}

#[cfg(test)]
pub(super) fn smoke_error_json_summary(
    smoke_error_json: &str,
) -> Result<SmokeErrorJsonSummary, String> {
    require_json_string_equals(
        smoke_error_json,
        "schema",
        SMOKE_ERROR_JSON_SCHEMA,
        "model pool smoke error JSON schema",
    )?;
    require_json_bool_equals(
        smoke_error_json,
        "read_only",
        true,
        "model pool smoke error JSON read_only",
    )?;
    require_json_bool_equals(
        smoke_error_json,
        "launches_process",
        false,
        "model pool smoke error JSON launches_process",
    )?;
    require_json_bool_equals(
        smoke_error_json,
        "sends_prompt",
        false,
        "model pool smoke error JSON sends_prompt",
    )?;
    let contract_ok = contract_ok_field(smoke_error_json)?;
    let error = required_json_string(
        smoke_error_json,
        "error",
        "model pool smoke error JSON error",
    )?;
    let user_message = required_json_string(
        smoke_error_json,
        "user_message",
        "model pool smoke error JSON user_message",
    )?;
    validate_user_message(&error, &user_message)?;

    Ok(SmokeErrorJsonSummary {
        contract_ok,
        error,
        user_message,
    })
}

fn smoke_error_json(error: &str) -> String {
    let user_message = smoke_error_user_message(error);
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":true,",
            "\"launches_process\":false,",
            "\"sends_prompt\":false,",
            "\"contract_ok\":false,",
            "\"error\":{},",
            "\"user_message\":{}",
            "}}"
        ),
        json_string_literal(SMOKE_ERROR_JSON_SCHEMA),
        json_string_literal(error),
        json_string_literal(&user_message),
    )
}

#[cfg(test)]
fn contract_ok_field(smoke_error_json: &str) -> Result<bool, String> {
    match crate::app::status_json::json_bool_field(smoke_error_json, "contract_ok") {
        Some(false) => Ok(false),
        Some(true) => {
            Err("model pool smoke error JSON contract_ok expected false, got true".to_owned())
        }
        None => Err("model pool smoke error JSON contract_ok missing contract_ok".to_owned()),
    }
}

#[cfg(test)]
fn validate_user_message(error: &str, user_message: &str) -> Result<(), String> {
    let expected = smoke_error_user_message(error);
    if user_message == expected {
        return Ok(());
    }
    Err(format!(
        "model pool smoke error JSON user_message drift: expected {expected:?}, got {user_message:?}"
    ))
}

fn smoke_error_user_message(error: &str) -> String {
    format!("model pool smoke contract error: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_error_status_preserves_text_and_adds_machine_readable_json() {
        let status = smoke_error_status("manifest unavailable");

        assert!(
            status.starts_with("model pool smoke contract_ok=false error=manifest unavailable")
        );
        assert!(status.contains("section=smoke_error_json"));
        let smoke_error_json = status
            .lines()
            .skip_while(|line| *line != "section=smoke_error_json")
            .nth(1)
            .expect("smoke_error_json section should include a JSON payload line");

        assert_eq!(
            smoke_error_json_summary(smoke_error_json).unwrap(),
            SmokeErrorJsonSummary {
                contract_ok: false,
                error: "manifest unavailable".to_owned(),
                user_message: "model pool smoke contract error: manifest unavailable".to_owned(),
            }
        );
    }

    #[test]
    fn smoke_error_json_summary_rejects_schema_and_side_effect_drift() {
        let json = smoke_error_json("manifest unavailable");
        let wrong_schema = json.replacen(
            "\"schema\":\"smartsteam.forge.model_pool_smoke_error.v1\"",
            "\"schema\":\"wrong.v1\"",
            1,
        );
        let launches_process =
            json.replacen("\"launches_process\":false", "\"launches_process\":true", 1);

        assert!(
            smoke_error_json_summary(&wrong_schema)
                .unwrap_err()
                .contains("smoke error JSON schema")
        );
        assert!(
            smoke_error_json_summary(&launches_process)
                .unwrap_err()
                .contains("smoke error JSON launches_process")
        );
    }

    #[test]
    fn smoke_error_json_summary_requires_failed_contract_and_error_text() {
        let contract_ok = smoke_error_json("manifest unavailable").replacen(
            "\"contract_ok\":false",
            "\"contract_ok\":true",
            1,
        );
        let empty_error = smoke_error_json("manifest unavailable").replacen(
            "\"error\":\"manifest unavailable\"",
            "\"error\":\"\"",
            1,
        );

        assert!(
            smoke_error_json_summary(&contract_ok)
                .unwrap_err()
                .contains("contract_ok expected false")
        );
        assert!(
            smoke_error_json_summary(&empty_error)
                .unwrap_err()
                .contains("smoke error JSON error")
        );
    }

    #[test]
    fn smoke_error_json_summary_rejects_user_message_drift() {
        let json = smoke_error_json("manifest unavailable").replacen(
            "\"error\":\"manifest unavailable\"",
            "\"error\":\"manifest ready\"",
            1,
        );

        assert!(
            smoke_error_json_summary(&json)
                .unwrap_err()
                .contains("user_message drift")
        );
    }
}
