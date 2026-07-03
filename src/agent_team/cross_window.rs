use std::collections::{BTreeMap, BTreeSet};

use crate::danger_signal::{DangerSignalInput, DangerSignalReview, review_danger_signals};
use crate::development_pollution::{
    DefenseSpacer, DefenseSpacerActivationGate, DefenseSpacerCandidate, DevelopmentPollutionEvent,
    classify_development_pollution_event, development_evidence_payload_reason,
    gate_defense_spacer_activation,
};
use crate::tenant_scope::TenantScope;

use super::handoff::{
    AgentHandoffAggregationReport, AgentHandoffContext, AgentHandoffInput, AgentHandoffSanitizer,
};
use super::types::AgentRole;
use super::util::{compact, stable_hash};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CrossWindowConflictClass {
    DuplicatePacket,
    FileOverlap,
    LaneOwnerCollision,
    StalePacket,
    PollutedPayload,
    BudgetExceeded,
    DangerSignal,
}

impl CrossWindowConflictClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DuplicatePacket => "duplicate_packet",
            Self::FileOverlap => "file_overlap",
            Self::LaneOwnerCollision => "lane_owner_collision",
            Self::StalePacket => "stale_packet",
            Self::PollutedPayload => "polluted_payload",
            Self::BudgetExceeded => "budget_exceeded",
            Self::DangerSignal => "danger_signal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossWindowPacketDecision {
    Accepted,
    Duplicate,
    Quarantined,
}

impl CrossWindowPacketDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Duplicate => "duplicate",
            Self::Quarantined => "quarantined",
        }
    }

    pub fn control_lifecycle_state(self) -> &'static str {
        match self {
            Self::Accepted => "active",
            Self::Duplicate => "recycle_candidate",
            Self::Quarantined => "quarantined",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CrossWindowBudget {
    pub token_budget: u64,
    pub token_spent: u64,
    pub step_budget: u64,
    pub step_spent: u64,
}

impl CrossWindowBudget {
    pub fn new(token_budget: u64, token_spent: u64, step_budget: u64, step_spent: u64) -> Self {
        Self {
            token_budget,
            token_spent,
            step_budget,
            step_spent,
        }
    }

    pub fn token_remaining(self) -> u64 {
        self.token_budget.saturating_sub(self.token_spent)
    }

    pub fn step_remaining(self) -> u64 {
        self.step_budget.saturating_sub(self.step_spent)
    }

    pub fn exhausted(self) -> bool {
        self.token_spent > self.token_budget || self.step_spent > self.step_budget
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossWindowExperiencePacket {
    pub packet_id: String,
    pub lane_id: String,
    pub source_window_id: String,
    pub owner_role: AgentRole,
    pub scope: TenantScope,
    pub freshness_epoch: u64,
    pub summary: String,
    pub files_touched: Vec<String>,
    pub tests_run: Vec<String>,
    pub decisions: Vec<String>,
    pub blockers: Vec<String>,
    pub risks: Vec<String>,
    pub next_handoff: String,
    pub next_recommended_issue: String,
    pub evidence_ids: Vec<String>,
    pub budget: CrossWindowBudget,
    pub packet_digest: String,
    pub provenance_digest: String,
    pub raw_payload_present: bool,
    pub private_payload_present: bool,
    pub redactions: usize,
}

impl CrossWindowExperiencePacket {
    pub fn new(
        source_window_id: impl AsRef<str>,
        lane_id: impl AsRef<str>,
        scope: TenantScope,
        owner_role: AgentRole,
        summary: impl AsRef<str>,
    ) -> Self {
        let mut redactions = 0;
        let source_window_id = sanitize_identifier(source_window_id.as_ref(), "window");
        let lane_id = sanitize_identifier(lane_id.as_ref(), "lane");
        let (summary, summary_redactions, summary_has_payload_marker) =
            sanitize_public_text(summary.as_ref(), 180);
        redactions += summary_redactions;
        let packet_id = format!(
            "window-packet-{:016x}",
            stable_hash(&format!(
                "{}:{}:{}:{}",
                source_window_id,
                lane_id,
                scope.scope_digest(),
                summary
            ))
        );
        let mut packet = Self {
            packet_id,
            lane_id,
            source_window_id,
            owner_role,
            scope,
            freshness_epoch: 0,
            summary,
            files_touched: Vec::new(),
            tests_run: Vec::new(),
            decisions: Vec::new(),
            blockers: Vec::new(),
            risks: Vec::new(),
            next_handoff: "none".to_owned(),
            next_recommended_issue: String::new(),
            evidence_ids: Vec::new(),
            budget: CrossWindowBudget::default(),
            packet_digest: String::new(),
            provenance_digest: String::new(),
            raw_payload_present: summary_has_payload_marker,
            private_payload_present: false,
            redactions,
        };
        packet.refresh_digests();
        packet
    }

    pub fn with_freshness_epoch(mut self, freshness_epoch: u64) -> Self {
        self.freshness_epoch = freshness_epoch;
        self.refresh_digests();
        self
    }

    pub fn with_file_touched(mut self, file: impl AsRef<str>) -> Self {
        push_unique_string(&mut self.files_touched, sanitize_path(file.as_ref()));
        self.refresh_digests();
        self
    }

    pub fn with_test_run(mut self, test: impl AsRef<str>) -> Self {
        let (value, redactions, has_payload_marker) = sanitize_public_text(test.as_ref(), 120);
        self.redactions = self.redactions.saturating_add(redactions);
        self.raw_payload_present |= has_payload_marker;
        push_unique_string(&mut self.tests_run, value);
        self.refresh_digests();
        self
    }

    pub fn with_decision(mut self, decision: impl AsRef<str>) -> Self {
        let (value, redactions, has_payload_marker) = sanitize_public_text(decision.as_ref(), 120);
        self.redactions = self.redactions.saturating_add(redactions);
        self.raw_payload_present |= has_payload_marker;
        push_unique_string(&mut self.decisions, value);
        self.refresh_digests();
        self
    }

    pub fn with_blocker(mut self, blocker: impl AsRef<str>) -> Self {
        let (value, redactions, has_payload_marker) = sanitize_public_text(blocker.as_ref(), 120);
        self.redactions = self.redactions.saturating_add(redactions);
        self.raw_payload_present |= has_payload_marker;
        push_unique_string(&mut self.blockers, value);
        self.refresh_digests();
        self
    }

    pub fn with_risk(mut self, risk: impl AsRef<str>) -> Self {
        let (value, redactions, has_payload_marker) = sanitize_public_text(risk.as_ref(), 120);
        self.redactions = self.redactions.saturating_add(redactions);
        self.raw_payload_present |= has_payload_marker;
        push_unique_string(&mut self.risks, value);
        self.refresh_digests();
        self
    }

    pub fn with_next_handoff(mut self, next_handoff: impl AsRef<str>) -> Self {
        let (value, redactions, has_payload_marker) =
            sanitize_public_text(next_handoff.as_ref(), 160);
        self.redactions = self.redactions.saturating_add(redactions);
        self.raw_payload_present |= has_payload_marker;
        self.next_handoff = value;
        self.refresh_digests();
        self
    }

    pub fn with_next_issue(mut self, next_issue: impl AsRef<str>) -> Self {
        self.next_recommended_issue = canonical_issue_ref(next_issue.as_ref());
        self.refresh_digests();
        self
    }

    pub fn with_evidence_id(mut self, evidence_id: impl AsRef<str>) -> Self {
        push_unique_string(
            &mut self.evidence_ids,
            redacted_evidence_id(evidence_id.as_ref()),
        );
        self.refresh_digests();
        self
    }

    pub fn with_budget(mut self, budget: CrossWindowBudget) -> Self {
        self.budget = budget;
        self.refresh_digests();
        self
    }

    pub fn with_raw_payload_present(mut self, present: bool) -> Self {
        self.raw_payload_present = present;
        self.refresh_digests();
        self
    }

    pub fn with_private_payload_present(mut self, present: bool) -> Self {
        self.private_payload_present = present;
        self.refresh_digests();
        self
    }

    fn to_handoff_input(&self) -> AgentHandoffInput {
        let mut handoff =
            AgentHandoffInput::new(&self.source_window_id, self.owner_role, &self.summary)
                .with_raw_payload_present(self.raw_payload_present)
                .with_private_payload_present(self.private_payload_present);
        for file in &self.files_touched {
            handoff = handoff.with_touched_file(file);
        }
        for test in &self.tests_run {
            handoff = handoff.with_validation(test);
        }
        for blocker in &self.blockers {
            handoff = handoff.with_unresolved_risk(blocker);
        }
        if !self.next_recommended_issue.is_empty() {
            handoff = handoff.with_issue(&self.next_recommended_issue);
        }
        handoff
    }

    fn refresh_digests(&mut self) {
        self.provenance_digest = format!(
            "provenance:{:016x}",
            stable_hash(&format!(
                "{}:{}:{}:{}",
                self.source_window_id,
                self.lane_id,
                self.scope.scope_digest(),
                self.freshness_epoch
            ))
        );
        self.packet_digest = format!(
            "packet:{:016x}",
            stable_hash(&format!(
                "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
                self.packet_id,
                self.source_window_id,
                self.lane_id,
                self.scope.scope_digest(),
                self.freshness_epoch,
                self.summary,
                self.files_touched.join(","),
                self.tests_run.join(","),
                self.decisions.join(","),
                self.blockers.join(","),
                self.risks.join(","),
                self.next_handoff,
                self.next_recommended_issue,
                self.evidence_ids.join(",")
            ))
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossWindowPacketReview {
    pub packet_id: String,
    pub source_window_id: String,
    pub lane_id: String,
    pub packet_digest: String,
    pub decision: CrossWindowPacketDecision,
    pub conflict_classes: Vec<CrossWindowConflictClass>,
    pub blocked_reasons: Vec<String>,
    pub defense_spacer_activation_gate: Option<DefenseSpacerActivationGate>,
    pub accepted: bool,
}

impl CrossWindowPacketReview {
    pub fn control_lifecycle_state(&self) -> &'static str {
        self.decision.control_lifecycle_state()
    }

    pub fn summary_line(&self) -> String {
        let conflicts = self
            .conflict_classes
            .iter()
            .map(|class| class.as_str())
            .collect::<Vec<_>>()
            .join("+");
        format!(
            "cross_window_packet packet={} source={} lane={} decision={} lifecycle={} accepted={} conflicts={} blocked={} defense_spacer_allowed={}",
            self.packet_id,
            self.source_window_id,
            self.lane_id,
            self.decision.as_str(),
            self.control_lifecycle_state(),
            self.accepted,
            conflicts,
            self.blocked_reasons.len(),
            self.defense_spacer_activation_gate
                .as_ref()
                .map_or(true, |gate| gate.allowed)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossWindowBudgetReport {
    pub windows: usize,
    pub lanes: usize,
    pub accepted_packets: usize,
    pub quarantined_packets: usize,
    pub duplicate_packets: usize,
    pub token_budget: u64,
    pub token_spent: u64,
    pub token_remaining: u64,
    pub step_budget: u64,
    pub step_spent: u64,
    pub step_remaining: u64,
    pub work_done: Vec<String>,
    pub tests_run: Vec<String>,
    pub unresolved_blockers: Vec<String>,
    pub next_recommended_issue: String,
}

impl CrossWindowBudgetReport {
    pub fn summary_line(&self) -> String {
        format!(
            "cross_window_budget windows={} lanes={} accepted={} quarantined={} duplicate={} tokens={}/{} remaining={} steps={}/{} remaining={} work_done={} tests={} blockers={} next={}",
            self.windows,
            self.lanes,
            self.accepted_packets,
            self.quarantined_packets,
            self.duplicate_packets,
            self.token_spent,
            self.token_budget,
            self.token_remaining,
            self.step_spent,
            self.step_budget,
            self.step_remaining,
            self.work_done.len(),
            self.tests_run.len(),
            self.unresolved_blockers.len(),
            self.next_recommended_issue
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossWindowExchangeReport {
    pub preview_only: bool,
    pub read_only: bool,
    pub report_only: bool,
    pub total_packets: usize,
    pub accepted_packets: usize,
    pub duplicate_packets: usize,
    pub stale_packets: usize,
    pub quarantined_packets: usize,
    pub merged_summaries: Vec<String>,
    pub files_touched: Vec<String>,
    pub tests_run: Vec<String>,
    pub decisions: Vec<String>,
    pub blockers: Vec<String>,
    pub risks: Vec<String>,
    pub evidence_digests: Vec<String>,
    pub reviews: Vec<CrossWindowPacketReview>,
    pub budget_report: CrossWindowBudgetReport,
    pub handoff_report: AgentHandoffAggregationReport,
    pub can_feed_agent_team: bool,
    pub can_promote_memory: bool,
    pub can_bypass_approval: bool,
}

impl CrossWindowExchangeReport {
    pub fn summary_line(&self) -> String {
        format!(
            "cross_window_exchange packets={} accepted={} duplicate={} stale={} quarantined={} files={} tests={} blockers={} risks={} digests={} can_feed_agent_team={} can_promote_memory={} can_bypass_approval={} preview_only={}",
            self.total_packets,
            self.accepted_packets,
            self.duplicate_packets,
            self.stale_packets,
            self.quarantined_packets,
            self.files_touched.len(),
            self.tests_run.len(),
            self.blockers.len(),
            self.risks.len(),
            self.evidence_digests.len(),
            self.can_feed_agent_team,
            self.can_promote_memory,
            self.can_bypass_approval,
            self.preview_only
        )
    }
}

#[derive(Debug, Clone)]
pub struct CrossWindowExchangeContext {
    pub current_epoch: u64,
    pub stale_after_epochs: u64,
    pub expected_scope: Option<TenantScope>,
    pub handoff_context: AgentHandoffContext,
}

impl Default for CrossWindowExchangeContext {
    fn default() -> Self {
        Self {
            current_epoch: 0,
            stale_after_epochs: 2,
            expected_scope: None,
            handoff_context: AgentHandoffContext::default(),
        }
    }
}

impl CrossWindowExchangeContext {
    pub fn new(current_epoch: u64) -> Self {
        Self {
            current_epoch,
            ..Self::default()
        }
    }

    pub fn with_stale_after_epochs(mut self, stale_after_epochs: u64) -> Self {
        self.stale_after_epochs = stale_after_epochs.max(1);
        self
    }

    pub fn with_expected_scope(mut self, scope: TenantScope) -> Self {
        self.expected_scope = Some(scope);
        self
    }

    pub fn with_handoff_context(mut self, handoff_context: AgentHandoffContext) -> Self {
        self.handoff_context = handoff_context;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct CrossWindowExchangeAggregator;

impl CrossWindowExchangeAggregator {
    pub fn new() -> Self {
        Self
    }

    pub fn aggregate(
        &self,
        context: &CrossWindowExchangeContext,
        packets: &[CrossWindowExperiencePacket],
    ) -> CrossWindowExchangeReport {
        let mut seen_packet_digests = BTreeSet::new();
        let mut seen_packet_ids = BTreeSet::new();
        let mut lane_owner = BTreeMap::<String, String>::new();
        let mut file_owner = BTreeMap::<String, String>::new();
        let mut accepted = Vec::<CrossWindowExperiencePacket>::new();
        let mut reviews = Vec::new();
        let mut duplicate_packets = 0usize;
        let mut stale_packets = 0usize;
        let mut quarantined_packets = 0usize;

        for packet in packets {
            let mut conflict_classes = BTreeSet::new();
            let mut blocked_reasons = Vec::new();
            let duplicate_digest = !seen_packet_digests.insert(packet.packet_digest.clone());
            let duplicate_id = !seen_packet_ids.insert(packet.packet_id.clone());
            if duplicate_digest || duplicate_id {
                conflict_classes.insert(CrossWindowConflictClass::DuplicatePacket);
                blocked_reasons.push("cross_window_duplicate_packet".to_owned());
            }

            if context.current_epoch.saturating_sub(packet.freshness_epoch)
                > context.stale_after_epochs
            {
                conflict_classes.insert(CrossWindowConflictClass::StalePacket);
                blocked_reasons.push("cross_window_stale_packet".to_owned());
            }
            if packet.raw_payload_present || packet.private_payload_present || packet.redactions > 0
            {
                conflict_classes.insert(CrossWindowConflictClass::PollutedPayload);
                blocked_reasons.push("cross_window_payload_pollution_blocked".to_owned());
            }
            if packet.budget.exhausted() {
                conflict_classes.insert(CrossWindowConflictClass::BudgetExceeded);
                blocked_reasons.push("cross_window_budget_exceeded".to_owned());
            }
            if let Some(expected_scope) = &context.expected_scope
                && &packet.scope != expected_scope
            {
                conflict_classes.insert(CrossWindowConflictClass::LaneOwnerCollision);
                blocked_reasons.push("cross_window_scope_mismatch".to_owned());
            }
            let pollution_reason = packet_development_pollution_reason(packet);
            if pollution_reason == "poisoned_handoff" {
                conflict_classes.insert(CrossWindowConflictClass::PollutedPayload);
                push_unique_string(
                    &mut blocked_reasons,
                    "cross_window_poisoned_handoff_packet".to_owned(),
                );
            }
            let danger_review = packet_danger_review(context, packet);
            if !danger_review.activation_allowed {
                conflict_classes.insert(CrossWindowConflictClass::DangerSignal);
                if danger_review
                    .reason_codes
                    .iter()
                    .any(|reason| reason == "cross_tenant_scope_mismatch")
                {
                    conflict_classes.insert(CrossWindowConflictClass::LaneOwnerCollision);
                }
                if danger_review.reason_codes.iter().any(|reason| {
                    reason.starts_with("raw_payload_marker:") || reason == "prompt_injection_marker"
                }) {
                    conflict_classes.insert(CrossWindowConflictClass::PollutedPayload);
                }
                push_unique_string(
                    &mut blocked_reasons,
                    format!("danger_signal_{}", danger_review.decision.as_str()),
                );
                for reason in &danger_review.reason_codes {
                    push_unique_string(
                        &mut blocked_reasons,
                        format!("danger_signal_reason_{reason}"),
                    );
                }
            }
            let defense_spacer_activation_gate =
                packet_defense_spacer_activation_gate(packet, &conflict_classes, pollution_reason);
            if let Some(gate) = &defense_spacer_activation_gate
                && !gate.allowed
            {
                conflict_classes.insert(CrossWindowConflictClass::PollutedPayload);
                push_unique_string(&mut blocked_reasons, gate.reason.clone());
            }

            if let Some(owner) = lane_owner.get(&packet.lane_id) {
                if owner != &packet.source_window_id {
                    conflict_classes.insert(CrossWindowConflictClass::LaneOwnerCollision);
                    blocked_reasons.push(format!(
                        "cross_window_lane_owner_collision:{} first_owner={}",
                        packet.lane_id, owner
                    ));
                }
            }
            for file in &packet.files_touched {
                if let Some(owner) = file_owner.get(file) {
                    if owner != &packet.source_window_id {
                        conflict_classes.insert(CrossWindowConflictClass::FileOverlap);
                        blocked_reasons.push(format!(
                            "cross_window_file_overlap:{} first_owner={}",
                            file, owner
                        ));
                    }
                }
            }

            let decision = if conflict_classes.contains(&CrossWindowConflictClass::DuplicatePacket)
            {
                duplicate_packets += 1;
                CrossWindowPacketDecision::Duplicate
            } else if conflict_classes.is_empty() {
                lane_owner.insert(packet.lane_id.clone(), packet.source_window_id.clone());
                for file in &packet.files_touched {
                    file_owner.insert(file.clone(), packet.source_window_id.clone());
                }
                accepted.push(packet.clone());
                CrossWindowPacketDecision::Accepted
            } else {
                quarantined_packets += 1;
                if conflict_classes.contains(&CrossWindowConflictClass::StalePacket) {
                    stale_packets += 1;
                }
                CrossWindowPacketDecision::Quarantined
            };

            reviews.push(CrossWindowPacketReview {
                packet_id: packet.packet_id.clone(),
                source_window_id: packet.source_window_id.clone(),
                lane_id: packet.lane_id.clone(),
                packet_digest: packet.packet_digest.clone(),
                accepted: decision == CrossWindowPacketDecision::Accepted,
                decision,
                conflict_classes: conflict_classes.into_iter().collect(),
                blocked_reasons,
                defense_spacer_activation_gate,
            });
        }

        let handoff_inputs = packets
            .iter()
            .map(CrossWindowExperiencePacket::to_handoff_input)
            .collect::<Vec<_>>();
        let handoff_report =
            AgentHandoffSanitizer::new().sanitize(&context.handoff_context, &handoff_inputs);
        let budget_report = build_budget_report(
            &accepted,
            packets.len(),
            duplicate_packets,
            quarantined_packets,
        );
        let can_feed_agent_team = !accepted.is_empty()
            && quarantined_packets == 0
            && duplicate_packets == 0
            && handoff_report.quarantined_handoffs == 0;

        CrossWindowExchangeReport {
            preview_only: true,
            read_only: true,
            report_only: true,
            total_packets: packets.len(),
            accepted_packets: accepted.len(),
            duplicate_packets,
            stale_packets,
            quarantined_packets,
            merged_summaries: accepted
                .iter()
                .map(|packet| packet.summary.clone())
                .collect(),
            files_touched: unique_flatten(&accepted, |packet| &packet.files_touched),
            tests_run: unique_flatten(&accepted, |packet| &packet.tests_run),
            decisions: unique_flatten(&accepted, |packet| &packet.decisions),
            blockers: unique_flatten_all(packets, |packet| &packet.blockers),
            risks: unique_flatten_all(packets, |packet| &packet.risks),
            evidence_digests: unique_packet_digests(&accepted),
            reviews,
            budget_report,
            handoff_report,
            can_feed_agent_team,
            can_promote_memory: false,
            can_bypass_approval: false,
        }
    }
}

fn packet_defense_spacer_activation_gate(
    packet: &CrossWindowExperiencePacket,
    conflict_classes: &BTreeSet<CrossWindowConflictClass>,
    reason_code: &'static str,
) -> Option<DefenseSpacerActivationGate> {
    if !conflict_classes.contains(&CrossWindowConflictClass::PollutedPayload) {
        return None;
    }

    let payload_digest = format!(
        "handoff_packet:{}:{}:{}",
        packet.packet_digest, packet.provenance_digest, packet.redactions
    );
    let finding = classify_development_pollution_event(&DevelopmentPollutionEvent::new(
        &packet.packet_id,
        "cross_window_handoff_packet",
        payload_digest,
        reason_code,
    ));
    let spacer = DefenseSpacer::from_finding(
        &finding,
        "cross_window_handoff_activation",
        "runtime-write",
        "clean_payload_and_operator_approval",
    );
    let candidate =
        DefenseSpacerCandidate::from_finding(&finding, "cross_window_handoff_activation");
    Some(gate_defense_spacer_activation(&[spacer], &candidate))
}

fn packet_development_pollution_reason(packet: &CrossWindowExperiencePacket) -> &'static str {
    development_evidence_payload_reason(&packet_marker_text(packet))
}

fn packet_danger_review(
    context: &CrossWindowExchangeContext,
    packet: &CrossWindowExperiencePacket,
) -> DangerSignalReview {
    let scope_mismatch = context
        .expected_scope
        .as_ref()
        .is_some_and(|expected| expected != &packet.scope);
    let source_digest =
        if packet.packet_digest.trim().is_empty() || packet.provenance_digest.trim().is_empty() {
            String::new()
        } else {
            format!(
                "fnv64:{:016x}",
                stable_hash(&format!(
                    "{}:{}",
                    packet.packet_digest, packet.provenance_digest
                ))
            )
        };
    review_danger_signals(
        DangerSignalInput::new("handoff_packet")
            .trusted_self_provenance(
                !source_digest.is_empty()
                    && !scope_mismatch
                    && !packet.raw_payload_present
                    && !packet.private_payload_present
                    && packet.redactions == 0,
            )
            .source_digest(source_digest)
            .affected_scope(if scope_mismatch {
                "cross_tenant_scope_mismatch".to_owned()
            } else {
                packet.scope.scope_digest()
            })
            .marker_text(packet_marker_text(packet)),
    )
}

fn packet_marker_text(packet: &CrossWindowExperiencePacket) -> String {
    let mut marker_parts = vec![
        packet.summary.clone(),
        packet.tests_run.join(" "),
        packet.decisions.join(" "),
        packet.blockers.join(" "),
        packet.risks.join(" "),
        packet.next_handoff.clone(),
    ];
    if packet.raw_payload_present {
        marker_parts.push("raw_prompt marker".to_owned());
    }
    if packet.private_payload_present || packet.redactions > 0 {
        marker_parts.push("private chat marker".to_owned());
    }
    marker_parts.join(" ")
}

fn build_budget_report(
    accepted: &[CrossWindowExperiencePacket],
    total_packets: usize,
    duplicate_packets: usize,
    quarantined_packets: usize,
) -> CrossWindowBudgetReport {
    let windows = accepted
        .iter()
        .map(|packet| packet.source_window_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let lanes = accepted
        .iter()
        .map(|packet| packet.lane_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let mut token_budget = 0u64;
    let mut token_spent = 0u64;
    let mut step_budget = 0u64;
    let mut step_spent = 0u64;
    for packet in accepted {
        token_budget = token_budget.saturating_add(packet.budget.token_budget);
        token_spent = token_spent.saturating_add(packet.budget.token_spent);
        step_budget = step_budget.saturating_add(packet.budget.step_budget);
        step_spent = step_spent.saturating_add(packet.budget.step_spent);
    }
    let next_recommended_issue = accepted
        .iter()
        .find_map(|packet| {
            (!packet.next_recommended_issue.trim().is_empty())
                .then(|| packet.next_recommended_issue.clone())
        })
        .unwrap_or_else(|| {
            if total_packets == 0 {
                "none".to_owned()
            } else {
                "repair-cross-window-exchange".to_owned()
            }
        });

    CrossWindowBudgetReport {
        windows,
        lanes,
        accepted_packets: accepted.len(),
        quarantined_packets,
        duplicate_packets,
        token_budget,
        token_spent,
        token_remaining: token_budget.saturating_sub(token_spent),
        step_budget,
        step_spent,
        step_remaining: step_budget.saturating_sub(step_spent),
        work_done: accepted
            .iter()
            .map(|packet| packet.summary.clone())
            .collect(),
        tests_run: unique_flatten(accepted, |packet| &packet.tests_run),
        unresolved_blockers: unique_flatten_all(accepted, |packet| &packet.blockers),
        next_recommended_issue,
    }
}

fn unique_flatten(
    packets: &[CrossWindowExperiencePacket],
    field: fn(&CrossWindowExperiencePacket) -> &Vec<String>,
) -> Vec<String> {
    unique_flatten_all(packets, field)
}

fn unique_flatten_all(
    packets: &[CrossWindowExperiencePacket],
    field: fn(&CrossWindowExperiencePacket) -> &Vec<String>,
) -> Vec<String> {
    packets
        .iter()
        .flat_map(|packet| field(packet).iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn unique_packet_digests(packets: &[CrossWindowExperiencePacket]) -> Vec<String> {
    packets
        .iter()
        .flat_map(|packet| {
            std::iter::once(packet.packet_digest.clone())
                .chain(std::iter::once(packet.provenance_digest.clone()))
                .chain(packet.evidence_ids.iter().cloned())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn sanitize_identifier(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '#') {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(120)
        .collect::<String>();
    if sanitized.is_empty() {
        fallback.to_owned()
    } else {
        sanitized
    }
}

fn sanitize_path(value: &str) -> String {
    sanitize_identifier(&value.replace('\\', "/"), "path")
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

fn redacted_evidence_id(value: &str) -> String {
    if let Some((prefix, _)) = value.split_once(':') {
        format!(
            "{}:evidence:{:016x}",
            sanitize_identifier(prefix, "evidence"),
            stable_hash(value)
        )
    } else {
        format!("evidence:{:016x}", stable_hash(value))
    }
}

fn canonical_issue_ref(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        String::new()
    } else if value.starts_with('#') {
        sanitize_identifier(value, "issue")
    } else if value.chars().all(|ch| ch.is_ascii_digit()) {
        format!("#{value}")
    } else {
        sanitize_identifier(value, "issue")
    }
}

fn push_unique_string(target: &mut Vec<String>, value: String) {
    if !value.trim().is_empty() && !target.contains(&value) {
        target.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_non_conflicting_packets_and_reports_budget() {
        let scope = scope();
        let packets = vec![
            packet("window-a", "runtime", "implemented session API", 10)
                .with_file_touched("src/session_state.rs")
                .with_test_run("cargo test -q session_state")
                .with_decision("closed #69 after CI passed")
                .with_next_issue("#70")
                .with_budget(CrossWindowBudget::new(10_000, 2_000, 10, 3)),
            packet("window-b", "docs", "documented runtime session handoff", 10)
                .with_file_touched("docs/architecture/runtime-session-state-api.md")
                .with_test_run("cargo fmt --all -- --check")
                .with_decision("docs ready for service API")
                .with_next_issue("#70")
                .with_budget(CrossWindowBudget::new(8_000, 1_000, 8, 2)),
        ];

        let report = aggregate(&scope, &packets);

        assert_eq!(report.accepted_packets, 2);
        assert_eq!(report.quarantined_packets, 0);
        assert_eq!(report.duplicate_packets, 0);
        assert!(
            report
                .reviews
                .iter()
                .all(|review| review.control_lifecycle_state() == "active")
        );
        assert!(report.can_feed_agent_team);
        assert!(!report.can_promote_memory);
        assert!(!report.can_bypass_approval);
        assert_eq!(report.budget_report.windows, 2);
        assert_eq!(report.budget_report.lanes, 2);
        assert_eq!(report.budget_report.token_spent, 3_000);
        assert_eq!(report.budget_report.step_remaining, 13);
        assert_eq!(report.tests_run.len(), 2);
        assert!(report.summary_line().contains("accepted=2"));
    }

    #[test]
    fn quarantines_file_overlap_conflict() {
        let scope = scope();
        let packets = vec![
            packet("window-a", "runtime", "owns session state", 10)
                .with_file_touched("src/session_state.rs")
                .with_test_run("cargo test -q session_state")
                .with_budget(CrossWindowBudget::new(10, 1, 4, 1)),
            packet("window-b", "agent", "also touched session state", 10)
                .with_file_touched("src\\session_state.rs")
                .with_test_run("cargo test -q session_state")
                .with_budget(CrossWindowBudget::new(10, 1, 4, 1)),
        ];

        let report = aggregate(&scope, &packets);

        assert_eq!(report.accepted_packets, 1);
        assert_eq!(report.quarantined_packets, 1);
        assert!(
            report.reviews[1]
                .conflict_classes
                .contains(&CrossWindowConflictClass::FileOverlap)
        );
        assert!(
            report.reviews[1]
                .blocked_reasons
                .iter()
                .any(|reason| reason.contains("cross_window_file_overlap"))
        );
        assert!(!report.can_feed_agent_team);
    }

    #[test]
    fn danger_signal_quarantines_cross_scope_packet() {
        let scope = scope();
        let foreign_scope = TenantScope::new("tenant-b", "workspace", "cross-window");
        let packets = vec![
            CrossWindowExperiencePacket::new(
                "window-b",
                "runtime",
                foreign_scope,
                AgentRole::Coder,
                "implemented runtime handoff in another tenant",
            )
            .with_freshness_epoch(10)
            .with_test_run("cargo test -q runtime_handoff")
            .with_budget(CrossWindowBudget::new(10, 1, 4, 1)),
        ];

        let report = aggregate(&scope, &packets);

        assert_eq!(report.accepted_packets, 0);
        assert_eq!(report.quarantined_packets, 1);
        assert!(
            report.reviews[0]
                .conflict_classes
                .contains(&CrossWindowConflictClass::DangerSignal)
        );
        assert!(
            report.reviews[0]
                .blocked_reasons
                .contains(&"danger_signal_quarantine_non_self".to_owned()),
            "{:?}",
            report.reviews[0].blocked_reasons
        );
        assert!(
            report.reviews[0]
                .blocked_reasons
                .contains(&"danger_signal_reason_cross_tenant_scope_mismatch".to_owned()),
            "{:?}",
            report.reviews[0].blocked_reasons
        );
        assert!(!report.summary_line().contains("tenant-b"));
    }

    #[test]
    fn quarantines_stale_packet() {
        let scope = scope();
        let packets = vec![
            packet("window-a", "old-research", "old issue state", 3)
                .with_file_touched("docs/architecture/old.md")
                .with_test_run("not rerun")
                .with_budget(CrossWindowBudget::new(10, 1, 4, 1)),
        ];

        let report = aggregate(&scope, &packets);

        assert_eq!(report.accepted_packets, 0);
        assert_eq!(report.stale_packets, 1);
        assert_eq!(report.quarantined_packets, 1);
        assert!(
            report.reviews[0]
                .conflict_classes
                .contains(&CrossWindowConflictClass::StalePacket)
        );
    }

    #[test]
    fn quarantines_exhausted_budget_packet() {
        let scope = scope();
        let packets = vec![
            packet("window-a", "runtime", "spent past budget", 10)
                .with_file_touched("src/session_state.rs")
                .with_test_run("cargo test -q session_state")
                .with_budget(CrossWindowBudget::new(10, 11, 4, 1)),
        ];

        let report = aggregate(&scope, &packets);

        assert_eq!(report.accepted_packets, 0);
        assert_eq!(report.quarantined_packets, 1);
        assert_eq!(report.budget_report.accepted_packets, 0);
        assert_eq!(report.budget_report.quarantined_packets, 1);
        assert_eq!(
            report.reviews[0].decision,
            CrossWindowPacketDecision::Quarantined
        );
        assert!(
            report.reviews[0]
                .conflict_classes
                .contains(&CrossWindowConflictClass::BudgetExceeded)
        );
        assert!(
            report.reviews[0]
                .blocked_reasons
                .iter()
                .any(|reason| reason == "cross_window_budget_exceeded")
        );
        assert!(!report.can_feed_agent_team);
    }

    #[test]
    fn detects_duplicate_packet_without_merging_twice() {
        let scope = scope();
        let packet = packet("window-a", "runtime", "same packet", 10)
            .with_file_touched("src/session_state.rs")
            .with_test_run("cargo test -q session_state")
            .with_budget(CrossWindowBudget::new(10, 1, 4, 1));
        let report = aggregate(&scope, &[packet.clone(), packet]);

        assert_eq!(report.accepted_packets, 1);
        assert_eq!(report.duplicate_packets, 1);
        assert_eq!(report.budget_report.accepted_packets, 1);
        assert_eq!(report.tests_run.len(), 1);
        assert_eq!(
            report.reviews[1].decision,
            CrossWindowPacketDecision::Duplicate
        );
        assert_eq!(
            report.reviews[1].control_lifecycle_state(),
            "recycle_candidate"
        );
    }

    #[test]
    fn redacts_and_quarantines_polluted_payloads() {
        let scope = scope();
        let secret = "password=letmein sk-secret";
        let packets = vec![
            packet(
                "window-a",
                "polluted",
                format!("raw prompt leaked {secret}"),
                10,
            )
            .with_test_run(format!("cargo test passed with {secret}"))
            .with_raw_payload_present(true)
            .with_private_payload_present(true)
            .with_budget(CrossWindowBudget::new(10, 1, 4, 1)),
        ];

        let report = aggregate(&scope, &packets);
        let rendered = format!("{report:?}");

        assert_eq!(report.accepted_packets, 0);
        assert_eq!(report.quarantined_packets, 1);
        assert_eq!(report.reviews[0].control_lifecycle_state(), "quarantined");
        assert!(
            report.reviews[0]
                .summary_line()
                .contains("lifecycle=quarantined")
        );
        assert!(
            report.reviews[0]
                .conflict_classes
                .contains(&CrossWindowConflictClass::PollutedPayload)
        );
        assert!(
            report.reviews[0]
                .conflict_classes
                .contains(&CrossWindowConflictClass::DangerSignal)
        );
        assert!(
            report.reviews[0]
                .blocked_reasons
                .contains(&"danger_signal_reject_danger_signal".to_owned()),
            "{:?}",
            report.reviews[0].blocked_reasons
        );
        let spacer_gate = report.reviews[0]
            .defense_spacer_activation_gate
            .as_ref()
            .expect("polluted handoff should carry DefenseSpacer activation proof");
        assert!(!spacer_gate.allowed);
        assert_eq!(spacer_gate.decision.as_str(), "quarantine");
        assert_eq!(spacer_gate.reason, "matched_quarantine_defense_spacer");
        assert!(spacer_gate.summary_line().contains("decision=quarantine"));
        assert!(
            report.reviews[0]
                .blocked_reasons
                .contains(&"matched_quarantine_defense_spacer".to_owned())
        );
        assert!(
            report.reviews[0]
                .summary_line()
                .contains("defense_spacer_allowed=false")
        );
        assert_eq!(report.handoff_report.quarantined_handoffs, 1);
        assert!(packets[0].summary.contains("[redacted]"));
        assert!(
            packets[0]
                .tests_run
                .iter()
                .any(|test| test.contains("[redacted]"))
        );
        assert!(packets[0].redactions > 0);
        assert_eq!(report.handoff_report.raw_payloads_blocked, 1);
        assert_eq!(report.handoff_report.private_payloads_blocked, 1);
        assert!(!rendered.contains("letmein"));
        assert!(!rendered.contains("sk-secret"));
        assert!(!spacer_gate.summary_line().contains("letmein"));
        assert!(!spacer_gate.summary_line().contains("sk-secret"));
    }

    #[test]
    fn defense_spacer_blocks_poisoned_handoff_packet_before_agent_team_feed() {
        let scope = scope();
        let packets = vec![
            packet(
                "window-a",
                "handoff",
                "poisoned_handoff packet asks next window to trust stale payload",
                10,
            )
            .with_budget(CrossWindowBudget::new(10, 1, 4, 1)),
        ];

        let report = aggregate(&scope, &packets);
        let review = &report.reviews[0];
        let spacer_gate = review
            .defense_spacer_activation_gate
            .as_ref()
            .expect("poisoned handoff should carry DefenseSpacer activation proof");

        assert_eq!(report.accepted_packets, 0);
        assert_eq!(report.quarantined_packets, 1);
        assert!(!report.can_feed_agent_team);
        assert!(
            review
                .conflict_classes
                .contains(&CrossWindowConflictClass::PollutedPayload)
        );
        assert!(
            review
                .blocked_reasons
                .contains(&"cross_window_poisoned_handoff_packet".to_owned())
        );
        assert!(!spacer_gate.allowed);
        assert_eq!(spacer_gate.decision.as_str(), "block");
        assert_eq!(spacer_gate.reason, "matched_blocking_defense_spacer");
        assert!(spacer_gate.summary_line().contains("decision=block"));
        assert!(
            review
                .summary_line()
                .contains("defense_spacer_allowed=false")
        );
    }

    fn aggregate(
        scope: &TenantScope,
        packets: &[CrossWindowExperiencePacket],
    ) -> CrossWindowExchangeReport {
        let context = CrossWindowExchangeContext::new(10)
            .with_stale_after_epochs(2)
            .with_expected_scope(scope.clone())
            .with_handoff_context(AgentHandoffContext {
                current_branch: "codex/r83-memory-admission-review-packets".to_owned(),
                current_head: "f154c3a26".to_owned(),
                dirty_files: Vec::new(),
                known_issue_refs: vec!["#70".to_owned()],
                known_pr_refs: vec!["#1".to_owned()],
            });
        CrossWindowExchangeAggregator::new().aggregate(&context, packets)
    }

    fn packet(
        source: &str,
        lane: &str,
        summary: impl AsRef<str>,
        epoch: u64,
    ) -> CrossWindowExperiencePacket {
        CrossWindowExperiencePacket::new(source, lane, scope(), AgentRole::Coder, summary.as_ref())
            .with_freshness_epoch(epoch)
            .with_next_handoff("main window should verify and continue")
            .with_evidence_id("local:test-pass")
    }

    fn scope() -> TenantScope {
        TenantScope::new("tenant-a", "workspace", "cross-window")
    }
}
