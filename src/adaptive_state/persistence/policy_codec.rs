use crate::kv_cache::{MemoryCompactionPolicy, MemoryRetentionPolicy};
use crate::tiered_cache::{MemoryPlacement, MemoryTier, TieredCachePlan};

pub(super) fn serialize_tier_plan(plan: &TieredCachePlan) -> String {
    plan.placements()
        .iter()
        .map(|placement| {
            format!(
                "{}\t{}\t{:.6}\t{}",
                placement.id,
                placement.tier.as_str(),
                placement.score,
                escape_field(&placement.reason)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn parse_tier_plan(value: &str) -> TieredCachePlan {
    let placements = value
        .lines()
        .filter_map(parse_memory_placement)
        .collect::<Vec<_>>();
    TieredCachePlan::new(placements)
}

pub(super) fn serialize_memory_retention_policy(policy: MemoryRetentionPolicy) -> String {
    format!(
        "{}\t{:.6}\t{:.6}\t{}",
        policy.stale_after,
        policy.decay_rate,
        policy.remove_below_strength,
        policy.remove_after_failures
    )
}

pub(super) fn parse_memory_retention_policy(value: &str) -> Option<MemoryRetentionPolicy> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 4 {
        return None;
    }

    Some(MemoryRetentionPolicy {
        stale_after: fields[0].parse::<u64>().ok()?.max(1),
        decay_rate: fields[1].parse::<f32>().ok()?.clamp(0.0, 0.95),
        remove_below_strength: fields[2].parse::<f32>().ok()?.clamp(0.0, 3.0),
        remove_after_failures: fields[3].parse::<u64>().ok()?.max(1),
    })
}

pub(super) fn serialize_memory_compaction_policy(policy: &MemoryCompactionPolicy) -> String {
    format!(
        "{:.6}\t{}\t{}",
        policy.similarity_threshold, policy.max_candidates, policy.max_merges
    )
}

pub(super) fn parse_memory_compaction_policy(value: &str) -> Option<MemoryCompactionPolicy> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 3 {
        return None;
    }

    Some(MemoryCompactionPolicy {
        similarity_threshold: fields[0].parse::<f32>().ok()?.clamp(0.10, 0.999),
        max_candidates: fields[1].parse::<usize>().ok()?.max(2),
        max_merges: fields[2].parse::<usize>().ok()?,
    })
}

fn parse_memory_placement(value: &str) -> Option<MemoryPlacement> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != 4 {
        return None;
    }

    Some(MemoryPlacement {
        id: fields[0].parse::<u64>().ok()?,
        tier: fields[1].parse::<MemoryTier>().ok()?,
        score: fields[2].parse::<f32>().ok()?,
        reason: unescape_field(fields[3]),
    })
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
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
