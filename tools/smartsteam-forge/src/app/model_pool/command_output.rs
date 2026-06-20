use std::io::{self, Write};

use crate::app::provider::ChatProvider;
use crate::app::status_json::json_string_literal;
#[cfg(test)]
use crate::app::status_json::{
    require_json_bool_equals, require_json_string_equals, required_json_string,
};

const MODEL_POOL_ERROR_JSON_SCHEMA: &str = "smartsteam.forge.model_pool_error.v1";

#[derive(Debug, PartialEq, Eq)]
#[cfg(test)]
pub(super) struct ModelPoolErrorJsonSummary {
    pub(super) action: String,
    pub(super) error: String,
    pub(super) user_message: String,
}

pub(super) fn record_and_write_summary<W: Write>(
    provider: &dyn ChatProvider,
    event_kind: &str,
    summary: &str,
    output: &mut W,
) -> io::Result<()> {
    let _ = provider.record_event(event_kind, summary);
    writeln!(output, "{summary}")
}

pub(super) fn evented_error(
    provider: &dyn ChatProvider,
    event_kind: &str,
    action: &str,
    error: &str,
) -> io::Error {
    let summary = model_pool_error_summary(action, error);
    let _ = provider.record_event(event_kind, &summary);
    io::Error::other(format!("model pool {action} failed: {error}"))
}

pub(in crate::app) fn model_pool_error_summary(action: &str, error: &str) -> String {
    let user_message = format!("model pool {action} failed: {error}");
    [
        format!("model_pool_error action={action} error={error}"),
        "section=error_json".to_owned(),
        model_pool_error_json(action, error, &user_message),
    ]
    .join("\n")
}

fn model_pool_error_json(action: &str, error: &str, user_message: &str) -> String {
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":true,",
            "\"launches_process\":false,",
            "\"sends_prompt\":false,",
            "\"action\":{},",
            "\"error\":{},",
            "\"user_message\":{}",
            "}}"
        ),
        json_string_literal(MODEL_POOL_ERROR_JSON_SCHEMA),
        json_string_literal(action),
        json_string_literal(error),
        json_string_literal(user_message),
    )
}

#[cfg(test)]
pub(super) fn validate_model_pool_error_json(error_json: &str) -> Result<(), String> {
    model_pool_error_json_summary(error_json).map(|_| ())
}

#[cfg(test)]
pub(super) fn model_pool_error_json_summary(
    error_json: &str,
) -> Result<ModelPoolErrorJsonSummary, String> {
    require_json_string_equals(
        error_json,
        "schema",
        MODEL_POOL_ERROR_JSON_SCHEMA,
        "model pool error JSON schema",
    )?;
    require_json_bool_equals(
        error_json,
        "read_only",
        true,
        "model pool error JSON read_only",
    )?;
    require_json_bool_equals(
        error_json,
        "launches_process",
        false,
        "model pool error JSON launches_process",
    )?;
    require_json_bool_equals(
        error_json,
        "sends_prompt",
        false,
        "model pool error JSON sends_prompt",
    )?;
    let action = required_json_string(error_json, "action", "model pool error JSON action")?;
    validate_model_pool_error_action(&action)?;
    let error = required_json_string(error_json, "error", "model pool error JSON error")?;
    let user_message = required_json_string(
        error_json,
        "user_message",
        "model pool error JSON user_message",
    )?;
    validate_model_pool_error_user_message(&action, &error, &user_message)?;

    Ok(ModelPoolErrorJsonSummary {
        action,
        error,
        user_message,
    })
}

#[cfg(test)]
fn validate_model_pool_error_action(action: &str) -> Result<(), String> {
    match action {
        "status" | "manifest" | "advice" | "route" | "call" => Ok(()),
        _ => Err(format!("model pool error JSON unknown action {action:?}")),
    }
}

#[cfg(test)]
fn validate_model_pool_error_user_message(
    action: &str,
    error: &str,
    user_message: &str,
) -> Result<(), String> {
    let expected = format!("model pool {action} failed: {error}");
    if user_message == expected {
        return Ok(());
    }
    Err(format!(
        "model pool error JSON user_message drift: expected {expected:?}, got {user_message:?}"
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    };

    use super::*;
    use crate::app::provider::ProviderEvent;

    #[derive(Clone, Default)]
    struct OutputProvider {
        events: Arc<Mutex<Vec<(String, String)>>>,
    }

    impl ChatProvider for OutputProvider {
        fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
            let (_tx, rx) = mpsc::channel();
            rx
        }

        fn record_event(&self, kind: &str, content: &str) -> Result<(), String> {
            self.events
                .lock()
                .unwrap()
                .push((kind.to_owned(), content.to_owned()));
            Ok(())
        }
    }

    #[test]
    fn record_and_write_summary_records_event_and_prints_line() {
        let provider = OutputProvider::default();
        let mut output = Vec::new();

        record_and_write_summary(&provider, "model_pool_status", "pool ok", &mut output).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "pool ok\n");
        assert_eq!(
            provider.events.lock().unwrap().as_slice(),
            &[("model_pool_status".to_owned(), "pool ok".to_owned())]
        );
    }

    #[test]
    fn evented_error_records_error_and_preserves_action_context() {
        let provider = OutputProvider::default();

        let error = evented_error(
            &provider,
            "model_pool_status_error",
            "status",
            "backend busy",
        );

        assert_eq!(error.to_string(), "model pool status failed: backend busy");
        let events = provider.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "model_pool_status_error");
        assert!(events[0].1.contains("model_pool_error action=status"));
        assert!(events[0].1.contains("section=error_json"));
        let error_json = events[0]
            .1
            .lines()
            .skip_while(|line| *line != "section=error_json")
            .nth(1)
            .expect("error_json section should include a JSON payload line");
        let summary = model_pool_error_json_summary(error_json).unwrap();
        assert_eq!(
            summary,
            ModelPoolErrorJsonSummary {
                action: "status".to_owned(),
                error: "backend busy".to_owned(),
                user_message: "model pool status failed: backend busy".to_owned(),
            }
        );
    }

    #[test]
    fn error_json_validation_rejects_schema_and_side_effect_drift() {
        let json = model_pool_error_json("status", "backend busy", "model pool status failed");
        let wrong_schema = json.replacen(
            "\"schema\":\"smartsteam.forge.model_pool_error.v1\"",
            "\"schema\":\"wrong.v1\"",
            1,
        );
        let launches_process =
            json.replacen("\"launches_process\":false", "\"launches_process\":true", 1);

        assert!(
            validate_model_pool_error_json(&wrong_schema)
                .unwrap_err()
                .contains("error JSON schema")
        );
        assert!(
            validate_model_pool_error_json(&launches_process)
                .unwrap_err()
                .contains("error JSON launches_process")
        );
    }

    #[test]
    fn error_json_validation_rejects_missing_user_message() {
        let json = model_pool_error_json("status", "backend busy", "model pool status failed")
            .replacen(
                "\"user_message\":\"model pool status failed\"",
                "\"user_message\":\"\"",
                1,
            );

        assert!(
            validate_model_pool_error_json(&json)
                .unwrap_err()
                .contains("error JSON user_message")
        );
    }

    #[test]
    fn error_json_validation_rejects_unknown_action() {
        let json = model_pool_error_json(
            "status",
            "backend busy",
            "model pool status failed: backend busy",
        )
        .replacen("\"action\":\"status\"", "\"action\":\"restart\"", 1);

        assert!(
            validate_model_pool_error_json(&json)
                .unwrap_err()
                .contains("unknown action")
        );
    }

    #[test]
    fn error_json_validation_rejects_user_message_drift() {
        let json = model_pool_error_json(
            "status",
            "backend busy",
            "model pool status failed: backend busy",
        )
        .replacen(
            "\"error\":\"backend busy\"",
            "\"error\":\"backend idle\"",
            1,
        );

        assert!(
            validate_model_pool_error_json(&json)
                .unwrap_err()
                .contains("user_message drift")
        );
    }
}
