mod catalog;
mod helpers;
mod model;
mod summary;

#[cfg(test)]
mod tests;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::provider::json::json_string;

pub use catalog::{list_recent_sessions, list_recent_sessions_filtered};
use helpers::{unix_timestamp_millis, unix_timestamp_secs};
pub use model::{ResumedSession, SessionFilter, SessionRecord, StoredSession, TranscriptMessage};
pub use summary::{SessionSummary, summarize_recent_session};

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct SessionStore {
    root: PathBuf,
    current: StoredSession,
}

impl SessionStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|error| {
            format!(
                "create SmartSteam session directory {} failed: {error}",
                root.display()
            )
        })?;
        let current = create_session_file(&root)?;
        Ok(Self { root, current })
    }

    pub fn open_default() -> Result<Self, String> {
        Self::open(Self::default_root()?)
    }

    pub fn default_root() -> Result<PathBuf, String> {
        let cwd =
            std::env::current_dir().map_err(|error| format!("read current dir failed: {error}"))?;
        Ok(cwd.join("state").join("sessions"))
    }

    pub fn current(&self) -> &StoredSession {
        &self.current
    }

    pub fn transcript_path(&self) -> &Path {
        &self.current.transcript_path
    }

    pub fn rotate(&mut self) -> Result<StoredSession, String> {
        self.current = create_session_file(&self.root)?;
        Ok(self.current.clone())
    }

    pub fn list_recent(&self, limit: usize) -> Result<Vec<SessionRecord>, String> {
        list_recent_sessions(&self.root, limit)
    }

    pub fn list_recent_filtered(
        &self,
        filter: SessionFilter,
        limit: usize,
    ) -> Result<Vec<SessionRecord>, String> {
        list_recent_sessions_filtered(&self.root, filter, limit)
    }

    pub fn resume(
        &mut self,
        selector: &str,
        max_messages: usize,
    ) -> Result<ResumedSession, String> {
        let records = self.list_recent(100)?;
        let record = catalog::select_record(&records, selector)?;
        let messages = catalog::read_transcript_messages(&record.transcript_path, max_messages)?;
        self.current = StoredSession {
            id: record.id.clone(),
            transcript_path: record.transcript_path.clone(),
        };
        self.append_event(
            "session_resume",
            &format!("resumed with {} messages", messages.len()),
        )?;
        Ok(ResumedSession { record, messages })
    }

    pub fn summarize(&self, selector: &str) -> Result<SessionSummary, String> {
        summarize_recent_session(&self.root, selector)
    }

    pub fn summarize_current(&self) -> Result<SessionSummary, String> {
        let record = catalog::read_session_record(&self.current.transcript_path)?;
        self.write_summary(record)
    }

    fn write_summary(&self, record: SessionRecord) -> Result<SessionSummary, String> {
        summary::write_session_summary(record)
    }

    pub fn append_message(&self, role: &str, content: &str) -> Result<(), String> {
        self.append_json_line(&format!(
            "{{\"ts\":{},\"session_id\":{},\"kind\":\"message\",\"role\":{},\"content\":{}}}",
            unix_timestamp_secs(),
            json_string(&self.current.id),
            json_string(role),
            json_string(content)
        ))
    }

    pub fn append_event(&self, kind: &str, content: &str) -> Result<(), String> {
        self.append_json_line(&format!(
            "{{\"ts\":{},\"session_id\":{},\"kind\":{},\"content\":{}}}",
            unix_timestamp_secs(),
            json_string(&self.current.id),
            json_string(kind),
            json_string(content)
        ))
    }

    fn append_json_line(&self, line: &str) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.current.transcript_path)
            .map_err(|error| {
                format!(
                    "open transcript {} failed: {error}",
                    self.current.transcript_path.display()
                )
            })?;
        writeln!(file, "{line}").map_err(|error| {
            format!(
                "write transcript {} failed: {error}",
                self.current.transcript_path.display()
            )
        })
    }
}

fn create_session_file(root: &Path) -> Result<StoredSession, String> {
    let counter = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let id = format!(
        "session_{}_{}_{}",
        unix_timestamp_millis(),
        std::process::id(),
        counter
    );
    let transcript_path = root.join(format!("{id}.jsonl"));
    let session = StoredSession {
        id,
        transcript_path,
    };
    let store = SessionStore {
        root: root.to_path_buf(),
        current: session.clone(),
    };
    store.append_event("session_start", "SmartSteam Forge session started")?;
    Ok(session)
}
