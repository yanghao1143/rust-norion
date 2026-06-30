use crate::hardware::RuntimeAdapterHint;
use crate::hierarchy::TaskProfile;

use super::{RuntimeMetadata, RuntimeRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAdapterLanguage {
    Auto,
    English,
    Chinese,
}

impl RuntimeAdapterLanguage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::English => "english",
            Self::Chinese => "chinese",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAdapterFallbackReason {
    UnavailableModel,
    UnsupportedLanguage,
    RustCodingUnsupported,
    StreamingUnsupported,
    MissingKvHooks,
    ContextTooSmall,
    FailedHealthCheck,
    BudgetExhausted,
    UnsafeDirectWrite,
}

impl RuntimeAdapterFallbackReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnavailableModel => "unavailable_model",
            Self::UnsupportedLanguage => "unsupported_language",
            Self::RustCodingUnsupported => "rust_coding_unsupported",
            Self::StreamingUnsupported => "streaming_unsupported",
            Self::MissingKvHooks => "missing_kv_hooks",
            Self::ContextTooSmall => "context_too_small",
            Self::FailedHealthCheck => "failed_health_check",
            Self::BudgetExhausted => "budget_exhausted",
            Self::UnsafeDirectWrite => "unsafe_direct_write",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeAdapterCapability {
    pub adapter_id: String,
    pub adapter_hint: Option<RuntimeAdapterHint>,
    pub languages: Vec<RuntimeAdapterLanguage>,
    pub supports_rust_coding: bool,
    pub supports_streaming: bool,
    pub supports_chunked_kv: bool,
    pub max_context_tokens: usize,
    pub quantization_bits: Vec<u8>,
    pub device_hints: Vec<RuntimeAdapterHint>,
    pub available: bool,
    pub healthy: bool,
    pub budget_available: bool,
    pub allows_memory_write: bool,
    pub allows_genome_write: bool,
    pub clean_room: bool,
}

impl RuntimeAdapterCapability {
    pub fn new(adapter_id: impl Into<String>) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            adapter_hint: None,
            languages: vec![RuntimeAdapterLanguage::Auto],
            supports_rust_coding: false,
            supports_streaming: false,
            supports_chunked_kv: false,
            max_context_tokens: 0,
            quantization_bits: vec![8],
            device_hints: vec![RuntimeAdapterHint::PortableRust],
            available: true,
            healthy: true,
            budget_available: true,
            allows_memory_write: false,
            allows_genome_write: false,
            clean_room: true,
        }
    }

    pub fn with_hint(mut self, hint: RuntimeAdapterHint) -> Self {
        self.adapter_hint = Some(hint);
        self.device_hints = vec![hint];
        self
    }

    pub fn with_languages(mut self, languages: &[RuntimeAdapterLanguage]) -> Self {
        self.languages = languages.to_vec();
        self
    }

    pub fn with_rust_coding(mut self, supported: bool) -> Self {
        self.supports_rust_coding = supported;
        self
    }

    pub fn with_streaming(mut self, supported: bool) -> Self {
        self.supports_streaming = supported;
        self
    }

    pub fn with_chunked_kv(mut self, supported: bool) -> Self {
        self.supports_chunked_kv = supported;
        self
    }

    pub fn with_context(mut self, tokens: usize) -> Self {
        self.max_context_tokens = tokens;
        self
    }

    pub fn with_quantization_bits(mut self, bits: &[u8]) -> Self {
        self.quantization_bits = bits
            .iter()
            .copied()
            .filter(|bit| matches!(bit, 2 | 3 | 4 | 5 | 6 | 8 | 16))
            .collect();
        if self.quantization_bits.is_empty() {
            self.quantization_bits.push(8);
        }
        self
    }

    pub fn with_device_hints(mut self, hints: &[RuntimeAdapterHint]) -> Self {
        self.device_hints = hints.to_vec();
        self
    }

    pub fn with_availability(mut self, available: bool) -> Self {
        self.available = available;
        self
    }

    pub fn with_health(mut self, healthy: bool) -> Self {
        self.healthy = healthy;
        self
    }

    pub fn with_budget(mut self, budget_available: bool) -> Self {
        self.budget_available = budget_available;
        self
    }

    pub fn with_direct_write_access(mut self, memory: bool, genome: bool) -> Self {
        self.allows_memory_write = memory;
        self.allows_genome_write = genome;
        self
    }

    pub fn supports_language(&self, language: RuntimeAdapterLanguage) -> bool {
        self.languages.contains(&RuntimeAdapterLanguage::Auto) || self.languages.contains(&language)
    }

    pub fn summary(&self) -> String {
        let languages = self
            .languages
            .iter()
            .map(|language| language.as_str())
            .collect::<Vec<_>>()
            .join("+");
        let devices = self
            .device_hints
            .iter()
            .map(|hint| hint.as_str())
            .collect::<Vec<_>>()
            .join("+");
        let quantization = self
            .quantization_bits
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join("+");
        format!(
            "adapter={} hint={} languages={} rust_coding={} streaming={} chunked_kv={} context={} quantization={} devices={} available={} healthy={} budget={} clean_room={} memory_write={} genome_write={} redacted=true",
            sanitize_token(&self.adapter_id),
            self.adapter_hint
                .map(RuntimeAdapterHint::as_str)
                .unwrap_or("none"),
            languages,
            self.supports_rust_coding,
            self.supports_streaming,
            self.supports_chunked_kv,
            self.max_context_tokens,
            quantization,
            devices,
            self.available,
            self.healthy,
            self.budget_available,
            self.clean_room,
            self.allows_memory_write,
            self.allows_genome_write
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeAdapterRequirement {
    pub language: RuntimeAdapterLanguage,
    pub profile: TaskProfile,
    pub rust_coding: bool,
    pub requires_streaming: bool,
    pub requires_chunked_kv: bool,
    pub min_context_tokens: usize,
    pub allowed_adapters: Vec<RuntimeAdapterHint>,
    pub observed_scores: Vec<(RuntimeAdapterHint, f32)>,
}

impl RuntimeAdapterRequirement {
    pub fn from_request(request: &RuntimeRequest, requires_streaming: bool) -> Self {
        let estimated_context_tokens =
            estimated_context_tokens(&request.prompt, request.max_tokens);
        let min_context_tokens = if request.recursive_schedule.requires_recursion {
            request.recursive_schedule.chunk_tokens.max(1)
        } else {
            estimated_context_tokens
        };
        Self {
            language: language_for_prompt(&request.prompt),
            profile: request.profile,
            rust_coding: request.profile == TaskProfile::Coding
                && prompt_mentions_rust(&request.prompt),
            requires_streaming,
            requires_chunked_kv: !request.imported_kv_blocks.is_empty(),
            min_context_tokens,
            allowed_adapters: request.hardware_plan.execution.adapter_hints.clone(),
            observed_scores: request
                .runtime_adapter_observations
                .iter()
                .map(|observation| (observation.adapter, observation.score))
                .collect(),
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "language={} profile={} rust_coding={} streaming={} chunked_kv={} context={}",
            self.language.as_str(),
            profile_str(self.profile),
            self.rust_coding,
            self.requires_streaming,
            self.requires_chunked_kv,
            self.min_context_tokens
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeAdapterFallback {
    pub adapter_id: String,
    pub reasons: Vec<RuntimeAdapterFallbackReason>,
}

impl RuntimeAdapterFallback {
    pub fn summary(&self) -> String {
        let reasons = self
            .reasons
            .iter()
            .map(|reason| reason.as_str())
            .collect::<Vec<_>>()
            .join("+");
        format!(
            "adapter={} reasons={} redacted=true",
            sanitize_token(&self.adapter_id),
            reasons
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeAdapterSelection {
    pub requirement: RuntimeAdapterRequirement,
    pub selected: Option<RuntimeAdapterCapability>,
    pub fallbacks: Vec<RuntimeAdapterFallback>,
}

impl RuntimeAdapterSelection {
    pub fn selected_id(&self) -> Option<&str> {
        self.selected
            .as_ref()
            .map(|capability| capability.adapter_id.as_str())
    }

    pub fn trace_summary(&self) -> String {
        let selected = self
            .selected_id()
            .map(sanitize_token)
            .unwrap_or_else(|| "none".to_owned());
        format!(
            "selected={} fallback_count={} {} redacted=true",
            selected,
            self.fallbacks.len(),
            self.requirement.summary()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RuntimeAdapterRegistry {
    adapters: Vec<RuntimeAdapterCapability>,
}

impl RuntimeAdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_adapter(mut self, adapter: RuntimeAdapterCapability) -> Self {
        self.adapters.push(adapter);
        self
    }

    pub fn adapters(&self) -> &[RuntimeAdapterCapability] {
        &self.adapters
    }

    pub fn from_runtime_metadata(metadata: &RuntimeMetadata) -> Self {
        let context = if metadata.native_context_window == 0 {
            usize::MAX / 4
        } else {
            metadata.native_context_window
        };
        Self::new()
            .with_adapter(
                RuntimeAdapterCapability::new(format!("active-{}", metadata.model_id))
                    .with_hint(RuntimeAdapterHint::PortableRust)
                    .with_languages(&[
                        RuntimeAdapterLanguage::English,
                        RuntimeAdapterLanguage::Chinese,
                    ])
                    .with_rust_coding(true)
                    .with_streaming(true)
                    .with_chunked_kv(metadata.supports_kv_import || metadata.supports_kv_export)
                    .with_context(context)
                    .with_quantization_bits(&[
                        metadata.hot_kv_precision_bits,
                        metadata.cold_kv_precision_bits,
                    ]),
            )
            .with_adapter(mock_ci_adapter())
            .with_adapter(rust_code_clean_room_adapter().with_availability(false))
            .with_adapter(candle_mistral_adapter().with_availability(false))
            .with_adapter(future_local_adapter().with_availability(false))
    }

    pub fn mock_ci() -> Self {
        Self::new()
            .with_adapter(mock_ci_adapter())
            .with_adapter(rust_code_clean_room_adapter().with_availability(false))
            .with_adapter(candle_mistral_adapter().with_availability(false))
            .with_adapter(future_local_adapter().with_availability(false))
    }

    pub fn select(&self, requirement: &RuntimeAdapterRequirement) -> RuntimeAdapterSelection {
        let mut selected: Option<(RuntimeAdapterCapability, f32)> = None;
        let mut fallbacks = Vec::new();

        for adapter in &self.adapters {
            let reasons = reject_reasons(adapter, requirement);
            if reasons.is_empty() {
                let score = selection_score(adapter, requirement);
                if selected
                    .as_ref()
                    .map(|(_, best_score)| score > *best_score)
                    .unwrap_or(true)
                {
                    selected = Some((adapter.clone(), score));
                }
            } else {
                fallbacks.push(RuntimeAdapterFallback {
                    adapter_id: adapter.adapter_id.clone(),
                    reasons,
                });
            }
        }

        RuntimeAdapterSelection {
            requirement: requirement.clone(),
            selected: selected.map(|(adapter, _)| adapter),
            fallbacks,
        }
    }
}

fn mock_ci_adapter() -> RuntimeAdapterCapability {
    RuntimeAdapterCapability::new("mock-deterministic")
        .with_hint(RuntimeAdapterHint::PortableRust)
        .with_languages(&[
            RuntimeAdapterLanguage::English,
            RuntimeAdapterLanguage::Chinese,
        ])
        .with_rust_coding(true)
        .with_streaming(true)
        .with_chunked_kv(false)
        .with_context(16_384)
        .with_quantization_bits(&[8, 4])
}

fn rust_code_clean_room_adapter() -> RuntimeAdapterCapability {
    RuntimeAdapterCapability::new("rust-code-clean-room")
        .with_hint(RuntimeAdapterHint::CpuSimd)
        .with_languages(&[RuntimeAdapterLanguage::English])
        .with_rust_coding(true)
        .with_streaming(true)
        .with_chunked_kv(true)
        .with_context(32_768)
        .with_quantization_bits(&[8, 4])
        .with_device_hints(&[
            RuntimeAdapterHint::PortableRust,
            RuntimeAdapterHint::CpuSimd,
        ])
}

fn candle_mistral_adapter() -> RuntimeAdapterCapability {
    RuntimeAdapterCapability::new("candle-mistral-clean-room")
        .with_hint(RuntimeAdapterHint::Wgpu)
        .with_languages(&[
            RuntimeAdapterLanguage::English,
            RuntimeAdapterLanguage::Chinese,
        ])
        .with_rust_coding(false)
        .with_streaming(true)
        .with_chunked_kv(true)
        .with_context(65_536)
        .with_quantization_bits(&[8, 4])
        .with_device_hints(&[RuntimeAdapterHint::Wgpu, RuntimeAdapterHint::Cuda])
}

fn future_local_adapter() -> RuntimeAdapterCapability {
    RuntimeAdapterCapability::new("future-local-adapter")
        .with_hint(RuntimeAdapterHint::CustomAccelerator)
        .with_languages(&[RuntimeAdapterLanguage::Auto])
        .with_rust_coding(true)
        .with_streaming(false)
        .with_chunked_kv(true)
        .with_context(131_072)
        .with_quantization_bits(&[8, 4])
        .with_device_hints(&[RuntimeAdapterHint::CustomAccelerator])
}

fn reject_reasons(
    adapter: &RuntimeAdapterCapability,
    requirement: &RuntimeAdapterRequirement,
) -> Vec<RuntimeAdapterFallbackReason> {
    let mut reasons = Vec::new();
    if !adapter.available {
        reasons.push(RuntimeAdapterFallbackReason::UnavailableModel);
    }
    if !adapter.supports_language(requirement.language) {
        reasons.push(RuntimeAdapterFallbackReason::UnsupportedLanguage);
    }
    if requirement.rust_coding && !adapter.supports_rust_coding {
        reasons.push(RuntimeAdapterFallbackReason::RustCodingUnsupported);
    }
    if requirement.requires_streaming && !adapter.supports_streaming {
        reasons.push(RuntimeAdapterFallbackReason::StreamingUnsupported);
    }
    if requirement.requires_chunked_kv && !adapter.supports_chunked_kv {
        reasons.push(RuntimeAdapterFallbackReason::MissingKvHooks);
    }
    if adapter.max_context_tokens > 0 && adapter.max_context_tokens < requirement.min_context_tokens
    {
        reasons.push(RuntimeAdapterFallbackReason::ContextTooSmall);
    }
    if !adapter.healthy {
        reasons.push(RuntimeAdapterFallbackReason::FailedHealthCheck);
    }
    if !adapter.budget_available {
        reasons.push(RuntimeAdapterFallbackReason::BudgetExhausted);
    }
    if adapter.allows_memory_write || adapter.allows_genome_write {
        reasons.push(RuntimeAdapterFallbackReason::UnsafeDirectWrite);
    }
    reasons
}

fn selection_score(
    adapter: &RuntimeAdapterCapability,
    requirement: &RuntimeAdapterRequirement,
) -> f32 {
    let mut score = 1.0;
    if let Some(hint) = adapter.adapter_hint
        && requirement.allowed_adapters.contains(&hint)
    {
        score += 1.0;
    }
    if adapter.supports_chunked_kv {
        score += 0.2;
    }
    if adapter.supports_streaming {
        score += 0.1;
    }
    if requirement.rust_coding && adapter.supports_rust_coding {
        score += 0.3;
    }
    if adapter.clean_room {
        score += 0.1;
    }
    if adapter.max_context_tokens >= requirement.min_context_tokens {
        score += (adapter.max_context_tokens.min(131_072) as f32 / 131_072.0) * 0.1;
    }
    if let Some(hint) = adapter.adapter_hint {
        score += requirement
            .observed_scores
            .iter()
            .find(|(observed, _)| *observed == hint)
            .map(|(_, observed_score)| observed_score.clamp(0.0, 1.0) * 0.4)
            .unwrap_or(0.0);
    }
    score
}

fn language_for_prompt(prompt: &str) -> RuntimeAdapterLanguage {
    if prompt.chars().any(is_cjk_unified_ideograph) {
        RuntimeAdapterLanguage::Chinese
    } else if prompt
        .chars()
        .any(|character| character.is_ascii_alphabetic())
    {
        RuntimeAdapterLanguage::English
    } else {
        RuntimeAdapterLanguage::Auto
    }
}

fn prompt_mentions_rust(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    contains_any(
        &lower,
        &[
            "rust",
            "cargo",
            "crate",
            "borrow",
            "ownership",
            "lifetime",
            "trait",
            "impl",
            "tokio",
            "axum",
            "clippy",
        ],
    ) || contains_any(
        prompt,
        &["所有权", "借用", "生命周期", "结构体", "特征", "编译"],
    )
}

fn estimated_context_tokens(prompt: &str, _max_tokens: usize) -> usize {
    let lexical = prompt.split_whitespace().count();
    let character_estimate = prompt.chars().count().div_ceil(4);
    lexical.max(character_estimate).max(1)
}

fn contains_any(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

fn is_cjk_unified_ideograph(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff
    )
}

fn profile_str(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn sanitize_token(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':' | '.') {
                character
            } else {
                '_'
            }
        })
        .take(96)
        .collect::<String>();
    if sanitized.is_empty() {
        "unknown".to_owned()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_team::AgentTeamPlan;
    use crate::hardware::HardwarePlan;
    use crate::hierarchy::HierarchyWeights;
    use crate::recursive_scheduler::{RecursiveSchedule, RecursiveScheduler};
    use crate::router::RouteBudget;
    use crate::runtime_manifest::TransformerRuntimeArchitecture;
    use crate::toolsmith::ToolsmithPlan;
    use crate::transformer::TransformerRefactorPlan;

    fn request(prompt: &str, profile: TaskProfile) -> RuntimeRequest {
        RuntimeRequest {
            prompt: prompt.to_owned(),
            profile,
            tenant_scope: None,
            runtime_metadata: RuntimeMetadata::new("mock", "tok", 4096, 8),
            runtime_architecture: TransformerRuntimeArchitecture::new(2, 8, 1, 1, 512),
            memory_hints: Vec::new(),
            infini_memory_hints: Vec::new(),
            experience_hints: Vec::new(),
            runtime_adapter_observations: Vec::new(),
            toolsmith_plan: ToolsmithPlan::default(),
            agent_team_plan: AgentTeamPlan::default(),
            route_budget: RouteBudget {
                threshold: 0.5,
                attention_tokens: 1,
                fast_tokens: 1,
                attention_fraction: 0.5,
            },
            hierarchy: HierarchyWeights::default(),
            transformer_plan: TransformerRefactorPlan::default(),
            recursive_schedule: RecursiveSchedule::default(),
            hardware_plan: HardwarePlan::default(),
            imported_kv_blocks: Vec::new(),
            max_tokens: 64,
        }
    }

    #[test]
    fn mock_registry_registers_expected_adapter_surfaces() {
        let registry = RuntimeAdapterRegistry::mock_ci();
        let ids = registry
            .adapters()
            .iter()
            .map(|adapter| adapter.adapter_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                "mock-deterministic",
                "rust-code-clean-room",
                "candle-mistral-clean-room",
                "future-local-adapter"
            ]
        );
        assert!(registry.adapters().iter().all(|adapter| {
            !adapter.allows_memory_write && !adapter.allows_genome_write && adapter.clean_room
        }));
    }

    #[test]
    fn mock_registry_routes_english_chinese_and_rust_coding() {
        let registry = RuntimeAdapterRegistry::mock_ci();
        for (prompt, profile) in [
            ("summarize local memory", TaskProfile::General),
            ("请用中文总结运行时状态", TaskProfile::Writing),
            ("write a Rust trait implementation", TaskProfile::Coding),
        ] {
            let request = request(prompt, profile);
            let requirement = RuntimeAdapterRequirement::from_request(&request, true);
            let selection = registry.select(&requirement);

            assert_eq!(selection.selected_id(), Some("mock-deterministic"));
            assert!(selection.trace_summary().contains("redacted=true"));
            assert!(!selection.trace_summary().contains(prompt));
        }
    }

    #[test]
    fn recursive_requests_use_chunk_context_for_capability_selection() {
        let prompt = "section ".repeat(200);
        let mut request = request(&prompt, TaskProfile::LongDocument);
        request.runtime_metadata.native_context_window = 64;
        request.recursive_schedule = RecursiveScheduler::new(64, 32, 8, 4).plan(&prompt);

        let requirement = RuntimeAdapterRequirement::from_request(&request, false);
        let selection = RuntimeAdapterRegistry::from_runtime_metadata(&request.runtime_metadata)
            .select(&requirement);

        assert!(request.recursive_schedule.requires_recursion);
        assert_eq!(requirement.min_context_tokens, 32);
        assert!(selection.selected.is_some());
        assert!(!selection.fallbacks.iter().any(|fallback| {
            fallback
                .reasons
                .contains(&RuntimeAdapterFallbackReason::ContextTooSmall)
        }));
    }

    #[test]
    fn fallback_matrix_records_unavailable_health_budget_and_kv_reasons() {
        let registry = RuntimeAdapterRegistry::new()
            .with_adapter(
                RuntimeAdapterCapability::new("missing")
                    .with_languages(&[RuntimeAdapterLanguage::English])
                    .with_rust_coding(true)
                    .with_streaming(true)
                    .with_availability(false),
            )
            .with_adapter(
                RuntimeAdapterCapability::new("unhealthy")
                    .with_languages(&[RuntimeAdapterLanguage::English])
                    .with_rust_coding(true)
                    .with_streaming(true)
                    .with_health(false),
            )
            .with_adapter(
                RuntimeAdapterCapability::new("over-budget")
                    .with_languages(&[RuntimeAdapterLanguage::English])
                    .with_rust_coding(true)
                    .with_streaming(true)
                    .with_budget(false),
            )
            .with_adapter(
                RuntimeAdapterCapability::new("no-kv")
                    .with_languages(&[RuntimeAdapterLanguage::English])
                    .with_rust_coding(true)
                    .with_streaming(true),
            )
            .with_adapter(
                RuntimeAdapterCapability::new("winner")
                    .with_languages(&[RuntimeAdapterLanguage::English])
                    .with_rust_coding(true)
                    .with_streaming(true)
                    .with_chunked_kv(true),
            );
        let mut request = request("write Rust", TaskProfile::Coding);
        request
            .imported_kv_blocks
            .push(crate::kv_exchange::RuntimeKvBlock::new(
                0,
                0,
                0,
                1,
                vec![0.1],
                vec![0.2],
            ));
        let requirement = RuntimeAdapterRequirement::from_request(&request, true);
        let selection = registry.select(&requirement);

        assert_eq!(selection.selected_id(), Some("winner"));
        assert!(selection.fallbacks.iter().any(|fallback| {
            fallback.adapter_id == "missing"
                && fallback
                    .reasons
                    .contains(&RuntimeAdapterFallbackReason::UnavailableModel)
        }));
        assert!(selection.fallbacks.iter().any(|fallback| {
            fallback.adapter_id == "unhealthy"
                && fallback
                    .reasons
                    .contains(&RuntimeAdapterFallbackReason::FailedHealthCheck)
        }));
        assert!(selection.fallbacks.iter().any(|fallback| {
            fallback.adapter_id == "over-budget"
                && fallback
                    .reasons
                    .contains(&RuntimeAdapterFallbackReason::BudgetExhausted)
        }));
        assert!(selection.fallbacks.iter().any(|fallback| {
            fallback.adapter_id == "no-kv"
                && fallback
                    .reasons
                    .contains(&RuntimeAdapterFallbackReason::MissingKvHooks)
        }));
    }

    #[test]
    fn unsupported_task_rejects_without_prompt_leak() {
        let registry = RuntimeAdapterRegistry::new().with_adapter(
            RuntimeAdapterCapability::new("english-only")
                .with_languages(&[RuntimeAdapterLanguage::English])
                .with_streaming(true),
        );
        let request = request("请处理中文 secret=sk-hidden", TaskProfile::General);
        let requirement = RuntimeAdapterRequirement::from_request(&request, true);
        let selection = registry.select(&requirement);

        assert_eq!(selection.selected_id(), None);
        assert!(
            selection.fallbacks[0]
                .reasons
                .contains(&RuntimeAdapterFallbackReason::UnsupportedLanguage)
        );
        assert!(!selection.trace_summary().contains("sk-hidden"));
        assert!(!selection.fallbacks[0].summary().contains("sk-hidden"));
    }
}
