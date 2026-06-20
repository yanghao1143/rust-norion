use std::path::{Path, PathBuf};

use rust_norion::{
    DeviceClass, NoironEngine, StateInspectionDeviceGateReport, StateInspectionGateReport,
    StateInspectionMatrixGateReport, StateInspectionReport,
};

use crate::Args;
use crate::engine_config::configure_engine;

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
