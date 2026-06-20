#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPromptMode {
    Stdin,
    Args,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandWireFormat {
    Text,
    Json,
}

impl CommandWireFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandTextOutputFilter {
    None,
    MistralRsCli,
}
