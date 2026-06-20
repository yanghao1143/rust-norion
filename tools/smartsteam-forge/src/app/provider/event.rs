#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderEvent {
    Delta(String),
    ReplaceAssistant(String),
    GateReport(String),
    Stage(String),
    Meta(String),
    Status(String),
    Heartbeat(String),
    Done,
    Error(String),
}
