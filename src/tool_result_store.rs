use crate::privacy_redaction::contains_private_or_executable_marker;
use crate::router::ToolResultProjectionTrace;
use norion_core::RuntimeToolResultProjectionBudget;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const METADATA_SCHEMA: &str = "rust-norion-tool-result-evidence-v1";
const METADATA_FILE: &str = "metadata.txt";
const PAYLOAD_FILE: &str = "payload.txt";
const MAX_METADATA_BYTES: u64 = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultStoreConfig {
    pub root: PathBuf,
    pub max_result_bytes: usize,
    pub max_total_bytes: u64,
    pub ttl_seconds: u64,
    pub projection_max_characters: usize,
    pub retrieval_max_characters: usize,
    pub max_grep_context_lines: usize,
}

impl ToolResultStoreConfig {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            max_result_bytes: 4 * 1024 * 1024,
            max_total_bytes: 64 * 1024 * 1024,
            ttl_seconds: 24 * 60 * 60,
            projection_max_characters: 2_048,
            retrieval_max_characters: 8_192,
            max_grep_context_lines: 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolResultProjectionStatus {
    Stored,
    DigestOnly,
    Reused,
}

impl ToolResultProjectionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stored => "stored",
            Self::DigestOnly => "digest_only",
            Self::Reused => "reused",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultEvidenceHandle {
    pub handle: String,
    pub tool_name: String,
    pub session_digest: String,
    pub character_count: usize,
    pub byte_count: usize,
    pub full_sha256: String,
    pub created_at_unix_seconds: u64,
    pub projection_status: ToolResultProjectionStatus,
    pub digest_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultProjection {
    pub evidence: ToolResultEvidenceHandle,
    pub provider_projection: String,
    pub omitted_characters: usize,
    pub budget: RuntimeToolResultProjectionBudget,
}

impl ToolResultProjection {
    pub fn trace_evidence(&self) -> ToolResultProjectionTrace {
        ToolResultProjectionTrace::new(&self.evidence.tool_name, self.budget)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolResultQuery {
    Metadata,
    Slice {
        start_character: usize,
        max_characters: usize,
    },
    Grep {
        needle: String,
        context_lines: usize,
        max_characters: usize,
    },
    HeadTail {
        head_characters: usize,
        tail_characters: usize,
        max_characters: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolResultRetrievalMode {
    Metadata,
    Slice,
    Grep,
    HeadTail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultRetrieval {
    pub mode: ToolResultRetrievalMode,
    pub text: String,
    pub character_count: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToolResultCleanupReport {
    pub scanned_records: usize,
    pub removed_records: usize,
    pub bytes_freed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolResultStoreError {
    InvalidSession,
    InvalidToolName,
    InvalidHandle,
    InvalidQuery,
    NotFound,
    DigestOnly,
    ResultTooLarge { bytes: usize, max_bytes: usize },
    DiskBudgetExceeded { required: u64, max_bytes: u64 },
    IntegrityCheckFailed,
    Io(String),
}

impl fmt::Display for ToolResultStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSession => formatter.write_str("invalid tool-result session scope"),
            Self::InvalidToolName => formatter.write_str("invalid tool-result tool name"),
            Self::InvalidHandle => formatter.write_str("invalid tool-result handle"),
            Self::InvalidQuery => formatter.write_str("invalid tool-result retrieval query"),
            Self::NotFound => formatter.write_str("tool-result handle not found in session"),
            Self::DigestOnly => formatter.write_str("tool-result payload is digest-only"),
            Self::ResultTooLarge { bytes, max_bytes } => {
                write!(formatter, "tool-result is too large: {bytes}>{max_bytes}")
            }
            Self::DiskBudgetExceeded {
                required,
                max_bytes,
            } => write!(
                formatter,
                "tool-result disk budget exceeded: {required}>{max_bytes}"
            ),
            Self::IntegrityCheckFailed => formatter.write_str("tool-result integrity check failed"),
            Self::Io(action) => write!(formatter, "tool-result I/O failed: {action}"),
        }
    }
}

impl std::error::Error for ToolResultStoreError {}

#[derive(Debug, Clone)]
pub struct ToolResultStore {
    config: ToolResultStoreConfig,
}

impl ToolResultStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self::with_config(ToolResultStoreConfig::new(root))
    }

    pub fn with_config(config: ToolResultStoreConfig) -> Self {
        Self { config }
    }

    pub fn store(
        &self,
        session_scope: &str,
        tool_name: &str,
        payload: &str,
    ) -> Result<ToolResultProjection, ToolResultStoreError> {
        self.store_at(
            session_scope,
            tool_name,
            payload,
            unix_seconds(SystemTime::now()),
        )
    }

    pub fn retrieve(
        &self,
        session_scope: &str,
        handle: &str,
        query: ToolResultQuery,
    ) -> Result<ToolResultRetrieval, ToolResultStoreError> {
        let (metadata, payload) = self.read_record(session_scope, handle)?;
        match query {
            ToolResultQuery::Metadata => {
                let text = metadata_summary(&metadata);
                Ok(bounded_retrieval(
                    ToolResultRetrievalMode::Metadata,
                    &text,
                    self.config.retrieval_max_characters.max(1),
                ))
            }
            ToolResultQuery::Slice {
                start_character,
                max_characters,
            } => {
                let payload = payload.ok_or(ToolResultStoreError::DigestOnly)?;
                let limit = self.retrieval_limit(max_characters);
                let text = payload
                    .chars()
                    .skip(start_character)
                    .take(limit)
                    .collect::<String>();
                let remaining = payload.chars().count().saturating_sub(start_character);
                Ok(ToolResultRetrieval {
                    mode: ToolResultRetrievalMode::Slice,
                    character_count: text.chars().count(),
                    truncated: remaining > limit,
                    text,
                })
            }
            ToolResultQuery::Grep {
                needle,
                context_lines,
                max_characters,
            } => {
                if needle.is_empty() {
                    return Err(ToolResultStoreError::InvalidQuery);
                }
                let payload = payload.ok_or(ToolResultStoreError::DigestOnly)?;
                let text = grep_with_context(
                    &payload,
                    &needle,
                    context_lines.min(self.config.max_grep_context_lines),
                );
                Ok(bounded_retrieval(
                    ToolResultRetrievalMode::Grep,
                    &text,
                    self.retrieval_limit(max_characters),
                ))
            }
            ToolResultQuery::HeadTail {
                head_characters,
                tail_characters,
                max_characters,
            } => {
                let payload = payload.ok_or(ToolResultStoreError::DigestOnly)?;
                let text = head_tail(&payload, head_characters, tail_characters);
                Ok(bounded_retrieval(
                    ToolResultRetrievalMode::HeadTail,
                    &text,
                    self.retrieval_limit(max_characters),
                ))
            }
        }
    }

    pub fn cleanup_expired(&self) -> Result<ToolResultCleanupReport, ToolResultStoreError> {
        self.cleanup_expired_at(unix_seconds(SystemTime::now()))
    }

    fn store_at(
        &self,
        session_scope: &str,
        tool_name: &str,
        payload: &str,
        now: u64,
    ) -> Result<ToolResultProjection, ToolResultStoreError> {
        let session = SessionKey::new(session_scope)?;
        let tool_name = sanitize_tool_name(tool_name)?;
        if payload.len() > self.config.max_result_bytes {
            return Err(ToolResultStoreError::ResultTooLarge {
                bytes: payload.len(),
                max_bytes: self.config.max_result_bytes,
            });
        }

        let full_sha256 = sha256_hex(payload.as_bytes());
        let handle = format!("tr-{}", &full_sha256[..32]);
        self.cleanup_expired_at(now)?;

        let record_dir = self.record_dir(&session, &handle);
        if record_dir.is_dir() {
            let (mut metadata, existing_payload) = self.read_record(session_scope, &handle)?;
            if metadata.full_sha256 != full_sha256 {
                return Err(ToolResultStoreError::IntegrityCheckFailed);
            }
            metadata.projection_status = ToolResultProjectionStatus::Reused;
            return Ok(self.project(metadata, existing_payload.as_deref()));
        }

        let digest_only = contains_private_or_executable_marker(payload);
        let metadata = ToolResultEvidenceHandle {
            handle: handle.clone(),
            tool_name,
            session_digest: session.digest.clone(),
            character_count: payload.chars().count(),
            byte_count: payload.len(),
            full_sha256,
            created_at_unix_seconds: now,
            projection_status: if digest_only {
                ToolResultProjectionStatus::DigestOnly
            } else {
                ToolResultProjectionStatus::Stored
            },
            digest_only,
        };
        let encoded_metadata = encode_metadata(&metadata);
        let required = self
            .disk_usage_bytes()?
            .saturating_add(encoded_metadata.len() as u64)
            .saturating_add(if digest_only { 0 } else { payload.len() as u64 });
        if required > self.config.max_total_bytes {
            return Err(ToolResultStoreError::DiskBudgetExceeded {
                required,
                max_bytes: self.config.max_total_bytes,
            });
        }

        fs::create_dir_all(self.session_dir(&session))
            .map_err(|error| io_error("create session directory", error))?;
        self.write_record_atomically(
            &record_dir,
            &encoded_metadata,
            (!digest_only).then_some(payload),
        )?;
        Ok(self.project(metadata, (!digest_only).then_some(payload)))
    }

    fn project(
        &self,
        metadata: ToolResultEvidenceHandle,
        payload: Option<&str>,
    ) -> ToolResultProjection {
        let (provider_projection, omitted_characters) = bounded_projection(
            &metadata,
            payload,
            self.config.projection_max_characters.max(128),
        );
        let budget = RuntimeToolResultProjectionBudget::new(
            metadata.character_count,
            provider_projection.chars().count(),
            true,
            metadata.digest_only,
        );
        ToolResultProjection {
            evidence: metadata,
            provider_projection,
            omitted_characters,
            budget,
        }
    }

    fn read_record(
        &self,
        session_scope: &str,
        handle: &str,
    ) -> Result<(ToolResultEvidenceHandle, Option<String>), ToolResultStoreError> {
        validate_handle(handle)?;
        let session = SessionKey::new(session_scope)?;
        let record_dir = self.record_dir(&session, handle);
        if !record_dir.is_dir() {
            return Err(ToolResultStoreError::NotFound);
        }
        let metadata = read_metadata(&record_dir)?;
        if metadata.handle != handle
            || metadata.session_digest != session.digest
            || session.directory_name()
                != session_directory_name_from_digest(&metadata.session_digest)?
        {
            return Err(ToolResultStoreError::IntegrityCheckFailed);
        }

        let payload_path = record_dir.join(PAYLOAD_FILE);
        if metadata.digest_only {
            if payload_path.exists() {
                return Err(ToolResultStoreError::IntegrityCheckFailed);
            }
            return Ok((metadata, None));
        }

        let file_size = fs::metadata(&payload_path)
            .map_err(|error| io_error("read payload metadata", error))?
            .len();
        if file_size != metadata.byte_count as u64
            || file_size > self.config.max_result_bytes as u64
        {
            return Err(ToolResultStoreError::IntegrityCheckFailed);
        }
        let bytes = fs::read(&payload_path).map_err(|error| io_error("read payload", error))?;
        if sha256_hex(&bytes) != metadata.full_sha256 {
            return Err(ToolResultStoreError::IntegrityCheckFailed);
        }
        let payload =
            String::from_utf8(bytes).map_err(|_| ToolResultStoreError::IntegrityCheckFailed)?;
        if payload.chars().count() != metadata.character_count {
            return Err(ToolResultStoreError::IntegrityCheckFailed);
        }
        Ok((metadata, Some(payload)))
    }

    fn write_record_atomically(
        &self,
        record_dir: &Path,
        metadata: &str,
        payload: Option<&str>,
    ) -> Result<(), ToolResultStoreError> {
        let parent = record_dir
            .parent()
            .ok_or(ToolResultStoreError::IntegrityCheckFailed)?;
        let handle = record_dir
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(ToolResultStoreError::InvalidHandle)?;
        validate_handle(handle)?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_dir = parent.join(format!(".{handle}.{}.{}.tmp", std::process::id(), nonce));
        fs::create_dir(&temp_dir).map_err(|error| io_error("create temporary record", error))?;

        let write_result = (|| {
            fs::write(temp_dir.join(METADATA_FILE), metadata)
                .map_err(|error| io_error("write metadata", error))?;
            if let Some(payload) = payload {
                fs::write(temp_dir.join(PAYLOAD_FILE), payload)
                    .map_err(|error| io_error("write payload", error))?;
            }
            if let Err(error) = fs::rename(&temp_dir, record_dir) {
                if record_dir.is_dir() {
                    fs::remove_dir_all(&temp_dir).map_err(|remove_error| {
                        io_error("remove duplicate record", remove_error)
                    })?;
                } else {
                    return Err(io_error("commit record", error));
                }
            }
            Ok(())
        })();

        if write_result.is_err() && temp_dir.exists() {
            let _ = fs::remove_dir_all(&temp_dir);
        }
        write_result
    }

    fn cleanup_expired_at(
        &self,
        now: u64,
    ) -> Result<ToolResultCleanupReport, ToolResultStoreError> {
        let mut report = ToolResultCleanupReport::default();
        if !self.config.root.is_dir() {
            return Ok(report);
        }
        for session_entry in fs::read_dir(&self.config.root)
            .map_err(|error| io_error("scan artifact root", error))?
        {
            let session_entry =
                session_entry.map_err(|error| io_error("read session entry", error))?;
            let Some(session_name) = session_entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !valid_session_directory_name(&session_name) || !session_entry.path().is_dir() {
                continue;
            }
            for record_entry in fs::read_dir(session_entry.path())
                .map_err(|error| io_error("scan session records", error))?
            {
                let record_entry =
                    record_entry.map_err(|error| io_error("read record entry", error))?;
                let Some(handle) = record_entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                if validate_handle(&handle).is_err() || !record_entry.path().is_dir() {
                    continue;
                }
                let Ok(metadata) = read_metadata(&record_entry.path()) else {
                    continue;
                };
                if metadata.handle != handle
                    || session_directory_name_from_digest(&metadata.session_digest)
                        .ok()
                        .as_deref()
                        != Some(&session_name)
                {
                    continue;
                }
                report.scanned_records = report.scanned_records.saturating_add(1);
                if metadata
                    .created_at_unix_seconds
                    .saturating_add(self.config.ttl_seconds)
                    <= now
                {
                    let bytes = directory_file_bytes(&record_entry.path())?;
                    fs::remove_dir_all(record_entry.path())
                        .map_err(|error| io_error("remove expired record", error))?;
                    report.removed_records = report.removed_records.saturating_add(1);
                    report.bytes_freed = report.bytes_freed.saturating_add(bytes);
                }
            }
            if fs::read_dir(session_entry.path())
                .map_err(|error| io_error("check empty session", error))?
                .next()
                .is_none()
            {
                fs::remove_dir(session_entry.path())
                    .map_err(|error| io_error("remove empty session", error))?;
            }
        }
        Ok(report)
    }

    fn disk_usage_bytes(&self) -> Result<u64, ToolResultStoreError> {
        if !self.config.root.is_dir() {
            return Ok(0);
        }
        let mut total = 0u64;
        for session_entry in fs::read_dir(&self.config.root)
            .map_err(|error| io_error("scan disk budget root", error))?
        {
            let session_entry =
                session_entry.map_err(|error| io_error("read disk budget session", error))?;
            let Some(session_name) = session_entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !valid_session_directory_name(&session_name) || !session_entry.path().is_dir() {
                continue;
            }
            for record_entry in fs::read_dir(session_entry.path())
                .map_err(|error| io_error("scan disk budget records", error))?
            {
                let record_entry =
                    record_entry.map_err(|error| io_error("read disk budget record", error))?;
                let Some(handle) = record_entry.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                if validate_handle(&handle).is_ok() && record_entry.path().is_dir() {
                    total = total.saturating_add(directory_file_bytes(&record_entry.path())?);
                }
            }
        }
        Ok(total)
    }

    fn session_dir(&self, session: &SessionKey) -> PathBuf {
        self.config.root.join(session.directory_name())
    }

    fn record_dir(&self, session: &SessionKey, handle: &str) -> PathBuf {
        self.session_dir(session).join(handle)
    }

    fn retrieval_limit(&self, requested: usize) -> usize {
        requested
            .max(1)
            .min(self.config.retrieval_max_characters.max(1))
    }
}

#[derive(Debug, Clone)]
struct SessionKey {
    digest: String,
}

impl SessionKey {
    fn new(scope: &str) -> Result<Self, ToolResultStoreError> {
        if scope.trim().is_empty() {
            return Err(ToolResultStoreError::InvalidSession);
        }
        Ok(Self {
            digest: format!("sha256:{}", sha256_hex(scope.as_bytes())),
        })
    }

    fn directory_name(&self) -> String {
        session_directory_name_from_digest(&self.digest).expect("generated session digest")
    }
}

fn bounded_projection(
    metadata: &ToolResultEvidenceHandle,
    payload: Option<&str>,
    max_characters: usize,
) -> (String, usize) {
    let header = format!(
        "[tool-result handle={} tool={} chars={} bytes={} status={}]\n",
        metadata.handle,
        metadata.tool_name,
        metadata.character_count,
        metadata.byte_count,
        metadata.projection_status.as_str(),
    );
    if metadata.digest_only || payload.is_none() {
        return (
            truncate_characters(
                &format!("{header}[digest-only raw payload not persisted]"),
                max_characters,
            ),
            metadata.character_count,
        );
    }

    let payload = payload.unwrap_or_default();
    let payload_characters = payload.chars().count();
    let evidence_budget = max_characters
        .saturating_sub(header.chars().count())
        .saturating_sub(48);
    if payload_characters <= evidence_budget {
        return (
            truncate_characters(&format!("{header}{payload}"), max_characters),
            0,
        );
    }
    let head_characters = evidence_budget / 2;
    let tail_characters = evidence_budget.saturating_sub(head_characters);
    let omitted = payload_characters
        .saturating_sub(head_characters)
        .saturating_sub(tail_characters);
    let head = payload.chars().take(head_characters).collect::<String>();
    let tail = payload
        .chars()
        .rev()
        .take(tail_characters)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    (
        truncate_characters(
            &format!("{header}{head}\n...[omitted_characters={omitted}]...\n{tail}"),
            max_characters,
        ),
        omitted,
    )
}

fn metadata_summary(metadata: &ToolResultEvidenceHandle) -> String {
    format!(
        "handle={} tool={} session_digest={} characters={} bytes={} sha256={} created_at={} status={} digest_only={}",
        metadata.handle,
        metadata.tool_name,
        metadata.session_digest,
        metadata.character_count,
        metadata.byte_count,
        metadata.full_sha256,
        metadata.created_at_unix_seconds,
        metadata.projection_status.as_str(),
        metadata.digest_only,
    )
}

fn bounded_retrieval(
    mode: ToolResultRetrievalMode,
    text: &str,
    max_characters: usize,
) -> ToolResultRetrieval {
    let original_characters = text.chars().count();
    let text = truncate_characters(text, max_characters);
    ToolResultRetrieval {
        mode,
        character_count: text.chars().count(),
        truncated: original_characters > max_characters,
        text,
    }
}

fn grep_with_context(payload: &str, needle: &str, context_lines: usize) -> String {
    let lines = payload.lines().collect::<Vec<_>>();
    let mut selected = vec![false; lines.len()];
    for (index, line) in lines.iter().enumerate() {
        if line.contains(needle) {
            let start = index.saturating_sub(context_lines);
            let end = index
                .saturating_add(context_lines)
                .saturating_add(1)
                .min(lines.len());
            for keep in &mut selected[start..end] {
                *keep = true;
            }
        }
    }

    let mut output = String::new();
    let mut previous_selected = false;
    for (index, line) in lines.iter().enumerate() {
        if !selected[index] {
            previous_selected = false;
            continue;
        }
        if !output.is_empty() {
            output.push('\n');
            if !previous_selected {
                output.push_str("...\n");
            }
        }
        output.push_str(line);
        previous_selected = true;
    }
    output
}

fn head_tail(payload: &str, head_characters: usize, tail_characters: usize) -> String {
    let total = payload.chars().count();
    if total <= head_characters.saturating_add(tail_characters) {
        return payload.to_owned();
    }
    let head = payload.chars().take(head_characters).collect::<String>();
    let tail = payload
        .chars()
        .rev()
        .take(tail_characters)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{head}\n...\n{tail}")
}

fn encode_metadata(metadata: &ToolResultEvidenceHandle) -> String {
    format!(
        "schema={METADATA_SCHEMA}\nhandle={}\ntool_name={}\nsession_digest={}\ncharacter_count={}\nbyte_count={}\nfull_sha256={}\ncreated_at_unix_seconds={}\nprojection_status={}\ndigest_only={}\n",
        metadata.handle,
        metadata.tool_name,
        metadata.session_digest,
        metadata.character_count,
        metadata.byte_count,
        metadata.full_sha256,
        metadata.created_at_unix_seconds,
        metadata.projection_status.as_str(),
        metadata.digest_only,
    )
}

fn read_metadata(record_dir: &Path) -> Result<ToolResultEvidenceHandle, ToolResultStoreError> {
    let path = record_dir.join(METADATA_FILE);
    let size = fs::metadata(&path)
        .map_err(|error| io_error("read metadata size", error))?
        .len();
    if size > MAX_METADATA_BYTES {
        return Err(ToolResultStoreError::IntegrityCheckFailed);
    }
    let text = fs::read_to_string(path).map_err(|error| io_error("read metadata", error))?;
    parse_metadata(&text)
}

fn parse_metadata(text: &str) -> Result<ToolResultEvidenceHandle, ToolResultStoreError> {
    if metadata_field(text, "schema")? != METADATA_SCHEMA {
        return Err(ToolResultStoreError::IntegrityCheckFailed);
    }
    let handle = metadata_field(text, "handle")?.to_owned();
    validate_handle(&handle)?;
    let tool_name = metadata_field(text, "tool_name")?.to_owned();
    if sanitize_tool_name(&tool_name)? != tool_name {
        return Err(ToolResultStoreError::IntegrityCheckFailed);
    }
    let session_digest = metadata_field(text, "session_digest")?.to_owned();
    session_directory_name_from_digest(&session_digest)?;
    let character_count = parse_usize(metadata_field(text, "character_count")?)?;
    let byte_count = parse_usize(metadata_field(text, "byte_count")?)?;
    let full_sha256 = metadata_field(text, "full_sha256")?.to_owned();
    if !is_lower_hex(&full_sha256, 64) {
        return Err(ToolResultStoreError::IntegrityCheckFailed);
    }
    let created_at_unix_seconds = parse_u64(metadata_field(text, "created_at_unix_seconds")?)?;
    let projection_status = match metadata_field(text, "projection_status")? {
        "stored" => ToolResultProjectionStatus::Stored,
        "digest_only" => ToolResultProjectionStatus::DigestOnly,
        _ => return Err(ToolResultStoreError::IntegrityCheckFailed),
    };
    let digest_only = match metadata_field(text, "digest_only")? {
        "true" => true,
        "false" => false,
        _ => return Err(ToolResultStoreError::IntegrityCheckFailed),
    };
    if digest_only != (projection_status == ToolResultProjectionStatus::DigestOnly) {
        return Err(ToolResultStoreError::IntegrityCheckFailed);
    }
    Ok(ToolResultEvidenceHandle {
        handle,
        tool_name,
        session_digest,
        character_count,
        byte_count,
        full_sha256,
        created_at_unix_seconds,
        projection_status,
        digest_only,
    })
}

fn metadata_field<'a>(text: &'a str, field: &str) -> Result<&'a str, ToolResultStoreError> {
    let prefix = format!("{field}=");
    let mut matches = text.lines().filter_map(|line| line.strip_prefix(&prefix));
    let value = matches
        .next()
        .ok_or(ToolResultStoreError::IntegrityCheckFailed)?;
    if matches.next().is_some() {
        return Err(ToolResultStoreError::IntegrityCheckFailed);
    }
    Ok(value)
}

fn parse_usize(value: &str) -> Result<usize, ToolResultStoreError> {
    value
        .parse()
        .map_err(|_| ToolResultStoreError::IntegrityCheckFailed)
}

fn parse_u64(value: &str) -> Result<u64, ToolResultStoreError> {
    value
        .parse()
        .map_err(|_| ToolResultStoreError::IntegrityCheckFailed)
}

fn validate_handle(handle: &str) -> Result<(), ToolResultStoreError> {
    if handle.len() == 35 && handle.starts_with("tr-") && is_lower_hex(&handle[3..], 32) {
        Ok(())
    } else {
        Err(ToolResultStoreError::InvalidHandle)
    }
}

fn valid_session_directory_name(name: &str) -> bool {
    name.len() == 72 && name.starts_with("session-") && is_lower_hex(&name[8..], 64)
}

fn session_directory_name_from_digest(digest: &str) -> Result<String, ToolResultStoreError> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err(ToolResultStoreError::IntegrityCheckFailed);
    };
    if !is_lower_hex(hex, 64) {
        return Err(ToolResultStoreError::IntegrityCheckFailed);
    }
    Ok(format!("session-{hex}"))
}

fn is_lower_hex(value: &str, expected_length: usize) -> bool {
    value.len() == expected_length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sanitize_tool_name(value: &str) -> Result<String, ToolResultStoreError> {
    if value.trim().is_empty() {
        return Err(ToolResultStoreError::InvalidToolName);
    }
    let mut output = String::new();
    for character in value.chars().take(64) {
        if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
            output.push(character);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        Err(ToolResultStoreError::InvalidToolName)
    } else {
        Ok(output)
    }
}

fn truncate_characters(value: &str, max_characters: usize) -> String {
    value.chars().take(max_characters).collect()
}

fn directory_file_bytes(path: &Path) -> Result<u64, ToolResultStoreError> {
    let mut bytes = 0u64;
    for entry in fs::read_dir(path).map_err(|error| io_error("scan record files", error))? {
        let entry = entry.map_err(|error| io_error("read record file", error))?;
        let metadata = entry
            .metadata()
            .map_err(|error| io_error("read record file metadata", error))?;
        if metadata.is_file() {
            bytes = bytes.saturating_add(metadata.len());
        }
    }
    Ok(bytes)
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn unix_seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn io_error(action: &str, _error: io::Error) -> ToolResultStoreError {
    ToolResultStoreError::Io(action.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            Self(std::env::temp_dir().join(format!(
                "rust-norion-tool-result-{name}-{}-{nonce}",
                std::process::id()
            )))
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn stores_projects_and_reuses_content_addressed_result() {
        let root = TestRoot::new("reuse");
        let mut config = ToolResultStoreConfig::new(&root.0);
        config.projection_max_characters = 320;
        let store = ToolResultStore::with_config(config);
        let payload = (0..200)
            .map(|index| format!("line-{index:03} value-{index:03}"))
            .collect::<Vec<_>>()
            .join("\n");

        let first = store.store("session-a", "cargo-test", &payload).unwrap();
        let second = store.store("session-a", "cargo-test", &payload).unwrap();

        assert!(first.evidence.handle.starts_with("tr-"));
        assert_eq!(first.evidence.handle, second.evidence.handle);
        assert_eq!(
            first.evidence.projection_status,
            ToolResultProjectionStatus::Stored
        );
        assert_eq!(
            second.evidence.projection_status,
            ToolResultProjectionStatus::Reused
        );
        assert!(first.provider_projection.chars().count() <= 320);
        assert!(first.omitted_characters > 0);
        assert!(first.budget.tokens_saved > 0);
        assert!(first.budget.accounting_is_consistent());
        let trace = first.trace_evidence().to_json();
        assert!(trace.contains("\"handle_present\":true"));
        assert!(!trace.contains("line-000"));

        let session = SessionKey::new("session-a").unwrap();
        assert_eq!(
            fs::read_dir(store.session_dir(&session)).unwrap().count(),
            1
        );
    }

    #[test]
    fn retrieves_metadata_slice_grep_and_head_tail_with_bounds() {
        let root = TestRoot::new("retrieve");
        let store = ToolResultStore::new(&root.0);
        let payload = "zero\nalpha\nbefore\nneedle target\nafter\nomega\nend";
        let projection = store.store("session-a", "file-read", payload).unwrap();
        let handle = &projection.evidence.handle;

        let metadata = store
            .retrieve("session-a", handle, ToolResultQuery::Metadata)
            .unwrap();
        assert!(metadata.text.contains(handle));
        assert!(!metadata.text.contains("needle target"));

        let start = payload.find("alpha").unwrap();
        let slice = store
            .retrieve(
                "session-a",
                handle,
                ToolResultQuery::Slice {
                    start_character: start,
                    max_characters: 5,
                },
            )
            .unwrap();
        assert_eq!(slice.text, "alpha");

        let grep = store
            .retrieve(
                "session-a",
                handle,
                ToolResultQuery::Grep {
                    needle: "needle".to_owned(),
                    context_lines: 1,
                    max_characters: 40,
                },
            )
            .unwrap();
        assert_eq!(grep.text, "before\nneedle target\nafter");
        assert!(grep.character_count <= 40);

        let head_tail = store
            .retrieve(
                "session-a",
                handle,
                ToolResultQuery::HeadTail {
                    head_characters: 4,
                    tail_characters: 3,
                    max_characters: 16,
                },
            )
            .unwrap();
        assert_eq!(head_tail.text, "zero\n...\nend");
    }

    #[test]
    fn rejects_cross_session_malformed_and_hash_mismatched_reads() {
        let root = TestRoot::new("integrity");
        let store = ToolResultStore::new(&root.0);
        let projection = store
            .store("session-a", "search", "alpha beta gamma")
            .unwrap();
        let handle = projection.evidence.handle;

        assert_eq!(
            store
                .retrieve("session-b", &handle, ToolResultQuery::Metadata)
                .unwrap_err(),
            ToolResultStoreError::NotFound
        );
        assert_eq!(
            store
                .retrieve("session-a", "../payload.txt", ToolResultQuery::Metadata)
                .unwrap_err(),
            ToolResultStoreError::InvalidHandle
        );

        let session = SessionKey::new("session-a").unwrap();
        fs::write(
            store.record_dir(&session, &handle).join(PAYLOAD_FILE),
            "tampered",
        )
        .unwrap();
        assert_eq!(
            store
                .retrieve("session-a", &handle, ToolResultQuery::Metadata)
                .unwrap_err(),
            ToolResultStoreError::IntegrityCheckFailed
        );
    }

    #[test]
    fn sensitive_results_persist_digest_only_metadata() {
        let root = TestRoot::new("sensitive");
        let store = ToolResultStore::new(&root.0);
        let payload = "api_key=secret-value";
        let projection = store.store("session-a", "http", payload).unwrap();
        let session = SessionKey::new("session-a").unwrap();
        let record_dir = store.record_dir(&session, &projection.evidence.handle);

        assert!(projection.evidence.digest_only);
        assert_eq!(
            projection.evidence.projection_status,
            ToolResultProjectionStatus::DigestOnly
        );
        assert!(!record_dir.join(PAYLOAD_FILE).exists());
        assert!(!projection.provider_projection.contains("secret-value"));
        assert_eq!(
            store
                .retrieve(
                    "session-a",
                    &projection.evidence.handle,
                    ToolResultQuery::Slice {
                        start_character: 0,
                        max_characters: 10,
                    },
                )
                .unwrap_err(),
            ToolResultStoreError::DigestOnly
        );
        let metadata = fs::read_to_string(record_dir.join(METADATA_FILE)).unwrap();
        assert!(!metadata.contains("secret-value"));
    }

    #[test]
    fn enforces_result_disk_budget_and_ttl_cleanup() {
        let root = TestRoot::new("limits");
        let mut result_config = ToolResultStoreConfig::new(root.0.join("result"));
        result_config.max_result_bytes = 4;
        let result_store = ToolResultStore::with_config(result_config);
        assert!(matches!(
            result_store.store("session-a", "tool", "12345"),
            Err(ToolResultStoreError::ResultTooLarge { .. })
        ));

        let mut disk_config = ToolResultStoreConfig::new(root.0.join("disk"));
        disk_config.max_total_bytes = 1;
        let disk_store = ToolResultStore::with_config(disk_config);
        assert!(matches!(
            disk_store.store("session-a", "tool", "small"),
            Err(ToolResultStoreError::DiskBudgetExceeded { .. })
        ));

        let mut ttl_config = ToolResultStoreConfig::new(root.0.join("ttl"));
        ttl_config.ttl_seconds = 5;
        let ttl_store = ToolResultStore::with_config(ttl_config);
        ttl_store
            .store_at("session-a", "tool", "expiring evidence", 10)
            .unwrap();
        let report = ttl_store.cleanup_expired_at(20).unwrap();
        assert_eq!(report.scanned_records, 1);
        assert_eq!(report.removed_records, 1);
        assert!(report.bytes_freed > 0);
        assert_eq!(fs::read_dir(&ttl_store.config.root).unwrap().count(), 0);
    }
}
