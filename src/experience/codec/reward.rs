use crate::process_reward::{ProcessRewardComponents, ProcessRewardReport, RewardAction};

use super::fields::sanitize_control_part;

pub(super) fn serialize_process_reward(report: &ProcessRewardReport) -> String {
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

pub(super) fn deserialize_process_reward(value: &str) -> Option<ProcessRewardReport> {
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
        action: fields[1].parse::<RewardAction>().ok()?,
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
