use crate::adaptive_state::LiveInferenceEvolution;
use crate::hierarchy::HierarchyWeights;
use crate::router::RouteBudget;

use super::model::ExperienceRecord;

mod fields;
mod gists;
mod live_evolution;
mod profile;
mod reflection_notes;
mod reward;
mod route_budget;
mod runtime_diagnostics;
mod runtime_tokens;

use fields::unescape_field;
use gists::{deserialize_gists, serialize_gists};
pub(super) use live_evolution::deserialize_live_evolution;
use live_evolution::serialize_live_evolution;
use profile::{profile_to_str, str_to_profile};
use reflection_notes::{
    deserialize_reflection_issues, deserialize_revision_actions, serialize_reflection_issues,
    serialize_revision_actions,
};
use reward::{deserialize_process_reward, serialize_process_reward};
use route_budget::{deserialize_route_budget, serialize_route_budget};
pub(super) use runtime_diagnostics::{
    deserialize_runtime_diagnostics, serialize_runtime_diagnostics,
};
use runtime_tokens::{deserialize_runtime_token_metrics, serialize_runtime_token_metrics};

pub(super) fn escape_field(value: &str) -> String {
    fields::escape_field(value)
}

pub(super) fn serialize_record(record: &ExperienceRecord) -> String {
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

pub(super) fn deserialize_record(line: &str) -> Option<ExperienceRecord> {
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

fn serialize_ids(ids: &[u64]) -> String {
    ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",")
}

fn deserialize_ids(value: &str) -> Vec<u64> {
    value
        .split(',')
        .filter_map(|item| item.parse::<u64>().ok())
        .collect()
}
