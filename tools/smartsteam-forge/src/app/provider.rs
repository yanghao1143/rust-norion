mod boxed;
mod event;
mod safety;

use std::sync::mpsc::{self, Receiver};

use smartsteam_forge::{SessionFilter, StreamEndpoint};

pub use event::ProviderEvent;

pub trait ChatProvider {
    fn send(&self, prompt: String) -> Receiver<ProviderEvent>;

    fn status(&self) -> String {
        "provider status unavailable".to_owned()
    }

    fn health_check(&self) -> Result<String, String> {
        Ok(self.status())
    }

    fn experience_hygiene(&self) -> Result<String, String> {
        Err("provider does not support experience hygiene checks".to_owned())
    }

    fn experience_hygiene_quarantine_dry_run(&self, _limit: usize) -> Result<String, String> {
        Err("provider does not support experience hygiene quarantine dry-run".to_owned())
    }

    fn experience_repair_dry_run(&self, _limit: usize) -> Result<String, String> {
        Err("provider does not support experience repair dry-run".to_owned())
    }

    fn experience_cleanup_audit(&self, _limit: usize) -> Result<String, String> {
        Err("provider does not support experience cleanup audit".to_owned())
    }

    fn experience_retrieval(&self, _prompt: &str, _limit: usize) -> Result<String, String> {
        Err("provider does not support experience retrieval preview".to_owned())
    }

    fn model_pool_status(&self) -> Result<String, String> {
        Err("provider does not support model pool status checks".to_owned())
    }

    fn model_pool_manifest(&self) -> Result<String, String> {
        Err("provider does not support model pool manifest checks".to_owned())
    }

    fn model_pool_status_async(&self) -> Receiver<Result<String, String>> {
        let (tx, rx) = mpsc::channel();
        let _ = tx.send(self.model_pool_status());
        rx
    }

    fn model_pool_route(&self, _task_kind: &str) -> Result<String, String> {
        Err("provider does not support model pool route planning".to_owned())
    }

    fn model_pool_call(&self, _task_kind: &str, _prompt: &str) -> Result<String, String> {
        Err("provider does not support model pool calls".to_owned())
    }

    fn readiness_check(&self) -> Result<String, String> {
        self.health_check()
    }

    fn prompt_preflight(&self, require_safe_device: bool) -> Result<String, String> {
        let summary = self.readiness_check()?;
        if require_safe_device && safety::unsafe_device_summary(&summary) {
            return Err(format!(
                "safe-device guard blocked prompt: {summary}. Start a GPU-backed Gemma runtime or run /safe-device off for tiny CPU fallback tests."
            ));
        }
        Ok(summary)
    }

    fn safe_device_check(&self) -> Result<String, String> {
        self.prompt_preflight(true)
    }

    fn health_and_readiness(&self) -> (Result<String, String>, Result<String, String>) {
        let health = self.health_check();
        let readiness = match &health {
            Ok(_) => self.readiness_check(),
            Err(error) => Err(error.clone()),
        };
        (health, readiness)
    }

    fn health_readiness_and_safe_device(
        &self,
    ) -> (
        Result<String, String>,
        Result<String, String>,
        Result<String, String>,
    ) {
        let (health, readiness) = self.health_and_readiness();
        let safe_device = match &readiness {
            Ok(_) => self.safe_device_check(),
            Err(error) => Err(error.clone()),
        };
        (health, readiness, safe_device)
    }

    fn diagnostic_target(&self) -> String {
        "target unavailable".to_owned()
    }

    fn settings(&self) -> String {
        "provider settings unavailable".to_owned()
    }

    fn context_preview(&self) -> Result<String, String> {
        Err("provider does not support context preview".to_owned())
    }

    fn sessions(&self, _filter: SessionFilter, _limit: usize) -> String {
        "session listing unavailable".to_owned()
    }

    fn resume_session(&self, _selector: &str) -> Result<String, String> {
        Err("provider does not support session resume".to_owned())
    }

    fn summarize_session(&self, _selector: &str) -> Result<String, String> {
        Err("provider does not support session summary".to_owned())
    }

    fn project_notes(&self) -> Result<String, String> {
        Err("provider does not support project notes".to_owned())
    }

    fn add_project_note(&self, _note: &str) -> Result<String, String> {
        Err("provider does not support project notes".to_owned())
    }

    fn add_model_pool_index_note(&self, note: &str) -> Result<String, String> {
        self.add_project_note(note)
    }

    fn set_project_notes(&self, _notes: &str) -> Result<String, String> {
        Err("provider does not support project notes".to_owned())
    }

    fn clear_project_notes(&self) -> Result<String, String> {
        Err("provider does not support project notes".to_owned())
    }

    fn model_pool_index_notes(&self) -> Result<String, String> {
        Err("provider does not support model-pool index notes".to_owned())
    }

    fn clear_model_pool_index_notes(&self) -> Result<String, String> {
        Err("provider does not support model-pool index notes".to_owned())
    }

    fn record_event(&self, _kind: &str, _content: &str) -> Result<(), String> {
        Ok(())
    }

    fn set_endpoint(&self, _endpoint: StreamEndpoint) -> Result<(), String> {
        Err("provider does not support mode changes".to_owned())
    }

    fn set_output(&self, _output: &str) -> Result<(), String> {
        Err("provider does not support output changes".to_owned())
    }

    fn set_profile(&self, _profile: &str) -> Result<(), String> {
        Err("provider does not support profile changes".to_owned())
    }

    fn set_feedback_amount(&self, _amount: &str) -> Result<(), String> {
        Err("provider does not support feedback changes".to_owned())
    }

    fn set_self_improve(&self, _enabled: bool) -> Result<(), String> {
        Err("provider does not support self-improve changes".to_owned())
    }

    fn set_context_window(&self, _max_messages: usize) -> Result<String, String> {
        Err("provider does not support context window changes".to_owned())
    }

    fn set_max_tokens(&self, _max_tokens: Option<usize>) -> Result<String, String> {
        Err("provider does not support max token changes".to_owned())
    }

    fn set_rust_check_inline(&self, _code: &str) -> Result<String, String> {
        Err("provider does not support rust-check changes".to_owned())
    }

    fn set_rust_check_file(&self, _path: &str) -> Result<String, String> {
        Err("provider does not support rust-check files".to_owned())
    }

    fn set_rust_check_edition(&self, _edition: &str) -> Result<(), String> {
        Err("provider does not support rust-check edition changes".to_owned())
    }

    fn set_rust_check_case(&self, _case_name: Option<String>) -> Result<(), String> {
        Err("provider does not support rust-check cases".to_owned())
    }

    fn clear_rust_check(&self) -> Result<(), String> {
        Err("provider does not support rust-check changes".to_owned())
    }

    fn reset(&self) {}

    fn new_session(&self) -> Result<String, String> {
        self.reset();
        Ok("new session started".to_owned())
    }
}
