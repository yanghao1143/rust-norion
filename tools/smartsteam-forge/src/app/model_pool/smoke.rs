use std::io::{self, Write};

use crate::app::provider::ChatProvider;

use super::{alignment::ModelPoolSmokeAlignment, model_pool_advice};

mod alignment_json;
mod contract;
mod error_json;
mod json_assert;
mod report;
mod route;
mod route_json;
mod sections;

use error_json::smoke_error_status;
use report::build_model_pool_smoke_report;
pub(in crate::app) use report::validate_model_pool_smoke_report;
use route::collect_route_smoke;

pub(crate) fn run_model_pool_smoke_to<W: Write>(
    provider: &dyn ChatProvider,
    output: &mut W,
) -> io::Result<()> {
    match model_pool_smoke(provider) {
        Ok(report) => {
            model_pool_smoke_events(provider, &report);
            writeln!(output, "{report}")?;
            output.flush()
        }
        Err(error) => {
            model_pool_smoke_error_events(provider, &error);
            Err(io::Error::other(format!(
                "model pool smoke failed: {error}"
            )))
        }
    }
}

pub(crate) fn model_pool_smoke(provider: &dyn ChatProvider) -> Result<String, String> {
    let manifest = provider.model_pool_manifest()?;
    let status = provider.model_pool_status()?;
    let advice = model_pool_advice(&status);
    let route_smoke = collect_route_smoke(provider);
    let alignment =
        ModelPoolSmokeAlignment::from_summaries(&manifest, &status, &route_smoke.results);
    let report =
        build_model_pool_smoke_report(manifest, status, advice, &alignment, route_smoke.reports);
    validate_model_pool_smoke_report(&report)?;
    Ok(report)
}

pub(crate) fn model_pool_smoke_status(report: &str) -> String {
    match summary_bool_value(report, "smoke_alignment_ok")
        .or_else(|| summary_bool_value(report, "alignment_ok"))
    {
        Some(alignment_ok) => format!("model pool smoke alignment_ok={}", bool_text(alignment_ok)),
        None => "model pool smoke complete".to_owned(),
    }
}

pub(crate) fn model_pool_smoke_contract_status(report: &str) -> String {
    match validate_model_pool_smoke_report(report) {
        Ok(()) => "model pool smoke contract_ok=true".to_owned(),
        Err(error) => format!("model pool smoke contract_ok=false error={error}"),
    }
}

pub(crate) fn model_pool_smoke_events(provider: &dyn ChatProvider, report: &str) -> String {
    let _ = provider.record_event("model_pool_smoke", report);
    let contract_status = model_pool_smoke_contract_status(report);
    let _ = provider.record_event("model_pool_smoke_contract", &contract_status);
    contract_status
}

pub(crate) fn model_pool_smoke_error_events(provider: &dyn ChatProvider, error: &str) -> String {
    let status = smoke_error_status(error);
    let _ = provider.record_event("model_pool_smoke_error", &status);
    let _ = provider.record_event("model_pool_smoke_contract", &status);
    status
}

fn summary_bool_value(summary: &str, key: &str) -> Option<bool> {
    summary
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .and_then(parse_bool)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    };

    use super::error_json::{SmokeErrorJsonSummary, smoke_error_json_summary};
    use super::*;
    use crate::app::provider::ProviderEvent;

    #[derive(Clone, Default)]
    struct SmokeProvider {
        events: Arc<Mutex<Vec<(String, String)>>>,
        fail_manifest: bool,
    }

    impl ChatProvider for SmokeProvider {
        fn send(&self, _prompt: String) -> Receiver<ProviderEvent> {
            let (_tx, rx) = mpsc::channel();
            rx
        }

        fn model_pool_status(&self) -> Result<String, String> {
            Ok("pool ok".to_owned())
        }

        fn model_pool_manifest(&self) -> Result<String, String> {
            if self.fail_manifest {
                return Err("manifest unavailable".to_owned());
            }
            Ok("manifest ok".to_owned())
        }

        fn model_pool_route(&self, task_kind: &str) -> Result<String, String> {
            Ok(format!("route task_kind={task_kind}"))
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
    fn smoke_cli_prints_status_advice_and_helper_routes_without_pool_calls() {
        let provider = SmokeProvider::default();
        let mut output = Vec::new();

        run_model_pool_smoke_to(&provider, &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        let events = provider.events.lock().unwrap();

        assert!(output.contains("SmartSteam model pool smoke"));
        assert!(output.contains("smoke_alignment_ok=false"));
        assert!(output.contains("contract_ok=true"));
        assert!(output.contains("section=contract_json"));
        assert!(output.contains("\"schema\":\"smartsteam.forge.model_pool_smoke_contract.v1\""));
        let smoke_alignment_line = output
            .lines()
            .position(|line| line == "smoke_alignment_ok=false")
            .expect("smoke_alignment_ok line should be in the report header");
        let manifest_section_line = output
            .lines()
            .position(|line| line == "section=manifest")
            .expect("manifest section should be present");
        assert!(smoke_alignment_line < manifest_section_line);
        assert!(output.contains("section=manifest"));
        assert!(output.contains("manifest ok"));
        assert!(output.contains("section=status"));
        assert!(output.contains("section=advice"));
        assert!(output.contains("section=alignment_json"));
        assert!(output.contains("\"schema\":\"smartsteam.forge.model_pool_smoke_alignment.v1\""));
        assert!(output.contains("\"read_only\":true"));
        assert!(output.contains("\"launches_process\":false"));
        assert!(output.contains("\"sends_prompt\":false"));
        assert!(output.contains("section=alignment"));
        assert!(output.contains("alignment_ok=false"));
        assert!(output.contains("unexpected_manifest_roles=none"));
        assert!(output.contains("unexpected_status_roles=none"));
        assert!(output.contains("extra_quality_12b_detected=false"));
        assert!(output.contains("helper_worker_count_aligned=false"));
        assert!(
            output.contains("missing_manifest_helper_roles=summary,router,review,index,test-gate")
        );
        assert!(output.contains("missing_route_smoke_tasks=none"));
        assert!(output.contains("unexpected_route_smoke_tasks=none"));
        assert!(output.contains("section=routes"));
        assert!(output.contains("route_smoke task_kind=summary ok=true route_allowed=unknown"));
        assert!(output.contains("section=route_smoke_json"));
        assert!(output.contains("\"schema\":\"smartsteam.forge.model_pool_route_smoke.v1\""));
        assert!(output.contains("route task_kind=review"));
        assert!(output.contains("route_smoke task_kind=index ok=true"));
        assert!(output.contains("route_smoke task_kind=test-gate ok=true"));
        assert!(output.contains("sends_prompt=false"));
        assert!(events.iter().any(|(kind, content)| {
            kind == "model_pool_smoke" && content.contains("SmartSteam model pool smoke")
        }));
        assert!(events.iter().any(|(kind, content)| {
            kind == "model_pool_smoke_contract" && content == "model pool smoke contract_ok=true"
        }));
    }

    #[test]
    fn smoke_status_reports_alignment_result() {
        assert_eq!(
            model_pool_smoke_status("SmartSteam model pool smoke\nsmoke_alignment_ok=true"),
            "model pool smoke alignment_ok=true"
        );
        assert_eq!(
            model_pool_smoke_status("SmartSteam model pool smoke\nsmoke_alignment_ok=false"),
            "model pool smoke alignment_ok=false"
        );
        assert_eq!(
            model_pool_smoke_status("SmartSteam model pool smoke"),
            "model pool smoke complete"
        );
    }

    #[test]
    fn smoke_status_prefers_header_and_keeps_legacy_alignment_fallback() {
        assert_eq!(
            model_pool_smoke_status(
                "SmartSteam model pool smoke\nsmoke_alignment_ok=true\nsection=alignment\nalignment_ok=false"
            ),
            "model pool smoke alignment_ok=true"
        );
        assert_eq!(
            model_pool_smoke_status(
                "SmartSteam model pool smoke\nsection=alignment\nalignment_ok=false"
            ),
            "model pool smoke alignment_ok=false"
        );
    }

    #[test]
    fn smoke_contract_status_reports_validation_result() {
        let report = model_pool_smoke(&SmokeProvider::default()).unwrap();

        assert_eq!(
            model_pool_smoke_contract_status(&report),
            "model pool smoke contract_ok=true"
        );
        assert!(
            model_pool_smoke_contract_status("SmartSteam model pool smoke")
                .contains("contract_ok=false")
        );
    }

    #[test]
    fn smoke_events_record_report_and_contract_status_together() {
        let provider = SmokeProvider::default();
        let report = model_pool_smoke(&provider).unwrap();

        let status = model_pool_smoke_events(&provider, &report);
        let events = provider.events.lock().unwrap();

        assert_eq!(status, "model pool smoke contract_ok=true");
        assert!(events.iter().any(|(kind, content)| {
            kind == "model_pool_smoke" && content.contains("SmartSteam model pool smoke")
        }));
        assert!(events.iter().any(|(kind, content)| {
            kind == "model_pool_smoke_contract" && content == "model pool smoke contract_ok=true"
        }));
    }

    #[test]
    fn smoke_error_events_record_error_and_contract_failure_context() {
        let provider = SmokeProvider {
            fail_manifest: true,
            ..SmokeProvider::default()
        };
        let mut output = Vec::new();

        let error = run_model_pool_smoke_to(&provider, &mut output).unwrap_err();
        let events = provider.events.lock().unwrap();

        assert!(error.to_string().contains("manifest unavailable"));
        assert!(output.is_empty());
        assert!(events.iter().any(|(kind, content)| {
            kind == "model_pool_smoke_error"
                && content.contains("model pool smoke contract_ok=false")
                && content.contains("section=smoke_error_json")
        }));
        assert!(events.iter().any(|(kind, content)| {
            kind == "model_pool_smoke_contract"
                && content.contains("model pool smoke contract_ok=false error=manifest unavailable")
                && content.contains("section=smoke_error_json")
        }));
    }

    #[test]
    fn smoke_error_events_return_machine_readable_contract_status() {
        let provider = SmokeProvider::default();

        let status = model_pool_smoke_error_events(&provider, "manifest unavailable");

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
}
