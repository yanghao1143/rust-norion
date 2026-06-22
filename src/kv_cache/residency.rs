#[derive(Debug, Clone, PartialEq)]
pub struct MemoryResidencyPolicy {
    pub tenant_id: String,
    pub hot_score_threshold: f32,
    pub warm_score_threshold: f32,
    pub cold_score_threshold: f32,
    pub max_shared_privacy_risk: f32,
    pub quarantine_privacy_risk: f32,
    pub stale_after_steps: u64,
    pub retire_after_steps: u64,
    pub max_hot: usize,
    pub max_warm: usize,
}

impl Default for MemoryResidencyPolicy {
    fn default() -> Self {
        Self {
            tenant_id: "local".to_owned(),
            hot_score_threshold: 0.78,
            warm_score_threshold: 0.52,
            cold_score_threshold: 0.24,
            max_shared_privacy_risk: 0.20,
            quarantine_privacy_risk: 0.72,
            stale_after_steps: 64,
            retire_after_steps: 256,
            max_hot: 16,
            max_warm: 64,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryResidencyCandidate {
    pub id: u64,
    pub tenant_id: String,
    pub namespace: String,
    pub usefulness: f32,
    pub hit_count: u64,
    pub failure_count: u64,
    pub last_access_step: u64,
    pub token_estimate: usize,
    pub privacy_checked: bool,
    pub privacy_risk: f32,
    pub validation_evidence_count: usize,
    pub rollback_anchor_id: String,
    pub protected_rollback_anchor: bool,
    pub high_frequency_gene: bool,
    pub session_local: bool,
}

impl MemoryResidencyCandidate {
    pub fn new(id: u64, tenant_id: impl Into<String>, namespace: impl Into<String>) -> Self {
        Self {
            id,
            tenant_id: tenant_id.into(),
            namespace: namespace.into(),
            usefulness: 0.50,
            hit_count: 0,
            failure_count: 0,
            last_access_step: 0,
            token_estimate: 1,
            privacy_checked: true,
            privacy_risk: 0.0,
            validation_evidence_count: 1,
            rollback_anchor_id: format!("rollback:memory:{id}"),
            protected_rollback_anchor: false,
            high_frequency_gene: false,
            session_local: false,
        }
    }

    pub fn with_scores(
        mut self,
        usefulness: f32,
        hit_count: u64,
        failure_count: u64,
        last_access_step: u64,
    ) -> Self {
        self.usefulness = clamp_unit(usefulness);
        self.hit_count = hit_count;
        self.failure_count = failure_count;
        self.last_access_step = last_access_step;
        self
    }

    pub fn with_privacy(mut self, privacy_checked: bool, privacy_risk: f32) -> Self {
        self.privacy_checked = privacy_checked;
        self.privacy_risk = clamp_unit(privacy_risk);
        self
    }

    pub fn with_validation_evidence_count(mut self, count: usize) -> Self {
        self.validation_evidence_count = count;
        self
    }

    pub fn with_rollback_anchor(
        mut self,
        rollback_anchor_id: impl Into<String>,
        protected: bool,
    ) -> Self {
        self.rollback_anchor_id = rollback_anchor_id.into();
        self.protected_rollback_anchor = protected;
        self
    }

    pub fn with_high_frequency_gene(mut self, high_frequency_gene: bool) -> Self {
        self.high_frequency_gene = high_frequency_gene;
        self
    }

    pub fn with_session_local(mut self, session_local: bool) -> Self {
        self.session_local = session_local;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryResidencyState {
    Hot,
    Warm,
    Cold,
    Quarantined,
    Retired,
}

impl MemoryResidencyState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hot => "hot",
            Self::Warm => "warm",
            Self::Cold => "cold",
            Self::Quarantined => "quarantined",
            Self::Retired => "retired",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryResidencyDecisionRecord {
    pub id: u64,
    pub target_state: MemoryResidencyState,
    pub score: f32,
    pub tenant_id_digest: String,
    pub namespace_digest: String,
    pub rollback_anchor_digest: String,
    pub protected_rollback_anchor: bool,
    pub blocked_reasons: Vec<String>,
    pub token_estimate: usize,
}

impl MemoryResidencyDecisionRecord {
    pub fn is_hot(&self) -> bool {
        self.target_state == MemoryResidencyState::Hot
    }

    pub fn is_warm(&self) -> bool {
        self.target_state == MemoryResidencyState::Warm
    }

    pub fn is_cold(&self) -> bool {
        self.target_state == MemoryResidencyState::Cold
    }

    pub fn is_quarantined(&self) -> bool {
        self.target_state == MemoryResidencyState::Quarantined
    }

    pub fn is_retired(&self) -> bool {
        self.target_state == MemoryResidencyState::Retired
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryResidencyPlan {
    pub decisions: Vec<MemoryResidencyDecisionRecord>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
    pub replay_digest: String,
}

impl MemoryResidencyPlan {
    pub fn protected_ids_for_compaction(&self) -> Vec<u64> {
        self.decisions
            .iter()
            .filter(|decision| decision.protected_rollback_anchor)
            .map(|decision| decision.id)
            .collect()
    }

    pub fn protected_rollback_anchor_count(&self) -> usize {
        self.decisions
            .iter()
            .filter(|decision| decision.protected_rollback_anchor)
            .count()
    }

    pub fn blocked_reason_count(&self) -> usize {
        self.decisions
            .iter()
            .map(|decision| decision.blocked_reasons.len())
            .sum()
    }

    pub fn total_token_estimate(&self) -> usize {
        self.decisions
            .iter()
            .map(|decision| decision.token_estimate)
            .sum()
    }

    pub fn count_state(&self, state: MemoryResidencyState) -> usize {
        self.decisions
            .iter()
            .filter(|decision| decision.target_state == state)
            .count()
    }

    pub fn hot_count(&self) -> usize {
        self.count_state(MemoryResidencyState::Hot)
    }

    pub fn warm_count(&self) -> usize {
        self.count_state(MemoryResidencyState::Warm)
    }

    pub fn cold_count(&self) -> usize {
        self.count_state(MemoryResidencyState::Cold)
    }

    pub fn quarantined_count(&self) -> usize {
        self.count_state(MemoryResidencyState::Quarantined)
    }

    pub fn retired_count(&self) -> usize {
        self.count_state(MemoryResidencyState::Retired)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_residency_plan decisions={} hot={} warm={} cold={} quarantined={} retired={} protected={} read_only={} write_allowed={} applied={} replay_digest={}",
            self.decisions.len(),
            self.hot_count(),
            self.warm_count(),
            self.cold_count(),
            self.quarantined_count(),
            self.retired_count(),
            self.protected_rollback_anchor_count(),
            self.read_only,
            self.write_allowed,
            self.applied,
            self.replay_digest
        )
    }
}

pub fn plan_memory_residency(
    candidates: &[MemoryResidencyCandidate],
    policy: &MemoryResidencyPolicy,
    current_step: u64,
) -> MemoryResidencyPlan {
    let tenant_id = if policy.tenant_id.trim().is_empty() {
        "local"
    } else {
        policy.tenant_id.trim()
    };
    let mut decisions = candidates
        .iter()
        .map(|candidate| initial_decision(candidate, policy, tenant_id, current_step))
        .collect::<Vec<_>>();

    decisions.sort_by(|left, right| {
        state_rank(left.target_state)
            .cmp(&state_rank(right.target_state))
            .then_with(|| {
                right
                    .score
                    .partial_cmp(&left.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.id.cmp(&right.id))
    });

    enforce_residency_budgets(&mut decisions, policy);
    decisions.sort_by_key(|decision| decision.id);

    let replay_digest = residency_replay_digest(&decisions);

    MemoryResidencyPlan {
        decisions,
        read_only: true,
        write_allowed: false,
        applied: false,
        replay_digest,
    }
}

fn initial_decision(
    candidate: &MemoryResidencyCandidate,
    policy: &MemoryResidencyPolicy,
    tenant_id: &str,
    current_step: u64,
) -> MemoryResidencyDecisionRecord {
    let mut blocked_reasons = Vec::new();
    let score = residency_score(candidate, current_step);
    let age = current_step.saturating_sub(candidate.last_access_step);
    let tenant_matches = candidate.tenant_id == tenant_id;
    let protected = candidate.protected_rollback_anchor && !candidate.rollback_anchor_id.is_empty();

    if !tenant_matches {
        blocked_reasons.push("memory_residency_tenant_mismatch".to_owned());
    }
    if !candidate.privacy_checked {
        blocked_reasons.push("memory_residency_privacy_check_missing".to_owned());
    }
    if candidate.privacy_risk > policy.max_shared_privacy_risk {
        blocked_reasons.push("memory_residency_shared_privacy_risk".to_owned());
    }
    if candidate.validation_evidence_count == 0 {
        blocked_reasons.push("memory_residency_validation_evidence_missing".to_owned());
    }
    if protected {
        blocked_reasons.push("memory_residency_rollback_anchor_protected".to_owned());
    }

    let target_state = if !tenant_matches
        || !candidate.privacy_checked
        || candidate.privacy_risk >= policy.quarantine_privacy_risk
    {
        MemoryResidencyState::Quarantined
    } else if protected {
        if score >= policy.warm_score_threshold {
            MemoryResidencyState::Warm
        } else {
            MemoryResidencyState::Cold
        }
    } else if candidate.validation_evidence_count == 0 {
        MemoryResidencyState::Cold
    } else if age >= policy.retire_after_steps && score < policy.cold_score_threshold {
        MemoryResidencyState::Retired
    } else if candidate.privacy_risk > policy.max_shared_privacy_risk {
        MemoryResidencyState::Cold
    } else if candidate.high_frequency_gene && score >= policy.hot_score_threshold {
        MemoryResidencyState::Hot
    } else if candidate.session_local && score >= policy.cold_score_threshold {
        MemoryResidencyState::Warm
    } else if score >= policy.warm_score_threshold {
        MemoryResidencyState::Warm
    } else if score >= policy.cold_score_threshold || age < policy.stale_after_steps {
        MemoryResidencyState::Cold
    } else {
        MemoryResidencyState::Retired
    };

    MemoryResidencyDecisionRecord {
        id: candidate.id,
        target_state,
        score,
        tenant_id_digest: stable_digest(&candidate.tenant_id),
        namespace_digest: stable_digest(&candidate.namespace),
        rollback_anchor_digest: stable_digest(&candidate.rollback_anchor_id),
        protected_rollback_anchor: protected,
        blocked_reasons,
        token_estimate: candidate.token_estimate.max(1),
    }
}

fn enforce_residency_budgets(
    decisions: &mut [MemoryResidencyDecisionRecord],
    policy: &MemoryResidencyPolicy,
) {
    let mut hot_seen = 0usize;
    let mut warm_seen = 0usize;
    for decision in decisions {
        match decision.target_state {
            MemoryResidencyState::Hot => {
                hot_seen = hot_seen.saturating_add(1);
                if hot_seen > policy.max_hot {
                    decision.target_state = MemoryResidencyState::Warm;
                    decision
                        .blocked_reasons
                        .push("memory_residency_hot_budget_exhausted".to_owned());
                    warm_seen = warm_seen.saturating_add(1);
                }
            }
            MemoryResidencyState::Warm => {
                warm_seen = warm_seen.saturating_add(1);
            }
            _ => {}
        }
        if decision.target_state == MemoryResidencyState::Warm && warm_seen > policy.max_warm {
            decision.target_state = MemoryResidencyState::Cold;
            decision
                .blocked_reasons
                .push("memory_residency_warm_budget_exhausted".to_owned());
        }
    }
}

fn residency_score(candidate: &MemoryResidencyCandidate, current_step: u64) -> f32 {
    let age = current_step.saturating_sub(candidate.last_access_step) as f32;
    let hit_boost = (candidate.hit_count as f32 * 0.025).min(0.22);
    let failure_penalty = (candidate.failure_count as f32 * 0.07).min(0.35);
    let age_penalty = (age / 512.0).min(0.30);
    let validation_boost = (candidate.validation_evidence_count as f32 * 0.035).min(0.14);
    let frequency_boost = if candidate.high_frequency_gene {
        0.08
    } else {
        0.0
    };
    let session_boost = if candidate.session_local { 0.035 } else { 0.0 };
    (clamp_unit(candidate.usefulness)
        + hit_boost
        + validation_boost
        + frequency_boost
        + session_boost
        - failure_penalty
        - age_penalty
        - candidate.privacy_risk.clamp(0.0, 1.0) * 0.18)
        .clamp(0.0, 1.0)
}

fn state_rank(state: MemoryResidencyState) -> usize {
    match state {
        MemoryResidencyState::Hot => 0,
        MemoryResidencyState::Warm => 1,
        MemoryResidencyState::Cold => 2,
        MemoryResidencyState::Quarantined => 3,
        MemoryResidencyState::Retired => 4,
    }
}

fn residency_replay_digest(decisions: &[MemoryResidencyDecisionRecord]) -> String {
    let mut payload = String::new();
    for decision in decisions {
        payload.push_str(&format!(
            "{}:{}:{:.6}:{}:{}:{}:{}|",
            decision.id,
            decision.target_state.as_str(),
            decision.score,
            decision.tenant_id_digest,
            decision.namespace_digest,
            decision.rollback_anchor_digest,
            decision.protected_rollback_anchor
        ));
    }
    stable_digest(&payload)
}

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
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
