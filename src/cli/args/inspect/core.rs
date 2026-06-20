use super::InspectFlagParse;
use crate::cli::args::values::{parse_f32, parse_usize};

pub(crate) fn parse(
    parser: &mut InspectFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--inspect-state" => {
            *parser.inspect_state = true;
            if *parser.benchmark_all_devices {
                *parser.inspect_gate = true;
            }
            Some(1)
        }
        "--inspect-gate" => {
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(1)
        }
        "--inspect-limit" if index + 1 < raw.len() => {
            *parser.inspect_limit = parse_usize(&raw[index + 1], *parser.inspect_limit).max(1);
            *parser.inspect_state = true;
            Some(2)
        }
        "--inspect-min-memories" if index + 1 < raw.len() => {
            *parser.inspect_min_memories = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-runtime-kv-memories" if index + 1 < raw.len() => {
            *parser.inspect_min_runtime_kv_memories = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-experiences" if index + 1 < raw.len() => {
            *parser.inspect_min_experiences = Some(parse_usize(&raw[index + 1], 0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-experience-hygiene-quarantine-candidates" if index + 1 < raw.len() => {
            *parser.inspect_max_experience_hygiene_quarantine_candidates =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-experience-repairable-legacy-metadata-lessons" if index + 1 < raw.len() => {
            *parser.inspect_max_experience_repairable_legacy_metadata_lessons =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-experience-repairable-index-records" if index + 1 < raw.len() => {
            *parser.inspect_max_experience_repairable_index_records =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-experience-repair-projected-legacy-metadata-lessons"
            if index + 1 < raw.len() =>
        {
            *parser.inspect_max_experience_repair_projected_legacy_metadata_lessons =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-experience-repair-skipped-missing-clean-gist" if index + 1 < raw.len() => {
            *parser.inspect_max_experience_repair_skipped_missing_clean_gist =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-experience-index-overlong-records" if index + 1 < raw.len() => {
            *parser.inspect_max_experience_index_overlong_records =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-experience-index-overlong-without-clean-gist" if index + 1 < raw.len() => {
            *parser.inspect_max_experience_index_overlong_without_clean_gist =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-experience-index-record-chars" if index + 1 < raw.len() => {
            *parser.inspect_max_experience_index_record_chars =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-experience-index-noisy-records" if index + 1 < raw.len() => {
            *parser.inspect_max_experience_index_noisy_records =
                Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-max-experience-index-noise-penalty" if index + 1 < raw.len() => {
            *parser.inspect_max_experience_index_noise_penalty =
                Some(parse_f32(&raw[index + 1], f32::MAX).max(0.0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-min-experience-index-quality-score" if index + 1 < raw.len() => {
            *parser.inspect_min_experience_index_quality_score =
                Some(parse_f32(&raw[index + 1], 0.0).clamp(0.0, 1.0));
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(2)
        }
        "--inspect-require-experience-index-retrieval-ready" => {
            *parser.inspect_require_experience_index_retrieval_ready = true;
            *parser.inspect_state = true;
            *parser.inspect_gate = true;
            Some(1)
        }
        _ => None,
    }
}
