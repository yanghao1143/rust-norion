pub const POLICY: &str = "one_quality_12b_plus_small_helpers";
pub const CAPACITY_POLICY: &str = "one_quality_plus_small_helpers";
pub const RECOMMENDED_LAUNCH_ORDER: &str = "quality,summary,router,review,index,test-gate";
pub const MAX_QUALITY_12B_WORKERS: usize = 1;
pub const HELPER_TARGET_WORKERS: usize = 5;
pub const QUALITY_ROLE: &str = "quality";
pub const RECOMMENDED_LAUNCH_ROLES: [&str; HELPER_TARGET_WORKERS + 1] = [
    QUALITY_ROLE,
    "summary",
    "router",
    "review",
    "index",
    "test-gate",
];
pub const HELPER_ROLES: [&str; HELPER_TARGET_WORKERS] = [
    RECOMMENDED_LAUNCH_ROLES[1],
    RECOMMENDED_LAUNCH_ROLES[2],
    RECOMMENDED_LAUNCH_ROLES[3],
    RECOMMENDED_LAUNCH_ROLES[4],
    RECOMMENDED_LAUNCH_ROLES[5],
];
pub const CPU_FALLBACK_HELPER_ROLES: [&str; 1] = ["index"];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelPoolFacts {
    pub quality_ready: Option<bool>,
    pub quality_context_sufficient: Option<bool>,
    pub quality_context_tokens: Option<String>,
    pub quality_required_context_tokens: Option<String>,
    pub quality_runtime_accelerated: Option<bool>,
    pub capacity_recommendation: Option<String>,
    pub expansion_allowed: Option<bool>,
    pub healthy_helper_worker_count: Option<usize>,
    pub unknown_runtime_worker_count: Option<usize>,
    pub has_summary: bool,
    pub has_router: bool,
    pub has_review: bool,
    pub has_index: bool,
    pub has_test_gate: bool,
    pub quality_worker_count: usize,
    pub helper_worker_count: usize,
    pub quality_cpu_fallback: bool,
    pub quality_zero_gpu_layers: bool,
    pub helper_cpu_or_no_gpu_roles: Vec<String>,
}

impl ModelPoolFacts {
    pub fn extra_quality_12b_detected(&self) -> bool {
        self.quality_worker_count > MAX_QUALITY_12B_WORKERS
    }

    pub fn full_helper_pool_visible(&self) -> bool {
        HELPER_ROLES
            .iter()
            .all(|role| helper_role_visible(self, role))
    }

    pub fn helper_cpu_or_no_gpu_detected(&self) -> bool {
        !self.blocking_helper_cpu_or_no_gpu_roles().is_empty()
    }

    pub fn blocking_helper_cpu_or_no_gpu_roles(&self) -> Vec<&str> {
        self.helper_cpu_or_no_gpu_roles
            .iter()
            .map(String::as_str)
            .filter(|role| !CPU_FALLBACK_HELPER_ROLES.contains(role))
            .collect()
    }

    pub fn allowed_cpu_fallback_helper_roles(&self) -> Vec<&str> {
        self.helper_cpu_or_no_gpu_roles
            .iter()
            .map(String::as_str)
            .filter(|role| CPU_FALLBACK_HELPER_ROLES.contains(role))
            .collect()
    }
}

pub fn missing_helper_roles(facts: &ModelPoolFacts) -> Vec<&'static str> {
    HELPER_ROLES
        .into_iter()
        .filter(|role| !helper_role_visible(facts, role))
        .collect()
}

fn helper_role_visible(facts: &ModelPoolFacts, role: &str) -> bool {
    match role {
        "summary" => facts.has_summary,
        "router" => facts.has_router,
        "review" => facts.has_review,
        "index" => facts.has_index,
        "test-gate" => facts.has_test_gate,
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelPoolDecision {
    pub safe_to_enable_pool_workers: bool,
    pub next_step: &'static str,
    pub reason: &'static str,
    pub kind: AdviceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdviceKind {
    Busy,
    Error,
}

impl AdviceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AdviceKind::Busy => "busy",
            AdviceKind::Error => "error",
        }
    }
}

pub fn model_pool_decision(facts: &ModelPoolFacts) -> ModelPoolDecision {
    if facts.quality_ready == Some(false) {
        return blocked(
            "start_or_fix_quality_worker_8686",
            "quality_worker_not_ready",
        );
    }
    if facts.quality_context_sufficient == Some(false) {
        return blocked(
            "restart_quality_with_required_context_tokens",
            "quality_context_window_insufficient",
        );
    }
    if facts.quality_cpu_fallback || facts.quality_zero_gpu_layers {
        return blocked(
            "fix_quality_metal_or_gpu_layers_before_expansion",
            "quality_worker_not_gpu_accelerated",
        );
    }
    if matches!(
        facts.capacity_recommendation.as_deref(),
        Some("restore_quality_gate_first")
    ) {
        return blocked(
            "restore_quality_gate_first",
            "capacity_gate_blocks_expansion",
        );
    }
    if matches!(
        facts.capacity_recommendation.as_deref(),
        Some("review_model_cell_policy_movement_before_expansion")
    ) {
        return blocked(
            "review_model_cell_policy_movement_before_expansion",
            "model_cell_policy_movement_review_required",
        );
    }
    if facts.extra_quality_12b_detected() {
        return blocked(
            "stop_extra_quality_12b_workers_keep_one_quality_plus_helpers",
            "extra_quality_12b_wastes_shared_apple_memory",
        );
    }
    if facts.helper_cpu_or_no_gpu_detected() {
        return blocked(
            "fix_helper_metal_or_gpu_layers_before_more_pool_workers",
            "helper_workers_not_gpu_accelerated",
        );
    }
    if facts.full_helper_pool_visible() {
        return allowed(
            "run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls",
            "full_helper_pool_visible",
        );
    }
    if facts.has_summary
        && (facts.has_router || facts.has_review || facts.has_index || facts.has_test_gate)
    {
        return allowed(
            "add_remaining_helper_roles_one_at_a_time",
            "partial_helper_pool_visible",
        );
    }
    if facts.has_summary {
        return allowed(
            "add_review_or_index_after_short_smoke",
            "summary_worker_visible",
        );
    }
    allowed(
        "add_summary_worker_first",
        "quality_chain_ready_no_helpers_visible",
    )
}

pub fn model_pool_advice_text_zh(facts: &ModelPoolFacts, decision: &ModelPoolDecision) -> String {
    let suffix = "; 不要多开 12B，优先一主多小";
    let context = context_text(facts);
    match decision.reason {
        "quality_worker_not_ready" => {
            format!("模型池建议：先恢复 quality 12B(8686)，{context}{suffix}")
        }
        "quality_context_window_insufficient" => {
            format!("模型池建议：重启 quality 并提高上下文窗口，{context}{suffix}")
        }
        "quality_worker_not_gpu_accelerated" => {
            format!("模型池建议：先修 Metal/GPU 或 gpu_layers，再加小模型{suffix}")
        }
        "capacity_gate_blocks_expansion" => {
            format!(
                "模型池建议：先恢复 quality gate，再考虑 summary/router/review/index/test-gate{suffix}"
            )
        }
        "model_cell_policy_movement_review_required" => {
            format!("模型池建议：先补 model-cell policy 移动审查，再扩展 helper 角色{suffix}")
        }
        "extra_quality_12b_wastes_shared_apple_memory" => {
            format!(
                "模型池建议：检测到多个 quality 12B，先停掉多余大模型，只保留 1 个 12B 主力，再挂 summary/router/review/index/test-gate 小模型{suffix}"
            )
        }
        "helper_workers_not_gpu_accelerated" => {
            let blocking_roles = facts.blocking_helper_cpu_or_no_gpu_roles();
            let roles = if blocking_roles.is_empty() {
                "unknown".to_owned()
            } else {
                blocking_roles.join(",")
            };
            format!(
                "模型池建议：helper 小模型仍在 CPU/无 GPU 路径({roles})，先修 Metal/gpu_layers 再继续扩池{suffix}"
            )
        }
        "quality_chain_ready_no_helpers_visible" => {
            format!("模型池建议：quality 可用，先加 summary 小模型，{context}{suffix}")
        }
        "summary_worker_visible" => {
            format!("模型池建议：summary 已可用，短 smoke 后补 review 或 index 小模型{suffix}")
        }
        "partial_helper_pool_visible" => {
            format!(
                "模型池建议：已有部分 helper，短 smoke 后按 summary/router/review/index/test-gate 补齐{suffix}"
            )
        }
        "full_helper_pool_visible" => {
            format!(
                "模型池建议：helper 池已成形，可用 /pool-call 与 evolution-loop helper 阶段联调{suffix}"
            )
        }
        _ => format!("模型池建议：按一主多小策略继续检查模型池，{context}{suffix}"),
    }
}

pub fn context_text(facts: &ModelPoolFacts) -> String {
    match (
        facts.quality_context_tokens.as_deref(),
        facts.quality_required_context_tokens.as_deref(),
    ) {
        (Some(actual), Some(required)) => format!("ctx {actual}/{required}"),
        (Some(actual), None) => format!("ctx {actual}"),
        _ => "ctx unknown".to_owned(),
    }
}

fn blocked(next_step: &'static str, reason: &'static str) -> ModelPoolDecision {
    ModelPoolDecision {
        safe_to_enable_pool_workers: false,
        next_step,
        reason,
        kind: AdviceKind::Error,
    }
}

fn allowed(next_step: &'static str, reason: &'static str) -> ModelPoolDecision {
    ModelPoolDecision {
        safe_to_enable_pool_workers: true,
        next_step,
        reason,
        kind: AdviceKind::Busy,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTaskKind {
    General,
    Code,
    Vision,
    Embedding,
    Safety,
}

impl ModelTaskKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Code => "code",
            Self::Vision => "vision",
            Self::Embedding => "embedding",
            Self::Safety => "safety",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelCallFailureClass {
    Timeout,
    Unauthorized,
    ProviderNotFound,
    MalformedResponse,
    EmptyOutput,
    QualityGate,
    Unavailable,
}

impl ModelCallFailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Unauthorized => "unauthorized",
            Self::ProviderNotFound => "provider_not_found",
            Self::MalformedResponse => "malformed_response",
            Self::EmptyOutput => "empty_output",
            Self::QualityGate => "quality_gate",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelCallStatus {
    Success,
    Failed(ModelCallFailureClass),
}

impl ModelCallStatus {
    pub fn failure_class(self) -> Option<ModelCallFailureClass> {
        match self {
            Self::Success => None,
            Self::Failed(failure) => Some(failure),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed(failure) => failure.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCallCandidate {
    pub model_id: String,
    pub task_role: String,
    pub status: ModelCallStatus,
    pub latency_ms: Option<u64>,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub supports_code: bool,
    pub supports_vision: bool,
    pub supports_embedding: bool,
    pub consecutive_failures: u32,
    pub last_failure_class: Option<ModelCallFailureClass>,
}

impl ModelCallCandidate {
    pub fn success(
        model_id: impl Into<String>,
        task_role: impl Into<String>,
        latency_ms: u64,
        completion_tokens: u64,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            task_role: task_role.into(),
            status: ModelCallStatus::Success,
            latency_ms: Some(latency_ms),
            prompt_tokens: 0,
            completion_tokens,
            supports_code: false,
            supports_vision: false,
            supports_embedding: false,
            consecutive_failures: 0,
            last_failure_class: None,
        }
    }

    pub fn failed(
        model_id: impl Into<String>,
        task_role: impl Into<String>,
        failure: ModelCallFailureClass,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            task_role: task_role.into(),
            status: ModelCallStatus::Failed(failure),
            latency_ms: None,
            prompt_tokens: 0,
            completion_tokens: 0,
            supports_code: false,
            supports_vision: false,
            supports_embedding: false,
            consecutive_failures: 1,
            last_failure_class: Some(failure),
        }
    }

    pub fn with_prompt_tokens(mut self, prompt_tokens: u64) -> Self {
        self.prompt_tokens = prompt_tokens;
        self
    }

    pub fn with_code_capability(mut self) -> Self {
        self.supports_code = true;
        self
    }

    pub fn with_vision_capability(mut self) -> Self {
        self.supports_vision = true;
        self
    }

    pub fn with_embedding_capability(mut self) -> Self {
        self.supports_embedding = true;
        self
    }

    pub fn with_consecutive_failures(mut self, failures: u32) -> Self {
        self.consecutive_failures = failures;
        self
    }

    pub fn with_last_failure(mut self, failure: ModelCallFailureClass) -> Self {
        self.last_failure_class = Some(failure);
        self
    }

    pub fn tokens_per_second(&self) -> Option<f64> {
        let latency_ms = self.latency_ms?;
        if latency_ms == 0 || self.completion_tokens == 0 {
            return None;
        }
        Some(self.completion_tokens as f64 * 1000.0 / latency_ms as f64)
    }

    pub fn matches_task(&self, task: ModelTaskKind) -> bool {
        match task {
            ModelTaskKind::General => true,
            ModelTaskKind::Code => self.supports_code || self.task_role == "code",
            ModelTaskKind::Vision => self.supports_vision,
            ModelTaskKind::Embedding => self.supports_embedding,
            ModelTaskKind::Safety => self.task_role == "safety",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelFallbackPolicy {
    pub max_attempts: usize,
    pub max_total_latency_ms: u64,
    pub cooldown_failure_threshold: u32,
}

impl Default for ModelFallbackPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 2,
            max_total_latency_ms: 60_000,
            cooldown_failure_threshold: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFallbackPlan {
    pub primary_model: String,
    pub fallback_model: Option<String>,
    pub failure_class: Option<ModelCallFailureClass>,
    pub fallback_used: bool,
    pub fallback_success: bool,
    pub fallback_latency_ms: Option<u64>,
    pub model_pool_degraded: bool,
    pub bounded_failure_reason: Option<String>,
    pub attempted_models: Vec<String>,
}

impl ModelFallbackPlan {
    pub fn evidence_line(&self) -> String {
        format!(
            "model_fallback primary_model={} fallback_model={} failure_class={} fallback_used={} fallback_success={} fallback_latency_ms={} model_pool_degraded={} attempts={} bounded_failure_reason={}",
            safe_evidence_token(&self.primary_model),
            self.fallback_model
                .as_deref()
                .map(safe_evidence_token)
                .unwrap_or_else(|| "none".to_owned()),
            self.failure_class
                .map(ModelCallFailureClass::as_str)
                .unwrap_or("none"),
            self.fallback_used,
            self.fallback_success,
            self.fallback_latency_ms
                .map(|latency| latency.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.model_pool_degraded,
            self.attempted_models.len(),
            self.bounded_failure_reason
                .as_deref()
                .map(safe_evidence_token)
                .unwrap_or_else(|| "none".to_owned())
        )
    }
}

pub fn predictive_model_fallback_preflight(
    candidates: &[ModelCallCandidate],
    task: ModelTaskKind,
    policy: ModelFallbackPolicy,
) -> ModelFallbackPlan {
    let ranked = ranked_available_candidates(candidates, task, policy);
    let primary = ranked.first().copied();
    let fallback = ranked.get(1).copied();

    ModelFallbackPlan {
        primary_model: primary
            .map(|candidate| candidate.model_id.clone())
            .unwrap_or_else(|| "none".to_owned()),
        fallback_model: fallback.map(|candidate| candidate.model_id.clone()),
        failure_class: None,
        fallback_used: false,
        fallback_success: primary.is_some(),
        fallback_latency_ms: None,
        model_pool_degraded: primary.is_none() || fallback.is_none(),
        bounded_failure_reason: if primary.is_none() {
            Some("no_available_model_candidate".to_owned())
        } else {
            None
        },
        attempted_models: primary
            .into_iter()
            .map(|candidate| candidate.model_id.clone())
            .collect(),
    }
}

pub fn model_fallback_plan_after_failure(
    primary_model: impl Into<String>,
    failure_class: ModelCallFailureClass,
    candidates: &[ModelCallCandidate],
    task: ModelTaskKind,
    policy: ModelFallbackPolicy,
) -> ModelFallbackPlan {
    let primary_model = primary_model.into();
    let primary_latency = candidates
        .iter()
        .find(|candidate| candidate.model_id == primary_model)
        .and_then(|candidate| candidate.latency_ms)
        .unwrap_or(0);
    let mut attempted_models = vec![primary_model.clone()];
    let mut fallback = None;

    if policy.max_attempts > 1 {
        fallback = ranked_available_candidates(candidates, task, policy)
            .into_iter()
            .find(|candidate| {
                candidate.model_id != primary_model
                    && !same_failure_loop(candidate, failure_class)
                    && primary_latency.saturating_add(candidate.latency_ms.unwrap_or(0))
                        <= policy.max_total_latency_ms
            });
    }

    if let Some(fallback) = fallback {
        attempted_models.push(fallback.model_id.clone());
        return ModelFallbackPlan {
            primary_model,
            fallback_model: Some(fallback.model_id.clone()),
            failure_class: Some(failure_class),
            fallback_used: true,
            fallback_success: true,
            fallback_latency_ms: fallback.latency_ms,
            model_pool_degraded: false,
            bounded_failure_reason: None,
            attempted_models,
        };
    }

    let reason = if policy.max_attempts <= 1 {
        "fallback_attempt_budget_exhausted"
    } else if candidates.iter().any(|candidate| {
        candidate.model_id != primary_model
            && candidate.matches_task(task)
            && same_failure_loop(candidate, failure_class)
    }) {
        "fallback_blocked_same_failure_class_or_cooldown"
    } else {
        "no_available_fallback_candidate"
    };

    ModelFallbackPlan {
        primary_model,
        fallback_model: None,
        failure_class: Some(failure_class),
        fallback_used: false,
        fallback_success: false,
        fallback_latency_ms: None,
        model_pool_degraded: true,
        bounded_failure_reason: Some(reason.to_owned()),
        attempted_models,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelPoolLiveSmokePolicy {
    pub min_available_models: usize,
    pub max_latency_ms: u64,
    pub require_code_capable: bool,
}

impl Default for ModelPoolLiveSmokePolicy {
    fn default() -> Self {
        Self {
            min_available_models: 1,
            max_latency_ms: 30_000,
            require_code_capable: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPoolLiveSmokeReport {
    pub passed: bool,
    pub failures: Vec<String>,
    pub available_models: usize,
    pub unavailable_models: usize,
    pub unauthorized_models: usize,
    pub timeout_models: usize,
    pub max_observed_latency_ms: Option<u64>,
    pub evidence_line: String,
}

pub fn evaluate_live_model_pool_smoke(
    candidates: &[ModelCallCandidate],
    policy: ModelPoolLiveSmokePolicy,
) -> ModelPoolLiveSmokeReport {
    let available_models = candidates
        .iter()
        .filter(|candidate| candidate.status == ModelCallStatus::Success)
        .count();
    let unavailable_models = candidates
        .iter()
        .filter(|candidate| {
            candidate.status.failure_class() == Some(ModelCallFailureClass::ProviderNotFound)
                || candidate.status.failure_class() == Some(ModelCallFailureClass::Unavailable)
        })
        .count();
    let unauthorized_models = candidates
        .iter()
        .filter(|candidate| {
            candidate.status.failure_class() == Some(ModelCallFailureClass::Unauthorized)
        })
        .count();
    let timeout_models = candidates
        .iter()
        .filter(|candidate| {
            candidate.status.failure_class() == Some(ModelCallFailureClass::Timeout)
        })
        .count();
    let max_observed_latency_ms = candidates
        .iter()
        .filter(|candidate| candidate.status == ModelCallStatus::Success)
        .filter_map(|candidate| candidate.latency_ms)
        .max();

    let mut failures = Vec::new();
    if available_models < policy.min_available_models {
        failures.push(format!(
            "available_models={available_models}<{}",
            policy.min_available_models
        ));
    }
    if let Some(latency) = max_observed_latency_ms
        && latency > policy.max_latency_ms
    {
        failures.push(format!(
            "max_latency_ms={latency}>{}",
            policy.max_latency_ms
        ));
    }
    if policy.require_code_capable
        && !candidates.iter().any(|candidate| {
            candidate.status == ModelCallStatus::Success && candidate.supports_code
        })
    {
        failures.push("code_capable_success_missing".to_owned());
    }

    let passed = failures.is_empty();
    let evidence_line = format!(
        "model_pool_live_smoke passed={} available_models={} unavailable_models={} unauthorized_models={} timeout_models={} max_latency_ms={} statuses={} failures={}",
        passed,
        available_models,
        unavailable_models,
        unauthorized_models,
        timeout_models,
        max_observed_latency_ms
            .map(|latency| latency.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        smoke_status_csv(candidates),
        if failures.is_empty() {
            "none".to_owned()
        } else {
            failures.join("|")
        }
    );

    ModelPoolLiveSmokeReport {
        passed,
        failures,
        available_models,
        unavailable_models,
        unauthorized_models,
        timeout_models,
        max_observed_latency_ms,
        evidence_line,
    }
}

pub fn model_pool_evidence_is_sanitized(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ![
        "sk-",
        "bearer ",
        "api_key",
        "authorization",
        "http://",
        "https://",
        "account",
        "raw_response",
        "raw_prompt",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn ranked_available_candidates(
    candidates: &[ModelCallCandidate],
    task: ModelTaskKind,
    policy: ModelFallbackPolicy,
) -> Vec<&ModelCallCandidate> {
    let mut ranked = candidates
        .iter()
        .filter(|candidate| candidate.status == ModelCallStatus::Success)
        .filter(|candidate| candidate.matches_task(task))
        .filter(|candidate| candidate.consecutive_failures < policy.cooldown_failure_threshold)
        .collect::<Vec<_>>();

    ranked.sort_by_key(|candidate| {
        (
            capability_rank(candidate, task),
            candidate.latency_ms.unwrap_or(u64::MAX),
            candidate.model_id.clone(),
        )
    });
    ranked
}

fn capability_rank(candidate: &ModelCallCandidate, task: ModelTaskKind) -> u8 {
    match task {
        ModelTaskKind::General => 0,
        ModelTaskKind::Code => {
            u8::from(!(candidate.supports_code || candidate.task_role == "code"))
        }
        ModelTaskKind::Vision => u8::from(!candidate.supports_vision),
        ModelTaskKind::Embedding => u8::from(!candidate.supports_embedding),
        ModelTaskKind::Safety => u8::from(candidate.task_role != "safety"),
    }
}

fn same_failure_loop(candidate: &ModelCallCandidate, failure_class: ModelCallFailureClass) -> bool {
    candidate.last_failure_class == Some(failure_class) && candidate.consecutive_failures > 0
}

fn smoke_status_csv(candidates: &[ModelCallCandidate]) -> String {
    if candidates.is_empty() {
        return "none".to_owned();
    }
    candidates
        .iter()
        .map(|candidate| {
            format!(
                "{}:{}:{}:{}",
                safe_evidence_token(&candidate.model_id),
                candidate.status.as_str(),
                candidate
                    .latency_ms
                    .map(|latency| latency.to_string())
                    .unwrap_or_else(|| "none".to_owned()),
                if candidate.supports_code {
                    "code"
                } else {
                    "general"
                }
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn safe_evidence_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' | '/' | ':' => ch,
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_extra_quality_12b_workers() {
        let facts = ModelPoolFacts {
            quality_ready: Some(true),
            quality_context_sufficient: Some(true),
            quality_runtime_accelerated: Some(true),
            has_summary: true,
            quality_worker_count: 2,
            helper_worker_count: 1,
            ..ModelPoolFacts::default()
        };

        let decision = model_pool_decision(&facts);

        assert!(facts.extra_quality_12b_detected());
        assert!(!decision.safe_to_enable_pool_workers);
        assert_eq!(
            decision.next_step,
            "stop_extra_quality_12b_workers_keep_one_quality_plus_helpers"
        );
        assert_eq!(
            decision.reason,
            "extra_quality_12b_wastes_shared_apple_memory"
        );
    }

    #[test]
    fn recommends_full_helper_pool_use() {
        let facts = ModelPoolFacts {
            quality_ready: Some(true),
            quality_context_sufficient: Some(true),
            quality_runtime_accelerated: Some(true),
            has_summary: true,
            has_router: true,
            has_review: true,
            has_index: true,
            has_test_gate: true,
            quality_worker_count: 1,
            helper_worker_count: HELPER_TARGET_WORKERS,
            ..ModelPoolFacts::default()
        };

        let decision = model_pool_decision(&facts);

        assert!(decision.safe_to_enable_pool_workers);
        assert_eq!(
            decision.next_step,
            "run_short_pool_smoke_then_use_evolution_loop_helper_stage_calls"
        );
        assert!(model_pool_advice_text_zh(&facts, &decision).contains("helper 池已成形"));
    }

    #[test]
    fn treats_summary_plus_test_gate_as_partial_helper_pool() {
        let facts = ModelPoolFacts {
            quality_ready: Some(true),
            quality_context_sufficient: Some(true),
            quality_runtime_accelerated: Some(true),
            has_summary: true,
            has_test_gate: true,
            quality_worker_count: 1,
            helper_worker_count: 2,
            ..ModelPoolFacts::default()
        };

        let decision = model_pool_decision(&facts);

        assert!(decision.safe_to_enable_pool_workers);
        assert_eq!(
            decision.next_step,
            "add_remaining_helper_roles_one_at_a_time"
        );
        assert_eq!(decision.reason, "partial_helper_pool_visible");
        assert_eq!(
            missing_helper_roles(&facts),
            vec!["router", "review", "index"]
        );
    }

    #[test]
    fn blocks_more_pool_workers_when_helpers_are_cpu_or_no_gpu() {
        let facts = ModelPoolFacts {
            quality_ready: Some(true),
            quality_context_sufficient: Some(true),
            quality_runtime_accelerated: Some(true),
            has_summary: true,
            has_review: true,
            quality_worker_count: 1,
            helper_worker_count: 2,
            helper_cpu_or_no_gpu_roles: vec!["review".to_owned()],
            ..ModelPoolFacts::default()
        };

        let decision = model_pool_decision(&facts);

        assert!(facts.helper_cpu_or_no_gpu_detected());
        assert!(!decision.safe_to_enable_pool_workers);
        assert_eq!(
            decision.next_step,
            "fix_helper_metal_or_gpu_layers_before_more_pool_workers"
        );
        assert_eq!(decision.reason, "helper_workers_not_gpu_accelerated");
        assert!(
            model_pool_advice_text_zh(&facts, &decision)
                .contains("helper 小模型仍在 CPU/无 GPU 路径(review)")
        );
    }

    #[test]
    fn blocks_when_model_cell_policy_movement_review_is_required() {
        let facts = ModelPoolFacts {
            quality_ready: Some(true),
            quality_context_sufficient: Some(true),
            quality_runtime_accelerated: Some(true),
            capacity_recommendation: Some(
                "review_model_cell_policy_movement_before_expansion".to_owned(),
            ),
            has_summary: true,
            has_router: true,
            has_review: true,
            has_index: true,
            has_test_gate: true,
            quality_worker_count: 1,
            helper_worker_count: HELPER_TARGET_WORKERS,
            ..ModelPoolFacts::default()
        };

        let decision = model_pool_decision(&facts);

        assert!(!decision.safe_to_enable_pool_workers);
        assert_eq!(
            decision.next_step,
            "review_model_cell_policy_movement_before_expansion"
        );
        assert_eq!(
            decision.reason,
            "model_cell_policy_movement_review_required"
        );
        assert!(model_pool_advice_text_zh(&facts, &decision).contains("移动审查"));
    }

    #[test]
    fn allows_index_cpu_fallback_for_low_priority_index_work() {
        let facts = ModelPoolFacts {
            quality_ready: Some(true),
            quality_context_sufficient: Some(true),
            quality_runtime_accelerated: Some(true),
            has_summary: true,
            has_router: true,
            has_review: true,
            has_index: true,
            has_test_gate: true,
            quality_worker_count: 1,
            helper_worker_count: HELPER_TARGET_WORKERS,
            helper_cpu_or_no_gpu_roles: vec!["index".to_owned()],
            ..ModelPoolFacts::default()
        };

        let decision = model_pool_decision(&facts);

        assert_eq!(
            facts.blocking_helper_cpu_or_no_gpu_roles(),
            Vec::<&str>::new()
        );
        assert_eq!(facts.allowed_cpu_fallback_helper_roles(), vec!["index"]);
        assert!(!facts.helper_cpu_or_no_gpu_detected());
        assert!(decision.safe_to_enable_pool_workers);
        assert_eq!(decision.reason, "full_helper_pool_visible");
    }

    #[test]
    fn launch_order_contract_matches_helper_roles() {
        assert_eq!(CAPACITY_POLICY, "one_quality_plus_small_helpers");
        assert_eq!(RECOMMENDED_LAUNCH_ROLES[0], QUALITY_ROLE);
        assert_eq!(&RECOMMENDED_LAUNCH_ROLES[1..], HELPER_ROLES);
        assert_eq!(RECOMMENDED_LAUNCH_ROLES.join(","), RECOMMENDED_LAUNCH_ORDER);
    }

    #[test]
    fn fallback_plan_uses_available_backup_after_primary_failure() {
        let candidates = vec![
            ModelCallCandidate::failed(
                "deepseek-ai/deepseek-coder-6.7b-instruct",
                "code",
                ModelCallFailureClass::ProviderNotFound,
            )
            .with_code_capability(),
            ModelCallCandidate::success("meta/llama-3.1-8b-instruct", "code", 2886, 126)
                .with_prompt_tokens(31)
                .with_code_capability(),
        ];

        let plan = model_fallback_plan_after_failure(
            "deepseek-ai/deepseek-coder-6.7b-instruct",
            ModelCallFailureClass::ProviderNotFound,
            &candidates,
            ModelTaskKind::Code,
            ModelFallbackPolicy::default(),
        );

        assert_eq!(
            plan.primary_model,
            "deepseek-ai/deepseek-coder-6.7b-instruct"
        );
        assert_eq!(
            plan.fallback_model.as_deref(),
            Some("meta/llama-3.1-8b-instruct")
        );
        assert_eq!(
            plan.failure_class,
            Some(ModelCallFailureClass::ProviderNotFound)
        );
        assert!(plan.fallback_used);
        assert!(plan.fallback_success);
        assert_eq!(plan.fallback_latency_ms, Some(2886));
        assert!(!plan.model_pool_degraded);
        assert!(model_pool_evidence_is_sanitized(&plan.evidence_line()));
    }

    #[test]
    fn fallback_plan_bounds_all_failed_candidates() {
        let candidates = vec![
            ModelCallCandidate::failed(
                "primary-model",
                "code",
                ModelCallFailureClass::Unauthorized,
            )
            .with_code_capability(),
            ModelCallCandidate::failed(
                "backup-model",
                "code",
                ModelCallFailureClass::ProviderNotFound,
            )
            .with_code_capability(),
        ];

        let plan = model_fallback_plan_after_failure(
            "primary-model",
            ModelCallFailureClass::Unauthorized,
            &candidates,
            ModelTaskKind::Code,
            ModelFallbackPolicy::default(),
        );

        assert_eq!(plan.fallback_model, None);
        assert!(!plan.fallback_used);
        assert!(!plan.fallback_success);
        assert!(plan.model_pool_degraded);
        assert_eq!(
            plan.bounded_failure_reason.as_deref(),
            Some("no_available_fallback_candidate")
        );
        assert_eq!(plan.attempted_models, vec!["primary-model"]);
    }

    #[test]
    fn fallback_plan_avoids_same_failure_cooldown_loop() {
        let candidates = vec![
            ModelCallCandidate::failed("primary-model", "code", ModelCallFailureClass::Timeout)
                .with_code_capability(),
            ModelCallCandidate::success("cooldown-model", "code", 900, 64)
                .with_code_capability()
                .with_consecutive_failures(2)
                .with_last_failure(ModelCallFailureClass::Timeout),
        ];

        let plan = model_fallback_plan_after_failure(
            "primary-model",
            ModelCallFailureClass::Timeout,
            &candidates,
            ModelTaskKind::Code,
            ModelFallbackPolicy::default(),
        );

        assert!(!plan.fallback_success);
        assert!(plan.model_pool_degraded);
        assert_eq!(
            plan.bounded_failure_reason.as_deref(),
            Some("fallback_blocked_same_failure_class_or_cooldown")
        );
    }

    #[test]
    fn live_smoke_gate_reports_latency_and_availability_without_secrets() {
        let candidates = vec![
            ModelCallCandidate::success("meta/llama-3.1-8b-instruct", "code", 2886, 126)
                .with_prompt_tokens(31)
                .with_code_capability(),
            ModelCallCandidate::success("qwen/qwen3-next-80b-a3b-instruct", "code", 45_441, 51)
                .with_code_capability(),
            ModelCallCandidate::failed(
                "mistralai/codestral-22b-instruct-v0.1",
                "code",
                ModelCallFailureClass::Unauthorized,
            )
            .with_code_capability(),
            ModelCallCandidate::failed(
                "deepseek-ai/deepseek-coder-6.7b-instruct",
                "code",
                ModelCallFailureClass::ProviderNotFound,
            )
            .with_code_capability(),
        ];

        let report = evaluate_live_model_pool_smoke(
            &candidates,
            ModelPoolLiveSmokePolicy {
                min_available_models: 1,
                max_latency_ms: 60_000,
                require_code_capable: true,
            },
        );

        assert!(report.passed, "{:?}", report.failures);
        assert_eq!(report.available_models, 2);
        assert_eq!(report.unavailable_models, 1);
        assert_eq!(report.unauthorized_models, 1);
        assert_eq!(report.max_observed_latency_ms, Some(45_441));
        assert!(report.evidence_line.contains("model_pool_live_smoke"));
        assert!(report.evidence_line.contains("unauthorized_models=1"));
        assert!(model_pool_evidence_is_sanitized(&report.evidence_line));
    }
}
