use super::super::*;

pub(super) fn evaluate_base_counts(
    report: &StateInspectionReport,
    gate: &StateInspectionGate,
    failures: &mut Vec<String>,
) {
    require_min_usize(
        failures,
        "memory_count",
        report.memory_count,
        gate.min_memories,
    );
    require_min_usize(
        failures,
        "runtime_kv_memory_count",
        report.runtime_kv_memory_count,
        gate.min_runtime_kv_memories,
    );
    require_min_usize(
        failures,
        "experience_count",
        report.experience_count,
        gate.min_experiences,
    );
    require_max_usize(
        failures,
        "experience_hygiene_quarantine_candidate_count",
        report.experience_hygiene_quarantine_candidate_count,
        gate.max_experience_hygiene_quarantine_candidates,
    );
    require_max_usize(
        failures,
        "experience_repairable_legacy_metadata_lesson_count",
        report.experience_repairable_legacy_metadata_lesson_count,
        gate.max_experience_repairable_legacy_metadata_lessons,
    );
    require_max_usize(
        failures,
        "experience_repairable_index_record_count",
        report.experience_repairable_index_record_count,
        gate.max_experience_repairable_index_records,
    );
    require_max_usize(
        failures,
        "experience_repair_projected_legacy_metadata_lesson_count",
        report.experience_repair_projected_legacy_metadata_lesson_count,
        gate.max_experience_repair_projected_legacy_metadata_lessons,
    );
    require_max_usize(
        failures,
        "experience_repair_skipped_missing_clean_gist_count",
        report.experience_repair_skipped_missing_clean_gist_count,
        gate.max_experience_repair_skipped_missing_clean_gist,
    );
    require_max_usize(
        failures,
        "experience_index_overlong_record_count",
        report.experience_index_overlong_record_count,
        gate.max_experience_index_overlong_records,
    );
    require_max_usize(
        failures,
        "experience_index_overlong_without_clean_gist_count",
        report.experience_index_overlong_without_clean_gist_count,
        gate.max_experience_index_overlong_without_clean_gist,
    );
    require_max_usize(
        failures,
        "experience_index_max_record_chars",
        report.experience_index_max_record_chars,
        gate.max_experience_index_record_chars,
    );
    require_max_usize(
        failures,
        "experience_index_noisy_record_count",
        report.experience_index_noisy_record_count,
        gate.max_experience_index_noisy_records,
    );
    require_max_f32(
        failures,
        "experience_index_max_noise_penalty",
        report.experience_index_max_noise_penalty,
        gate.max_experience_index_noise_penalty,
    );
    require_min_f32(
        failures,
        "experience_index_quality_score",
        report.experience_index_quality_score,
        gate.min_experience_index_quality_score,
    );
    if gate.require_experience_index_retrieval_ready && !report.experience_index_retrieval_ready {
        failures.push("experience_index_retrieval_ready false but required true".to_owned());
    }
}
