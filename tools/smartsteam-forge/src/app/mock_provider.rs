use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread,
    time::Duration,
};

use model_pool_advice_core::{CAPACITY_POLICY, HELPER_ROLES, RECOMMENDED_LAUNCH_ROLES};
use smartsteam_forge::{SessionFilter, StreamEndpoint};

use super::provider::{ChatProvider, ProviderEvent};

const DEFAULT_CONTEXT_WINDOW: usize = 64;
const DEFAULT_MAX_TOKENS: usize = 4096;

#[derive(Clone)]
pub struct MockProvider {
    endpoint: Arc<Mutex<StreamEndpoint>>,
    context_window: Arc<Mutex<usize>>,
    max_tokens: Arc<Mutex<Option<usize>>>,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self {
            endpoint: Arc::new(Mutex::new(StreamEndpoint::Chat)),
            context_window: Arc::new(Mutex::new(DEFAULT_CONTEXT_WINDOW)),
            max_tokens: Arc::new(Mutex::new(Some(DEFAULT_MAX_TOKENS))),
        }
    }
}

impl ChatProvider for MockProvider {
    fn send(&self, prompt: String) -> Receiver<ProviderEvent> {
        let (tx, rx) = mpsc::channel();
        let endpoint = self
            .endpoint
            .lock()
            .map(|endpoint| *endpoint)
            .unwrap_or(StreamEndpoint::Chat);

        thread::spawn(move || match endpoint {
            StreamEndpoint::BusinessCycle => stream_mock_business_cycle(prompt, tx),
            StreamEndpoint::Chat | StreamEndpoint::Generate => stream_mock_text(prompt, tx),
        });

        rx
    }

    fn status(&self) -> String {
        "mock provider ready".to_owned()
    }

    fn health_check(&self) -> Result<String, String> {
        Ok(self.status())
    }

    fn experience_hygiene(&self) -> Result<String, String> {
        Ok("Noiron experience hygiene\nmock provider: no backend state".to_owned())
    }

    fn experience_hygiene_quarantine_dry_run(&self, limit: usize) -> Result<String, String> {
        Ok(format!(
            "Noiron experience hygiene quarantine dry-run\nmock provider: no backend state\nlimit={}",
            limit.max(1)
        ))
    }

    fn experience_repair_dry_run(&self, limit: usize) -> Result<String, String> {
        Ok(format!(
            "Noiron experience repair dry-run\nmock provider: no backend state\nlimit={}\napply=false",
            limit.max(1)
        ))
    }

    fn experience_cleanup_audit(&self, limit: usize) -> Result<String, String> {
        let limit = limit.max(1);
        Ok(format!(
            "Noiron experience cleanup audit\nmock provider: no backend state\nwrites_experience_state=false\nsample_limit={limit}\n\n## Hygiene\nmock clean\n\n## Quarantine dry-run\napply=false\n\n## Repair dry-run\napply=false"
        ))
    }

    fn experience_retrieval(&self, prompt: &str, limit: usize) -> Result<String, String> {
        Ok(format!(
            "Noiron experience retrieval preview\nmock provider: no backend state\nprompt={prompt}\ntotal_records=1\nrequested_limit={}\nmatch_count=1\nmatches=1\nmatch id=mock runtime_model=mock-gemma-12b runtime_adapter=mock runtime_device=mock-gpu runtime_primary_lane=quality runtime_fallback_lane=summary runtime_memory_mode=kv runtime_device_execution_source=mock-gpu runtime_forward_energy=0.25 runtime_kv_influence=0.50 runtime_uncertainty_perplexity=1.00 recursive_runtime_calls=1",
            limit.max(1)
        ))
    }

    fn model_pool_status(&self) -> Result<String, String> {
        Ok("SmartSteam model pool status\nmock provider: no backend state\nread_only=true\nlaunches_process=false\nsends_prompt=false\nlaunch_allowed=false\nreason=mock".to_owned())
    }

    fn model_pool_manifest(&self) -> Result<String, String> {
        Ok(format!(
            "SmartSteam model pool manifest\nmock provider: no backend state\nread_only=true\nlaunches_process=false\nsends_prompt=false\ncontract_version=gemma-chain.v1\nmanifest_kind=rust-norion.model-pool\ncapacity_policy policy={} target_host=apple_silicon avoid_extra_12b=true max_quality_12b_workers=1 helper_roles={} recommended_launch_order={}\nmanifest_workers=0",
            CAPACITY_POLICY,
            HELPER_ROLES.join(","),
            RECOMMENDED_LAUNCH_ROLES.join(",")
        ))
    }

    fn model_pool_route(&self, task_kind: &str) -> Result<String, String> {
        Ok(format!(
            "SmartSteam model pool route plan\nmock provider: no backend state\ntask_kind={task_kind}\nread_only=true\nlaunches_process=false\nsends_prompt=false\nroute_allowed=false\nreason=mock"
        ))
    }

    fn diagnostic_target(&self) -> String {
        "mock provider (offline)".to_owned()
    }

    fn settings(&self) -> String {
        let endpoint = self
            .endpoint
            .lock()
            .map(|endpoint| endpoint.label())
            .unwrap_or("unknown");
        let context_window = self
            .context_window
            .lock()
            .map(|context_window| *context_window)
            .unwrap_or(DEFAULT_CONTEXT_WINDOW);
        let max_tokens = self
            .max_tokens
            .lock()
            .map(|max_tokens| {
                max_tokens
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "backend-default".to_owned())
            })
            .unwrap_or_else(|_| "unknown".to_owned());
        format!(
            "mock mode={endpoint} output=raw max_tokens={max_tokens} profile=coding feedback=0.5 self_improve=true context_window={context_window} rust_check=accepted"
        )
    }

    fn sessions(&self, filter: SessionFilter, limit: usize) -> String {
        format!(
            "mock sessions: transcript disabled filter={} limit={}",
            filter.label(),
            limit
        )
    }

    fn resume_session(&self, selector: &str) -> Result<String, String> {
        Ok(format!("mock resumed selector={selector}"))
    }

    fn summarize_session(&self, selector: &str) -> Result<String, String> {
        Ok(format!("mock summary selector={selector}"))
    }

    fn set_endpoint(&self, endpoint: StreamEndpoint) -> Result<(), String> {
        *self
            .endpoint
            .lock()
            .map_err(|error| format!("mock endpoint lock poisoned: {error}"))? = endpoint;
        Ok(())
    }

    fn set_output(&self, _output: &str) -> Result<(), String> {
        Ok(())
    }

    fn set_profile(&self, _profile: &str) -> Result<(), String> {
        Ok(())
    }

    fn set_feedback_amount(&self, _amount: &str) -> Result<(), String> {
        Ok(())
    }

    fn set_self_improve(&self, _enabled: bool) -> Result<(), String> {
        Ok(())
    }

    fn set_context_window(&self, max_messages: usize) -> Result<String, String> {
        let max_messages = max_messages.max(2);
        *self
            .context_window
            .lock()
            .map_err(|error| format!("mock context window lock poisoned: {error}"))? = max_messages;
        Ok(format!(
            "context_budget: mock max_context_messages={max_messages}"
        ))
    }

    fn set_max_tokens(&self, max_tokens: Option<usize>) -> Result<String, String> {
        *self
            .max_tokens
            .lock()
            .map_err(|error| format!("mock max tokens lock poisoned: {error}"))? = max_tokens;
        Ok(format!(
            "max_tokens={}",
            max_tokens
                .map(|value| value.to_string())
                .unwrap_or_else(|| "backend-default".to_owned())
        ))
    }

    fn set_rust_check_inline(&self, code: &str) -> Result<String, String> {
        Ok(format!(
            "mock rust_check inline accepted chars={}",
            code.chars().count()
        ))
    }

    fn set_rust_check_file(&self, path: &str) -> Result<String, String> {
        Ok(format!("mock rust_check file accepted path={path}"))
    }

    fn set_rust_check_edition(&self, _edition: &str) -> Result<(), String> {
        Ok(())
    }

    fn set_rust_check_case(&self, _case_name: Option<String>) -> Result<(), String> {
        Ok(())
    }

    fn clear_rust_check(&self) -> Result<(), String> {
        Ok(())
    }

    fn new_session(&self) -> Result<String, String> {
        Ok("mock transcript=disabled".to_owned())
    }
}

fn stream_mock_text(prompt: String, tx: mpsc::Sender<ProviderEvent>) {
    let response = format!(
        "SmartSteam Forge mock stream received: {prompt}. Provider can later route this to rust-norion /v1/chat-stream."
    );

    for chunk in response.split_whitespace() {
        if tx.send(ProviderEvent::Delta(format!("{chunk} "))).is_err() {
            return;
        }
        thread::sleep(Duration::from_millis(35));
    }

    let _ = tx.send(ProviderEvent::Done);
}

fn stream_mock_business_cycle(prompt: String, tx: mpsc::Sender<ProviderEvent>) {
    if tx
        .send(ProviderEvent::Stage(
            "mock business-cycle generate/feedback/rust-check".to_owned(),
        ))
        .is_err()
    {
        return;
    }

    let response = format!("Mock business-cycle accepted: {prompt}. ");
    for chunk in response.split_whitespace() {
        if tx.send(ProviderEvent::Delta(format!("{chunk} "))).is_err() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }

    let final_payload = mock_business_cycle_final_payload(&prompt);
    let summary = smartsteam_forge::FinalPayloadSummary::parse(&final_payload);
    let _ = tx.send(ProviderEvent::Status(summary.status_line()));
    if let Some(report) = summary.gate_report() {
        let _ = tx.send(ProviderEvent::GateReport(report));
    }
    let _ = tx.send(ProviderEvent::ReplaceAssistant(
        summary
            .answer()
            .unwrap_or("Mock business-cycle completed.")
            .to_owned(),
    ));
    let _ = tx.send(ProviderEvent::Done);
}

fn mock_business_cycle_final_payload(prompt: &str) -> String {
    let answer = format!("Mock business-cycle completed for: {prompt}");
    format!(
        "{{\"ok\":true,\"business_cycle\":{{\"passed\":true,\"generate_passed\":true,\"feedback_passed\":true,\"feedback_applied\":1,\"rust_check_checked\":true,\"rust_check_passed\":true,\"rust_check_feedback_applied\":1,\"self_improve_checked\":true,\"self_improve_passed\":true,\"state_gate_checked\":true,\"state_gate_passed\":true,\"trace_gate_checked\":true,\"trace_gate_passed\":true}},\"generate\":{{\"answer\":{},\"runtime_model\":\"mock-gemma-12b\",\"runtime_token_count\":24,\"runtime_uncertainty_signal\":false,\"runtime_device_execution_source\":\"mock-gpu\"}}}}",
        mock_json_string(&answer)
    )
}

fn mock_json_string(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other if other.is_control() => escaped.push_str(&format!("\\u{:04x}", other as u32)),
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_provider_streams_business_cycle_gate_report() {
        let provider = MockProvider::default();
        provider
            .set_endpoint(StreamEndpoint::BusinessCycle)
            .unwrap();

        let rx = provider.send("检查业务闭环".to_owned());
        let events = rx.iter().collect::<Vec<_>>();

        assert!(
            events
                .iter()
                .any(|event| matches!(event, ProviderEvent::Stage(_)))
        );
        assert!(events.iter().any(|event| {
            matches!(event, ProviderEvent::Status(status) if status.contains("feedback_applied_count=1"))
        }));
        assert!(events.iter().any(|event| {
            matches!(event, ProviderEvent::GateReport(report) if report.contains("overall: PASS") && report.contains("feedback applied: PASS count=1"))
        }));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ProviderEvent::Done))
        );
    }

    #[test]
    fn mock_provider_retrieval_previews_runtime_diagnostics() {
        let provider = MockProvider::default();

        let summary = provider.experience_retrieval("route planning", 0).unwrap();
        let parsed =
            crate::app::retrieval_preview::experience_retrieval_preview_summary(&summary).unwrap();

        assert_eq!(parsed.total_records, Some(1));
        assert!(summary.contains("requested_limit=1"));
        assert!(summary.contains("match_count=1"));
        assert!(summary.contains("matches=1"));
        assert!(summary.contains("runtime_model=mock-gemma-12b"));
        assert!(summary.contains("runtime_adapter=mock"));
        assert!(summary.contains("runtime_device=mock-gpu"));
        assert!(summary.contains("runtime_device_execution_source=mock-gpu"));
        assert!(summary.contains("runtime_forward_energy=0.25"));
        assert!(summary.contains("runtime_kv_influence=0.50"));
        assert_eq!(
            parsed
                .matches
                .first()
                .and_then(|item| item.recursive_runtime_calls),
            Some(1)
        );
    }
}
