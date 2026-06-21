use super::{TraceRequiredField, required_field};

pub(super) const ROUTING_TRACE_REQUIRED_FIELDS: &[TraceRequiredField] = &[
    required_field("adaptive_routing", "\"adaptive_routing\":{"),
    required_field("adaptive_routing_candidates", "\"candidates\":"),
    required_field("adaptive_routing_include", "\"include\":"),
    required_field("adaptive_routing_compress", "\"compress\":"),
    required_field("adaptive_routing_defer", "\"defer\":"),
    required_field("adaptive_routing_skip", "\"skip\":"),
    required_field("adaptive_routing_input_tokens", "\"input_tokens\":"),
    required_field("adaptive_routing_retained_tokens", "\"retained_tokens\":"),
    required_field("adaptive_routing_saved_tokens", "\"saved_tokens\":"),
    required_field("adaptive_routing_min_score", "\"min_score\":"),
    required_field("adaptive_routing_max_score", "\"max_score\":"),
    required_field("adaptive_routing_average_score", "\"average_score\":"),
    required_field("adaptive_routing_actions", "\"actions\":"),
    required_field("adaptive_routing_selected_routes", "\"selected_routes\":"),
    required_field("adaptive_routing_score_summaries", "\"score_summaries\":"),
    required_field("adaptive_routing_read_only", "\"read_only\":"),
    required_field("adaptive_routing_write_allowed", "\"write_allowed\":"),
    required_field("adaptive_routing_applied", "\"applied\":"),
];
