use std::collections::BTreeMap;

use norion_agent::AgentModelRouteProof;
use norion_eval::{
    CODING_EVAL_SCHEMA_VERSION, CodingEvalCorpus, CodingEvalFixture, CodingEvalObservation,
    CodingEvalProfileKind, CodingEvalSuiteReport, CodingEvalThresholds,
    coding_eval_corpus_from_fixture_tsv, default_coding_eval_corpus, sample_passing_observations,
};
use norion_service::{
    ChatChunkKind, ChatMessage, ChatRequest, ChatSession, ChatSessionConfig, ModelEndpoint,
    ModelRole, RoutingPreference, StreamState, request_json,
};

use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};

pub const CODING_SERVICE_EVAL_SCHEMA_VERSION: &str = "coding_service_eval_v1";
pub const CODING_SERVICE_EVAL_TRACE_SCHEMA: &str = "rust-norion-coding-service-eval-readiness-v1";
pub const CODING_SERVICE_EVAL_RUNNER_SCHEMA_VERSION: &str = "coding_service_eval_runner_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CodingServiceEvalCapability {
    OpenAiChatRequest,
    Streaming,
    Cancellation,
    MaxTokens,
    Diagnostics,
    Health,
    ModelCapabilities,
    OfflineMockBackend,
    EvaluationEvidence,
    RustValidation,
}

impl CodingServiceEvalCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAiChatRequest => "openai_chat_request",
            Self::Streaming => "streaming",
            Self::Cancellation => "cancellation",
            Self::MaxTokens => "max_tokens",
            Self::Diagnostics => "diagnostics",
            Self::Health => "health",
            Self::ModelCapabilities => "model_capabilities",
            Self::OfflineMockBackend => "offline_mock_backend",
            Self::EvaluationEvidence => "evaluation_evidence",
            Self::RustValidation => "rust_validation",
        }
    }

    pub fn expected() -> [Self; 10] {
        [
            Self::OpenAiChatRequest,
            Self::Streaming,
            Self::Cancellation,
            Self::MaxTokens,
            Self::Diagnostics,
            Self::Health,
            Self::ModelCapabilities,
            Self::OfflineMockBackend,
            Self::EvaluationEvidence,
            Self::RustValidation,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CodingServiceEvalLanguage {
    English,
    Chinese,
    Rust,
    MixedEnglishChinese,
}

impl CodingServiceEvalLanguage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::English => "english",
            Self::Chinese => "chinese",
            Self::Rust => "rust",
            Self::MixedEnglishChinese => "mixed_english_chinese",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodingServiceEvalRequestPlan {
    pub fixture_id: String,
    pub profile: CodingEvalProfileKind,
    pub language: CodingServiceEvalLanguage,
    pub request: ChatRequest,
    pub requires_streaming: bool,
    pub requires_cancellation_probe: bool,
    pub requires_diagnostics: bool,
    pub requires_health: bool,
    pub requires_model_capabilities: bool,
    pub requires_offline_mock_backend: bool,
    pub requires_rust_validation: bool,
}

impl CodingServiceEvalRequestPlan {
    pub fn prompt_digest(&self) -> String {
        let prompt = self
            .request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        stable_redaction_digest(["coding-service-eval-prompt", &self.fixture_id, &prompt])
    }

    pub fn capability_labels(&self) -> Vec<&'static str> {
        let mut labels = vec![
            CodingServiceEvalCapability::OpenAiChatRequest.as_str(),
            CodingServiceEvalCapability::EvaluationEvidence.as_str(),
        ];
        if self.requires_streaming {
            labels.push(CodingServiceEvalCapability::Streaming.as_str());
        }
        if self.requires_cancellation_probe {
            labels.push(CodingServiceEvalCapability::Cancellation.as_str());
        }
        if self.request.max_tokens.is_some() {
            labels.push(CodingServiceEvalCapability::MaxTokens.as_str());
        }
        if self.requires_diagnostics {
            labels.push(CodingServiceEvalCapability::Diagnostics.as_str());
        }
        if self.requires_health {
            labels.push(CodingServiceEvalCapability::Health.as_str());
        }
        if self.requires_model_capabilities {
            labels.push(CodingServiceEvalCapability::ModelCapabilities.as_str());
        }
        if self.requires_offline_mock_backend {
            labels.push(CodingServiceEvalCapability::OfflineMockBackend.as_str());
        }
        if self.requires_rust_validation {
            labels.push(CodingServiceEvalCapability::RustValidation.as_str());
        }
        labels.sort();
        labels.dedup();
        labels
    }

    pub fn evidence_packet_line(&self) -> String {
        let wire = self.request.wire_snapshot();
        [
            CODING_SERVICE_EVAL_SCHEMA_VERSION.to_owned(),
            self.fixture_id.clone(),
            self.profile.as_str().to_owned(),
            self.language.as_str().to_owned(),
            self.prompt_digest(),
            wire.model_role_label,
            wire.routing_preference_label,
            wire.endpoint_kind_label,
            wire.model_endpoint_label
                .unwrap_or_else(|| "auto".to_owned()),
            wire.max_tokens
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.requires_streaming.to_string(),
            self.requires_cancellation_probe.to_string(),
            self.requires_diagnostics.to_string(),
            self.requires_health.to_string(),
            self.requires_model_capabilities.to_string(),
            self.requires_offline_mock_backend.to_string(),
            self.requires_rust_validation.to_string(),
            self.capability_labels().join("|"),
            stable_redaction_digest([
                "coding-service-eval-request",
                self.fixture_id.as_str(),
                self.profile.as_str(),
                self.language.as_str(),
            ]),
        ]
        .into_iter()
        .map(|field| escape_field(&field))
        .collect::<Vec<_>>()
        .join("\t")
    }

    pub fn request_wire_json(&self) -> String {
        request_json(&self.request)
    }

    pub fn evidence_is_redacted(&self) -> bool {
        let line = self.evidence_packet_line();
        self.prompt_digest().starts_with("redaction-digest:")
            && line.contains("redaction-digest:")
            && !contains_private_or_executable_marker(&line)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodingServiceEvalReadinessReport {
    pub schema_version: &'static str,
    pub trace_schema: &'static str,
    pub coding_eval_schema_version: &'static str,
    pub request_plan_count: usize,
    pub corpus_fixture_count: usize,
    pub corpus_validation_failures: Vec<String>,
    pub suite_report: CodingEvalSuiteReport,
    pub profile_counts: BTreeMap<String, usize>,
    pub language_counts: BTreeMap<String, usize>,
    pub capability_counts: BTreeMap<String, usize>,
    pub missing_capabilities: Vec<String>,
    pub request_evidence_packets: Vec<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodingServiceEvalRunnerConfig {
    pub history_limit: usize,
    pub offline_model_label: String,
    pub memory_hit_rate: f32,
    pub base_latency_ms: u64,
    pub per_message_latency_ms: u64,
}

impl Default for CodingServiceEvalRunnerConfig {
    fn default() -> Self {
        Self {
            history_limit: 16,
            offline_model_label: "norion-offline-mock-coding-service".to_owned(),
            memory_hit_rate: 0.72,
            base_latency_ms: 700,
            per_message_latency_ms: 75,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodingServiceEvalRunRecord {
    pub fixture_id: String,
    pub profile: CodingEvalProfileKind,
    pub language: CodingServiceEvalLanguage,
    pub model_label: String,
    pub final_state: StreamState,
    pub final_state_label: String,
    pub stream_chunk_count: usize,
    pub delta_chunk_count: usize,
    pub status_chunk_count: usize,
    pub metadata_chunk_count: usize,
    pub final_chunk_count: usize,
    pub done_chunk_count: usize,
    pub cancellation_probed: bool,
    pub cancellation_passed: bool,
    pub cancellation_state: Option<StreamState>,
    pub cancellation_state_label: Option<String>,
    pub cancellation_partial_chars: usize,
    pub diagnostics_seen: bool,
    pub health_seen: bool,
    pub model_capabilities_seen: bool,
    pub max_tokens_respected: bool,
    pub rust_validation_checked: bool,
    pub compile_checked: bool,
    pub unit_test_checked: bool,
    pub benchmark_checked: bool,
    pub benchmark_passed: bool,
    pub layer_b_route_proof_ready: bool,
    pub rust_validation_layer_b_route_ready: bool,
    pub observation: CodingEvalObservation,
    pub evidence_packet_line: String,
}

impl CodingServiceEvalRunRecord {
    pub fn passed_runner_contract(&self) -> bool {
        self.final_state == StreamState::Completed
            && self.delta_chunk_count > 0
            && self.final_chunk_count > 0
            && self.done_chunk_count > 0
            && (!self.cancellation_probed || self.cancellation_passed)
            && self.diagnostics_seen
            && self.health_seen
            && self.model_capabilities_seen
            && self.max_tokens_respected
            && self.benchmark_checked
            && self.benchmark_passed
            && self.layer_b_route_proof_ready
            && (!self.rust_validation_checked || self.rust_validation_layer_b_route_ready)
            && self.evidence_is_redacted()
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.evidence_packet_line.contains("redaction-digest:")
            && !contains_private_or_executable_marker(&self.evidence_packet_line)
            && !self.evidence_packet_line.contains(&self.observation.output)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodingServiceEvalRunnerReport {
    pub schema_version: &'static str,
    pub trace_schema: &'static str,
    pub plan_count: usize,
    pub completed_count: usize,
    pub failed_runner_contract_count: usize,
    pub cancellation_probe_count: usize,
    pub cancellation_passed_count: usize,
    pub diagnostics_seen_count: usize,
    pub health_seen_count: usize,
    pub model_capabilities_seen_count: usize,
    pub max_tokens_respected_count: usize,
    pub rust_validation_checked_count: usize,
    pub benchmark_checked_count: usize,
    pub benchmark_passed_count: usize,
    pub layer_b_route_proof_ready_count: usize,
    pub rust_validation_layer_b_route_ready_count: usize,
    pub result_class_counts: BTreeMap<String, usize>,
    pub failure_class_counts: BTreeMap<String, usize>,
    pub suite_report: CodingEvalSuiteReport,
    pub run_records: Vec<CodingServiceEvalRunRecord>,
    pub evidence_packets: Vec<String>,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl CodingServiceEvalRunnerReport {
    pub fn from_corpus(corpus: &CodingEvalCorpus, config: &CodingServiceEvalRunnerConfig) -> Self {
        let plans = request_plans_from_corpus(corpus);
        let mut run_records = Vec::with_capacity(plans.len());

        for plan in &plans {
            if let Some(fixture) = corpus
                .fixtures
                .iter()
                .find(|fixture| fixture.id == plan.fixture_id)
            {
                run_records.push(run_plan(fixture, plan, config));
            }
        }

        let observations = run_records
            .iter()
            .map(|record| record.observation.clone())
            .collect::<Vec<_>>();
        let suite_report = corpus.score_observations(&observations);
        let completed_count = run_records
            .iter()
            .filter(|record| record.final_state == StreamState::Completed)
            .count();
        let failed_runner_contract_count = run_records
            .iter()
            .filter(|record| !record.passed_runner_contract())
            .count();
        let cancellation_probe_count = run_records
            .iter()
            .filter(|record| record.cancellation_probed)
            .count();
        let cancellation_passed_count = run_records
            .iter()
            .filter(|record| record.cancellation_passed)
            .count();
        let diagnostics_seen_count = run_records
            .iter()
            .filter(|record| record.diagnostics_seen)
            .count();
        let health_seen_count = run_records
            .iter()
            .filter(|record| record.health_seen)
            .count();
        let model_capabilities_seen_count = run_records
            .iter()
            .filter(|record| record.model_capabilities_seen)
            .count();
        let max_tokens_respected_count = run_records
            .iter()
            .filter(|record| record.max_tokens_respected)
            .count();
        let rust_validation_checked_count = run_records
            .iter()
            .filter(|record| record.rust_validation_checked)
            .count();
        let benchmark_checked_count = run_records
            .iter()
            .filter(|record| record.benchmark_checked)
            .count();
        let benchmark_passed_count = run_records
            .iter()
            .filter(|record| record.benchmark_passed)
            .count();
        let layer_b_route_proof_ready_count = run_records
            .iter()
            .filter(|record| record.layer_b_route_proof_ready)
            .count();
        let rust_validation_layer_b_route_ready_count = run_records
            .iter()
            .filter(|record| record.rust_validation_layer_b_route_ready)
            .count();
        let result_class_counts = result_class_counts(&suite_report, failed_runner_contract_count);
        let failure_class_counts = suite_report.failure_category_counts.clone();
        let evidence_packets = run_records
            .iter()
            .map(|record| record.evidence_packet_line.clone())
            .collect();

        Self {
            schema_version: CODING_SERVICE_EVAL_RUNNER_SCHEMA_VERSION,
            trace_schema: CODING_SERVICE_EVAL_TRACE_SCHEMA,
            plan_count: plans.len(),
            completed_count,
            failed_runner_contract_count,
            cancellation_probe_count,
            cancellation_passed_count,
            diagnostics_seen_count,
            health_seen_count,
            model_capabilities_seen_count,
            max_tokens_respected_count,
            rust_validation_checked_count,
            benchmark_checked_count,
            benchmark_passed_count,
            layer_b_route_proof_ready_count,
            rust_validation_layer_b_route_ready_count,
            result_class_counts,
            failure_class_counts,
            suite_report,
            run_records,
            evidence_packets,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn passed(&self) -> bool {
        self.plan_count == self.run_records.len()
            && self.completed_count == self.plan_count
            && self.failed_runner_contract_count == 0
            && self.cancellation_probe_count == self.cancellation_passed_count
            && self.diagnostics_seen_count == self.plan_count
            && self.health_seen_count == self.plan_count
            && self.model_capabilities_seen_count == self.plan_count
            && self.max_tokens_respected_count == self.plan_count
            && self.benchmark_checked_count == self.plan_count
            && self.benchmark_passed_count == self.plan_count
            && self.layer_b_route_proof_ready_count == self.plan_count
            && self.rust_validation_layer_b_route_ready_count == self.rust_validation_checked_count
            && self.rust_validation_checked_count > 0
            && self.result_class_counts.get("passed").copied().unwrap_or(0) == self.plan_count
            && self.failure_class_counts.is_empty()
            && self.suite_report.failed_count == 0
            && self.suite_report.profile_coverage()
                == CodingEvalProfileKind::expected_profiles().len()
            && self.evidence_is_redacted()
            && self.read_only
            && !self.write_allowed
            && !self.applied
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.suite_report.evidence_is_redacted()
            && self
                .run_records
                .iter()
                .all(CodingServiceEvalRunRecord::evidence_is_redacted)
            && self.evidence_packets.iter().all(|line| {
                line.contains("redaction-digest:") && !contains_private_or_executable_marker(line)
            })
    }

    pub fn summary_line(&self) -> String {
        format!(
            "coding_service_eval_runner schema={} trace_schema={} passed={} plans={} completed={} failed_runner_contract={} cancellation={}/{} diagnostics={} health={} model_capabilities={} max_tokens_respected={} rust_validation_checked={} benchmark_checked={} benchmark_passed={} layer_b_route_proof_ready={} rust_validation_layer_b_route_ready={} result_classes={} failure_classes={} suite_pass_rate={:.3} evidence_redacted={} read_only={} write_allowed={} applied={}",
            self.schema_version,
            self.trace_schema,
            self.passed(),
            self.plan_count,
            self.completed_count,
            self.failed_runner_contract_count,
            self.cancellation_passed_count,
            self.cancellation_probe_count,
            self.diagnostics_seen_count,
            self.health_seen_count,
            self.model_capabilities_seen_count,
            self.max_tokens_respected_count,
            self.rust_validation_checked_count,
            self.benchmark_checked_count,
            self.benchmark_passed_count,
            self.layer_b_route_proof_ready_count,
            self.rust_validation_layer_b_route_ready_count,
            map_summary(&self.result_class_counts),
            map_summary(&self.failure_class_counts),
            self.suite_report.pass_rate(),
            self.evidence_is_redacted(),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

impl CodingServiceEvalReadinessReport {
    pub fn from_corpus(corpus: &CodingEvalCorpus) -> Self {
        let validation = corpus.validate();
        let request_plans = request_plans_from_corpus(corpus);
        let suite_report = corpus.score_observations(&sample_passing_observations(corpus));
        let mut profile_counts = BTreeMap::new();
        let mut language_counts = BTreeMap::new();
        let mut capability_counts = BTreeMap::new();
        let mut request_evidence_packets = Vec::with_capacity(request_plans.len());

        for plan in &request_plans {
            *profile_counts
                .entry(plan.profile.as_str().to_owned())
                .or_insert(0) += 1;
            *language_counts
                .entry(plan.language.as_str().to_owned())
                .or_insert(0) += 1;
            for label in plan.capability_labels() {
                *capability_counts.entry(label.to_owned()).or_insert(0) += 1;
            }
            request_evidence_packets.push(plan.evidence_packet_line());
        }

        let missing_capabilities = CodingServiceEvalCapability::expected()
            .into_iter()
            .map(|capability| capability.as_str().to_owned())
            .filter(|label| !capability_counts.contains_key(label))
            .collect();

        Self {
            schema_version: CODING_SERVICE_EVAL_SCHEMA_VERSION,
            trace_schema: CODING_SERVICE_EVAL_TRACE_SCHEMA,
            coding_eval_schema_version: CODING_EVAL_SCHEMA_VERSION,
            request_plan_count: request_plans.len(),
            corpus_fixture_count: validation.fixture_count,
            corpus_validation_failures: validation.failures,
            suite_report,
            profile_counts,
            language_counts,
            capability_counts,
            missing_capabilities,
            request_evidence_packets,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn passed(&self) -> bool {
        self.corpus_validation_failures.is_empty()
            && self.missing_capabilities.is_empty()
            && self.request_plan_count == self.corpus_fixture_count
            && self.suite_report.failed_count == 0
            && self.suite_report.profile_coverage()
                == CodingEvalProfileKind::expected_profiles().len()
            && self.evidence_is_redacted()
            && self.read_only
            && !self.write_allowed
            && !self.applied
    }

    pub fn evidence_is_redacted(&self) -> bool {
        self.suite_report.evidence_is_redacted()
            && self.request_evidence_packets.iter().all(|line| {
                line.contains("redaction-digest:") && !contains_private_or_executable_marker(line)
            })
    }

    pub fn summary_line(&self) -> String {
        format!(
            "coding_service_eval schema={} trace_schema={} eval_schema={} passed={} requests={} fixtures={} suite_pass_rate={:.3} profiles={} languages={} missing_capabilities={} evidence_redacted={} read_only={} write_allowed={} applied={}",
            self.schema_version,
            self.trace_schema,
            self.coding_eval_schema_version,
            self.passed(),
            self.request_plan_count,
            self.corpus_fixture_count,
            self.suite_report.pass_rate(),
            map_summary(&self.profile_counts),
            map_summary(&self.language_counts),
            self.missing_capabilities.join("|"),
            self.evidence_is_redacted(),
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }
}

pub fn default_coding_service_eval_request_plans() -> Vec<CodingServiceEvalRequestPlan> {
    request_plans_from_corpus(&default_coding_eval_corpus())
}

pub fn default_coding_service_eval_readiness_report() -> CodingServiceEvalReadinessReport {
    CodingServiceEvalReadinessReport::from_corpus(&default_coding_eval_corpus())
}

pub fn default_coding_service_eval_runner_report() -> CodingServiceEvalRunnerReport {
    CodingServiceEvalRunnerReport::from_corpus(
        &default_coding_eval_corpus(),
        &CodingServiceEvalRunnerConfig::default(),
    )
}

pub fn coding_service_eval_readiness_report_from_fixture_tsv(
    input: &str,
) -> Result<CodingServiceEvalReadinessReport, Vec<String>> {
    coding_eval_corpus_from_fixture_tsv(input)
        .map(|corpus| CodingServiceEvalReadinessReport::from_corpus(&corpus))
}

pub fn coding_service_eval_runner_report_from_fixture_tsv(
    input: &str,
    config: &CodingServiceEvalRunnerConfig,
) -> Result<CodingServiceEvalRunnerReport, Vec<String>> {
    coding_eval_corpus_from_fixture_tsv(input)
        .map(|corpus| CodingServiceEvalRunnerReport::from_corpus(&corpus, config))
}

fn request_plans_from_corpus(corpus: &CodingEvalCorpus) -> Vec<CodingServiceEvalRequestPlan> {
    corpus.fixtures.iter().map(plan_from_fixture).collect()
}

fn run_plan(
    fixture: &CodingEvalFixture,
    plan: &CodingServiceEvalRequestPlan,
    config: &CodingServiceEvalRunnerConfig,
) -> CodingServiceEvalRunRecord {
    let output = deterministic_service_output(fixture);
    let mut session = ChatSession::new(
        plan.request.session_id.clone(),
        ChatSessionConfig::new(config.history_limit)
            .with_default_max_tokens(plan.request.max_tokens),
    );
    session.begin_stream();
    session.push_status(format!(
        "health online model={} route={}",
        config.offline_model_label,
        plan.request.routing_intent().summary()
    ));
    session.push_metadata(format!(
        "capabilities streaming cancellation max_tokens diagnostics health model_capabilities offline_mock rust_validation={}",
        plan.requires_rust_validation
    ));
    session.push_metadata(format!(
        "diagnostics role={} routing={} endpoint={} profile={} output={}",
        plan.request.model_role_label(),
        plan.request.routing_preference_label(),
        plan.request.endpoint_label(),
        plan.request.profile,
        plan.request.output
    ));
    for delta in streaming_deltas(&output) {
        session.push_delta(delta);
    }
    session.push_final_payload_with_answer(
        format!(
            "answer_digest={} fixture={} profile={}",
            stable_redaction_digest(["coding-service-eval-answer", fixture.id.as_str(), &output]),
            fixture.id,
            fixture.profile.as_str()
        ),
        output.clone(),
    );
    session.finish();

    let final_state = session.state();
    let stream_chunk_count = session.chunks().len();
    let delta_chunk_count = session
        .chunks()
        .iter()
        .filter(|chunk| chunk.kind == ChatChunkKind::Delta)
        .count();
    let status_chunk_count = session
        .chunks()
        .iter()
        .filter(|chunk| chunk.kind == ChatChunkKind::Status)
        .count();
    let metadata_chunk_count = session
        .chunks()
        .iter()
        .filter(|chunk| chunk.kind == ChatChunkKind::Metadata)
        .count();
    let final_chunk_count = session
        .chunks()
        .iter()
        .filter(|chunk| chunk.kind == ChatChunkKind::Final)
        .count();
    let done_chunk_count = session
        .chunks()
        .iter()
        .filter(|chunk| chunk.kind == ChatChunkKind::Done)
        .count();
    let health_seen = session
        .chunks()
        .iter()
        .any(|chunk| chunk.content.contains("health online"));
    let model_capabilities_seen = session
        .chunks()
        .iter()
        .any(|chunk| chunk.content.contains("capabilities streaming"));
    let diagnostics_seen = session
        .chunks()
        .iter()
        .any(|chunk| chunk.content.contains("diagnostics role="));

    let cancellation = if plan.requires_cancellation_probe {
        Some(run_cancellation_probe(plan, config))
    } else {
        None
    };
    let tokens = estimate_tokens(&output);
    let latency_ms =
        config.base_latency_ms + config.per_message_latency_ms * plan.request.messages.len() as u64;
    let max_tokens_respected = plan
        .request
        .max_tokens
        .is_some_and(|max_tokens| tokens <= max_tokens as u64);
    let rust_validation_checked = plan.requires_rust_validation
        && fixture.requires_cargo_check()
        && fixture.requires_cargo_test();
    let benchmark_checked = true;
    let benchmark_passed =
        benchmark_passed(plan.profile, config.memory_hit_rate, tokens, latency_ms);
    let layer_b_route_proof = coding_service_eval_layer_b_route_proof(plan, config);
    let layer_b_route_proof_ready = layer_b_route_proof.validate().is_ok();
    let rust_validation_layer_b_route_ready = rust_validation_checked && layer_b_route_proof_ready;
    let observation = CodingEvalObservation::new(fixture.id.clone(), output.clone())
        .with_compile(
            fixture.requires_cargo_check(),
            fixture.requires_cargo_check(),
        )
        .with_unit_tests(fixture.requires_cargo_test(), fixture.requires_cargo_test())
        .with_runtime_metrics(config.memory_hit_rate, tokens, latency_ms)
        .with_benchmark_regression(0.0)
        .with_redaction(!contains_private_or_executable_marker(&output));

    let (
        cancellation_probed,
        cancellation_passed,
        cancellation_state,
        cancellation_state_label,
        cancellation_partial_chars,
    ) = cancellation
        .map(|(state, state_label, partial_chars, passed)| {
            (true, passed, Some(state), Some(state_label), partial_chars)
        })
        .unwrap_or((false, false, None, None, 0));
    let evidence_packet_line = run_evidence_packet_line(
        fixture,
        plan,
        config,
        &observation,
        final_state,
        stream_chunk_count,
        delta_chunk_count,
        status_chunk_count,
        metadata_chunk_count,
        final_chunk_count,
        done_chunk_count,
        cancellation_probed,
        cancellation_passed,
        cancellation_state,
        cancellation_partial_chars,
        diagnostics_seen,
        health_seen,
        model_capabilities_seen,
        max_tokens_respected,
        rust_validation_checked,
        benchmark_checked,
        benchmark_passed,
        layer_b_route_proof_ready,
        rust_validation_layer_b_route_ready,
        &layer_b_route_proof,
    );

    CodingServiceEvalRunRecord {
        fixture_id: fixture.id.clone(),
        profile: fixture.profile,
        language: plan.language,
        model_label: config.offline_model_label.clone(),
        final_state,
        final_state_label: final_state.as_str().to_owned(),
        stream_chunk_count,
        delta_chunk_count,
        status_chunk_count,
        metadata_chunk_count,
        final_chunk_count,
        done_chunk_count,
        cancellation_probed,
        cancellation_passed,
        cancellation_state,
        cancellation_state_label,
        cancellation_partial_chars,
        diagnostics_seen,
        health_seen,
        model_capabilities_seen,
        max_tokens_respected,
        rust_validation_checked,
        compile_checked: fixture.requires_cargo_check(),
        unit_test_checked: fixture.requires_cargo_test(),
        benchmark_checked,
        benchmark_passed,
        layer_b_route_proof_ready,
        rust_validation_layer_b_route_ready,
        observation,
        evidence_packet_line,
    }
}

fn run_cancellation_probe(
    plan: &CodingServiceEvalRequestPlan,
    config: &CodingServiceEvalRunnerConfig,
) -> (StreamState, String, usize, bool) {
    let mut session = ChatSession::new(
        format!("{}-cancel-probe", plan.request.session_id),
        ChatSessionConfig::new(config.history_limit)
            .with_default_max_tokens(plan.request.max_tokens),
    );
    session.begin_stream();
    session.push_status("cancellation probe active");
    session.push_delta("partial cancellation probe");
    session.cancel_stream();
    let snapshot = session.outcome().snapshot();
    let passed = snapshot.state == StreamState::Interrupted
        && snapshot.has_partial
        && !snapshot.state_blocks_prompt_submit;

    (
        snapshot.state,
        snapshot.state_label,
        snapshot.partial_chars,
        passed,
    )
}

fn deterministic_service_output(fixture: &CodingEvalFixture) -> String {
    match fixture.profile {
        CodingEvalProfileKind::EnglishInstruction => {
            "Use Result with error context, return recoverable errors with no panic, and include a validation step for the caller.".to_owned()
        }
        CodingEvalProfileKind::ChineseInstruction => {
            "借用 和 所有权 让 Rust 在编译期避免 数据竞争；修改建议 是缩小可变借用作用域并保持清晰生命周期。".to_owned()
        }
        CodingEvalProfileKind::RustCodeGeneration => {
            "fn parse_port(input: &str) -> Result<u16, String> { let value: u16 = input.trim().parse().map_err(|_| \"parse error\".to_owned())?; if value == 0 { return Err(\"zero port\".to_owned()); } Ok(value) } Validation: run cargo check and cargo test.".to_owned()
        }
        CodingEvalProfileKind::RustRepair => {
            "Use std::borrow::Cow and return Cow::Borrowed for the prefix while falling back to Cow::Owned only when allocation is needed. The lifetime is valid because the borrowed slice is tied to the input, and cargo test covers the repair.".to_owned()
        }
        CodingEvalProfileKind::MultilingualCodingExplanation => {
            "Return Result instead of unwrap so request handling can report recoverable errors. 中文: Result 让 错误处理 可恢复，避免 unwrap 导致 请求处理 崩溃。".to_owned()
        }
    }
}

fn streaming_deltas(output: &str) -> Vec<String> {
    let words = output.split_whitespace().collect::<Vec<_>>();
    if words.len() <= 8 {
        return vec![output.to_owned()];
    }
    let split_at = words.len() / 2;
    vec![
        format!("{} ", words[..split_at].join(" ")),
        words[split_at..].join(" "),
    ]
}

fn estimate_tokens(output: &str) -> u64 {
    output
        .split_whitespace()
        .count()
        .max(output.chars().count().div_ceil(8))
        .max(1) as u64
}

#[allow(clippy::too_many_arguments)]
fn run_evidence_packet_line(
    fixture: &CodingEvalFixture,
    plan: &CodingServiceEvalRequestPlan,
    config: &CodingServiceEvalRunnerConfig,
    observation: &CodingEvalObservation,
    final_state: StreamState,
    stream_chunk_count: usize,
    delta_chunk_count: usize,
    status_chunk_count: usize,
    metadata_chunk_count: usize,
    final_chunk_count: usize,
    done_chunk_count: usize,
    cancellation_probed: bool,
    cancellation_passed: bool,
    cancellation_state: Option<StreamState>,
    cancellation_partial_chars: usize,
    diagnostics_seen: bool,
    health_seen: bool,
    model_capabilities_seen: bool,
    max_tokens_respected: bool,
    rust_validation_checked: bool,
    benchmark_checked: bool,
    benchmark_passed: bool,
    layer_b_route_proof_ready: bool,
    rust_validation_layer_b_route_ready: bool,
    layer_b_route_proof: &AgentModelRouteProof,
) -> String {
    [
        CODING_SERVICE_EVAL_RUNNER_SCHEMA_VERSION.to_owned(),
        fixture.id.clone(),
        fixture.profile.as_str().to_owned(),
        plan.language.as_str().to_owned(),
        final_state.as_str().to_owned(),
        stream_chunk_count.to_string(),
        delta_chunk_count.to_string(),
        status_chunk_count.to_string(),
        metadata_chunk_count.to_string(),
        final_chunk_count.to_string(),
        done_chunk_count.to_string(),
        cancellation_probed.to_string(),
        cancellation_passed.to_string(),
        cancellation_state
            .map(|state| state.as_str().to_owned())
            .unwrap_or_else(|| "none".to_owned()),
        cancellation_partial_chars.to_string(),
        diagnostics_seen.to_string(),
        health_seen.to_string(),
        model_capabilities_seen.to_string(),
        max_tokens_respected.to_string(),
        rust_validation_checked.to_string(),
        observation.compile_checked.to_string(),
        observation.compile_passed.to_string(),
        observation.unit_test_checked.to_string(),
        observation.unit_test_passed.to_string(),
        benchmark_checked.to_string(),
        benchmark_passed.to_string(),
        layer_b_route_proof_ready.to_string(),
        rust_validation_layer_b_route_ready.to_string(),
        observation.tokens.to_string(),
        observation.latency_ms.to_string(),
        format!("{:.3}", observation.memory_hit_rate),
        stable_redaction_digest([
            "coding-service-eval-run-output",
            fixture.id.as_str(),
            observation.output.as_str(),
        ]),
        stable_redaction_digest([
            "coding-service-eval-run-request",
            fixture.id.as_str(),
            plan.request.session_id.as_str(),
            config.offline_model_label.as_str(),
        ]),
        stable_redaction_digest([
            "coding-service-eval-layer-b-route",
            fixture.id.as_str(),
            layer_b_route_proof.model_registry_id.as_str(),
            layer_b_route_proof.model_profile_id.as_str(),
            layer_b_route_proof.inference_backend_id.as_str(),
            layer_b_route_proof.model_pool_id.as_str(),
            layer_b_route_proof
                .selected_role
                .as_deref()
                .unwrap_or("none"),
        ]),
    ]
    .into_iter()
    .map(|field| escape_field(&field))
    .collect::<Vec<_>>()
    .join("\t")
}

fn plan_from_fixture(fixture: &norion_eval::CodingEvalFixture) -> CodingServiceEvalRequestPlan {
    let language = language_for_profile(fixture.profile);
    let mut request = ChatRequest::new(
        format!("r97-{}", fixture.id),
        vec![
            ChatMessage::system(system_prompt_for_profile(fixture.profile)),
            ChatMessage::user(fixture.prompt.clone()),
        ],
    );
    request.profile = "coding".to_owned();
    request.output = "json".to_owned();
    request.stream = true;
    request.max_tokens = Some(max_tokens_for_profile(fixture.profile));
    request.model_role = model_role_for_profile(fixture.profile);
    request.routing_preference = routing_preference_for_profile(fixture.profile);
    request.model_endpoint = endpoint_for_profile(fixture.profile);

    CodingServiceEvalRequestPlan {
        fixture_id: fixture.id.clone(),
        profile: fixture.profile,
        language,
        request,
        requires_streaming: true,
        requires_cancellation_probe: cancellation_probe_for_profile(fixture.profile),
        requires_diagnostics: true,
        requires_health: true,
        requires_model_capabilities: true,
        requires_offline_mock_backend: true,
        requires_rust_validation: fixture.requires_cargo_check() || fixture.requires_cargo_test(),
    }
}

fn language_for_profile(profile: CodingEvalProfileKind) -> CodingServiceEvalLanguage {
    match profile {
        CodingEvalProfileKind::EnglishInstruction => CodingServiceEvalLanguage::English,
        CodingEvalProfileKind::ChineseInstruction => CodingServiceEvalLanguage::Chinese,
        CodingEvalProfileKind::RustCodeGeneration | CodingEvalProfileKind::RustRepair => {
            CodingServiceEvalLanguage::Rust
        }
        CodingEvalProfileKind::MultilingualCodingExplanation => {
            CodingServiceEvalLanguage::MixedEnglishChinese
        }
    }
}

fn system_prompt_for_profile(profile: CodingEvalProfileKind) -> &'static str {
    match profile {
        CodingEvalProfileKind::EnglishInstruction => {
            "You are Norion local service eval. Answer in English with concise Rust guidance."
        }
        CodingEvalProfileKind::ChineseInstruction => {
            "You are Norion local service eval. Answer in Chinese with concise Rust guidance."
        }
        CodingEvalProfileKind::RustCodeGeneration => {
            "You are Norion local service eval. Produce safe Rust code and validation notes."
        }
        CodingEvalProfileKind::RustRepair => {
            "You are Norion local service eval. Repair Rust code with borrow-aware reasoning and validation notes."
        }
        CodingEvalProfileKind::MultilingualCodingExplanation => {
            "You are Norion local service eval. Answer in English and Chinese for Rust service guidance."
        }
    }
}

fn max_tokens_for_profile(profile: CodingEvalProfileKind) -> usize {
    match profile {
        CodingEvalProfileKind::EnglishInstruction | CodingEvalProfileKind::ChineseInstruction => {
            900
        }
        CodingEvalProfileKind::RustCodeGeneration => 1_600,
        CodingEvalProfileKind::RustRepair => 1_800,
        CodingEvalProfileKind::MultilingualCodingExplanation => 1_200,
    }
}

fn model_role_for_profile(profile: CodingEvalProfileKind) -> ModelRole {
    match profile {
        CodingEvalProfileKind::EnglishInstruction | CodingEvalProfileKind::ChineseInstruction => {
            ModelRole::Assistant
        }
        CodingEvalProfileKind::RustCodeGeneration | CodingEvalProfileKind::RustRepair => {
            ModelRole::Tester
        }
        CodingEvalProfileKind::MultilingualCodingExplanation => ModelRole::Reviewer,
    }
}

fn routing_preference_for_profile(profile: CodingEvalProfileKind) -> RoutingPreference {
    match profile {
        CodingEvalProfileKind::RustCodeGeneration
        | CodingEvalProfileKind::RustRepair
        | CodingEvalProfileKind::MultilingualCodingExplanation => RoutingPreference::PreferQuality,
        CodingEvalProfileKind::EnglishInstruction | CodingEvalProfileKind::ChineseInstruction => {
            RoutingPreference::Balanced
        }
    }
}

fn endpoint_for_profile(profile: CodingEvalProfileKind) -> Option<ModelEndpoint> {
    match profile {
        CodingEvalProfileKind::RustCodeGeneration | CodingEvalProfileKind::RustRepair => {
            Some(ModelEndpoint::SummaryTester)
        }
        _ => None,
    }
}

fn coding_service_eval_layer_b_route_proof(
    plan: &CodingServiceEvalRequestPlan,
    config: &CodingServiceEvalRunnerConfig,
) -> AgentModelRouteProof {
    let model_profile_id = plan
        .request
        .model_endpoint
        .as_ref()
        .map(ModelEndpoint::label)
        .unwrap_or(match plan.request.routing_preference {
            RoutingPreference::Balanced => "coding-balanced-auto-profile",
            RoutingPreference::PreferFast => "coding-fast-auto-profile",
            RoutingPreference::PreferQuality => "coding-quality-auto-profile",
        });

    AgentModelRouteProof::new(
        "model-registry-v1",
        model_profile_id,
        config.offline_model_label.as_str(),
        "coding-service-eval-model-pool",
    )
    .with_selected_role(plan.request.model_role_label())
}

fn benchmark_passed(
    profile: CodingEvalProfileKind,
    memory_hit_rate: f32,
    tokens: u64,
    latency_ms: u64,
) -> bool {
    let thresholds = CodingEvalThresholds::for_profile(profile);
    memory_hit_rate >= thresholds.min_memory_hit_rate
        && tokens <= thresholds.max_tokens
        && latency_ms <= thresholds.max_latency_ms
}

fn cancellation_probe_for_profile(profile: CodingEvalProfileKind) -> bool {
    matches!(
        profile,
        CodingEvalProfileKind::RustRepair | CodingEvalProfileKind::MultilingualCodingExplanation
    )
}

fn map_summary(map: &BTreeMap<String, usize>) -> String {
    if map.is_empty() {
        return "none".to_owned();
    }
    map.iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn result_class_counts(
    suite_report: &CodingEvalSuiteReport,
    failed_runner_contract_count: usize,
) -> BTreeMap<String, usize> {
    BTreeMap::from([
        ("failed".to_owned(), suite_report.failed_count),
        ("passed".to_owned(), suite_report.passed_count),
        (
            "runner_contract_failed".to_owned(),
            failed_runner_contract_count,
        ),
    ])
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;
    use norion_service::{ModelEndpointSelectionKind, ModelRole};

    const FIXTURE_TSV: &str = "\
english-loaded\tenglish_instruction\tExplain recoverable Rust parse errors without panic.\tResult|error context|validation|no panic\tnone
chinese-loaded\tchinese_instruction\t用中文解释 Rust 借用检查器如何避免数据竞争。\t借用|数据竞争|所有权|修改建议\tnone
rust-codegen-loaded\trust_code_generation\tWrite parse_port(input: &str) -> Result<u16, String>.\tfn parse_port|Result<u16|trim|parse|zero\tcargo
rust-repair-loaded\trust_repair\tRepair a helper so borrowed prefixes avoid cloning.\tCow|Borrowed|Owned|lifetime|cargo test\tcargo
mixed-loaded\tmultilingual_coding_explanation\tExplain Result vs unwrap in English and Chinese.\tResult|unwrap|错误处理|请求处理\tnone";

    #[test]
    fn default_readiness_report_covers_r97_service_and_eval_contract() {
        let report = default_coding_service_eval_readiness_report();

        assert_eq!(report.schema_version, CODING_SERVICE_EVAL_SCHEMA_VERSION);
        assert_eq!(report.trace_schema, CODING_SERVICE_EVAL_TRACE_SCHEMA);
        assert!(report.passed(), "{:?}", report.missing_capabilities);
        assert_eq!(
            report.profile_counts.len(),
            CodingEvalProfileKind::expected_profiles().len()
        );
        assert!(report.language_counts.contains_key("english"));
        assert!(report.language_counts.contains_key("chinese"));
        assert!(report.language_counts.contains_key("rust"));
        assert!(report.language_counts.contains_key("mixed_english_chinese"));
        for capability in CodingServiceEvalCapability::expected() {
            assert!(
                report.capability_counts.contains_key(capability.as_str()),
                "missing {}",
                capability.as_str()
            );
        }
        assert!(report.summary_line().contains("passed=true"));
        assert!(report.read_only && !report.write_allowed && !report.applied);
    }

    #[test]
    fn fixture_tsv_loader_runs_multilingual_readiness_and_mock_runner() {
        let readiness = coding_service_eval_readiness_report_from_fixture_tsv(FIXTURE_TSV)
            .expect("fixture tsv readiness");
        let runner = coding_service_eval_runner_report_from_fixture_tsv(
            FIXTURE_TSV,
            &CodingServiceEvalRunnerConfig::default(),
        )
        .expect("fixture tsv runner");

        assert!(readiness.passed(), "{}", readiness.summary_line());
        assert!(runner.passed(), "{}", runner.summary_line());
        assert_eq!(readiness.corpus_fixture_count, 5);
        assert_eq!(readiness.language_counts.get("english"), Some(&1));
        assert_eq!(readiness.language_counts.get("chinese"), Some(&1));
        assert_eq!(readiness.language_counts.get("rust"), Some(&2));
        assert_eq!(
            readiness.language_counts.get("mixed_english_chinese"),
            Some(&1)
        );
        assert_eq!(runner.rust_validation_checked_count, 2);
        assert_eq!(runner.benchmark_checked_count, 5);
        assert_eq!(runner.benchmark_passed_count, 5);
        assert_eq!(runner.layer_b_route_proof_ready_count, 5);
        assert_eq!(runner.rust_validation_layer_b_route_ready_count, 2);
        assert!(readiness.read_only && !readiness.write_allowed && !readiness.applied);
        assert!(runner.read_only && !runner.write_allowed && !runner.applied);
    }

    #[test]
    fn fixture_tsv_loader_rejects_incomplete_fixture_set() {
        let errors = coding_service_eval_readiness_report_from_fixture_tsv(
            "english-only\tenglish_instruction\tExplain Result.\tResult|validation\tnone",
        )
        .expect_err("incomplete fixture set must fail");

        assert!(
            errors
                .iter()
                .any(|error| error.contains("missing_fixture_profile:chinese_instruction"))
        );
    }

    #[test]
    fn request_plans_preserve_task_intent_and_wire_metadata() {
        let plans = default_coding_service_eval_request_plans();
        let rust_plan = plans
            .iter()
            .find(|plan| plan.profile == CodingEvalProfileKind::RustCodeGeneration)
            .expect("rust codegen plan");
        let chinese_plan = plans
            .iter()
            .find(|plan| plan.profile == CodingEvalProfileKind::ChineseInstruction)
            .expect("chinese plan");

        assert_eq!(rust_plan.request.model_role, ModelRole::Tester);
        assert_eq!(
            rust_plan.request.endpoint_kind(),
            ModelEndpointSelectionKind::BuiltIn
        );
        assert_eq!(rust_plan.request.max_tokens, Some(1_600));
        assert!(rust_plan.requires_rust_validation);
        assert!(rust_plan.request.stream);
        assert_eq!(chinese_plan.language, CodingServiceEvalLanguage::Chinese);
        assert_eq!(chinese_plan.request.model_role, ModelRole::Assistant);
        assert_eq!(
            chinese_plan.request.endpoint_kind(),
            ModelEndpointSelectionKind::Auto
        );
    }

    #[test]
    fn request_wire_json_covers_streaming_max_tokens_and_routing() {
        let plans = default_coding_service_eval_request_plans();
        let repair = plans
            .iter()
            .find(|plan| plan.profile == CodingEvalProfileKind::RustRepair)
            .expect("rust repair plan");
        let json = repair.request_wire_json();

        assert!(json.contains("\"stream\":true"));
        assert!(json.contains("\"max_tokens\":1800"));
        assert!(json.contains("\"model_role\":\"tester\""));
        assert!(json.contains("\"routing_preference\":\"prefer_quality\""));
        assert!(json.contains("\"prefer_quality\":true"));
        assert!(json.contains("\"endpoint_kind\":\"built_in\""));
        assert!(json.contains("\"model_endpoint\":\"summary-tester\""));
        assert!(repair.requires_cancellation_probe);
        assert!(repair.requires_diagnostics);
    }

    #[test]
    fn evidence_packets_are_digest_only_and_do_not_leak_prompts() {
        let plans = default_coding_service_eval_request_plans();
        let report = default_coding_service_eval_readiness_report();

        assert!(report.evidence_is_redacted());
        for plan in &plans {
            assert!(plan.evidence_is_redacted());
            let line = plan.evidence_packet_line();
            assert!(line.contains("redaction-digest:"));
            for message in &plan.request.messages {
                assert!(!line.contains(&message.content));
            }
        }
    }

    #[test]
    fn readiness_report_fails_when_required_capability_is_missing() {
        let mut corpus = default_coding_eval_corpus();
        corpus.fixtures.retain(|fixture| {
            !matches!(
                fixture.profile,
                CodingEvalProfileKind::RustRepair
                    | CodingEvalProfileKind::MultilingualCodingExplanation
            )
        });

        let report = CodingServiceEvalReadinessReport::from_corpus(&corpus);

        assert!(!report.passed());
        assert!(
            report
                .corpus_validation_failures
                .iter()
                .any(|failure| { failure.contains("missing_fixture_profile:rust_repair") })
        );
        assert!(
            report
                .missing_capabilities
                .contains(&"cancellation".to_owned())
        );
    }

    #[test]
    fn default_mock_runner_executes_plans_and_scores_suite() {
        let report = default_coding_service_eval_runner_report();

        assert_eq!(
            report.schema_version,
            CODING_SERVICE_EVAL_RUNNER_SCHEMA_VERSION
        );
        assert!(report.passed(), "{}", report.summary_line());
        assert_eq!(report.plan_count, 5);
        assert_eq!(report.completed_count, report.plan_count);
        assert_eq!(report.failed_runner_contract_count, 0);
        assert_eq!(report.cancellation_probe_count, 2);
        assert_eq!(report.cancellation_passed_count, 2);
        assert_eq!(report.diagnostics_seen_count, report.plan_count);
        assert_eq!(report.health_seen_count, report.plan_count);
        assert_eq!(report.model_capabilities_seen_count, report.plan_count);
        assert_eq!(report.max_tokens_respected_count, report.plan_count);
        assert_eq!(report.rust_validation_checked_count, 2);
        assert_eq!(report.benchmark_checked_count, report.plan_count);
        assert_eq!(report.benchmark_passed_count, report.plan_count);
        assert_eq!(report.layer_b_route_proof_ready_count, report.plan_count);
        assert_eq!(report.rust_validation_layer_b_route_ready_count, 2);
        assert_eq!(report.result_class_counts.get("passed"), Some(&5));
        assert_eq!(report.result_class_counts.get("failed"), Some(&0));
        assert_eq!(
            report.result_class_counts.get("runner_contract_failed"),
            Some(&0)
        );
        assert!(report.failure_class_counts.is_empty());
        assert_eq!(report.suite_report.failed_count, 0);
        assert_eq!(
            report.suite_report.profile_coverage(),
            CodingEvalProfileKind::expected_profiles().len()
        );
        assert!(report.summary_line().contains("passed=true"));
        assert!(report.summary_line().contains("benchmark_passed=5"));
        assert!(
            report
                .summary_line()
                .contains("rust_validation_layer_b_route_ready=2")
        );
        assert!(report.summary_line().contains("result_classes="));
        assert!(report.read_only && !report.write_allowed && !report.applied);
    }

    #[test]
    fn mock_runner_surfaces_eval_failure_classification() {
        let mut corpus = default_coding_eval_corpus();
        corpus.fixtures[0].expected_markers = vec!["missing-result-class-marker".to_owned()];

        let report = CodingServiceEvalRunnerReport::from_corpus(
            &corpus,
            &CodingServiceEvalRunnerConfig::default(),
        );

        assert!(!report.passed());
        assert_eq!(report.result_class_counts.get("passed"), Some(&4));
        assert_eq!(report.result_class_counts.get("failed"), Some(&1));
        assert_eq!(
            report.failure_class_counts.get("missing_expected_marker"),
            Some(&1)
        );
        assert!(
            report
                .summary_line()
                .contains("failure_classes=missing_expected_marker:1")
        );
    }

    #[test]
    fn mock_runner_uses_stream_session_and_cancellation_contract() {
        let report = default_coding_service_eval_runner_report();
        let repair = report
            .run_records
            .iter()
            .find(|record| record.profile == CodingEvalProfileKind::RustRepair)
            .expect("rust repair run");
        let english = report
            .run_records
            .iter()
            .find(|record| record.profile == CodingEvalProfileKind::EnglishInstruction)
            .expect("english run");

        assert!(repair.passed_runner_contract());
        assert_eq!(repair.final_state, StreamState::Completed);
        assert!(repair.delta_chunk_count >= 2);
        assert!(repair.status_chunk_count >= 1);
        assert!(repair.metadata_chunk_count >= 2);
        assert_eq!(repair.final_chunk_count, 1);
        assert_eq!(repair.done_chunk_count, 1);
        assert!(repair.cancellation_probed);
        assert!(repair.cancellation_passed);
        assert_eq!(repair.cancellation_state, Some(StreamState::Interrupted));
        assert!(repair.cancellation_partial_chars > 0);
        assert!(repair.rust_validation_checked);
        assert!(repair.compile_checked);
        assert!(repair.unit_test_checked);
        assert!(repair.benchmark_checked);
        assert!(repair.benchmark_passed);
        assert!(repair.layer_b_route_proof_ready);
        assert!(repair.rust_validation_layer_b_route_ready);

        assert!(english.passed_runner_contract());
        assert!(!english.cancellation_probed);
        assert_eq!(english.cancellation_state, None);
        assert!(!english.rust_validation_checked);
        assert!(!english.compile_checked);
        assert!(!english.unit_test_checked);
        assert!(english.benchmark_checked);
        assert!(english.benchmark_passed);
        assert!(english.layer_b_route_proof_ready);
        assert!(!english.rust_validation_layer_b_route_ready);
    }

    #[test]
    fn mock_runner_evidence_is_digest_only() {
        let plans = default_coding_service_eval_request_plans();
        let report = default_coding_service_eval_runner_report();

        assert!(report.evidence_is_redacted());
        for record in &report.run_records {
            assert!(record.evidence_is_redacted());
            assert!(record.evidence_packet_line.contains("redaction-digest:"));
            assert!(
                !record
                    .evidence_packet_line
                    .contains(&record.observation.output)
            );
            for plan in plans
                .iter()
                .filter(|plan| plan.fixture_id == record.fixture_id)
            {
                for message in &plan.request.messages {
                    assert!(!record.evidence_packet_line.contains(&message.content));
                }
            }
        }
    }
}
