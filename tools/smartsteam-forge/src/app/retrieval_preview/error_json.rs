use crate::app::status_json::json_string_literal;
#[cfg(test)]
use crate::app::status_json::{
    require_json_bool_equals, require_json_string_equals, required_json_number,
    required_json_string, validate_error_kind, validate_error_user_message,
};

const EXPERIENCE_RETRIEVAL_ERROR_JSON_SCHEMA: &str =
    "smartsteam.forge.experience_retrieval_error.v1";

#[derive(Debug, PartialEq, Eq)]
#[cfg(test)]
pub(in crate::app) struct ExperienceRetrievalErrorJsonSummary {
    pub(in crate::app) error_kind: String,
    pub(in crate::app) prompt: String,
    pub(in crate::app) requested_limit: usize,
    pub(in crate::app) error: String,
    pub(in crate::app) user_message: String,
}

pub(in crate::app) fn experience_retrieval_error_status(
    error_kind: &str,
    prompt: &str,
    requested_limit: usize,
    error: &str,
) -> String {
    let requested_limit = requested_limit.max(1);
    let user_message = format!("experience retrieval {error_kind} error: {error}");
    [
        format!(
            "experience_retrieval_error kind={error_kind} requested_limit={requested_limit} error={error}"
        ),
        "section=retrieval_error_json".to_owned(),
        experience_retrieval_error_json(error_kind, prompt, requested_limit, error, &user_message),
    ]
    .join("\n")
}

#[cfg(test)]
pub(in crate::app) fn experience_retrieval_error_json_summary(
    error_json: &str,
) -> Result<ExperienceRetrievalErrorJsonSummary, String> {
    require_json_string_equals(
        error_json,
        "schema",
        EXPERIENCE_RETRIEVAL_ERROR_JSON_SCHEMA,
        "experience retrieval error JSON schema",
    )?;
    require_json_bool_equals(
        error_json,
        "read_only",
        true,
        "experience retrieval error JSON read_only",
    )?;
    require_json_bool_equals(
        error_json,
        "writes_experience_state",
        false,
        "experience retrieval error JSON writes_experience_state",
    )?;
    require_json_bool_equals(
        error_json,
        "streams_model",
        false,
        "experience retrieval error JSON streams_model",
    )?;
    let error_kind = required_json_string(
        error_json,
        "error_kind",
        "experience retrieval error JSON error_kind",
    )?;
    validate_error_kind(&error_kind, "experience retrieval error JSON")?;
    let prompt = required_json_string(
        error_json,
        "prompt",
        "experience retrieval error JSON prompt",
    )?;
    let requested_limit = required_json_number(
        error_json,
        "requested_limit",
        "experience retrieval error JSON requested_limit",
    )?
    .parse::<usize>()
    .map_err(|_| "experience retrieval error JSON requested_limit must be usize".to_owned())?;
    if requested_limit == 0 {
        return Err("experience retrieval error JSON requested_limit must be positive".to_owned());
    }
    let error = required_json_string(error_json, "error", "experience retrieval error JSON error")?;
    let user_message = required_json_string(
        error_json,
        "user_message",
        "experience retrieval error JSON user_message",
    )?;
    validate_error_user_message("experience retrieval", &error_kind, &error, &user_message)?;

    Ok(ExperienceRetrievalErrorJsonSummary {
        error_kind,
        prompt,
        requested_limit,
        error,
        user_message,
    })
}

fn experience_retrieval_error_json(
    error_kind: &str,
    prompt: &str,
    requested_limit: usize,
    error: &str,
    user_message: &str,
) -> String {
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":true,",
            "\"writes_experience_state\":false,",
            "\"streams_model\":false,",
            "\"error_kind\":{},",
            "\"prompt\":{},",
            "\"requested_limit\":{},",
            "\"error\":{},",
            "\"user_message\":{}",
            "}}"
        ),
        json_string_literal(EXPERIENCE_RETRIEVAL_ERROR_JSON_SCHEMA),
        json_string_literal(error_kind),
        json_string_literal(prompt),
        requested_limit,
        json_string_literal(error),
        json_string_literal(user_message),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieval_error_status_carries_machine_readable_context() {
        let status = experience_retrieval_error_status(
            "provider",
            "model pool route code",
            0,
            "backend busy",
        );

        assert!(status.contains("experience_retrieval_error kind=provider requested_limit=1"));
        assert!(status.contains("section=retrieval_error_json"));
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=retrieval_error_json")
            .nth(1)
            .expect("retrieval_error_json section should include body");

        assert_eq!(
            experience_retrieval_error_json_summary(error_json).unwrap(),
            ExperienceRetrievalErrorJsonSummary {
                error_kind: "provider".to_owned(),
                prompt: "model pool route code".to_owned(),
                requested_limit: 1,
                error: "backend busy".to_owned(),
                user_message: "experience retrieval provider error: backend busy".to_owned(),
            }
        );
    }

    #[test]
    fn retrieval_error_json_rejects_side_effect_drift() {
        let status =
            experience_retrieval_error_status("contract", "prompt", 2, "missing requested_limit");
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=retrieval_error_json")
            .nth(1)
            .unwrap();
        let writes_state = error_json.replace(
            "\"writes_experience_state\":false",
            "\"writes_experience_state\":true",
        );

        assert!(
            experience_retrieval_error_json_summary(&writes_state)
                .unwrap_err()
                .contains("writes_experience_state")
        );
    }

    #[test]
    fn retrieval_error_json_rejects_unknown_error_kind() {
        let status =
            experience_retrieval_error_status("contract", "prompt", 2, "missing requested_limit");
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=retrieval_error_json")
            .nth(1)
            .unwrap();
        let drifted = error_json.replace("\"error_kind\":\"contract\"", "\"error_kind\":\"route\"");

        assert!(
            experience_retrieval_error_json_summary(&drifted)
                .unwrap_err()
                .contains("unknown error_kind")
        );
    }

    #[test]
    fn retrieval_error_json_rejects_user_message_drift() {
        let status = experience_retrieval_error_status("provider", "prompt", 2, "backend busy");
        let error_json = status
            .lines()
            .skip_while(|line| *line != "section=retrieval_error_json")
            .nth(1)
            .unwrap();
        let drifted =
            error_json.replace("\"error\":\"backend busy\"", "\"error\":\"backend idle\"");

        assert!(
            experience_retrieval_error_json_summary(&drifted)
                .unwrap_err()
                .contains("user_message drift")
        );
    }
}
