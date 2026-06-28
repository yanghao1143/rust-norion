use rust_norion::{
    append_rust_check_trace_jsonl, DraftToken, InferenceBackend, NoironEngine,
    StateInspectionReport,
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
use crate::cli::state::runtime_state_bucket;
use crate::gemma_business::contract::annotate_model_service_business_case_for_timed_to_paths;
use crate::inference_runner::{
    inference_trace_output_paths_for_args, persist_self_evolving_writeback_note_for_args,
    record_self_evolving_experience_for_args, record_self_evolving_experience_trace_for_args,
    run_timed_inference_stream_checked_with_external_experience_hints_to_trace_paths,
    self_evolving_experience_hints_for_args,
};
use crate::Args;

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
    let runtime_state_failures = runtime_state_bucket(args).blocking_failures();
    if !runtime_state_failures.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            runtime_state_failures.join("; "),
        ));
    }

    let profile = request
        .profile
        .unwrap_or_else(|| detect_profile(&request.prompt));
    let case_name = request.case_name.clone();
    let pool_dispatch = request.pool_dispatch.clone();
    let pool_stage_dispatch = request.pool_stage_dispatch.clone();
    let prompt = request.prompt.clone();
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
    let external_experience_hints =
        self_evolving_experience_hints_for_args(args, &request.prompt, profile)?;
    observer(ModelServiceBusinessCycleEvent::Stage("generate:start"));
    let mut stream_cancel_requested = false;
    let timed = run_timed_inference_stream_checked_with_external_experience_hints_to_trace_paths(
        engine,
        backend,
        request.prompt,
        profile,
        max_tokens,
        external_experience_hints,
        inference_trace_output_paths_for_args(args),
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
    annotate_model_service_business_case_for_timed_to_paths(
        engine,
        &mut timed,
        case_name.as_deref(),
        inference_trace_output_paths_for_args(args),
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
        for trace_path in inference_trace_output_paths_for_args(args)
            .into_iter()
            .flatten()
        {
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
    let sem_writeback = record_self_evolving_experience_for_args(
        args,
        &prompt,
        profile,
        &timed.outcome,
        "model_service_business_cycle",
    )?;
    persist_self_evolving_writeback_note_for_args(engine, args, &sem_writeback)?;
    record_self_evolving_experience_trace_for_args(args, &sem_writeback)?;
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
        last_external_hints: Vec<String>,
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
            self.last_external_hints = context.external_experience_hints.to_vec();
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
        assert!(error
            .to_string()
            .contains("pool_dispatch selected_base_url rejected"));
        assert_eq!(backend.override_calls, 1);
        assert_eq!(backend.generate_calls, 0);
        assert!(events.is_empty());
    }

    #[test]
    fn business_cycle_rejects_runtime_state_bucket_mismatch_before_generation() {
        let mut engine = NoironEngine::new();
        let mut backend = RejectingEndpointBackend::default();
        let args = Args::parse(vec![
            "--memory".to_owned(),
            "legacy-memory.ndkv".to_owned(),
            "--experience".to_owned(),
            "legacy-experience.ndkv".to_owned(),
            "--adaptive".to_owned(),
            "legacy-adaptive.ndkv".to_owned(),
        ]);
        let request = ModelServiceBusinessCycleRequest {
            prompt: "do not run on dirty state".to_owned(),
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
            pool_dispatch: None,
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
            Ok(_) => panic!("business-cycle unexpectedly accepted dirty runtime state"),
            Err(error) => error,
        };

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(error
            .to_string()
            .contains("outside the current version bucket"));
        assert_eq!(backend.override_calls, 0);
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
        assert!(events
            .iter()
            .any(|event| event.contains("pool_stage_dispatch task_kind=summary")));
        assert!(events.contains(&"stage:gates:inspection:start".to_owned()));
        assert!(events.contains(&"stage:gates:inspection:done".to_owned()));
        assert!(events.contains(&"stage:gates:state:start".to_owned()));
        assert!(events.contains(&"stage:gates:state:done".to_owned()));
        assert!(events
            .iter()
            .any(|event| event.starts_with("meta:gates inspection mode=online limit=")));
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

    #[test]
    fn business_cycle_stream_generation_loads_self_evolving_snapshot_hints() {
        let mut engine = NoironEngine::new();
        let mut backend = AcceptingEndpointBackend::default();
        let mut args = temp_state_args("business-cycle-sem-hints");
        args.trace_path = Some(args.experience_path.with_extension("trace.jsonl"));
        let sem_path = args
            .experience_path
            .with_extension("self-evolving-memory.tsv");
        let approval = rust_norion::SelfEvolvingMemoryApproval::approved(
            "rollback:business-cycle-sem".to_owned(),
            vec!["business-cycle-sem-test".to_owned()],
        );
        let mut store = rust_norion::SelfEvolvingMemoryStore::new();
        store.append_episode(
            rust_norion::SelfEvolvingEpisodeInput {
                problem: "private business-cycle sem prompt".to_owned(),
                solution_path: "streaming service hint reuse".to_owned(),
                outcome: "positive streaming reuse".to_owned(),
                key_insights: vec!["business-cycle hint enters stream context".to_owned()],
                tags: vec!["runtime".to_owned()],
                profile: TaskProfile::Coding,
                quality: 0.91,
                token_estimate: 8,
                source_case_id: "case:business-cycle-sem".to_owned(),
            },
            &approval,
        );
        store.append_heuristic(
            rust_norion::SelfEvolvingHeuristicInput {
                rule: "prefer digest SEM hints before business-cycle streaming compute".to_owned(),
                tags: vec!["runtime".to_owned()],
                profile: TaskProfile::Coding,
                priority: 0.82,
                confidence: 0.84,
                source_case_id: "case:business-cycle-sem".to_owned(),
                updated_step: 1,
            },
            &approval,
        );
        store.observe_tool(
            rust_norion::ToolReliabilityObservationInput {
                tool_name: "model_service_business_cycle".to_owned(),
                profile: TaskProfile::Coding,
                success: true,
                quality: 0.88,
                source_case_id: "case:business-cycle-sem".to_owned(),
                observed_step: 1,
            },
            &approval,
        );
        let sem_before_digest = store.snapshot_digest();
        store.save_snapshot(&sem_path).unwrap();
        let sem_snapshot = fs::read_to_string(&sem_path).unwrap();
        assert!(!sem_snapshot.contains("private business-cycle sem prompt"));

        let request = ModelServiceBusinessCycleRequest {
            prompt: "review streaming SEM reuse for Smart Steam".to_owned(),
            profile: Some(TaskProfile::Coding),
            case_name: Some("business-cycle-sem-hints".to_owned()),
            max_tokens: Some(128),
            feedback_action: RewardAction::Reinforce,
            feedback_amount: 0.5,
            rust_check_code: None,
            rust_check_edition: "2021".to_owned(),
            rust_check_case_name: None,
            self_improve: false,
            self_improve_limit: 1,
            pool_dispatch: None,
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

        assert_eq!(backend.generate_calls, 1);
        assert_eq!(backend.last_external_hints.len(), 3);
        assert!(backend
            .last_external_hints
            .iter()
            .all(|hint| !hint.contains("private business-cycle sem prompt")));
        assert!(report.timed.outcome.experience_id > 0);
        let sem_after = rust_norion::SelfEvolvingMemoryStore::load_snapshot(&sem_path).unwrap();
        let trace_path = args.trace_path.as_ref().unwrap();
        let trace_report = rust_norion::evaluate_trace_schema_jsonl(trace_path).unwrap();
        let trace = fs::read_to_string(trace_path).unwrap();
        let writeback_line = trace
            .lines()
            .find(|line| line.contains("rust-norion-self-evolving-memory-writeback-v1"))
            .unwrap();
        assert!(trace_report.passed, "{:?}", trace_report.failures);
        assert_eq!(trace_report.self_evolving_memory_writeback_events, 1);
        assert_eq!(
            trace_report.self_evolving_memory_writeback_applied_to_disk,
            1
        );
        assert!(!writeback_line.contains("private business-cycle sem prompt"));
        assert!(writeback_line.contains("\"tool\":\"model_service_business_cycle\""));
        assert!(writeback_line.contains(&format!(
            "\"snapshot_before_digest\":\"{sem_before_digest}\""
        )));
        assert!(writeback_line.contains(&format!(
            "\"snapshot_digest\":\"{}\"",
            sem_after.snapshot_digest()
        )));
        assert!(writeback_line.contains(&format!(
            "\"disk_snapshot_digest\":\"{}\"",
            sem_after.snapshot_digest()
        )));
        assert_ne!(sem_before_digest, sem_after.snapshot_digest());
        assert_eq!(sem_after.episodes().len(), 2);
        assert_eq!(sem_after.heuristics().len(), 2);
        assert_eq!(sem_after.tool_reliability().len(), 1);
        assert_eq!(sem_after.tool_observations().len(), 2);
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
