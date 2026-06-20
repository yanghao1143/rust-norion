use std::collections::BTreeSet;

use crate::{
    ContextCandidate, ContextInjectionGate, ContextInjectionPlan, DefaultContextInjectionGate,
    KvPrefetchPlan, KvSwap, LongTermMatch, LongTermMemory, LongTermQuery, MemoryAdapter,
    MemoryAdapterCapability, MemoryAdapterDescriptor, MemoryAdapterHealth, MemoryIndexSource,
    MemoryRequestContext, MemoryResult,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryReusePolicy {
    pub max_prefetch_ids: usize,
    pub include_runtime_kv_candidate_ids: bool,
    pub kv_metadata_keys: Vec<String>,
}

impl Default for MemoryReusePolicy {
    fn default() -> Self {
        Self {
            max_prefetch_ids: 16,
            include_runtime_kv_candidate_ids: true,
            kv_metadata_keys: vec![
                "kv_shard_id".to_owned(),
                "kv_shard_ids".to_owned(),
                "runtime_kv_id".to_owned(),
                "runtime_kv_ids".to_owned(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryReusePlan {
    pub read_only: bool,
    pub candidate_count: usize,
    pub long_term_matches: Vec<LongTermMatch>,
    pub context_plan: ContextInjectionPlan,
    pub requested_kv_ids: Vec<String>,
    pub kv_prefetch_plan: Option<KvPrefetchPlan>,
}

impl MemoryReusePlan {
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    pub fn accepted_context_count(&self) -> usize {
        self.context_plan.accepted_ids().len()
    }

    pub fn rejected_context_count(&self) -> usize {
        self.context_plan.rejected_ids().len()
    }

    pub fn kv_promote_count(&self) -> usize {
        self.kv_prefetch_plan
            .as_ref()
            .map_or(0, KvPrefetchPlan::promote_count)
    }

    pub fn kv_missing_count(&self) -> usize {
        self.kv_prefetch_plan
            .as_ref()
            .map_or(0, KvPrefetchPlan::missing_count)
    }

    pub fn kv_already_hot_count(&self) -> usize {
        self.kv_prefetch_plan
            .as_ref()
            .map_or(0, KvPrefetchPlan::already_hot_count)
    }

    pub fn kv_duplicate_count(&self) -> usize {
        self.kv_prefetch_plan
            .as_ref()
            .map_or(0, KvPrefetchPlan::duplicate_count)
    }

    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        codes.insert("read_only".to_owned());
        if self.candidate_count > 0 {
            codes.insert("reuse_candidates".to_owned());
        }
        if !self.long_term_matches.is_empty() {
            codes.insert("long_term_matches".to_owned());
        }
        if self.accepted_context_count() > 0 {
            codes.insert("context_accepted".to_owned());
        }
        if self.rejected_context_count() > 0 {
            codes.insert("context_rejected".to_owned());
        }
        if !self.requested_kv_ids.is_empty() {
            codes.insert("kv_prefetch_requested".to_owned());
        }
        if self.kv_prefetch_plan.is_none() && !self.requested_kv_ids.is_empty() {
            codes.insert("kv_backend_unavailable".to_owned());
        }
        codes.extend(
            self.context_plan
                .reason_codes()
                .into_iter()
                .map(|code| format!("context_{code}")),
        );
        if let Some(prefetch) = &self.kv_prefetch_plan {
            codes.extend(
                prefetch
                    .reason_codes()
                    .into_iter()
                    .map(|code| format!("kv_{code}")),
            );
        }
        codes.into_iter().collect()
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let mut codes = BTreeSet::new();
        codes.extend(
            self.long_term_matches
                .iter()
                .map(|item| format!("long_term_match:{}", item.id)),
        );
        codes.extend(
            self.context_plan
                .detail_codes()
                .into_iter()
                .map(|code| format!("context:{code}")),
        );
        codes.extend(
            self.requested_kv_ids
                .iter()
                .map(|id| format!("kv_requested:{}", hex_id(id))),
        );
        if let Some(prefetch) = &self.kv_prefetch_plan {
            codes.extend(
                prefetch
                    .detail_codes()
                    .into_iter()
                    .map(|code| format!("kv_prefetch:{code}")),
            );
        } else if !self.requested_kv_ids.is_empty() {
            codes.insert("kv_backend_unavailable".to_owned());
        }
        codes.into_iter().collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "memory_reuse_dry_run read_only={} candidates={} long_term_matches={} context_decisions={} context_accepted={} context_rejected={} used_tokens={} kv_requested={} kv_promote={} kv_missing={} kv_hot={} kv_duplicate={} reason_codes={} detail_codes={}",
            self.read_only,
            self.candidate_count,
            self.long_term_matches.len(),
            self.context_plan.decisions.len(),
            self.accepted_context_count(),
            self.rejected_context_count(),
            self.context_plan.used_tokens,
            self.requested_kv_ids.len(),
            self.kv_promote_count(),
            self.kv_missing_count(),
            self.kv_already_hot_count(),
            self.kv_duplicate_count(),
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }
}

pub trait MemoryReusePlanner {
    fn plan_from_candidates(
        &self,
        candidates: &[ContextCandidate],
        request: &MemoryRequestContext,
        kv_swap: Option<&dyn KvSwap>,
    ) -> MemoryReusePlan;
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DefaultMemoryReusePlanner {
    pub policy: MemoryReusePolicy,
    pub context_gate: DefaultContextInjectionGate,
}

impl DefaultMemoryReusePlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_context_gate(mut self, context_gate: DefaultContextInjectionGate) -> Self {
        self.context_gate = context_gate;
        self
    }

    pub fn with_policy(mut self, policy: MemoryReusePolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn plan_from_long_term<M: LongTermMemory>(
        &self,
        memory: &M,
        query: LongTermQuery,
        request: &MemoryRequestContext,
        kv_swap: Option<&dyn KvSwap>,
    ) -> MemoryResult<MemoryReusePlan> {
        let matches = memory.search(query)?;
        let candidates = matches
            .iter()
            .map(ContextCandidate::from_long_term_match)
            .collect::<Vec<_>>();
        let mut plan = self.plan_from_candidates(&candidates, request, kv_swap);
        plan.long_term_matches = matches;
        Ok(plan)
    }
}

impl MemoryAdapter for DefaultMemoryReusePlanner {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_memory_reuse_planner",
            vec![
                MemoryAdapterCapability::LongTermMemory,
                MemoryAdapterCapability::ContextInjection,
                MemoryAdapterCapability::KvSwap,
            ],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        Ok(MemoryAdapterHealth::ready(None))
    }
}

impl MemoryReusePlanner for DefaultMemoryReusePlanner {
    fn plan_from_candidates(
        &self,
        candidates: &[ContextCandidate],
        request: &MemoryRequestContext,
        kv_swap: Option<&dyn KvSwap>,
    ) -> MemoryReusePlan {
        let context_plan = self.context_gate.plan(candidates, request);
        let accepted_ids = context_plan
            .accepted_ids()
            .into_iter()
            .collect::<BTreeSet<_>>();
        let requested_kv_ids =
            prefetch_ids_for_accepted_candidates(candidates, &accepted_ids, &self.policy);
        let kv_prefetch_plan = kv_swap.map(|swap| swap.plan_prefetch(&requested_kv_ids));

        MemoryReusePlan {
            read_only: true,
            candidate_count: candidates.len(),
            long_term_matches: Vec::new(),
            context_plan,
            requested_kv_ids,
            kv_prefetch_plan,
        }
    }
}

fn prefetch_ids_for_accepted_candidates(
    candidates: &[ContextCandidate],
    accepted_ids: &BTreeSet<String>,
    policy: &MemoryReusePolicy,
) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = BTreeSet::new();

    for candidate in candidates {
        if !accepted_ids.contains(&candidate.id) {
            continue;
        }
        if policy.include_runtime_kv_candidate_ids
            && candidate.source == MemoryIndexSource::RuntimeKv
        {
            push_prefetch_id(&mut ids, &mut seen, &candidate.id, policy.max_prefetch_ids);
        }
        for key in &policy.kv_metadata_keys {
            if let Some(value) = candidate.metadata.get(key) {
                for id in split_prefetch_ids(value) {
                    push_prefetch_id(&mut ids, &mut seen, id, policy.max_prefetch_ids);
                }
            }
        }
        if ids.len() >= policy.max_prefetch_ids {
            break;
        }
    }

    ids
}

fn split_prefetch_ids(value: &str) -> impl Iterator<Item = &str> {
    value.split([',', ';', '|']).map(str::trim)
}

fn push_prefetch_id(
    ids: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    id: &str,
    max_prefetch_ids: usize,
) {
    if ids.len() >= max_prefetch_ids {
        return;
    }
    let id = id.trim();
    if !id.is_empty() && seen.insert(id.to_owned()) {
        ids.push(id.to_owned());
    }
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

fn hex_id(id: &str) -> String {
    id.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ContextDecisionKind, InMemoryDiskKvOffload, InMemoryLongTermMemory, KvSwapManager, KvTier,
        MemoryAccessPurpose, MemoryDocumentInput, MemoryScope, Metadata,
    };

    fn request(task: &str) -> MemoryRequestContext {
        MemoryRequestContext::new(MemoryScope::for_task(task), MemoryAccessPurpose::Recall)
            .with_limit(8)
    }

    fn metadata_with_kv(ids: &str) -> Metadata {
        let mut metadata = Metadata::new();
        metadata.insert("kv_shard_ids".to_owned(), ids.to_owned());
        metadata.insert("domain".to_owned(), "runtime_reuse".to_owned());
        metadata
    }

    #[test]
    fn reuse_planner_retrieves_long_term_context_and_prefetches_accepted_kv() {
        let mut memory = InMemoryLongTermMemory::new();
        let memory_id = memory
            .remember(
                MemoryDocumentInput::new("Rust borrow checker reuse lesson", vec![1.0, 0.0])
                    .with_scope(MemoryScope::for_task("runtime"))
                    .with_strength(0.9)
                    .with_metadata(metadata_with_kv("cold-a, hot-a, missing-a")),
            )
            .unwrap();
        memory
            .remember(
                MemoryDocumentInput::new("unrelated deployment note", vec![0.0, 1.0])
                    .with_scope(MemoryScope::for_task("runtime")),
            )
            .unwrap();

        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        swap.stage_hot("cold-a".to_owned(), b"cold bytes".to_vec(), 0.5)
            .unwrap();
        let eviction = swap.plan_eviction(0);
        swap.evict(&eviction).unwrap();
        swap.stage_hot("hot-a".to_owned(), b"hot bytes".to_vec(), 0.9)
            .unwrap();

        let plan = DefaultMemoryReusePlanner::new()
            .plan_from_long_term(
                &memory,
                LongTermQuery::by_text("borrow checker", 4)
                    .with_scope(MemoryScope::for_task("runtime"))
                    .with_metadata_filter("domain", "runtime_reuse"),
                &request("runtime"),
                Some(&swap),
            )
            .unwrap();

        assert!(plan.is_read_only());
        assert_eq!(plan.long_term_matches.len(), 1);
        assert_eq!(plan.long_term_matches[0].id, memory_id);
        assert_eq!(
            plan.context_plan.accepted_ids(),
            vec![memory_id.to_string()]
        );
        assert_eq!(
            plan.requested_kv_ids,
            vec![
                "cold-a".to_owned(),
                "hot-a".to_owned(),
                "missing-a".to_owned()
            ]
        );

        let prefetch = plan.kv_prefetch_plan.as_ref().unwrap();
        assert_eq!(prefetch.promote_ids, vec!["cold-a".to_owned()]);
        assert_eq!(prefetch.already_hot_ids, vec!["hot-a".to_owned()]);
        assert_eq!(prefetch.missing_ids, vec!["missing-a".to_owned()]);
        assert!(swap.hot_bytes("cold-a").is_none());
        assert_eq!(swap.metadata("cold-a").unwrap().tier, KvTier::Cold);
        assert!(plan.summary_line().contains("read_only=true"));
    }

    #[test]
    fn reuse_planner_ignores_rejected_context_for_kv_prefetch() {
        let mut accepted = ContextCandidate::new("accepted", "safe runtime lesson", 0.9)
            .with_scope(MemoryScope::for_task("runtime"));
        accepted
            .metadata
            .insert("kv_shard_id".to_owned(), "safe-cold".to_owned());
        let mut risky = ContextCandidate::new("risky", "polluted transcript", 0.95)
            .with_scope(MemoryScope::for_task("runtime"))
            .with_risk_reasons(vec!["cross_task_transcript_pollution".to_owned()]);
        risky
            .metadata
            .insert("kv_shard_id".to_owned(), "risky-cold".to_owned());

        let backend = InMemoryDiskKvOffload::new();
        let mut swap = KvSwapManager::new(backend);
        for id in ["safe-cold", "risky-cold"] {
            swap.stage_hot(id.to_owned(), b"bytes".to_vec(), 0.5)
                .unwrap();
        }
        let eviction = swap.plan_eviction(0);
        swap.evict(&eviction).unwrap();

        let plan = DefaultMemoryReusePlanner::new().plan_from_candidates(
            &[accepted, risky],
            &request("runtime"),
            Some(&swap),
        );

        assert_eq!(
            plan.context_plan.accepted_ids(),
            vec!["accepted".to_owned()]
        );
        assert!(plan.context_plan.decisions.iter().any(|decision| {
            decision.candidate_id == "risky" && decision.kind == ContextDecisionKind::RejectRisk
        }));
        assert_eq!(plan.requested_kv_ids, vec!["safe-cold".to_owned()]);
        assert_eq!(
            plan.kv_prefetch_plan.as_ref().unwrap().promote_ids,
            vec!["safe-cold".to_owned()]
        );
    }

    #[test]
    fn reuse_plan_without_kv_backend_reports_context_only_dry_run() {
        let runtime_kv = ContextCandidate::new("runtime-kv-1", "hot runtime adapter state", 0.9)
            .with_source(MemoryIndexSource::RuntimeKv)
            .with_scope(MemoryScope::for_task("runtime"));

        let plan = DefaultMemoryReusePlanner::new().plan_from_candidates(
            &[runtime_kv],
            &request("runtime"),
            None,
        );

        assert!(plan.is_read_only());
        assert_eq!(plan.accepted_context_count(), 1);
        assert_eq!(plan.requested_kv_ids, vec!["runtime-kv-1".to_owned()]);
        assert!(plan.kv_prefetch_plan.is_none());
        assert!(
            plan.reason_codes()
                .contains(&"kv_backend_unavailable".to_owned())
        );
        assert!(plan.summary_line().contains("kv_requested=1"));
    }

    #[test]
    fn reuse_planner_is_read_only_adapter() {
        let descriptor = DefaultMemoryReusePlanner::new().descriptor();

        assert_eq!(descriptor.name, "default_memory_reuse_planner");
        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::LongTermMemory)
        );
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::ContextInjection)
        );
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::KvSwap)
        );
    }
}
