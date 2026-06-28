use super::model::ExperienceRecord;
use super::noise::{retrieval_noise, text_has_metadata_lesson_shape, text_has_transcript_shape};
use crate::process_reward::RewardAction;

const PREVIEW_CHARS: usize = 160;
const QUARANTINE_QUALITY_CAP: f32 = 0.05;

const POLLUTION_MARKERS: &[(&str, &str)] = &[
    ("ssh_connect_timeout", "ssh -o connecttimeout"),
    ("product_automation_token", "product_automation_token"),
    ("owner_bot_merge_token", "owner_bot_merge_token"),
    ("gitlab_local", "gitlab.local"),
    ("merge_requests", "merge_requests"),
    ("merge_request_phrase", "merge requests"),
    ("review_merge", "review + merge"),
    ("bash_command", "bash command"),
    ("remote_script", "<<'remote'"),
];

const PROMPT_DOMAIN_TERMS: &[&str] = &[
    "ssh",
    "gitlab",
    "merge_request",
    "merge requests",
    "product_automation_token",
    "owner_bot_merge_token",
    "bash command",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperienceHygieneSeverity {
    Watch,
    QuarantineCandidate,
}

impl ExperienceHygieneSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Watch => "watch",
            Self::QuarantineCandidate => "quarantine_candidate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperienceHygieneFinding {
    pub experience_id: u64,
    pub severity: ExperienceHygieneSeverity,
    pub reason: String,
    pub markers: Vec<String>,
    pub prompt_preview: String,
    pub lesson_preview: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExperienceHygieneReport {
    pub total_records: usize,
    pub finding_count: usize,
    pub watch_count: usize,
    pub quarantine_candidate_count: usize,
    pub legacy_metadata_lesson_count: usize,
    pub legacy_metadata_without_clean_gist_count: usize,
    pub findings: Vec<ExperienceHygieneFinding>,
}

pub fn inspect_records(records: &[ExperienceRecord], limit: usize) -> ExperienceHygieneReport {
    let mut all_findings = Vec::new();
    let mut legacy_metadata_lesson_count = 0usize;
    let mut legacy_metadata_without_clean_gist_count = 0usize;

    for record in records {
        let cross_task_finding = cross_task_shell_transcript_finding(record);
        if let Some(finding) = cross_task_finding.clone() {
            all_findings.push(finding);
        }

        if let Some(finding) = legacy_metadata_lesson_finding(record) {
            legacy_metadata_lesson_count += 1;
            if finding
                .markers
                .iter()
                .any(|marker| marker == "missing_clean_gist")
            {
                legacy_metadata_without_clean_gist_count += 1;
            }
            if cross_task_finding.is_none() {
                all_findings.push(finding);
            }
        }
    }

    all_findings.sort_by(|left, right| {
        severity_rank(left.severity)
            .cmp(&severity_rank(right.severity))
            .then_with(|| right.experience_id.cmp(&left.experience_id))
    });
    let quarantine_candidate_count = all_findings
        .iter()
        .filter(|finding| finding.severity == ExperienceHygieneSeverity::QuarantineCandidate)
        .count();
    let watch_count = all_findings
        .iter()
        .filter(|finding| finding.severity == ExperienceHygieneSeverity::Watch)
        .count();
    let finding_count = all_findings.len();
    let findings = all_findings.into_iter().take(limit).collect::<Vec<_>>();

    ExperienceHygieneReport {
        total_records: records.len(),
        finding_count,
        watch_count,
        quarantine_candidate_count,
        legacy_metadata_lesson_count,
        legacy_metadata_without_clean_gist_count,
        findings,
    }
}

pub fn cross_task_transcript_pollution(record: &ExperienceRecord, prompt: &str) -> bool {
    cross_task_shell_transcript_finding(record).is_some()
        && !prompt_mentions_pollution_domain(prompt)
}

pub fn apply_admission_hygiene(record: &mut ExperienceRecord) -> Option<ExperienceHygieneFinding> {
    let finding = cross_task_shell_transcript_finding(record)?;
    if prompt_mentions_pollution_domain(&record.prompt) {
        return Some(finding);
    }

    record.quality = record.quality.min(QUARANTINE_QUALITY_CAP);
    record.process_reward.total = record.process_reward.total.min(QUARANTINE_QUALITY_CAP);
    record.process_reward.action = RewardAction::Penalize;
    record.process_reward.notes.push(format!(
        "experience_hygiene={} markers={}",
        finding.reason,
        finding.markers.join(",")
    ));
    Some(finding)
}

pub(super) fn admission_persistence_block(
    record: &ExperienceRecord,
) -> Option<ExperienceHygieneFinding> {
    let finding = cross_task_shell_transcript_finding(record)?;
    if prompt_mentions_pollution_domain(&record.prompt) {
        None
    } else {
        Some(finding)
    }
}

fn cross_task_shell_transcript_finding(
    record: &ExperienceRecord,
) -> Option<ExperienceHygieneFinding> {
    let text = record_text(record);
    if !has_transcript_shape(&text) {
        return None;
    }

    let markers = matched_markers(&text);
    if markers.is_empty() {
        return None;
    }

    Some(ExperienceHygieneFinding {
        experience_id: record.id,
        severity: ExperienceHygieneSeverity::QuarantineCandidate,
        reason: "cross_task_shell_transcript".to_owned(),
        markers,
        prompt_preview: compact(&record.prompt, PREVIEW_CHARS),
        lesson_preview: compact(&record.lesson, PREVIEW_CHARS),
    })
}

fn legacy_metadata_lesson_finding(record: &ExperienceRecord) -> Option<ExperienceHygieneFinding> {
    if !text_has_metadata_lesson_shape(&record.lesson) {
        return None;
    }

    let retrieval_noise = retrieval_noise(record);
    let mut markers = vec!["legacy_metadata_lesson".to_owned()];
    if retrieval_noise.has_clean_gist {
        markers.push("clean_gist_fallback".to_owned());
    } else {
        markers.push("missing_clean_gist".to_owned());
    }
    if retrieval_noise.prompt_transcript_like {
        markers.push("transcript_prompt".to_owned());
    }

    Some(ExperienceHygieneFinding {
        experience_id: record.id,
        severity: ExperienceHygieneSeverity::Watch,
        reason: "legacy_metadata_lesson".to_owned(),
        markers,
        prompt_preview: compact(&record.prompt, PREVIEW_CHARS),
        lesson_preview: compact(&record.lesson, PREVIEW_CHARS),
    })
}

fn severity_rank(severity: ExperienceHygieneSeverity) -> usize {
    match severity {
        ExperienceHygieneSeverity::QuarantineCandidate => 0,
        ExperienceHygieneSeverity::Watch => 1,
    }
}

fn record_text(record: &ExperienceRecord) -> String {
    let mut parts = vec![record.prompt.as_str(), record.lesson.as_str()];
    parts.extend(record.process_reward.notes.iter().map(String::as_str));
    parts.extend(record.revision_actions.iter().map(String::as_str));
    parts.join(" ").to_ascii_lowercase()
}

fn has_transcript_shape(text: &str) -> bool {
    text_has_transcript_shape(text)
}

fn matched_markers(text: &str) -> Vec<String> {
    POLLUTION_MARKERS
        .iter()
        .filter(|(_, marker)| text.contains(marker))
        .map(|(name, _)| (*name).to_owned())
        .collect()
}

fn prompt_mentions_pollution_domain(prompt: &str) -> bool {
    let prompt = prompt.to_ascii_lowercase();
    PROMPT_DOMAIN_TERMS
        .iter()
        .any(|domain_term| prompt.contains(domain_term))
}

fn compact(value: &str, max_chars: usize) -> String {
    let _ = max_chars;
    format!("chars={}", value.chars().count())
}
