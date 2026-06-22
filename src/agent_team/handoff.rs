use std::collections::{BTreeMap, BTreeSet};

use super::types::AgentRole;
use super::util::{compact, stable_hash};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgentHandoffTrustState {
    Trusted,
    NeedsReview,
    Quarantined,
}

impl AgentHandoffTrustState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trusted => "trusted",
            Self::NeedsReview => "needs_review",
            Self::Quarantined => "quarantined",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHandoffContext {
    pub current_branch: String,
    pub current_head: String,
    pub dirty_files: Vec<String>,
    pub known_issue_refs: Vec<String>,
    pub known_pr_refs: Vec<String>,
}

impl Default for AgentHandoffContext {
    fn default() -> Self {
        Self {
            current_branch: String::new(),
            current_head: String::new(),
            dirty_files: Vec::new(),
            known_issue_refs: Vec::new(),
            known_pr_refs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHandoffInput {
    pub source_id: String,
    pub role: AgentRole,
    pub summary: String,
    pub touched_files: Vec<String>,
    pub claimed_validation: Vec<String>,
    pub unresolved_risks: Vec<String>,
    pub stale_assumptions: Vec<String>,
    pub referenced_issues: Vec<String>,
    pub referenced_prs: Vec<String>,
    pub claimed_branch: Option<String>,
    pub claimed_head: Option<String>,
    pub raw_payload_present: bool,
    pub private_payload_present: bool,
}

impl AgentHandoffInput {
    pub fn new(source_id: impl Into<String>, role: AgentRole, summary: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            role,
            summary: summary.into(),
            touched_files: Vec::new(),
            claimed_validation: Vec::new(),
            unresolved_risks: Vec::new(),
            stale_assumptions: Vec::new(),
            referenced_issues: Vec::new(),
            referenced_prs: Vec::new(),
            claimed_branch: None,
            claimed_head: None,
            raw_payload_present: false,
            private_payload_present: false,
        }
    }

    pub fn with_touched_file(mut self, file: impl Into<String>) -> Self {
        push_unique_string(&mut self.touched_files, file.into());
        self
    }

    pub fn with_validation(mut self, validation: impl Into<String>) -> Self {
        push_unique_string(&mut self.claimed_validation, validation.into());
        self
    }

    pub fn with_unresolved_risk(mut self, risk: impl Into<String>) -> Self {
        push_unique_string(&mut self.unresolved_risks, risk.into());
        self
    }

    pub fn with_stale_assumption(mut self, assumption: impl Into<String>) -> Self {
        push_unique_string(&mut self.stale_assumptions, assumption.into());
        self
    }

    pub fn with_issue(mut self, issue: impl Into<String>) -> Self {
        push_unique_string(&mut self.referenced_issues, issue.into());
        self
    }

    pub fn with_pr(mut self, pr: impl Into<String>) -> Self {
        push_unique_string(&mut self.referenced_prs, pr.into());
        self
    }

    pub fn with_claimed_branch(mut self, branch: impl Into<String>) -> Self {
        self.claimed_branch = Some(branch.into());
        self
    }

    pub fn with_claimed_head(mut self, head: impl Into<String>) -> Self {
        self.claimed_head = Some(head.into());
        self
    }

    pub fn with_raw_payload_present(mut self, present: bool) -> Self {
        self.raw_payload_present = present;
        self
    }

    pub fn with_private_payload_present(mut self, present: bool) -> Self {
        self.private_payload_present = present;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHandoffReview {
    pub source_id: String,
    pub role: AgentRole,
    pub trust: AgentHandoffTrustState,
    pub accepted_facts: Vec<String>,
    pub rejected_claims: Vec<String>,
    pub conflicts: Vec<String>,
    pub follow_up_issues: Vec<String>,
    pub evidence_digests: Vec<String>,
    pub redactions: usize,
}

impl AgentHandoffReview {
    pub fn summary(&self) -> String {
        format!(
            "source={} role={} trust={} accepted={} rejected={} conflicts={} follow_up={} digests={} redactions={}",
            self.source_id,
            self.role.as_str(),
            self.trust.as_str(),
            self.accepted_facts.len(),
            self.rejected_claims.len(),
            self.conflicts.len(),
            self.follow_up_issues.len(),
            self.evidence_digests.len(),
            self.redactions
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHandoffAggregationReport {
    pub preview_only: bool,
    pub total_handoffs: usize,
    pub trusted_handoffs: usize,
    pub needs_review_handoffs: usize,
    pub quarantined_handoffs: usize,
    pub accepted_facts: Vec<String>,
    pub rejected_claims: Vec<String>,
    pub conflicts: Vec<String>,
    pub follow_up_issues: Vec<String>,
    pub evidence_digests: Vec<String>,
    pub redactions: usize,
    pub raw_payloads_blocked: usize,
    pub private_payloads_blocked: usize,
    pub duplicate_sources: usize,
    pub duplicate_claims: usize,
    pub reviews: Vec<AgentHandoffReview>,
}

impl Default for AgentHandoffAggregationReport {
    fn default() -> Self {
        Self {
            preview_only: true,
            total_handoffs: 0,
            trusted_handoffs: 0,
            needs_review_handoffs: 0,
            quarantined_handoffs: 0,
            accepted_facts: Vec::new(),
            rejected_claims: Vec::new(),
            conflicts: Vec::new(),
            follow_up_issues: Vec::new(),
            evidence_digests: Vec::new(),
            redactions: 0,
            raw_payloads_blocked: 0,
            private_payloads_blocked: 0,
            duplicate_sources: 0,
            duplicate_claims: 0,
            reviews: Vec::new(),
        }
    }
}

impl AgentHandoffAggregationReport {
    pub fn can_influence_main_thread(&self) -> bool {
        self.preview_only
            && self.quarantined_handoffs == 0
            && self.needs_review_handoffs == 0
            && self.conflicts.is_empty()
    }

    pub fn trusted_lessons(&self) -> Vec<String> {
        self.reviews
            .iter()
            .filter(|review| review.trust == AgentHandoffTrustState::Trusted)
            .flat_map(|review| review.accepted_facts.iter().cloned())
            .collect()
    }

    pub fn summary(&self) -> String {
        format!(
            "handoffs={} trusted={} needs_review={} quarantined={} accepted={} rejected={} conflicts={} follow_up={} digests={} redactions={} raw_blocked={} private_blocked={} duplicate_sources={} duplicate_claims={} preview_only={} can_influence={}",
            self.total_handoffs,
            self.trusted_handoffs,
            self.needs_review_handoffs,
            self.quarantined_handoffs,
            self.accepted_facts.len(),
            self.rejected_claims.len(),
            self.conflicts.len(),
            self.follow_up_issues.len(),
            self.evidence_digests.len(),
            self.redactions,
            self.raw_payloads_blocked,
            self.private_payloads_blocked,
            self.duplicate_sources,
            self.duplicate_claims,
            self.preview_only,
            self.can_influence_main_thread()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHandoffSanitizer {
    max_summary_chars: usize,
}

impl Default for AgentHandoffSanitizer {
    fn default() -> Self {
        Self {
            max_summary_chars: 160,
        }
    }
}

impl AgentHandoffSanitizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_summary_chars(mut self, max_summary_chars: usize) -> Self {
        self.max_summary_chars = max_summary_chars.max(16);
        self
    }

    pub fn sanitize(
        &self,
        context: &AgentHandoffContext,
        handoffs: &[AgentHandoffInput],
    ) -> AgentHandoffAggregationReport {
        let mut report = AgentHandoffAggregationReport {
            total_handoffs: handoffs.len(),
            ..AgentHandoffAggregationReport::default()
        };
        let mut seen_sources = BTreeSet::<String>::new();
        let mut seen_claims = BTreeMap::<String, String>::new();

        for handoff in handoffs {
            let mut review = self.review_handoff(context, handoff);
            let source_seen = !seen_sources.insert(review.source_id.clone());
            if source_seen {
                report.duplicate_sources += 1;
                escalate(&mut review.trust, AgentHandoffTrustState::NeedsReview);
                push_unique_string(
                    &mut review.conflicts,
                    format!("duplicate_source_id:{}", review.source_id),
                );
            }

            let claim_fingerprint = handoff_claim_fingerprint(handoff);
            if let Some(first_source) = seen_claims.get(&claim_fingerprint) {
                if first_source != &review.source_id {
                    report.duplicate_claims += 1;
                    escalate(&mut review.trust, AgentHandoffTrustState::NeedsReview);
                    push_unique_string(
                        &mut review.conflicts,
                        format!(
                            "duplicate_claim_fingerprint:{} first_source={}",
                            claim_fingerprint, first_source
                        ),
                    );
                }
            } else {
                seen_claims.insert(claim_fingerprint, review.source_id.clone());
            }

            if review.trust != AgentHandoffTrustState::Trusted {
                review.accepted_facts.clear();
            }

            match review.trust {
                AgentHandoffTrustState::Trusted => report.trusted_handoffs += 1,
                AgentHandoffTrustState::NeedsReview => report.needs_review_handoffs += 1,
                AgentHandoffTrustState::Quarantined => report.quarantined_handoffs += 1,
            }

            report.redactions = report.redactions.saturating_add(review.redactions);
            report.raw_payloads_blocked += usize::from(handoff.raw_payload_present);
            report.private_payloads_blocked += usize::from(handoff.private_payload_present);
            push_unique_all(&mut report.accepted_facts, &review.accepted_facts);
            push_unique_all(&mut report.rejected_claims, &review.rejected_claims);
            push_unique_all(&mut report.conflicts, &review.conflicts);
            push_unique_all(&mut report.follow_up_issues, &review.follow_up_issues);
            push_unique_all(&mut report.evidence_digests, &review.evidence_digests);
            report.reviews.push(review);
        }

        report
    }

    fn review_handoff(
        &self,
        context: &AgentHandoffContext,
        handoff: &AgentHandoffInput,
    ) -> AgentHandoffReview {
        let source_id = sanitize_identifier(&handoff.source_id, "source-unknown");
        let (summary, summary_redactions, summary_has_payload_marker) =
            sanitize_public_text(&handoff.summary, self.max_summary_chars);
        let mut review = AgentHandoffReview {
            source_id,
            role: handoff.role,
            trust: AgentHandoffTrustState::Trusted,
            accepted_facts: Vec::new(),
            rejected_claims: Vec::new(),
            conflicts: Vec::new(),
            follow_up_issues: Vec::new(),
            evidence_digests: vec![format!(
                "handoff:{:016x}",
                stable_hash(&handoff_digest_seed(handoff))
            )],
            redactions: summary_redactions,
        };

        let raw_or_private_payload = handoff.raw_payload_present
            || handoff.private_payload_present
            || summary_has_payload_marker
            || summary_redactions > 0;
        if raw_or_private_payload {
            escalate(&mut review.trust, AgentHandoffTrustState::Quarantined);
            push_unique_string(
                &mut review.rejected_claims,
                "handoff_raw_or_private_payload_blocked".to_owned(),
            );
        }

        if handoff.claimed_validation.is_empty() {
            escalate(&mut review.trust, AgentHandoffTrustState::NeedsReview);
            push_unique_string(
                &mut review.rejected_claims,
                "handoff_validation_missing".to_owned(),
            );
        }

        for risk in &handoff.unresolved_risks {
            escalate(&mut review.trust, AgentHandoffTrustState::NeedsReview);
            let (risk, redactions, _) = sanitize_public_text(risk, 96);
            review.redactions = review.redactions.saturating_add(redactions);
            push_unique_string(&mut review.conflicts, format!("unresolved_risk:{}", risk));
        }

        for assumption in &handoff.stale_assumptions {
            escalate(&mut review.trust, AgentHandoffTrustState::NeedsReview);
            let (assumption, redactions, _) = sanitize_public_text(assumption, 96);
            review.redactions = review.redactions.saturating_add(redactions);
            push_unique_string(
                &mut review.conflicts,
                format!("stale_assumption:{}", assumption),
            );
        }

        if let Some(claimed_branch) = &handoff.claimed_branch {
            let claimed_branch = sanitize_identifier(claimed_branch, "branch-unknown");
            if !context.current_branch.is_empty() && claimed_branch != context.current_branch {
                escalate(&mut review.trust, AgentHandoffTrustState::NeedsReview);
                push_unique_string(
                    &mut review.conflicts,
                    format!(
                        "branch_mismatch:claimed={} current={}",
                        claimed_branch, context.current_branch
                    ),
                );
            }
        }

        if let Some(claimed_head) = &handoff.claimed_head {
            let claimed_head = sanitize_identifier(claimed_head, "head-unknown");
            if !context.current_head.is_empty() && claimed_head != context.current_head {
                escalate(&mut review.trust, AgentHandoffTrustState::NeedsReview);
                push_unique_string(
                    &mut review.conflicts,
                    format!(
                        "head_mismatch:claimed={} current={}",
                        claimed_head, context.current_head
                    ),
                );
            }
        }

        let dirty_files = context
            .dirty_files
            .iter()
            .map(|file| sanitize_path(file))
            .collect::<BTreeSet<_>>();
        for touched_file in &handoff.touched_files {
            let touched_file = sanitize_path(touched_file);
            if dirty_files.contains(&touched_file) {
                escalate(&mut review.trust, AgentHandoffTrustState::NeedsReview);
                push_unique_string(
                    &mut review.conflicts,
                    format!("touched_file_dirty_in_main:{}", touched_file),
                );
            }
        }

        record_unknown_refs(
            &mut review,
            &handoff.referenced_issues,
            &context.known_issue_refs,
            "issue",
        );
        record_unknown_refs(
            &mut review,
            &handoff.referenced_prs,
            &context.known_pr_refs,
            "pr",
        );

        if review.trust == AgentHandoffTrustState::Trusted {
            push_unique_string(
                &mut review.accepted_facts,
                format!(
                    "source={} role={} summary={}",
                    review.source_id,
                    review.role.as_str(),
                    summary
                ),
            );
            for validation in &handoff.claimed_validation {
                let (validation, redactions, has_payload_marker) =
                    sanitize_public_text(validation, 96);
                review.redactions = review.redactions.saturating_add(redactions);
                if has_payload_marker || redactions > 0 {
                    escalate(&mut review.trust, AgentHandoffTrustState::Quarantined);
                    review.accepted_facts.clear();
                    push_unique_string(
                        &mut review.rejected_claims,
                        "handoff_validation_payload_blocked".to_owned(),
                    );
                    break;
                }
                push_unique_string(
                    &mut review.accepted_facts,
                    format!("validation={validation}"),
                );
            }
        }

        review
    }
}

fn record_unknown_refs(
    review: &mut AgentHandoffReview,
    refs: &[String],
    known_refs: &[String],
    kind: &str,
) {
    if known_refs.is_empty() {
        return;
    }

    let known = known_refs
        .iter()
        .map(|value| canonical_ref(value))
        .collect::<BTreeSet<_>>();
    for value in refs {
        let canonical = canonical_ref(value);
        if !known.contains(&canonical) {
            escalate(&mut review.trust, AgentHandoffTrustState::NeedsReview);
            push_unique_string(
                &mut review.follow_up_issues,
                format!("verify_{}_ref:{}", kind, canonical),
            );
        }
    }
}

fn escalate(trust: &mut AgentHandoffTrustState, next: AgentHandoffTrustState) {
    if next > *trust {
        *trust = next;
    }
}

fn handoff_claim_fingerprint(handoff: &AgentHandoffInput) -> String {
    let mut touched_files = handoff
        .touched_files
        .iter()
        .map(|file| sanitize_path(file))
        .collect::<Vec<_>>();
    touched_files.sort();
    let seed = format!(
        "{}|{}|{}",
        normalize_text_for_fingerprint(&handoff.summary),
        touched_files.join(","),
        handoff.role.as_str()
    );
    format!("{:016x}", stable_hash(&seed))
}

fn handoff_digest_seed(handoff: &AgentHandoffInput) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        handoff.source_id,
        handoff.role.as_str(),
        handoff.summary,
        handoff.touched_files.join(","),
        handoff.claimed_validation.join(","),
        handoff.unresolved_risks.join(",")
    )
}

fn normalize_text_for_fingerprint(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn canonical_ref(value: &str) -> String {
    sanitize_identifier(value.trim_start_matches('#'), "ref").to_ascii_lowercase()
}

fn sanitize_identifier(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':' | '#') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    if sanitized.is_empty() {
        fallback.to_owned()
    } else {
        sanitized.chars().take(120).collect()
    }
}

fn sanitize_path(value: &str) -> String {
    sanitize_identifier(&value.replace('\\', "/"), "path-unknown")
}

fn sanitize_public_text(value: &str, max_chars: usize) -> (String, usize, bool) {
    let lower = value.to_ascii_lowercase();
    let has_payload_marker = [
        "raw prompt",
        "raw_prompt",
        "raw response",
        "raw_response",
        "conversation transcript",
        "<conversation",
        "begin private",
        "-----begin",
    ]
    .iter()
    .any(|marker| lower.contains(marker));

    let mut redactions = 0usize;
    let sanitized = value
        .split_whitespace()
        .map(|token| {
            if token_is_sensitive(token) {
                redactions += 1;
                "[redacted]".to_owned()
            } else {
                token
                    .chars()
                    .filter(|ch| !ch.is_control())
                    .collect::<String>()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    (
        compact(&sanitized, max_chars),
        redactions,
        has_payload_marker,
    )
}

fn token_is_sensitive(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    lower.contains("password")
        || lower.contains("passwd")
        || lower.contains("secret")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("token=")
        || lower.contains("access_token")
        || lower.contains("bearer")
        || lower.starts_with("sk-")
}

fn push_unique_all(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        push_unique_string(target, value.clone());
    }
}

fn push_unique_string(target: &mut Vec<String>, value: String) {
    if !value.is_empty() && !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
}
