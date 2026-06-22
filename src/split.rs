//! Namespaced access to the split Norion crates.
//!
//! The root crate keeps its existing monolith-facing API stable while the
//! modular crates mature. This module makes the split boundaries visible to
//! downstream callers without flattening every type into the root namespace.
//!
//! Use these namespaces when new code wants the smaller crate contracts
//! directly, but keep the existing root re-exports for monolith-compatible
//! callers while the migration is still in progress.

pub use norion_agent as agent;
pub use norion_cli as cli;
pub use norion_core as core;
pub use norion_eval as eval;
pub use norion_memory as memory;
pub use norion_service as service;

pub mod bridge {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct KvFusionRewardPreviewPolicy {
        pub max_abs_reward: f32,
        pub require_runtime_kv_source: bool,
    }

    impl Default for KvFusionRewardPreviewPolicy {
        fn default() -> Self {
            Self {
                max_abs_reward: 1.0,
                require_runtime_kv_source: true,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct KvFusionRewardPreviewItem {
        pub task_id: String,
        pub record_id: String,
        pub source: String,
        pub action: super::agent::AgentRecallOutcomeAttributionAction,
        pub amount: f32,
        pub reward: f32,
        pub reason_codes: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct KvFusionRewardPreviewReport {
        pub source_read_only: bool,
        pub source_memory_store_write_allowed: bool,
        pub preview_only: bool,
        pub memory_store_write_allowed: bool,
        pub kv_cache_write_allowed: bool,
        pub policy_observation_ready: bool,
        pub policy_observation_applied: bool,
        pub reward: f32,
        pub items: Vec<KvFusionRewardPreviewItem>,
        pub reinforced_count: usize,
        pub penalized_count: usize,
        pub skipped_non_runtime_kv_count: usize,
        pub skipped_non_finite_amount_count: usize,
        pub blocked_reasons: Vec<String>,
        pub telemetry: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct KvFusionPolicyObservationDryRunReport {
        pub preview_only: bool,
        pub policy_observation_ready: bool,
        pub policy_observation_applied: bool,
        pub policy_write_allowed: bool,
        pub reward_preview_source_read_only: bool,
        pub reward_preview_source_memory_store_write_allowed: bool,
        pub reward_preview_memory_store_write_allowed: bool,
        pub reward_preview_kv_cache_write_allowed: bool,
        pub reward: f32,
        pub previous_similarity_threshold: f32,
        pub preview_similarity_threshold: f32,
        pub threshold_delta: f32,
        pub threshold_changed: bool,
        pub threshold_within_bounds: bool,
        pub blocked_reasons: Vec<String>,
        pub telemetry: Vec<String>,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct RootRouteBudgetReadinessProjection {
        pub route_budget: super::core::RouteBudget,
        pub readiness: super::core::RouteBudgetReadinessSummary,
        pub commit: super::core::RouteBudgetReadinessCommitSummary,
        pub synthetic_decision_summary: bool,
    }

    impl RootRouteBudgetReadinessProjection {
        pub fn can_commit(self) -> bool {
            self.commit.can_admit_committed_route_budget()
        }
    }

    pub fn root_task_profile_to_core(
        profile: crate::hierarchy::TaskProfile,
    ) -> super::core::TaskProfile {
        match profile {
            crate::hierarchy::TaskProfile::General => super::core::TaskProfile::General,
            crate::hierarchy::TaskProfile::Coding => super::core::TaskProfile::Coding,
            crate::hierarchy::TaskProfile::Writing => super::core::TaskProfile::Writing,
            crate::hierarchy::TaskProfile::LongDocument => super::core::TaskProfile::LongDocument,
        }
    }

    pub fn root_generation_metrics_to_core(
        metrics: crate::router::GenerationMetrics,
    ) -> super::core::GenerationMetrics {
        super::core::GenerationMetrics {
            perplexity: metrics.perplexity,
            semantic_consistency: metrics.semantic_consistency,
            contradiction_count: metrics.contradiction_count,
            token_count: metrics.token_count,
        }
    }

    pub fn root_generation_metrics_to_core_routing_feedback_summary(
        profile: crate::hierarchy::TaskProfile,
        metrics: crate::router::GenerationMetrics,
    ) -> super::core::RoutingFeedbackSummary {
        super::core::RoutingFeedback {
            profile: root_task_profile_to_core(profile),
            quality: metrics.quality_score(),
            perplexity: metrics.perplexity,
            contradiction_count: metrics.contradiction_count,
        }
        .feedback_summary()
    }

    pub fn root_generation_metrics_to_core_hierarchy_feedback_summary(
        profile: crate::hierarchy::TaskProfile,
        metrics: crate::router::GenerationMetrics,
    ) -> super::core::HierarchyAdjustmentFeedbackSummary {
        super::core::HierarchyAdjustmentFeedback::new(
            root_task_profile_to_core(profile),
            metrics.quality_score(),
            metrics.perplexity,
            metrics.contradiction_count,
        )
        .feedback_summary()
    }

    pub fn root_route_budget_to_core(
        budget: crate::router::RouteBudget,
    ) -> super::core::RouteBudget {
        super::core::RouteBudget {
            threshold: budget.threshold,
            attention_tokens: budget.attention_tokens,
            fast_tokens: budget.fast_tokens,
            attention_fraction: budget.attention_fraction,
        }
    }

    pub fn root_route_budget_to_core_readiness(
        budget: crate::router::RouteBudget,
    ) -> RootRouteBudgetReadinessProjection {
        let route_budget = root_route_budget_to_core(budget);
        let decision_summary = synthetic_route_decision_summary_from_budget(route_budget);
        root_route_budget_to_core_readiness_projection(decision_summary, budget, true)
    }

    pub fn root_route_budget_to_core_readiness_from_decision_summary(
        decision_summary: super::core::RoutingDecisionSummary,
        budget: crate::router::RouteBudget,
    ) -> RootRouteBudgetReadinessProjection {
        root_route_budget_to_core_readiness_projection(decision_summary, budget, false)
    }

    fn root_route_budget_to_core_readiness_projection(
        decision_summary: super::core::RoutingDecisionSummary,
        budget: crate::router::RouteBudget,
        synthetic_decision_summary: bool,
    ) -> RootRouteBudgetReadinessProjection {
        let route_budget = root_route_budget_to_core(budget);
        let readiness =
            super::core::RouteBudgetReadinessSummary::new(decision_summary, route_budget);
        let commit = readiness.commit_summary();

        RootRouteBudgetReadinessProjection {
            route_budget,
            readiness,
            commit,
            synthetic_decision_summary,
        }
    }

    pub fn root_profile_thresholds_to_core(
        thresholds: crate::router::ProfileThresholds,
    ) -> super::core::ProfileThresholds {
        super::core::ProfileThresholds {
            general: thresholds.general,
            coding: thresholds.coding,
            writing: thresholds.writing,
            long_document: thresholds.long_document,
        }
    }

    pub fn root_profile_observations_to_core(
        observations: crate::router::ProfileObservations,
    ) -> super::core::ProfileObservations {
        super::core::ProfileObservations {
            general: observations.general,
            coding: observations.coding,
            writing: observations.writing,
            long_document: observations.long_document,
        }
    }

    pub fn root_router_state_to_core(
        state: crate::router::RouterState,
    ) -> super::core::RouterState {
        super::core::RouterState {
            threshold: state.threshold,
            observations: state.observations,
            profile_thresholds: root_profile_thresholds_to_core(state.profile_thresholds),
            profile_observations: root_profile_observations_to_core(state.profile_observations),
        }
    }

    pub fn root_hierarchy_weights_to_core(
        weights: crate::hierarchy::HierarchyWeights,
    ) -> super::core::HierarchyWeights {
        super::core::HierarchyWeights::new(weights.global, weights.local, weights.convolution)
    }

    pub fn root_hierarchy_weights_to_core_summary(
        weights: crate::hierarchy::HierarchyWeights,
    ) -> super::core::HierarchyWeightsSummary {
        root_hierarchy_weights_to_core(weights).summary()
    }

    pub fn memory_reuse_dry_run_to_agent_evidence(
        summary: &super::memory::MemoryReuseDryRunSummary,
    ) -> super::agent::MemoryRecallDryRunEvidence {
        super::agent::MemoryRecallDryRunEvidence {
            source: "norion_memory_reuse_dry_run".to_owned(),
            read_only: summary.read_only,
            candidate_count: summary.candidate_count,
            long_term_match_count: summary.long_term_match_count,
            context_decision_count: summary.context_decision_count,
            accepted_context_count: summary.accepted_context_count,
            rejected_context_count: summary.rejected_context_count,
            used_tokens: summary.used_tokens,
            requested_kv_count: summary.requested_kv_count,
            kv_promote_count: summary.kv_promote_count,
            kv_missing_count: summary.kv_missing_count,
            kv_already_hot_count: summary.kv_already_hot_count,
            kv_duplicate_count: summary.kv_duplicate_count,
            kv_backend_available: summary.kv_backend_available,
            memory_store_write_allowed: summary.memory_store_write_allowed,
            kv_prefetch_apply_allowed: summary.kv_prefetch_apply_allowed,
            reason_codes: summary.reason_codes.clone(),
            detail_codes: summary.detail_codes.clone(),
        }
    }

    pub fn recall_outcome_attribution_to_kv_fusion_reward_preview(
        report: &super::agent::AgentRecallOutcomeAttributionReport,
    ) -> KvFusionRewardPreviewReport {
        recall_outcome_attribution_to_kv_fusion_reward_preview_with_policy(
            report,
            KvFusionRewardPreviewPolicy::default(),
        )
    }

    pub fn recall_outcome_attribution_to_kv_fusion_reward_preview_with_policy(
        report: &super::agent::AgentRecallOutcomeAttributionReport,
        policy: KvFusionRewardPreviewPolicy,
    ) -> KvFusionRewardPreviewReport {
        let max_abs_reward = normalized_max_abs_reward(policy.max_abs_reward);
        let mut blocked_reasons = Vec::new();
        if !report.read_only {
            blocked_reasons.push("recall_attribution_not_read_only".to_owned());
        }
        if report.memory_store_write_allowed {
            blocked_reasons.push("recall_attribution_memory_store_write_allowed".to_owned());
        }

        if !blocked_reasons.is_empty() {
            return kv_fusion_reward_preview_report(report, Vec::new(), 0, 0, 0.0, blocked_reasons);
        }

        let mut items = Vec::new();
        let mut skipped_non_runtime_kv_count = 0usize;
        let mut skipped_non_finite_amount_count = 0usize;
        let mut reward_sum = 0.0f32;

        for attribution in &report.attributions {
            if policy.require_runtime_kv_source && !attribution_has_runtime_kv_source(attribution) {
                skipped_non_runtime_kv_count = skipped_non_runtime_kv_count.saturating_add(1);
                continue;
            }
            if !attribution.amount.is_finite() {
                skipped_non_finite_amount_count = skipped_non_finite_amount_count.saturating_add(1);
                continue;
            }

            let amount = attribution.amount.clamp(0.0, max_abs_reward);
            let reward = match attribution.action {
                super::agent::AgentRecallOutcomeAttributionAction::Reinforce => amount,
                super::agent::AgentRecallOutcomeAttributionAction::Penalize => -amount,
            };
            reward_sum += reward;
            items.push(KvFusionRewardPreviewItem {
                task_id: attribution.task_id.clone(),
                record_id: attribution.record_id.clone(),
                source: attribution.source.clone(),
                action: attribution.action,
                amount,
                reward,
                reason_codes: attribution.reason_codes.clone(),
            });
        }

        kv_fusion_reward_preview_report(
            report,
            items,
            skipped_non_runtime_kv_count,
            skipped_non_finite_amount_count,
            reward_sum.clamp(-max_abs_reward, max_abs_reward),
            blocked_reasons,
        )
    }

    impl KvFusionRewardPreviewReport {
        pub fn has_reward(&self) -> bool {
            self.reward.abs() > f32::EPSILON
        }

        pub fn summary_line(&self) -> String {
            format!(
                "kv_fusion_reward_preview preview_only={} policy_observation_ready={} reward={:.3} items={} reinforce={} penalize={} skipped_non_runtime_kv={} skipped_non_finite_amount={} blocked_reasons={}",
                self.preview_only,
                self.policy_observation_ready,
                self.reward,
                self.items.len(),
                self.reinforced_count,
                self.penalized_count,
                self.skipped_non_runtime_kv_count,
                self.skipped_non_finite_amount_count,
                self.blocked_reasons.len(),
            )
        }
    }

    impl KvFusionPolicyObservationDryRunReport {
        pub fn can_use_policy_observation_preview(&self) -> bool {
            self.preview_only
                && self.policy_observation_ready
                && !self.policy_observation_applied
                && !self.policy_write_allowed
                && self.reward_preview_source_read_only
                && !self.reward_preview_source_memory_store_write_allowed
                && !self.reward_preview_memory_store_write_allowed
                && !self.reward_preview_kv_cache_write_allowed
                && self.threshold_within_bounds
                && self.blocked_reasons.is_empty()
        }

        pub fn summary_line(&self) -> String {
            format!(
                "kv_fusion_policy_observation_dry_run preview_only={} ready={} applied={} source_read_only={} reward={:.3} previous_threshold={:.3} preview_threshold={:.3} changed={} blocked_reasons={}",
                self.preview_only,
                self.policy_observation_ready,
                self.policy_observation_applied,
                self.reward_preview_source_read_only,
                self.reward,
                self.previous_similarity_threshold,
                self.preview_similarity_threshold,
                self.threshold_changed,
                self.blocked_reasons.len(),
            )
        }
    }

    pub fn kv_fusion_reward_policy_observation_dry_run(
        preview: &KvFusionRewardPreviewReport,
        policy: super::core::ReinforcedKvFusionPolicy,
    ) -> KvFusionPolicyObservationDryRunReport {
        let previous_similarity_threshold = policy.similarity_threshold;
        let mut preview_policy = policy;
        let mut blocked_reasons = preview.blocked_reasons.clone();

        if !preview.preview_only {
            blocked_reasons.push("kv_fusion_reward_preview_not_preview_only".to_owned());
        }
        if preview.memory_store_write_allowed {
            blocked_reasons.push("kv_fusion_reward_preview_memory_store_write_allowed".to_owned());
        }
        if preview.kv_cache_write_allowed {
            blocked_reasons.push("kv_fusion_reward_preview_kv_cache_write_allowed".to_owned());
        }
        if preview.policy_observation_applied {
            blocked_reasons.push("kv_fusion_reward_preview_already_applied".to_owned());
        }
        if !preview.policy_observation_ready {
            blocked_reasons.push("kv_fusion_reward_preview_not_ready".to_owned());
        }

        let policy_observation_ready = blocked_reasons.is_empty();
        if policy_observation_ready {
            super::core::KvFusionPolicy::observe_reward(&mut preview_policy, preview.reward);
        }

        let preview_similarity_threshold = preview_policy.similarity_threshold;
        let threshold_delta = preview_similarity_threshold - previous_similarity_threshold;
        let threshold_changed =
            !float_close(preview_similarity_threshold, previous_similarity_threshold);
        let threshold_within_bounds =
            threshold_within_reward_observation_bounds(previous_similarity_threshold)
                && threshold_within_reward_observation_bounds(preview_similarity_threshold);
        let telemetry = kv_fusion_policy_observation_dry_run_telemetry(
            policy_observation_ready,
            preview.source_read_only,
            preview.source_memory_store_write_allowed,
            preview.memory_store_write_allowed,
            preview.kv_cache_write_allowed,
            preview.reward,
            previous_similarity_threshold,
            preview_similarity_threshold,
            threshold_changed,
            threshold_within_bounds,
            &blocked_reasons,
        );

        KvFusionPolicyObservationDryRunReport {
            preview_only: true,
            policy_observation_ready,
            policy_observation_applied: false,
            policy_write_allowed: false,
            reward_preview_source_read_only: preview.source_read_only,
            reward_preview_source_memory_store_write_allowed: preview
                .source_memory_store_write_allowed,
            reward_preview_memory_store_write_allowed: preview.memory_store_write_allowed,
            reward_preview_kv_cache_write_allowed: preview.kv_cache_write_allowed,
            reward: preview.reward,
            previous_similarity_threshold,
            preview_similarity_threshold,
            threshold_delta,
            threshold_changed,
            threshold_within_bounds,
            blocked_reasons,
            telemetry,
        }
    }

    fn kv_fusion_reward_preview_report(
        source: &super::agent::AgentRecallOutcomeAttributionReport,
        items: Vec<KvFusionRewardPreviewItem>,
        skipped_non_runtime_kv_count: usize,
        skipped_non_finite_amount_count: usize,
        reward: f32,
        blocked_reasons: Vec<String>,
    ) -> KvFusionRewardPreviewReport {
        let reinforced_count = items
            .iter()
            .filter(|item| {
                item.action == super::agent::AgentRecallOutcomeAttributionAction::Reinforce
            })
            .count();
        let penalized_count = items
            .iter()
            .filter(|item| {
                item.action == super::agent::AgentRecallOutcomeAttributionAction::Penalize
            })
            .count();
        let policy_observation_ready =
            blocked_reasons.is_empty() && !items.is_empty() && reward.abs() > f32::EPSILON;
        let telemetry = kv_fusion_reward_preview_telemetry(
            source.read_only,
            source.memory_store_write_allowed,
            policy_observation_ready,
            reward,
            items.len(),
            reinforced_count,
            penalized_count,
            skipped_non_runtime_kv_count,
            skipped_non_finite_amount_count,
            &blocked_reasons,
        );

        KvFusionRewardPreviewReport {
            source_read_only: source.read_only,
            source_memory_store_write_allowed: source.memory_store_write_allowed,
            preview_only: true,
            memory_store_write_allowed: false,
            kv_cache_write_allowed: false,
            policy_observation_ready,
            policy_observation_applied: false,
            reward,
            items,
            reinforced_count,
            penalized_count,
            skipped_non_runtime_kv_count,
            skipped_non_finite_amount_count,
            blocked_reasons,
            telemetry,
        }
    }

    fn normalized_max_abs_reward(max_abs_reward: f32) -> f32 {
        if max_abs_reward.is_finite() {
            max_abs_reward.clamp(0.01, 1.0)
        } else {
            0.01
        }
    }

    fn synthetic_route_decision_summary_from_budget(
        budget: super::core::RouteBudget,
    ) -> super::core::RoutingDecisionSummary {
        let token_count = budget.total_tokens();
        let threshold_score = finite_unit_or_zero(budget.threshold);
        let min_score = if budget.fast_tokens > 0 && budget.attention_tokens > 0 {
            finite_unit_or_zero(threshold_score - 0.01)
        } else {
            threshold_score
        };
        let max_score = threshold_score.max(min_score);
        let average_score = if token_count == 0 {
            0.0
        } else {
            (min_score + max_score) * 0.5
        };

        super::core::RoutingDecisionSummary {
            threshold: budget.threshold,
            token_count,
            layer_counts: super::core::RouteLayerCounts {
                fast_projection: budget.fast_tokens,
                local_window: budget.attention_tokens,
                global: 0,
                fusion: 0,
            },
            attention_fraction: budget.attention_fraction,
            average_score,
            min_score,
            max_score,
            above_threshold_tokens: budget.attention_tokens,
            below_threshold_tokens: budget.fast_tokens,
        }
    }

    fn finite_unit_or_zero(value: f32) -> f32 {
        if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn kv_fusion_policy_observation_dry_run_telemetry(
        policy_observation_ready: bool,
        reward_preview_source_read_only: bool,
        reward_preview_source_memory_store_write_allowed: bool,
        reward_preview_memory_store_write_allowed: bool,
        reward_preview_kv_cache_write_allowed: bool,
        reward: f32,
        previous_similarity_threshold: f32,
        preview_similarity_threshold: f32,
        threshold_changed: bool,
        threshold_within_bounds: bool,
        blocked_reasons: &[String],
    ) -> Vec<String> {
        let mut telemetry = vec![
            "kv_fusion_policy_observation_dry_run=true".to_owned(),
            "kv_fusion_policy_observation_dry_run_preview_only=true".to_owned(),
            "kv_fusion_policy_observation_dry_run_policy_write_allowed=false".to_owned(),
            "kv_fusion_policy_observation_dry_run_applied=false".to_owned(),
            format!(
                "kv_fusion_policy_observation_dry_run_reward_preview_source_read_only={reward_preview_source_read_only}"
            ),
            format!(
                "kv_fusion_policy_observation_dry_run_reward_preview_source_memory_store_write_allowed={reward_preview_source_memory_store_write_allowed}"
            ),
            format!(
                "kv_fusion_policy_observation_dry_run_reward_preview_memory_store_write_allowed={reward_preview_memory_store_write_allowed}"
            ),
            format!(
                "kv_fusion_policy_observation_dry_run_reward_preview_kv_cache_write_allowed={reward_preview_kv_cache_write_allowed}"
            ),
            format!("kv_fusion_policy_observation_dry_run_ready={policy_observation_ready}"),
            format!("kv_fusion_policy_observation_dry_run_reward={reward:.3}"),
            format!(
                "kv_fusion_policy_observation_dry_run_previous_threshold={previous_similarity_threshold:.3}"
            ),
            format!(
                "kv_fusion_policy_observation_dry_run_preview_threshold={preview_similarity_threshold:.3}"
            ),
            format!("kv_fusion_policy_observation_dry_run_threshold_changed={threshold_changed}"),
            format!(
                "kv_fusion_policy_observation_dry_run_threshold_within_bounds={threshold_within_bounds}"
            ),
            format!(
                "kv_fusion_policy_observation_dry_run_blocked_reasons={}",
                blocked_reasons.len()
            ),
        ];
        telemetry.extend(
            blocked_reasons.iter().map(|reason| {
                format!("kv_fusion_policy_observation_dry_run_blocked_reason={reason}")
            }),
        );
        telemetry
    }

    fn threshold_within_reward_observation_bounds(threshold: f32) -> bool {
        threshold.is_finite() && (0.75..=0.99).contains(&threshold)
    }

    fn float_close(left: f32, right: f32) -> bool {
        (left - right).abs() <= 0.0001
    }

    fn attribution_has_runtime_kv_source(
        attribution: &super::agent::AgentRecallOutcomeAttribution,
    ) -> bool {
        let source = attribution.source.to_ascii_lowercase();
        let record_id = attribution.record_id.to_ascii_lowercase();
        source == "runtime_kv"
            || source == "runtime-kv"
            || source == "norion_memory_reuse_dry_run"
            || record_id.starts_with("runtime_kv:")
            || record_id.starts_with("runtime-kv:")
            || record_id.starts_with("memory-reuse-dry-run:")
    }

    #[allow(clippy::too_many_arguments)]
    fn kv_fusion_reward_preview_telemetry(
        source_read_only: bool,
        source_memory_store_write_allowed: bool,
        policy_observation_ready: bool,
        reward: f32,
        items: usize,
        reinforced_count: usize,
        penalized_count: usize,
        skipped_non_runtime_kv_count: usize,
        skipped_non_finite_amount_count: usize,
        blocked_reasons: &[String],
    ) -> Vec<String> {
        let mut telemetry = vec![
            "kv_fusion_reward_preview=true".to_owned(),
            format!("kv_fusion_reward_preview_source_read_only={source_read_only}"),
            format!(
                "kv_fusion_reward_preview_source_memory_store_write_allowed={source_memory_store_write_allowed}"
            ),
            "kv_fusion_reward_preview_only=true".to_owned(),
            "kv_fusion_reward_preview_memory_store_write_allowed=false".to_owned(),
            "kv_fusion_reward_preview_kv_cache_write_allowed=false".to_owned(),
            "kv_fusion_reward_preview_policy_observation_applied=false".to_owned(),
            format!("kv_fusion_reward_preview_policy_observation_ready={policy_observation_ready}"),
            format!("kv_fusion_reward_preview_reward={reward:.3}"),
            format!("kv_fusion_reward_preview_items={items}"),
            format!("kv_fusion_reward_preview_reinforce={reinforced_count}"),
            format!("kv_fusion_reward_preview_penalize={penalized_count}"),
            format!(
                "kv_fusion_reward_preview_skipped_non_runtime_kv={skipped_non_runtime_kv_count}"
            ),
            format!(
                "kv_fusion_reward_preview_skipped_non_finite_amount={skipped_non_finite_amount_count}"
            ),
            format!(
                "kv_fusion_reward_preview_blocked_reasons={}",
                blocked_reasons.len()
            ),
        ];
        telemetry.extend(
            blocked_reasons
                .iter()
                .map(|reason| format!("kv_fusion_reward_preview_blocked_reason={reason}")),
        );
        telemetry
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_split_crates_under_stable_namespaces_with_public_contracts() {
        let budget = super::agent::AgentBudget::new(1024, 8, 3);
        assert_eq!(budget.tokens, 1024);
        assert_eq!(budget.steps, 8);
        assert_eq!(budget.messages, 3);

        let cli = super::cli::CliRuntimeConfig::default();
        assert_eq!(cli.startup_snapshot().banner, "norion-cli protocol shell");

        let runtime =
            super::core::RuntimeMetadata::new("gemma-12b", "gemma-tokenizer", 262_144, 3_072);
        assert_eq!(runtime.model_id, "gemma-12b");
        assert_eq!(runtime.native_context_window, 262_144);

        let ledger = super::eval::LedgerSummary::from_records(&[]);
        assert_eq!(ledger.total_rounds, 0);
        assert_eq!(ledger.unique_rounds, 0);

        let document = super::memory::MemoryDocumentInput::new("keep clean memory", vec![0.5]);
        assert_eq!(document.content, "keep clean memory");
        assert_eq!(document.embedding, vec![0.5]);

        let message = super::service::ChatMessage::user("hello split service");
        assert_eq!(message.role, super::service::ChatRole::User);
        assert_eq!(message.content, "hello split service");
    }

    #[test]
    fn bridges_root_generation_metrics_to_core_feedback_readiness() {
        let metrics = crate::router::GenerationMetrics {
            perplexity: 18.0,
            semantic_consistency: 0.40,
            contradiction_count: 2,
            token_count: 128,
        };

        let core_metrics = super::bridge::root_generation_metrics_to_core(metrics);
        let routing_feedback =
            super::bridge::root_generation_metrics_to_core_routing_feedback_summary(
                crate::hierarchy::TaskProfile::Coding,
                metrics,
            );
        let hierarchy_feedback =
            super::bridge::root_generation_metrics_to_core_hierarchy_feedback_summary(
                crate::hierarchy::TaskProfile::Coding,
                metrics,
            );

        assert_eq!(core_metrics.perplexity, metrics.perplexity);
        assert_eq!(
            core_metrics.semantic_consistency,
            metrics.semantic_consistency
        );
        assert_eq!(
            core_metrics.contradiction_count,
            metrics.contradiction_count
        );
        assert_eq!(core_metrics.token_count, metrics.token_count);
        assert_eq!(routing_feedback.profile, super::core::TaskProfile::Coding);
        assert_eq!(hierarchy_feedback.profile, super::core::TaskProfile::Coding);
        assert!((routing_feedback.quality - metrics.quality_score()).abs() < 0.0001);
        assert!((hierarchy_feedback.quality - metrics.quality_score()).abs() < 0.0001);
        assert!(routing_feedback.can_use_routing_feedback());
        assert!(hierarchy_feedback.can_use_feedback());

        let non_finite = crate::router::GenerationMetrics {
            perplexity: f32::NAN,
            semantic_consistency: f32::NAN,
            contradiction_count: 0,
            token_count: 4,
        };
        let non_finite_feedback =
            super::bridge::root_generation_metrics_to_core_routing_feedback_summary(
                crate::hierarchy::TaskProfile::General,
                non_finite,
            );

        assert!(non_finite_feedback.quality.is_finite());
        assert!(non_finite_feedback.quality_shape_is_valid());
        assert!(!non_finite_feedback.perplexity_shape_is_valid());
        assert!(!non_finite_feedback.can_use_routing_feedback());
    }

    #[test]
    fn bridges_root_route_budget_to_core_readiness_with_real_decision_summary() {
        let decisions = [
            super::core::RoutingDecision {
                token: "fast".to_owned(),
                score: 0.18,
                layer: super::core::RouteLayer::FastProjection,
            },
            super::core::RoutingDecision {
                token: "local".to_owned(),
                score: 0.62,
                layer: super::core::RouteLayer::LocalWindow,
            },
        ];
        let decision_summary =
            super::core::RoutingDecisionSummary::from_decisions(0.60, &decisions);
        let root_budget = crate::router::RouteBudget {
            threshold: 0.60,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.50,
        };

        let projection = super::bridge::root_route_budget_to_core_readiness_from_decision_summary(
            decision_summary,
            root_budget,
        );

        assert!(!projection.synthetic_decision_summary);
        assert_eq!(projection.route_budget, decision_summary.route_budget());
        assert!(projection.readiness.can_commit_route_budget_readiness());
        assert!(projection.commit.can_admit_committed_route_budget());
        assert!(projection.can_commit());
        assert_eq!(
            projection.commit.committed_route_budget,
            Some(decision_summary.route_budget())
        );
    }

    #[test]
    fn root_route_budget_core_readiness_marks_synthetic_and_blocks_stale_budget() {
        let decisions = [
            super::core::RoutingDecision {
                token: "fast".to_owned(),
                score: 0.18,
                layer: super::core::RouteLayer::FastProjection,
            },
            super::core::RoutingDecision {
                token: "local".to_owned(),
                score: 0.62,
                layer: super::core::RouteLayer::LocalWindow,
            },
        ];
        let decision_summary =
            super::core::RoutingDecisionSummary::from_decisions(0.60, &decisions);
        let root_budget = crate::router::RouteBudget {
            threshold: 0.60,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.50,
        };

        let synthetic = super::bridge::root_route_budget_to_core_readiness(root_budget);

        assert!(synthetic.synthetic_decision_summary);
        assert_eq!(synthetic.readiness.decision_summary.token_count, 2);
        assert_eq!(
            synthetic
                .readiness
                .decision_summary
                .layer_counts
                .local_window,
            1
        );
        assert_eq!(synthetic.readiness.decision_summary.layer_counts.fusion, 0);
        assert!(synthetic.can_commit());

        let stale_budget = crate::router::RouteBudget {
            fast_tokens: 2,
            attention_fraction: 1.0 / 3.0,
            ..root_budget
        };
        let stale = super::bridge::root_route_budget_to_core_readiness_from_decision_summary(
            decision_summary,
            stale_budget,
        );

        assert!(!stale.synthetic_decision_summary);
        assert!(!stale.can_commit());
        assert_eq!(
            stale.commit.action,
            super::core::RouteBudgetReadinessCommitAction::RepairRouteBudget
        );
        assert_eq!(
            stale.commit.first_blocking_stage,
            Some(super::core::RouteBudgetReadinessStage::BudgetParity)
        );
        assert_eq!(stale.commit.committed_route_budget, None);
    }

    #[test]
    fn bridges_root_router_state_to_core_observation_accounting() {
        let state = crate::router::RouterState {
            threshold: 0.52,
            observations: 4,
            profile_thresholds: crate::router::ProfileThresholds {
                general: 0.52,
                coding: 0.48,
                writing: 0.61,
                long_document: 0.57,
            },
            profile_observations: crate::router::ProfileObservations {
                general: 1,
                coding: 2,
                writing: 0,
                long_document: 1,
            },
        };

        let core_state = super::bridge::root_router_state_to_core(state);

        assert_eq!(core_state.threshold, 0.52);
        assert_eq!(core_state.observations, 4);
        assert_eq!(
            core_state
                .profile_thresholds
                .get(super::core::TaskProfile::Coding),
            0.48
        );
        assert_eq!(
            core_state
                .profile_observations
                .get(super::core::TaskProfile::LongDocument),
            1
        );
        assert_eq!(core_state.profile_observation_total(), 4);
        assert!(!core_state.has_observation_drift());
    }

    #[test]
    fn bridges_root_hierarchy_weights_convolution_to_core_fusion_summary() {
        let weights = crate::hierarchy::HierarchyWeights::new(0.20, 0.60, 0.20);

        let core_weights = super::bridge::root_hierarchy_weights_to_core(weights);
        let summary = super::bridge::root_hierarchy_weights_to_core_summary(weights);

        assert!((core_weights.global - weights.global).abs() < 0.0001);
        assert!((core_weights.local - weights.local).abs() < 0.0001);
        assert!((core_weights.fusion - weights.convolution).abs() < 0.0001);
        assert_eq!(summary.dominant, super::core::HierarchyWeightFocus::Local);
        assert!(summary.is_normalized);
        assert!(summary.can_use_hierarchy_weights());

        let repaired = super::bridge::root_hierarchy_weights_to_core_summary(
            crate::hierarchy::HierarchyWeights {
                global: f32::NAN,
                local: f32::NEG_INFINITY,
                convolution: 2.0,
            },
        );

        assert_eq!(repaired.dominant, super::core::HierarchyWeightFocus::Fusion);
        assert!(repaired.can_use_hierarchy_weights());
    }

    #[test]
    fn bridges_memory_reuse_dry_run_into_agent_recall_sidecar() {
        let summary = super::memory::MemoryReuseDryRunSummary {
            read_only: true,
            candidate_count: 2,
            long_term_match_count: 2,
            context_decision_count: 2,
            accepted_context_count: 1,
            rejected_context_count: 1,
            used_tokens: 96,
            requested_kv_count: 3,
            kv_promote_count: 1,
            kv_missing_count: 1,
            kv_already_hot_count: 1,
            kv_duplicate_count: 0,
            kv_backend_available: true,
            memory_store_write_allowed: false,
            kv_prefetch_apply_allowed: false,
            reason_codes: vec!["read_only".to_owned(), "context_accepted".to_owned()],
            detail_codes: vec!["kv_prefetch:promote:636f6c64".to_owned()],
        };
        let task = super::agent::AgentTask::new(
            "runtime-recall",
            super::agent::AgentRole::Coder,
            "reuse a memory dry run without prompt injection",
            super::agent::AgentBudget::new(8, 1, 1),
        );

        let evidence = super::bridge::memory_reuse_dry_run_to_agent_evidence(&summary);
        let context = super::agent::MemoryRecallContextPlanner::new()
            .plan_from_dry_run_evidence(&task, &evidence);

        assert!(evidence.safe_for_recall_sidecar());
        assert!(context.read_only);
        assert_eq!(context.returned_records, 1);
        assert_eq!(context.accepted_count(), 1);
        assert_eq!(context.rejected_count(), 0);
        assert_eq!(
            context.accepted_items()[0].source,
            "norion_memory_reuse_dry_run"
        );
        assert!(
            context.context_lines()[0]
                .contains("source=norion_memory_reuse_dry_run read_only=true")
        );
        assert!(
            context
                .reason_codes()
                .contains(&"dry_run:context_accepted".to_owned())
        );
        assert!(
            context
                .telemetry
                .iter()
                .any(|line| { line == "agent_memory_recall_dry_run_evidence_kv_promote=1" })
        );
        assert!(context.summary_line().contains("admitted=1"));
    }

    #[test]
    fn bridges_recall_attribution_into_kv_fusion_reward_preview_without_apply() {
        let report = super::agent::AgentRecallOutcomeAttributionReport {
            attributions: vec![super::agent::AgentRecallOutcomeAttribution {
                task_id: "runtime-recall".to_owned(),
                record_id: "runtime_kv:l0h0:0-8".to_owned(),
                source: "runtime_kv".to_owned(),
                action: super::agent::AgentRecallOutcomeAttributionAction::Reinforce,
                amount: 0.24,
                reason_codes: vec!["result_accepted".to_owned()],
            }],
            reinforced_count: 1,
            penalized_count: 0,
            skipped_rejected_recall_count: 0,
            skipped_missing_outcome_task_ids: Vec::new(),
            read_only: true,
            memory_store_write_allowed: false,
            telemetry: Vec::new(),
        };

        let preview =
            super::bridge::recall_outcome_attribution_to_kv_fusion_reward_preview(&report);

        assert!(preview.preview_only);
        assert!(!preview.memory_store_write_allowed);
        assert!(!preview.kv_cache_write_allowed);
        assert!(preview.policy_observation_ready);
        assert!(!preview.policy_observation_applied);
        assert!(preview.has_reward());
        assert_eq!(preview.reward, 0.24);
        assert_eq!(preview.items.len(), 1);
        assert_eq!(preview.reinforced_count, 1);
        assert_eq!(preview.penalized_count, 0);
        assert!(preview.blocked_reasons.is_empty());
        assert!(
            preview
                .telemetry
                .iter()
                .any(|line| { line == "kv_fusion_reward_preview_policy_observation_ready=true" })
        );
        assert!(preview.summary_line().contains("preview_only=true"));
    }

    #[test]
    fn kv_fusion_policy_observation_dry_run_previews_reward_without_mutating_policy() {
        let report = super::agent::AgentRecallOutcomeAttributionReport {
            attributions: vec![super::agent::AgentRecallOutcomeAttribution {
                task_id: "runtime-recall".to_owned(),
                record_id: "runtime_kv:l0h0:0-8".to_owned(),
                source: "runtime_kv".to_owned(),
                action: super::agent::AgentRecallOutcomeAttributionAction::Reinforce,
                amount: 0.24,
                reason_codes: vec!["result_accepted".to_owned()],
            }],
            reinforced_count: 1,
            penalized_count: 0,
            skipped_rejected_recall_count: 0,
            skipped_missing_outcome_task_ids: Vec::new(),
            read_only: true,
            memory_store_write_allowed: false,
            telemetry: Vec::new(),
        };
        let preview =
            super::bridge::recall_outcome_attribution_to_kv_fusion_reward_preview(&report);
        let policy = super::core::ReinforcedKvFusionPolicy::new(0.92, 64);

        let dry_run = super::bridge::kv_fusion_reward_policy_observation_dry_run(&preview, policy);

        assert!(dry_run.preview_only);
        assert!(dry_run.policy_observation_ready);
        assert!(!dry_run.policy_observation_applied);
        assert!(!dry_run.policy_write_allowed);
        assert!(dry_run.reward_preview_source_read_only);
        assert!(!dry_run.reward_preview_source_memory_store_write_allowed);
        assert!(!dry_run.reward_preview_memory_store_write_allowed);
        assert!(!dry_run.reward_preview_kv_cache_write_allowed);
        assert!(dry_run.can_use_policy_observation_preview());
        assert_eq!(dry_run.reward, 0.24);
        assert_eq!(dry_run.previous_similarity_threshold, 0.92);
        assert!((dry_run.preview_similarity_threshold - 0.9152).abs() < 0.0001);
        assert!((dry_run.threshold_delta + 0.0048).abs() < 0.0001);
        assert!(dry_run.threshold_changed);
        assert!(dry_run.threshold_within_bounds);
        assert!(dry_run.blocked_reasons.is_empty());
        assert_eq!(policy.similarity_threshold, 0.92);
        assert!(
            dry_run
                .telemetry
                .iter()
                .any(|line| line == "kv_fusion_policy_observation_dry_run_applied=false")
        );
        assert!(
            dry_run
                .summary_line()
                .contains("kv_fusion_policy_observation_dry_run preview_only=true")
        );
    }

    #[test]
    fn kv_fusion_policy_observation_dry_run_blocks_unsafe_reward_preview() {
        let report = super::agent::AgentRecallOutcomeAttributionReport {
            attributions: vec![super::agent::AgentRecallOutcomeAttribution {
                task_id: "runtime-recall".to_owned(),
                record_id: "runtime_kv:l0h0:0-8".to_owned(),
                source: "runtime_kv".to_owned(),
                action: super::agent::AgentRecallOutcomeAttributionAction::Penalize,
                amount: 0.32,
                reason_codes: vec!["execution_failed".to_owned()],
            }],
            reinforced_count: 0,
            penalized_count: 1,
            skipped_rejected_recall_count: 0,
            skipped_missing_outcome_task_ids: Vec::new(),
            read_only: false,
            memory_store_write_allowed: true,
            telemetry: Vec::new(),
        };
        let preview =
            super::bridge::recall_outcome_attribution_to_kv_fusion_reward_preview(&report);
        let policy = super::core::ReinforcedKvFusionPolicy::new(0.92, 64);

        let dry_run = super::bridge::kv_fusion_reward_policy_observation_dry_run(&preview, policy);

        assert!(!dry_run.policy_observation_ready);
        assert!(!dry_run.policy_observation_applied);
        assert!(!dry_run.policy_write_allowed);
        assert!(!dry_run.reward_preview_source_read_only);
        assert!(dry_run.reward_preview_source_memory_store_write_allowed);
        assert!(!dry_run.reward_preview_memory_store_write_allowed);
        assert!(!dry_run.reward_preview_kv_cache_write_allowed);
        assert!(!dry_run.can_use_policy_observation_preview());
        assert_eq!(dry_run.reward, 0.0);
        assert_eq!(dry_run.previous_similarity_threshold, 0.92);
        assert_eq!(dry_run.preview_similarity_threshold, 0.92);
        assert_eq!(dry_run.threshold_delta, 0.0);
        assert!(!dry_run.threshold_changed);
        assert!(dry_run.threshold_within_bounds);
        assert_eq!(
            dry_run.blocked_reasons,
            vec![
                "recall_attribution_not_read_only".to_owned(),
                "recall_attribution_memory_store_write_allowed".to_owned(),
                "kv_fusion_reward_preview_not_ready".to_owned(),
            ]
        );
        assert_eq!(policy.similarity_threshold, 0.92);
        assert!(
            dry_run
                .telemetry
                .iter()
                .any(|line| line == "kv_fusion_policy_observation_dry_run_ready=false")
        );
        assert!(dry_run.telemetry.iter().any(|line| {
            line == "kv_fusion_policy_observation_dry_run_reward_preview_source_read_only=false"
        }));
    }

    #[test]
    fn kv_fusion_policy_observation_dry_run_preserves_threshold_bounds() {
        let report = super::agent::AgentRecallOutcomeAttributionReport {
            attributions: vec![super::agent::AgentRecallOutcomeAttribution {
                task_id: "runtime-recall".to_owned(),
                record_id: "runtime_kv:l0h0:0-8".to_owned(),
                source: "runtime_kv".to_owned(),
                action: super::agent::AgentRecallOutcomeAttributionAction::Reinforce,
                amount: 1.0,
                reason_codes: vec!["result_accepted".to_owned()],
            }],
            reinforced_count: 1,
            penalized_count: 0,
            skipped_rejected_recall_count: 0,
            skipped_missing_outcome_task_ids: Vec::new(),
            read_only: true,
            memory_store_write_allowed: false,
            telemetry: Vec::new(),
        };
        let preview =
            super::bridge::recall_outcome_attribution_to_kv_fusion_reward_preview(&report);
        let policy = super::core::ReinforcedKvFusionPolicy::new(0.75, 64);

        let dry_run = super::bridge::kv_fusion_reward_policy_observation_dry_run(&preview, policy);

        assert!(dry_run.policy_observation_ready);
        assert!(!dry_run.policy_observation_applied);
        assert_eq!(dry_run.reward, 1.0);
        assert_eq!(dry_run.previous_similarity_threshold, 0.75);
        assert_eq!(dry_run.preview_similarity_threshold, 0.75);
        assert_eq!(dry_run.threshold_delta, 0.0);
        assert!(!dry_run.threshold_changed);
        assert!(dry_run.threshold_within_bounds);
        assert!(dry_run.can_use_policy_observation_preview());
        assert_eq!(policy.similarity_threshold, 0.75);
        assert!(dry_run.telemetry.iter().any(|line| {
            line == "kv_fusion_policy_observation_dry_run_threshold_within_bounds=true"
        }));
    }

    #[test]
    fn kv_fusion_reward_preview_skips_non_kv_and_non_finite_attribution() {
        let report = super::agent::AgentRecallOutcomeAttributionReport {
            attributions: vec![
                super::agent::AgentRecallOutcomeAttribution {
                    task_id: "semantic-recall".to_owned(),
                    record_id: "lesson-1".to_owned(),
                    source: "long_term".to_owned(),
                    action: super::agent::AgentRecallOutcomeAttributionAction::Reinforce,
                    amount: 0.50,
                    reason_codes: vec!["result_accepted".to_owned()],
                },
                super::agent::AgentRecallOutcomeAttribution {
                    task_id: "runtime-nan".to_owned(),
                    record_id: "runtime_kv:l0h0:8-16".to_owned(),
                    source: "runtime_kv".to_owned(),
                    action: super::agent::AgentRecallOutcomeAttributionAction::Reinforce,
                    amount: f32::NAN,
                    reason_codes: vec!["result_accepted".to_owned()],
                },
                super::agent::AgentRecallOutcomeAttribution {
                    task_id: "runtime-big".to_owned(),
                    record_id: "memory-reuse-dry-run:abcd".to_owned(),
                    source: "norion_memory_reuse_dry_run".to_owned(),
                    action: super::agent::AgentRecallOutcomeAttributionAction::Reinforce,
                    amount: 2.0,
                    reason_codes: vec!["result_accepted".to_owned()],
                },
            ],
            reinforced_count: 3,
            penalized_count: 0,
            skipped_rejected_recall_count: 0,
            skipped_missing_outcome_task_ids: Vec::new(),
            read_only: true,
            memory_store_write_allowed: false,
            telemetry: Vec::new(),
        };

        let preview =
            super::bridge::recall_outcome_attribution_to_kv_fusion_reward_preview_with_policy(
                &report,
                super::bridge::KvFusionRewardPreviewPolicy {
                    max_abs_reward: 0.50,
                    require_runtime_kv_source: true,
                },
            );

        assert!(preview.policy_observation_ready);
        assert_eq!(preview.items.len(), 1);
        assert_eq!(preview.reward, 0.50);
        assert_eq!(preview.skipped_non_runtime_kv_count, 1);
        assert_eq!(preview.skipped_non_finite_amount_count, 1);
        assert_eq!(preview.items[0].record_id, "memory-reuse-dry-run:abcd");
        assert!(
            preview
                .telemetry
                .iter()
                .any(|line| { line == "kv_fusion_reward_preview_skipped_non_runtime_kv=1" })
        );
        assert!(
            preview
                .telemetry
                .iter()
                .any(|line| { line == "kv_fusion_reward_preview_skipped_non_finite_amount=1" })
        );
    }

    #[test]
    fn kv_fusion_reward_preview_requires_explicit_runtime_kv_source() {
        let report = super::agent::AgentRecallOutcomeAttributionReport {
            attributions: vec![
                super::agent::AgentRecallOutcomeAttribution {
                    task_id: "semantic-kv".to_owned(),
                    record_id: "lesson-1".to_owned(),
                    source: "semantic_kv".to_owned(),
                    action: super::agent::AgentRecallOutcomeAttributionAction::Reinforce,
                    amount: 0.25,
                    reason_codes: vec!["result_accepted".to_owned()],
                },
                super::agent::AgentRecallOutcomeAttribution {
                    task_id: "ordinary-kv-token".to_owned(),
                    record_id: "fix-kv-cache".to_owned(),
                    source: "long_term".to_owned(),
                    action: super::agent::AgentRecallOutcomeAttributionAction::Reinforce,
                    amount: 0.25,
                    reason_codes: vec!["result_accepted".to_owned()],
                },
                super::agent::AgentRecallOutcomeAttribution {
                    task_id: "runtime-kv".to_owned(),
                    record_id: "runtime-kv:l0h0:16-32".to_owned(),
                    source: "runtime-kv".to_owned(),
                    action: super::agent::AgentRecallOutcomeAttributionAction::Reinforce,
                    amount: 0.25,
                    reason_codes: vec!["result_accepted".to_owned()],
                },
            ],
            reinforced_count: 3,
            penalized_count: 0,
            skipped_rejected_recall_count: 0,
            skipped_missing_outcome_task_ids: Vec::new(),
            read_only: true,
            memory_store_write_allowed: false,
            telemetry: Vec::new(),
        };

        let preview =
            super::bridge::recall_outcome_attribution_to_kv_fusion_reward_preview(&report);

        assert!(preview.policy_observation_ready);
        assert_eq!(preview.items.len(), 1);
        assert_eq!(preview.items[0].task_id, "runtime-kv");
        assert_eq!(preview.reward, 0.25);
        assert_eq!(preview.skipped_non_runtime_kv_count, 2);
    }

    #[test]
    fn kv_fusion_reward_preview_normalizes_invalid_reward_bounds_conservatively() {
        let report = super::agent::AgentRecallOutcomeAttributionReport {
            attributions: vec![super::agent::AgentRecallOutcomeAttribution {
                task_id: "runtime-recall".to_owned(),
                record_id: "runtime_kv:l0h0:0-8".to_owned(),
                source: "runtime_kv".to_owned(),
                action: super::agent::AgentRecallOutcomeAttributionAction::Reinforce,
                amount: 1.0,
                reason_codes: vec!["result_accepted".to_owned()],
            }],
            reinforced_count: 1,
            penalized_count: 0,
            skipped_rejected_recall_count: 0,
            skipped_missing_outcome_task_ids: Vec::new(),
            read_only: true,
            memory_store_write_allowed: false,
            telemetry: Vec::new(),
        };

        for max_abs_reward in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.0, -1.0] {
            let preview =
                super::bridge::recall_outcome_attribution_to_kv_fusion_reward_preview_with_policy(
                    &report,
                    super::bridge::KvFusionRewardPreviewPolicy {
                        max_abs_reward,
                        require_runtime_kv_source: true,
                    },
                );

            assert!(preview.policy_observation_ready);
            assert!((preview.reward - 0.01).abs() <= f32::EPSILON);
            assert!((preview.items[0].amount - 0.01).abs() <= f32::EPSILON);
        }
    }

    #[test]
    fn kv_fusion_reward_preview_blocks_unsafe_attribution_report() {
        let report = super::agent::AgentRecallOutcomeAttributionReport {
            attributions: vec![super::agent::AgentRecallOutcomeAttribution {
                task_id: "runtime-recall".to_owned(),
                record_id: "runtime_kv:l0h0:0-8".to_owned(),
                source: "runtime_kv".to_owned(),
                action: super::agent::AgentRecallOutcomeAttributionAction::Penalize,
                amount: 0.32,
                reason_codes: vec!["execution_failed".to_owned()],
            }],
            reinforced_count: 0,
            penalized_count: 1,
            skipped_rejected_recall_count: 0,
            skipped_missing_outcome_task_ids: Vec::new(),
            read_only: false,
            memory_store_write_allowed: true,
            telemetry: Vec::new(),
        };

        let preview =
            super::bridge::recall_outcome_attribution_to_kv_fusion_reward_preview(&report);

        assert!(!preview.policy_observation_ready);
        assert!(!preview.policy_observation_applied);
        assert_eq!(preview.reward, 0.0);
        assert!(preview.items.is_empty());
        assert_eq!(
            preview.blocked_reasons,
            vec![
                "recall_attribution_not_read_only".to_owned(),
                "recall_attribution_memory_store_write_allowed".to_owned(),
            ]
        );
        assert!(
            preview
                .telemetry
                .iter()
                .any(|line| { line == "kv_fusion_reward_preview_policy_observation_ready=false" })
        );
        assert!(preview.summary_line().contains("blocked_reasons=2"));
    }
}
