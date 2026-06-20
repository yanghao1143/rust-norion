mod parse;

#[cfg(test)]
mod tests;

use smartsteam_forge::{SessionFilter, StreamEndpoint};

pub use parse::parse_slash_command;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelPoolWatchCommand {
    pub interval_secs: u64,
    pub max_iterations: Option<usize>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SlashCommand {
    Help,
    Clear,
    New,
    Status,
    EvolutionStrictSummary(Option<String>),
    Ready,
    Hygiene,
    HygieneQuarantineDryRun(usize),
    ExperienceRepairDryRun(usize),
    ExperienceCleanupAudit(usize),
    RetrievalPreview { prompt: String, limit: usize },
    ModelPoolStatus,
    ModelPoolManifest,
    ModelPoolAdvice,
    ModelPoolSmoke,
    ModelPoolWatch(ModelPoolWatchCommand),
    ModelPoolRoute(String),
    ModelPoolCall { task_kind: String, prompt: String },
    Doctor,
    Cancel,
    Guard(bool),
    SafeDeviceGuard(bool),
    Show,
    Context,
    Sessions { filter: SessionFilter, limit: usize },
    Resume(String),
    Summary(String),
    NotesShow,
    NotesAdd(String),
    NotesSet(String),
    NotesClear,
    IndexNotesShow,
    IndexNotesClear,
    Mode(StreamEndpoint),
    Output(String),
    Profile(String),
    Feedback(String),
    SelfImprove(bool),
    ContextWindow(usize),
    MaxTokens(Option<usize>),
    RustCheckInline(String),
    RustCheckFile(String),
    RustCheckEdition(String),
    RustCheckCase(Option<String>),
    RustCheckClear,
    Quit,
    Unknown(String),
}
