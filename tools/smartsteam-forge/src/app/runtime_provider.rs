use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver},
    },
    thread,
};

use smartsteam_forge::{
    ForgeProvider, ForgeSession, ProjectNotesStore, ProviderConfig, SessionFilter, SessionStore,
    StreamEndpoint,
};

use super::provider::{ChatProvider, ProviderEvent};

mod health_checks;
mod session_control;
mod stream;
mod transcript;

use stream::{final_answer_differs, stream_prompt};
use transcript::{record_answer, record_error};

#[derive(Clone)]
pub struct RuntimeProvider {
    provider: ForgeProvider,
    session: Arc<Mutex<ForgeSession>>,
    store: Arc<Mutex<Option<SessionStore>>>,
    notes: Arc<Mutex<Option<ProjectNotesStore>>>,
}

impl RuntimeProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            provider: ForgeProvider::new(config),
            session: Arc::new(Mutex::new(ForgeSession::default())),
            store: Arc::new(Mutex::new(SessionStore::open_default().ok())),
            notes: Arc::new(Mutex::new(ProjectNotesStore::open_default().ok())),
        }
    }
}

impl Default for RuntimeProvider {
    fn default() -> Self {
        Self::new(ProviderConfig::default())
    }
}

impl ChatProvider for RuntimeProvider {
    fn send(&self, prompt: String) -> Receiver<ProviderEvent> {
        let (tx, rx) = mpsc::channel();
        let provider = self.provider.clone();
        let session = Arc::clone(&self.session);
        let store = Arc::clone(&self.store);
        let notes = Arc::clone(&self.notes);

        thread::spawn(move || {
            let mut saw_text = false;
            let mut saw_done = false;
            let result = match session.lock() {
                Ok(mut session) => {
                    match session_control::project_notes_context(&notes) {
                        Ok(Some(context)) => session.set_project_notes_context(context),
                        Ok(None) => session.clear_project_notes_context(),
                        Err(error) => {
                            let _ = tx.send(ProviderEvent::Status(format!(
                                "project notes unavailable: {error}"
                            )));
                        }
                    }
                    stream_prompt(
                        &provider,
                        &mut session,
                        &prompt,
                        &tx,
                        &mut saw_text,
                        &mut saw_done,
                    )
                }
                Err(error) => Err(format!("session lock poisoned: {error}")),
            };

            match result {
                Ok(answer) => {
                    record_answer(&store, &prompt, &answer, &tx);
                    if !saw_text && !answer.assistant_message.trim().is_empty() {
                        let _ = tx.send(ProviderEvent::Delta(answer.assistant_message));
                    } else if final_answer_differs(&answer) {
                        let _ = tx.send(ProviderEvent::ReplaceAssistant(answer.assistant_message));
                    }
                    if !saw_done {
                        let _ = tx.send(ProviderEvent::Done);
                    }
                }
                Err(error) => {
                    record_error(&store, &prompt, &error, &tx);
                    let _ = tx.send(ProviderEvent::Error(error));
                }
            }
        });

        rx
    }

    fn status(&self) -> String {
        health_checks::status(&self.provider)
    }

    fn health_check(&self) -> Result<String, String> {
        health_checks::health_check(&self.provider)
    }

    fn experience_hygiene(&self) -> Result<String, String> {
        health_checks::experience_hygiene(&self.provider)
    }

    fn experience_hygiene_quarantine_dry_run(&self, limit: usize) -> Result<String, String> {
        health_checks::experience_hygiene_quarantine_dry_run(&self.provider, limit)
    }

    fn experience_repair_dry_run(&self, limit: usize) -> Result<String, String> {
        health_checks::experience_repair_dry_run(&self.provider, limit)
    }

    fn experience_cleanup_audit(&self, limit: usize) -> Result<String, String> {
        health_checks::experience_cleanup_audit(&self.provider, limit)
    }

    fn experience_retrieval(&self, prompt: &str, limit: usize) -> Result<String, String> {
        let profile = self
            .session
            .lock()
            .map_err(|error| format!("session lock poisoned: {error}"))?
            .settings()
            .profile
            .clone();
        let project_notes_context = session_control::project_notes_context(&self.notes)?;
        let retrieval_prompt = session_control::retrieval_prompt_with_index_context(
            prompt,
            project_notes_context.as_deref(),
        );
        let mut summary = health_checks::experience_retrieval(
            &self.provider,
            &retrieval_prompt.prompt,
            &profile,
            limit,
            retrieval_prompt.index_context.as_deref(),
        )?;
        session_control::append_index_context_query_status(
            &mut summary,
            retrieval_prompt.index_context_chars,
            retrieval_prompt.index_context_active_trusted,
        );
        Ok(summary)
    }

    fn model_pool_status(&self) -> Result<String, String> {
        self.provider.model_pool_status()
    }

    fn model_pool_manifest(&self) -> Result<String, String> {
        self.provider.model_pool_manifest()
    }

    fn model_pool_status_async(&self) -> Receiver<Result<String, String>> {
        let (tx, rx) = mpsc::channel();
        let provider = self.provider.clone();
        thread::spawn(move || {
            let _ = tx.send(provider.model_pool_status());
        });
        rx
    }

    fn model_pool_route(&self, task_kind: &str) -> Result<String, String> {
        self.provider
            .model_pool_route_with_max_tokens(task_kind, self.session_max_tokens()?)
    }

    fn model_pool_call(&self, task_kind: &str, prompt: &str) -> Result<String, String> {
        self.provider
            .model_pool_call_with_max_tokens(task_kind, prompt, self.session_max_tokens()?)
    }

    fn readiness_check(&self) -> Result<String, String> {
        health_checks::readiness_check(&self.provider)
    }

    fn prompt_preflight(&self, require_safe_device: bool) -> Result<String, String> {
        health_checks::prompt_preflight(&self.provider, require_safe_device)
    }

    fn safe_device_check(&self) -> Result<String, String> {
        health_checks::safe_device_check(&self.provider)
    }

    fn health_and_readiness(&self) -> (Result<String, String>, Result<String, String>) {
        health_checks::health_and_readiness(&self.provider)
    }

    fn health_readiness_and_safe_device(
        &self,
    ) -> (
        Result<String, String>,
        Result<String, String>,
        Result<String, String>,
    ) {
        health_checks::health_readiness_and_safe_device(&self.provider)
    }

    fn diagnostic_target(&self) -> String {
        health_checks::diagnostic_target(&self.provider)
    }

    fn settings(&self) -> String {
        session_control::settings(&self.session, &self.store, &self.notes)
    }

    fn context_preview(&self) -> Result<String, String> {
        session_control::context_preview(&self.session, &self.notes)
    }

    fn sessions(&self, filter: SessionFilter, limit: usize) -> String {
        session_control::sessions(&self.store, filter, limit)
    }

    fn resume_session(&self, selector: &str) -> Result<String, String> {
        session_control::resume_session(&self.session, &self.store, selector)
    }

    fn summarize_session(&self, selector: &str) -> Result<String, String> {
        session_control::summarize_session(&self.store, selector)
    }

    fn project_notes(&self) -> Result<String, String> {
        session_control::project_notes(&self.notes)
    }

    fn add_project_note(&self, note: &str) -> Result<String, String> {
        session_control::add_project_note(&self.session, &self.notes, note)
    }

    fn add_model_pool_index_note(&self, note: &str) -> Result<String, String> {
        session_control::add_model_pool_index_note(&self.session, &self.notes, note)
    }

    fn set_project_notes(&self, notes: &str) -> Result<String, String> {
        session_control::set_project_notes(&self.session, &self.notes, notes)
    }

    fn clear_project_notes(&self) -> Result<String, String> {
        session_control::clear_project_notes(&self.session, &self.notes)
    }

    fn model_pool_index_notes(&self) -> Result<String, String> {
        session_control::model_pool_index_notes(&self.notes)
    }

    fn clear_model_pool_index_notes(&self) -> Result<String, String> {
        session_control::clear_model_pool_index_notes(&self.session, &self.notes)
    }

    fn record_event(&self, kind: &str, content: &str) -> Result<(), String> {
        session_control::record_event(&self.store, kind, content)
    }

    fn set_endpoint(&self, endpoint: StreamEndpoint) -> Result<(), String> {
        session_control::set_endpoint(&self.session, endpoint)
    }

    fn set_output(&self, output: &str) -> Result<(), String> {
        session_control::set_output(&self.session, output)
    }

    fn set_profile(&self, profile: &str) -> Result<(), String> {
        session_control::set_profile(&self.session, profile)
    }

    fn set_feedback_amount(&self, amount: &str) -> Result<(), String> {
        session_control::set_feedback_amount(&self.session, amount)
    }

    fn set_self_improve(&self, enabled: bool) -> Result<(), String> {
        session_control::set_self_improve(&self.session, enabled)
    }

    fn set_context_window(&self, max_messages: usize) -> Result<String, String> {
        session_control::set_context_window(&self.session, max_messages)
    }

    fn set_max_tokens(&self, max_tokens: Option<usize>) -> Result<String, String> {
        session_control::set_max_tokens(&self.session, max_tokens)
    }

    fn set_rust_check_inline(&self, code: &str) -> Result<String, String> {
        session_control::set_rust_check_inline(&self.session, code)
    }

    fn set_rust_check_file(&self, path: &str) -> Result<String, String> {
        session_control::set_rust_check_file(&self.session, path)
    }

    fn set_rust_check_edition(&self, edition: &str) -> Result<(), String> {
        session_control::set_rust_check_edition(&self.session, edition)
    }

    fn set_rust_check_case(&self, case_name: Option<String>) -> Result<(), String> {
        session_control::set_rust_check_case(&self.session, case_name)
    }

    fn clear_rust_check(&self) -> Result<(), String> {
        session_control::clear_rust_check(&self.session)
    }

    fn reset(&self) {
        session_control::reset(&self.session);
    }

    fn new_session(&self) -> Result<String, String> {
        session_control::new_session(&self.session, &self.store)
    }
}

impl RuntimeProvider {
    fn session_max_tokens(&self) -> Result<Option<usize>, String> {
        Ok(self
            .session
            .lock()
            .map_err(|error| format!("session lock poisoned: {error}"))?
            .settings()
            .max_tokens)
    }
}
