use crate::{
    ExperienceEnvelope, MemoryAdapter, MemoryAdapterCapability, MemoryAdapterDescriptor,
    MemoryAdapterHealth, MemoryResult, clamp01,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GistLevel {
    Document,
    Section,
    Paragraph,
}

impl GistLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Section => "section",
            Self::Paragraph => "paragraph",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Document => 0,
            Self::Section => 1,
            Self::Paragraph => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryGist {
    pub level: GistLevel,
    pub title: String,
    pub summary: String,
    pub source_tokens: usize,
    pub importance: f32,
}

impl MemoryGist {
    pub fn new(level: GistLevel, title: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            level,
            title: title.into(),
            summary: summary.into(),
            source_tokens: 0,
            importance: 0.5,
        }
    }

    pub fn with_source_tokens(mut self, source_tokens: usize) -> Self {
        self.source_tokens = source_tokens;
        self
    }

    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = clamp01(importance);
        self
    }

    pub fn hint(&self) -> String {
        format!(
            "{}:{} importance={:.3} tokens={} summary={}",
            self.level.as_str(),
            self.title,
            self.importance,
            self.source_tokens,
            self.summary
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanGistPolicy {
    pub max_chars: usize,
    pub min_signal_chars: usize,
}

impl Default for CleanGistPolicy {
    fn default() -> Self {
        Self {
            max_chars: 420,
            min_signal_chars: 12,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CleanGistSelectionReport {
    pub candidate_count: usize,
    pub selected: bool,
    pub selected_level: Option<GistLevel>,
    pub selected_source_tokens: usize,
    pub selected_importance: f32,
    pub rejected_empty: usize,
    pub rejected_transcript: usize,
    pub rejected_metadata: usize,
    pub rejected_low_signal: usize,
}

impl CleanGistSelectionReport {
    pub fn reason_codes(&self) -> Vec<String> {
        let mut codes = Vec::new();
        if self.selected {
            codes.push("selected".to_owned());
        } else {
            codes.push("no_selection".to_owned());
        }
        if self.rejected_empty > 0 {
            codes.push("rejected_empty".to_owned());
        }
        if self.rejected_transcript > 0 {
            codes.push("rejected_transcript".to_owned());
        }
        if self.rejected_metadata > 0 {
            codes.push("rejected_metadata".to_owned());
        }
        if self.rejected_low_signal > 0 {
            codes.push("rejected_low_signal".to_owned());
        }
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn detail_codes(&self) -> Vec<String> {
        let mut codes = Vec::new();
        match self.selected_level {
            Some(level) if self.selected => {
                codes.push(format!("selected_level:{}", level.as_str()));
            }
            _ => codes.push("selected:none".to_owned()),
        }
        if self.rejected_empty > 0 {
            codes.push("rejected_empty".to_owned());
        }
        if self.rejected_transcript > 0 {
            codes.push("rejected_transcript".to_owned());
        }
        if self.rejected_metadata > 0 {
            codes.push("rejected_metadata".to_owned());
        }
        if self.rejected_low_signal > 0 {
            codes.push("rejected_low_signal".to_owned());
        }
        codes.sort();
        codes.dedup();
        codes
    }

    pub fn summary_line(&self) -> String {
        format!(
            "clean_gist_selection candidates={} selected={} selected_level={} selected_tokens={} selected_importance={:.3} rejected_empty={} rejected_transcript={} rejected_metadata={} rejected_low_signal={} reason_codes={} detail_codes={}",
            self.candidate_count,
            self.selected,
            self.selected_level.map(GistLevel::as_str).unwrap_or("none"),
            self.selected_source_tokens,
            self.selected_importance,
            self.rejected_empty,
            self.rejected_transcript,
            self.rejected_metadata,
            self.rejected_low_signal,
            join_codes(self.reason_codes()),
            join_codes(self.detail_codes()),
        )
    }
}

pub trait CleanGistSelector {
    fn best_clean_gist(&self, gists: &[MemoryGist]) -> Option<String>;

    fn attach_clean_gist(
        &self,
        envelope: ExperienceEnvelope,
        gists: &[MemoryGist],
    ) -> ExperienceEnvelope {
        match self.best_clean_gist(gists) {
            Some(gist) => envelope.with_clean_gist(gist),
            None => envelope,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DefaultCleanGistSelector {
    pub policy: CleanGistPolicy,
}

impl DefaultCleanGistSelector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn selection_report(&self, gists: &[MemoryGist]) -> CleanGistSelectionReport {
        let mut rejected_empty = 0;
        let mut rejected_transcript = 0;
        let mut rejected_metadata = 0;
        let mut rejected_low_signal = 0;
        let mut selected = None::<(&MemoryGist, String)>;

        for gist in gists {
            match clean_gist_text_with_rejection(&gist.summary, self.policy) {
                Ok(text) => {
                    let replace = selected.as_ref().is_none_or(|(current, _)| {
                        gist.importance
                            .partial_cmp(&current.importance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| current.level.rank().cmp(&gist.level.rank()))
                            .then_with(|| gist.source_tokens.cmp(&current.source_tokens))
                            .is_gt()
                    });
                    if replace {
                        selected = Some((gist, text));
                    }
                }
                Err(GistRejectionReason::Empty) => rejected_empty += 1,
                Err(GistRejectionReason::TranscriptShape) => rejected_transcript += 1,
                Err(GistRejectionReason::MetadataShape) => rejected_metadata += 1,
                Err(GistRejectionReason::LowSignal) => rejected_low_signal += 1,
            }
        }

        let selected_gist = selected.as_ref().map(|(gist, _)| *gist);
        CleanGistSelectionReport {
            candidate_count: gists.len(),
            selected: selected_gist.is_some(),
            selected_level: selected_gist.map(|gist| gist.level),
            selected_source_tokens: selected_gist.map_or(0, |gist| gist.source_tokens),
            selected_importance: selected_gist.map_or(0.0, |gist| gist.importance),
            rejected_empty,
            rejected_transcript,
            rejected_metadata,
            rejected_low_signal,
        }
    }
}

impl CleanGistSelector for DefaultCleanGistSelector {
    fn best_clean_gist(&self, gists: &[MemoryGist]) -> Option<String> {
        gists
            .iter()
            .filter_map(|gist| clean_gist_text(&gist.summary, self.policy).map(|text| (gist, text)))
            .max_by(|(left, _), (right, _)| {
                left.importance
                    .partial_cmp(&right.importance)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| right.level.rank().cmp(&left.level.rank()))
                    .then_with(|| right.source_tokens.cmp(&left.source_tokens))
            })
            .map(|(_, text)| text)
    }
}

impl MemoryAdapter for DefaultCleanGistSelector {
    fn descriptor(&self) -> MemoryAdapterDescriptor {
        MemoryAdapterDescriptor::new(
            "default_clean_gist_selector",
            vec![MemoryAdapterCapability::CleanGistSelection],
        )
        .read_only()
    }

    fn health(&self) -> MemoryResult<MemoryAdapterHealth> {
        let mut warnings = Vec::new();
        if self.policy.max_chars == 0 {
            warnings.push("max_chars_zero".to_owned());
        }
        if self.policy.min_signal_chars == 0 {
            warnings.push("min_signal_chars_zero".to_owned());
        }
        Ok(MemoryAdapterHealth {
            ready: warnings.is_empty(),
            record_count: None,
            warnings,
        })
    }
}

fn clean_gist_text(value: &str, policy: CleanGistPolicy) -> Option<String> {
    clean_gist_text_with_rejection(value, policy).ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GistRejectionReason {
    Empty,
    TranscriptShape,
    MetadataShape,
    LowSignal,
}

fn clean_gist_text_with_rejection(
    value: &str,
    policy: CleanGistPolicy,
) -> Result<String, GistRejectionReason> {
    let trimmed = strip_generated_prefixes(value.trim());
    if trimmed.is_empty() {
        return Err(GistRejectionReason::Empty);
    }
    if has_transcript_shape(trimmed) {
        return Err(GistRejectionReason::TranscriptShape);
    }
    if has_metadata_lesson_shape(trimmed) {
        return Err(GistRejectionReason::MetadataShape);
    }

    let compact = compact_chars(trimmed, policy.max_chars);
    let signal_chars = compact
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .take(policy.min_signal_chars)
        .count();
    if signal_chars >= policy.min_signal_chars {
        Ok(compact)
    } else {
        Err(GistRejectionReason::LowSignal)
    }
}

fn join_codes(codes: Vec<String>) -> String {
    if codes.is_empty() {
        "none".to_owned()
    } else {
        codes.join("|")
    }
}

fn strip_generated_prefixes(value: &str) -> &str {
    value
        .strip_prefix("asthought ")
        .or_else(|| value.strip_prefix("asthought:"))
        .unwrap_or(value)
        .trim_start()
}

fn compact_chars(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut previous_space = false;
    for ch in value.chars().take(max_chars) {
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
    out.trim().to_owned()
}

fn has_transcript_shape(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("conversation transcript:")
        || (value.contains("user:") && value.contains("assistant:"))
}

fn has_metadata_lesson_shape(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.starts_with("accepted_pattern ")
        || value.starts_with("rejected_pattern ")
        || ((value.contains("quality=") || value.contains("overlap="))
            && value.contains("max_severity="))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExperienceEnvelope;

    #[test]
    fn selector_chooses_highest_importance_clean_gist() {
        let gists = vec![
            MemoryGist::new(
                GistLevel::Document,
                "dirty",
                "Conversation Transcript:\nUser: stale\nAssistant: stale",
            )
            .with_importance(0.99),
            MemoryGist::new(
                GistLevel::Section,
                "clean",
                "Use scoped governance before injecting retrieved memory.",
            )
            .with_importance(0.7),
            MemoryGist::new(
                GistLevel::Paragraph,
                "cleaner",
                "A lower value clean gist should not win this selection.",
            )
            .with_importance(0.2),
        ];

        let selected = DefaultCleanGistSelector::new()
            .best_clean_gist(&gists)
            .unwrap();
        assert_eq!(
            selected,
            "Use scoped governance before injecting retrieved memory."
        );
    }

    #[test]
    fn selector_strips_generated_prefix_and_attaches_to_envelope() {
        let gists = vec![
            MemoryGist::new(
                GistLevel::Document,
                "summary",
                "asthought: Prefer copied fixtures when validating memory repair.",
            )
            .with_importance(0.8),
        ];
        let envelope = ExperienceEnvelope::new("1", "prompt", "lesson");

        let envelope = DefaultCleanGistSelector::new().attach_clean_gist(envelope, &gists);
        assert_eq!(
            envelope.clean_gist.as_deref(),
            Some("Prefer copied fixtures when validating memory repair.")
        );
    }

    #[test]
    fn selector_rejects_short_or_metadata_gists() {
        let gists = vec![
            MemoryGist::new(GistLevel::Document, "short", "tiny"),
            MemoryGist::new(
                GistLevel::Section,
                "metadata",
                "accepted_pattern quality=0.9 max_severity=watch",
            ),
        ];

        assert!(
            DefaultCleanGistSelector::new()
                .best_clean_gist(&gists)
                .is_none()
        );
    }

    #[test]
    fn selection_report_summarizes_rejections_without_gist_text() {
        let gists = vec![
            MemoryGist::new(GistLevel::Document, "empty", "   "),
            MemoryGist::new(
                GistLevel::Section,
                "transcript",
                "Conversation Transcript:\nUser: stale\nAssistant: stale",
            ),
            MemoryGist::new(
                GistLevel::Section,
                "metadata",
                "accepted_pattern quality=0.9 max_severity=watch",
            ),
            MemoryGist::new(GistLevel::Paragraph, "short", "tiny"),
            MemoryGist::new(
                GistLevel::Section,
                "clean",
                "Use scoped governance before injecting retrieved memory.",
            )
            .with_importance(0.7)
            .with_source_tokens(42),
        ];

        let report = DefaultCleanGistSelector::new().selection_report(&gists);

        assert_eq!(report.candidate_count, 5);
        assert!(report.selected);
        assert_eq!(report.selected_level, Some(GistLevel::Section));
        assert_eq!(report.selected_source_tokens, 42);
        assert_eq!(report.selected_importance, 0.7);
        assert_eq!(report.rejected_empty, 1);
        assert_eq!(report.rejected_transcript, 1);
        assert_eq!(report.rejected_metadata, 1);
        assert_eq!(report.rejected_low_signal, 1);
        assert_eq!(
            report.reason_codes(),
            vec![
                "rejected_empty".to_owned(),
                "rejected_low_signal".to_owned(),
                "rejected_metadata".to_owned(),
                "rejected_transcript".to_owned(),
                "selected".to_owned(),
            ]
        );
        assert_eq!(
            report.summary_line(),
            "clean_gist_selection candidates=5 selected=true selected_level=section selected_tokens=42 selected_importance=0.700 rejected_empty=1 rejected_transcript=1 rejected_metadata=1 rejected_low_signal=1 reason_codes=rejected_empty|rejected_low_signal|rejected_metadata|rejected_transcript|selected detail_codes=rejected_empty|rejected_low_signal|rejected_metadata|rejected_transcript|selected_level:section"
        );
    }

    #[test]
    fn selection_report_detail_codes_are_payload_safe() {
        let clean_secret = "CLEAN_GIST_SELECTED_SECRET_DO_NOT_LOG";
        let transcript_secret = "CLEAN_GIST_TRANSCRIPT_SECRET_DO_NOT_LOG";
        let metadata_secret = "CLEAN_GIST_METADATA_SECRET_DO_NOT_LOG";
        let gists = vec![
            MemoryGist::new(
                GistLevel::Section,
                "selected-title",
                format!("Use scoped memory cleanup before recall. {clean_secret}"),
            )
            .with_importance(0.7)
            .with_source_tokens(21),
            MemoryGist::new(
                GistLevel::Document,
                "transcript-title",
                format!("Conversation Transcript:\nUser: {transcript_secret}\nAssistant: ok"),
            )
            .with_importance(0.9),
            MemoryGist::new(
                GistLevel::Paragraph,
                "metadata-title",
                format!("accepted_pattern quality=0.9 max_severity=watch {metadata_secret}"),
            ),
        ];

        let report = DefaultCleanGistSelector::new().selection_report(&gists);
        let summary = report.summary_line();
        let detail_codes = report.detail_codes();

        assert_eq!(
            detail_codes,
            vec![
                "rejected_metadata".to_owned(),
                "rejected_transcript".to_owned(),
                "selected_level:section".to_owned(),
            ]
        );
        assert!(
            summary.contains(
                "detail_codes=rejected_metadata|rejected_transcript|selected_level:section"
            )
        );
        for forbidden in [clean_secret, transcript_secret, metadata_secret] {
            assert!(
                !summary.contains(forbidden),
                "clean gist summary leaked payload: {forbidden}"
            );
            assert!(
                !detail_codes.iter().any(|code| code.contains(forbidden)),
                "clean gist detail codes leaked payload: {forbidden}"
            );
        }
    }

    #[test]
    fn gist_hint_matches_legacy_shape_for_adapter_mapping() {
        let gist = MemoryGist::new(GistLevel::Paragraph, "title", "summary")
            .with_source_tokens(12)
            .with_importance(0.321);

        assert_eq!(
            gist.hint(),
            "paragraph:title importance=0.321 tokens=12 summary=summary"
        );
    }

    #[test]
    fn selector_reports_clean_gist_adapter_capability() {
        let selector = DefaultCleanGistSelector::new();

        let descriptor = selector.descriptor();
        assert_eq!(descriptor.name, "default_clean_gist_selector");
        assert!(descriptor.read_only);
        assert!(
            descriptor
                .capabilities
                .contains(&MemoryAdapterCapability::CleanGistSelection)
        );

        let health = selector.health().unwrap();
        assert!(health.ready);
        assert_eq!(health.record_count, None);
        assert!(health.warnings.is_empty());
    }
}
