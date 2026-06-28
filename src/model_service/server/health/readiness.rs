use rust_norion::{ComputeLane, HardwarePlan};

use super::hygiene::ExperienceHygieneHealthStatus;
use crate::cli::state::{runtime_state_bucket, RuntimeStateBucketSummary};
use crate::Args;

pub(super) struct HealthReadinessReport {
    pub(super) readiness_failures: Vec<String>,
    pub(super) safe_device_failures: Vec<String>,
    pub(super) warnings: Vec<String>,
}

pub(super) fn health_readiness_report(
    args: &Args,
    gemma_runtime_reachable: Option<bool>,
    gemma_runtime_context_window: Option<usize>,
    active_engine_requests: usize,
    plan: &HardwarePlan,
    experience_hygiene: &ExperienceHygieneHealthStatus,
) -> HealthReadinessReport {
    let readiness_failures = readiness_failures(
        args,
        gemma_runtime_reachable,
        gemma_runtime_context_window,
        active_engine_requests,
    );
    let safe_device_failures = safe_device_failures(args, plan);
    let warnings = readiness_warnings(
        &readiness_failures,
        &safe_device_failures,
        plan,
        experience_hygiene,
    );

    HealthReadinessReport {
        readiness_failures,
        safe_device_failures,
        warnings,
    }
}

fn readiness_failures(
    args: &Args,
    gemma_runtime_reachable: Option<bool>,
    gemma_runtime_context_window: Option<usize>,
    active_engine_requests: usize,
) -> Vec<String> {
    let mut failures = Vec::new();

    if active_engine_requests > 0 {
        failures.push(
            "engine_busy: wait for current inference before sending another prompt".to_owned(),
        );
    }

    if args.gemma_runtime_server.is_some() && gemma_runtime_reachable != Some(true) {
        failures.push("gemma_runtime: configured Gemma HTTP runtime is not reachable".to_owned());
    }

    if let Some(actual_window) = gemma_runtime_context_window
        && args.gemma_runtime_server.is_some()
        && actual_window < args.runtime_metadata.native_context_window
    {
        failures.push(format!(
            "gemma_runtime_context: runtime n_ctx={} is below configured runtime-native-window={}",
            actual_window, args.runtime_metadata.native_context_window
        ));
    }

    failures.extend(runtime_state_bucket(args).blocking_failures());

    failures
}

fn safe_device_failures(args: &Args, plan: &HardwarePlan) -> Vec<String> {
    let mut failures = Vec::new();

    if (args.gemma_12b_runtime || args.gemma_runtime_server.is_some())
        && is_cpu_or_disk_lane(plan.execution.primary_lane)
    {
        failures.push(
            "gemma_12b_device: selected plan is CPU/disk-first; expect very slow inference and high RAM pressure unless the external runtime uses GPU".to_owned(),
        );
    }

    failures
}

fn readiness_warnings(
    readiness_failures: &[String],
    safe_device_failures: &[String],
    plan: &HardwarePlan,
    experience_hygiene: &ExperienceHygieneHealthStatus,
) -> Vec<String> {
    let mut warnings = Vec::new();
    warnings.extend(readiness_failures.iter().cloned());
    warnings.extend(safe_device_failures.iter().cloned());

    if plan.pressure >= 0.72 {
        warnings.push(
            "device_pressure: high load; reduce concurrency/context before local 12B inference"
                .to_owned(),
        );
    }

    warnings.extend(experience_hygiene.warnings());

    warnings
}

fn is_cpu_or_disk_lane(lane: ComputeLane) -> bool {
    matches!(
        lane,
        ComputeLane::CpuPortable | ComputeLane::CpuVector | ComputeLane::DiskBackedStreaming
    )
}

#[cfg(test)]
mod tests {
    use super::super::hygiene::{ExperienceHygieneRepairHealthStatus, ExperienceIndexHealthStatus};
    use super::*;
    use rust_norion::{HardwareAllocator, HierarchyWeights};
    use std::path::PathBuf;

    fn plan_for_args(args: &Args) -> HardwarePlan {
        let probe = args.effective_probe_report();
        HardwareAllocator::new().plan(
            probe.snapshot(),
            args.profile,
            args.prompt_token_estimate(),
            HierarchyWeights::default(),
        )
    }

    fn clean_hygiene_status() -> ExperienceHygieneHealthStatus {
        ExperienceHygieneHealthStatus {
            experience_file: PathBuf::from("test-experience.ndkv"),
            checked: true,
            clean: Some(true),
            findings: Some(0),
            watch: Some(0),
            quarantine_candidates: Some(0),
            legacy_metadata_lessons: Some(0),
            legacy_metadata_without_clean_gist: Some(0),
            repair: Some(clean_repair_status()),
            index: None,
            error: None,
        }
    }

    fn clean_repair_status() -> ExperienceHygieneRepairHealthStatus {
        ExperienceHygieneRepairHealthStatus {
            repairable_legacy_metadata_lessons: 0,
            repairable_index_records: 0,
            projected_findings_after_repair: 0,
            projected_watch_after_repair: 0,
            projected_quarantine_candidates_after_repair: 0,
            projected_legacy_metadata_lessons_after_repair: 0,
            projected_legacy_metadata_without_clean_gist_after_repair: 0,
            skipped_quarantine_candidates: 0,
            skipped_missing_clean_gist: 0,
        }
    }

    fn runtime_bucket_summary(
        in_current_bucket: bool,
        legacy_root_artifacts: usize,
        stale_version_buckets: usize,
    ) -> RuntimeStateBucketSummary {
        let current = PathBuf::from("state").join("rust-norion-v0.1.0");
        RuntimeStateBucketSummary {
            memory_path: current.join("memory.ndkv"),
            experience_path: current.join("experience.ndkv"),
            adaptive_path: current.join("adaptive.ndkv"),
            current,
            in_current_bucket,
            legacy_root_artifacts,
            stale_version_buckets,
        }
    }

    #[test]
    fn runtime_state_bucket_failures_block_dirty_state_windows() {
        let failures = runtime_bucket_summary(false, 2, 1).blocking_failures();

        assert_eq!(failures.len(), 3);
        assert!(failures[0].contains("outside the current version bucket"));
        assert!(failures[1].contains("2 legacy root artifacts"));
        assert!(failures[2].contains("1 stale version buckets"));
    }

    #[test]
    fn readiness_report_warns_when_gemma_12b_is_cpu_first() {
        let args = Args::parse(vec![
            "--device".to_owned(),
            "cpu".to_owned(),
            "--gemma-12b-runtime".to_owned(),
        ]);
        let plan = plan_for_args(&args);

        let report = health_readiness_report(&args, None, None, 0, &plan, &clean_hygiene_status());

        assert!(report.readiness_failures.is_empty());
        assert_eq!(report.safe_device_failures.len(), 1);
        assert!(report.safe_device_failures[0].contains("gemma_12b_device"));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("CPU/disk-first")));
    }

    #[test]
    fn readiness_report_blocks_when_engine_is_busy() {
        let args = Args::parse(Vec::<String>::new());
        let plan = plan_for_args(&args);

        let report = health_readiness_report(&args, None, None, 1, &plan, &clean_hygiene_status());

        assert_eq!(report.readiness_failures.len(), 1);
        assert!(report.readiness_failures[0].contains("engine_busy"));
        assert!(report.safe_device_failures.is_empty());
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("engine_busy")));
    }

    #[test]
    fn readiness_report_warns_for_dirty_experience_hygiene() {
        let args = Args::parse(Vec::<String>::new());
        let plan = plan_for_args(&args);
        let dirty_hygiene = ExperienceHygieneHealthStatus {
            experience_file: PathBuf::from("test-experience.ndkv"),
            checked: true,
            clean: Some(false),
            findings: Some(4),
            watch: Some(0),
            quarantine_candidates: Some(4),
            legacy_metadata_lessons: Some(0),
            legacy_metadata_without_clean_gist: Some(0),
            repair: Some(clean_repair_status()),
            index: None,
            error: None,
        };

        let report = health_readiness_report(&args, None, None, 0, &plan, &dirty_hygiene);

        assert!(report.readiness_failures.is_empty());
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("experience_hygiene: 4 quarantine candidates")));
    }

    #[test]
    fn readiness_report_warns_for_repairable_legacy_metadata() {
        let args = Args::parse(Vec::<String>::new());
        let plan = plan_for_args(&args);
        let repairable_hygiene = ExperienceHygieneHealthStatus {
            experience_file: PathBuf::from("test-experience.ndkv"),
            checked: true,
            clean: Some(false),
            findings: Some(1),
            watch: Some(1),
            quarantine_candidates: Some(0),
            legacy_metadata_lessons: Some(1),
            legacy_metadata_without_clean_gist: Some(0),
            repair: Some(ExperienceHygieneRepairHealthStatus {
                repairable_legacy_metadata_lessons: 1,
                repairable_index_records: 0,
                projected_findings_after_repair: 0,
                projected_watch_after_repair: 0,
                projected_quarantine_candidates_after_repair: 0,
                projected_legacy_metadata_lessons_after_repair: 0,
                projected_legacy_metadata_without_clean_gist_after_repair: 0,
                skipped_quarantine_candidates: 0,
                skipped_missing_clean_gist: 0,
            }),
            index: None,
            error: None,
        };

        let report = health_readiness_report(&args, None, None, 0, &plan, &repairable_hygiene);

        assert!(report.readiness_failures.is_empty());
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("experience_repair: 1 legacy metadata lessons")));
    }

    #[test]
    fn readiness_report_warns_for_repairable_index_records() {
        let args = Args::parse(Vec::<String>::new());
        let plan = plan_for_args(&args);
        let repairable_hygiene = ExperienceHygieneHealthStatus {
            experience_file: PathBuf::from("test-experience.ndkv"),
            checked: true,
            clean: Some(false),
            findings: Some(1),
            watch: Some(1),
            quarantine_candidates: Some(0),
            legacy_metadata_lessons: Some(0),
            legacy_metadata_without_clean_gist: Some(0),
            repair: Some(ExperienceHygieneRepairHealthStatus {
                repairable_legacy_metadata_lessons: 0,
                repairable_index_records: 1,
                projected_findings_after_repair: 0,
                projected_watch_after_repair: 0,
                projected_quarantine_candidates_after_repair: 0,
                projected_legacy_metadata_lessons_after_repair: 0,
                projected_legacy_metadata_without_clean_gist_after_repair: 0,
                skipped_quarantine_candidates: 0,
                skipped_missing_clean_gist: 0,
            }),
            index: None,
            error: None,
        };

        let report = health_readiness_report(&args, None, None, 0, &plan, &repairable_hygiene);

        assert!(report.readiness_failures.is_empty());
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("experience_repair: 1 index records")));
    }

    #[test]
    fn readiness_report_warns_for_degraded_experience_index() {
        let args = Args::parse(Vec::<String>::new());
        let plan = plan_for_args(&args);
        let mut degraded_index = clean_hygiene_status();
        degraded_index.index = Some(ExperienceIndexHealthStatus {
            total_records: 4,
            compacted_records: 1,
            noisy_records: 1,
            duplicate_outputs: 1,
            max_noise_penalty: 0.18,
            quality_score: 0.603333,
            retrieval_ready: true,
            risk_level: "degraded".to_owned(),
        });

        let report = health_readiness_report(&args, None, None, 0, &plan, &degraded_index);

        assert!(report.readiness_failures.is_empty());
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("experience_index: risk_level=degraded")));
    }

    #[test]
    fn readiness_report_blocks_when_gemma_context_is_below_configured_window() {
        let args = Args::parse(vec![
            "--gemma-runtime-server".to_owned(),
            "http://127.0.0.1:8686".to_owned(),
            "--runtime-native-window".to_owned(),
            "262144".to_owned(),
        ]);
        let plan = plan_for_args(&args);

        let report = health_readiness_report(
            &args,
            Some(true),
            Some(4096),
            0,
            &plan,
            &clean_hygiene_status(),
        );

        assert_eq!(report.readiness_failures.len(), 1);
        assert!(report.readiness_failures[0].contains("gemma_runtime_context"));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("runtime n_ctx=4096")));
    }
}
