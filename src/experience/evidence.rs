use super::text_normalize::normalize_full_width_ascii;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExperienceEvidenceNote {
    kind: String,
    tags: Vec<String>,
    fields: Vec<ExperienceEvidenceField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExperienceEvidenceField {
    key: String,
    value: String,
}

impl ExperienceEvidenceNote {
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        let (kind, rest) = split_once_note_separator(raw)?;
        let kind = kind.trim();
        if kind.is_empty() {
            return None;
        }

        let mut tags = Vec::new();
        let mut fields = Vec::<ExperienceEvidenceField>::new();
        for (separator, part) in NoteSegments::new(rest) {
            if part.is_empty() {
                if let Some(field) = fields.last_mut() {
                    if let Some(separator) = separator {
                        field.value.push(separator);
                    }
                }
                continue;
            }

            if let Some((key, value)) = split_once_field_separator(part)
                && let Some(key) = normalized_field_key(key)
            {
                fields.push(ExperienceEvidenceField {
                    key,
                    value: value.to_owned(),
                });
                continue;
            }

            if let Some(field) = fields.last_mut() {
                if let Some(separator) = separator {
                    field.value.push(separator);
                }
                field.value.push_str(part);
            } else {
                tags.push(part.to_owned());
            }
        }

        Some(Self {
            kind: kind.to_owned(),
            tags,
            fields,
        })
    }

    pub(crate) fn is_kind(&self, kind: &str) -> bool {
        normalize_full_width_ascii(self.kind.trim())
            .eq_ignore_ascii_case(&normalize_full_width_ascii(kind.trim()))
    }

    pub(crate) fn first_tag(&self) -> Option<&str> {
        self.tags.first().map(String::as_str)
    }

    pub(crate) fn first_tag_matches(&self, tag: &str) -> bool {
        self.first_tag().is_some_and(|first_tag| {
            normalize_full_width_ascii(first_tag.trim())
                .eq_ignore_ascii_case(&normalize_full_width_ascii(tag.trim()))
        })
    }

    pub(crate) fn field(&self, key: &str) -> Option<&str> {
        let key = normalize_full_width_ascii(key.trim());
        self.fields
            .iter()
            .find(|field| field.key.eq_ignore_ascii_case(&key))
            .map(|field| field.value.as_str())
    }

    pub(crate) fn field_normalized_ascii_trimmed(&self, key: &str) -> Option<String> {
        self.field(key)
            .map(|value| normalize_full_width_ascii(value.trim()))
            .filter(|value| !value.is_empty())
    }

    pub(crate) fn field_matches(&self, key: &str, expected: &str) -> bool {
        self.field(key).is_some_and(|value| {
            normalize_full_width_ascii(value.trim())
                .eq_ignore_ascii_case(&normalize_full_width_ascii(expected.trim()))
        })
    }

    pub(crate) fn field_bool(&self, key: &str) -> Option<bool> {
        let value = normalize_full_width_ascii(self.field(key)?.trim());
        if value.eq_ignore_ascii_case("true") {
            Some(true)
        } else if value.eq_ignore_ascii_case("false") {
            Some(false)
        } else {
            None
        }
    }

    pub(crate) fn field_usize(&self, key: &str) -> Option<usize> {
        normalize_full_width_ascii(self.field(key)?.trim())
            .parse::<usize>()
            .ok()
    }

    pub(crate) fn field_positive_usize(&self, key: &str) -> Option<usize> {
        self.field_usize(key).filter(|value| *value > 0)
    }

    pub(crate) fn field_f32(&self, key: &str) -> Option<f32> {
        normalize_full_width_ascii(self.field(key)?.trim())
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
    }
}

fn split_once_note_separator(value: &str) -> Option<(&str, &str)> {
    let index = value
        .char_indices()
        .find_map(|(index, ch)| note_separator(ch).then_some(index))?;
    let separator_len = value[index..].chars().next()?.len_utf8();
    Some((&value[..index], &value[index + separator_len..]))
}

fn split_once_field_separator(value: &str) -> Option<(&str, &str)> {
    let index = value
        .char_indices()
        .find_map(|(index, ch)| field_separator(ch).then_some(index))?;
    let separator_len = value[index..].chars().next()?.len_utf8();
    Some((&value[..index], &value[index + separator_len..]))
}

struct NoteSegments<'a> {
    rest: &'a str,
    previous_separator: Option<char>,
    emitted_trailing_empty: bool,
}

impl<'a> NoteSegments<'a> {
    fn new(rest: &'a str) -> Self {
        Self {
            rest,
            previous_separator: None,
            emitted_trailing_empty: false,
        }
    }
}

impl<'a> Iterator for NoteSegments<'a> {
    type Item = (Option<char>, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            if self.previous_separator.is_some() && !self.emitted_trailing_empty {
                self.emitted_trailing_empty = true;
                return Some((self.previous_separator.take(), ""));
            }
            return None;
        }

        let separator = self.previous_separator.take();
        if let Some((index, ch)) = self.rest.char_indices().find(|(_, ch)| note_separator(*ch)) {
            let part = &self.rest[..index];
            self.rest = &self.rest[index + ch.len_utf8()..];
            self.previous_separator = Some(ch);
            Some((separator, part))
        } else {
            let part = self.rest;
            self.rest = "";
            Some((separator, part))
        }
    }
}

fn note_separator(ch: char) -> bool {
    matches!(ch, ':' | '：')
}

fn field_separator(ch: char) -> bool {
    matches!(ch, '=' | '＝')
}

fn normalized_field_key(value: &str) -> Option<String> {
    let value = normalize_full_width_ascii(value.trim());
    is_normalized_field_key(&value).then_some(value)
}

fn is_normalized_field_key(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub(crate) fn evidence_notes_by_kind<'a>(
    notes: &'a [String],
    kind: &'a str,
) -> impl Iterator<Item = ExperienceEvidenceNote> + 'a {
    notes
        .iter()
        .filter_map(|note| ExperienceEvidenceNote::parse(note))
        .filter(move |note| note.is_kind(kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_source_tag_and_numeric_fields() {
        let note = ExperienceEvidenceNote::parse(
            "memory_feedback:rust_check:reinforced=2:penalized=0:strength_delta=0.180000",
        )
        .unwrap();

        assert!(note.is_kind("memory_feedback"));
        assert_eq!(note.first_tag(), Some("rust_check"));
        assert_eq!(note.field_usize("reinforced"), Some(2));
        assert_eq!(note.field_f32("strength_delta"), Some(0.18));
    }

    #[test]
    fn keeps_colons_inside_field_values() {
        let note = ExperienceEvidenceNote::parse(
            "pool_dispatch:selected_role=review:selected_endpoint=http://127.0.0.1:8688:forwarded=true",
        )
        .unwrap();

        assert_eq!(note.field("selected_role"), Some("review"));
        assert_eq!(
            note.field("selected_endpoint"),
            Some("http://127.0.0.1:8688")
        );
        assert_eq!(note.field_bool("forwarded"), Some(true));
    }

    #[test]
    fn keeps_equals_inside_colon_continuation_values() {
        let note = ExperienceEvidenceNote::parse(
            "trace:url=http://example.test/callback?next=http://127.0.0.1:8688/path?x=1:handled=true",
        )
        .unwrap();

        assert_eq!(
            note.field("url"),
            Some("http://example.test/callback?next=http://127.0.0.1:8688/path?x=1")
        );
        assert_eq!(note.field_bool("handled"), Some(true));
        assert!(note.field("8688/path?x").is_none());
    }

    #[test]
    fn preserves_empty_colon_segments_inside_field_values() {
        let note = ExperienceEvidenceNote::parse("trace:path=alpha::beta:tail=value:").unwrap();

        assert_eq!(note.field("path"), Some("alpha::beta"));
        assert_eq!(note.field("tail"), Some("value:"));
    }

    #[test]
    fn ignores_empty_tag_segments_before_first_field() {
        let note = ExperienceEvidenceNote::parse("trace::runtime:key=value").unwrap();

        assert_eq!(note.first_tag(), Some("runtime"));
        assert_eq!(note.field("key"), Some("value"));
    }

    #[test]
    fn matches_kind_and_fields_case_insensitively() {
        let notes = vec!["Recursive:Runtime_Calls= 13 :Waves= 2 ".to_owned()];
        let note = evidence_notes_by_kind(&notes, "recursive").next().unwrap();

        assert!(note.is_kind("recursive"));
        assert_eq!(note.field_positive_usize("runtime_calls"), Some(13));
        assert_eq!(note.field_positive_usize("waves"), Some(2));
    }

    #[test]
    fn parses_full_width_note_and_field_separators() {
        let note = ExperienceEvidenceNote::parse(
            "Recursive：Chunks＝ 8 ：Merge_Rounds＝2：Waves=4:Runtime_Calls＝ 13 ",
        )
        .unwrap();

        assert!(note.is_kind("recursive"));
        assert_eq!(note.field_positive_usize("chunks"), Some(8));
        assert_eq!(note.field_positive_usize("merge_rounds"), Some(2));
        assert_eq!(note.field_positive_usize("waves"), Some(4));
        assert_eq!(note.field_positive_usize("runtime_calls"), Some(13));
    }

    #[test]
    fn keeps_full_width_colons_inside_field_values() {
        let note = ExperienceEvidenceNote::parse(
            "trace：message＝错误：需要压缩上下文：retry=true：handled＝true",
        )
        .unwrap();

        assert_eq!(note.field("message"), Some("错误：需要压缩上下文"));
        assert_eq!(note.field_bool("retry"), Some(true));
        assert_eq!(note.field_bool("handled"), Some(true));
    }

    #[test]
    fn matches_first_tag_case_insensitively_after_trimming() {
        let note =
            ExperienceEvidenceNote::parse("memory_feedback: Rust_Check :reinforced=2:penalized=0")
                .unwrap();

        assert_eq!(note.first_tag(), Some(" Rust_Check "));
        assert!(note.first_tag_matches("rust_check"));
        assert!(note.first_tag_matches(" RUST_CHECK "));
        assert!(!note.first_tag_matches("business_contract"));
    }

    #[test]
    fn typed_fields_trim_values_without_changing_raw_field_access() {
        let note = ExperienceEvidenceNote::parse(
            "runtime_error:timeout= TRUE :handled= False :normalization= Sanitized :message_chars= 42 :ratio= 0.25 ",
        )
        .unwrap();

        assert_eq!(note.field("message_chars"), Some(" 42 "));
        assert_eq!(
            note.field_normalized_ascii_trimmed("message_chars"),
            Some("42".to_owned())
        );
        assert_eq!(note.field_bool("TIMEOUT"), Some(true));
        assert_eq!(note.field_bool("handled"), Some(false));
        assert!(note.field_matches("normalization", "sanitized"));
        assert!(note.field_matches("normalization", " SANITIZED "));
        assert_eq!(note.field_usize("MESSAGE_CHARS"), Some(42));
        assert_eq!(note.field_f32("ratio"), Some(0.25));
    }

    #[test]
    fn typed_fields_accept_full_width_ascii_values_without_changing_raw_access() {
        let note = ExperienceEvidenceNote::parse(
            "rust_check：passed＝ ｔｒｕｅ ：normalization＝ Ｓａｎｉｔｉｚｅｄ ：message_chars＝ ４２ ：ratio＝ ０．２５ ",
        )
        .unwrap();

        assert_eq!(note.field("message_chars"), Some(" ４２ "));
        assert_eq!(
            note.field_normalized_ascii_trimmed("message_chars"),
            Some("42".to_owned())
        );
        assert_eq!(note.field_bool("passed"), Some(true));
        assert!(note.field_matches("normalization", "sanitized"));
        assert_eq!(note.field_usize("message_chars"), Some(42));
        assert_eq!(note.field_f32("ratio"), Some(0.25));
    }

    #[test]
    fn matches_full_width_ascii_kind_tags_and_field_keys() {
        let notes = vec![
            "ｒｕｓｔ＿ｃｈｅｃｋ： Ｍｅｍｏｒｙ＿Ｆｅｅｄｂａｃｋ ：ｐａｓｓｅｄ＝ｔｒｕｅ：ｄｉａｇｎｏｓｔｉｃ＿ｃｈａｒｓ＝４２"
                .to_owned(),
        ];
        let note = evidence_notes_by_kind(&notes, "rust_check").next().unwrap();

        assert!(note.is_kind("RUST_CHECK"));
        assert!(note.first_tag_matches("memory_feedback"));
        assert_eq!(note.field("diagnostic_chars"), Some("４２"));
        assert_eq!(note.field("ｄｉａｇｎｏｓｔｉｃ＿ｃｈａｒｓ"), Some("４２"));
        assert_eq!(note.field_bool("passed"), Some(true));
        assert_eq!(note.field_usize("diagnostic_chars"), Some(42));
    }
}
