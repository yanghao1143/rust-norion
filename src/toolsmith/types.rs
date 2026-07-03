use crate::danger_signal::{DangerSignalInput, DangerSignalReview, review_danger_signals};
use crate::development_pollution::{
    DefenseSpacer, DefenseSpacerActivationGate, DefenseSpacerCandidate, DevelopmentPollutionEvent,
    classify_development_pollution_event, development_evidence_payload_reason,
    gate_defense_spacer_activation,
};
use crate::privacy_redaction::stable_redaction_digest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolIntent {
    Discovery,
    TraceAnalysis,
    StateInspection,
    BenchmarkGate,
    RuntimeAdapter,
    MemoryMaintenance,
    Generic,
}

impl ToolIntent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::TraceAnalysis => "trace_analysis",
            Self::StateInspection => "state_inspection",
            Self::BenchmarkGate => "benchmark_gate",
            Self::RuntimeAdapter => "runtime_adapter",
            Self::MemoryMaintenance => "memory_maintenance",
            Self::Generic => "generic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolBuildStatus {
    Ready,
    Held,
    Rejected,
    Duplicate,
    Quarantined,
}

impl ToolBuildStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Held => "held",
            Self::Rejected => "rejected",
            Self::Duplicate => "duplicate",
            Self::Quarantined => "quarantined",
        }
    }

    pub fn control_lifecycle_state(self) -> &'static str {
        match self {
            Self::Ready => "active",
            Self::Held => "repaired_candidate",
            Self::Rejected => "rejected_final",
            Self::Duplicate => "recycle_candidate",
            Self::Quarantined => "quarantined",
        }
    }

    pub fn readmission_gate(self) -> &'static str {
        match self {
            Self::Ready => "none",
            Self::Held => "contract_review_and_operator_approval",
            Self::Rejected => "none",
            Self::Duplicate => "deduplicate_before_readmission",
            Self::Quarantined => "validation_repair_and_operator_approval",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBlueprint {
    pub id: String,
    pub name: String,
    pub intent: ToolIntent,
    pub trigger: String,
    pub rust_crate: String,
    pub entrypoint: String,
    pub allowed_io: Vec<String>,
    pub denied_capabilities: Vec<String>,
    pub build_steps: Vec<String>,
    pub validation_steps: Vec<String>,
    pub source_outline: Vec<String>,
    pub provenance: String,
    pub status: ToolBuildStatus,
    pub gate_notes: Vec<String>,
}

impl ToolBlueprint {
    pub fn danger_signal_review(&self) -> DangerSignalReview {
        let denied = self.denied_capabilities.join("|");
        let allowed = self.allowed_io.join("|");
        let marker_text = format!(
            "{} {} {} {} {}",
            self.trigger,
            self.gate_notes.join(" "),
            self.source_outline.join(" "),
            self.build_steps.join(" "),
            self.validation_steps.join(" ")
        );

        review_danger_signals(
            DangerSignalInput::new("tool_blueprint")
                .trusted_self_provenance(
                    self.rust_only()
                        && self.provenance.starts_with("toolsmith-planner:v1")
                        && self
                            .gate_notes
                            .iter()
                            .any(|note| note == "rust_source_only"),
                )
                .source_digest(stable_redaction_digest([
                    "tool-blueprint",
                    self.id.as_str(),
                    self.provenance.as_str(),
                    self.entrypoint.as_str(),
                ]))
                .lifecycle_state(self.control_lifecycle_state())
                .marker_text(marker_text)
                .unexpected_tool_permission(
                    allowed.contains("network")
                        || allowed.contains("shell")
                        || !denied.contains("network")
                        || !denied.contains("arbitrary-shell")
                        || !denied.contains("implicit-state-mutation"),
                ),
        )
    }

    pub fn control_lifecycle_state(&self) -> &'static str {
        self.status.control_lifecycle_state()
    }

    pub fn lifecycle_evidence_summary(&self) -> String {
        let reason_code = self
            .gate_notes
            .first()
            .map(String::as_str)
            .unwrap_or("tool_blueprint_clean");
        format!(
            "lifecycle={} reason_code={} source_digest={} parent_lineage=toolsmith:{} rollback_anchor=blueprint_preview_only affected_scope=tool_blueprint readmission_gate={} operator_approval_required={}",
            self.control_lifecycle_state(),
            reason_code,
            self.provenance,
            self.id,
            self.status.readmission_gate(),
            !matches!(
                self.status,
                ToolBuildStatus::Ready | ToolBuildStatus::Rejected
            )
        )
    }

    pub fn defense_spacer_activation_gate(&self) -> Option<DefenseSpacerActivationGate> {
        let payload = self.development_pollution_payload();
        let reason = development_evidence_payload_reason(&payload);
        if reason == "current_validated_path" {
            return None;
        }

        let finding = classify_development_pollution_event(&DevelopmentPollutionEvent::new(
            format!("toolsmith-blueprint-{}", self.id),
            "toolsmith_blueprint",
            payload,
            reason,
        ));
        let spacer = DefenseSpacer::from_finding(
            &finding,
            "tool_blueprint_activation",
            "toolsmith_plan_preview",
            "operator_review",
        );
        let candidate = DefenseSpacerCandidate::from_finding(&finding, "tool_blueprint_activation");
        Some(gate_defense_spacer_activation(&[spacer], &candidate))
    }

    pub fn summary(&self) -> String {
        format!(
            "id={} name={} intent={} crate={} entrypoint={} status={} lifecycle={} notes={}",
            self.id,
            self.name,
            self.intent.as_str(),
            self.rust_crate,
            self.entrypoint,
            self.status.as_str(),
            self.control_lifecycle_state(),
            self.gate_notes.join("|")
        )
    }

    pub fn rust_only(&self) -> bool {
        self.rust_crate == "rust" && self.entrypoint.ends_with(".rs")
    }

    fn defense_spacer_activation_allowed(&self) -> bool {
        self.defense_spacer_activation_gate()
            .map_or(true, |gate| gate.allowed)
    }

    fn development_pollution_payload(&self) -> String {
        let gate_notes = self.gate_notes.join(" ");
        let source_outline = self.source_outline.join(" ");
        let build_steps = self.build_steps.join(" ");
        let validation_steps = self.validation_steps.join(" ");
        [
            self.trigger.as_str(),
            self.provenance.as_str(),
            gate_notes.as_str(),
            source_outline.as_str(),
            build_steps.as_str(),
            validation_steps.as_str(),
        ]
        .join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolsmithPlan {
    pub rust_only: bool,
    pub exploration_required: bool,
    pub blueprints: Vec<ToolBlueprint>,
    pub rejected_requests: Vec<String>,
    pub notes: Vec<String>,
}

impl Default for ToolsmithPlan {
    fn default() -> Self {
        Self {
            rust_only: true,
            exploration_required: false,
            blueprints: Vec::new(),
            rejected_requests: Vec::new(),
            notes: Vec::new(),
        }
    }
}

impl ToolsmithPlan {
    pub fn blueprint_count(&self) -> usize {
        self.blueprints.len()
    }

    pub fn ready_count(&self) -> usize {
        self.blueprints
            .iter()
            .filter(|blueprint| blueprint.status == ToolBuildStatus::Ready)
            .count()
    }

    pub fn held_count(&self) -> usize {
        self.blueprints
            .iter()
            .filter(|blueprint| blueprint.status == ToolBuildStatus::Held)
            .count()
    }

    pub fn rejected_count(&self) -> usize {
        self.rejected_requests.len()
            + self
                .blueprints
                .iter()
                .filter(|blueprint| blueprint.status == ToolBuildStatus::Rejected)
                .count()
    }

    pub fn duplicate_count(&self) -> usize {
        self.blueprints
            .iter()
            .filter(|blueprint| blueprint.status == ToolBuildStatus::Duplicate)
            .count()
    }

    pub fn failed_validation_count(&self) -> usize {
        self.blueprints
            .iter()
            .filter(|blueprint| {
                blueprint.status == ToolBuildStatus::Quarantined
                    && blueprint
                        .gate_notes
                        .iter()
                        .any(|note| note == "failed_validation")
            })
            .count()
    }

    pub fn quarantined_count(&self) -> usize {
        self.blueprints
            .iter()
            .filter(|blueprint| blueprint.status == ToolBuildStatus::Quarantined)
            .count()
    }

    pub fn passed_rust_gate(&self) -> bool {
        self.rust_only
            && self.rejected_requests.is_empty()
            && self.blueprints.iter().all(ToolBlueprint::rust_only)
            && self
                .blueprints
                .iter()
                .all(|blueprint| blueprint.danger_signal_review().activation_allowed)
            && self
                .blueprints
                .iter()
                .all(ToolBlueprint::defense_spacer_activation_allowed)
            && self.duplicate_count() == 0
            && self.quarantined_count() == 0
    }

    pub fn has_blueprints(&self) -> bool {
        !self.blueprints.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "rust_only={} exploration_required={} blueprints={} ready={} held={} rejected={} gate_passed={}",
            self.rust_only,
            self.exploration_required,
            self.blueprint_count(),
            self.ready_count(),
            self.held_count(),
            self.rejected_count() + self.duplicate_count() + self.failed_validation_count(),
            self.passed_rust_gate()
        )
    }

    pub fn reward_notes(&self) -> Vec<String> {
        if self.blueprints.is_empty() && self.rejected_requests.is_empty() {
            return Vec::new();
        }

        let mut notes = vec![format!(
            "toolsmith:blueprints={}:ready={}:held={}:rejected={}:rust_only={}",
            self.blueprint_count(),
            self.ready_count(),
            self.held_count(),
            self.rejected_count(),
            self.rust_only
        )];
        notes.extend(
            self.blueprints.iter().take(3).map(|blueprint| {
                format!("toolsmith:{}:{}", blueprint.id, blueprint.status.as_str())
            }),
        );
        notes
    }

    pub fn memory_admission_candidates(&self) -> Vec<String> {
        self.blueprints
            .iter()
            .filter(|blueprint| {
                matches!(
                    blueprint.status,
                    ToolBuildStatus::Ready | ToolBuildStatus::Held | ToolBuildStatus::Quarantined
                ) && blueprint.danger_signal_review().activation_allowed
                    && blueprint.defense_spacer_activation_allowed()
            })
            .map(|blueprint| {
                format!(
                    "tool_reliability:{}:intent={}:status={}:lifecycle={}:validation_steps={}:provenance={}",
                    blueprint.id,
                    blueprint.intent.as_str(),
                    blueprint.status.as_str(),
                    blueprint.control_lifecycle_state(),
                    blueprint.validation_steps.len(),
                    blueprint.provenance
                )
            })
            .collect()
    }
}
