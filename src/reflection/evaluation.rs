use super::metrics::{
    contains_conflicting_markers, lexical_overlap, repetition_ratio, token_uncertainty,
};
use super::model::InferenceDraft;
use super::report::{ReflectionIssue, ReflectionReport, ReflectionSeverity};

const LESSON_BODY_CHARS: usize = 280;

pub(super) fn evaluate_draft(
    prompt: &str,
    draft: &InferenceDraft,
    min_answer_chars: usize,
    revision_passes: usize,
) -> ReflectionReport {
    let mut issues = Vec::new();
    let mut revision_actions = Vec::new();
    let answer = draft.answer.trim();

    collect_answer_issues(answer, min_answer_chars, &mut issues, &mut revision_actions);
    collect_trace_issues(draft, &mut issues, &mut revision_actions);

    let average_confidence = average_trace_confidence(draft);
    let overlap = lexical_overlap(prompt, answer);
    if !answer.is_empty() && overlap < 0.12 {
        add_issue(
            &mut issues,
            &mut revision_actions,
            "low_prompt_overlap",
            ReflectionSeverity::Warning,
            format!("lexical overlap {:.3} below grounding threshold", overlap),
            "increase_prompt_grounding",
        );
    }

    let repetition = repetition_ratio(answer);
    if repetition >= 0.58 {
        add_issue(
            &mut issues,
            &mut revision_actions,
            "repetitive_answer",
            ReflectionSeverity::Warning,
            format!("unique-token repetition ratio {:.3}", repetition),
            "deduplicate_repeated_phrases",
        );
    }

    if let Some(uncertainty) = token_uncertainty(&draft.tokens)
        && uncertainty >= 0.72
    {
        add_issue(
            &mut issues,
            &mut revision_actions,
            "high_token_uncertainty",
            ReflectionSeverity::Warning,
            format!("token uncertainty {:.3}", uncertainty),
            "increase_attention_or_resample",
        );
    }

    let scores = ReflectionScores::new(answer, overlap, average_confidence, &issues);
    build_report(answer, issues, revision_actions, revision_passes, scores)
}

fn collect_answer_issues(
    answer: &str,
    min_answer_chars: usize,
    issues: &mut Vec<ReflectionIssue>,
    revision_actions: &mut Vec<String>,
) {
    if answer.is_empty() {
        add_issue(
            issues,
            revision_actions,
            "empty_answer",
            ReflectionSeverity::Critical,
            "draft answer is empty",
            "reject_empty_answer",
        );
    }
    if answer.chars().count() < min_answer_chars {
        add_issue(
            issues,
            revision_actions,
            "answer_too_short",
            ReflectionSeverity::Warning,
            "draft answer is shorter than the configured minimum",
            "expand_short_answer",
        );
    }
    if contains_conflicting_markers(answer) {
        add_issue(
            issues,
            revision_actions,
            "conflicting_certainty_markers",
            ReflectionSeverity::Critical,
            "answer mixes certainty and uncertainty markers",
            "mark_tentative_for_conflicting_certainty",
        );
    }
}

fn collect_trace_issues(
    draft: &InferenceDraft,
    issues: &mut Vec<ReflectionIssue>,
    revision_actions: &mut Vec<String>,
) {
    for step in &draft.trace {
        if step.confidence < 0.28 {
            add_issue(
                issues,
                revision_actions,
                format!("low_confidence_step:{}", step.label),
                if step.confidence < 0.14 {
                    ReflectionSeverity::Critical
                } else {
                    ReflectionSeverity::Warning
                },
                format!("reasoning step confidence {:.3}", step.confidence),
                format!("review_low_confidence_step:{}", step.label),
            );
        }
    }
}

fn average_trace_confidence(draft: &InferenceDraft) -> f32 {
    let scored_steps = draft
        .trace
        .iter()
        .filter(|step| step.label != "runtime_adapter_selection")
        .collect::<Vec<_>>();
    if scored_steps.is_empty() {
        0.5
    } else {
        scored_steps.iter().map(|step| step.confidence).sum::<f32>() / scored_steps.len() as f32
    }
}

fn build_report(
    answer: &str,
    issues: Vec<ReflectionIssue>,
    revision_actions: Vec<String>,
    revision_passes: usize,
    scores: ReflectionScores,
) -> ReflectionReport {
    let contradictions = issues
        .iter()
        .filter(|issue| issue.severity == ReflectionSeverity::Critical)
        .map(|issue| issue.code.clone())
        .collect::<Vec<_>>();
    let revised_answer = revised_answer(answer, &contradictions, &revision_actions, &issues);
    let critical_issue_count = contradictions.len();
    let store_as_memory = scores.quality >= 0.46
        && critical_issue_count == 0
        && issues.len() <= 2
        && answer.len() >= 24;
    let max_severity = issues
        .iter()
        .map(|issue| issue.severity)
        .max_by_key(|severity| severity.rank())
        .unwrap_or(ReflectionSeverity::Info);
    let lesson = lesson(
        answer,
        store_as_memory,
        issues.len(),
        critical_issue_count,
        max_severity,
        &revision_actions,
        &scores,
    );

    ReflectionReport {
        quality: scores.quality,
        contradictions,
        issues,
        revision_actions,
        revision_passes,
        revised_answer,
        store_as_memory,
        lesson,
    }
}

fn revised_answer(
    answer: &str,
    contradictions: &[String],
    revision_actions: &[String],
    issues: &[ReflectionIssue],
) -> String {
    if issues.is_empty() {
        return answer.to_owned();
    }

    let answer_for_note = if answer.is_empty() {
        "[empty draft]"
    } else {
        answer
    };
    format!(
        "{answer_for_note}\n\nReflection note: flagged {}; actions {}; keep this response as tentative.",
        contradictions.join(","),
        revision_actions.join(",")
    )
}

fn lesson(
    answer: &str,
    store_as_memory: bool,
    issue_count: usize,
    critical_issue_count: usize,
    max_severity: ReflectionSeverity,
    revision_actions: &[String],
    scores: &ReflectionScores,
) -> String {
    if store_as_memory {
        format!(
            "reuse_response: {} [reflection accepted q={:.3} ov={:.3} issues={} severity={}]",
            compact_lesson_body(answer, LESSON_BODY_CHARS),
            scores.quality,
            scores.overlap,
            issue_count,
            max_severity.as_str()
        )
    } else {
        format!(
            "revise_response: {} [reflection rejected q={:.3} issues={} critical={} severity={} actions={}]",
            compact_lesson_body(answer, LESSON_BODY_CHARS),
            scores.quality,
            issue_count,
            critical_issue_count,
            max_severity.as_str(),
            revision_actions.join(",")
        )
    }
}

fn compact_lesson_body(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut previous_space = false;
    for ch in value.trim().chars().take(max_chars) {
        if ch.is_whitespace() {
            if !previous_space {
                out.push(' ');
                previous_space = true;
            }
        } else {
            out.push(ch);
            previous_space = false;
        }
    }
    if value.trim().chars().count() > max_chars {
        out.push_str("...");
    }
    if out.is_empty() {
        "empty draft".to_owned()
    } else {
        out
    }
}

fn add_issue(
    issues: &mut Vec<ReflectionIssue>,
    revision_actions: &mut Vec<String>,
    code: impl Into<String>,
    severity: ReflectionSeverity,
    detail: impl Into<String>,
    action: impl Into<String>,
) {
    issues.push(ReflectionIssue::new(code, severity, detail));
    revision_actions.push(action.into());
}

struct ReflectionScores {
    quality: f32,
    overlap: f32,
}

impl ReflectionScores {
    fn new(
        answer: &str,
        overlap: f32,
        average_confidence: f32,
        issues: &[ReflectionIssue],
    ) -> Self {
        let length_score = (answer.chars().count() as f32 / 240.0).min(1.0);
        let penalty = issues
            .iter()
            .map(|issue| issue.severity.penalty())
            .sum::<f32>()
            .min(0.72);
        let quality = (overlap * 0.42 + average_confidence * 0.38 + length_score * 0.20 - penalty)
            .clamp(0.0, 1.0);

        Self { quality, overlap }
    }
}
