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
}
