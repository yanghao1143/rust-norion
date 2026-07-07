use std::str::FromStr;

use crate::hardware::{DeviceClass, RuntimeAdapterHint};

pub const RESEARCH_DEPLOYMENT_SCHEMA_VERSION: &str = "research_deployment_v1";
pub const RESEARCH_SANDBOX_EVIDENCE_SCHEMA_VERSION: &str = "research_sandbox_evidence_v1";
pub const ENTERPRISE_SIDECAR_BOUNDARY_SCHEMA_VERSION: &str = "enterprise_sidecar_boundary_v1";
pub const ENTERPRISE_SIDECAR_ENV_VAR: &str = "NORION_ENTERPRISE_SIDECAR_ENDPOINT";

const RESEARCH_SANDBOX_PERSISTENT_STATE: &[&str] = &[
    "disk_kv_cache",
    "runtime_state",
    "experiment_ledger_preview",
    "redacted_evidence_packets",
];

const RESEARCH_SANDBOX_LOCAL_ONLY_DATA: &[&str] = &[
    "model_artifacts",
    "raw_traces",
    "secrets",
    "private_prompts",
];

const ENTERPRISE_PRIVATE_CAPABILITIES: &[&str] = &[
    "license_validation",
    "signed_entitlement_checks",
    "enterprise_policy_packs",
    "private_connectors",
    "cloud_control_plane_calls",
    "signed_model_data_policy_bundle_delivery",
];

const GPL_CORE_FORBIDDEN_LINK_MARKERS: &[(&str, &str)] = &[
    (".dll", "native_dynamic_library_link"),
    (".so", "native_shared_object_link"),
    (".dylib", "native_dylib_link"),
    ("proprietary", "proprietary_marker"),
    ("closed-source", "closed_source_marker"),
    ("closed_source", "closed_source_marker"),
    ("license-key", "license_key_marker"),
    ("license_key", "license_key_marker"),
    ("entitlement", "entitlement_marker"),
    ("private_connector", "private_connector_marker"),
    ("enterprise_connector", "enterprise_connector_marker"),
    ("enterprise-policy", "enterprise_policy_marker"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterpriseSidecarMode {
    Community,
    EnterpriseSidecar,
}

impl EnterpriseSidecarMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Community => "community",
            Self::EnterpriseSidecar => "enterprise-sidecar",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterpriseSidecarReachability {
    NotConfigured,
    Unreachable,
    Reachable,
}

impl EnterpriseSidecarReachability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotConfigured => "not-configured",
            Self::Unreachable => "unreachable",
            Self::Reachable => "reachable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnterpriseSidecarBoundaryReport {
    pub schema_version: &'static str,
    pub env_var: &'static str,
    pub mode: EnterpriseSidecarMode,
    pub reachability: EnterpriseSidecarReachability,
    pub endpoint_configured: bool,
    pub endpoint_digest: Option<String>,
    pub public_artifact: &'static str,
    pub enterprise_artifact: &'static str,
    pub private_capabilities: Vec<&'static str>,
    pub direct_proprietary_link_allowed: bool,
    pub community_fallback: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub applied: bool,
}

impl EnterpriseSidecarBoundaryReport {
    pub fn community() -> Self {
        Self::evaluate(None, false)
    }

    pub fn from_env() -> Self {
        let endpoint = std::env::var(ENTERPRISE_SIDECAR_ENV_VAR).ok();
        Self::evaluate(endpoint.as_deref(), false)
    }

    pub fn evaluate(endpoint: Option<&str>, endpoint_reachable: bool) -> Self {
        let endpoint = endpoint.map(str::trim).filter(|value| !value.is_empty());
        let (mode, reachability, community_fallback) = match (endpoint, endpoint_reachable) {
            (None, _) => (
                EnterpriseSidecarMode::Community,
                EnterpriseSidecarReachability::NotConfigured,
                true,
            ),
            (Some(_), false) => (
                EnterpriseSidecarMode::Community,
                EnterpriseSidecarReachability::Unreachable,
                true,
            ),
            (Some(_), true) => (
                EnterpriseSidecarMode::EnterpriseSidecar,
                EnterpriseSidecarReachability::Reachable,
                false,
            ),
        };

        Self {
            schema_version: ENTERPRISE_SIDECAR_BOUNDARY_SCHEMA_VERSION,
            env_var: ENTERPRISE_SIDECAR_ENV_VAR,
            mode,
            reachability,
            endpoint_configured: endpoint.is_some(),
            endpoint_digest: endpoint.map(stable_digest),
            public_artifact: "gpl_core_only",
            enterprise_artifact: "separate_sidecar_or_container",
            private_capabilities: ENTERPRISE_PRIVATE_CAPABILITIES.to_vec(),
            direct_proprietary_link_allowed: false,
            community_fallback,
            read_only: true,
            write_allowed: false,
            applied: false,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "enterprise_sidecar_boundary schema={} env_var={} mode={} reachability={} endpoint_configured={} endpoint_digest={} public_artifact={} enterprise_artifact={} private_capabilities={} direct_proprietary_link_allowed={} community_fallback={} read_only={} write_allowed={} applied={}",
            self.schema_version,
            self.env_var,
            self.mode.as_str(),
            self.reachability.as_str(),
            self.endpoint_configured,
            self.endpoint_digest.as_deref().unwrap_or("none"),
            self.public_artifact,
            self.enterprise_artifact,
            self.private_capabilities.join("|"),
            self.direct_proprietary_link_allowed,
            self.community_fallback,
            self.read_only,
            self.write_allowed,
            self.applied
        )
    }

    pub fn evidence_digest(&self) -> String {
        stable_digest(&self.summary_line())
    }
}

pub fn gpl_core_manifest_boundary_findings(manifests: &[&str]) -> Vec<&'static str> {
    let mut findings = Vec::new();
    for manifest in manifests {
        let normalized = manifest.to_ascii_lowercase();
        for (marker, reason) in GPL_CORE_FORBIDDEN_LINK_MARKERS {
            if normalized.contains(marker) && !findings.contains(reason) {
                findings.push(*reason);
            }
        }
    }
    findings
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResearchSandboxTarget {
    Local,
    Wsl,
    Container,
    SmallVps,
}

impl ResearchSandboxTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Wsl => "wsl",
            Self::Container => "container",
            Self::SmallVps => "small-vps",
        }
    }

    pub fn expected_targets() -> [Self; 4] {
        [Self::Local, Self::Wsl, Self::Container, Self::SmallVps]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchSandboxEvidenceReport {
    pub schema_version: &'static str,
    pub target: ResearchSandboxTarget,
    pub profile: ResearchDeploymentProfileKind,
    pub noncommercial_only: bool,
    pub contributor_pr_only: bool,
    pub maintainer_approval_required: bool,
    pub persistent_state: Vec<&'static str>,
    pub local_only_data: Vec<&'static str>,
    pub private_trace_publish_allowed: bool,
    pub redacted_issue_comment_ready: bool,
    pub wipe_test_state_supported: bool,
    pub preview_only: bool,
    pub write_allowed: bool,
    pub durable_write_allowed: bool,
    pub applied: bool,
}

impl ResearchSandboxEvidenceReport {
    pub fn from_profile(
        target: ResearchSandboxTarget,
        profile: &ResearchDeploymentProfile,
    ) -> Self {
        Self {
            schema_version: RESEARCH_SANDBOX_EVIDENCE_SCHEMA_VERSION,
            target,
            profile: profile.kind,
            noncommercial_only: profile.noncommercial_only,
            contributor_pr_only: true,
            maintainer_approval_required: profile.write_guards.operator_approval_required,
            persistent_state: RESEARCH_SANDBOX_PERSISTENT_STATE.to_vec(),
            local_only_data: RESEARCH_SANDBOX_LOCAL_ONLY_DATA.to_vec(),
            private_trace_publish_allowed: false,
            redacted_issue_comment_ready: true,
            wipe_test_state_supported: true,
            preview_only: profile.write_guards.write_mode()
                == ResearchDeploymentWriteMode::PreviewOnly,
            write_allowed: false,
            durable_write_allowed: false,
            applied: false,
        }
    }

    pub fn issue_comment_safe(&self) -> bool {
        self.noncommercial_only
            && self.contributor_pr_only
            && self.maintainer_approval_required
            && !self.private_trace_publish_allowed
            && self.redacted_issue_comment_ready
            && self.wipe_test_state_supported
            && !self.write_allowed
            && !self.durable_write_allowed
            && !self.applied
    }

    pub fn summary_line(&self) -> String {
        format!(
            "research_sandbox_evidence schema={} target={} profile={} noncommercial_only={} contributor_pr_only={} maintainer_approval_required={} persistent_state={} local_only_data={} private_trace_publish_allowed={} redacted_issue_comment_ready={} wipe_test_state_supported={} preview_only={} write_allowed={} durable_write_allowed={} applied={}",
            self.schema_version,
            self.target.as_str(),
            self.profile.as_str(),
            self.noncommercial_only,
            self.contributor_pr_only,
            self.maintainer_approval_required,
            self.persistent_state.join("|"),
            self.local_only_data.join("|"),
            self.private_trace_publish_allowed,
            self.redacted_issue_comment_ready,
            self.wipe_test_state_supported,
            self.preview_only,
            self.write_allowed,
            self.durable_write_allowed,
            self.applied
        )
    }

    pub fn evidence_digest(&self) -> String {
        stable_digest(&self.summary_line())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResearchDeploymentProfileKind {
    CpuOnly,
    SingleGpu,
    LowMemory,
    BenchmarkReplay,
}

impl ResearchDeploymentProfileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CpuOnly => "cpu-only",
            Self::SingleGpu => "single-gpu",
            Self::LowMemory => "low-memory",
            Self::BenchmarkReplay => "benchmark-replay",
        }
    }

    pub fn expected_profiles() -> [Self; 4] {
        [
            Self::CpuOnly,
            Self::SingleGpu,
            Self::LowMemory,
            Self::BenchmarkReplay,
        ]
    }
}

impl FromStr for ResearchDeploymentProfileKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_research_deployment_profile(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchDeploymentWriteMode {
    PreviewOnly,
    ApprovalGated,
}

impl ResearchDeploymentWriteMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreviewOnly => "preview-only",
            Self::ApprovalGated => "approval-gated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResearchDeploymentResourceLimits {
    pub max_context_tokens: usize,
    pub max_generation_tokens: usize,
    pub max_kv_tokens: usize,
    pub max_concurrent_requests: usize,
    pub max_background_reflections: usize,
    pub max_stream_buffer_tokens: usize,
    pub max_stream_chunks_in_flight: usize,
    pub cancellation_poll_ms: u64,
    pub request_timeout_ms: u64,
    pub allow_streaming: bool,
    pub allow_background_reflection: bool,
    pub allow_disk_spill: bool,
}

impl ResearchDeploymentResourceLimits {
    pub fn summary(&self) -> String {
        format!(
            "context={} generation={} kv={} concurrency={} background_reflections={} stream_buffer={} stream_chunks={} cancellation_poll_ms={} request_timeout_ms={} streaming={} background_reflection={} disk_spill={}",
            self.max_context_tokens,
            self.max_generation_tokens,
            self.max_kv_tokens,
            self.max_concurrent_requests,
            self.max_background_reflections,
            self.max_stream_buffer_tokens,
            self.max_stream_chunks_in_flight,
            self.cancellation_poll_ms,
            self.request_timeout_ms,
            self.allow_streaming,
            self.allow_background_reflection,
            self.allow_disk_spill
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResearchDeploymentWriteGuards {
    pub durable_memory_writes: bool,
    pub genome_writes: bool,
    pub experiment_ledger_writes: bool,
    pub operator_approval_required: bool,
    pub privacy_gate_required: bool,
    pub preview_to_write_gate_required: bool,
}

impl Default for ResearchDeploymentWriteGuards {
    fn default() -> Self {
        Self {
            durable_memory_writes: false,
            genome_writes: false,
            experiment_ledger_writes: false,
            operator_approval_required: true,
            privacy_gate_required: true,
            preview_to_write_gate_required: true,
        }
    }
}

impl ResearchDeploymentWriteGuards {
    pub fn write_mode(self) -> ResearchDeploymentWriteMode {
        if self.durable_memory_writes || self.genome_writes || self.experiment_ledger_writes {
            ResearchDeploymentWriteMode::ApprovalGated
        } else {
            ResearchDeploymentWriteMode::PreviewOnly
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "mode={} durable_memory={} genome={} experiment_ledger={} operator_approval_required={} privacy_gate_required={} preview_to_write_gate_required={}",
            self.write_mode().as_str(),
            self.durable_memory_writes,
            self.genome_writes,
            self.experiment_ledger_writes,
            self.operator_approval_required,
            self.privacy_gate_required,
            self.preview_to_write_gate_required
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchDeploymentProfile {
    pub schema_version: &'static str,
    pub kind: ResearchDeploymentProfileKind,
    pub device_class: DeviceClass,
    pub adapter_hint: RuntimeAdapterHint,
    pub limits: ResearchDeploymentResourceLimits,
    pub write_guards: ResearchDeploymentWriteGuards,
    pub noncommercial_only: bool,
    pub description: &'static str,
}

impl ResearchDeploymentProfile {
    pub fn template(kind: ResearchDeploymentProfileKind) -> Self {
        match kind {
            ResearchDeploymentProfileKind::CpuOnly => Self {
                schema_version: RESEARCH_DEPLOYMENT_SCHEMA_VERSION,
                kind,
                device_class: DeviceClass::CpuOnly,
                adapter_hint: RuntimeAdapterHint::PortableRust,
                limits: ResearchDeploymentResourceLimits {
                    max_context_tokens: 4_096,
                    max_generation_tokens: 1_024,
                    max_kv_tokens: 8_192,
                    max_concurrent_requests: 1,
                    max_background_reflections: 1,
                    max_stream_buffer_tokens: 512,
                    max_stream_chunks_in_flight: 2,
                    cancellation_poll_ms: 250,
                    request_timeout_ms: 120_000,
                    allow_streaming: true,
                    allow_background_reflection: true,
                    allow_disk_spill: true,
                },
                write_guards: ResearchDeploymentWriteGuards::default(),
                noncommercial_only: true,
                description: "local CPU-only research run",
            },
            ResearchDeploymentProfileKind::SingleGpu => Self {
                schema_version: RESEARCH_DEPLOYMENT_SCHEMA_VERSION,
                kind,
                device_class: DeviceClass::DiscreteGpu,
                adapter_hint: RuntimeAdapterHint::Cuda,
                limits: ResearchDeploymentResourceLimits {
                    max_context_tokens: 16_384,
                    max_generation_tokens: 2_048,
                    max_kv_tokens: 32_768,
                    max_concurrent_requests: 2,
                    max_background_reflections: 2,
                    max_stream_buffer_tokens: 2_048,
                    max_stream_chunks_in_flight: 4,
                    cancellation_poll_ms: 125,
                    request_timeout_ms: 180_000,
                    allow_streaming: true,
                    allow_background_reflection: true,
                    allow_disk_spill: true,
                },
                write_guards: ResearchDeploymentWriteGuards::default(),
                noncommercial_only: true,
                description: "single accelerator local research run",
            },
            ResearchDeploymentProfileKind::LowMemory => Self {
                schema_version: RESEARCH_DEPLOYMENT_SCHEMA_VERSION,
                kind,
                device_class: DeviceClass::Embedded,
                adapter_hint: RuntimeAdapterHint::PortableRust,
                limits: ResearchDeploymentResourceLimits {
                    max_context_tokens: 1_024,
                    max_generation_tokens: 256,
                    max_kv_tokens: 2_048,
                    max_concurrent_requests: 1,
                    max_background_reflections: 0,
                    max_stream_buffer_tokens: 128,
                    max_stream_chunks_in_flight: 1,
                    cancellation_poll_ms: 500,
                    request_timeout_ms: 60_000,
                    allow_streaming: true,
                    allow_background_reflection: false,
                    allow_disk_spill: false,
                },
                write_guards: ResearchDeploymentWriteGuards::default(),
                noncommercial_only: true,
                description: "constrained local research run",
            },
            ResearchDeploymentProfileKind::BenchmarkReplay => Self {
                schema_version: RESEARCH_DEPLOYMENT_SCHEMA_VERSION,
                kind,
                device_class: DeviceClass::CpuOnly,
                adapter_hint: RuntimeAdapterHint::PortableRust,
                limits: ResearchDeploymentResourceLimits {
                    max_context_tokens: 8_192,
                    max_generation_tokens: 1_024,
                    max_kv_tokens: 16_384,
                    max_concurrent_requests: 1,
                    max_background_reflections: 0,
                    max_stream_buffer_tokens: 0,
                    max_stream_chunks_in_flight: 0,
                    cancellation_poll_ms: 250,
                    request_timeout_ms: 240_000,
                    allow_streaming: false,
                    allow_background_reflection: false,
                    allow_disk_spill: true,
                },
                write_guards: ResearchDeploymentWriteGuards::default(),
                noncommercial_only: true,
                description: "deterministic benchmark and replay run",
            },
        }
    }

    pub fn from_label(label: &str) -> Result<Self, String> {
        parse_research_deployment_profile(label).map(Self::template)
    }

    pub fn guard(&self, request: ResearchDeploymentRequest) -> ResearchDeploymentGuardReport {
        ResearchDeploymentGuardReport::evaluate(self, request)
    }

    pub fn operator_health(&self) -> ResearchDeploymentOperatorHealth {
        ResearchDeploymentOperatorHealth {
            schema_version: RESEARCH_DEPLOYMENT_SCHEMA_VERSION,
            active_profile: self.kind,
            device_class: self.device_class,
            adapter_hint: self.adapter_hint,
            write_mode: self.write_guards.write_mode(),
            limits: self.limits,
            noncommercial_only: self.noncommercial_only,
            durable_memory_writes: self.write_guards.durable_memory_writes,
            genome_writes: self.write_guards.genome_writes,
            experiment_ledger_writes: self.write_guards.experiment_ledger_writes,
            operator_approval_required: self.write_guards.operator_approval_required,
            privacy_gate_required: self.write_guards.privacy_gate_required,
            preview_to_write_gate_required: self.write_guards.preview_to_write_gate_required,
            read_only: true,
            write_allowed: false,
            durable_write_allowed: false,
            applied: false,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "research_deployment_profile schema={} profile={} device={} adapter={} noncommercial_only={} limits=({}) writes=({}) description={}",
            self.schema_version,
            self.kind.as_str(),
            self.device_class.as_str(),
            self.adapter_hint.as_str(),
            self.noncommercial_only,
            self.limits.summary(),
            self.write_guards.summary(),
            self.description
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResearchDeploymentRequest {
    pub context_tokens: usize,
    pub generation_tokens: usize,
    pub kv_tokens: usize,
    pub concurrent_requests: usize,
    pub background_reflections: usize,
    pub stream_buffer_tokens: usize,
    pub stream_chunks_in_flight: usize,
    pub streaming_enabled: bool,
    pub cancellation_poll_ms: u64,
    pub request_timeout_ms: u64,
    pub durable_memory_write_requested: bool,
    pub genome_write_requested: bool,
    pub experiment_ledger_write_requested: bool,
}

impl ResearchDeploymentRequest {
    pub fn fixture_for(profile: &ResearchDeploymentProfile) -> Self {
        Self {
            context_tokens: profile.limits.max_context_tokens.min(2_048).max(1),
            generation_tokens: profile.limits.max_generation_tokens.min(512).max(1),
            kv_tokens: profile.limits.max_kv_tokens.min(4_096).max(1),
            concurrent_requests: profile.limits.max_concurrent_requests.min(1).max(1),
            background_reflections: profile.limits.max_background_reflections.min(1),
            stream_buffer_tokens: profile.limits.max_stream_buffer_tokens.min(128),
            stream_chunks_in_flight: profile.limits.max_stream_chunks_in_flight.min(1),
            streaming_enabled: profile.limits.allow_streaming,
            cancellation_poll_ms: profile.limits.cancellation_poll_ms,
            request_timeout_ms: profile.limits.request_timeout_ms,
            durable_memory_write_requested: false,
            genome_write_requested: false,
            experiment_ledger_write_requested: false,
        }
    }

    pub fn with_context_tokens(mut self, context_tokens: usize) -> Self {
        self.context_tokens = context_tokens;
        self
    }

    pub fn with_generation_tokens(mut self, generation_tokens: usize) -> Self {
        self.generation_tokens = generation_tokens;
        self
    }

    pub fn with_kv_tokens(mut self, kv_tokens: usize) -> Self {
        self.kv_tokens = kv_tokens;
        self
    }

    pub fn with_concurrent_requests(mut self, concurrent_requests: usize) -> Self {
        self.concurrent_requests = concurrent_requests;
        self
    }

    pub fn with_background_reflections(mut self, background_reflections: usize) -> Self {
        self.background_reflections = background_reflections;
        self
    }

    pub fn with_streaming(
        mut self,
        enabled: bool,
        stream_buffer_tokens: usize,
        stream_chunks_in_flight: usize,
    ) -> Self {
        self.streaming_enabled = enabled;
        self.stream_buffer_tokens = stream_buffer_tokens;
        self.stream_chunks_in_flight = stream_chunks_in_flight;
        self
    }

    pub fn with_write_requests(
        mut self,
        durable_memory: bool,
        genome: bool,
        experiment_ledger: bool,
    ) -> Self {
        self.durable_memory_write_requested = durable_memory;
        self.genome_write_requested = genome;
        self.experiment_ledger_write_requested = experiment_ledger;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchDeploymentGuardDecision {
    Allow,
    Backpressure,
    Reject,
}

impl ResearchDeploymentGuardDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Backpressure => "backpressure",
            Self::Reject => "reject",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchDeploymentGuardReport {
    pub schema_version: &'static str,
    pub active_profile: ResearchDeploymentProfileKind,
    pub decision: ResearchDeploymentGuardDecision,
    pub reason_codes: Vec<String>,
    pub device_class: DeviceClass,
    pub adapter_hint: RuntimeAdapterHint,
    pub write_mode: ResearchDeploymentWriteMode,
    pub limits: ResearchDeploymentResourceLimits,
    pub requested_context_tokens: usize,
    pub requested_generation_tokens: usize,
    pub requested_kv_tokens: usize,
    pub requested_concurrency: usize,
    pub requested_background_reflections: usize,
    pub requested_stream_buffer_tokens: usize,
    pub requested_stream_chunks_in_flight: usize,
    pub streaming_enabled: bool,
    pub noncommercial_only: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub durable_write_allowed: bool,
    pub applied: bool,
}

impl ResearchDeploymentGuardReport {
    fn evaluate(profile: &ResearchDeploymentProfile, request: ResearchDeploymentRequest) -> Self {
        let mut reject_reasons = Vec::new();
        let mut pressure_reasons = Vec::new();
        push_limit(
            &mut reject_reasons,
            request.context_tokens,
            profile.limits.max_context_tokens,
            "context_tokens_exceed_profile_limit",
        );
        push_limit(
            &mut reject_reasons,
            request.generation_tokens,
            profile.limits.max_generation_tokens,
            "generation_tokens_exceed_profile_limit",
        );
        push_limit(
            &mut reject_reasons,
            request.kv_tokens,
            profile.limits.max_kv_tokens,
            "kv_tokens_exceed_profile_limit",
        );
        push_limit(
            &mut reject_reasons,
            request.concurrent_requests,
            profile.limits.max_concurrent_requests,
            "concurrency_exceeds_profile_limit",
        );
        if !profile.limits.allow_background_reflection && request.background_reflections > 0 {
            reject_reasons.push("background_reflection_disabled_for_profile".to_owned());
        }
        push_limit(
            &mut reject_reasons,
            request.background_reflections,
            profile.limits.max_background_reflections,
            "background_reflections_exceed_profile_limit",
        );
        if !profile.limits.allow_streaming && request.streaming_enabled {
            reject_reasons.push("streaming_disabled_for_profile".to_owned());
        }
        if request.streaming_enabled && profile.limits.allow_streaming {
            push_limit(
                &mut pressure_reasons,
                request.stream_buffer_tokens,
                profile.limits.max_stream_buffer_tokens,
                "stream_buffer_exceeds_backpressure_limit",
            );
            push_limit(
                &mut pressure_reasons,
                request.stream_chunks_in_flight,
                profile.limits.max_stream_chunks_in_flight,
                "stream_chunks_exceed_backpressure_limit",
            );
        }
        if request.cancellation_poll_ms > profile.limits.cancellation_poll_ms {
            pressure_reasons.push("cancellation_poll_slower_than_profile".to_owned());
        }
        if request.request_timeout_ms > profile.limits.request_timeout_ms {
            pressure_reasons.push("request_timeout_exceeds_profile".to_owned());
        }
        if request.durable_memory_write_requested
            || request.genome_write_requested
            || request.experiment_ledger_write_requested
        {
            reject_reasons.push("durable_writes_require_preview_to_write_gate".to_owned());
        }
        if !profile.noncommercial_only {
            reject_reasons.push("noncommercial_research_boundary_missing".to_owned());
        }

        let (decision, reason_codes) = if !reject_reasons.is_empty() {
            (ResearchDeploymentGuardDecision::Reject, reject_reasons)
        } else if !pressure_reasons.is_empty() {
            (
                ResearchDeploymentGuardDecision::Backpressure,
                pressure_reasons,
            )
        } else {
            (
                ResearchDeploymentGuardDecision::Allow,
                vec!["within_profile_budget".to_owned()],
            )
        };

        Self {
            schema_version: RESEARCH_DEPLOYMENT_SCHEMA_VERSION,
            active_profile: profile.kind,
            decision,
            reason_codes,
            device_class: profile.device_class,
            adapter_hint: profile.adapter_hint,
            write_mode: profile.write_guards.write_mode(),
            limits: profile.limits,
            requested_context_tokens: request.context_tokens,
            requested_generation_tokens: request.generation_tokens,
            requested_kv_tokens: request.kv_tokens,
            requested_concurrency: request.concurrent_requests,
            requested_background_reflections: request.background_reflections,
            requested_stream_buffer_tokens: request.stream_buffer_tokens,
            requested_stream_chunks_in_flight: request.stream_chunks_in_flight,
            streaming_enabled: request.streaming_enabled,
            noncommercial_only: profile.noncommercial_only,
            read_only: true,
            write_allowed: false,
            durable_write_allowed: false,
            applied: false,
        }
    }

    pub fn passed(&self) -> bool {
        self.decision == ResearchDeploymentGuardDecision::Allow
    }

    pub fn summary_line(&self) -> String {
        format!(
            "research_deployment_guard schema={} profile={} decision={} device={} adapter={} write_mode={} requested=context:{}|generation:{}|kv:{}|concurrency:{}|background_reflections:{}|stream_buffer:{}|stream_chunks:{} streaming={} limits=({}) noncommercial_only={} read_only={} write_allowed={} durable_write_allowed={} applied={} reasons={}",
            self.schema_version,
            self.active_profile.as_str(),
            self.decision.as_str(),
            self.device_class.as_str(),
            self.adapter_hint.as_str(),
            self.write_mode.as_str(),
            self.requested_context_tokens,
            self.requested_generation_tokens,
            self.requested_kv_tokens,
            self.requested_concurrency,
            self.requested_background_reflections,
            self.requested_stream_buffer_tokens,
            self.requested_stream_chunks_in_flight,
            self.streaming_enabled,
            self.limits.summary(),
            self.noncommercial_only,
            self.read_only,
            self.write_allowed,
            self.durable_write_allowed,
            self.applied,
            self.reason_codes.join("|")
        )
    }

    pub fn evidence_digest(&self) -> String {
        stable_digest(&self.summary_line())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchDeploymentOperatorHealth {
    pub schema_version: &'static str,
    pub active_profile: ResearchDeploymentProfileKind,
    pub device_class: DeviceClass,
    pub adapter_hint: RuntimeAdapterHint,
    pub write_mode: ResearchDeploymentWriteMode,
    pub limits: ResearchDeploymentResourceLimits,
    pub noncommercial_only: bool,
    pub durable_memory_writes: bool,
    pub genome_writes: bool,
    pub experiment_ledger_writes: bool,
    pub operator_approval_required: bool,
    pub privacy_gate_required: bool,
    pub preview_to_write_gate_required: bool,
    pub read_only: bool,
    pub write_allowed: bool,
    pub durable_write_allowed: bool,
    pub applied: bool,
}

impl ResearchDeploymentOperatorHealth {
    pub fn summary_line(&self) -> String {
        format!(
            "research_deployment_health schema={} active_profile={} write_mode={} device={} adapter={} noncommercial_only={} durable_memory_writes={} genome_writes={} experiment_ledger_writes={} operator_approval_required={} privacy_gate_required={} preview_to_write_gate_required={} limits=({}) read_only={} write_allowed={} durable_write_allowed={} applied={}",
            self.schema_version,
            self.active_profile.as_str(),
            self.write_mode.as_str(),
            self.device_class.as_str(),
            self.adapter_hint.as_str(),
            self.noncommercial_only,
            self.durable_memory_writes,
            self.genome_writes,
            self.experiment_ledger_writes,
            self.operator_approval_required,
            self.privacy_gate_required,
            self.preview_to_write_gate_required,
            self.limits.summary(),
            self.read_only,
            self.write_allowed,
            self.durable_write_allowed,
            self.applied
        )
    }

    pub fn evidence_digest(&self) -> String {
        stable_digest(&self.summary_line())
    }
}

pub fn default_research_deployment_profiles() -> Vec<ResearchDeploymentProfile> {
    ResearchDeploymentProfileKind::expected_profiles()
        .into_iter()
        .map(ResearchDeploymentProfile::template)
        .collect()
}

pub fn parse_research_deployment_profile(
    value: &str,
) -> Result<ResearchDeploymentProfileKind, String> {
    let normalized = value.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    match normalized.as_str() {
        "cpu" | "cpu-only" | "local-cpu" => Ok(ResearchDeploymentProfileKind::CpuOnly),
        "gpu" | "single-gpu" | "local-gpu" | "cuda" => Ok(ResearchDeploymentProfileKind::SingleGpu),
        "low-memory" | "lowmem" | "constrained" | "small" => {
            Ok(ResearchDeploymentProfileKind::LowMemory)
        }
        "benchmark" | "replay" | "benchmark-replay" | "bench" => {
            Ok(ResearchDeploymentProfileKind::BenchmarkReplay)
        }
        "" => Err("deployment profile is empty".to_owned()),
        other => Err(format!("unknown research deployment profile: {other}")),
    }
}

pub fn parse_research_sandbox_target(value: &str) -> Result<ResearchSandboxTarget, String> {
    let normalized = value.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    match normalized.as_str() {
        "local" | "host" => Ok(ResearchSandboxTarget::Local),
        "wsl" | "wsl2" => Ok(ResearchSandboxTarget::Wsl),
        "container" | "docker" | "podman" => Ok(ResearchSandboxTarget::Container),
        "small-vps" | "vps" | "small-vm" => Ok(ResearchSandboxTarget::SmallVps),
        "" => Err("research sandbox target is empty".to_owned()),
        other => Err(format!("unknown research sandbox target: {other}")),
    }
}

fn push_limit(reasons: &mut Vec<String>, requested: usize, limit: usize, reason: &'static str) {
    if requested > limit {
        reasons.push(reason.to_owned());
    }
}

fn stable_digest(value: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("fnv64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enterprise_sidecar_defaults_to_community_mode_without_endpoint() {
        let report = EnterpriseSidecarBoundaryReport::community();
        let line = report.summary_line();

        assert_eq!(report.mode, EnterpriseSidecarMode::Community);
        assert_eq!(
            report.reachability,
            EnterpriseSidecarReachability::NotConfigured
        );
        assert!(!report.endpoint_configured);
        assert_eq!(report.endpoint_digest, None);
        assert!(report.community_fallback);
        assert_eq!(report.public_artifact, "gpl_core_only");
        assert_eq!(report.enterprise_artifact, "separate_sidecar_or_container");
        assert!(report.private_capabilities.contains(&"license_validation"));
        assert!(!report.direct_proprietary_link_allowed);
        assert!(report.read_only);
        assert!(!report.write_allowed);
        assert!(!report.applied);
        assert!(line.contains("mode=community"));
        assert!(line.contains("endpoint_digest=none"));
    }

    #[test]
    fn enterprise_sidecar_unreachable_endpoint_degrades_to_community_mode() {
        let report = EnterpriseSidecarBoundaryReport::evaluate(
            Some("http://127.0.0.1:19099/token-secret"),
            false,
        );
        let line = report.summary_line();

        assert_eq!(report.mode, EnterpriseSidecarMode::Community);
        assert_eq!(
            report.reachability,
            EnterpriseSidecarReachability::Unreachable
        );
        assert!(report.endpoint_configured);
        assert!(
            report
                .endpoint_digest
                .as_deref()
                .unwrap()
                .starts_with("fnv64:")
        );
        assert!(report.community_fallback);
        assert!(!line.contains("token-secret"));
        assert!(line.contains("direct_proprietary_link_allowed=false"));
    }

    #[test]
    fn enterprise_sidecar_reachable_endpoint_stays_outside_gpl_core() {
        let report = EnterpriseSidecarBoundaryReport::evaluate(
            Some("https://enterprise-sidecar.local"),
            true,
        );

        assert_eq!(report.mode, EnterpriseSidecarMode::EnterpriseSidecar);
        assert_eq!(
            report.reachability,
            EnterpriseSidecarReachability::Reachable
        );
        assert!(!report.community_fallback);
        assert_eq!(report.public_artifact, "gpl_core_only");
        assert_eq!(report.enterprise_artifact, "separate_sidecar_or_container");
        assert!(report.private_capabilities.contains(&"private_connectors"));
        assert!(!report.direct_proprietary_link_allowed);
        assert!(report.evidence_digest().starts_with("fnv64:"));
    }

    #[test]
    fn public_cargo_manifests_do_not_link_enterprise_sidecar_artifacts() {
        let manifests = [
            include_str!("../Cargo.toml"),
            include_str!("../crates/norion-agent/Cargo.toml"),
            include_str!("../crates/norion-cli/Cargo.toml"),
            include_str!("../crates/norion-core/Cargo.toml"),
            include_str!("../crates/norion-eval/Cargo.toml"),
            include_str!("../crates/norion-memory/Cargo.toml"),
            include_str!("../crates/norion-service/Cargo.toml"),
            include_str!("../crates/norion-test/Cargo.toml"),
            include_str!("../tools/evolution-loop/Cargo.toml"),
            include_str!("../tools/model-pool-advice-core/Cargo.toml"),
            include_str!("../tools/rustgpt-lab/Cargo.toml"),
            include_str!("../tools/smartsteam-forge/Cargo.toml"),
        ];

        assert!(gpl_core_manifest_boundary_findings(&manifests).is_empty());

        let bad_manifest =
            "[dependencies]\nenterprise_connector = { path = \"vendor/closed.dll\" }\n";
        let findings = gpl_core_manifest_boundary_findings(&[bad_manifest]);
        assert!(findings.contains(&"enterprise_connector_marker"));
        assert!(findings.contains(&"native_dynamic_library_link"));
    }

    #[test]
    fn research_sandbox_evidence_covers_targets_as_redacted_issue_comment_packets() {
        let profile = ResearchDeploymentProfile::template(ResearchDeploymentProfileKind::CpuOnly);

        for target in ResearchSandboxTarget::expected_targets() {
            let report = ResearchSandboxEvidenceReport::from_profile(target, &profile);
            let line = report.summary_line();

            assert_eq!(
                report.schema_version,
                RESEARCH_SANDBOX_EVIDENCE_SCHEMA_VERSION
            );
            assert_eq!(report.profile, ResearchDeploymentProfileKind::CpuOnly);
            assert!(report.noncommercial_only);
            assert!(report.contributor_pr_only);
            assert!(report.maintainer_approval_required);
            assert!(report.persistent_state.contains(&"disk_kv_cache"));
            assert!(
                report
                    .persistent_state
                    .contains(&"redacted_evidence_packets")
            );
            assert!(report.local_only_data.contains(&"raw_traces"));
            assert!(!report.private_trace_publish_allowed);
            assert!(report.redacted_issue_comment_ready);
            assert!(report.wipe_test_state_supported);
            assert!(report.preview_only);
            assert!(!report.write_allowed);
            assert!(!report.durable_write_allowed);
            assert!(!report.applied);
            assert!(report.issue_comment_safe());
            assert!(report.evidence_digest().starts_with("fnv64:"));
            assert!(line.contains(target.as_str()));
            assert!(!line.contains("C:\\"));
            assert!(!line.contains("/home/"));
            assert!(!line.contains("secret="));
        }
    }

    #[test]
    fn research_sandbox_target_parser_accepts_runtime_aliases() {
        assert_eq!(
            parse_research_sandbox_target("host").unwrap(),
            ResearchSandboxTarget::Local
        );
        assert_eq!(
            parse_research_sandbox_target("wsl2").unwrap(),
            ResearchSandboxTarget::Wsl
        );
        assert_eq!(
            parse_research_sandbox_target("podman").unwrap(),
            ResearchSandboxTarget::Container
        );
        assert_eq!(
            parse_research_sandbox_target("small vm").unwrap(),
            ResearchSandboxTarget::SmallVps
        );
        assert!(parse_research_sandbox_target("commercial-prod").is_err());
    }

    #[test]
    fn default_profiles_cover_local_research_modes_with_disabled_writes() {
        let profiles = default_research_deployment_profiles();

        assert_eq!(profiles.len(), 4);
        for expected in ResearchDeploymentProfileKind::expected_profiles() {
            let profile = profiles
                .iter()
                .find(|profile| profile.kind == expected)
                .expect("expected deployment profile");
            assert_eq!(profile.schema_version, RESEARCH_DEPLOYMENT_SCHEMA_VERSION);
            assert!(profile.noncommercial_only);
            assert!(!profile.write_guards.durable_memory_writes);
            assert!(!profile.write_guards.genome_writes);
            assert!(!profile.write_guards.experiment_ledger_writes);
            assert!(profile.write_guards.operator_approval_required);
            assert!(profile.write_guards.privacy_gate_required);
            assert!(profile.write_guards.preview_to_write_gate_required);
            assert_eq!(
                profile.write_guards.write_mode(),
                ResearchDeploymentWriteMode::PreviewOnly
            );
        }
    }

    #[test]
    fn profile_parsing_accepts_aliases_and_rejects_unknown_values() {
        assert_eq!(
            parse_research_deployment_profile("cpu").unwrap(),
            ResearchDeploymentProfileKind::CpuOnly
        );
        assert_eq!(
            parse_research_deployment_profile("single_gpu").unwrap(),
            ResearchDeploymentProfileKind::SingleGpu
        );
        assert_eq!(
            parse_research_deployment_profile("constrained").unwrap(),
            ResearchDeploymentProfileKind::LowMemory
        );
        assert_eq!(
            parse_research_deployment_profile("replay").unwrap(),
            ResearchDeploymentProfileKind::BenchmarkReplay
        );
        assert!(parse_research_deployment_profile("commercial-prod").is_err());
    }

    #[test]
    fn resource_guard_allows_requests_inside_profile_budget() {
        let profile = ResearchDeploymentProfile::template(ResearchDeploymentProfileKind::CpuOnly);
        let report = profile.guard(ResearchDeploymentRequest::fixture_for(&profile));

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.decision, ResearchDeploymentGuardDecision::Allow);
        assert!(
            report
                .reason_codes
                .contains(&"within_profile_budget".to_owned())
        );
        assert!(report.read_only);
        assert!(!report.write_allowed);
        assert!(!report.durable_write_allowed);
        assert!(!report.applied);
        assert!(report.evidence_digest().starts_with("fnv64:"));
    }

    #[test]
    fn resource_guard_rejects_unbounded_context_kv_concurrency_and_writes() {
        let profile = ResearchDeploymentProfile::template(ResearchDeploymentProfileKind::LowMemory);
        let request = ResearchDeploymentRequest::fixture_for(&profile)
            .with_context_tokens(profile.limits.max_context_tokens + 1)
            .with_generation_tokens(profile.limits.max_generation_tokens + 1)
            .with_kv_tokens(profile.limits.max_kv_tokens + 1)
            .with_concurrent_requests(profile.limits.max_concurrent_requests + 1)
            .with_write_requests(true, true, true);

        let report = profile.guard(request);

        assert_eq!(report.decision, ResearchDeploymentGuardDecision::Reject);
        for reason in [
            "context_tokens_exceed_profile_limit",
            "generation_tokens_exceed_profile_limit",
            "kv_tokens_exceed_profile_limit",
            "concurrency_exceeds_profile_limit",
            "durable_writes_require_preview_to_write_gate",
        ] {
            assert!(
                report.reason_codes.contains(&reason.to_owned()),
                "{:?}",
                report.reason_codes
            );
        }
        assert!(!report.summary_line().contains("write_allowed=true"));
    }

    #[test]
    fn streaming_over_budget_enters_backpressure_without_enabling_writes() {
        let profile = ResearchDeploymentProfile::template(ResearchDeploymentProfileKind::SingleGpu);
        let request = ResearchDeploymentRequest::fixture_for(&profile).with_streaming(
            true,
            profile.limits.max_stream_buffer_tokens + 1,
            profile.limits.max_stream_chunks_in_flight + 1,
        );

        let report = profile.guard(request);

        assert_eq!(
            report.decision,
            ResearchDeploymentGuardDecision::Backpressure
        );
        assert!(
            report
                .reason_codes
                .contains(&"stream_buffer_exceeds_backpressure_limit".to_owned())
        );
        assert!(
            report
                .reason_codes
                .contains(&"stream_chunks_exceed_backpressure_limit".to_owned())
        );
        assert!(!report.write_allowed);
        assert!(!report.durable_write_allowed);
    }

    #[test]
    fn benchmark_replay_disables_streaming_and_background_reflection() {
        let profile =
            ResearchDeploymentProfile::template(ResearchDeploymentProfileKind::BenchmarkReplay);
        let request = ResearchDeploymentRequest::fixture_for(&profile)
            .with_background_reflections(1)
            .with_streaming(true, 1, 1);

        let report = profile.guard(request);

        assert_eq!(report.decision, ResearchDeploymentGuardDecision::Reject);
        assert!(
            report
                .reason_codes
                .contains(&"background_reflection_disabled_for_profile".to_owned())
        );
        assert!(
            report
                .reason_codes
                .contains(&"streaming_disabled_for_profile".to_owned())
        );
    }

    #[test]
    fn operator_health_reports_active_profile_capability_and_budget_limits() {
        let profile = ResearchDeploymentProfile::template(ResearchDeploymentProfileKind::SingleGpu);
        let health = profile.operator_health();
        let line = health.summary_line();

        assert_eq!(
            health.active_profile,
            ResearchDeploymentProfileKind::SingleGpu
        );
        assert_eq!(health.device_class, DeviceClass::DiscreteGpu);
        assert_eq!(health.adapter_hint, RuntimeAdapterHint::Cuda);
        assert_eq!(health.write_mode, ResearchDeploymentWriteMode::PreviewOnly);
        assert!(line.contains("active_profile=single-gpu"));
        assert!(line.contains("write_mode=preview-only"));
        assert!(line.contains("adapter=cuda"));
        assert!(line.contains("context=16384"));
        assert!(line.contains("noncommercial_only=true"));
        assert!(line.contains("write_allowed=false"));
        assert!(health.evidence_digest().starts_with("fnv64:"));
    }
}
