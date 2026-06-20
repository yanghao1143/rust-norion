use super::hygiene::{experience_hygiene_quarantine_summary, experience_hygiene_report_summary};
use super::json::{
    json_bool_field, json_number_field, json_object_field, json_string, json_string_field,
};
use super::repair::experience_repair_summary;

pub(crate) struct ExperienceCleanupAuditParts {
    pub(crate) limit: usize,
    pub(crate) hygiene: String,
    pub(crate) quarantine: String,
    pub(crate) repair: String,
}

pub(crate) fn experience_cleanup_audit_response_summary(body: &str) -> Result<String, String> {
    if json_bool_field(body, "ok") == Some(false) {
        let error = json_string_field(body, "error")
            .unwrap_or_else(|| "unknown cleanup audit error".to_owned());
        return Err(format!("experience cleanup audit failed: {error}"));
    }
    require_read_only_cleanup_audit(body)?;
    let limit = json_number_field(body, "sample_limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);
    let hygiene = experience_hygiene_report_summary(body)?;
    let quarantine = experience_hygiene_quarantine_summary(&synthetic_quarantine_body(body))?;
    let repair = experience_repair_summary(&synthetic_repair_body(body))?;
    Ok(experience_cleanup_audit_summary(
        ExperienceCleanupAuditParts {
            limit,
            hygiene,
            quarantine,
            repair,
        },
    ))
}

fn require_read_only_cleanup_audit(body: &str) -> Result<(), String> {
    let mut saw_false = false;
    let needle = "\"writes_experience_state\"";
    for (index, _) in body.match_indices(needle) {
        if json_quote_is_escaped(body, index) {
            continue;
        }
        let Some(after_field) = body.get(index + needle.len()..) else {
            continue;
        };
        let Some(after_colon) = after_field.trim_start().strip_prefix(':') else {
            continue;
        };
        let value = after_colon.trim_start();
        if json_literal_is_delimited(value, "true") {
            return Err(
                "experience cleanup audit rejected non-read-only response: writes_experience_state=true"
                    .to_owned(),
            );
        }
        if json_literal_is_delimited(value, "false") {
            saw_false = true;
        }
    }
    if saw_false {
        Ok(())
    } else {
        Err("experience cleanup audit response missing writes_experience_state=false".to_owned())
    }
}

fn json_quote_is_escaped(input: &str, quote_index: usize) -> bool {
    input
        .as_bytes()
        .get(..quote_index)
        .unwrap_or_default()
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count()
        % 2
        == 1
}

fn json_literal_is_delimited(input: &str, literal: &str) -> bool {
    let Some(trailing) = input.strip_prefix(literal).map(str::trim_start) else {
        return false;
    };
    trailing.is_empty() || matches!(trailing.as_bytes().first(), Some(b',' | b'}' | b']'))
}

pub(crate) fn cleanup_audit_endpoint_missing(error: &str) -> bool {
    error.contains("unsupported HTTP path")
        || error.contains("HTTP 404")
        || error.contains("HTTP 405")
}

pub(crate) fn experience_cleanup_audit_summary(parts: ExperienceCleanupAuditParts) -> String {
    let ExperienceCleanupAuditParts {
        limit,
        hygiene,
        quarantine,
        repair,
    } = parts;
    let limit = limit.max(1);
    let readiness = cleanup_readiness_summary(&hygiene, &quarantine, &repair);
    format!(
        "Noiron experience cleanup audit\nwrites_experience_state=false\nsample_limit={limit}\n\n## Readiness gate\n{readiness}\n\n## Hygiene\n{hygiene}\n\n## Quarantine dry-run\n{quarantine}\n\n## Repair dry-run\n{repair}\n\nnext_step=Review this audit, then apply quarantine or repair only after explicit confirmation and backup review."
    )
}

fn cleanup_readiness_summary(hygiene: &str, quarantine: &str, repair: &str) -> String {
    let quarantine_candidates = first_number_after_any(
        &[hygiene, quarantine, repair],
        &[
            "quarantine_candidates=",
            "remaining_quarantine_candidates_after_repair=",
        ],
    );
    let repairable =
        first_number_after_any(&[repair, hygiene], &["repairable_legacy_metadata_lessons="]);
    let repairable_index =
        first_number_after_any(&[repair, hygiene], &["repairable_index_records="]);
    let noisy_records = first_number_after_any(&[hygiene], &["noisy_records="]);
    let max_noise = first_decimal_after_any(&[hygiene], &["max_noise_penalty="]);
    let index_retrieval_ready = first_allowed_token_after_any(
        &[hygiene, repair],
        &["retrieval_ready=", "index_retrieval_ready="],
        &["true", "false"],
    );
    let index_risk = first_allowed_token_after_any(
        &[hygiene, repair],
        &["risk_level=", "index_risk_level="],
        &["clean", "watch", "blocked"],
    );
    let hygiene_clean = first_allowed_token_after_any(&[hygiene], &["clean="], &["true", "false"]);
    let candidate_ids = first_value_after_line_prefix(&[quarantine, hygiene], "candidate_ids=");
    let dirty = quarantine_candidates.unwrap_or(0) > 0
        || repairable.unwrap_or(0) > 0
        || repairable_index.unwrap_or(0) > 0
        || index_retrieval_ready.as_deref() == Some("false")
        || index_risk.as_deref() == Some("blocked")
        || hygiene_clean.as_deref() == Some("false");

    let mut lines = Vec::new();
    lines.push(format!("ready_to_chat={}", !dirty));
    if dirty {
        lines.push(format!(
            "blocking_reasons=quarantine_candidates={} repairable_legacy_metadata_lessons={} repairable_index_records={} index_retrieval_ready={} index_risk_level={}",
            quarantine_candidates
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_owned()),
            repairable
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_owned()),
            repairable_index
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_owned()),
            index_retrieval_ready
                .as_deref()
                .unwrap_or("unknown"),
            index_risk.as_deref().unwrap_or("unknown")
        ));
        lines.push(
            "recommended_order=review audit -> dry-run quarantine -> explicit apply after backup -> dry-run repair -> explicit apply after backup -> rerun smoke/preflight"
                .to_owned(),
        );
    } else {
        lines.push("blocking_reasons=none".to_owned());
        lines.push(
            "recommended_order=rerun --preflight --require-safe-device, then send one short prompt"
                .to_owned(),
        );
    }
    if noisy_records.is_some()
        || max_noise.is_some()
        || index_retrieval_ready.is_some()
        || index_risk.is_some()
    {
        lines.push(format!(
            "index_quality=noisy_records={} max_noise_penalty={} retrieval_ready={} risk_level={}",
            noisy_records
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_owned()),
            max_noise.unwrap_or_else(|| "unknown".to_owned()),
            index_retrieval_ready.as_deref().unwrap_or("unknown"),
            index_risk.as_deref().unwrap_or("unknown")
        ));
    }
    if let Some(candidate_ids) = candidate_ids {
        lines.push(format!("candidate_ids={candidate_ids}"));
    }
    lines.push(
        "apply_guard=Forge audit is read-only; no .ndkv changes happen without an explicit rust-norion apply command."
            .to_owned(),
    );
    lines.join("\n")
}

fn first_number_after_any(inputs: &[&str], keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        inputs
            .iter()
            .find_map(|input| first_digits_after(input, key)?.parse::<u64>().ok())
    })
}

fn first_decimal_after_any(inputs: &[&str], keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        inputs
            .iter()
            .find_map(|input| first_decimal_after(input, key))
    })
}

fn first_digits_after<'a>(input: &'a str, key: &str) -> Option<&'a str> {
    summary_key_indices(input, key).find_map(|index| {
        let value = input.get(index + key.len()..)?;
        let end = value
            .char_indices()
            .find(|(_, character)| !character.is_ascii_digit())
            .map(|(index, _)| index)
            .unwrap_or(value.len());
        if end == 0 || !summary_value_tail_is_delimited(value.get(end..)?) {
            return None;
        }
        Some(&value[..end])
    })
}

fn first_decimal_after(input: &str, key: &str) -> Option<String> {
    summary_key_indices(input, key).find_map(|index| {
        let value = input.get(index + key.len()..)?;
        let end = summary_decimal_literal_len(value)?;
        if !summary_value_tail_is_delimited(value.get(end..)?) {
            return None;
        }
        value.get(..end).map(ToOwned::to_owned)
    })
}

fn summary_decimal_literal_len(value: &str) -> Option<usize> {
    let bytes = value.as_bytes();
    let mut index = 0usize;
    while matches!(bytes.get(index), Some(b'0'..=b'9')) {
        index += 1;
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let fraction_start = index;
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }
        if index == fraction_start {
            return None;
        }
    }
    (index > 0).then_some(index)
}

fn summary_value_tail_is_delimited(tail: &str) -> bool {
    tail.is_empty() || tail.chars().next().is_some_and(char::is_whitespace)
}

fn first_allowed_token_after_any(
    inputs: &[&str],
    keys: &[&str],
    allowed: &[&str],
) -> Option<String> {
    keys.iter().find_map(|key| {
        inputs.iter().find_map(|input| {
            summary_key_indices(input, key).find_map(|index| {
                let token = token_after(input, *key, index)?;
                allowed.contains(&token.as_str()).then_some(token)
            })
        })
    })
}

fn token_after(input: &str, key: &str, index: usize) -> Option<String> {
    let value = input.get(index + key.len()..)?;
    let token = value
        .chars()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || matches!(*character, '_' | '-' | '.')
        })
        .collect::<String>();
    (!token.is_empty()).then_some(token)
}

fn summary_key_indices<'a>(input: &'a str, key: &'a str) -> impl Iterator<Item = usize> + 'a {
    input.match_indices(key).filter_map(|(index, _)| {
        let boundary = index == 0
            || input
                .get(..index)?
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace);
        boundary.then_some(index)
    })
}

fn first_value_after_line_prefix(inputs: &[&str], prefix: &str) -> Option<String> {
    inputs.iter().find_map(|input| {
        input.lines().find_map(|line| {
            line.strip_prefix(prefix)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .filter(|value| summary_candidate_ids_value_is_valid(value))
                .map(str::to_owned)
        })
    })
}

fn summary_candidate_ids_value_is_valid(value: &str) -> bool {
    let Some(inner) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return false;
    };
    let inner = inner.trim();
    if inner.is_empty() {
        return true;
    }
    let mut saw_ellipsis = false;
    let mut saw_number = false;
    for raw in inner.split(',') {
        let item = raw.trim();
        if item == "..." {
            if saw_ellipsis || !saw_number {
                return false;
            }
            saw_ellipsis = true;
            continue;
        }
        if saw_ellipsis || !summary_unsigned_integer_is_valid(item) {
            return false;
        }
        saw_number = true;
    }
    true
}

fn summary_unsigned_integer_is_valid(value: &str) -> bool {
    match value.as_bytes() {
        [b'0'] => true,
        [b'1'..=b'9', rest @ ..] => rest.iter().all(u8::is_ascii_digit),
        _ => false,
    }
}

fn synthetic_quarantine_body(body: &str) -> String {
    let experience_file =
        json_string_field(body, "experience_file").unwrap_or_else(|| "unknown".to_owned());
    let plan = json_object_field(body, "quarantine_plan").unwrap_or("null");
    format!(
        "{{\"ok\":true,\"experience_file\":{},\"applied\":false,\"backup_file\":null,\"quarantine_file\":null,\"plan\":{plan}}}",
        json_string(&experience_file)
    )
}

fn synthetic_repair_body(body: &str) -> String {
    let experience_file =
        json_string_field(body, "experience_file").unwrap_or_else(|| "unknown".to_owned());
    let plan = json_object_field(body, "repair_plan").unwrap_or("null");
    format!(
        "{{\"ok\":true,\"experience_file\":{},\"applied\":false,\"backup_file\":null,\"plan\":{plan}}}",
        json_string(&experience_file)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_read_only_cleanup_audit_summary() {
        let summary = experience_cleanup_audit_summary(ExperienceCleanupAuditParts {
            limit: 0,
            hygiene: "Noiron experience hygiene\nreport: clean=false".to_owned(),
            quarantine: "Noiron experience hygiene quarantine dry-run\napply=false".to_owned(),
            repair: "Noiron experience repair dry-run\napply=false".to_owned(),
        });

        assert!(summary.contains("Noiron experience cleanup audit"));
        assert!(summary.contains("writes_experience_state=false"));
        assert!(summary.contains("sample_limit=1"));
        assert!(summary.contains("## Readiness gate"));
        assert!(summary.contains("ready_to_chat=false"));
        assert!(summary.contains("## Hygiene"));
        assert!(summary.contains("## Quarantine dry-run"));
        assert!(summary.contains("## Repair dry-run"));
        assert!(summary.contains("only after explicit confirmation"));
    }

    #[test]
    fn parses_backend_cleanup_audit_response() {
        let body = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"writes_experience_state\":false,\"sample_limit\":7,\"error\":null,\"report\":{\"total_records\":863,\"findings\":4,\"quarantine_candidates\":4,\"clean\":false,\"listed_findings\":[]},\"index_report\":{\"total_records\":863,\"compacted_records\":194,\"noisy_records\":1,\"max_noise_penalty\":0.18,\"listed_findings\":[]},\"quarantine_plan\":{\"applied\":false,\"total_records\":863,\"retained_records\":859,\"quarantine_candidates\":4,\"candidate_ids\":[851,861],\"listed_findings\":[]},\"repair_plan\":{\"total_records\":863,\"legacy_metadata_lessons\":860,\"repairable_legacy_metadata_lessons\":828,\"index_noisy_records\":1,\"index_duplicate_outputs\":1,\"repairable_index_records\":1,\"remaining_legacy_metadata_lessons_after_repair\":32,\"remaining_watch_after_repair\":28,\"remaining_quarantine_candidates_after_repair\":4,\"skipped_quarantine_candidates\":4,\"skipped_missing_clean_gist\":28,\"projected_hygiene_after_repair\":{\"total_records\":863,\"findings\":32,\"watch\":28,\"quarantine_candidates\":4,\"legacy_metadata_lessons\":32,\"legacy_metadata_without_clean_gist\":29,\"index_quality_score\":0.88,\"index_noisy_records\":0,\"index_duplicate_outputs\":0,\"index_retrieval_ready\":true,\"index_risk_level\":\"watch\"},\"listed_repairs\":[]}}";

        let summary = experience_cleanup_audit_response_summary(body).unwrap();

        assert!(summary.contains("sample_limit=7"));
        assert!(summary.contains("writes_experience_state=false"));
        assert!(summary.contains("ready_to_chat=false"));
        assert!(summary.contains("blocking_reasons=quarantine_candidates=4"));
        assert!(summary.contains("repairable_index_records=1"));
        assert!(summary.contains("index_retrieval_ready=true"));
        assert!(summary.contains("index_risk_level=watch"));
        assert!(summary.contains("index_quality=noisy_records=1"));
        assert!(summary.contains("index_report: total_records=863"));
        assert!(summary.contains("candidate_ids=[851,861]"));
        assert!(summary.contains("repairable_legacy_metadata_lessons=828"));
        assert!(summary.contains("index_noisy_records=1"));
        assert!(summary.contains("index_quality_score=0.88"));
    }

    #[test]
    fn cleanup_audit_blocks_on_index_retrieval_not_ready() {
        let body = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"writes_experience_state\":false,\"sample_limit\":5,\"error\":null,\"report\":{\"total_records\":42,\"findings\":0,\"quarantine_candidates\":0,\"clean\":true,\"listed_findings\":[]},\"index_report\":{\"total_records\":42,\"compacted_records\":8,\"noisy_records\":2,\"duplicate_outputs\":1,\"max_noise_penalty\":0.34,\"quality_score\":0.34,\"retrieval_ready\":false,\"risk_level\":\"blocked\",\"listed_findings\":[]},\"quarantine_plan\":{\"applied\":false,\"total_records\":42,\"retained_records\":42,\"quarantine_candidates\":0,\"candidate_ids\":[],\"listed_findings\":[]},\"repair_plan\":{\"total_records\":42,\"legacy_metadata_lessons\":0,\"repairable_legacy_metadata_lessons\":0,\"index_noisy_records\":2,\"index_duplicate_outputs\":1,\"repairable_index_records\":0,\"remaining_legacy_metadata_lessons_after_repair\":0,\"remaining_watch_after_repair\":0,\"remaining_quarantine_candidates_after_repair\":0,\"skipped_quarantine_candidates\":0,\"skipped_missing_clean_gist\":0,\"listed_repairs\":[]}}";

        let summary = experience_cleanup_audit_response_summary(body).unwrap();

        assert!(summary.contains("ready_to_chat=false"));
        assert!(summary.contains("blocking_reasons=quarantine_candidates=0"));
        assert!(summary.contains("repairable_legacy_metadata_lessons=0"));
        assert!(summary.contains("repairable_index_records=0"));
        assert!(summary.contains("index_retrieval_ready=false"));
        assert!(summary.contains("index_risk_level=blocked"));
        assert!(summary.contains(
            "index_quality=noisy_records=2 max_noise_penalty=0.34 retrieval_ready=false risk_level=blocked"
        ));
    }

    #[test]
    fn readiness_summary_ignores_malformed_numeric_tokens() {
        let summary = experience_cleanup_audit_summary(ExperienceCleanupAuditParts {
            limit: 1,
            hygiene: "Noiron experience hygiene\nreport: clean=true quarantine_candidates=4x\nindex_report: noisy_records=2x max_noise_penalty=0.18x retrieval_ready=true risk_level=watch".to_owned(),
            quarantine: "Noiron experience hygiene quarantine dry-run\nquarantine_candidates=5x".to_owned(),
            repair: "Noiron experience repair dry-run\nrepairable_legacy_metadata_lessons=7x repairable_index_records=9x".to_owned(),
        });

        assert!(summary.contains("ready_to_chat=true"));
        assert!(summary.contains("blocking_reasons=none"));
        assert!(summary.contains(
            "index_quality=noisy_records=unknown max_noise_penalty=unknown retrieval_ready=true risk_level=watch"
        ));
    }

    #[test]
    fn readiness_summary_ignores_unknown_index_tokens() {
        let summary = experience_cleanup_audit_summary(ExperienceCleanupAuditParts {
            limit: 1,
            hygiene: "Noiron experience hygiene\nreport: clean=true\nindex_report: retrieval_ready=falsex risk_level=blockedx noisy_records=0 max_noise_penalty=0.0".to_owned(),
            quarantine: "Noiron experience hygiene quarantine dry-run\nquarantine_candidates=0"
                .to_owned(),
            repair: "Noiron experience repair dry-run\nrepairable_legacy_metadata_lessons=0 repairable_index_records=0".to_owned(),
        });

        assert!(summary.contains("ready_to_chat=true"));
        assert!(summary.contains("blocking_reasons=none"));
        assert!(summary.contains(
            "index_quality=noisy_records=0 max_noise_penalty=0.0 retrieval_ready=unknown risk_level=unknown"
        ));
    }

    #[test]
    fn readiness_summary_skips_unknown_index_tokens_before_valid_fallbacks() {
        let summary = experience_cleanup_audit_summary(ExperienceCleanupAuditParts {
            limit: 1,
            hygiene: "Noiron experience hygiene\nreport: clean=true\nindex_report: retrieval_ready=falsex risk_level=blockedx".to_owned(),
            quarantine: "Noiron experience hygiene quarantine dry-run\nquarantine_candidates=0"
                .to_owned(),
            repair: "Noiron experience repair dry-run\nrepairable_legacy_metadata_lessons=0 repairable_index_records=0 index_retrieval_ready=false index_risk_level=blocked".to_owned(),
        });

        assert!(summary.contains("ready_to_chat=false"));
        assert!(summary.contains("index_retrieval_ready=false"));
        assert!(summary.contains("index_risk_level=blocked"));
    }

    #[test]
    fn readiness_summary_skips_malformed_repeated_keys_in_same_input() {
        let summary = experience_cleanup_audit_summary(ExperienceCleanupAuditParts {
            limit: 1,
            hygiene: "Noiron experience hygiene\nreport: clean=true quarantine_candidates=4x quarantine_candidates=4\nindex_report: noisy_records=2x noisy_records=2 max_noise_penalty=0.x max_noise_penalty=0.34 retrieval_ready=falsex retrieval_ready=false risk_level=blockedx risk_level=blocked".to_owned(),
            quarantine: "Noiron experience hygiene quarantine dry-run\nquarantine_candidates=0"
                .to_owned(),
            repair: "Noiron experience repair dry-run\nrepairable_legacy_metadata_lessons=0 repairable_index_records=0".to_owned(),
        });

        assert!(summary.contains("ready_to_chat=false"));
        assert!(summary.contains("blocking_reasons=quarantine_candidates=4"));
        assert!(summary.contains(
            "index_quality=noisy_records=2 max_noise_penalty=0.34 retrieval_ready=false risk_level=blocked"
        ));
    }

    #[test]
    fn readiness_summary_ignores_malformed_clean_tokens() {
        let summary = experience_cleanup_audit_summary(ExperienceCleanupAuditParts {
            limit: 1,
            hygiene: "Noiron experience hygiene\nreport: unclean=false clean=falsex".to_owned(),
            quarantine: "Noiron experience hygiene quarantine dry-run\nquarantine_candidates=0"
                .to_owned(),
            repair: "Noiron experience repair dry-run\nrepairable_legacy_metadata_lessons=0 repairable_index_records=0".to_owned(),
        });

        assert!(summary.contains("ready_to_chat=true"));
        assert!(summary.contains("blocking_reasons=none"));
    }

    #[test]
    fn readiness_summary_ignores_malformed_candidate_id_lines() {
        let summary = experience_cleanup_audit_summary(ExperienceCleanupAuditParts {
            limit: 1,
            hygiene: "Noiron experience hygiene\nreport: clean=true\ncandidate_ids=[861] trailing]"
                .to_owned(),
            quarantine: "Noiron experience hygiene quarantine dry-run\nquarantine_candidates=0\ncandidate_ids=[851x]\ncandidate_ids=[...]\ncandidate_ids=[851,...,...]".to_owned(),
            repair: "Noiron experience repair dry-run\nrepairable_legacy_metadata_lessons=0 repairable_index_records=0".to_owned(),
        });

        let readiness = summary.split("## Hygiene").next().unwrap_or(&summary);
        assert!(!readiness.contains("candidate_ids="));
    }

    #[test]
    fn readiness_summary_accepts_preview_candidate_id_values() {
        let summary = experience_cleanup_audit_summary(ExperienceCleanupAuditParts {
            limit: 1,
            hygiene: "Noiron experience hygiene\nreport: clean=true".to_owned(),
            quarantine:
                "Noiron experience hygiene quarantine dry-run\ncandidate_ids=[0,42,861,...]"
                    .to_owned(),
            repair: "Noiron experience repair dry-run".to_owned(),
        });

        let readiness = summary.split("## Hygiene").next().unwrap_or(&summary);
        assert!(readiness.contains("candidate_ids=[0,42,861,...]"));
    }

    #[test]
    fn rejects_cleanup_audit_response_that_is_not_read_only() {
        let body = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"writes_experience_state\":true,\"sample_limit\":7,\"error\":null,\"report\":null,\"index_report\":null,\"quarantine_plan\":null,\"repair_plan\":null}";

        let error = experience_cleanup_audit_response_summary(body).unwrap_err();

        assert!(error.contains("rejected non-read-only response"));
    }

    #[test]
    fn rejects_cleanup_audit_response_without_read_only_marker() {
        let body = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"sample_limit\":7,\"error\":null,\"report\":null,\"index_report\":null,\"quarantine_plan\":null,\"repair_plan\":null}";

        let error = experience_cleanup_audit_response_summary(body).unwrap_err();

        assert!(error.contains("missing writes_experience_state=false"));
    }

    #[test]
    fn rejects_cleanup_audit_response_with_malformed_read_only_marker() {
        let trueish = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"writes_experience_state\":trueish,\"sample_limit\":7,\"error\":null,\"report\":null,\"index_report\":null,\"quarantine_plan\":null,\"repair_plan\":null}";
        let falsehood = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"writes_experience_state\":falsehood,\"sample_limit\":7,\"error\":null,\"report\":null,\"index_report\":null,\"quarantine_plan\":null,\"repair_plan\":null}";

        let trueish_error = experience_cleanup_audit_response_summary(trueish).unwrap_err();
        let falsehood_error = experience_cleanup_audit_response_summary(falsehood).unwrap_err();

        assert!(trueish_error.contains("missing writes_experience_state=false"));
        assert!(falsehood_error.contains("missing writes_experience_state=false"));
    }

    #[test]
    fn rejects_cleanup_audit_response_with_any_true_read_only_marker() {
        let false_then_true = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"writes_experience_state\":false,\"writes_experience_state\":true,\"sample_limit\":7,\"error\":null,\"report\":null,\"index_report\":null,\"quarantine_plan\":null,\"repair_plan\":null}";
        let true_then_false = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"writes_experience_state\":true,\"writes_experience_state\":false,\"sample_limit\":7,\"error\":null,\"report\":null,\"index_report\":null,\"quarantine_plan\":null,\"repair_plan\":null}";

        let false_then_true_error =
            experience_cleanup_audit_response_summary(false_then_true).unwrap_err();
        let true_then_false_error =
            experience_cleanup_audit_response_summary(true_then_false).unwrap_err();

        assert!(false_then_true_error.contains("rejected non-read-only response"));
        assert!(true_then_false_error.contains("rejected non-read-only response"));
    }

    #[test]
    fn accepts_cleanup_audit_response_with_repeated_false_read_only_markers() {
        let body = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"writes_experience_state\":false,\"writes_experience_state\":false,\"sample_limit\":7,\"error\":null,\"report\":null,\"index_report\":null,\"quarantine_plan\":null,\"repair_plan\":null}";

        let summary = experience_cleanup_audit_response_summary(body).unwrap();

        assert!(summary.contains("writes_experience_state=false"));
    }

    #[test]
    fn accepts_cleanup_audit_response_with_malformed_marker_before_false_marker() {
        let body = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"writes_experience_state\":falsehood,\"writes_experience_state\":false,\"sample_limit\":7,\"error\":null,\"report\":null,\"index_report\":null,\"quarantine_plan\":null,\"repair_plan\":null}";

        let summary = experience_cleanup_audit_response_summary(body).unwrap();

        assert!(summary.contains("writes_experience_state=false"));
    }

    #[test]
    fn ignores_read_only_marker_text_inside_string_values() {
        let body = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"error\":\"escaped \\\"writes_experience_state\\\":true marker\",\"writes_experience_state\":false,\"sample_limit\":7,\"report\":null,\"index_report\":null,\"quarantine_plan\":null,\"repair_plan\":null}";
        let missing = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"error\":\"escaped \\\"writes_experience_state\\\":false marker\",\"sample_limit\":7,\"report\":null,\"index_report\":null,\"quarantine_plan\":null,\"repair_plan\":null}";

        let summary = experience_cleanup_audit_response_summary(body).unwrap();
        let missing_error = experience_cleanup_audit_response_summary(missing).unwrap_err();

        assert!(summary.contains("writes_experience_state=false"));
        assert!(missing_error.contains("missing writes_experience_state=false"));
    }

    #[test]
    fn ignores_non_field_read_only_marker_text() {
        let missing_colon = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"writes_experience_state\" false,\"sample_limit\":7,\"report\":null,\"index_report\":null,\"quarantine_plan\":null,\"repair_plan\":null}";
        let longer_field = "{\"ok\":true,\"request_id\":1,\"experience_file\":\"noiron-experience.ndkv\",\"checked\":true,\"writes_experience_state_extra\":false,\"sample_limit\":7,\"report\":null,\"index_report\":null,\"quarantine_plan\":null,\"repair_plan\":null}";

        let missing_colon_error =
            experience_cleanup_audit_response_summary(missing_colon).unwrap_err();
        let longer_field_error =
            experience_cleanup_audit_response_summary(longer_field).unwrap_err();

        assert!(missing_colon_error.contains("missing writes_experience_state=false"));
        assert!(longer_field_error.contains("missing writes_experience_state=false"));
    }

    #[test]
    fn recognizes_missing_cleanup_audit_endpoint_errors() {
        assert!(cleanup_audit_endpoint_missing(
            "backend /v1/experience-cleanup-audit returned HTTP 400: unsupported HTTP path"
        ));
        assert!(cleanup_audit_endpoint_missing(
            "backend /v1/experience-cleanup-audit returned HTTP 404: not found"
        ));
        assert!(!cleanup_audit_endpoint_missing("connect backend failed"));
    }
}
