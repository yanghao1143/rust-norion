use crate::privacy_redaction::{privacy_redaction_reason_codes, stable_redaction_digest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DangerSignalDecision {
    SelfTrusted,
    ObserveOnly,
    HoldForProvenance,
    QuarantineNonSelf,
    RejectDangerSignal,
    RequireOperatorReview,
}

impl DangerSignalDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SelfTrusted => "self_trusted",
            Self::ObserveOnly => "observe_only",
            Self::HoldForProvenance => "hold_for_provenance",
            Self::QuarantineNonSelf => "quarantine_non_self",
            Self::RejectDangerSignal => "reject_danger_signal",
            Self::RequireOperatorReview => "require_operator_review",
        }
    }

    pub fn activation_allowed(self) -> bool {
        matches!(self, Self::SelfTrusted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DangerSignalInput {
    pub candidate_kind: &'static str,
    pub trusted_self_provenance: bool,
    pub source_digest: String,
    pub lifecycle_state: String,
    pub affected_scope: String,
    pub marker_text: String,
    pub unexpected_tool_permission: bool,
    pub benchmark_or_verifier_damage: bool,
}

impl DangerSignalInput {
    pub fn new(candidate_kind: &'static str) -> Self {
        Self {
            candidate_kind,
            trusted_self_provenance: false,
            source_digest: String::new(),
            lifecycle_state: String::new(),
            affected_scope: String::new(),
            marker_text: String::new(),
            unexpected_tool_permission: false,
            benchmark_or_verifier_damage: false,
        }
    }

    pub fn trusted_self_provenance(mut self, trusted: bool) -> Self {
        self.trusted_self_provenance = trusted;
        self
    }

    pub fn source_digest(mut self, source_digest: impl Into<String>) -> Self {
        self.source_digest = source_digest.into();
        self
    }

    pub fn lifecycle_state(mut self, lifecycle_state: impl Into<String>) -> Self {
        self.lifecycle_state = lifecycle_state.into();
        self
    }

    pub fn affected_scope(mut self, affected_scope: impl Into<String>) -> Self {
        self.affected_scope = affected_scope.into();
        self
    }

    pub fn marker_text(mut self, marker_text: impl Into<String>) -> Self {
        self.marker_text = marker_text.into();
        self
    }

    pub fn unexpected_tool_permission(mut self, unexpected: bool) -> Self {
        self.unexpected_tool_permission = unexpected;
        self
    }

    pub fn benchmark_or_verifier_damage(mut self, damaged: bool) -> Self {
        self.benchmark_or_verifier_damage = damaged;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DangerSignalReview {
    pub candidate_kind: &'static str,
    pub decision: DangerSignalDecision,
    pub reason_codes: Vec<String>,
    pub evidence_digest: String,
    pub activation_allowed: bool,
    pub trace_safe: bool,
}

impl DangerSignalReview {
    pub fn summary_line(&self) -> String {
        format!(
            "danger_signal candidate={} decision={} reasons={} evidence_digest={} activation_allowed={} trace_safe={}",
            self.candidate_kind,
            self.decision.as_str(),
            self.reason_codes.join("|"),
            self.evidence_digest,
            self.activation_allowed,
            self.trace_safe
        )
    }
}

pub fn review_danger_signals(input: DangerSignalInput) -> DangerSignalReview {
    let mut reason_codes = Vec::new();
    let lifecycle = input.lifecycle_state.trim().to_ascii_lowercase();

    if !input.trusted_self_provenance {
        reason_codes.push("missing_trusted_self_provenance".to_owned());
    }
    if !source_digest_trusted(&input.source_digest) {
        reason_codes.push("missing_or_unknown_source_digest".to_owned());
    }
    if matches!(
        lifecycle.as_str(),
        "retired_blocked" | "tombstone_preview" | "quarantined"
    ) {
        reason_codes.push("retired_version_source".to_owned());
    }
    if matches!(
        lifecycle.as_str(),
        "recycle_candidate" | "repaired_candidate"
    ) {
        reason_codes.push("readmission_candidate_hold".to_owned());
    }
    if input
        .affected_scope
        .to_ascii_lowercase()
        .contains("cross_tenant")
    {
        reason_codes.push("cross_tenant_scope_mismatch".to_owned());
    }
    if input.unexpected_tool_permission {
        reason_codes.push("unexpected_tool_permission".to_owned());
    }
    if input.benchmark_or_verifier_damage {
        reason_codes.push("benchmark_or_verifier_damage".to_owned());
    }
    reason_codes.extend(
        privacy_redaction_reason_codes(&input.marker_text)
            .into_iter()
            .map(|code| format!("raw_payload_marker:{code}")),
    );
    if contains_prompt_injection_marker(&input.marker_text) {
        reason_codes.push("prompt_injection_marker".to_owned());
    }
    dedup_stable(&mut reason_codes);

    let decision = if has_any(
        &reason_codes,
        &["raw_payload_marker:", "prompt_injection_marker"],
    ) {
        DangerSignalDecision::RejectDangerSignal
    } else if has_exact(&reason_codes, "benchmark_or_verifier_damage") {
        DangerSignalDecision::RequireOperatorReview
    } else if has_exact(&reason_codes, "retired_version_source")
        || has_exact(&reason_codes, "cross_tenant_scope_mismatch")
    {
        DangerSignalDecision::QuarantineNonSelf
    } else if has_exact(&reason_codes, "readmission_candidate_hold") {
        DangerSignalDecision::HoldForProvenance
    } else if has_exact(&reason_codes, "missing_trusted_self_provenance")
        || has_exact(&reason_codes, "missing_or_unknown_source_digest")
    {
        DangerSignalDecision::HoldForProvenance
    } else if input.candidate_kind == "external_reference" {
        DangerSignalDecision::ObserveOnly
    } else {
        DangerSignalDecision::SelfTrusted
    };

    DangerSignalReview {
        candidate_kind: input.candidate_kind,
        decision,
        evidence_digest: stable_redaction_digest([
            "danger-signal",
            input.candidate_kind,
            decision.as_str(),
            input.source_digest.as_str(),
            lifecycle.as_str(),
            input.affected_scope.as_str(),
            reason_codes.join("|").as_str(),
        ]),
        activation_allowed: decision.activation_allowed(),
        trace_safe: true,
        reason_codes,
    }
}

fn source_digest_trusted(source_digest: &str) -> bool {
    let value = source_digest.trim();
    value.starts_with("sha256:")
        || value.starts_with("redaction-digest:")
        || value.starts_with("fnv64:")
}

fn contains_prompt_injection_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "ignore previous",
        "system prompt",
        "developer message",
        "jailbreak",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn has_exact(reason_codes: &[String], needle: &str) -> bool {
    reason_codes.iter().any(|code| code == needle)
}

fn has_any(reason_codes: &[String], prefixes: &[&str]) -> bool {
    reason_codes
        .iter()
        .any(|code| prefixes.iter().any(|prefix| code.starts_with(prefix)))
}

fn dedup_stable(values: &mut Vec<String>) {
    let mut deduped = Vec::new();
    for value in values.drain(..) {
        if !deduped.contains(&value) {
            deduped.push(value);
        }
    }
    *values = deduped;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn danger_signal_covers_core_decisions_without_raw_payloads() {
        let trusted = review_danger_signals(
            DangerSignalInput::new("runtime_asset")
                .trusted_self_provenance(true)
                .source_digest("sha256:self-runtime")
                .lifecycle_state("active"),
        );
        assert_eq!(trusted.decision, DangerSignalDecision::SelfTrusted);
        assert!(trusted.activation_allowed);

        let unknown = review_danger_signals(DangerSignalInput::new("memory_candidate"));
        assert_eq!(unknown.decision, DangerSignalDecision::HoldForProvenance);
        assert!(!unknown.activation_allowed);

        let retired = review_danger_signals(
            DangerSignalInput::new("runtime_asset")
                .trusted_self_provenance(true)
                .source_digest("sha256:retired")
                .lifecycle_state("retired_blocked"),
        );
        assert_eq!(retired.decision, DangerSignalDecision::QuarantineNonSelf);

        let polluted = review_danger_signals(
            DangerSignalInput::new("handoff_packet")
                .trusted_self_provenance(true)
                .source_digest("redaction-digest:abc")
                .marker_text("private chat raw_prompt with ignore previous instruction"),
        );
        assert_eq!(polluted.decision, DangerSignalDecision::RejectDangerSignal);
        assert!(
            polluted
                .reason_codes
                .contains(&"prompt_injection_marker".to_owned())
        );
        assert!(!polluted.summary_line().contains("private chat"));

        let cross_tenant = review_danger_signals(
            DangerSignalInput::new("tool_blueprint")
                .trusted_self_provenance(true)
                .source_digest("sha256:tool")
                .affected_scope("cross_tenant:alpha->beta"),
        );
        assert_eq!(
            cross_tenant.decision,
            DangerSignalDecision::QuarantineNonSelf
        );

        let damaged = review_danger_signals(
            DangerSignalInput::new("genome_mutation_candidate")
                .trusted_self_provenance(true)
                .source_digest("sha256:genome")
                .benchmark_or_verifier_damage(true),
        );
        assert_eq!(
            damaged.decision,
            DangerSignalDecision::RequireOperatorReview
        );
    }
}
