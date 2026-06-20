use super::evolution_candidate_model::EvolutionCandidate;
use super::status_json::json_string_literal as json_string;

pub(super) fn candidate_id(candidate: &EvolutionCandidate) -> String {
    format!(
        "smartsteam-candidate-{:016x}",
        fnv1a64(
            &[
                candidate.round.as_str(),
                candidate.case_name.as_str(),
                candidate.model.as_str(),
                candidate.answer_preview.as_str(),
            ]
            .join("\n")
        )
    )
}

pub(super) fn candidate_backlog_json(candidate: &EvolutionCandidate, candidate_id: &str) -> String {
    format!(
        "{{\"schema\":\"smartsteam.evolution_candidate.v1\",\"candidate_id\":{},\"status\":\"new\",\"source\":{},\"round\":{},\"case\":{},\"model\":{},\"tokens\":{},\"elapsed_ms\":{},\"feedback\":{},\"self_improve\":{},\"answer_preview\":{}}}",
        json_string(candidate_id),
        json_string(&candidate.source),
        json_string(&candidate.round),
        json_string(&candidate.case_name),
        json_string(&candidate.model),
        json_string(&candidate.tokens),
        json_string(&candidate.elapsed_ms),
        json_string(&candidate.feedback),
        json_string(&candidate.self_improve),
        json_string(&candidate.answer_preview)
    )
}

pub(super) fn candidate_status_event_json(
    candidate_id: &str,
    status: &str,
    note: &str,
    changed_unix: u64,
) -> String {
    format!(
        "{{\"schema\":\"smartsteam.evolution_candidate_status.v1\",\"candidate_id\":{},\"status\":{},\"note\":{},\"changed_unix\":{}}}",
        json_string(candidate_id),
        json_string(status),
        json_string(note),
        changed_unix
    )
}

pub(super) fn candidate_validation_event_json(
    candidate_id: &str,
    command: &str,
    status_code: i32,
    passed: bool,
    note: &str,
    validated_unix: u64,
) -> String {
    format!(
        "{{\"schema\":\"smartsteam.evolution_candidate_validation.v1\",\"candidate_id\":{},\"command\":{},\"status_code\":{},\"passed\":{},\"note\":{},\"validated_unix\":{}}}",
        json_string(candidate_id),
        json_string(command),
        status_code,
        passed,
        json_string(note),
        validated_unix
    )
}

fn fnv1a64(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_candidate() -> EvolutionCandidate {
        EvolutionCandidate {
            source: "report.last".to_owned(),
            round: "9".to_owned(),
            case_name: "smartsteam-evolution-loop-0009".to_owned(),
            model: "google/gemma-4-12B-it".to_owned(),
            tokens: "64".to_owned(),
            elapsed_ms: "999".to_owned(),
            feedback: "4".to_owned(),
            self_improve: "true".to_owned(),
            answer_preview: "**Improvement Candidate:** persist this one".to_owned(),
        }
    }

    #[test]
    fn backlog_event_preserves_schema_and_candidate_payload() {
        let candidate = sample_candidate();
        let json = candidate_backlog_json(&candidate, "smartsteam-candidate-test");

        assert!(json.contains("\"schema\":\"smartsteam.evolution_candidate.v1\""));
        assert!(json.contains("\"candidate_id\":\"smartsteam-candidate-test\""));
        assert!(json.contains("\"status\":\"new\""));
        assert!(json.contains("\"source\":\"report.last\""));
        assert!(
            json.contains("\"answer_preview\":\"**Improvement Candidate:** persist this one\"")
        );
    }

    #[test]
    fn status_and_validation_events_escape_user_text() {
        let status = candidate_status_event_json(
            "smartsteam-candidate-test",
            "accepted",
            "line one\nline two with \"quote\"",
            123,
        );
        let validation = candidate_validation_event_json(
            "smartsteam-candidate-test",
            "cargo test",
            0,
            true,
            "green",
            456,
        );

        assert!(status.contains("\"note\":\"line one\\nline two with \\\"quote\\\"\""));
        assert!(validation.contains("\"passed\":true"));
        assert!(validation.contains("\"validated_unix\":456"));
    }
}
