use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::adaptive_state::EvolutionLedger;
use crate::engine::NoironEngine;
use crate::experience::{ExperienceMatch, ExperienceRecord, recursive_runtime_calls_from_notes};
use crate::experience_replay::LiveMemoryFeedbackStats;
use crate::hardware::{DeviceClass, HardwarePlan};
use crate::hierarchy::{
    HierarchyWeights, ProfileHierarchyObservations, ProfileHierarchyWeights, TaskProfile,
};
use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};
use crate::process_reward::RewardAction;
use crate::router::{ProfileObservations, ProfileThresholds};
use crate::runtime::RuntimeAdapterObservation;
use crate::tiered_cache::TierCounts;

#[derive(Debug, Clone)]
pub struct StateMemorySummary {
    pub id: u64,
    pub key: String,
    pub vector_dimensions: usize,
    pub strength: f32,
    pub hits: u64,
    pub failures: u64,
    pub last_score: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateMemoryVectorDimensions {
    pub dimensions: usize,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct StateExperienceSummary {
    pub id: u64,
    pub profile: TaskProfile,
    pub quality: f32,
    pub process_reward: f32,
    pub reward_action: RewardAction,
    pub runtime_model_id: Option<String>,
    pub runtime_selected_adapter: Option<String>,
    pub runtime_device_profile: Option<String>,
    pub runtime_primary_lane: Option<String>,
    pub runtime_fallback_lane: Option<String>,
    pub runtime_memory_mode: Option<String>,
    pub runtime_layer_count: usize,
    pub runtime_global_layers: usize,
    pub runtime_local_window_layers: usize,
    pub runtime_convolutional_fusion_layers: usize,
    pub runtime_hidden_size: usize,
    pub runtime_local_window_tokens: usize,
    pub runtime_forward_energy: Option<f32>,
    pub runtime_kv_influence: Option<f32>,
    pub runtime_hot_kv_precision_bits: Option<u8>,
    pub runtime_cold_kv_precision_bits: Option<u8>,
    pub runtime_imported_kv_blocks: usize,
    pub runtime_exported_kv_blocks: usize,
    pub recursive_runtime_calls: Option<usize>,
    pub live_memory_feedback_updates: usize,
    pub live_memory_feedback_reinforced: usize,
    pub live_memory_feedback_penalized: usize,
    pub live_memory_feedback_applied: usize,
    pub live_memory_feedback_removed: usize,
    pub live_memory_feedback_missing: usize,
    pub live_memory_feedback_strength_delta: f32,
    pub live_memory_feedback_detail: bool,
    pub reflection_issues: usize,
    pub critical_reflection_issues: usize,
    pub revision_actions: usize,
    pub lesson: String,
}

#[derive(Debug, Clone, Default)]
pub struct StateInspectionGate {
    pub min_memories: Option<usize>,
    pub min_runtime_kv_memories: Option<usize>,
    pub min_experiences: Option<usize>,
    pub min_runtime_model_experiences: Option<usize>,
    pub min_runtime_adapter_experiences: Option<usize>,
    pub max_runtime_adapter_selection_mismatches: Option<usize>,
    pub min_runtime_forward_energy_experiences: Option<usize>,
    pub min_runtime_kv_influence_experiences: Option<usize>,
    pub min_runtime_kv_precision_experiences: Option<usize>,
    pub max_runtime_kv_precision_mismatches: Option<usize>,
    pub min_runtime_device_execution_experiences: Option<usize>,
    pub min_runtime_layer_mode_experiences: Option<usize>,
    pub min_runtime_all_layer_mode_experiences: Option<usize>,
    pub min_runtime_global_layers: Option<usize>,
    pub min_runtime_local_window_layers: Option<usize>,
    pub min_runtime_convolutional_fusion_layers: Option<usize>,
    pub min_runtime_kv_import_experiences: Option<usize>,
    pub min_runtime_kv_export_experiences: Option<usize>,
    pub min_runtime_kv_hold_experiences: Option<usize>,
    pub min_runtime_kv_held_blocks: Option<usize>,
    pub min_reflection_issue_experiences: Option<usize>,
    pub min_critical_reflection_issue_experiences: Option<usize>,
    pub min_revision_action_experiences: Option<usize>,
    pub min_live_memory_feedback_experiences: Option<usize>,
    pub min_live_memory_feedback_updates: Option<usize>,
    pub min_live_memory_feedback_detail_experiences: Option<usize>,
    pub min_live_memory_feedback_applied: Option<usize>,
    pub min_live_memory_feedback_strength_delta: Option<f32>,
    pub min_router_observations: Option<u64>,
    pub min_evolution_live_inference_runs: Option<u64>,
    pub min_evolution_live_router_threshold_mutations: Option<u64>,
    pub min_evolution_live_hierarchy_weight_mutations: Option<u64>,
    pub min_evolution_live_router_threshold_delta: Option<f32>,
    pub min_evolution_live_hierarchy_weight_delta: Option<f32>,
    pub min_evolution_live_memory_updates: Option<u64>,
    pub min_evolution_live_stored_memory_updates: Option<u64>,
    pub min_evolution_live_reflection_issues: Option<u64>,
    pub min_evolution_live_critical_reflection_issues: Option<u64>,
    pub min_evolution_live_revision_actions: Option<u64>,
    pub min_evolution_replay_runs: Option<u64>,
    pub min_evolution_replay_items: Option<u64>,
    pub min_evolution_router_threshold_mutations: Option<u64>,
    pub min_evolution_hierarchy_weight_mutations: Option<u64>,
    pub min_evolution_router_threshold_delta: Option<f32>,
    pub min_evolution_hierarchy_weight_delta: Option<f32>,
    pub min_evolution_memory_updates: Option<u64>,
    pub min_evolution_replay_live_memory_feedback_updates: Option<u64>,
    pub min_evolution_replay_live_memory_feedback_detail_items: Option<u64>,
    pub min_evolution_replay_live_memory_feedback_applied: Option<u64>,
    pub min_evolution_replay_live_memory_feedback_strength_delta: Option<f32>,
    pub min_evolution_replay_live_evolution_items: Option<u64>,
    pub min_evolution_replay_live_evolution_memory_updates: Option<u64>,
    pub min_evolution_replay_live_evolution_stored_memory_updates: Option<u64>,
    pub min_evolution_replay_live_evolution_reflection_issues: Option<u64>,
    pub min_evolution_replay_live_evolution_critical_reflection_issues: Option<u64>,
    pub min_evolution_replay_live_evolution_revision_actions: Option<u64>,
    pub min_evolution_recursive_replay_items: Option<u64>,
    pub min_evolution_recursive_runtime_calls: Option<u64>,
    pub max_evolution_drift_rollbacks: Option<u64>,
    pub max_evolution_rollback_router_threshold_delta: Option<f32>,
    pub max_evolution_rollback_hierarchy_weight_delta: Option<f32>,
    pub require_runtime_kv_dimensions: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StateInspectionMatrixGate {
    pub min_runtime_kv_memory_device_profiles: Option<usize>,
    pub min_runtime_model_device_profiles: Option<usize>,
    pub min_runtime_adapter_device_profiles: Option<usize>,
    pub max_runtime_adapter_selection_mismatches: Option<usize>,
    pub min_runtime_forward_energy_device_profiles: Option<usize>,
    pub min_runtime_kv_influence_device_profiles: Option<usize>,
    pub min_runtime_kv_precision_device_profiles: Option<usize>,
    pub max_runtime_kv_precision_mismatches: Option<usize>,
    pub min_runtime_device_execution_device_profiles: Option<usize>,
    pub min_runtime_layer_mode_device_profiles: Option<usize>,
    pub min_runtime_all_layer_mode_device_profiles: Option<usize>,
    pub min_runtime_kv_import_device_profiles: Option<usize>,
    pub min_runtime_kv_export_device_profiles: Option<usize>,
    pub min_runtime_kv_hold_device_profiles: Option<usize>,
    pub min_reflection_issue_device_profiles: Option<usize>,
    pub min_critical_reflection_issue_device_profiles: Option<usize>,
    pub min_revision_action_device_profiles: Option<usize>,
    pub min_live_memory_feedback_device_profiles: Option<usize>,
    pub min_evolution_live_inference_device_profiles: Option<usize>,
    pub min_evolution_live_router_threshold_mutation_device_profiles: Option<usize>,
    pub min_evolution_live_hierarchy_weight_mutation_device_profiles: Option<usize>,
    pub min_evolution_live_memory_update_device_profiles: Option<usize>,
    pub min_evolution_live_stored_memory_update_device_profiles: Option<usize>,
    pub min_evolution_live_reflection_issue_device_profiles: Option<usize>,
    pub min_evolution_live_critical_reflection_issue_device_profiles: Option<usize>,
    pub min_evolution_live_revision_action_device_profiles: Option<usize>,
    pub min_evolution_replay_run_device_profiles: Option<usize>,
    pub min_evolution_replay_item_device_profiles: Option<usize>,
    pub min_evolution_router_threshold_mutation_device_profiles: Option<usize>,
    pub min_evolution_hierarchy_weight_mutation_device_profiles: Option<usize>,
    pub min_evolution_memory_update_device_profiles: Option<usize>,
    pub min_evolution_replay_live_memory_feedback_device_profiles: Option<usize>,
    pub min_evolution_replay_live_memory_feedback_detail_device_profiles: Option<usize>,
    pub min_evolution_replay_live_evolution_device_profiles: Option<usize>,
    pub min_evolution_replay_live_evolution_memory_update_device_profiles: Option<usize>,
    pub min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles:
        Option<usize>,
    pub min_evolution_replay_live_evolution_revision_action_device_profiles: Option<usize>,
    pub min_evolution_recursive_replay_device_profiles: Option<usize>,
    pub min_evolution_recursive_runtime_call_device_profiles: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateInspectionGateReport {
    pub passed: bool,
    pub failures: Vec<String>,
}

impl StateInspectionGateReport {
    pub fn passed(&self) -> bool {
        self.passed
    }

    pub fn summary_line(&self) -> String {
        format!(
            "state_inspection_gate: passed={} failures={}",
            self.passed,
            self.failures.len()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateInspectionDeviceGateReport {
    pub device: DeviceClass,
    pub report: StateInspectionGateReport,
    pub runtime_kv_memories: usize,
    pub runtime_model_experiences: usize,
    pub runtime_adapter_experiences: usize,
    pub runtime_adapter_selection_mismatches: usize,
    pub runtime_forward_energy_experiences: usize,
    pub runtime_kv_influence_experiences: usize,
    pub runtime_kv_precision_experiences: usize,
    pub runtime_kv_precision_mismatches: usize,
    pub runtime_device_execution_experiences: usize,
    pub runtime_layer_mode_experiences: usize,
    pub runtime_all_layer_mode_experiences: usize,
    pub runtime_kv_import_experiences: usize,
    pub runtime_kv_export_experiences: usize,
    pub runtime_kv_hold_experiences: usize,
    pub runtime_kv_held_blocks: usize,
    pub reflection_issue_experiences: usize,
    pub critical_reflection_issue_experiences: usize,
    pub revision_action_experiences: usize,
    pub live_memory_feedback_experiences: usize,
    pub live_memory_feedback_updates: usize,
    pub live_memory_feedback_detail_experiences: usize,
    pub live_memory_feedback_applied: usize,
    pub live_memory_feedback_removed: usize,
    pub live_memory_feedback_missing: usize,
    pub live_memory_feedback_strength_delta: f32,
    pub evolution_live_inference_runs: u64,
    pub evolution_live_router_threshold_mutations: u64,
    pub evolution_live_hierarchy_weight_mutations: u64,
    pub evolution_live_memory_updates: u64,
    pub evolution_live_stored_memory_updates: u64,
    pub evolution_live_reflection_issues: u64,
    pub evolution_live_critical_reflection_issues: u64,
    pub evolution_live_revision_actions: u64,
    pub evolution_replay_runs: u64,
    pub evolution_replay_items: u64,
    pub evolution_router_threshold_mutations: u64,
    pub evolution_hierarchy_weight_mutations: u64,
    pub evolution_memory_updates: u64,
    pub evolution_replay_live_memory_feedback_updates: u64,
    pub evolution_replay_live_memory_feedback_detail_items: u64,
    pub evolution_replay_live_memory_feedback_applied: u64,
    pub evolution_replay_live_memory_feedback_removed: u64,
    pub evolution_replay_live_memory_feedback_missing: u64,
    pub evolution_replay_live_memory_feedback_strength_delta: f32,
    pub evolution_replay_live_evolution_items: u64,
    pub evolution_replay_live_evolution_memory_updates: u64,
    pub evolution_replay_live_evolution_stored_memory_updates: u64,
    pub evolution_replay_live_evolution_reflection_issues: u64,
    pub evolution_replay_live_evolution_critical_reflection_issues: u64,
    pub evolution_replay_live_evolution_revision_actions: u64,
    pub evolution_recursive_replay_items: u64,
    pub evolution_recursive_runtime_calls: u64,
}

impl StateInspectionDeviceGateReport {
    pub fn new(device: DeviceClass, report: StateInspectionGateReport) -> Self {
        Self {
            device,
            report,
            runtime_kv_memories: 0,
            runtime_model_experiences: 0,
            runtime_adapter_experiences: 0,
            runtime_adapter_selection_mismatches: 0,
            runtime_forward_energy_experiences: 0,
            runtime_kv_influence_experiences: 0,
            runtime_kv_precision_experiences: 0,
            runtime_kv_precision_mismatches: 0,
            runtime_device_execution_experiences: 0,
            runtime_layer_mode_experiences: 0,
            runtime_all_layer_mode_experiences: 0,
            runtime_kv_import_experiences: 0,
            runtime_kv_export_experiences: 0,
            runtime_kv_hold_experiences: 0,
            runtime_kv_held_blocks: 0,
            reflection_issue_experiences: 0,
            critical_reflection_issue_experiences: 0,
            revision_action_experiences: 0,
            live_memory_feedback_experiences: 0,
            live_memory_feedback_updates: 0,
            live_memory_feedback_detail_experiences: 0,
            live_memory_feedback_applied: 0,
            live_memory_feedback_removed: 0,
            live_memory_feedback_missing: 0,
            live_memory_feedback_strength_delta: 0.0,
            evolution_live_inference_runs: 0,
            evolution_live_router_threshold_mutations: 0,
            evolution_live_hierarchy_weight_mutations: 0,
            evolution_live_memory_updates: 0,
            evolution_live_stored_memory_updates: 0,
            evolution_live_reflection_issues: 0,
            evolution_live_critical_reflection_issues: 0,
            evolution_live_revision_actions: 0,
            evolution_replay_runs: 0,
            evolution_replay_items: 0,
            evolution_router_threshold_mutations: 0,
            evolution_hierarchy_weight_mutations: 0,
            evolution_memory_updates: 0,
            evolution_replay_live_memory_feedback_updates: 0,
            evolution_replay_live_memory_feedback_detail_items: 0,
            evolution_replay_live_memory_feedback_applied: 0,
            evolution_replay_live_memory_feedback_removed: 0,
            evolution_replay_live_memory_feedback_missing: 0,
            evolution_replay_live_memory_feedback_strength_delta: 0.0,
            evolution_replay_live_evolution_items: 0,
            evolution_replay_live_evolution_memory_updates: 0,
            evolution_replay_live_evolution_stored_memory_updates: 0,
            evolution_replay_live_evolution_reflection_issues: 0,
            evolution_replay_live_evolution_critical_reflection_issues: 0,
            evolution_replay_live_evolution_revision_actions: 0,
            evolution_recursive_replay_items: 0,
            evolution_recursive_runtime_calls: 0,
        }
    }

    pub fn from_report(
        device: DeviceClass,
        inspection: &StateInspectionReport,
        report: StateInspectionGateReport,
    ) -> Self {
        Self {
            device,
            report,
            runtime_kv_memories: inspection.runtime_kv_memory_count,
            runtime_model_experiences: inspection.runtime_model_experience_count,
            runtime_adapter_experiences: inspection.runtime_adapter_experience_count,
            runtime_adapter_selection_mismatches: inspection
                .runtime_adapter_selection_mismatch_count,
            runtime_forward_energy_experiences: inspection.runtime_forward_energy_experience_count,
            runtime_kv_influence_experiences: inspection.runtime_kv_influence_experience_count,
            runtime_kv_precision_experiences: inspection.runtime_kv_precision_experience_count,
            runtime_kv_precision_mismatches: inspection.runtime_kv_precision_mismatch_count,
            runtime_device_execution_experiences: inspection
                .runtime_device_execution_experience_count,
            runtime_layer_mode_experiences: inspection.runtime_layer_mode_experience_count,
            runtime_all_layer_mode_experiences: inspection.runtime_all_layer_mode_experience_count,
            runtime_kv_import_experiences: inspection.runtime_kv_import_experience_count,
            runtime_kv_export_experiences: inspection.runtime_kv_export_experience_count,
            runtime_kv_hold_experiences: inspection.runtime_kv_hold_experience_count,
            runtime_kv_held_blocks: inspection.runtime_kv_held_blocks,
            reflection_issue_experiences: inspection.reflection_issue_experience_count,
            critical_reflection_issue_experiences: inspection
                .critical_reflection_issue_experience_count,
            revision_action_experiences: inspection.revision_action_experience_count,
            live_memory_feedback_experiences: inspection.live_memory_feedback_experience_count,
            live_memory_feedback_updates: inspection.live_memory_feedback_update_count,
            live_memory_feedback_detail_experiences: inspection
                .live_memory_feedback_detail_experience_count,
            live_memory_feedback_applied: inspection.live_memory_feedback_applied_count,
            live_memory_feedback_removed: inspection.live_memory_feedback_removed_count,
            live_memory_feedback_missing: inspection.live_memory_feedback_missing_count,
            live_memory_feedback_strength_delta: inspection.live_memory_feedback_strength_delta,
            evolution_live_inference_runs: inspection.evolution_ledger.live_inference_runs,
            evolution_live_router_threshold_mutations: inspection
                .evolution_ledger
                .live_router_threshold_mutations,
            evolution_live_hierarchy_weight_mutations: inspection
                .evolution_ledger
                .live_hierarchy_weight_mutations,
            evolution_live_memory_updates: inspection.evolution_ledger.live_memory_updates(),
            evolution_live_stored_memory_updates: inspection
                .evolution_ledger
                .live_stored_memory_updates(),
            evolution_live_reflection_issues: inspection.evolution_ledger.live_reflection_issues,
            evolution_live_critical_reflection_issues: inspection
                .evolution_ledger
                .live_critical_reflection_issues,
            evolution_live_revision_actions: inspection.evolution_ledger.live_revision_actions,
            evolution_replay_runs: inspection.evolution_ledger.replay_runs,
            evolution_replay_items: inspection.evolution_ledger.replay_items,
            evolution_router_threshold_mutations: inspection
                .evolution_ledger
                .router_threshold_mutations,
            evolution_hierarchy_weight_mutations: inspection
                .evolution_ledger
                .hierarchy_weight_mutations,
            evolution_memory_updates: inspection.evolution_ledger.memory_updates(),
            evolution_replay_live_memory_feedback_updates: inspection
                .evolution_ledger
                .replay_live_memory_feedback_updates(),
            evolution_replay_live_memory_feedback_detail_items: inspection
                .evolution_ledger
                .replay_live_memory_feedback_detail_items,
            evolution_replay_live_memory_feedback_applied: inspection
                .evolution_ledger
                .replay_live_memory_feedback_applied,
            evolution_replay_live_memory_feedback_removed: inspection
                .evolution_ledger
                .replay_live_memory_feedback_removed,
            evolution_replay_live_memory_feedback_missing: inspection
                .evolution_ledger
                .replay_live_memory_feedback_missing,
            evolution_replay_live_memory_feedback_strength_delta: inspection
                .evolution_ledger
                .replay_live_memory_feedback_strength_delta,
            evolution_replay_live_evolution_items: inspection
                .evolution_ledger
                .replay_live_evolution_items,
            evolution_replay_live_evolution_memory_updates: inspection
                .evolution_ledger
                .replay_live_evolution_memory_updates,
            evolution_replay_live_evolution_stored_memory_updates: inspection
                .evolution_ledger
                .replay_live_evolution_stored_memory_updates,
            evolution_replay_live_evolution_reflection_issues: inspection
                .evolution_ledger
                .replay_live_evolution_reflection_issues,
            evolution_replay_live_evolution_critical_reflection_issues: inspection
                .evolution_ledger
                .replay_live_evolution_critical_reflection_issues,
            evolution_replay_live_evolution_revision_actions: inspection
                .evolution_ledger
                .replay_live_evolution_revision_actions,
            evolution_recursive_replay_items: inspection.evolution_ledger.recursive_replay_items,
            evolution_recursive_runtime_calls: inspection.evolution_ledger.recursive_runtime_calls,
        }
    }

    pub fn with_runtime_evidence(
        mut self,
        runtime_kv_memories: usize,
        runtime_model_experiences: usize,
        runtime_adapter_experiences: usize,
        runtime_forward_energy_experiences: usize,
        runtime_kv_influence_experiences: usize,
        runtime_device_execution_experiences: usize,
        runtime_kv_import_experiences: usize,
        runtime_kv_export_experiences: usize,
    ) -> Self {
        self.runtime_kv_memories = runtime_kv_memories;
        self.runtime_model_experiences = runtime_model_experiences;
        self.runtime_adapter_experiences = runtime_adapter_experiences;
        self.runtime_forward_energy_experiences = runtime_forward_energy_experiences;
        self.runtime_kv_influence_experiences = runtime_kv_influence_experiences;
        self.runtime_device_execution_experiences = runtime_device_execution_experiences;
        self.runtime_kv_import_experiences = runtime_kv_import_experiences;
        self.runtime_kv_export_experiences = runtime_kv_export_experiences;
        self
    }

    pub fn with_runtime_kv_hold_evidence(
        mut self,
        runtime_kv_hold_experiences: usize,
        runtime_kv_held_blocks: usize,
    ) -> Self {
        self.runtime_kv_hold_experiences = runtime_kv_hold_experiences;
        self.runtime_kv_held_blocks = runtime_kv_held_blocks;
        self
    }

    pub fn with_runtime_adapter_selection_mismatches(
        mut self,
        runtime_adapter_selection_mismatches: usize,
    ) -> Self {
        self.runtime_adapter_selection_mismatches = runtime_adapter_selection_mismatches;
        self
    }

    pub fn with_runtime_kv_precision_evidence(
        mut self,
        runtime_kv_precision_experiences: usize,
    ) -> Self {
        self.runtime_kv_precision_experiences = runtime_kv_precision_experiences;
        self
    }

    pub fn with_runtime_kv_precision_mismatches(
        mut self,
        runtime_kv_precision_mismatches: usize,
    ) -> Self {
        self.runtime_kv_precision_mismatches = runtime_kv_precision_mismatches;
        self
    }

    pub fn with_runtime_layer_mode_evidence(
        mut self,
        runtime_layer_mode_experiences: usize,
        runtime_all_layer_mode_experiences: usize,
    ) -> Self {
        self.runtime_layer_mode_experiences = runtime_layer_mode_experiences;
        self.runtime_all_layer_mode_experiences = runtime_all_layer_mode_experiences;
        self
    }

    pub fn with_reflection_evidence(
        mut self,
        reflection_issue_experiences: usize,
        critical_reflection_issue_experiences: usize,
        revision_action_experiences: usize,
    ) -> Self {
        self.reflection_issue_experiences = reflection_issue_experiences;
        self.critical_reflection_issue_experiences = critical_reflection_issue_experiences;
        self.revision_action_experiences = revision_action_experiences;
        self
    }

    pub fn with_live_memory_feedback_evidence(
        mut self,
        live_memory_feedback_experiences: usize,
        live_memory_feedback_updates: usize,
    ) -> Self {
        self.live_memory_feedback_experiences = live_memory_feedback_experiences;
        self.live_memory_feedback_updates = live_memory_feedback_updates;
        self
    }

    pub fn with_live_memory_feedback_detail_evidence(
        mut self,
        live_memory_feedback_detail_experiences: usize,
        live_memory_feedback_applied: usize,
        live_memory_feedback_removed: usize,
        live_memory_feedback_missing: usize,
        live_memory_feedback_strength_delta: f32,
    ) -> Self {
        self.live_memory_feedback_detail_experiences = live_memory_feedback_detail_experiences;
        self.live_memory_feedback_applied = live_memory_feedback_applied;
        self.live_memory_feedback_removed = live_memory_feedback_removed;
        self.live_memory_feedback_missing = live_memory_feedback_missing;
        self.live_memory_feedback_strength_delta = live_memory_feedback_strength_delta;
        self
    }

    pub fn with_live_evolution_evidence(
        mut self,
        inference_runs: u64,
        router_threshold_mutations: u64,
        hierarchy_weight_mutations: u64,
        memory_updates: u64,
        stored_memory_updates: u64,
        reflection_issues: u64,
        critical_reflection_issues: u64,
        revision_actions: u64,
    ) -> Self {
        self.evolution_live_inference_runs = inference_runs;
        self.evolution_live_router_threshold_mutations = router_threshold_mutations;
        self.evolution_live_hierarchy_weight_mutations = hierarchy_weight_mutations;
        self.evolution_live_memory_updates = memory_updates;
        self.evolution_live_stored_memory_updates = stored_memory_updates;
        self.evolution_live_reflection_issues = reflection_issues;
        self.evolution_live_critical_reflection_issues = critical_reflection_issues;
        self.evolution_live_revision_actions = revision_actions;
        self
    }

    pub fn with_evolution_evidence(
        mut self,
        replay_runs: u64,
        replay_items: u64,
        router_threshold_mutations: u64,
        hierarchy_weight_mutations: u64,
        memory_updates: u64,
        replay_live_memory_feedback_updates: u64,
        recursive_replay_items: u64,
        recursive_runtime_calls: u64,
    ) -> Self {
        self.evolution_replay_runs = replay_runs;
        self.evolution_replay_items = replay_items;
        self.evolution_router_threshold_mutations = router_threshold_mutations;
        self.evolution_hierarchy_weight_mutations = hierarchy_weight_mutations;
        self.evolution_memory_updates = memory_updates;
        self.evolution_replay_live_memory_feedback_updates = replay_live_memory_feedback_updates;
        self.evolution_recursive_replay_items = recursive_replay_items;
        self.evolution_recursive_runtime_calls = recursive_runtime_calls;
        self
    }

    pub fn with_evolution_replay_live_memory_feedback_detail_evidence(
        mut self,
        detail_items: u64,
        applied: u64,
        removed: u64,
        missing: u64,
        strength_delta: f32,
    ) -> Self {
        self.evolution_replay_live_memory_feedback_detail_items = detail_items;
        self.evolution_replay_live_memory_feedback_applied = applied;
        self.evolution_replay_live_memory_feedback_removed = removed;
        self.evolution_replay_live_memory_feedback_missing = missing;
        self.evolution_replay_live_memory_feedback_strength_delta = strength_delta;
        self
    }

    pub fn with_evolution_replay_live_evolution_evidence(
        mut self,
        items: u64,
        memory_updates: u64,
        stored_memory_updates: u64,
        reflection_issues: u64,
        critical_reflection_issues: u64,
        revision_actions: u64,
    ) -> Self {
        self.evolution_replay_live_evolution_items = items;
        self.evolution_replay_live_evolution_memory_updates = memory_updates;
        self.evolution_replay_live_evolution_stored_memory_updates = stored_memory_updates;
        self.evolution_replay_live_evolution_reflection_issues = reflection_issues;
        self.evolution_replay_live_evolution_critical_reflection_issues =
            critical_reflection_issues;
        self.evolution_replay_live_evolution_revision_actions = revision_actions;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateInspectionMatrixGateReport {
    pub passed: bool,
    pub device_reports: Vec<StateInspectionDeviceGateReport>,
    pub failures: Vec<String>,
}

impl StateInspectionMatrixGateReport {
    pub fn evaluate(device_reports: Vec<StateInspectionDeviceGateReport>) -> Self {
        Self::evaluate_with_gate(device_reports, &StateInspectionMatrixGate::default())
    }

    pub fn evaluate_with_gate(
        device_reports: Vec<StateInspectionDeviceGateReport>,
        gate: &StateInspectionMatrixGate,
    ) -> Self {
        let mut failures = Vec::new();

        if device_reports.is_empty() {
            failures.push("no state inspection device reports were recorded".to_owned());
        }

        let missing = missing_state_inspection_devices(&device_reports);
        if !missing.is_empty() {
            failures.push(format!(
                "state_inspection_devices {} below expected {} missing={}",
                explicit_state_inspection_devices(&device_reports),
                DeviceClass::explicit_profiles().len(),
                missing
                    .iter()
                    .map(|device| device.as_str())
                    .collect::<Vec<_>>()
                    .join("+")
            ));
        }

        for device_report in &device_reports {
            if !device_report.report.passed() {
                failures.push(format!(
                    "device {} state inspection failed with {} failures",
                    device_report.device.as_str(),
                    device_report.report.failures.len()
                ));
            }
        }

        require_min_device_profiles(
            &mut failures,
            "runtime_kv_memory_device_profiles",
            runtime_kv_memory_device_profiles(&device_reports),
            gate.min_runtime_kv_memory_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_model_device_profiles",
            runtime_model_device_profiles(&device_reports),
            gate.min_runtime_model_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_adapter_device_profiles",
            runtime_adapter_device_profiles(&device_reports),
            gate.min_runtime_adapter_device_profiles,
        );
        require_max_usize(
            &mut failures,
            "runtime_adapter_selection_mismatches",
            runtime_adapter_selection_mismatches(&device_reports),
            gate.max_runtime_adapter_selection_mismatches,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_forward_energy_device_profiles",
            runtime_forward_energy_device_profiles(&device_reports),
            gate.min_runtime_forward_energy_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_influence_device_profiles",
            runtime_kv_influence_device_profiles(&device_reports),
            gate.min_runtime_kv_influence_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_precision_device_profiles",
            runtime_kv_precision_device_profiles(&device_reports),
            gate.min_runtime_kv_precision_device_profiles,
        );
        require_max_usize(
            &mut failures,
            "runtime_kv_precision_mismatches",
            runtime_kv_precision_mismatches(&device_reports),
            gate.max_runtime_kv_precision_mismatches,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_device_execution_device_profiles",
            runtime_device_execution_device_profiles(&device_reports),
            gate.min_runtime_device_execution_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_layer_mode_device_profiles",
            runtime_layer_mode_device_profiles(&device_reports),
            gate.min_runtime_layer_mode_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_all_layer_mode_device_profiles",
            runtime_all_layer_mode_device_profiles(&device_reports),
            gate.min_runtime_all_layer_mode_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_import_device_profiles",
            runtime_kv_import_device_profiles(&device_reports),
            gate.min_runtime_kv_import_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_export_device_profiles",
            runtime_kv_export_device_profiles(&device_reports),
            gate.min_runtime_kv_export_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "runtime_kv_hold_device_profiles",
            runtime_kv_hold_device_profiles(&device_reports),
            gate.min_runtime_kv_hold_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "reflection_issue_device_profiles",
            reflection_issue_device_profiles(&device_reports),
            gate.min_reflection_issue_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "critical_reflection_issue_device_profiles",
            critical_reflection_issue_device_profiles(&device_reports),
            gate.min_critical_reflection_issue_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "revision_action_device_profiles",
            revision_action_device_profiles(&device_reports),
            gate.min_revision_action_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "live_memory_feedback_device_profiles",
            live_memory_feedback_device_profiles(&device_reports),
            gate.min_live_memory_feedback_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_inference_device_profiles",
            evolution_live_inference_device_profiles(&device_reports),
            gate.min_evolution_live_inference_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_router_threshold_mutation_device_profiles",
            evolution_live_router_threshold_mutation_device_profiles(&device_reports),
            gate.min_evolution_live_router_threshold_mutation_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_hierarchy_weight_mutation_device_profiles",
            evolution_live_hierarchy_weight_mutation_device_profiles(&device_reports),
            gate.min_evolution_live_hierarchy_weight_mutation_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_memory_update_device_profiles",
            evolution_live_memory_update_device_profiles(&device_reports),
            gate.min_evolution_live_memory_update_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_stored_memory_update_device_profiles",
            evolution_live_stored_memory_update_device_profiles(&device_reports),
            gate.min_evolution_live_stored_memory_update_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_reflection_issue_device_profiles",
            evolution_live_reflection_issue_device_profiles(&device_reports),
            gate.min_evolution_live_reflection_issue_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_critical_reflection_issue_device_profiles",
            evolution_live_critical_reflection_issue_device_profiles(&device_reports),
            gate.min_evolution_live_critical_reflection_issue_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_live_revision_action_device_profiles",
            evolution_live_revision_action_device_profiles(&device_reports),
            gate.min_evolution_live_revision_action_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_run_device_profiles",
            evolution_replay_run_device_profiles(&device_reports),
            gate.min_evolution_replay_run_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_item_device_profiles",
            evolution_replay_item_device_profiles(&device_reports),
            gate.min_evolution_replay_item_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_router_threshold_mutation_device_profiles",
            evolution_router_threshold_mutation_device_profiles(&device_reports),
            gate.min_evolution_router_threshold_mutation_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_hierarchy_weight_mutation_device_profiles",
            evolution_hierarchy_weight_mutation_device_profiles(&device_reports),
            gate.min_evolution_hierarchy_weight_mutation_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_memory_update_device_profiles",
            evolution_memory_update_device_profiles(&device_reports),
            gate.min_evolution_memory_update_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_memory_feedback_device_profiles",
            evolution_replay_live_memory_feedback_device_profiles(&device_reports),
            gate.min_evolution_replay_live_memory_feedback_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_memory_feedback_detail_device_profiles",
            evolution_replay_live_memory_feedback_detail_device_profiles(&device_reports),
            gate.min_evolution_replay_live_memory_feedback_detail_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_evolution_device_profiles",
            evolution_replay_live_evolution_device_profiles(&device_reports),
            gate.min_evolution_replay_live_evolution_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_evolution_memory_update_device_profiles",
            evolution_replay_live_evolution_memory_update_device_profiles(&device_reports),
            gate.min_evolution_replay_live_evolution_memory_update_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_evolution_critical_reflection_issue_device_profiles",
            evolution_replay_live_evolution_critical_reflection_issue_device_profiles(
                &device_reports,
            ),
            gate.min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_replay_live_evolution_revision_action_device_profiles",
            evolution_replay_live_evolution_revision_action_device_profiles(&device_reports),
            gate.min_evolution_replay_live_evolution_revision_action_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_recursive_replay_device_profiles",
            evolution_recursive_replay_device_profiles(&device_reports),
            gate.min_evolution_recursive_replay_device_profiles,
        );
        require_min_device_profiles(
            &mut failures,
            "evolution_recursive_runtime_call_device_profiles",
            evolution_recursive_runtime_call_device_profiles(&device_reports),
            gate.min_evolution_recursive_runtime_call_device_profiles,
        );

        Self {
            passed: failures.is_empty(),
            device_reports,
            failures,
        }
    }

    pub fn passed(&self) -> bool {
        self.passed
    }

    pub fn covered_devices(&self) -> usize {
        explicit_state_inspection_devices(&self.device_reports)
    }

    pub fn missing_devices(&self) -> Vec<DeviceClass> {
        missing_state_inspection_devices(&self.device_reports)
    }

    pub fn failed_devices(&self) -> Vec<DeviceClass> {
        self.device_reports
            .iter()
            .filter(|device_report| !device_report.report.passed())
            .map(|device_report| device_report.device)
            .collect()
    }

    pub fn runtime_kv_memory_device_profiles(&self) -> usize {
        runtime_kv_memory_device_profiles(&self.device_reports)
    }

    pub fn runtime_model_device_profiles(&self) -> usize {
        runtime_model_device_profiles(&self.device_reports)
    }

    pub fn runtime_adapter_device_profiles(&self) -> usize {
        runtime_adapter_device_profiles(&self.device_reports)
    }

    pub fn runtime_adapter_selection_mismatches(&self) -> usize {
        runtime_adapter_selection_mismatches(&self.device_reports)
    }

    pub fn runtime_forward_energy_device_profiles(&self) -> usize {
        runtime_forward_energy_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_influence_device_profiles(&self) -> usize {
        runtime_kv_influence_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_precision_device_profiles(&self) -> usize {
        runtime_kv_precision_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_precision_mismatches(&self) -> usize {
        runtime_kv_precision_mismatches(&self.device_reports)
    }

    pub fn runtime_device_execution_device_profiles(&self) -> usize {
        runtime_device_execution_device_profiles(&self.device_reports)
    }

    pub fn runtime_layer_mode_device_profiles(&self) -> usize {
        runtime_layer_mode_device_profiles(&self.device_reports)
    }

    pub fn runtime_all_layer_mode_device_profiles(&self) -> usize {
        runtime_all_layer_mode_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_import_device_profiles(&self) -> usize {
        runtime_kv_import_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_export_device_profiles(&self) -> usize {
        runtime_kv_export_device_profiles(&self.device_reports)
    }

    pub fn runtime_kv_hold_device_profiles(&self) -> usize {
        runtime_kv_hold_device_profiles(&self.device_reports)
    }

    pub fn reflection_issue_device_profiles(&self) -> usize {
        reflection_issue_device_profiles(&self.device_reports)
    }

    pub fn critical_reflection_issue_device_profiles(&self) -> usize {
        critical_reflection_issue_device_profiles(&self.device_reports)
    }

    pub fn revision_action_device_profiles(&self) -> usize {
        revision_action_device_profiles(&self.device_reports)
    }

    pub fn live_memory_feedback_device_profiles(&self) -> usize {
        live_memory_feedback_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_inference_device_profiles(&self) -> usize {
        evolution_live_inference_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_router_threshold_mutation_device_profiles(&self) -> usize {
        evolution_live_router_threshold_mutation_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_hierarchy_weight_mutation_device_profiles(&self) -> usize {
        evolution_live_hierarchy_weight_mutation_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_memory_update_device_profiles(&self) -> usize {
        evolution_live_memory_update_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_stored_memory_update_device_profiles(&self) -> usize {
        evolution_live_stored_memory_update_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_reflection_issue_device_profiles(&self) -> usize {
        evolution_live_reflection_issue_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_critical_reflection_issue_device_profiles(&self) -> usize {
        evolution_live_critical_reflection_issue_device_profiles(&self.device_reports)
    }

    pub fn evolution_live_revision_action_device_profiles(&self) -> usize {
        evolution_live_revision_action_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_run_device_profiles(&self) -> usize {
        evolution_replay_run_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_item_device_profiles(&self) -> usize {
        evolution_replay_item_device_profiles(&self.device_reports)
    }

    pub fn evolution_router_threshold_mutation_device_profiles(&self) -> usize {
        evolution_router_threshold_mutation_device_profiles(&self.device_reports)
    }

    pub fn evolution_hierarchy_weight_mutation_device_profiles(&self) -> usize {
        evolution_hierarchy_weight_mutation_device_profiles(&self.device_reports)
    }

    pub fn evolution_memory_update_device_profiles(&self) -> usize {
        evolution_memory_update_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_live_memory_feedback_device_profiles(&self) -> usize {
        evolution_replay_live_memory_feedback_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_live_memory_feedback_detail_device_profiles(&self) -> usize {
        evolution_replay_live_memory_feedback_detail_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_live_evolution_device_profiles(&self) -> usize {
        evolution_replay_live_evolution_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_live_evolution_memory_update_device_profiles(&self) -> usize {
        evolution_replay_live_evolution_memory_update_device_profiles(&self.device_reports)
    }

    pub fn evolution_replay_live_evolution_critical_reflection_issue_device_profiles(
        &self,
    ) -> usize {
        evolution_replay_live_evolution_critical_reflection_issue_device_profiles(
            &self.device_reports,
        )
    }

    pub fn evolution_replay_live_evolution_revision_action_device_profiles(&self) -> usize {
        evolution_replay_live_evolution_revision_action_device_profiles(&self.device_reports)
    }

    pub fn evolution_recursive_replay_device_profiles(&self) -> usize {
        evolution_recursive_replay_device_profiles(&self.device_reports)
    }

    pub fn evolution_recursive_runtime_call_device_profiles(&self) -> usize {
        evolution_recursive_runtime_call_device_profiles(&self.device_reports)
    }

    pub fn summary_line(&self) -> String {
        format!(
            "state_inspection_matrix_gate: passed={} devices={} expected_devices={} failed_devices={} runtime_kv_memory_device_profiles={} runtime_model_device_profiles={} runtime_adapter_device_profiles={} runtime_adapter_selection_mismatches={} runtime_forward_energy_device_profiles={} runtime_kv_influence_device_profiles={} runtime_kv_precision_device_profiles={} runtime_kv_precision_mismatches={} runtime_device_execution_device_profiles={} runtime_layer_mode_device_profiles={} runtime_all_layer_mode_device_profiles={} runtime_kv_import_device_profiles={} runtime_kv_export_device_profiles={} runtime_kv_hold_device_profiles={} reflection_issue_device_profiles={} critical_reflection_issue_device_profiles={} revision_action_device_profiles={} live_memory_feedback_device_profiles={} evolution_live_inference_device_profiles={} evolution_live_router_threshold_mutation_device_profiles={} evolution_live_hierarchy_weight_mutation_device_profiles={} evolution_live_memory_update_device_profiles={} evolution_live_stored_memory_update_device_profiles={} evolution_live_reflection_issue_device_profiles={} evolution_live_critical_reflection_issue_device_profiles={} evolution_live_revision_action_device_profiles={} evolution_replay_run_device_profiles={} evolution_replay_item_device_profiles={} evolution_router_threshold_mutation_device_profiles={} evolution_hierarchy_weight_mutation_device_profiles={} evolution_memory_update_device_profiles={} evolution_replay_live_memory_feedback_device_profiles={} evolution_replay_live_memory_feedback_detail_device_profiles={} evolution_replay_live_evolution_device_profiles={} evolution_replay_live_evolution_memory_update_device_profiles={} evolution_replay_live_evolution_critical_reflection_issue_device_profiles={} evolution_replay_live_evolution_revision_action_device_profiles={} evolution_recursive_replay_device_profiles={} evolution_recursive_runtime_call_device_profiles={} failures={}",
            self.passed,
            self.covered_devices(),
            DeviceClass::explicit_profiles().len(),
            self.failed_devices().len(),
            self.runtime_kv_memory_device_profiles(),
            self.runtime_model_device_profiles(),
            self.runtime_adapter_device_profiles(),
            self.runtime_adapter_selection_mismatches(),
            self.runtime_forward_energy_device_profiles(),
            self.runtime_kv_influence_device_profiles(),
            self.runtime_kv_precision_device_profiles(),
            self.runtime_kv_precision_mismatches(),
            self.runtime_device_execution_device_profiles(),
            self.runtime_layer_mode_device_profiles(),
            self.runtime_all_layer_mode_device_profiles(),
            self.runtime_kv_import_device_profiles(),
            self.runtime_kv_export_device_profiles(),
            self.runtime_kv_hold_device_profiles(),
            self.reflection_issue_device_profiles(),
            self.critical_reflection_issue_device_profiles(),
            self.revision_action_device_profiles(),
            self.live_memory_feedback_device_profiles(),
            self.evolution_live_inference_device_profiles(),
            self.evolution_live_router_threshold_mutation_device_profiles(),
            self.evolution_live_hierarchy_weight_mutation_device_profiles(),
            self.evolution_live_memory_update_device_profiles(),
            self.evolution_live_stored_memory_update_device_profiles(),
            self.evolution_live_reflection_issue_device_profiles(),
            self.evolution_live_critical_reflection_issue_device_profiles(),
            self.evolution_live_revision_action_device_profiles(),
            self.evolution_replay_run_device_profiles(),
            self.evolution_replay_item_device_profiles(),
            self.evolution_router_threshold_mutation_device_profiles(),
            self.evolution_hierarchy_weight_mutation_device_profiles(),
            self.evolution_memory_update_device_profiles(),
            self.evolution_replay_live_memory_feedback_device_profiles(),
            self.evolution_replay_live_memory_feedback_detail_device_profiles(),
            self.evolution_replay_live_evolution_device_profiles(),
            self.evolution_replay_live_evolution_memory_update_device_profiles(),
            self.evolution_replay_live_evolution_critical_reflection_issue_device_profiles(),
            self.evolution_replay_live_evolution_revision_action_device_profiles(),
            self.evolution_recursive_replay_device_profiles(),
            self.evolution_recursive_runtime_call_device_profiles(),
            self.failures.len()
        )
    }
}

fn runtime_kv_memory_device_profiles(device_reports: &[StateInspectionDeviceGateReport]) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_memories > 0
    })
}

fn runtime_model_device_profiles(device_reports: &[StateInspectionDeviceGateReport]) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_model_experiences > 0
    })
}

fn runtime_adapter_device_profiles(device_reports: &[StateInspectionDeviceGateReport]) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_adapter_experiences > 0
    })
}

fn runtime_adapter_selection_mismatches(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    device_reports
        .iter()
        .map(|device_report| device_report.runtime_adapter_selection_mismatches)
        .sum()
}

fn runtime_forward_energy_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_forward_energy_experiences > 0
    })
}

fn runtime_kv_influence_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_influence_experiences > 0
    })
}

fn runtime_kv_precision_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_precision_experiences > 0
    })
}

fn runtime_kv_precision_mismatches(device_reports: &[StateInspectionDeviceGateReport]) -> usize {
    device_reports
        .iter()
        .map(|device_report| device_report.runtime_kv_precision_mismatches)
        .sum()
}

fn runtime_device_execution_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_device_execution_experiences > 0
    })
}

fn runtime_layer_mode_device_profiles(device_reports: &[StateInspectionDeviceGateReport]) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_layer_mode_experiences > 0
    })
}

fn runtime_all_layer_mode_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_all_layer_mode_experiences > 0
    })
}

fn runtime_kv_import_device_profiles(device_reports: &[StateInspectionDeviceGateReport]) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_import_experiences > 0
    })
}

fn runtime_kv_export_device_profiles(device_reports: &[StateInspectionDeviceGateReport]) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_export_experiences > 0
    })
}

fn runtime_kv_hold_device_profiles(device_reports: &[StateInspectionDeviceGateReport]) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.runtime_kv_hold_experiences > 0 || device_report.runtime_kv_held_blocks > 0
    })
}

fn reflection_issue_device_profiles(device_reports: &[StateInspectionDeviceGateReport]) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.reflection_issue_experiences > 0
    })
}

fn runtime_kv_held_blocks(record: &ExperienceRecord) -> usize {
    record
        .runtime_diagnostics
        .exported_kv_blocks
        .saturating_sub(record.stored_runtime_kv_memory_ids.len())
}

fn runtime_kv_was_held(record: &ExperienceRecord) -> bool {
    runtime_kv_held_blocks(record) > 0
}

fn critical_reflection_issue_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.critical_reflection_issue_experiences > 0
    })
}

fn revision_action_device_profiles(device_reports: &[StateInspectionDeviceGateReport]) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.revision_action_experiences > 0
    })
}

fn live_memory_feedback_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.live_memory_feedback_experiences > 0
            && device_report.live_memory_feedback_updates > 0
            && device_report.live_memory_feedback_detail_experiences > 0
            && device_report
                .live_memory_feedback_applied
                .saturating_add(device_report.live_memory_feedback_missing)
                == device_report.live_memory_feedback_updates
            && device_report.live_memory_feedback_removed
                <= device_report.live_memory_feedback_applied
            && device_report
                .live_memory_feedback_strength_delta
                .is_finite()
            && device_report.live_memory_feedback_strength_delta >= 0.0
    })
}

fn evolution_live_inference_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_inference_runs > 0
    })
}

fn evolution_live_router_threshold_mutation_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_router_threshold_mutations > 0
    })
}

fn evolution_live_hierarchy_weight_mutation_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_hierarchy_weight_mutations > 0
    })
}

fn evolution_live_memory_update_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_memory_updates > 0
    })
}

fn evolution_live_stored_memory_update_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_stored_memory_updates > 0
    })
}

fn evolution_live_reflection_issue_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_reflection_issues > 0
    })
}

fn evolution_live_critical_reflection_issue_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_critical_reflection_issues > 0
    })
}

fn evolution_live_revision_action_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_live_revision_actions > 0
    })
}

fn evolution_replay_run_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_runs > 0
    })
}

fn evolution_replay_item_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_items > 0
    })
}

fn evolution_router_threshold_mutation_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_router_threshold_mutations > 0
    })
}

fn evolution_hierarchy_weight_mutation_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_hierarchy_weight_mutations > 0
    })
}

fn evolution_memory_update_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_memory_updates > 0
    })
}

fn evolution_replay_live_memory_feedback_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_memory_feedback_updates > 0
    })
}

fn evolution_replay_live_memory_feedback_detail_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_memory_feedback_detail_items > 0
            && device_report
                .evolution_replay_live_memory_feedback_applied
                .saturating_add(device_report.evolution_replay_live_memory_feedback_missing)
                <= device_report.evolution_replay_live_memory_feedback_updates
            && device_report.evolution_replay_live_memory_feedback_removed
                <= device_report.evolution_replay_live_memory_feedback_applied
            && device_report
                .evolution_replay_live_memory_feedback_strength_delta
                .is_finite()
            && device_report.evolution_replay_live_memory_feedback_strength_delta >= 0.0
    })
}

fn evolution_replay_live_evolution_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_evolution_items > 0
    })
}

fn evolution_replay_live_evolution_memory_update_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_evolution_memory_updates > 0
    })
}

fn evolution_replay_live_evolution_critical_reflection_issue_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_evolution_critical_reflection_issues > 0
    })
}

fn evolution_replay_live_evolution_revision_action_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_replay_live_evolution_revision_actions > 0
    })
}

fn evolution_recursive_replay_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_recursive_replay_items > 0
    })
}

fn evolution_recursive_runtime_call_device_profiles(
    device_reports: &[StateInspectionDeviceGateReport],
) -> usize {
    explicit_state_inspection_evidence_devices(device_reports, |device_report| {
        device_report.evolution_recursive_runtime_calls > 0
    })
}

fn explicit_state_inspection_evidence_devices<F>(
    device_reports: &[StateInspectionDeviceGateReport],
    has_evidence: F,
) -> usize
where
    F: Fn(&StateInspectionDeviceGateReport) -> bool,
{
    DeviceClass::explicit_profiles()
        .iter()
        .filter(|device| {
            device_reports.iter().any(|device_report| {
                device_report.device == **device && has_evidence(device_report)
            })
        })
        .count()
}

fn require_min_device_profiles(
    failures: &mut Vec<String>,
    name: &str,
    actual: usize,
    required: Option<usize>,
) {
    if let Some(required) = required {
        if actual < required {
            failures.push(format!("{name} {actual} below required {required}"));
        }
    }
}

fn explicit_state_inspection_devices(device_reports: &[StateInspectionDeviceGateReport]) -> usize {
    DeviceClass::explicit_profiles()
        .iter()
        .filter(|device| {
            device_reports
                .iter()
                .any(|device_report| device_report.device == **device)
        })
        .count()
}

fn missing_state_inspection_devices(
    device_reports: &[StateInspectionDeviceGateReport],
) -> Vec<DeviceClass> {
    DeviceClass::explicit_profiles()
        .iter()
        .copied()
        .filter(|device| {
            !device_reports
                .iter()
                .any(|device_report| device_report.device == *device)
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct StateInspectionReport {
    pub memory_count: usize,
    pub runtime_kv_memory_count: usize,
    pub experience_count: usize,
    pub runtime_model_experience_count: usize,
    pub runtime_adapter_experience_count: usize,
    pub runtime_adapter_selection_mismatch_count: usize,
    pub runtime_forward_energy_experience_count: usize,
    pub runtime_kv_influence_experience_count: usize,
    pub runtime_kv_precision_experience_count: usize,
    pub runtime_kv_precision_mismatch_count: usize,
    pub runtime_device_execution_experience_count: usize,
    pub runtime_layer_mode_experience_count: usize,
    pub runtime_all_layer_mode_experience_count: usize,
    pub runtime_global_layers: usize,
    pub runtime_local_window_layers: usize,
    pub runtime_convolutional_fusion_layers: usize,
    pub runtime_kv_import_experience_count: usize,
    pub runtime_kv_export_experience_count: usize,
    pub runtime_kv_hold_experience_count: usize,
    pub runtime_kv_held_blocks: usize,
    pub reflection_issue_experience_count: usize,
    pub critical_reflection_issue_experience_count: usize,
    pub revision_action_experience_count: usize,
    pub live_memory_feedback_experience_count: usize,
    pub live_memory_feedback_update_count: usize,
    pub live_memory_feedback_detail_experience_count: usize,
    pub live_memory_feedback_applied_count: usize,
    pub live_memory_feedback_removed_count: usize,
    pub live_memory_feedback_missing_count: usize,
    pub live_memory_feedback_strength_delta: f32,
    pub router_threshold: f32,
    pub router_observations: u64,
    pub profile_thresholds: ProfileThresholds,
    pub profile_observations: ProfileObservations,
    pub hierarchy: HierarchyWeights,
    pub profile_hierarchy_weights: ProfileHierarchyWeights,
    pub profile_hierarchy_observations: ProfileHierarchyObservations,
    pub tier_counts: TierCounts,
    pub memory_retention_policy: MemoryRetentionPolicy,
    pub memory_compaction_policy: MemoryCompactionPolicy,
    pub evolution_ledger: EvolutionLedger,
    pub memory_vector_dimensions: Vec<StateMemoryVectorDimensions>,
    pub runtime_kv_vector_dimensions: Vec<StateMemoryVectorDimensions>,
    pub top_memories: Vec<StateMemorySummary>,
    pub top_runtime_kv_memories: Vec<StateMemorySummary>,
    pub top_experiences: Vec<StateExperienceSummary>,
}

impl StateInspectionReport {
    pub fn from_engine(engine: &NoironEngine, limit: usize) -> Self {
        let limit = limit.max(1);
        let adaptive_state = engine.adaptive_state();
        let top_memories = top_memory_summaries(engine, limit, |_| true);
        let top_runtime_kv_memories =
            top_memory_summaries(engine, limit, |key| key.starts_with("runtime_kv:"));
        let runtime_model_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| has_text(record.runtime_diagnostics.model_id.as_deref()))
            .count();
        let runtime_adapter_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| has_text(record.runtime_diagnostics.selected_adapter.as_deref()))
            .count();
        let runtime_adapter_selection_mismatch_count =
            runtime_adapter_selection_mismatch_count(engine, &inspection_hardware_plan(engine));
        let runtime_forward_energy_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| record.runtime_diagnostics.forward_energy.is_some())
            .count();
        let runtime_kv_influence_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| record.runtime_diagnostics.kv_influence.is_some())
            .count();
        let runtime_kv_precision_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| record.runtime_diagnostics.has_valid_kv_precision_signal())
            .count();
        let runtime_kv_precision_mismatch_count =
            runtime_kv_precision_mismatch_count(engine, &inspection_hardware_plan(engine));
        let runtime_device_execution_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| record.runtime_diagnostics.has_device_execution_signal())
            .count();
        let runtime_layer_mode_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| record.runtime_diagnostics.has_layer_mode_signal())
            .count();
        let runtime_all_layer_mode_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| record.runtime_diagnostics.has_all_layer_modes())
            .count();
        let runtime_global_layers = engine
            .experience
            .records()
            .iter()
            .map(|record| record.runtime_diagnostics.global_layers)
            .sum();
        let runtime_local_window_layers = engine
            .experience
            .records()
            .iter()
            .map(|record| record.runtime_diagnostics.local_window_layers)
            .sum();
        let runtime_convolutional_fusion_layers = engine
            .experience
            .records()
            .iter()
            .map(|record| record.runtime_diagnostics.convolutional_fusion_layers)
            .sum();
        let runtime_kv_import_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| record.runtime_diagnostics.imported_kv_blocks > 0)
            .count();
        let runtime_kv_export_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| {
                record.runtime_diagnostics.exported_kv_blocks > 0
                    || !record.stored_runtime_kv_memory_ids.is_empty()
            })
            .count();
        let runtime_kv_hold_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| runtime_kv_was_held(record))
            .count();
        let runtime_kv_held_blocks = engine
            .experience
            .records()
            .iter()
            .map(runtime_kv_held_blocks)
            .sum::<usize>();
        let reflection_issue_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| !record.reflection_issues.is_empty())
            .count();
        let critical_reflection_issue_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| {
                record
                    .reflection_issues
                    .iter()
                    .any(|issue| issue.severity == crate::reflection::ReflectionSeverity::Critical)
            })
            .count();
        let revision_action_experience_count = engine
            .experience
            .records()
            .iter()
            .filter(|record| !record.revision_actions.is_empty())
            .count();
        let live_memory_feedback_stats = engine
            .experience
            .records()
            .iter()
            .filter_map(|record| LiveMemoryFeedbackStats::from_notes(&record.process_reward.notes))
            .collect::<Vec<_>>();
        let live_memory_feedback_experience_count = live_memory_feedback_stats.len();
        let live_memory_feedback_update_count = live_memory_feedback_stats
            .iter()
            .map(LiveMemoryFeedbackStats::updates)
            .sum::<usize>();
        let live_memory_feedback_detail_experience_count = live_memory_feedback_stats
            .iter()
            .filter(|stats| stats.has_detailed_update_evidence())
            .count();
        let live_memory_feedback_applied_count = live_memory_feedback_stats
            .iter()
            .map(|stats| stats.applied)
            .sum::<usize>();
        let live_memory_feedback_removed_count = live_memory_feedback_stats
            .iter()
            .map(|stats| stats.removed)
            .sum::<usize>();
        let live_memory_feedback_missing_count = live_memory_feedback_stats
            .iter()
            .map(|stats| stats.missing)
            .sum::<usize>();
        let live_memory_feedback_strength_delta = live_memory_feedback_stats
            .iter()
            .map(|stats| stats.strength_delta)
            .sum::<f32>();

        let mut top_experiences = engine.experience.records().iter().collect::<Vec<_>>();
        top_experiences.sort_by(|left, right| {
            right
                .process_reward
                .total
                .partial_cmp(&left.process_reward.total)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    right
                        .quality
                        .partial_cmp(&left.quality)
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| left.id.cmp(&right.id))
        });

        let top_experiences = top_experiences
            .into_iter()
            .take(limit)
            .map(|record| {
                let live_memory_feedback =
                    LiveMemoryFeedbackStats::from_notes(&record.process_reward.notes);
                StateExperienceSummary {
                    id: record.id,
                    profile: record.profile,
                    quality: record.quality,
                    process_reward: record.process_reward.total,
                    reward_action: record.process_reward.action,
                    runtime_model_id: record.runtime_diagnostics.model_id.clone(),
                    runtime_selected_adapter: record.runtime_diagnostics.selected_adapter.clone(),
                    runtime_device_profile: record.runtime_diagnostics.device_profile.clone(),
                    runtime_primary_lane: record.runtime_diagnostics.primary_lane.clone(),
                    runtime_fallback_lane: record.runtime_diagnostics.fallback_lane.clone(),
                    runtime_memory_mode: record.runtime_diagnostics.memory_mode.clone(),
                    runtime_layer_count: record.runtime_diagnostics.layer_count,
                    runtime_global_layers: record.runtime_diagnostics.global_layers,
                    runtime_local_window_layers: record.runtime_diagnostics.local_window_layers,
                    runtime_convolutional_fusion_layers: record
                        .runtime_diagnostics
                        .convolutional_fusion_layers,
                    runtime_hidden_size: record.runtime_diagnostics.hidden_size,
                    runtime_local_window_tokens: record.runtime_diagnostics.local_window_tokens,
                    runtime_forward_energy: record.runtime_diagnostics.forward_energy,
                    runtime_kv_influence: record.runtime_diagnostics.kv_influence,
                    runtime_hot_kv_precision_bits: record.runtime_diagnostics.hot_kv_precision_bits,
                    runtime_cold_kv_precision_bits: record
                        .runtime_diagnostics
                        .cold_kv_precision_bits,
                    runtime_imported_kv_blocks: record.runtime_diagnostics.imported_kv_blocks,
                    runtime_exported_kv_blocks: record.runtime_diagnostics.exported_kv_blocks,
                    recursive_runtime_calls: recursive_runtime_calls_from_notes(
                        &record.process_reward.notes,
                    ),
                    live_memory_feedback_updates: live_memory_feedback
                        .map(|stats| stats.updates())
                        .unwrap_or(0),
                    live_memory_feedback_reinforced: live_memory_feedback
                        .map(|stats| stats.reinforced)
                        .unwrap_or(0),
                    live_memory_feedback_penalized: live_memory_feedback
                        .map(|stats| stats.penalized)
                        .unwrap_or(0),
                    live_memory_feedback_applied: live_memory_feedback
                        .map(|stats| stats.applied)
                        .unwrap_or(0),
                    live_memory_feedback_removed: live_memory_feedback
                        .map(|stats| stats.removed)
                        .unwrap_or(0),
                    live_memory_feedback_missing: live_memory_feedback
                        .map(|stats| stats.missing)
                        .unwrap_or(0),
                    live_memory_feedback_strength_delta: live_memory_feedback
                        .map(|stats| stats.strength_delta)
                        .unwrap_or(0.0),
                    live_memory_feedback_detail: live_memory_feedback
                        .map(|stats| stats.has_detailed_update_evidence())
                        .unwrap_or(false),
                    reflection_issues: record.reflection_issues.len(),
                    critical_reflection_issues: record
                        .reflection_issues
                        .iter()
                        .filter(|issue| {
                            issue.severity == crate::reflection::ReflectionSeverity::Critical
                        })
                        .count(),
                    revision_actions: record.revision_actions.len(),
                    lesson: compact(&record.lesson, 160),
                }
            })
            .collect::<Vec<_>>();

        Self {
            memory_count: engine.cache.len(),
            runtime_kv_memory_count: engine
                .cache
                .entries()
                .iter()
                .filter(|entry| entry.key.starts_with("runtime_kv:"))
                .count(),
            experience_count: engine.experience.len(),
            runtime_model_experience_count,
            runtime_adapter_experience_count,
            runtime_adapter_selection_mismatch_count,
            runtime_forward_energy_experience_count,
            runtime_kv_influence_experience_count,
            runtime_kv_precision_experience_count,
            runtime_kv_precision_mismatch_count,
            runtime_device_execution_experience_count,
            runtime_layer_mode_experience_count,
            runtime_all_layer_mode_experience_count,
            runtime_global_layers,
            runtime_local_window_layers,
            runtime_convolutional_fusion_layers,
            runtime_kv_import_experience_count,
            runtime_kv_export_experience_count,
            runtime_kv_hold_experience_count,
            runtime_kv_held_blocks,
            reflection_issue_experience_count,
            critical_reflection_issue_experience_count,
            revision_action_experience_count,
            live_memory_feedback_experience_count,
            live_memory_feedback_update_count,
            live_memory_feedback_detail_experience_count,
            live_memory_feedback_applied_count,
            live_memory_feedback_removed_count,
            live_memory_feedback_missing_count,
            live_memory_feedback_strength_delta,
            router_threshold: adaptive_state.router.threshold,
            router_observations: adaptive_state.router.observations,
            profile_thresholds: adaptive_state.router.profile_thresholds,
            profile_observations: adaptive_state.router.profile_observations,
            hierarchy: adaptive_state.hierarchy.current,
            profile_hierarchy_weights: adaptive_state.hierarchy.profile_weights,
            profile_hierarchy_observations: adaptive_state.hierarchy.profile_observations,
            tier_counts: adaptive_state.tier_plan.counts(),
            memory_retention_policy: engine.memory_retention_policy,
            memory_compaction_policy: engine.memory_compaction_policy.clone(),
            evolution_ledger: adaptive_state.evolution_ledger,
            memory_vector_dimensions: memory_vector_dimensions(engine),
            runtime_kv_vector_dimensions: runtime_kv_vector_dimensions(engine),
            top_memories,
            top_runtime_kv_memories,
            top_experiences,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "state: memories={} runtime_kv_memories={} experiences={} runtime_model_experiences={} runtime_adapter_experiences={} runtime_adapter_selection_mismatches={} runtime_forward_energy_experiences={} runtime_kv_influence_experiences={} runtime_kv_precision_experiences={} runtime_kv_precision_mismatches={} runtime_device_execution_experiences={} runtime_layer_mode_experiences={} runtime_all_layer_mode_experiences={} runtime_global_layers={} runtime_local_window_layers={} runtime_convolutional_fusion_layers={} runtime_kv_import_experiences={} runtime_kv_export_experiences={} runtime_kv_hold_experiences={} runtime_kv_held_blocks={} reflection_issue_experiences={} critical_reflection_issue_experiences={} revision_action_experiences={} live_memory_feedback_experiences={} live_memory_feedback_updates={} live_memory_feedback_detail_experiences={} live_memory_feedback_applied={} live_memory_feedback_removed={} live_memory_feedback_missing={} live_memory_feedback_strength_delta={:.6} router_threshold={:.3} router_observations={} profile_thresholds=(general:{:.3},coding:{:.3},writing:{:.3},long:{:.3}) hierarchy=({:.2},{:.2},{:.2}) profile_hierarchy_local=(general:{:.2},coding:{:.2},writing:{:.2},long:{:.2}) tiers=({},{},{}) evolution_live_inference_runs={} evolution_live_router_threshold_mutations={} evolution_live_hierarchy_weight_mutations={} evolution_live_router_threshold_delta={:.6} evolution_live_hierarchy_weight_delta={:.6} evolution_live_memory_updates={} evolution_live_stored_memory_updates={} evolution_live_reflection_issues={} evolution_live_critical_reflection_issues={} evolution_live_revision_actions={} evolution_replay_runs={} evolution_replay_items={} evolution_router_threshold_mutations={} evolution_hierarchy_weight_mutations={} evolution_router_threshold_delta={:.6} evolution_hierarchy_weight_delta={:.6} evolution_memory_updates={} evolution_replay_live_memory_feedback_items={} evolution_replay_live_memory_feedback_updates={} evolution_replay_live_memory_feedback_reinforcements={} evolution_replay_live_memory_feedback_penalties={} evolution_replay_live_memory_feedback_detail_items={} evolution_replay_live_memory_feedback_applied={} evolution_replay_live_memory_feedback_removed={} evolution_replay_live_memory_feedback_missing={} evolution_replay_live_memory_feedback_strength_delta={:.6} evolution_replay_live_evolution_items={} evolution_replay_live_evolution_router_threshold_mutations={} evolution_replay_live_evolution_hierarchy_weight_mutations={} evolution_replay_live_evolution_router_threshold_delta={:.6} evolution_replay_live_evolution_hierarchy_weight_delta={:.6} evolution_replay_live_evolution_memory_updates={} evolution_replay_live_evolution_stored_memory_updates={} evolution_replay_live_evolution_reflection_issues={} evolution_replay_live_evolution_critical_reflection_issues={} evolution_replay_live_evolution_revision_actions={} evolution_recursive_replay_items={} evolution_recursive_runtime_calls={} evolution_drift_rollbacks={} evolution_rollback_router_threshold_delta={:.6} evolution_rollback_hierarchy_weight_delta={:.6} memory_vector_dimensions={} runtime_kv_vector_dimensions={}",
            self.memory_count,
            self.runtime_kv_memory_count,
            self.experience_count,
            self.runtime_model_experience_count,
            self.runtime_adapter_experience_count,
            self.runtime_adapter_selection_mismatch_count,
            self.runtime_forward_energy_experience_count,
            self.runtime_kv_influence_experience_count,
            self.runtime_kv_precision_experience_count,
            self.runtime_kv_precision_mismatch_count,
            self.runtime_device_execution_experience_count,
            self.runtime_layer_mode_experience_count,
            self.runtime_all_layer_mode_experience_count,
            self.runtime_global_layers,
            self.runtime_local_window_layers,
            self.runtime_convolutional_fusion_layers,
            self.runtime_kv_import_experience_count,
            self.runtime_kv_export_experience_count,
            self.runtime_kv_hold_experience_count,
            self.runtime_kv_held_blocks,
            self.reflection_issue_experience_count,
            self.critical_reflection_issue_experience_count,
            self.revision_action_experience_count,
            self.live_memory_feedback_experience_count,
            self.live_memory_feedback_update_count,
            self.live_memory_feedback_detail_experience_count,
            self.live_memory_feedback_applied_count,
            self.live_memory_feedback_removed_count,
            self.live_memory_feedback_missing_count,
            self.live_memory_feedback_strength_delta,
            self.router_threshold,
            self.router_observations,
            self.profile_thresholds.general,
            self.profile_thresholds.coding,
            self.profile_thresholds.writing,
            self.profile_thresholds.long_document,
            self.hierarchy.global,
            self.hierarchy.local,
            self.hierarchy.convolution,
            self.profile_hierarchy_weights.general.local,
            self.profile_hierarchy_weights.coding.local,
            self.profile_hierarchy_weights.writing.local,
            self.profile_hierarchy_weights.long_document.local,
            self.tier_counts.hot_gpu,
            self.tier_counts.warm_ram,
            self.tier_counts.cold_disk,
            self.evolution_ledger.live_inference_runs,
            self.evolution_ledger.live_router_threshold_mutations,
            self.evolution_ledger.live_hierarchy_weight_mutations,
            self.evolution_ledger.live_router_threshold_delta,
            self.evolution_ledger.live_hierarchy_weight_delta,
            self.evolution_ledger.live_memory_updates(),
            self.evolution_ledger.live_stored_memory_updates(),
            self.evolution_ledger.live_reflection_issues,
            self.evolution_ledger.live_critical_reflection_issues,
            self.evolution_ledger.live_revision_actions,
            self.evolution_ledger.replay_runs,
            self.evolution_ledger.replay_items,
            self.evolution_ledger.router_threshold_mutations,
            self.evolution_ledger.hierarchy_weight_mutations,
            self.evolution_ledger.router_threshold_delta,
            self.evolution_ledger.hierarchy_weight_delta,
            self.evolution_ledger.memory_updates(),
            self.evolution_ledger.replay_live_memory_feedback_items,
            self.evolution_ledger.replay_live_memory_feedback_updates(),
            self.evolution_ledger
                .replay_live_memory_feedback_reinforcements,
            self.evolution_ledger.replay_live_memory_feedback_penalties,
            self.evolution_ledger
                .replay_live_memory_feedback_detail_items,
            self.evolution_ledger.replay_live_memory_feedback_applied,
            self.evolution_ledger.replay_live_memory_feedback_removed,
            self.evolution_ledger.replay_live_memory_feedback_missing,
            self.evolution_ledger
                .replay_live_memory_feedback_strength_delta,
            self.evolution_ledger.replay_live_evolution_items,
            self.evolution_ledger
                .replay_live_evolution_router_threshold_mutations,
            self.evolution_ledger
                .replay_live_evolution_hierarchy_weight_mutations,
            self.evolution_ledger
                .replay_live_evolution_router_threshold_delta,
            self.evolution_ledger
                .replay_live_evolution_hierarchy_weight_delta,
            self.evolution_ledger.replay_live_evolution_memory_updates,
            self.evolution_ledger
                .replay_live_evolution_stored_memory_updates,
            self.evolution_ledger
                .replay_live_evolution_reflection_issues,
            self.evolution_ledger
                .replay_live_evolution_critical_reflection_issues,
            self.evolution_ledger.replay_live_evolution_revision_actions,
            self.evolution_ledger.recursive_replay_items,
            self.evolution_ledger.recursive_runtime_calls,
            self.evolution_ledger.drift_rollbacks,
            self.evolution_ledger.rollback_router_threshold_delta,
            self.evolution_ledger.rollback_hierarchy_weight_delta,
            format_memory_vector_dimensions(&self.memory_vector_dimensions),
            format_memory_vector_dimensions(&self.runtime_kv_vector_dimensions)
        )
    }

    pub fn evaluate(&self, gate: &StateInspectionGate) -> StateInspectionGateReport {
        let mut failures = Vec::new();

        require_min_usize(
            &mut failures,
            "memory_count",
            self.memory_count,
            gate.min_memories,
        );
        require_min_usize(
            &mut failures,
            "runtime_kv_memory_count",
            self.runtime_kv_memory_count,
            gate.min_runtime_kv_memories,
        );
        require_min_usize(
            &mut failures,
            "experience_count",
            self.experience_count,
            gate.min_experiences,
        );
        require_min_usize(
            &mut failures,
            "runtime_model_experience_count",
            self.runtime_model_experience_count,
            gate.min_runtime_model_experiences,
        );
        require_min_usize(
            &mut failures,
            "runtime_adapter_experience_count",
            self.runtime_adapter_experience_count,
            gate.min_runtime_adapter_experiences,
        );
        require_max_usize(
            &mut failures,
            "runtime_adapter_selection_mismatch_count",
            self.runtime_adapter_selection_mismatch_count,
            gate.max_runtime_adapter_selection_mismatches,
        );
        require_min_usize(
            &mut failures,
            "runtime_forward_energy_experience_count",
            self.runtime_forward_energy_experience_count,
            gate.min_runtime_forward_energy_experiences,
        );
        require_min_usize(
            &mut failures,
            "runtime_kv_influence_experience_count",
            self.runtime_kv_influence_experience_count,
            gate.min_runtime_kv_influence_experiences,
        );
        require_min_usize(
            &mut failures,
            "runtime_kv_precision_experience_count",
            self.runtime_kv_precision_experience_count,
            gate.min_runtime_kv_precision_experiences,
        );
        require_max_usize(
            &mut failures,
            "runtime_kv_precision_mismatch_count",
            self.runtime_kv_precision_mismatch_count,
            gate.max_runtime_kv_precision_mismatches,
        );
        require_min_usize(
            &mut failures,
            "runtime_device_execution_experience_count",
            self.runtime_device_execution_experience_count,
            gate.min_runtime_device_execution_experiences,
        );
        require_min_usize(
            &mut failures,
            "runtime_layer_mode_experience_count",
            self.runtime_layer_mode_experience_count,
            gate.min_runtime_layer_mode_experiences,
        );
        require_min_usize(
            &mut failures,
            "runtime_all_layer_mode_experience_count",
            self.runtime_all_layer_mode_experience_count,
            gate.min_runtime_all_layer_mode_experiences,
        );
        require_min_usize(
            &mut failures,
            "runtime_global_layers",
            self.runtime_global_layers,
            gate.min_runtime_global_layers,
        );
        require_min_usize(
            &mut failures,
            "runtime_local_window_layers",
            self.runtime_local_window_layers,
            gate.min_runtime_local_window_layers,
        );
        require_min_usize(
            &mut failures,
            "runtime_convolutional_fusion_layers",
            self.runtime_convolutional_fusion_layers,
            gate.min_runtime_convolutional_fusion_layers,
        );
        require_min_usize(
            &mut failures,
            "runtime_kv_import_experience_count",
            self.runtime_kv_import_experience_count,
            gate.min_runtime_kv_import_experiences,
        );
        require_min_usize(
            &mut failures,
            "runtime_kv_export_experience_count",
            self.runtime_kv_export_experience_count,
            gate.min_runtime_kv_export_experiences,
        );
        require_min_usize(
            &mut failures,
            "runtime_kv_hold_experience_count",
            self.runtime_kv_hold_experience_count,
            gate.min_runtime_kv_hold_experiences,
        );
        require_min_usize(
            &mut failures,
            "runtime_kv_held_blocks",
            self.runtime_kv_held_blocks,
            gate.min_runtime_kv_held_blocks,
        );
        require_min_usize(
            &mut failures,
            "reflection_issue_experience_count",
            self.reflection_issue_experience_count,
            gate.min_reflection_issue_experiences,
        );
        require_min_usize(
            &mut failures,
            "critical_reflection_issue_experience_count",
            self.critical_reflection_issue_experience_count,
            gate.min_critical_reflection_issue_experiences,
        );
        require_min_usize(
            &mut failures,
            "revision_action_experience_count",
            self.revision_action_experience_count,
            gate.min_revision_action_experiences,
        );
        require_min_usize(
            &mut failures,
            "live_memory_feedback_experience_count",
            self.live_memory_feedback_experience_count,
            gate.min_live_memory_feedback_experiences,
        );
        require_min_usize(
            &mut failures,
            "live_memory_feedback_update_count",
            self.live_memory_feedback_update_count,
            gate.min_live_memory_feedback_updates,
        );
        require_min_usize(
            &mut failures,
            "live_memory_feedback_detail_experience_count",
            self.live_memory_feedback_detail_experience_count,
            gate.min_live_memory_feedback_detail_experiences,
        );
        require_min_usize(
            &mut failures,
            "live_memory_feedback_applied_count",
            self.live_memory_feedback_applied_count,
            gate.min_live_memory_feedback_applied,
        );
        require_min_f32(
            &mut failures,
            "live_memory_feedback_strength_delta",
            self.live_memory_feedback_strength_delta,
            gate.min_live_memory_feedback_strength_delta,
        );
        require_min_u64(
            &mut failures,
            "router_observations",
            self.router_observations,
            gate.min_router_observations,
        );
        require_min_u64(
            &mut failures,
            "evolution_live_inference_runs",
            self.evolution_ledger.live_inference_runs,
            gate.min_evolution_live_inference_runs,
        );
        require_min_u64(
            &mut failures,
            "evolution_live_router_threshold_mutations",
            self.evolution_ledger.live_router_threshold_mutations,
            gate.min_evolution_live_router_threshold_mutations,
        );
        require_min_u64(
            &mut failures,
            "evolution_live_hierarchy_weight_mutations",
            self.evolution_ledger.live_hierarchy_weight_mutations,
            gate.min_evolution_live_hierarchy_weight_mutations,
        );
        require_min_f32(
            &mut failures,
            "evolution_live_router_threshold_delta",
            self.evolution_ledger.live_router_threshold_delta,
            gate.min_evolution_live_router_threshold_delta,
        );
        require_min_f32(
            &mut failures,
            "evolution_live_hierarchy_weight_delta",
            self.evolution_ledger.live_hierarchy_weight_delta,
            gate.min_evolution_live_hierarchy_weight_delta,
        );
        require_min_u64(
            &mut failures,
            "evolution_live_memory_updates",
            self.evolution_ledger.live_memory_updates(),
            gate.min_evolution_live_memory_updates,
        );
        require_min_u64(
            &mut failures,
            "evolution_live_stored_memory_updates",
            self.evolution_ledger.live_stored_memory_updates(),
            gate.min_evolution_live_stored_memory_updates,
        );
        require_min_u64(
            &mut failures,
            "evolution_live_reflection_issues",
            self.evolution_ledger.live_reflection_issues,
            gate.min_evolution_live_reflection_issues,
        );
        require_min_u64(
            &mut failures,
            "evolution_live_critical_reflection_issues",
            self.evolution_ledger.live_critical_reflection_issues,
            gate.min_evolution_live_critical_reflection_issues,
        );
        require_min_u64(
            &mut failures,
            "evolution_live_revision_actions",
            self.evolution_ledger.live_revision_actions,
            gate.min_evolution_live_revision_actions,
        );
        require_min_u64(
            &mut failures,
            "evolution_replay_runs",
            self.evolution_ledger.replay_runs,
            gate.min_evolution_replay_runs,
        );
        require_min_u64(
            &mut failures,
            "evolution_replay_items",
            self.evolution_ledger.replay_items,
            gate.min_evolution_replay_items,
        );
        require_min_u64(
            &mut failures,
            "evolution_router_threshold_mutations",
            self.evolution_ledger.router_threshold_mutations,
            gate.min_evolution_router_threshold_mutations,
        );
        require_min_u64(
            &mut failures,
            "evolution_hierarchy_weight_mutations",
            self.evolution_ledger.hierarchy_weight_mutations,
            gate.min_evolution_hierarchy_weight_mutations,
        );
        require_min_f32(
            &mut failures,
            "evolution_router_threshold_delta",
            self.evolution_ledger.router_threshold_delta,
            gate.min_evolution_router_threshold_delta,
        );
        require_min_f32(
            &mut failures,
            "evolution_hierarchy_weight_delta",
            self.evolution_ledger.hierarchy_weight_delta,
            gate.min_evolution_hierarchy_weight_delta,
        );
        require_min_u64(
            &mut failures,
            "evolution_memory_updates",
            self.evolution_ledger.memory_updates(),
            gate.min_evolution_memory_updates,
        );
        require_min_u64(
            &mut failures,
            "evolution_replay_live_memory_feedback_updates",
            self.evolution_ledger.replay_live_memory_feedback_updates(),
            gate.min_evolution_replay_live_memory_feedback_updates,
        );
        require_min_u64(
            &mut failures,
            "evolution_replay_live_memory_feedback_detail_items",
            self.evolution_ledger
                .replay_live_memory_feedback_detail_items,
            gate.min_evolution_replay_live_memory_feedback_detail_items,
        );
        require_min_u64(
            &mut failures,
            "evolution_replay_live_memory_feedback_applied",
            self.evolution_ledger.replay_live_memory_feedback_applied,
            gate.min_evolution_replay_live_memory_feedback_applied,
        );
        require_min_f32(
            &mut failures,
            "evolution_replay_live_memory_feedback_strength_delta",
            self.evolution_ledger
                .replay_live_memory_feedback_strength_delta,
            gate.min_evolution_replay_live_memory_feedback_strength_delta,
        );
        require_min_u64(
            &mut failures,
            "evolution_replay_live_evolution_items",
            self.evolution_ledger.replay_live_evolution_items,
            gate.min_evolution_replay_live_evolution_items,
        );
        require_min_u64(
            &mut failures,
            "evolution_replay_live_evolution_memory_updates",
            self.evolution_ledger.replay_live_evolution_memory_updates,
            gate.min_evolution_replay_live_evolution_memory_updates,
        );
        require_min_u64(
            &mut failures,
            "evolution_replay_live_evolution_stored_memory_updates",
            self.evolution_ledger
                .replay_live_evolution_stored_memory_updates,
            gate.min_evolution_replay_live_evolution_stored_memory_updates,
        );
        require_min_u64(
            &mut failures,
            "evolution_replay_live_evolution_reflection_issues",
            self.evolution_ledger
                .replay_live_evolution_reflection_issues,
            gate.min_evolution_replay_live_evolution_reflection_issues,
        );
        require_min_u64(
            &mut failures,
            "evolution_replay_live_evolution_critical_reflection_issues",
            self.evolution_ledger
                .replay_live_evolution_critical_reflection_issues,
            gate.min_evolution_replay_live_evolution_critical_reflection_issues,
        );
        require_min_u64(
            &mut failures,
            "evolution_replay_live_evolution_revision_actions",
            self.evolution_ledger.replay_live_evolution_revision_actions,
            gate.min_evolution_replay_live_evolution_revision_actions,
        );
        require_min_u64(
            &mut failures,
            "evolution_recursive_replay_items",
            self.evolution_ledger.recursive_replay_items,
            gate.min_evolution_recursive_replay_items,
        );
        require_min_u64(
            &mut failures,
            "evolution_recursive_runtime_calls",
            self.evolution_ledger.recursive_runtime_calls,
            gate.min_evolution_recursive_runtime_calls,
        );
        require_max_u64(
            &mut failures,
            "evolution_drift_rollbacks",
            self.evolution_ledger.drift_rollbacks,
            gate.max_evolution_drift_rollbacks,
        );
        require_max_f32(
            &mut failures,
            "evolution_rollback_router_threshold_delta",
            self.evolution_ledger.rollback_router_threshold_delta,
            gate.max_evolution_rollback_router_threshold_delta,
        );
        require_max_f32(
            &mut failures,
            "evolution_rollback_hierarchy_weight_delta",
            self.evolution_ledger.rollback_hierarchy_weight_delta,
            gate.max_evolution_rollback_hierarchy_weight_delta,
        );

        if gate.require_runtime_kv_dimensions && self.runtime_kv_vector_dimensions.is_empty() {
            failures.push("runtime_kv_vector_dimensions missing required buckets".to_owned());
        }

        StateInspectionGateReport {
            passed: failures.is_empty(),
            failures,
        }
    }
}

fn require_min_usize(
    failures: &mut Vec<String>,
    name: &str,
    actual: usize,
    required: Option<usize>,
) {
    if let Some(required) = required {
        if actual < required {
            failures.push(format!("{name} {actual} below required {required}"));
        }
    }
}

fn require_max_usize(
    failures: &mut Vec<String>,
    name: &str,
    actual: usize,
    maximum: Option<usize>,
) {
    if let Some(maximum) = maximum {
        if actual > maximum {
            failures.push(format!("{name} {actual} above maximum {maximum}"));
        }
    }
}

fn require_min_u64(failures: &mut Vec<String>, name: &str, actual: u64, required: Option<u64>) {
    if let Some(required) = required {
        if actual < required {
            failures.push(format!("{name} {actual} below required {required}"));
        }
    }
}

fn require_min_f32(failures: &mut Vec<String>, name: &str, actual: f32, required: Option<f32>) {
    if let Some(required) = required {
        if actual < required {
            failures.push(format!("{name} {actual:.6} below required {required:.6}"));
        }
    }
}

fn require_max_f32(failures: &mut Vec<String>, name: &str, actual: f32, maximum: Option<f32>) {
    if let Some(maximum) = maximum {
        if actual > maximum {
            failures.push(format!("{name} {actual:.6} above maximum {maximum:.6}"));
        }
    }
}

fn require_max_u64(failures: &mut Vec<String>, name: &str, actual: u64, maximum: Option<u64>) {
    if let Some(maximum) = maximum {
        if actual > maximum {
            failures.push(format!("{name} {actual} above maximum {maximum}"));
        }
    }
}

fn top_memory_summaries(
    engine: &NoironEngine,
    limit: usize,
    include: impl Fn(&str) -> bool,
) -> Vec<StateMemorySummary> {
    let mut top_memories = engine
        .cache
        .entries()
        .iter()
        .filter(|entry| include(&entry.key))
        .map(|entry| {
            let value_score =
                entry.strength + entry.hits as f32 * 0.04 - entry.failures as f32 * 0.10;
            (value_score, entry)
        })
        .collect::<Vec<_>>();
    top_memories.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.1.id.cmp(&right.1.id))
    });

    top_memories
        .into_iter()
        .take(limit)
        .map(|(_, entry)| StateMemorySummary {
            id: entry.id,
            key: compact(&entry.key, 120),
            vector_dimensions: entry.vector.len(),
            strength: entry.strength,
            hits: entry.hits,
            failures: entry.failures,
            last_score: entry.last_score,
        })
        .collect()
}

fn memory_vector_dimensions(engine: &NoironEngine) -> Vec<StateMemoryVectorDimensions> {
    let mut buckets = BTreeMap::<usize, usize>::new();
    for entry in engine.cache.entries() {
        *buckets.entry(entry.vector.len()).or_insert(0) += 1;
    }

    buckets
        .into_iter()
        .map(|(dimensions, count)| StateMemoryVectorDimensions { dimensions, count })
        .collect()
}

fn runtime_kv_vector_dimensions(engine: &NoironEngine) -> Vec<StateMemoryVectorDimensions> {
    let mut buckets = BTreeMap::<usize, usize>::new();
    for entry in engine
        .cache
        .entries()
        .iter()
        .filter(|entry| entry.key.starts_with("runtime_kv:"))
    {
        *buckets.entry(entry.vector.len()).or_insert(0) += 1;
    }

    buckets
        .into_iter()
        .map(|(dimensions, count)| StateMemoryVectorDimensions { dimensions, count })
        .collect()
}

fn format_memory_vector_dimensions(buckets: &[StateMemoryVectorDimensions]) -> String {
    if buckets.is_empty() {
        return "none".to_owned();
    }

    buckets
        .iter()
        .map(|bucket| format!("{}:{}", bucket.dimensions, bucket.count))
        .collect::<Vec<_>>()
        .join("|")
}

fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn inspection_hardware_plan(engine: &NoironEngine) -> HardwarePlan {
    engine.hardware_allocator.plan(
        engine.hardware_snapshot,
        TaskProfile::General,
        1,
        engine.hierarchy.current(),
    )
}

fn runtime_kv_precision_mismatch_count(
    engine: &NoironEngine,
    hardware_plan: &HardwarePlan,
) -> usize {
    engine
        .experience
        .records()
        .iter()
        .filter(|record| {
            let diagnostics = &record.runtime_diagnostics;
            diagnostics.has_device_execution_signal()
                && diagnostics.has_valid_kv_precision_signal()
                && (diagnostics.hot_kv_precision_bits
                    != Some(hardware_plan.execution.hot_kv_precision_bits)
                    || diagnostics.cold_kv_precision_bits
                        != Some(hardware_plan.execution.cold_kv_precision_bits))
        })
        .count()
}

fn runtime_adapter_selection_mismatch_count(
    engine: &NoironEngine,
    hardware_plan: &HardwarePlan,
) -> usize {
    let matches = runtime_adapter_experience_matches(engine);
    let observations =
        RuntimeAdapterObservation::from_experiences_for_hardware(&matches, "", hardware_plan);
    let Some(best_adapter) = observations
        .iter()
        .filter(|observation| observation.score >= 0.50)
        .map(|observation| observation.adapter.as_str())
        .next()
    else {
        return 0;
    };

    let Some(selected_adapter) =
        latest_runtime_selected_adapter_for_hardware(engine, hardware_plan)
    else {
        return 1;
    };

    usize::from(selected_adapter != best_adapter)
}

fn latest_runtime_selected_adapter_for_hardware<'a>(
    engine: &'a NoironEngine,
    hardware_plan: &HardwarePlan,
) -> Option<&'a str> {
    engine
        .experience
        .records()
        .iter()
        .rev()
        .filter(|record| record_matches_hardware_plan(record, hardware_plan))
        .filter_map(|record| record.runtime_diagnostics.selected_adapter.as_deref())
        .find(|adapter| {
            hardware_plan
                .execution
                .adapter_hints
                .iter()
                .any(|hint| hint.as_str() == *adapter)
        })
}

fn runtime_adapter_experience_matches(engine: &NoironEngine) -> Vec<ExperienceMatch> {
    engine
        .experience
        .records()
        .iter()
        .filter_map(|record| {
            let selected_adapter = record.runtime_diagnostics.selected_adapter.clone()?;
            Some(ExperienceMatch {
                id: record.id,
                prompt: record.prompt.clone(),
                lesson: record.lesson.clone(),
                quality: record.quality,
                score: runtime_adapter_record_score(record),
                gist_hints: Vec::new(),
                reflection_issue_codes: Vec::new(),
                revision_actions: record.revision_actions.clone(),
                process_reward: record.process_reward.total,
                reward_action: record.process_reward.action,
                runtime_model_id: record.runtime_diagnostics.model_id.clone(),
                runtime_selected_adapter: Some(selected_adapter),
                runtime_device_profile: record.runtime_diagnostics.device_profile.clone(),
                runtime_primary_lane: record.runtime_diagnostics.primary_lane.clone(),
                runtime_fallback_lane: record.runtime_diagnostics.fallback_lane.clone(),
                runtime_memory_mode: record.runtime_diagnostics.memory_mode.clone(),
                runtime_forward_energy: record.runtime_diagnostics.forward_energy,
                runtime_kv_influence: record.runtime_diagnostics.kv_influence,
                recursive_runtime_calls: recursive_runtime_calls_from_notes(
                    &record.process_reward.notes,
                ),
            })
        })
        .collect()
}

fn runtime_adapter_record_score(record: &crate::experience::ExperienceRecord) -> f32 {
    let reward_bonus = record.process_reward.total.clamp(0.0, 1.0) * 0.20;
    let issue_penalty = (record.reflection_issues.len() as f32 * 0.03).min(0.18);
    let contradiction_penalty = (record.contradictions.len() as f32 * 0.05).min(0.25);
    (record.quality * 0.80 + reward_bonus - issue_penalty - contradiction_penalty).clamp(0.0, 1.0)
}

fn record_matches_hardware_plan(
    record: &crate::experience::ExperienceRecord,
    hardware_plan: &HardwarePlan,
) -> bool {
    let diagnostics = &record.runtime_diagnostics;
    runtime_diagnostic_matches(
        diagnostics.device_profile.as_deref(),
        hardware_plan.device.as_str(),
    ) && runtime_diagnostic_matches(
        diagnostics.primary_lane.as_deref(),
        hardware_plan.execution.primary_lane.as_str(),
    ) && runtime_diagnostic_matches(
        diagnostics.fallback_lane.as_deref(),
        hardware_plan.execution.fallback_lane.as_str(),
    ) && runtime_diagnostic_matches(
        diagnostics.memory_mode.as_deref(),
        hardware_plan.execution.memory_mode.as_str(),
    )
}

fn runtime_diagnostic_matches(actual: Option<&str>, expected: &str) -> bool {
    actual.map(|actual| actual == expected).unwrap_or(true)
}

fn has_text(value: Option<&str>) -> bool {
    value.map(|value| !value.trim().is_empty()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::NoironEngine;
    use crate::experience::ExperienceInput;
    use crate::hierarchy::{HierarchyWeights, TaskProfile};
    use crate::process_reward::{ProcessRewardComponents, ProcessRewardReport, RewardAction};
    use crate::reflection::{ReflectionIssue, ReflectionSeverity};
    use crate::router::RouteBudget;

    #[test]
    fn inspection_report_summarizes_memory_experience_and_adaptive_state() {
        let mut engine = NoironEngine::new();
        let memory_id =
            engine
                .cache
                .store_or_fuse("inspectable reinforced memory", vec![1.0, 0.0, 0.0], 0.9);
        let fallback_memory_id =
            engine
                .cache
                .store_or_fuse("fallback embedding memory", vec![0.0, 1.0, 0.0, 0.0], 0.7);
        let runtime_kv_memory_id = engine.cache.store_or_fuse(
            "runtime_kv:l2h1:0-1 :: inspect runtime KV",
            vec![0.1, 0.2, 0.3, 0.4, 0.5],
            0.95,
        );
        engine.cache.reinforce(memory_id, 0.8);
        engine.cache.reinforce(runtime_kv_memory_id, 0.9);
        engine.evolution_ledger = EvolutionLedger {
            live_inference_runs: 3,
            live_router_threshold_mutations: 2,
            live_hierarchy_weight_mutations: 1,
            live_router_threshold_delta: 0.05,
            live_hierarchy_weight_delta: 0.04,
            live_memory_reinforcements: 4,
            live_memory_penalties: 1,
            live_stored_memories: 2,
            live_stored_gist_memories: 3,
            live_stored_runtime_kv_memories: 1,
            live_reflection_issues: 5,
            live_critical_reflection_issues: 1,
            live_revision_actions: 6,
            replay_runs: 2,
            replay_items: 5,
            router_threshold_mutations: 3,
            hierarchy_weight_mutations: 4,
            router_threshold_delta: 0.17,
            hierarchy_weight_delta: 0.08,
            memory_reinforcements: 6,
            memory_penalties: 1,
            replay_live_memory_feedback_items: 2,
            replay_live_memory_feedback_reinforcements: 2,
            replay_live_memory_feedback_penalties: 1,
            replay_live_memory_feedback_detail_items: 2,
            replay_live_memory_feedback_applied: 3,
            replay_live_memory_feedback_removed: 1,
            replay_live_memory_feedback_missing: 1,
            replay_live_memory_feedback_strength_delta: 0.52,
            replay_live_evolution_items: 2,
            replay_live_evolution_router_threshold_mutations: 1,
            replay_live_evolution_hierarchy_weight_mutations: 1,
            replay_live_evolution_router_threshold_delta: 0.04,
            replay_live_evolution_hierarchy_weight_delta: 0.03,
            replay_live_evolution_memory_updates: 3,
            replay_live_evolution_stored_memory_updates: 2,
            replay_live_evolution_reflection_issues: 2,
            replay_live_evolution_critical_reflection_issues: 1,
            replay_live_evolution_revision_actions: 2,
            recursive_replay_items: 8,
            recursive_runtime_calls: 9,
            drift_rollbacks: 2,
            rollback_router_threshold_delta: 0.03,
            rollback_hierarchy_weight_delta: 0.04,
        };
        engine.set_memory_retention_policy(MemoryRetentionPolicy {
            stale_after: 12,
            decay_rate: 0.12,
            remove_below_strength: 0.08,
            remove_after_failures: 7,
        });
        engine.set_memory_compaction_policy(MemoryCompactionPolicy {
            similarity_threshold: 0.91,
            max_candidates: 64,
            max_merges: 4,
        });
        engine.experience.record(ExperienceInput {
            prompt: "inspect state".to_owned(),
            profile: TaskProfile::Coding,
            lesson: "state inspection should expose learned control decisions".to_owned(),
            quality: 0.91,
            contradictions: Vec::new(),
            reflection_issues: vec![ReflectionIssue::new(
                "needs_grounding",
                ReflectionSeverity::Warning,
                "inspect warning",
            )],
            revision_actions: vec!["increase_prompt_grounding".to_owned()],
            stored_memory_id: Some(memory_id),
            router_threshold_after: 0.62,
            stream_windows: 2,
            route_budget: RouteBudget {
                threshold: 0.62,
                attention_tokens: 2,
                fast_tokens: 2,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            used_memory_ids: vec![memory_id],
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            runtime_diagnostics: crate::reflection::RuntimeDiagnostics {
                model_id: Some("inspect-runtime".to_owned()),
                selected_adapter: Some("portable-rust".to_owned()),
                device_profile: Some("cpu".to_owned()),
                primary_lane: Some("cpu-vector".to_owned()),
                fallback_lane: Some("cpu-portable".to_owned()),
                memory_mode: Some("tiered-disk".to_owned()),
                layer_count: 12,
                global_layers: 3,
                local_window_layers: 6,
                convolutional_fusion_layers: 3,
                hidden_size: 128,
                local_window_tokens: 4096,
                forward_energy: Some(0.34),
                kv_influence: Some(0.56),
                imported_kv_blocks: 2,
                exported_kv_blocks: 3,
                hot_kv_precision_bits: Some(8),
                cold_kv_precision_bits: Some(4),
            },
            process_reward: ProcessRewardReport {
                total: 0.88,
                action: RewardAction::Reinforce,
                components: ProcessRewardComponents::default(),
                notes: vec![
                    "recursive:chunks=5:merge_rounds=2:waves=3:parallel=2:runtime_calls=9"
                        .to_owned(),
                    "memory_feedback:reinforced=2:penalized=1:reinforcement_amount=1.400000:penalty_amount=0.300000:applied=2:removed=0:missing=1:strength_delta=0.510000"
                        .to_owned(),
                ],
            },
            live_evolution: Default::default(),
        });
        engine.experience.record(ExperienceInput {
            prompt: "inspect critical reflection state".to_owned(),
            profile: TaskProfile::General,
            lesson: "critical reflection diagnostics should remain inspectable".to_owned(),
            quality: 0.42,
            contradictions: vec!["unsupported claim".to_owned()],
            reflection_issues: vec![ReflectionIssue::new(
                "unsupported_claim",
                ReflectionSeverity::Critical,
                "critical inspect issue",
            )],
            revision_actions: Vec::new(),
            stored_memory_id: None,
            router_threshold_after: 0.48,
            stream_windows: 1,
            route_budget: RouteBudget {
                threshold: 0.48,
                attention_tokens: 1,
                fast_tokens: 2,
                attention_fraction: 0.33,
            },
            hierarchy: HierarchyWeights::new(0.34, 0.33, 0.33),
            used_memory_ids: Vec::new(),
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            runtime_diagnostics: crate::reflection::RuntimeDiagnostics::default(),
            process_reward: ProcessRewardReport {
                total: 0.12,
                action: RewardAction::Penalize,
                components: ProcessRewardComponents::default(),
                notes: Vec::new(),
            },
            live_evolution: Default::default(),
        });

        let report = StateInspectionReport::from_engine(&engine, 3);

        assert_eq!(report.memory_count, 3);
        assert_eq!(report.runtime_kv_memory_count, 1);
        assert_eq!(report.experience_count, 2);
        assert_eq!(report.runtime_model_experience_count, 1);
        assert_eq!(report.runtime_adapter_experience_count, 1);
        assert_eq!(report.runtime_adapter_selection_mismatch_count, 0);
        assert_eq!(report.runtime_forward_energy_experience_count, 1);
        assert_eq!(report.runtime_kv_influence_experience_count, 1);
        assert_eq!(report.runtime_kv_precision_experience_count, 1);
        assert_eq!(report.runtime_kv_precision_mismatch_count, 0);
        assert_eq!(report.runtime_device_execution_experience_count, 1);
        assert_eq!(report.runtime_kv_import_experience_count, 1);
        assert_eq!(report.runtime_kv_export_experience_count, 1);
        assert_eq!(report.runtime_kv_hold_experience_count, 1);
        assert_eq!(report.runtime_kv_held_blocks, 3);
        assert_eq!(report.reflection_issue_experience_count, 2);
        assert_eq!(report.critical_reflection_issue_experience_count, 1);
        assert_eq!(report.revision_action_experience_count, 1);
        assert_eq!(report.live_memory_feedback_experience_count, 1);
        assert_eq!(report.live_memory_feedback_update_count, 3);
        assert_eq!(report.live_memory_feedback_detail_experience_count, 1);
        assert_eq!(report.live_memory_feedback_applied_count, 2);
        assert_eq!(report.live_memory_feedback_removed_count, 0);
        assert_eq!(report.live_memory_feedback_missing_count, 1);
        assert!((report.live_memory_feedback_strength_delta - 0.51).abs() < 0.0001);
        assert!(
            report
                .top_memories
                .iter()
                .any(|memory| memory.id == memory_id
                    && memory.key.contains("inspectable")
                    && memory.vector_dimensions == 3)
        );
        assert_eq!(report.top_runtime_kv_memories.len(), 1);
        assert_eq!(report.top_runtime_kv_memories[0].id, runtime_kv_memory_id);
        assert!(
            report.top_runtime_kv_memories[0]
                .key
                .starts_with("runtime_kv:")
        );
        assert_eq!(report.top_runtime_kv_memories[0].vector_dimensions, 5);
        assert!(
            report
                .top_memories
                .iter()
                .any(|memory| memory.id == fallback_memory_id && memory.vector_dimensions == 4)
        );
        assert_eq!(
            report.memory_vector_dimensions,
            vec![
                StateMemoryVectorDimensions {
                    dimensions: 3,
                    count: 1
                },
                StateMemoryVectorDimensions {
                    dimensions: 4,
                    count: 1
                },
                StateMemoryVectorDimensions {
                    dimensions: 5,
                    count: 1
                }
            ]
        );
        assert_eq!(
            report.runtime_kv_vector_dimensions,
            vec![StateMemoryVectorDimensions {
                dimensions: 5,
                count: 1
            }]
        );
        assert_eq!(report.memory_retention_policy.stale_after, 12);
        assert_eq!(report.memory_compaction_policy.max_merges, 4);
        assert_eq!(report.evolution_ledger.replay_runs, 2);
        assert_eq!(report.evolution_ledger.live_inference_runs, 3);
        assert_eq!(report.evolution_ledger.live_memory_updates(), 5);
        assert_eq!(report.evolution_ledger.live_stored_memory_updates(), 6);
        assert_eq!(report.evolution_ledger.live_revision_actions, 6);
        assert_eq!(report.evolution_ledger.memory_updates(), 7);
        assert_eq!(report.evolution_ledger.recursive_replay_items, 8);
        assert_eq!(report.evolution_ledger.recursive_runtime_calls, 9);
        assert_eq!(report.evolution_ledger.drift_rollbacks, 2);
        assert_eq!(
            report.top_experiences[0].reward_action,
            RewardAction::Reinforce
        );
        assert_eq!(
            report.top_experiences[0].runtime_model_id.as_deref(),
            Some("inspect-runtime")
        );
        assert_eq!(
            report.top_experiences[0]
                .runtime_selected_adapter
                .as_deref(),
            Some("portable-rust")
        );
        assert_eq!(
            report.top_experiences[0].runtime_device_profile.as_deref(),
            Some("cpu")
        );
        assert_eq!(
            report.top_experiences[0].runtime_primary_lane.as_deref(),
            Some("cpu-vector")
        );
        assert_eq!(
            report.top_experiences[0].runtime_fallback_lane.as_deref(),
            Some("cpu-portable")
        );
        assert_eq!(
            report.top_experiences[0].runtime_memory_mode.as_deref(),
            Some("tiered-disk")
        );
        assert_eq!(report.top_experiences[0].runtime_layer_count, 12);
        assert_eq!(report.top_experiences[0].runtime_global_layers, 3);
        assert_eq!(report.top_experiences[0].runtime_local_window_layers, 6);
        assert_eq!(
            report.top_experiences[0].runtime_convolutional_fusion_layers,
            3
        );
        assert_eq!(report.top_experiences[0].runtime_hidden_size, 128);
        assert_eq!(report.top_experiences[0].runtime_local_window_tokens, 4096);
        assert_eq!(report.top_experiences[0].runtime_forward_energy, Some(0.34));
        assert_eq!(report.top_experiences[0].runtime_kv_influence, Some(0.56));
        assert_eq!(report.top_experiences[0].runtime_imported_kv_blocks, 2);
        assert_eq!(report.top_experiences[0].runtime_exported_kv_blocks, 3);
        assert_eq!(report.top_experiences[0].recursive_runtime_calls, Some(9));
        assert_eq!(report.top_experiences[0].live_memory_feedback_updates, 3);
        assert_eq!(report.top_experiences[0].live_memory_feedback_reinforced, 2);
        assert_eq!(report.top_experiences[0].live_memory_feedback_penalized, 1);
        assert_eq!(report.top_experiences[0].live_memory_feedback_applied, 2);
        assert_eq!(report.top_experiences[0].live_memory_feedback_removed, 0);
        assert_eq!(report.top_experiences[0].live_memory_feedback_missing, 1);
        assert!(
            (report.top_experiences[0].live_memory_feedback_strength_delta - 0.51).abs() < 0.0001
        );
        assert!(report.top_experiences[0].live_memory_feedback_detail);
        assert_eq!(report.top_experiences[0].reflection_issues, 1);
        assert_eq!(report.top_experiences[0].revision_actions, 1);
        assert!(report.summary_line().contains("memories=3"));
        assert!(report.summary_line().contains("runtime_kv_memories=1"));
        assert!(
            report
                .summary_line()
                .contains("runtime_model_experiences=1")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_adapter_experiences=1")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_adapter_selection_mismatches=0")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_forward_energy_experiences=1")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_kv_influence_experiences=1")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_kv_precision_experiences=1")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_kv_precision_mismatches=0")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_device_execution_experiences=1")
        );
        assert_eq!(report.runtime_layer_mode_experience_count, 1);
        assert_eq!(report.runtime_all_layer_mode_experience_count, 1);
        assert_eq!(report.runtime_global_layers, 3);
        assert_eq!(report.runtime_local_window_layers, 6);
        assert_eq!(report.runtime_convolutional_fusion_layers, 3);
        assert!(
            report
                .summary_line()
                .contains("runtime_layer_mode_experiences=1")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_all_layer_mode_experiences=1")
        );
        assert!(report.summary_line().contains("runtime_global_layers=3"));
        assert!(
            report
                .summary_line()
                .contains("runtime_local_window_layers=6")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_convolutional_fusion_layers=3")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_kv_import_experiences=1")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_kv_export_experiences=1")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_kv_hold_experiences=1")
        );
        assert!(report.summary_line().contains("runtime_kv_held_blocks=3"));
        assert!(
            report
                .summary_line()
                .contains("reflection_issue_experiences=2")
        );
        assert!(
            report
                .summary_line()
                .contains("critical_reflection_issue_experiences=1")
        );
        assert!(
            report
                .summary_line()
                .contains("revision_action_experiences=1")
        );
        assert!(
            report
                .summary_line()
                .contains("live_memory_feedback_experiences=1")
        );
        assert!(
            report
                .summary_line()
                .contains("live_memory_feedback_updates=3")
        );
        assert!(
            report
                .summary_line()
                .contains("live_memory_feedback_detail_experiences=1")
        );
        assert!(
            report
                .summary_line()
                .contains("live_memory_feedback_applied=2")
        );
        assert!(
            report
                .summary_line()
                .contains("live_memory_feedback_missing=1")
        );
        assert!(
            report
                .summary_line()
                .contains("live_memory_feedback_strength_delta=0.510000")
        );
        assert!(
            report
                .summary_line()
                .contains("memory_vector_dimensions=3:1|4:1|5:1")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_kv_vector_dimensions=5:1")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_router_threshold_mutations=3")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_hierarchy_weight_mutations=4")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_router_threshold_delta=0.170000")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_hierarchy_weight_delta=0.080000")
        );
        assert!(report.summary_line().contains("evolution_memory_updates=7"));
        assert!(
            report
                .summary_line()
                .contains("evolution_live_inference_runs=3")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_live_memory_updates=5")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_live_stored_memory_updates=6")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_memory_feedback_updates=3")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_memory_feedback_detail_items=2")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_memory_feedback_applied=3")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_memory_feedback_strength_delta=0.520000")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_evolution_items=2")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_evolution_memory_updates=3")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_evolution_stored_memory_updates=2")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_evolution_reflection_issues=2")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_recursive_replay_items=8")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_drift_rollbacks=2")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_rollback_router_threshold_delta=0.030000")
        );

        let passing_gate = StateInspectionGate {
            min_memories: Some(3),
            min_runtime_kv_memories: Some(1),
            min_experiences: Some(1),
            min_runtime_model_experiences: Some(1),
            min_runtime_adapter_experiences: Some(1),
            max_runtime_adapter_selection_mismatches: Some(0),
            min_runtime_forward_energy_experiences: Some(1),
            min_runtime_kv_influence_experiences: Some(1),
            min_runtime_kv_precision_experiences: Some(1),
            max_runtime_kv_precision_mismatches: Some(0),
            min_runtime_device_execution_experiences: Some(1),
            min_runtime_layer_mode_experiences: Some(1),
            min_runtime_all_layer_mode_experiences: Some(1),
            min_runtime_global_layers: Some(3),
            min_runtime_local_window_layers: Some(6),
            min_runtime_convolutional_fusion_layers: Some(3),
            min_runtime_kv_import_experiences: Some(1),
            min_runtime_kv_export_experiences: Some(1),
            min_runtime_kv_hold_experiences: Some(1),
            min_runtime_kv_held_blocks: Some(3),
            min_reflection_issue_experiences: Some(2),
            min_critical_reflection_issue_experiences: Some(1),
            min_revision_action_experiences: Some(1),
            min_live_memory_feedback_experiences: Some(1),
            min_live_memory_feedback_updates: Some(3),
            min_live_memory_feedback_detail_experiences: Some(1),
            min_live_memory_feedback_applied: Some(2),
            min_live_memory_feedback_strength_delta: Some(0.51),
            min_router_observations: Some(0),
            min_evolution_live_inference_runs: Some(3),
            min_evolution_live_router_threshold_mutations: Some(2),
            min_evolution_live_hierarchy_weight_mutations: Some(1),
            min_evolution_live_router_threshold_delta: Some(0.05),
            min_evolution_live_hierarchy_weight_delta: Some(0.04),
            min_evolution_live_memory_updates: Some(5),
            min_evolution_live_stored_memory_updates: Some(6),
            min_evolution_live_reflection_issues: Some(5),
            min_evolution_live_critical_reflection_issues: Some(1),
            min_evolution_live_revision_actions: Some(6),
            min_evolution_replay_runs: Some(2),
            min_evolution_replay_items: Some(5),
            min_evolution_router_threshold_mutations: Some(3),
            min_evolution_hierarchy_weight_mutations: Some(4),
            min_evolution_router_threshold_delta: Some(0.17),
            min_evolution_hierarchy_weight_delta: Some(0.08),
            min_evolution_memory_updates: Some(7),
            min_evolution_replay_live_memory_feedback_updates: Some(3),
            min_evolution_replay_live_memory_feedback_detail_items: Some(2),
            min_evolution_replay_live_memory_feedback_applied: Some(3),
            min_evolution_replay_live_memory_feedback_strength_delta: Some(0.52),
            min_evolution_replay_live_evolution_items: Some(2),
            min_evolution_replay_live_evolution_memory_updates: Some(3),
            min_evolution_replay_live_evolution_stored_memory_updates: Some(2),
            min_evolution_replay_live_evolution_reflection_issues: Some(2),
            min_evolution_replay_live_evolution_critical_reflection_issues: Some(1),
            min_evolution_replay_live_evolution_revision_actions: Some(2),
            min_evolution_recursive_replay_items: Some(8),
            min_evolution_recursive_runtime_calls: Some(9),
            max_evolution_drift_rollbacks: Some(2),
            max_evolution_rollback_router_threshold_delta: Some(0.03),
            max_evolution_rollback_hierarchy_weight_delta: Some(0.04),
            require_runtime_kv_dimensions: true,
        };
        let passing_report = report.evaluate(&passing_gate);
        assert!(passing_report.passed());
        assert_eq!(
            passing_report.summary_line(),
            "state_inspection_gate: passed=true failures=0"
        );

        let failing_gate = StateInspectionGate {
            min_memories: Some(4),
            min_runtime_kv_memories: Some(2),
            min_experiences: Some(2),
            min_runtime_model_experiences: Some(2),
            min_runtime_adapter_experiences: Some(2),
            max_runtime_adapter_selection_mismatches: Some(0),
            min_runtime_forward_energy_experiences: Some(2),
            min_runtime_kv_influence_experiences: Some(2),
            min_runtime_kv_precision_experiences: Some(2),
            max_runtime_kv_precision_mismatches: Some(0),
            min_runtime_device_execution_experiences: Some(2),
            min_runtime_layer_mode_experiences: Some(2),
            min_runtime_all_layer_mode_experiences: Some(2),
            min_runtime_global_layers: Some(4),
            min_runtime_local_window_layers: Some(7),
            min_runtime_convolutional_fusion_layers: Some(4),
            min_runtime_kv_import_experiences: Some(2),
            min_runtime_kv_export_experiences: Some(2),
            min_runtime_kv_hold_experiences: Some(2),
            min_runtime_kv_held_blocks: Some(4),
            min_reflection_issue_experiences: Some(3),
            min_critical_reflection_issue_experiences: Some(2),
            min_revision_action_experiences: Some(2),
            min_live_memory_feedback_experiences: Some(2),
            min_live_memory_feedback_updates: Some(4),
            min_live_memory_feedback_detail_experiences: Some(2),
            min_live_memory_feedback_applied: Some(3),
            min_live_memory_feedback_strength_delta: Some(0.52),
            min_router_observations: Some(1),
            min_evolution_live_inference_runs: Some(4),
            min_evolution_live_router_threshold_mutations: Some(3),
            min_evolution_live_hierarchy_weight_mutations: Some(2),
            min_evolution_live_router_threshold_delta: Some(0.06),
            min_evolution_live_hierarchy_weight_delta: Some(0.05),
            min_evolution_live_memory_updates: Some(6),
            min_evolution_live_stored_memory_updates: Some(7),
            min_evolution_live_reflection_issues: Some(6),
            min_evolution_live_critical_reflection_issues: Some(2),
            min_evolution_live_revision_actions: Some(7),
            min_evolution_replay_runs: Some(3),
            min_evolution_replay_items: Some(6),
            min_evolution_router_threshold_mutations: Some(4),
            min_evolution_hierarchy_weight_mutations: Some(5),
            min_evolution_router_threshold_delta: Some(0.18),
            min_evolution_hierarchy_weight_delta: Some(0.09),
            min_evolution_memory_updates: Some(8),
            min_evolution_replay_live_memory_feedback_updates: Some(4),
            min_evolution_replay_live_memory_feedback_detail_items: Some(3),
            min_evolution_replay_live_memory_feedback_applied: Some(4),
            min_evolution_replay_live_memory_feedback_strength_delta: Some(0.53),
            min_evolution_replay_live_evolution_items: Some(3),
            min_evolution_replay_live_evolution_memory_updates: Some(4),
            min_evolution_replay_live_evolution_stored_memory_updates: Some(3),
            min_evolution_replay_live_evolution_reflection_issues: Some(3),
            min_evolution_replay_live_evolution_critical_reflection_issues: Some(2),
            min_evolution_replay_live_evolution_revision_actions: Some(3),
            min_evolution_recursive_replay_items: Some(9),
            min_evolution_recursive_runtime_calls: Some(10),
            max_evolution_drift_rollbacks: Some(1),
            max_evolution_rollback_router_threshold_delta: Some(0.02),
            max_evolution_rollback_hierarchy_weight_delta: Some(0.03),
            require_runtime_kv_dimensions: true,
        };
        let failing_report = report.evaluate(&failing_gate);
        assert!(!failing_report.passed());
        assert!(
            failing_report
                .failures
                .contains(&"memory_count 3 below required 4".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"runtime_kv_memory_count 1 below required 2".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"runtime_model_experience_count 1 below required 2".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"runtime_forward_energy_experience_count 1 below required 2".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"runtime_kv_import_experience_count 1 below required 2".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"runtime_kv_hold_experience_count 1 below required 2".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"runtime_kv_held_blocks 3 below required 4".to_owned())
        );
        assert!(
            failing_report.failures.contains(
                &"runtime_device_execution_experience_count 1 below required 2".to_owned()
            )
        );
        assert!(
            failing_report
                .failures
                .contains(&"runtime_layer_mode_experience_count 1 below required 2".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"runtime_all_layer_mode_experience_count 1 below required 2".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"runtime_global_layers 3 below required 4".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"runtime_local_window_layers 6 below required 7".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"runtime_convolutional_fusion_layers 3 below required 4".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"reflection_issue_experience_count 2 below required 3".to_owned())
        );
        assert!(
            failing_report.failures.contains(
                &"critical_reflection_issue_experience_count 1 below required 2".to_owned()
            )
        );
        assert!(
            failing_report
                .failures
                .contains(&"revision_action_experience_count 1 below required 2".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"live_memory_feedback_experience_count 1 below required 2".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"live_memory_feedback_update_count 3 below required 4".to_owned())
        );
        assert!(failing_report.failures.contains(
            &"live_memory_feedback_detail_experience_count 1 below required 2".to_owned()
        ));
        assert!(
            failing_report
                .failures
                .contains(&"live_memory_feedback_applied_count 2 below required 3".to_owned())
        );
        assert!(failing_report.failures.contains(
            &"live_memory_feedback_strength_delta 0.510000 below required 0.520000".to_owned()
        ));
        assert!(
            failing_report
                .failures
                .contains(&"evolution_live_inference_runs 3 below required 4".to_owned())
        );
        assert!(
            failing_report.failures.contains(
                &"evolution_live_router_threshold_mutations 2 below required 3".to_owned()
            )
        );
        assert!(
            failing_report.failures.contains(
                &"evolution_live_hierarchy_weight_mutations 1 below required 2".to_owned()
            )
        );
        assert!(failing_report.failures.contains(
            &"evolution_live_router_threshold_delta 0.050000 below required 0.060000".to_owned()
        ));
        assert!(failing_report.failures.contains(
            &"evolution_live_hierarchy_weight_delta 0.040000 below required 0.050000".to_owned()
        ));
        assert!(
            failing_report
                .failures
                .contains(&"evolution_live_memory_updates 5 below required 6".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"evolution_live_stored_memory_updates 6 below required 7".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"evolution_live_reflection_issues 5 below required 6".to_owned())
        );
        assert!(
            failing_report.failures.contains(
                &"evolution_live_critical_reflection_issues 1 below required 2".to_owned()
            )
        );
        assert!(
            failing_report
                .failures
                .contains(&"evolution_live_revision_actions 6 below required 7".to_owned())
        );
        assert!(failing_report.failures.contains(
            &"evolution_router_threshold_delta 0.170000 below required 0.180000".to_owned()
        ));
        assert!(failing_report.failures.contains(
            &"evolution_hierarchy_weight_delta 0.080000 below required 0.090000".to_owned()
        ));
        assert!(
            failing_report
                .failures
                .contains(&"evolution_memory_updates 7 below required 8".to_owned())
        );
        assert!(failing_report.failures.contains(
            &"evolution_replay_live_memory_feedback_updates 3 below required 4".to_owned()
        ));
        assert!(failing_report.failures.contains(
            &"evolution_replay_live_memory_feedback_detail_items 2 below required 3".to_owned()
        ));
        assert!(failing_report.failures.contains(
            &"evolution_replay_live_memory_feedback_applied 3 below required 4".to_owned()
        ));
        assert!(failing_report.failures.contains(
            &"evolution_replay_live_memory_feedback_strength_delta 0.520000 below required 0.530000"
                .to_owned()
        ));
        assert!(
            failing_report
                .failures
                .contains(&"evolution_replay_live_evolution_items 2 below required 3".to_owned())
        );
        assert!(failing_report.failures.contains(
            &"evolution_replay_live_evolution_memory_updates 3 below required 4".to_owned()
        ));
        assert!(failing_report.failures.contains(
            &"evolution_replay_live_evolution_stored_memory_updates 2 below required 3".to_owned()
        ));
        assert!(failing_report.failures.contains(
            &"evolution_replay_live_evolution_reflection_issues 2 below required 3".to_owned()
        ));
        assert!(
            failing_report.failures.contains(
                &"evolution_replay_live_evolution_critical_reflection_issues 1 below required 2"
                    .to_owned()
            )
        );
        assert!(failing_report.failures.contains(
            &"evolution_replay_live_evolution_revision_actions 2 below required 3".to_owned()
        ));
        assert!(
            failing_report
                .failures
                .contains(&"evolution_recursive_replay_items 8 below required 9".to_owned())
        );
        assert!(
            failing_report
                .failures
                .contains(&"evolution_drift_rollbacks 2 above maximum 1".to_owned())
        );
        assert!(failing_report.failures.contains(
            &"evolution_rollback_router_threshold_delta 0.030000 above maximum 0.020000".to_owned()
        ));
        assert!(failing_report.failures.contains(
            &"evolution_rollback_hierarchy_weight_delta 0.040000 above maximum 0.030000".to_owned()
        ));
    }

    fn record_cpu_runtime_adapter_experience(
        engine: &mut NoironEngine,
        adapter: &str,
        quality: f32,
        reward: f32,
        forward_energy: f32,
    ) {
        engine.experience.record(ExperienceInput {
            prompt: format!("inspect runtime adapter {adapter}"),
            profile: TaskProfile::General,
            lesson: format!("reuse {adapter} only when persisted evidence still wins"),
            quality,
            contradictions: Vec::new(),
            reflection_issues: Vec::new(),
            revision_actions: Vec::new(),
            stored_memory_id: None,
            router_threshold_after: 0.50,
            stream_windows: 1,
            route_budget: RouteBudget {
                threshold: 0.50,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::new(0.34, 0.33, 0.33),
            used_memory_ids: Vec::new(),
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            runtime_diagnostics: crate::reflection::RuntimeDiagnostics {
                model_id: Some("inspect-runtime".to_owned()),
                selected_adapter: Some(adapter.to_owned()),
                device_profile: Some("cpu".to_owned()),
                primary_lane: Some("cpu-vector".to_owned()),
                fallback_lane: Some("cpu-portable".to_owned()),
                memory_mode: Some("tiered-disk".to_owned()),
                layer_count: 8,
                global_layers: 2,
                local_window_layers: 4,
                convolutional_fusion_layers: 2,
                hidden_size: 96,
                local_window_tokens: 2048,
                forward_energy: Some(forward_energy),
                kv_influence: Some(0.42),
                imported_kv_blocks: 1,
                exported_kv_blocks: 1,
                hot_kv_precision_bits: Some(8),
                cold_kv_precision_bits: Some(4),
            },
            process_reward: ProcessRewardReport {
                total: reward,
                action: RewardAction::Reinforce,
                components: ProcessRewardComponents::default(),
                notes: Vec::new(),
            },
            live_evolution: Default::default(),
        });
    }

    #[test]
    fn state_inspection_matrix_gate_requires_every_explicit_device_to_pass() {
        let passing = StateInspectionGateReport {
            passed: true,
            failures: Vec::new(),
        };
        let failing = StateInspectionGateReport {
            passed: false,
            failures: vec!["runtime_kv_memory_count 0 below required 1".to_owned()],
        };

        let complete = StateInspectionMatrixGateReport::evaluate(
            DeviceClass::explicit_profiles()
                .iter()
                .copied()
                .map(|device| {
                    StateInspectionDeviceGateReport::new(device, passing.clone())
                        .with_runtime_evidence(1, 1, 1, 1, 1, 1, 1, 1)
                        .with_reflection_evidence(1, 1, 1)
                        .with_live_memory_feedback_evidence(1, 2)
                        .with_live_memory_feedback_detail_evidence(1, 2, 0, 0, 0.20)
                })
                .collect(),
        );

        assert!(complete.passed(), "{:?}", complete.failures);
        assert_eq!(
            complete.covered_devices(),
            DeviceClass::explicit_profiles().len()
        );
        assert!(complete.missing_devices().is_empty());
        assert!(complete.failed_devices().is_empty());
        assert!(
            complete
                .summary_line()
                .contains("state_inspection_matrix_gate: passed=true")
        );
        assert!(
            complete
                .summary_line()
                .contains("runtime_device_execution_device_profiles=12")
        );
        assert!(
            complete
                .summary_line()
                .contains("live_memory_feedback_device_profiles=12")
        );

        let incomplete = StateInspectionMatrixGateReport::evaluate(vec![
            StateInspectionDeviceGateReport::new(DeviceClass::CpuOnly, passing)
                .with_runtime_evidence(1, 1, 1, 1, 1, 1, 1, 1)
                .with_reflection_evidence(1, 1, 1)
                .with_live_memory_feedback_evidence(1, 2)
                .with_live_memory_feedback_detail_evidence(1, 2, 0, 0, 0.20),
            StateInspectionDeviceGateReport::new(DeviceClass::IntegratedGpu, failing),
        ]);

        assert!(!incomplete.passed());
        assert_eq!(incomplete.covered_devices(), 2);
        assert_eq!(
            incomplete.missing_devices().len(),
            DeviceClass::explicit_profiles().len() - 2
        );
        assert_eq!(
            incomplete.failed_devices(),
            vec![DeviceClass::IntegratedGpu]
        );
        assert!(
            incomplete
                .failures
                .iter()
                .any(|failure| failure.contains("missing="))
        );
        assert!(
            incomplete
                .failures
                .iter()
                .any(|failure| failure.contains("device integrated state inspection failed"))
        );
    }

    #[test]
    fn state_inspection_matrix_gate_can_require_runtime_evidence_per_device() {
        let passing = StateInspectionGateReport {
            passed: true,
            failures: Vec::new(),
        };
        let gate = StateInspectionMatrixGate {
            min_runtime_kv_memory_device_profiles: Some(2),
            min_runtime_model_device_profiles: Some(2),
            min_runtime_adapter_device_profiles: Some(2),
            min_runtime_forward_energy_device_profiles: Some(1),
            min_runtime_kv_influence_device_profiles: Some(1),
            min_runtime_kv_precision_device_profiles: Some(2),
            max_runtime_kv_precision_mismatches: Some(0),
            min_runtime_device_execution_device_profiles: Some(2),
            min_runtime_layer_mode_device_profiles: Some(2),
            min_runtime_all_layer_mode_device_profiles: Some(1),
            min_runtime_kv_import_device_profiles: Some(1),
            min_runtime_kv_export_device_profiles: Some(1),
            ..StateInspectionMatrixGate::default()
        };

        let report = StateInspectionMatrixGateReport::evaluate_with_gate(
            DeviceClass::explicit_profiles()
                .iter()
                .copied()
                .map(|device| {
                    let mut device_report =
                        StateInspectionDeviceGateReport::new(device, passing.clone());
                    match device {
                        DeviceClass::CpuOnly => {
                            device_report =
                                device_report.with_runtime_evidence(1, 1, 1, 1, 1, 1, 1, 1);
                            device_report = device_report.with_runtime_kv_precision_evidence(1);
                            device_report = device_report.with_runtime_layer_mode_evidence(1, 1);
                        }
                        DeviceClass::IntegratedGpu => {
                            device_report =
                                device_report.with_runtime_evidence(2, 1, 1, 0, 0, 1, 0, 0);
                            device_report = device_report.with_runtime_kv_precision_evidence(1);
                            device_report = device_report.with_runtime_layer_mode_evidence(1, 0);
                        }
                        _ => {}
                    }
                    device_report
                })
                .collect(),
            &gate,
        );

        assert!(report.passed(), "{:?}", report.failures);
        assert_eq!(report.runtime_kv_memory_device_profiles(), 2);
        assert_eq!(report.runtime_model_device_profiles(), 2);
        assert_eq!(report.runtime_adapter_device_profiles(), 2);
        assert_eq!(report.runtime_forward_energy_device_profiles(), 1);
        assert_eq!(report.runtime_kv_influence_device_profiles(), 1);
        assert_eq!(report.runtime_kv_precision_device_profiles(), 2);
        assert_eq!(report.runtime_kv_precision_mismatches(), 0);
        assert_eq!(report.runtime_device_execution_device_profiles(), 2);
        assert_eq!(report.runtime_layer_mode_device_profiles(), 2);
        assert_eq!(report.runtime_all_layer_mode_device_profiles(), 1);
        assert_eq!(report.runtime_kv_import_device_profiles(), 1);
        assert_eq!(report.runtime_kv_export_device_profiles(), 1);
        assert!(
            report
                .summary_line()
                .contains("runtime_kv_memory_device_profiles=2")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_model_device_profiles=2")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_device_execution_device_profiles=2")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_kv_precision_device_profiles=2")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_kv_precision_mismatches=0")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_layer_mode_device_profiles=2")
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_all_layer_mode_device_profiles=1")
        );

        let failing = StateInspectionMatrixGateReport::evaluate_with_gate(
            vec![
                StateInspectionDeviceGateReport::new(DeviceClass::CpuOnly, passing.clone())
                    .with_runtime_evidence(1, 1, 0, 0, 0, 0, 0, 0),
            ],
            &gate,
        );

        assert!(!failing.passed());
        assert!(
            failing.failures.iter().any(|failure| {
                failure == "runtime_kv_memory_device_profiles 1 below required 2"
            })
        );
        assert!(
            failing
                .failures
                .iter()
                .any(|failure| { failure == "runtime_adapter_device_profiles 0 below required 2" })
        );
        assert!(failing.failures.iter().any(|failure| {
            failure == "runtime_forward_energy_device_profiles 0 below required 1"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "runtime_kv_precision_device_profiles 0 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "runtime_device_execution_device_profiles 0 below required 2"
        }));
        assert!(
            failing.failures.iter().any(|failure| {
                failure == "runtime_layer_mode_device_profiles 0 below required 2"
            })
        );
        assert!(failing.failures.iter().any(|failure| {
            failure == "runtime_all_layer_mode_device_profiles 0 below required 1"
        }));
        assert!(
            failing.failures.iter().any(|failure| {
                failure == "runtime_kv_export_device_profiles 0 below required 1"
            })
        );

        let mismatch = StateInspectionMatrixGateReport::evaluate_with_gate(
            vec![
                StateInspectionDeviceGateReport::new(DeviceClass::CpuOnly, passing.clone())
                    .with_runtime_kv_precision_evidence(1)
                    .with_runtime_kv_precision_mismatches(1),
                StateInspectionDeviceGateReport::new(DeviceClass::IntegratedGpu, passing)
                    .with_runtime_kv_precision_evidence(1),
            ],
            &StateInspectionMatrixGate {
                min_runtime_kv_precision_device_profiles: Some(2),
                max_runtime_kv_precision_mismatches: Some(0),
                ..StateInspectionMatrixGate::default()
            },
        );

        assert_eq!(mismatch.runtime_kv_precision_device_profiles(), 2);
        assert_eq!(mismatch.runtime_kv_precision_mismatches(), 1);
        assert!(!mismatch.passed());
        assert!(
            mismatch
                .failures
                .iter()
                .any(|failure| { failure == "runtime_kv_precision_mismatches 1 above maximum 0" })
        );
        assert!(
            mismatch
                .summary_line()
                .contains("runtime_kv_precision_mismatches=1")
        );

        let adapter_mismatch_passing = StateInspectionGateReport {
            passed: true,
            failures: Vec::new(),
        };
        let adapter_mismatch = StateInspectionMatrixGateReport::evaluate_with_gate(
            vec![
                StateInspectionDeviceGateReport::new(
                    DeviceClass::CpuOnly,
                    adapter_mismatch_passing.clone(),
                )
                .with_runtime_adapter_selection_mismatches(1),
                StateInspectionDeviceGateReport::new(
                    DeviceClass::IntegratedGpu,
                    adapter_mismatch_passing,
                ),
            ],
            &StateInspectionMatrixGate {
                max_runtime_adapter_selection_mismatches: Some(0),
                ..StateInspectionMatrixGate::default()
            },
        );

        assert_eq!(adapter_mismatch.runtime_adapter_selection_mismatches(), 1);
        assert!(!adapter_mismatch.passed());
        assert!(adapter_mismatch.failures.iter().any(|failure| {
            failure == "runtime_adapter_selection_mismatches 1 above maximum 0"
        }));
        assert!(
            adapter_mismatch
                .summary_line()
                .contains("runtime_adapter_selection_mismatches=1")
        );
    }

    #[test]
    fn inspection_gate_rejects_runtime_kv_precision_execution_mismatch() {
        let mut engine = NoironEngine::new();
        engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
            DeviceClass::Embedded,
            0.35,
            0.00,
            0.45,
            0.20,
        ));
        engine.experience.record(ExperienceInput {
            prompt: "inspect runtime kv precision mismatch".to_owned(),
            profile: TaskProfile::General,
            lesson: "persisted diagnostics must match the device execution precision".to_owned(),
            quality: 0.88,
            contradictions: Vec::new(),
            reflection_issues: Vec::new(),
            revision_actions: Vec::new(),
            stored_memory_id: None,
            router_threshold_after: 0.50,
            stream_windows: 1,
            route_budget: RouteBudget {
                threshold: 0.50,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::new(0.34, 0.33, 0.33),
            used_memory_ids: Vec::new(),
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            runtime_diagnostics: crate::reflection::RuntimeDiagnostics {
                model_id: Some("inspect-runtime".to_owned()),
                selected_adapter: Some("portable-rust".to_owned()),
                device_profile: Some("embedded".to_owned()),
                primary_lane: Some("disk-streaming".to_owned()),
                fallback_lane: Some("cpu-portable".to_owned()),
                memory_mode: Some("minimal-disk".to_owned()),
                layer_count: 4,
                global_layers: 1,
                local_window_layers: 2,
                convolutional_fusion_layers: 1,
                hidden_size: 64,
                local_window_tokens: 512,
                forward_energy: Some(0.24),
                kv_influence: Some(0.36),
                imported_kv_blocks: 1,
                exported_kv_blocks: 1,
                hot_kv_precision_bits: Some(8),
                cold_kv_precision_bits: Some(4),
            },
            process_reward: ProcessRewardReport::default(),
            live_evolution: Default::default(),
        });

        let report = StateInspectionReport::from_engine(&engine, 1);
        let gate_report = report.evaluate(&StateInspectionGate {
            min_runtime_kv_precision_experiences: Some(1),
            max_runtime_kv_precision_mismatches: Some(0),
            ..StateInspectionGate::default()
        });

        assert_eq!(report.runtime_kv_precision_experience_count, 1);
        assert_eq!(report.runtime_kv_precision_mismatch_count, 1);
        assert!(!gate_report.passed());
        assert!(
            gate_report.failures.iter().any(|failure| {
                failure == "runtime_kv_precision_mismatch_count 1 above maximum 0"
            })
        );
        assert!(
            report
                .summary_line()
                .contains("runtime_kv_precision_mismatches=1")
        );
    }

    #[test]
    fn inspection_gate_rejects_runtime_adapter_selection_mismatch() {
        let mut engine = NoironEngine::new();
        engine.set_hardware_snapshot(crate::hardware::HardwareSnapshot::new(
            DeviceClass::CpuOnly,
            0.35,
            0.00,
            0.45,
            0.20,
        ));
        record_cpu_runtime_adapter_experience(&mut engine, "cpu-simd", 0.96, 0.92, 0.10);
        record_cpu_runtime_adapter_experience(&mut engine, "portable-rust", 0.56, 0.42, 0.35);

        let report = StateInspectionReport::from_engine(&engine, 2);
        let gate_report = report.evaluate(&StateInspectionGate {
            min_runtime_adapter_experiences: Some(2),
            max_runtime_adapter_selection_mismatches: Some(0),
            ..StateInspectionGate::default()
        });

        assert_eq!(report.runtime_adapter_experience_count, 2);
        assert_eq!(report.runtime_adapter_selection_mismatch_count, 1);
        assert!(!gate_report.passed());
        assert!(gate_report.failures.iter().any(|failure| {
            failure == "runtime_adapter_selection_mismatch_count 1 above maximum 0"
        }));
        assert!(
            report
                .summary_line()
                .contains("runtime_adapter_selection_mismatches=1")
        );
    }

    #[test]
    fn inspection_gate_tracks_runtime_kv_hold_evidence() {
        let mut engine = NoironEngine::new();
        engine.experience.record(ExperienceInput {
            prompt: "fast path exported runtime kv should be held".to_owned(),
            profile: TaskProfile::General,
            lesson: "runtime kv export can be audited even when durable runtime kv write is held"
                .to_owned(),
            quality: 0.64,
            contradictions: Vec::new(),
            reflection_issues: Vec::new(),
            revision_actions: Vec::new(),
            stored_memory_id: None,
            router_threshold_after: 0.5,
            stream_windows: 1,
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 0,
                fast_tokens: 4,
                attention_fraction: 0.0,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            used_memory_ids: Vec::new(),
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: vec![41],
            runtime_diagnostics: crate::reflection::RuntimeDiagnostics {
                exported_kv_blocks: 3,
                ..crate::reflection::RuntimeDiagnostics::default()
            },
            process_reward: ProcessRewardReport::default(),
            live_evolution: Default::default(),
        });

        let report = StateInspectionReport::from_engine(&engine, 1);
        let gate_report = report.evaluate(&StateInspectionGate {
            min_runtime_kv_export_experiences: Some(1),
            min_runtime_kv_hold_experiences: Some(1),
            min_runtime_kv_held_blocks: Some(2),
            ..StateInspectionGate::default()
        });

        assert_eq!(report.runtime_kv_export_experience_count, 1);
        assert_eq!(report.runtime_kv_hold_experience_count, 1);
        assert_eq!(report.runtime_kv_held_blocks, 2);
        assert!(gate_report.passed(), "{:?}", gate_report.failures);
        assert!(
            report
                .summary_line()
                .contains("runtime_kv_hold_experiences=1")
        );
        assert!(report.summary_line().contains("runtime_kv_held_blocks=2"));

        let failing = report.evaluate(&StateInspectionGate {
            min_runtime_kv_hold_experiences: Some(2),
            min_runtime_kv_held_blocks: Some(3),
            ..StateInspectionGate::default()
        });
        assert!(!failing.passed());
        assert!(
            failing.failures.iter().any(|failure| {
                failure == "runtime_kv_hold_experience_count 1 below required 2"
            })
        );
        assert!(
            failing
                .failures
                .iter()
                .any(|failure| { failure == "runtime_kv_held_blocks 2 below required 3" })
        );

        let passing = StateInspectionGateReport {
            passed: true,
            failures: Vec::new(),
        };
        let matrix = StateInspectionMatrixGateReport::evaluate_with_gate(
            DeviceClass::explicit_profiles()
                .iter()
                .copied()
                .map(|device| {
                    let report = StateInspectionDeviceGateReport::new(device, passing.clone());
                    if device == DeviceClass::CpuOnly {
                        report.with_runtime_kv_hold_evidence(1, 2)
                    } else {
                        report
                    }
                })
                .collect(),
            &StateInspectionMatrixGate {
                min_runtime_kv_hold_device_profiles: Some(1),
                ..StateInspectionMatrixGate::default()
            },
        );

        assert!(matrix.passed(), "{:?}", matrix.failures);
        assert_eq!(matrix.runtime_kv_hold_device_profiles(), 1);
        assert!(
            matrix
                .summary_line()
                .contains("runtime_kv_hold_device_profiles=1")
        );
    }

    #[test]
    fn state_inspection_matrix_gate_can_require_reflection_evidence_per_device() {
        let passing = StateInspectionGateReport {
            passed: true,
            failures: Vec::new(),
        };
        let gate = StateInspectionMatrixGate {
            min_reflection_issue_device_profiles: Some(2),
            min_critical_reflection_issue_device_profiles: Some(1),
            min_revision_action_device_profiles: Some(2),
            ..StateInspectionMatrixGate::default()
        };

        let report = StateInspectionMatrixGateReport::evaluate_with_gate(
            DeviceClass::explicit_profiles()
                .iter()
                .copied()
                .map(|device| {
                    let mut device_report =
                        StateInspectionDeviceGateReport::new(device, passing.clone());
                    match device {
                        DeviceClass::CpuOnly => {
                            device_report = device_report.with_reflection_evidence(1, 1, 1);
                        }
                        DeviceClass::IntegratedGpu => {
                            device_report = device_report.with_reflection_evidence(1, 0, 1);
                        }
                        _ => {}
                    }
                    device_report
                })
                .collect(),
            &gate,
        );

        assert!(report.passed(), "{:?}", report.failures);
        assert_eq!(report.reflection_issue_device_profiles(), 2);
        assert_eq!(report.critical_reflection_issue_device_profiles(), 1);
        assert_eq!(report.revision_action_device_profiles(), 2);
        assert!(
            report
                .summary_line()
                .contains("reflection_issue_device_profiles=2")
        );

        let failing = StateInspectionMatrixGateReport::evaluate_with_gate(
            vec![
                StateInspectionDeviceGateReport::new(DeviceClass::CpuOnly, passing)
                    .with_reflection_evidence(1, 0, 0),
            ],
            &gate,
        );

        assert!(!failing.passed());
        assert!(
            failing.failures.iter().any(|failure| {
                failure == "reflection_issue_device_profiles 1 below required 2"
            })
        );
        assert!(failing.failures.iter().any(|failure| {
            failure == "critical_reflection_issue_device_profiles 0 below required 1"
        }));
        assert!(
            failing
                .failures
                .iter()
                .any(|failure| { failure == "revision_action_device_profiles 0 below required 2" })
        );
    }

    #[test]
    fn state_inspection_matrix_gate_can_require_live_memory_feedback_per_device() {
        let passing = StateInspectionGateReport {
            passed: true,
            failures: Vec::new(),
        };
        let gate = StateInspectionMatrixGate {
            min_live_memory_feedback_device_profiles: Some(2),
            ..StateInspectionMatrixGate::default()
        };

        let report = StateInspectionMatrixGateReport::evaluate_with_gate(
            DeviceClass::explicit_profiles()
                .iter()
                .copied()
                .map(|device| {
                    let mut device_report =
                        StateInspectionDeviceGateReport::new(device, passing.clone());
                    match device {
                        DeviceClass::CpuOnly => {
                            device_report = device_report
                                .with_live_memory_feedback_evidence(1, 2)
                                .with_live_memory_feedback_detail_evidence(1, 2, 0, 0, 0.20);
                        }
                        DeviceClass::IntegratedGpu => {
                            device_report = device_report
                                .with_live_memory_feedback_evidence(2, 4)
                                .with_live_memory_feedback_detail_evidence(2, 3, 1, 1, 0.40);
                        }
                        _ => {}
                    }
                    device_report
                })
                .collect(),
            &gate,
        );

        assert!(report.passed(), "{:?}", report.failures);
        assert_eq!(report.live_memory_feedback_device_profiles(), 2);
        assert!(
            report
                .summary_line()
                .contains("live_memory_feedback_device_profiles=2")
        );

        let failing = StateInspectionMatrixGateReport::evaluate_with_gate(
            vec![
                StateInspectionDeviceGateReport::new(DeviceClass::CpuOnly, passing)
                    .with_live_memory_feedback_evidence(1, 0),
            ],
            &gate,
        );

        assert!(!failing.passed());
        assert!(failing.failures.iter().any(|failure| {
            failure == "live_memory_feedback_device_profiles 0 below required 2"
        }));
    }

    #[test]
    fn state_inspection_matrix_gate_can_require_evolution_evidence_per_device() {
        let passing = StateInspectionGateReport {
            passed: true,
            failures: Vec::new(),
        };
        let gate = StateInspectionMatrixGate {
            min_evolution_live_inference_device_profiles: Some(2),
            min_evolution_live_router_threshold_mutation_device_profiles: Some(1),
            min_evolution_live_hierarchy_weight_mutation_device_profiles: Some(1),
            min_evolution_live_memory_update_device_profiles: Some(2),
            min_evolution_live_stored_memory_update_device_profiles: Some(2),
            min_evolution_live_reflection_issue_device_profiles: Some(2),
            min_evolution_live_critical_reflection_issue_device_profiles: Some(1),
            min_evolution_live_revision_action_device_profiles: Some(2),
            min_evolution_replay_run_device_profiles: Some(2),
            min_evolution_replay_item_device_profiles: Some(2),
            min_evolution_router_threshold_mutation_device_profiles: Some(1),
            min_evolution_hierarchy_weight_mutation_device_profiles: Some(1),
            min_evolution_memory_update_device_profiles: Some(2),
            min_evolution_replay_live_memory_feedback_device_profiles: Some(2),
            min_evolution_replay_live_memory_feedback_detail_device_profiles: Some(2),
            min_evolution_replay_live_evolution_device_profiles: Some(2),
            min_evolution_replay_live_evolution_memory_update_device_profiles: Some(2),
            min_evolution_replay_live_evolution_critical_reflection_issue_device_profiles: Some(1),
            min_evolution_replay_live_evolution_revision_action_device_profiles: Some(2),
            min_evolution_recursive_replay_device_profiles: Some(1),
            min_evolution_recursive_runtime_call_device_profiles: Some(1),
            ..StateInspectionMatrixGate::default()
        };

        let report = StateInspectionMatrixGateReport::evaluate_with_gate(
            DeviceClass::explicit_profiles()
                .iter()
                .copied()
                .map(|device| {
                    let mut device_report =
                        StateInspectionDeviceGateReport::new(device, passing.clone());
                    match device {
                        DeviceClass::CpuOnly => {
                            device_report = device_report
                                .with_live_evolution_evidence(1, 1, 1, 3, 2, 1, 1, 1)
                                .with_evolution_evidence(1, 2, 1, 1, 3, 2, 1, 1)
                                .with_evolution_replay_live_memory_feedback_detail_evidence(
                                    1, 1, 0, 1, 0.2,
                                )
                                .with_evolution_replay_live_evolution_evidence(1, 2, 1, 1, 1, 1);
                        }
                        DeviceClass::IntegratedGpu => {
                            device_report = device_report
                                .with_live_evolution_evidence(1, 0, 0, 2, 1, 1, 0, 1)
                                .with_evolution_evidence(1, 1, 0, 0, 2, 1, 0, 0)
                                .with_evolution_replay_live_memory_feedback_detail_evidence(
                                    1, 1, 0, 0, 0.1,
                                )
                                .with_evolution_replay_live_evolution_evidence(1, 1, 0, 0, 0, 1);
                        }
                        _ => {}
                    }
                    device_report
                })
                .collect(),
            &gate,
        );

        assert!(report.passed(), "{:?}", report.failures);
        assert_eq!(report.evolution_live_inference_device_profiles(), 2);
        assert_eq!(
            report.evolution_live_router_threshold_mutation_device_profiles(),
            1
        );
        assert_eq!(
            report.evolution_live_hierarchy_weight_mutation_device_profiles(),
            1
        );
        assert_eq!(report.evolution_live_memory_update_device_profiles(), 2);
        assert_eq!(
            report.evolution_live_stored_memory_update_device_profiles(),
            2
        );
        assert_eq!(report.evolution_live_reflection_issue_device_profiles(), 2);
        assert_eq!(
            report.evolution_live_critical_reflection_issue_device_profiles(),
            1
        );
        assert_eq!(report.evolution_live_revision_action_device_profiles(), 2);
        assert_eq!(report.evolution_replay_run_device_profiles(), 2);
        assert_eq!(report.evolution_replay_item_device_profiles(), 2);
        assert_eq!(
            report.evolution_router_threshold_mutation_device_profiles(),
            1
        );
        assert_eq!(
            report.evolution_hierarchy_weight_mutation_device_profiles(),
            1
        );
        assert_eq!(report.evolution_memory_update_device_profiles(), 2);
        assert_eq!(
            report.evolution_replay_live_memory_feedback_device_profiles(),
            2
        );
        assert_eq!(
            report.evolution_replay_live_memory_feedback_detail_device_profiles(),
            2
        );
        assert_eq!(report.evolution_replay_live_evolution_device_profiles(), 2);
        assert_eq!(
            report.evolution_replay_live_evolution_memory_update_device_profiles(),
            2
        );
        assert_eq!(
            report.evolution_replay_live_evolution_critical_reflection_issue_device_profiles(),
            1
        );
        assert_eq!(
            report.evolution_replay_live_evolution_revision_action_device_profiles(),
            2
        );
        assert_eq!(report.evolution_recursive_replay_device_profiles(), 1);
        assert_eq!(report.evolution_recursive_runtime_call_device_profiles(), 1);
        assert!(
            report
                .summary_line()
                .contains("evolution_memory_update_device_profiles=2")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_live_inference_device_profiles=2")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_live_critical_reflection_issue_device_profiles=1")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_memory_feedback_device_profiles=2")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_memory_feedback_detail_device_profiles=2")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_evolution_device_profiles=2")
        );
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_evolution_memory_update_device_profiles=2")
        );
        assert!(report.summary_line().contains(
            "evolution_replay_live_evolution_critical_reflection_issue_device_profiles=1"
        ));
        assert!(
            report
                .summary_line()
                .contains("evolution_replay_live_evolution_revision_action_device_profiles=2")
        );

        let failing = StateInspectionMatrixGateReport::evaluate_with_gate(
            vec![
                StateInspectionDeviceGateReport::new(DeviceClass::CpuOnly, passing)
                    .with_live_evolution_evidence(1, 0, 0, 0, 0, 0, 0, 0)
                    .with_evolution_evidence(1, 0, 0, 0, 0, 0, 0, 0),
            ],
            &gate,
        );

        assert!(!failing.passed());
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_live_inference_device_profiles 1 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_live_memory_update_device_profiles 0 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_live_stored_memory_update_device_profiles 0 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_live_reflection_issue_device_profiles 0 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_live_critical_reflection_issue_device_profiles 0 below required 1"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_live_revision_action_device_profiles 0 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_replay_run_device_profiles 1 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_replay_item_device_profiles 0 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_router_threshold_mutation_device_profiles 0 below required 1"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_hierarchy_weight_mutation_device_profiles 0 below required 1"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_memory_update_device_profiles 0 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_replay_live_memory_feedback_device_profiles 0 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure
                == "evolution_replay_live_memory_feedback_detail_device_profiles 0 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_replay_live_evolution_device_profiles 0 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure
                == "evolution_replay_live_evolution_memory_update_device_profiles 0 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure
                == "evolution_replay_live_evolution_critical_reflection_issue_device_profiles 0 below required 1"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure
                == "evolution_replay_live_evolution_revision_action_device_profiles 0 below required 2"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_recursive_replay_device_profiles 0 below required 1"
        }));
        assert!(failing.failures.iter().any(|failure| {
            failure == "evolution_recursive_runtime_call_device_profiles 0 below required 1"
        }));
    }

    #[test]
    fn inspection_gate_rejects_experiences_without_runtime_evidence() {
        let mut engine = NoironEngine::new();
        engine.experience.record(ExperienceInput {
            prompt: "plain heuristic answer".to_owned(),
            profile: TaskProfile::General,
            lesson: "experience without runtime diagnostics should not satisfy runtime gates"
                .to_owned(),
            quality: 0.72,
            contradictions: Vec::new(),
            reflection_issues: Vec::new(),
            revision_actions: Vec::new(),
            stored_memory_id: None,
            router_threshold_after: 0.5,
            stream_windows: 1,
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::new(0.34, 0.33, 0.33),
            used_memory_ids: Vec::new(),
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: Vec::new(),
            runtime_diagnostics: crate::reflection::RuntimeDiagnostics::default(),
            process_reward: ProcessRewardReport {
                total: 0.62,
                action: RewardAction::Hold,
                components: ProcessRewardComponents::default(),
                notes: Vec::new(),
            },
            live_evolution: Default::default(),
        });

        let report = StateInspectionReport::from_engine(&engine, 3);
        let gate = StateInspectionGate {
            min_memories: None,
            min_runtime_kv_memories: None,
            min_experiences: Some(1),
            min_runtime_model_experiences: Some(1),
            min_runtime_adapter_experiences: Some(1),
            max_runtime_adapter_selection_mismatches: Some(0),
            min_runtime_forward_energy_experiences: Some(1),
            min_runtime_kv_influence_experiences: Some(1),
            min_runtime_kv_precision_experiences: Some(1),
            max_runtime_kv_precision_mismatches: Some(0),
            min_runtime_device_execution_experiences: Some(1),
            min_runtime_layer_mode_experiences: Some(1),
            min_runtime_all_layer_mode_experiences: Some(1),
            min_runtime_global_layers: Some(1),
            min_runtime_local_window_layers: Some(1),
            min_runtime_convolutional_fusion_layers: Some(1),
            min_runtime_kv_import_experiences: Some(1),
            min_runtime_kv_export_experiences: Some(1),
            min_runtime_kv_hold_experiences: None,
            min_runtime_kv_held_blocks: None,
            min_reflection_issue_experiences: None,
            min_critical_reflection_issue_experiences: None,
            min_revision_action_experiences: None,
            min_live_memory_feedback_experiences: None,
            min_live_memory_feedback_updates: None,
            min_live_memory_feedback_detail_experiences: None,
            min_live_memory_feedback_applied: None,
            min_live_memory_feedback_strength_delta: None,
            min_router_observations: None,
            min_evolution_live_inference_runs: None,
            min_evolution_live_router_threshold_mutations: None,
            min_evolution_live_hierarchy_weight_mutations: None,
            min_evolution_live_router_threshold_delta: None,
            min_evolution_live_hierarchy_weight_delta: None,
            min_evolution_live_memory_updates: None,
            min_evolution_live_stored_memory_updates: None,
            min_evolution_live_reflection_issues: None,
            min_evolution_live_critical_reflection_issues: None,
            min_evolution_live_revision_actions: None,
            min_evolution_replay_runs: None,
            min_evolution_replay_items: None,
            min_evolution_router_threshold_mutations: None,
            min_evolution_hierarchy_weight_mutations: None,
            min_evolution_router_threshold_delta: None,
            min_evolution_hierarchy_weight_delta: None,
            min_evolution_memory_updates: None,
            min_evolution_replay_live_memory_feedback_updates: None,
            min_evolution_replay_live_memory_feedback_detail_items: None,
            min_evolution_replay_live_memory_feedback_applied: None,
            min_evolution_replay_live_memory_feedback_strength_delta: None,
            min_evolution_replay_live_evolution_items: None,
            min_evolution_replay_live_evolution_memory_updates: None,
            min_evolution_replay_live_evolution_stored_memory_updates: None,
            min_evolution_replay_live_evolution_reflection_issues: None,
            min_evolution_replay_live_evolution_critical_reflection_issues: None,
            min_evolution_replay_live_evolution_revision_actions: None,
            min_evolution_recursive_replay_items: None,
            min_evolution_recursive_runtime_calls: None,
            max_evolution_drift_rollbacks: None,
            max_evolution_rollback_router_threshold_delta: None,
            max_evolution_rollback_hierarchy_weight_delta: None,
            require_runtime_kv_dimensions: false,
        };

        let gate_report = report.evaluate(&gate);

        assert_eq!(report.experience_count, 1);
        assert_eq!(report.runtime_model_experience_count, 0);
        assert_eq!(report.runtime_adapter_experience_count, 0);
        assert_eq!(report.runtime_forward_energy_experience_count, 0);
        assert_eq!(report.runtime_kv_influence_experience_count, 0);
        assert_eq!(report.runtime_kv_precision_experience_count, 0);
        assert_eq!(report.runtime_kv_precision_mismatch_count, 0);
        assert_eq!(report.runtime_device_execution_experience_count, 0);
        assert_eq!(report.runtime_layer_mode_experience_count, 0);
        assert_eq!(report.runtime_all_layer_mode_experience_count, 0);
        assert_eq!(report.runtime_global_layers, 0);
        assert_eq!(report.runtime_local_window_layers, 0);
        assert_eq!(report.runtime_convolutional_fusion_layers, 0);
        assert_eq!(report.runtime_kv_import_experience_count, 0);
        assert_eq!(report.runtime_kv_export_experience_count, 0);
        assert!(!gate_report.passed());
        assert!(
            gate_report
                .failures
                .contains(&"runtime_model_experience_count 0 below required 1".to_owned())
        );
        assert!(
            gate_report
                .failures
                .contains(&"runtime_adapter_experience_count 0 below required 1".to_owned())
        );
        assert!(
            gate_report
                .failures
                .contains(&"runtime_kv_export_experience_count 0 below required 1".to_owned())
        );
        assert!(
            gate_report.failures.contains(
                &"runtime_device_execution_experience_count 0 below required 1".to_owned()
            )
        );
        assert!(
            gate_report
                .failures
                .contains(&"runtime_layer_mode_experience_count 0 below required 1".to_owned())
        );
        assert!(
            gate_report
                .failures
                .contains(&"runtime_all_layer_mode_experience_count 0 below required 1".to_owned())
        );
        assert!(
            gate_report
                .failures
                .contains(&"runtime_global_layers 0 below required 1".to_owned())
        );
        assert!(
            gate_report
                .failures
                .contains(&"runtime_local_window_layers 0 below required 1".to_owned())
        );
        assert!(
            gate_report
                .failures
                .contains(&"runtime_convolutional_fusion_layers 0 below required 1".to_owned())
        );
    }
}
