use crate::gist_memory::{GistLevel, GistRecord};

use super::fields::sanitize_control_part;

pub(super) fn serialize_gists(records: &[GistRecord]) -> String {
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

pub(super) fn deserialize_gists(value: &str) -> Vec<GistRecord> {
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
                level: fields[0].parse::<GistLevel>().ok()?,
                importance: fields[1].parse::<f32>().ok()?.clamp(0.0, 1.0),
                source_tokens: fields[2].parse::<usize>().ok()?,
                title: fields[3].to_owned(),
                summary: fields[4].to_owned(),
            })
        })
        .collect()
}

fn sanitize_gist_part(value: &str) -> String {
    sanitize_control_part(value)
}
