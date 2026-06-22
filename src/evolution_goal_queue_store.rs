use std::io;
use std::path::Path;

use crate::disk_kv::DiskKvStore;
use crate::evolution_goal::EvolutionGoalQueue;
use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};
use crate::self_goal_proposal::SelfGoalQueueAppendExecutionReport;
use crate::tenant_scope::{
    TenantAccessKind, TenantIsolationGate, TenantIsolationReport, TenantResourceLane, TenantScope,
    TenantScopedKey, tenant_scoped_get, tenant_scoped_put,
};

pub const EVOLUTION_GOAL_QUEUE_STORE_SCHEMA_VERSION: &str = "evolution_goal_queue_store_v1";
pub const EVOLUTION_GOAL_QUEUE_STORE_APPROVAL_SCHEMA_VERSION: &str =
    "evolution_goal_queue_store_approval_v1";
pub const EVOLUTION_GOAL_QUEUE_STORE_WRITE_TRACE_SCHEMA: &str =
    "rust-norion-evolution-goal-queue-store-write-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvolutionGoalQueueStorePolicy {
    pub require_tenant_isolation: bool,
    pub require_operator_approval: bool,
    pub require_rollback_anchor: bool,
    pub require_preview_queue: bool,
    pub require_digest_only_evidence: bool,
    pub allow_durable_write: bool,
}

impl Default for EvolutionGoalQueueStorePolicy {
    fn default() -> Self {
        Self {
            require_tenant_isolation: true,
            require_operator_approval: true,
            require_rollback_anchor: true,
            require_preview_queue: true,
            require_digest_only_evidence: true,
            allow_durable_write: false,
        }
    }
}

impl EvolutionGoalQueueStorePolicy {
    pub fn explicit_durable_write() -> Self {
        Self {
            allow_durable_write: true,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionGoalQueueStoreApproval {
    pub schema_version: &'static str,
    pub operator_id: String,
    pub approval_ticket_id: String,
    pub approved_key_digest: String,
    pub approved_queue_digest: String,
    pub approved_rollback_anchor_digest: String,
    pub approval_attestation_digest: String,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl EvolutionGoalQueueStoreApproval {
    pub fn for_queue(
        operator_id: impl Into<String>,
        approval_ticket_id: impl Into<String>,
        key: &TenantScopedKey,
        queue: &EvolutionGoalQueue,
        rollback_anchor_digest: impl AsRef<str>,
    ) -> Self {
        let operator_id = safe_text(operator_id.into());
        let approval_ticket_id = safe_text(approval_ticket_id.into());
        let approved_key_digest = key.key_digest();
        let approved_queue_digest = queue.redaction_digest();
        let approved_rollback_anchor_digest = require_digest_or_hash(rollback_anchor_digest);
        let approval_attestation_digest = approval_digest(
            &operator_id,
            &approval_ticket_id,
            &approved_key_digest,
            &approved_queue_digest,
            &approved_rollback_anchor_digest,
        );

        Self {
            schema_version: EVOLUTION_GOAL_QUEUE_STORE_APPROVAL_SCHEMA_VERSION,
            operator_id,
            approval_ticket_id,
            approved_key_digest,
            approved_queue_digest,
            approved_rollback_anchor_digest,
            approval_attestation_digest,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    fn expected_attestation_digest(&self) -> String {
        approval_digest(
            &self.operator_id,
            &self.approval_ticket_id,
            &self.approved_key_digest,
            &self.approved_queue_digest,
            &self.approved_rollback_anchor_digest,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvolutionGoalQueueStoreWriteDecision {
    Applied,
    Hold,
    Rejected,
}

impl EvolutionGoalQueueStoreWriteDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Hold => "hold",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionGoalQueueStoreWriteReport {
    pub schema_version: &'static str,
    pub decision: EvolutionGoalQueueStoreWriteDecision,
    pub policy: EvolutionGoalQueueStorePolicy,
    pub isolation: TenantIsolationReport,
    pub reason_codes: Vec<String>,
    pub key_digest: String,
    pub queue_digest: String,
    pub rollback_anchor_digest: String,
    pub approval_attestation_digest: Option<String>,
    pub durable_write_allowed: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl EvolutionGoalQueueStoreWriteReport {
    pub fn passed(&self) -> bool {
        self.decision == EvolutionGoalQueueStoreWriteDecision::Applied
            && self.applied
            && self.write_allowed
            && !self.read_only
            && self.durable_write_allowed
            && self.reason_codes.is_empty()
            && self.evidence_is_redacted()
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.key_digest.starts_with("fnv64:")
            && self.queue_digest.starts_with("redaction-digest:")
            && self.rollback_anchor_digest.starts_with("redaction-digest:")
            && self
                .approval_attestation_digest
                .as_ref()
                .is_none_or(|digest| digest.starts_with("redaction-digest:"))
            && self
                .reason_codes
                .iter()
                .all(|reason| !contains_private_or_executable_marker(reason))
            && self.isolation.audit_event.redacted
    }

    pub fn summary_line(&self) -> String {
        format!(
            "evolution_goal_queue_store_write schema={} decision={} passed={} reasons={} key={} queue={} rollback_anchor={} durable_write_allowed={} read_only={} write_allowed={} applied={} isolation={}",
            self.schema_version,
            self.decision.as_str(),
            self.passed(),
            self.reason_codes.len(),
            self.key_digest,
            self.queue_digest,
            self.rollback_anchor_digest,
            self.durable_write_allowed,
            self.read_only,
            self.write_allowed,
            self.applied,
            self.isolation.audit_event.decision.as_str(),
        )
    }

    pub fn json_line(&self) -> String {
        let approval_digest = self
            .approval_attestation_digest
            .as_deref()
            .unwrap_or("none");
        format!(
            "{{\"schema\":\"{}\",\"store_schema\":\"{}\",\"decision\":\"{}\",\"reason_code_count\":{},\"key_digest\":\"{}\",\"queue_digest\":\"{}\",\"rollback_anchor_digest\":\"{}\",\"approval_attestation_digest\":\"{}\",\"tenant_isolation_allowed\":{},\"isolation_decision\":\"{}\",\"durable_write_allowed\":{},\"read_only\":{},\"write_allowed\":{},\"applied\":{},\"summary\":\"{}\"}}",
            json_escape(EVOLUTION_GOAL_QUEUE_STORE_WRITE_TRACE_SCHEMA),
            json_escape(self.schema_version),
            json_escape(self.decision.as_str()),
            self.reason_codes.len(),
            json_escape(&self.key_digest),
            json_escape(&self.queue_digest),
            json_escape(&self.rollback_anchor_digest),
            json_escape(approval_digest),
            self.isolation.allowed,
            json_escape(self.isolation.audit_event.decision.as_str()),
            self.durable_write_allowed,
            self.read_only,
            self.write_allowed,
            self.applied,
            json_escape(&self.summary_line())
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionGoalQueueStoreReadReport {
    pub schema_version: &'static str,
    pub isolation: TenantIsolationReport,
    pub key_digest: String,
    pub found: bool,
    pub decoded: bool,
    pub queue_digest: Option<String>,
    pub decode_error_digest: Option<String>,
    pub queue: Option<EvolutionGoalQueue>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl EvolutionGoalQueueStoreReadReport {
    pub fn summary_line(&self) -> String {
        format!(
            "evolution_goal_queue_store_read schema={} found={} decoded={} key={} queue={} decode_error={} read_only={} write_allowed={} applied={} isolation={}",
            self.schema_version,
            self.found,
            self.decoded,
            self.key_digest,
            self.queue_digest.as_deref().unwrap_or("none"),
            self.decode_error_digest.as_deref().unwrap_or("none"),
            self.read_only,
            self.write_allowed,
            self.applied,
            self.isolation.audit_event.decision.as_str(),
        )
    }
}

#[derive(Debug)]
pub struct EvolutionGoalQueueDiskStore {
    store: DiskKvStore,
    pub policy: EvolutionGoalQueueStorePolicy,
}

impl EvolutionGoalQueueDiskStore {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::open_with_policy(path, EvolutionGoalQueueStorePolicy::default())
    }

    pub fn open_with_policy(
        path: impl AsRef<Path>,
        policy: EvolutionGoalQueueStorePolicy,
    ) -> io::Result<Self> {
        Ok(Self {
            store: DiskKvStore::open(path)?,
            policy,
        })
    }

    pub fn write_queue(
        &mut self,
        actor_scope: &TenantScope,
        key: &TenantScopedKey,
        queue: &EvolutionGoalQueue,
        rollback_anchor_digest: impl AsRef<str>,
        approval: Option<&EvolutionGoalQueueStoreApproval>,
    ) -> io::Result<EvolutionGoalQueueStoreWriteReport> {
        let rollback_anchor_raw = rollback_anchor_digest.as_ref().trim().to_owned();
        let rollback_anchor_digest = require_digest_or_hash(rollback_anchor_digest);
        let key_digest = key.key_digest();
        let queue_digest = queue.redaction_digest();
        let isolation =
            TenantIsolationGate::new().check_key_access(actor_scope, key, TenantAccessKind::Write);
        let mut reason_codes = Vec::new();

        if key.lane != TenantResourceLane::EvolutionGoalQueue {
            reason_codes.push("queue_store_wrong_lane".to_owned());
        }
        if self.policy.require_tenant_isolation && !isolation.allowed {
            reason_codes.push("queue_store_tenant_isolation_rejected".to_owned());
        }
        if !self.policy.allow_durable_write {
            reason_codes.push("queue_store_durable_write_disabled".to_owned());
        }
        if self.policy.require_preview_queue
            && (!queue.read_only || queue.write_allowed || queue.applied)
        {
            reason_codes.push("queue_store_queue_not_preview_only".to_owned());
        }
        if self.policy.require_rollback_anchor
            && !rollback_anchor_raw.starts_with("redaction-digest:")
        {
            reason_codes.push("queue_store_rollback_anchor_not_redacted".to_owned());
        }
        if self.policy.require_digest_only_evidence
            && (!queue_digest.starts_with("redaction-digest:")
                || contains_private_or_executable_marker(&queue_digest)
                || contains_private_or_executable_marker(&rollback_anchor_digest)
                || queue
                    .goals
                    .iter()
                    .any(|goal| contains_private_or_executable_marker(&goal.objective)))
        {
            reason_codes.push("queue_store_evidence_not_redacted".to_owned());
        }

        if self.policy.require_operator_approval {
            match approval {
                Some(approval) => push_approval_reasons(
                    &mut reason_codes,
                    approval,
                    &key_digest,
                    &queue_digest,
                    &rollback_anchor_digest,
                ),
                None => reason_codes.push("queue_store_operator_approval_missing".to_owned()),
            }
        }

        let rejected = reason_codes
            .iter()
            .any(|reason| queue_store_rejection_reason(reason));
        let mut applied = false;
        if reason_codes.is_empty() {
            let write = tenant_scoped_put(
                &mut self.store,
                actor_scope,
                key,
                queue.to_record_text().as_bytes(),
            )?;
            applied = write.applied;
            if !write.applied {
                reason_codes.push("queue_store_tenant_write_rejected".to_owned());
            }
        }

        let decision = if applied {
            EvolutionGoalQueueStoreWriteDecision::Applied
        } else if rejected {
            EvolutionGoalQueueStoreWriteDecision::Rejected
        } else {
            EvolutionGoalQueueStoreWriteDecision::Hold
        };
        let approval_attestation_digest =
            approval.map(|approval| approval.approval_attestation_digest.clone());

        Ok(EvolutionGoalQueueStoreWriteReport {
            schema_version: EVOLUTION_GOAL_QUEUE_STORE_SCHEMA_VERSION,
            decision,
            policy: self.policy,
            isolation,
            reason_codes,
            key_digest,
            queue_digest,
            rollback_anchor_digest,
            approval_attestation_digest,
            durable_write_allowed: applied,
            read_only: !applied,
            write_allowed: applied,
            applied,
        })
    }

    pub fn write_append_execution_result(
        &mut self,
        actor_scope: &TenantScope,
        key: &TenantScopedKey,
        append_report: &SelfGoalQueueAppendExecutionReport,
        approval: Option<&EvolutionGoalQueueStoreApproval>,
    ) -> io::Result<EvolutionGoalQueueStoreWriteReport> {
        let key_digest = key.key_digest();
        let isolation =
            TenantIsolationGate::new().check_key_access(actor_scope, key, TenantAccessKind::Write);
        let queue_digest = append_report
            .resulting_queue_digest
            .clone()
            .unwrap_or_else(|| stable_redaction_digest(["queue-store-missing-resulting-digest"]));
        let approval_attestation_digest =
            approval.map(|approval| approval.approval_attestation_digest.clone());
        let mut reason_codes = Vec::new();

        if !append_report.passed() {
            reason_codes.push("queue_store_append_execution_not_passed".to_owned());
        }
        if append_report.durable_write_allowed {
            reason_codes.push("queue_store_append_execution_claimed_durable_write".to_owned());
        }
        let Some(resulting_queue) = append_report.resulting_queue.as_ref() else {
            reason_codes.push("queue_store_append_execution_resulting_queue_missing".to_owned());
            return Ok(EvolutionGoalQueueStoreWriteReport {
                schema_version: EVOLUTION_GOAL_QUEUE_STORE_SCHEMA_VERSION,
                decision: EvolutionGoalQueueStoreWriteDecision::Rejected,
                policy: self.policy,
                isolation,
                reason_codes,
                key_digest,
                queue_digest,
                rollback_anchor_digest: append_report.rollback_anchor_digest.clone(),
                approval_attestation_digest,
                durable_write_allowed: false,
                read_only: true,
                write_allowed: false,
                applied: false,
            });
        };

        let computed_queue_digest = resulting_queue.redaction_digest();
        if queue_digest != computed_queue_digest {
            reason_codes.push("queue_store_append_execution_resulting_digest_mismatch".to_owned());
        }

        if !reason_codes.is_empty() {
            let rejected = reason_codes
                .iter()
                .any(|reason| queue_store_rejection_reason(reason));
            return Ok(EvolutionGoalQueueStoreWriteReport {
                schema_version: EVOLUTION_GOAL_QUEUE_STORE_SCHEMA_VERSION,
                decision: if rejected {
                    EvolutionGoalQueueStoreWriteDecision::Rejected
                } else {
                    EvolutionGoalQueueStoreWriteDecision::Hold
                },
                policy: self.policy,
                isolation,
                reason_codes,
                key_digest,
                queue_digest: computed_queue_digest,
                rollback_anchor_digest: append_report.rollback_anchor_digest.clone(),
                approval_attestation_digest,
                durable_write_allowed: false,
                read_only: true,
                write_allowed: false,
                applied: false,
            });
        }

        self.write_queue(
            actor_scope,
            key,
            resulting_queue,
            &append_report.rollback_anchor_digest,
            approval,
        )
    }

    pub fn read_queue(
        &self,
        actor_scope: &TenantScope,
        scoped_key: &str,
    ) -> io::Result<EvolutionGoalQueueStoreReadReport> {
        let key_digest = TenantScopedKey::parse(scoped_key)
            .map(|key| key.key_digest())
            .unwrap_or_else(|| {
                stable_redaction_digest(["malformed-evolution-goal-queue-key", scoped_key])
            });
        let read = tenant_scoped_get(&self.store, actor_scope, scoped_key)?;
        let found = read.value.is_some();
        let mut decoded = false;
        let mut queue_digest = None;
        let mut decode_error_digest = None;
        let mut queue = None;

        if let Some(value) = read.value {
            match String::from_utf8(value) {
                Ok(text) => match EvolutionGoalQueue::from_record_text(&text) {
                    Ok(parsed_queue) => {
                        decoded = true;
                        queue_digest = Some(parsed_queue.redaction_digest());
                        queue = Some(parsed_queue);
                    }
                    Err(error) => {
                        decode_error_digest = Some(error.error_digest);
                    }
                },
                Err(error) => {
                    decode_error_digest = Some(decode_error_digest_from_utf8(error));
                }
            }
        }

        Ok(EvolutionGoalQueueStoreReadReport {
            schema_version: EVOLUTION_GOAL_QUEUE_STORE_SCHEMA_VERSION,
            isolation: read.isolation,
            key_digest,
            found,
            decoded,
            queue_digest,
            decode_error_digest,
            queue,
            read_only: true,
            write_allowed: false,
            applied: false,
        })
    }
}

fn push_approval_reasons(
    reasons: &mut Vec<String>,
    approval: &EvolutionGoalQueueStoreApproval,
    expected_key_digest: &str,
    expected_queue_digest: &str,
    expected_rollback_anchor_digest: &str,
) {
    if approval.schema_version != EVOLUTION_GOAL_QUEUE_STORE_APPROVAL_SCHEMA_VERSION {
        reasons.push("queue_store_approval_schema_mismatch".to_owned());
    }
    if approval.operator_id.trim().is_empty() {
        reasons.push("queue_store_approval_operator_id_empty".to_owned());
    }
    if approval.approval_ticket_id.trim().is_empty() {
        reasons.push("queue_store_approval_ticket_id_empty".to_owned());
    }
    if !approval.read_only || approval.write_allowed || approval.applied {
        reasons.push("queue_store_approval_not_preview_only".to_owned());
    }
    if contains_private_or_executable_marker(&approval.operator_id)
        || contains_private_or_executable_marker(&approval.approval_ticket_id)
        || contains_private_or_executable_marker(&approval.approval_attestation_digest)
    {
        reasons.push("queue_store_approval_private_marker".to_owned());
    }
    if approval.approved_key_digest != expected_key_digest {
        reasons.push("queue_store_approval_key_digest_mismatch".to_owned());
    }
    if approval.approved_queue_digest != expected_queue_digest {
        reasons.push("queue_store_approval_queue_digest_mismatch".to_owned());
    }
    if approval.approved_rollback_anchor_digest != expected_rollback_anchor_digest {
        reasons.push("queue_store_approval_rollback_anchor_mismatch".to_owned());
    }
    if approval.approval_attestation_digest != approval.expected_attestation_digest()
        || !approval
            .approval_attestation_digest
            .starts_with("redaction-digest:")
    {
        reasons.push("queue_store_approval_attestation_mismatch".to_owned());
    }
}

fn queue_store_rejection_reason(reason: &str) -> bool {
    matches!(
        reason,
        "queue_store_wrong_lane"
            | "queue_store_append_execution_not_passed"
            | "queue_store_append_execution_claimed_durable_write"
            | "queue_store_append_execution_resulting_queue_missing"
            | "queue_store_append_execution_resulting_digest_mismatch"
            | "queue_store_tenant_isolation_rejected"
            | "queue_store_queue_not_preview_only"
            | "queue_store_rollback_anchor_not_redacted"
            | "queue_store_evidence_not_redacted"
            | "queue_store_approval_schema_mismatch"
            | "queue_store_approval_operator_id_empty"
            | "queue_store_approval_ticket_id_empty"
            | "queue_store_approval_not_preview_only"
            | "queue_store_approval_private_marker"
            | "queue_store_approval_key_digest_mismatch"
            | "queue_store_approval_queue_digest_mismatch"
            | "queue_store_approval_rollback_anchor_mismatch"
            | "queue_store_approval_attestation_mismatch"
    )
}

fn approval_digest(
    operator_id: &str,
    approval_ticket_id: &str,
    key_digest: &str,
    queue_digest: &str,
    rollback_anchor_digest: &str,
) -> String {
    stable_redaction_digest([
        EVOLUTION_GOAL_QUEUE_STORE_APPROVAL_SCHEMA_VERSION,
        operator_id,
        approval_ticket_id,
        key_digest,
        queue_digest,
        rollback_anchor_digest,
    ])
}

fn require_digest_or_hash(value: impl AsRef<str>) -> String {
    let value = value.as_ref().trim();
    if value.starts_with("redaction-digest:") || value.starts_with("fnv64:") {
        value.to_owned()
    } else {
        stable_redaction_digest(["evolution-goal-queue-store-anchor", value])
    }
}

fn decode_error_digest_from_utf8(error: std::string::FromUtf8Error) -> String {
    stable_redaction_digest([
        "evolution-goal-queue-store-utf8-decode",
        &error.utf8_error().valid_up_to().to_string(),
    ])
}

fn safe_text(value: String) -> String {
    if contains_private_or_executable_marker(&value) {
        stable_redaction_digest(["redacted-evolution-goal-queue-store-text", value.trim()])
    } else {
        value.trim().to_owned()
    }
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push(' '),
            ch => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evolution_goal::default_noiron_pursuit_goal_queue;

    #[test]
    fn queue_store_default_policy_holds_without_disk_write() {
        let path = temp_path("queue-store-default-hold");
        let mut store = EvolutionGoalQueueDiskStore::open(&path).unwrap();
        let scope = TenantScope::local_single_user();
        let key = scope.scoped_key(TenantResourceLane::EvolutionGoalQueue, "active");
        let queue = default_noiron_pursuit_goal_queue();
        let rollback = queue.redaction_digest();
        let approval = EvolutionGoalQueueStoreApproval::for_queue(
            "operator", "ticket", &key, &queue, &rollback,
        );

        let write = store
            .write_queue(&scope, &key, &queue, &rollback, Some(&approval))
            .unwrap();
        let read = store.read_queue(&scope, key.as_str()).unwrap();

        assert_eq!(write.decision, EvolutionGoalQueueStoreWriteDecision::Hold);
        assert!(!write.applied);
        assert!(!write.durable_write_allowed);
        assert!(
            write
                .reason_codes
                .contains(&"queue_store_durable_write_disabled".to_owned())
        );
        assert!(!read.found);
        assert!(write.summary_line().contains("queue=redaction-digest:"));
        cleanup(path);
    }

    #[test]
    fn queue_store_writes_and_reads_with_explicit_policy_and_approval() {
        let path = temp_path("queue-store-explicit-write");
        let mut store = EvolutionGoalQueueDiskStore::open_with_policy(
            &path,
            EvolutionGoalQueueStorePolicy::explicit_durable_write(),
        )
        .unwrap();
        let scope = TenantScope::new("tenant-a", "workspace", "session");
        let other_scope = TenantScope::new("tenant-b", "workspace", "session");
        let key = scope.scoped_key(TenantResourceLane::EvolutionGoalQueue, "active");
        let queue = default_noiron_pursuit_goal_queue();
        let rollback = queue.redaction_digest();
        let approval = EvolutionGoalQueueStoreApproval::for_queue(
            "operator", "ticket", &key, &queue, &rollback,
        );

        let write = store
            .write_queue(&scope, &key, &queue, &rollback, Some(&approval))
            .unwrap();
        let read_owner = store.read_queue(&scope, key.as_str()).unwrap();
        let read_other = store.read_queue(&other_scope, key.as_str()).unwrap();

        assert!(write.passed());
        assert_eq!(
            write.decision,
            EvolutionGoalQueueStoreWriteDecision::Applied
        );
        assert_eq!(read_owner.queue_digest, Some(queue.redaction_digest()));
        assert_eq!(read_owner.queue, Some(queue));
        assert!(read_owner.decoded);
        assert!(!read_other.isolation.allowed);
        assert!(!read_other.found);
        assert!(!write.summary_line().contains("R97"));
        cleanup(path);
    }

    #[test]
    fn queue_store_rejects_wrong_lane_and_tampered_approval() {
        let path = temp_path("queue-store-rejects");
        let mut store = EvolutionGoalQueueDiskStore::open_with_policy(
            &path,
            EvolutionGoalQueueStorePolicy::explicit_durable_write(),
        )
        .unwrap();
        let scope = TenantScope::local_single_user();
        let wrong_lane_key = scope.scoped_key(TenantResourceLane::KvMemory, "active");
        let queue = default_noiron_pursuit_goal_queue();
        let rollback = queue.redaction_digest();
        let mut approval = EvolutionGoalQueueStoreApproval::for_queue(
            "operator",
            "ticket",
            &wrong_lane_key,
            &queue,
            &rollback,
        );
        approval.approved_queue_digest = "redaction-digest:tampered".to_owned();

        let write = store
            .write_queue(&scope, &wrong_lane_key, &queue, &rollback, Some(&approval))
            .unwrap();

        assert_eq!(
            write.decision,
            EvolutionGoalQueueStoreWriteDecision::Rejected
        );
        assert!(!write.applied);
        assert!(
            write
                .reason_codes
                .contains(&"queue_store_wrong_lane".to_owned())
        );
        assert!(
            write
                .reason_codes
                .contains(&"queue_store_approval_queue_digest_mismatch".to_owned())
        );
        cleanup(path);
    }

    #[test]
    fn queue_store_rejects_raw_rollback_anchor_even_with_explicit_policy() {
        let path = temp_path("queue-store-raw-rollback");
        let mut store = EvolutionGoalQueueDiskStore::open_with_policy(
            &path,
            EvolutionGoalQueueStorePolicy::explicit_durable_write(),
        )
        .unwrap();
        let scope = TenantScope::local_single_user();
        let key = scope.scoped_key(TenantResourceLane::EvolutionGoalQueue, "active");
        let queue = default_noiron_pursuit_goal_queue();
        let approval = EvolutionGoalQueueStoreApproval::for_queue(
            "operator",
            "ticket",
            &key,
            &queue,
            "raw rollback anchor",
        );

        let write = store
            .write_queue(&scope, &key, &queue, "raw rollback anchor", Some(&approval))
            .unwrap();

        assert_eq!(
            write.decision,
            EvolutionGoalQueueStoreWriteDecision::Rejected
        );
        assert!(!write.applied);
        assert!(
            write
                .reason_codes
                .contains(&"queue_store_rollback_anchor_not_redacted".to_owned())
        );
        cleanup(path);
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-norion-{label}-{}-{nanos}.ndkv",
            std::process::id()
        ))
    }

    fn cleanup(path: std::path::PathBuf) {
        let _ = std::fs::remove_file(path);
    }
}
