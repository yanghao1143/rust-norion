use std::collections::BTreeMap;

use norion_eval::{
    CODING_EVAL_SCHEMA_VERSION, CodingEvalCorpus, CodingEvalProfileKind, CodingEvalSuiteReport,
    default_coding_eval_corpus, sample_passing_observations,
};
use norion_service::{
    ChatMessage, ChatRequest, ModelEndpoint, ModelRole, RoutingPreference, request_json,
};

use crate::privacy_redaction::{contains_private_or_executable_marker, stable_redaction_digest};

pub const CODING_SERVICE_EVAL_SCHEMA_VERSION: &str = "coding_service_eval_v1";
pub const CODING_SERVICE_EVAL_TRACE_SCHEMA: &str = "rust-norion-coding-service-eval-readiness-v1";

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

fn request_plans_from_corpus(corpus: &CodingEvalCorpus) -> Vec<CodingServiceEvalRequestPlan> {
    corpus.fixtures.iter().map(plan_from_fixture).collect()
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
}
