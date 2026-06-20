use super::{TraceRequiredField, required_field};

pub(super) const RETENTION_TRACE_REQUIRED_FIELDS: &[TraceRequiredField] = &[
    required_field("retention", "\"retention\":{"),
    required_field("remove_below_strength", "\"remove_below_strength\":"),
    required_field("remove_after_failures", "\"remove_after_failures\":"),
    required_field("memory_compaction", "\"memory_compaction\":{"),
    required_field("similarity_threshold", "\"similarity_threshold\":"),
    required_field("max_merges", "\"max_merges\":"),
    required_field("experience_id", "\"experience_id\":"),
];
