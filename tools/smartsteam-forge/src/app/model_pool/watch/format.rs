use crate::app::status_json::json_string_literal;
#[cfg(test)]
use crate::app::status_json::{
    require_json_bool_equals, require_json_string_equals, required_json_number,
    required_json_string,
};

const WATCH_ITERATION_JSON_SCHEMA: &str = "smartsteam.forge.model_pool_watch_iteration.v1";
const WATCH_ERROR_JSON_SCHEMA: &str = "smartsteam.forge.model_pool_watch_error.v1";

#[derive(Debug, PartialEq, Eq)]
#[cfg(test)]
pub(super) struct WatchIterationJsonSummary {
    pub(super) iteration: String,
}

#[derive(Debug, PartialEq, Eq)]
#[cfg(test)]
pub(super) struct WatchErrorJsonSummary {
    pub(super) iteration: String,
    pub(super) error: String,
}

pub(super) fn watch_iteration_line(iteration: usize) -> String {
    format!("model_pool_watch iteration={iteration}")
}

pub(super) fn watch_iteration_report(iteration: usize) -> String {
    [
        watch_iteration_line(iteration),
        "section=watch_iteration_json".to_owned(),
        watch_iteration_json(iteration),
    ]
    .join("\n")
}

pub(super) fn watch_error_report(iteration: usize, error: &str) -> String {
    [
        format!("model_pool_watch_error iteration={iteration} error={error}"),
        "section=watch_error_json".to_owned(),
        watch_error_json(iteration, error),
    ]
    .join("\n")
}

fn watch_iteration_json(iteration: usize) -> String {
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":true,",
            "\"launches_process\":false,",
            "\"sends_prompt\":false,",
            "\"iteration\":{}",
            "}}"
        ),
        json_string_literal(WATCH_ITERATION_JSON_SCHEMA),
        iteration,
    )
}

fn watch_error_json(iteration: usize, error: &str) -> String {
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":true,",
            "\"launches_process\":false,",
            "\"sends_prompt\":false,",
            "\"iteration\":{},",
            "\"error\":{}",
            "}}"
        ),
        json_string_literal(WATCH_ERROR_JSON_SCHEMA),
        iteration,
        json_string_literal(error),
    )
}

#[cfg(test)]
pub(super) fn watch_iteration_json_summary(
    iteration_json: &str,
) -> Result<WatchIterationJsonSummary, String> {
    require_json_string_equals(
        iteration_json,
        "schema",
        WATCH_ITERATION_JSON_SCHEMA,
        "model pool watch iteration JSON schema",
    )?;
    require_common_read_only_flags(iteration_json, "model pool watch iteration JSON")?;
    let iteration = required_json_number(
        iteration_json,
        "iteration",
        "model pool watch iteration JSON iteration",
    )?;

    Ok(WatchIterationJsonSummary { iteration })
}

#[cfg(test)]
pub(super) fn watch_error_json_summary(error_json: &str) -> Result<WatchErrorJsonSummary, String> {
    require_json_string_equals(
        error_json,
        "schema",
        WATCH_ERROR_JSON_SCHEMA,
        "model pool watch error JSON schema",
    )?;
    require_common_read_only_flags(error_json, "model pool watch error JSON")?;
    let iteration = required_json_number(
        error_json,
        "iteration",
        "model pool watch error JSON iteration",
    )?;
    let error = required_json_string(error_json, "error", "model pool watch error JSON error")?;

    Ok(WatchErrorJsonSummary { iteration, error })
}

#[cfg(test)]
fn require_common_read_only_flags(object: &str, label: &str) -> Result<(), String> {
    require_json_bool_equals(object, "read_only", true, &format!("{label} read_only"))?;
    require_json_bool_equals(
        object,
        "launches_process",
        false,
        &format!("{label} launches_process"),
    )?;
    require_json_bool_equals(
        object,
        "sends_prompt",
        false,
        &format!("{label} sends_prompt"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iteration_line_keeps_watch_prefix_and_counter() {
        assert_eq!(watch_iteration_line(7), "model_pool_watch iteration=7");
    }

    #[test]
    fn iteration_report_adds_machine_readable_counter_without_side_effects() {
        let report = watch_iteration_report(7);

        assert!(report.starts_with("model_pool_watch iteration=7"));
        assert!(report.contains("section=watch_iteration_json"));
        let iteration_json = report
            .lines()
            .skip_while(|line| *line != "section=watch_iteration_json")
            .nth(1)
            .expect("watch_iteration_json section should include a JSON payload line");
        assert_eq!(
            watch_iteration_json_summary(iteration_json).unwrap(),
            WatchIterationJsonSummary {
                iteration: "7".to_owned()
            }
        );
    }

    #[test]
    fn error_line_keeps_iteration_and_error_text() {
        let report = watch_error_report(3, "backend busy");

        assert!(report.contains("model_pool_watch_error iteration=3 error=backend busy"));
        assert!(report.contains("section=watch_error_json"));
        assert!(report.contains("\"schema\":\"smartsteam.forge.model_pool_watch_error.v1\""));
        assert!(report.contains("\"iteration\":3"));
        let error_json = report
            .lines()
            .skip_while(|line| *line != "section=watch_error_json")
            .nth(1)
            .expect("watch_error_json section should include a JSON payload line");

        assert_eq!(
            watch_error_json_summary(error_json).unwrap(),
            WatchErrorJsonSummary {
                iteration: "3".to_owned(),
                error: "backend busy".to_owned(),
            }
        );
    }

    #[test]
    fn watch_json_summaries_reject_schema_and_side_effect_drift() {
        let iteration = watch_iteration_json(1);
        let wrong_schema = iteration.replacen(
            "\"schema\":\"smartsteam.forge.model_pool_watch_iteration.v1\"",
            "\"schema\":\"wrong.v1\"",
            1,
        );
        let sends_prompt = watch_error_json(1, "backend busy").replacen(
            "\"sends_prompt\":false",
            "\"sends_prompt\":true",
            1,
        );

        assert!(
            watch_iteration_json_summary(&wrong_schema)
                .unwrap_err()
                .contains("watch iteration JSON schema")
        );
        assert!(
            watch_error_json_summary(&sends_prompt)
                .unwrap_err()
                .contains("watch error JSON sends_prompt")
        );
    }

    #[test]
    fn watch_error_json_summary_rejects_empty_error() {
        let value = watch_error_json(1, "backend busy").replacen(
            "\"error\":\"backend busy\"",
            "\"error\":\"\"",
            1,
        );

        assert!(
            watch_error_json_summary(&value)
                .unwrap_err()
                .contains("watch error JSON error")
        );
    }
}
