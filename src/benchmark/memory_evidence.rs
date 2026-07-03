use crate::engine::InferenceOutcome;
use crate::hardware::DeviceClass;
use crate::privacy_redaction::contains_private_or_executable_marker;

use super::{BenchmarkCase, explicit_device_count, push_unique_device};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BenchmarkMemoryGovernanceEvidence {
    pub cases: usize,
    pub memory_admission_cases: usize,
    pub memory_admission_candidates: usize,
    pub memory_admission_ready: usize,
    pub memory_admission_blocked: usize,
    pub memory_admission_admitted: usize,
    pub memory_admission_hold: usize,
    pub memory_admission_reject: usize,
    pub memory_admission_quarantine: usize,
    pub memory_admission_review_packets: usize,
    pub memory_admission_ledger_records: usize,
    pub memory_admission_ledger_authorized: usize,
    pub memory_admission_ledger_applied: usize,
    pub memory_admission_ledger_preview_only: usize,
    pub memory_admission_ledger_held: usize,
    pub memory_admission_ledger_rejected: usize,
    pub memory_admission_ledger_duplicate: usize,
    pub memory_admission_ledger_decayed: usize,
    pub memory_admission_ledger_merged: usize,
    pub memory_admission_ledger_rollback: usize,
    pub kv_fusion_cases: usize,
    pub kv_fusion_candidates: usize,
    pub kv_fusion_fused: usize,
    pub kv_fusion_compressed: usize,
    pub kv_fusion_skipped: usize,
    pub kv_fusion_held: usize,
    pub kv_fusion_rejected: usize,
    pub kv_fusion_approval_blocked: usize,
    pub kv_fusion_input_tokens: usize,
    pub kv_fusion_retained_tokens: usize,
    pub kv_fusion_saved_tokens: usize,
    pub retention_activity_cases: usize,
    pub compaction_activity_cases: usize,
    pub total_retention_decayed: usize,
    pub total_retention_removed: usize,
    pub total_compaction_merged: usize,
    pub total_compaction_removed: usize,
    pub total_compaction_pair_evidence: usize,
    pub memory_storage_samples: usize,
    pub memory_storage_entries_before: usize,
    pub memory_storage_entries_after: usize,
    pub memory_storage_entries_removed: usize,
    pub memory_retrieval_latency_samples: usize,
    pub total_memory_retrieval_latency_ms: u128,
    pub max_memory_retrieval_latency_ms: u128,
    pub memory_retained_usefulness_delta_milli: i64,
    pub memory_retained_usefulness_abs_delta_milli: usize,
    pub failures: Vec<String>,
    pub(super) governance_devices: Vec<DeviceClass>,
    pub(super) memory_admission_devices: Vec<DeviceClass>,
    pub(super) retention_activity_devices: Vec<DeviceClass>,
    pub(super) compaction_activity_devices: Vec<DeviceClass>,
}

impl BenchmarkMemoryGovernanceEvidence {
    pub(super) fn record(
        &mut self,
        case: &BenchmarkCase,
        elapsed_ms: u128,
        outcome: &InferenceOutcome,
    ) {
        self.cases += 1;
        let device = outcome.hardware_plan.device;
        push_unique_device(&mut self.governance_devices, device);
        self.memory_retrieval_latency_samples += 1;
        self.total_memory_retrieval_latency_ms = self
            .total_memory_retrieval_latency_ms
            .saturating_add(elapsed_ms);
        self.max_memory_retrieval_latency_ms = self.max_memory_retrieval_latency_ms.max(elapsed_ms);
        let retained_usefulness_delta =
            outcome.memory_feedback.reinforcement_amount - outcome.memory_feedback.penalty_amount;
        let retained_usefulness_delta_milli = (retained_usefulness_delta * 1000.0).round() as i64;
        self.memory_retained_usefulness_delta_milli = self
            .memory_retained_usefulness_delta_milli
            .saturating_add(retained_usefulness_delta_milli);
        self.memory_retained_usefulness_abs_delta_milli = self
            .memory_retained_usefulness_abs_delta_milli
            .saturating_add(retained_usefulness_delta_milli.unsigned_abs() as usize);

        let admission = &outcome.memory_admission;
        let admission_candidates = admission.candidate_count();
        self.memory_admission_candidates += admission_candidates;
        self.memory_admission_ready += admission.ready_count();
        self.memory_admission_blocked += admission.blocked_count();
        self.memory_admission_admitted += admission.admitted_count();
        self.memory_admission_hold += admission.hold_count();
        self.memory_admission_reject += admission.reject_count();
        self.memory_admission_quarantine += admission.quarantine_count();
        self.memory_admission_review_packets += admission.review_packet_count();
        self.memory_admission_ledger_records += admission.ledger_record_count();
        self.memory_admission_ledger_authorized += admission.ledger_authorized_count();
        self.memory_admission_ledger_applied += admission.ledger_applied_count();
        self.memory_admission_ledger_preview_only += admission.ledger_preview_only_count();
        self.memory_admission_ledger_held += admission.ledger_held_count();
        self.memory_admission_ledger_rejected += admission.ledger_rejected_count();
        self.memory_admission_ledger_duplicate += admission.ledger_duplicate_count();
        self.memory_admission_ledger_decayed += admission.ledger_decayed_count();
        self.memory_admission_ledger_merged += admission.ledger_merged_count();
        self.memory_admission_ledger_rollback += admission.ledger_rollback_count();
        self.kv_fusion_candidates += admission.fusion_plan.candidates;
        self.kv_fusion_fused += admission.fusion_plan.fused;
        self.kv_fusion_compressed += admission.fusion_plan.compressed;
        self.kv_fusion_skipped += admission.fusion_plan.skipped;
        self.kv_fusion_held += admission.fusion_plan.held;
        self.kv_fusion_rejected += admission.fusion_plan.rejected;
        self.kv_fusion_approval_blocked += admission.fusion_plan.approval_blocked;
        self.kv_fusion_input_tokens += admission.fusion_plan.input_tokens;
        self.kv_fusion_retained_tokens += admission.fusion_plan.retained_tokens;
        self.kv_fusion_saved_tokens += admission.fusion_plan.saved_tokens;
        if admission.fusion_plan.candidates > 0 {
            self.kv_fusion_cases += 1;
        }
        if admission_candidates > 0 {
            self.memory_admission_cases += 1;
            push_unique_device(&mut self.memory_admission_devices, device);
        } else {
            self.failures.push(format!(
                "{}:{} memory_admission must include at least one candidate",
                device.as_str(),
                case.name
            ));
        }
        validate_memory_admission_preview(
            &mut self.failures,
            device,
            &case.name,
            case.profile,
            &case.prompt,
            admission,
        );

        let retention = &outcome.retention_report;
        let retention_removed = retention.removed.len();
        self.total_retention_decayed += retention.decayed;
        self.total_retention_removed += retention_removed;
        if retention.decayed > 0 || retention_removed > 0 {
            self.retention_activity_cases += 1;
            push_unique_device(&mut self.retention_activity_devices, device);
        }

        if outcome.memory_retention_policy.stale_after == 0 {
            self.failures.push(format!(
                "{}:{} retention stale_after must be > 0",
                device.as_str(),
                case.name
            ));
        }
        if !(0.0..=0.95).contains(&outcome.memory_retention_policy.decay_rate) {
            self.failures.push(format!(
                "{}:{} retention decay_rate {:.6} outside 0.0..=0.95",
                device.as_str(),
                case.name,
                outcome.memory_retention_policy.decay_rate
            ));
        }
        if !(0.0..=3.0).contains(&outcome.memory_retention_policy.remove_below_strength) {
            self.failures.push(format!(
                "{}:{} retention remove_below_strength {:.6} outside 0.0..=3.0",
                device.as_str(),
                case.name,
                outcome.memory_retention_policy.remove_below_strength
            ));
        }
        if outcome.memory_retention_policy.remove_after_failures == 0 {
            self.failures.push(format!(
                "{}:{} retention remove_after_failures must be > 0",
                device.as_str(),
                case.name
            ));
        }
        if retention.decayed > retention.before {
            self.failures.push(format!(
                "{}:{} retention decayed {} exceeds before {}",
                device.as_str(),
                case.name,
                retention.decayed,
                retention.before
            ));
        }
        if retention_removed > retention.before {
            self.failures.push(format!(
                "{}:{} retention removed {} exceeds before {}",
                device.as_str(),
                case.name,
                retention_removed,
                retention.before
            ));
        }
        if retention.after > retention.before {
            self.failures.push(format!(
                "{}:{} retention after {} exceeds before {}",
                device.as_str(),
                case.name,
                retention.after,
                retention.before
            ));
        }
        if retention.after.saturating_add(retention_removed) != retention.before {
            self.failures.push(format!(
                "{}:{} retention before {} does not match after+removed {}",
                device.as_str(),
                case.name,
                retention.before,
                retention.after.saturating_add(retention_removed)
            ));
        }

        let compaction = &outcome.memory_compaction_report;
        let compaction_merged = compaction.merged.len();
        let compaction_removed = compaction.removed.len();
        self.total_compaction_merged += compaction_merged;
        self.total_compaction_removed += compaction_removed;
        self.total_compaction_pair_evidence += compaction.merged.len();
        self.memory_storage_samples += 1;
        self.memory_storage_entries_before = self
            .memory_storage_entries_before
            .saturating_add(retention.before);
        self.memory_storage_entries_after = self
            .memory_storage_entries_after
            .saturating_add(compaction.after);
        self.memory_storage_entries_removed = self
            .memory_storage_entries_removed
            .saturating_add(retention_removed)
            .saturating_add(compaction_removed);
        if compaction_merged > 0 || compaction_removed > 0 {
            self.compaction_activity_cases += 1;
            push_unique_device(&mut self.compaction_activity_devices, device);
        }

        if !(0.10..=0.999).contains(&outcome.memory_compaction_policy.similarity_threshold) {
            self.failures.push(format!(
                "{}:{} memory_compaction similarity_threshold {:.6} outside 0.10..=0.999",
                device.as_str(),
                case.name,
                outcome.memory_compaction_policy.similarity_threshold
            ));
        }
        if compaction.merged.len() != compaction.removed.len() {
            self.failures.push(format!(
                "{}:{} memory_compaction merged {} does not match removed {}",
                device.as_str(),
                case.name,
                compaction_merged,
                compaction_removed
            ));
        }
        if compaction_merged > outcome.memory_compaction_policy.max_merges {
            self.failures.push(format!(
                "{}:{} memory_compaction merged {} exceeds max_merges {}",
                device.as_str(),
                case.name,
                compaction_merged,
                outcome.memory_compaction_policy.max_merges
            ));
        }
        if compaction_removed > compaction.before {
            self.failures.push(format!(
                "{}:{} memory_compaction removed {} exceeds before {}",
                device.as_str(),
                case.name,
                compaction_removed,
                compaction.before
            ));
        }
        if compaction.after > compaction.before {
            self.failures.push(format!(
                "{}:{} memory_compaction after {} exceeds before {}",
                device.as_str(),
                case.name,
                compaction.after,
                compaction.before
            ));
        }
        if compaction.after.saturating_add(compaction_removed) != compaction.before {
            self.failures.push(format!(
                "{}:{} memory_compaction before {} does not match after+removed {}",
                device.as_str(),
                case.name,
                compaction.before,
                compaction.after.saturating_add(compaction_removed)
            ));
        }
        if (compaction.before < 2
            || outcome.memory_compaction_policy.max_candidates < 2
            || outcome.memory_compaction_policy.max_merges == 0)
            && (compaction_merged > 0
                || compaction_removed > 0
                || compaction.after != compaction.before)
        {
            self.failures.push(format!(
                    "{}:{} memory_compaction skipped state requires merged=0 removed=0 after=before, got merged={} removed={} before={} after={}",
                    device.as_str(),
                    case.name,
                    compaction_merged,
                    compaction_removed,
                    compaction.before,
                    compaction.after
                ));
        }
        for (index, pair) in compaction.merged.iter().enumerate() {
            validate_compaction_pair_evidence(
                &mut self.failures,
                device,
                &case.name,
                index,
                pair,
                &compaction.removed,
            );
        }
    }

    pub fn device_profiles(&self) -> usize {
        explicit_device_count(&self.governance_devices)
    }

    pub fn retention_activity_device_profiles(&self) -> usize {
        explicit_device_count(&self.retention_activity_devices)
    }

    pub fn memory_admission_device_profiles(&self) -> usize {
        explicit_device_count(&self.memory_admission_devices)
    }

    pub fn compaction_activity_device_profiles(&self) -> usize {
        explicit_device_count(&self.compaction_activity_devices)
    }

    pub fn kv_fusion_decision_total(&self) -> usize {
        self.kv_fusion_fused
            .saturating_add(self.kv_fusion_compressed)
            .saturating_add(self.kv_fusion_skipped)
            .saturating_add(self.kv_fusion_held)
            .saturating_add(self.kv_fusion_rejected)
            .saturating_add(self.kv_fusion_approval_blocked)
    }
}

fn validate_memory_admission_preview(
    failures: &mut Vec<String>,
    device: DeviceClass,
    case_name: &str,
    profile: crate::hierarchy::TaskProfile,
    prompt: &str,
    admission: &crate::memory_admission::MemoryAdmissionPreview,
) {
    let candidates = admission.candidate_count();
    let prompt_chars = prompt.chars().count();
    let prompt_leak_check = prompt.len() > 16;
    let decision_total = admission
        .ready_count()
        .saturating_add(admission.hold_count())
        .saturating_add(admission.reject_count())
        .saturating_add(admission.quarantine_count());
    if decision_total != candidates {
        failures.push(format!(
            "{}:{} memory_admission decision counts {} do not match candidates {}",
            device.as_str(),
            case_name,
            decision_total,
            candidates
        ));
    }
    let summaries = admission.candidate_summaries();
    let review_summaries = admission.review_packet_summaries();
    if summaries.len() != candidates {
        failures.push(format!(
            "{}:{} memory_admission summaries {} do not match candidates {}",
            device.as_str(),
            case_name,
            summaries.len(),
            candidates
        ));
    }
    if admission.blocked_count()
        != admission
            .hold_count()
            .saturating_add(admission.quarantine_count())
    {
        failures.push(format!(
            "{}:{} memory_admission blocked {} does not match hold+quarantine {}",
            device.as_str(),
            case_name,
            admission.blocked_count(),
            admission
                .hold_count()
                .saturating_add(admission.quarantine_count())
        ));
    }
    if admission.review_packet_count() != candidates {
        failures.push(format!(
            "{}:{} memory_admission review_packets {} do not match candidates {}",
            device.as_str(),
            case_name,
            admission.review_packet_count(),
            candidates
        ));
    }
    if admission.ledger_record_count() != candidates {
        failures.push(format!(
            "{}:{} memory_admission ledger records {} do not match candidates {}",
            device.as_str(),
            case_name,
            admission.ledger_record_count(),
            candidates
        ));
    }
    if admission.ledger_applied_count() > admission.ledger_authorized_count() {
        failures.push(format!(
            "{}:{} memory_admission ledger applied {} exceeds authorized {}",
            device.as_str(),
            case_name,
            admission.ledger_applied_count(),
            admission.ledger_authorized_count()
        ));
    }
    if admission.read_only
        && (admission.ledger_authorized_count() > 0 || admission.ledger_applied_count() > 0)
    {
        failures.push(format!(
            "{}:{} memory_admission preview must not authorize or apply ledger writes",
            device.as_str(),
            case_name
        ));
    }
    if admission.ledger_summaries().len() != admission.ledger_record_count() {
        failures.push(format!(
            "{}:{} memory_admission ledger summaries {} do not match ledger records {}",
            device.as_str(),
            case_name,
            admission.ledger_summaries().len(),
            admission.ledger_record_count()
        ));
    }
    if review_summaries.len() != admission.review_packet_count() {
        failures.push(format!(
            "{}:{} memory_admission review summaries {} do not match review packets {}",
            device.as_str(),
            case_name,
            review_summaries.len(),
            admission.review_packet_count()
        ));
    }
    if !admission.is_read_only_preview() {
        failures.push(format!(
            "{}:{} memory_admission preview must remain read-only and unapplied",
            device.as_str(),
            case_name
        ));
    }
    let fusion = &admission.fusion_plan;
    if !fusion.decision_count_matches() {
        failures.push(format!(
            "{}:{} kv_fusion decision counts do not match candidates {}",
            device.as_str(),
            case_name,
            fusion.candidates
        ));
    }
    if !fusion.token_accounting_matches() {
        failures.push(format!(
            "{}:{} kv_fusion retained+saved {} does not match input_tokens {}",
            device.as_str(),
            case_name,
            fusion.retained_tokens.saturating_add(fusion.saved_tokens),
            fusion.input_tokens
        ));
    }
    if !fusion.is_read_only_preview() {
        failures.push(format!(
            "{}:{} kv_fusion preview must remain read-only and unapplied",
            device.as_str(),
            case_name
        ));
    }
    if fusion.candidates > 0 && fusion.score_summaries(12).is_empty() {
        failures.push(format!(
            "{}:{} kv_fusion candidates require score summaries",
            device.as_str(),
            case_name
        ));
    }
    for (index, summary) in fusion.score_summaries(12).iter().enumerate() {
        if !summary.contains("components=") {
            failures.push(format!(
                "{}:{} kv_fusion score summary {} must explain score components",
                device.as_str(),
                case_name,
                index
            ));
        }
    }
    if fusion.score_summaries(12).iter().any(|summary| {
        contains_private_or_executable_marker(summary)
            || (prompt_leak_check && summary.contains(prompt))
    }) {
        failures.push(format!(
            "{}:{} kv_fusion summaries must not leak raw prompt or answer payloads",
            device.as_str(),
            case_name
        ));
    }
    if summaries
        .iter()
        .chain(review_summaries.iter())
        .chain(admission.ledger_summaries().iter())
        .any(|summary| contains_private_or_executable_marker(summary))
    {
        failures.push(format!(
            "{}:{} memory_admission summaries, review packets, or ledger rows must not leak raw prompt or answer payloads",
            device.as_str(),
            case_name
        ));
    }
    if admission.read_only && admission.admitted_count() > 0 {
        failures.push(format!(
            "{}:{} memory_admission read-only preview must not report admitted packets",
            device.as_str(),
            case_name
        ));
    }
    for (index, summary) in review_summaries.iter().enumerate() {
        if !summary.contains("approval=")
            || !summary.contains("next=")
            || !summary.contains("rollback=")
        {
            failures.push(format!(
                "{}:{} memory_admission review packet {index} is missing approval, next action, or rollback evidence",
                device.as_str(),
                case_name
            ));
        }
    }
    for (index, candidate) in admission.candidates.iter().enumerate() {
        if candidate.profile != profile {
            failures.push(format!(
                "{}:{} memory_admission candidate {index} profile {:?} does not match case profile {:?}",
                device.as_str(),
                case_name,
                candidate.profile,
                profile
            ));
        }
        if candidate.kind == crate::memory_admission::MemoryAdmissionKind::GeneSegmentKvEvidence {
            if candidate.prompt_chars == 0 {
                failures.push(format!(
                    "{}:{} memory_admission gene-segment candidate {index} token count must be non-zero",
                    device.as_str(),
                    case_name
                ));
            }
        } else if candidate.prompt_chars != prompt_chars {
            failures.push(format!(
                "{}:{} memory_admission candidate {index} prompt_chars {} does not match case prompt_chars {}",
                device.as_str(),
                case_name,
                candidate.prompt_chars,
                prompt_chars
            ));
        }
        if candidate.prompt_digest.is_empty() || candidate.prompt_digest.len() > 32 {
            failures.push(format!(
                "{}:{} memory_admission candidate {index} has invalid prompt digest evidence",
                device.as_str(),
                case_name
            ));
        }
        if !memory_admission_source_hash_has_digest_evidence(candidate) {
            failures.push(format!(
                "{}:{} memory_admission candidate {index} has invalid source hash evidence",
                device.as_str(),
                case_name
            ));
        }
        if candidate.validation_evidence.is_empty() {
            failures.push(format!(
                "{}:{} memory_admission candidate {index} is missing validation evidence",
                device.as_str(),
                case_name
            ));
        }
        if !candidate.quality.is_finite() || !(0.0..=1.0).contains(&candidate.quality) {
            failures.push(format!(
                "{}:{} memory_admission candidate {index} quality {:.6} outside 0.0..=1.0",
                device.as_str(),
                case_name,
                candidate.quality
            ));
        }
        if !candidate.process_reward.is_finite() || !(0.0..=1.0).contains(&candidate.process_reward)
        {
            failures.push(format!(
                "{}:{} memory_admission candidate {index} process_reward {:.6} outside 0.0..=1.0",
                device.as_str(),
                case_name,
                candidate.process_reward
            ));
        }
        if !candidate.is_read_only_preview() {
            failures.push(format!(
                "{}:{} memory_admission candidate {index} must remain read-only and unapplied",
                device.as_str(),
                case_name
            ));
        }
        if prompt_leak_check && candidate.id.contains(prompt) {
            failures.push(format!(
                "{}:{} memory_admission candidate {index} id leaks raw prompt text",
                device.as_str(),
                case_name
            ));
        }
    }
}

fn memory_admission_source_hash_has_digest_evidence(
    candidate: &crate::memory_admission::MemoryAdmissionCandidate,
) -> bool {
    let source_hash = candidate.source_hash.trim();
    if source_hash.is_empty() {
        return false;
    }
    if source_hash.starts_with("sha256:") {
        return true;
    }
    candidate.kind == crate::memory_admission::MemoryAdmissionKind::GeneSegmentKvEvidence
        && source_hash.starts_with("redaction-digest:")
}

fn validate_compaction_pair_evidence(
    failures: &mut Vec<String>,
    device: DeviceClass,
    case_name: &str,
    index: usize,
    pair: &crate::kv_cache::MemoryCompactionMerge,
    removed_ids: &[u64],
) {
    if pair.primary_id == 0 || pair.removed_id == 0 {
        failures.push(format!(
            "{}:{} memory_compaction pair {index} primary_id and removed_id must be non-zero",
            device.as_str(),
            case_name
        ));
    }
    if pair.primary_id == pair.removed_id {
        failures.push(format!(
            "{}:{} memory_compaction pair {index} primary_id must differ from removed_id",
            device.as_str(),
            case_name
        ));
    }
    if !removed_ids.contains(&pair.removed_id) {
        failures.push(format!(
            "{}:{} memory_compaction pair {index} removed_id {} is missing from removed set",
            device.as_str(),
            case_name,
            pair.removed_id
        ));
    }
    if !(0.10..=1.0).contains(&pair.similarity) {
        failures.push(format!(
            "{}:{} memory_compaction pair {index} similarity {:.6} outside 0.10..=1.0",
            device.as_str(),
            case_name,
            pair.similarity
        ));
    }
    if !namespace_is_safe_for_compaction_evidence(&pair.namespace) {
        failures.push(format!(
            "{}:{} memory_compaction pair {index} namespace is empty, too broad, or leaks prompt text",
            device.as_str(),
            case_name
        ));
    }
    if pair.primary_vector_dimensions == 0 || pair.removed_vector_dimensions == 0 {
        failures.push(format!(
            "{}:{} memory_compaction pair {index} vector dimensions must be non-zero",
            device.as_str(),
            case_name
        ));
    }
    if pair.removed_protected {
        failures.push(format!(
            "{}:{} memory_compaction pair {index} must not remove a protected memory",
            device.as_str(),
            case_name
        ));
    }
}

fn namespace_is_safe_for_compaction_evidence(namespace: &str) -> bool {
    if namespace.is_empty() || namespace.len() > 96 || namespace.contains(" :: ") {
        return false;
    }
    namespace == "semantic"
        || namespace == "gist"
        || (namespace.starts_with("runtime_kv:")
            && namespace
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '-' | '_')))
}
