use crate::experience::ExperienceMatch;
use crate::hardware::HardwarePlan;
use crate::hierarchy::TaskProfile;
use crate::kv_cache::MemoryMatch;

use super::blueprints::build_blueprint;
use super::types::{ToolIntent, ToolsmithPlan};
use super::util::contains_any;

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
