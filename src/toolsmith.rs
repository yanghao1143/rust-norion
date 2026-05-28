use crate::experience::ExperienceMatch;
use crate::hardware::HardwarePlan;
use crate::hierarchy::TaskProfile;
use crate::kv_cache::MemoryMatch;

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
}

impl ToolBuildStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Held => "held",
            Self::Rejected => "rejected",
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

    pub fn passed_rust_gate(&self) -> bool {
        self.rust_only
            && self.rejected_requests.is_empty()
            && self.blueprints.iter().all(ToolBlueprint::rust_only)
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
            self.rejected_count(),
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
}

#[derive(Debug, Clone, Copy)]
pub struct ToolsmithInput<'a> {
    pub prompt: &'a str,
    pub profile: TaskProfile,
    pub memories: &'a [MemoryMatch],
    pub experiences: &'a [ExperienceMatch],
    pub hardware_plan: &'a HardwarePlan,
}

#[derive(Debug, Clone)]
pub struct ToolsmithPlanner {
    max_blueprints: usize,
}

impl Default for ToolsmithPlanner {
    fn default() -> Self {
        Self { max_blueprints: 3 }
    }
}

impl ToolsmithPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_blueprints(mut self, max_blueprints: usize) -> Self {
        self.max_blueprints = max_blueprints.max(1);
        self
    }

    pub fn plan(&self, input: ToolsmithInput<'_>) -> ToolsmithPlan {
        let lower = input.prompt.to_ascii_lowercase();
        let wants_tool = contains_any(
            input.prompt,
            &[
                "工具", "造", "摸索", "探索", "tool", "tools", "plugin", "script", "agent", "cli",
                "runner",
            ],
        );
        let asks_non_rust = contains_any(
            &lower,
            &[
                "python",
                "javascript",
                "typescript",
                "node",
                "shell",
                "bash",
                "powershell",
                ".py",
                ".js",
                ".ts",
            ],
        );

        let mut plan = ToolsmithPlan {
            rust_only: !asks_non_rust,
            exploration_required: contains_any(
                input.prompt,
                &["摸索", "探索", "discover", "explore", "probe", "learn"],
            ),
            ..ToolsmithPlan::default()
        };

        if asks_non_rust {
            plan.rejected_requests
                .push("non_rust_tool_request_blocked".to_owned());
            plan.notes
                .push("toolsmith keeps the tool surface Rust-only".to_owned());
        }

        if !wants_tool {
            plan.notes
                .push("no explicit tool-building need detected".to_owned());
            return plan;
        }

        let mut intents = detected_intents(input.prompt, input.profile);
        if intents.is_empty() {
            intents.push(ToolIntent::Generic);
        }
        if plan.exploration_required && !intents.contains(&ToolIntent::Discovery) {
            intents.insert(0, ToolIntent::Discovery);
        }
        intents.truncate(self.max_blueprints);

        for intent in intents {
            plan.blueprints
                .push(build_blueprint(intent, input, plan.rust_only));
        }

        if !input.experiences.is_empty() {
            plan.notes.push(format!(
                "reuse_experience_hints={}",
                input.experiences.len().min(3)
            ));
        }
        if !input.memories.is_empty() {
            plan.notes.push(format!(
                "reuse_memory_hints={}",
                input.memories.len().min(4)
            ));
        }
        plan.notes.push(format!(
            "device={} adapter_budget={} kv_prefetch={}",
            input.hardware_plan.device.as_str(),
            input.hardware_plan.execution.adapter_hints.len(),
            input.hardware_plan.execution.kv_prefetch_blocks
        ));
        plan
    }
}

fn detected_intents(prompt: &str, profile: TaskProfile) -> Vec<ToolIntent> {
    let mut intents = Vec::new();
    if contains_any(prompt, &["trace", "jsonl", "schema", "日志", "轨迹"]) {
        intents.push(ToolIntent::TraceAnalysis);
    }
    if contains_any(prompt, &["inspect", "state", "memory", "状态", "记忆"]) {
        intents.push(ToolIntent::StateInspection);
    }
    if contains_any(
        prompt,
        &["benchmark", "gate", "test", "bench", "门禁", "测试"],
    ) {
        intents.push(ToolIntent::BenchmarkGate);
    }
    if contains_any(prompt, &["runtime", "adapter", "kernel", "模型", "运行时"]) {
        intents.push(ToolIntent::RuntimeAdapter);
    }
    if contains_any(prompt, &["compact", "retention", "kv", "缓存", "压缩"]) {
        intents.push(ToolIntent::MemoryMaintenance);
    }
    if profile == TaskProfile::Coding && intents.is_empty() {
        intents.push(ToolIntent::Discovery);
    }
    intents.sort_by_key(|intent| intent.as_str());
    intents.dedup();
    intents
}

fn build_blueprint(
    intent: ToolIntent,
    input: ToolsmithInput<'_>,
    rust_only_gate: bool,
) -> ToolBlueprint {
    let (id, name, trigger, entrypoint, outline) = match intent {
        ToolIntent::Discovery => (
            "rust_toolsmith_probe",
            "Rust Toolsmith Probe",
            "discover missing local capabilities before editing code",
            "src/bin/noiron_toolsmith_probe.rs",
            vec![
                "parse task, profile, device, memory and experience hints from stdin",
                "rank missing capabilities by local evidence and expected reuse",
                "emit a ToolBlueprint JSONL row without creating files",
            ],
        ),
        ToolIntent::TraceAnalysis => (
            "rust_trace_lens",
            "Rust Trace Lens",
            "inspect JSONL traces and explain regressions",
            "src/bin/noiron_trace_lens.rs",
            vec![
                "stream JSONL records with std::io::BufRead",
                "count required control-plane fields and drift/reward failures",
                "emit compact text or JSONL diagnostics",
            ],
        ),
        ToolIntent::StateInspection => (
            "rust_state_lens",
            "Rust State Lens",
            "summarize local memory, experience, and adaptive state",
            "src/bin/noiron_state_lens.rs",
            vec![
                "open DiskKvStore paths in read-only inspection mode",
                "bucket memory vector dimensions and reward actions",
                "report stale or incompatible records without mutating state",
            ],
        ),
        ToolIntent::BenchmarkGate => (
            "rust_gate_runner",
            "Rust Gate Runner",
            "run focused benchmark gates for a proposed capability",
            "src/bin/noiron_gate_runner.rs",
            vec![
                "parse gate thresholds from args",
                "run existing benchmark, trace-schema, and kv-quant gates",
                "return nonzero only when a declared gate fails",
            ],
        ),
        ToolIntent::RuntimeAdapter => (
            "rust_runtime_adapter_probe",
            "Rust Runtime Adapter Probe",
            "probe self-developed runtime adapter fit without vendor lock-in",
            "src/bin/noiron_runtime_adapter_probe.rs",
            vec![
                "read RuntimeManifest and HardwarePlan summaries",
                "intersect manifest adapters with device adapter hints",
                "emit selected portable Rust fallback and rejected adapters",
            ],
        ),
        ToolIntent::MemoryMaintenance => (
            "rust_kv_maintainer",
            "Rust KV Maintainer",
            "inspect retention and KV compaction candidates",
            "src/bin/noiron_kv_maintainer.rs",
            vec![
                "scan persistent KV metadata with bounded candidate windows",
                "simulate retention and compaction before any mutation",
                "print protected ids and proposed merge pairs",
            ],
        ),
        ToolIntent::Generic => (
            "rust_task_tool",
            "Rust Task Tool",
            "create a small task-specific local Rust CLI",
            "src/bin/noiron_task_tool.rs",
            vec![
                "define a typed input struct and deterministic output report",
                "use std-only parsing unless the main crate already depends on a parser",
                "add a unit test and one CLI smoke path before promotion",
            ],
        ),
    };

    let status = if rust_only_gate {
        match intent {
            ToolIntent::RuntimeAdapter => ToolBuildStatus::Held,
            _ => ToolBuildStatus::Ready,
        }
    } else {
        ToolBuildStatus::Rejected
    };
    let mut gate_notes = vec![
        "rust_source_only".to_owned(),
        "no_dynamic_shell".to_owned(),
        "no_network_default".to_owned(),
    ];
    if status == ToolBuildStatus::Held {
        gate_notes.push("requires_runtime_contract_review".to_owned());
    }
    if status == ToolBuildStatus::Rejected {
        gate_notes.push("blocked_by_non_rust_request".to_owned());
    }

    ToolBlueprint {
        id: id.to_owned(),
        name: name.to_owned(),
        intent,
        trigger: format!("{}; prompt={}", trigger, compact(input.prompt, 80)),
        rust_crate: "rust".to_owned(),
        entrypoint: entrypoint.to_owned(),
        allowed_io: vec![
            "stdin".to_owned(),
            "stdout".to_owned(),
            "explicit-local-files".to_owned(),
        ],
        denied_capabilities: vec![
            "network".to_owned(),
            "arbitrary-shell".to_owned(),
            "non-rust-runtime".to_owned(),
            "implicit-state-mutation".to_owned(),
        ],
        build_steps: vec![
            "cargo fmt".to_owned(),
            "cargo test".to_owned(),
            "cargo run -- --trace-schema-gate <trace>".to_owned(),
        ],
        validation_steps: vec![
            "unit_test_blueprint_contract".to_owned(),
            "cli_smoke_with_sample_input".to_owned(),
            "trace_schema_gate_if_trace_output".to_owned(),
        ],
        source_outline: outline.into_iter().map(ToOwned::to_owned).collect(),
        status,
        gate_notes,
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    let lower = value.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::HardwarePlan;

    #[test]
    fn plans_rust_only_toolsmith_probe_for_exploration_prompt() {
        let planner = ToolsmithPlanner::new();
        let plan = planner.plan(ToolsmithInput {
            prompt: "我想让他有自己摸索造工具的能力 工具全部由rust写",
            profile: TaskProfile::Coding,
            memories: &[],
            experiences: &[],
            hardware_plan: &HardwarePlan::default(),
        });

        assert!(plan.rust_only);
        assert!(plan.exploration_required);
        assert!(plan.passed_rust_gate());
        assert!(
            plan.blueprints
                .iter()
                .any(|blueprint| blueprint.id == "rust_toolsmith_probe")
        );
        assert!(
            plan.blueprints
                .iter()
                .all(|blueprint| blueprint.entrypoint.ends_with(".rs"))
        );
    }

    #[test]
    fn blocks_non_rust_tool_requests() {
        let planner = ToolsmithPlanner::new();
        let plan = planner.plan(ToolsmithInput {
            prompt: "build a python tool for trace analysis",
            profile: TaskProfile::Coding,
            memories: &[],
            experiences: &[],
            hardware_plan: &HardwarePlan::default(),
        });

        assert!(!plan.rust_only);
        assert!(!plan.passed_rust_gate());
        assert_eq!(
            plan.rejected_requests,
            vec!["non_rust_tool_request_blocked"]
        );
        assert!(
            plan.blueprints
                .iter()
                .all(|blueprint| blueprint.status == ToolBuildStatus::Rejected)
        );
    }
}
