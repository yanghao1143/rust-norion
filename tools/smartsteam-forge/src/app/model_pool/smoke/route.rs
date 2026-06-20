use crate::app::provider::ChatProvider;
use crate::app::status_json::json_string_literal;
use model_pool_advice_core::HELPER_ROLES as MODEL_POOL_SMOKE_TASK_KINDS;

use super::super::alignment::RouteSmokeResult;
use super::route_json::ROUTE_SMOKE_JSON_SCHEMA;
use super::summary_bool_value;

pub(super) struct RouteSmokeBatch {
    pub(super) reports: Vec<String>,
    pub(super) results: Vec<RouteSmokeResult>,
}

pub(super) fn collect_route_smoke(provider: &dyn ChatProvider) -> RouteSmokeBatch {
    let mut reports = Vec::new();
    let mut results = Vec::new();
    for task_kind in MODEL_POOL_SMOKE_TASK_KINDS {
        match provider.model_pool_route(task_kind) {
            Ok(route) => {
                let route_allowed = summary_bool_value(&route, "route_allowed");
                reports.push(format!(
                    "route_smoke task_kind={task_kind} ok=true route_allowed={}",
                    option_bool_text(route_allowed)
                ));
                reports.push("section=route_smoke_json".to_owned());
                reports.push(route_smoke_json(task_kind, true, route_allowed, None));
                reports.push(route);
                results.push(RouteSmokeResult {
                    task_kind: task_kind.to_owned(),
                    request_ok: true,
                    route_allowed,
                });
            }
            Err(error) => {
                reports.push(format!(
                    "route_smoke task_kind={task_kind} ok=false route_allowed=unknown error={error}"
                ));
                reports.push("section=route_smoke_json".to_owned());
                reports.push(route_smoke_json(task_kind, false, None, Some(&error)));
                results.push(RouteSmokeResult {
                    task_kind: task_kind.to_owned(),
                    request_ok: false,
                    route_allowed: None,
                });
            }
        }
    }

    RouteSmokeBatch { reports, results }
}

fn route_smoke_json(
    task_kind: &str,
    ok: bool,
    route_allowed: Option<bool>,
    error: Option<&str>,
) -> String {
    format!(
        concat!(
            "{{",
            "\"schema\":{},",
            "\"read_only\":true,",
            "\"launches_process\":false,",
            "\"sends_prompt\":false,",
            "\"task_kind\":{},",
            "\"ok\":{},",
            "\"route_allowed\":{},",
            "\"error\":{}",
            "}}"
        ),
        json_string_literal(ROUTE_SMOKE_JSON_SCHEMA),
        json_string_literal(task_kind),
        bool_json(ok),
        option_bool_json(route_allowed),
        option_string_json(error),
    )
}

fn option_bool_text(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "unknown",
    }
}

fn bool_json(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn option_bool_json(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    }
}

fn option_string_json(value: Option<&str>) -> String {
    value
        .map(json_string_literal)
        .unwrap_or_else(|| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{self, Receiver};

    use super::*;
    use crate::app::provider::ProviderEvent;
    use crate::app::status_json::{json_bool_field, json_string_field};

    #[derive(Clone, Default)]
    struct RouteProvider {
        fail_task: Option<&'static str>,
    }

    impl ChatProvider for RouteProvider {
        fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
            let (_tx, rx) = mpsc::channel();
            rx
        }

        fn model_pool_route(&self, task_kind: &str) -> Result<String, String> {
            if self.fail_task == Some(task_kind) {
                return Err(format!("{task_kind} unavailable"));
            }
            Ok(format!("route task_kind={task_kind}\nroute_allowed=true"))
        }
    }

    #[test]
    fn route_smoke_collects_allowed_helper_routes() {
        let batch = collect_route_smoke(&RouteProvider::default());

        assert_eq!(batch.results.len(), MODEL_POOL_SMOKE_TASK_KINDS.len());
        assert!(batch.results.iter().all(|result| result.request_ok));
        assert!(
            batch
                .results
                .iter()
                .all(|result| result.route_allowed == Some(true))
        );
        assert!(
            batch
                .reports
                .iter()
                .any(|line| line == "route_smoke task_kind=summary ok=true route_allowed=true")
        );
        let summary_json = route_smoke_json_for(&batch, "summary")
            .expect("summary route smoke should include machine-readable JSON");
        assert_eq!(
            json_string_field(summary_json, "schema").as_deref(),
            Some(ROUTE_SMOKE_JSON_SCHEMA)
        );
        assert_eq!(
            json_string_field(summary_json, "task_kind").as_deref(),
            Some("summary")
        );
        assert_eq!(json_bool_field(summary_json, "ok"), Some(true));
        assert_eq!(json_bool_field(summary_json, "route_allowed"), Some(true));
        assert!(summary_json.contains("\"error\":null"));
    }

    #[test]
    fn route_smoke_records_route_errors_without_stopping_batch() {
        let batch = collect_route_smoke(&RouteProvider {
            fail_task: Some("review"),
        });
        let review = batch
            .results
            .iter()
            .find(|result| result.task_kind == "review")
            .expect("review route smoke result should be present");

        assert_eq!(batch.results.len(), MODEL_POOL_SMOKE_TASK_KINDS.len());
        assert!(!review.request_ok);
        assert_eq!(review.route_allowed, None);
        assert!(batch.reports.iter().any(|line| {
            line == "route_smoke task_kind=review ok=false route_allowed=unknown error=review unavailable"
        }));
        let review_json = route_smoke_json_for(&batch, "review")
            .expect("review route error should include machine-readable JSON");
        assert_eq!(
            json_string_field(review_json, "schema").as_deref(),
            Some(ROUTE_SMOKE_JSON_SCHEMA)
        );
        assert_eq!(
            json_string_field(review_json, "task_kind").as_deref(),
            Some("review")
        );
        assert_eq!(json_bool_field(review_json, "ok"), Some(false));
        assert_eq!(
            json_string_field(review_json, "error").as_deref(),
            Some("review unavailable")
        );
        assert!(review_json.contains("\"route_allowed\":null"));
        assert!(
            batch
                .results
                .iter()
                .any(|result| result.task_kind == "summary" && result.request_ok)
        );
    }

    fn route_smoke_json_for<'a>(batch: &'a RouteSmokeBatch, task_kind: &str) -> Option<&'a str> {
        batch.reports.windows(3).find_map(|window| {
            (window[0].starts_with(&format!("route_smoke task_kind={task_kind} "))
                && window[1] == "section=route_smoke_json")
                .then_some(window[2].as_str())
        })
    }
}
