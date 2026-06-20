use std::{
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

mod commands;
mod events;

#[cfg(test)]
mod tests;

use super::commands::parse_slash_command;
use super::input_buffer::InputCursor;
use super::provider::{ChatProvider, ProviderEvent};
use crate::text_width::terminal_width;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

impl Message {
    pub fn new(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppAction {
    None,
    Quit,
    DispatchPrompt(String),
}

#[derive(Clone, Debug)]
struct ModelPoolWatchState {
    interval: Duration,
    next_poll_at: Instant,
    remaining_iterations: Option<usize>,
    iteration: usize,
}

impl ModelPoolWatchState {
    fn new(interval_secs: u64, max_iterations: Option<usize>) -> Self {
        Self {
            interval: Duration::from_secs(interval_secs.max(1)),
            next_poll_at: Instant::now(),
            remaining_iterations: max_iterations,
            iteration: 0,
        }
    }
}

pub struct App<P> {
    pub input: String,
    pub messages: Vec<Message>,
    pub status: String,
    pub should_quit: bool,
    pub provider_busy: bool,
    pub readiness_guard: bool,
    pub safe_device_guard: bool,
    pub scroll: u16,
    pub auto_follow: bool,
    provider: P,
    stream: Option<Receiver<ProviderEvent>>,
    model_pool_watch: Option<ModelPoolWatchState>,
    model_pool_watch_result: Option<Receiver<Result<String, String>>>,
    input_cursor: InputCursor,
    transcript_visible_lines: u16,
    transcript_wrap_width: u16,
}

impl<P: ChatProvider> App<P> {
    pub fn new(provider: P) -> Self {
        Self::with_readiness_guard(provider, false)
    }

    pub fn with_readiness_guard(provider: P, readiness_guard: bool) -> Self {
        Self::with_guards(provider, readiness_guard, false)
    }

    pub fn with_guards(provider: P, readiness_guard: bool, safe_device_guard: bool) -> Self {
        Self {
            input: String::new(),
            messages: vec![Message::new(
                MessageRole::System,
                "SmartSteam Forge ready. Type /help for commands.",
            )],
            status: "ready".to_string(),
            should_quit: false,
            provider_busy: false,
            readiness_guard,
            safe_device_guard,
            scroll: 0,
            auto_follow: true,
            provider,
            stream: None,
            model_pool_watch: None,
            model_pool_watch_result: None,
            input_cursor: InputCursor::default(),
            transcript_visible_lines: 1,
            transcript_wrap_width: u16::MAX,
        }
    }

    pub fn push_char(&mut self, ch: char) {
        self.input_cursor.insert_char(&mut self.input, ch);
    }

    pub fn push_text(&mut self, text: &str) {
        self.input_cursor.insert_text(&mut self.input, text);
    }

    pub fn backspace(&mut self) {
        self.input_cursor.backspace(&mut self.input);
    }

    pub fn delete_input_char(&mut self) {
        self.input_cursor.delete(&mut self.input);
    }

    pub fn move_input_cursor_left(&mut self) {
        self.input_cursor.move_left(&self.input);
    }

    pub fn move_input_cursor_right(&mut self) {
        self.input_cursor.move_right(&self.input);
    }

    pub fn move_input_cursor_start(&mut self) {
        self.input_cursor.move_start();
    }

    pub fn move_input_cursor_end(&mut self) {
        self.input_cursor.move_end();
    }

    pub fn clear_input_before_cursor(&mut self) {
        self.input_cursor.clear_before(&mut self.input);
    }

    pub fn delete_word_before_cursor(&mut self) {
        self.input_cursor.delete_word_before(&mut self.input);
    }

    pub fn input_cursor_byte_index(&self) -> usize {
        self.input_cursor.byte_index(&self.input)
    }

    pub fn submit(&mut self) -> AppAction {
        let prompt = self.input.trim().to_string();
        self.input.clear();
        self.input_cursor.reset_to_end();

        if prompt.is_empty() {
            return AppAction::None;
        }

        if let Some(command) = parse_slash_command(&prompt) {
            return self.apply_slash_command(command);
        }

        if self.provider_busy {
            self.input = prompt;
            self.input_cursor.reset_to_end();
            self.status = "provider busy: cancel or wait before sending another prompt".to_owned();
            self.messages.push(Message::new(
                MessageRole::System,
                "Provider is still streaming. Use /cancel to stop it, or wait for it to finish.",
            ));
            self.auto_scroll();
            return AppAction::None;
        }

        if self.readiness_guard || self.safe_device_guard {
            match self.provider.prompt_preflight(self.safe_device_guard) {
                Ok(summary) => {
                    self.status = format!("provider preflight ready: {summary}");
                }
                Err(error) => {
                    self.input = prompt;
                    self.input_cursor.reset_to_end();
                    self.status = format!("prompt blocked: {error}");
                    self.messages.push(Message::new(
                        MessageRole::System,
                        format!("Prompt blocked by preflight guard: {error}"),
                    ));
                    self.auto_scroll();
                    return AppAction::None;
                }
            }
        }

        self.messages
            .push(Message::new(MessageRole::User, prompt.clone()));
        self.messages
            .push(Message::new(MessageRole::Assistant, String::new()));
        self.provider_busy = true;
        self.status = "streaming provider response".to_string();
        self.stream = Some(self.provider.send(prompt.clone()));
        self.auto_scroll();

        AppAction::DispatchPrompt(prompt)
    }

    pub fn clear(&mut self) {
        self.stream = None;
        self.model_pool_watch = None;
        self.model_pool_watch_result = None;
        self.provider_busy = false;
        self.messages.clear();
        self.messages.push(Message::new(
            MessageRole::System,
            "Conversation cleared. SmartSteam Forge is ready.",
        ));
        self.provider.reset();
        self.scroll = 0;
        self.auto_follow = true;
    }

    pub fn cancel_current_stream(&mut self) -> AppAction {
        if self.stream.take().is_some() || self.provider_busy {
            self.provider_busy = false;
            self.status = "cancel requested".to_owned();
            self.messages.push(Message::new(
                MessageRole::System,
                "Canceled current provider stream. Backend cleanup may take one heartbeat.",
            ));
        } else {
            self.status = "ready".to_owned();
            self.messages.push(Message::new(
                MessageRole::System,
                "No provider stream is running.",
            ));
        }
        self.auto_scroll();
        AppAction::None
    }

    pub fn auto_scroll(&mut self) {
        if !self.auto_follow {
            return;
        }
        self.scroll_to_bottom();
    }

    pub fn set_transcript_viewport(&mut self, height: u16, width: u16) {
        self.transcript_visible_lines = height.saturating_sub(2).max(1);
        self.transcript_wrap_width = width.saturating_sub(2).max(1);
        if self.auto_follow {
            self.scroll_to_bottom();
        } else {
            self.scroll = self.scroll.min(self.bottom_scroll());
        }
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll = self.scroll.saturating_sub(amount.max(1));
        self.auto_follow = false;
        self.status = "scrollback: manual".to_owned();
    }

    pub fn scroll_down(&mut self, amount: u16) {
        let bottom = self.bottom_scroll();
        self.scroll = self.scroll.saturating_add(amount.max(1)).min(bottom);
        self.auto_follow = self.scroll >= bottom;
        self.status = if self.auto_follow {
            "scrollback: following latest output".to_owned()
        } else {
            "scrollback: manual".to_owned()
        };
    }

    pub fn scroll_top(&mut self) {
        self.scroll = 0;
        self.auto_follow = false;
        self.status = "scrollback: top".to_owned();
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll = self.bottom_scroll();
        self.auto_follow = true;
    }

    pub fn transcript_content_lines(&self) -> usize {
        let legacy_line_count = self
            .messages
            .iter()
            .map(|message| message.content.lines().count().max(1) as u16 + 1)
            .map(|legacy_lines| legacy_lines as usize)
            .sum::<usize>();
        let wrapped_line_count = self
            .messages
            .iter()
            .map(|message| wrapped_message_line_count(&message.content, self.transcript_wrap_width))
            .sum::<usize>();
        legacy_line_count.max(wrapped_line_count)
    }

    fn bottom_scroll(&self) -> u16 {
        let bottom = self
            .transcript_content_lines()
            .saturating_sub(self.transcript_visible_lines as usize);
        bottom.min(u16::MAX as usize) as u16
    }
}

fn wrapped_message_line_count(content: &str, wrap_width: u16) -> usize {
    let content_lines = content
        .split('\n')
        .map(|line| wrapped_line_count(line, wrap_width))
        .sum::<usize>()
        .max(1);
    content_lines + 1
}

fn wrapped_line_count(line: &str, wrap_width: u16) -> usize {
    let width = wrap_width.max(1) as usize;
    let display_width = terminal_width(line);
    display_width
        .saturating_add(width - 1)
        .checked_div(width)
        .unwrap_or(1)
        .max(1)
}
