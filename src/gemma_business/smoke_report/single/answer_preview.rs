use crate::gemma_business::response_json::response_optional_string_field;

use super::super::preview::compact_business_answer_preview;

const SINGLE_REPORT_ANSWER_PREVIEW_CHARS: usize = 180;

pub(super) fn single_report_answer_preview(cycle_body: &str) -> String {
    response_optional_string_field(cycle_body, "answer")
        .map(|answer| compact_business_answer_preview(&answer, SINGLE_REPORT_ANSWER_PREVIEW_CHARS))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::single_report_answer_preview;

    #[test]
    fn single_report_answer_preview_compacts_answer_field() {
        let body = r#"{"answer":"line one\n\nline two"}"#;

        assert_eq!(single_report_answer_preview(body), "answer_chars=18");
        assert!(!single_report_answer_preview(body).contains("line one"));
    }

    #[test]
    fn single_report_answer_preview_defaults_missing_answer() {
        assert_eq!(single_report_answer_preview("{}"), "");
    }
}
