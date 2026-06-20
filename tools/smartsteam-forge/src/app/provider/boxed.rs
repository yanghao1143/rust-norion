use std::sync::mpsc::Receiver;

use smartsteam_forge::{SessionFilter, StreamEndpoint};

use super::{ChatProvider, ProviderEvent};

impl<T: ChatProvider + ?Sized> ChatProvider for Box<T> {
    fn send(&self, prompt: String) -> Receiver<ProviderEvent> {
        (**self).send(prompt)
    }

    fn status(&self) -> String {
        (**self).status()
    }

    fn health_check(&self) -> Result<String, String> {
        (**self).health_check()
    }

    fn experience_hygiene(&self) -> Result<String, String> {
        (**self).experience_hygiene()
    }

    fn experience_hygiene_quarantine_dry_run(&self, limit: usize) -> Result<String, String> {
        (**self).experience_hygiene_quarantine_dry_run(limit)
    }

    fn experience_repair_dry_run(&self, limit: usize) -> Result<String, String> {
        (**self).experience_repair_dry_run(limit)
    }

    fn experience_cleanup_audit(&self, limit: usize) -> Result<String, String> {
        (**self).experience_cleanup_audit(limit)
    }

    fn experience_retrieval(&self, prompt: &str, limit: usize) -> Result<String, String> {
        (**self).experience_retrieval(prompt, limit)
    }

    fn model_pool_status(&self) -> Result<String, String> {
        (**self).model_pool_status()
    }

    fn model_pool_manifest(&self) -> Result<String, String> {
        (**self).model_pool_manifest()
    }

    fn model_pool_status_async(&self) -> Receiver<Result<String, String>> {
        (**self).model_pool_status_async()
    }

    fn model_pool_route(&self, task_kind: &str) -> Result<String, String> {
        (**self).model_pool_route(task_kind)
    }

    fn model_pool_call(&self, task_kind: &str, prompt: &str) -> Result<String, String> {
        (**self).model_pool_call(task_kind, prompt)
    }

    fn readiness_check(&self) -> Result<String, String> {
        (**self).readiness_check()
    }

    fn prompt_preflight(&self, require_safe_device: bool) -> Result<String, String> {
        (**self).prompt_preflight(require_safe_device)
    }

    fn safe_device_check(&self) -> Result<String, String> {
        (**self).safe_device_check()
    }

    fn health_and_readiness(&self) -> (Result<String, String>, Result<String, String>) {
        (**self).health_and_readiness()
    }

    fn health_readiness_and_safe_device(
        &self,
    ) -> (
        Result<String, String>,
        Result<String, String>,
        Result<String, String>,
    ) {
        (**self).health_readiness_and_safe_device()
    }

    fn diagnostic_target(&self) -> String {
        (**self).diagnostic_target()
    }

    fn settings(&self) -> String {
        (**self).settings()
    }

    fn context_preview(&self) -> Result<String, String> {
        (**self).context_preview()
    }

    fn sessions(&self, filter: SessionFilter, limit: usize) -> String {
        (**self).sessions(filter, limit)
    }

    fn resume_session(&self, selector: &str) -> Result<String, String> {
        (**self).resume_session(selector)
    }

    fn summarize_session(&self, selector: &str) -> Result<String, String> {
        (**self).summarize_session(selector)
    }

    fn project_notes(&self) -> Result<String, String> {
        (**self).project_notes()
    }

    fn add_project_note(&self, note: &str) -> Result<String, String> {
        (**self).add_project_note(note)
    }

    fn set_project_notes(&self, notes: &str) -> Result<String, String> {
        (**self).set_project_notes(notes)
    }

    fn clear_project_notes(&self) -> Result<String, String> {
        (**self).clear_project_notes()
    }

    fn model_pool_index_notes(&self) -> Result<String, String> {
        (**self).model_pool_index_notes()
    }

    fn clear_model_pool_index_notes(&self) -> Result<String, String> {
        (**self).clear_model_pool_index_notes()
    }

    fn record_event(&self, kind: &str, content: &str) -> Result<(), String> {
        (**self).record_event(kind, content)
    }

    fn set_endpoint(&self, endpoint: StreamEndpoint) -> Result<(), String> {
        (**self).set_endpoint(endpoint)
    }

    fn set_output(&self, output: &str) -> Result<(), String> {
        (**self).set_output(output)
    }

    fn set_profile(&self, profile: &str) -> Result<(), String> {
        (**self).set_profile(profile)
    }

    fn set_feedback_amount(&self, amount: &str) -> Result<(), String> {
        (**self).set_feedback_amount(amount)
    }

    fn set_self_improve(&self, enabled: bool) -> Result<(), String> {
        (**self).set_self_improve(enabled)
    }

    fn set_context_window(&self, max_messages: usize) -> Result<String, String> {
        (**self).set_context_window(max_messages)
    }

    fn set_max_tokens(&self, max_tokens: Option<usize>) -> Result<String, String> {
        (**self).set_max_tokens(max_tokens)
    }

    fn set_rust_check_inline(&self, code: &str) -> Result<String, String> {
        (**self).set_rust_check_inline(code)
    }

    fn set_rust_check_file(&self, path: &str) -> Result<String, String> {
        (**self).set_rust_check_file(path)
    }

    fn set_rust_check_edition(&self, edition: &str) -> Result<(), String> {
        (**self).set_rust_check_edition(edition)
    }

    fn set_rust_check_case(&self, case_name: Option<String>) -> Result<(), String> {
        (**self).set_rust_check_case(case_name)
    }

    fn clear_rust_check(&self) -> Result<(), String> {
        (**self).clear_rust_check()
    }

    fn reset(&self) {
        (**self).reset();
    }

    fn new_session(&self) -> Result<String, String> {
        (**self).new_session()
    }
}
