use std::sync::mpsc::TryRecvError;
use std::time::Instant;

use super::super::provider::{ChatProvider, ProviderEvent};
use super::{App, Message, MessageRole};

impl<P: ChatProvider> App<P> {
    pub fn tick(&mut self) {
        self.drain_provider_events();
        self.drain_model_pool_watch();
        self.poll_model_pool_watch();
    }

    fn drain_provider_events(&mut self) {
        let Some(stream) = self.stream.take() else {
            return;
        };
        let mut keep_stream = true;

        loop {
            match stream.try_recv() {
                Ok(event) => self.apply_provider_event(event, &mut keep_stream),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.provider_busy = false;
                    keep_stream = false;
                    break;
                }
            }
        }

        if keep_stream {
            self.stream = Some(stream);
        }
        self.auto_scroll();
    }

    fn apply_provider_event(&mut self, event: ProviderEvent, keep_stream: &mut bool) {
        match event {
            ProviderEvent::Delta(delta) => self.append_assistant(delta),
            ProviderEvent::ReplaceAssistant(answer) => self.replace_assistant(answer),
            ProviderEvent::GateReport(report)
            | ProviderEvent::Stage(report)
            | ProviderEvent::Meta(report)
            | ProviderEvent::Status(report)
            | ProviderEvent::Heartbeat(report) => self.push_system(report),
            ProviderEvent::Done => {
                self.provider_busy = false;
                *keep_stream = false;
                self.status = "ready".to_owned();
            }
            ProviderEvent::Error(error) => {
                self.provider_busy = false;
                *keep_stream = false;
                self.push_system(format!("provider error: {error}"));
            }
        }
    }

    fn drain_model_pool_watch(&mut self) {
        let Some(receiver) = self.model_pool_watch_result.take() else {
            return;
        };

        match receiver.try_recv() {
            Ok(Ok(summary)) => {
                self.push_system(summary);
                if let Some(watch) = &mut self.model_pool_watch {
                    watch.iteration = watch.iteration.saturating_add(1);
                    if let Some(remaining) = &mut watch.remaining_iterations {
                        *remaining = remaining.saturating_sub(1);
                        if *remaining == 0 {
                            self.model_pool_watch = None;
                        }
                    }
                }
            }
            Ok(Err(error)) => self.push_system(format!("model pool watch error: {error}")),
            Err(TryRecvError::Empty) => self.model_pool_watch_result = Some(receiver),
            Err(TryRecvError::Disconnected) => {}
        }
    }

    fn poll_model_pool_watch(&mut self) {
        if self.model_pool_watch_result.is_some() {
            return;
        }
        let Some(watch) = &mut self.model_pool_watch else {
            return;
        };
        if Instant::now() < watch.next_poll_at {
            return;
        }
        watch.next_poll_at = Instant::now() + watch.interval;
        self.model_pool_watch_result = Some(self.provider.model_pool_status_async());
    }

    fn append_assistant(&mut self, delta: String) {
        if let Some(message) = self
            .messages
            .iter_mut()
            .rev()
            .find(|message| message.role == MessageRole::Assistant)
        {
            message.content.push_str(&delta);
        } else {
            self.messages
                .push(Message::new(MessageRole::Assistant, delta));
        }
    }

    fn replace_assistant(&mut self, answer: String) {
        if let Some(message) = self
            .messages
            .iter_mut()
            .rev()
            .find(|message| message.role == MessageRole::Assistant)
        {
            message.content = answer;
        } else {
            self.messages
                .push(Message::new(MessageRole::Assistant, answer));
        }
    }
}
