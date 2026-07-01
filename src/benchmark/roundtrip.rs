use crate::clean_room_audit::{
    CleanRoomAuditRecord, CleanRoomAuditReport, CleanRoomLicenseClass, CleanRoomMaterialKind,
};
use crate::development_pollution::{
    DevelopmentEvidenceAdmissionDecision, DevelopmentEvidenceUseSurface, DevelopmentPollutionEvent,
    DevelopmentPollutionLifecycleStage, admit_development_evidence_for_current_use,
    classify_development_pollution_event, gate_development_evidence_surface,
};
use crate::drift::{DriftReport, DriftSeverity};
use crate::hardware::{DeviceClass, RuntimeAdapterHint};
use crate::hierarchy::TaskProfile;
use crate::memory_admission::{MemoryAdmissionInput, MemoryAdmissionPreview};
use crate::privacy_redaction::contains_private_or_executable_marker;
use crate::process_reward::ProcessRewardReport;
use crate::reflection::ReflectionReport;
use crate::tenant_scope::{
    TenantAccessKind, TenantIsolationGate, TenantResourceLane, TenantScope, TenantScopedKey,
};

use super::display::{option_f32_display, option_str_display};

#[derive(Debug, Clone)]
pub struct PersistentRoundtripInput {
    pub first_stored_memory: bool,
    pub first_runtime_kv_stored: usize,
    pub first_runtime_kv_namespace_preserved: bool,
    pub second_used_memories: usize,
    pub second_used_runtime_kv_memory: bool,
    pub second_used_experiences: usize,
    pub second_imported_runtime_kv_blocks: usize,
    pub second_imported_runtime_kv_from_namespace: bool,
    pub second_runtime_adapter_observations: usize,
    pub second_runtime_adapter_best_score: Option<f32>,
    pub second_runtime_adapter_best_adapter: Option<String>,
    pub second_runtime_selected_adapter: Option<String>,
    pub second_compute_budget_saved_tokens: usize,
    pub second_compute_budget_avoided_tokens: usize,
    pub second_compute_budget_kv_lookups_skipped: usize,
    pub second_quality: f32,
    pub first_drift_severity: DriftSeverity,
    pub second_drift_severity: DriftSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentRoundtripReport {
    pub passed: bool,
    pub first_stored_memory: bool,
    pub first_runtime_kv_stored: usize,
    pub first_runtime_kv_namespace_preserved: bool,
    pub second_used_memories: usize,
    pub second_used_runtime_kv_memory: bool,
    pub second_used_experiences: usize,
    pub second_imported_runtime_kv_blocks: usize,
    pub second_imported_runtime_kv_from_namespace: bool,
    pub second_runtime_adapter_observations: usize,
    pub second_runtime_adapter_best_score: Option<f32>,
    pub second_runtime_adapter_best_adapter: Option<String>,
    pub second_runtime_selected_adapter: Option<String>,
    pub second_compute_budget_saved_tokens: usize,
    pub second_compute_budget_avoided_tokens: usize,
    pub second_compute_budget_kv_lookups_skipped: usize,
    pub second_quality: f32,
    pub first_drift_severity: DriftSeverity,
    pub second_drift_severity: DriftSeverity,
    pub negative_gate_evidence: PersistentRoundtripNegativeGateEvidence,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistentRoundtripNegativeGateEvidence {
    pub unauthorized_write_allowed: bool,
    pub polluted_evidence_blocked: bool,
    pub polluted_evidence_quarantined: bool,
    pub bad_candidate_held_or_rolled_back: bool,
    pub rollback_anchor_present: bool,
    pub tenant_scope_write_denied: bool,
    pub single_tenant_preview: bool,
    pub provenance_license_redaction_passed: bool,
    pub digest_only: bool,
}

impl PersistentRoundtripNegativeGateEvidence {
    pub fn passed(&self) -> bool {
        !self.unauthorized_write_allowed
            && (self.polluted_evidence_blocked || self.polluted_evidence_quarantined)
            && self.bad_candidate_held_or_rolled_back
            && self.rollback_anchor_present
            && self.tenant_scope_write_denied
            && self.single_tenant_preview
            && self.provenance_license_redaction_passed
            && self.digest_only
    }

    pub fn failure_reasons(&self) -> Vec<String> {
        let mut reasons = Vec::new();
        if self.unauthorized_write_allowed {
            reasons.push("negative_gate_unauthorized_write_allowed".to_owned());
        }
        if !self.polluted_evidence_blocked && !self.polluted_evidence_quarantined {
            reasons.push("negative_gate_polluted_evidence_not_blocked_or_quarantined".to_owned());
        }
        if !self.bad_candidate_held_or_rolled_back {
            reasons.push("negative_gate_bad_candidate_not_held_or_rolled_back".to_owned());
        }
        if !self.rollback_anchor_present {
            reasons.push("negative_gate_rollback_anchor_missing".to_owned());
        }
        if !self.tenant_scope_write_denied {
            reasons.push("negative_gate_tenant_scope_write_not_denied".to_owned());
        }
        if !self.single_tenant_preview {
            reasons.push("negative_gate_single_tenant_preview_missing".to_owned());
        }
        if !self.provenance_license_redaction_passed {
            reasons.push("negative_gate_provenance_license_redaction_not_passed".to_owned());
        }
        if !self.digest_only {
            reasons.push("negative_gate_not_digest_only".to_owned());
        }
        reasons
    }
}

pub fn issue30_roundtrip_negative_gate_evidence() -> PersistentRoundtripNegativeGateEvidence {
    let unauthorized_memory_write_allowed =
        issue30_unauthorized_memory_write_allowed_for_roundtrip();
    let finding = classify_development_pollution_event(
        &DevelopmentPollutionEvent::new(
            "issue-30-roundtrip-polluted-evidence",
            "roundtrip_negative_gate",
            "digest-only-polluted-evidence",
            "development_evidence_contamination",
        )
        .with_hit_count(2),
    );
    let admission = admit_development_evidence_for_current_use(&finding);
    let benchmark_gate =
        gate_development_evidence_surface(&admission, DevelopmentEvidenceUseSurface::Benchmark);
    let durable_gate =
        gate_development_evidence_surface(&admission, DevelopmentEvidenceUseSurface::DurableMemory);
    let digest_gate =
        gate_development_evidence_surface(&admission, DevelopmentEvidenceUseSurface::DigestMarker);

    let local_scope = TenantScope::local_single_user();
    let rollback_anchor = local_scope.scoped_key(
        TenantResourceLane::SessionState,
        "issue-30-roundtrip-rollback-anchor",
    );
    let foreign_key = TenantScope::new("tenant-b", "default", "interactive").scoped_key(
        TenantResourceLane::SelfEvolvingMemory,
        "issue-30-cross-scope-write",
    );
    let tenant_scope_write_denied = !TenantIsolationGate::new()
        .check_key_access(&local_scope, &foreign_key, TenantAccessKind::Write)
        .allowed;
    let clean_room = CleanRoomAuditReport::from_records(&[CleanRoomAuditRecord {
        stable_id: "issue-30-roundtrip-negative-gate",
        source_id: "rust-norion:roundtrip-negative-gate",
        source_name: "rust-norion generated roundtrip negative gate fixture",
        license_spdx: Some("MIT"),
        license_class: CleanRoomLicenseClass::ProjectOwned,
        material_kind: CleanRoomMaterialKind::GeneratedFixture,
        target_issue: "#30",
        target_module: "src/benchmark/roundtrip.rs",
        copied_external_material: false,
        vendored_external_source: false,
        generated_from_external_source: false,
        carries_raw_private_payload: false,
        attribution_recorded: true,
        scoped_port_plan_recorded: true,
        maintainer_review_recorded: true,
        norion_owned_reimplementation: true,
        evidence_ref: "roundtrip-negative-gate-fixture-v1",
    }]);
    let clean_room_digest_only = clean_room.evidence_packet_lines.iter().all(|line| {
        line.contains("digest=redaction-digest:") && !contains_private_or_executable_marker(line)
    });

    PersistentRoundtripNegativeGateEvidence {
        unauthorized_write_allowed: unauthorized_memory_write_allowed,
        polluted_evidence_blocked: !benchmark_gate.allowed && !durable_gate.allowed,
        polluted_evidence_quarantined: admission.decision
            == DevelopmentEvidenceAdmissionDecision::DigestOnlyQuarantine
            && finding.lifecycle_stage == DevelopmentPollutionLifecycleStage::Quarantine
            && digest_gate.allowed,
        bad_candidate_held_or_rolled_back: !benchmark_gate.allowed
            && finding.lifecycle_stage == DevelopmentPollutionLifecycleStage::Quarantine,
        rollback_anchor_present: TenantScopedKey::parse(rollback_anchor.as_str()).is_some(),
        tenant_scope_write_denied,
        single_tenant_preview: local_scope == TenantScope::local_single_user(),
        provenance_license_redaction_passed: clean_room.passed() && clean_room_digest_only,
        digest_only: finding.source_digest.starts_with("redaction-digest:")
            && digest_gate.allowed
            && clean_room_digest_only,
    }
}

fn issue30_unauthorized_memory_write_allowed_for_roundtrip() -> bool {
    let report = ReflectionReport {
        quality: 0.82,
        contradictions: Vec::new(),
        issues: Vec::new(),
        revision_actions: Vec::new(),
        revision_passes: 0,
        revised_answer: String::new(),
        store_as_memory: true,
        lesson: "issue-30 roundtrip negative gate preview".to_owned(),
    };
    let process_reward = ProcessRewardReport::default();
    let drift_report = DriftReport {
        severity: DriftSeverity::Stable,
        allow_memory_write: true,
        allow_runtime_kv_write: true,
        penalize_used_memory: false,
        rollback_adaptive: false,
        notes: Vec::new(),
    };
    let preview = MemoryAdmissionPreview::from_feedback(MemoryAdmissionInput {
        prompt: "issue-30 unauthorized durable write negative gate",
        profile: TaskProfile::Coding,
        report: &report,
        process_reward: &process_reward,
        drift_report: &drift_report,
        stored_memory: true,
        gist_records: 0,
        stored_gist_memories: 0,
        imported_runtime_kv_blocks: 0,
        exported_runtime_kv_blocks: 0,
        stored_runtime_kv_memories: 0,
        weak_runtime_kv_imports_skipped: 0,
        runtime_kv_hold: false,
        runtime_kv_influence: None,
        budget_limited_runtime_kv_imports_skipped: 0,
        runtime_kv_segments_included: 0,
        runtime_kv_segments_skipped: 0,
        runtime_kv_segments_rejected: 0,
        used_memories: 1,
        memory_feedback_updates: 0,
        runtime_adapter_observations: 0,
        runtime_adapter_current_signal: false,
        runtime_adapter_selection_mismatch: false,
        runtime_adapter_best_score: None,
        runtime_adapter_best_reward: None,
        runtime_adapter_best_quality: None,
        toolsmith_blueprints: 0,
        toolsmith_ready: 0,
        toolsmith_held: 0,
        toolsmith_rejected: 0,
        toolsmith_gate_passed: false,
    });

    preview.candidate_count() == 0
        || !preview.ledger_plan.is_read_only_preview()
        || preview.ledger_plan.write_allowed
}

impl PersistentRoundtripReport {
    pub fn evaluate(input: PersistentRoundtripInput) -> Self {
        let mut failures = Vec::new();
        let negative_gate_evidence = issue30_roundtrip_negative_gate_evidence();
        let second_runtime_adapter_best_adapter = input
            .second_runtime_adapter_best_adapter
            .as_deref()
            .and_then(RuntimeAdapterHint::canonical_name)
            .map(str::to_owned);
        let second_runtime_selected_adapter = input
            .second_runtime_selected_adapter
            .as_deref()
            .and_then(RuntimeAdapterHint::canonical_name)
            .map(str::to_owned);

        if !input.first_stored_memory {
            failures.push("first run did not store durable memory".to_owned());
        }
        if input.first_runtime_kv_stored == 0 {
            failures.push("first run did not store runtime KV memory".to_owned());
        }
        if !input.first_runtime_kv_namespace_preserved {
            failures.push("first run stored runtime KV without runtime_kv namespace".to_owned());
        }
        if input.second_used_memories == 0 {
            failures.push("second run did not retrieve persisted memory".to_owned());
        }
        if !input.second_used_runtime_kv_memory {
            failures.push("second run did not retrieve persisted runtime KV memory".to_owned());
        }
        if input.second_used_experiences == 0 {
            failures.push("second run did not retrieve persisted experience".to_owned());
        }
        if input.second_imported_runtime_kv_blocks == 0 {
            failures.push("second run did not import persisted runtime KV".to_owned());
        }
        if !input.second_imported_runtime_kv_from_namespace {
            failures.push(
                "second run did not import KV reconstructed from persisted runtime_kv namespace"
                    .to_owned(),
            );
        }
        if input.second_runtime_adapter_observations == 0 {
            failures.push(
                "second run did not derive runtime adapter observations from persisted experience"
                    .to_owned(),
            );
        }
        if input
            .second_runtime_adapter_best_score
            .filter(|score| score.is_finite() && *score > 0.0)
            .is_none()
        {
            failures.push(
                "second run did not expose a positive runtime adapter observation score".to_owned(),
            );
        }
        match (
            second_runtime_adapter_best_adapter.as_deref(),
            second_runtime_selected_adapter.as_deref(),
        ) {
            (Some(best_adapter), Some(selected_adapter)) if best_adapter == selected_adapter => {}
            (None, _) => failures.push(
                "second run did not expose a trusted best runtime adapter observation".to_owned(),
            ),
            (_, None) => {
                failures.push("second run did not select a trusted runtime adapter".to_owned())
            }
            (Some(best_adapter), Some(selected_adapter)) => failures.push(format!(
                "second run selected adapter {selected_adapter} but best persisted observation was {best_adapter}"
            )),
        }
        if input.second_compute_budget_avoided_tokens == 0 {
            failures.push("second run did not report compute budget avoided tokens".to_owned());
        }
        if input.second_quality < 0.50 {
            failures.push(format!(
                "second_quality {:.3} below minimum 0.500",
                input.second_quality
            ));
        }
        if input.first_drift_severity == DriftSeverity::Rollback {
            failures.push("first run triggered drift rollback".to_owned());
        }
        if matches!(
            input.second_drift_severity,
            DriftSeverity::Block | DriftSeverity::Rollback
        ) {
            failures.push(format!(
                "second run drift severity was {}",
                input.second_drift_severity.as_str()
            ));
        }
        failures.extend(negative_gate_evidence.failure_reasons());

        Self {
            passed: failures.is_empty(),
            first_stored_memory: input.first_stored_memory,
            first_runtime_kv_stored: input.first_runtime_kv_stored,
            first_runtime_kv_namespace_preserved: input.first_runtime_kv_namespace_preserved,
            second_used_memories: input.second_used_memories,
            second_used_runtime_kv_memory: input.second_used_runtime_kv_memory,
            second_used_experiences: input.second_used_experiences,
            second_imported_runtime_kv_blocks: input.second_imported_runtime_kv_blocks,
            second_imported_runtime_kv_from_namespace: input
                .second_imported_runtime_kv_from_namespace,
            second_runtime_adapter_observations: input.second_runtime_adapter_observations,
            second_runtime_adapter_best_score: input.second_runtime_adapter_best_score,
            second_runtime_adapter_best_adapter,
            second_runtime_selected_adapter,
            second_compute_budget_saved_tokens: input.second_compute_budget_saved_tokens,
            second_compute_budget_avoided_tokens: input.second_compute_budget_avoided_tokens,
            second_compute_budget_kv_lookups_skipped: input
                .second_compute_budget_kv_lookups_skipped,
            second_quality: input.second_quality,
            first_drift_severity: input.first_drift_severity,
            second_drift_severity: input.second_drift_severity,
            negative_gate_evidence,
            failures,
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "persistent_roundtrip: passed={} first_stored_memory={} first_runtime_kv_stored={} first_runtime_kv_namespace_preserved={} second_used_memories={} second_used_runtime_kv_memory={} second_used_experiences={} second_imported_runtime_kv_blocks={} second_imported_runtime_kv_from_namespace={} second_runtime_adapter_observations={} second_runtime_adapter_best_score={} second_runtime_adapter_best_adapter={} second_runtime_selected_adapter={} second_compute_budget_saved_tokens={} second_compute_budget_avoided_tokens={} second_compute_budget_kv_lookups_skipped={} negative_unauthorized_write_allowed={} negative_polluted_evidence_blocked={} negative_polluted_evidence_quarantined={} negative_bad_candidate_held_or_rolled_back={} negative_rollback_anchor_present={} negative_tenant_scope_write_denied={} negative_single_tenant_preview={} negative_provenance_license_redaction_passed={} negative_digest_only={} second_quality={:.3} first_drift={} second_drift={} failures={}",
            self.passed,
            self.first_stored_memory,
            self.first_runtime_kv_stored,
            self.first_runtime_kv_namespace_preserved,
            self.second_used_memories,
            self.second_used_runtime_kv_memory,
            self.second_used_experiences,
            self.second_imported_runtime_kv_blocks,
            self.second_imported_runtime_kv_from_namespace,
            self.second_runtime_adapter_observations,
            option_f32_display(self.second_runtime_adapter_best_score),
            option_str_display(self.second_runtime_adapter_best_adapter.as_deref()),
            option_str_display(self.second_runtime_selected_adapter.as_deref()),
            self.second_compute_budget_saved_tokens,
            self.second_compute_budget_avoided_tokens,
            self.second_compute_budget_kv_lookups_skipped,
            self.negative_gate_evidence.unauthorized_write_allowed,
            self.negative_gate_evidence.polluted_evidence_blocked,
            self.negative_gate_evidence.polluted_evidence_quarantined,
            self.negative_gate_evidence
                .bad_candidate_held_or_rolled_back,
            self.negative_gate_evidence.rollback_anchor_present,
            self.negative_gate_evidence.tenant_scope_write_denied,
            self.negative_gate_evidence.single_tenant_preview,
            self.negative_gate_evidence
                .provenance_license_redaction_passed,
            self.negative_gate_evidence.digest_only,
            self.second_quality,
            self.first_drift_severity.as_str(),
            self.second_drift_severity.as_str(),
            self.failures.len()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentRoundtripDeviceReport {
    pub device: DeviceClass,
    pub report: PersistentRoundtripReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PersistentRoundtripMatrixReport {
    pub passed: bool,
    pub device_reports: Vec<PersistentRoundtripDeviceReport>,
    pub failures: Vec<String>,
}

impl PersistentRoundtripMatrixReport {
    pub fn evaluate(device_reports: Vec<PersistentRoundtripDeviceReport>) -> Self {
        let mut failures = Vec::new();

        if device_reports.is_empty() {
            failures.push("no persistent roundtrip device reports were recorded".to_owned());
        }

        let missing = missing_persistent_roundtrip_devices(&device_reports);
        if !missing.is_empty() {
            let missing_devices = missing
                .iter()
                .map(|device| device.as_str())
                .collect::<Vec<_>>()
                .join("+");
            failures.push(format!(
                "persistent_roundtrip_devices {} below expected {} missing={}",
                explicit_persistent_roundtrip_devices(&device_reports),
                DeviceClass::explicit_profiles().len(),
                missing_devices
            ));
        }

        for device_report in &device_reports {
            if !device_report.report.passed {
                failures.push(format!(
                    "device {} persistent roundtrip failed with {} failures",
                    device_report.device.as_str(),
                    device_report.report.failures.len()
                ));
            }
        }

        Self {
            passed: failures.is_empty(),
            device_reports,
            failures,
        }
    }

    pub fn covered_devices(&self) -> usize {
        explicit_persistent_roundtrip_devices(&self.device_reports)
    }

    pub fn missing_devices(&self) -> Vec<DeviceClass> {
        missing_persistent_roundtrip_devices(&self.device_reports)
    }

    pub fn failed_devices(&self) -> Vec<DeviceClass> {
        self.device_reports
            .iter()
            .filter(|device_report| !device_report.report.passed)
            .map(|device_report| device_report.device)
            .collect()
    }

    pub fn second_compute_budget_saved_tokens(&self) -> usize {
        self.device_reports
            .iter()
            .map(|device_report| device_report.report.second_compute_budget_saved_tokens)
            .sum()
    }

    pub fn second_compute_budget_avoided_tokens(&self) -> usize {
        self.device_reports
            .iter()
            .map(|device_report| device_report.report.second_compute_budget_avoided_tokens)
            .sum()
    }

    pub fn second_compute_budget_kv_lookups_skipped(&self) -> usize {
        self.device_reports
            .iter()
            .map(|device_report| {
                device_report
                    .report
                    .second_compute_budget_kv_lookups_skipped
            })
            .sum()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "persistent_roundtrip_matrix: passed={} devices={} expected_devices={} failed_devices={} second_compute_budget_saved_tokens={} second_compute_budget_avoided_tokens={} second_compute_budget_kv_lookups_skipped={} failures={}",
            self.passed,
            self.covered_devices(),
            DeviceClass::explicit_profiles().len(),
            self.failed_devices().len(),
            self.second_compute_budget_saved_tokens(),
            self.second_compute_budget_avoided_tokens(),
            self.second_compute_budget_kv_lookups_skipped(),
            self.failures.len()
        )
    }
}

fn explicit_persistent_roundtrip_devices(
    device_reports: &[PersistentRoundtripDeviceReport],
) -> usize {
    DeviceClass::explicit_profiles()
        .iter()
        .filter(|device| {
            device_reports
                .iter()
                .any(|device_report| device_report.device == **device)
        })
        .count()
}

fn missing_persistent_roundtrip_devices(
    device_reports: &[PersistentRoundtripDeviceReport],
) -> Vec<DeviceClass> {
    DeviceClass::explicit_profiles()
        .iter()
        .copied()
        .filter(|device| {
            !device_reports
                .iter()
                .any(|device_report| device_report.device == *device)
        })
        .collect()
}
