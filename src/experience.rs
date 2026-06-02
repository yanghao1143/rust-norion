use std::collections::HashSet;
use std::io;
use std::path::Path;

use crate::adaptive_state::LiveInferenceEvolution;
use crate::disk_kv::DiskKvStore;
use crate::gist_memory::{GistLevel, GistRecord};
use crate::hierarchy::{HierarchyWeights, TaskProfile};
use crate::process_reward::{ProcessRewardComponents, ProcessRewardReport, RewardAction};
use crate::reflection::{ReflectionIssue, ReflectionSeverity, RuntimeDiagnostics};
use crate::router::RouteBudget;

const EXPERIENCE_FLOAT_EPSILON: f32 = 0.000_001;

#[derive(Debug, Clone)]
pub struct ExperienceInput {
    pub prompt: String,
    pub profile: TaskProfile,
    pub lesson: String,
    pub quality: f32,
    pub contradictions: Vec<String>,
    pub reflection_issues: Vec<ReflectionIssue>,
    pub revision_actions: Vec<String>,
    pub stored_memory_id: Option<u64>,
    pub router_threshold_after: f32,
    pub stream_windows: usize,
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub used_memory_ids: Vec<u64>,
    pub gist_records: Vec<GistRecord>,
    pub gist_memory_ids: Vec<u64>,
    pub stored_runtime_kv_memory_ids: Vec<u64>,
    pub runtime_diagnostics: RuntimeDiagnostics,
    pub runtime_token_metrics: ExperienceRuntimeTokenMetrics,
    pub process_reward: ProcessRewardReport,
    pub live_evolution: LiveInferenceEvolution,
}

#[derive(Debug, Clone)]
pub struct ExperienceRecord {
    pub id: u64,
    pub prompt: String,
    pub profile: TaskProfile,
    pub lesson: String,
    pub quality: f32,
    pub contradictions: Vec<String>,
    pub reflection_issues: Vec<ReflectionIssue>,
    pub revision_actions: Vec<String>,
    pub stored_memory_id: Option<u64>,
    pub router_threshold_after: f32,
    pub stream_windows: usize,
    pub route_budget: RouteBudget,
    pub hierarchy: HierarchyWeights,
    pub used_memory_ids: Vec<u64>,
    pub gist_records: Vec<GistRecord>,
    pub gist_memory_ids: Vec<u64>,
    pub stored_runtime_kv_memory_ids: Vec<u64>,
    pub runtime_diagnostics: RuntimeDiagnostics,
    pub runtime_token_metrics: ExperienceRuntimeTokenMetrics,
    pub process_reward: ProcessRewardReport,
    pub live_evolution: LiveInferenceEvolution,
}

#[derive(Debug, Clone)]
pub struct ExperienceMatch {
    pub id: u64,
    pub prompt: String,
    pub lesson: String,
    pub quality: f32,
    pub score: f32,
    pub gist_hints: Vec<String>,
    pub reflection_issue_codes: Vec<String>,
    pub revision_actions: Vec<String>,
    pub process_reward: f32,
    pub reward_action: RewardAction,
    pub runtime_model_id: Option<String>,
    pub runtime_selected_adapter: Option<String>,
    pub runtime_device_profile: Option<String>,
    pub runtime_primary_lane: Option<String>,
    pub runtime_fallback_lane: Option<String>,
    pub runtime_memory_mode: Option<String>,
    pub runtime_forward_energy: Option<f32>,
    pub runtime_kv_influence: Option<f32>,
    pub runtime_uncertainty_perplexity: Option<f32>,
    pub recursive_runtime_calls: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ExperienceRuntimeTokenMetrics {
    pub token_count: usize,
    pub entropy_count: usize,
    pub logprob_count: usize,
    pub average_entropy: Option<f32>,
    pub average_neg_logprob: Option<f32>,
    pub uncertainty_perplexity: Option<f32>,
}

impl ExperienceRuntimeTokenMetrics {
    pub fn has_uncertainty_signal(&self) -> bool {
        self.average_entropy.is_some()
            || self.average_neg_logprob.is_some()
            || self.uncertainty_perplexity.is_some()
            || self.entropy_count > 0
            || self.logprob_count > 0
    }
}

#[derive(Debug, Clone)]
pub struct ExperienceStore {
    records: Vec<ExperienceRecord>,
    next_id: u64,
}

impl Default for ExperienceStore {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
        }
    }
}

impl ExperienceStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn records(&self) -> &[ExperienceRecord] {
        &self.records
    }

    pub fn record(&mut self, input: ExperienceInput) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.records.push(ExperienceRecord {
            id,
            prompt: input.prompt,
            profile: input.profile,
            lesson: input.lesson,
            quality: input.quality.clamp(0.0, 1.0),
            contradictions: input.contradictions,
            reflection_issues: input.reflection_issues,
            revision_actions: input.revision_actions,
            stored_memory_id: input.stored_memory_id,
            router_threshold_after: input.router_threshold_after,
            stream_windows: input.stream_windows,
            route_budget: input.route_budget,
            hierarchy: input.hierarchy,
            used_memory_ids: input.used_memory_ids,
            gist_records: input.gist_records,
            gist_memory_ids: input.gist_memory_ids,
            stored_runtime_kv_memory_ids: input.stored_runtime_kv_memory_ids,
            runtime_diagnostics: input.runtime_diagnostics,
            runtime_token_metrics: input.runtime_token_metrics,
            process_reward: input.process_reward,
            live_evolution: input.live_evolution,
        });
        id
    }

    pub fn recent(&self, limit: usize) -> Vec<&ExperienceRecord> {
        self.records.iter().rev().take(limit).collect()
    }

    pub fn top_lessons(&self, min_quality: f32, limit: usize) -> Vec<&ExperienceRecord> {
        let mut records = self
            .records
            .iter()
            .filter(|record| record.quality >= min_quality)
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            right
                .quality
                .partial_cmp(&left.quality)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        records.truncate(limit);
        records
    }

    pub fn retrieve_lessons(
        &self,
        prompt: &str,
        profile: TaskProfile,
        limit: usize,
    ) -> Vec<ExperienceMatch> {
        let mut matches = self
            .records
            .iter()
            .filter_map(|record| {
                let gist_text = record
                    .gist_records
                    .iter()
                    .map(|gist| format!("{} {}", gist.title, gist.summary))
                    .collect::<Vec<_>>()
                    .join(" ");
                let reflection_text = record
                    .reflection_issues
                    .iter()
                    .map(|issue| format!("{} {}", issue.code, issue.detail))
                    .chain(record.revision_actions.iter().cloned())
                    .collect::<Vec<_>>()
                    .join(" ");
                let runtime_text = runtime_diagnostics_text(&record.runtime_diagnostics);
                let recursive_runtime_calls =
                    recursive_runtime_calls_from_notes(&record.process_reward.notes);
                let recursive_text = recursive_runtime_calls
                    .map(|calls| format!("recursive_runtime_calls {calls}"))
                    .unwrap_or_default();
                let overlap = lexical_overlap(
                    prompt,
                    &format!(
                        "{} {} {} {} {} {}",
                        record.prompt,
                        record.lesson,
                        gist_text,
                        reflection_text,
                        runtime_text,
                        recursive_text
                    ),
                );
                let profile_bonus = if record.profile == profile { 0.16 } else { 0.0 };
                let gist_bonus = record
                    .gist_records
                    .iter()
                    .map(|gist| gist.importance)
                    .fold(0.0, f32::max)
                    * 0.08;
                let reward_bonus = record.process_reward.total * 0.08;
                let contradiction_penalty = (record.contradictions.len() as f32 * 0.08).min(0.32);
                let reflection_penalty = reflection_issue_penalty(&record.reflection_issues);
                let score = (overlap * 0.52
                    + record.quality * 0.36
                    + profile_bonus
                    + gist_bonus
                    + reward_bonus
                    - contradiction_penalty
                    - reflection_penalty)
                    .clamp(0.0, 1.0);

                if score < 0.12 {
                    return None;
                }

                Some(ExperienceMatch {
                    id: record.id,
                    prompt: record.prompt.clone(),
                    lesson: record.lesson.clone(),
                    quality: record.quality,
                    score,
                    gist_hints: record
                        .gist_records
                        .iter()
                        .take(3)
                        .map(GistRecord::hint)
                        .collect(),
                    reflection_issue_codes: record
                        .reflection_issues
                        .iter()
                        .map(|issue| issue.code.clone())
                        .collect(),
                    revision_actions: record.revision_actions.clone(),
                    process_reward: record.process_reward.total,
                    reward_action: record.process_reward.action,
                    runtime_model_id: record.runtime_diagnostics.model_id.clone(),
                    runtime_selected_adapter: record.runtime_diagnostics.selected_adapter.clone(),
                    runtime_device_profile: record.runtime_diagnostics.device_profile.clone(),
                    runtime_primary_lane: record.runtime_diagnostics.primary_lane.clone(),
                    runtime_fallback_lane: record.runtime_diagnostics.fallback_lane.clone(),
                    runtime_memory_mode: record.runtime_diagnostics.memory_mode.clone(),
                    runtime_forward_energy: record.runtime_diagnostics.forward_energy,
                    runtime_kv_influence: record.runtime_diagnostics.kv_influence,
                    runtime_uncertainty_perplexity: record
                        .runtime_token_metrics
                        .uncertainty_perplexity,
                    recursive_runtime_calls,
                })
            })
            .collect::<Vec<_>>();

        matches.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.truncate(limit);
        matches
    }

    pub fn save_to_disk_kv(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut store = DiskKvStore::open(path)?;
        let mut live_keys = HashSet::new();

        for record in &self.records {
            let key = format!("experience/{}", record.id);
            live_keys.insert(key.clone());
            store.put(&key, serialize_record(record).as_bytes())?;
        }

        for stale_key in store.keys_with_prefix("experience/") {
            if !live_keys.contains(&stale_key) {
                store.delete(&stale_key)?;
            }
        }

        store.put(
            "meta/next_experience_id",
            self.next_id.to_string().as_bytes(),
        )?;
        store.compact()
    }

    pub fn load_from_disk_kv(path: impl AsRef<Path>) -> io::Result<Self> {
        let store = DiskKvStore::open(path)?;
        let mut out = Self::new();

        for key in store.keys_with_prefix("experience/") {
            let Some(value) = store.get(&key)? else {
                continue;
            };
            let Ok(line) = String::from_utf8(value) else {
                continue;
            };
            let Some(record) = deserialize_record(&line) else {
                continue;
            };
            out.next_id = out.next_id.max(record.id + 1);
            out.records.push(record);
        }

        out.records.sort_by_key(|record| record.id);
        if let Some(value) = store.get("meta/next_experience_id")? {
            if let Ok(next_id) = String::from_utf8_lossy(&value).parse::<u64>() {
                out.next_id = out.next_id.max(next_id);
            }
        }

        Ok(out)
    }
}

fn serialize_record(record: &ExperienceRecord) -> String {
    let stored_memory_id = record
        .stored_memory_id
        .map(|id| id.to_string())
        .unwrap_or_default();
    let contradictions = record
        .contradictions
        .iter()
        .map(|item| escape_field(item))
        .collect::<Vec<_>>()
        .join("|");
    let gist_records = serialize_gists(&record.gist_records);
    let gist_memory_ids = serialize_ids(&record.gist_memory_ids);
    let used_memory_ids = serialize_ids(&record.used_memory_ids);
    let stored_runtime_kv_memory_ids = serialize_ids(&record.stored_runtime_kv_memory_ids);
    let process_reward = serialize_process_reward(&record.process_reward);
    let route_budget = serialize_route_budget(record.route_budget);
    let reflection_issues = serialize_reflection_issues(&record.reflection_issues);
    let revision_actions = serialize_revision_actions(&record.revision_actions);
    let runtime_diagnostics = serialize_runtime_diagnostics(&record.runtime_diagnostics);
    let runtime_token_metrics = serialize_runtime_token_metrics(record.runtime_token_metrics);
    let live_evolution = serialize_live_evolution(record.live_evolution);

    format!(
        "{}\t{}\t{:.6}\t{}\t{:.6}\t{}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        record.id,
        profile_to_str(record.profile),
        record.quality,
        stored_memory_id,
        record.router_threshold_after,
        record.stream_windows,
        record.hierarchy.global,
        record.hierarchy.local,
        record.hierarchy.convolution,
        escape_field(&record.prompt),
        escape_field(&record.lesson),
        contradictions,
        escape_field(&gist_records),
        escape_field(&gist_memory_ids),
        escape_field(&process_reward),
        escape_field(&route_budget),
        escape_field(&used_memory_ids),
        escape_field(&stored_runtime_kv_memory_ids),
        escape_field(&reflection_issues),
        escape_field(&revision_actions),
        escape_field(&runtime_diagnostics),
        escape_field(&live_evolution),
        escape_field(&runtime_token_metrics)
    )
}

fn deserialize_record(line: &str) -> Option<ExperienceRecord> {
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() < 12 {
        return None;
    }

    let id = fields[0].parse::<u64>().ok()?;
    let profile = str_to_profile(fields[1])?;
    let quality = fields[2].parse::<f32>().ok()?;
    let stored_memory_id = if fields[3].is_empty() {
        None
    } else {
        Some(fields[3].parse::<u64>().ok()?)
    };
    let router_threshold_after = fields[4].parse::<f32>().ok()?;
    let stream_windows = fields[5].parse::<usize>().ok()?;
    let hierarchy = HierarchyWeights::new(
        fields[6].parse::<f32>().ok()?,
        fields[7].parse::<f32>().ok()?,
        fields[8].parse::<f32>().ok()?,
    );
    let prompt = unescape_field(fields[9]);
    let lesson = unescape_field(fields[10]);
    let contradictions = if fields[11].is_empty() {
        Vec::new()
    } else {
        fields[11].split('|').map(unescape_field).collect()
    };
    let gist_records = fields
        .get(12)
        .map(|value| deserialize_gists(&unescape_field(value)))
        .unwrap_or_default();
    let gist_memory_ids = fields
        .get(13)
        .map(|value| deserialize_ids(&unescape_field(value)))
        .unwrap_or_default();
    let process_reward = fields
        .get(14)
        .and_then(|value| deserialize_process_reward(&unescape_field(value)))
        .unwrap_or_default();
    let route_budget = fields
        .get(15)
        .and_then(|value| deserialize_route_budget(&unescape_field(value)))
        .unwrap_or(RouteBudget {
            threshold: router_threshold_after,
            attention_tokens: 0,
            fast_tokens: 0,
            attention_fraction: 0.0,
        });
    let used_memory_ids = fields
        .get(16)
        .map(|value| deserialize_ids(&unescape_field(value)))
        .unwrap_or_default();
    let stored_runtime_kv_memory_ids = fields
        .get(17)
        .map(|value| deserialize_ids(&unescape_field(value)))
        .unwrap_or_default();
    let reflection_issues = fields
        .get(18)
        .map(|value| deserialize_reflection_issues(&unescape_field(value)))
        .unwrap_or_default();
    let revision_actions = fields
        .get(19)
        .map(|value| deserialize_revision_actions(&unescape_field(value)))
        .unwrap_or_default();
    let runtime_diagnostics = fields
        .get(20)
        .and_then(|value| deserialize_runtime_diagnostics(&unescape_field(value)))
        .unwrap_or_default();
    let live_evolution = match fields.get(21) {
        Some(value) => deserialize_live_evolution(&unescape_field(value))?,
        None => LiveInferenceEvolution::default(),
    };
    let runtime_token_metrics = fields
        .get(22)
        .and_then(|value| deserialize_runtime_token_metrics(&unescape_field(value)))
        .unwrap_or_default();

    Some(ExperienceRecord {
        id,
        prompt,
        profile,
        lesson,
        quality,
        contradictions,
        reflection_issues,
        revision_actions,
        stored_memory_id,
        router_threshold_after,
        stream_windows,
        route_budget,
        hierarchy,
        used_memory_ids,
        gist_records,
        gist_memory_ids,
        stored_runtime_kv_memory_ids,
        runtime_diagnostics,
        runtime_token_metrics,
        process_reward,
        live_evolution,
    })
}

fn serialize_runtime_token_metrics(metrics: ExperienceRuntimeTokenMetrics) -> String {
    [
        metrics.token_count.to_string(),
        metrics.entropy_count.to_string(),
        metrics.logprob_count.to_string(),
        option_f32_to_field(metrics.average_entropy),
        option_f32_to_field(metrics.average_neg_logprob),
        option_f32_to_field(metrics.uncertainty_perplexity),
    ]
    .join(",")
}

fn deserialize_runtime_token_metrics(value: &str) -> Option<ExperienceRuntimeTokenMetrics> {
    if value.is_empty() {
        return Some(ExperienceRuntimeTokenMetrics::default());
    }

    let fields = value.split(',').collect::<Vec<_>>();
    if fields.len() != 6 {
        return None;
    }

    Some(ExperienceRuntimeTokenMetrics {
        token_count: fields[0].parse::<usize>().ok()?,
        entropy_count: fields[1].parse::<usize>().ok()?,
        logprob_count: fields[2].parse::<usize>().ok()?,
        average_entropy: field_to_finite_f32(fields[3]),
        average_neg_logprob: field_to_finite_f32(fields[4]),
        uncertainty_perplexity: field_to_finite_f32(fields[5]),
    })
}

fn serialize_route_budget(route_budget: RouteBudget) -> String {
    format!(
        "{:.6},{},{},{:.6}",
        route_budget.threshold,
        route_budget.attention_tokens,
        route_budget.fast_tokens,
        route_budget.attention_fraction
    )
}

fn deserialize_route_budget(value: &str) -> Option<RouteBudget> {
    let fields = value.split(',').collect::<Vec<_>>();
    if fields.len() != 4 {
        return None;
    }

    Some(RouteBudget {
        threshold: fields[0].parse::<f32>().ok()?,
        attention_tokens: fields[1].parse::<usize>().ok()?,
        fast_tokens: fields[2].parse::<usize>().ok()?,
        attention_fraction: fields[3].parse::<f32>().ok()?.clamp(0.0, 1.0),
    })
}

fn serialize_live_evolution(report: LiveInferenceEvolution) -> String {
    [
        finite_f32_to_field(report.router_threshold_delta),
        finite_f32_to_field(report.hierarchy_weight_delta),
        report.online_reward_feedbacks.to_string(),
        report.online_reward_reinforcements.to_string(),
        report.online_reward_penalties.to_string(),
        finite_f32_to_field(report.online_reward_strength.max(0.0)),
        finite_f32_to_field(report.online_reward_reinforcement_strength.max(0.0)),
        finite_f32_to_field(report.online_reward_penalty_strength.max(0.0)),
        report.memory_reinforcements.to_string(),
        report.memory_penalties.to_string(),
        bool_to_field(report.stored_memory).to_owned(),
        report.stored_gist_memories.to_string(),
        report.stored_runtime_kv_memories.to_string(),
        report.reflection_issues.to_string(),
        report.critical_reflection_issues.to_string(),
        report.revision_actions.to_string(),
    ]
    .join(",")
}

fn deserialize_live_evolution(value: &str) -> Option<LiveInferenceEvolution> {
    if value.is_empty() {
        return Some(LiveInferenceEvolution::default());
    }

    let fields = value.split(',').collect::<Vec<_>>();
    if fields.len() != 10 && fields.len() != 13 && fields.len() != 16 {
        return None;
    }
    let has_online_reward_feedback = fields.len() >= 13;
    let has_online_reward_strength = fields.len() == 16;
    let memory_index = if has_online_reward_strength {
        8
    } else if has_online_reward_feedback {
        5
    } else {
        2
    };

    let online_reward_feedbacks = if has_online_reward_feedback {
        fields[2].parse::<usize>().ok()?
    } else {
        0
    };
    let online_reward_reinforcements = if has_online_reward_feedback {
        fields[3].parse::<usize>().ok()?
    } else {
        0
    };
    let online_reward_penalties = if has_online_reward_feedback {
        fields[4].parse::<usize>().ok()?
    } else {
        0
    };
    if has_online_reward_feedback
        && online_reward_feedbacks
            != online_reward_reinforcements.saturating_add(online_reward_penalties)
    {
        return None;
    }

    let online_reward_strength = if has_online_reward_strength {
        nonnegative_finite_f32_field(fields[5])?
    } else {
        0.0
    };
    let online_reward_reinforcement_strength = if has_online_reward_strength {
        nonnegative_finite_f32_field(fields[6])?
    } else {
        0.0
    };
    let online_reward_penalty_strength = if has_online_reward_strength {
        nonnegative_finite_f32_field(fields[7])?
    } else {
        0.0
    };

    let report = LiveInferenceEvolution {
        router_threshold_delta: field_to_finite_f32(fields[0])?.max(0.0),
        hierarchy_weight_delta: field_to_finite_f32(fields[1])?.max(0.0),
        online_reward_feedbacks,
        online_reward_reinforcements,
        online_reward_penalties,
        online_reward_strength,
        online_reward_reinforcement_strength,
        online_reward_penalty_strength,
        memory_reinforcements: fields[memory_index].parse::<usize>().ok()?,
        memory_penalties: fields[memory_index + 1].parse::<usize>().ok()?,
        stored_memory: field_to_bool(fields[memory_index + 2])?,
        stored_gist_memories: fields[memory_index + 3].parse::<usize>().ok()?,
        stored_runtime_kv_memories: fields[memory_index + 4].parse::<usize>().ok()?,
        reflection_issues: fields[memory_index + 5].parse::<usize>().ok()?,
        critical_reflection_issues: fields[memory_index + 6].parse::<usize>().ok()?,
        revision_actions: fields[memory_index + 7].parse::<usize>().ok()?,
    };

    if has_online_reward_strength && !live_online_reward_strength_is_consistent(&report) {
        return None;
    }

    Some(report)
}

fn nonnegative_finite_f32_field(value: &str) -> Option<f32> {
    field_to_finite_f32(value).filter(|value| *value >= 0.0)
}

fn live_online_reward_strength_is_consistent(report: &LiveInferenceEvolution) -> bool {
    let has_reinforcement_strength =
        report.online_reward_reinforcement_strength > EXPERIENCE_FLOAT_EPSILON;
    let has_penalty_strength = report.online_reward_penalty_strength > EXPERIENCE_FLOAT_EPSILON;
    report.online_reward_strength.is_finite()
        && report.online_reward_reinforcement_strength.is_finite()
        && report.online_reward_penalty_strength.is_finite()
        && report.online_reward_feedbacks
            == report
                .online_reward_reinforcements
                .saturating_add(report.online_reward_penalties)
        && report.online_reward_strength >= 0.0
        && report.online_reward_reinforcement_strength >= 0.0
        && report.online_reward_penalty_strength >= 0.0
        && !(report.online_reward_strength > EXPERIENCE_FLOAT_EPSILON
            && report.online_reward_feedbacks == 0)
        && !(report.online_reward_feedbacks > 0
            && report.online_reward_strength <= EXPERIENCE_FLOAT_EPSILON)
        && !(has_reinforcement_strength && report.online_reward_reinforcements == 0)
        && !(report.online_reward_reinforcements > 0
            && report.online_reward_reinforcement_strength <= EXPERIENCE_FLOAT_EPSILON)
        && !(has_penalty_strength && report.online_reward_penalties == 0)
        && !(report.online_reward_penalties > 0
            && report.online_reward_penalty_strength <= EXPERIENCE_FLOAT_EPSILON)
        && (report.online_reward_strength
            - (report.online_reward_reinforcement_strength + report.online_reward_penalty_strength))
            .abs()
            <= EXPERIENCE_FLOAT_EPSILON
}

fn serialize_gists(records: &[GistRecord]) -> String {
    records
        .iter()
        .map(|record| {
            [
                record.level.as_str().to_owned(),
                format!("{:.6}", record.importance),
                record.source_tokens.to_string(),
                sanitize_gist_part(&record.title),
                sanitize_gist_part(&record.summary),
            ]
            .join("\u{1f}")
        })
        .collect::<Vec<_>>()
        .join("\u{1e}")
}

fn deserialize_gists(value: &str) -> Vec<GistRecord> {
    if value.is_empty() {
        return Vec::new();
    }

    value
        .split('\u{1e}')
        .filter_map(|item| {
            let fields = item.split('\u{1f}').collect::<Vec<_>>();
            if fields.len() != 5 {
                return None;
            }

            Some(GistRecord {
                level: GistLevel::from_str(fields[0])?,
                importance: fields[1].parse::<f32>().ok()?.clamp(0.0, 1.0),
                source_tokens: fields[2].parse::<usize>().ok()?,
                title: fields[3].to_owned(),
                summary: fields[4].to_owned(),
            })
        })
        .collect()
}

fn serialize_ids(ids: &[u64]) -> String {
    ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",")
}

fn deserialize_ids(value: &str) -> Vec<u64> {
    value
        .split(',')
        .filter_map(|item| item.parse::<u64>().ok())
        .collect()
}

fn serialize_reflection_issues(issues: &[ReflectionIssue]) -> String {
    issues
        .iter()
        .map(|issue| {
            [
                sanitize_control_part(&issue.code),
                issue.severity.as_str().to_owned(),
                sanitize_control_part(&issue.detail),
            ]
            .join("\u{1f}")
        })
        .collect::<Vec<_>>()
        .join("\u{1e}")
}

fn deserialize_reflection_issues(value: &str) -> Vec<ReflectionIssue> {
    if value.is_empty() {
        return Vec::new();
    }

    value
        .split('\u{1e}')
        .filter_map(|item| {
            let fields = item.split('\u{1f}').collect::<Vec<_>>();
            if fields.len() != 3 {
                return None;
            }

            Some(ReflectionIssue::new(
                fields[0],
                ReflectionSeverity::from_str(fields[1])?,
                fields[2],
            ))
        })
        .collect()
}

fn serialize_revision_actions(actions: &[String]) -> String {
    actions
        .iter()
        .map(|action| sanitize_control_part(action))
        .collect::<Vec<_>>()
        .join("\u{1e}")
}

fn deserialize_revision_actions(value: &str) -> Vec<String> {
    if value.is_empty() {
        Vec::new()
    } else {
        value.split('\u{1e}').map(ToOwned::to_owned).collect()
    }
}

fn serialize_runtime_diagnostics(diagnostics: &RuntimeDiagnostics) -> String {
    [
        diagnostics
            .model_id
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .selected_adapter
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .device_profile
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .primary_lane
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .fallback_lane
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics
            .memory_mode
            .as_deref()
            .map(sanitize_control_part)
            .unwrap_or_default(),
        diagnostics.layer_count.to_string(),
        diagnostics.global_layers.to_string(),
        diagnostics.local_window_layers.to_string(),
        diagnostics.convolutional_fusion_layers.to_string(),
        diagnostics.hidden_size.to_string(),
        diagnostics.local_window_tokens.to_string(),
        option_f32_to_field(diagnostics.forward_energy),
        option_f32_to_field(diagnostics.kv_influence),
        diagnostics.imported_kv_blocks.to_string(),
        diagnostics.exported_kv_blocks.to_string(),
        option_u8_to_field(diagnostics.hot_kv_precision_bits),
        option_u8_to_field(diagnostics.cold_kv_precision_bits),
    ]
    .join("\u{1f}")
}

fn deserialize_runtime_diagnostics(value: &str) -> Option<RuntimeDiagnostics> {
    if value.is_empty() {
        return Some(RuntimeDiagnostics::default());
    }

    let fields = value.split('\u{1f}').collect::<Vec<_>>();
    match fields.len() {
        9 => Some(RuntimeDiagnostics {
            model_id: non_empty_string(fields[0]),
            selected_adapter: non_empty_string(fields[1]),
            device_profile: None,
            primary_lane: None,
            fallback_lane: None,
            memory_mode: None,
            layer_count: fields[2].parse::<usize>().ok()?,
            global_layers: 0,
            local_window_layers: 0,
            convolutional_fusion_layers: 0,
            hidden_size: fields[3].parse::<usize>().ok()?,
            local_window_tokens: fields[4].parse::<usize>().ok()?,
            forward_energy: field_to_finite_f32(fields[5]),
            kv_influence: field_to_finite_f32(fields[6]),
            imported_kv_blocks: fields[7].parse::<usize>().ok()?,
            exported_kv_blocks: fields[8].parse::<usize>().ok()?,
            hot_kv_precision_bits: None,
            cold_kv_precision_bits: None,
        }),
        12 => Some(RuntimeDiagnostics {
            model_id: non_empty_string(fields[0]),
            selected_adapter: non_empty_string(fields[1]),
            device_profile: None,
            primary_lane: None,
            fallback_lane: None,
            memory_mode: None,
            layer_count: fields[2].parse::<usize>().ok()?,
            global_layers: fields[3].parse::<usize>().ok()?,
            local_window_layers: fields[4].parse::<usize>().ok()?,
            convolutional_fusion_layers: fields[5].parse::<usize>().ok()?,
            hidden_size: fields[6].parse::<usize>().ok()?,
            local_window_tokens: fields[7].parse::<usize>().ok()?,
            forward_energy: field_to_finite_f32(fields[8]),
            kv_influence: field_to_finite_f32(fields[9]),
            imported_kv_blocks: fields[10].parse::<usize>().ok()?,
            exported_kv_blocks: fields[11].parse::<usize>().ok()?,
            hot_kv_precision_bits: None,
            cold_kv_precision_bits: None,
        }),
        16 | 18 => Some(RuntimeDiagnostics {
            model_id: non_empty_string(fields[0]),
            selected_adapter: non_empty_string(fields[1]),
            device_profile: non_empty_string(fields[2]),
            primary_lane: non_empty_string(fields[3]),
            fallback_lane: non_empty_string(fields[4]),
            memory_mode: non_empty_string(fields[5]),
            layer_count: fields[6].parse::<usize>().ok()?,
            global_layers: fields[7].parse::<usize>().ok()?,
            local_window_layers: fields[8].parse::<usize>().ok()?,
            convolutional_fusion_layers: fields[9].parse::<usize>().ok()?,
            hidden_size: fields[10].parse::<usize>().ok()?,
            local_window_tokens: fields[11].parse::<usize>().ok()?,
            forward_energy: field_to_finite_f32(fields[12]),
            kv_influence: field_to_finite_f32(fields[13]),
            imported_kv_blocks: fields[14].parse::<usize>().ok()?,
            exported_kv_blocks: fields[15].parse::<usize>().ok()?,
            hot_kv_precision_bits: fields
                .get(16)
                .and_then(|value| field_to_kv_precision_bits(value)),
            cold_kv_precision_bits: fields
                .get(17)
                .and_then(|value| field_to_kv_precision_bits(value)),
        }),
        _ => None,
    }
}

fn runtime_diagnostics_text(diagnostics: &RuntimeDiagnostics) -> String {
    let mut parts = [
        diagnostics.model_id.as_deref().unwrap_or_default(),
        diagnostics.selected_adapter.as_deref().unwrap_or_default(),
        diagnostics.device_profile.as_deref().unwrap_or_default(),
        diagnostics.primary_lane.as_deref().unwrap_or_default(),
        diagnostics.fallback_lane.as_deref().unwrap_or_default(),
        diagnostics.memory_mode.as_deref().unwrap_or_default(),
    ]
    .into_iter()
    .filter(|item| !item.is_empty())
    .map(ToOwned::to_owned)
    .collect::<Vec<_>>();
    if diagnostics.has_valid_kv_precision_signal() {
        parts.push(format!(
            "kv_bits={}/{}",
            diagnostics.hot_kv_precision_bits.unwrap_or_default(),
            diagnostics.cold_kv_precision_bits.unwrap_or_default()
        ));
    }
    parts.join(" ")
}

fn option_u8_to_field(value: Option<u8>) -> String {
    value
        .filter(|value| matches!(value, 4 | 8))
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn field_to_kv_precision_bits(value: &str) -> Option<u8> {
    if value.is_empty() {
        return None;
    }
    value
        .parse::<u8>()
        .ok()
        .filter(|value| matches!(value, 4 | 8))
}

fn option_f32_to_field(value: Option<f32>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.6}"))
        .unwrap_or_default()
}

fn field_to_finite_f32(value: &str) -> Option<f32> {
    if value.is_empty() {
        return None;
    }
    value.parse::<f32>().ok().filter(|value| value.is_finite())
}

fn finite_f32_to_field(value: f32) -> String {
    if value.is_finite() {
        format!("{value:.6}")
    } else {
        "0.000000".to_owned()
    }
}

fn bool_to_field(value: bool) -> &'static str {
    if value { "1" } else { "0" }
}

fn field_to_bool(value: &str) -> Option<bool> {
    match value {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn serialize_process_reward(report: &ProcessRewardReport) -> String {
    let notes = report
        .notes
        .iter()
        .map(|note| sanitize_control_part(note))
        .collect::<Vec<_>>()
        .join("\u{1e}");
    [
        format!("{:.6}", report.total),
        report.action.as_str().to_owned(),
        format!("{:.6}", report.components.route),
        format!("{:.6}", report.components.memory),
        format!("{:.6}", report.components.hierarchy),
        format!("{:.6}", report.components.reflection),
        format!("{:.6}", report.components.latency),
        format!("{:.6}", report.components.admission),
        notes,
    ]
    .join("\u{1f}")
}

fn deserialize_process_reward(value: &str) -> Option<ProcessRewardReport> {
    if value.is_empty() {
        return Some(ProcessRewardReport::default());
    }

    let fields = value.split('\u{1f}').collect::<Vec<_>>();
    if fields.len() != 9 {
        return None;
    }

    let notes = if fields[8].is_empty() {
        Vec::new()
    } else {
        fields[8].split('\u{1e}').map(ToOwned::to_owned).collect()
    };

    Some(ProcessRewardReport {
        total: fields[0].parse::<f32>().ok()?.clamp(0.0, 1.0),
        action: RewardAction::from_str(fields[1])?,
        components: ProcessRewardComponents {
            route: fields[2].parse::<f32>().ok()?.clamp(0.0, 1.0),
            memory: fields[3].parse::<f32>().ok()?.clamp(0.0, 1.0),
            hierarchy: fields[4].parse::<f32>().ok()?.clamp(0.0, 1.0),
            reflection: fields[5].parse::<f32>().ok()?.clamp(0.0, 1.0),
            latency: fields[6].parse::<f32>().ok()?.clamp(0.0, 1.0),
            admission: fields[7].parse::<f32>().ok()?.clamp(0.0, 1.0),
        },
        notes,
    })
}

pub fn recursive_runtime_calls_from_notes(notes: &[String]) -> Option<usize> {
    notes.iter().find_map(|note| {
        note.split(':')
            .find_map(|part| part.strip_prefix("runtime_calls="))
            .or_else(|| note.strip_prefix("latency:recursive_runtime_calls="))
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|calls| *calls > 0)
    })
}

fn sanitize_gist_part(value: &str) -> String {
    sanitize_control_part(value)
}

fn sanitize_control_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\u{1e}' | '\u{1f}' | '\t' | '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

fn profile_to_str(profile: TaskProfile) -> &'static str {
    match profile {
        TaskProfile::General => "general",
        TaskProfile::Coding => "coding",
        TaskProfile::Writing => "writing",
        TaskProfile::LongDocument => "long_document",
    }
}

fn str_to_profile(value: &str) -> Option<TaskProfile> {
    match value {
        "general" => Some(TaskProfile::General),
        "coding" => Some(TaskProfile::Coding),
        "writing" => Some(TaskProfile::Writing),
        "long_document" => Some(TaskProfile::LongDocument),
        _ => None,
    }
}

fn lexical_overlap(left: &str, right: &str) -> f32 {
    let left_chars = left
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .collect::<HashSet<_>>();
    let right_chars = right
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .collect::<HashSet<_>>();

    if left_chars.is_empty() || right_chars.is_empty() {
        return 0.0;
    }

    let shared = left_chars.intersection(&right_chars).count() as f32;
    let denom = left_chars.len().min(right_chars.len()) as f32;
    (shared / denom).clamp(0.0, 1.0)
}

fn reflection_issue_penalty(issues: &[ReflectionIssue]) -> f32 {
    issues
        .iter()
        .map(|issue| match issue.severity {
            ReflectionSeverity::Info => 0.01,
            ReflectionSeverity::Warning => 0.04,
            ReflectionSeverity::Critical => 0.14,
        })
        .sum::<f32>()
        .min(0.36)
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('|', "\\p")
}

fn unescape_field(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('p') => out.push('|'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn records_and_ranks_lessons() {
        let mut store = ExperienceStore::new();
        store.record(input("weak", 0.2));
        store.record(input("strong", 0.9));

        let lessons = store.top_lessons(0.5, 4);

        assert_eq!(lessons.len(), 1);
        assert_eq!(lessons[0].lesson, "strong");
    }

    #[test]
    fn retrieves_relevant_lessons() {
        let mut store = ExperienceStore::new();
        store.record(ExperienceInput {
            prompt: "Rust adaptive router".to_owned(),
            lesson: "prefer token-window feedback for router stability".to_owned(),
            ..input("router", 0.9)
        });
        store.record(ExperienceInput {
            prompt: "long form story writing".to_owned(),
            profile: TaskProfile::Writing,
            lesson: "prefer global continuity".to_owned(),
            ..input("writing", 0.9)
        });

        let matches = store.retrieve_lessons("Rust router feedback", TaskProfile::Coding, 2);

        assert!(!matches.is_empty());
        assert!(matches[0].lesson.contains("router"));
    }

    #[test]
    fn disk_kv_roundtrip_preserves_experience() {
        let path = temp_path("experience");
        let mut store = ExperienceStore::new();
        let id = store.record(ExperienceInput {
            gist_records: vec![gist("document", GistLevel::Document, 0.88)],
            gist_memory_ids: vec![7, 8],
            ..input("stored", 0.87)
        });

        store.save_to_disk_kv(&path).unwrap();
        let loaded = ExperienceStore::load_from_disk_kv(&path).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.records()[0].id, id);
        assert_eq!(loaded.records()[0].lesson, "stored");
        assert_eq!(loaded.records()[0].profile, TaskProfile::Coding);
        assert_eq!(loaded.records()[0].gist_records.len(), 1);
        assert_eq!(loaded.records()[0].gist_memory_ids, vec![7, 8]);
        assert_eq!(loaded.records()[0].used_memory_ids, vec![3, 5]);
        assert_eq!(loaded.records()[0].stored_runtime_kv_memory_ids, vec![11]);
        assert!(
            loaded.records()[0]
                .process_reward
                .notes
                .iter()
                .any(|note| note.starts_with("memory_feedback:"))
        );
        assert_eq!(
            loaded.records()[0].runtime_diagnostics.model_id.as_deref(),
            Some("noiron-test-runtime")
        );
        assert_eq!(
            loaded.records()[0]
                .runtime_diagnostics
                .selected_adapter
                .as_deref(),
            Some("portable-rust")
        );
        assert_eq!(
            loaded.records()[0]
                .runtime_diagnostics
                .device_profile
                .as_deref(),
            Some("cpu")
        );
        assert_eq!(
            loaded.records()[0]
                .runtime_diagnostics
                .primary_lane
                .as_deref(),
            Some("cpu-vector")
        );
        assert_eq!(
            loaded.records()[0]
                .runtime_diagnostics
                .fallback_lane
                .as_deref(),
            Some("cpu-portable")
        );
        assert_eq!(
            loaded.records()[0]
                .runtime_diagnostics
                .memory_mode
                .as_deref(),
            Some("tiered-disk")
        );
        assert_eq!(loaded.records()[0].runtime_diagnostics.layer_count, 8);
        assert_eq!(
            loaded.records()[0].runtime_diagnostics.forward_energy,
            Some(0.25)
        );
        assert_eq!(
            loaded.records()[0].runtime_diagnostics.kv_influence,
            Some(0.75)
        );
        assert_eq!(loaded.records()[0].runtime_token_metrics.token_count, 3);
        assert_eq!(loaded.records()[0].runtime_token_metrics.entropy_count, 3);
        assert_eq!(loaded.records()[0].runtime_token_metrics.logprob_count, 2);
        assert_eq!(
            loaded.records()[0].runtime_token_metrics.average_entropy,
            Some(0.42)
        );
        assert_eq!(
            loaded.records()[0]
                .runtime_token_metrics
                .average_neg_logprob,
            Some(0.70)
        );
        assert_eq!(
            loaded.records()[0]
                .runtime_token_metrics
                .uncertainty_perplexity,
            Some(4.38)
        );
        assert_eq!(
            loaded.records()[0]
                .runtime_diagnostics
                .hot_kv_precision_bits,
            Some(8)
        );
        assert_eq!(
            loaded.records()[0]
                .runtime_diagnostics
                .cold_kv_precision_bits,
            Some(4)
        );
        assert!(
            loaded.records()[0]
                .runtime_diagnostics
                .has_valid_kv_precision_signal()
        );
        assert_eq!(loaded.records()[0].reflection_issues.len(), 1);
        assert_eq!(
            loaded.records()[0].reflection_issues[0].severity,
            ReflectionSeverity::Warning
        );
        assert_eq!(
            loaded.records()[0].revision_actions,
            vec!["revise_reflection_signal".to_owned()]
        );
        assert!((loaded.records()[0].route_budget.attention_fraction - 0.4).abs() < 0.0001);
        assert!((loaded.records()[0].process_reward.total - 0.5).abs() < 0.0001);
        assert!(
            (loaded.records()[0].live_evolution.router_threshold_delta - 0.030000).abs() < 0.0001
        );
        assert!(
            (loaded.records()[0].live_evolution.hierarchy_weight_delta - 0.040000).abs() < 0.0001
        );
        assert_eq!(
            loaded.records()[0].live_evolution.online_reward_feedbacks,
            1
        );
        assert_eq!(
            loaded.records()[0]
                .live_evolution
                .online_reward_reinforcements,
            1
        );
        assert_eq!(
            loaded.records()[0].live_evolution.online_reward_penalties,
            0
        );
        assert!((loaded.records()[0].live_evolution.online_reward_strength - 0.72).abs() < 0.0001);
        assert!(
            (loaded.records()[0]
                .live_evolution
                .online_reward_reinforcement_strength
                - 0.72)
                .abs()
                < 0.0001
        );
        assert_eq!(
            loaded.records()[0]
                .live_evolution
                .online_reward_penalty_strength,
            0.0
        );
        assert_eq!(loaded.records()[0].live_evolution.memory_reinforcements, 1);
        assert_eq!(loaded.records()[0].live_evolution.memory_penalties, 0);
        assert!(loaded.records()[0].live_evolution.stored_memory);
        assert_eq!(loaded.records()[0].live_evolution.stored_gist_memories, 2);
        assert_eq!(
            loaded.records()[0]
                .live_evolution
                .stored_runtime_kv_memories,
            1
        );
        assert_eq!(loaded.records()[0].live_evolution.reflection_issues, 1);
        assert_eq!(
            loaded.records()[0]
                .live_evolution
                .critical_reflection_issues,
            0
        );
        assert_eq!(loaded.records()[0].live_evolution.revision_actions, 1);
        cleanup(path);
    }

    #[test]
    fn legacy_live_evolution_without_online_reward_feedback_loads_defaults() {
        let loaded = deserialize_live_evolution("0.030000,0.040000,1,0,1,2,1,1,0,1").unwrap();

        assert_eq!(loaded.online_reward_feedbacks, 0);
        assert_eq!(loaded.online_reward_reinforcements, 0);
        assert_eq!(loaded.online_reward_penalties, 0);
        assert_eq!(loaded.online_reward_strength, 0.0);
        assert_eq!(loaded.online_reward_reinforcement_strength, 0.0);
        assert_eq!(loaded.online_reward_penalty_strength, 0.0);
        assert_eq!(loaded.memory_reinforcements, 1);
        assert_eq!(loaded.stored_gist_memories, 2);
        assert_eq!(loaded.revision_actions, 1);
    }

    #[test]
    fn legacy_live_evolution_without_online_reward_strength_loads_defaults() {
        let loaded = deserialize_live_evolution("0.030000,0.040000,1,1,0,1,0,1,2,1,1,0,1").unwrap();

        assert_eq!(loaded.online_reward_feedbacks, 1);
        assert_eq!(loaded.online_reward_reinforcements, 1);
        assert_eq!(loaded.online_reward_penalties, 0);
        assert_eq!(loaded.online_reward_strength, 0.0);
        assert_eq!(loaded.online_reward_reinforcement_strength, 0.0);
        assert_eq!(loaded.online_reward_penalty_strength, 0.0);
        assert_eq!(loaded.memory_reinforcements, 1);
        assert_eq!(loaded.stored_runtime_kv_memories, 1);
        assert_eq!(loaded.revision_actions, 1);
    }

    #[test]
    fn deserialize_live_evolution_rejects_online_reward_feedback_count_mismatch() {
        assert!(
            deserialize_live_evolution(
                "0.030000,0.040000,2,1,0,0.720000,0.720000,0.000000,1,0,1,2,1,1,0,1"
            )
            .is_none()
        );
    }

    #[test]
    fn deserialize_live_evolution_rejects_online_reward_strength_total_mismatch() {
        assert!(
            deserialize_live_evolution(
                "0.030000,0.040000,2,1,1,0.720000,0.720000,0.250000,1,0,1,2,1,1,0,1"
            )
            .is_none()
        );
    }

    #[test]
    fn deserialize_live_evolution_rejects_feedback_without_strength_when_strength_fields_present() {
        assert!(
            deserialize_live_evolution(
                "0.030000,0.040000,1,1,0,0.000000,0.000000,0.000000,1,0,1,2,1,1,0,1"
            )
            .is_none()
        );
    }

    #[test]
    fn deserialize_live_evolution_rejects_component_strength_without_count() {
        assert!(
            deserialize_live_evolution(
                "0.030000,0.040000,1,1,0,0.920000,0.720000,0.200000,1,0,1,2,1,1,0,1"
            )
            .is_none()
        );
    }

    #[test]
    fn deserialize_record_rejects_malformed_live_evolution_field() {
        let mut store = ExperienceStore::new();
        store.record(input("malformed live evolution", 0.82));
        let current = serialize_record(&store.records()[0]);
        let mut fields = current.split('\t').map(str::to_owned).collect::<Vec<_>>();
        fields[21] =
            escape_field("0.030000,0.040000,1,1,0,0.000000,0.000000,0.000000,1,0,1,2,1,1,0,1");
        let malformed = fields.join("\t");

        assert!(deserialize_record(&malformed).is_none());
    }

    #[test]
    fn legacy_experience_records_without_runtime_token_metrics_load_defaults() {
        let mut store = ExperienceStore::new();
        store.record(input("legacy", 0.82));
        let current = serialize_record(&store.records()[0]);
        let legacy = current
            .rsplit_once('\t')
            .map(|(legacy, _)| legacy)
            .unwrap_or(&current);

        let loaded = deserialize_record(legacy).unwrap();

        assert_eq!(loaded.lesson, "legacy");
        assert_eq!(
            loaded.runtime_token_metrics,
            ExperienceRuntimeTokenMetrics::default()
        );
        assert!(!loaded.runtime_token_metrics.has_uncertainty_signal());
    }

    #[test]
    fn retrieve_lessons_includes_gist_hints() {
        let mut store = ExperienceStore::new();
        store.record(ExperienceInput {
            prompt: "long context scheduler".to_owned(),
            lesson: "reuse recursive chunk summaries".to_owned(),
            gist_records: vec![gist(
                "recursive chunks preserve overlap",
                GistLevel::Section,
                0.91,
            )],
            ..input("gist", 0.9)
        });

        let matches = store.retrieve_lessons("recursive overlap", TaskProfile::Coding, 1);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].gist_hints.len(), 1);
        assert!(matches[0].gist_hints[0].contains("recursive chunks"));
        assert_eq!(matches[0].reward_action, RewardAction::Hold);
    }

    #[test]
    fn retrieval_uses_reflection_issue_text_but_penalizes_severity() {
        let mut store = ExperienceStore::new();
        store.record(ExperienceInput {
            prompt: "generic prompt".to_owned(),
            lesson: "avoid repeating weak answers".to_owned(),
            quality: 0.86,
            reflection_issues: vec![ReflectionIssue::new(
                "repetitive_answer",
                ReflectionSeverity::Warning,
                "deduplicate repeated phrases",
            )],
            revision_actions: vec!["deduplicate_repeated_phrases".to_owned()],
            ..input("issue", 0.86)
        });

        let matches =
            store.retrieve_lessons("deduplicate repeated phrases", TaskProfile::Coding, 1);

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].reflection_issue_codes,
            vec!["repetitive_answer".to_owned()]
        );
        assert_eq!(
            matches[0].revision_actions,
            vec!["deduplicate_repeated_phrases".to_owned()]
        );
    }

    #[test]
    fn retrieval_exposes_runtime_diagnostics() {
        let mut store = ExperienceStore::new();
        store.record(ExperienceInput {
            prompt: "adapter selection for local runtime".to_owned(),
            lesson: "reuse portable-rust runtime diagnostics".to_owned(),
            runtime_diagnostics: RuntimeDiagnostics {
                model_id: Some("noiron-runtime-v2".to_owned()),
                selected_adapter: Some("portable-rust".to_owned()),
                device_profile: Some("cpu".to_owned()),
                primary_lane: Some("cpu-vector".to_owned()),
                fallback_lane: Some("cpu-portable".to_owned()),
                memory_mode: Some("tiered-disk".to_owned()),
                layer_count: 16,
                global_layers: 4,
                local_window_layers: 8,
                convolutional_fusion_layers: 4,
                hidden_size: 128,
                local_window_tokens: 4096,
                forward_energy: Some(0.33),
                kv_influence: Some(0.44),
                imported_kv_blocks: 2,
                exported_kv_blocks: 3,
                hot_kv_precision_bits: Some(8),
                cold_kv_precision_bits: Some(4),
            },
            ..input("runtime", 0.9)
        });

        let matches = store.retrieve_lessons("portable-rust adapter", TaskProfile::Coding, 1);

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].runtime_model_id.as_deref(),
            Some("noiron-runtime-v2")
        );
        assert_eq!(
            matches[0].runtime_selected_adapter.as_deref(),
            Some("portable-rust")
        );
        assert_eq!(matches[0].runtime_device_profile.as_deref(), Some("cpu"));
        assert_eq!(
            matches[0].runtime_primary_lane.as_deref(),
            Some("cpu-vector")
        );
        assert_eq!(
            matches[0].runtime_fallback_lane.as_deref(),
            Some("cpu-portable")
        );
        assert_eq!(
            matches[0].runtime_memory_mode.as_deref(),
            Some("tiered-disk")
        );
        assert_eq!(matches[0].runtime_forward_energy, Some(0.33));
        assert_eq!(matches[0].runtime_kv_influence, Some(0.44));
        assert_eq!(matches[0].runtime_uncertainty_perplexity, Some(4.38));
    }

    #[test]
    fn legacy_runtime_diagnostics_deserialize_without_kv_precision() {
        let legacy = [
            "model",
            "portable-rust",
            "cpu",
            "cpu-vector",
            "cpu-portable",
            "tiered-disk",
            "8",
            "2",
            "4",
            "2",
            "64",
            "2048",
            "0.250000",
            "0.750000",
            "1",
            "2",
        ]
        .join("\u{1f}");

        let diagnostics = deserialize_runtime_diagnostics(&legacy).unwrap();

        assert_eq!(diagnostics.model_id.as_deref(), Some("model"));
        assert_eq!(diagnostics.hot_kv_precision_bits, None);
        assert_eq!(diagnostics.cold_kv_precision_bits, None);
        assert!(!diagnostics.has_valid_kv_precision_signal());
    }

    #[test]
    fn retrieval_exposes_recursive_runtime_calls_from_reward_notes() {
        let mut store = ExperienceStore::new();
        store.record(ExperienceInput {
            prompt: "long document recursive runtime".to_owned(),
            lesson: "expensive recursive runtime calls should be reusable control feedback"
                .to_owned(),
            process_reward: ProcessRewardReport {
                total: 0.77,
                action: RewardAction::Reinforce,
                components: ProcessRewardComponents::default(),
                notes: vec![
                    "recursive:chunks=8:merge_rounds=2:waves=4:parallel=2:runtime_calls=13"
                        .to_owned(),
                ],
            },
            ..input("recursive runtime", 0.9)
        });

        let matches = store.retrieve_lessons("runtime_calls", TaskProfile::LongDocument, 1);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].recursive_runtime_calls, Some(13));
    }

    fn input(lesson: &str, quality: f32) -> ExperienceInput {
        ExperienceInput {
            prompt: "build a Noiron loop".to_owned(),
            profile: TaskProfile::Coding,
            lesson: lesson.to_owned(),
            quality,
            contradictions: Vec::new(),
            reflection_issues: vec![ReflectionIssue::new(
                "needs_grounding",
                ReflectionSeverity::Warning,
                "needs grounding detail",
            )],
            revision_actions: vec!["revise_reflection_signal".to_owned()],
            stored_memory_id: Some(42),
            router_threshold_after: 0.55,
            stream_windows: 3,
            route_budget: RouteBudget {
                threshold: 0.55,
                attention_tokens: 2,
                fast_tokens: 3,
                attention_fraction: 0.4,
            },
            hierarchy: HierarchyWeights::new(0.2, 0.6, 0.2),
            used_memory_ids: vec![3, 5],
            gist_records: Vec::new(),
            gist_memory_ids: Vec::new(),
            stored_runtime_kv_memory_ids: vec![11],
            runtime_token_metrics: ExperienceRuntimeTokenMetrics {
                token_count: 3,
                entropy_count: 3,
                logprob_count: 2,
                average_entropy: Some(0.42),
                average_neg_logprob: Some(0.70),
                uncertainty_perplexity: Some(4.38),
            },
            runtime_diagnostics: RuntimeDiagnostics {
                model_id: Some("noiron-test-runtime".to_owned()),
                selected_adapter: Some("portable-rust".to_owned()),
                device_profile: Some("cpu".to_owned()),
                primary_lane: Some("cpu-vector".to_owned()),
                fallback_lane: Some("cpu-portable".to_owned()),
                memory_mode: Some("tiered-disk".to_owned()),
                layer_count: 8,
                global_layers: 2,
                local_window_layers: 4,
                convolutional_fusion_layers: 2,
                hidden_size: 64,
                local_window_tokens: 2048,
                forward_energy: Some(0.25),
                kv_influence: Some(0.75),
                imported_kv_blocks: 1,
                exported_kv_blocks: 2,
                hot_kv_precision_bits: Some(8),
                cold_kv_precision_bits: Some(4),
            },
            process_reward: ProcessRewardReport {
                notes: vec![
                    "memory_feedback:reinforced=1:penalized=0:reinforcement_amount=0.820000:penalty_amount=0.000000"
                        .to_owned(),
                ],
                ..ProcessRewardReport::default()
            },
            live_evolution: LiveInferenceEvolution {
                router_threshold_delta: 0.03,
                hierarchy_weight_delta: 0.04,
                online_reward_feedbacks: 1,
                online_reward_reinforcements: 1,
                online_reward_penalties: 0,
                online_reward_strength: 0.72,
                online_reward_reinforcement_strength: 0.72,
                online_reward_penalty_strength: 0.0,
                memory_reinforcements: 1,
                memory_penalties: 0,
                stored_memory: true,
                stored_gist_memories: 2,
                stored_runtime_kv_memories: 1,
                reflection_issues: 1,
                critical_reflection_issues: 0,
                revision_actions: 1,
            },
        }
    }

    fn gist(summary: &str, level: GistLevel, importance: f32) -> GistRecord {
        GistRecord {
            level,
            title: "gist title".to_owned(),
            summary: summary.to_owned(),
            source_tokens: 8,
            importance,
        }
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rust-norion-{label}-{}-{nanos}.ndkv",
            std::process::id()
        ))
    }

    fn cleanup(path: std::path::PathBuf) {
        let _ = fs::remove_file(path);
    }
}
