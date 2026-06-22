use std::collections::BTreeSet;
use std::io;

use crate::disk_kv::DiskKvStore;
use crate::hierarchy::TaskProfile;
use crate::tenant_scope::{
    TenantAccessKind, TenantIsolationGate, TenantIsolationReport, TenantResourceLane, TenantScope,
    TenantScopedKey, TenantScopedKvWriteReport, tenant_scoped_get, tenant_scoped_put,
};

const SESSION_SCHEMA: &str = "rust-norion-session-state-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SessionAnchorKind {
    MemoryChain,
    GeneChain,
    RuntimeKv,
    Experience,
    Retrieval,
    Routing,
}

impl SessionAnchorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MemoryChain => "memory_chain",
            Self::GeneChain => "gene_chain",
            Self::RuntimeKv => "runtime_kv",
            Self::Experience => "experience",
            Self::Retrieval => "retrieval",
            Self::Routing => "routing",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "memory_chain" => Some(Self::MemoryChain),
            "gene_chain" => Some(Self::GeneChain),
            "runtime_kv" => Some(Self::RuntimeKv),
            "experience" => Some(Self::Experience),
            "retrieval" => Some(Self::Retrieval),
            "routing" => Some(Self::Routing),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStateAnchor {
    pub kind: SessionAnchorKind,
    pub anchor_id: String,
    pub evidence_digest: String,
}

impl SessionStateAnchor {
    pub fn new(
        kind: SessionAnchorKind,
        anchor_id: impl AsRef<str>,
        evidence_payload: impl AsRef<str>,
    ) -> Self {
        let evidence_payload = evidence_payload.as_ref();
        Self {
            kind,
            anchor_id: sanitize_identifier(anchor_id.as_ref(), kind.as_str()),
            evidence_digest: if evidence_payload.starts_with("fnv64:") {
                evidence_payload.to_owned()
            } else {
                stable_digest(evidence_payload)
            },
        }
    }

    fn summary(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.as_str(),
            self.anchor_id,
            self.evidence_digest
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTurnRole {
    User,
    Assistant,
    Tool,
    System,
}

impl SessionTurnRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
            Self::System => "system",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Self::User),
            "assistant" => Some(Self::Assistant),
            "tool" => Some(Self::Tool),
            "system" => Some(Self::System),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTurnDigest {
    pub turn_id: String,
    pub role: SessionTurnRole,
    pub payload_digest: String,
    pub summary_preview: String,
    pub token_estimate: usize,
}

impl SessionTurnDigest {
    pub fn from_payload(
        turn_id: impl AsRef<str>,
        role: SessionTurnRole,
        payload: impl AsRef<str>,
    ) -> Self {
        let payload = payload.as_ref();
        let digest = stable_digest(payload);
        Self {
            turn_id: sanitize_identifier(turn_id.as_ref(), "turn"),
            role,
            payload_digest: digest.clone(),
            summary_preview: format!("payload_digest:{digest}"),
            token_estimate: estimate_tokens(payload),
        }
    }

    pub fn from_summary(
        turn_id: impl AsRef<str>,
        role: SessionTurnRole,
        summary: impl AsRef<str>,
        payload_digest: impl AsRef<str>,
        token_estimate: usize,
    ) -> Self {
        Self {
            turn_id: sanitize_identifier(turn_id.as_ref(), "turn"),
            role,
            payload_digest: digest_or_hash(payload_digest.as_ref()),
            summary_preview: sanitize_public_text(summary.as_ref(), 96),
            token_estimate,
        }
    }

    fn summary(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.turn_id,
            self.role.as_str(),
            self.payload_digest,
            self.token_estimate,
            self.summary_preview
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRuntimeProfile {
    pub task_profile: TaskProfile,
    pub model_id: String,
    pub tokenizer: String,
    pub native_context_window: usize,
    pub max_tokens: usize,
    pub streaming_enabled: bool,
    pub cancellation_anchor_digest: Option<String>,
}

impl SessionRuntimeProfile {
    pub fn new(task_profile: TaskProfile) -> Self {
        Self {
            task_profile,
            model_id: "unknown-self-developed-runtime".to_owned(),
            tokenizer: "unknown".to_owned(),
            native_context_window: 0,
            max_tokens: 256,
            streaming_enabled: false,
            cancellation_anchor_digest: None,
        }
    }

    pub fn with_model(mut self, model_id: impl AsRef<str>, tokenizer: impl AsRef<str>) -> Self {
        self.model_id = sanitize_identifier(model_id.as_ref(), "model");
        self.tokenizer = sanitize_identifier(tokenizer.as_ref(), "tokenizer");
        self
    }

    pub fn with_native_context_window(mut self, native_context_window: usize) -> Self {
        self.native_context_window = native_context_window;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens.max(1);
        self
    }

    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.streaming_enabled = enabled;
        self
    }

    pub fn with_cancellation_anchor(mut self, anchor: impl AsRef<str>) -> Self {
        self.cancellation_anchor_digest = Some(digest_or_hash(anchor.as_ref()));
        self
    }

    fn summary(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            profile_slug(self.task_profile),
            self.model_id,
            self.tokenizer,
            self.native_context_window,
            self.max_tokens,
            self.streaming_enabled,
            self.cancellation_anchor_digest.as_deref().unwrap_or("none")
        )
    }
}

#[derive(Debug, Clone)]
pub struct SessionStateInput {
    pub state_id: String,
    pub scope: TenantScope,
    pub runtime_profile: SessionRuntimeProfile,
    pub memory_anchors: Vec<SessionStateAnchor>,
    pub gene_anchors: Vec<SessionStateAnchor>,
    pub retrieval_evidence: Vec<SessionStateAnchor>,
    pub routing_evidence: Vec<SessionStateAnchor>,
    pub turns: Vec<SessionTurnDigest>,
    pub source_trace_ids: Vec<String>,
}

impl SessionStateInput {
    pub fn new(
        state_id: impl AsRef<str>,
        scope: TenantScope,
        runtime_profile: SessionRuntimeProfile,
    ) -> Self {
        Self {
            state_id: state_id.as_ref().to_owned(),
            scope,
            runtime_profile,
            memory_anchors: Vec::new(),
            gene_anchors: Vec::new(),
            retrieval_evidence: Vec::new(),
            routing_evidence: Vec::new(),
            turns: Vec::new(),
            source_trace_ids: Vec::new(),
        }
    }

    pub fn with_memory_anchor(mut self, anchor: SessionStateAnchor) -> Self {
        self.memory_anchors.push(anchor);
        self
    }

    pub fn with_gene_anchor(mut self, anchor: SessionStateAnchor) -> Self {
        self.gene_anchors.push(anchor);
        self
    }

    pub fn with_retrieval_evidence(mut self, evidence: SessionStateAnchor) -> Self {
        self.retrieval_evidence.push(evidence);
        self
    }

    pub fn with_routing_evidence(mut self, evidence: SessionStateAnchor) -> Self {
        self.routing_evidence.push(evidence);
        self
    }

    pub fn with_turn(mut self, turn: SessionTurnDigest) -> Self {
        self.turns.push(turn);
        self
    }

    pub fn with_source_trace_id(mut self, trace_id: impl AsRef<str>) -> Self {
        push_unique_string(
            &mut self.source_trace_ids,
            sanitize_identifier(trace_id.as_ref(), "trace"),
        );
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStateRecord {
    pub schema: String,
    pub state_id: String,
    pub scope: TenantScope,
    pub runtime_profile: SessionRuntimeProfile,
    pub memory_anchors: Vec<SessionStateAnchor>,
    pub gene_anchors: Vec<SessionStateAnchor>,
    pub retrieval_evidence: Vec<SessionStateAnchor>,
    pub routing_evidence: Vec<SessionStateAnchor>,
    pub turns: Vec<SessionTurnDigest>,
    pub source_trace_ids: Vec<String>,
    pub state_digest: String,
    pub raw_messages_stored: bool,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
    pub write_allowed: bool,
}

impl SessionStateRecord {
    pub fn from_input(input: SessionStateInput) -> Self {
        let mut record = Self {
            schema: SESSION_SCHEMA.to_owned(),
            state_id: sanitize_identifier(&input.state_id, "session-state"),
            scope: input.scope,
            runtime_profile: input.runtime_profile,
            memory_anchors: dedupe_anchors(input.memory_anchors),
            gene_anchors: dedupe_anchors(input.gene_anchors),
            retrieval_evidence: dedupe_anchors(input.retrieval_evidence),
            routing_evidence: dedupe_anchors(input.routing_evidence),
            turns: dedupe_turns(input.turns),
            source_trace_ids: sanitize_id_list(input.source_trace_ids),
            state_digest: String::new(),
            raw_messages_stored: false,
            read_only: true,
            report_only: true,
            preview_only: true,
            write_allowed: false,
        };
        record.state_digest = record.compute_digest();
        record
    }

    pub fn scoped_key(&self) -> TenantScopedKey {
        self.scope
            .scoped_key(TenantResourceLane::SessionState, &self.state_id)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.to_wire().into_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SessionStateDecodeError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| SessionStateDecodeError::new("session_state_invalid_utf8", bytes))?;
        Self::from_wire(text)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "session_state state={} scope={} profile={} memory_anchors={} gene_anchors={} retrieval_evidence={} routing_evidence={} turns={} raw_messages_stored={} read_only={} report_only={} preview_only={} write_allowed={} state_digest={}",
            self.state_id,
            self.scope.scope_digest(),
            profile_slug(self.runtime_profile.task_profile),
            self.memory_anchors.len(),
            self.gene_anchors.len(),
            self.retrieval_evidence.len(),
            self.routing_evidence.len(),
            self.turns.len(),
            self.raw_messages_stored,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.write_allowed,
            self.state_digest
        )
    }

    fn validation_blockers(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != SESSION_SCHEMA {
            blockers.push("session_state_schema_mismatch".to_owned());
        }
        if self.state_id.trim().is_empty() {
            blockers.push("session_state_id_missing".to_owned());
        }
        if self.memory_anchors.is_empty() {
            blockers.push("session_state_memory_anchor_missing".to_owned());
        }
        if self.gene_anchors.is_empty() {
            blockers.push("session_state_gene_anchor_missing".to_owned());
        }
        if self.raw_messages_stored {
            blockers.push("session_state_raw_messages_forbidden".to_owned());
        }
        if !self.read_only || !self.report_only || !self.preview_only || self.write_allowed {
            blockers.push("session_state_preview_flags_invalid".to_owned());
        }
        if self.compute_digest() != self.state_digest {
            blockers.push("session_state_digest_mismatch".to_owned());
        }
        blockers
    }

    fn compute_digest(&self) -> String {
        stable_digest(&format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.schema,
            self.state_id,
            self.scope.scope_digest(),
            self.runtime_profile.summary(),
            anchors_wire(&self.memory_anchors),
            anchors_wire(&self.gene_anchors),
            anchors_wire(&self.retrieval_evidence),
            anchors_wire(&self.routing_evidence),
            turns_wire(&self.turns),
            self.source_trace_ids.join(",")
        ))
    }

    fn to_wire(&self) -> String {
        [
            format!("schema={}", self.schema),
            format!("state_id={}", self.state_id),
            format!("tenant_id={}", self.scope.tenant_id),
            format!("workspace_id={}", self.scope.workspace_id),
            format!("session_id={}", self.scope.session_id),
            format!(
                "profile={}",
                profile_slug(self.runtime_profile.task_profile)
            ),
            format!("model_id={}", self.runtime_profile.model_id),
            format!("tokenizer={}", self.runtime_profile.tokenizer),
            format!(
                "native_context_window={}",
                self.runtime_profile.native_context_window
            ),
            format!("max_tokens={}", self.runtime_profile.max_tokens),
            format!(
                "streaming_enabled={}",
                self.runtime_profile.streaming_enabled
            ),
            format!(
                "cancellation_anchor_digest={}",
                self.runtime_profile
                    .cancellation_anchor_digest
                    .as_deref()
                    .unwrap_or("none")
            ),
            format!("memory_anchors={}", anchors_wire(&self.memory_anchors)),
            format!("gene_anchors={}", anchors_wire(&self.gene_anchors)),
            format!(
                "retrieval_evidence={}",
                anchors_wire(&self.retrieval_evidence)
            ),
            format!("routing_evidence={}", anchors_wire(&self.routing_evidence)),
            format!("turns={}", turns_wire(&self.turns)),
            format!("source_trace_ids={}", self.source_trace_ids.join(",")),
            format!("raw_messages_stored={}", self.raw_messages_stored),
            format!("read_only={}", self.read_only),
            format!("report_only={}", self.report_only),
            format!("preview_only={}", self.preview_only),
            format!("write_allowed={}", self.write_allowed),
            format!("state_digest={}", self.state_digest),
        ]
        .join("\n")
    }

    fn from_wire(text: &str) -> Result<Self, SessionStateDecodeError> {
        let map = parse_key_value_lines(text)?;
        require_value(&map, "schema", text).and_then(|schema| {
            if schema == SESSION_SCHEMA {
                Ok(())
            } else {
                Err(SessionStateDecodeError::new(
                    "session_state_schema_mismatch",
                    text.as_bytes(),
                ))
            }
        })?;
        let scope = TenantScope::new(
            require_value(&map, "tenant_id", text)?,
            require_value(&map, "workspace_id", text)?,
            require_value(&map, "session_id", text)?,
        );
        let profile = parse_profile(require_value(&map, "profile", text)?)
            .ok_or_else(|| SessionStateDecodeError::new("session_state_bad_profile", text))?;
        let runtime_profile = SessionRuntimeProfile {
            task_profile: profile,
            model_id: sanitize_identifier(require_value(&map, "model_id", text)?, "model"),
            tokenizer: sanitize_identifier(require_value(&map, "tokenizer", text)?, "tokenizer"),
            native_context_window: parse_usize(&map, "native_context_window", text)?,
            max_tokens: parse_usize(&map, "max_tokens", text)?.max(1),
            streaming_enabled: parse_bool(&map, "streaming_enabled", text)?,
            cancellation_anchor_digest: parse_optional_digest(require_value(
                &map,
                "cancellation_anchor_digest",
                text,
            )?),
        };
        let record = Self {
            schema: SESSION_SCHEMA.to_owned(),
            state_id: sanitize_identifier(require_value(&map, "state_id", text)?, "session-state"),
            scope,
            runtime_profile,
            memory_anchors: parse_anchors(require_value(&map, "memory_anchors", text)?, text)?,
            gene_anchors: parse_anchors(require_value(&map, "gene_anchors", text)?, text)?,
            retrieval_evidence: parse_anchors(
                require_value(&map, "retrieval_evidence", text)?,
                text,
            )?,
            routing_evidence: parse_anchors(require_value(&map, "routing_evidence", text)?, text)?,
            turns: parse_turns(require_value(&map, "turns", text)?, text)?,
            source_trace_ids: parse_id_list(require_value(&map, "source_trace_ids", text)?),
            raw_messages_stored: parse_bool(&map, "raw_messages_stored", text)?,
            read_only: parse_bool(&map, "read_only", text)?,
            report_only: parse_bool(&map, "report_only", text)?,
            preview_only: parse_bool(&map, "preview_only", text)?,
            write_allowed: parse_bool(&map, "write_allowed", text)?,
            state_digest: digest_or_hash(require_value(&map, "state_digest", text)?),
        };
        let blockers = record.validation_blockers();
        if blockers.is_empty() {
            Ok(record)
        } else {
            Err(SessionStateDecodeError::new(
                blockers
                    .first()
                    .map(String::as_str)
                    .unwrap_or("session_state_invalid"),
                text.as_bytes(),
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReplayPreview {
    pub allowed: bool,
    pub isolation: TenantIsolationReport,
    pub retrieval_inputs: Vec<String>,
    pub routing_inputs: Vec<String>,
    pub gene_inputs: Vec<String>,
    pub evidence_digest: String,
    pub blocked_reasons: Vec<String>,
    pub raw_payload_exposed: bool,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
}

impl SessionReplayPreview {
    pub fn summary_line(&self) -> String {
        format!(
            "session_replay_preview allowed={} retrieval_inputs={} routing_inputs={} gene_inputs={} raw_payload_exposed={} read_only={} report_only={} preview_only={} evidence_digest={} blocked_reasons={}",
            self.allowed,
            self.retrieval_inputs.len(),
            self.routing_inputs.len(),
            self.gene_inputs.len(),
            self.raw_payload_exposed,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.evidence_digest,
            self.blocked_reasons.len()
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionReplayPlanner;

impl SessionReplayPlanner {
    pub fn new() -> Self {
        Self
    }

    pub fn preview(
        &self,
        actor_scope: &TenantScope,
        record: &SessionStateRecord,
        max_inputs: usize,
    ) -> SessionReplayPreview {
        let key_digest = record.scoped_key().key_digest();
        let isolation = TenantIsolationGate::new().check_scope_access(
            actor_scope,
            &record.scope,
            TenantResourceLane::SessionState,
            TenantAccessKind::RollbackReplay,
            &key_digest,
        );
        let mut blocked_reasons = record.validation_blockers();
        if !isolation.allowed {
            blocked_reasons.push("session_replay_cross_tenant_rejected".to_owned());
        }
        let allowed = isolation.allowed && blocked_reasons.is_empty();
        let limit = max_inputs.max(1);
        let retrieval_inputs = if allowed {
            replay_inputs(
                record
                    .memory_anchors
                    .iter()
                    .chain(record.retrieval_evidence.iter()),
                limit,
            )
        } else {
            Vec::new()
        };
        let routing_inputs = if allowed {
            let mut inputs = replay_inputs(record.routing_evidence.iter(), limit);
            inputs.push(format!(
                "profile:{} model:{} window:{} max_tokens:{} streaming:{}",
                profile_slug(record.runtime_profile.task_profile),
                record.runtime_profile.model_id,
                record.runtime_profile.native_context_window,
                record.runtime_profile.max_tokens,
                record.runtime_profile.streaming_enabled
            ));
            inputs.truncate(limit);
            inputs
        } else {
            Vec::new()
        };
        let gene_inputs = if allowed {
            replay_inputs(record.gene_anchors.iter(), limit)
        } else {
            Vec::new()
        };
        let evidence_digest = stable_digest(&format!(
            "{}|{}|{}|{}|{}",
            record.state_digest,
            actor_scope.scope_digest(),
            retrieval_inputs.join("|"),
            routing_inputs.join("|"),
            gene_inputs.join("|")
        ));
        SessionReplayPreview {
            allowed,
            isolation,
            retrieval_inputs,
            routing_inputs,
            gene_inputs,
            evidence_digest,
            blocked_reasons,
            raw_payload_exposed: false,
            read_only: true,
            report_only: true,
            preview_only: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionStateWritePolicy {
    pub durable_writes_enabled: bool,
    pub operator_approved: bool,
    pub require_valid_record: bool,
}

impl Default for SessionStateWritePolicy {
    fn default() -> Self {
        Self {
            durable_writes_enabled: false,
            operator_approved: false,
            require_valid_record: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStateWriteReport {
    pub isolation: TenantIsolationReport,
    pub key: String,
    pub key_digest: String,
    pub applied: bool,
    pub durable_write_requested: bool,
    pub blocked_reasons: Vec<String>,
    pub read_only: bool,
    pub report_only: bool,
    pub preview_only: bool,
}

impl SessionStateWriteReport {
    pub fn summary_line(&self) -> String {
        format!(
            "session_state_write applied={} key_digest={} durable_write_requested={} read_only={} report_only={} preview_only={} blocked_reasons={}",
            self.applied,
            self.key_digest,
            self.durable_write_requested,
            self.read_only,
            self.report_only,
            self.preview_only,
            self.blocked_reasons.len()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStateReadReport {
    pub isolation: Option<TenantIsolationReport>,
    pub key_digest: String,
    pub record: Option<SessionStateRecord>,
    pub corrupt: bool,
    pub redacted_error: Option<String>,
    pub error_digest: Option<String>,
    pub raw_payload_exposed: bool,
}

impl SessionStateReadReport {
    pub fn summary_line(&self) -> String {
        format!(
            "session_state_read found={} corrupt={} key_digest={} raw_payload_exposed={} error_digest={}",
            self.record.is_some(),
            self.corrupt,
            self.key_digest,
            self.raw_payload_exposed,
            self.error_digest.as_deref().unwrap_or("none")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStateDecodeError {
    pub redacted_error: String,
    pub error_digest: String,
}

impl SessionStateDecodeError {
    fn new(reason: &str, payload: impl AsRef<[u8]>) -> Self {
        Self {
            redacted_error: sanitize_identifier(reason, "session_state_decode_error"),
            error_digest: stable_digest_bytes(payload.as_ref()),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SessionStateStore {
    pub policy: SessionStateWritePolicy,
}

impl SessionStateStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: SessionStateWritePolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn write(
        &self,
        store: &mut DiskKvStore,
        actor_scope: &TenantScope,
        record: &SessionStateRecord,
    ) -> io::Result<SessionStateWriteReport> {
        let key = record.scoped_key();
        let isolation =
            TenantIsolationGate::new().check_key_access(actor_scope, &key, TenantAccessKind::Write);
        let mut blocked_reasons = Vec::new();
        if !self.policy.durable_writes_enabled {
            blocked_reasons.push("session_state_durable_writes_disabled".to_owned());
        }
        if !self.policy.operator_approved {
            blocked_reasons.push("session_state_operator_approval_missing".to_owned());
        }
        if !isolation.allowed {
            blocked_reasons.push("session_state_cross_tenant_write_rejected".to_owned());
        }
        if self.policy.require_valid_record {
            blocked_reasons.extend(record.validation_blockers());
        }
        blocked_reasons.sort();
        blocked_reasons.dedup();

        let applied = if blocked_reasons.is_empty() {
            tenant_scoped_put(store, actor_scope, &key, record.to_bytes())?.applied
        } else {
            false
        };

        Ok(SessionStateWriteReport {
            isolation,
            key: key.as_str().to_owned(),
            key_digest: key.key_digest(),
            applied,
            durable_write_requested: self.policy.durable_writes_enabled,
            blocked_reasons,
            read_only: true,
            report_only: true,
            preview_only: !applied,
        })
    }

    pub fn read(
        &self,
        store: &DiskKvStore,
        actor_scope: &TenantScope,
        scoped_key: &str,
    ) -> io::Result<SessionStateReadReport> {
        let key_digest = stable_digest(scoped_key);
        let read = tenant_scoped_get(store, actor_scope, scoped_key)?;
        if !read.isolation.allowed {
            return Ok(SessionStateReadReport {
                isolation: Some(read.isolation),
                key_digest,
                record: None,
                corrupt: false,
                redacted_error: None,
                error_digest: None,
                raw_payload_exposed: false,
            });
        }

        let Some(bytes) = read.value else {
            return Ok(SessionStateReadReport {
                isolation: Some(read.isolation),
                key_digest,
                record: None,
                corrupt: false,
                redacted_error: None,
                error_digest: None,
                raw_payload_exposed: false,
            });
        };

        match SessionStateRecord::from_bytes(&bytes) {
            Ok(record) => Ok(SessionStateReadReport {
                isolation: Some(read.isolation),
                key_digest,
                record: Some(record),
                corrupt: false,
                redacted_error: None,
                error_digest: None,
                raw_payload_exposed: false,
            }),
            Err(error) => Ok(SessionStateReadReport {
                isolation: Some(read.isolation),
                key_digest,
                record: None,
                corrupt: true,
                redacted_error: Some(error.redacted_error),
                error_digest: Some(error.error_digest),
                raw_payload_exposed: false,
            }),
        }
    }
}

impl From<TenantScopedKvWriteReport> for SessionStateWriteReport {
    fn from(value: TenantScopedKvWriteReport) -> Self {
        Self {
            key: String::new(),
            key_digest: value.isolation.audit_event.key_digest.clone(),
            applied: value.applied,
            durable_write_requested: value.applied,
            blocked_reasons: Vec::new(),
            read_only: true,
            report_only: true,
            preview_only: !value.applied,
            isolation: value.isolation,
        }
    }
}

fn replay_inputs<'a>(
    anchors: impl Iterator<Item = &'a SessionStateAnchor>,
    limit: usize,
) -> Vec<String> {
    anchors
        .take(limit)
        .map(|anchor| {
            format!(
                "{}:{}:{}",
                anchor.kind.as_str(),
                anchor.anchor_id,
                anchor.evidence_digest
            )
        })
        .collect()
}

fn dedupe_anchors(anchors: Vec<SessionStateAnchor>) -> Vec<SessionStateAnchor> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for anchor in anchors {
        if seen.insert(anchor.summary()) {
            out.push(anchor);
        }
    }
    out
}

fn dedupe_turns(turns: Vec<SessionTurnDigest>) -> Vec<SessionTurnDigest> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for turn in turns {
        if seen.insert(turn.summary()) {
            out.push(turn);
        }
    }
    out
}

fn parse_key_value_lines(
    text: &str,
) -> Result<std::collections::BTreeMap<String, String>, SessionStateDecodeError> {
    let mut map = std::collections::BTreeMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            return Err(SessionStateDecodeError::new(
                "session_state_malformed_line",
                text.as_bytes(),
            ));
        };
        if key.trim().is_empty() {
            return Err(SessionStateDecodeError::new(
                "session_state_empty_key",
                text.as_bytes(),
            ));
        }
        map.insert(key.to_owned(), value.to_owned());
    }
    Ok(map)
}

fn require_value<'a>(
    map: &'a std::collections::BTreeMap<String, String>,
    key: &str,
    payload: impl AsRef<[u8]>,
) -> Result<&'a str, SessionStateDecodeError> {
    map.get(key)
        .map(String::as_str)
        .ok_or_else(|| SessionStateDecodeError::new("session_state_missing_field", payload))
}

fn parse_anchors(
    value: &str,
    payload: impl AsRef<[u8]>,
) -> Result<Vec<SessionStateAnchor>, SessionStateDecodeError> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .map(|item| {
            let parts = item.split('~').collect::<Vec<_>>();
            if parts.len() != 3 {
                return Err(SessionStateDecodeError::new(
                    "session_state_bad_anchor",
                    payload.as_ref(),
                ));
            }
            let kind = SessionAnchorKind::from_str(parts[0]).ok_or_else(|| {
                SessionStateDecodeError::new("session_state_bad_anchor_kind", payload.as_ref())
            })?;
            Ok(SessionStateAnchor {
                kind,
                anchor_id: sanitize_identifier(parts[1], kind.as_str()),
                evidence_digest: digest_or_hash(parts[2]),
            })
        })
        .collect()
}

fn parse_turns(
    value: &str,
    payload: impl AsRef<[u8]>,
) -> Result<Vec<SessionTurnDigest>, SessionStateDecodeError> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .map(|item| {
            let parts = item.split('~').collect::<Vec<_>>();
            if parts.len() != 5 {
                return Err(SessionStateDecodeError::new(
                    "session_state_bad_turn",
                    payload.as_ref(),
                ));
            }
            let role = SessionTurnRole::from_str(parts[1]).ok_or_else(|| {
                SessionStateDecodeError::new("session_state_bad_turn_role", payload.as_ref())
            })?;
            Ok(SessionTurnDigest {
                turn_id: sanitize_identifier(parts[0], "turn"),
                role,
                payload_digest: digest_or_hash(parts[2]),
                token_estimate: parts[3].parse::<usize>().unwrap_or(0),
                summary_preview: sanitize_public_text(parts[4], 96),
            })
        })
        .collect()
}

fn anchors_wire(anchors: &[SessionStateAnchor]) -> String {
    anchors
        .iter()
        .map(|anchor| {
            format!(
                "{}~{}~{}",
                anchor.kind.as_str(),
                anchor.anchor_id,
                anchor.evidence_digest
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn turns_wire(turns: &[SessionTurnDigest]) -> String {
    turns
        .iter()
        .map(|turn| {
            format!(
                "{}~{}~{}~{}~{}",
                turn.turn_id,
                turn.role.as_str(),
                turn.payload_digest,
                turn.token_estimate,
                sanitize_identifier(&turn.summary_preview, "summary")
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_id_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter(|value| !value.trim().is_empty())
        .map(|value| sanitize_identifier(value, "id"))
        .collect()
}

fn parse_usize(
    map: &std::collections::BTreeMap<String, String>,
    key: &str,
    payload: impl AsRef<[u8]>,
) -> Result<usize, SessionStateDecodeError> {
    require_value(map, key, payload.as_ref())?
        .parse::<usize>()
        .map_err(|_| SessionStateDecodeError::new("session_state_bad_usize", payload.as_ref()))
}

fn parse_bool(
    map: &std::collections::BTreeMap<String, String>,
    key: &str,
    payload: impl AsRef<[u8]>,
) -> Result<bool, SessionStateDecodeError> {
    require_value(map, key, payload.as_ref())?
        .parse::<bool>()
        .map_err(|_| SessionStateDecodeError::new("session_state_bad_bool", payload.as_ref()))
}

fn parse_optional_digest(value: &str) -> Option<String> {
    (value != "none").then(|| digest_or_hash(value))
}

fn sanitize_id_list(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        push_unique_string(&mut out, sanitize_identifier(&value, "id"));
    }
    out
}

fn push_unique_string(values: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    if !value.trim().is_empty() && !values.contains(&value) {
        values.push(value);
    }
}

fn parse_profile(value: &str) -> Option<TaskProfile> {
    match value {
        "general" => Some(TaskProfile::General),
        "coding" => Some(TaskProfile::Coding),
        "writing" => Some(TaskProfile::Writing),
        "long_document" => Some(TaskProfile::LongDocument),
        _ => None,
    }
}

fn profile_slug(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn digest_or_hash(value: &str) -> String {
    if value.starts_with("fnv64:") {
        value.to_owned()
    } else {
        stable_digest(value)
    }
}

fn estimate_tokens(value: &str) -> usize {
    value.split_whitespace().count().max(1)
}

fn sanitize_identifier(value: &str, fallback: &str) -> String {
    if contains_sensitive_payload(value) {
        return format!("{fallback}:{}", stable_digest(value));
    }
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, ':' | '-' | '_' | '.') {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .take(128)
        .collect::<String>();
    let sanitized = sanitized.trim_matches('_').to_owned();
    if sanitized.is_empty() {
        fallback.to_owned()
    } else {
        sanitized
    }
}

fn sanitize_public_text(value: &str, max_chars: usize) -> String {
    let mut out = Vec::new();
    for word in value.split_whitespace() {
        if contains_sensitive_payload(word) {
            out.push("[redacted]");
        } else {
            out.push(word);
        }
    }
    let sanitized = out.join(" ");
    let mut preview = sanitized.chars().take(max_chars).collect::<String>();
    if sanitized.chars().count() > max_chars {
        preview.push_str("...");
    }
    preview
}

fn contains_sensitive_payload(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "secret",
        "password",
        "passwd",
        "token=",
        "private:",
        "private_key",
        "begin private key",
        "sk-",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn stable_digest(value: &str) -> String {
    stable_digest_bytes(value.as_bytes())
}

fn stable_digest_bytes(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn session_record_keeps_scope_profile_anchors_and_redacted_turn_digests() {
        let secret = "private: password=correct-horse";
        let record = sample_record("session-state-redact")
            .with_turn(SessionTurnDigest::from_payload(
                "turn-secret",
                SessionTurnRole::User,
                secret,
            ))
            .into_record();
        let wire = String::from_utf8(record.to_bytes()).unwrap();
        let summary = record.summary_line();

        assert_eq!(record.scope.tenant_id, "tenanta");
        assert_eq!(record.runtime_profile.task_profile, TaskProfile::Coding);
        assert_eq!(record.memory_anchors.len(), 1);
        assert_eq!(record.gene_anchors.len(), 1);
        assert!(!record.raw_messages_stored);
        assert!(!record.write_allowed);
        assert!(!wire.contains(secret));
        assert!(!summary.contains("tenanta"));
        assert!(wire.contains("payload_digest:fnv64:"));
        assert!(summary.contains("state_digest=fnv64:"));
    }

    #[test]
    fn replay_preview_explains_retrieval_and_routing_without_raw_payloads() {
        let scope = TenantScope::new("tenant-a", "workspace", "chat-1");
        let record = sample_record_with_scope("session-state-replay", scope.clone()).into_record();
        let preview = SessionReplayPlanner::new().preview(&scope, &record, 8);

        assert!(preview.allowed, "{:?}", preview.blocked_reasons);
        assert!(!preview.raw_payload_exposed);
        assert!(!preview.retrieval_inputs.is_empty());
        assert!(!preview.routing_inputs.is_empty());
        assert!(!preview.gene_inputs.is_empty());
        assert!(
            preview
                .retrieval_inputs
                .iter()
                .all(|input| input.contains("fnv64:"))
        );
        assert!(preview.summary_line().contains("session_replay_preview"));
        assert!(!preview.summary_line().contains("user:"));
    }

    #[test]
    fn default_store_write_is_preview_only_until_policy_allows_durable_write() {
        let path = temp_path("session-default-write");
        let mut store = DiskKvStore::open(&path).unwrap();
        let scope = TenantScope::new("tenant-a", "workspace", "chat-1");
        let record = sample_record_with_scope("session-state-preview", scope.clone()).into_record();
        let report = SessionStateStore::new()
            .write(&mut store, &scope, &record)
            .unwrap();

        assert!(!report.applied);
        assert!(report.preview_only);
        assert!(
            report
                .blocked_reasons
                .contains(&"session_state_durable_writes_disabled".to_owned())
        );
        assert!(!store.contains_key(record.scoped_key().as_str()));
        cleanup(path);
    }

    #[test]
    fn tenant_isolation_blocks_cross_session_and_cross_tenant_reads() {
        let path = temp_path("session-tenant-isolation");
        let mut store = DiskKvStore::open(&path).unwrap();
        let tenant_a = TenantScope::new("tenant-a", "workspace", "chat-1");
        let tenant_b = TenantScope::new("tenant-b", "workspace", "chat-1");
        let other_session = TenantScope::new("tenant-a", "workspace", "chat-2");
        let record =
            sample_record_with_scope("session-state-tenant", tenant_a.clone()).into_record();
        let writer = SessionStateStore::new().with_policy(SessionStateWritePolicy {
            durable_writes_enabled: true,
            operator_approved: true,
            require_valid_record: true,
        });
        let write = writer.write(&mut store, &tenant_a, &record).unwrap();
        let key = write.key.clone();
        let read_a = writer.read(&store, &tenant_a, &key).unwrap();
        let read_b = writer.read(&store, &tenant_b, &key).unwrap();
        let read_other_session = writer.read(&store, &other_session, &key).unwrap();

        assert!(write.applied);
        assert!(read_a.record.is_some());
        assert!(
            read_a
                .isolation
                .as_ref()
                .is_some_and(|report| report.allowed)
        );
        assert!(read_b.record.is_none());
        assert!(
            read_b
                .isolation
                .as_ref()
                .is_some_and(|report| !report.allowed)
        );
        assert!(read_other_session.record.is_none());
        assert!(
            read_other_session
                .isolation
                .as_ref()
                .is_some_and(|report| !report.allowed)
        );
        assert!(!read_b.summary_line().contains("tenant-a"));
        cleanup(path);
    }

    #[test]
    fn corrupt_session_state_fails_closed_with_redacted_error() {
        let path = temp_path("session-corrupt");
        let mut store = DiskKvStore::open(&path).unwrap();
        let scope = TenantScope::new("tenant-a", "workspace", "chat-1");
        let key = scope.scoped_key(TenantResourceLane::SessionState, "corrupt-state");
        tenant_scoped_put(
            &mut store,
            &scope,
            &key,
            b"schema=wrong\nraw_secret=private: password=do-not-leak",
        )
        .unwrap();

        let read = SessionStateStore::new()
            .read(&store, &scope, key.as_str())
            .unwrap();
        let summary = read.summary_line();

        assert!(read.corrupt);
        assert!(read.record.is_none());
        assert!(read.redacted_error.is_some());
        assert!(read.error_digest.is_some());
        assert!(!summary.contains("do-not-leak"));
        assert!(!summary.contains("password"));
        cleanup(path);
    }

    trait IntoRecord {
        fn into_record(self) -> SessionStateRecord;
    }

    impl IntoRecord for SessionStateInput {
        fn into_record(self) -> SessionStateRecord {
            SessionStateRecord::from_input(self)
        }
    }

    fn sample_record(state_id: &str) -> SessionStateInput {
        sample_record_with_scope(
            state_id,
            TenantScope::new("Tenant A", "Workspace", "Chat 1"),
        )
    }

    fn sample_record_with_scope(state_id: &str, scope: TenantScope) -> SessionStateInput {
        SessionStateInput::new(
            state_id,
            scope,
            SessionRuntimeProfile::new(TaskProfile::Coding)
                .with_model("noiron-dev-transformer", "noiron-bpe")
                .with_native_context_window(4096)
                .with_max_tokens(512)
                .with_streaming(true)
                .with_cancellation_anchor("cancel:chat-1"),
        )
        .with_memory_anchor(SessionStateAnchor::new(
            SessionAnchorKind::MemoryChain,
            "memory:active",
            "retrieved memory digest",
        ))
        .with_gene_anchor(SessionStateAnchor::new(
            SessionAnchorKind::GeneChain,
            "gene:rust-coding",
            "active gene digest",
        ))
        .with_retrieval_evidence(SessionStateAnchor::new(
            SessionAnchorKind::Retrieval,
            "retrieval:semantic-index",
            "semantic retrieval inputs",
        ))
        .with_routing_evidence(SessionStateAnchor::new(
            SessionAnchorKind::Routing,
            "routing:fht-dke",
            "route budget and threshold digest",
        ))
        .with_turn(SessionTurnDigest::from_payload(
            "turn-1",
            SessionTurnRole::User,
            "user: build Rust replay API with session state",
        ))
        .with_source_trace_id("trace:session-state")
    }

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-norion-{label}-{}-{nanos}.ndkv",
            std::process::id()
        ))
    }

    fn cleanup(path: PathBuf) {
        let _ = std::fs::remove_file(path);
    }
}
