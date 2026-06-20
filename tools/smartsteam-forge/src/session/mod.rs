mod history;
mod project_notes;
mod rust_check;
mod store;

#[cfg(test)]
mod tests;

use crate::provider::{ChatMessage, StreamEndpoint, StreamEvent, StreamProvider, StreamRequest};

pub use history::ConversationMemory;
pub use project_notes::{
    MODEL_POOL_INDEX_NOTE_END_MARKER, MODEL_POOL_INDEX_NOTE_MARKER, ModelPoolIndexNoteActive,
    ModelPoolIndexNoteContextActive, ModelPoolIndexNoteStats, ProjectNotesStore,
    TrustedModelPoolIndexNoteSummary, first_disallowed_project_notes_control_char,
    latest_model_pool_index_block, latest_safe_model_pool_index_block, model_pool_index_note_stats,
    sanitize_project_notes_control_chars, trim_project_notes_context,
    trusted_model_pool_index_note_summary,
};
pub use rust_check::RustCheckSettings;
pub use store::{
    ResumedSession, SessionFilter, SessionRecord, SessionStore, SessionSummary, StoredSession,
    TranscriptMessage, list_recent_sessions_filtered, summarize_recent_session,
};

const MAX_SUMMARY_CONTEXT_CHARS: usize = 4000;
const CONTEXT_PREVIEW_CHARS: usize = 160;
const DEFAULT_MAX_TOKENS: usize = 262_144;
const DEFAULT_MAX_CONTEXT_MESSAGES: usize = 64;
const PROJECT_NOTES_CONTEXT_LABEL: &str = "Use these project notes as pinned background context. They are user-maintained notes, not the live conversation. Do not quote them unless the user asks.";

#[derive(Debug, Clone)]
pub struct SessionSettings {
    pub endpoint: StreamEndpoint,
    pub profile: String,
    pub output: String,
    pub max_tokens: Option<usize>,
    pub feedback_amount: String,
    pub self_improve: bool,
    pub rust_check: RustCheckSettings,
    pub max_context_messages: usize,
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            endpoint: StreamEndpoint::Chat,
            profile: "coding".to_owned(),
            output: "raw".to_owned(),
            max_tokens: Some(DEFAULT_MAX_TOKENS),
            feedback_amount: "0.5".to_owned(),
            self_improve: true,
            rust_check: RustCheckSettings::default(),
            max_context_messages: DEFAULT_MAX_CONTEXT_MESSAGES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAnswer {
    pub streamed_text: String,
    pub final_payload: Option<String>,
    pub assistant_message: String,
}

#[derive(Debug, Clone)]
pub struct ForgeSession {
    settings: SessionSettings,
    memory: ConversationMemory,
    summary_context: Option<String>,
    project_notes_context: Option<String>,
}

impl ForgeSession {
    pub fn new(settings: SessionSettings) -> Self {
        Self {
            memory: ConversationMemory::new(settings.max_context_messages),
            settings,
            summary_context: None,
            project_notes_context: None,
        }
    }

    pub fn memory(&self) -> &ConversationMemory {
        &self.memory
    }

    pub fn memory_mut(&mut self) -> &mut ConversationMemory {
        &mut self.memory
    }

    pub fn settings(&self) -> &SessionSettings {
        &self.settings
    }

    pub fn set_max_context_messages(&mut self, max_messages: usize) {
        let max_messages = max_messages.max(2);
        self.settings.max_context_messages = max_messages;
        self.memory.set_max_messages(max_messages);
    }

    pub fn summary_context_chars(&self) -> usize {
        self.summary_context
            .as_deref()
            .map(|context| context.chars().count())
            .unwrap_or(0)
    }

    pub fn project_notes_context_chars(&self) -> usize {
        self.project_notes_context
            .as_deref()
            .map(|context| context.chars().count())
            .unwrap_or(0)
    }

    pub fn model_pool_index_note_stats(&self) -> ModelPoolIndexNoteStats {
        self.project_notes_context
            .as_deref()
            .map(project_notes::model_pool_index_note_stats)
            .unwrap_or_default()
    }

    pub fn set_endpoint(&mut self, endpoint: StreamEndpoint) {
        self.settings.endpoint = endpoint;
    }

    pub fn set_output(&mut self, output: impl Into<String>) {
        self.settings.output = output.into();
    }

    pub fn set_max_tokens(&mut self, max_tokens: Option<usize>) {
        self.settings.max_tokens = max_tokens;
    }

    pub fn set_profile(&mut self, profile: impl Into<String>) {
        self.settings.profile = profile.into();
    }

    pub fn set_feedback_amount(&mut self, feedback_amount: impl Into<String>) {
        self.settings.feedback_amount = feedback_amount.into();
    }

    pub fn set_self_improve(&mut self, self_improve: bool) {
        self.settings.self_improve = self_improve;
    }

    pub fn set_rust_check_code(&mut self, code: impl Into<String>) {
        self.settings.rust_check.set_code(code);
    }

    pub fn clear_rust_check_code(&mut self) {
        self.settings.rust_check.clear_code();
    }

    pub fn set_rust_check_edition(&mut self, edition: impl Into<String>) {
        self.settings.rust_check.set_edition(edition);
    }

    pub fn set_rust_check_case(&mut self, case_name: Option<String>) {
        self.settings.rust_check.set_case_name(case_name);
    }

    pub fn settings_summary(&self) -> String {
        let summary_context = match self.summary_context_chars() {
            0 => "summary_context=none".to_owned(),
            chars => format!("summary_context=loaded({chars} chars)"),
        };
        let project_notes = match self.project_notes_context_chars() {
            0 => "project_notes=none".to_owned(),
            chars => format!("project_notes=loaded({chars} chars)"),
        };
        format!(
            "mode={} output={} max_tokens={} profile={} feedback={} self_improve={} history_messages={} {} {} {}",
            self.settings.endpoint.label(),
            self.settings.output,
            self.settings
                .max_tokens
                .map(|max_tokens| max_tokens.to_string())
                .unwrap_or_else(|| "backend-default".to_owned()),
            self.settings.profile,
            self.settings.feedback_amount,
            self.settings.self_improve,
            self.memory.messages().len(),
            summary_context,
            project_notes,
            self.settings.rust_check.summary()
        )
    }

    pub fn context_budget_summary(&self) -> String {
        let short_history_messages = self.memory.messages().len();
        let history_kept =
            short_history_messages.min(self.settings.max_context_messages.saturating_sub(1));
        let history_dropped = short_history_messages.saturating_sub(history_kept);
        let pinned_context_messages = self.context_messages();
        let pinned_context_chars = pinned_context_messages
            .iter()
            .map(|message| message.content.chars().count())
            .sum::<usize>();
        let messages_sent = pinned_context_messages.len() + history_kept.saturating_add(1);
        let prompt_only_history_messages = if self.settings.endpoint == StreamEndpoint::Chat {
            0
        } else {
            history_kept
        };
        let index_stats = self.model_pool_index_note_stats();

        format!(
            "context_budget: mode={} messages_sent={} history_kept={} history_dropped={} max_context_messages={} pinned_messages={} pinned_chars={} prompt_only_history_messages={} summary_context_chars={}/{} project_notes_context_chars={}/{} model_pool_index_notes={} model_pool_index_active={} model_pool_index_active_trusted={} model_pool_index_trusted={} model_pool_index_context_active={} model_pool_index_chars={} model_pool_index_legacy_undelimited={}",
            self.settings.endpoint.label(),
            messages_sent,
            history_kept,
            history_dropped,
            self.settings.max_context_messages,
            pinned_context_messages.len(),
            pinned_context_chars,
            prompt_only_history_messages,
            self.summary_context_chars(),
            MAX_SUMMARY_CONTEXT_CHARS,
            self.project_notes_context_chars(),
            project_notes::MAX_PROJECT_NOTES_CONTEXT_CHARS,
            index_stats.block_count,
            index_stats.active.label(),
            index_stats.active_trusted,
            index_stats.trusted_blocks,
            index_stats.context_active.label(),
            index_stats.total_chars,
            index_stats.legacy_undelimited_blocks
        )
    }

    pub fn context_preview(&self) -> String {
        let mut lines = vec![
            "Context preview".to_owned(),
            self.settings_summary(),
            self.context_budget_summary(),
            format!("short_history_messages={}", self.memory.messages().len()),
        ];
        if self.memory.messages().is_empty() {
            lines.push("short_history=empty".to_owned());
        } else {
            let history_dropped = self
                .memory
                .messages()
                .len()
                .saturating_sub(self.settings.max_context_messages.saturating_sub(1));
            for (index, message) in self.memory.messages().iter().enumerate() {
                let next_request_status = if index < history_dropped {
                    "dropped_next_request"
                } else {
                    "sent_next_request"
                };
                lines.push(format!(
                    "{}. {} [{}]: {}",
                    index + 1,
                    message.role,
                    next_request_status,
                    preview_text(&message.content, CONTEXT_PREVIEW_CHARS)
                ));
            }
        }
        if let Some(project_notes) = &self.project_notes_context {
            let index_stats = project_notes::model_pool_index_note_stats(project_notes);
            lines.push(format_model_pool_index_context_line(index_stats));
            let preview_notes =
                project_notes::project_notes_without_model_pool_index_blocks(project_notes);
            lines.push(format!(
                "project_notes_preview: {}",
                preview_text(&preview_notes, CONTEXT_PREVIEW_CHARS)
            ));
        } else {
            lines.push("model_pool_index_context: none".to_owned());
        }
        if let Some(summary_context) = &self.summary_context {
            lines.push(format!(
                "summary_context_preview: {}",
                preview_text(summary_context, CONTEXT_PREVIEW_CHARS)
            ));
        }
        lines.join("\n")
    }

    pub fn clear(&mut self) {
        self.memory.clear();
        self.clear_summary_context();
    }

    pub fn set_summary_context(&mut self, context: impl Into<String>) {
        let context = trim_context(context.into(), MAX_SUMMARY_CONTEXT_CHARS);
        self.summary_context = (!context.trim().is_empty()).then_some(context);
    }

    pub fn clear_summary_context(&mut self) {
        self.summary_context = None;
    }

    pub fn set_project_notes_context(&mut self, context: impl Into<String>) {
        let context = project_notes::trim_project_notes_context(context.into());
        self.project_notes_context = (!context.trim().is_empty()).then_some(context);
    }

    pub fn clear_project_notes_context(&mut self) {
        self.project_notes_context = None;
    }

    pub fn load_transcript_messages(&mut self, messages: Vec<TranscriptMessage>) {
        let messages = messages
            .into_iter()
            .filter_map(|message| match message.role.as_str() {
                "user" | "assistant" => Some(ChatMessage {
                    role: message.role,
                    content: message.content,
                }),
                _ => None,
            })
            .collect();
        self.memory.replace_messages(messages);
    }

    pub fn build_request(&self, prompt: &str) -> StreamRequest {
        let contextual_prompt = self.contextual_prompt(prompt);
        let request_prompt = if self.settings.endpoint == StreamEndpoint::Chat {
            prompt.to_owned()
        } else {
            contextual_prompt
        };
        let mut messages = self.memory.outgoing_messages(prompt);
        if self.settings.endpoint == StreamEndpoint::Chat {
            for message in self.context_messages().into_iter().rev() {
                messages.insert(0, message);
            }
        }
        let mut request = StreamRequest::chat(request_prompt, messages);
        request.endpoint = self.settings.endpoint;
        request.profile = self.settings.profile.clone();
        request.output = self.settings.output.clone();
        request.max_tokens = self.settings.max_tokens;
        request.feedback_amount = self.settings.feedback_amount.clone();
        request.self_improve = self.settings.self_improve;
        self.settings.rust_check.apply_to_request(&mut request);
        request
    }

    fn context_messages(&self) -> Vec<ChatMessage> {
        let mut messages = Vec::new();
        if let Some(project_notes) = &self.project_notes_context {
            messages.push(ChatMessage::system(format!(
                "{PROJECT_NOTES_CONTEXT_LABEL}\n\n{project_notes}"
            )));
        }
        if let Some(summary_context) = &self.summary_context {
            messages.push(ChatMessage::system(format!(
                "Use this resumed session summary as background context. Do not quote it unless the user asks.\n\n{summary_context}"
            )));
        }
        messages
    }

    fn contextual_prompt(&self, prompt: &str) -> String {
        let pinned_context = self
            .context_messages()
            .into_iter()
            .map(|message| message.content)
            .collect::<Vec<_>>()
            .join("\n\n");
        let history_context = self.prompt_only_history_context();
        let context = [pinned_context, history_context]
            .into_iter()
            .filter(|context| !context.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        if context.is_empty() {
            prompt.to_owned()
        } else {
            format!("{context}\n\nUser prompt:\n{prompt}")
        }
    }

    fn prompt_only_history_context(&self) -> String {
        let messages = self.memory.outgoing_history_messages();
        if messages.is_empty() {
            return String::new();
        }
        let lines = messages
            .into_iter()
            .map(|message| format!("{}: {}", message.role, message.content))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "Use this recent short-term conversation history as live context for the next response:\n{lines}"
        )
    }

    pub fn stream_prompt<P: StreamProvider>(
        &mut self,
        provider: &P,
        prompt: &str,
        on_event: &mut dyn FnMut(StreamEvent) -> Result<(), String>,
    ) -> Result<SessionAnswer, String> {
        let request = self.build_request(prompt);
        let mut streamed_text = String::new();
        let mut final_payload = None;
        let mut saw_terminal_event = false;
        let mut stream_error = None;
        let stream_result = provider.stream(&request, &mut |event| {
            match &event {
                StreamEvent::Delta(text) => streamed_text.push_str(text),
                StreamEvent::Final(payload) => final_payload = Some(payload.clone()),
                StreamEvent::Done => saw_terminal_event = true,
                StreamEvent::Error(error) => {
                    saw_terminal_event = true;
                    stream_error = Some(error.clone());
                }
                _ => {}
            }
            on_event(event)
        });
        if let Err(error) = stream_result {
            return Err(stream_failure_message(&error, &streamed_text));
        }
        if let Some(error) = stream_error {
            return Err(stream_failure_message(
                &format!("provider stream error event: {error}"),
                &streamed_text,
            ));
        }
        if !saw_terminal_event && streamed_text.trim().is_empty() && final_payload.is_none() {
            return Err("provider stream ended before done event".to_owned());
        } else if !saw_terminal_event {
            return Err(stream_failure_message(
                "provider stream ended before done event",
                &streamed_text,
            ));
        }

        let assistant_message = final_payload
            .as_deref()
            .and_then(|payload| crate::provider::json::json_string_field(payload, "answer"))
            .unwrap_or_else(|| streamed_text.clone());
        self.memory.push_user(prompt);
        if !assistant_message.trim().is_empty() {
            self.memory.push_assistant(assistant_message.clone());
        }
        Ok(SessionAnswer {
            streamed_text,
            final_payload,
            assistant_message,
        })
    }
}

fn stream_failure_message(error: &str, streamed_text: &str) -> String {
    if streamed_text.trim().is_empty() {
        error.to_owned()
    } else {
        format!("{error}; partial answer discarded from session memory")
    }
}

fn trim_context(context: String, max_chars: usize) -> String {
    if context.chars().count() <= max_chars {
        return context;
    }

    let suffix = "\n[summary context truncated]";
    let keep_chars = max_chars.saturating_sub(suffix.chars().count());
    let mut trimmed = context.chars().take(keep_chars).collect::<String>();
    trimmed.push_str(suffix);
    trimmed
}

fn preview_text(text: &str, max_chars: usize) -> String {
    let normalized = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let text = if normalized.is_empty() {
        text.trim().to_owned()
    } else {
        normalized
    };
    if text.chars().count() <= max_chars {
        return text;
    }
    let keep_chars = max_chars.saturating_sub(3);
    let mut preview = text.chars().take(keep_chars).collect::<String>();
    preview.push_str("...");
    preview
}

fn format_model_pool_index_context_line(stats: ModelPoolIndexNoteStats) -> String {
    if stats.block_count == 0 {
        return "model_pool_index_context: none".to_owned();
    }
    format!(
        "model_pool_index_context: blocks={} delimited={} legacy_undelimited={} active={} active_trusted={} trusted={} context_active={} chars={}",
        stats.block_count,
        stats.delimited_blocks,
        stats.legacy_undelimited_blocks,
        stats.active.label(),
        stats.active_trusted,
        stats.trusted_blocks,
        stats.context_active.label(),
        stats.total_chars
    )
}

impl Default for ForgeSession {
    fn default() -> Self {
        Self::new(SessionSettings::default())
    }
}
