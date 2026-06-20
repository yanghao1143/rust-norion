#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamEndpoint {
    Chat,
    Generate,
    BusinessCycle,
}

impl StreamEndpoint {
    pub fn label(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Generate => "generate",
            Self::BusinessCycle => "business-cycle",
        }
    }

    pub(crate) fn stream_path(self) -> &'static str {
        match self {
            Self::Chat => "/v1/chat-stream",
            Self::Generate => "/v1/generate-stream",
            Self::BusinessCycle => "/v1/business-cycle-stream",
        }
    }
}
