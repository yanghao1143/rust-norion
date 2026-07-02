use std::time::Instant;

use super::super::commands::{ModelPoolWatchCommand, SlashCommand};
use super::super::provider::ChatProvider;
use super::{App, AppAction, Message, MessageRole, ModelPoolWatchState};

impl<P: ChatProvider> App<P> {
    pub(super) fn apply_slash_command(&mut self, command: SlashCommand) -> AppAction {
        match command {
            SlashCommand::Help => {
                self.push_system("Commands: /status /ready /retrieve /pool /doctor /cancel /quit")
            }
            SlashCommand::Clear => self.clear(),
            SlashCommand::New => self.push_provider_result("new", self.provider.new_session()),
            SlashCommand::Status => self.push_system(self.provider.status()),
            SlashCommand::EvolutionStrictSummary(_) => self
                .push_system("strict summary is available from the CLI command, not the TUI state"),
            SlashCommand::Ready => {
                self.push_provider_result("ready", self.provider.readiness_check())
            }
            SlashCommand::Hygiene => {
                self.push_provider_result("hygiene", self.provider.experience_hygiene())
            }
            SlashCommand::HygieneQuarantineDryRun(limit) => self.push_provider_result(
                "hygiene quarantine",
                self.provider.experience_hygiene_quarantine_dry_run(limit),
            ),
            SlashCommand::ExperienceRepairDryRun(limit) => {
                self.push_provider_result("repair", self.provider.experience_repair_dry_run(limit))
            }
            SlashCommand::ExperienceCleanupAudit(limit) => {
                self.push_provider_result("audit", self.provider.experience_cleanup_audit(limit))
            }
            SlashCommand::RetrievalPreview { prompt, limit } => self.push_provider_result(
                "retrieve",
                self.provider.experience_retrieval(&prompt, limit),
            ),
            SlashCommand::ModelPoolStatus => {
                self.push_provider_result("model pool", self.provider.model_pool_status())
            }
            SlashCommand::ModelPoolManifest => self
                .push_provider_result("model pool manifest", self.provider.model_pool_manifest()),
            SlashCommand::ModelPoolAdvice => self.push_system(
                "model pool advice is available from the CLI command, not the TUI state",
            ),
            SlashCommand::ModelPoolSmoke => self.push_system(
                "model pool smoke is available from the CLI command, not the TUI state",
            ),
            SlashCommand::ModelPoolWatch(command) => self.apply_model_pool_watch(command),
            SlashCommand::ModelPoolRoute(task_kind) => self.push_provider_result(
                "model pool route",
                self.provider.model_pool_route(&task_kind),
            ),
            SlashCommand::ModelPoolCall { task_kind, prompt } => self.push_provider_result(
                "model pool call",
                self.provider.model_pool_call(&task_kind, &prompt),
            ),
            SlashCommand::Doctor => self.push_doctor(),
            SlashCommand::Cancel => return self.cancel_current_stream(),
            SlashCommand::Guard(enabled) => {
                self.readiness_guard = enabled;
                self.push_system(format!("readiness guard={enabled}"));
            }
            SlashCommand::SafeDeviceGuard(enabled) => {
                self.safe_device_guard = enabled;
                self.push_system(format!("safe_device_guard={enabled}"));
            }
            SlashCommand::Show => self.push_system(self.provider.settings()),
            SlashCommand::Context => {
                self.push_provider_result("context", self.provider.context_preview())
            }
            SlashCommand::Sessions { filter, limit } => {
                self.push_system(self.provider.sessions(filter, limit))
            }
            SlashCommand::Resume(selector) => {
                self.push_provider_result("resume", self.provider.resume_session(&selector))
            }
            SlashCommand::Summary(selector) => {
                self.push_provider_result("summary", self.provider.summarize_session(&selector))
            }
            SlashCommand::NotesShow => {
                self.push_provider_result("notes", self.provider.project_notes())
            }
            SlashCommand::NotesAdd(note) => {
                self.push_provider_result("notes add", self.provider.add_project_note(&note))
            }
            SlashCommand::NotesSet(notes) => {
                self.push_provider_result("notes set", self.provider.set_project_notes(&notes))
            }
            SlashCommand::NotesClear => {
                self.push_provider_result("notes clear", self.provider.clear_project_notes())
            }
            SlashCommand::IndexNotesShow => {
                self.push_provider_result("index notes", self.provider.model_pool_index_notes())
            }
            SlashCommand::IndexNotesClear => self.push_provider_result(
                "index notes clear",
                self.provider.clear_model_pool_index_notes(),
            ),
            SlashCommand::Mode(endpoint) => {
                self.push_unit_result("mode", self.provider.set_endpoint(endpoint))
            }
            SlashCommand::Output(output) => {
                self.push_unit_result("output", self.provider.set_output(&output))
            }
            SlashCommand::Profile(profile) => {
                self.push_unit_result("profile", self.provider.set_profile(&profile))
            }
            SlashCommand::Feedback(amount) => {
                self.push_unit_result("feedback", self.provider.set_feedback_amount(&amount))
            }
            SlashCommand::SelfImprove(enabled) => {
                self.push_unit_result("self improve", self.provider.set_self_improve(enabled))
            }
            SlashCommand::ContextWindow(max_messages) => self.push_provider_result(
                "context window",
                self.provider.set_context_window(max_messages),
            ),
            SlashCommand::MaxTokens(max_tokens) => {
                self.push_provider_result("max tokens", self.provider.set_max_tokens(max_tokens))
            }
            SlashCommand::RustCheckInline(code) => {
                self.push_provider_result("rust check", self.provider.set_rust_check_inline(&code))
            }
            SlashCommand::RustCheckFile(path) => {
                self.push_provider_result("rust check", self.provider.set_rust_check_file(&path))
            }
            SlashCommand::RustCheckEdition(edition) => self.push_unit_result(
                "rust check edition",
                self.provider.set_rust_check_edition(&edition),
            ),
            SlashCommand::RustCheckCase(case_name) => self.push_unit_result(
                "rust check case",
                self.provider.set_rust_check_case(case_name),
            ),
            SlashCommand::RustCheckClear => {
                self.push_unit_result("rust check clear", self.provider.clear_rust_check())
            }
            SlashCommand::Quit => {
                self.should_quit = true;
                return AppAction::Quit;
            }
            SlashCommand::Unknown(command) => {
                self.push_system(format!("unknown command: {command}"))
            }
        }

        self.auto_scroll();
        AppAction::None
    }

    fn apply_model_pool_watch(&mut self, command: ModelPoolWatchCommand) {
        if !command.enabled {
            self.model_pool_watch = None;
            self.model_pool_watch_result = None;
            self.push_system("model pool watch stopped");
            return;
        }

        self.model_pool_watch = Some(ModelPoolWatchState::new(
            command.interval_secs,
            command.max_iterations,
        ));
        if let Some(watch) = &mut self.model_pool_watch {
            watch.next_poll_at = Instant::now() + watch.interval;
        }
        self.model_pool_watch_result = Some(self.provider.model_pool_status_async());
        self.push_system("model pool watch started");
    }

    fn push_doctor(&mut self) {
        let (health, readiness, safe_device) = self.provider.health_readiness_and_safe_device();
        self.push_system(format!(
            "health:\n{}\n\nreadiness:\n{}\n\nsafe_device:\n{}",
            result_text(health),
            result_text(readiness),
            result_text(safe_device)
        ));
    }

    fn push_provider_result(&mut self, label: &str, result: Result<String, String>) {
        match result {
            Ok(value) => self.push_system(value),
            Err(error) => self.push_system(format!("{label} error: {error}")),
        }
    }

    fn push_unit_result(&mut self, label: &str, result: Result<(), String>) {
        match result {
            Ok(()) => self.push_system(format!("{label}: ok")),
            Err(error) => self.push_system(format!("{label} error: {error}")),
        }
    }

    pub(super) fn push_system(&mut self, content: impl Into<String>) {
        let content = content.into();
        self.status = first_line(&content);
        self.messages
            .push(Message::new(MessageRole::System, content));
    }
}

fn result_text(result: Result<String, String>) -> String {
    result.unwrap_or_else(|error| format!("error: {error}"))
}

fn first_line(value: &str) -> String {
    value.lines().next().unwrap_or_default().to_owned()
}
