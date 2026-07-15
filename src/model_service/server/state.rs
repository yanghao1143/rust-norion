use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::model_service::response::{
    model_service_dna_closed_loop_json, model_service_model_fallback_json,
    model_service_runtime_closed_loop_counters_json,
};
use crate::model_service::types::TimedOutcome;
use rust_norion::development_pollution::{
    DevelopmentEvidenceUseSurface, gate_development_evidence_payload_surface,
};
use rust_norion::{GenomeEvolutionPreview, TaskProfile, TenantScope, stable_redaction_digest};

pub(super) const MAX_ACTIVE_STREAM_ENGINE_REQUESTS: usize = 4;
const EVOLUTION_CANDIDATE_TTL: Duration = Duration::from_secs(300);
const MAX_EVOLUTION_CANDIDATES: usize = 16;
const BEHAVIOR_FEEDBACK_TTL: Duration = Duration::from_secs(300);
const MAX_BEHAVIOR_FEEDBACK_LEASES: usize = 32;

#[derive(Default)]
pub(super) struct ModelServiceServerState {
    active_engine_requests: AtomicUsize,
    stream_backpressure_rejections: AtomicUsize,
    active_requests: Mutex<Vec<ModelServiceActiveRequestTelemetry>>,
    cancellation_intents: Mutex<Vec<ModelServiceRequestCancellation>>,
    last_inference: Mutex<Option<ModelServiceLastInferenceTelemetry>>,
    evolution_candidates: Mutex<Vec<ModelServiceEvolutionCandidateLease>>,
    evolution_rollbacks: Mutex<Vec<ModelServiceEvolutionRollbackLease>>,
    behavior_feedback_leases: Mutex<Vec<ModelServiceBehaviorFeedbackLease>>,
}

#[derive(Debug, Clone)]
pub(super) struct ModelServiceEvolutionCandidateLease {
    pub(super) token: String,
    pub(super) prompt_digest: String,
    pub(super) prompt: String,
    pub(super) scope: TenantScope,
    pub(super) preview: GenomeEvolutionPreview,
    pub(super) max_tokens: Option<usize>,
    pub(super) baseline_elapsed_ms: u128,
    pub(super) baseline_token_count: usize,
    created_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ModelServiceEvolutionCandidateReceipt {
    pub(super) eligible: bool,
    pub(super) token: Option<String>,
    pub(super) prompt_digest: String,
    pub(super) candidate_digest: String,
    pub(super) generation_before: u64,
    pub(super) candidate_count: usize,
    pub(super) expires_in_seconds: u64,
    pub(super) reason: String,
}

#[derive(Debug, Clone)]
pub(super) struct ModelServiceBehaviorFeedbackLease {
    pub(super) token: String,
    pub(super) experience_id: u64,
    pub(super) scope: TenantScope,
    pub(super) runtime_model: Option<String>,
    pub(super) task_kind: String,
    created_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ModelServiceBehaviorFeedbackReceipt {
    pub(super) token: String,
    pub(super) experience_id: u64,
    pub(super) expires_in_seconds: u64,
    pub(super) runtime_model: Option<String>,
    pub(super) task_kind: String,
}

#[derive(Debug, Clone)]
pub(super) struct ModelServiceEvolutionRollbackLease {
    pub(super) token: String,
    pub(super) scope: TenantScope,
    pub(super) profile: TaskProfile,
    pub(super) expected_generation: u64,
    pub(super) candidate_digest: String,
    created_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ModelServiceEvolutionTokenError {
    Missing,
    Expired,
    ScopeMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ModelServiceBehaviorFeedbackTokenError {
    Missing,
    Expired,
    ScopeMismatch,
    ExperienceMismatch,
}

impl ModelServiceBehaviorFeedbackTokenError {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "behavior_feedback_token_missing_or_consumed",
            Self::Expired => "behavior_feedback_token_expired",
            Self::ScopeMismatch => "behavior_feedback_token_scope_mismatch",
            Self::ExperienceMismatch => "behavior_feedback_token_experience_mismatch",
        }
    }
}

impl ModelServiceEvolutionTokenError {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "evolution_token_missing_or_consumed",
            Self::Expired => "evolution_token_expired",
            Self::ScopeMismatch => "evolution_token_scope_mismatch",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ModelServiceActiveRequestTelemetry {
    pub(super) request_id: usize,
    pub(super) endpoint: String,
    pub(super) prompt_preview: String,
    pub(super) cancel_requested: bool,
    pub(super) cancel_reason: Option<String>,
    pub(super) repair_factor: Option<String>,
    pub(super) retag_label: Option<String>,
    started: Instant,
}

impl ModelServiceActiveRequestTelemetry {
    fn new(request_id: usize, endpoint: impl Into<String>, prompt: &str) -> Self {
        Self {
            request_id,
            endpoint: endpoint.into(),
            prompt_preview: prompt_preview(prompt, 160),
            cancel_requested: false,
            cancel_reason: None,
            repair_factor: None,
            retag_label: None,
            started: Instant::now(),
        }
    }

    pub(super) fn elapsed_ms(&self) -> u128 {
        self.started.elapsed().as_millis()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ModelServiceLastInferenceTelemetry {
    pub(super) request_id: usize,
    pub(super) endpoint: String,
    pub(super) elapsed_ms: u128,
    pub(super) runtime_model: Option<String>,
    pub(super) runtime_adapter: Option<String>,
    pub(super) runtime_device: Option<String>,
    pub(super) runtime_primary_lane: Option<String>,
    pub(super) runtime_fallback_lane: Option<String>,
    pub(super) runtime_memory_mode: Option<String>,
    pub(super) runtime_forward_energy: Option<f32>,
    pub(super) runtime_hot_kv_precision_bits: Option<usize>,
    pub(super) runtime_cold_kv_precision_bits: Option<usize>,
    pub(super) runtime_token_count: usize,
    pub(super) used_memory_count: usize,
    pub(super) stored_runtime_kv_memory_ids: Vec<u64>,
    pub(super) route_threshold: f32,
    pub(super) route_attention_tokens: usize,
    pub(super) route_fast_tokens: usize,
    pub(super) route_attention_fraction: f32,
    pub(super) runtime_kv_influence: Option<f32>,
    pub(super) runtime_imported_kv_blocks: usize,
    pub(super) runtime_weak_kv_imports_skipped: usize,
    pub(super) runtime_budget_limited_kv_imports_skipped: usize,
    pub(super) runtime_kv_budget_pressure: f32,
    pub(super) runtime_exported_kv_blocks: usize,
    pub(super) runtime_kv_segments_included: usize,
    pub(super) runtime_kv_segments_skipped: usize,
    pub(super) runtime_kv_segments_rejected: usize,
    pub(super) runtime_kv_segment_yield: Option<f32>,
    pub(super) model_fallback_json: Option<String>,
    pub(super) runtime_closed_loop_counters_json: Option<String>,
    pub(super) dna_closed_loop_json: Option<String>,
    pub(super) quality: f32,
    pub(super) process_reward: f32,
    pub(super) action: String,
    pub(super) error: Option<String>,
    pub(super) cancelled: bool,
    pub(super) timeout: bool,
    pub(super) retryable: bool,
    pub(super) runtime_error_note: Option<String>,
}

impl ModelServiceLastInferenceTelemetry {
    pub(super) fn from_timed(
        request_id: usize,
        endpoint: impl Into<String>,
        timed: &TimedOutcome,
    ) -> Self {
        let diagnostics = &timed.outcome.runtime_diagnostics;
        Self {
            request_id,
            endpoint: endpoint.into(),
            elapsed_ms: timed.elapsed_ms,
            runtime_model: diagnostics.model_id.clone(),
            runtime_adapter: diagnostics.selected_adapter.clone(),
            runtime_device: diagnostics.device_profile.clone(),
            runtime_primary_lane: diagnostics.primary_lane.clone(),
            runtime_fallback_lane: diagnostics.fallback_lane.clone(),
            runtime_memory_mode: diagnostics.memory_mode.clone(),
            runtime_forward_energy: diagnostics.forward_energy,
            runtime_hot_kv_precision_bits: diagnostics.hot_kv_precision_bits.map(usize::from),
            runtime_cold_kv_precision_bits: diagnostics.cold_kv_precision_bits.map(usize::from),
            runtime_token_count: timed.outcome.runtime_token_metrics.token_count,
            used_memory_count: timed.outcome.used_memories.len(),
            stored_runtime_kv_memory_ids: timed.outcome.stored_runtime_kv_memory_ids.clone(),
            route_threshold: timed.outcome.route_budget.threshold,
            route_attention_tokens: timed.outcome.route_budget.attention_tokens,
            route_fast_tokens: timed.outcome.route_budget.fast_tokens,
            route_attention_fraction: timed.outcome.route_budget.attention_fraction,
            runtime_kv_influence: diagnostics.kv_influence,
            runtime_imported_kv_blocks: diagnostics.imported_kv_blocks,
            runtime_weak_kv_imports_skipped: diagnostics.weak_runtime_kv_imports_skipped,
            runtime_budget_limited_kv_imports_skipped: diagnostics
                .budget_limited_runtime_kv_imports_skipped,
            runtime_kv_budget_pressure: diagnostics.runtime_kv_budget_pressure(),
            runtime_exported_kv_blocks: diagnostics.exported_kv_blocks,
            runtime_kv_segments_included: diagnostics.runtime_kv_segments_included,
            runtime_kv_segments_skipped: diagnostics.runtime_kv_segments_skipped,
            runtime_kv_segments_rejected: diagnostics.runtime_kv_segments_rejected,
            runtime_kv_segment_yield: diagnostics.runtime_kv_segment_yield(),
            model_fallback_json: Some(model_service_model_fallback_json(&timed.outcome)),
            runtime_closed_loop_counters_json: Some(
                model_service_runtime_closed_loop_counters_json(&timed.outcome),
            ),
            dna_closed_loop_json: Some(model_service_dna_closed_loop_json(&timed.outcome)),
            quality: timed.outcome.report.quality,
            process_reward: timed.outcome.process_reward.total,
            action: timed.outcome.process_reward.action.as_str().to_owned(),
            error: None,
            cancelled: false,
            timeout: false,
            retryable: false,
            runtime_error_note: None,
        }
    }

    pub(super) fn error(
        request_id: usize,
        endpoint: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            request_id,
            endpoint: endpoint.into(),
            elapsed_ms: 0,
            runtime_model: None,
            runtime_adapter: None,
            runtime_device: None,
            runtime_primary_lane: None,
            runtime_fallback_lane: None,
            runtime_memory_mode: None,
            runtime_forward_energy: None,
            runtime_hot_kv_precision_bits: None,
            runtime_cold_kv_precision_bits: None,
            runtime_token_count: 0,
            used_memory_count: 0,
            stored_runtime_kv_memory_ids: Vec::new(),
            route_threshold: 0.0,
            route_attention_tokens: 0,
            route_fast_tokens: 0,
            route_attention_fraction: 0.0,
            runtime_kv_influence: None,
            runtime_imported_kv_blocks: 0,
            runtime_weak_kv_imports_skipped: 0,
            runtime_budget_limited_kv_imports_skipped: 0,
            runtime_kv_budget_pressure: 0.0,
            runtime_exported_kv_blocks: 0,
            runtime_kv_segments_included: 0,
            runtime_kv_segments_skipped: 0,
            runtime_kv_segments_rejected: 0,
            runtime_kv_segment_yield: None,
            model_fallback_json: None,
            runtime_closed_loop_counters_json: None,
            dna_closed_loop_json: None,
            quality: 0.0,
            process_reward: 0.0,
            action: "error".to_owned(),
            error: Some(error.into()),
            cancelled: false,
            timeout: false,
            retryable: false,
            runtime_error_note: None,
        }
    }

    pub(super) fn error_with_state(
        request_id: usize,
        endpoint: impl Into<String>,
        error: impl Into<String>,
        cancelled: bool,
        timeout: bool,
        retryable: bool,
        runtime_error_note: Option<&str>,
    ) -> Self {
        let mut telemetry = Self::error(request_id, endpoint, error);
        telemetry.cancelled = cancelled;
        telemetry.timeout = timeout;
        telemetry.retryable = retryable;
        telemetry.runtime_error_note = runtime_error_note.map(str::to_owned);
        telemetry
    }

    pub(super) fn error_with_timed_state(
        request_id: usize,
        endpoint: impl Into<String>,
        error: impl Into<String>,
        timed: &TimedOutcome,
        cancelled: bool,
        timeout: bool,
        retryable: bool,
        runtime_error_note: Option<&str>,
    ) -> Self {
        let mut telemetry = Self::from_timed(request_id, endpoint, timed);
        telemetry.action = "error".to_owned();
        telemetry.error = Some(error.into());
        telemetry.cancelled = cancelled;
        telemetry.timeout = timeout;
        telemetry.retryable = retryable;
        telemetry.runtime_error_note = runtime_error_note.map(str::to_owned);
        telemetry
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ModelServiceRequestCancellation {
    pub(super) request_id: usize,
    pub(super) endpoint: Option<String>,
    pub(super) reason: String,
    pub(super) repair_factor: String,
    pub(super) retag_label: String,
    pub(super) target_active: bool,
}

impl ModelServiceRequestCancellation {
    fn new(
        request_id: usize,
        endpoint: Option<String>,
        reason: impl Into<String>,
        retag_label: impl Into<String>,
        target_active: bool,
    ) -> Self {
        Self {
            request_id,
            endpoint,
            reason: reason.into(),
            repair_factor: "runtime_request_splice".to_owned(),
            retag_label: retag_label.into(),
            target_active,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ModelServiceBackpressureRejection {
    pub(super) request_id: usize,
    pub(super) endpoint: String,
    pub(super) active_engine_requests: usize,
    pub(super) max_active_engine_requests: usize,
}

impl ModelServiceBackpressureRejection {
    pub(super) fn message(&self) -> String {
        format!(
            "model service backpressure: active_engine_requests={} max_active_engine_requests={}",
            self.active_engine_requests, self.max_active_engine_requests
        )
    }
}

impl ModelServiceServerState {
    pub(super) fn begin_engine_request(
        &self,
        request_id: usize,
        endpoint: impl Into<String>,
        prompt: &str,
    ) -> ModelServiceEngineRequestGuard<'_> {
        let endpoint = endpoint.into();
        self.active_engine_requests.fetch_add(1, Ordering::SeqCst);
        self.push_active_request(request_id, endpoint.clone(), prompt);
        ModelServiceEngineRequestGuard {
            state: self,
            request_id,
            endpoint,
        }
    }

    pub(super) fn try_begin_stream_engine_request(
        &self,
        request_id: usize,
        endpoint: impl Into<String>,
        prompt: &str,
    ) -> Result<ModelServiceEngineRequestGuard<'_>, ModelServiceBackpressureRejection> {
        let endpoint = endpoint.into();
        match self.active_engine_requests.fetch_update(
            Ordering::SeqCst,
            Ordering::SeqCst,
            |active| (active < MAX_ACTIVE_STREAM_ENGINE_REQUESTS).then_some(active + 1),
        ) {
            Ok(_) => {
                self.push_active_request(request_id, endpoint.clone(), prompt);
                Ok(ModelServiceEngineRequestGuard {
                    state: self,
                    request_id,
                    endpoint,
                })
            }
            Err(active_engine_requests) => {
                self.stream_backpressure_rejections
                    .fetch_add(1, Ordering::SeqCst);
                Err(ModelServiceBackpressureRejection {
                    request_id,
                    endpoint,
                    active_engine_requests,
                    max_active_engine_requests: MAX_ACTIVE_STREAM_ENGINE_REQUESTS,
                })
            }
        }
    }

    fn push_active_request(&self, request_id: usize, endpoint: String, prompt: &str) {
        if let Ok(mut active_requests) = self.active_requests.lock() {
            active_requests.push(ModelServiceActiveRequestTelemetry::new(
                request_id, endpoint, prompt,
            ));
        }
    }

    pub(super) fn active_engine_requests(&self) -> usize {
        self.active_engine_requests.load(Ordering::SeqCst)
    }

    pub(super) fn stream_backpressure_rejections(&self) -> usize {
        self.stream_backpressure_rejections.load(Ordering::SeqCst)
    }

    pub(super) fn active_requests(&self) -> Vec<ModelServiceActiveRequestTelemetry> {
        self.active_requests
            .lock()
            .map(|active_requests| active_requests.clone())
            .unwrap_or_default()
    }

    pub(super) fn request_cancel(
        &self,
        request_id: usize,
        reason: impl Into<String>,
        retag_label: impl Into<String>,
    ) -> ModelServiceRequestCancellation {
        let reason = reason.into();
        let retag_label = retag_label.into();
        let mut endpoint = None;
        let mut target_active = false;
        if let Ok(mut active_requests) = self.active_requests.lock()
            && let Some(request) = active_requests
                .iter_mut()
                .find(|request| request.request_id == request_id)
        {
            endpoint = Some(request.endpoint.clone());
            target_active = true;
            request.cancel_requested = true;
            request.cancel_reason = Some(reason.clone());
            request.repair_factor = Some("runtime_request_splice".to_owned());
            request.retag_label = Some(retag_label.clone());
        }

        let cancellation = ModelServiceRequestCancellation::new(
            request_id,
            endpoint,
            reason,
            retag_label,
            target_active,
        );
        if target_active && let Ok(mut cancellation_intents) = self.cancellation_intents.lock() {
            cancellation_intents.retain(|intent| intent.request_id != request_id);
            cancellation_intents.push(cancellation.clone());
        }
        cancellation
    }

    pub(super) fn cancellation_intent(
        &self,
        request_id: usize,
    ) -> Option<ModelServiceRequestCancellation> {
        self.cancellation_intents.lock().ok().and_then(|intents| {
            intents
                .iter()
                .find(|intent| intent.request_id == request_id)
                .cloned()
        })
    }

    pub(super) fn is_cancel_requested(&self, request_id: usize) -> bool {
        self.cancellation_intent(request_id).is_some()
    }

    pub(super) fn record_inference(&self, telemetry: ModelServiceLastInferenceTelemetry) {
        if let Ok(mut last_inference) = self.last_inference.lock() {
            *last_inference = Some(telemetry);
        }
    }

    pub(super) fn last_inference(&self) -> Option<ModelServiceLastInferenceTelemetry> {
        self.last_inference
            .lock()
            .ok()
            .and_then(|last_inference| last_inference.clone())
    }

    pub(super) fn register_evolution_candidate(
        &self,
        request_id: usize,
        prompt: &str,
        scope: &TenantScope,
        max_tokens: Option<usize>,
        timed: &TimedOutcome,
    ) -> ModelServiceEvolutionCandidateReceipt {
        let preview = &timed.outcome.genome_evolution_preview;
        let scope_digest = scope.scope_digest();
        let prompt_digest = stable_redaction_digest([
            "model-service-evolution-prompt-v1",
            scope_digest.as_str(),
            prompt,
        ]);
        let reason = preview
            .eligibility_reason()
            .unwrap_or("ready_for_explicit_apply");
        if !preview.is_eligible() {
            return ModelServiceEvolutionCandidateReceipt {
                eligible: false,
                token: None,
                prompt_digest,
                candidate_digest: preview.candidate_digest.clone(),
                generation_before: preview.generation_before,
                candidate_count: preview.candidate_count(),
                expires_in_seconds: 0,
                reason: reason.to_owned(),
            };
        }

        let request_id = request_id.to_string();
        let token = evolution_capability_token([
            "model-service-evolution-token-v1",
            request_id.as_str(),
            prompt_digest.as_str(),
            preview.candidate_digest.as_str(),
        ]);
        let mut candidates = self
            .evolution_candidates
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        candidates.retain(|candidate| {
            candidate.created_at.elapsed() < EVOLUTION_CANDIDATE_TTL
                && !(candidate.scope == *scope
                    && candidate.preview.profile == preview.profile
                    && candidate.preview.candidate_digest == preview.candidate_digest)
        });
        candidates.push(ModelServiceEvolutionCandidateLease {
            token: token.clone(),
            prompt_digest: prompt_digest.clone(),
            prompt: prompt.to_owned(),
            scope: scope.clone(),
            preview: preview.clone(),
            max_tokens,
            baseline_elapsed_ms: timed.elapsed_ms,
            baseline_token_count: timed.outcome.runtime_token_metrics.token_count,
            created_at: Instant::now(),
        });
        if candidates.len() > MAX_EVOLUTION_CANDIDATES {
            let overflow = candidates.len() - MAX_EVOLUTION_CANDIDATES;
            candidates.drain(..overflow);
        }

        ModelServiceEvolutionCandidateReceipt {
            eligible: true,
            token: Some(token),
            prompt_digest,
            candidate_digest: preview.candidate_digest.clone(),
            generation_before: preview.generation_before,
            candidate_count: preview.candidate_count(),
            expires_in_seconds: EVOLUTION_CANDIDATE_TTL.as_secs(),
            reason: reason.to_owned(),
        }
    }

    pub(super) fn consume_evolution_candidate(
        &self,
        token: &str,
        scope: &TenantScope,
    ) -> Result<ModelServiceEvolutionCandidateLease, ModelServiceEvolutionTokenError> {
        let mut candidates = self
            .evolution_candidates
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(index) = candidates
            .iter()
            .position(|candidate| candidate.token == token)
        else {
            candidates.retain(|candidate| candidate.created_at.elapsed() < EVOLUTION_CANDIDATE_TTL);
            return Err(ModelServiceEvolutionTokenError::Missing);
        };
        if candidates[index].created_at.elapsed() >= EVOLUTION_CANDIDATE_TTL {
            candidates.remove(index);
            return Err(ModelServiceEvolutionTokenError::Expired);
        }
        if candidates[index].scope != *scope {
            return Err(ModelServiceEvolutionTokenError::ScopeMismatch);
        }
        Ok(candidates.remove(index))
    }

    pub(super) fn register_behavior_feedback(
        &self,
        request_id: usize,
        experience_id: u64,
        scope: &TenantScope,
        runtime_model: Option<&str>,
        task_kind: &str,
    ) -> ModelServiceBehaviorFeedbackReceipt {
        let request_id = request_id.to_string();
        let experience_id_text = experience_id.to_string();
        let scope_digest = scope.scope_digest();
        let runtime_model = runtime_model.map(str::to_owned);
        let runtime_model_text = runtime_model.as_deref().unwrap_or("none");
        let token = evolution_capability_token([
            "model-service-behavior-feedback-token-v1",
            request_id.as_str(),
            experience_id_text.as_str(),
            scope_digest.as_str(),
            runtime_model_text,
            task_kind,
        ]);
        let mut leases = self
            .behavior_feedback_leases
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        leases.retain(|lease| {
            lease.created_at.elapsed() < BEHAVIOR_FEEDBACK_TTL
                && !(lease.scope == *scope && lease.experience_id == experience_id)
        });
        leases.push(ModelServiceBehaviorFeedbackLease {
            token: token.clone(),
            experience_id,
            scope: scope.clone(),
            runtime_model: runtime_model.clone(),
            task_kind: task_kind.to_owned(),
            created_at: Instant::now(),
        });
        if leases.len() > MAX_BEHAVIOR_FEEDBACK_LEASES {
            let overflow = leases.len() - MAX_BEHAVIOR_FEEDBACK_LEASES;
            leases.drain(..overflow);
        }
        ModelServiceBehaviorFeedbackReceipt {
            token,
            experience_id,
            expires_in_seconds: BEHAVIOR_FEEDBACK_TTL.as_secs(),
            runtime_model,
            task_kind: task_kind.to_owned(),
        }
    }

    pub(super) fn consume_behavior_feedback(
        &self,
        token: &str,
        experience_id: u64,
        scope: &TenantScope,
    ) -> Result<ModelServiceBehaviorFeedbackLease, ModelServiceBehaviorFeedbackTokenError> {
        let mut leases = self
            .behavior_feedback_leases
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(index) = leases.iter().position(|lease| lease.token == token) else {
            leases.retain(|lease| lease.created_at.elapsed() < BEHAVIOR_FEEDBACK_TTL);
            return Err(ModelServiceBehaviorFeedbackTokenError::Missing);
        };
        if leases[index].created_at.elapsed() >= BEHAVIOR_FEEDBACK_TTL {
            leases.remove(index);
            return Err(ModelServiceBehaviorFeedbackTokenError::Expired);
        }
        if leases[index].scope != *scope {
            return Err(ModelServiceBehaviorFeedbackTokenError::ScopeMismatch);
        }
        if leases[index].experience_id != experience_id {
            return Err(ModelServiceBehaviorFeedbackTokenError::ExperienceMismatch);
        }
        Ok(leases.remove(index))
    }

    pub(super) fn register_evolution_rollback(
        &self,
        request_id: usize,
        scope: &TenantScope,
        profile: TaskProfile,
        expected_generation: u64,
        candidate_digest: &str,
    ) -> String {
        let request_id = request_id.to_string();
        let expected_generation_text = expected_generation.to_string();
        let scope_digest = scope.scope_digest();
        let token = evolution_capability_token([
            "model-service-evolution-rollback-token-v1",
            request_id.as_str(),
            scope_digest.as_str(),
            expected_generation_text.as_str(),
            candidate_digest,
        ]);
        let mut rollbacks = self
            .evolution_rollbacks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        rollbacks.retain(|rollback| {
            rollback.created_at.elapsed() < EVOLUTION_CANDIDATE_TTL
                && !(rollback.scope == *scope && rollback.profile == profile)
        });
        rollbacks.push(ModelServiceEvolutionRollbackLease {
            token: token.clone(),
            scope: scope.clone(),
            profile,
            expected_generation,
            candidate_digest: candidate_digest.to_owned(),
            created_at: Instant::now(),
        });
        if rollbacks.len() > MAX_EVOLUTION_CANDIDATES {
            let overflow = rollbacks.len() - MAX_EVOLUTION_CANDIDATES;
            rollbacks.drain(..overflow);
        }
        token
    }

    pub(super) fn consume_evolution_rollback(
        &self,
        token: &str,
        scope: &TenantScope,
    ) -> Result<ModelServiceEvolutionRollbackLease, ModelServiceEvolutionTokenError> {
        let mut rollbacks = self
            .evolution_rollbacks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(index) = rollbacks
            .iter()
            .position(|rollback| rollback.token == token)
        else {
            rollbacks.retain(|rollback| rollback.created_at.elapsed() < EVOLUTION_CANDIDATE_TTL);
            return Err(ModelServiceEvolutionTokenError::Missing);
        };
        if rollbacks[index].created_at.elapsed() >= EVOLUTION_CANDIDATE_TTL {
            rollbacks.remove(index);
            return Err(ModelServiceEvolutionTokenError::Expired);
        }
        if rollbacks[index].scope != *scope {
            return Err(ModelServiceEvolutionTokenError::ScopeMismatch);
        }
        Ok(rollbacks.remove(index))
    }

    fn finish_engine_request(&self, request_id: usize, endpoint: &str) {
        self.active_engine_requests.fetch_sub(1, Ordering::SeqCst);
        if let Ok(mut active_requests) = self.active_requests.lock()
            && let Some(index) = active_requests.iter().position(|request| {
                request.request_id == request_id && request.endpoint == endpoint
            })
        {
            active_requests.remove(index);
        }
        if let Ok(mut cancellation_intents) = self.cancellation_intents.lock() {
            cancellation_intents.retain(|intent| intent.request_id != request_id);
        }
    }
}

pub(super) struct ModelServiceEngineRequestGuard<'a> {
    state: &'a ModelServiceServerState,
    request_id: usize,
    endpoint: String,
}

impl Drop for ModelServiceEngineRequestGuard<'_> {
    fn drop(&mut self) {
        self.state
            .finish_engine_request(self.request_id, &self.endpoint);
    }
}

fn prompt_preview(prompt: &str, max_chars: usize) -> String {
    let gate = gate_development_evidence_payload_surface(
        "model-service-active-request",
        "model_service_active_request",
        prompt,
        DevelopmentEvidenceUseSurface::Prompt,
    );
    if !gate.allowed {
        return gate.source_digest;
    }

    let normalized = prompt
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let text = if normalized.is_empty() {
        prompt.trim()
    } else {
        normalized.as_str()
    };
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let keep_chars = max_chars.saturating_sub(3);
    let mut preview = text.chars().take(keep_chars).collect::<String>();
    preview.push_str("...");
    preview
}

fn evolution_capability_token<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let parts = parts.into_iter().collect::<Vec<_>>();
    let hash = |domain: u8| {
        let mut hasher = RandomState::new().build_hasher();
        domain.hash(&mut hasher);
        parts.hash(&mut hasher);
        hasher.finish()
    };
    format!("redaction-digest:{:016x}{:016x}", hash(0), hash(1))
}

#[cfg(test)]
mod tests {
    use rust_norion::{HeuristicBackend, InferenceRequest, NoironEngine, TaskProfile, TenantScope};

    use super::*;

    fn preview_fixture() -> GenomeEvolutionPreview {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        engine
            .infer(
                InferenceRequest::new("runtime evolution token fixture", TaskProfile::General)
                    .with_tenant_scope(TenantScope::new("tenant", "workspace", "session")),
                &mut backend,
            )
            .genome_evolution_preview
    }

    #[test]
    fn cancellation_retags_active_request_until_guard_drops() {
        let state = ModelServiceServerState::default();
        let active = state.begin_engine_request(42, "business-cycle-stream", "repair me");

        let cancellation =
            state.request_cancel(42, "stalled_generation", "repair_factor:runtime_splice");

        assert!(cancellation.target_active);
        assert_eq!(
            cancellation.endpoint.as_deref(),
            Some("business-cycle-stream")
        );
        assert_eq!(cancellation.repair_factor, "runtime_request_splice");
        assert!(state.is_cancel_requested(42));
        let active_requests = state.active_requests();
        assert_eq!(active_requests.len(), 1);
        assert!(active_requests[0].cancel_requested);
        assert_eq!(
            active_requests[0].cancel_reason.as_deref(),
            Some("stalled_generation")
        );
        assert_eq!(
            active_requests[0].retag_label.as_deref(),
            Some("repair_factor:runtime_splice")
        );

        drop(active);

        assert!(!state.is_cancel_requested(42));
        assert!(state.active_requests().is_empty());
    }

    #[test]
    fn active_request_prompt_preview_redacts_polluted_marker() {
        let state = ModelServiceServerState::default();
        let _active = state.begin_engine_request(
            43,
            "chat",
            "retired_version_marker:v0.305.0 C:/private/old_prompt.txt",
        );

        let active_requests = state.active_requests();
        let preview = &active_requests[0].prompt_preview;

        assert!(preview.starts_with("redaction-digest:"));
        assert!(!preview.contains("retired_version_marker"));
        assert!(!preview.contains("C:/private"));
    }

    #[test]
    fn cancellation_for_inactive_request_is_not_held_for_future_id_reuse() {
        let state = ModelServiceServerState::default();

        let cancellation = state.request_cancel(7, "already_done", "repair_requested");

        assert!(!cancellation.target_active);
        assert!(!state.is_cancel_requested(7));
    }

    #[test]
    fn timed_error_preserves_fallback_diagnostics() {
        let mut engine = NoironEngine::new();
        let mut backend = HeuristicBackend;
        let mut outcome = engine.infer(
            InferenceRequest::new("failed fallback telemetry", TaskProfile::General),
            &mut backend,
        );
        outcome.runtime_diagnostics.model_fallback_configured = true;
        outcome.runtime_diagnostics.model_fallback_primary_failed = true;
        outcome.runtime_diagnostics.model_fallback_attempts = 2;
        outcome.runtime_diagnostics.model_fallback_failures = 2;
        outcome.runtime_diagnostics.model_fallback_all_failed = true;
        let timed = TimedOutcome {
            outcome,
            elapsed_ms: 41,
        };

        let telemetry = ModelServiceLastInferenceTelemetry::error_with_timed_state(
            9,
            "chat-completions",
            "runtime failed after fallback",
            &timed,
            false,
            false,
            true,
            Some("runtime_error:label=runtime_error:timeout=false"),
        );

        let fallback = telemetry.model_fallback_json.as_deref().unwrap();
        assert!(fallback.contains("\"configured\":true"));
        assert!(fallback.contains("\"primary_failed\":true"));
        assert!(fallback.contains("\"attempts\":2"));
        assert!(fallback.contains("\"failures\":2"));
        assert!(fallback.contains("\"all_failed\":true"));
        assert_eq!(telemetry.elapsed_ms, 41);
        assert_eq!(telemetry.action, "error");
        assert_eq!(
            telemetry.error.as_deref(),
            Some("runtime failed after fallback")
        );
        assert!(telemetry.retryable);
    }

    #[test]
    fn stream_backpressure_rejects_over_active_limit_and_recovers_after_drop() {
        let state = ModelServiceServerState::default();
        assert_eq!(state.stream_backpressure_rejections(), 0);
        let guards = (0..MAX_ACTIVE_STREAM_ENGINE_REQUESTS)
            .map(|index| {
                state
                    .try_begin_stream_engine_request(index + 1, "generate-stream", "hold stream")
                    .expect("stream slot should be available")
            })
            .collect::<Vec<_>>();

        let rejection =
            match state.try_begin_stream_engine_request(99, "generate-stream", "overflow stream") {
                Ok(_) => panic!("stream request should be backpressure rejected"),
                Err(rejection) => rejection,
            };

        assert_eq!(rejection.request_id, 99);
        assert_eq!(rejection.endpoint, "generate-stream");
        assert_eq!(
            rejection.active_engine_requests,
            MAX_ACTIVE_STREAM_ENGINE_REQUESTS
        );
        assert_eq!(
            rejection.max_active_engine_requests,
            MAX_ACTIVE_STREAM_ENGINE_REQUESTS
        );
        assert_eq!(state.stream_backpressure_rejections(), 1);
        assert!(rejection.message().contains("backpressure"));

        drop(guards);
        assert_eq!(state.active_engine_requests(), 0);
        let recovered = state
            .try_begin_stream_engine_request(100, "chat-stream", "new stream")
            .expect("stream slot should recover after guards drop");
        assert_eq!(state.active_engine_requests(), 1);
        assert_eq!(state.stream_backpressure_rejections(), 1);
        drop(recovered);
    }

    #[test]
    fn evolution_candidate_token_is_scope_bound_and_one_shot() {
        let state = ModelServiceServerState::default();
        let scope = TenantScope::new("tenant", "workspace", "session");
        state
            .evolution_candidates
            .lock()
            .unwrap()
            .push(ModelServiceEvolutionCandidateLease {
                token: "candidate-token".to_owned(),
                prompt_digest: "redaction-digest:prompt".to_owned(),
                prompt: "candidate prompt".to_owned(),
                scope: scope.clone(),
                preview: preview_fixture(),
                max_tokens: Some(64),
                baseline_elapsed_ms: 120,
                baseline_token_count: 24,
                created_at: Instant::now(),
            });

        let wrong_scope = TenantScope::new("tenant", "workspace", "other-session");
        let error = state
            .consume_evolution_candidate("candidate-token", &wrong_scope)
            .unwrap_err();
        assert_eq!(error, ModelServiceEvolutionTokenError::ScopeMismatch);
        assert!(
            state
                .consume_evolution_candidate("candidate-token", &scope)
                .is_ok()
        );
        assert_eq!(
            state
                .consume_evolution_candidate("candidate-token", &scope)
                .unwrap_err(),
            ModelServiceEvolutionTokenError::Missing
        );
    }

    #[test]
    fn evolution_capability_tokens_are_randomized_and_128_bit() {
        let first = evolution_capability_token(["candidate", "scope", "digest"]);
        let second = evolution_capability_token(["candidate", "scope", "digest"]);

        assert_ne!(first, second);
        assert_eq!(first.len(), "redaction-digest:".len() + 32);
        assert!(first.starts_with("redaction-digest:"));
    }

    #[test]
    fn evolution_candidate_token_expires_and_is_removed() {
        let state = ModelServiceServerState::default();
        let scope = TenantScope::new("tenant", "workspace", "session");
        state
            .evolution_candidates
            .lock()
            .unwrap()
            .push(ModelServiceEvolutionCandidateLease {
                token: "expired-token".to_owned(),
                prompt_digest: "redaction-digest:prompt".to_owned(),
                prompt: "expired prompt".to_owned(),
                scope: scope.clone(),
                preview: preview_fixture(),
                max_tokens: Some(64),
                baseline_elapsed_ms: 120,
                baseline_token_count: 24,
                created_at: Instant::now() - EVOLUTION_CANDIDATE_TTL - Duration::from_secs(1),
            });

        assert_eq!(
            state
                .consume_evolution_candidate("expired-token", &scope)
                .unwrap_err(),
            ModelServiceEvolutionTokenError::Expired
        );
        assert_eq!(
            state
                .consume_evolution_candidate("expired-token", &scope)
                .unwrap_err(),
            ModelServiceEvolutionTokenError::Missing
        );
    }

    #[test]
    fn behavior_feedback_token_is_scope_experience_bound_and_one_shot() {
        let state = ModelServiceServerState::default();
        let scope = TenantScope::new("tenant", "workspace", "session");
        let receipt = state.register_behavior_feedback(17, 42, &scope, Some("model-a"), "gomoku");
        assert_eq!(receipt.runtime_model.as_deref(), Some("model-a"));
        assert_eq!(receipt.task_kind, "gomoku");

        assert_eq!(
            state
                .consume_behavior_feedback(&receipt.token, 43, &scope)
                .unwrap_err(),
            ModelServiceBehaviorFeedbackTokenError::ExperienceMismatch
        );
        let wrong_scope = TenantScope::new("tenant", "workspace", "other-session");
        assert_eq!(
            state
                .consume_behavior_feedback(&receipt.token, 42, &wrong_scope)
                .unwrap_err(),
            ModelServiceBehaviorFeedbackTokenError::ScopeMismatch
        );
        let lease = state
            .consume_behavior_feedback(&receipt.token, 42, &scope)
            .unwrap();
        assert_eq!(lease.runtime_model.as_deref(), Some("model-a"));
        assert_eq!(lease.task_kind, "gomoku");
        assert_eq!(
            state
                .consume_behavior_feedback(&receipt.token, 42, &scope)
                .unwrap_err(),
            ModelServiceBehaviorFeedbackTokenError::Missing
        );
    }
}
