use super::report::ReflectionReport;

pub(super) fn should_attempt_repair(report: &ReflectionReport) -> bool {
    report.revision_passes == 0
        && report.critical_issue_count() == 0
        && !report.revision_actions.is_empty()
        && report.quality < 0.72
}

pub(super) fn repair_answer(prompt: &str, answer: &str, report: &ReflectionReport) -> String {
    let base = if answer.is_empty() {
        "The draft did not produce a reliable answer.".to_owned()
    } else {
        dedupe_repeated_words(answer)
    };
    let prompt_anchor = compact(prompt, 96);
    let action_summary = report.revision_actions.join(",");

    format!(
        "{base}\n\nReflection repair: ground the answer in `{prompt_anchor}`; address actions `{action_summary}`; keep the conclusion tentative where confidence is limited."
    )
}

pub(super) fn merged_actions(left: &[String], right: &[String]) -> Vec<String> {
    let mut actions = Vec::new();

    for action in left.iter().chain(right) {
        if !actions.iter().any(|existing| existing == action) {
            actions.push(action.clone());
        }
    }

    actions
}

fn dedupe_repeated_words(text: &str) -> String {
    let mut out = Vec::new();
    let mut previous = "";

    for word in text.split_whitespace() {
        if word != previous {
            out.push(word);
        }
        previous = word;
    }

    if out.is_empty() {
        text.to_owned()
    } else {
        out.join(" ")
    }
}

fn compact(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}
