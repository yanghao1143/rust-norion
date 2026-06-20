use std::collections::BTreeSet;

use crate::{
    LongTermMatch, MemoryAdapter, MemoryAdapterCapability, MemoryAdapterDescriptor,
    MemoryAdapterHealth, MemoryIndexDocument, MemoryIndexSource, MemoryRequestContext,
    MemoryResult, MemoryScope, Metadata,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ContextCandidate {
    pub id: String,
    pub source: MemoryIndexSource,
    pub content: String,
    pub score: f32,
    pub scope: MemoryScope,
    pub metadata: Metadata,
    pub risk_reasons: Vec<String>,
    pub estimated_tokens: usize,
}

impl ContextCandidate {
    pub fn new(id: impl Into<String>, content: impl Into<String>, score: f32) -> Self {
        let content = content.into();
        let estimated_tokens = estimate_tokens(&content);
        Self {
            id: id.into(),
            source: MemoryIndexSource::LongTerm,
            content,
            score: score.clamp(0.0, 1.0),
            scope: MemoryScope::default(),
            metadata: Metadata::new(),
            risk_reasons: Vec::new(),
            estimated_tokens,
        }
    }

    pub fn from_long_term_match(item: &LongTermMatch) -> Self {
        Self {
            id: item.id.to_string(),
            source: MemoryIndexSource::LongTerm,
            content: item.content.clone(),
            score: item.score,
            scope: item.scope.clone(),
            metadata: item.metadata.clone(),
            risk_reasons: index_quality_risk_reasons(&item.metadata),
            estimated_tokens: estimate_tokens(&item.content),
        }
    }

    pub fn from_index_document(document: &MemoryIndexDocument) -> Self {
        Self {
            id: document.id.clone(),
            source: document.source,
            content: document.content.clone(),
            score: document.strength,
            scope: document.scope.clone(),
            metadata: document.metadata.clone(),
            risk_reasons: metadata_risk_reasons(&document.metadata),
            estimated_tokens: estimate_tokens(&document.content),
        }
    }

    pub fn with_source(mut self, source: MemoryIndexSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_scope(mut self, scope: MemoryScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_risk_reasons(mut self, reasons: Vec<String>) -> Self {
        self.risk_reasons = reasons;
        self.risk_reasons.sort();
        self.risk_reasons.dedup();
        self
    }

    pub fn with_estimated_tokens(mut self, estimated_tokens: usize) -> Self {
        self.estimated_tokens = estimated_tokens.max(1);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextInjectionPolicy {
    pub min_score: f32,
    pub max_items: usize,
    pub max_tokens: usize,
    pub summarize_above_tokens: usize,
    pub max_summary_chars: usize,
    pub allow_cross_task: bool,
    pub reject_risk_reasons: Vec<String>,
}

impl Default for ContextInjectionPolicy {
    fn default() -> Self {
        Self {
            min_score: 0.20,
            max_items: 8,
            max_tokens: 1_024,
            summarize_above_tokens: 256,
            max_summary_chars: 420,
            allow_cross_task: false,
            reject_risk_reasons: vec![
                "cross_task_transcript_pollution".to_owned(),
                "dirty_clean_gist".to_owned(),
                "missing_clean_gist".to_owned(),
                "quarantine_high_noise_records".to_owned(),
                "transcript_anchor_risk".to_owned(),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextDecisionKind {
    Admit,
    Summarize,
    RejectBudget,
    RejectRisk,
    RejectScope,
    RejectScore,
}

impl ContextDecisionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admit => "admit",
            Self::Summarize => "summarize",
            Self::RejectBudget => "reject_budget",
            Self::RejectRisk => "reject_risk",
            Self::RejectScope => "reject_scope",
            Self::RejectScore => "reject_score",
        }
    }

    pub fn accepted(self) -> bool {
        matches!(self, Self::Admit | Self::Summarize)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContextDecision {
    pub candidate_id: String,
    pub kind: ContextDecisionKind,
    pub injected_text: Option<String>,
    pub score: f32,
    pub estimated_tokens: usize,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContextInjectionPlan {
    pub decisions: Vec<ContextDecision>,
    pub used_tokens: usize,
}

impl ContextInjectionPlan {
    pub fn accepted_ids(&self) -> Vec<String> {
        self.decisions
            .iter()
            .filter(|decision| decision.kind.accepted())
            .map(|decision| decision.candidate_id.clone())
            .collect()
    }

    pub fn rejected_ids(&self) -> Vec<String> {
        self.decisions
            .iter()
            .filter(|decision| !decision.kind.accepted())
            .map(|decision| decision.candidate_id.clone())
            .collect()
    }

    pub fn injected_context(&self) -> Vec<&str> {
        self.decisions
            .iter()
            .filter_map(|decision| decision.injected_text.as_deref())
            .collect()
    }

    pub fn reason_codes(&self) -> Vec<String> {
        self.decisions
            .iter()
            .flat_map(|decision| decision.reasons.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        self.decisions
            .iter()
            .flat_map(|decision| {
                decision
                    .reasons
                    .iter()
                    .filter(|reason| !reason.is_empty())
                    .map(move |reason| {
                        format!(
                            "{}:{}:{}",
                            decision.kind.as_str(),
                            reason,
                            hex_id(&decision.candidate_id)
                        )
                    })
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn accepted_risk_count(&self) -> usize {
        self.decisions
            .iter()
            .filter(|decision| decision.kind.accepted() && !decision.reasons.is_empty())
            .count()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_context_injection decisions={} admit={} summarize={} reject_budget={} reject_risk={} reject_scope={} reject_score={} accepted_risk={} used_tokens={} reason_codes={} detail_codes={}",
            self.decisions.len(),
            self.count_kind(ContextDecisionKind::Admit),
            self.count_kind(ContextDecisionKind::Summarize),
            self.count_kind(ContextDecisionKind::RejectBudget),
            self.count_kind(ContextDecisionKind::RejectRisk),
            self.count_kind(ContextDecisionKind::RejectScope),
            self.count_kind(ContextDecisionKind::RejectScore),
            self.accepted_risk_count(),
            self.used_tokens,
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }

    fn count_kind(&self, kind: ContextDecisionKind) -> usize {
        self.decisions
            .iter()
            .filter(|decision| decision.kind == kind)
            .count()
    }
}

pub trait ContextInjectionGate {
    fn plan(
        &self,
        candidates: &[ContextCandidate],
        request: &MemoryRequestContext,
    ) -> ContextInjectionPlan;
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DefaultContextInjectionGate {
    pub policy: ContextInjectionPolicy,
}

impl DefaultContextInjectionGate {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MemoryAdapter for DefaultContextInjectionGate {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_context_injection_gate",
            vec![MemoryAdapterCapability::ContextInjection],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(None))
    }
}

impl ContextInjectionGate for DefaultContextInjectionGate {
    fn plan(
        &self,
        candidates: &[ContextCandidate],
        request: &MemoryRequestContext,
    ) -> ContextInjectionPlan {
        let mut ranked = candidates.to_vec();
        ranked.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.id.cmp(&right.id))
        });

        let mut decisions = Vec::new();
        let mut used_tokens = 0usize;
        let max_items = self.policy.max_items.min(request.limit.max(1));
        let mut accepted_items = 0usize;

        for candidate in ranked {
            let decision = self.decide_candidate(
                candidate,
                &request.scope,
                used_tokens,
                accepted_items,
                max_items,
            );
            if decision.kind.accepted() {
                used_tokens = used_tokens.saturating_add(decision.estimated_tokens);
                accepted_items = accepted_items.saturating_add(1);
            }
            decisions.push(decision);
        }

        ContextInjectionPlan {
            decisions,
            used_tokens,
        }
    }
}

impl DefaultContextInjectionGate {
    fn decide_candidate(
        &self,
        candidate: ContextCandidate,
        scope: &MemoryScope,
        used_tokens: usize,
        accepted_items: usize,
        max_items: usize,
    ) -> ContextDecision {
        if !self.policy.allow_cross_task && scope.same_task_as(&candidate.scope) == Some(false) {
            return reject(
                candidate,
                ContextDecisionKind::RejectScope,
                "cross_task_scope",
            );
        }
        if candidate.score < self.policy.min_score {
            return reject(
                candidate,
                ContextDecisionKind::RejectScore,
                "below_min_score",
            );
        }
        if let Some(reason) = candidate
            .risk_reasons
            .iter()
            .find(|reason| self.policy.reject_risk_reasons.contains(reason))
            .cloned()
        {
            let reasons = normalized_reasons(
                std::iter::once(reason)
                    .chain(candidate.risk_reasons.iter().cloned())
                    .collect(),
            );
            return reject_with_reasons(candidate, ContextDecisionKind::RejectRisk, reasons);
        }
        if accepted_items >= max_items {
            return reject(candidate, ContextDecisionKind::RejectBudget, "max_items");
        }

        let (kind, injected_text, tokens) =
            if candidate.estimated_tokens > self.policy.summarize_above_tokens {
                let summary = compact_context(&candidate.content, self.policy.max_summary_chars);
                let tokens = estimate_tokens(&summary);
                (
                    ContextDecisionKind::Summarize,
                    format!("summary: {summary}"),
                    tokens,
                )
            } else {
                (
                    ContextDecisionKind::Admit,
                    candidate.content.clone(),
                    candidate.estimated_tokens,
                )
            };

        if used_tokens.saturating_add(tokens) > self.policy.max_tokens {
            return reject(candidate, ContextDecisionKind::RejectBudget, "max_tokens");
        }

        let reasons = normalized_reasons(candidate.risk_reasons);
        ContextDecision {
            candidate_id: candidate.id,
            kind,
            injected_text: Some(injected_text),
            score: candidate.score,
            estimated_tokens: tokens,
            reasons,
        }
    }
}

fn reject(
    candidate: ContextCandidate,
    kind: ContextDecisionKind,
    reason: impl Into<String>,
) -> ContextDecision {
    reject_with_reasons(candidate, kind, vec![reason.into()])
}

fn reject_with_reasons(
    candidate: ContextCandidate,
    kind: ContextDecisionKind,
    reasons: Vec<String>,
) -> ContextDecision {
    ContextDecision {
        candidate_id: candidate.id,
        kind,
        injected_text: None,
        score: candidate.score,
        estimated_tokens: 0,
        reasons,
    }
}

fn normalized_reasons(mut reasons: Vec<String>) -> Vec<String> {
    reasons.retain(|reason| !reason.trim().is_empty());
    reasons.sort();
    reasons.dedup();
    reasons
}

fn metadata_risk_reasons(metadata: &Metadata) -> Vec<String> {
    let mut reasons = index_quality_risk_reasons(metadata);
    if let Some(tags) = metadata.get("tags") {
        reasons.extend(tags.split(',').filter_map(|tag| {
            tag.trim()
                .strip_prefix("risk:")
                .map(str::trim)
                .filter(|reason| !reason.is_empty())
                .map(str::to_owned)
        }));
    }
    normalized_reasons(reasons)
}

fn index_quality_risk_reasons(metadata: &Metadata) -> Vec<String> {
    let mut reasons = Vec::new();
    if metadata
        .get("content_basis")
        .is_some_and(|value| value == "raw_fallback")
    {
        reasons.push("raw_fallback_index_content".to_owned());
    }
    if metadata
        .get("content_truncated")
        .is_some_and(|value| value == "true")
    {
        reasons.push("truncated_index_content".to_owned());
    }
    reasons
}

fn estimate_tokens(value: &str) -> usize {
    let rough = value.split_whitespace().count();
    rough.max(value.chars().count() / 6).max(1)
}

fn compact_context(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut previous_space = false;
    for ch in value.chars().take(max_chars) {
        if ch.is_whitespace() {
            if !previous_space {
                out.push(' ');
                previous_space = true;
            }
        } else {
            out.push(ch);
            previous_space = false;
        }
    }
    out.trim().to_owned()
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

fn hex_id(id: &str) -> String {
    id.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LongTermMatch, MemoryAccessPurpose};

    fn request(task: &str) -> MemoryRequestContext {
        MemoryRequestContext::new(MemoryScope::for_task(task), MemoryAccessPurpose::Recall)
            .with_limit(8)
    }

    #[test]
    fn gate_rejects_cross_task_risky_and_low_score_candidates() {
        let candidates = vec![
            ContextCandidate::new("good", "safe runtime memory", 0.9)
                .with_scope(MemoryScope::for_task("runtime")),
            ContextCandidate::new("ops", "ops transcript", 0.95)
                .with_scope(MemoryScope::for_task("ops")),
            ContextCandidate::new("risky", "bad context", 0.8)
                .with_risk_reasons(vec!["cross_task_transcript_pollution".to_owned()]),
            ContextCandidate::new("weak", "weak memory", 0.1),
        ];

        let plan = DefaultContextInjectionGate::new().plan(&candidates, &request("runtime"));
        assert_eq!(plan.accepted_ids(), vec!["good".to_owned()]);
        assert!(plan.decisions.iter().any(|decision| {
            decision.candidate_id == "ops" && decision.kind == ContextDecisionKind::RejectScope
        }));
        assert!(plan.decisions.iter().any(|decision| {
            decision.candidate_id == "risky" && decision.kind == ContextDecisionKind::RejectRisk
        }));
        assert!(plan.decisions.iter().any(|decision| {
            decision.candidate_id == "weak" && decision.kind == ContextDecisionKind::RejectScore
        }));
        assert_eq!(
            plan.summary_line(),
            "memory_context_injection decisions=4 admit=1 summarize=0 reject_budget=0 reject_risk=1 reject_scope=1 reject_score=1 accepted_risk=0 used_tokens=3 reason_codes=below_min_score|cross_task_scope|cross_task_transcript_pollution detail_codes=reject_risk:cross_task_transcript_pollution:7269736b79|reject_scope:cross_task_scope:6f7073|reject_score:below_min_score:7765616b"
        );
        assert_eq!(
            plan.reason_codes(),
            vec![
                "below_min_score".to_owned(),
                "cross_task_scope".to_owned(),
                "cross_task_transcript_pollution".to_owned(),
            ]
        );
        assert_eq!(
            plan.detail_codes(),
            vec![
                "reject_risk:cross_task_transcript_pollution:7269736b79".to_owned(),
                "reject_scope:cross_task_scope:6f7073".to_owned(),
                "reject_score:below_min_score:7765616b".to_owned(),
            ]
        );
    }

    #[test]
    fn gate_summarizes_long_context_and_respects_budget() {
        let gate = DefaultContextInjectionGate {
            policy: ContextInjectionPolicy {
                max_tokens: 18,
                summarize_above_tokens: 8,
                max_summary_chars: 48,
                ..ContextInjectionPolicy::default()
            },
        };
        let candidates = vec![
            ContextCandidate::new(
                "long",
                "alpha beta gamma delta epsilon zeta eta theta iota",
                0.9,
            )
            .with_estimated_tokens(32),
            ContextCandidate::new("second", "short useful memory", 0.8).with_estimated_tokens(4),
            ContextCandidate::new(
                "overflow",
                "one two three four five six seven eight nine ten eleven twelve thirteen fourteen",
                0.7,
            )
            .with_estimated_tokens(20),
        ];

        let plan = gate.plan(&candidates, &request("runtime"));
        let long = plan
            .decisions
            .iter()
            .find(|decision| decision.candidate_id == "long")
            .unwrap();
        assert_eq!(long.kind, ContextDecisionKind::Summarize);
        assert!(
            long.injected_text
                .as_deref()
                .unwrap()
                .starts_with("summary: alpha beta")
        );
        assert!(plan.used_tokens <= 18);
        assert!(plan.decisions.iter().any(|decision| {
            decision.candidate_id == "overflow"
                && decision.kind == ContextDecisionKind::RejectBudget
        }));
        assert_eq!(
            plan.summary_line(),
            "memory_context_injection decisions=3 admit=1 summarize=1 reject_budget=1 reject_risk=0 reject_scope=0 reject_score=0 accepted_risk=0 used_tokens=13 reason_codes=max_tokens detail_codes=reject_budget:max_tokens:6f766572666c6f77"
        );
        assert_eq!(
            plan.detail_codes(),
            vec!["reject_budget:max_tokens:6f766572666c6f77".to_owned()]
        );
    }

    #[test]
    fn gate_keeps_index_quality_reasons_on_accepted_candidates() {
        let candidate = ContextCandidate::new("raw", "bounded raw fallback lesson", 0.8)
            .with_scope(MemoryScope::for_task("runtime"))
            .with_risk_reasons(vec![
                "truncated_index_content".to_owned(),
                "raw_fallback_index_content".to_owned(),
            ]);

        let plan = DefaultContextInjectionGate::new().plan(&[candidate], &request("runtime"));
        let decision = &plan.decisions[0];

        assert_eq!(decision.kind, ContextDecisionKind::Admit);
        assert_eq!(plan.accepted_risk_count(), 1);
        assert_eq!(
            plan.summary_line(),
            "memory_context_injection decisions=1 admit=1 summarize=0 reject_budget=0 reject_risk=0 reject_scope=0 reject_score=0 accepted_risk=1 used_tokens=4 reason_codes=raw_fallback_index_content|truncated_index_content detail_codes=admit:raw_fallback_index_content:726177|admit:truncated_index_content:726177"
        );
        assert_eq!(
            decision.reasons,
            vec![
                "raw_fallback_index_content".to_owned(),
                "truncated_index_content".to_owned()
            ]
        );
        assert_eq!(
            plan.reason_codes(),
            vec![
                "raw_fallback_index_content".to_owned(),
                "truncated_index_content".to_owned()
            ]
        );
        assert_eq!(
            plan.detail_codes(),
            vec![
                "admit:raw_fallback_index_content:726177".to_owned(),
                "admit:truncated_index_content:726177".to_owned()
            ]
        );
    }

    #[test]
    fn gate_reject_risk_preserves_complete_normalized_reason_set() {
        let candidate =
            ContextCandidate::new("rot", "raw fallback context with transcript anchors", 0.8)
                .with_scope(MemoryScope::for_task("runtime"))
                .with_risk_reasons(vec![
                    "raw_fallback_index_content".to_owned(),
                    "missing_clean_gist".to_owned(),
                    "transcript_anchor_risk".to_owned(),
                    "missing_clean_gist".to_owned(),
                    String::new(),
                ]);

        let plan = DefaultContextInjectionGate::new().plan(&[candidate], &request("runtime"));
        let decision = &plan.decisions[0];

        assert_eq!(decision.kind, ContextDecisionKind::RejectRisk);
        assert_eq!(
            decision.reasons,
            vec![
                "missing_clean_gist".to_owned(),
                "raw_fallback_index_content".to_owned(),
                "transcript_anchor_risk".to_owned(),
            ]
        );
        assert_eq!(
            plan.reason_codes(),
            vec![
                "missing_clean_gist".to_owned(),
                "raw_fallback_index_content".to_owned(),
                "transcript_anchor_risk".to_owned(),
            ]
        );
        assert_eq!(
            plan.detail_codes(),
            vec![
                "reject_risk:missing_clean_gist:726f74".to_owned(),
                "reject_risk:raw_fallback_index_content:726f74".to_owned(),
                "reject_risk:transcript_anchor_risk:726f74".to_owned(),
            ]
        );
        assert_eq!(
            plan.summary_line(),
            "memory_context_injection decisions=1 admit=0 summarize=0 reject_budget=0 reject_risk=1 reject_scope=0 reject_score=0 accepted_risk=0 used_tokens=0 reason_codes=missing_clean_gist|raw_fallback_index_content|transcript_anchor_risk detail_codes=reject_risk:missing_clean_gist:726f74|reject_risk:raw_fallback_index_content:726f74|reject_risk:transcript_anchor_risk:726f74"
        );
    }

    #[test]
    fn long_term_match_projects_to_context_candidate() {
        let mut metadata = Metadata::new();
        metadata.insert("kind".to_owned(), "lesson".to_owned());
        metadata.insert("content_basis".to_owned(), "raw_fallback".to_owned());
        metadata.insert("content_truncated".to_owned(), "true".to_owned());
        let item = LongTermMatch {
            id: 7,
            content: "scoped long-term match".to_owned(),
            score: 0.77,
            strength: 0.8,
            metadata,
            scope: MemoryScope::for_task("runtime"),
        };

        let candidate = ContextCandidate::from_long_term_match(&item);
        assert_eq!(candidate.id, "7");
        assert_eq!(candidate.source, MemoryIndexSource::LongTerm);
        assert_eq!(candidate.scope.task_id.as_deref(), Some("runtime"));
        assert_eq!(
            candidate.metadata.get("kind").map(String::as_str),
            Some("lesson")
        );
        assert_eq!(
            candidate.risk_reasons,
            vec![
                "raw_fallback_index_content".to_owned(),
                "truncated_index_content".to_owned()
            ]
        );
    }

    #[test]
    fn index_document_projects_to_context_candidate_with_quality_risks() {
        let mut metadata = Metadata::new();
        metadata.insert("content_basis".to_owned(), "raw_fallback".to_owned());
        metadata.insert("content_truncated".to_owned(), "true".to_owned());
        metadata.insert(
            "tags".to_owned(),
            "runtime,risk:cross_task_transcript_pollution".to_owned(),
        );
        let document = MemoryIndexDocument::new(
            "experience-7",
            MemoryIndexSource::Experience,
            "bounded fallback",
        )
        .with_scope(MemoryScope::for_task("runtime"))
        .with_strength(0.73)
        .with_metadata(metadata);

        let candidate = ContextCandidate::from_index_document(&document);

        assert_eq!(candidate.id, "experience-7");
        assert_eq!(candidate.source, MemoryIndexSource::Experience);
        assert_eq!(candidate.score, 0.73);
        assert_eq!(candidate.scope.task_id.as_deref(), Some("runtime"));
        assert_eq!(
            candidate.risk_reasons,
            vec![
                "cross_task_transcript_pollution".to_owned(),
                "raw_fallback_index_content".to_owned(),
                "truncated_index_content".to_owned()
            ]
        );
    }

    #[test]
    fn gate_is_read_only_adapter() {
        let descriptor = DefaultContextInjectionGate::new().descriptor();
        assert_eq!(descriptor.name, "default_context_injection_gate");
        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::ContextInjection)
        );
    }
}
