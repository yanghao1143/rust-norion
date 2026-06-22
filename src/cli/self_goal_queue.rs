use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use rust_norion::{
    EvolutionGoalEvidence, EvolutionGoalEvidenceKind, EvolutionGoalQueue,
    EvolutionGoalQueueDiskStore, EvolutionGoalQueueReport, EvolutionGoalQueueStoreApproval,
    EvolutionGoalQueueStorePolicy, EvolutionGoalQueueStoreReadReport,
    EvolutionGoalQueueStoreWriteReport, EvolutionGoalRunEvidence, EvolutionGoalStatus,
    SelfGoalAdmissionReport, SelfGoalProposalCandidate, SelfGoalProposalReport,
    SelfGoalQueueAppendApproval, SelfGoalQueueAppendExecutionReport, SelfGoalQueueAppendExecutor,
    SelfGoalQueueApplyReport, SelfGoalQueuePreviewReport, TenantResourceLane, TenantScope,
    TenantScopedKey, UnifiedWriterGate, UnifiedWriterGateCandidate, UnifiedWriterGateDecision,
    UnifiedWriterGateDomain, UnifiedWriterGatePolicy, UnifiedWriterGateReport,
    UnifiedWriterGateWriteScope, append_evolution_goal_queue_store_write_trace_jsonl,
    append_self_goal_queue_append_execution_trace_jsonl, append_self_goal_queue_apply_trace_jsonl,
    default_noiron_pursuit_goal_queue, default_self_goal_admission_report,
    default_self_goal_proposal_report, default_self_goal_queue_apply_report,
    default_self_goal_queue_preview_report, stable_redaction_digest,
};

use crate::cli::args::Args;

const SELF_GOAL_QUEUE_CONTINUATION_PLAN_SCHEMA_VERSION: &str =
    "self_goal_queue_continuation_plan_v1";
const SELF_GOAL_QUEUE_CONTINUATION_PLAN_TRACE_SCHEMA: &str =
    "rust-norion-self-goal-queue-continuation-plan-v1";
const SELF_GOAL_QUEUE_EVIDENCE_PLAN_SCHEMA_VERSION: &str = "self_goal_queue_evidence_plan_v1";
const SELF_GOAL_QUEUE_EVIDENCE_PLAN_TRACE_SCHEMA: &str =
    "rust-norion-self-goal-queue-evidence-plan-v1";
const SELF_GOAL_QUEUE_EVIDENCE_COLLECTION_SCHEMA_VERSION: &str =
    "self_goal_queue_evidence_collection_v1";
const SELF_GOAL_QUEUE_EVIDENCE_COLLECTION_TRACE_SCHEMA: &str =
    "rust-norion-self-goal-queue-evidence-collection-v1";

#[derive(Debug, Clone)]
pub(crate) struct SelfGoalQueueCliReport {
    pub(crate) current_queue_digest: String,
    pub(crate) current_goal_count: usize,
    pub(crate) current_queue_loaded_from_store: bool,
    pub(crate) store_read: Option<EvolutionGoalQueueStoreReadReport>,
    pub(crate) evidence: SelfGoalQueueCliEvidenceReport,
    pub(crate) queue_run: EvolutionGoalQueueReport,
    pub(crate) run_plan: SelfGoalQueueCliRunPlan,
    pub(crate) completion_preview: SelfGoalQueueCliCompletionPreview,
    pub(crate) completion_writer_gate: UnifiedWriterGateReport,
    pub(crate) continuation_plan: SelfGoalQueueCliContinuationPlan,
    pub(crate) evidence_plan: SelfGoalQueueCliEvidencePlan,
    pub(crate) evidence_collection: SelfGoalQueueCliEvidenceCollection,
    pub(crate) proposal: SelfGoalProposalReport,
    pub(crate) admission: SelfGoalAdmissionReport,
    pub(crate) queue_preview: SelfGoalQueuePreviewReport,
    pub(crate) writer_gate: UnifiedWriterGateReport,
    pub(crate) apply: SelfGoalQueueApplyReport,
    pub(crate) append_execution: SelfGoalQueueAppendExecutionReport,
    pub(crate) store_write: Option<EvolutionGoalQueueStoreWriteReport>,
}

impl SelfGoalQueueCliReport {
    pub(crate) fn summary_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!(
                "self_goal_queue_current goals={} digest={} loaded_from_store={}",
                self.current_goal_count,
                self.current_queue_digest,
                self.current_queue_loaded_from_store
            ),
            self.evidence.summary_line(),
            queue_run_summary_line(&self.queue_run),
            self.run_plan.summary_line(),
            self.completion_preview.summary_line(),
            format!(
                "self_goal_queue_completion_{}",
                self.completion_writer_gate.summary_line()
            ),
            self.continuation_plan.summary_line(),
            self.evidence_plan.summary_line(),
            self.evidence_collection.summary_line(),
        ];
        lines.extend(
            self.evidence_plan
                .steps
                .iter()
                .map(SelfGoalQueueCliEvidencePlanStep::summary_line),
        );
        lines.extend(
            self.evidence_collection
                .steps
                .iter()
                .map(SelfGoalQueueCliEvidenceCollectionStep::summary_line),
        );
        lines.extend(
            self.queue_run
                .decisions
                .iter()
                .map(|decision| format!("self_goal_queue_run_{}", decision.summary_line())),
        );
        lines.extend([
            self.proposal.summary_line(),
            self.admission.summary_line(),
            self.queue_preview.summary_line(),
            self.writer_gate.summary_line(),
            self.apply.summary_line(),
            self.append_execution.summary_line(),
        ]);
        if let Some(store_read) = &self.store_read {
            lines.push(store_read.summary_line());
        }
        if let Some(store_write) = &self.store_write {
            lines.push(store_write.summary_line());
        }
        lines
    }
}

pub(crate) fn run_self_goal_queue_report(args: &Args) -> io::Result<SelfGoalQueueCliReport> {
    let scope = self_goal_queue_scope(args);
    let key = scope.scoped_key(
        TenantResourceLane::EvolutionGoalQueue,
        &args.self_goal_queue_key,
    );
    let store_policy = if args.self_goal_queue_store_apply {
        EvolutionGoalQueueStorePolicy::explicit_durable_write()
    } else {
        EvolutionGoalQueueStorePolicy::default()
    };

    let (current_queue, store_read) = read_current_queue(args, &scope, key.as_str(), store_policy)?;
    let current_queue_digest = current_queue.redaction_digest();
    let current_goal_count = current_queue.goals.len();
    let current_queue_loaded_from_store = store_read
        .as_ref()
        .is_some_and(|read| read.found && read.decoded && read.queue.is_some());

    let proposal = default_self_goal_proposal_report(&current_queue);
    let (evidence_runs, evidence) = load_self_goal_queue_evidence(args, &current_queue, &proposal)?;
    let queue_run = current_queue.evaluate(&evidence_runs);
    let run_plan = SelfGoalQueueCliRunPlan::from_queue_run(&current_queue, &queue_run);
    let completion_preview =
        SelfGoalQueueCliCompletionPreview::from_queue_run(&current_queue, &queue_run);
    let completion_writer_gate = UnifiedWriterGate::new()
        .with_policy(UnifiedWriterGatePolicy {
            durable_writes_enabled: args.self_goal_queue_store_apply,
            ..UnifiedWriterGatePolicy::default()
        })
        .evaluate([completion_preview.writer_gate_candidate()]);
    let continuation_plan =
        SelfGoalQueueCliContinuationPlan::from_completion(&current_queue, &completion_preview);
    let evidence_plan = SelfGoalQueueCliEvidencePlan::from_continuation(&continuation_plan);
    let evidence_collection =
        SelfGoalQueueCliEvidenceCollection::from_plan(&evidence_plan, &evidence_runs);
    let admission = default_self_goal_admission_report(&proposal, &evidence_runs);
    let queue_preview =
        default_self_goal_queue_preview_report(&current_queue, &proposal, &admission);
    let writer_gate_policy = UnifiedWriterGatePolicy {
        durable_writes_enabled: args.self_goal_queue_store_apply,
        ..UnifiedWriterGatePolicy::default()
    };
    let writer_gate = UnifiedWriterGate::new()
        .with_policy(writer_gate_policy)
        .evaluate([UnifiedWriterGateCandidate::self_goal_queue_preview(
            &queue_preview,
        )]);
    let apply = default_self_goal_queue_apply_report(&current_queue, &queue_preview, &writer_gate);
    let append_approval = SelfGoalQueueAppendApproval::from_apply_report(
        &args.self_goal_queue_operator,
        &args.self_goal_queue_ticket,
        &apply,
    );
    let append_execution = SelfGoalQueueAppendExecutor::default().evaluate(
        &current_queue,
        &proposal,
        &queue_preview,
        &apply,
        Some(&append_approval),
    );

    let store_write = if args.self_goal_queue_store_apply && append_execution.applied {
        write_append_execution_result(args, &scope, &key, store_policy, &append_execution)?
    } else if args.self_goal_queue_store_apply && completion_preview.ready {
        write_completion_preview_result(
            args,
            &scope,
            &key,
            store_policy,
            &completion_preview,
            &completion_writer_gate,
        )?
    } else if args.self_goal_queue_store_apply {
        write_append_execution_result(args, &scope, &key, store_policy, &append_execution)?
    } else {
        None
    };

    if let Some(trace_path) = self_goal_trace_path(args) {
        append_self_goal_queue_apply_trace_jsonl(trace_path, &apply)?;
        append_self_goal_queue_append_execution_trace_jsonl(trace_path, &append_execution)?;
        append_self_goal_queue_continuation_trace_jsonl(trace_path, &continuation_plan)?;
        append_self_goal_queue_evidence_plan_trace_jsonl(trace_path, &evidence_plan)?;
        append_self_goal_queue_evidence_collection_trace_jsonl(trace_path, &evidence_collection)?;
        if let Some(store_write) = &store_write {
            append_evolution_goal_queue_store_write_trace_jsonl(trace_path, store_write)?;
        }
    }

    Ok(SelfGoalQueueCliReport {
        current_queue_digest,
        current_goal_count,
        current_queue_loaded_from_store,
        store_read,
        evidence,
        queue_run,
        run_plan,
        completion_preview,
        completion_writer_gate,
        continuation_plan,
        evidence_plan,
        evidence_collection,
        proposal,
        admission,
        queue_preview,
        writer_gate,
        apply,
        append_execution,
        store_write,
    })
}

pub(crate) fn print_self_goal_queue_report(report: &SelfGoalQueueCliReport) {
    for line in report.summary_lines() {
        println!("{line}");
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelfGoalQueueCliRunPlan {
    pub(crate) active: bool,
    pub(crate) active_goal_id: Option<String>,
    pub(crate) required_evidence: Vec<String>,
    pub(crate) evidence_template_digest: String,
    pub(crate) max_attempts: u32,
    pub(crate) max_steps: u32,
    pub(crate) max_tokens: u64,
    pub(crate) max_runtime_ms: u64,
}

impl SelfGoalQueueCliRunPlan {
    fn from_queue_run(queue: &EvolutionGoalQueue, queue_run: &EvolutionGoalQueueReport) -> Self {
        let active_goal = queue_run
            .active_goal_id
            .as_deref()
            .and_then(|goal_id| queue.goals.iter().find(|goal| goal.stable_id == goal_id));
        let required_evidence = active_goal
            .map(|goal| {
                goal.success_gate
                    .required_evidence
                    .iter()
                    .map(|kind| kind.as_str().to_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let required_joined = required_evidence.join("|");
        let evidence_template_digest = stable_redaction_digest([
            "self-goal-queue-run-plan-v1",
            active_goal
                .map(|goal| goal.stable_id.as_str())
                .unwrap_or("no-active-goal"),
            required_joined.as_str(),
        ]);
        let budget_cap = active_goal.map(|goal| goal.budget_cap).unwrap_or_default();

        Self {
            active: active_goal.is_some(),
            active_goal_id: active_goal.map(|goal| goal.stable_id.clone()),
            required_evidence,
            evidence_template_digest,
            max_attempts: budget_cap.max_attempts,
            max_steps: budget_cap.max_steps,
            max_tokens: budget_cap.max_tokens,
            max_runtime_ms: budget_cap.max_runtime_ms,
        }
    }

    fn from_queue_without_evidence(queue: &EvolutionGoalQueue) -> Self {
        let queue_run = queue.evaluate(&[]);
        Self::from_queue_run(queue, &queue_run)
    }

    pub(crate) fn summary_line(&self) -> String {
        let required = if self.required_evidence.is_empty() {
            "none".to_owned()
        } else {
            self.required_evidence.join("|")
        };
        format!(
            "self_goal_queue_run_plan active={} goal={} required={} template={} budget_attempts={} budget_steps={} budget_tokens={} budget_runtime_ms={}",
            self.active,
            self.active_goal_id.as_deref().unwrap_or("none"),
            required,
            self.evidence_template_digest,
            self.max_attempts,
            self.max_steps,
            self.max_tokens,
            self.max_runtime_ms
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelfGoalQueueCliContinuationPlan {
    pub(crate) source: &'static str,
    pub(crate) ready: bool,
    pub(crate) queue_digest: String,
    pub(crate) goal_count: usize,
    pub(crate) plan: SelfGoalQueueCliRunPlan,
    pub(crate) continuation_digest: String,
    pub(crate) reason_codes: Vec<String>,
}

impl SelfGoalQueueCliContinuationPlan {
    fn from_completion(
        current_queue: &EvolutionGoalQueue,
        completion_preview: &SelfGoalQueueCliCompletionPreview,
    ) -> Self {
        let (source, queue, mut reason_codes) = match completion_preview.resulting_queue.as_ref() {
            Some(resulting_queue) if completion_preview.ready => (
                "completion_resulting_queue",
                resulting_queue,
                vec!["completion_pruned_prefix".to_owned()],
            ),
            _ => (
                "current_queue",
                current_queue,
                vec!["completion_not_ready".to_owned()],
            ),
        };
        let plan = SelfGoalQueueCliRunPlan::from_queue_without_evidence(queue);
        let queue_digest = queue.redaction_digest();
        if plan.active {
            reason_codes.push("next_goal_ready_for_evidence".to_owned());
        } else {
            reason_codes.push("no_next_goal_ready".to_owned());
        }
        let goal_count_text = queue.goals.len().to_string();
        let reason_text = reason_codes.join("|");
        let continuation_digest = stable_redaction_digest([
            "self-goal-queue-continuation-plan-v1",
            source,
            queue_digest.as_str(),
            goal_count_text.as_str(),
            plan.evidence_template_digest.as_str(),
            reason_text.as_str(),
        ]);

        Self {
            source,
            ready: plan.active,
            queue_digest,
            goal_count: queue.goals.len(),
            plan,
            continuation_digest,
            reason_codes,
        }
    }

    pub(crate) fn summary_line(&self) -> String {
        let required = if self.plan.required_evidence.is_empty() {
            "none".to_owned()
        } else {
            self.plan.required_evidence.join("|")
        };
        format!(
            "self_goal_queue_continuation source={} ready={} goals={} queue={} active={} goal={} required={} template={} digest={} reasons={}",
            self.source,
            self.ready,
            self.goal_count,
            self.queue_digest,
            self.plan.active,
            self.plan.active_goal_id.as_deref().unwrap_or("none"),
            required,
            self.plan.evidence_template_digest,
            self.continuation_digest,
            self.reason_codes.join("|")
        )
    }

    fn json_line(&self) -> String {
        format!(
            "{{\"schema\":\"{}\",\"plan_schema\":\"{}\",\"source\":\"{}\",\"ready\":{},\"queue_digest\":\"{}\",\"goals\":{},\"active\":{},\"active_goal_id\":\"{}\",\"required_evidence_count\":{},\"required_evidence\":{},\"evidence_template_digest\":\"{}\",\"continuation_digest\":\"{}\",\"budget_attempts\":{},\"budget_steps\":{},\"budget_tokens\":{},\"budget_runtime_ms\":{},\"reason_code_count\":{},\"reason_codes\":{},\"read_only\":true,\"write_allowed\":false,\"applied\":false,\"summary\":\"{}\"}}",
            json_escape(SELF_GOAL_QUEUE_CONTINUATION_PLAN_TRACE_SCHEMA),
            json_escape(SELF_GOAL_QUEUE_CONTINUATION_PLAN_SCHEMA_VERSION),
            json_escape(self.source),
            self.ready,
            json_escape(&self.queue_digest),
            self.goal_count,
            self.plan.active,
            json_escape(self.plan.active_goal_id.as_deref().unwrap_or("none")),
            self.plan.required_evidence.len(),
            json_string_array(&self.plan.required_evidence),
            json_escape(&self.plan.evidence_template_digest),
            json_escape(&self.continuation_digest),
            self.plan.max_attempts,
            self.plan.max_steps,
            self.plan.max_tokens,
            self.plan.max_runtime_ms,
            self.reason_codes.len(),
            json_string_array(&self.reason_codes),
            json_escape(&self.summary_line())
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelfGoalQueueCliEvidencePlan {
    pub(crate) source: &'static str,
    pub(crate) ready: bool,
    pub(crate) active_goal_id: Option<String>,
    pub(crate) required_evidence: Vec<String>,
    pub(crate) evidence_template_digest: String,
    pub(crate) evidence_plan_digest: String,
    pub(crate) steps: Vec<SelfGoalQueueCliEvidencePlanStep>,
}

impl SelfGoalQueueCliEvidencePlan {
    fn from_continuation(continuation: &SelfGoalQueueCliContinuationPlan) -> Self {
        let active_goal_id = continuation.plan.active_goal_id.clone();
        let steps = if continuation.ready {
            active_goal_id
                .as_deref()
                .map(|goal_id| {
                    continuation
                        .plan
                        .required_evidence
                        .iter()
                        .enumerate()
                        .map(|(index, kind)| {
                            SelfGoalQueueCliEvidencePlanStep::new(
                                index + 1,
                                goal_id,
                                kind,
                                continuation.source,
                                &continuation.plan.evidence_template_digest,
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let step_digest_text = steps
            .iter()
            .map(|step| step.packet_template_digest.as_str())
            .collect::<Vec<_>>()
            .join("|");
        let required_text = continuation.plan.required_evidence.join("|");
        let evidence_plan_digest = stable_redaction_digest([
            "self-goal-queue-evidence-plan-v1",
            continuation.source,
            active_goal_id.as_deref().unwrap_or("none"),
            continuation.plan.evidence_template_digest.as_str(),
            required_text.as_str(),
            step_digest_text.as_str(),
        ]);

        Self {
            source: continuation.source,
            ready: continuation.ready && !steps.is_empty(),
            active_goal_id,
            required_evidence: continuation.plan.required_evidence.clone(),
            evidence_template_digest: continuation.plan.evidence_template_digest.clone(),
            evidence_plan_digest,
            steps,
        }
    }

    pub(crate) fn auto_collectible_steps(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.auto_collectible)
            .count()
    }

    pub(crate) fn manual_steps(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.manual_required)
            .count()
    }

    pub(crate) fn summary_line(&self) -> String {
        let required = if self.required_evidence.is_empty() {
            "none".to_owned()
        } else {
            self.required_evidence.join("|")
        };
        format!(
            "self_goal_queue_evidence_plan source={} ready={} goal={} required={} steps={} auto_collectible={} manual={} template={} digest={}",
            self.source,
            self.ready,
            self.active_goal_id.as_deref().unwrap_or("none"),
            required,
            self.steps.len(),
            self.auto_collectible_steps(),
            self.manual_steps(),
            self.evidence_template_digest,
            self.evidence_plan_digest
        )
    }

    fn json_line(&self) -> String {
        let packet_digests = self
            .steps
            .iter()
            .map(|step| step.packet_template_digest.clone())
            .collect::<Vec<_>>();
        let command_digests = self
            .steps
            .iter()
            .map(|step| step.command_digest.clone())
            .collect::<Vec<_>>();
        let step_kinds = self
            .steps
            .iter()
            .map(|step| step.evidence_kind.clone())
            .collect::<Vec<_>>();
        format!(
            "{{\"schema\":\"{}\",\"plan_schema\":\"{}\",\"source\":\"{}\",\"ready\":{},\"active_goal_id\":\"{}\",\"required_evidence_count\":{},\"required_evidence\":{},\"planned_step_count\":{},\"step_kinds\":{},\"auto_collectible_steps\":{},\"manual_steps\":{},\"evidence_template_digest\":\"{}\",\"evidence_plan_digest\":\"{}\",\"packet_template_digests\":{},\"command_digests\":{},\"read_only\":true,\"write_allowed\":false,\"applied\":false,\"summary\":\"{}\"}}",
            json_escape(SELF_GOAL_QUEUE_EVIDENCE_PLAN_TRACE_SCHEMA),
            json_escape(SELF_GOAL_QUEUE_EVIDENCE_PLAN_SCHEMA_VERSION),
            json_escape(self.source),
            self.ready,
            json_escape(self.active_goal_id.as_deref().unwrap_or("none")),
            self.required_evidence.len(),
            json_string_array(&self.required_evidence),
            self.steps.len(),
            json_string_array(&step_kinds),
            self.auto_collectible_steps(),
            self.manual_steps(),
            json_escape(&self.evidence_template_digest),
            json_escape(&self.evidence_plan_digest),
            json_string_array(&packet_digests),
            json_string_array(&command_digests),
            json_escape(&self.summary_line())
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelfGoalQueueCliEvidencePlanStep {
    pub(crate) sequence: usize,
    pub(crate) evidence_kind: String,
    pub(crate) runner: &'static str,
    pub(crate) auto_collectible: bool,
    pub(crate) manual_required: bool,
    pub(crate) packet_template_digest: String,
    pub(crate) command_digest: String,
}

impl SelfGoalQueueCliEvidencePlanStep {
    fn new(
        sequence: usize,
        goal_id: &str,
        evidence_kind: &str,
        source: &str,
        evidence_template_digest: &str,
    ) -> Self {
        let runner = evidence_runner(evidence_kind);
        let manual_required = evidence_kind == "operator_approval";
        let packet_template_digest = stable_redaction_digest([
            "self-goal-queue-evidence-packet-template-v1",
            goal_id,
            evidence_kind,
            source,
            evidence_template_digest,
            &sequence.to_string(),
        ]);
        let command_digest = stable_redaction_digest([
            "self-goal-queue-evidence-command-template-v1",
            runner,
            evidence_kind,
            source,
            &sequence.to_string(),
        ]);

        Self {
            sequence,
            evidence_kind: evidence_kind.to_owned(),
            runner,
            auto_collectible: !manual_required,
            manual_required,
            packet_template_digest,
            command_digest,
        }
    }

    fn summary_line(&self) -> String {
        format!(
            "self_goal_queue_evidence_step index={} kind={} runner={} auto_collectible={} manual={} packet={} command={}",
            self.sequence,
            self.evidence_kind,
            self.runner,
            self.auto_collectible,
            self.manual_required,
            self.packet_template_digest,
            self.command_digest
        )
    }
}

fn evidence_runner(evidence_kind: &str) -> &'static str {
    match evidence_kind {
        "cargo_check" => "cargo_check",
        "focused_tests" => "focused_tests",
        "benchmark_gate" => "benchmark_gate",
        "trace_schema_gate" => "trace_schema_gate",
        "experiment_ledger" => "experiment_ledger_gate",
        "operator_approval" => "operator_approval",
        _ => "unsupported",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelfGoalQueueCliEvidenceCollection {
    pub(crate) source: &'static str,
    pub(crate) ready: bool,
    pub(crate) collection_complete: bool,
    pub(crate) active_goal_id: Option<String>,
    pub(crate) evidence_collection_digest: String,
    pub(crate) steps: Vec<SelfGoalQueueCliEvidenceCollectionStep>,
}

impl SelfGoalQueueCliEvidenceCollection {
    fn from_plan(plan: &SelfGoalQueueCliEvidencePlan, runs: &[EvolutionGoalRunEvidence]) -> Self {
        let active_goal_id = plan.active_goal_id.clone().filter(|_| plan.ready);
        let run = active_goal_id
            .as_deref()
            .and_then(|goal_id| runs.iter().find(|run| run.goal_id == goal_id));
        let steps = if plan.ready {
            plan.steps
                .iter()
                .map(|step| SelfGoalQueueCliEvidenceCollectionStep::from_plan_step(step, run))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let packet_digest_text = steps
            .iter()
            .map(|step| step.collection_packet_digest.as_str())
            .collect::<Vec<_>>()
            .join("|");
        let status_text = steps
            .iter()
            .map(|step| step.status)
            .collect::<Vec<_>>()
            .join("|");
        let evidence_collection_digest = stable_redaction_digest([
            "self-goal-queue-evidence-collection-v1",
            plan.source,
            active_goal_id.as_deref().unwrap_or("none"),
            plan.evidence_plan_digest.as_str(),
            status_text.as_str(),
            packet_digest_text.as_str(),
        ]);
        let collection_complete = plan.ready
            && !steps.is_empty()
            && steps
                .iter()
                .all(|step| step.status == "passed" && step.evidence_digest.is_some());

        Self {
            source: plan.source,
            ready: plan.ready,
            collection_complete,
            active_goal_id,
            evidence_collection_digest,
            steps,
        }
    }

    pub(crate) fn passed_steps(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.status == "passed")
            .count()
    }

    pub(crate) fn failed_steps(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.status == "failed")
            .count()
    }

    pub(crate) fn missing_steps(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.status == "missing")
            .count()
    }

    pub(crate) fn manual_missing_steps(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.status == "manual_missing")
            .count()
    }

    pub(crate) fn auto_collectible_steps(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.auto_collectible)
            .count()
    }

    pub(crate) fn manual_required_steps(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.manual_required)
            .count()
    }

    pub(crate) fn collected_evidence_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| step.evidence_digest.is_some())
            .count()
    }

    pub(crate) fn summary_line(&self) -> String {
        format!(
            "self_goal_queue_evidence_collection source={} ready={} complete={} goal={} planned={} collected={} passed={} failed={} missing={} manual_missing={} auto_collectible={} manual={} digest={}",
            self.source,
            self.ready,
            self.collection_complete,
            self.active_goal_id.as_deref().unwrap_or("none"),
            self.steps.len(),
            self.collected_evidence_count(),
            self.passed_steps(),
            self.failed_steps(),
            self.missing_steps(),
            self.manual_missing_steps(),
            self.auto_collectible_steps(),
            self.manual_required_steps(),
            self.evidence_collection_digest
        )
    }

    fn json_line(&self) -> String {
        let step_kinds = self
            .steps
            .iter()
            .map(|step| step.evidence_kind.clone())
            .collect::<Vec<_>>();
        let step_statuses = self
            .steps
            .iter()
            .map(|step| step.status.to_owned())
            .collect::<Vec<_>>();
        let collection_packet_digests = self
            .steps
            .iter()
            .map(|step| step.collection_packet_digest.clone())
            .collect::<Vec<_>>();
        let collected_evidence_digests = self
            .steps
            .iter()
            .filter_map(|step| step.evidence_digest.clone())
            .collect::<Vec<_>>();
        format!(
            "{{\"schema\":\"{}\",\"collection_schema\":\"{}\",\"source\":\"{}\",\"ready\":{},\"collection_complete\":{},\"active_goal_id\":\"{}\",\"planned_step_count\":{},\"step_kinds\":{},\"step_statuses\":{},\"passed_steps\":{},\"failed_steps\":{},\"missing_steps\":{},\"manual_missing_steps\":{},\"auto_collectible_steps\":{},\"manual_required_steps\":{},\"collected_evidence_count\":{},\"collected_evidence_digests\":{},\"collection_packet_digests\":{},\"evidence_collection_digest\":\"{}\",\"read_only\":true,\"write_allowed\":false,\"applied\":false,\"summary\":\"{}\"}}",
            json_escape(SELF_GOAL_QUEUE_EVIDENCE_COLLECTION_TRACE_SCHEMA),
            json_escape(SELF_GOAL_QUEUE_EVIDENCE_COLLECTION_SCHEMA_VERSION),
            json_escape(self.source),
            self.ready,
            self.collection_complete,
            json_escape(self.active_goal_id.as_deref().unwrap_or("none")),
            self.steps.len(),
            json_string_array(&step_kinds),
            json_string_array(&step_statuses),
            self.passed_steps(),
            self.failed_steps(),
            self.missing_steps(),
            self.manual_missing_steps(),
            self.auto_collectible_steps(),
            self.manual_required_steps(),
            self.collected_evidence_count(),
            json_string_array(&collected_evidence_digests),
            json_string_array(&collection_packet_digests),
            json_escape(&self.evidence_collection_digest),
            json_escape(&self.summary_line())
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelfGoalQueueCliEvidenceCollectionStep {
    pub(crate) sequence: usize,
    pub(crate) evidence_kind: String,
    pub(crate) runner: &'static str,
    pub(crate) status: &'static str,
    pub(crate) auto_collectible: bool,
    pub(crate) manual_required: bool,
    pub(crate) evidence_digest: Option<String>,
    pub(crate) collection_packet_digest: String,
}

impl SelfGoalQueueCliEvidenceCollectionStep {
    fn from_plan_step(
        step: &SelfGoalQueueCliEvidencePlanStep,
        run: Option<&EvolutionGoalRunEvidence>,
    ) -> Self {
        let evidence = matching_evidence(run, &step.evidence_kind);
        let approval_granted = run.is_some_and(|run| run.approval_granted);
        let status = if step.evidence_kind == "operator_approval" {
            match evidence {
                Some(evidence) if evidence.passed && approval_granted => "passed",
                Some(evidence) if !evidence.passed => "failed",
                _ => "manual_missing",
            }
        } else {
            match evidence {
                Some(evidence) if evidence.passed => "passed",
                Some(_) => "failed",
                None => "missing",
            }
        };
        let evidence_digest = match status {
            "passed" | "failed" => evidence.map(|evidence| evidence.evidence_digest.clone()),
            _ => None,
        };
        let collection_packet_digest = stable_redaction_digest([
            "self-goal-queue-evidence-collection-packet-v1",
            &step.sequence.to_string(),
            &step.evidence_kind,
            status,
            evidence_digest.as_deref().unwrap_or("none"),
            &step.packet_template_digest,
        ]);

        Self {
            sequence: step.sequence,
            evidence_kind: step.evidence_kind.clone(),
            runner: step.runner,
            status,
            auto_collectible: step.auto_collectible,
            manual_required: step.manual_required,
            evidence_digest,
            collection_packet_digest,
        }
    }

    fn summary_line(&self) -> String {
        format!(
            "self_goal_queue_evidence_collection_step index={} kind={} runner={} status={} auto_collectible={} manual={} evidence={} packet={}",
            self.sequence,
            self.evidence_kind,
            self.runner,
            self.status,
            self.auto_collectible,
            self.manual_required,
            self.evidence_digest.as_deref().unwrap_or("none"),
            self.collection_packet_digest
        )
    }
}

fn matching_evidence<'a>(
    run: Option<&'a EvolutionGoalRunEvidence>,
    evidence_kind: &str,
) -> Option<&'a EvolutionGoalEvidence> {
    let run = run?;
    run.evidence
        .iter()
        .filter(|evidence| evidence.kind.as_str() == evidence_kind)
        .find(|evidence| evidence.passed)
        .or_else(|| {
            run.evidence
                .iter()
                .find(|evidence| evidence.kind.as_str() == evidence_kind)
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelfGoalQueueCliCompletionPreview {
    pub(crate) ready: bool,
    pub(crate) completed_count: usize,
    pub(crate) retained_count: usize,
    pub(crate) current_queue_digest: String,
    pub(crate) resulting_queue_digest: String,
    pub(crate) rollback_anchor_digest: String,
    pub(crate) completion_digest: String,
    pub(crate) reason_codes: Vec<String>,
    pub(crate) resulting_queue: Option<EvolutionGoalQueue>,
}

impl SelfGoalQueueCliCompletionPreview {
    fn from_queue_run(queue: &EvolutionGoalQueue, queue_run: &EvolutionGoalQueueReport) -> Self {
        let current_queue_digest = queue.redaction_digest();
        let completed_count = queue_run
            .decisions
            .iter()
            .take_while(|decision| decision.status == EvolutionGoalStatus::Passed)
            .count();
        let retained_goals = queue
            .goals
            .iter()
            .skip(completed_count)
            .cloned()
            .collect::<Vec<_>>();
        let resulting_queue = EvolutionGoalQueue::new(retained_goals);
        let resulting_queue_digest = resulting_queue.redaction_digest();
        let mut reason_codes = Vec::new();

        if queue.goals.is_empty() {
            reason_codes.push("completion_queue_empty".to_owned());
        }
        if completed_count == 0 {
            reason_codes.push("completion_no_passed_prefix".to_owned());
        }
        if queue_run
            .decisions
            .iter()
            .skip(completed_count)
            .any(|decision| decision.status == EvolutionGoalStatus::Passed)
        {
            reason_codes.push("completion_non_prefix_passed_goal_retained".to_owned());
        }

        let ready = completed_count > 0;
        if ready {
            reason_codes.push("completion_prune_preview_ready".to_owned());
        }
        let completed_count_text = completed_count.to_string();
        let retained_count_text = resulting_queue.goals.len().to_string();
        let reason_text = reason_codes.join("|");
        let completion_digest = stable_redaction_digest([
            "self-goal-queue-completion-preview-v1",
            current_queue_digest.as_str(),
            resulting_queue_digest.as_str(),
            completed_count_text.as_str(),
            retained_count_text.as_str(),
            reason_text.as_str(),
        ]);

        Self {
            ready,
            completed_count,
            retained_count: resulting_queue.goals.len(),
            current_queue_digest: current_queue_digest.clone(),
            resulting_queue_digest,
            rollback_anchor_digest: current_queue_digest,
            completion_digest,
            reason_codes,
            resulting_queue: ready.then_some(resulting_queue),
        }
    }

    fn writer_gate_candidate(&self) -> UnifiedWriterGateCandidate {
        let has_result = self.resulting_queue.is_some();
        let review_packet_ids = self
            .ready
            .then(|| format!("self-goal-queue-completion:{}", self.completion_digest))
            .into_iter()
            .collect::<Vec<_>>();
        let evidence_ids = if self.ready {
            vec![
                self.current_queue_digest.clone(),
                self.resulting_queue_digest.clone(),
                self.completion_digest.clone(),
            ]
        } else {
            Vec::new()
        };
        let rollback_anchor_ids = self
            .ready
            .then(|| self.rollback_anchor_digest.clone())
            .into_iter()
            .collect::<Vec<_>>();
        let content_digests = if self.ready {
            vec![
                self.current_queue_digest.clone(),
                self.resulting_queue_digest.clone(),
            ]
        } else {
            Vec::new()
        };
        let source_report_schemas = self
            .ready
            .then(|| "self-goal-queue-completion-preview-v1".to_owned())
            .into_iter()
            .collect::<Vec<_>>();

        UnifiedWriterGateCandidate::new(
            UnifiedWriterGateDomain::EvolutionGoalQueue,
            format!("self-goal-queue-completion:{}", self.completion_digest),
            [UnifiedWriterGateWriteScope::EvolutionGoalQueue],
        )
        .with_refs(
            review_packet_ids,
            evidence_ids,
            rollback_anchor_ids,
            content_digests,
            source_report_schemas,
        )
        .with_evidence(self.ready, self.ready, self.ready, self.ready, true)
        .with_operator_approval(self.ready, self.ready)
        .with_source_flags(true, true, false, false, false)
        .with_raw_payload_redacted(has_result || !self.ready)
    }

    pub(crate) fn summary_line(&self) -> String {
        format!(
            "self_goal_queue_completion ready={} completed={} retained={} current={} resulting={} rollback={} digest={} reasons={}",
            self.ready,
            self.completed_count,
            self.retained_count,
            self.current_queue_digest,
            self.resulting_queue_digest,
            self.rollback_anchor_digest,
            self.completion_digest,
            self.reason_codes.join("|")
        )
    }
}

fn queue_run_summary_line(report: &EvolutionGoalQueueReport) -> String {
    format!(
        "self_goal_queue_run decisions={} active={} passed={} failed={} rolled_back={} budget_exhausted={} approval_holds={} preview_only={}",
        report.decisions.len(),
        report.active_goal_id.as_deref().unwrap_or("none"),
        report.passed_count,
        report.failed_count,
        report.rolled_back_count,
        report.budget_exhausted_count,
        report.approval_hold_count,
        report.is_preview_only()
    )
}

#[derive(Debug, Clone)]
pub(crate) struct SelfGoalQueueCliEvidenceReport {
    pub(crate) packet_count: usize,
    pub(crate) valid_packet_count: usize,
    pub(crate) invalid_packet_count: usize,
    pub(crate) run_count: usize,
    pub(crate) evidence_count: usize,
    pub(crate) approval_count: usize,
    pub(crate) evidence_digest: String,
}

impl SelfGoalQueueCliEvidenceReport {
    pub(crate) fn summary_line(&self) -> String {
        format!(
            "self_goal_queue_evidence packets={} valid={} invalid={} runs={} evidence={} approvals={} digest={}",
            self.packet_count,
            self.valid_packet_count,
            self.invalid_packet_count,
            self.run_count,
            self.evidence_count,
            self.approval_count,
            self.evidence_digest
        )
    }
}

#[derive(Debug, Default)]
struct RunEvidenceBuilder {
    evidence: Vec<EvolutionGoalEvidence>,
    approval_granted: bool,
}

#[derive(Debug)]
struct ParsedEvidencePacket {
    goal_id: String,
    evidence: EvolutionGoalEvidence,
    approval_granted: bool,
}

fn load_self_goal_queue_evidence(
    args: &Args,
    current_queue: &EvolutionGoalQueue,
    proposal: &SelfGoalProposalReport,
) -> io::Result<(
    Vec<EvolutionGoalRunEvidence>,
    SelfGoalQueueCliEvidenceReport,
)> {
    let packets = read_self_goal_queue_evidence_packets(args)?;
    let packet_digest = stable_redaction_digest(
        packets
            .iter()
            .map(String::as_str)
            .chain(["self-goal-cli-evidence-v1"]),
    );
    let mut builders = BTreeMap::<String, RunEvidenceBuilder>::new();
    let mut valid_packet_count = 0;
    let mut invalid_packet_count = 0;
    let mut evidence_count = 0;
    let mut approval_count = 0;

    for packet in &packets {
        match parse_self_goal_queue_evidence_packet(packet, current_queue, proposal) {
            Some(parsed) => {
                valid_packet_count += 1;
                evidence_count += 1;
                if parsed.approval_granted {
                    approval_count += 1;
                }
                let builder = builders.entry(parsed.goal_id).or_default();
                builder.evidence.push(parsed.evidence);
                builder.approval_granted |= parsed.approval_granted;
            }
            None => invalid_packet_count += 1,
        }
    }

    let runs = if invalid_packet_count == 0 {
        builders
            .into_iter()
            .map(|(goal_id, builder)| {
                let mut run =
                    EvolutionGoalRunEvidence::new(goal_id).with_evidence(builder.evidence);
                if builder.approval_granted {
                    run = run.with_approval();
                }
                run
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let report = SelfGoalQueueCliEvidenceReport {
        packet_count: packets.len(),
        valid_packet_count,
        invalid_packet_count,
        run_count: runs.len(),
        evidence_count,
        approval_count,
        evidence_digest: packet_digest,
    };
    Ok((runs, report))
}

fn read_self_goal_queue_evidence_packets(args: &Args) -> io::Result<Vec<String>> {
    let mut packets = args.self_goal_queue_evidence_packets.clone();
    if let Some(path) = args.self_goal_queue_evidence_path.as_ref() {
        let text = fs::read_to_string(path)?;
        packets.extend(text.lines().filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                None
            } else {
                Some(line.to_owned())
            }
        }));
    }
    Ok(packets)
}

fn parse_self_goal_queue_evidence_packet(
    packet: &str,
    current_queue: &EvolutionGoalQueue,
    proposal: &SelfGoalProposalReport,
) -> Option<ParsedEvidencePacket> {
    let fields = parse_packet_fields(packet)?;
    let goal_id = evidence_packet_goal_id(&fields, current_queue, proposal)?;
    let kind = fields
        .get("kind")
        .and_then(|value| parse_evidence_kind(value))?;
    let passed = fields
        .get("passed")
        .and_then(|value| parse_bool(value))
        .unwrap_or(true);
    let item_count = fields
        .get("items")
        .or_else(|| fields.get("item_count"))
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1);
    let failure_count = fields
        .get("failures")
        .or_else(|| fields.get("failure_count"))
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(u64::from(!passed));
    let label = fields
        .get("label")
        .cloned()
        .unwrap_or_else(|| format!("self-goal-cli:{}", kind.as_str()));
    let approval_granted = kind == EvolutionGoalEvidenceKind::OperatorApproval
        && passed
        && fields
            .get("approval")
            .or_else(|| fields.get("approved"))
            .and_then(|value| parse_bool(value))
            .unwrap_or(false);

    Some(ParsedEvidencePacket {
        goal_id,
        evidence: EvolutionGoalEvidence::new(kind, label, passed, item_count.max(1), failure_count),
        approval_granted,
    })
}

fn parse_packet_fields(packet: &str) -> Option<BTreeMap<String, String>> {
    let mut fields = BTreeMap::new();
    for segment in packet.split(';') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let (key, value) = segment.split_once('=')?;
        let key = key.trim().to_ascii_lowercase().replace('-', "_");
        let value = value.trim();
        if key.is_empty() || value.is_empty() || fields.insert(key, value.to_owned()).is_some() {
            return None;
        }
    }
    (!fields.is_empty()).then_some(fields)
}

fn evidence_packet_goal_id(
    fields: &BTreeMap<String, String>,
    current_queue: &EvolutionGoalQueue,
    proposal: &SelfGoalProposalReport,
) -> Option<String> {
    if let Some(goal_id) = fields.get("goal").or_else(|| fields.get("goal_id")) {
        return Some(goal_id.to_owned());
    }
    if let Some(queue_goal_id) = fields
        .get("queue_goal")
        .or_else(|| fields.get("queue_goal_id"))
    {
        return current_queue
            .goals
            .iter()
            .find(|goal| goal.stable_id == *queue_goal_id)
            .map(|goal| goal.stable_id.clone());
    }
    if let Some(queue_index) = fields
        .get("queue_index")
        .or_else(|| fields.get("goal_index"))
        .and_then(|value| value.parse::<usize>().ok())
    {
        return current_queue
            .goals
            .get(queue_index)
            .map(|goal| goal.stable_id.clone());
    }
    if let Some(candidate_id) = fields
        .get("candidate")
        .or_else(|| fields.get("candidate_id"))
    {
        return proposal
            .candidates
            .iter()
            .find(|candidate| {
                candidate.stable_id == *candidate_id
                    || candidate.proposed_goal.stable_id == *candidate_id
            })
            .map(candidate_goal_id);
    }
    let candidate_index = fields
        .get("candidate_index")
        .or_else(|| fields.get("index"))
        .and_then(|value| value.parse::<usize>().ok())?;
    proposal
        .candidates
        .get(candidate_index)
        .map(candidate_goal_id)
}

fn candidate_goal_id(candidate: &SelfGoalProposalCandidate) -> String {
    candidate.proposed_goal.stable_id.clone()
}

fn parse_evidence_kind(value: &str) -> Option<EvolutionGoalEvidenceKind> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "cargo_check" => Some(EvolutionGoalEvidenceKind::CargoCheck),
        "focused_tests" => Some(EvolutionGoalEvidenceKind::FocusedTests),
        "benchmark_gate" => Some(EvolutionGoalEvidenceKind::BenchmarkGate),
        "trace_schema_gate" => Some(EvolutionGoalEvidenceKind::TraceSchemaGate),
        "experiment_ledger" => Some(EvolutionGoalEvidenceKind::ExperimentLedger),
        "operator_approval" => Some(EvolutionGoalEvidenceKind::OperatorApproval),
        _ => None,
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "pass" | "passed" | "ok" | "success" => Some(true),
        "0" | "false" | "no" | "n" | "fail" | "failed" | "error" => Some(false),
        _ => None,
    }
}

fn read_current_queue(
    args: &Args,
    scope: &TenantScope,
    scoped_key: &str,
    store_policy: EvolutionGoalQueueStorePolicy,
) -> io::Result<(
    EvolutionGoalQueue,
    Option<EvolutionGoalQueueStoreReadReport>,
)> {
    let Some(path) = args.self_goal_queue_store_path.as_ref() else {
        return Ok((default_noiron_pursuit_goal_queue(), None));
    };
    let store = EvolutionGoalQueueDiskStore::open_with_policy(path, store_policy)?;
    let read = store.read_queue(scope, scoped_key)?;
    let queue = read
        .queue
        .clone()
        .unwrap_or_else(default_noiron_pursuit_goal_queue);
    Ok((queue, Some(read)))
}

fn write_append_execution_result(
    args: &Args,
    scope: &TenantScope,
    key: &TenantScopedKey,
    store_policy: EvolutionGoalQueueStorePolicy,
    append_execution: &SelfGoalQueueAppendExecutionReport,
) -> io::Result<Option<EvolutionGoalQueueStoreWriteReport>> {
    let Some(path) = args.self_goal_queue_store_path.as_ref() else {
        return Ok(None);
    };
    let mut store = EvolutionGoalQueueDiskStore::open_with_policy(path, store_policy)?;
    let approval = append_execution.resulting_queue.as_ref().map(|queue| {
        EvolutionGoalQueueStoreApproval::for_queue(
            &args.self_goal_queue_operator,
            &args.self_goal_queue_ticket,
            key,
            queue,
            &append_execution.rollback_anchor_digest,
        )
    });
    store
        .write_append_execution_result(scope, key, append_execution, approval.as_ref())
        .map(Some)
}

fn write_completion_preview_result(
    args: &Args,
    scope: &TenantScope,
    key: &TenantScopedKey,
    store_policy: EvolutionGoalQueueStorePolicy,
    completion_preview: &SelfGoalQueueCliCompletionPreview,
    completion_writer_gate: &UnifiedWriterGateReport,
) -> io::Result<Option<EvolutionGoalQueueStoreWriteReport>> {
    let Some(path) = args.self_goal_queue_store_path.as_ref() else {
        return Ok(None);
    };
    if completion_writer_gate.decision != UnifiedWriterGateDecision::ReadyForExplicitApply {
        return Ok(None);
    }
    let Some(resulting_queue) = completion_preview.resulting_queue.as_ref() else {
        return Ok(None);
    };
    let mut store = EvolutionGoalQueueDiskStore::open_with_policy(path, store_policy)?;
    let approval = EvolutionGoalQueueStoreApproval::for_queue(
        &args.self_goal_queue_operator,
        &args.self_goal_queue_ticket,
        key,
        resulting_queue,
        &completion_preview.rollback_anchor_digest,
    );
    store
        .write_queue(
            scope,
            key,
            resulting_queue,
            &completion_preview.rollback_anchor_digest,
            Some(&approval),
        )
        .map(Some)
}

fn self_goal_queue_scope(args: &Args) -> TenantScope {
    TenantScope::new(
        &args.self_goal_queue_tenant,
        &args.self_goal_queue_workspace,
        &args.self_goal_queue_session,
    )
}

fn self_goal_trace_path(args: &Args) -> Option<&Path> {
    args.trace_path
        .as_deref()
        .or(args.trace_schema_gate_path.as_deref())
}

fn append_self_goal_queue_continuation_trace_jsonl(
    path: impl AsRef<Path>,
    plan: &SelfGoalQueueCliContinuationPlan,
) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", plan.json_line())
}

fn append_self_goal_queue_evidence_plan_trace_jsonl(
    path: impl AsRef<Path>,
    plan: &SelfGoalQueueCliEvidencePlan,
) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", plan.json_line())
}

fn append_self_goal_queue_evidence_collection_trace_jsonl(
    path: impl AsRef<Path>,
    collection: &SelfGoalQueueCliEvidenceCollection,
) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", collection.json_line())
}

fn json_string_array(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>();
    format!("[{}]", values.join(","))
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
