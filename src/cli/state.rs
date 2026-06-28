use std::fs;
use std::path::{Path, PathBuf};

use rust_norion::{
    DeviceClass, NoironEngine, StateInspectionDeviceGateReport, StateInspectionGateReport,
    StateInspectionMatrixGateReport, StateInspectionReport, stable_redaction_digest,
};

use crate::Args;
use crate::engine_config::configure_engine;

const RUNTIME_STATE_DIR_PREFIX: &str = "rust-norion-v";
const LEGACY_RUNTIME_STATE_ARTIFACTS: [&str; 5] = [
    "noiron-memory.tsv",
    "noiron-memory.ndkv",
    "noiron-experience.ndkv",
    "noiron-experience.self-evolving-memory.tsv",
    "noiron-adaptive.ndkv",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeStateBucketSummary {
    pub(crate) current: PathBuf,
    pub(crate) in_current_bucket: bool,
    pub(crate) legacy_root_artifacts: usize,
    pub(crate) stale_version_buckets: usize,
}

impl RuntimeStateBucketSummary {
    pub(crate) fn summary_line(&self) -> String {
        format!(
            "runtime_state_bucket current={} in_current_bucket={} legacy_root_artifacts={} stale_version_buckets={}",
            self.current.display(),
            self.in_current_bucket,
            self.legacy_root_artifacts,
            self.stale_version_buckets
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeStateRetireReport {
    pub(crate) applied: bool,
    pub(crate) current: PathBuf,
    pub(crate) retire_root: PathBuf,
    pub(crate) candidates: Vec<PathBuf>,
    pub(crate) retired_to: Vec<PathBuf>,
    pub(crate) lifecycle_records: Vec<RuntimeStateRetireLifecycleRecord>,
}

impl RuntimeStateRetireReport {
    fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    fn retired_count(&self) -> usize {
        self.retired_to.len()
    }

    fn lifecycle_record_count(&self) -> usize {
        self.lifecycle_records.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeStateRetireCandidate {
    source: PathBuf,
    destination: PathBuf,
    kind: RuntimeStateRetireCandidateKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeStateRetireCandidateKind {
    LegacyRootArtifact,
    StaleVersionBucket,
}

impl RuntimeStateRetireCandidateKind {
    fn reason_code(self) -> &'static str {
        match self {
            Self::LegacyRootArtifact => "legacy_root_runtime_state",
            Self::StaleVersionBucket => "stale_runtime_state_bucket",
        }
    }

    fn parent_lineage(self) -> &'static str {
        match self {
            Self::LegacyRootArtifact => "runtime_state:legacy_root",
            Self::StaleVersionBucket => "runtime_state:version_bucket",
        }
    }

    fn affected_scope(self) -> &'static str {
        match self {
            Self::LegacyRootArtifact => "runtime_state_legacy_root_artifact",
            Self::StaleVersionBucket => "runtime_state_stale_version_bucket",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeStateRetireLifecycleRecord {
    pub(crate) state: &'static str,
    pub(crate) source: PathBuf,
    pub(crate) destination: PathBuf,
    pub(crate) reason_code: &'static str,
    pub(crate) source_digest: String,
    pub(crate) parent_lineage: &'static str,
    pub(crate) rollback_anchor: String,
    pub(crate) affected_scope: &'static str,
    pub(crate) readmission_gate: &'static str,
    pub(crate) operator_approval_required: bool,
}

impl RuntimeStateRetireLifecycleRecord {
    fn evidence_line(&self) -> String {
        format!(
            "runtime_state_lifecycle state={} source={} destination={} reason_code={} source_digest={} parent_lineage={} rollback_anchor={} affected_scope={} readmission_gate={} operator_approval_required={}",
            self.state,
            self.source.display(),
            self.destination.display(),
            self.reason_code,
            self.source_digest,
            self.parent_lineage,
            self.rollback_anchor,
            self.affected_scope,
            self.readmission_gate,
            self.operator_approval_required
        )
    }
}

impl RuntimeStateRetireCandidate {
    fn lifecycle_record(
        &self,
        destination: PathBuf,
        state: &'static str,
    ) -> RuntimeStateRetireLifecycleRecord {
        let source_text = self.source.display().to_string();
        let destination_text = destination.display().to_string();
        RuntimeStateRetireLifecycleRecord {
            state,
            source: self.source.clone(),
            destination,
            reason_code: self.kind.reason_code(),
            source_digest: stable_redaction_digest([
                "runtime-state-retire-source",
                source_text.as_str(),
            ]),
            parent_lineage: self.kind.parent_lineage(),
            rollback_anchor: stable_redaction_digest([
                "runtime-state-retire-rollback",
                source_text.as_str(),
                destination_text.as_str(),
            ]),
            affected_scope: self.kind.affected_scope(),
            readmission_gate: "manual_restore_to_current_bucket_and_state_inspection_gate",
            operator_approval_required: true,
        }
    }
}

pub(crate) fn run_state_inspection(args: &Args) -> std::io::Result<StateInspectionReport> {
    let mut engine = NoironEngine::load_full_state(
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    )?;
    configure_engine(&mut engine, args);
    Ok(StateInspectionReport::from_engine(
        &engine,
        args.inspect_limit,
    ))
}

pub(crate) fn run_state_inspection_all_devices(
    args: &Args,
) -> std::io::Result<StateInspectionMatrixGateReport> {
    let mut device_reports = Vec::new();
    let gate = args.state_inspection_gate();
    let matrix_gate = args.state_inspection_matrix_gate();

    for device in DeviceClass::explicit_profiles() {
        let device_args = args.for_inspect_device(*device);
        let mut state_file_failures = Vec::new();
        if !device_args.memory_path.exists() {
            state_file_failures.push(format!(
                "memory file missing: {}",
                device_args.memory_path.display()
            ));
        }
        if !device_args.experience_path.exists() {
            state_file_failures.push(format!(
                "experience file missing: {}",
                device_args.experience_path.display()
            ));
        }
        if !device_args.adaptive_path.exists() {
            state_file_failures.push(format!(
                "adaptive file missing: {}",
                device_args.adaptive_path.display()
            ));
        }
        let mut engine = NoironEngine::load_full_state(
            &device_args.memory_path,
            &device_args.experience_path,
            &device_args.adaptive_path,
        )?;
        configure_engine(&mut engine, &device_args);
        let report = StateInspectionReport::from_engine(&engine, device_args.inspect_limit);
        let mut gate_report = report.evaluate(&gate);
        gate_report.failures.extend(state_file_failures);
        gate_report.passed = gate_report.failures.is_empty();
        device_reports.push(StateInspectionDeviceGateReport::from_report(
            *device,
            &report,
            gate_report,
        ));
    }

    Ok(StateInspectionMatrixGateReport::evaluate_with_gate(
        device_reports,
        &matrix_gate,
    ))
}

pub(crate) fn device_scoped_path(path: &Path, device: DeviceClass) -> PathBuf {
    let parent = path.parent();
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("state");
    let extension = path.extension().and_then(|value| value.to_str());
    let file_name = match extension {
        Some(extension) if !extension.is_empty() => {
            format!("{}.{}.{}", stem, device.as_str(), extension)
        }
        _ => format!("{}.{}", stem, device.as_str()),
    };

    parent
        .map(|parent| parent.join(&file_name))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

pub(crate) fn print_state_inspection_report(args: &Args, report: &StateInspectionReport) {
    println!("Noiron state inspection");
    println!("memory_file: {}", args.memory_path.display());
    println!("experience_file: {}", args.experience_path.display());
    println!("adaptive_file: {}", args.adaptive_path.display());
    println!("{}", runtime_state_bucket(args).summary_line());
    println!("{}", report.summary_line());
    println!(
        "profile_observations: general={} coding={} writing={} long={}",
        report.profile_observations.general,
        report.profile_observations.coding,
        report.profile_observations.writing,
        report.profile_observations.long_document
    );
    println!(
        "profile_hierarchy_observations: general={} coding={} writing={} long={}",
        report.profile_hierarchy_observations.general,
        report.profile_hierarchy_observations.coding,
        report.profile_hierarchy_observations.writing,
        report.profile_hierarchy_observations.long_document
    );
    println!(
        "memory_retention_policy: stale_after={} decay_rate={:.3} remove_below_strength={:.3} remove_after_failures={}",
        report.memory_retention_policy.stale_after,
        report.memory_retention_policy.decay_rate,
        report.memory_retention_policy.remove_below_strength,
        report.memory_retention_policy.remove_after_failures
    );
    println!(
        "memory_compaction_policy: similarity_threshold={:.3} max_candidates={} max_merges={}",
        report.memory_compaction_policy.similarity_threshold,
        report.memory_compaction_policy.max_candidates,
        report.memory_compaction_policy.max_merges
    );
    if report.memory_vector_dimensions.is_empty() {
        println!("memory_vector_dimensions: none");
    } else {
        let dimensions = report
            .memory_vector_dimensions
            .iter()
            .map(|bucket| format!("{}:{}", bucket.dimensions, bucket.count))
            .collect::<Vec<_>>()
            .join(" ");
        println!("memory_vector_dimensions: {dimensions}");
    }
    if report.runtime_kv_vector_dimensions.is_empty() {
        println!("runtime_kv_vector_dimensions: none");
    } else {
        let dimensions = report
            .runtime_kv_vector_dimensions
            .iter()
            .map(|bucket| format!("{}:{}", bucket.dimensions, bucket.count))
            .collect::<Vec<_>>()
            .join(" ");
        println!("runtime_kv_vector_dimensions: {dimensions}");
    }

    println!("top_memories:");
    if report.top_memories.is_empty() {
        println!("  none");
    } else {
        for memory in &report.top_memories {
            println!(
                "  id={} dims={} strength={:.3} hits={} failures={} last_score={:.3} key={}",
                memory.id,
                memory.vector_dimensions,
                memory.strength,
                memory.hits,
                memory.failures,
                memory.last_score,
                memory.key
            );
        }
    }

    println!("top_runtime_kv_memories:");
    if report.top_runtime_kv_memories.is_empty() {
        println!("  none");
    } else {
        for memory in &report.top_runtime_kv_memories {
            println!(
                "  id={} dims={} strength={:.3} hits={} failures={} last_score={:.3} key={}",
                memory.id,
                memory.vector_dimensions,
                memory.strength,
                memory.hits,
                memory.failures,
                memory.last_score,
                memory.key
            );
        }
    }

    println!("top_experiences:");
    if report.top_experiences.is_empty() {
        println!("  none");
    } else {
        for experience in &report.top_experiences {
            println!(
                "  id={} profile={:?} quality={:.3} reward={:.3} action={} runtime_model={} adapter={} layers={} hidden={} local_window={} forward_energy={} kv_influence={} runtime_kv_imported={} runtime_kv_exported={} recursive_runtime_calls={} runtime_errors={} runtime_timeouts={} runtime_error_message_chars={} live_memory_feedback_updates={} live_memory_feedback_reinforced={} live_memory_feedback_penalized={} live_memory_feedback_applied={} live_memory_feedback_removed={} live_memory_feedback_missing={} live_memory_feedback_strength_delta={:.6} live_memory_feedback_detail={} rust_check_passed={} rust_check_failed={} rust_check_diagnostic_chars={} pool_dispatch_items={} pool_dispatch_roles={} pool_dispatch_forwarded={} pool_dispatch_clamped={} pool_dispatch_low_priority={} reflection_issues={} critical={} revision_actions={} lesson={}",
                experience.id,
                experience.profile,
                experience.quality,
                experience.process_reward,
                experience.reward_action.as_str(),
                option_text(experience.runtime_model_id.as_deref()),
                option_text(experience.runtime_selected_adapter.as_deref()),
                experience.runtime_layer_count,
                experience.runtime_hidden_size,
                experience.runtime_local_window_tokens,
                option_f32_text(experience.runtime_forward_energy),
                option_f32_text(experience.runtime_kv_influence),
                experience.runtime_imported_kv_blocks,
                experience.runtime_exported_kv_blocks,
                option_usize_text(experience.recursive_runtime_calls),
                experience.runtime_errors,
                experience.runtime_timeouts,
                experience.runtime_error_message_chars,
                experience.live_memory_feedback_updates,
                experience.live_memory_feedback_reinforced,
                experience.live_memory_feedback_penalized,
                experience.live_memory_feedback_applied,
                experience.live_memory_feedback_removed,
                experience.live_memory_feedback_missing,
                experience.live_memory_feedback_strength_delta,
                experience.live_memory_feedback_detail,
                experience.rust_check_passed,
                experience.rust_check_failed,
                experience.rust_check_diagnostic_chars,
                experience.pool_dispatch_items,
                string_list_text(&experience.pool_dispatch_selected_roles),
                experience.pool_dispatch_forwarded,
                experience.pool_dispatch_clamped,
                experience.pool_dispatch_low_priority,
                experience.reflection_issues,
                experience.critical_reflection_issues,
                experience.revision_actions,
                experience.lesson
            );
        }
    }

    println!(
        "experience_hygiene: findings={} watch={} quarantine_candidates={} legacy_metadata_lessons={} legacy_metadata_without_clean_gist={}",
        report.experience_hygiene_finding_count,
        report.experience_hygiene_watch_count,
        report.experience_hygiene_quarantine_candidate_count,
        report.experience_hygiene_legacy_metadata_lesson_count,
        report.experience_hygiene_legacy_metadata_without_clean_gist_count
    );
    println!(
        "experience_repair: repairable_legacy_metadata_lessons={} repairable_index_records={} projected_findings={} projected_watch={} projected_quarantine_candidates={} projected_legacy_metadata_lessons={} projected_legacy_metadata_without_clean_gist={} skipped_quarantine_candidates={} skipped_missing_clean_gist={}",
        report.experience_repairable_legacy_metadata_lesson_count,
        report.experience_repairable_index_record_count,
        report.experience_repair_projected_hygiene_finding_count,
        report.experience_repair_projected_hygiene_watch_count,
        report.experience_repair_projected_hygiene_quarantine_candidate_count,
        report.experience_repair_projected_legacy_metadata_lesson_count,
        report.experience_repair_projected_legacy_metadata_without_clean_gist_count,
        report.experience_repair_skipped_quarantine_candidate_count,
        report.experience_repair_skipped_missing_clean_gist_count
    );
    if report.experience_hygiene_findings.is_empty() {
        println!("  none");
    } else {
        for finding in &report.experience_hygiene_findings {
            println!(
                "  id={} severity={} reason={} markers={} prompt={} lesson={}",
                finding.experience_id,
                finding.severity.as_str(),
                finding.reason,
                finding.markers.join(","),
                finding.prompt_preview,
                finding.lesson_preview
            );
        }
    }

    println!(
        "experience_index: compacted_records={} noisy_records={} duplicate_outputs={} max_noise_penalty={:.6} quality_score={:.6} retrieval_ready={} risk_level={} listed={}",
        report.experience_index_compacted_record_count,
        report.experience_index_noisy_record_count,
        report.experience_index_duplicate_output_count,
        report.experience_index_max_noise_penalty,
        report.experience_index_quality_score,
        report.experience_index_retrieval_ready,
        report.experience_index_risk_level,
        report.experience_index_findings.len()
    );
    if report.experience_index_findings.is_empty() {
        println!("  none");
    } else {
        for finding in &report.experience_index_findings {
            println!(
                "  id={} reason={} compacted={} noise_penalty={:.6} duplicate_of={} prompt_chars={} lesson_chars={} prompt={} lesson={}",
                finding.experience_id,
                finding.reason,
                finding.compacted,
                finding.noise_penalty,
                finding
                    .duplicate_of
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "none".to_owned()),
                finding.prompt_chars,
                finding.lesson_chars,
                finding.prompt_preview,
                finding.lesson_preview
            );
        }
    }
}

pub(crate) fn run_runtime_state_retire(args: &Args) -> std::io::Result<RuntimeStateRetireReport> {
    retire_runtime_state_at(
        Path::new("."),
        Path::new("state"),
        &[
            &args.memory_path,
            &args.experience_path,
            &args.adaptive_path,
        ],
        args.runtime_state_retire_apply,
    )
}

pub(crate) fn print_runtime_state_retire_report(report: &RuntimeStateRetireReport) {
    println!("Noiron runtime state retire");
    println!(
        "runtime_state_retire applied={} candidates={} retired={}",
        report.applied,
        report.candidate_count(),
        report.retired_count()
    );
    println!("current_bucket: {}", report.current.display());
    println!("retire_root: {}", report.retire_root.display());
    println!(
        "runtime_state_lifecycle_records: {}",
        report.lifecycle_record_count()
    );
    for candidate in &report.candidates {
        println!("retire_candidate: {}", candidate.display());
    }
    for retired in &report.retired_to {
        println!("retired_to: {}", retired.display());
    }
    for record in &report.lifecycle_records {
        println!("{}", record.evidence_line());
    }
}

pub(crate) fn runtime_state_bucket(args: &Args) -> RuntimeStateBucketSummary {
    runtime_state_bucket_at(args, Path::new("."), Path::new("state"))
}

fn runtime_state_bucket_at(
    args: &Args,
    project_root: &Path,
    state_root: &Path,
) -> RuntimeStateBucketSummary {
    let current = current_runtime_state_dir(state_root);
    let in_current_bucket = [
        &args.memory_path,
        &args.experience_path,
        &args.adaptive_path,
    ]
    .iter()
    .all(|path| runtime_state_path_parent_matches_current_bucket(path, &current, project_root));
    RuntimeStateBucketSummary {
        current,
        in_current_bucket,
        legacy_root_artifacts: legacy_runtime_state_artifact_count(project_root),
        stale_version_buckets: stale_runtime_state_bucket_count(state_root),
    }
}

fn current_runtime_state_dir(state_root: &Path) -> PathBuf {
    state_root.join(format!(
        "{RUNTIME_STATE_DIR_PREFIX}{}",
        env!("CARGO_PKG_VERSION")
    ))
}

fn runtime_state_path_parent_matches_current_bucket(
    path: &Path,
    current: &Path,
    project_root: &Path,
) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    normalized_runtime_state_path(parent, project_root)
        == normalized_runtime_state_path(current, project_root)
}

fn normalized_runtime_state_path(path: &Path, project_root: &Path) -> PathBuf {
    let path = path.strip_prefix(".").unwrap_or(path);
    if path.is_absolute() {
        return normalize_path_dots(path);
    }
    normalize_path_dots(&project_root.join(path))
}

fn normalize_path_dots(path: &Path) -> PathBuf {
    path.components().collect()
}

fn legacy_runtime_state_artifact_count(project_root: &Path) -> usize {
    LEGACY_RUNTIME_STATE_ARTIFACTS
        .iter()
        .filter(|name| project_root.join(name).exists())
        .count()
}

fn stale_runtime_state_bucket_count(state_root: &Path) -> usize {
    fs::read_dir(state_root)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .is_some_and(|name| name.starts_with(RUNTIME_STATE_DIR_PREFIX))
                        && entry.path() != current_runtime_state_dir(state_root)
                })
                .count()
        })
        .unwrap_or(0)
}

fn retire_runtime_state_at(
    project_root: &Path,
    state_root: &Path,
    active_paths: &[&PathBuf],
    apply: bool,
) -> std::io::Result<RuntimeStateRetireReport> {
    let current = current_runtime_state_dir(state_root);
    let retire_root = state_root.join("retired").join(format!(
        "{RUNTIME_STATE_DIR_PREFIX}{}",
        env!("CARGO_PKG_VERSION")
    ));
    let candidates =
        runtime_state_retire_candidates(project_root, state_root, &current, &retire_root)
            .into_iter()
            .filter(|candidate| !runtime_state_retire_candidate_is_active(candidate, active_paths))
            .collect::<Vec<_>>();
    let candidate_paths = candidates
        .iter()
        .map(|candidate| candidate.source.clone())
        .collect::<Vec<_>>();
    let mut retired_to = Vec::new();
    let mut lifecycle_records = Vec::new();

    if apply {
        for candidate in candidates {
            let destination = unique_retire_destination(&candidate.destination);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(&candidate.source, &destination)?;
            lifecycle_records
                .push(candidate.lifecycle_record(destination.clone(), "retired_blocked"));
            retired_to.push(destination);
        }
    } else {
        lifecycle_records.extend(candidates.iter().map(|candidate| {
            candidate.lifecycle_record(candidate.destination.clone(), "tombstone_preview")
        }));
    }

    Ok(RuntimeStateRetireReport {
        applied: apply,
        current,
        retire_root,
        candidates: candidate_paths,
        retired_to,
        lifecycle_records,
    })
}

fn runtime_state_retire_candidate_is_active(
    candidate: &RuntimeStateRetireCandidate,
    active_paths: &[&PathBuf],
) -> bool {
    active_paths
        .iter()
        .any(|path| runtime_state_paths_overlap(path, &candidate.source))
}

fn runtime_state_paths_overlap(active: &Path, candidate: &Path) -> bool {
    let active = active.strip_prefix(".").unwrap_or(active);
    let candidate = candidate.strip_prefix(".").unwrap_or(candidate);
    active == candidate || active.starts_with(candidate)
}

fn runtime_state_retire_candidates(
    project_root: &Path,
    state_root: &Path,
    current: &Path,
    retire_root: &Path,
) -> Vec<RuntimeStateRetireCandidate> {
    let mut candidates = Vec::new();
    for name in LEGACY_RUNTIME_STATE_ARTIFACTS {
        let source = project_root.join(name);
        if source.exists() {
            candidates.push(RuntimeStateRetireCandidate {
                source,
                destination: retire_root.join("legacy-root").join(name),
                kind: RuntimeStateRetireCandidateKind::LegacyRootArtifact,
            });
        }
    }

    if let Ok(entries) = fs::read_dir(state_root) {
        for entry in entries.filter_map(Result::ok) {
            if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name();
            let Some(name_text) = name.to_str() else {
                continue;
            };
            if name_text.starts_with(RUNTIME_STATE_DIR_PREFIX) && entry.path() != current {
                candidates.push(RuntimeStateRetireCandidate {
                    source: entry.path(),
                    destination: retire_root.join("stale-buckets").join(name),
                    kind: RuntimeStateRetireCandidateKind::StaleVersionBucket,
                });
            }
        }
    }

    candidates
}

fn unique_retire_destination(destination: &Path) -> PathBuf {
    if !destination.exists() {
        return destination.to_path_buf();
    }
    let Some(file_name) = destination.file_name().and_then(|name| name.to_str()) else {
        return destination.to_path_buf();
    };
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    for index in 1.. {
        let candidate = parent.join(format!("{file_name}.{index}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    destination.to_path_buf()
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("rust-norion-{name}-{}-{stamp}", std::process::id()))
    }

    #[test]
    fn runtime_state_bucket_summary_counts_legacy_and_stale_buckets() {
        let root = temp_dir("runtime-state-bucket");
        let state_root = root.join("state");
        let current = current_runtime_state_dir(&state_root);
        let stale = state_root.join("rust-norion-v0.0.1");
        fs::create_dir_all(&current).unwrap();
        fs::create_dir_all(&stale).unwrap();
        File::create(root.join("noiron-experience.ndkv")).unwrap();
        let args = Args::parse(vec![
            "--memory".to_owned(),
            current.join("memory.ndkv").display().to_string(),
            "--experience".to_owned(),
            current.join("experience.ndkv").display().to_string(),
            "--adaptive".to_owned(),
            current.join("adaptive.ndkv").display().to_string(),
        ]);

        let summary = runtime_state_bucket_at(&args, &root, &state_root);

        assert!(summary.in_current_bucket);
        assert_eq!(summary.legacy_root_artifacts, 1);
        assert_eq!(summary.stale_version_buckets, 1);
        assert!(summary.summary_line().contains("in_current_bucket=true"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn runtime_state_retire_dry_run_keeps_candidates_in_place() {
        let root = temp_dir("runtime-state-retire-dry-run");
        let state_root = root.join("state");
        let current = current_runtime_state_dir(&state_root);
        let stale = state_root.join("rust-norion-v0.0.1");
        fs::create_dir_all(&current).unwrap();
        fs::create_dir_all(&stale).unwrap();
        File::create(root.join("noiron-memory.ndkv")).unwrap();

        let report = retire_runtime_state_at(&root, &state_root, &[], false).unwrap();

        assert!(!report.applied);
        assert_eq!(report.candidate_count(), 2);
        assert_eq!(report.retired_count(), 0);
        assert_eq!(report.lifecycle_record_count(), 2);
        assert!(
            report
                .lifecycle_records
                .iter()
                .all(|record| record.state == "tombstone_preview")
        );
        assert!(report.lifecycle_records.iter().any(|record| {
            record.reason_code == "legacy_root_runtime_state"
                && record.parent_lineage == "runtime_state:legacy_root"
                && record.affected_scope == "runtime_state_legacy_root_artifact"
                && record.source_digest.starts_with("redaction-digest:")
                && record.rollback_anchor.starts_with("redaction-digest:")
                && record.readmission_gate
                    == "manual_restore_to_current_bucket_and_state_inspection_gate"
                && record.operator_approval_required
        }));
        assert!(root.join("noiron-memory.ndkv").exists());
        assert!(stale.exists());
        assert!(current.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn runtime_state_retire_apply_moves_legacy_and_stale_buckets() {
        let root = temp_dir("runtime-state-retire-apply");
        let state_root = root.join("state");
        let current = current_runtime_state_dir(&state_root);
        let stale = state_root.join("rust-norion-v0.0.1");
        fs::create_dir_all(&current).unwrap();
        fs::create_dir_all(&stale).unwrap();
        File::create(root.join("noiron-experience.ndkv")).unwrap();

        let report = retire_runtime_state_at(&root, &state_root, &[], true).unwrap();

        assert!(report.applied);
        assert_eq!(report.candidate_count(), 2);
        assert_eq!(report.retired_count(), 2);
        assert_eq!(report.lifecycle_record_count(), 2);
        assert!(
            report
                .lifecycle_records
                .iter()
                .all(|record| record.state == "retired_blocked")
        );
        assert!(report.lifecycle_records.iter().any(|record| {
            record.reason_code == "stale_runtime_state_bucket"
                && record.parent_lineage == "runtime_state:version_bucket"
                && record.affected_scope == "runtime_state_stale_version_bucket"
                && record.evidence_line().contains("readmission_gate=")
                && record
                    .evidence_line()
                    .contains("operator_approval_required=true")
        }));
        for record in &report.lifecycle_records {
            assert!(report.retired_to.contains(&record.destination));
        }
        assert!(!root.join("noiron-experience.ndkv").exists());
        assert!(!stale.exists());
        assert!(current.exists());
        assert!(
            report
                .retired_to
                .iter()
                .any(|path| path.ends_with("noiron-experience.ndkv"))
        );
        assert!(
            report
                .retired_to
                .iter()
                .any(|path| path.ends_with("rust-norion-v0.0.1"))
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn runtime_state_retire_apply_keeps_active_custom_paths() {
        let root = temp_dir("runtime-state-retire-active");
        let state_root = root.join("state");
        let current = current_runtime_state_dir(&state_root);
        let stale = state_root.join("rust-norion-v0.0.1");
        fs::create_dir_all(&current).unwrap();
        fs::create_dir_all(&stale).unwrap();
        let active_memory = stale.join("memory.ndkv");
        File::create(&active_memory).unwrap();
        File::create(root.join("noiron-adaptive.ndkv")).unwrap();

        let report = retire_runtime_state_at(&root, &state_root, &[&active_memory], true).unwrap();

        assert!(report.applied);
        assert_eq!(report.candidate_count(), 1);
        assert_eq!(report.retired_count(), 1);
        assert_eq!(report.lifecycle_record_count(), 1);
        assert_eq!(
            report.lifecycle_records[0].reason_code,
            "legacy_root_runtime_state"
        );
        assert!(stale.exists());
        assert!(active_memory.exists());
        assert!(!root.join("noiron-adaptive.ndkv").exists());
        assert!(current.exists());
        fs::remove_dir_all(root).unwrap();
    }
}

pub(crate) fn print_state_inspection_gate_report(report: &StateInspectionGateReport) {
    println!("{}", report.summary_line());
    for failure in &report.failures {
        println!("state_inspection_gate_failure: {failure}");
    }
}

pub(crate) fn print_state_inspection_matrix_gate_report(
    args: &Args,
    report: &StateInspectionMatrixGateReport,
) {
    println!("Noiron state inspection all-device gate");
    println!("memory_file_pattern: {}", args.memory_path.display());
    println!(
        "experience_file_pattern: {}",
        args.experience_path.display()
    );
    println!("adaptive_file_pattern: {}", args.adaptive_path.display());
    println!("{}", report.summary_line());
    for device_report in &report.device_reports {
        println!(
            "device={} {} runtime_kv_memories={} runtime_model_experiences={} runtime_adapter_experiences={} runtime_forward_energy_experiences={} runtime_kv_influence_experiences={} runtime_uncertainty_experiences={} runtime_uncertainty_tokens={} runtime_architecture_experiences={} runtime_kv_precision_experiences={} runtime_kv_precision_mismatches={} runtime_kv_import_experiences={} runtime_kv_export_experiences={} runtime_kv_hold_experiences={} runtime_kv_held_blocks={} reflection_issue_experiences={} critical_reflection_issue_experiences={} revision_action_experiences={} live_memory_feedback_experiences={} live_memory_feedback_updates={} live_memory_feedback_detail_experiences={} live_memory_feedback_applied={} live_memory_feedback_removed={} live_memory_feedback_missing={} live_memory_feedback_strength_delta={:.6} evolution_live_inference_runs={} evolution_live_router_threshold_mutations={} evolution_live_hierarchy_weight_mutations={} evolution_live_memory_updates={} evolution_live_stored_memory_updates={} evolution_live_reflection_issues={} evolution_live_critical_reflection_issues={} evolution_live_revision_actions={} evolution_replay_runs={} evolution_replay_items={} evolution_router_threshold_mutations={} evolution_hierarchy_weight_mutations={} evolution_memory_updates={} evolution_replay_live_memory_feedback_updates={} evolution_replay_live_memory_feedback_detail_items={} evolution_replay_live_memory_feedback_applied={} evolution_replay_live_memory_feedback_removed={} evolution_replay_live_memory_feedback_missing={} evolution_replay_live_memory_feedback_strength_delta={:.6} evolution_recursive_replay_items={} evolution_recursive_runtime_calls={}",
            device_report.device.as_str(),
            device_report.report.summary_line(),
            device_report.runtime_kv_memories,
            device_report.runtime_model_experiences,
            device_report.runtime_adapter_experiences,
            device_report.runtime_forward_energy_experiences,
            device_report.runtime_kv_influence_experiences,
            device_report.runtime_uncertainty_experiences,
            device_report.runtime_uncertainty_tokens,
            device_report.runtime_architecture_experiences,
            device_report.runtime_kv_precision_experiences,
            device_report.runtime_kv_precision_mismatches,
            device_report.runtime_kv_import_experiences,
            device_report.runtime_kv_export_experiences,
            device_report.runtime_kv_hold_experiences,
            device_report.runtime_kv_held_blocks,
            device_report.reflection_issue_experiences,
            device_report.critical_reflection_issue_experiences,
            device_report.revision_action_experiences,
            device_report.live_memory_feedback_experiences,
            device_report.live_memory_feedback_updates,
            device_report.live_memory_feedback_detail_experiences,
            device_report.live_memory_feedback_applied,
            device_report.live_memory_feedback_removed,
            device_report.live_memory_feedback_missing,
            device_report.live_memory_feedback_strength_delta,
            device_report.evolution_live_inference_runs,
            device_report.evolution_live_router_threshold_mutations,
            device_report.evolution_live_hierarchy_weight_mutations,
            device_report.evolution_live_memory_updates,
            device_report.evolution_live_stored_memory_updates,
            device_report.evolution_live_reflection_issues,
            device_report.evolution_live_critical_reflection_issues,
            device_report.evolution_live_revision_actions,
            device_report.evolution_replay_runs,
            device_report.evolution_replay_items,
            device_report.evolution_router_threshold_mutations,
            device_report.evolution_hierarchy_weight_mutations,
            device_report.evolution_memory_updates,
            device_report.evolution_replay_live_memory_feedback_updates,
            device_report.evolution_replay_live_memory_feedback_detail_items,
            device_report.evolution_replay_live_memory_feedback_applied,
            device_report.evolution_replay_live_memory_feedback_removed,
            device_report.evolution_replay_live_memory_feedback_missing,
            device_report.evolution_replay_live_memory_feedback_strength_delta,
            device_report.evolution_recursive_replay_items,
            device_report.evolution_recursive_runtime_calls
        );
        for failure in &device_report.report.failures {
            println!(
                "state_inspection_matrix_gate_failure: device={} {}",
                device_report.device.as_str(),
                failure
            );
        }
    }
    for failure in &report.failures {
        println!("state_inspection_matrix_gate_failure: {failure}");
    }
}

fn option_text(value: Option<&str>) -> &str {
    value.filter(|item| !item.is_empty()).unwrap_or("none")
}

fn option_f32_text(value: Option<f32>) -> String {
    value
        .filter(|item| item.is_finite())
        .map(|item| format!("{item:.3}"))
        .unwrap_or_else(|| "none".to_owned())
}

fn option_usize_text(value: Option<usize>) -> String {
    value
        .map(|item| item.to_string())
        .unwrap_or_else(|| "none".to_owned())
}

fn string_list_text(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(",")
    }
}
