use std::io;
use std::path::Path;

use rust_norion::{
    EvolutionGoalQueue, EvolutionGoalQueueDiskStore, EvolutionGoalQueueStoreApproval,
    EvolutionGoalQueueStorePolicy, EvolutionGoalQueueStoreReadReport,
    EvolutionGoalQueueStoreWriteReport, SelfGoalAdmissionReport, SelfGoalProposalReport,
    SelfGoalQueueAppendApproval, SelfGoalQueueAppendExecutionReport, SelfGoalQueueAppendExecutor,
    SelfGoalQueueApplyReport, SelfGoalQueuePreviewReport, TenantResourceLane, TenantScope,
    TenantScopedKey, UnifiedWriterGate, UnifiedWriterGateCandidate, UnifiedWriterGateReport,
    append_evolution_goal_queue_store_write_trace_jsonl,
    append_self_goal_queue_append_execution_trace_jsonl, append_self_goal_queue_apply_trace_jsonl,
    default_noiron_pursuit_goal_queue, default_self_goal_admission_report,
    default_self_goal_proposal_report, default_self_goal_queue_apply_report,
    default_self_goal_queue_preview_report,
};

use crate::cli::args::Args;

#[derive(Debug, Clone)]
pub(crate) struct SelfGoalQueueCliReport {
    pub(crate) current_queue_digest: String,
    pub(crate) current_goal_count: usize,
    pub(crate) current_queue_loaded_from_store: bool,
    pub(crate) store_read: Option<EvolutionGoalQueueStoreReadReport>,
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
            self.proposal.summary_line(),
            self.admission.summary_line(),
            self.queue_preview.summary_line(),
            self.writer_gate.summary_line(),
            self.apply.summary_line(),
            self.append_execution.summary_line(),
        ];
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
    let admission = default_self_goal_admission_report(&proposal, &[]);
    let queue_preview =
        default_self_goal_queue_preview_report(&current_queue, &proposal, &admission);
    let writer_gate =
        UnifiedWriterGate::new().evaluate([UnifiedWriterGateCandidate::self_goal_queue_preview(
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

    let store_write = if args.self_goal_queue_store_apply {
        write_append_execution_result(args, &scope, &key, store_policy, &append_execution)?
    } else {
        None
    };

    if let Some(trace_path) = self_goal_trace_path(args) {
        append_self_goal_queue_apply_trace_jsonl(trace_path, &apply)?;
        append_self_goal_queue_append_execution_trace_jsonl(trace_path, &append_execution)?;
        if let Some(store_write) = &store_write {
            append_evolution_goal_queue_store_write_trace_jsonl(trace_path, store_write)?;
        }
    }

    Ok(SelfGoalQueueCliReport {
        current_queue_digest,
        current_goal_count,
        current_queue_loaded_from_store,
        store_read,
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
