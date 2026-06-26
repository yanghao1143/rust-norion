use rust_norion::{
    DraftToken, InferenceBackend, NoironEngine, StateInspectionReport,
    append_rust_check_trace_jsonl,
};

use super::feedback::{
    annotate_model_service_feedback_experience_with_source,
    annotate_model_service_rust_check_experience, apply_model_service_feedback,
    model_service_feedback_memory_ids, model_service_rust_check_feedback_request,
};
use super::gates::{
    model_service_state_gate_report_for_request,
    model_service_state_gate_requires_full_experience_scan,
    model_service_trace_gate_report_for_request,
};
use super::profile::detect_profile;
use super::request::{
    ModelServiceBusinessCycleRequest, ModelServiceFeedbackRequest, ModelServicePoolDispatchRequest,
    ModelServicePoolStageDispatchRequest, ModelServiceRustCheckRequest,
};
use super::rust_check::model_service_rust_check_report;
use super::types::ModelServiceBusinessCycleReport;
use crate::Args;
use crate::gemma_business::contract::annotate_model_service_business_case_for_timed;
use crate::inference_runner::run_timed_inference_stream_checked_with_options;

pub(crate) enum ModelServiceBusinessCycleEvent<'a> {
    Stage(&'static str),
    Token(&'a DraftToken),
    Meta(String),
}

fn business_cycle_cancel_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Interrupted,
        "business_cycle cancellation requested by runtime_request_splice",
    )
}

fn check_business_cycle_cancel(should_cancel: &mut dyn FnMut() -> bool) -> std::io::Result<()> {
    if should_cancel() {
        Err(business_cycle_cancel_error())
    } else {
        Ok(())
    }
}

fn annotate_model_service_pool_dispatch_experience(
    engine: &mut NoironEngine,
    experience_id: u64,
    dispatch: Option<&ModelServicePoolDispatchRequest>,
    worker_forwarded: bool,
) -> bool {
    let Some(dispatch) = dispatch else {
        return false;
    };
    let Some(record) = engine.experience.record_mut(experience_id) else {
        return false;
    };
    record.process_reward.notes.insert(
        0,
        model_service_pool_dispatch_note(dispatch, worker_forwarded),
    );
    true
}

fn model_service_pool_dispatch_note(
    dispatch: &ModelServicePoolDispatchRequest,
    worker_forwarded: bool,
) -> String {
    format!(
        "pool_dispatch:selected_role={}:selected_port={}:selected_endpoint={}:context_window={}:default_max_tokens={}:configured_max_tokens={}:effective_max_tokens={}:max_tokens_clamped={}:max_tokens_clamp_reason={}:low_priority={}:forwarded={}:dispatch_mode={}:dispatch_reason={}",
        dispatch.selected_role,
        option_u64_note_value(dispatch.selected_port),
        dispatch.selected_base_url.as_deref().unwrap_or("none"),
        option_u64_note_value(dispatch.context_window),
        option_u64_note_value(dispatch.default_max_tokens),
        option_usize_note_value(dispatch.configured_max_tokens),
        option_usize_note_value(dispatch.effective_max_tokens),
        dispatch.max_tokens_clamped,
        dispatch
            .max_tokens_clamp_reason
            .as_deref()
            .unwrap_or("none"),
        dispatch.can_accept_low_priority_task,
        worker_forwarded,
        ModelServicePoolDispatchRequest::dispatch_mode(worker_forwarded),
        dispatch.dispatch_reason(worker_forwarded),
    )
}

fn annotate_model_service_pool_stage_dispatch_experience(
    engine: &mut NoironEngine,
    experience_id: u64,
    dispatches: &[ModelServicePoolStageDispatchRequest],
) -> bool {
    if dispatches.is_empty() {
        return false;
    }
    let Some(record) = engine.experience.record_mut(experience_id) else {
        return false;
    };
    for dispatch in dispatches.iter().rev() {
        record
            .process_reward
            .notes
            .insert(0, model_service_pool_stage_dispatch_note(dispatch));
    }
    true
}

fn model_service_pool_stage_dispatch_note(
    dispatch: &ModelServicePoolStageDispatchRequest,
) -> String {
    format!(
        "pool_stage_dispatch:task_kind={}:selected_role={}:selected_port={}:selected_endpoint={}:context_window={}:default_max_tokens={}:configured_max_tokens={}:effective_max_tokens={}:max_tokens_clamped={}:max_tokens_clamp_reason={}:low_priority={}:dispatch_mode={}:dispatch_reason={}",
        dispatch.task_kind,
        dispatch.selected_role,
        option_u64_note_value(dispatch.selected_port),
        dispatch.selected_base_url.as_deref().unwrap_or("none"),
        option_u64_note_value(dispatch.context_window),
        option_u64_note_value(dispatch.default_max_tokens),
        option_usize_note_value(dispatch.configured_max_tokens),
        option_usize_note_value(dispatch.effective_max_tokens),
        dispatch.max_tokens_clamped,
        dispatch
            .max_tokens_clamp_reason
            .as_deref()
            .unwrap_or("none"),
        dispatch.can_accept_low_priority_task,
        ModelServicePoolStageDispatchRequest::dispatch_mode(),
        ModelServicePoolStageDispatchRequest::dispatch_reason(),
    )
}

fn option_u64_note_value(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn option_usize_note_value(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

pub(crate) fn run_model_service_business_cycle<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    args: &Args,
    request: ModelServiceBusinessCycleRequest,
) -> std::io::Result<ModelServiceBusinessCycleReport> {
    run_model_service_business_cycle_observed(engine, backend, args, request, &mut |_| {})
}

pub(crate) fn run_model_service_business_cycle_observed<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    args: &Args,
    request: ModelServiceBusinessCycleRequest,
    observer: &mut dyn FnMut(ModelServiceBusinessCycleEvent<'_>),
) -> std::io::Result<ModelServiceBusinessCycleReport> {
    let mut never_cancel = || false;
    run_model_service_business_cycle_observed_cancelable(
        engine,
        backend,
        args,
        request,
        observer,
        &mut never_cancel,
    )
}

pub(crate) fn run_model_service_business_cycle_observed_cancelable<B: InferenceBackend>(
    engine: &mut NoironEngine,
    backend: &mut B,
    args: &Args,
    request: ModelServiceBusinessCycleRequest,
    observer: &mut dyn FnMut(ModelServiceBusinessCycleEvent<'_>),
    should_cancel: &mut dyn FnMut() -> bool,
) -> std::io::Result<ModelServiceBusinessCycleReport> {
    let profile = request
        .profile
        .unwrap_or_else(|| detect_profile(&request.prompt));
    let case_name = request.case_name.clone();
    let pool_dispatch = request.pool_dispatch.clone();
    let pool_stage_dispatch = request.pool_stage_dispatch.clone();
    let max_tokens = pool_dispatch
        .as_ref()
        .and_then(|dispatch| dispatch.effective_max_tokens)
        .or(request.max_tokens);
    let pool_dispatch_forwarded = match pool_dispatch
        .as_ref()
        .and_then(|dispatch| dispatch.selected_base_url.as_deref())
    {
        Some(endpoint) => backend
            .configure_runtime_endpoint_override(Some(endpoint))
            .map_err(|error| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("pool_dispatch selected_base_url rejected: {error}"),
                )
            })?,
        None => false,
    };
    if let Some(dispatch) = &pool_dispatch {
        observer(ModelServiceBusinessCycleEvent::Meta(
            dispatch.summary(pool_dispatch_forwarded),
        ));
    }
    for dispatch in &pool_stage_dispatch {
        observer(ModelServiceBusinessCycleEvent::Meta(dispatch.summary()));
    }
    observer(ModelServiceBusinessCycleEvent::Stage("generate:start"));
    let mut stream_cancel_requested = false;
    let timed = run_timed_inference_stream_checked_with_options(
        engine,
        backend,
        request.prompt,
        profile,
        max_tokens,
        args.trace_path.as_ref(),
        case_name.as_deref(),
        &mut |token| {
            if stream_cancel_requested {
                return Err(business_cycle_cancel_error());
            }
            if should_cancel() {
                stream_cancel_requested = true;
                return Err(business_cycle_cancel_error());
            }
            observer(ModelServiceBusinessCycleEvent::Token(token));
            Ok(())
        },
    );
    if pool_dispatch_forwarded {
        let _ = backend.configure_runtime_endpoint_override(None);
    }
    if stream_cancel_requested {
        return Err(business_cycle_cancel_error());
    }
    check_business_cycle_cancel(should_cancel)?;
    let mut timed = timed?;
    observer(ModelServiceBusinessCycleEvent::Meta(format!(
        "generate elapsed_ms={} runtime_tokens={} experience_id={}",
        timed.elapsed_ms,
        timed.outcome.runtime_token_metrics.token_count,
        timed.outcome.experience_id
    )));
    annotate_model_service_business_case_for_timed(
        engine,
        &mut timed,
        case_name.as_deref(),
        args.trace_path.as_ref(),
    )?;
    annotate_model_service_pool_dispatch_experience(
        engine,
        timed.outcome.experience_id,
        pool_dispatch.as_ref(),
        pool_dispatch_forwarded,
    );
    annotate_model_service_pool_stage_dispatch_experience(
        engine,
        timed.outcome.experience_id,
        &pool_stage_dispatch,
    );
    observer(ModelServiceBusinessCycleEvent::Stage("generate:done"));

    check_business_cycle_cancel(should_cancel)?;
    observer(ModelServiceBusinessCycleEvent::Stage("feedback:start"));
    let feedback_request = ModelServiceFeedbackRequest {
        action: request.feedback_action,
        amount: request.feedback_amount,
        experience_id: Some(timed.outcome.experience_id),
        memory_id: None,
    };
    let feedback_memory_ids = model_service_feedback_memory_ids(engine, &feedback_request);
    if feedback_memory_ids.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "business_cycle feedback found no generated memory ids",
        ));
    }
    let feedback_updates =
        apply_model_service_feedback(engine, &feedback_request, &feedback_memory_ids);
    engine
        .evolution_ledger
        .record_external_feedback(&feedback_updates);
    annotate_model_service_feedback_experience_with_source(
        engine,
        &feedback_request,
        &feedback_updates,
        "business_cycle",
    );
    observer(ModelServiceBusinessCycleEvent::Meta(format!(
        "feedback memory_ids={} updates={}",
        feedback_memory_ids.len(),
        feedback_updates.len()
    )));
    observer(ModelServiceBusinessCycleEvent::Stage("feedback:done"));

    check_business_cycle_cancel(should_cancel)?;
    let mut rust_check_request = None;
    let mut rust_check_report = None;
    let mut rust_check_feedback_request = None;
    let mut rust_check_memory_ids = Vec::new();
    let mut rust_check_updates = Vec::new();
    if let Some(code) = request.rust_check_code {
        observer(ModelServiceBusinessCycleEvent::Stage("rust_check:start"));
        let rust_request = ModelServiceRustCheckRequest {
            code,
            edition: request.rust_check_edition,
            case_name: request.rust_check_case_name.or_else(|| {
                case_name
                    .as_ref()
                    .map(|case_name| format!("{case_name}-compiler-feedback"))
            }),
            amount: None,
            experience_id: Some(timed.outcome.experience_id),
            memory_id: None,
        };
        let check_report = model_service_rust_check_report(
            &rust_request,
            "model-service-business-cycle-rust-check",
        )?;
        let rust_feedback_request =
            model_service_rust_check_feedback_request(&rust_request, &check_report);
        rust_check_memory_ids = model_service_feedback_memory_ids(engine, &rust_feedback_request);
        if rust_check_memory_ids.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "business_cycle rust_check feedback found no generated memory ids",
            ));
        }
        rust_check_updates =
            apply_model_service_feedback(engine, &rust_feedback_request, &rust_check_memory_ids);
        engine
            .evolution_ledger
            .record_external_feedback(&rust_check_updates);
        annotate_model_service_feedback_experience_with_source(
            engine,
            &rust_feedback_request,
            &rust_check_updates,
            "business_cycle_rust_check",
        );
        annotate_model_service_rust_check_experience(engine, &rust_request, &check_report);
        if let Some(trace_path) = &args.trace_path {
            append_rust_check_trace_jsonl(
                trace_path,
                rust_request.case_name.as_deref(),
                &check_report,
                rust_feedback_request.action,
                rust_feedback_request.amount,
                rust_request.experience_id,
                rust_request.memory_id,
                &rust_check_memory_ids,
                &rust_check_updates,
            )?;
        }
        rust_check_feedback_request = Some(rust_feedback_request);
        rust_check_report = Some(check_report);
        rust_check_request = Some(rust_request);
        observer(ModelServiceBusinessCycleEvent::Meta(format!(
            "rust_check memory_ids={} updates={}",
            rust_check_memory_ids.len(),
            rust_check_updates.len()
        )));
        observer(ModelServiceBusinessCycleEvent::Stage("rust_check:done"));
    } else {
        observer(ModelServiceBusinessCycleEvent::Stage("rust_check:skipped"));
    }

    let replay_report = if request.self_improve {
        check_business_cycle_cancel(should_cancel)?;
        observer(ModelServiceBusinessCycleEvent::Stage("self_improve:start"));
        let report = engine.replay_experience(request.self_improve_limit);
        observer(ModelServiceBusinessCycleEvent::Meta(format!(
            "self_improve applied={} limit={}",
            report.applied, request.self_improve_limit
        )));
        observer(ModelServiceBusinessCycleEvent::Stage("self_improve:done"));
        Some(report)
    } else {
        observer(ModelServiceBusinessCycleEvent::Stage(
            "self_improve:skipped",
        ));
        None
    };
    check_business_cycle_cancel(should_cancel)?;
    observer(ModelServiceBusinessCycleEvent::Stage("save_state:start"));
    engine.save_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    observer(ModelServiceBusinessCycleEvent::Stage("save_state:done"));
    check_business_cycle_cancel(should_cancel)?;
    observer(ModelServiceBusinessCycleEvent::Stage("gates:start"));
    observer(ModelServiceBusinessCycleEvent::Stage(
        "gates:inspection:start",
    ));
    let full_experience_scan =
        model_service_state_gate_requires_full_experience_scan(&request.inspect, args);
    let inspection_mode = if full_experience_scan {
        "full"
    } else {
        "online"
    };
    observer(ModelServiceBusinessCycleEvent::Meta(format!(
        "gates inspection mode={} limit={}",
        inspection_mode, args.inspect_limit
    )));
    let inspection = if full_experience_scan {
        StateInspectionReport::from_engine(engine, args.inspect_limit)
    } else {
        StateInspectionReport::from_engine_online(engine, args.inspect_limit)
    };
    observer(ModelServiceBusinessCycleEvent::Stage(
        "gates:inspection:done",
    ));
    observer(ModelServiceBusinessCycleEvent::Stage("gates:state:start"));
    let state_gate_report =
        model_service_state_gate_report_for_request(&request.inspect, &inspection, args);
    observer(ModelServiceBusinessCycleEvent::Stage("gates:state:done"));
    observer(ModelServiceBusinessCycleEvent::Stage("gates:trace:start"));
    let trace_gate_report = model_service_trace_gate_report_for_request(&request.inspect, args)?;
    observer(ModelServiceBusinessCycleEvent::Stage("gates:trace:done"));
    observer(ModelServiceBusinessCycleEvent::Stage("gates:done"));

    Ok(ModelServiceBusinessCycleReport {
        profile,
        traceable: args.trace_path.is_some(),
        pool_dispatch,
        pool_stage_dispatch,
        pool_dispatch_forwarded,
        timed,
        feedback_request,
        feedback_memory_ids,
        feedback_updates,
        rust_check_request,
        rust_check_report,
        rust_check_feedback_request,
        rust_check_memory_ids,
        rust_check_updates,
        self_improve_enabled: request.self_improve,
        self_improve_limit: request.self_improve_limit,
        replay_report,
        inspection,
        state_gate_report,
        trace_gate_report,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use rust_norion::{
        DraftToken, GenerationContext, InferenceDraft, ReasoningStep, RewardAction, TaskProfile,
    };

    use super::super::request::{
        ModelServiceInspectRequest, ModelServicePoolDispatchRequest,
        ModelServicePoolStageDispatchRequest,
    };
    use super::*;

    #[derive(Debug, Default)]
    struct AcceptingEndpointBackend {
        override_calls: usize,
        clear_calls: usize,
        generate_calls: usize,
        configured_generations: Vec<Option<usize>>,
        active_endpoint: Option<String>,
    }

    impl InferenceBackend for AcceptingEndpointBackend {
        fn configure_generation(&mut self, max_tokens: Option<usize>) {
            self.configured_generations.push(max_tokens);
        }

        fn configure_runtime_endpoint_override(
            &mut self,
            base_url: Option<&str>,
        ) -> Result<bool, String> {
            match base_url {
                Some(endpoint) => {
                    self.override_calls += 1;
                    self.active_endpoint = Some(endpoint.to_owned());
                    Ok(true)
                }
                None => {
                    self.clear_calls += 1;
                    self.active_endpoint = None;
                    Ok(false)
                }
            }
        }

        fn runtime_endpoint_override_active(&self) -> Option<&str> {
            self.active_endpoint.as_deref()
        }

        fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft {
            self.generate_calls += 1;
            let answer = format!(
                "Review this Rust model pool dispatch plan for Smart Steam by keeping the selected \
                 model role explicit, preserving the effective token budget, and recording whether \
                 the runtime endpoint override forwarded the request. Prompt grounding: {}",
                context.prompt
            );
            InferenceDraft::new(
                answer,
                vec![ReasoningStep::new(
                    "pool_dispatch_contract",
                    "selected model role, token budget, and forwarded state were checked",
                    0.92,
                )],
            )
            .with_tokens(vec![
                DraftToken::new("Review "),
                DraftToken::new("Rust "),
                DraftToken::new("model "),
                DraftToken::new("pool "),
            ])
        }
    }

    #[derive(Debug, Default)]
    struct RejectingEndpointBackend {
        override_calls: usize,
        generate_calls: usize,
    }

    impl InferenceBackend for RejectingEndpointBackend {
        fn configure_runtime_endpoint_override(
            &mut self,
            base_url: Option<&str>,
        ) -> Result<bool, String> {
            self.override_calls += 1;
            match base_url {
                Some(_) => Err("bad endpoint".to_owned()),
                None => Ok(false),
            }
        }

        fn generate(&mut self, _context: GenerationContext<'_>) -> InferenceDraft {
            self.generate_calls += 1;
            panic!("business-cycle should reject bad pool endpoint before generate")
        }
    }

    #[test]
    fn business_cycle_rejects_bad_pool_endpoint_before_generation() {
        let mut engine = NoironEngine::new();
        let mut backend = RejectingEndpointBackend::default();
        let args = Args::parse(Vec::new());
        let request = ModelServiceBusinessCycleRequest {
            prompt: "route this review task".to_owned(),
            profile: Some(TaskProfile::Coding),
            case_name: None,
            max_tokens: Some(4096),
            feedback_action: RewardAction::Reinforce,
            feedback_amount: 0.5,
            rust_check_code: None,
            rust_check_edition: "2021".to_owned(),
            rust_check_case_name: None,
            self_improve: false,
            self_improve_limit: 1,
            pool_dispatch: Some(ModelServicePoolDispatchRequest {
                selected_role: "review".to_owned(),
                selected_port: Some(8688),
                selected_base_url: Some("https://127.0.0.1:8688".to_owned()),
                context_window: Some(8192),
                default_max_tokens: Some(128),
                runtime_backend: None,
                runtime_device: None,
                runtime_accelerator: None,
                gpu_layers: None,
                configured_max_tokens: Some(4096),
                effective_max_tokens: Some(128),
                max_tokens_clamped: true,
                max_tokens_clamp_reason: Some("low_priority_worker_default_max_tokens".to_owned()),
                can_accept_low_priority_task: false,
            }),
            pool_stage_dispatch: Vec::new(),
            inspect: ModelServiceInspectRequest::default(),
        };
        let mut events = Vec::new();

        let result = run_model_service_business_cycle_observed(
            &mut engine,
            &mut backend,
            &args,
            request,
            &mut |event| match event {
                ModelServiceBusinessCycleEvent::Stage(stage) => {
                    events.push(format!("stage:{stage}"))
                }
                ModelServiceBusinessCycleEvent::Meta(meta) => events.push(format!("meta:{meta}")),
                ModelServiceBusinessCycleEvent::Token(token) => {
                    events.push(format!("token:{}", token.text))
                }
            },
        );
        let error = match result {
            Ok(_) => panic!("business-cycle unexpectedly accepted a bad pool endpoint"),
            Err(error) => error,
        };

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            error
                .to_string()
                .contains("pool_dispatch selected_base_url rejected")
        );
        assert_eq!(backend.override_calls, 1);
        assert_eq!(backend.generate_calls, 0);
        assert!(events.is_empty());
    }

    #[test]
    fn business_cycle_records_pool_dispatch_note_on_generated_experience() {
        let mut engine = NoironEngine::new();
        let mut backend = AcceptingEndpointBackend::default();
        let args = temp_state_args("business-cycle-pool-dispatch-note");
        let request = ModelServiceBusinessCycleRequest {
            prompt: "review this Rust model pool dispatch plan for Smart Steam".to_owned(),
            profile: Some(TaskProfile::Coding),
            case_name: Some("pool-dispatch-note".to_owned()),
            max_tokens: Some(4096),
            feedback_action: RewardAction::Reinforce,
            feedback_amount: 0.5,
            rust_check_code: None,
            rust_check_edition: "2021".to_owned(),
            rust_check_case_name: None,
            self_improve: false,
            self_improve_limit: 1,
            pool_dispatch: Some(ModelServicePoolDispatchRequest {
                selected_role: "review".to_owned(),
                selected_port: Some(8688),
                selected_base_url: Some("http://127.0.0.1:8688".to_owned()),
                context_window: Some(8192),
                default_max_tokens: Some(1024),
                runtime_backend: Some("llama.cpp".to_owned()),
                runtime_device: Some("metal".to_owned()),
                runtime_accelerator: Some("metal".to_owned()),
                gpu_layers: Some(99),
                configured_max_tokens: Some(4096),
                effective_max_tokens: Some(768),
                max_tokens_clamped: true,
                max_tokens_clamp_reason: Some("low_priority_worker_default_max_tokens".to_owned()),
                can_accept_low_priority_task: true,
            }),
            pool_stage_dispatch: Vec::new(),
            inspect: ModelServiceInspectRequest::default(),
        };

        let report = run_model_service_business_cycle_observed(
            &mut engine,
            &mut backend,
            &args,
            request,
            &mut |_| {},
        )
        .unwrap();

        assert!(report.pool_dispatch_forwarded);
        assert_eq!(backend.override_calls, 1);
        assert_eq!(backend.clear_calls, 1);
        assert_eq!(backend.generate_calls, 1);
        assert_eq!(backend.configured_generations, vec![Some(768)]);
        let record = engine
            .experience
            .records()
            .iter()
            .find(|record| record.id == report.timed.outcome.experience_id)
            .unwrap();
        let pool_note = record
            .process_reward
            .notes
            .iter()
            .find(|note| note.starts_with("pool_dispatch:"))
            .expect("pool dispatch note should be recorded");
        assert!(pool_note.contains("selected_role=review"));
        assert!(pool_note.contains("selected_port=8688"));
        assert!(pool_note.contains("selected_endpoint=http://127.0.0.1:8688"));
        assert!(pool_note.contains("effective_max_tokens=768"));
        assert!(pool_note.contains("max_tokens_clamped=true"));
        assert!(
            pool_note.contains("max_tokens_clamp_reason=low_priority_worker_default_max_tokens")
        );
        assert!(pool_note.contains("low_priority=true"));
        assert!(pool_note.contains("forwarded=true"));
        assert!(pool_note.contains("dispatch_mode=runtime_endpoint_override"));
        assert!(pool_note.contains("dispatch_reason=runtime_endpoint_override_active"));
    }

    #[test]
    fn business_cycle_records_pool_stage_dispatch_without_extra_generation() {
        let mut engine = NoironEngine::new();
        let mut backend = AcceptingEndpointBackend::default();
        let args = temp_state_args("business-cycle-pool-stage-dispatch-note");
        let request = ModelServiceBusinessCycleRequest {
            prompt: "coordinate summary and test-gate helpers for Smart Steam".to_owned(),
            profile: Some(TaskProfile::Coding),
            case_name: Some("pool-stage-dispatch-note".to_owned()),
            max_tokens: Some(2048),
            feedback_action: RewardAction::Reinforce,
            feedback_amount: 0.5,
            rust_check_code: None,
            rust_check_edition: "2021".to_owned(),
            rust_check_case_name: None,
            self_improve: false,
            self_improve_limit: 1,
            pool_dispatch: None,
            pool_stage_dispatch: vec![ModelServicePoolStageDispatchRequest {
                task_kind: "summary".to_owned(),
                selected_role: "summary".to_owned(),
                selected_port: Some(8687),
                selected_base_url: Some("http://127.0.0.1:8687".to_owned()),
                context_window: Some(8192),
                default_max_tokens: Some(768),
                runtime_backend: Some("llama.cpp".to_owned()),
                runtime_device: Some("metal".to_owned()),
                runtime_accelerator: Some("metal".to_owned()),
                gpu_layers: Some(99),
                configured_max_tokens: Some(4096),
                effective_max_tokens: Some(768),
                max_tokens_clamped: true,
                max_tokens_clamp_reason: Some("low_priority_worker_default_max_tokens".to_owned()),
                can_accept_low_priority_task: true,
            }],
            inspect: ModelServiceInspectRequest::default(),
        };
        let mut events = Vec::new();

        let report = run_model_service_business_cycle_observed(
            &mut engine,
            &mut backend,
            &args,
            request,
            &mut |event| match event {
                ModelServiceBusinessCycleEvent::Stage(stage) => {
                    events.push(format!("stage:{stage}"))
                }
                ModelServiceBusinessCycleEvent::Meta(meta) => events.push(format!("meta:{meta}")),
                ModelServiceBusinessCycleEvent::Token(token) => {
                    events.push(format!("token:{}", token.text))
                }
            },
        )
        .unwrap();

        assert_eq!(backend.generate_calls, 1);
        assert_eq!(backend.override_calls, 0);
        assert_eq!(report.pool_stage_dispatch.len(), 1);
        assert!(
            events
                .iter()
                .any(|event| event.contains("pool_stage_dispatch task_kind=summary"))
        );
        assert!(events.contains(&"stage:gates:inspection:start".to_owned()));
        assert!(events.contains(&"stage:gates:inspection:done".to_owned()));
        assert!(events.contains(&"stage:gates:state:start".to_owned()));
        assert!(events.contains(&"stage:gates:state:done".to_owned()));
        assert!(
            events
                .iter()
                .any(|event| event.starts_with("meta:gates inspection mode=online limit="))
        );
        assert_eq!(
            report.inspection.experience_index_risk_level,
            "online_deferred"
        );
        let record = engine
            .experience
            .records()
            .iter()
            .find(|record| record.id == report.timed.outcome.experience_id)
            .unwrap();
        let stage_note = record
            .process_reward
            .notes
            .iter()
            .find(|note| note.starts_with("pool_stage_dispatch:"))
            .expect("pool stage dispatch note should be recorded");
        assert!(stage_note.contains("task_kind=summary"));
        assert!(stage_note.contains("selected_role=summary"));
        assert!(stage_note.contains("dispatch_mode=stage_plan_only"));
        assert!(stage_note.contains("dispatch_reason=stage_dispatch_observed"));
    }

    fn temp_state_args(case_name: &str) -> Args {
        let mut args = Args::parse(Vec::new());
        let dir = temp_state_dir(case_name);
        fs::create_dir_all(&dir).unwrap();
        args.memory_path = dir.join("memory.nkv");
        args.experience_path = dir.join("experience.nkv");
        args.adaptive_path = dir.join("adaptive.nkv");
        args
    }

    fn temp_state_dir(case_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-norion-{case_name}-{}-{nanos}",
            std::process::id()
        ))
    }
}
