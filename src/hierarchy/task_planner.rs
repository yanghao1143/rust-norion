use super::profile::TaskProfile;
use super::weights::HierarchyWeights;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskLanguageMode {
    English,
    Chinese,
    Mixed,
}

impl TaskLanguageMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::Chinese => "chinese",
            Self::Mixed => "mixed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskComputeBudget {
    Low,
    Normal,
    Expanded,
}

impl TaskComputeBudget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::Expanded => "expanded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskMode {
    EnglishChat,
    ChineseChat,
    RustCoding,
    CodeReview,
    CompilerFix,
    BenchmarkAnalysis,
    Research,
    LowBudget,
}

impl TaskMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EnglishChat => "english_chat",
            Self::ChineseChat => "chinese_chat",
            Self::RustCoding => "rust_coding",
            Self::CodeReview => "code_review",
            Self::CompilerFix => "compiler_fix",
            Self::BenchmarkAnalysis => "benchmark_analysis",
            Self::Research => "research",
            Self::LowBudget => "low_budget",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskModeSignals {
    pub language: TaskLanguageMode,
    pub coding_intent: bool,
    pub validation_mode: bool,
    pub memory_need: f32,
    pub compute_budget: TaskComputeBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskHierarchyMutationKind {
    Threshold,
    HierarchyWeights,
    RouteFanout,
}

impl TaskHierarchyMutationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Threshold => "threshold",
            Self::HierarchyWeights => "hierarchy_weights",
            Self::RouteFanout => "route_fanout",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskHierarchyMutationRecord {
    pub id: String,
    pub kind: TaskHierarchyMutationKind,
    pub before: String,
    pub after: String,
    pub delta: f32,
    pub rollback_anchor_id: String,
    pub evidence: Vec<String>,
    pub replayable: bool,
    pub reverted: bool,
    pub preview_only: bool,
}

impl TaskHierarchyMutationRecord {
    pub fn summary(&self) -> String {
        format!(
            "id={} kind={} before={} after={} delta={:.6} rollback={} replayable={} reverted={} preview_only={} evidence={}",
            self.id,
            self.kind.as_str(),
            self.before,
            self.after,
            self.delta,
            self.rollback_anchor_id,
            self.replayable,
            self.reverted,
            self.preview_only,
            self.evidence.join("|")
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskHierarchyReplayReport {
    pub mutation_records: usize,
    pub threshold_after: f32,
    pub hierarchy_after: HierarchyWeights,
    pub route_fanout_after: usize,
    pub rollback_anchor_id: String,
    pub replayable: bool,
    pub reverted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskAwareHierarchyPlan {
    pub profile: TaskProfile,
    pub mode: TaskMode,
    pub signals: TaskModeSignals,
    pub hierarchy_depth: usize,
    pub route_fanout: usize,
    pub route_pressure: f32,
    pub compute_reduction: f32,
    pub threshold_before: f32,
    pub threshold_after: f32,
    pub hierarchy_before: HierarchyWeights,
    pub hierarchy_after: HierarchyWeights,
    pub selected_lanes: Vec<String>,
    pub skipped_lanes: Vec<String>,
    pub memory_lanes: Vec<String>,
    pub skipped_memory_lanes: Vec<String>,
    pub rollback_anchor_id: String,
    pub mutation_history: Vec<TaskHierarchyMutationRecord>,
    pub runtime_applied: bool,
    pub state_write_allowed: bool,
    pub adaptive_state_write_allowed: bool,
    pub ndkv_write_allowed: bool,
}

impl TaskAwareHierarchyPlan {
    pub fn mutation_summaries(&self, limit: usize) -> Vec<String> {
        self.mutation_history
            .iter()
            .take(limit)
            .map(TaskHierarchyMutationRecord::summary)
            .collect()
    }

    pub fn mutation_count(&self) -> usize {
        self.mutation_history.len()
    }

    pub fn replay_mutations(&self) -> TaskHierarchyReplayReport {
        TaskHierarchyReplayReport {
            mutation_records: self.mutation_history.len(),
            threshold_after: self.threshold_after,
            hierarchy_after: self.hierarchy_after,
            route_fanout_after: self.route_fanout,
            rollback_anchor_id: self.rollback_anchor_id.clone(),
            replayable: self
                .mutation_history
                .iter()
                .all(|record| record.replayable && record.preview_only),
            reverted: false,
        }
    }

    pub fn revert_mutations(&self) -> TaskHierarchyReplayReport {
        let before_fanout = self
            .mutation_history
            .iter()
            .find(|record| record.kind == TaskHierarchyMutationKind::RouteFanout)
            .and_then(|record| record.before.parse::<usize>().ok())
            .unwrap_or(self.route_fanout);
        TaskHierarchyReplayReport {
            mutation_records: self.mutation_history.len(),
            threshold_after: self.threshold_before,
            hierarchy_after: self.hierarchy_before,
            route_fanout_after: before_fanout,
            rollback_anchor_id: self.rollback_anchor_id.clone(),
            replayable: self
                .mutation_history
                .iter()
                .all(|record| record.replayable && record.preview_only),
            reverted: true,
        }
    }

    pub fn mutation_history_replayable(&self) -> bool {
        let replay = self.replay_mutations();
        let revert = self.revert_mutations();
        replay.replayable
            && !replay.reverted
            && revert.reverted
            && same_weights(replay.hierarchy_after, self.hierarchy_after)
            && same_weights(revert.hierarchy_after, self.hierarchy_before)
            && (revert.threshold_after - self.threshold_before).abs() <= f32::EPSILON
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TaskAwareHierarchyInput<'a> {
    pub prompt: &'a str,
    pub profile: TaskProfile,
    pub max_tokens: Option<usize>,
    pub prompt_tokens: usize,
    pub used_memories: usize,
    pub threshold_before: f32,
    pub hierarchy_before: HierarchyWeights,
}

#[derive(Debug, Clone)]
pub struct TaskAwareHierarchyPlanner {
    pub low_budget_token_limit: usize,
    pub long_context_tokens: usize,
}

impl Default for TaskAwareHierarchyPlanner {
    fn default() -> Self {
        Self {
            low_budget_token_limit: 96,
            long_context_tokens: 512,
        }
    }
}

impl TaskAwareHierarchyPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn plan(&self, input: TaskAwareHierarchyInput<'_>) -> TaskAwareHierarchyPlan {
        let signals = classify_signals(
            input.prompt,
            input.profile,
            input.max_tokens,
            input.prompt_tokens,
            input.used_memories,
            self.low_budget_token_limit,
            self.long_context_tokens,
        );
        let mode = classify_mode(input.prompt, input.profile, &signals);
        let hierarchy_after = target_hierarchy(mode, input.hierarchy_before);
        let threshold_after = target_threshold(mode, input.threshold_before, &signals);
        let hierarchy_depth = target_hierarchy_depth(mode, input.prompt_tokens);
        let route_fanout = target_route_fanout(mode);
        let route_pressure = route_pressure(input.prompt_tokens, input.max_tokens, &signals);
        let compute_reduction = compute_reduction(
            mode,
            route_pressure,
            input.threshold_before,
            threshold_after,
        );
        let selected_lanes = selected_lanes(hierarchy_after);
        let skipped_lanes = skipped_lanes(&selected_lanes);
        let memory_lanes = memory_lanes(mode, signals.memory_need);
        let skipped_memory_lanes = skipped_memory_lanes(&memory_lanes);
        let rollback_anchor_id = format!(
            "task_hierarchy:{}:{}:stable",
            profile_slug(input.profile),
            mode.as_str()
        );
        let mutation_history = mutation_history(
            input.profile,
            mode,
            input.threshold_before,
            threshold_after,
            input.hierarchy_before,
            hierarchy_after,
            route_fanout,
            &rollback_anchor_id,
            &signals,
        );

        TaskAwareHierarchyPlan {
            profile: input.profile,
            mode,
            signals,
            hierarchy_depth,
            route_fanout,
            route_pressure,
            compute_reduction,
            threshold_before: sanitize_unit(input.threshold_before),
            threshold_after,
            hierarchy_before: input.hierarchy_before,
            hierarchy_after,
            selected_lanes,
            skipped_lanes,
            memory_lanes,
            skipped_memory_lanes,
            rollback_anchor_id,
            mutation_history,
            runtime_applied: true,
            state_write_allowed: false,
            adaptive_state_write_allowed: false,
            ndkv_write_allowed: false,
        }
    }
}

fn classify_mode(prompt: &str, profile: TaskProfile, signals: &TaskModeSignals) -> TaskMode {
    let lower = prompt.to_ascii_lowercase();
    if signals.compute_budget == TaskComputeBudget::Low {
        return TaskMode::LowBudget;
    }
    if contains_any(
        &lower,
        &[
            "cargo test",
            "cargo check",
            "error[",
            "failing test",
            "test failure",
            "compile error",
            "编译错误",
            "测试失败",
        ],
    ) {
        return TaskMode::CompilerFix;
    }
    if contains_any(
        &lower,
        &[
            "benchmark",
            "bench",
            "criterion",
            "latency",
            "throughput",
            "基准",
        ],
    ) {
        return TaskMode::BenchmarkAnalysis;
    }
    if contains_any(&lower, &["code review", "review code", "审查", "代码审查"]) {
        return TaskMode::CodeReview;
    }
    if profile == TaskProfile::Coding
        || contains_any(
            &lower,
            &[
                "rust", "trait", "borrow", "lifetime", "tokio", "unsafe", "cargo",
            ],
        )
    {
        return TaskMode::RustCoding;
    }
    if contains_any(
        &lower,
        &["research", "paper", "论文", "arxiv", "study", "experiment"],
    ) {
        return TaskMode::Research;
    }
    match signals.language {
        TaskLanguageMode::Chinese | TaskLanguageMode::Mixed => TaskMode::ChineseChat,
        TaskLanguageMode::English => TaskMode::EnglishChat,
    }
}

fn classify_signals(
    prompt: &str,
    profile: TaskProfile,
    max_tokens: Option<usize>,
    prompt_tokens: usize,
    used_memories: usize,
    low_budget_token_limit: usize,
    long_context_tokens: usize,
) -> TaskModeSignals {
    let lower = prompt.to_ascii_lowercase();
    let cjk = prompt.chars().filter(|ch| is_cjk(*ch)).count();
    let ascii_letters = prompt.chars().filter(|ch| ch.is_ascii_alphabetic()).count();
    let language = match (cjk > 0, ascii_letters > 0) {
        (true, true) => TaskLanguageMode::Mixed,
        (true, false) => TaskLanguageMode::Chinese,
        (false, _) => TaskLanguageMode::English,
    };
    let coding_intent = profile == TaskProfile::Coding
        || contains_any(
            &lower,
            &[
                "rust", "code", "cargo", "test", "compiler", "trait", "borrow",
            ],
        );
    let validation_mode = contains_any(
        &lower,
        &[
            "cargo test",
            "cargo check",
            "benchmark",
            "bench",
            "criterion",
            "compile",
            "error[",
            "测试",
            "编译",
        ],
    );
    let compute_budget = if max_tokens
        .map(|tokens| tokens <= low_budget_token_limit)
        .unwrap_or(false)
        || contains_any(
            &lower,
            &["low budget", "cheap", "fast only", "省算力", "低预算"],
        ) {
        TaskComputeBudget::Low
    } else if prompt_tokens >= long_context_tokens || profile == TaskProfile::LongDocument {
        TaskComputeBudget::Expanded
    } else {
        TaskComputeBudget::Normal
    };
    let mut memory_need: f32 = 0.28 + (used_memories.min(4) as f32 * 0.10);
    if validation_mode {
        memory_need += 0.18;
    }
    if profile == TaskProfile::LongDocument || prompt_tokens > long_context_tokens {
        memory_need += 0.22;
    }
    if contains_any(
        &lower,
        &["remember", "context", "history", "memory", "上下文", "记忆"],
    ) {
        memory_need += 0.18;
    }

    TaskModeSignals {
        language,
        coding_intent,
        validation_mode,
        memory_need: memory_need.clamp(0.0, 1.0),
        compute_budget,
    }
}

fn target_hierarchy(mode: TaskMode, fallback: HierarchyWeights) -> HierarchyWeights {
    match mode {
        TaskMode::EnglishChat => HierarchyWeights::new(0.46, 0.36, 0.18),
        TaskMode::ChineseChat => HierarchyWeights::new(0.50, 0.34, 0.16),
        TaskMode::RustCoding => HierarchyWeights::new(0.22, 0.62, 0.16),
        TaskMode::CodeReview => HierarchyWeights::new(0.34, 0.50, 0.16),
        TaskMode::CompilerFix => HierarchyWeights::new(0.18, 0.66, 0.16),
        TaskMode::BenchmarkAnalysis => HierarchyWeights::new(0.34, 0.24, 0.42),
        TaskMode::Research => HierarchyWeights::new(0.52, 0.20, 0.28),
        TaskMode::LowBudget => fallback.blend(HierarchyWeights::new(0.24, 0.56, 0.20), 0.70),
    }
}

fn target_threshold(mode: TaskMode, threshold_before: f32, signals: &TaskModeSignals) -> f32 {
    let threshold_before = threshold_before.clamp(0.18, 0.88);
    if threshold_before >= 0.80 && mode != TaskMode::LowBudget {
        return threshold_before;
    }
    let target = match mode {
        TaskMode::EnglishChat => 0.56,
        TaskMode::ChineseChat => 0.54,
        TaskMode::RustCoding => 0.43,
        TaskMode::CodeReview => 0.47,
        TaskMode::CompilerFix => 0.39,
        TaskMode::BenchmarkAnalysis => 0.50,
        TaskMode::Research => 0.46,
        TaskMode::LowBudget => 0.78,
    };
    let memory_hit_lift = (signals.memory_need - 0.55).max(0.0) * 0.24;
    threshold_before
        .mul_add(0.35, (target + memory_hit_lift) * 0.65)
        .clamp(0.18, 0.88)
}

fn target_hierarchy_depth(mode: TaskMode, prompt_tokens: usize) -> usize {
    let depth = match mode {
        TaskMode::EnglishChat | TaskMode::ChineseChat => 2,
        TaskMode::RustCoding | TaskMode::CodeReview | TaskMode::CompilerFix => 3,
        TaskMode::BenchmarkAnalysis | TaskMode::Research => 4,
        TaskMode::LowBudget => 1,
    };
    if prompt_tokens > 512 {
        depth.max(3)
    } else {
        depth
    }
}

fn target_route_fanout(mode: TaskMode) -> usize {
    match mode {
        TaskMode::EnglishChat | TaskMode::ChineseChat => 2,
        TaskMode::RustCoding | TaskMode::CodeReview | TaskMode::CompilerFix => 3,
        TaskMode::BenchmarkAnalysis | TaskMode::Research => 4,
        TaskMode::LowBudget => 1,
    }
}

fn selected_lanes(weights: HierarchyWeights) -> Vec<String> {
    let mut lanes = Vec::new();
    if weights.global >= 0.18 {
        lanes.push("global".to_owned());
    }
    if weights.local >= 0.18 {
        lanes.push("local".to_owned());
    }
    if weights.convolution >= 0.18 {
        lanes.push("convolution".to_owned());
    }
    if lanes.is_empty() {
        lanes.push("local".to_owned());
    }
    lanes
}

fn skipped_lanes(selected: &[String]) -> Vec<String> {
    ["global", "local", "convolution"]
        .into_iter()
        .filter(|lane| !selected.iter().any(|selected| selected == lane))
        .map(str::to_owned)
        .collect()
}

fn memory_lanes(mode: TaskMode, memory_need: f32) -> Vec<String> {
    let mut lanes = match mode {
        TaskMode::CompilerFix => vec!["runtime_kv", "semantic_memory", "tool_output"],
        TaskMode::RustCoding | TaskMode::CodeReview => {
            vec!["semantic_memory", "runtime_kv", "reasoning_genome"]
        }
        TaskMode::BenchmarkAnalysis => vec!["runtime_kv", "gist_memory", "semantic_memory"],
        TaskMode::Research => vec!["semantic_memory", "gist_memory", "reasoning_genome"],
        TaskMode::LowBudget => vec!["gist_memory"],
        TaskMode::EnglishChat | TaskMode::ChineseChat => vec!["gist_memory", "semantic_memory"],
    }
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    if memory_need >= 0.70 && !lanes.iter().any(|lane| lane == "reasoning_genome") {
        lanes.push("reasoning_genome".to_owned());
    }
    lanes
}

fn skipped_memory_lanes(selected: &[String]) -> Vec<String> {
    [
        "semantic_memory",
        "gist_memory",
        "runtime_kv",
        "reasoning_genome",
        "tool_output",
    ]
    .into_iter()
    .filter(|lane| !selected.iter().any(|selected| selected == lane))
    .map(str::to_owned)
    .collect()
}

fn route_pressure(
    prompt_tokens: usize,
    max_tokens: Option<usize>,
    signals: &TaskModeSignals,
) -> f32 {
    let token_pressure = (prompt_tokens as f32 / 1024.0).min(1.0) * 0.52;
    let output_pressure = max_tokens
        .map(|tokens| (96.0 / tokens.max(1) as f32).min(1.0) * 0.24)
        .unwrap_or(0.08);
    let budget_pressure = match signals.compute_budget {
        TaskComputeBudget::Low => 0.30,
        TaskComputeBudget::Normal => 0.10,
        TaskComputeBudget::Expanded => 0.18,
    };
    (token_pressure + output_pressure + budget_pressure).clamp(0.0, 1.0)
}

fn compute_reduction(
    mode: TaskMode,
    route_pressure: f32,
    threshold_before: f32,
    threshold_after: f32,
) -> f32 {
    let threshold_savings = (threshold_after - threshold_before).max(0.0) * 0.42;
    let mode_savings = match mode {
        TaskMode::LowBudget => 0.38,
        TaskMode::BenchmarkAnalysis => 0.18,
        TaskMode::EnglishChat | TaskMode::ChineseChat => 0.14,
        TaskMode::RustCoding | TaskMode::CodeReview | TaskMode::CompilerFix => 0.10,
        TaskMode::Research => 0.08,
    };
    (threshold_savings + mode_savings + route_pressure * 0.20).clamp(0.0, 1.0)
}

fn mutation_history(
    profile: TaskProfile,
    mode: TaskMode,
    threshold_before: f32,
    threshold_after: f32,
    hierarchy_before: HierarchyWeights,
    hierarchy_after: HierarchyWeights,
    route_fanout: usize,
    rollback_anchor_id: &str,
    signals: &TaskModeSignals,
) -> Vec<TaskHierarchyMutationRecord> {
    let threshold_delta = threshold_after - sanitize_unit(threshold_before);
    let hierarchy_delta = weight_delta(hierarchy_before, hierarchy_after);
    let default_fanout = 2usize;
    let fanout_delta = route_fanout as f32 - default_fanout as f32;
    vec![
        TaskHierarchyMutationRecord {
            id: format!(
                "task_hierarchy:{}:{}:threshold",
                profile_slug(profile),
                mode.as_str()
            ),
            kind: TaskHierarchyMutationKind::Threshold,
            before: format!("{:.6}", sanitize_unit(threshold_before)),
            after: format!("{threshold_after:.6}"),
            delta: threshold_delta,
            rollback_anchor_id: rollback_anchor_id.to_owned(),
            evidence: evidence(mode, signals),
            replayable: true,
            reverted: false,
            preview_only: true,
        },
        TaskHierarchyMutationRecord {
            id: format!(
                "task_hierarchy:{}:{}:weights",
                profile_slug(profile),
                mode.as_str()
            ),
            kind: TaskHierarchyMutationKind::HierarchyWeights,
            before: weight_summary(hierarchy_before),
            after: weight_summary(hierarchy_after),
            delta: hierarchy_delta,
            rollback_anchor_id: rollback_anchor_id.to_owned(),
            evidence: evidence(mode, signals),
            replayable: true,
            reverted: false,
            preview_only: true,
        },
        TaskHierarchyMutationRecord {
            id: format!(
                "task_hierarchy:{}:{}:fanout",
                profile_slug(profile),
                mode.as_str()
            ),
            kind: TaskHierarchyMutationKind::RouteFanout,
            before: default_fanout.to_string(),
            after: route_fanout.to_string(),
            delta: fanout_delta,
            rollback_anchor_id: rollback_anchor_id.to_owned(),
            evidence: evidence(mode, signals),
            replayable: true,
            reverted: false,
            preview_only: true,
        },
    ]
}

fn evidence(mode: TaskMode, signals: &TaskModeSignals) -> Vec<String> {
    vec![
        format!("mode={}", mode.as_str()),
        format!("language={}", signals.language.as_str()),
        format!("coding_intent={}", signals.coding_intent),
        format!("validation_mode={}", signals.validation_mode),
        format!("memory_need={:.3}", signals.memory_need),
        format!("compute_budget={}", signals.compute_budget.as_str()),
    ]
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF)
}

fn sanitize_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.52
    }
}

fn weight_delta(before: HierarchyWeights, after: HierarchyWeights) -> f32 {
    (after.global - before.global).abs()
        + (after.local - before.local).abs()
        + (after.convolution - before.convolution).abs()
}

fn weight_summary(weights: HierarchyWeights) -> String {
    format!(
        "g:{:.3}|l:{:.3}|c:{:.3}",
        weights.global, weights.local, weights.convolution
    )
}

fn same_weights(left: HierarchyWeights, right: HierarchyWeights) -> bool {
    (left.global - right.global).abs() <= 0.000_001
        && (left.local - right.local).abs() <= 0.000_001
        && (left.convolution - right.convolution).abs() <= 0.000_001
}

fn profile_slug(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(
        prompt: &str,
        profile: TaskProfile,
        max_tokens: Option<usize>,
    ) -> TaskAwareHierarchyPlan {
        TaskAwareHierarchyPlanner::new().plan(TaskAwareHierarchyInput {
            prompt,
            profile,
            max_tokens,
            prompt_tokens: prompt.split_whitespace().count(),
            used_memories: 1,
            threshold_before: 0.52,
            hierarchy_before: HierarchyWeights::default(),
        })
    }

    #[test]
    fn classifies_english_and_chinese_chat_modes() {
        let english = plan(
            "Explain how memory helps a local assistant.",
            TaskProfile::General,
            None,
        );
        let chinese = plan(
            "请解释本地模型如何使用长期记忆。",
            TaskProfile::General,
            None,
        );

        assert_eq!(english.mode, TaskMode::EnglishChat);
        assert_eq!(english.signals.language, TaskLanguageMode::English);
        assert_eq!(chinese.mode, TaskMode::ChineseChat);
        assert_eq!(chinese.signals.language, TaskLanguageMode::Chinese);
    }

    #[test]
    fn classifies_rust_compiler_benchmark_and_low_budget_modes() {
        let rust = plan(
            "Write Rust code using tokio and trait bounds.",
            TaskProfile::Coding,
            None,
        );
        let compiler = plan(
            "cargo test fails with error[E0502], fix the borrow issue.",
            TaskProfile::Coding,
            None,
        );
        let benchmark = plan(
            "Analyze criterion benchmark latency regression.",
            TaskProfile::Coding,
            None,
        );
        let low_budget = plan(
            "Answer fast with low budget.",
            TaskProfile::General,
            Some(32),
        );

        assert_eq!(rust.mode, TaskMode::RustCoding);
        assert_eq!(compiler.mode, TaskMode::CompilerFix);
        assert!(compiler.signals.validation_mode);
        assert_eq!(benchmark.mode, TaskMode::BenchmarkAnalysis);
        assert_eq!(low_budget.mode, TaskMode::LowBudget);
        assert_eq!(low_budget.signals.compute_budget, TaskComputeBudget::Low);
    }

    #[test]
    fn mutation_history_replays_and_reverts() {
        let plan = plan(
            "cargo check reports a compiler error in Rust code.",
            TaskProfile::Coding,
            None,
        );
        let replay = plan.replay_mutations();
        let revert = plan.revert_mutations();

        assert_eq!(plan.mutation_count(), 3);
        assert!(plan.mutation_history_replayable());
        assert!(replay.replayable);
        assert!(!replay.reverted);
        assert!(revert.reverted);
        assert_eq!(revert.threshold_after, plan.threshold_before);
        assert_eq!(revert.hierarchy_after, plan.hierarchy_before);
        assert_eq!(replay.rollback_anchor_id, plan.rollback_anchor_id);
    }
}
