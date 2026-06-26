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
    pub fn summary(&self) -> String {
        format!(
            "id={} name={} intent={} crate={} entrypoint={} status={} notes={}",
            self.id,
            self.name,
            self.intent.as_str(),
            self.rust_crate,
            self.entrypoint,
            self.status.as_str(),
            self.gate_notes.join("|")
        )
    }

    pub fn rust_only(&self) -> bool {
        self.rust_crate == "rust" && self.entrypoint.ends_with(".rs")
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
                )
            })
            .map(|blueprint| {
                format!(
                    "tool_reliability:{}:intent={}:status={}:validation_steps={}:provenance={}",
                    blueprint.id,
                    blueprint.intent.as_str(),
                    blueprint.status.as_str(),
                    blueprint.validation_steps.len(),
                    blueprint.provenance
                )
            })
            .collect()
    }
}
