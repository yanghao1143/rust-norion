use crate::adaptive_state::{EvolutionLedger, GenomeEvolutionApplyReceipt, LiveInferenceEvolution};
use crate::agent_team::AgentTeamPlan;
use crate::drift::DriftReport;
use crate::experience::{ExperienceMatch, ExperienceRuntimeTokenMetrics};
use crate::experience_replay::ExperienceReplayReport;
use crate::gist_memory::GistRecord;
use crate::hardware::HardwarePlan;
use crate::hierarchy::{HierarchyWeights, TaskAwareHierarchyPlan, TaskProfile};
use crate::homeostasis::HomeostaticGateReport;
use crate::infini_memory::InfiniMemoryPlan;
use crate::kv_cache::{
    MemoryCompactionPolicy, MemoryCompactionReport, MemoryMatch, MemoryRetentionPolicy,
    MemoryUpdateReport, RetentionReport,
};
use crate::memory_admission::MemoryAdmissionPreview;
use crate::privacy_redaction::stable_redaction_digest;
use crate::process_reward::ProcessRewardReport;
use crate::reasoning_genome::{
    DnaEvolutionApplyPlan, DnaEvolutionControllerReport, DnaEvolutionPolicy,
    DnaEvolutionValidationEvidence, DnaEvolutionValidationStatus, DnaGeneChain, DnaSplicePreview,
    GenePurposeRelabelProposal, GeneScissorsIntent, GeneScissorsTransactionJournal,
    GeneValidationStatus, GenomeExpression, MutationPlan, ReasoningFrame, ReasoningGenome,
    ReasoningGenomeStrategy, TaskGeneAdmissionReview, TaskGeneCascade, TaskSkillGeneCandidate,
    TaskSkillGeneEvidence,
};
use crate::recursive_scheduler::RecursiveSchedule;
use crate::reflection::{
    DraftToken, InferenceDraft, ReasoningStep, ReflectionReport, RuntimeDiagnostics,
};
use crate::router::{AdaptiveRoutingPlan, ComputeBudgetSchedule, GenerationMetrics, RouteBudget};
use crate::runtime::RuntimeAdapterObservation;
use crate::runtime::RuntimeError;
use crate::self_evolution::SelfEvolutionPromotionPreflightReport;
use crate::tenant_scope::TenantScope;
use crate::tiered_cache::{TierMigration, TieredCachePlan};
use crate::token_stream::TokenWindowReport;
use crate::toolsmith::ToolsmithPlan;
use crate::transformer::TransformerRefactorPlan;
use norion_agent::AgentModelRouteProof;

use crate::writer_gate::UnifiedWriterGateReport;

#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub prompt: String,
    pub profile: TaskProfile,
    pub max_tokens: Option<usize>,
    pub tenant_scope: Option<TenantScope>,
    pub agent_team_route_proof: Option<AgentModelRouteProof>,
    pub genome_evolution_authorization: Option<GenomeEvolutionAuthorization>,
}

impl InferenceRequest {
    pub fn new(prompt: impl Into<String>, profile: TaskProfile) -> Self {
        Self {
            prompt: prompt.into(),
            profile,
            max_tokens: None,
            tenant_scope: None,
            agent_team_route_proof: None,
            genome_evolution_authorization: None,
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: Option<usize>) -> Self {
        self.max_tokens = max_tokens.map(|value| value.max(1));
        self
    }

    pub fn with_tenant_scope(mut self, tenant_scope: TenantScope) -> Self {
        self.tenant_scope = Some(tenant_scope);
        self
    }

    pub fn with_agent_team_route_proof(mut self, route_proof: AgentModelRouteProof) -> Self {
        self.agent_team_route_proof = Some(route_proof);
        self
    }

    pub fn with_genome_evolution_authorization(
        mut self,
        authorization: GenomeEvolutionAuthorization,
    ) -> Self {
        self.genome_evolution_authorization = Some(authorization);
        self
    }

    pub fn try_with_agent_team_route_plan_json(
        self,
        route_plan_json: &str,
    ) -> Result<Self, String> {
        let route_proof = agent_model_route_proof_from_route_plan_json(route_plan_json)?;
        Ok(self.with_agent_team_route_proof(route_proof))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenomeEvolutionAuthorization {
    validation: DnaEvolutionValidationEvidence,
    task_skill_evidence: TaskSkillGeneEvidence,
    approval_ref: String,
    rollback: bool,
}

pub const GENOME_EVOLUTION_PREVIEW_SCHEMA_VERSION: &str = "genome_evolution_preview_v1";

#[derive(Debug, Clone, PartialEq)]
pub struct GenomeEvolutionPreview {
    pub schema_version: &'static str,
    pub profile: TaskProfile,
    pub generation_before: u64,
    pub source_genome_id: String,
    pub reasoning_frame_id: String,
    pub candidate: ReasoningGenome,
    pub plans: Vec<MutationPlan>,
    pub candidate_digest: String,
    pub quality_milli: u16,
    pub process_reward_milli: i32,
    pub critical_reflection_issues: usize,
    pub contradiction_count: usize,
    pub output_integrity_passed: bool,
    pub reasoning_frame_valid: bool,
    pub expression_vm_executed: bool,
    pub transaction_replay_passed: bool,
    pub preview_only: bool,
}

impl GenomeEvolutionPreview {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        profile: TaskProfile,
        generation_before: u64,
        reasoning_frame: &ReasoningFrame,
        candidate: ReasoningGenome,
        plans: Vec<MutationPlan>,
        quality: f32,
        process_reward: f32,
        critical_reflection_issues: usize,
        contradiction_count: usize,
        answer: &str,
        transaction_replay_passed: bool,
        preview_only: bool,
    ) -> Self {
        let source_genome_id = candidate.id.clone();
        let reasoning_frame_id = reasoning_frame.frame_id.clone();
        let quality_milli = bounded_milli(quality);
        let process_reward_milli = bounded_signed_milli(process_reward);
        let output_integrity_passed = evolution_output_integrity_passes(answer);
        let reasoning_frame_valid = reasoning_frame.validate_preview().is_ok();
        let expression_vm_executed =
            reasoning_frame.executed_opcodes == reasoning_frame.genome_isa.opcodes;
        let candidate_digest = genome_evolution_preview_digest(
            profile,
            generation_before,
            &reasoning_frame_id,
            &candidate,
            &plans,
            quality_milli,
            process_reward_milli,
            critical_reflection_issues,
            contradiction_count,
            output_integrity_passed,
            reasoning_frame_valid,
            expression_vm_executed,
            transaction_replay_passed,
            preview_only,
        );
        Self {
            schema_version: GENOME_EVOLUTION_PREVIEW_SCHEMA_VERSION,
            profile,
            generation_before,
            source_genome_id,
            reasoning_frame_id,
            candidate,
            plans,
            candidate_digest,
            quality_milli,
            process_reward_milli,
            critical_reflection_issues,
            contradiction_count,
            output_integrity_passed,
            reasoning_frame_valid,
            expression_vm_executed,
            transaction_replay_passed,
            preview_only,
        }
    }

    pub fn candidate_count(&self) -> usize {
        self.plans.len()
    }

    pub fn eligibility_reason(&self) -> Option<&'static str> {
        if self.schema_version != GENOME_EVOLUTION_PREVIEW_SCHEMA_VERSION {
            return Some("preview_schema_mismatch");
        }
        if !self.preview_only {
            return Some("preview_source_not_read_only");
        }
        if self.plans.is_empty() {
            return Some("no_mutation_candidate");
        }
        if self.plans.iter().any(|plan| !plan.is_read_only_preview()) {
            return Some("mutation_candidate_not_read_only");
        }
        if self
            .plans
            .iter()
            .any(|plan| plan.validation_status == GeneValidationStatus::Failed)
        {
            return Some("mutation_candidate_validation_failed");
        }
        if self
            .plans
            .iter()
            .any(|plan| plan.intent == GeneScissorsIntent::Rollback)
        {
            return Some("explicit_rollback_requires_rollback_token");
        }
        if !self.reasoning_frame_valid {
            return Some("reasoning_frame_invalid");
        }
        if !self.expression_vm_executed {
            return Some("expression_vm_incomplete");
        }
        if !self.transaction_replay_passed {
            return Some("transaction_replay_failed");
        }
        if self.critical_reflection_issues > 0 {
            return Some("critical_reflection_issue");
        }
        if self.contradiction_count > 0 {
            return Some("reflection_contradiction");
        }
        if !self.output_integrity_passed {
            return Some("output_integrity_failed");
        }
        if self.quality_milli < 750 {
            return Some("quality_below_evolution_gate");
        }
        if self.process_reward_milli < 500 {
            return Some("process_reward_below_evolution_gate");
        }
        if self.candidate_digest != self.recomputed_candidate_digest() {
            return Some("candidate_digest_mismatch");
        }
        None
    }

    pub fn is_eligible(&self) -> bool {
        self.eligibility_reason().is_none()
    }

    pub fn recomputed_candidate_digest(&self) -> String {
        genome_evolution_preview_digest(
            self.profile,
            self.generation_before,
            &self.reasoning_frame_id,
            &self.candidate,
            &self.plans,
            self.quality_milli,
            self.process_reward_milli,
            self.critical_reflection_issues,
            self.contradiction_count,
            self.output_integrity_passed,
            self.reasoning_frame_valid,
            self.expression_vm_executed,
            self.transaction_replay_passed,
            self.preview_only,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenomeEvolutionExplicitApplyReport {
    pub candidate_digest: String,
    pub controller: DnaEvolutionControllerReport,
    pub writer_gate: UnifiedWriterGateReport,
    pub apply_plan: DnaEvolutionApplyPlan,
    pub receipt: GenomeEvolutionApplyReceipt,
}

fn genome_evolution_preview_digest(
    profile: TaskProfile,
    generation_before: u64,
    reasoning_frame_id: &str,
    candidate: &ReasoningGenome,
    plans: &[MutationPlan],
    quality_milli: u16,
    process_reward_milli: i32,
    critical_reflection_issues: usize,
    contradiction_count: usize,
    output_integrity_passed: bool,
    reasoning_frame_valid: bool,
    expression_vm_executed: bool,
    transaction_replay_passed: bool,
    preview_only: bool,
) -> String {
    let candidate_snapshot = format!("{candidate:?}");
    let plan_snapshot = format!("{plans:?}");
    let generation = generation_before.to_string();
    let runtime_evidence = format!(
        "quality={quality_milli};reward={process_reward_milli};critical={critical_reflection_issues};contradictions={contradiction_count};output_integrity={output_integrity_passed};frame={reasoning_frame_valid};vm={expression_vm_executed};replay={transaction_replay_passed};preview={preview_only}"
    );
    stable_redaction_digest([
        GENOME_EVOLUTION_PREVIEW_SCHEMA_VERSION,
        task_profile_slug(profile),
        generation.as_str(),
        reasoning_frame_id,
        candidate_snapshot.as_str(),
        plan_snapshot.as_str(),
        runtime_evidence.as_str(),
    ])
}

pub(crate) fn evolution_output_integrity_passes(answer: &str) -> bool {
    let answer = answer.trim();
    if answer.chars().count() < 64 {
        return false;
    }
    if answer.matches("```").count() % 2 != 0 {
        return false;
    }
    let inline_backticks = answer.replace("```", "").matches('`').count();
    if inline_backticks % 2 != 0 {
        return false;
    }
    let complete_ending = answer.ends_with("```")
        || answer.chars().last().is_some_and(|character| {
            matches!(
                character,
                '.' | '。'
                    | '!'
                    | '！'
                    | '?'
                    | '？'
                    | ';'
                    | '；'
                    | ')'
                    | '）'
                    | ']'
                    | '】'
                    | '}'
                    | '"'
                    | '\''
            )
        });
    if !complete_ending {
        return false;
    }

    let mut clauses = std::collections::HashMap::new();
    for clause in answer.split(['。', '！', '？', ';', '；', ',', '，', '\n']) {
        let normalized = clause.split_whitespace().collect::<String>();
        if normalized.chars().count() < 12 {
            continue;
        }
        let count = clauses.entry(normalized).or_insert(0usize);
        *count += 1;
        if *count >= 3 {
            return false;
        }
    }
    true
}

fn task_profile_slug(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long",
    }
}

fn bounded_milli(value: f32) -> u16 {
    if value.is_finite() {
        (value.clamp(0.0, 1.0) * 1000.0).round() as u16
    } else {
        0
    }
}

fn bounded_signed_milli(value: f32) -> i32 {
    if value.is_finite() {
        (value.clamp(-1.0, 1.0) * 1000.0).round() as i32
    } else {
        i32::MIN
    }
}

#[cfg(test)]
mod evolution_preview_tests {
    use super::bounded_signed_milli;

    #[test]
    fn process_reward_milli_is_bounded_to_signed_unit_range() {
        assert_eq!(bounded_signed_milli(5.0), 1000);
        assert_eq!(bounded_signed_milli(-5.0), -1000);
        assert_eq!(bounded_signed_milli(f32::NAN), i32::MIN);
    }
}

impl GenomeEvolutionAuthorization {
    pub fn from_promotion_preflight(
        preflight: &SelfEvolutionPromotionPreflightReport,
        task_skill_evidence: TaskSkillGeneEvidence,
        rollback: bool,
    ) -> Result<Self, String> {
        if !preflight.is_ready_for_explicit_promotion() {
            return Err("genome evolution promotion preflight is not ready".to_owned());
        }
        if task_skill_evidence.validation_status() != GeneValidationStatus::Passed
            || !task_skill_evidence.user_approved
        {
            return Err("genome evolution task skill evidence is not approved".to_owned());
        }
        let validation = validation_from_promotion_preflight(preflight);
        if validation.status(DnaEvolutionPolicy::default()) != DnaEvolutionValidationStatus::Passed
        {
            return Err("genome evolution validation evidence is incomplete".to_owned());
        }
        let mode = if rollback { "rollback" } else { "apply" };
        let approval_ref = stable_redaction_digest([
            "genome-evolution-authorization-v1",
            preflight.candidate_id.as_str(),
            preflight.content_digest.as_str(),
            mode,
        ]);
        Ok(Self {
            validation,
            task_skill_evidence,
            approval_ref,
            rollback,
        })
    }

    #[cfg(test)]
    pub(crate) fn apply(
        validation: DnaEvolutionValidationEvidence,
        approval_ref: impl Into<String>,
    ) -> Self {
        Self {
            validation,
            task_skill_evidence: TaskSkillGeneEvidence::passing(),
            approval_ref: approval_ref.into(),
            rollback: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn rollback(
        validation: DnaEvolutionValidationEvidence,
        approval_ref: impl Into<String>,
    ) -> Self {
        Self {
            validation,
            task_skill_evidence: TaskSkillGeneEvidence::passing(),
            approval_ref: approval_ref.into(),
            rollback: true,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.approval_ref.trim().is_empty()
            && self.validation.status(DnaEvolutionPolicy::default())
                == DnaEvolutionValidationStatus::Passed
            && self.task_skill_evidence.validation_status() == GeneValidationStatus::Passed
            && self.task_skill_evidence.user_approved
    }

    pub fn validation(&self) -> &DnaEvolutionValidationEvidence {
        &self.validation
    }

    pub fn task_skill_evidence(&self) -> &TaskSkillGeneEvidence {
        &self.task_skill_evidence
    }

    pub fn approval_ref(&self) -> &str {
        &self.approval_ref
    }

    pub fn rollback_requested(&self) -> bool {
        self.rollback
    }
}

fn validation_from_promotion_preflight(
    preflight: &SelfEvolutionPromotionPreflightReport,
) -> DnaEvolutionValidationEvidence {
    DnaEvolutionValidationEvidence {
        compiler_passed: preflight.rust_validation_passed,
        tests_passed: preflight.validation_passed,
        benchmark_passed: preflight.benchmark_gate_passed,
        trace_gate_passed: preflight.source_report_schema_count > 0,
        privacy_gate_passed: preflight.adaptive_preview_evidence_present,
        canary_replay_passed: preflight.evidence_id_count > 0,
        rollback_replay_passed: preflight.rollback_anchor_count > 0,
        artifact_digests: vec![
            stable_redaction_digest([
                "genome-authorization-candidate",
                preflight.candidate_id.as_str(),
            ]),
            stable_redaction_digest([
                "genome-authorization-preflight",
                preflight.content_digest.as_str(),
            ]),
            stable_redaction_digest([
                "genome-authorization-evidence",
                preflight.evidence_id_count.to_string().as_str(),
                preflight.source_report_schema_count.to_string().as_str(),
            ]),
            stable_redaction_digest([
                "genome-authorization-rollback",
                preflight.rollback_anchor_count.to_string().as_str(),
                preflight.review_packet_count.to_string().as_str(),
            ]),
        ],
    }
}

fn agent_model_route_proof_from_route_plan_json(
    route_plan_json: &str,
) -> Result<AgentModelRouteProof, String> {
    if json_bool_field(route_plan_json, "ok") == Some(false) {
        let error =
            json_string_field(route_plan_json, "error").unwrap_or_else(|| "unknown".to_owned());
        return Err(format!("model pool route failed: {error}"));
    }
    for (field, expected) in [
        ("read_only", true),
        ("launches_process", false),
        ("sends_prompt", false),
    ] {
        let value = json_bool_field(route_plan_json, field)
            .ok_or_else(|| format!("model pool route response missing {field} contract field"))?;
        if value != expected {
            return Err(format!(
                "model pool route response failed safety contract: {field}={value}"
            ));
        }
    }
    if json_bool_field(route_plan_json, "route_allowed") != Some(true) {
        let reason =
            json_string_field(route_plan_json, "reason").unwrap_or_else(|| "unknown".to_owned());
        return Err(format!("model pool route is blocked: {reason}"));
    }

    let selected_role = required_json_string_field(route_plan_json, "selected_role")?;
    let source = json_object_field(route_plan_json, "agent_model_route_source")
        .ok_or_else(|| "model pool route missing agent_model_route_source".to_owned())?;
    if json_bool_field(&source, "route_allowed") != Some(true) {
        return Err("model pool route source proof blocks route".to_owned());
    }
    if json_bool_field(&source, "proof_ready") != Some(true) {
        let reason = json_string_field(&source, "proof_block_reason")
            .unwrap_or_else(|| "unknown".to_owned());
        return Err(format!("model pool route source proof not ready: {reason}"));
    }

    let source_role = required_agent_route_source_field(&source, "selected_role")?;
    if source_role != selected_role {
        return Err(format!(
            "model pool route source selected_role mismatch: selected_role={selected_role} proof_selected_role={source_role}"
        ));
    }

    Ok(AgentModelRouteProof::new(
        required_agent_route_source_field(&source, "model_registry_id")?,
        required_agent_route_source_field(&source, "model_profile_id")?,
        required_agent_route_source_field(&source, "inference_backend_id")?,
        required_agent_route_source_field(&source, "model_pool_id")?,
    )
    .with_selected_role(source_role))
}

fn required_agent_route_source_field(source: &str, field: &str) -> Result<String, String> {
    required_json_string_field(source, field)
        .map_err(|_| format!("model pool route source proof missing {field}"))
}

fn required_json_string_field(body: &str, field: &str) -> Result<String, String> {
    json_string_field(body, field)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("model pool route missing {field}"))
}

fn json_string_field(body: &str, field: &str) -> Option<String> {
    let value = json_value_after_colon(body, field)?;
    parse_json_string(value).map(|(parsed, _)| parsed)
}

fn json_bool_field(body: &str, field: &str) -> Option<bool> {
    let value = json_value_after_colon(body, field)?;
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        parse_json_string(value).and_then(|(parsed, _)| match parsed.trim() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        })
    }
}

fn json_object_field(body: &str, field: &str) -> Option<String> {
    parse_json_object(json_value_after_colon(body, field)?).map(ToOwned::to_owned)
}

fn json_value_after_colon<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("\"{field}\"");
    let after_field = body.get(body.find(&needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?;
    Some(after_colon.trim_start())
}

fn parse_json_string(input: &str) -> Option<(String, usize)> {
    let mut chars = input.char_indices();
    if chars.next()?.1 != '"' {
        return None;
    }

    let mut output = String::new();
    let mut escaped = false;
    for (index, character) in chars {
        if escaped {
            match character {
                '"' => output.push('"'),
                '\\' => output.push('\\'),
                '/' => output.push('/'),
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                'b' => output.push('\u{0008}'),
                'f' => output.push('\u{000c}'),
                other => output.push(other),
            }
            escaped = false;
            continue;
        }

        match character {
            '\\' => escaped = true,
            '"' => return Some((output, index + character.len_utf8())),
            other => output.push(other),
        }
    }

    None
}

fn parse_json_object(input: &str) -> Option<&str> {
    let mut chars = input.char_indices();
    if chars.next()?.1 != '{' {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => depth = depth.saturating_add(1),
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return input.get(..=index);
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct GenerationContext<'a> {
    pub prompt: &'a str,
    pub profile: TaskProfile,
    pub tenant_scope: Option<&'a TenantScope>,
    pub memories: &'a [MemoryMatch],
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub tier_plan: &'a TieredCachePlan,
    pub infini_memory_plan: &'a InfiniMemoryPlan,
    pub recursive_schedule: &'a RecursiveSchedule,
    pub hardware_plan: &'a HardwarePlan,
    pub experiences: &'a [ExperienceMatch],
    pub toolsmith_plan: &'a ToolsmithPlan,
    pub agent_team_plan: &'a AgentTeamPlan,
    pub transformer_plan: &'a TransformerRefactorPlan,
}

impl<'a> GenerationContext<'a> {
    pub(super) fn with_prompt<'b>(&'b self, prompt: &'b str) -> GenerationContext<'b>
    where
        'a: 'b,
    {
        GenerationContext {
            prompt,
            profile: self.profile,
            tenant_scope: self.tenant_scope,
            memories: self.memories,
            route_budget: self.route_budget,
            hierarchy: self.hierarchy,
            tier_plan: self.tier_plan,
            infini_memory_plan: self.infini_memory_plan,
            recursive_schedule: self.recursive_schedule,
            hardware_plan: self.hardware_plan,
            experiences: self.experiences,
            toolsmith_plan: self.toolsmith_plan,
            agent_team_plan: self.agent_team_plan,
            transformer_plan: self.transformer_plan,
        }
    }
}

pub trait InferenceBackend {
    fn configure_generation(&mut self, _max_tokens: Option<usize>) {}

    fn configure_runtime_endpoint_override(
        &mut self,
        _base_url: Option<&str>,
    ) -> Result<bool, String> {
        Ok(false)
    }

    fn runtime_endpoint_override_active(&self) -> Option<&str> {
        None
    }

    fn runtime_native_context_window(&self) -> Option<usize> {
        None
    }

    fn embed_text(&mut self, _text: &str) -> Option<Vec<f32>> {
        None
    }

    fn generate(&mut self, context: GenerationContext<'_>) -> InferenceDraft;

    fn generate_stream(
        &mut self,
        context: GenerationContext<'_>,
        on_token: &mut dyn FnMut(&DraftToken),
    ) -> InferenceDraft {
        let mut checked = |token: &DraftToken| {
            on_token(token);
            Ok(())
        };
        self.generate_stream_checked(context, &mut checked)
    }

    fn generate_stream_checked(
        &mut self,
        context: GenerationContext<'_>,
        on_token: &mut dyn FnMut(&DraftToken) -> Result<(), RuntimeError>,
    ) -> InferenceDraft {
        let draft = self.generate(context);
        for token in &draft.tokens {
            if let Err(error) = on_token(token) {
                return stream_observer_error_draft(error);
            }
        }
        draft
    }
}

pub(crate) fn stream_observer_error_draft(error: RuntimeError) -> InferenceDraft {
    InferenceDraft::new(
        format!("Runtime backend error: {}", error.message()),
        vec![ReasoningStep::new(
            "runtime_stream_observer_error",
            error.message(),
            0.0,
        )],
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingSource {
    Runtime,
    Fallback,
}

impl EmbeddingSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Runtime => "runtime",
            Self::Fallback => "fallback",
        }
    }
}

impl Default for EmbeddingSource {
    fn default() -> Self {
        Self::Fallback
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EmbeddingCallDiagnostics {
    pub source: EmbeddingSource,
    pub dimensions: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmbeddingDiagnostics {
    pub query: EmbeddingCallDiagnostics,
    pub memory_write: Option<EmbeddingCallDiagnostics>,
    pub gist_writes: Vec<EmbeddingCallDiagnostics>,
    pub runtime_calls: usize,
    pub fallback_calls: usize,
}

impl EmbeddingDiagnostics {
    pub(super) fn from_query(query: EmbeddingCallDiagnostics) -> Self {
        let mut diagnostics = Self {
            query,
            ..Self::default()
        };
        diagnostics.record_call(query);
        diagnostics
    }

    pub(super) fn record_memory_write(&mut self, call: EmbeddingCallDiagnostics) {
        self.memory_write = Some(call);
        self.record_call(call);
    }

    pub(super) fn record_gist_write(&mut self, call: EmbeddingCallDiagnostics) {
        self.gist_writes.push(call);
        self.record_call(call);
    }

    fn record_call(&mut self, call: EmbeddingCallDiagnostics) {
        match call.source {
            EmbeddingSource::Runtime => self.runtime_calls += 1,
            EmbeddingSource::Fallback => self.fallback_calls += 1,
        }
    }

    pub fn runtime_embedding_available(&self) -> bool {
        self.runtime_calls > 0
    }

    pub fn fallback_embedding_used(&self) -> bool {
        self.fallback_calls > 0
    }

    pub fn total_calls(&self) -> usize {
        1 + usize::from(self.memory_write.is_some()) + self.gist_writes.len()
    }

    pub fn gist_write_runtime_calls(&self) -> usize {
        self.gist_writes
            .iter()
            .filter(|call| call.source == EmbeddingSource::Runtime)
            .count()
    }

    pub fn gist_write_fallback_calls(&self) -> usize {
        self.gist_writes
            .iter()
            .filter(|call| call.source == EmbeddingSource::Fallback)
            .count()
    }
}

#[derive(Debug, Clone)]
pub(super) struct EmbeddingCall {
    pub(super) diagnostics: EmbeddingCallDiagnostics,
    pub(super) vector: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct InferenceOutcome {
    pub raw_answer: String,
    pub answer: String,
    pub report: ReflectionReport,
    pub auto_replay_report: Option<ExperienceReplayReport>,
    pub metrics: GenerationMetrics,
    pub runtime_token_metrics: RuntimeTokenMetrics,
    pub embedding_diagnostics: EmbeddingDiagnostics,
    pub runtime_diagnostics: RuntimeDiagnostics,
    pub runtime_adapter_observations: Vec<RuntimeAdapterObservation>,
    pub recursive_runtime_calls: usize,
    pub homeostatic_gate: HomeostaticGateReport,
    pub route_budget: RouteBudget,
    pub adaptive_route_plan: AdaptiveRoutingPlan,
    pub compute_budget_schedule: ComputeBudgetSchedule,
    pub task_hierarchy_plan: TaskAwareHierarchyPlan,
    pub hierarchy: HierarchyWeights,
    pub tier_plan: TieredCachePlan,
    pub tier_migrations: Vec<TierMigration>,
    pub infini_memory_plan: InfiniMemoryPlan,
    pub recursive_schedule: RecursiveSchedule,
    pub hardware_plan: HardwarePlan,
    pub transformer_plan: TransformerRefactorPlan,
    pub toolsmith_plan: ToolsmithPlan,
    pub agent_team_plan: AgentTeamPlan,
    pub stream_reports: Vec<TokenWindowReport>,
    pub used_memories: Vec<MemoryMatch>,
    pub memory_feedback: MemoryFeedbackReport,
    pub used_experiences: Vec<ExperienceMatch>,
    pub gist_records: Vec<GistRecord>,
    pub stored_memory_id: Option<u64>,
    pub stored_gist_memory_ids: Vec<u64>,
    pub exported_runtime_kv_blocks: usize,
    pub stored_runtime_kv_memory_ids: Vec<u64>,
    pub memory_admission: MemoryAdmissionPreview,
    pub drift_report: DriftReport,
    pub process_reward: ProcessRewardReport,
    pub genome_generation_before: u64,
    pub genome_strategy: ReasoningGenomeStrategy,
    pub strategy_genome: GenomeExpression,
    pub pre_reasoning_genome: GenomeExpression,
    pub pre_reasoning_genome_chain: DnaGeneChain,
    pub pre_reasoning_genome_splice: DnaSplicePreview,
    pub reasoning_frame: ReasoningFrame,
    pub reasoning_frame_valid: bool,
    pub task_gene_cascade: TaskGeneCascade,
    pub task_gene_review: TaskGeneAdmissionReview,
    pub task_skill_gene: TaskSkillGeneCandidate,
    pub reasoning_genome: GenomeExpression,
    pub reasoning_genome_chain: DnaGeneChain,
    pub reasoning_genome_splice: DnaSplicePreview,
    pub gene_purpose_reviews: Vec<GenePurposeRelabelProposal>,
    pub gene_scissors_journal: GeneScissorsTransactionJournal,
    pub dna_evolution_controller: DnaEvolutionControllerReport,
    pub dna_writer_gate: UnifiedWriterGateReport,
    pub dna_apply_plan: DnaEvolutionApplyPlan,
    pub dna_apply_receipt: GenomeEvolutionApplyReceipt,
    pub genome_evolution_preview: GenomeEvolutionPreview,
    pub memory_retention_policy: MemoryRetentionPolicy,
    pub memory_compaction_policy: MemoryCompactionPolicy,
    pub retention_report: RetentionReport,
    pub memory_compaction_report: MemoryCompactionReport,
    pub experience_id: u64,
    pub router_threshold_after: f32,
    pub live_evolution: LiveInferenceEvolution,
    pub evolution_ledger: EvolutionLedger,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MemoryFeedbackReport {
    pub reinforced: usize,
    pub penalized: usize,
    pub reinforcement_amount: f32,
    pub penalty_amount: f32,
    pub updates: Vec<MemoryUpdateReport>,
}

impl MemoryFeedbackReport {
    pub fn total_updates(&self) -> usize {
        self.reinforced + self.penalized
    }

    pub fn record_reinforcement(&mut self, amount: f32, update: MemoryUpdateReport) {
        self.reinforced += 1;
        self.reinforcement_amount += amount;
        self.updates.push(update);
    }

    pub fn record_penalty(&mut self, amount: f32, update: MemoryUpdateReport) {
        self.penalized += 1;
        self.penalty_amount += amount;
        self.updates.push(update);
    }

    pub fn applied_updates(&self) -> usize {
        self.updates
            .iter()
            .filter(|update| update.was_applied())
            .count()
    }

    pub fn removed_updates(&self) -> usize {
        self.updates.iter().filter(|update| update.removed).count()
    }

    pub fn strength_delta(&self) -> f32 {
        self.updates
            .iter()
            .map(|update| update.strength_delta.abs())
            .sum()
    }

    pub fn missing_updates(&self) -> usize {
        self.updates
            .iter()
            .filter(|update| !update.was_applied())
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeTokenMetrics {
    pub token_count: usize,
    pub entropy_count: usize,
    pub logprob_count: usize,
    pub average_entropy: Option<f32>,
    pub average_neg_logprob: Option<f32>,
    pub uncertainty_perplexity: Option<f32>,
}

impl RuntimeTokenMetrics {
    pub fn from_draft(draft: &InferenceDraft) -> Self {
        let mut entropy_total = 0.0;
        let mut entropy_count = 0;
        let mut neg_logprob_total = 0.0;
        let mut logprob_count = 0;
        let mut loss_total = 0.0;
        let mut loss_count = 0;

        for token in &draft.tokens {
            let entropy = token.entropy.and_then(bounded_entropy);
            let neg_logprob = token.logprob.and_then(bounded_neg_logprob);

            if let Some(entropy) = entropy {
                entropy_total += entropy;
                entropy_count += 1;
            }
            if let Some(neg_logprob) = neg_logprob {
                neg_logprob_total += neg_logprob;
                logprob_count += 1;
            }

            match (entropy, neg_logprob) {
                (Some(entropy), Some(neg_logprob)) => {
                    loss_total += 2.0 + entropy * 4.0 + neg_logprob;
                    loss_count += 1;
                }
                (Some(entropy), None) => {
                    loss_total += 2.0 + entropy * 4.0;
                    loss_count += 1;
                }
                (None, Some(neg_logprob)) => {
                    loss_total += 2.0 + neg_logprob;
                    loss_count += 1;
                }
                (None, None) => {}
            }
        }

        Self {
            token_count: draft.tokens.len(),
            entropy_count,
            logprob_count,
            average_entropy: average(entropy_total, entropy_count),
            average_neg_logprob: average(neg_logprob_total, logprob_count),
            uncertainty_perplexity: average(loss_total, loss_count),
        }
    }

    pub fn has_uncertainty_signal(self) -> bool {
        self.uncertainty_perplexity.is_some()
            || self.average_entropy.is_some()
            || self.average_neg_logprob.is_some()
    }
}

impl From<RuntimeTokenMetrics> for ExperienceRuntimeTokenMetrics {
    fn from(metrics: RuntimeTokenMetrics) -> Self {
        Self {
            token_count: metrics.token_count,
            entropy_count: metrics.entropy_count,
            logprob_count: metrics.logprob_count,
            average_entropy: metrics.average_entropy,
            average_neg_logprob: metrics.average_neg_logprob,
            uncertainty_perplexity: metrics.uncertainty_perplexity,
        }
    }
}

pub(super) fn bounded_entropy(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 4.0))
}

pub(super) fn bounded_neg_logprob(value: f32) -> Option<f32> {
    let value = -value;
    value.is_finite().then(|| value.clamp(0.0, 12.0))
}

fn average(total: f32, count: usize) -> Option<f32> {
    if count == 0 {
        None
    } else {
        Some(total / count as f32)
    }
}
