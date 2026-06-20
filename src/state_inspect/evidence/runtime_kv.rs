use crate::experience::ExperienceRecord;

pub(in crate::state_inspect) fn runtime_kv_held_blocks(record: &ExperienceRecord) -> usize {
    record
        .runtime_diagnostics
        .exported_kv_blocks
        .saturating_sub(record.stored_runtime_kv_memory_ids.len())
}

pub(in crate::state_inspect) fn runtime_kv_was_held(record: &ExperienceRecord) -> bool {
    runtime_kv_held_blocks(record) > 0
}
