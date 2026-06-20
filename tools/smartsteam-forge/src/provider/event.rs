#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEvent {
    Stage(String),
    Delta(String),
    Meta(String),
    Final(String),
    Done,
    Error(String),
    Heartbeat(String),
    Status(String),
    Message { event: String, data: String },
}

impl StreamEvent {
    pub(crate) fn from_sse(event: &str, data: String) -> Self {
        match event {
            "stage" => Self::Stage(data),
            "delta" => Self::Delta(data),
            "meta" => Self::Meta(data),
            "final" => Self::Final(data),
            "done" => Self::Done,
            "error" => Self::Error(data),
            "heartbeat" => Self::Heartbeat(data),
            "status" => Self::Status(data),
            other => Self::Message {
                event: other.to_owned(),
                data,
            },
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done | Self::Error(_))
    }
}
