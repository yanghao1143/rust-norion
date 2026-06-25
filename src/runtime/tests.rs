#![allow(clippy::too_many_arguments)]

use super::*;
use crate::agent_team::AgentTeamPlan;
use crate::experience::ExperienceMatch;
use crate::hardware::{HardwareAllocator, HardwareSnapshot};
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::infini_memory::{InfiniMemoryItem, InfiniMemoryPlan, InfiniMemoryScope};
use crate::kv_cache::MemoryMatch;
use crate::recursive_scheduler::RecursiveSchedule;
use crate::router::RouteBudget;
use crate::runtime_manifest::TransformerRuntimeArchitecture;
use crate::tiered_cache::TieredCachePlan;
use crate::toolsmith::ToolsmithPlan;
use crate::transformer::TransformerRefactorPlan;

fn default_agent_team_plan() -> &'static AgentTeamPlan {
    static PLAN: std::sync::OnceLock<AgentTeamPlan> = std::sync::OnceLock::new();
    PLAN.get_or_init(AgentTeamPlan::default)
}

fn default_toolsmith_plan() -> &'static ToolsmithPlan {
    static PLAN: std::sync::OnceLock<ToolsmithPlan> = std::sync::OnceLock::new();
    PLAN.get_or_init(ToolsmithPlan::default)
}

#[derive(Debug, Default, Clone)]
struct MockRuntime {
    seen: Option<RuntimeRequest>,
}

impl ModelRuntime for MockRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata::new("mock-self-transformer", "mock-bpe", 32_768, 128)
            .with_kv_exchange(true, true)
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        TransformerRuntimeArchitecture::new(18, 128, 8, 4, 4096)
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        self.seen = Some(request.clone());
        let mut response = RuntimeResponse::new(format!(
            "runtime saw {} memories and {} experiences",
            request.memory_hints.len(),
            request.experience_hints.len()
        ));
        response.tokens = vec![RuntimeToken {
            text: "runtime".to_owned(),
            logprob: Some(-0.1),
            entropy: Some(0.2),
        }];
        Ok(response)
    }
}

#[derive(Debug, Default, Clone)]
struct EndpointOverrideRuntime {
    endpoint: Option<String>,
}

impl ModelRuntime for EndpointOverrideRuntime {
    fn supports_endpoint_override(&self) -> bool {
        true
    }

    fn clone_for_endpoint_override(&self, base_url: &str) -> Result<Option<Self>, RuntimeError> {
        Ok(Some(Self {
            endpoint: Some(base_url.to_owned()),
        }))
    }

    fn generate(&mut self, _request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        Ok(RuntimeResponse::new(format!(
            "endpoint={}",
            self.endpoint.as_deref().unwrap_or("default")
        )))
    }
}

fn sample_generation_context<'a>(
    prompt: &'a str,
    memories: &'a [MemoryMatch],
    experiences: &'a [ExperienceMatch],
    tier_plan: &'a TieredCachePlan,
    infini_memory_plan: &'a InfiniMemoryPlan,
    recursive_schedule: &'a RecursiveSchedule,
    hardware_plan: &'a HardwarePlan,
    transformer_plan: &'a TransformerRefactorPlan,
) -> GenerationContext<'a> {
    GenerationContext {
        prompt,
        profile: TaskProfile::Coding,
        memories,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        tier_plan,
        infini_memory_plan,
        recursive_schedule,
        hardware_plan,
        experiences,
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan,
    }
}

#[test]
fn runtime_backend_endpoint_override_is_opt_in_and_clearable() {
    let mut unsupported = RuntimeBackend::new(MockRuntime::default());
    assert!(
        !unsupported
            .configure_runtime_endpoint_override(Some("http://127.0.0.1:8687"))
            .unwrap()
    );
    assert_eq!(unsupported.runtime_endpoint_override_active(), None);

    let mut backend = RuntimeBackend::new(EndpointOverrideRuntime::default());
    assert!(
        backend
            .configure_runtime_endpoint_override(Some("http://127.0.0.1:8687"))
            .unwrap()
    );
    assert_eq!(
        backend.runtime_endpoint_override_active(),
        Some("http://127.0.0.1:8687")
    );

    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let context = sample_generation_context(
        "route summary worker",
        &[],
        &[],
        &tier_plan,
        &infini_memory_plan,
        &recursive_schedule,
        &hardware_plan,
        &transformer_plan,
    );

    let draft = backend.generate(context);

    assert!(draft.answer.contains("http://127.0.0.1:8687"));
    assert!(
        draft
            .trace
            .iter()
            .any(|step| step.label == "runtime_endpoint_override")
    );
    assert!(!backend.configure_runtime_endpoint_override(None).unwrap());
    assert_eq!(backend.runtime_endpoint_override_active(), None);
}

#[test]
fn runtime_backend_endpoint_override_rejects_invalid_http_endpoint() {
    let runtime = MistralRsHttpRuntime::new("http://127.0.0.1:8686").unwrap();
    let mut backend = RuntimeBackend::new(runtime);
    assert!(
        backend
            .configure_runtime_endpoint_override(Some("http://127.0.0.1:8687"))
            .unwrap()
    );

    let error = backend
        .configure_runtime_endpoint_override(Some("https://127.0.0.1:8688"))
        .unwrap_err();

    assert!(error.contains("only supports local http:// endpoints"));
    assert_eq!(backend.runtime_endpoint_override_active(), None);
}

#[test]
fn runtime_backend_maps_context_to_request() {
    let memories = vec![MemoryMatch {
        id: 1,
        key: "kv memory".to_owned(),
        similarity: 0.8,
        strength: 1.2,
        vector: vec![0.1, 0.2, 0.3],
    }];
    let experiences = vec![ExperienceMatch {
        id: 1,
        prompt: "prompt".to_owned(),
        lesson: "lesson".to_owned(),
        quality: 0.9,
        score: 0.88,
        gist_hints: vec!["document:gist importance=0.900".to_owned()],
        reflection_issue_codes: Vec::new(),
        revision_actions: Vec::new(),
        process_reward: 0.81,
        reward_action: crate::process_reward::RewardAction::Reinforce,
        runtime_model_id: Some("mock-self-transformer".to_owned()),
        runtime_selected_adapter: Some("portable-rust".to_owned()),
        runtime_device_profile: None,
        runtime_primary_lane: None,
        runtime_fallback_lane: None,
        runtime_memory_mode: None,
        runtime_device_execution_source: None,
        runtime_forward_energy: Some(0.2),
        runtime_kv_influence: Some(0.1),
        runtime_uncertainty_perplexity: None,
        recursive_runtime_calls: None,
    }];
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::new(
        vec![InfiniMemoryItem {
            id: 1,
            key: "local kv memory".to_owned(),
            vector: vec![0.1, 0.2, 0.3],
            scope: InfiniMemoryScope::LocalWindow,
            score: 0.91,
            estimated_tokens: 3,
            reason: "test local".to_owned(),
        }],
        Vec::new(),
        Vec::new(),
    );
    let transformer_plan = TransformerRefactorPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let context = GenerationContext {
        prompt: "build runtime",
        profile: TaskProfile::Coding,
        memories: &memories,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        tier_plan: &tier_plan,
        infini_memory_plan: &infini_memory_plan,
        recursive_schedule: &recursive_schedule,
        hardware_plan: &hardware_plan,
        experiences: &experiences,
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan: &transformer_plan,
    };
    let mut backend = RuntimeBackend::new(MockRuntime::default()).with_max_tokens(128);

    let draft = backend.generate(context);
    let seen = backend.runtime().seen.as_ref().unwrap();

    assert!(draft.answer.contains("1 memories and 1 experiences"));
    assert_eq!(seen.max_tokens, 128);
    assert_eq!(seen.runtime_metadata.model_id, "mock-self-transformer");
    assert_eq!(seen.runtime_metadata.native_context_window, 32_768);
    assert_eq!(seen.runtime_architecture.layer_count, 18);
    assert_eq!(seen.runtime_architecture.hidden_size, 128);
    assert_eq!(seen.runtime_architecture.attention_heads, 8);
    assert_eq!(seen.runtime_architecture.kv_heads, 4);
    assert_eq!(seen.runtime_architecture.local_window_tokens, 4096);
    assert!(seen.runtime_metadata.supports_kv_import);
    assert!(seen.runtime_metadata.supports_kv_export);
    assert_eq!(seen.memory_hints.len(), 1);
    assert_eq!(seen.infini_memory_hints.len(), 1);
    assert_eq!(seen.experience_hints.len(), 1);
    assert_eq!(seen.runtime_adapter_observations.len(), 1);
    assert_eq!(
        seen.runtime_adapter_observations[0].adapter,
        RuntimeAdapterHint::PortableRust
    );
    assert!(seen.runtime_adapter_observations[0].score > 0.70);
    assert!(!seen.recursive_schedule.requires_recursion);
    assert!(seen.hardware_plan.local_kv_token_budget > 0);
    assert!(seen.transformer_plan.is_empty());
}

#[test]
fn runtime_request_renders_metadata_experience_from_clean_gist() {
    let experiences = vec![ExperienceMatch {
        id: 7,
        prompt: "Conversation transcript:\nuser: 帮我用rust输出一段for循环代码\nassistant:"
            .to_owned(),
        lesson: "accepted_pattern quality=0.778 overlap=0.640 issues=0 max_severity=info"
            .to_owned(),
        quality: 0.78,
        score: 0.97,
        gist_hints: vec![
            "document:Conversation transcript importance=0.850 tokens=42 summary=这是一个 Rust for 循环代码示例，使用 for i in 0..10 并 println 输出"
                .to_owned(),
        ],
        reflection_issue_codes: Vec::new(),
        revision_actions: Vec::new(),
        process_reward: 0.78,
        reward_action: crate::process_reward::RewardAction::Reinforce,
        runtime_model_id: Some("mock-self-transformer".to_owned()),
        runtime_selected_adapter: Some("portable-rust".to_owned()),
        runtime_device_profile: None,
        runtime_primary_lane: None,
        runtime_fallback_lane: None,
        runtime_memory_mode: None,
        runtime_device_execution_source: None,
        runtime_forward_energy: Some(0.2),
        runtime_kv_influence: Some(0.1),
        runtime_uncertainty_perplexity: None,
        recursive_runtime_calls: None,
    }];
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let context = sample_generation_context(
        "帮我用rust输出一段for循环代码",
        &[],
        &experiences,
        &tier_plan,
        &infini_memory_plan,
        &recursive_schedule,
        &hardware_plan,
        &transformer_plan,
    );

    let request = RuntimeRequest::from_context(
        &context,
        128,
        RuntimeMetadata::new("mock-self-transformer", "mock-bpe", 32_768, 128),
        TransformerRuntimeArchitecture::new(18, 128, 8, 4, 4096),
    );

    assert_eq!(request.experience_hints.len(), 1);
    assert!(request.experience_hints[0].contains("Rust for 循环代码示例"));
    assert!(!request.experience_hints[0].contains("accepted_pattern"));
    assert!(!request.experience_hints[0].contains("Conversation transcript"));
}

#[test]
fn runtime_request_filters_adapter_observations_to_device_plan() {
    let experiences = vec![
        ExperienceMatch {
            id: 1,
            prompt: "prompt".to_owned(),
            lesson: "portable lesson".to_owned(),
            quality: 0.9,
            score: 0.88,
            gist_hints: Vec::new(),
            reflection_issue_codes: Vec::new(),
            revision_actions: Vec::new(),
            process_reward: 0.81,
            reward_action: crate::process_reward::RewardAction::Reinforce,
            runtime_model_id: Some("mock-self-transformer".to_owned()),
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_device_profile: None,
            runtime_primary_lane: None,
            runtime_fallback_lane: None,
            runtime_memory_mode: None,
            runtime_device_execution_source: None,
            runtime_forward_energy: Some(0.2),
            runtime_kv_influence: Some(0.1),
            runtime_uncertainty_perplexity: None,
            recursive_runtime_calls: None,
        },
        ExperienceMatch {
            id: 2,
            prompt: "prompt".to_owned(),
            lesson: "cuda lesson".to_owned(),
            quality: 0.99,
            score: 0.99,
            gist_hints: Vec::new(),
            reflection_issue_codes: Vec::new(),
            revision_actions: Vec::new(),
            process_reward: 0.99,
            reward_action: crate::process_reward::RewardAction::Reinforce,
            runtime_model_id: Some("mock-self-transformer".to_owned()),
            runtime_selected_adapter: Some("cuda".to_owned()),
            runtime_device_profile: None,
            runtime_primary_lane: None,
            runtime_fallback_lane: None,
            runtime_memory_mode: None,
            runtime_device_execution_source: None,
            runtime_forward_energy: Some(0.1),
            runtime_kv_influence: Some(0.9),
            runtime_uncertainty_perplexity: None,
            recursive_runtime_calls: None,
        },
    ];
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let context = sample_generation_context(
        "filter adapters",
        &[],
        &experiences,
        &tier_plan,
        &infini_memory_plan,
        &recursive_schedule,
        &hardware_plan,
        &transformer_plan,
    );
    let mut backend = RuntimeBackend::new(MockRuntime::default());

    let _draft = backend.generate(context);
    let seen = backend.runtime().seen.as_ref().unwrap();

    assert_eq!(seen.runtime_adapter_observations.len(), 1);
    assert_eq!(
        seen.runtime_adapter_observations[0].adapter,
        RuntimeAdapterHint::PortableRust
    );
}

#[test]
fn runtime_request_filters_adapter_observations_to_device_execution_contract() {
    let cpu_plan = HardwareAllocator::new().plan(
        HardwareSnapshot::new(DeviceClass::CpuOnly, 0.30, 0.10, 0.40, 0.20),
        TaskProfile::Coding,
        2048,
        HierarchyWeights::default(),
    );
    let experiences = vec![
        ExperienceMatch {
            id: 1,
            prompt: "prompt".to_owned(),
            lesson: "cpu portable lesson".to_owned(),
            quality: 0.8,
            score: 0.8,
            gist_hints: Vec::new(),
            reflection_issue_codes: Vec::new(),
            revision_actions: Vec::new(),
            process_reward: 0.8,
            reward_action: crate::process_reward::RewardAction::Reinforce,
            runtime_model_id: Some("mock-self-transformer".to_owned()),
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_device_profile: Some(cpu_plan.device.as_str().to_owned()),
            runtime_primary_lane: Some(cpu_plan.execution.primary_lane.as_str().to_owned()),
            runtime_fallback_lane: Some(cpu_plan.execution.fallback_lane.as_str().to_owned()),
            runtime_memory_mode: Some(cpu_plan.execution.memory_mode.as_str().to_owned()),
            runtime_device_execution_source: Some(
                RuntimeDiagnostics::runtime_reported_device_execution_source().to_owned(),
            ),
            runtime_forward_energy: Some(0.2),
            runtime_kv_influence: Some(0.2),
            runtime_uncertainty_perplexity: None,
            recursive_runtime_calls: None,
        },
        ExperienceMatch {
            id: 2,
            prompt: "prompt".to_owned(),
            lesson: "gpu portable lesson should not leak to cpu".to_owned(),
            quality: 0.99,
            score: 0.99,
            gist_hints: Vec::new(),
            reflection_issue_codes: Vec::new(),
            revision_actions: Vec::new(),
            process_reward: 0.99,
            reward_action: crate::process_reward::RewardAction::Reinforce,
            runtime_model_id: Some("mock-self-transformer".to_owned()),
            runtime_selected_adapter: Some("portable-rust".to_owned()),
            runtime_device_profile: Some(DeviceClass::DiscreteGpu.as_str().to_owned()),
            runtime_primary_lane: Some(ComputeLane::DiscreteGpu.as_str().to_owned()),
            runtime_fallback_lane: Some(ComputeLane::CpuPortable.as_str().to_owned()),
            runtime_memory_mode: Some(DeviceMemoryMode::GpuResident.as_str().to_owned()),
            runtime_device_execution_source: Some(
                RuntimeDiagnostics::runtime_reported_device_execution_source().to_owned(),
            ),
            runtime_forward_energy: Some(0.1),
            runtime_kv_influence: Some(0.9),
            runtime_uncertainty_perplexity: None,
            recursive_runtime_calls: None,
        },
    ];

    let observations = RuntimeAdapterObservation::from_experiences_for_hardware(
        &experiences,
        "mock-self-transformer",
        &cpu_plan,
    );

    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].experience_id, 1);
}

#[derive(Debug, Default, Clone)]
struct SelfDevelopedRuntime {
    imported_blocks: usize,
    imported_heads: Vec<usize>,
    imported_keys: Vec<Vec<f32>>,
}

impl ModelRuntime for SelfDevelopedRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata::new("noiron-dev-transformer", "noiron-wordpiece", 65_536, 256)
            .with_kv_exchange(true, true)
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        TransformerRuntimeArchitecture::new(24, 256, 8, 4, 8192)
    }

    fn tokenize(&self, prompt: &str) -> Result<Vec<RuntimeTokenId>, RuntimeError> {
        Ok(prompt
            .split_whitespace()
            .enumerate()
            .map(|(index, text)| RuntimeTokenId::new(10_000 + index as u32, text))
            .collect())
    }

    fn embed(&self, tokens: &[RuntimeTokenId]) -> Result<RuntimeEmbedding, RuntimeError> {
        Ok(RuntimeEmbedding::new(vec![tokens.len() as f32, 1.0, 0.5]))
    }

    fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        self.imported_blocks += blocks.len();
        self.imported_heads
            .extend(blocks.iter().map(|block| block.head));
        self.imported_keys
            .extend(blocks.iter().map(|block| block.key.clone()));
        Ok(blocks.len())
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        Ok(vec![RuntimeKvBlock::new(
            1,
            2,
            0,
            4,
            vec![0.1, 0.2],
            vec![0.3, 0.4],
        )])
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        Ok(RuntimeResponse::new(format!(
            "generated with {}",
            request.runtime_metadata.model_id
        )))
    }
}

#[test]
fn runtime_metadata_clamps_cold_kv_precision_to_hot_precision() {
    let metadata =
        RuntimeMetadata::new("compact-runtime", "tok", 4096, 128).with_kv_precision(4, 8);

    assert_eq!(metadata.hot_kv_precision_bits, 4);
    assert_eq!(metadata.cold_kv_precision_bits, 4);
    assert!(metadata.summary().contains("kv_bits=4/4"));
}

#[derive(Debug, Default, Clone)]
struct ManifestBoundRuntime {
    imported_blocks: usize,
}

impl ModelRuntime for ManifestBoundRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata::new(
            "manifest-bound-transformer",
            "noiron-wordpiece",
            65_536,
            256,
        )
        .with_kv_exchange(true, true)
        .with_kv_limits(1, 2)
        .with_kv_precision(4, 4)
    }

    fn import_kv(&mut self, blocks: &[RuntimeKvBlock]) -> Result<usize, RuntimeError> {
        self.imported_blocks += blocks.len();
        Ok(blocks.len())
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        Ok(RuntimeResponse::new(format!(
            "generated with max import {}",
            request.runtime_metadata.max_kv_import_blocks
        )))
    }
}

#[derive(Debug, Default, Clone)]
struct ContractViolatingRuntime {
    exported: bool,
}

impl ModelRuntime for ContractViolatingRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata::new("contract-runtime", "noiron-wordpiece", 65_536, 256)
            .with_kv_exchange(false, true)
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        TransformerRuntimeArchitecture::new(24, 256, 8, 4, 8192)
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        self.exported = true;
        Ok(vec![RuntimeKvBlock::new(
            2,
            0,
            0,
            2,
            vec![0.1, 0.2],
            vec![0.3, 0.4],
        )])
    }

    fn generate(&mut self, _request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        Ok(
            RuntimeResponse::new("contract violating answer").with_diagnostics(
                RuntimeDiagnostics {
                    model_id: Some("contract-runtime".to_owned()),
                    selected_adapter: Some("cuda".to_owned()),
                    layer_count: 24,
                    hidden_size: 256,
                    local_window_tokens: 8192,
                    forward_energy: Some(0.2),
                    kv_influence: Some(0.1),
                    imported_kv_blocks: 0,
                    exported_kv_blocks: 1,
                    ..RuntimeDiagnostics::default()
                },
            ),
        )
    }
}

#[derive(Debug, Default, Clone)]
struct UnsafeExportRuntime;

impl ModelRuntime for UnsafeExportRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata::new("unsafe-export-runtime", "tok", 4096, 16).with_kv_exchange(false, true)
    }

    fn architecture(&self) -> TransformerRuntimeArchitecture {
        TransformerRuntimeArchitecture::new(4, 16, 4, 2, 1024)
    }

    fn export_kv(&mut self) -> Result<Vec<RuntimeKvBlock>, RuntimeError> {
        Ok(vec![
            RuntimeKvBlock::new(0, 0, 4, 4, vec![0.1], vec![0.2]),
            RuntimeKvBlock::new(1, 1, 0, 1, vec![0.3], vec![0.4]),
        ])
    }

    fn generate(&mut self, _request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        Ok(RuntimeResponse::new("unsafe export filtered"))
    }
}

#[derive(Debug, Default, Clone)]
struct DeviceSilentRuntime;

impl ModelRuntime for DeviceSilentRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata::new("device-silent-runtime", "tok", 4096, 64)
    }

    fn generate(&mut self, _request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        Ok(
            RuntimeResponse::new("device silent").with_diagnostics(RuntimeDiagnostics {
                model_id: Some("device-silent-runtime".to_owned()),
                layer_count: 1,
                forward_energy: Some(0.1),
                ..RuntimeDiagnostics::default()
            }),
        )
    }
}

#[derive(Debug, Default, Clone)]
struct DeviceReportingRuntime;

impl ModelRuntime for DeviceReportingRuntime {
    fn metadata(&self) -> RuntimeMetadata {
        RuntimeMetadata::new("device-reporting-runtime", "tok", 4096, 64)
    }

    fn generate(&mut self, request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        let execution = &request.hardware_plan.execution;
        Ok(RuntimeResponse::new("device reporting").with_diagnostics(
            RuntimeDiagnostics::default().with_device_execution(
                request.hardware_plan.device.as_str(),
                execution.primary_lane.as_str(),
                execution.fallback_lane.as_str(),
                execution.memory_mode.as_str(),
            ),
        ))
    }
}

#[test]
fn self_developed_runtime_abi_exposes_tokens_embeddings_and_kv_exchange() {
    let mut runtime = SelfDevelopedRuntime::default();

    let metadata = runtime.metadata();
    let architecture = runtime.architecture();
    let tokens = runtime.tokenize("alpha beta").unwrap();
    let embedding = runtime.embed(&tokens).unwrap();
    let text_embedding = runtime.embed_text("alpha beta gamma").unwrap();
    let imported = runtime
        .import_kv(&[RuntimeKvBlock::new(
            0,
            1,
            0,
            2,
            vec![0.1, 0.2],
            vec![0.3, 0.4],
        )])
        .unwrap();
    let exported = runtime.export_kv().unwrap();

    assert_eq!(metadata.model_id, "noiron-dev-transformer");
    assert_eq!(metadata.tokenizer, "noiron-wordpiece");
    assert_eq!(architecture.layer_count, 24);
    assert_eq!(architecture.hidden_size, 256);
    assert_eq!(architecture.attention_heads, 8);
    assert_eq!(architecture.kv_heads, 4);
    assert_eq!(architecture.local_window_tokens, 8192);
    assert_eq!(tokens[0], RuntimeTokenId::new(10_000, "alpha"));
    assert_eq!(embedding.dimensions, 3);
    assert_eq!(text_embedding.values, vec![3.0, 1.0, 0.5]);
    assert_eq!(imported, 1);
    assert_eq!(runtime.imported_blocks, 1);
    assert_eq!(exported[0].layer, 1);
    assert_eq!(exported[0].head, 2);
}

#[test]
fn runtime_backend_marks_control_plane_filled_device_execution() {
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let context = GenerationContext {
        prompt: "silent runtime device",
        profile: TaskProfile::General,
        memories: &[],
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::default(),
        tier_plan: &tier_plan,
        infini_memory_plan: &infini_memory_plan,
        recursive_schedule: &recursive_schedule,
        hardware_plan: &hardware_plan,
        experiences: &[],
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan: &transformer_plan,
    };
    let mut backend = RuntimeBackend::new(DeviceSilentRuntime);

    let draft = backend.generate(context);

    assert!(draft.runtime_diagnostics.has_device_execution_signal());
    assert!(
        draft
            .runtime_diagnostics
            .has_control_plane_filled_device_execution_signal()
    );
    assert!(
        !draft
            .runtime_diagnostics
            .has_runtime_reported_device_execution_signal()
    );
}

#[test]
fn runtime_backend_preserves_runtime_reported_device_execution() {
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let context = GenerationContext {
        prompt: "reported runtime device",
        profile: TaskProfile::General,
        memories: &[],
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::default(),
        tier_plan: &tier_plan,
        infini_memory_plan: &infini_memory_plan,
        recursive_schedule: &recursive_schedule,
        hardware_plan: &hardware_plan,
        experiences: &[],
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan: &transformer_plan,
    };
    let mut backend = RuntimeBackend::new(DeviceReportingRuntime);

    let draft = backend.generate(context);

    assert!(
        draft
            .runtime_diagnostics
            .has_runtime_reported_device_execution_signal()
    );
    assert!(
        !draft
            .runtime_diagnostics
            .has_control_plane_filled_device_execution_signal()
    );
}

#[test]
fn runtime_backend_exposes_model_side_embeddings() {
    let mut backend = RuntimeBackend::new(SelfDevelopedRuntime::default());

    let embedding = backend.embed_text("alpha beta").unwrap();

    assert_eq!(embedding, vec![2.0, 1.0, 0.5]);
}

#[test]
fn runtime_backend_can_drive_rust_native_adapter_bridge_with_chunked_kv_hooks() {
    let memories = vec![MemoryMatch {
        id: 38,
        key: "runtime bridge kv".to_owned(),
        similarity: 0.91,
        strength: 0.85,
        vector: vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
    }];
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let context = sample_generation_context(
        "drive rust native adapter through runtime backend",
        &memories,
        &[],
        &tier_plan,
        &infini_memory_plan,
        &recursive_schedule,
        &hardware_plan,
        &transformer_plan,
    );
    let runtime = RustNativeModelRuntime::new(MockRustNativeAdapter::new())
        .with_cache_mode(ChunkedKvCacheMode::ChunkedCache);
    let mut backend = RuntimeBackend::new(runtime).with_max_tokens(32);
    let mut streamed = Vec::new();

    let draft = backend.generate_stream(context, &mut |token| {
        streamed.push(token.text.clone());
    });
    let report = backend.runtime().last_report().expect("adapter report");

    assert!(draft.answer.contains("rust-native:runtime-"));
    assert_eq!(streamed.len(), 4);
    assert_eq!(streamed[0], "rust-native");
    assert!(streamed[1].starts_with("runtime-"));
    assert_eq!(streamed[2], "chunked_cache");
    assert_eq!(streamed[3], "1");
    assert_eq!(report.included_segments(), 1);
    assert_eq!(report.imported_kv_blocks, 1);
    assert_eq!(draft.runtime_diagnostics.imported_kv_blocks, 1);
    assert_eq!(draft.runtime_diagnostics.exported_kv_blocks, 1);
    assert_eq!(draft.runtime_diagnostics.runtime_kv_segments_included, 1);
    assert_eq!(draft.runtime_diagnostics.runtime_kv_segments_skipped, 0);
    assert_eq!(draft.runtime_diagnostics.runtime_kv_segments_rejected, 0);
    assert!(draft.runtime_diagnostics.has_runtime_kv_segment_signal());
    assert_eq!(draft.exported_kv_blocks.len(), 1);
    assert_eq!(draft.exported_kv_blocks[0].key.len(), 8);
    assert!(draft.trace.iter().any(|step| {
        step.label == "rust_native_chunked_kv" && step.content.contains("included=1")
    }));
    assert!(
        report
            .hook_summaries()
            .iter()
            .all(|summary| summary.contains("cache_ref=fnv64:"))
    );
}

#[test]
fn runtime_native_bridge_uses_adaptive_kv_attention_thresholds() {
    let memories = vec![
        MemoryMatch {
            id: 38,
            key: "strong runtime kv".to_owned(),
            similarity: 0.94,
            strength: 0.95,
            vector: vec![0.4; 8],
        },
        MemoryMatch {
            id: 39,
            key: "weak runtime kv".to_owned(),
            similarity: 0.80,
            strength: 0.10,
            vector: vec![0.4; 8],
        },
    ];
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let mut context = sample_generation_context(
        "prefer strong kv under tight runtime budget",
        &memories,
        &[],
        &tier_plan,
        &infini_memory_plan,
        &recursive_schedule,
        &hardware_plan,
        &transformer_plan,
    );
    context.route_budget.threshold = 0.35;
    let runtime = RustNativeModelRuntime::new(MockRustNativeAdapter::new())
        .with_cache_mode(ChunkedKvCacheMode::ChunkedCache);
    let mut backend = RuntimeBackend::new(runtime).with_max_tokens(32);

    let draft = backend.generate(context);
    let report = backend.runtime().last_report().expect("adapter report");
    let summaries = report.hook_summaries();

    assert_eq!(report.included_segments(), 1);
    assert_eq!(report.skipped_segments(), 1);
    assert_eq!(report.imported_kv_blocks, 1);
    assert_eq!(draft.runtime_diagnostics.runtime_kv_segments_included, 1);
    assert_eq!(draft.runtime_diagnostics.runtime_kv_segments_skipped, 1);
    assert_eq!(draft.runtime_diagnostics.runtime_kv_segments_rejected, 0);
    assert_eq!(draft.runtime_diagnostics.runtime_kv_segment_count(), 2);
    assert_eq!(draft.exported_kv_blocks.len(), 1);
    assert!(summaries.iter().any(|summary| {
        summary.contains("segment=runtime-import-0")
            && summary.contains("decision=include")
            && summary.contains("attention_threshold=0.228")
    }));
    assert!(summaries.iter().any(|summary| {
        summary.contains("segment=runtime-import-1")
            && summary.contains("decision=skip")
            && summary.contains("reason=attention_threshold_above_budget")
            && summary.contains("attention_threshold=0.695")
    }));
    assert!(draft.trace.iter().any(|step| {
        step.label == "rust_native_chunked_kv"
            && step.content.contains("included=1 skipped=1 rejected=0")
    }));
}

#[test]
fn runtime_backend_imports_memory_kv_and_returns_exported_blocks() {
    let memories = vec![MemoryMatch {
        id: 7,
        key: "hot runtime memory".to_owned(),
        similarity: 0.91,
        strength: 1.25,
        vector: vec![0.1, 0.2, 0.3],
    }];
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let context = GenerationContext {
        prompt: "use runtime kv",
        profile: TaskProfile::Coding,
        memories: &memories,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        tier_plan: &tier_plan,
        infini_memory_plan: &infini_memory_plan,
        recursive_schedule: &recursive_schedule,
        hardware_plan: &hardware_plan,
        experiences: &[],
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan: &transformer_plan,
    };
    let mut backend = RuntimeBackend::new(SelfDevelopedRuntime::default());

    let draft = backend.generate(context);

    assert_eq!(backend.runtime().imported_blocks, 1);
    assert_eq!(backend.runtime().imported_heads, vec![0]);
    assert_eq!(draft.exported_kv_blocks.len(), 1);
    assert!(
        draft
            .trace
            .iter()
            .any(|step| step.label == "runtime_kv_import")
    );
    assert!(
        draft
            .trace
            .iter()
            .any(|step| step.label == "runtime_kv_export")
    );
}

#[test]
fn runtime_backend_rejects_non_finite_imported_kv_before_runtime_call() {
    let memories = vec![MemoryMatch {
        id: 7,
        key: "unsafe runtime memory".to_owned(),
        similarity: 0.91,
        strength: 1.25,
        vector: vec![f32::NAN, 0.2, 0.3],
    }];
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let context = GenerationContext {
        prompt: "reject unsafe runtime kv",
        profile: TaskProfile::Coding,
        memories: &memories,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        tier_plan: &tier_plan,
        infini_memory_plan: &infini_memory_plan,
        recursive_schedule: &recursive_schedule,
        hardware_plan: &hardware_plan,
        experiences: &[],
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan: &transformer_plan,
    };
    let mut backend = RuntimeBackend::new(SelfDevelopedRuntime::default());

    let draft = backend.generate(context);

    assert_eq!(backend.runtime().imported_blocks, 0);
    assert!(
        !draft
            .trace
            .iter()
            .any(|step| step.label == "runtime_kv_import")
    );
    assert!(draft.trace.iter().any(|step| {
        step.label == "runtime_kv_import_safety"
            && step.content.contains("non-finite")
            && step.confidence < 0.20
    }));
}

#[test]
fn runtime_backend_bounds_imported_kv_heads_to_runtime_architecture() {
    let memories = (0..6)
        .map(|index| MemoryMatch {
            id: 100 + index,
            key: format!("hot runtime memory {index}"),
            similarity: 0.95,
            strength: 1.10,
            vector: vec![0.1 + index as f32, 0.2, 0.3],
        })
        .collect::<Vec<_>>();
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let mut hardware_plan = HardwarePlan::default();
    hardware_plan.execution.kv_prefetch_blocks = 6;
    let context = GenerationContext {
        prompt: "bound runtime kv heads",
        profile: TaskProfile::Coding,
        memories: &memories,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        tier_plan: &tier_plan,
        infini_memory_plan: &infini_memory_plan,
        recursive_schedule: &recursive_schedule,
        hardware_plan: &hardware_plan,
        experiences: &[],
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan: &transformer_plan,
    };
    let mut backend = RuntimeBackend::new(SelfDevelopedRuntime::default());

    let _draft = backend.generate(context);

    assert_eq!(backend.runtime().imported_blocks, 6);
    assert_eq!(backend.runtime().imported_heads, vec![0, 1, 2, 3, 0, 1]);
}

#[test]
fn runtime_kv_import_uses_infini_sparse_plan_before_active_memories() {
    let memories = vec![
        MemoryMatch {
            id: 7,
            key: "local planned memory".to_owned(),
            similarity: 0.95,
            strength: 1.25,
            vector: vec![0.1, 0.2, 0.3],
        },
        MemoryMatch {
            id: 8,
            key: "sparse skipped memory".to_owned(),
            similarity: 0.94,
            strength: 1.20,
            vector: vec![0.4, 0.5, 0.6],
        },
        MemoryMatch {
            id: 9,
            key: "global planned memory".to_owned(),
            similarity: 0.20,
            strength: 1.60,
            vector: vec![0.7, 0.8, 0.9],
        },
    ];
    let infini_memory_plan = InfiniMemoryPlan::new(
        vec![InfiniMemoryItem {
            id: 7,
            key: "local planned memory".to_owned(),
            vector: vec![0.1, 0.2, 0.3],
            scope: InfiniMemoryScope::LocalWindow,
            score: 0.92,
            estimated_tokens: 3,
            reason: "local_window:test".to_owned(),
        }],
        vec![InfiniMemoryItem {
            id: 9,
            key: "global planned memory".to_owned(),
            vector: vec![0.7, 0.8, 0.9],
            scope: InfiniMemoryScope::GlobalMemory,
            score: 0.84,
            estimated_tokens: 3,
            reason: "global_memory:test".to_owned(),
        }],
        vec![InfiniMemoryItem {
            id: 8,
            key: "sparse skipped memory".to_owned(),
            vector: vec![0.4, 0.5, 0.6],
            scope: InfiniMemoryScope::Skipped,
            score: 0.90,
            estimated_tokens: 3,
            reason: "sparse_filter:test".to_owned(),
        }],
    );
    let tier_plan = TieredCachePlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let mut hardware_plan = HardwarePlan::default();
    hardware_plan.execution.kv_prefetch_blocks = 3;
    let context = GenerationContext {
        prompt: "sparse runtime kv",
        profile: TaskProfile::Coding,
        memories: &memories,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        tier_plan: &tier_plan,
        infini_memory_plan: &infini_memory_plan,
        recursive_schedule: &recursive_schedule,
        hardware_plan: &hardware_plan,
        experiences: &[],
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan: &transformer_plan,
    };
    let mut backend = RuntimeBackend::new(SelfDevelopedRuntime::default());

    let _draft = backend.generate(context);

    assert_eq!(backend.runtime().imported_blocks, 2);
    assert_eq!(backend.runtime().imported_heads, vec![0, 1]);
}

#[test]
fn runtime_kv_import_skips_weak_runtime_kv_without_consuming_prefetch() {
    let memories = vec![
        MemoryMatch {
            id: 7,
            key: "runtime_kv:l0h0:0-1 :: low yield".to_owned(),
            similarity: 0.99,
            strength: 0.30,
            vector: vec![0.1, 0.2, 0.3],
        },
        MemoryMatch {
            id: 8,
            key: "semantic runtime fallback evidence".to_owned(),
            similarity: 0.88,
            strength: 0.82,
            vector: vec![0.7, 0.8, 0.9],
        },
    ];
    let infini_memory_plan = InfiniMemoryPlan::new(
        vec![
            InfiniMemoryItem {
                id: 7,
                key: "runtime_kv:l0h0:0-1 :: low yield".to_owned(),
                vector: vec![0.1, 0.2, 0.3],
                scope: InfiniMemoryScope::LocalWindow,
                score: 0.99,
                estimated_tokens: 4,
                reason: "local_window:test".to_owned(),
            },
            InfiniMemoryItem {
                id: 8,
                key: "semantic runtime fallback evidence".to_owned(),
                vector: vec![0.7, 0.8, 0.9],
                scope: InfiniMemoryScope::LocalWindow,
                score: 0.72,
                estimated_tokens: 4,
                reason: "local_window:test".to_owned(),
            },
        ],
        Vec::new(),
        Vec::new(),
    );
    let tier_plan = TieredCachePlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let mut hardware_plan = HardwarePlan::default();
    hardware_plan.execution.kv_prefetch_blocks = 1;
    let context = GenerationContext {
        prompt: "skip weak runtime kv import",
        profile: TaskProfile::Coding,
        memories: &memories,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        tier_plan: &tier_plan,
        infini_memory_plan: &infini_memory_plan,
        recursive_schedule: &recursive_schedule,
        hardware_plan: &hardware_plan,
        experiences: &[],
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan: &transformer_plan,
    };
    let mut backend = RuntimeBackend::new(SelfDevelopedRuntime::default());

    let draft = backend.generate(context);

    assert_eq!(backend.runtime().imported_blocks, 1);
    assert_eq!(backend.runtime().imported_keys.len(), 1);
    assert_eq!(&backend.runtime().imported_keys[0][..3], &[0.7, 0.8, 0.9]);
    assert_eq!(backend.runtime().imported_keys[0].len(), 256);
    assert_eq!(draft.runtime_diagnostics.weak_runtime_kv_imports_skipped, 1);
    assert!(draft.trace.iter().any(|step| {
        step.label == "runtime_kv_import_selection"
            && step
                .content
                .contains("skipped 1 weak runtime KV candidates")
    }));
}

#[test]
fn runtime_kv_import_does_not_fallback_when_infini_skips_everything() {
    let memories = vec![MemoryMatch {
        id: 8,
        key: "skipped memory should stay out".to_owned(),
        similarity: 0.94,
        strength: 1.20,
        vector: vec![0.4, 0.5, 0.6],
    }];
    let infini_memory_plan = InfiniMemoryPlan::new(
        Vec::new(),
        Vec::new(),
        vec![InfiniMemoryItem {
            id: 8,
            key: "skipped memory should stay out".to_owned(),
            vector: vec![0.4, 0.5, 0.6],
            scope: InfiniMemoryScope::Skipped,
            score: 0.90,
            estimated_tokens: 5,
            reason: "sparse_filter:test".to_owned(),
        }],
    );
    let tier_plan = TieredCachePlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let context = GenerationContext {
        prompt: "skip all sparse runtime kv",
        profile: TaskProfile::Coding,
        memories: &memories,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        tier_plan: &tier_plan,
        infini_memory_plan: &infini_memory_plan,
        recursive_schedule: &recursive_schedule,
        hardware_plan: &hardware_plan,
        experiences: &[],
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan: &transformer_plan,
    };
    let mut backend = RuntimeBackend::new(SelfDevelopedRuntime::default());

    let draft = backend.generate(context);

    assert_eq!(backend.runtime().imported_blocks, 0);
    assert!(
        !draft
            .trace
            .iter()
            .any(|step| step.label == "runtime_kv_import")
    );
}

#[test]
fn runtime_kv_import_respects_device_prefetch_budget() {
    let memories = vec![
        MemoryMatch {
            id: 7,
            key: "hot runtime memory one".to_owned(),
            similarity: 0.95,
            strength: 1.25,
            vector: vec![0.1, 0.2, 0.3],
        },
        MemoryMatch {
            id: 8,
            key: "hot runtime memory two".to_owned(),
            similarity: 0.90,
            strength: 1.10,
            vector: vec![0.4, 0.5, 0.6],
        },
    ];
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let mut hardware_plan = HardwarePlan::default();
    hardware_plan.execution.kv_prefetch_blocks = 1;
    let context = GenerationContext {
        prompt: "limit runtime kv",
        profile: TaskProfile::Coding,
        memories: &memories,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        tier_plan: &tier_plan,
        infini_memory_plan: &infini_memory_plan,
        recursive_schedule: &recursive_schedule,
        hardware_plan: &hardware_plan,
        experiences: &[],
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan: &transformer_plan,
    };
    let mut backend = RuntimeBackend::new(SelfDevelopedRuntime::default());

    let draft = backend.generate(context);

    assert_eq!(backend.runtime().imported_blocks, 1);
    assert_eq!(draft.exported_kv_blocks.len(), 1);
}

#[test]
fn runtime_kv_import_respects_manifest_import_limit() {
    let memories = vec![
        MemoryMatch {
            id: 7,
            key: "hot runtime memory one".to_owned(),
            similarity: 0.95,
            strength: 1.25,
            vector: vec![0.1, 0.2, 0.3],
        },
        MemoryMatch {
            id: 8,
            key: "hot runtime memory two".to_owned(),
            similarity: 0.90,
            strength: 1.10,
            vector: vec![0.4, 0.5, 0.6],
        },
    ];
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let mut hardware_plan = HardwarePlan::default();
    hardware_plan.execution.kv_prefetch_blocks = 4;
    let context = GenerationContext {
        prompt: "limit runtime kv with manifest",
        profile: TaskProfile::Coding,
        memories: &memories,
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        tier_plan: &tier_plan,
        infini_memory_plan: &infini_memory_plan,
        recursive_schedule: &recursive_schedule,
        hardware_plan: &hardware_plan,
        experiences: &[],
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan: &transformer_plan,
    };
    let mut backend = RuntimeBackend::new(ManifestBoundRuntime::default());

    let draft = backend.generate(context);

    assert_eq!(backend.runtime().imported_blocks, 1);
    assert!(draft.answer.contains("max import 1"));
}

#[test]
fn runtime_response_contract_blocks_out_of_device_adapter_and_kv_export() {
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let context = sample_generation_context(
        "contract gate",
        &[],
        &[],
        &tier_plan,
        &infini_memory_plan,
        &recursive_schedule,
        &hardware_plan,
        &transformer_plan,
    );
    let mut backend = RuntimeBackend::new(ContractViolatingRuntime::default());

    let draft = backend.generate(context);

    assert!(draft.answer.contains("contract violating answer"));
    assert!(draft.exported_kv_blocks.is_empty());
    assert!(!backend.runtime().exported);
    assert_eq!(draft.runtime_diagnostics.selected_adapter, None);
    assert_eq!(draft.runtime_diagnostics.exported_kv_blocks, 0);
    assert!(draft.trace.iter().any(|step| {
        step.label == "runtime_contract_violation"
            && step
                .content
                .contains("outside device execution adapter hints")
            && step.confidence < 0.10
    }));
}

#[test]
fn runtime_backend_filters_unsafe_exported_kv_blocks() {
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let context = sample_generation_context(
        "filter unsafe export",
        &[],
        &[],
        &tier_plan,
        &infini_memory_plan,
        &recursive_schedule,
        &hardware_plan,
        &transformer_plan,
    );
    let mut backend = RuntimeBackend::new(UnsafeExportRuntime);

    let draft = backend.generate(context);

    assert_eq!(draft.exported_kv_blocks.len(), 1);
    assert_eq!(draft.exported_kv_blocks[0].layer, 1);
    assert!(draft.trace.iter().any(|step| {
        step.label == "runtime_kv_export_safety"
            && step.content.contains("token range is empty")
            && step.confidence < 0.20
    }));
}

#[test]
fn default_runtime_abi_keeps_command_runtime_compatible() {
    let runtime = CommandRuntime::new("runner");

    let metadata = runtime.metadata();
    let tokens = runtime.tokenize("fallback tokenize").unwrap();
    let embedding = runtime.embed(&tokens).unwrap();

    assert_eq!(metadata, RuntimeMetadata::default());
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].text, "fallback");
    assert_eq!(embedding.dimensions, 0);
}

#[derive(Debug, Default, Clone)]
struct FailingRuntime;

impl ModelRuntime for FailingRuntime {
    fn generate(&mut self, _request: RuntimeRequest) -> Result<RuntimeResponse, RuntimeError> {
        Err(RuntimeError::new("model file missing"))
    }
}

#[test]
fn runtime_errors_become_low_confidence_drafts() {
    let tier_plan = TieredCachePlan::default();
    let infini_memory_plan = InfiniMemoryPlan::default();
    let transformer_plan = TransformerRefactorPlan::default();
    let recursive_schedule = RecursiveSchedule::default();
    let hardware_plan = HardwarePlan::default();
    let context = GenerationContext {
        prompt: "build runtime",
        profile: TaskProfile::Coding,
        memories: &[],
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 1,
            fast_tokens: 1,
            attention_fraction: 0.5,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        tier_plan: &tier_plan,
        infini_memory_plan: &infini_memory_plan,
        recursive_schedule: &recursive_schedule,
        hardware_plan: &hardware_plan,
        experiences: &[],
        toolsmith_plan: default_toolsmith_plan(),
        agent_team_plan: default_agent_team_plan(),
        transformer_plan: &transformer_plan,
    };
    let mut backend = RuntimeBackend::new(FailingRuntime);

    let draft = backend.generate(context);

    assert!(draft.answer.contains("model file missing"));
    assert_eq!(draft.trace[0].confidence, 0.0);
    assert_eq!(
        backend.last_error().unwrap().message(),
        "model file missing"
    );
}

#[test]
fn command_runtime_formats_prompt_and_expands_placeholders() {
    let metadata = RuntimeMetadata::new("command-model", "command-tokenizer", 16_384, 384)
        .with_kv_exchange(true, false);
    let runtime = CommandRuntime::new("runner")
        .with_metadata(metadata)
        .arg("--prompt")
        .arg("{prompt}")
        .arg("--user-prompt")
        .arg("{user_prompt}")
        .arg("--task-prompt")
        .arg("{task_prompt}")
        .arg("--max")
        .arg("{max_tokens}")
        .arg("--runtime")
        .arg("{runtime_metadata}")
        .arg("--architecture")
        .arg("{runtime_architecture}")
        .arg("--device-contract")
        .arg("{runtime_device_contract}")
        .arg("--imported")
        .arg("{imported_kv_blocks}")
        .prompt_mode(CommandPromptMode::Args);
    let request = sample_request();
    let prompt = format_runtime_prompt(&request);
    let args = runtime.expanded_args(&request, &prompt);

    assert!(prompt.contains("runtime:"));
    assert!(prompt.contains("runtime_architecture:"));
    assert!(prompt.contains("model_id=sample-transformer"));
    assert!(prompt.contains("native_context_window=8192"));
    assert!(prompt.contains("layers=16"));
    assert!(prompt.contains("attention_heads=8"));
    assert!(prompt.contains("local_window=2048"));
    assert!(prompt.contains("max_kv_import_blocks=8"));
    assert!(prompt.contains("kv_bits=8/4"));
    assert!(prompt.contains("memory_hints"));
    assert!(prompt.contains("infini_memory_hints"));
    assert!(prompt.contains("experience_hints"));
    assert!(prompt.contains("recursive:"));
    assert!(prompt.contains("hardware:"));
    assert!(prompt.contains("runtime_device_contract:"));
    assert!(prompt.contains("imported_kv_blocks:"));
    assert!(prompt.contains("layer=1 head=0 tokens=0..1 key_dims=2 value_dims=2"));
    assert!(prompt.contains("primary=cpu-vector"));
    assert!(prompt.contains("fallback=cpu-portable"));
    assert!(prompt.contains("kv_prefetch="));
    assert!(prompt.contains("transformer: template=none"));
    assert!(
        prompt.contains(
            "task_intent: language=english coding_language=unspecified rust_coding=false"
        )
    );
    assert!(args[1].contains("Noiron runtime request"));
    assert_eq!(args[3], request.prompt);
    assert_eq!(args[5], request.prompt);
    assert_eq!(args[7], "64");
    assert!(args[9].contains("model_id=sample-transformer"));
    assert!(args[11].contains("layers=16"));
    assert!(args[13].contains("primary=cpu-vector"));
    assert!(args[13].contains("adapters="));
    assert!(args[13].contains("portable-rust"));
    assert!(args[15].contains("layer=1 head=0"));
}

#[test]
fn runtime_request_json_includes_control_plane_sections() {
    let request = sample_request();

    let payload = runtime_request_json(&request);

    assert_eq!(
        extract_json_string_field(&payload, "schema").unwrap(),
        "rust-norion-runtime-request-v1"
    );
    assert_eq!(
        extract_json_string_field(&payload, "profile").unwrap(),
        "coding"
    );
    assert_eq!(
        extract_json_string_field(&payload, "language_mode").unwrap(),
        "english"
    );
    assert_eq!(
        extract_json_string_field(&payload, "coding_language").unwrap(),
        "unspecified"
    );
    assert!(payload.contains("\"rust_coding\":false"));
    assert_eq!(
        extract_json_string_field(&payload, "model_id").unwrap(),
        "sample-transformer"
    );
    assert_eq!(
        extract_json_number_field(&payload, "max_kv_import_blocks").unwrap(),
        8.0
    );
    assert_eq!(
        extract_json_number_field(&payload, "max_kv_export_blocks").unwrap(),
        4.0
    );
    assert_eq!(
        extract_json_number_field(&payload, "hot_kv_precision_bits").unwrap(),
        8.0
    );
    assert_eq!(
        extract_json_number_field(&payload, "cold_kv_precision_bits").unwrap(),
        4.0
    );
    assert_eq!(
        extract_json_number_field(&payload, "layer_count").unwrap(),
        16.0
    );
    assert_eq!(
        extract_json_number_field(&payload, "hidden_size").unwrap(),
        64.0
    );
    assert_eq!(
        extract_json_number_field(&payload, "attention_heads").unwrap(),
        8.0
    );
    assert_eq!(
        extract_json_number_field(&payload, "kv_heads").unwrap(),
        4.0
    );
    assert_eq!(
        extract_json_number_field(&payload, "local_window_tokens").unwrap(),
        2048.0
    );
    assert_eq!(
        extract_json_number_field(&payload, "attention_tokens").unwrap(),
        2.0
    );
    assert_eq!(
        extract_json_string_field(&payload, "primary_lane").unwrap(),
        "cpu-vector"
    );
    assert_eq!(
        extract_json_string_field(&payload, "runtime_device_contract").unwrap(),
        request.hardware_plan.runtime_contract_summary()
    );
    assert!(payload.contains("\"execution_waves\""));
    assert!(payload.contains("\"max_parallel_chunks\""));
    assert!(payload.contains("\"template\":\"none\""));
    assert!(payload.contains("\"memory_hints\":[\"memory hint\"]"));
    assert!(payload.contains("\"imported_kv_blocks\":["));
    assert!(payload.contains("\"layer\":1"));
    assert!(payload.contains("\"key\":[0.100000,0.200000]"));
    assert_eq!(extract_json_array_field(&payload, "layers").unwrap(), "[]");
    assert!(payload.contains("\"runtime_adapter_observations\":["));
    assert!(payload.contains("\"adapter\":\"cpu-simd\""));
    assert!(payload.contains("\"experience_id\":9"));
}

#[test]
fn runtime_request_wire_marks_chinese_rust_coding_intent() {
    let mut request = sample_request();
    request.prompt = "请用中文解释 Rust 所有权，并给出 cargo test 建议".to_owned();

    let text_payload = format_runtime_prompt(&request);
    let json_payload = runtime_request_json(&request);

    assert!(
        text_payload
            .contains("task_intent: language=chinese coding_language=rust rust_coding=true")
    );
    assert_eq!(
        extract_json_string_field(&json_payload, "language_mode").unwrap(),
        "chinese"
    );
    assert_eq!(
        extract_json_string_field(&json_payload, "coding_language").unwrap(),
        "rust"
    );
    assert!(json_payload.contains("\"rust_coding\":true"));
}

#[test]
fn runtime_response_json_parses_tokens_and_trace() {
    let payload = r#"{
            "schema": "rust-norion-runtime-response-v1",
            "answer": "structured runtime answer",
            "tokens": [
                {"text": "structured", "logprob": -0.2, "entropy": 0.3},
                {"text": "answer", "entropy": 0.4}
            ],
            "trace": [
                {"label": "runtime", "content": "generated with JSON ABI", "confidence": 0.91}
            ],
            "exported_kv_blocks": [
                {
                    "layer": 1,
                    "head": 0,
                    "token_start": 0,
                    "token_end": 1,
                    "key": [0.1, 0.2],
                    "value": [0.3, 0.4]
                }
            ],
            "diagnostics": {
                "model_id": "json-self-runtime",
                "selected_adapter": "portable-rust",
                "adapter_cache_mode": "chunked_cache",
                "layer_count": 24,
                "hidden_size": 256,
                "local_window_tokens": 4096,
                "forward_energy": 0.42,
                "kv_influence": 0.18,
                "imported_kv_blocks": 2,
                "weak_runtime_kv_imports_skipped": 1,
                "exported_kv_blocks": 3,
                "runtime_kv_segments_included": 2,
                "runtime_kv_segments_skipped": 1,
                "runtime_kv_segments_rejected": 0,
                "hot_kv_precision_bits": 8,
                "cold_kv_precision_bits": 4
            }
        }"#;

    let response = parse_runtime_response_json(payload).unwrap();

    assert_eq!(response.answer, "structured runtime answer");
    assert_eq!(response.tokens.len(), 2);
    assert_eq!(response.tokens[0].logprob, Some(-0.2));
    assert_eq!(response.tokens[1].entropy, Some(0.4));
    assert_eq!(response.trace[0].label, "runtime");
    assert!((response.trace[0].confidence - 0.91).abs() < 0.0001);
    assert_eq!(
        response.diagnostics.model_id.as_deref(),
        Some("json-self-runtime")
    );
    assert_eq!(
        response.diagnostics.selected_adapter.as_deref(),
        Some("portable-rust")
    );
    assert_eq!(
        response.diagnostics.adapter_cache_mode.as_deref(),
        Some("chunked_cache")
    );
    assert_eq!(response.diagnostics.layer_count, 24);
    assert_eq!(response.diagnostics.hidden_size, 256);
    assert_eq!(response.diagnostics.local_window_tokens, 4096);
    assert_eq!(response.diagnostics.forward_energy, Some(0.42));
    assert_eq!(response.diagnostics.kv_influence, Some(0.18));
    assert_eq!(response.diagnostics.imported_kv_blocks, 2);
    assert_eq!(response.diagnostics.weak_runtime_kv_imports_skipped, 1);
    assert_eq!(response.diagnostics.exported_kv_blocks, 3);
    assert_eq!(response.diagnostics.runtime_kv_segments_included, 2);
    assert_eq!(response.diagnostics.runtime_kv_segments_skipped, 1);
    assert_eq!(response.diagnostics.runtime_kv_segments_rejected, 0);
    assert!(response.diagnostics.has_runtime_kv_segment_signal());
    assert_eq!(response.diagnostics.hot_kv_precision_bits, Some(8));
    assert_eq!(response.diagnostics.cold_kv_precision_bits, Some(4));
    assert!(response.diagnostics.has_valid_kv_precision_signal());
    assert_eq!(response.exported_kv_blocks.len(), 1);
    assert_eq!(response.exported_kv_blocks[0].layer, 1);
    assert_eq!(response.exported_kv_blocks[0].key, vec![0.1, 0.2]);
    assert_eq!(response.exported_kv_blocks[0].value, vec![0.3, 0.4]);
}

#[test]
fn command_runtime_can_expand_json_wire_payload() {
    let runtime = CommandRuntime::new("runner")
        .wire_format(CommandWireFormat::Json)
        .arg("--wire")
        .arg("{wire_format}")
        .arg("--payload")
        .arg("{runtime_payload}")
        .prompt_mode(CommandPromptMode::Args);
    let request = sample_request();
    let payload = format_runtime_payload(&request, CommandWireFormat::Json);
    let args = runtime.expanded_args(&request, &payload);

    assert_eq!(args[1], "json");
    assert!(args[3].contains("\"schema\":\"rust-norion-runtime-request-v1\""));
    assert!(args[3].contains("\"hardware\""));
    assert!(args[3].contains("\"runtime_architecture\""));
    assert!(args[3].contains("\"primary_lane\":\"cpu-vector\""));
}

#[test]
fn command_runtime_user_prompt_expansion_does_not_rewrite_prompt_text() {
    let runtime = CommandRuntime::new("runner")
        .arg("{user_prompt}")
        .arg("{task_prompt}")
        .arg("wire={wire_format};prompt={user_prompt}")
        .prompt_mode(CommandPromptMode::Args);
    let mut request = sample_request();
    request.prompt =
        "Keep literal placeholders like {max_tokens} and {runtime_metadata}.".to_owned();
    let payload = format_runtime_prompt(&request);
    let args = runtime.expanded_args(&request, &payload);

    assert_eq!(args[0], request.prompt);
    assert_eq!(args[1], request.prompt);
    assert_eq!(args[2], format!("wire=text;prompt={}", request.prompt));
}

#[test]
fn command_runtime_mistralrs_cli_filter_strips_ansi_and_stats_footer() {
    let raw =
        "\x1b[90mfinal answer\x1b[0m\nStats:\nCLI time to first token: 0.1s\nDecode: 2 tokens";

    let filtered = filter_command_text_output(raw, CommandTextOutputFilter::MistralRsCli);
    let stats = parse_mistralrs_cli_stats(raw).unwrap();

    assert_eq!(filtered, "final answer");
    assert_eq!(stats.decode_tokens, Some(2));
}

#[test]
fn command_runtime_text_response_carries_static_runtime_diagnostics() {
    let (program, args) = text_echo_command("plain runtime answer");
    let metadata = RuntimeMetadata::new("command-model", "command-tokenizer", 4096, 384);
    let architecture = TransformerRuntimeArchitecture::new(48, 384, 8, 4, 1024);
    let mut runtime = CommandRuntime::new(program)
        .args(args)
        .with_metadata(metadata)
        .with_architecture(architecture)
        .prompt_mode(CommandPromptMode::Args);

    let response = runtime.generate(sample_request()).unwrap();

    assert_eq!(response.answer, "plain runtime answer");
    assert_eq!(
        response.diagnostics.model_id.as_deref(),
        Some("command-model")
    );
    assert_eq!(response.diagnostics.selected_adapter, None);
    assert_eq!(response.diagnostics.layer_count, 48);
    assert_eq!(response.diagnostics.hidden_size, 384);
    assert_eq!(response.diagnostics.local_window_tokens, 1024);
    assert_eq!(response.diagnostics.hot_kv_precision_bits, Some(8));
    assert_eq!(response.diagnostics.cold_kv_precision_bits, Some(4));
    assert_eq!(response.diagnostics.forward_energy, None);
    assert!(response.diagnostics.has_forward_signal());
}

#[test]
fn command_runtime_mistralrs_cli_stats_become_token_evidence() {
    let raw = "业务回答\nStats:\nCLI time to first token: 0.05s\nPrompt: 25 tokens, 568.18 T/s\nDecode: 3 tokens, 23.17 T/s\n";
    let (program, args) = text_echo_command(raw);
    let mut runtime = CommandRuntime::new(program)
        .args(args)
        .text_output_filter(CommandTextOutputFilter::MistralRsCli)
        .prompt_mode(CommandPromptMode::Args);

    let response = runtime.generate(sample_request()).unwrap();

    assert_eq!(response.answer, "业务回答");
    assert_eq!(response.tokens.len(), 3);
    assert!(
        response
            .trace
            .iter()
            .any(|step| step.label == "mistralrs_cli_stats"
                && step.content.contains("decode_tokens=3")
                && step.content.contains("prompt_tokens=25"))
    );
}

#[test]
fn command_runtime_reports_spawn_errors() {
    let mut runtime =
        CommandRuntime::new("__rust_norion_missing_command__").prompt_mode(CommandPromptMode::Args);

    let error = runtime.generate(sample_request()).unwrap_err();

    assert!(error.message().contains("failed to spawn runtime command"));
}

#[test]
fn command_runtime_times_out_long_running_command() {
    let (program, args) = slow_command();
    let mut runtime = CommandRuntime::new(program)
        .args(args)
        .prompt_mode(CommandPromptMode::Args)
        .with_timeout_ms(20);

    let error = runtime.generate(sample_request()).unwrap_err();

    assert!(error.message().contains("timed out after 20 ms"));
}

#[test]
fn command_runtime_exports_kv_blocks_from_json_response() {
    let (program, args) = json_echo_command(
        "{\"schema\":\"rust-norion-runtime-response-v1\",\"answer\":\"helper runtime answer\",\"tokens\":[{\"text\":\"helper\",\"entropy\":0.2}],\"trace\":[{\"label\":\"helper\",\"content\":\"json runtime helper\",\"confidence\":0.9}],\"exported_kv_blocks\":[{\"layer\":0,\"head\":0,\"token_start\":0,\"token_end\":1,\"key\":[0.1],\"value\":[0.2]}],\"diagnostics\":{\"model_id\":\"sample-transformer\",\"selected_adapter\":\"portable-rust\",\"global_layers\":1,\"local_window_layers\":1,\"convolutional_fusion_layers\":1,\"forward_energy\":0.4,\"kv_influence\":0.2}}",
    );
    let mut runtime = CommandRuntime::new(program)
        .args(args)
        .wire_format(CommandWireFormat::Json)
        .prompt_mode(CommandPromptMode::Args);
    let response = runtime.generate(sample_request()).unwrap();
    let exported = runtime.export_kv().unwrap();

    assert_eq!(response.answer, "helper runtime answer");
    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0].layer, 0);
    assert_eq!(exported[0].value, vec![0.2]);
}

#[test]
fn command_runtime_json_stdin_receives_imported_kv_blocks() {
    let (program, args) = json_stdin_import_probe_command();
    let mut runtime = CommandRuntime::new(program)
        .args(args)
        .wire_format(CommandWireFormat::Json)
        .prompt_mode(CommandPromptMode::Stdin);

    let response = runtime.generate(sample_request()).unwrap();

    assert_eq!(response.answer, "imported kv visible");
    assert_eq!(response.diagnostics.imported_kv_blocks, 1);
    assert_eq!(runtime.export_kv().unwrap().len(), 1);
}

fn text_echo_command(payload: &str) -> (String, Vec<String>) {
    json_echo_command(payload)
}

#[cfg(windows)]
fn slow_command() -> (String, Vec<String>) {
    (
        "powershell.exe".to_owned(),
        vec![
            "-NoProfile".to_owned(),
            "-NonInteractive".to_owned(),
            "-Command".to_owned(),
            "Start-Sleep -Milliseconds 250; Write-Output 'late runtime answer'".to_owned(),
        ],
    )
}

#[cfg(not(windows))]
fn slow_command() -> (String, Vec<String>) {
    (
        "sh".to_owned(),
        vec![
            "-c".to_owned(),
            "sleep 0.25; printf 'late runtime answer'".to_owned(),
        ],
    )
}

#[cfg(windows)]
fn json_echo_command(payload: &str) -> (String, Vec<String>) {
    let escaped = payload.replace('\'', "''");
    (
        "powershell.exe".to_owned(),
        vec![
            "-NoProfile".to_owned(),
            "-NonInteractive".to_owned(),
            "-Command".to_owned(),
            format!("Write-Output '{escaped}'"),
        ],
    )
}

#[cfg(not(windows))]
fn json_echo_command(payload: &str) -> (String, Vec<String>) {
    let escaped = payload.replace('\'', "'\\''");
    (
        "sh".to_owned(),
        vec!["-c".to_owned(), format!("printf '%s' '{escaped}'")],
    )
}

#[cfg(windows)]
fn json_stdin_import_probe_command() -> (String, Vec<String>) {
    let response = imported_kv_probe_response().replace('\'', "''");
    (
        "powershell.exe".to_owned(),
        vec![
            "-NoProfile".to_owned(),
            "-NonInteractive".to_owned(),
            "-Command".to_owned(),
            format!(
                "$payload = [Console]::In.ReadToEnd(); if (-not $payload.Contains('\"imported_kv_blocks\":[') -or -not $payload.Contains('\"layer\":1')) {{ exit 7 }}; Write-Output '{response}'"
            ),
        ],
    )
}

#[cfg(not(windows))]
fn json_stdin_import_probe_command() -> (String, Vec<String>) {
    let response = imported_kv_probe_response().replace('\'', "'\\''");
    (
        "sh".to_owned(),
        vec![
            "-c".to_owned(),
            format!(
                "payload=$(cat); case \"$payload\" in *'\"imported_kv_blocks\":['*'\"layer\":1'*) printf '%s' '{response}' ;; *) exit 7 ;; esac"
            ),
        ],
    )
}

fn imported_kv_probe_response() -> &'static str {
    "{\"schema\":\"rust-norion-runtime-response-v1\",\"answer\":\"imported kv visible\",\"tokens\":[{\"text\":\"imported\",\"entropy\":0.2}],\"trace\":[{\"label\":\"import_probe\",\"content\":\"saw imported KV blocks in JSON request\",\"confidence\":0.93}],\"exported_kv_blocks\":[{\"layer\":0,\"head\":0,\"token_start\":0,\"token_end\":1,\"key\":[0.5],\"value\":[0.6]}],\"diagnostics\":{\"model_id\":\"sample-transformer\",\"selected_adapter\":\"portable-rust\",\"global_layers\":1,\"local_window_layers\":1,\"convolutional_fusion_layers\":1,\"forward_energy\":0.4,\"kv_influence\":0.2,\"imported_kv_blocks\":1}}"
}

fn sample_request() -> RuntimeRequest {
    RuntimeRequest {
        prompt: "build a command runtime".to_owned(),
        profile: TaskProfile::Coding,
        runtime_metadata: RuntimeMetadata::new("sample-transformer", "sample-tokenizer", 8192, 64)
            .with_kv_exchange(true, true),
        runtime_architecture: TransformerRuntimeArchitecture::new(16, 64, 8, 4, 2048),
        memory_hints: vec!["memory hint".to_owned()],
        infini_memory_hints: vec!["LocalWindow:memory hint score=0.900".to_owned()],
        experience_hints: vec!["experience hint".to_owned()],
        runtime_adapter_observations: vec![RuntimeAdapterObservation::new(
            RuntimeAdapterHint::CpuSimd,
            0.82,
            0.80,
            0.86,
            Some(0.20),
            Some(0.30),
            9,
        )],
        toolsmith_plan: ToolsmithPlan::default(),
        agent_team_plan: AgentTeamPlan::default(),
        route_budget: RouteBudget {
            threshold: 0.5,
            attention_tokens: 2,
            fast_tokens: 1,
            attention_fraction: 0.66,
        },
        hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
        transformer_plan: TransformerRefactorPlan::default(),
        recursive_schedule: RecursiveSchedule::default(),
        hardware_plan: HardwarePlan::default(),
        imported_kv_blocks: vec![RuntimeKvBlock::new(
            1,
            0,
            0,
            1,
            vec![0.1, 0.2],
            vec![0.3, 0.4],
        )],
        max_tokens: 64,
    }
}
