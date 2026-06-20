pub(super) fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

pub(super) fn require_line(lines: &[&str], index: usize, expected: &str) -> Result<(), String> {
    match lines.get(index) {
        Some(line) if *line == expected => Ok(()),
        Some(line) => Err(format!(
            "model pool smoke line {index} expected {expected:?}, got {line:?}"
        )),
        None => Err(format!(
            "model pool smoke missing line {index} expected {expected:?}"
        )),
    }
}

pub(super) fn require_bool_prefix(
    lines: &[&str],
    index: usize,
    prefix: &str,
) -> Result<bool, String> {
    let Some(line) = lines.get(index) else {
        return Err(format!(
            "model pool smoke missing line {index} expected {prefix}<bool>"
        ));
    };
    let Some(value) = line.strip_prefix(prefix) else {
        return Err(format!(
            "model pool smoke line {index} expected prefix {prefix:?}, got {line:?}"
        ));
    };
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!(
            "model pool smoke line {index} expected boolean for {prefix:?}, got {value:?}"
        )),
    }
}

pub(super) fn require_ordered_sections(lines: &[&str], sections: &[&str]) -> Result<(), String> {
    let mut cursor = 0usize;
    for section in sections {
        let Some(offset) = lines.iter().skip(cursor).position(|line| line == section) else {
            return Err(format!("model pool smoke missing {section}"));
        };
        cursor += offset + 1;
    }
    Ok(())
}

pub(super) fn require_section_body<'a>(
    lines: &'a [&str],
    section: &str,
) -> Result<&'a str, String> {
    let Some(index) = lines.iter().position(|line| *line == section) else {
        return Err(format!("model pool smoke missing {section}"));
    };
    let Some(body) = lines.get(index + 1) else {
        return Err(format!("model pool smoke missing body for {section}"));
    };
    if body.starts_with("section=") {
        return Err(format!("model pool smoke missing body for {section}"));
    }
    Ok(body)
}

pub(super) fn require_section_lines<'a>(
    lines: &'a [&'a str],
    section: &str,
) -> Result<&'a [&'a str], String> {
    let Some(index) = lines.iter().position(|line| *line == section) else {
        return Err(format!("model pool smoke missing {section}"));
    };
    let start = index + 1;
    let end = lines
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(index, line)| line.starts_with("section=").then_some(index))
        .unwrap_or(lines.len());
    if start >= end {
        return Err(format!("model pool smoke missing body for {section}"));
    }
    Ok(&lines[start..end])
}

pub(super) fn require_section_lines_before<'a>(
    lines: &'a [&'a str],
    section: &str,
    next_section: &str,
) -> Result<&'a [&'a str], String> {
    let Some(index) = lines.iter().position(|line| *line == section) else {
        return Err(format!("model pool smoke missing {section}"));
    };
    let start = index + 1;
    let Some(end_offset) = lines
        .iter()
        .skip(start)
        .position(|line| *line == next_section)
    else {
        return Err(format!(
            "model pool smoke missing {next_section} after {section}"
        ));
    };
    let end = start + end_offset;
    if start >= end {
        return Err(format!("model pool smoke missing body for {section}"));
    }
    Ok(&lines[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_line_accepts_exact_line() {
        let lines = ["header", "read_only=true"];

        require_line(&lines, 1, "read_only=true").unwrap();
    }

    #[test]
    fn require_line_reports_wrong_or_missing_line() {
        let lines = ["header"];

        assert!(
            require_line(&lines, 0, "other")
                .unwrap_err()
                .contains("expected \"other\", got \"header\"")
        );
        assert!(
            require_line(&lines, 2, "missing")
                .unwrap_err()
                .contains("missing line 2 expected \"missing\"")
        );
    }

    #[test]
    fn require_bool_prefix_parses_strict_bool_values() {
        let lines = ["smoke_alignment_ok=true", "smoke_alignment_ok=false"];

        assert!(require_bool_prefix(&lines, 0, "smoke_alignment_ok=").unwrap());
        assert!(!require_bool_prefix(&lines, 1, "smoke_alignment_ok=").unwrap());
    }

    #[test]
    fn require_bool_prefix_rejects_wrong_prefix_and_non_bool() {
        let lines = ["alignment_ok=true", "smoke_alignment_ok=yes"];

        assert!(
            require_bool_prefix(&lines, 0, "smoke_alignment_ok=")
                .unwrap_err()
                .contains("expected prefix \"smoke_alignment_ok=\"")
        );
        assert!(
            require_bool_prefix(&lines, 1, "smoke_alignment_ok=")
                .unwrap_err()
                .contains("expected boolean")
        );
    }

    #[test]
    fn require_ordered_sections_allows_content_between_sections() {
        let lines = [
            "section=contract_json",
            "{}",
            "section=manifest",
            "manifest",
            "section=status",
        ];

        require_ordered_sections(
            &lines,
            &[
                "section=contract_json",
                "section=manifest",
                "section=status",
            ],
        )
        .unwrap();
    }

    #[test]
    fn require_ordered_sections_rejects_missing_or_out_of_order_sections() {
        let lines = ["section=status", "section=manifest"];

        assert!(
            require_ordered_sections(&lines, &["section=manifest", "section=status"])
                .unwrap_err()
                .contains("missing section=status")
        );
    }

    #[test]
    fn require_section_body_returns_next_non_section_line() {
        let lines = ["section=status", "status body", "section=advice"];

        assert_eq!(
            require_section_body(&lines, "section=status").unwrap(),
            "status body"
        );
    }

    #[test]
    fn require_section_body_rejects_missing_empty_body() {
        let lines = ["section=status", "section=advice"];

        assert!(
            require_section_body(&lines, "section=status")
                .unwrap_err()
                .contains("missing body for section=status")
        );
        assert!(
            require_section_body(&lines, "section=missing")
                .unwrap_err()
                .contains("missing section=missing")
        );
    }

    #[test]
    fn require_section_lines_returns_multiline_body_until_next_section() {
        let lines = [
            "section=alignment",
            "alignment_ok=true",
            "manifest_roles=quality,summary",
            "section=routes",
            "route body",
        ];

        assert_eq!(
            require_section_lines(&lines, "section=alignment").unwrap(),
            &["alignment_ok=true", "manifest_roles=quality,summary"]
        );
    }

    #[test]
    fn require_section_lines_rejects_missing_empty_body() {
        let lines = ["section=alignment", "section=routes"];

        assert!(
            require_section_lines(&lines, "section=alignment")
                .unwrap_err()
                .contains("missing body for section=alignment")
        );
        assert!(
            require_section_lines(&lines, "section=missing")
                .unwrap_err()
                .contains("missing section=missing")
        );
    }

    #[test]
    fn require_section_lines_before_allows_nested_sections_until_named_boundary() {
        let lines = [
            "section=advice",
            "SmartSteam Apple model pool advice",
            "section=advice_json",
            "{}",
            "section=alignment_json",
            "{}",
        ];

        assert_eq!(
            require_section_lines_before(&lines, "section=advice", "section=alignment_json")
                .unwrap(),
            &[
                "SmartSteam Apple model pool advice",
                "section=advice_json",
                "{}"
            ]
        );
    }

    #[test]
    fn require_section_lines_before_rejects_missing_boundary_or_body() {
        let missing_boundary = ["section=advice", "advice body"];
        let missing_body = ["section=advice", "section=alignment_json"];

        assert!(
            require_section_lines_before(
                &missing_boundary,
                "section=advice",
                "section=alignment_json"
            )
            .unwrap_err()
            .contains("missing section=alignment_json after section=advice")
        );
        assert!(
            require_section_lines_before(&missing_body, "section=advice", "section=alignment_json")
                .unwrap_err()
                .contains("missing body for section=advice")
        );
    }
}
