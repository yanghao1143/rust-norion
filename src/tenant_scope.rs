use std::collections::BTreeSet;
use std::io;

use crate::development_pollution::{
    DefenseSpacer, DefenseSpacerActivationGate, DefenseSpacerCandidate, DevelopmentPollutionEvent,
    classify_development_pollution_event, gate_defense_spacer_activation,
};
use crate::disk_kv::DiskKvStore;
use crate::reasoning_genome::DnaGeneChain;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TenantScope {
    pub tenant_id: String,
    pub workspace_id: String,
    pub session_id: String,
}

impl TenantScope {
    pub fn new(
        tenant_id: impl AsRef<str>,
        workspace_id: impl AsRef<str>,
        session_id: impl AsRef<str>,
    ) -> Self {
        Self {
            tenant_id: sanitize_scope_id(tenant_id.as_ref(), "local"),
            workspace_id: sanitize_scope_id(workspace_id.as_ref(), "default"),
            session_id: sanitize_scope_id(session_id.as_ref(), "interactive"),
        }
    }

    pub fn local_single_user() -> Self {
        Self::new("local", "default", "interactive")
    }

    pub fn lineage_tenant_scope(&self) -> String {
        format!("tenant:{}:workspace:{}", self.tenant_id, self.workspace_id)
    }

    pub fn scope_digest(&self) -> String {
        stable_digest(&format!(
            "{}:{}:{}",
            self.tenant_id, self.workspace_id, self.session_id
        ))
    }

    pub fn scoped_key(
        &self,
        lane: TenantResourceLane,
        local_key: impl AsRef<str>,
    ) -> TenantScopedKey {
        TenantScopedKey::new(self.clone(), lane, local_key.as_ref())
    }

    fn matches_lineage(&self, tenant_scope: &str, session_id: &str) -> bool {
        tenant_scope == self.lineage_tenant_scope() && session_id == self.session_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TenantResourceLane {
    KvMemory,
    ReasoningGenome,
    RuntimeKv,
    SessionState,
    TraceEvidence,
    SelfEvolvingMemory,
    ApprovalPacket,
    EvolutionGoalQueue,
}

impl TenantResourceLane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KvMemory => "kv_memory",
            Self::ReasoningGenome => "reasoning_genome",
            Self::RuntimeKv => "runtime_kv",
            Self::SessionState => "session_state",
            Self::TraceEvidence => "trace_evidence",
            Self::SelfEvolvingMemory => "self_evolving_memory",
            Self::ApprovalPacket => "approval_packet",
            Self::EvolutionGoalQueue => "evolution_goal_queue",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenantAccessKind {
    Read,
    Write,
    Delete,
    Inherit,
    Score,
    RollbackReplay,
}

impl TenantAccessKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Delete => "delete",
            Self::Inherit => "inherit",
            Self::Score => "score",
            Self::RollbackReplay => "rollback_replay",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenantAccessDecision {
    Allowed,
    Rejected,
}

impl TenantAccessDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantScopedKey {
    pub scope: TenantScope,
    pub lane: TenantResourceLane,
    pub local_key: String,
    key: String,
}

impl TenantScopedKey {
    pub fn new(scope: TenantScope, lane: TenantResourceLane, local_key: &str) -> Self {
        let local_key = sanitize_key_fragment(local_key, "record");
        let key = format!(
            "tenant={};workspace={};session={};lane={};key={}",
            scope.tenant_id,
            scope.workspace_id,
            scope.session_id,
            lane.as_str(),
            local_key
        );
        Self {
            scope,
            lane,
            local_key,
            key,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        let mut tenant_id = None;
        let mut workspace_id = None;
        let mut session_id = None;
        let mut lane = None;
        let mut local_key = None;

        for field in value.split(';') {
            let (name, value) = field.split_once('=')?;
            match name {
                "tenant" => tenant_id = Some(value.to_owned()),
                "workspace" => workspace_id = Some(value.to_owned()),
                "session" => session_id = Some(value.to_owned()),
                "lane" => lane = str_to_lane(value),
                "key" => local_key = Some(value.to_owned()),
                _ => return None,
            }
        }

        let scope = TenantScope::new(tenant_id?, workspace_id?, session_id?);
        let lane = lane?;
        let local_key = sanitize_key_fragment(&local_key?, "record");
        let parsed = Self::new(scope, lane, &local_key);
        (parsed.key == value).then_some(parsed)
    }

    pub fn as_str(&self) -> &str {
        &self.key
    }

    pub fn key_digest(&self) -> String {
        stable_digest(&self.key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantIsolationAuditEvent {
    pub access: TenantAccessKind,
    pub lane: TenantResourceLane,
    pub decision: TenantAccessDecision,
    pub actor_scope_digest: String,
    pub target_scope_digest: String,
    pub key_digest: String,
    pub reason: String,
    pub redacted: bool,
}

impl TenantIsolationAuditEvent {
    pub fn summary_line(&self) -> String {
        format!(
            "tenant_isolation access={} lane={} decision={} actor_scope={} target_scope={} key={} reason={} redacted={}",
            self.access.as_str(),
            self.lane.as_str(),
            self.decision.as_str(),
            self.actor_scope_digest,
            self.target_scope_digest,
            self.key_digest,
            self.reason,
            self.redacted
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantIsolationReport {
    pub allowed: bool,
    pub access: TenantAccessKind,
    pub lane: TenantResourceLane,
    pub audit_event: TenantIsolationAuditEvent,
    pub defense_spacer_activation_gate: Option<DefenseSpacerActivationGate>,
}

impl TenantIsolationReport {
    pub fn summary_line(&self) -> String {
        format!(
            "{} defense_spacer_allowed={}",
            self.audit_event.summary_line(),
            self.defense_spacer_activation_gate
                .as_ref()
                .map_or(true, |gate| gate.allowed)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TenantIsolationGate {
    pub allow_local_single_user_scope: bool,
}

impl Default for TenantIsolationGate {
    fn default() -> Self {
        Self {
            allow_local_single_user_scope: true,
        }
    }
}

impl TenantIsolationGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check_key_access(
        &self,
        actor_scope: &TenantScope,
        target_key: &TenantScopedKey,
        access: TenantAccessKind,
    ) -> TenantIsolationReport {
        self.check_scope_access(
            actor_scope,
            &target_key.scope,
            target_key.lane,
            access,
            &target_key.key_digest(),
        )
    }

    pub fn check_scope_access(
        &self,
        actor_scope: &TenantScope,
        target_scope: &TenantScope,
        lane: TenantResourceLane,
        access: TenantAccessKind,
        key_digest: &str,
    ) -> TenantIsolationReport {
        let local_single_user = actor_scope == target_scope
            && *actor_scope == TenantScope::local_single_user()
            && self.allow_local_single_user_scope;
        let same_scope = actor_scope == target_scope;
        let allowed = same_scope || local_single_user;
        let reason = if allowed {
            "scope_match"
        } else {
            "cross_tenant_scope_rejected"
        };
        tenant_report(
            allowed,
            access,
            lane,
            actor_scope,
            target_scope,
            key_digest,
            reason,
        )
    }

    pub fn check_genome_chain_access(
        &self,
        actor_scope: &TenantScope,
        chain: &DnaGeneChain,
        access: TenantAccessKind,
    ) -> TenantIsolationReport {
        let key_digest = stable_digest(&format!(
            "{}:{}:{}",
            chain.genome_id, chain.stable_anchor_id, chain.schema_version
        ));
        let mut target_scope: Option<TenantScope> = None;
        for record in chain.express_chain.iter().chain(chain.memory_chain.iter()) {
            if record.lineage.tenant_scope.trim().is_empty()
                || record.lineage.session_id.trim().is_empty()
            {
                return tenant_report(
                    false,
                    access,
                    TenantResourceLane::ReasoningGenome,
                    actor_scope,
                    actor_scope,
                    &key_digest,
                    "genome_lineage_missing",
                );
            }
            let record_scope =
                TenantScope::from_lineage(&record.lineage.tenant_scope, &record.lineage.session_id)
                    .unwrap_or_else(|| actor_scope.clone());
            if !record_scope
                .matches_lineage(&record.lineage.tenant_scope, &record.lineage.session_id)
            {
                return tenant_report(
                    false,
                    access,
                    TenantResourceLane::ReasoningGenome,
                    actor_scope,
                    actor_scope,
                    &key_digest,
                    "genome_lineage_malformed",
                );
            }
            match &target_scope {
                Some(existing) if existing != &record_scope => {
                    return tenant_report(
                        false,
                        access,
                        TenantResourceLane::ReasoningGenome,
                        actor_scope,
                        &record_scope,
                        &key_digest,
                        "genome_mixed_tenant_lineage_rejected",
                    );
                }
                Some(_) => {}
                None => target_scope = Some(record_scope),
            }
        }

        let Some(target_scope) = target_scope else {
            return tenant_report(
                false,
                access,
                TenantResourceLane::ReasoningGenome,
                actor_scope,
                actor_scope,
                &key_digest,
                "genome_empty_lineage_rejected",
            );
        };
        let scope_report = self.check_scope_access(
            actor_scope,
            &target_scope,
            TenantResourceLane::ReasoningGenome,
            access,
            &key_digest,
        );
        if !scope_report.allowed {
            return scope_report;
        }
        if matches!(access, TenantAccessKind::Write) && chain.read_only {
            return tenant_report(
                false,
                access,
                TenantResourceLane::ReasoningGenome,
                actor_scope,
                &target_scope,
                &key_digest,
                "genome_preview_write_blocked",
            );
        }
        scope_report
    }
}

impl TenantScope {
    fn from_lineage(tenant_scope: &str, session_id: &str) -> Option<Self> {
        let prefix = "tenant:";
        let workspace_marker = ":workspace:";
        let rest = tenant_scope.strip_prefix(prefix)?;
        let (tenant_id, workspace_id) = rest.split_once(workspace_marker)?;
        Some(Self::new(tenant_id, workspace_id, session_id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantScopedKvReadReport {
    pub isolation: TenantIsolationReport,
    pub value: Option<Vec<u8>>,
}

impl TenantScopedKvReadReport {
    pub fn summary_line(&self) -> String {
        self.isolation.summary_line()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantScopedKvWriteReport {
    pub isolation: TenantIsolationReport,
    pub applied: bool,
}

impl TenantScopedKvWriteReport {
    pub fn summary_line(&self) -> String {
        self.isolation.summary_line()
    }
}

pub fn tenant_scoped_put(
    store: &mut DiskKvStore,
    actor_scope: &TenantScope,
    key: &TenantScopedKey,
    value: impl AsRef<[u8]>,
) -> io::Result<TenantScopedKvWriteReport> {
    let gate = TenantIsolationGate::new();
    let isolation = gate.check_key_access(actor_scope, key, TenantAccessKind::Write);
    if !isolation.allowed {
        return Ok(TenantScopedKvWriteReport {
            isolation,
            applied: false,
        });
    }
    store.put(key.as_str(), value)?;
    Ok(TenantScopedKvWriteReport {
        isolation,
        applied: true,
    })
}

pub fn tenant_scoped_get(
    store: &DiskKvStore,
    actor_scope: &TenantScope,
    scoped_key: &str,
) -> io::Result<TenantScopedKvReadReport> {
    let parsed_key = TenantScopedKey::parse(scoped_key);
    let (isolation, value) = match parsed_key {
        Some(key) => {
            let gate = TenantIsolationGate::new();
            let isolation = gate.check_key_access(actor_scope, &key, TenantAccessKind::Read);
            if isolation.allowed {
                let value = store.get(key.as_str())?;
                (isolation, value)
            } else {
                (isolation, None)
            }
        }
        None => (
            tenant_report(
                false,
                TenantAccessKind::Read,
                TenantResourceLane::KvMemory,
                actor_scope,
                actor_scope,
                &stable_digest(scoped_key),
                "unscoped_or_malformed_key_rejected",
            ),
            None,
        ),
    };
    Ok(TenantScopedKvReadReport { isolation, value })
}

pub fn tenant_scoped_delete(
    store: &mut DiskKvStore,
    actor_scope: &TenantScope,
    scoped_key: &str,
) -> io::Result<TenantScopedKvWriteReport> {
    let parsed_key = TenantScopedKey::parse(scoped_key);
    let Some(key) = parsed_key else {
        return Ok(TenantScopedKvWriteReport {
            isolation: tenant_report(
                false,
                TenantAccessKind::Delete,
                TenantResourceLane::KvMemory,
                actor_scope,
                actor_scope,
                &stable_digest(scoped_key),
                "unscoped_or_malformed_key_rejected",
            ),
            applied: false,
        });
    };
    let gate = TenantIsolationGate::new();
    let isolation = gate.check_key_access(actor_scope, &key, TenantAccessKind::Delete);
    if !isolation.allowed {
        return Ok(TenantScopedKvWriteReport {
            isolation,
            applied: false,
        });
    }
    let applied = store.delete(key.as_str())?;
    Ok(TenantScopedKvWriteReport { isolation, applied })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantMigrationAction {
    ScopeLegacyKey,
    KeepScopedKey,
    RejectMalformedScopedKey,
    RejectCrossTenantScopedKey,
}

impl TenantMigrationAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ScopeLegacyKey => "scope_legacy_key",
            Self::KeepScopedKey => "keep_scoped_key",
            Self::RejectMalformedScopedKey => "reject_malformed_scoped_key",
            Self::RejectCrossTenantScopedKey => "reject_cross_tenant_scoped_key",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantMigrationRecord {
    pub original_key_digest: String,
    pub scoped_key: Option<String>,
    pub action: TenantMigrationAction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantMigrationPlan {
    pub default_scope: TenantScope,
    pub records: Vec<TenantMigrationRecord>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl TenantMigrationPlan {
    pub fn preview(default_scope: TenantScope, lane: TenantResourceLane, keys: &[String]) -> Self {
        let mut seen = BTreeSet::new();
        let mut records = Vec::new();
        for key in keys {
            if !seen.insert(key.clone()) {
                continue;
            }
            let parsed = TenantScopedKey::parse(key);
            let record = if key.starts_with("tenant=") && parsed.is_none() {
                TenantMigrationRecord {
                    original_key_digest: stable_digest(key),
                    scoped_key: None,
                    action: TenantMigrationAction::RejectMalformedScopedKey,
                }
            } else if let Some(scoped) = parsed {
                let same_scope = scoped.scope == default_scope && scoped.lane == lane;
                TenantMigrationRecord {
                    original_key_digest: stable_digest(key),
                    scoped_key: same_scope.then(|| scoped.as_str().to_owned()),
                    action: if same_scope {
                        TenantMigrationAction::KeepScopedKey
                    } else {
                        TenantMigrationAction::RejectCrossTenantScopedKey
                    },
                }
            } else {
                let scoped = default_scope.scoped_key(lane, key);
                TenantMigrationRecord {
                    original_key_digest: stable_digest(key),
                    scoped_key: Some(scoped.as_str().to_owned()),
                    action: TenantMigrationAction::ScopeLegacyKey,
                }
            };
            records.push(record);
        }

        Self {
            default_scope,
            records,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn legacy_scope_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.action == TenantMigrationAction::ScopeLegacyKey)
            .count()
    }

    pub fn rejected_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| {
                matches!(
                    record.action,
                    TenantMigrationAction::RejectMalformedScopedKey
                        | TenantMigrationAction::RejectCrossTenantScopedKey
                )
            })
            .count()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "tenant_migration_preview records={} legacy_scoped={} rejected={} read_only={} write_allowed={} applied={}",
            self.records.len(),
            self.legacy_scope_count(),
            self.rejected_count(),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

fn tenant_report(
    allowed: bool,
    access: TenantAccessKind,
    lane: TenantResourceLane,
    actor_scope: &TenantScope,
    target_scope: &TenantScope,
    key_digest: &str,
    reason: &str,
) -> TenantIsolationReport {
    let decision = if allowed {
        TenantAccessDecision::Allowed
    } else {
        TenantAccessDecision::Rejected
    };
    let defense_spacer_activation_gate = tenant_defense_spacer_activation_gate(
        allowed,
        access,
        lane,
        actor_scope,
        target_scope,
        key_digest,
        reason,
    );
    TenantIsolationReport {
        allowed,
        access,
        lane,
        audit_event: TenantIsolationAuditEvent {
            access,
            lane,
            decision,
            actor_scope_digest: actor_scope.scope_digest(),
            target_scope_digest: target_scope.scope_digest(),
            key_digest: key_digest.to_owned(),
            reason: reason.to_owned(),
            redacted: true,
        },
        defense_spacer_activation_gate,
    }
}

fn tenant_defense_spacer_activation_gate(
    allowed: bool,
    access: TenantAccessKind,
    lane: TenantResourceLane,
    actor_scope: &TenantScope,
    target_scope: &TenantScope,
    key_digest: &str,
    reason: &str,
) -> Option<DefenseSpacerActivationGate> {
    if allowed
        || !matches!(
            reason,
            "cross_tenant_scope_rejected" | "genome_mixed_tenant_lineage_rejected"
        )
    {
        return None;
    }

    let actor_digest = actor_scope.scope_digest();
    let target_digest = target_scope.scope_digest();
    let event_id = format!(
        "tenant-isolation-{}",
        stable_digest(&format!(
            "{}:{}:{}:{}:{}",
            actor_digest,
            target_digest,
            lane.as_str(),
            access.as_str(),
            key_digest
        ))
    );
    let payload_digest = format!(
        "tenant_scope_boundary actor={} target={} lane={} access={} key={}",
        actor_digest,
        target_digest,
        lane.as_str(),
        access.as_str(),
        key_digest
    );
    let finding = classify_development_pollution_event(&DevelopmentPollutionEvent::new(
        event_id,
        "tenant_scope_boundary",
        payload_digest,
        "cross_tenant_memory_or_genome",
    ));
    let spacer = DefenseSpacer::from_finding(
        &finding,
        "tenant_scope_boundary_activation",
        "runtime-write",
        "tenant_scope_match_or_operator_approval",
    );
    let candidate =
        DefenseSpacerCandidate::from_finding(&finding, "tenant_scope_boundary_activation");
    Some(gate_defense_spacer_activation(&[spacer], &candidate))
}

fn str_to_lane(value: &str) -> Option<TenantResourceLane> {
    match value {
        "kv_memory" => Some(TenantResourceLane::KvMemory),
        "reasoning_genome" => Some(TenantResourceLane::ReasoningGenome),
        "runtime_kv" => Some(TenantResourceLane::RuntimeKv),
        "session_state" => Some(TenantResourceLane::SessionState),
        "trace_evidence" => Some(TenantResourceLane::TraceEvidence),
        "self_evolving_memory" => Some(TenantResourceLane::SelfEvolvingMemory),
        "approval_packet" => Some(TenantResourceLane::ApprovalPacket),
        "evolution_goal_queue" => Some(TenantResourceLane::EvolutionGoalQueue),
        _ => None,
    }
}

fn sanitize_scope_id(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        .flat_map(char::to_lowercase)
        .take(64)
        .collect::<String>();
    if sanitized.is_empty() {
        fallback.to_owned()
    } else {
        sanitized
    }
}

fn sanitize_key_fragment(value: &str, fallback: &str) -> String {
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
    if sanitized.trim_matches('_').is_empty() {
        fallback.to_owned()
    } else {
        sanitized
    }
}

fn stable_digest(value: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("fnv64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning_genome::{
        DnaGeneChain, DnaGeneEvidenceKind, DnaGeneSourceEvidence, ReasoningGenome,
    };

    #[test]
    fn scoped_keys_roundtrip_and_hide_raw_audit_identifiers() {
        let scope = TenantScope::new("Tenant A", "Workspace One", "Session 01");
        let key = scope.scoped_key(TenantResourceLane::RuntimeKv, "Layer 1 / Head 2");
        let parsed = TenantScopedKey::parse(key.as_str()).expect("scoped key");
        let report =
            TenantIsolationGate::new().check_key_access(&scope, &key, TenantAccessKind::Read);

        assert_eq!(parsed.scope, scope);
        assert_eq!(parsed.lane, TenantResourceLane::RuntimeKv);
        assert!(key.as_str().contains("tenant=tenanta"));
        assert!(report.allowed);
        assert!(report.audit_event.redacted);
        assert!(!report.summary_line().contains("Tenant A"));
        assert!(!report.summary_line().contains("Workspace One"));
        assert!(report.summary_line().contains("actor_scope=fnv64:"));
    }

    #[test]
    fn disk_kv_scoped_access_rejects_cross_tenant_reads_and_deletes() {
        let path = temp_path("tenant-kv");
        let mut store = DiskKvStore::open(&path).unwrap();
        let tenant_a = TenantScope::new("tenant-a", "workspace", "session");
        let tenant_b = TenantScope::new("tenant-b", "workspace", "session");
        let key_a = tenant_a.scoped_key(TenantResourceLane::KvMemory, "episode:1");

        let write = tenant_scoped_put(&mut store, &tenant_a, &key_a, b"tenant-a-memory").unwrap();
        let read_a = tenant_scoped_get(&store, &tenant_a, key_a.as_str()).unwrap();
        let read_b = tenant_scoped_get(&store, &tenant_b, key_a.as_str()).unwrap();
        let delete_b = tenant_scoped_delete(&mut store, &tenant_b, key_a.as_str()).unwrap();
        let read_after_rejected_delete =
            tenant_scoped_get(&store, &tenant_a, key_a.as_str()).unwrap();

        assert!(write.applied);
        assert!(read_a.isolation.allowed);
        assert_eq!(read_a.value, Some(b"tenant-a-memory".to_vec()));
        assert!(!read_b.isolation.allowed);
        assert_eq!(read_b.value, None);
        assert!(!delete_b.applied);
        assert!(read_after_rejected_delete.value.is_some());
        let spacer_gate = read_b
            .isolation
            .defense_spacer_activation_gate
            .as_ref()
            .expect("cross-tenant read should carry DefenseSpacer activation proof");
        assert!(!spacer_gate.allowed);
        assert_eq!(spacer_gate.decision.as_str(), "block");
        assert_eq!(spacer_gate.reason, "matched_blocking_defense_spacer");
        assert!(
            read_b
                .summary_line()
                .contains("reason=cross_tenant_scope_rejected")
        );
        assert!(
            read_b
                .summary_line()
                .contains("defense_spacer_allowed=false")
        );
        assert!(!read_b.summary_line().contains("tenant-a"));
        assert!(!read_b.summary_line().contains("episode:1"));
        cleanup(path);
    }

    #[test]
    fn genome_inheritance_and_scoring_are_rejected_across_tenants() {
        let tenant_a = TenantScope::new("tenant-a", "workspace", "session");
        let tenant_b = TenantScope::new("tenant-b", "workspace", "session");
        let genome = ReasoningGenome::default_for_profile(crate::hierarchy::TaskProfile::Coding);
        let chain = DnaGeneChain::preview_from_genome(
            &genome,
            tenant_a.lineage_tenant_scope(),
            tenant_a.session_id.clone(),
            DnaGeneSourceEvidence::new(
                DnaGeneEvidenceKind::SyntheticDefault,
                "sha256:tenant-a-genome",
                "redacted tenant genome preview",
            )
            .with_privacy_gate(),
        );
        let gate = TenantIsolationGate::new();

        let allowed = gate.check_genome_chain_access(&tenant_a, &chain, TenantAccessKind::Inherit);
        let rejected = gate.check_genome_chain_access(&tenant_b, &chain, TenantAccessKind::Inherit);
        let score_allowed =
            gate.check_genome_chain_access(&tenant_a, &chain, TenantAccessKind::Score);
        let score_rejected =
            gate.check_genome_chain_access(&tenant_b, &chain, TenantAccessKind::Score);
        let write_rejected =
            gate.check_genome_chain_access(&tenant_a, &chain, TenantAccessKind::Write);
        let cross_tenant_write_rejected =
            gate.check_genome_chain_access(&tenant_b, &chain, TenantAccessKind::Write);

        assert!(allowed.allowed);
        assert!(allowed.defense_spacer_activation_gate.is_none());
        assert!(!rejected.allowed);
        assert!(score_allowed.allowed);
        assert!(!score_rejected.allowed);
        assert_eq!(score_rejected.access, TenantAccessKind::Score);
        assert_eq!(
            score_rejected.audit_event.reason,
            "cross_tenant_scope_rejected"
        );
        assert!(
            rejected
                .summary_line()
                .contains("reason=cross_tenant_scope_rejected")
        );
        assert!(!write_rejected.allowed);
        assert!(
            write_rejected
                .summary_line()
                .contains("genome_preview_write_blocked")
        );
        assert!(!cross_tenant_write_rejected.allowed);
        assert_eq!(
            cross_tenant_write_rejected.audit_event.reason,
            "cross_tenant_scope_rejected"
        );
        assert_eq!(cross_tenant_write_rejected.access, TenantAccessKind::Write);
        let spacer_gate = cross_tenant_write_rejected
            .defense_spacer_activation_gate
            .as_ref()
            .expect("cross-tenant genome write should carry DefenseSpacer activation proof");
        assert!(!spacer_gate.allowed);
        assert_eq!(spacer_gate.decision.as_str(), "block");
        assert!(spacer_gate.summary_line().contains("decision=block"));
        assert!(write_rejected.defense_spacer_activation_gate.is_none());
        assert!(!rejected.summary_line().contains("tenant-a"));
    }

    #[test]
    fn migration_preview_scopes_legacy_keys_without_applying() {
        let default_scope = TenantScope::local_single_user();
        let same_key = default_scope.scoped_key(TenantResourceLane::KvMemory, "already-scoped");
        let foreign_scope = TenantScope::new("foreign", "default", "interactive");
        let foreign_key = foreign_scope.scoped_key(TenantResourceLane::KvMemory, "foreign-record");
        let keys = vec![
            "legacy-memory-row".to_owned(),
            same_key.as_str().to_owned(),
            foreign_key.as_str().to_owned(),
            "tenant=broken;lane=kv_memory".to_owned(),
        ];

        let plan = TenantMigrationPlan::preview(default_scope, TenantResourceLane::KvMemory, &keys);

        assert!(plan.read_only);
        assert!(!plan.write_allowed);
        assert!(!plan.applied);
        assert_eq!(plan.legacy_scope_count(), 1);
        assert_eq!(plan.rejected_count(), 2);
        assert!(plan.records.iter().any(|record| {
            record.action == TenantMigrationAction::KeepScopedKey && record.scoped_key.is_some()
        }));
        assert!(plan.summary_line().contains("legacy_scoped=1"));
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
