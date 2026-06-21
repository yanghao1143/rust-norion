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
        pub reward: f32,
        pub previous_similarity_threshold: f32,
        pub preview_similarity_threshold: f32,
        pub threshold_delta: f32,
        pub threshold_changed: bool,
        pub threshold_within_bounds: bool,
        pub blocked_reasons: Vec<String>,
        pub telemetry: Vec<String>,
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
                && self.threshold_within_bounds
                && self.blocked_reasons.is_empty()
        }

        pub fn summary_line(&self) -> String {
            format!(
                "kv_fusion_policy_observation_dry_run preview_only={} ready={} applied={} reward={:.3} previous_threshold={:.3} preview_threshold={:.3} changed={} blocked_reasons={}",
                self.preview_only,
                self.policy_observation_ready,
                self.policy_observation_applied,
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

    #[allow(clippy::too_many_arguments)]
    fn kv_fusion_policy_observation_dry_run_telemetry(
        policy_observation_ready: bool,
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
