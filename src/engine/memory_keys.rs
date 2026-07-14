use crate::gist_memory::GistRecord;
use crate::kv_cache::MemoryMatch;
use crate::kv_exchange::RuntimeKvBlock;

use super::text::compact;

pub(super) fn summarize_key(prompt: &str, lesson: &str) -> String {
    format!("{} :: {}", compact(prompt, 96), compact(lesson, 64))
}

pub(super) fn format_gist_key(prompt: &str, gist: &GistRecord) -> String {
    format!(
        "gist:{}:{} :: {}",
        gist.level.as_str(),
        compact(prompt, 64),
        compact(&gist.title, 64)
    )
}

pub(super) fn format_runtime_kv_key(prompt: &str, block: &RuntimeKvBlock) -> String {
    format!(
        "runtime_kv:l{}h{}:{}-{}:k{}v{} :: {}",
        block.layer,
        block.head,
        block.token_start,
        block.token_end,
        block.key.len(),
        block.value.len(),
        compact(prompt, 64)
    )
}

pub(super) fn protected_memory_ids(
    used_memories: &[MemoryMatch],
    stored_memory_id: Option<u64>,
    stored_gist_memory_ids: &[u64],
    stored_runtime_kv_memory_ids: &[u64],
) -> Vec<u64> {
    let mut ids = used_memories
        .iter()
        .map(|memory| memory.id)
        .collect::<Vec<_>>();
    if let Some(id) = stored_memory_id {
        ids.push(id);
    }
    ids.extend_from_slice(stored_gist_memory_ids);
    ids.extend_from_slice(stored_runtime_kv_memory_ids);
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub(super) fn retention_protected_memory_ids(
    used_memories: &[MemoryMatch],
    stored_memory_id: Option<u64>,
    stored_gist_memory_ids: &[u64],
    stored_runtime_kv_memory_ids: &[u64],
) -> Vec<u64> {
    let mut ids = used_memories
        .iter()
        .filter(|memory| is_rollback_anchor_key(&memory.key))
        .map(|memory| memory.id)
        .collect::<Vec<_>>();
    if let Some(id) = stored_memory_id {
        ids.push(id);
    }
    ids.extend_from_slice(stored_gist_memory_ids);
    ids.extend_from_slice(stored_runtime_kv_memory_ids);
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn is_rollback_anchor_key(key: &str) -> bool {
    key.contains("rollback-anchor") || key.contains("rollback_anchor")
}
