use std::path::{Path, PathBuf};

use crate::kv_quant::{QuantizationBits, QuantizedVector};

use super::MemoryEntry;

pub(super) fn serialize_entry(entry: &MemoryEntry) -> String {
    let vector = entry
        .vector
        .iter()
        .map(|value| format!("{value:.6}"))
        .collect::<Vec<_>>()
        .join(",");

    serialize_entry_with_vector(entry, &vector)
}

pub(super) fn serialize_entry_quantized(entry: &MemoryEntry, bits: QuantizationBits) -> String {
    let vector = QuantizedVector::quantize(&entry.vector, bits).encode();
    serialize_entry_with_vector(entry, &vector)
}

pub(super) fn deserialize_entry(line: &str) -> Option<MemoryEntry> {
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() != 7 && fields.len() != 9 {
        return None;
    }

    let id = fields[0].parse::<u64>().ok()?;
    let strength = fields[1].parse::<f32>().ok()?;
    let hits = fields[2].parse::<u64>().ok()?;
    let failures = fields[3].parse::<u64>().ok()?;
    let last_score = fields[4].parse::<f32>().ok()?;
    let (created_at, last_access, key, vector) = match fields.len() {
        7 => (
            0,
            hits.saturating_add(failures),
            unescape_field(fields[5]),
            deserialize_vector(fields[6])?,
        ),
        9 => (
            fields[5].parse::<u64>().ok()?,
            fields[6].parse::<u64>().ok()?,
            unescape_field(fields[7]),
            deserialize_vector(fields[8])?,
        ),
        _ => return None,
    };

    Some(MemoryEntry {
        id,
        key,
        vector,
        strength,
        hits,
        failures,
        last_score,
        created_at,
        last_access,
    })
}

pub(super) fn legacy_backup_path(path: &Path) -> PathBuf {
    for index in 0..1024 {
        let extension = if index == 0 {
            "legacy.tsv".to_owned()
        } else {
            format!("legacy.{index}.tsv")
        };
        let candidate = path.with_extension(extension);
        if !candidate.exists() {
            return candidate;
        }
    }

    path.with_extension("legacy.tsv")
}

fn serialize_entry_with_vector(entry: &MemoryEntry, vector: &str) -> String {
    format!(
        "{}\t{:.6}\t{}\t{}\t{:.6}\t{}\t{}\t{}\t{}",
        entry.id,
        entry.strength,
        entry.hits,
        entry.failures,
        entry.last_score,
        entry.created_at,
        entry.last_access,
        escape_field(&entry.key),
        vector
    )
}

fn deserialize_vector(encoded: &str) -> Option<Vec<f32>> {
    if encoded.starts_with('q') {
        return QuantizedVector::decode(encoded)
            .ok()
            .map(|vector| vector.dequantize());
    }

    if encoded.is_empty() {
        return Some(Vec::new());
    }

    Some(
        encoded
            .split(',')
            .filter_map(|value| value.parse::<f32>().ok())
            .collect::<Vec<_>>(),
    )
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
