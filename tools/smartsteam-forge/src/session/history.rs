use super::DEFAULT_MAX_CONTEXT_MESSAGES;
use crate::provider::ChatMessage;

#[derive(Debug, Clone)]
pub struct ConversationMemory {
    max_messages: usize,
    messages: Vec<ChatMessage>,
}

impl ConversationMemory {
    pub fn new(max_messages: usize) -> Self {
        Self {
            max_messages: max_messages.max(2),
            messages: Vec::new(),
        }
    }

    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn max_messages(&self) -> usize {
        self.max_messages
    }

    pub fn set_max_messages(&mut self, max_messages: usize) {
        self.max_messages = max_messages.max(2);
        self.trim();
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn replace_messages(&mut self, messages: Vec<ChatMessage>) {
        self.messages = messages;
        self.trim();
    }

    pub fn push_user(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessage::user(content));
        self.trim();
    }

    pub fn push_assistant(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessage::assistant(content));
        self.trim();
    }

    pub fn outgoing_messages(&self, prompt: &str) -> Vec<ChatMessage> {
        let mut messages = self.outgoing_history_messages();
        messages.push(ChatMessage::user(prompt));
        messages
    }

    pub fn outgoing_history_messages(&self) -> Vec<ChatMessage> {
        let mut messages = self
            .messages
            .iter()
            .rev()
            .take(self.max_messages.saturating_sub(1))
            .cloned()
            .collect::<Vec<_>>();
        messages.reverse();
        messages
    }

    fn trim(&mut self) {
        if self.messages.len() > self.max_messages {
            let drop_count = self.messages.len() - self.max_messages;
            self.messages.drain(..drop_count);
        }
    }
}

impl Default for ConversationMemory {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_CONTEXT_MESSAGES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_short_context_window() {
        let mut memory = ConversationMemory::new(4);
        for index in 0..6 {
            memory.push_user(format!("m{index}"));
        }

        let outgoing = memory.outgoing_messages("next");

        assert_eq!(outgoing.len(), 4);
        assert_eq!(outgoing.first().unwrap().content, "m3");
        assert_eq!(outgoing.last().unwrap().content, "next");
    }

    #[test]
    fn replace_messages_trims_to_window() {
        let mut memory = ConversationMemory::new(3);
        let messages = (0..5)
            .map(|index| ChatMessage::user(format!("m{index}")))
            .collect();

        memory.replace_messages(messages);

        assert_eq!(memory.messages().len(), 3);
        assert_eq!(memory.messages()[0].content, "m2");
    }

    #[test]
    fn updating_max_messages_trims_existing_history() {
        let mut memory = ConversationMemory::new(6);
        for index in 0..6 {
            memory.push_user(format!("m{index}"));
        }

        memory.set_max_messages(3);

        assert_eq!(memory.max_messages(), 3);
        assert_eq!(memory.messages().len(), 3);
        assert_eq!(memory.messages()[0].content, "m3");
    }
}
