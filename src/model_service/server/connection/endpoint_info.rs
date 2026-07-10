use std::net::TcpStream;

use super::super::super::json::{service_json_string, write_http_json};
use crate::Args;

pub(super) fn handle_endpoint_info(
    stream: &mut TcpStream,
    request_id: usize,
    endpoint: &str,
) -> std::io::Result<()> {
    let body = model_service_endpoint_info_json(request_id, endpoint);
    write_http_json(stream, 200, "OK", &body)
}

pub(super) fn handle_model_capabilities(
    stream: &mut TcpStream,
    request_id: usize,
    args: &Args,
) -> std::io::Result<()> {
    let body = model_service_model_capabilities_json(request_id, args);
    write_http_json(stream, 200, "OK", &body)
}

fn model_service_endpoint_info_json(request_id: usize, endpoint: &str) -> String {
    let spec = EndpointInfoSpec::for_endpoint(endpoint);
    let method = endpoint_method(endpoint);
    let response_fields = endpoint_response_fields(endpoint);
    let stream_response_fields = endpoint_stream_response_fields(endpoint);
    let stream_response_fields_json = if stream_response_fields.is_empty() {
        String::new()
    } else {
        format!(
            ",\"stream_response_fields\":{}",
            str_array_json(stream_response_fields)
        )
    };
    format!(
        "{{\"ok\":true,\"request_id\":{},\"endpoint\":\"{}\",\"method\":\"{}\",\"content_type\":\"application/json\",\"example\":{},\"supported_fields\":{},\"response_fields\":{},\"unsupported_fields\":{}{}}}",
        request_id,
        spec.path,
        method,
        spec.example,
        str_array_json(spec.supported_fields),
        str_array_json(response_fields),
        str_array_json(spec.unsupported_fields),
        stream_response_fields_json
    )
}

fn model_service_model_capabilities_json(request_id: usize, args: &Args) -> String {
    format!(
        "{{\"object\":\"list\",\"data\":[{{\"id\":\"rust-norion-local\",\"object\":\"model\",\"created\":0,\"owned_by\":\"rust-norion\",\"root\":\"rust-norion-local\",\"parent\":null,\"norion\":{{\"display_name\":\"北极星\",\"runtime_mode\":\"{}\",\"supported_endpoints\":{},\"supported_request_fields\":{},\"unsupported_features\":{},\"capabilities\":{{\"chat\":true,\"completions\":true,\"streaming\":true,\"cancellation\":true,\"max_tokens\":true,\"diagnostics\":true,\"state_inspection\":true,\"feedback\":true,\"rust_check\":true,\"experience_replay\":true,\"hierarchical_routing\":true,\"experience_retrieval\":true,\"experience_hygiene_quarantine\":true,\"experience_cleanup_audit\":true,\"experience_repair\":true,\"persistent_kv_memory\":true,\"self_improvement\":true,\"weight_retraining_required\":false}}}}}}],\"norion\":{{\"request_id\":{},\"default_model\":\"rust-norion-local\",\"diagnostics_endpoint\":\"/v1/diagnostics\",\"health_response_fields\":{},\"diagnostics_response_fields\":{},\"contracts_endpoint\":\"GET /v1/{{endpoint}}\"}}}}",
        model_service_runtime_mode(args),
        str_array_json(MODEL_SERVICE_SUPPORTED_ENDPOINTS),
        str_array_json(MODEL_SERVICE_SUPPORTED_REQUEST_FIELDS),
        str_array_json(MODEL_SERVICE_UNSUPPORTED_FEATURES),
        request_id,
        str_array_json(HEALTH_DIAGNOSTICS_RESPONSE_FIELDS),
        str_array_json(HEALTH_DIAGNOSTICS_RESPONSE_FIELDS)
    )
}

const MODEL_SERVICE_SUPPORTED_ENDPOINTS: &[&str] = &[
    "/v1/models",
    "/v1/chat/completions",
    "/v1/completions",
    "/v1/generate",
    "/v1/chat",
    "/v1/generate-stream",
    "/v1/chat-stream",
    "/v1/state",
    "/v1/inspect",
    "/v1/feedback",
    "/v1/rust-check",
    "/v1/replay",
    "/v1/self-improve",
    "/v1/business-cycle",
    "/v1/business-cycle-stream",
    "/v1/experience-retrieval",
    "/v1/experience-hygiene/quarantine",
    "/v1/experience-cleanup-audit",
    "/v1/experience-repair",
    "/v1/model-pool/route-plan",
    "/v1/model-pool/call",
    "/v1/requests/cancel",
    "/v1/diagnostics",
    "/health",
];

const MODEL_SERVICE_SUPPORTED_REQUEST_FIELDS: &[&str] = &[
    "model",
    "messages",
    "prompt",
    "stream",
    "max_tokens",
    "max_completion_tokens",
    "n",
    "case",
    "output",
    "experience_id",
    "memory_id",
    "action",
    "amount",
    "code",
    "edition",
    "limit",
    "gate",
    "state_gate",
    "business_gate",
    "business_cycle_gate",
    "model_service_gate",
    "trace_gate",
    "tenant_id",
    "workspace_id",
    "session_id",
];

const MODEL_SERVICE_UNSUPPORTED_FEATURES: &[&str] = &[
    "tools",
    "tool_choice",
    "response_format",
    "logprobs",
    "multiple_choices",
    "sampling_controls",
    "stop_sequences",
    "stream_usage_chunks",
];

fn model_service_runtime_mode(args: &Args) -> &'static str {
    if args.gemma_runtime_server.is_some() {
        "gemma-http"
    } else if args.gemma_12b_runtime {
        "gemma-command"
    } else {
        "built-in"
    }
}

fn endpoint_method(endpoint: &str) -> &'static str {
    match endpoint {
        "state" => "GET",
        _ => "POST",
    }
}

fn str_array_json(values: &[&str]) -> String {
    let items = values
        .iter()
        .map(|value| service_json_string(value))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

struct EndpointInfoSpec {
    path: &'static str,
    example: &'static str,
    supported_fields: &'static [&'static str],
    unsupported_fields: &'static [&'static str],
}

const BUSINESS_CYCLE_SUPPORTED_FIELDS: &[&str] = &[
    "prompt",
    "profile",
    "case",
    "max_tokens",
    "max",
    "feedback_action",
    "action",
    "feedback_amount",
    "amount",
    "rust_check_code",
    "code",
    "rust_check_edition",
    "edition",
    "rust_check_case",
    "rust_case",
    "self_improve",
    "self_improve_limit",
    "limit",
    "pool_dispatch",
    "pool_stage_dispatch",
    "gate",
    "trace_gate",
    "tenant_id",
    "workspace_id",
    "session_id",
];

const BUSINESS_CYCLE_UNSUPPORTED_FIELDS: &[&str] = &[
    "model",
    "messages",
    "stream",
    "tools",
    "tool_choice",
    "response_format",
    "logprobs",
];

const HEALTH_DIAGNOSTICS_RESPONSE_FIELDS: &[&str] = &[
    "ok",
    "service",
    "display_name",
    "requests_seen",
    "active_engine_requests",
    "stream_backpressure_rejections",
    "engine_busy",
    "active_requests",
    "active_requests.request_id",
    "active_requests.endpoint",
    "active_requests.elapsed_ms",
    "active_requests.prompt_preview",
    "active_requests.cancel_requested",
    "active_requests.repair_factor",
    "active_requests.retag_label",
    "active_requests.cancel_reason",
    "request_id",
    "endpoint",
    "elapsed_ms",
    "prompt_preview",
    "cancel_requested",
    "repair_factor",
    "retag_label",
    "cancel_reason",
    "runtime_mode",
    "gemma_runtime_server",
    "gemma_runtime_reachable",
    "gemma_runtime_model",
    "gemma_runtime_context_window",
    "gemma_runtime_train_context_window",
    "gemma_runtime_vocab_size",
    "gemma_runtime_metadata_error",
    "experience_hygiene",
    "experience_hygiene.experience_file",
    "experience_hygiene.checked",
    "experience_hygiene.clean",
    "experience_hygiene.findings",
    "experience_hygiene.watch",
    "experience_hygiene.quarantine_candidates",
    "experience_hygiene.legacy_metadata_lessons",
    "experience_hygiene.legacy_metadata_without_clean_gist",
    "experience_hygiene.repair",
    "experience_hygiene.repair.repairable_legacy_metadata_lessons",
    "experience_hygiene.repair.repairable_index_records",
    "experience_hygiene.repair.projected_findings_after_repair",
    "experience_hygiene.repair.projected_watch_after_repair",
    "experience_hygiene.repair.projected_quarantine_candidates_after_repair",
    "experience_hygiene.repair.projected_legacy_metadata_lessons_after_repair",
    "experience_hygiene.repair.projected_legacy_metadata_without_clean_gist_after_repair",
    "experience_hygiene.repair.skipped_quarantine_candidates",
    "experience_hygiene.repair.skipped_missing_clean_gist",
    "experience_hygiene.index",
    "experience_hygiene.index.total_records",
    "experience_hygiene.index.compacted_records",
    "experience_hygiene.index.noisy_records",
    "experience_hygiene.index.duplicate_outputs",
    "experience_hygiene.index.max_noise_penalty",
    "experience_hygiene.index.quality_score",
    "experience_hygiene.index.retrieval_ready",
    "experience_hygiene.index.risk_level",
    "experience_hygiene.error",
    "readiness_ok",
    "safe_device_ok",
    "readiness_failures",
    "safe_device_failures",
    "device_profile",
    "device_reason",
    "device_accelerators",
    "device_pressure",
    "device_primary_lane",
    "device_fallback_lane",
    "device_memory_mode",
    "device_adapter_hints",
    "device_parallel_chunks",
    "device_kv_prefetch",
    "device_hot_kv_bits",
    "device_cold_kv_bits",
    "device_allow_disk_spill",
    "device_plan_summary",
    "device_probe_summary",
    "readiness_warnings",
    "last_inference",
    "last_inference.request_id",
    "last_inference.endpoint",
    "last_inference.elapsed_ms",
    "last_inference.runtime_model",
    "last_inference.runtime_adapter",
    "last_inference.runtime_device",
    "last_inference.runtime_primary_lane",
    "last_inference.runtime_fallback_lane",
    "last_inference.runtime_memory_mode",
    "last_inference.runtime_forward_energy",
    "last_inference.runtime_hot_kv_precision_bits",
    "last_inference.runtime_cold_kv_precision_bits",
    "last_inference.runtime_token_count",
    "last_inference.used_memory_count",
    "last_inference.stored_runtime_kv_memory_ids",
    "last_inference.route_threshold",
    "last_inference.route_attention_tokens",
    "last_inference.route_fast_tokens",
    "last_inference.route_attention_fraction",
    "last_inference.runtime_kv_influence",
    "last_inference.runtime_imported_kv_blocks",
    "last_inference.runtime_weak_kv_imports_skipped",
    "last_inference.runtime_budget_limited_kv_imports_skipped",
    "last_inference.runtime_kv_budget_pressure",
    "last_inference.runtime_exported_kv_blocks",
    "last_inference.runtime_kv_segments_included",
    "last_inference.runtime_kv_segments_skipped",
    "last_inference.runtime_kv_segments_rejected",
    "last_inference.runtime_kv_segment_yield",
    "last_inference.runtime_closed_loop_counters",
    "last_inference.runtime_closed_loop_counters.adaptive_routing_candidates",
    "last_inference.runtime_closed_loop_counters.adaptive_routing_saved_tokens",
    "last_inference.runtime_closed_loop_counters.adaptive_routing_threshold_delta_milli",
    "last_inference.runtime_closed_loop_counters.task_hierarchy_mutation_records",
    "last_inference.runtime_closed_loop_counters.task_hierarchy_compute_reduction_milli",
    "last_inference.runtime_closed_loop_counters.task_hierarchy_weight_delta_milli",
    "last_inference.runtime_closed_loop_counters.compute_budget_saved_tokens",
    "last_inference.runtime_closed_loop_counters.compute_budget_avoided_tokens",
    "last_inference.runtime_closed_loop_counters.memory_admission_ledger_authorized",
    "last_inference.runtime_closed_loop_counters.memory_admission_ledger_applied",
    "last_inference.runtime_closed_loop_counters.kv_fusion_saved_tokens",
    "last_inference.runtime_closed_loop_counters.self_evolving_memory_store_updates",
    "last_inference.runtime_closed_loop_counters.self_evolving_memory_store_primary_applied",
    "last_inference.runtime_closed_loop_counters.self_evolving_memory_store_gist_applied",
    "last_inference.runtime_closed_loop_counters.self_evolving_memory_store_runtime_kv_applied",
    "last_inference.runtime_closed_loop_counters.memory_residency_retention_decayed",
    "last_inference.runtime_closed_loop_counters.memory_residency_retention_removed",
    "last_inference.runtime_closed_loop_counters.memory_residency_compaction_merged",
    "last_inference.runtime_closed_loop_counters.memory_residency_compaction_removed",
    "last_inference.runtime_closed_loop_counters.reflection_issues",
    "last_inference.runtime_closed_loop_counters.reflection_critical_issues",
    "last_inference.runtime_closed_loop_counters.reflection_revision_actions",
    "last_inference.runtime_closed_loop_counters.online_reward_feedbacks",
    "last_inference.runtime_closed_loop_counters.online_reward_reinforcements",
    "last_inference.runtime_closed_loop_counters.online_reward_penalties",
    "last_inference.runtime_closed_loop_counters.online_reward_strength_milli",
    "last_inference.runtime_closed_loop_counters.online_reward_reinforcement_strength_milli",
    "last_inference.runtime_closed_loop_counters.online_reward_penalty_strength_milli",
    "last_inference.runtime_closed_loop_counters.memory_feedback_updates",
    "last_inference.runtime_closed_loop_counters.memory_feedback_reinforcements",
    "last_inference.runtime_closed_loop_counters.memory_feedback_penalties",
    "last_inference.runtime_closed_loop_counters.noiron_orchestration_stages",
    "last_inference.runtime_closed_loop_counters.noiron_orchestration_completed_stages",
    "last_inference.runtime_closed_loop_counters.noiron_orchestration_failed_stages",
    "last_inference.runtime_closed_loop_counters.noiron_orchestration_preview_only_stages",
    "last_inference.runtime_closed_loop_counters.noiron_orchestration_gated_stages",
    "last_inference.runtime_closed_loop_counters.noiron_orchestration_rolled_back_stages",
    "last_inference.runtime_closed_loop_counters.noiron_orchestration_rollback_records",
    "last_inference.runtime_closed_loop_counters.noiron_orchestration_writes_gated",
    "last_inference.runtime_closed_loop_counters.noiron_orchestration_live_feedback_closed",
    "last_inference.runtime_closed_loop_counters.noiron_orchestration_durable_memory_ledger_authorized",
    "last_inference.runtime_closed_loop_counters.noiron_orchestration_durable_memory_ledger_applied",
    "last_inference.runtime_closed_loop_counters.control_expression_profile_selected",
    "last_inference.runtime_closed_loop_counters.control_expression_context_anchor_promoted",
    "last_inference.runtime_closed_loop_counters.control_expression_suppression_gate_triggered",
    "last_inference.runtime_closed_loop_counters.control_expression_checkpoint_repair_requested",
    "last_inference.runtime_closed_loop_counters.control_expression_checkpoint_rejected",
    "last_inference.runtime_closed_loop_counters.control_expression_memory_refresh_candidate",
    "last_inference.runtime_closed_loop_counters.control_expression_memory_tombstone_candidate",
    "last_inference.runtime_closed_loop_counters.control_expression_preview_admission",
    "last_inference.runtime_closed_loop_counters.control_expression_write_allowed",
    "last_inference.runtime_closed_loop_counters.control_expression_applied",
    "last_inference.runtime_closed_loop_counters.control_expression_operator_approval_required",
    "last_inference.runtime_closed_loop_counters.control_expression_ready",
    "last_inference.dna_closed_loop",
    "last_inference.dna_closed_loop.strategy",
    "last_inference.dna_closed_loop.strategy_genome_id",
    "last_inference.dna_closed_loop.strategy_gene_count",
    "last_inference.dna_closed_loop.generation_before",
    "last_inference.dna_closed_loop.generation_after",
    "last_inference.dna_closed_loop.active_genome_id_after",
    "last_inference.dna_closed_loop.reasoning_frame_id",
    "last_inference.dna_closed_loop.reasoning_frame_valid",
    "last_inference.dna_closed_loop.reasoning_frame_vm_executed",
    "last_inference.dna_closed_loop.reasoning_frame_opcode_count",
    "last_inference.dna_closed_loop.task_gene_decision",
    "last_inference.dna_closed_loop.task_skill_decision",
    "last_inference.dna_closed_loop.writer_gate_decision",
    "last_inference.dna_closed_loop.apply_plan_decision",
    "last_inference.dna_closed_loop.mutation_count",
    "last_inference.dna_closed_loop.dual_chain_committed",
    "last_inference.dna_closed_loop.express_chain_records",
    "last_inference.dna_closed_loop.memory_chain_records",
    "last_inference.dna_closed_loop.mutation_applied",
    "last_inference.dna_closed_loop.rollback_applied",
    "last_inference.dna_closed_loop.receipt_reason",
    "last_inference.quality",
    "last_inference.process_reward",
    "last_inference.action",
    "last_inference.error",
    "last_inference.cancelled",
    "last_inference.timeout",
    "last_inference.retryable",
    "last_inference.runtime_error_note",
    "runtime_token_count",
    "quality",
    "process_reward",
    "action",
    "error",
];

impl EndpointInfoSpec {
    fn for_endpoint(endpoint: &str) -> Self {
        match endpoint {
            "generate" => Self {
                path: "/v1/generate",
                example: "{\"prompt\":\"用中文给一个 rust-norion 业务联调建议。\",\"profile\":\"coding\",\"case\":\"manual-generate\",\"output\":\"raw\"}",
                supported_fields: &[
                    "prompt",
                    "profile",
                    "case",
                    "output",
                    "max_tokens",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &[
                    "messages",
                    "stream",
                    "tools",
                    "tool_choice",
                    "response_format",
                ],
            },
            "chat" => Self {
                path: "/v1/chat",
                example: "{\"messages\":[{\"role\":\"user\",\"content\":\"用中文给一个 rust-norion 业务联调建议。\"}],\"profile\":\"coding\",\"case\":\"manual-chat\",\"output\":\"raw\"}",
                supported_fields: &[
                    "messages",
                    "profile",
                    "case",
                    "output",
                    "max_tokens",
                    "max_completion_tokens",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &["stream", "tools", "tool_choice", "response_format"],
            },
            "chat-completions" => Self {
                path: "/v1/chat/completions",
                example: "{\"model\":\"rust-norion-local\",\"messages\":[{\"role\":\"user\",\"content\":\"用中文给一个 rust-norion 业务联调建议。\"}],\"max_tokens\":256,\"stream\":true}",
                supported_fields: &[
                    "model",
                    "messages",
                    "max_tokens",
                    "max_completion_tokens",
                    "n",
                    "stream",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &[
                    "tools",
                    "tool_choice",
                    "response_format",
                    "logprobs",
                    "temperature",
                    "top_p",
                    "presence_penalty",
                    "frequency_penalty",
                    "stop",
                    "seed",
                    "logit_bias",
                    "stream_options",
                ],
            },
            "completions" => Self {
                path: "/v1/completions",
                example: "{\"model\":\"rust-norion-local\",\"prompt\":\"用中文给一个 rust-norion 业务联调建议。\",\"max_tokens\":256}",
                supported_fields: &[
                    "model",
                    "prompt",
                    "max_tokens",
                    "n",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &[
                    "stream",
                    "logprobs",
                    "suffix",
                    "temperature",
                    "top_p",
                    "presence_penalty",
                    "frequency_penalty",
                    "stop",
                    "seed",
                    "logit_bias",
                    "stream_options",
                ],
            },
            "chat-stream" => Self {
                path: "/v1/chat-stream",
                example: "{\"messages\":[{\"role\":\"user\",\"content\":\"用中文流式测试 SmartSteam Forge。\"}],\"profile\":\"coding\",\"case\":\"manual-chat-stream\",\"output\":\"raw\"}",
                supported_fields: &[
                    "messages",
                    "profile",
                    "case",
                    "output",
                    "max_tokens",
                    "max_completion_tokens",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &["tools", "tool_choice", "response_format"],
            },
            "generate-stream" => Self {
                path: "/v1/generate-stream",
                example: "{\"prompt\":\"用中文流式测试 rust-norion 本地模型服务。\",\"profile\":\"coding\",\"case\":\"manual-generate-stream\",\"output\":\"raw\"}",
                supported_fields: &[
                    "prompt",
                    "profile",
                    "case",
                    "output",
                    "max_tokens",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &["messages", "tools", "tool_choice", "response_format"],
            },
            "business-cycle" => Self {
                path: "/v1/business-cycle",
                example: "{\"prompt\":\"用中文完成一次 SmartSteam 业务联调自检。\",\"feedback_amount\":0.4,\"self_improve\":true,\"rust_check_code\":\"fn main() {}\"}",
                supported_fields: BUSINESS_CYCLE_SUPPORTED_FIELDS,
                unsupported_fields: BUSINESS_CYCLE_UNSUPPORTED_FIELDS,
            },
            "business-cycle-stream" => Self {
                path: "/v1/business-cycle-stream",
                example: "{\"prompt\":\"用中文流式完成一次 SmartSteam 业务联调自检。\",\"feedback_amount\":0.4,\"self_improve\":true,\"rust_check_code\":\"fn main() {}\"}",
                supported_fields: BUSINESS_CYCLE_SUPPORTED_FIELDS,
                unsupported_fields: BUSINESS_CYCLE_UNSUPPORTED_FIELDS,
            },
            "experience-hygiene-quarantine" => Self {
                path: "/v1/experience-hygiene/quarantine",
                example: "{\"apply\":false,\"limit\":20}",
                supported_fields: &["apply", "limit", "backup_path", "quarantine_path"],
                unsupported_fields: &[
                    "prompt",
                    "profile",
                    "model",
                    "messages",
                    "stream",
                    "max_tokens",
                    "tools",
                    "tool_choice",
                    "response_format",
                    "logprobs",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
            },
            "experience-cleanup-audit" => Self {
                path: "/v1/experience-cleanup-audit",
                example: "{\"limit\":20}",
                supported_fields: &["limit"],
                unsupported_fields: &[
                    "apply",
                    "backup_path",
                    "quarantine_path",
                    "prompt",
                    "profile",
                    "model",
                    "messages",
                    "stream",
                    "max_tokens",
                    "tools",
                    "tool_choice",
                    "response_format",
                    "logprobs",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
            },
            "experience-repair" => Self {
                path: "/v1/experience-repair",
                example: "{\"apply\":false,\"limit\":20}",
                supported_fields: &["apply", "limit", "backup_path"],
                unsupported_fields: &[
                    "prompt",
                    "profile",
                    "model",
                    "messages",
                    "stream",
                    "max_tokens",
                    "tools",
                    "tool_choice",
                    "response_format",
                    "logprobs",
                    "quarantine_path",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
            },
            "experience-retrieval" => Self {
                path: "/v1/experience-retrieval",
                example: "{\"prompt\":\"帮我用rust输出一段for循环代码\",\"profile\":\"coding\",\"limit\":5,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"retrieval-1\"}",
                supported_fields: &[
                    "prompt",
                    "profile",
                    "limit",
                    "index_context",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &[
                    "model",
                    "messages",
                    "stream",
                    "max_tokens",
                    "tools",
                    "tool_choice",
                    "response_format",
                    "logprobs",
                ],
            },
            "model-pool-route-plan" => Self {
                path: "/v1/model-pool/route-plan",
                example: "{\"task_kind\":\"review\",\"max_tokens\":4096,\"prompt\":\"route this Rust coding request\",\"completed_roles\":[\"quality\",\"router\"]}",
                supported_fields: &[
                    "task_kind",
                    "task",
                    "max_tokens",
                    "max",
                    "prompt",
                    "content",
                    "completed_roles",
                    "completed_stage_roles",
                ],
                unsupported_fields: &[
                    "model",
                    "messages",
                    "stream",
                    "tools",
                    "tool_choice",
                    "response_format",
                ],
            },
            "model-pool-call" => Self {
                path: "/v1/model-pool/call",
                example: "{\"task_kind\":\"summary\",\"prompt\":\"summarize this runtime trace\",\"max_tokens\":4096,\"completed_roles\":[\"quality\",\"router\"]}",
                supported_fields: &[
                    "task_kind",
                    "task",
                    "prompt",
                    "content",
                    "max_tokens",
                    "max",
                    "completed_roles",
                    "completed_stage_roles",
                ],
                unsupported_fields: &[
                    "model",
                    "messages",
                    "stream",
                    "tools",
                    "tool_choice",
                    "response_format",
                ],
            },
            "feedback" => Self {
                path: "/v1/feedback",
                example: "{\"experience_id\":7,\"action\":\"reinforce\",\"amount\":0.5,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"feedback-1\"}",
                supported_fields: &[
                    "experience_id",
                    "memory_id",
                    "action",
                    "amount",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: MODEL_SERVICE_SCOPED_FEEDBACK_UNSUPPORTED_FIELDS,
            },
            "rust-check" => Self {
                path: "/v1/rust-check",
                example: "{\"experience_id\":7,\"code\":\"pub fn ok() -> u32 { 1 }\",\"edition\":\"2021\",\"amount\":0.4,\"case\":\"compiler-feedback\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"rust-check-1\"}",
                supported_fields: &[
                    "code",
                    "edition",
                    "case",
                    "amount",
                    "experience_id",
                    "memory_id",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: MODEL_SERVICE_SCOPED_FEEDBACK_UNSUPPORTED_FIELDS,
            },
            "replay" => Self {
                path: "/v1/replay",
                example: "{\"limit\":1,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"replay-1\"}",
                supported_fields: &["limit", "tenant_id", "workspace_id", "session_id"],
                unsupported_fields: MODEL_SERVICE_EVOLUTION_UNSUPPORTED_FIELDS,
            },
            "self-improve" => Self {
                path: "/v1/self-improve",
                example: "{\"limit\":1,\"gate\":\"gemma_model_service_smoke\",\"trace_gate\":true,\"require_deep_self_evolution\":true,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"self-improve-1\"}",
                supported_fields: &[
                    "limit",
                    "gate",
                    "state_gate",
                    "business_gate",
                    "business_cycle_gate",
                    "model_service_gate",
                    "trace_gate",
                    "require_deep_self_evolution",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: MODEL_SERVICE_EVOLUTION_UNSUPPORTED_FIELDS,
            },
            "state" => Self {
                path: "/v1/state",
                example: "{}",
                supported_fields: &[],
                unsupported_fields: MODEL_SERVICE_EVOLUTION_UNSUPPORTED_FIELDS,
            },
            "inspect" => Self {
                path: "/v1/inspect",
                example: "{\"gate\":\"gemma_model_service_smoke\",\"trace_gate\":true,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"inspect-1\"}",
                supported_fields: &[
                    "gate",
                    "state_gate",
                    "business_gate",
                    "business_cycle_gate",
                    "model_service_gate",
                    "trace_gate",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: MODEL_SERVICE_EVOLUTION_UNSUPPORTED_FIELDS,
            },
            "requests-cancel" => Self {
                path: "/v1/requests/cancel",
                example: "{\"request_id\":42,\"reason\":\"operator_runtime_splice\",\"retag_label\":\"repair_factor:runtime_splice\"}",
                supported_fields: &["request_id", "reason", "retag_label"],
                unsupported_fields: &[
                    "prompt",
                    "messages",
                    "stream",
                    "tools",
                    "tool_choice",
                    "response_format",
                ],
            },
            _ => Self {
                path: "/v1/generate",
                example: "{\"prompt\":\"用中文给一个 rust-norion 业务联调建议。\",\"profile\":\"coding\",\"case\":\"manual-generate\",\"output\":\"raw\"}",
                supported_fields: &[
                    "prompt",
                    "profile",
                    "case",
                    "output",
                    "max_tokens",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &[
                    "messages",
                    "stream",
                    "tools",
                    "tool_choice",
                    "response_format",
                ],
            },
        }
    }
}

const MODEL_SERVICE_EVOLUTION_UNSUPPORTED_FIELDS: &[&str] = &[
    "prompt",
    "profile",
    "model",
    "messages",
    "stream",
    "max_tokens",
    "tools",
    "tool_choice",
    "response_format",
    "logprobs",
    "tenant_id",
    "workspace_id",
    "session_id",
];

const MODEL_SERVICE_SCOPED_FEEDBACK_UNSUPPORTED_FIELDS: &[&str] = &[
    "prompt",
    "profile",
    "model",
    "messages",
    "stream",
    "max_tokens",
    "tools",
    "tool_choice",
    "response_format",
    "logprobs",
];

const OPENAI_RESPONSE_FIELDS: &[&str] = &[
    "ok",
    "id",
    "object",
    "created",
    "model",
    "choices",
    "usage",
    "error",
    "error.message",
    "error.type",
    "error.param",
    "error.code",
    "norion",
    "norion.request_id",
    "norion.endpoint",
    "norion.model",
    "norion.profile",
    "norion.language_mode",
    "norion.coding_language",
    "norion.rust_coding",
    "norion.task_mode",
    "norion.task_language",
    "norion.coding_intent",
    "norion.validation_mode",
    "norion.memory_need",
    "norion.compute_budget",
    "norion.compute_budget_summary",
    "norion.compute_budget_saved_tokens",
    "norion.compute_budget_avoided_tokens",
    "norion.compute_budget_kv_lookups_skipped",
    "norion.compute_budget_fanout_reduction",
    "norion.compute_budget_read_only",
    "norion.compute_budget_write_allowed",
    "norion.compute_budget_applied",
    "norion.route_threshold",
    "norion.route_attention_tokens",
    "norion.route_fast_tokens",
    "norion.route_attention_fraction",
    "norion.cancelled",
    "norion.timeout",
    "norion.retryable",
    "norion.runtime_error_note",
    "norion.elapsed_ms",
    "norion.output_mode",
    "norion.quality",
    "norion.experience_id",
    "norion.memory_stored",
    "norion.used_memory_count",
    "norion.stored_runtime_kv_memory_ids",
    "norion.runtime_model",
    "norion.runtime_adapter",
    "norion.runtime_device",
    "norion.runtime_primary_lane",
    "norion.runtime_fallback_lane",
    "norion.runtime_memory_mode",
    "norion.runtime_forward_energy",
    "norion.runtime_hot_kv_precision_bits",
    "norion.runtime_cold_kv_precision_bits",
    "norion.runtime_token_count",
    "norion.runtime_entropy_count",
    "norion.runtime_logprob_count",
    "norion.runtime_uncertainty_token_count",
    "norion.runtime_uncertainty_signal",
    "norion.runtime_average_entropy",
    "norion.runtime_average_neg_logprob",
    "norion.runtime_uncertainty_perplexity",
    "norion.runtime_architecture_signal",
    "norion.runtime_kv_precision_signal",
    "norion.runtime_device_execution_source",
    "norion.runtime_kv_influence",
    "norion.runtime_imported_kv_blocks",
    "norion.runtime_weak_kv_imports_skipped",
    "norion.runtime_budget_limited_kv_imports_skipped",
    "norion.runtime_kv_budget_pressure",
    "norion.runtime_exported_kv_blocks",
    "norion.runtime_kv_segments_included",
    "norion.runtime_kv_segments_skipped",
    "norion.runtime_kv_segments_rejected",
    "norion.runtime_kv_segment_yield",
    "norion.runtime_closed_loop_counters",
    "norion.runtime_closed_loop_counters.adaptive_routing_candidates",
    "norion.runtime_closed_loop_counters.adaptive_routing_saved_tokens",
    "norion.runtime_closed_loop_counters.adaptive_routing_threshold_delta_milli",
    "norion.runtime_closed_loop_counters.task_hierarchy_mutation_records",
    "norion.runtime_closed_loop_counters.task_hierarchy_compute_reduction_milli",
    "norion.runtime_closed_loop_counters.task_hierarchy_weight_delta_milli",
    "norion.runtime_closed_loop_counters.compute_budget_selected_candidates",
    "norion.runtime_closed_loop_counters.compute_budget_kv_lookups_skipped",
    "norion.runtime_closed_loop_counters.compute_budget_saved_tokens",
    "norion.runtime_closed_loop_counters.compute_budget_avoided_tokens",
    "norion.runtime_closed_loop_counters.compute_budget_write_allowed",
    "norion.runtime_closed_loop_counters.compute_budget_applied",
    "norion.runtime_closed_loop_counters.memory_admission_candidates",
    "norion.runtime_closed_loop_counters.memory_admission_ready",
    "norion.runtime_closed_loop_counters.memory_admission_blocked",
    "norion.runtime_closed_loop_counters.memory_admission_ledger_records",
    "norion.runtime_closed_loop_counters.memory_admission_ledger_preview_only",
    "norion.runtime_closed_loop_counters.memory_admission_ledger_authorized",
    "norion.runtime_closed_loop_counters.memory_admission_ledger_applied",
    "norion.runtime_closed_loop_counters.memory_admission_write_allowed",
    "norion.runtime_closed_loop_counters.memory_admission_applied",
    "norion.runtime_closed_loop_counters.kv_fusion_candidates",
    "norion.runtime_closed_loop_counters.kv_fusion_fused",
    "norion.runtime_closed_loop_counters.kv_fusion_compressed",
    "norion.runtime_closed_loop_counters.kv_fusion_skipped",
    "norion.runtime_closed_loop_counters.kv_fusion_held",
    "norion.runtime_closed_loop_counters.kv_fusion_rejected",
    "norion.runtime_closed_loop_counters.kv_fusion_approval_blocked",
    "norion.runtime_closed_loop_counters.kv_fusion_input_tokens",
    "norion.runtime_closed_loop_counters.kv_fusion_retained_tokens",
    "norion.runtime_closed_loop_counters.kv_fusion_saved_tokens",
    "norion.runtime_closed_loop_counters.kv_fusion_write_allowed",
    "norion.runtime_closed_loop_counters.kv_fusion_applied",
    "norion.runtime_closed_loop_counters.self_evolving_memory_store_updates",
    "norion.runtime_closed_loop_counters.self_evolving_memory_store_primary_applied",
    "norion.runtime_closed_loop_counters.self_evolving_memory_store_gist_applied",
    "norion.runtime_closed_loop_counters.self_evolving_memory_store_runtime_kv_applied",
    "norion.runtime_closed_loop_counters.memory_residency_retention_decayed",
    "norion.runtime_closed_loop_counters.memory_residency_retention_removed",
    "norion.runtime_closed_loop_counters.memory_residency_compaction_merged",
    "norion.runtime_closed_loop_counters.memory_residency_compaction_removed",
    "norion.runtime_closed_loop_counters.reflection_issues",
    "norion.runtime_closed_loop_counters.reflection_critical_issues",
    "norion.runtime_closed_loop_counters.reflection_revision_actions",
    "norion.runtime_closed_loop_counters.online_reward_feedbacks",
    "norion.runtime_closed_loop_counters.online_reward_reinforcements",
    "norion.runtime_closed_loop_counters.online_reward_penalties",
    "norion.runtime_closed_loop_counters.online_reward_strength_milli",
    "norion.runtime_closed_loop_counters.online_reward_reinforcement_strength_milli",
    "norion.runtime_closed_loop_counters.online_reward_penalty_strength_milli",
    "norion.runtime_closed_loop_counters.memory_feedback_updates",
    "norion.runtime_closed_loop_counters.memory_feedback_reinforcements",
    "norion.runtime_closed_loop_counters.memory_feedback_penalties",
    "norion.runtime_closed_loop_counters.noiron_orchestration_stages",
    "norion.runtime_closed_loop_counters.noiron_orchestration_completed_stages",
    "norion.runtime_closed_loop_counters.noiron_orchestration_failed_stages",
    "norion.runtime_closed_loop_counters.noiron_orchestration_preview_only_stages",
    "norion.runtime_closed_loop_counters.noiron_orchestration_gated_stages",
    "norion.runtime_closed_loop_counters.noiron_orchestration_rolled_back_stages",
    "norion.runtime_closed_loop_counters.noiron_orchestration_rollback_records",
    "norion.runtime_closed_loop_counters.noiron_orchestration_writes_gated",
    "norion.runtime_closed_loop_counters.noiron_orchestration_live_feedback_closed",
    "norion.runtime_closed_loop_counters.noiron_orchestration_durable_memory_ledger_authorized",
    "norion.runtime_closed_loop_counters.noiron_orchestration_durable_memory_ledger_applied",
    "norion.runtime_closed_loop_counters.control_expression_profile_selected",
    "norion.runtime_closed_loop_counters.control_expression_context_anchor_promoted",
    "norion.runtime_closed_loop_counters.control_expression_suppression_gate_triggered",
    "norion.runtime_closed_loop_counters.control_expression_checkpoint_repair_requested",
    "norion.runtime_closed_loop_counters.control_expression_checkpoint_rejected",
    "norion.runtime_closed_loop_counters.control_expression_memory_refresh_candidate",
    "norion.runtime_closed_loop_counters.control_expression_memory_tombstone_candidate",
    "norion.runtime_closed_loop_counters.control_expression_preview_admission",
    "norion.runtime_closed_loop_counters.control_expression_write_allowed",
    "norion.runtime_closed_loop_counters.control_expression_applied",
    "norion.runtime_closed_loop_counters.control_expression_operator_approval_required",
    "norion.runtime_closed_loop_counters.control_expression_ready",
    "norion.dna_closed_loop",
    "norion.dna_closed_loop.strategy",
    "norion.dna_closed_loop.strategy_genome_id",
    "norion.dna_closed_loop.strategy_gene_count",
    "norion.dna_closed_loop.generation_before",
    "norion.dna_closed_loop.generation_after",
    "norion.dna_closed_loop.active_genome_id_after",
    "norion.dna_closed_loop.reasoning_frame_id",
    "norion.dna_closed_loop.reasoning_frame_valid",
    "norion.dna_closed_loop.reasoning_frame_vm_executed",
    "norion.dna_closed_loop.reasoning_frame_opcode_count",
    "norion.dna_closed_loop.task_gene_decision",
    "norion.dna_closed_loop.task_skill_decision",
    "norion.dna_closed_loop.writer_gate_decision",
    "norion.dna_closed_loop.apply_plan_decision",
    "norion.dna_closed_loop.mutation_count",
    "norion.dna_closed_loop.dual_chain_committed",
    "norion.dna_closed_loop.express_chain_records",
    "norion.dna_closed_loop.memory_chain_records",
    "norion.dna_closed_loop.mutation_applied",
    "norion.dna_closed_loop.rollback_applied",
    "norion.dna_closed_loop.receipt_reason",
    "norion.persistent_writes",
    "norion.memory_write_allowed",
    "norion.genome_write_allowed",
    "norion.self_evolution_write_allowed",
];

const MODEL_SERVICE_STREAM_RESPONSE_FIELDS: &[&str] = &[
    "event:status",
    "event:delta",
    "event:final",
    "event:final.ok",
    "event:final.request_id",
    "event:final.profile",
    "event:final.language_mode",
    "event:final.coding_language",
    "event:final.rust_coding",
    "event:final.task_mode",
    "event:final.task_language",
    "event:final.coding_intent",
    "event:final.validation_mode",
    "event:final.memory_need",
    "event:final.compute_budget",
    "event:final.elapsed_ms",
    "event:final.output_mode",
    "event:final.quality",
    "event:final.process_reward",
    "event:final.action",
    "event:final.memory_stored",
    "event:final.stored_memory_id",
    "event:final.used_memory_ids",
    "event:final.stored_gist_memory_ids",
    "event:final.stored_runtime_kv_memory_ids",
    "event:final.feedback_memory_ids",
    "event:final.experience_id",
    "event:final.runtime_model",
    "event:final.runtime_adapter",
    "event:final.dna_closed_loop",
    "event:final.dna_closed_loop.strategy",
    "event:final.dna_closed_loop.strategy_genome_id",
    "event:final.dna_closed_loop.strategy_gene_count",
    "event:final.dna_closed_loop.generation_before",
    "event:final.dna_closed_loop.generation_after",
    "event:final.dna_closed_loop.active_genome_id_after",
    "event:final.dna_closed_loop.reasoning_frame_vm_executed",
    "event:final.dna_closed_loop.reasoning_frame_opcode_count",
    "event:final.dna_closed_loop.writer_gate_decision",
    "event:final.dna_closed_loop.apply_plan_decision",
    "event:final.dna_closed_loop.dual_chain_committed",
    "event:final.dna_closed_loop.express_chain_records",
    "event:final.dna_closed_loop.memory_chain_records",
    "event:final.dna_closed_loop.mutation_applied",
    "event:final.dna_closed_loop.rollback_applied",
    "event:final.runtime_device",
    "event:final.runtime_primary_lane",
    "event:final.runtime_fallback_lane",
    "event:final.runtime_memory_mode",
    "event:final.runtime_forward_energy",
    "event:final.runtime_hot_kv_precision_bits",
    "event:final.runtime_cold_kv_precision_bits",
    "event:final.runtime_token_count",
    "event:final.runtime_entropy_count",
    "event:final.runtime_logprob_count",
    "event:final.runtime_uncertainty_token_count",
    "event:final.runtime_uncertainty_signal",
    "event:final.runtime_average_entropy",
    "event:final.runtime_average_neg_logprob",
    "event:final.runtime_uncertainty_perplexity",
    "event:final.runtime_architecture_signal",
    "event:final.runtime_kv_precision_signal",
    "event:final.runtime_device_execution_source",
    "event:final.runtime_kv_influence",
    "event:final.runtime_imported_kv_blocks",
    "event:final.runtime_weak_kv_imports_skipped",
    "event:final.runtime_budget_limited_kv_imports_skipped",
    "event:final.runtime_kv_budget_pressure",
    "event:final.runtime_exported_kv_blocks",
    "event:final.runtime_kv_segments_included",
    "event:final.runtime_kv_segments_skipped",
    "event:final.runtime_kv_segments_rejected",
    "event:final.runtime_kv_segment_yield",
    "event:final.runtime_closed_loop_counters",
    "event:final.runtime_closed_loop_counters.adaptive_routing_candidates",
    "event:final.runtime_closed_loop_counters.adaptive_routing_saved_tokens",
    "event:final.runtime_closed_loop_counters.adaptive_routing_threshold_delta_milli",
    "event:final.runtime_closed_loop_counters.task_hierarchy_mutation_records",
    "event:final.runtime_closed_loop_counters.task_hierarchy_compute_reduction_milli",
    "event:final.runtime_closed_loop_counters.task_hierarchy_weight_delta_milli",
    "event:final.runtime_closed_loop_counters.compute_budget_selected_candidates",
    "event:final.runtime_closed_loop_counters.compute_budget_kv_lookups_skipped",
    "event:final.runtime_closed_loop_counters.compute_budget_saved_tokens",
    "event:final.runtime_closed_loop_counters.compute_budget_avoided_tokens",
    "event:final.runtime_closed_loop_counters.compute_budget_write_allowed",
    "event:final.runtime_closed_loop_counters.compute_budget_applied",
    "event:final.runtime_closed_loop_counters.memory_admission_candidates",
    "event:final.runtime_closed_loop_counters.memory_admission_ready",
    "event:final.runtime_closed_loop_counters.memory_admission_blocked",
    "event:final.runtime_closed_loop_counters.memory_admission_ledger_records",
    "event:final.runtime_closed_loop_counters.memory_admission_ledger_preview_only",
    "event:final.runtime_closed_loop_counters.memory_admission_ledger_authorized",
    "event:final.runtime_closed_loop_counters.memory_admission_ledger_applied",
    "event:final.runtime_closed_loop_counters.memory_admission_write_allowed",
    "event:final.runtime_closed_loop_counters.memory_admission_applied",
    "event:final.runtime_closed_loop_counters.kv_fusion_candidates",
    "event:final.runtime_closed_loop_counters.kv_fusion_fused",
    "event:final.runtime_closed_loop_counters.kv_fusion_compressed",
    "event:final.runtime_closed_loop_counters.kv_fusion_skipped",
    "event:final.runtime_closed_loop_counters.kv_fusion_held",
    "event:final.runtime_closed_loop_counters.kv_fusion_rejected",
    "event:final.runtime_closed_loop_counters.kv_fusion_approval_blocked",
    "event:final.runtime_closed_loop_counters.kv_fusion_input_tokens",
    "event:final.runtime_closed_loop_counters.kv_fusion_retained_tokens",
    "event:final.runtime_closed_loop_counters.kv_fusion_saved_tokens",
    "event:final.runtime_closed_loop_counters.kv_fusion_write_allowed",
    "event:final.runtime_closed_loop_counters.kv_fusion_applied",
    "event:final.runtime_closed_loop_counters.self_evolving_memory_store_updates",
    "event:final.runtime_closed_loop_counters.self_evolving_memory_store_primary_applied",
    "event:final.runtime_closed_loop_counters.self_evolving_memory_store_gist_applied",
    "event:final.runtime_closed_loop_counters.self_evolving_memory_store_runtime_kv_applied",
    "event:final.runtime_closed_loop_counters.memory_residency_retention_decayed",
    "event:final.runtime_closed_loop_counters.memory_residency_retention_removed",
    "event:final.runtime_closed_loop_counters.memory_residency_compaction_merged",
    "event:final.runtime_closed_loop_counters.memory_residency_compaction_removed",
    "event:final.runtime_closed_loop_counters.reflection_issues",
    "event:final.runtime_closed_loop_counters.reflection_critical_issues",
    "event:final.runtime_closed_loop_counters.reflection_revision_actions",
    "event:final.runtime_closed_loop_counters.online_reward_feedbacks",
    "event:final.runtime_closed_loop_counters.online_reward_reinforcements",
    "event:final.runtime_closed_loop_counters.online_reward_penalties",
    "event:final.runtime_closed_loop_counters.online_reward_strength_milli",
    "event:final.runtime_closed_loop_counters.online_reward_reinforcement_strength_milli",
    "event:final.runtime_closed_loop_counters.online_reward_penalty_strength_milli",
    "event:final.runtime_closed_loop_counters.memory_feedback_updates",
    "event:final.runtime_closed_loop_counters.memory_feedback_reinforcements",
    "event:final.runtime_closed_loop_counters.memory_feedback_penalties",
    "event:final.runtime_closed_loop_counters.noiron_orchestration_stages",
    "event:final.runtime_closed_loop_counters.noiron_orchestration_completed_stages",
    "event:final.runtime_closed_loop_counters.noiron_orchestration_failed_stages",
    "event:final.runtime_closed_loop_counters.noiron_orchestration_preview_only_stages",
    "event:final.runtime_closed_loop_counters.noiron_orchestration_gated_stages",
    "event:final.runtime_closed_loop_counters.noiron_orchestration_rolled_back_stages",
    "event:final.runtime_closed_loop_counters.noiron_orchestration_rollback_records",
    "event:final.runtime_closed_loop_counters.noiron_orchestration_writes_gated",
    "event:final.runtime_closed_loop_counters.noiron_orchestration_durable_memory_ledger_authorized",
    "event:final.runtime_closed_loop_counters.noiron_orchestration_durable_memory_ledger_applied",
    "event:final.traceable",
    "event:final.endpoint",
    "event:final.stream_state",
    "event:final.cancelled",
    "event:final.timeout",
    "event:final.retryable",
    "event:final.runtime_error_note",
    "event:final.partial_result",
    "event:final.partial_finalized",
    "event:final.streamed_tokens",
    "event:final.queue_time_ms",
    "event:final.cancellation_reason",
    "event:final.compute_budget_summary",
    "event:final.compute_budget_saved_tokens",
    "event:final.compute_budget_avoided_tokens",
    "event:final.compute_budget_kv_lookups_skipped",
    "event:final.compute_budget_fanout_reduction",
    "event:final.compute_budget_read_only",
    "event:final.compute_budget_write_allowed",
    "event:final.compute_budget_applied",
    "event:final.route_threshold",
    "event:final.route_attention_tokens",
    "event:final.route_fast_tokens",
    "event:final.route_attention_fraction",
    "event:final.persistent_writes",
    "event:final.memory_write_allowed",
    "event:final.genome_write_allowed",
    "event:final.self_evolution_write_allowed",
    "event:final.error",
    "event:done",
    "event:error",
];

const OPENAI_CHAT_STREAM_RESPONSE_FIELDS: &[&str] = &[
    "data:chunk",
    "data:[DONE]",
    "object:chat.completion.chunk",
    "choices.delta",
    "choices.finish_reason",
    "error",
    "error.message",
    "error.type",
    "norion.request_id",
    "norion.endpoint",
    "norion.model",
    "norion.profile",
    "norion.language_mode",
    "norion.coding_language",
    "norion.rust_coding",
    "norion.task_mode",
    "norion.task_language",
    "norion.coding_intent",
    "norion.validation_mode",
    "norion.memory_need",
    "norion.compute_budget",
    "norion.compute_budget_summary",
    "norion.compute_budget_saved_tokens",
    "norion.compute_budget_avoided_tokens",
    "norion.compute_budget_kv_lookups_skipped",
    "norion.compute_budget_fanout_reduction",
    "norion.compute_budget_read_only",
    "norion.compute_budget_write_allowed",
    "norion.compute_budget_applied",
    "norion.stream_state",
    "norion.streamed_tokens",
    "norion.elapsed_ms",
    "norion.runtime_model",
    "norion.runtime_adapter",
    "norion.runtime_device",
    "norion.runtime_primary_lane",
    "norion.runtime_fallback_lane",
    "norion.runtime_memory_mode",
    "norion.runtime_forward_energy",
    "norion.runtime_hot_kv_precision_bits",
    "norion.runtime_cold_kv_precision_bits",
    "norion.runtime_token_count",
    "norion.runtime_entropy_count",
    "norion.runtime_logprob_count",
    "norion.runtime_uncertainty_token_count",
    "norion.runtime_uncertainty_signal",
    "norion.runtime_average_entropy",
    "norion.runtime_average_neg_logprob",
    "norion.runtime_uncertainty_perplexity",
    "norion.runtime_architecture_signal",
    "norion.runtime_kv_precision_signal",
    "norion.runtime_device_execution_source",
    "norion.runtime_kv_influence",
    "norion.runtime_imported_kv_blocks",
    "norion.runtime_weak_kv_imports_skipped",
    "norion.runtime_budget_limited_kv_imports_skipped",
    "norion.runtime_kv_budget_pressure",
    "norion.runtime_exported_kv_blocks",
    "norion.runtime_kv_segments_included",
    "norion.runtime_kv_segments_skipped",
    "norion.runtime_kv_segments_rejected",
    "norion.runtime_kv_segment_yield",
    "norion.runtime_closed_loop_counters",
    "norion.runtime_closed_loop_counters.adaptive_routing_candidates",
    "norion.runtime_closed_loop_counters.adaptive_routing_saved_tokens",
    "norion.runtime_closed_loop_counters.adaptive_routing_threshold_delta_milli",
    "norion.runtime_closed_loop_counters.task_hierarchy_mutation_records",
    "norion.runtime_closed_loop_counters.task_hierarchy_compute_reduction_milli",
    "norion.runtime_closed_loop_counters.task_hierarchy_weight_delta_milli",
    "norion.runtime_closed_loop_counters.compute_budget_selected_candidates",
    "norion.runtime_closed_loop_counters.compute_budget_kv_lookups_skipped",
    "norion.runtime_closed_loop_counters.compute_budget_saved_tokens",
    "norion.runtime_closed_loop_counters.compute_budget_avoided_tokens",
    "norion.runtime_closed_loop_counters.compute_budget_write_allowed",
    "norion.runtime_closed_loop_counters.compute_budget_applied",
    "norion.runtime_closed_loop_counters.memory_admission_candidates",
    "norion.runtime_closed_loop_counters.memory_admission_ready",
    "norion.runtime_closed_loop_counters.memory_admission_blocked",
    "norion.runtime_closed_loop_counters.memory_admission_ledger_records",
    "norion.runtime_closed_loop_counters.memory_admission_ledger_preview_only",
    "norion.runtime_closed_loop_counters.memory_admission_ledger_authorized",
    "norion.runtime_closed_loop_counters.memory_admission_ledger_applied",
    "norion.runtime_closed_loop_counters.memory_admission_write_allowed",
    "norion.runtime_closed_loop_counters.memory_admission_applied",
    "norion.runtime_closed_loop_counters.kv_fusion_candidates",
    "norion.runtime_closed_loop_counters.kv_fusion_fused",
    "norion.runtime_closed_loop_counters.kv_fusion_compressed",
    "norion.runtime_closed_loop_counters.kv_fusion_skipped",
    "norion.runtime_closed_loop_counters.kv_fusion_held",
    "norion.runtime_closed_loop_counters.kv_fusion_rejected",
    "norion.runtime_closed_loop_counters.kv_fusion_approval_blocked",
    "norion.runtime_closed_loop_counters.kv_fusion_input_tokens",
    "norion.runtime_closed_loop_counters.kv_fusion_retained_tokens",
    "norion.runtime_closed_loop_counters.kv_fusion_saved_tokens",
    "norion.runtime_closed_loop_counters.kv_fusion_write_allowed",
    "norion.runtime_closed_loop_counters.kv_fusion_applied",
    "norion.runtime_closed_loop_counters.self_evolving_memory_store_updates",
    "norion.runtime_closed_loop_counters.self_evolving_memory_store_primary_applied",
    "norion.runtime_closed_loop_counters.self_evolving_memory_store_gist_applied",
    "norion.runtime_closed_loop_counters.self_evolving_memory_store_runtime_kv_applied",
    "norion.runtime_closed_loop_counters.memory_residency_retention_decayed",
    "norion.runtime_closed_loop_counters.memory_residency_retention_removed",
    "norion.runtime_closed_loop_counters.memory_residency_compaction_merged",
    "norion.runtime_closed_loop_counters.memory_residency_compaction_removed",
    "norion.runtime_closed_loop_counters.reflection_issues",
    "norion.runtime_closed_loop_counters.reflection_critical_issues",
    "norion.runtime_closed_loop_counters.reflection_revision_actions",
    "norion.runtime_closed_loop_counters.online_reward_feedbacks",
    "norion.runtime_closed_loop_counters.online_reward_reinforcements",
    "norion.runtime_closed_loop_counters.online_reward_penalties",
    "norion.runtime_closed_loop_counters.online_reward_strength_milli",
    "norion.runtime_closed_loop_counters.online_reward_reinforcement_strength_milli",
    "norion.runtime_closed_loop_counters.online_reward_penalty_strength_milli",
    "norion.runtime_closed_loop_counters.memory_feedback_updates",
    "norion.runtime_closed_loop_counters.memory_feedback_reinforcements",
    "norion.runtime_closed_loop_counters.memory_feedback_penalties",
    "norion.runtime_closed_loop_counters.noiron_orchestration_stages",
    "norion.runtime_closed_loop_counters.noiron_orchestration_completed_stages",
    "norion.runtime_closed_loop_counters.noiron_orchestration_failed_stages",
    "norion.runtime_closed_loop_counters.noiron_orchestration_preview_only_stages",
    "norion.runtime_closed_loop_counters.noiron_orchestration_gated_stages",
    "norion.runtime_closed_loop_counters.noiron_orchestration_rolled_back_stages",
    "norion.runtime_closed_loop_counters.noiron_orchestration_rollback_records",
    "norion.runtime_closed_loop_counters.noiron_orchestration_writes_gated",
    "norion.runtime_closed_loop_counters.noiron_orchestration_durable_memory_ledger_authorized",
    "norion.runtime_closed_loop_counters.noiron_orchestration_durable_memory_ledger_applied",
    "norion.used_memory_count",
    "norion.stored_runtime_kv_memory_ids",
    "norion.route_threshold",
    "norion.route_attention_tokens",
    "norion.route_fast_tokens",
    "norion.route_attention_fraction",
    "norion.cancelled",
    "norion.timeout",
    "norion.retryable",
    "norion.runtime_error_note",
    "norion.persistent_writes",
    "norion.memory_write_allowed",
    "norion.genome_write_allowed",
    "norion.self_evolution_write_allowed",
];

const MODEL_SERVICE_FEEDBACK_RESPONSE_FIELDS: &[&str] = &[
    "ok",
    "request_id",
    "feedback",
    "feedback.action",
    "feedback.amount",
    "feedback.experience_id",
    "feedback.memory_id",
    "feedback.memory_ids",
    "feedback.applied",
    "feedback.missing",
    "feedback.removed",
    "feedback.strength_delta",
    "feedback.updates",
    "feedback.updates.id",
    "feedback.updates.action",
    "feedback.updates.requested_amount",
    "feedback.updates.applied",
    "feedback.updates.removed",
    "feedback.updates.strength_before",
    "feedback.updates.strength_after",
    "feedback.updates.strength_delta",
    "state",
    "state.evolution_external_feedbacks",
    "state.evolution_external_feedback_memory_updates",
    "state.evolution_external_feedback_strength_delta",
];

const MODEL_SERVICE_RUST_CHECK_RESPONSE_FIELDS: &[&str] = &[
    "ok",
    "request_id",
    "rust_check",
    "rust_check.passed",
    "rust_check.label",
    "rust_check.edition",
    "rust_check.status_code",
    "rust_check.diagnostic_chars",
    "rust_check.stdout",
    "rust_check.stderr",
    "rust_check.source_path",
    "rust_check.metadata_path",
    "feedback",
    "feedback.action",
    "feedback.amount",
    "feedback.experience_id",
    "feedback.memory_id",
    "feedback.memory_ids",
    "feedback.applied",
    "feedback.missing",
    "feedback.removed",
    "feedback.strength_delta",
    "feedback.updates",
    "feedback.updates.id",
    "feedback.updates.action",
    "feedback.updates.requested_amount",
    "feedback.updates.applied",
    "feedback.updates.removed",
    "feedback.updates.strength_before",
    "feedback.updates.strength_after",
    "feedback.updates.strength_delta",
    "state",
    "state.rust_check_passed",
    "state.rust_check_failed",
    "state.evolution_external_feedbacks",
];

const MODEL_SERVICE_REPLAY_RESPONSE_FIELDS: &[&str] = &[
    "ok",
    "request_id",
    "limit",
    "replay",
    "replay.summary",
    "replay.planned",
    "replay.applied",
    "replay.router_updates",
    "replay.hierarchy_updates",
    "replay.memory_updates",
    "replay.runtime_kv_budget_pressure_items",
    "replay.avg_runtime_kv_budget_pressure",
    "replay.max_runtime_kv_budget_pressure",
    "replay.runtime_kv_weak_import_pressure_items",
    "replay.avg_runtime_kv_weak_import_pressure",
    "replay.max_runtime_kv_weak_import_pressure",
    "replay.recursive_runtime_items",
    "replay.recursive_runtime_calls",
    "replay.avg_recursive_call_pressure",
    "replay.max_recursive_call_pressure",
    "replay.live_memory_feedback_items",
    "replay.live_memory_feedback_updates",
    "replay.live_memory_feedback_reinforcements",
    "replay.live_memory_feedback_penalties",
    "replay.live_memory_feedback_detail_items",
    "replay.live_memory_feedback_applied",
    "replay.live_memory_feedback_removed",
    "replay.live_memory_feedback_missing",
    "replay.live_memory_feedback_strength_delta",
    "replay.rust_check_items",
    "replay.rust_check_passed",
    "replay.rust_check_failed",
    "replay.rust_check_diagnostic_chars",
    "replay.rust_check_live_memory_feedback_items",
    "replay.rust_check_live_memory_feedback_updates",
    "replay.rust_check_live_memory_feedback_applied",
    "replay.rust_check_live_memory_feedback_missing",
    "replay.rust_check_live_memory_feedback_strength_delta",
    "replay.business_contract_items",
    "replay.business_contract_passed",
    "replay.business_contract_failed",
    "replay.business_contract_raw_passed",
    "replay.business_contract_raw_failed",
    "replay.business_contract_response_normalized",
    "replay.business_contract_sanitized",
    "replay.business_contract_canonical_fallbacks",
    "replay.pool_dispatch_items",
    "replay.pool_dispatch_forwarded",
    "replay.pool_dispatch_clamped",
    "replay.pool_dispatch_low_priority",
    "replay.live_evolution_items",
    "replay.live_evolution_router_threshold_mutations",
    "replay.live_evolution_hierarchy_weight_mutations",
    "replay.live_evolution_router_threshold_delta",
    "replay.live_evolution_hierarchy_weight_delta",
    "replay.live_evolution_online_reward_feedbacks",
    "replay.live_evolution_online_reward_reinforcements",
    "replay.live_evolution_online_reward_penalties",
    "replay.live_evolution_online_reward_strength",
    "replay.live_evolution_online_reward_reinforcement_strength",
    "replay.live_evolution_online_reward_penalty_strength",
    "replay.live_evolution_memory_updates",
    "replay.live_evolution_stored_memory_updates",
    "replay.live_evolution_reflection_issues",
    "replay.live_evolution_critical_reflection_issues",
    "replay.live_evolution_revision_actions",
    "state",
    "state.evolution_replay_runs",
    "state.evolution_replay_items",
    "state.evolution_recursive_runtime_calls",
];

const MODEL_SERVICE_SELF_IMPROVE_RESPONSE_FIELDS: &[&str] = &[
    "ok",
    "request_id",
    "limit",
    "self_improve",
    "self_improve.passed",
    "self_improve.replay_passed",
    "self_improve.replay_planned",
    "self_improve.replay_applied",
    "self_improve.state_gate_checked",
    "self_improve.state_gate_passed",
    "self_improve.trace_gate_checked",
    "self_improve.trace_gate_passed",
    "self_improve.state_gate",
    "self_improve.business_gate",
    "self_improve.business_cycle_gate",
    "self_improve.model_service_gate",
    "self_improve.require_deep_self_evolution",
    "self_improve.deep_self_evolution_checked",
    "self_improve.deep_self_evolution_passed",
    "self_improve.depth_status",
    "self_improve.reflection_issue_experiences",
    "self_improve.critical_reflection_issue_experiences",
    "self_improve.revision_action_experiences",
    "self_improve.live_memory_feedback_updates",
    "self_improve.live_memory_feedback_applied",
    "self_improve.depth_failures",
    "self_improve.self_evolution_admission_checked",
    "self_improve.self_evolution_admission_admitted_for_human_review",
    "self_improve.self_evolution_admission_human_approval_required",
    "self_improve.self_evolution_admission_blocked",
    "self_improve.self_evolution_admission_blocked_reasons",
    "self_improve.self_evolution_admission_trace_events",
    "self_improve.self_evolution_admission_trace_admitted",
    "self_improve.self_evolution_admission_trace_blocked",
    "replay",
    "state",
    "state_gate",
    "trace_gate",
    "trace_gate.runtime_closed_loop_counters",
    "trace_gate.runtime_closed_loop_counters.compute_budget_saved_tokens",
    "trace_gate.runtime_closed_loop_counters.compute_budget_avoided_tokens",
    "trace_gate.runtime_closed_loop_counters.compute_budget_write_allowed",
    "trace_gate.runtime_closed_loop_counters.compute_budget_applied",
    "trace_gate.runtime_closed_loop_counters.memory_admission_ledger_authorized",
    "trace_gate.runtime_closed_loop_counters.memory_admission_ledger_applied",
    "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_write_allowed",
    "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_durable_write_allowed",
    "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_applied",
    "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_applied_to_disk",
    "trace_gate.runtime_closed_loop_counters.memory_residency_write_allowed",
    "trace_gate.runtime_closed_loop_counters.memory_residency_durable_write_allowed",
    "trace_gate.runtime_closed_loop_counters.memory_residency_applied",
    "trace_gate.runtime_closed_loop_counters.auto_replay_live_memory_feedback_applied",
    "trace_gate.runtime_closed_loop_counters.auto_replay_business_contract_passed",
    "trace_gate.runtime_closed_loop_counters.auto_replay_live_evolution_memory_updates",
    "trace_gate.runtime_closed_loop_counters.auto_replay_recursive_runtime_calls",
    "trace_gate.runtime_closed_loop_counters.auto_replay_runtime_kv_budget_pressure_items",
    "trace_gate.runtime_closed_loop_counters.kv_fusion_saved_tokens",
    "trace_gate.runtime_closed_loop_counters.self_evolution_rollback_replay_apply_ready",
    "trace_gate.runtime_closed_loop_counters.self_evolution_promotion_preflight_ready",
    "trace_gate.runtime_closed_loop_counters.self_evolution_operator_approval_held",
    "trace_gate.runtime_closed_loop_counters.reasoning_genome_mutation_applied",
    "self_evolution_admission",
    "self_evolution_admission.read_only",
    "self_evolution_admission.validation_passed",
    "self_evolution_admission.validation.passed",
    "self_evolution_admission.validation.compiler.items",
    "self_evolution_admission.validation.compiler.passed",
    "self_evolution_admission.validation.compiler.failed",
    "self_evolution_admission.validation.compiler.validation_passed",
    "self_evolution_admission.validation.tests.items",
    "self_evolution_admission.validation.tests.passed",
    "self_evolution_admission.validation.tests.failed",
    "self_evolution_admission.validation.tests.validation_passed",
    "self_evolution_admission.validation.benchmarks.items",
    "self_evolution_admission.validation.benchmarks.passed",
    "self_evolution_admission.validation.benchmarks.failed",
    "self_evolution_admission.validation.benchmarks.validation_passed",
    "self_evolution_admission.validation.experiments.items",
    "self_evolution_admission.validation.experiments.passed",
    "self_evolution_admission.validation.experiments.failed",
    "self_evolution_admission.validation.experiments.validation_passed",
    "self_evolution_admission.memory_store_write_allowed",
    "self_evolution_admission.ndkv_write_allowed",
    "self_evolution_admission.model_weight_write_allowed",
    "self_evolution_admission.git_write_allowed",
    "self_evolution_admission.blocked_reasons",
];

const MODEL_SERVICE_INSPECT_RESPONSE_FIELDS: &[&str] = &[
    "ok",
    "request_id",
    "state",
    "state.summary",
    "state.genome_profiles",
    "state.genome_profiles.profile",
    "state.genome_profiles.generation",
    "state.genome_profiles.active_genome_id",
    "state.genome_profiles.previous_genome_id",
    "state.genome_profiles.active_gene_count",
    "state.genome_profiles.express_chain_record_count",
    "state.genome_profiles.memory_chain_record_count",
    "state.genome_profiles.dual_chain_consistent",
    "state.genome_profiles.journal_record_count",
    "state.memories",
    "state.runtime_kv_memories",
    "state.experiences",
    "state.experience_hygiene_findings",
    "state.experience_hygiene_watch",
    "state.experience_hygiene_quarantine_candidates",
    "state.experience_hygiene_legacy_metadata_lessons",
    "state.experience_hygiene_legacy_metadata_without_clean_gist",
    "state.experience_repairable_legacy_metadata_lessons",
    "state.experience_repairable_index_records",
    "state.experience_repair_projected_findings",
    "state.experience_repair_projected_watch",
    "state.experience_repair_projected_quarantine_candidates",
    "state.experience_repair_projected_legacy_metadata_lessons",
    "state.experience_repair_projected_legacy_metadata_without_clean_gist",
    "state.experience_repair_skipped_quarantine_candidates",
    "state.experience_repair_skipped_missing_clean_gist",
    "state.experience_hygiene_clean",
    "state.experience_hygiene_samples",
    "state.experience_hygiene_samples.experience_id",
    "state.experience_hygiene_samples.severity",
    "state.experience_hygiene_samples.reason",
    "state.experience_hygiene_samples.markers",
    "state.experience_hygiene_samples.prompt_preview",
    "state.experience_hygiene_samples.lesson_preview",
    "state.runtime_model_experiences",
    "state.runtime_tokens",
    "state.runtime_architecture_experiences",
    "state.runtime_kv_precision_experiences",
    "state.runtime_device_execution_experiences",
    "state.runtime_error_experiences",
    "state.runtime_errors",
    "state.runtime_timeout_experiences",
    "state.runtime_timeouts",
    "state.runtime_error_message_chars",
    "state.rust_check_experiences",
    "state.rust_check_passed",
    "state.rust_check_failed",
    "state.rust_check_diagnostic_chars",
    "state.business_contract_experiences",
    "state.business_contract_passed",
    "state.business_contract_failed",
    "state.business_contract_required_signals",
    "state.business_contract_matched_signals",
    "state.business_contract_missing_signals",
    "state.business_contract_protocol_leaks",
    "state.business_contract_substitutions",
    "state.business_contract_evasive_denials",
    "state.business_contract_missing_handling_signals",
    "state.business_contract_raw_passed",
    "state.business_contract_raw_failed",
    "state.business_contract_response_normalized",
    "state.business_contract_sanitized",
    "state.business_contract_canonical_fallbacks",
    "state.pool_dispatch_experiences",
    "state.pool_dispatch_items",
    "state.pool_dispatch_forwarded",
    "state.pool_dispatch_clamped",
    "state.pool_dispatch_low_priority",
    "state.runtime_adapter_experiences",
    "state.runtime_adapter_selection_mismatches",
    "state.runtime_forward_energy_experiences",
    "state.runtime_kv_influence_experiences",
    "state.runtime_uncertainty_experiences",
    "state.runtime_uncertainty_tokens",
    "state.runtime_kv_precision_mismatches",
    "state.runtime_layer_mode_experiences",
    "state.runtime_all_layer_mode_experiences",
    "state.runtime_global_layers",
    "state.runtime_local_window_layers",
    "state.runtime_convolutional_fusion_layers",
    "state.runtime_kv_import_experiences",
    "state.runtime_kv_weak_import_skip_experiences",
    "state.weak_runtime_kv_imports_skipped",
    "state.runtime_kv_weak_import_pressure_experiences",
    "state.runtime_kv_weak_import_pressure_avg",
    "state.runtime_kv_weak_import_pressure_max",
    "state.runtime_kv_budget_import_skip_experiences",
    "state.budget_limited_runtime_kv_imports_skipped",
    "state.runtime_kv_budget_pressure_experiences",
    "state.runtime_kv_budget_pressure_avg",
    "state.runtime_kv_budget_pressure_max",
    "state.runtime_kv_export_experiences",
    "state.runtime_kv_segment_experiences",
    "state.runtime_kv_segments_included",
    "state.runtime_kv_segments_skipped",
    "state.runtime_kv_segments_rejected",
    "state.runtime_kv_hold_experiences",
    "state.runtime_kv_held_blocks",
    "state.memory_vector_dimensions",
    "state.memory_vector_dimensions.dimensions",
    "state.memory_vector_dimensions.count",
    "state.runtime_kv_vector_dimensions",
    "state.runtime_kv_vector_dimensions.dimensions",
    "state.runtime_kv_vector_dimensions.count",
    "state.top_memories",
    "state.top_memories.id",
    "state.top_memories.key",
    "state.top_memories.vector_dimensions",
    "state.top_memories.strength",
    "state.top_memories.hits",
    "state.top_memories.failures",
    "state.top_memories.last_score",
    "state.top_runtime_kv_memories",
    "state.top_runtime_kv_memories.id",
    "state.top_runtime_kv_memories.key",
    "state.top_runtime_kv_memories.vector_dimensions",
    "state.top_runtime_kv_memories.strength",
    "state.top_runtime_kv_memories.hits",
    "state.top_runtime_kv_memories.failures",
    "state.top_runtime_kv_memories.last_score",
    "state.top_experiences",
    "state.top_experiences.id",
    "state.top_experiences.profile",
    "state.top_experiences.quality",
    "state.top_experiences.process_reward",
    "state.top_experiences.reward_action",
    "state.top_experiences.used_memory_count",
    "state.top_experiences.route_threshold",
    "state.top_experiences.route_attention_tokens",
    "state.top_experiences.route_fast_tokens",
    "state.top_experiences.route_attention_fraction",
    "state.top_experiences.runtime_model",
    "state.top_experiences.runtime_adapter",
    "state.top_experiences.runtime_device",
    "state.top_experiences.runtime_primary_lane",
    "state.top_experiences.runtime_fallback_lane",
    "state.top_experiences.runtime_memory_mode",
    "state.top_experiences.runtime_layer_count",
    "state.top_experiences.runtime_global_layers",
    "state.top_experiences.runtime_local_window_layers",
    "state.top_experiences.runtime_convolutional_fusion_layers",
    "state.top_experiences.runtime_hidden_size",
    "state.top_experiences.runtime_local_window_tokens",
    "state.top_experiences.runtime_forward_energy",
    "state.top_experiences.runtime_kv_influence",
    "state.top_experiences.runtime_token_count",
    "state.top_experiences.runtime_uncertainty_token_count",
    "state.top_experiences.runtime_uncertainty_perplexity",
    "state.top_experiences.runtime_hot_kv_precision_bits",
    "state.top_experiences.runtime_cold_kv_precision_bits",
    "state.top_experiences.runtime_imported_kv_blocks",
    "state.top_experiences.runtime_weak_kv_imports_skipped",
    "state.top_experiences.runtime_budget_limited_kv_imports_skipped",
    "state.top_experiences.runtime_kv_budget_pressure",
    "state.top_experiences.runtime_exported_kv_blocks",
    "state.top_experiences.runtime_kv_segments_included",
    "state.top_experiences.runtime_kv_segments_skipped",
    "state.top_experiences.runtime_kv_segments_rejected",
    "state.top_experiences.runtime_kv_segment_yield",
    "state.top_experiences.recursive_runtime_calls",
    "state.top_experiences.live_online_reward_feedbacks",
    "state.top_experiences.live_online_reward_reinforcements",
    "state.top_experiences.live_online_reward_penalties",
    "state.top_experiences.live_memory_feedback_updates",
    "state.top_experiences.live_memory_feedback_reinforced",
    "state.top_experiences.live_memory_feedback_penalized",
    "state.top_experiences.live_memory_feedback_applied",
    "state.top_experiences.live_memory_feedback_removed",
    "state.top_experiences.live_memory_feedback_missing",
    "state.top_experiences.live_memory_feedback_strength_delta",
    "state.top_experiences.live_memory_feedback_detail",
    "state.top_experiences.runtime_errors",
    "state.top_experiences.runtime_timeouts",
    "state.top_experiences.runtime_error_message_chars",
    "state.top_experiences.rust_check_passed",
    "state.top_experiences.rust_check_failed",
    "state.top_experiences.rust_check_diagnostic_chars",
    "state.top_experiences.business_contract_passed",
    "state.top_experiences.business_contract_failed",
    "state.top_experiences.business_contract_missing_signals",
    "state.top_experiences.business_contract_protocol_leaks",
    "state.top_experiences.business_contract_substitutions",
    "state.top_experiences.business_contract_evasive_denials",
    "state.top_experiences.business_contract_missing_handling_signals",
    "state.top_experiences.business_contract_raw_passed",
    "state.top_experiences.business_contract_raw_failed",
    "state.top_experiences.business_contract_response_normalized",
    "state.top_experiences.business_contract_sanitized",
    "state.top_experiences.business_contract_canonical_fallbacks",
    "state.top_experiences.pool_dispatch_items",
    "state.top_experiences.pool_dispatch_selected_roles",
    "state.top_experiences.pool_dispatch_forwarded",
    "state.top_experiences.pool_dispatch_clamped",
    "state.top_experiences.pool_dispatch_low_priority",
    "state.top_experiences.reflection_issues",
    "state.top_experiences.critical_reflection_issues",
    "state.top_experiences.revision_actions",
    "state.reflection_issue_experiences",
    "state.critical_reflection_issue_experiences",
    "state.revision_action_experiences",
    "state.live_memory_feedback_experiences",
    "state.live_memory_feedback_updates",
    "state.live_memory_feedback_detail_experiences",
    "state.live_memory_feedback_applied",
    "state.live_memory_feedback_removed",
    "state.live_memory_feedback_missing",
    "state.live_memory_feedback_strength_delta",
    "state.profile_observations_general",
    "state.profile_observations_coding",
    "state.profile_observations_writing",
    "state.profile_observations_long_document",
    "state.profile_threshold_general",
    "state.profile_threshold_coding",
    "state.profile_threshold_writing",
    "state.profile_threshold_long_document",
    "state.hierarchy_global",
    "state.hierarchy_local",
    "state.hierarchy_convolution",
    "state.profile_hierarchy_global_general",
    "state.profile_hierarchy_local_general",
    "state.profile_hierarchy_convolution_general",
    "state.profile_hierarchy_global_coding",
    "state.profile_hierarchy_local_coding",
    "state.profile_hierarchy_convolution_coding",
    "state.profile_hierarchy_global_writing",
    "state.profile_hierarchy_local_writing",
    "state.profile_hierarchy_convolution_writing",
    "state.profile_hierarchy_global_long_document",
    "state.profile_hierarchy_local_long_document",
    "state.profile_hierarchy_convolution_long_document",
    "state.profile_hierarchy_observations_general",
    "state.profile_hierarchy_observations_coding",
    "state.profile_hierarchy_observations_writing",
    "state.profile_hierarchy_observations_long_document",
    "state.tier_hot_gpu",
    "state.tier_warm_ram",
    "state.tier_cold_disk",
    "state.memory_retention_stale_after",
    "state.memory_retention_decay_rate",
    "state.memory_retention_remove_below_strength",
    "state.memory_retention_remove_after_failures",
    "state.memory_compaction_similarity_threshold",
    "state.memory_compaction_max_candidates",
    "state.memory_compaction_max_merges",
    "state.experience_index_compacted_records",
    "state.experience_index_overlong_records",
    "state.experience_index_overlong_without_clean_gist",
    "state.experience_index_max_record_chars",
    "state.experience_index_noisy_records",
    "state.experience_index_duplicate_outputs",
    "state.experience_index_max_noise_penalty",
    "state.experience_index_quality_score",
    "state.experience_index_retrieval_ready",
    "state.experience_index_risk_level",
    "state.experience_index_samples",
    "state.experience_index_samples.experience_id",
    "state.experience_index_samples.reason",
    "state.experience_index_samples.compacted",
    "state.experience_index_samples.noise_penalty",
    "state.experience_index_samples.prompt_chars",
    "state.experience_index_samples.lesson_chars",
    "state.experience_index_samples.prompt_preview",
    "state.experience_index_samples.lesson_preview",
    "state.router_threshold",
    "state.router_observations",
    "state.evolution_live_inference_runs",
    "state.evolution_live_router_threshold_mutations",
    "state.evolution_live_hierarchy_weight_mutations",
    "state.evolution_live_router_threshold_delta",
    "state.evolution_live_hierarchy_weight_delta",
    "state.evolution_live_online_reward_feedbacks",
    "state.evolution_live_online_reward_reinforcements",
    "state.evolution_live_online_reward_penalties",
    "state.evolution_live_online_reward_strength",
    "state.evolution_live_online_reward_reinforcement_strength",
    "state.evolution_live_online_reward_penalty_strength",
    "state.evolution_live_memory_updates",
    "state.evolution_live_memory_reinforcements",
    "state.evolution_live_memory_penalties",
    "state.evolution_live_stored_memory_updates",
    "state.evolution_live_stored_memories",
    "state.evolution_live_stored_gist_memories",
    "state.evolution_live_stored_runtime_kv_memories",
    "state.evolution_live_reflection_issues",
    "state.evolution_live_critical_reflection_issues",
    "state.evolution_live_revision_actions",
    "state.evolution_replay_runs",
    "state.evolution_replay_items",
    "state.evolution_external_feedbacks",
    "state.evolution_external_feedback_reinforcements",
    "state.evolution_external_feedback_penalties",
    "state.evolution_external_feedback_memory_updates",
    "state.evolution_external_feedback_removed",
    "state.evolution_external_feedback_missing",
    "state.evolution_external_feedback_strength_delta",
    "state.evolution_memory_updates",
    "state.evolution_memory_reinforcements",
    "state.evolution_memory_penalties",
    "state.evolution_replay_live_memory_feedback_items",
    "state.evolution_replay_live_memory_feedback_updates",
    "state.evolution_replay_live_memory_feedback_reinforcements",
    "state.evolution_replay_live_memory_feedback_penalties",
    "state.evolution_replay_live_memory_feedback_detail_items",
    "state.evolution_replay_live_memory_feedback_applied",
    "state.evolution_replay_live_memory_feedback_removed",
    "state.evolution_replay_live_memory_feedback_missing",
    "state.evolution_replay_live_memory_feedback_strength_delta",
    "state.evolution_replay_rust_check_items",
    "state.evolution_replay_rust_check_passed",
    "state.evolution_replay_rust_check_failed",
    "state.evolution_replay_rust_check_diagnostic_chars",
    "state.evolution_replay_rust_check_live_memory_feedback_items",
    "state.evolution_replay_rust_check_live_memory_feedback_updates",
    "state.evolution_replay_rust_check_live_memory_feedback_applied",
    "state.evolution_replay_rust_check_live_memory_feedback_strength_delta",
    "state.evolution_replay_business_contract_items",
    "state.evolution_replay_business_contract_passed",
    "state.evolution_replay_business_contract_failed",
    "state.evolution_replay_business_contract_raw_passed",
    "state.evolution_replay_business_contract_raw_failed",
    "state.evolution_replay_business_contract_response_normalized",
    "state.evolution_replay_business_contract_sanitized",
    "state.evolution_replay_business_contract_canonical_fallbacks",
    "state.evolution_router_threshold_mutations",
    "state.evolution_hierarchy_weight_mutations",
    "state.evolution_router_threshold_delta",
    "state.evolution_hierarchy_weight_delta",
    "state.evolution_replay_live_evolution_items",
    "state.evolution_replay_live_evolution_router_threshold_mutations",
    "state.evolution_replay_live_evolution_hierarchy_weight_mutations",
    "state.evolution_replay_live_evolution_router_threshold_delta",
    "state.evolution_replay_live_evolution_hierarchy_weight_delta",
    "state.evolution_replay_live_evolution_online_reward_feedbacks",
    "state.evolution_replay_live_evolution_online_reward_reinforcements",
    "state.evolution_replay_live_evolution_online_reward_penalties",
    "state.evolution_replay_live_evolution_online_reward_strength",
    "state.evolution_replay_live_evolution_online_reward_reinforcement_strength",
    "state.evolution_replay_live_evolution_online_reward_penalty_strength",
    "state.evolution_replay_live_evolution_memory_updates",
    "state.evolution_replay_live_evolution_stored_memory_updates",
    "state.evolution_replay_live_evolution_reflection_issues",
    "state.evolution_replay_live_evolution_critical_reflection_issues",
    "state.evolution_replay_live_evolution_revision_actions",
    "state.evolution_drift_rollbacks",
    "state.evolution_rollback_router_threshold_delta",
    "state.evolution_rollback_hierarchy_weight_delta",
    "state.evolution_recursive_replay_items",
    "state.evolution_recursive_runtime_calls",
    "state_gate",
    "trace_gate",
    "trace_gate.runtime_closed_loop_counters",
    "trace_gate.runtime_closed_loop_counters.compute_budget_saved_tokens",
    "trace_gate.runtime_closed_loop_counters.compute_budget_avoided_tokens",
    "trace_gate.runtime_closed_loop_counters.compute_budget_write_allowed",
    "trace_gate.runtime_closed_loop_counters.compute_budget_applied",
    "trace_gate.runtime_closed_loop_counters.memory_admission_ledger_authorized",
    "trace_gate.runtime_closed_loop_counters.memory_admission_ledger_applied",
    "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_write_allowed",
    "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_durable_write_allowed",
    "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_applied",
    "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_applied_to_disk",
    "trace_gate.runtime_closed_loop_counters.memory_residency_write_allowed",
    "trace_gate.runtime_closed_loop_counters.memory_residency_durable_write_allowed",
    "trace_gate.runtime_closed_loop_counters.memory_residency_applied",
    "trace_gate.runtime_closed_loop_counters.auto_replay_live_memory_feedback_applied",
    "trace_gate.runtime_closed_loop_counters.auto_replay_business_contract_passed",
    "trace_gate.runtime_closed_loop_counters.auto_replay_live_evolution_memory_updates",
    "trace_gate.runtime_closed_loop_counters.auto_replay_recursive_runtime_calls",
    "trace_gate.runtime_closed_loop_counters.auto_replay_runtime_kv_budget_pressure_items",
    "trace_gate.runtime_closed_loop_counters.kv_fusion_saved_tokens",
    "trace_gate.runtime_closed_loop_counters.self_evolution_rollback_replay_apply_ready",
    "trace_gate.runtime_closed_loop_counters.self_evolution_promotion_preflight_ready",
    "trace_gate.runtime_closed_loop_counters.self_evolution_operator_approval_held",
    "trace_gate.runtime_closed_loop_counters.reasoning_genome_mutation_applied",
];

fn endpoint_response_fields(endpoint: &str) -> &'static [&'static str] {
    match endpoint {
        "chat-completions" | "completions" => OPENAI_RESPONSE_FIELDS,
        "chat-stream" | "generate-stream" => MODEL_SERVICE_STREAM_RESPONSE_FIELDS,
        "business-cycle-stream" => &[
            "event:status",
            "event:stage",
            "event:delta",
            "event:meta",
            "event:final",
            "event:done",
            "event:error",
        ],
        "generate" | "chat" => &[
            "ok",
            "request_id",
            "profile",
            "language_mode",
            "coding_language",
            "rust_coding",
            "task_mode",
            "task_language",
            "coding_intent",
            "validation_mode",
            "memory_need",
            "compute_budget",
            "compute_budget_summary",
            "compute_budget_saved_tokens",
            "compute_budget_avoided_tokens",
            "compute_budget_kv_lookups_skipped",
            "compute_budget_fanout_reduction",
            "compute_budget_read_only",
            "compute_budget_write_allowed",
            "compute_budget_applied",
            "requested_max_tokens",
            "route_threshold",
            "route_attention_tokens",
            "route_fast_tokens",
            "route_attention_fraction",
            "elapsed_ms",
            "output_mode",
            "answer",
            "raw_answer",
            "enhanced_answer",
            "quality",
            "process_reward",
            "action",
            "memory_stored",
            "stored_memory_id",
            "used_memory_count",
            "used_memory_ids",
            "stored_gist_memory_ids",
            "stored_runtime_kv_memory_ids",
            "feedback_memory_ids",
            "experience_id",
            "runtime_model",
            "runtime_adapter",
            "runtime_device",
            "runtime_primary_lane",
            "runtime_fallback_lane",
            "runtime_memory_mode",
            "runtime_forward_energy",
            "runtime_hot_kv_precision_bits",
            "runtime_cold_kv_precision_bits",
            "runtime_token_count",
            "runtime_entropy_count",
            "runtime_logprob_count",
            "runtime_uncertainty_token_count",
            "runtime_uncertainty_signal",
            "runtime_average_entropy",
            "runtime_average_neg_logprob",
            "runtime_uncertainty_perplexity",
            "runtime_architecture_signal",
            "runtime_kv_precision_signal",
            "runtime_device_execution_source",
            "runtime_kv_influence",
            "runtime_imported_kv_blocks",
            "runtime_weak_kv_imports_skipped",
            "runtime_budget_limited_kv_imports_skipped",
            "runtime_kv_budget_pressure",
            "runtime_exported_kv_blocks",
            "runtime_kv_segments_included",
            "runtime_kv_segments_skipped",
            "runtime_kv_segments_rejected",
            "runtime_kv_segment_yield",
            "runtime_closed_loop_counters",
            "runtime_closed_loop_counters.adaptive_routing_candidates",
            "runtime_closed_loop_counters.adaptive_routing_saved_tokens",
            "runtime_closed_loop_counters.adaptive_routing_threshold_delta_milli",
            "runtime_closed_loop_counters.task_hierarchy_mutation_records",
            "runtime_closed_loop_counters.task_hierarchy_compute_reduction_milli",
            "runtime_closed_loop_counters.task_hierarchy_weight_delta_milli",
            "runtime_closed_loop_counters.compute_budget_selected_candidates",
            "runtime_closed_loop_counters.compute_budget_kv_lookups_skipped",
            "runtime_closed_loop_counters.compute_budget_saved_tokens",
            "runtime_closed_loop_counters.compute_budget_avoided_tokens",
            "runtime_closed_loop_counters.compute_budget_write_allowed",
            "runtime_closed_loop_counters.compute_budget_applied",
            "runtime_closed_loop_counters.memory_admission_candidates",
            "runtime_closed_loop_counters.memory_admission_ready",
            "runtime_closed_loop_counters.memory_admission_blocked",
            "runtime_closed_loop_counters.memory_admission_ledger_records",
            "runtime_closed_loop_counters.memory_admission_ledger_preview_only",
            "runtime_closed_loop_counters.memory_admission_ledger_authorized",
            "runtime_closed_loop_counters.memory_admission_ledger_applied",
            "runtime_closed_loop_counters.memory_admission_write_allowed",
            "runtime_closed_loop_counters.memory_admission_applied",
            "runtime_closed_loop_counters.kv_fusion_candidates",
            "runtime_closed_loop_counters.kv_fusion_fused",
            "runtime_closed_loop_counters.kv_fusion_compressed",
            "runtime_closed_loop_counters.kv_fusion_skipped",
            "runtime_closed_loop_counters.kv_fusion_held",
            "runtime_closed_loop_counters.kv_fusion_rejected",
            "runtime_closed_loop_counters.kv_fusion_approval_blocked",
            "runtime_closed_loop_counters.kv_fusion_input_tokens",
            "runtime_closed_loop_counters.kv_fusion_retained_tokens",
            "runtime_closed_loop_counters.kv_fusion_saved_tokens",
            "runtime_closed_loop_counters.kv_fusion_write_allowed",
            "runtime_closed_loop_counters.kv_fusion_applied",
            "runtime_closed_loop_counters.self_evolving_memory_store_updates",
            "runtime_closed_loop_counters.self_evolving_memory_store_primary_applied",
            "runtime_closed_loop_counters.self_evolving_memory_store_gist_applied",
            "runtime_closed_loop_counters.self_evolving_memory_store_runtime_kv_applied",
            "runtime_closed_loop_counters.memory_residency_retention_decayed",
            "runtime_closed_loop_counters.memory_residency_retention_removed",
            "runtime_closed_loop_counters.memory_residency_compaction_merged",
            "runtime_closed_loop_counters.memory_residency_compaction_removed",
            "runtime_closed_loop_counters.reflection_issues",
            "runtime_closed_loop_counters.reflection_critical_issues",
            "runtime_closed_loop_counters.reflection_revision_actions",
            "runtime_closed_loop_counters.online_reward_feedbacks",
            "runtime_closed_loop_counters.online_reward_reinforcements",
            "runtime_closed_loop_counters.online_reward_penalties",
            "runtime_closed_loop_counters.online_reward_strength_milli",
            "runtime_closed_loop_counters.online_reward_reinforcement_strength_milli",
            "runtime_closed_loop_counters.online_reward_penalty_strength_milli",
            "runtime_closed_loop_counters.memory_feedback_updates",
            "runtime_closed_loop_counters.memory_feedback_reinforcements",
            "runtime_closed_loop_counters.memory_feedback_penalties",
            "runtime_closed_loop_counters.noiron_orchestration_stages",
            "runtime_closed_loop_counters.noiron_orchestration_completed_stages",
            "runtime_closed_loop_counters.noiron_orchestration_failed_stages",
            "runtime_closed_loop_counters.noiron_orchestration_preview_only_stages",
            "runtime_closed_loop_counters.noiron_orchestration_gated_stages",
            "runtime_closed_loop_counters.noiron_orchestration_rolled_back_stages",
            "runtime_closed_loop_counters.noiron_orchestration_rollback_records",
            "runtime_closed_loop_counters.noiron_orchestration_writes_gated",
            "runtime_closed_loop_counters.noiron_orchestration_durable_memory_ledger_authorized",
            "runtime_closed_loop_counters.noiron_orchestration_durable_memory_ledger_applied",
            "dna_closed_loop",
            "dna_closed_loop.strategy",
            "dna_closed_loop.strategy_genome_id",
            "dna_closed_loop.strategy_gene_count",
            "dna_closed_loop.generation_before",
            "dna_closed_loop.generation_after",
            "dna_closed_loop.active_genome_id_after",
            "dna_closed_loop.reasoning_frame_id",
            "dna_closed_loop.reasoning_frame_valid",
            "dna_closed_loop.reasoning_frame_vm_executed",
            "dna_closed_loop.reasoning_frame_opcode_count",
            "dna_closed_loop.task_gene_decision",
            "dna_closed_loop.task_skill_decision",
            "dna_closed_loop.writer_gate_decision",
            "dna_closed_loop.apply_plan_decision",
            "dna_closed_loop.mutation_count",
            "dna_closed_loop.dual_chain_committed",
            "dna_closed_loop.express_chain_records",
            "dna_closed_loop.memory_chain_records",
            "dna_closed_loop.mutation_applied",
            "dna_closed_loop.rollback_applied",
            "dna_closed_loop.receipt_reason",
            "traceable",
            "endpoint",
            "error",
            "error_type",
            "cancelled",
            "timeout",
            "retryable",
            "runtime_error_note",
            "persistent_writes",
            "memory_write_allowed",
            "genome_write_allowed",
            "self_evolution_write_allowed",
        ],
        "business-cycle" => &[
            "ok",
            "request_id",
            "pool_dispatch",
            "pool_stage_dispatch",
            "business_cycle",
            "generate",
            "feedback",
            "rust_check",
            "self_improve",
            "replay",
            "state",
            "state_gate",
            "trace_gate",
            "trace_gate.runtime_closed_loop_counters",
            "trace_gate.runtime_closed_loop_counters.compute_budget_saved_tokens",
            "trace_gate.runtime_closed_loop_counters.compute_budget_avoided_tokens",
            "trace_gate.runtime_closed_loop_counters.compute_budget_write_allowed",
            "trace_gate.runtime_closed_loop_counters.compute_budget_applied",
            "trace_gate.runtime_closed_loop_counters.memory_admission_ledger_authorized",
            "trace_gate.runtime_closed_loop_counters.memory_admission_ledger_applied",
            "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_write_allowed",
            "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_durable_write_allowed",
            "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_applied",
            "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_applied_to_disk",
            "trace_gate.runtime_closed_loop_counters.memory_residency_write_allowed",
            "trace_gate.runtime_closed_loop_counters.memory_residency_durable_write_allowed",
            "trace_gate.runtime_closed_loop_counters.memory_residency_applied",
            "trace_gate.runtime_closed_loop_counters.auto_replay_live_memory_feedback_applied",
            "trace_gate.runtime_closed_loop_counters.auto_replay_business_contract_passed",
            "trace_gate.runtime_closed_loop_counters.auto_replay_live_evolution_memory_updates",
            "trace_gate.runtime_closed_loop_counters.auto_replay_recursive_runtime_calls",
            "trace_gate.runtime_closed_loop_counters.auto_replay_runtime_kv_budget_pressure_items",
            "trace_gate.runtime_closed_loop_counters.kv_fusion_saved_tokens",
            "trace_gate.runtime_closed_loop_counters.self_evolution_rollback_replay_apply_ready",
            "trace_gate.runtime_closed_loop_counters.self_evolution_promotion_preflight_ready",
            "trace_gate.runtime_closed_loop_counters.self_evolution_operator_approval_held",
            "trace_gate.runtime_closed_loop_counters.reasoning_genome_mutation_applied",
            "eval",
            "error",
        ],
        "feedback" => MODEL_SERVICE_FEEDBACK_RESPONSE_FIELDS,
        "rust-check" => MODEL_SERVICE_RUST_CHECK_RESPONSE_FIELDS,
        "replay" => MODEL_SERVICE_REPLAY_RESPONSE_FIELDS,
        "self-improve" => MODEL_SERVICE_SELF_IMPROVE_RESPONSE_FIELDS,
        "state" => &[
            "ok",
            "request_id",
            "state",
            "state.genome_profiles",
            "state.genome_profiles.profile",
            "state.genome_profiles.generation",
            "state.genome_profiles.active_genome_id",
            "state.genome_profiles.previous_genome_id",
            "state.genome_profiles.active_gene_count",
            "state.genome_profiles.express_chain_record_count",
            "state.genome_profiles.memory_chain_record_count",
            "state.genome_profiles.dual_chain_consistent",
            "state.genome_profiles.journal_record_count",
            "state_gate",
            "trace_gate",
            "error",
        ],
        "inspect" => MODEL_SERVICE_INSPECT_RESPONSE_FIELDS,
        "experience-retrieval" => &[
            "ok",
            "request_id",
            "retrieval",
            "prompt",
            "profile",
            "retrieval_elapsed_ms",
            "index_context_used",
            "index_context_chars",
            "total_records",
            "requested_limit",
            "matches",
            "match_count",
            "skipped_cross_task_pollution",
            "development_evidence_surface_blocked_candidates",
            "retrieval_noise_penalized_candidates",
            "retrieval_noise_filtered_candidates",
            "suppressed_prompt_index_candidates",
            "max_retrieval_noise_penalty",
            "max_score",
            "experience_id",
            "score",
            "quality",
            "process_reward",
            "reward_action",
            "used_memory_count",
            "stored_runtime_kv_memory_ids",
            "route_threshold",
            "route_attention_tokens",
            "route_fast_tokens",
            "route_attention_fraction",
            "prompt_preview",
            "lesson_preview",
            "usable_hint_preview",
            "gist_hints",
            "reflection_issue_codes",
            "revision_actions",
            "runtime_model",
            "runtime_adapter",
            "runtime_device",
            "runtime_primary_lane",
            "runtime_fallback_lane",
            "runtime_memory_mode",
            "runtime_device_execution_source",
            "runtime_forward_energy",
            "runtime_kv_influence",
            "runtime_uncertainty_perplexity",
            "recursive_runtime_calls",
        ],
        "experience-hygiene-quarantine" => &[
            "ok",
            "request_id",
            "experience_file",
            "applied",
            "backup_file",
            "quarantine_file",
            "plan",
            "total_records",
            "retained_records",
            "quarantine_candidates",
            "candidate_ids",
            "listed_findings",
            "experience_id",
            "severity",
            "reason",
            "markers",
            "prompt_preview",
            "lesson_preview",
        ],
        "experience-cleanup-audit" => &[
            "ok",
            "request_id",
            "experience_file",
            "checked",
            "writes_experience_state",
            "sample_limit",
            "error",
            "report",
            "index_report",
            "quarantine_plan",
            "repair_plan",
            "next_step",
            "total_records",
            "findings",
            "watch",
            "quarantine_candidates",
            "legacy_metadata_lessons",
            "legacy_metadata_without_clean_gist",
            "clean",
            "listed_findings",
            "compacted_records",
            "overlong_records",
            "overlong_without_clean_gist",
            "max_record_chars",
            "noisy_records",
            "duplicate_outputs",
            "max_noise_penalty",
            "quality_score",
            "retrieval_ready",
            "risk_level",
            "recommended_action",
            "retained_records",
            "candidate_ids",
            "experience_id",
            "severity",
            "reason",
            "markers",
            "prompt_preview",
            "lesson_preview",
            "repairable_legacy_metadata_lessons",
            "repairable_index_records",
            "remaining_legacy_metadata_lessons_after_repair",
            "remaining_watch_after_repair",
            "remaining_quarantine_candidates_after_repair",
            "skipped_quarantine_candidates",
            "skipped_missing_clean_gist",
            "projected_hygiene_after_repair",
            "listed_repairs",
            "listed_skipped_quarantine_candidates",
            "listed_skipped_missing_clean_gist",
        ],
        "experience-repair" => &[
            "ok",
            "request_id",
            "experience_file",
            "applied",
            "backup_file",
            "plan",
            "total_records",
            "legacy_metadata_lessons",
            "repairable_legacy_metadata_lessons",
            "index_noisy_records",
            "index_duplicate_outputs",
            "repairable_index_records",
            "remaining_legacy_metadata_lessons_after_repair",
            "remaining_watch_after_repair",
            "remaining_quarantine_candidates_after_repair",
            "skipped_quarantine_candidates",
            "skipped_missing_clean_gist",
            "projected_hygiene_after_repair",
            "legacy_metadata_without_clean_gist",
            "index_quality_score",
            "index_retrieval_ready",
            "index_risk_level",
            "listed_repairs",
            "listed_skipped_quarantine_candidates",
            "listed_skipped_missing_clean_gist",
            "experience_id",
            "action",
            "source",
            "old_lesson_preview",
            "proposed_lesson_preview",
            "source_gist_preview",
            "reason",
            "prompt_preview",
            "gist_count",
        ],
        "requests-cancel" => &[
            "ok",
            "request_id",
            "target_request_id",
            "target_active",
            "target_endpoint",
            "repair_factor_released",
            "repair_factor",
            "retag_applied",
            "retag_label",
            "reason",
            "cooperative_only",
            "persistent_writes",
            "next_step",
        ],
        "model-pool-route-plan" => &[
            "ok",
            "request_id",
            "schema_version",
            "contract_version",
            "task_kind",
            "read_only",
            "launches_process",
            "sends_prompt",
            "route_allowed",
            "reason",
            "route_block_reason",
            "agent_model_route_source",
            "agent_model_route_source.route_allowed",
            "agent_model_route_source.proof_ready",
            "agent_model_route_source.proof_block_reason",
            "agent_model_route_source.selected_role",
            "agent_model_route_source.model_registry_id",
            "agent_model_route_source.model_profile_id",
            "agent_model_route_source.inference_backend_id",
            "agent_model_route_source.model_pool_id",
            "role_candidates",
            "routing_weights",
            "service_backpressure",
            "dependency_precheck",
            "quality_context_tokens",
            "quality_context_required_tokens",
            "quality_context_sufficient",
            "quality_block_reason",
            "selected_role",
            "selected_base_url",
            "selected_port",
            "selected_default_max_tokens",
            "selected_context_window",
            "selected_context_required_tokens",
            "selected_context_buffer_tokens",
            "selected_context_buffer_policy",
            "selected_context_sufficient",
            "selected_context_block_reason",
            "configured_max_tokens",
            "effective_max_tokens",
            "max_tokens_clamped",
            "max_tokens_clamp_reason",
            "compute_budget_summary",
            "compute_budget_configured_max_tokens",
            "compute_budget_effective_max_tokens",
            "compute_budget_saved_tokens",
            "compute_budget_avoided_tokens",
            "compute_budget_max_tokens_clamped",
            "runtime_closed_loop_counters",
            "runtime_closed_loop_counters.compute_budget_saved_tokens",
            "runtime_closed_loop_counters.compute_budget_avoided_tokens",
            "runtime_closed_loop_counters.compute_budget_max_tokens_clamped",
            "runtime_closed_loop_counters.model_pool_budget_applied",
            "pool_dispatch",
            "route_metrics",
            "route_metrics.success_rate_milli",
            "route_metrics.failure_rate_milli",
            "worker_metrics",
            "worker_metrics.success_rate_milli",
            "worker_metrics.failure_rate_milli",
            "candidate_workers",
        ],
        "model-pool-call" => &[
            "ok",
            "request_id",
            "schema_version",
            "contract_version",
            "task_kind",
            "read_only",
            "launches_process",
            "sends_prompt",
            "route_allowed",
            "reason",
            "route_block_reason",
            "role_candidates",
            "dependency_precheck",
            "quality_context_tokens",
            "quality_context_required_tokens",
            "quality_context_sufficient",
            "quality_block_reason",
            "selected_role",
            "selected_base_url",
            "selected_port",
            "selected_default_max_tokens",
            "configured_max_tokens",
            "effective_max_tokens",
            "max_tokens_clamped",
            "max_tokens_clamp_reason",
            "pool_dispatch",
            "route_metrics",
            "route_metrics.success_rate_milli",
            "route_metrics.failure_rate_milli",
            "worker_metrics",
            "worker_metrics.success_rate_milli",
            "worker_metrics.failure_rate_milli",
            "candidate_workers",
            "elapsed_ms",
            "answer_chars",
            "answer_bytes",
            "answer_approx_tokens",
            "answer",
            "endpoint",
            "call_state",
            "cancelled",
            "timeout",
            "partial_result",
            "partial_finalized",
            "queue_time_ms",
            "compute_budget_summary",
            "compute_budget_configured_max_tokens",
            "compute_budget_effective_max_tokens",
            "compute_budget_saved_tokens",
            "compute_budget_avoided_tokens",
            "compute_budget_max_tokens_clamped",
            "runtime_closed_loop_counters",
            "runtime_closed_loop_counters.compute_budget_saved_tokens",
            "runtime_closed_loop_counters.compute_budget_avoided_tokens",
            "runtime_closed_loop_counters.compute_budget_max_tokens_clamped",
            "runtime_closed_loop_counters.model_pool_budget_applied",
            "error",
            "retryable",
            "dispatch_attempted",
            "persistent_writes",
            "memory_write_allowed",
            "genome_write_allowed",
            "self_evolution_write_allowed",
        ],
        _ => &["ok", "request_id", "error"],
    }
}

fn endpoint_stream_response_fields(endpoint: &str) -> &'static [&'static str] {
    match endpoint {
        "chat-completions" => OPENAI_CHAT_STREAM_RESPONSE_FIELDS,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Args;

    #[test]
    fn endpoint_info_json_reports_business_cycle_stream_route() {
        let json = model_service_endpoint_info_json(7, "business-cycle-stream");

        assert!(json.contains("\"request_id\":7"));
        assert!(json.contains("\"endpoint\":\"/v1/business-cycle-stream\""));
        assert!(json.contains("\"self_improve\":true"));
        assert!(json.contains("\"supported_fields\":[\"prompt\",\"profile\",\"case\",\"max_tokens\",\"max\",\"feedback_action\",\"action\",\"feedback_amount\",\"amount\",\"rust_check_code\",\"code\",\"rust_check_edition\",\"edition\",\"rust_check_case\",\"rust_case\",\"self_improve\",\"self_improve_limit\",\"limit\",\"pool_dispatch\",\"pool_stage_dispatch\",\"gate\",\"trace_gate\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"response_fields\":[\"event:status\",\"event:stage\",\"event:delta\",\"event:meta\",\"event:final\",\"event:done\",\"event:error\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"model\",\"messages\",\"stream\",\"tools\",\"tool_choice\",\"response_format\",\"logprobs\"]"));
        assert!(!json.contains("\"endpoint\":\"/v1/generate\""));
    }

    #[test]
    fn endpoint_info_json_reports_business_cycle_contract() {
        let json = model_service_endpoint_info_json(17, "business-cycle");

        assert!(json.contains("\"request_id\":17"));
        assert!(json.contains("\"endpoint\":\"/v1/business-cycle\""));
        assert!(json.contains("\"feedback_amount\":0.4"));
        assert!(json.contains("\"supported_fields\":[\"prompt\",\"profile\",\"case\",\"max_tokens\",\"max\",\"feedback_action\",\"action\",\"feedback_amount\",\"amount\",\"rust_check_code\",\"code\",\"rust_check_edition\",\"edition\",\"rust_check_case\",\"rust_case\",\"self_improve\",\"self_improve_limit\",\"limit\",\"pool_dispatch\",\"pool_stage_dispatch\",\"gate\",\"trace_gate\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"pool_dispatch\",\"pool_stage_dispatch\",\"business_cycle\",\"generate\",\"feedback\",\"rust_check\",\"self_improve\",\"replay\",\"state\",\"state_gate\",\"trace_gate\",\"trace_gate.runtime_closed_loop_counters\",\"trace_gate.runtime_closed_loop_counters.compute_budget_saved_tokens\",\"trace_gate.runtime_closed_loop_counters.compute_budget_avoided_tokens\",\"trace_gate.runtime_closed_loop_counters.compute_budget_write_allowed\",\"trace_gate.runtime_closed_loop_counters.compute_budget_applied\",\"trace_gate.runtime_closed_loop_counters.memory_admission_ledger_authorized\",\"trace_gate.runtime_closed_loop_counters.memory_admission_ledger_applied\",\"trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_write_allowed\",\"trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_durable_write_allowed\",\"trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_applied\",\"trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_applied_to_disk\",\"trace_gate.runtime_closed_loop_counters.memory_residency_write_allowed\",\"trace_gate.runtime_closed_loop_counters.memory_residency_durable_write_allowed\",\"trace_gate.runtime_closed_loop_counters.memory_residency_applied\",\"trace_gate.runtime_closed_loop_counters.auto_replay_live_memory_feedback_applied\",\"trace_gate.runtime_closed_loop_counters.auto_replay_business_contract_passed\",\"trace_gate.runtime_closed_loop_counters.auto_replay_live_evolution_memory_updates\",\"trace_gate.runtime_closed_loop_counters.auto_replay_recursive_runtime_calls\",\"trace_gate.runtime_closed_loop_counters.auto_replay_runtime_kv_budget_pressure_items\",\"trace_gate.runtime_closed_loop_counters.kv_fusion_saved_tokens\",\"trace_gate.runtime_closed_loop_counters.self_evolution_rollback_replay_apply_ready\",\"trace_gate.runtime_closed_loop_counters.self_evolution_promotion_preflight_ready\",\"trace_gate.runtime_closed_loop_counters.self_evolution_operator_approval_held\",\"trace_gate.runtime_closed_loop_counters.reasoning_genome_mutation_applied\",\"eval\",\"error\"]"));
        assert_trace_gate_runtime_closed_loop_contract_fields(&json);
        assert!(json.contains("\"unsupported_fields\":[\"model\",\"messages\",\"stream\",\"tools\",\"tool_choice\",\"response_format\",\"logprobs\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_evolution_endpoint_contracts() {
        let feedback = model_service_endpoint_info_json(18, "feedback");
        assert!(feedback.contains("\"endpoint\":\"/v1/feedback\""));
        assert!(feedback.contains(
            "\"supported_fields\":[\"experience_id\",\"memory_id\",\"action\",\"amount\",\"tenant_id\",\"workspace_id\",\"session_id\"]"
        ));
        assert!(feedback.contains("\"feedback.strength_delta\""));
        assert!(feedback.contains("\"feedback.updates.strength_delta\""));
        assert!(feedback.contains("\"state.evolution_external_feedbacks\""));

        let rust_check = model_service_endpoint_info_json(19, "rust-check");
        assert!(rust_check.contains("\"endpoint\":\"/v1/rust-check\""));
        assert!(rust_check.contains(
            "\"supported_fields\":[\"code\",\"edition\",\"case\",\"amount\",\"experience_id\",\"memory_id\",\"tenant_id\",\"workspace_id\",\"session_id\"]"
        ));
        assert!(rust_check.contains("\"rust_check.passed\""));
        assert!(rust_check.contains("\"feedback.experience_id\""));
        assert!(rust_check.contains("\"feedback.memory_id\""));
        assert!(rust_check.contains("\"feedback.memory_ids\""));
        assert!(rust_check.contains("\"feedback.missing\""));
        assert!(rust_check.contains("\"feedback.removed\""));
        assert!(rust_check.contains("\"feedback.updates\""));
        assert!(rust_check.contains("\"feedback.updates.id\""));
        assert!(rust_check.contains("\"feedback.updates.strength_after\""));

        let replay = model_service_endpoint_info_json(20, "replay");
        assert!(replay.contains("\"endpoint\":\"/v1/replay\""));
        assert!(replay.contains(
            "\"supported_fields\":[\"limit\",\"tenant_id\",\"workspace_id\",\"session_id\"]"
        ));
        assert!(replay.contains("\"replay.runtime_kv_budget_pressure_items\""));
        assert!(replay.contains("\"replay.avg_runtime_kv_weak_import_pressure\""));
        assert!(replay.contains("\"replay.recursive_runtime_items\""));
        assert!(replay.contains("\"replay.max_recursive_call_pressure\""));
        assert!(replay.contains("\"replay.live_memory_feedback_detail_items\""));
        assert!(replay.contains("\"replay.live_memory_feedback_removed\""));
        assert!(replay.contains("\"replay.rust_check_failed\""));
        assert!(replay.contains("\"replay.rust_check_live_memory_feedback_applied\""));
        assert!(replay.contains("\"replay.business_contract_response_normalized\""));
        assert!(replay.contains("\"replay.business_contract_canonical_fallbacks\""));
        assert!(replay.contains("\"replay.pool_dispatch_forwarded\""));
        assert!(replay.contains("\"replay.pool_dispatch_clamped\""));
        assert!(replay.contains("\"replay.pool_dispatch_low_priority\""));
        assert!(replay.contains("\"replay.live_evolution_items\""));
        assert!(replay.contains("\"replay.live_evolution_router_threshold_mutations\""));
        assert!(replay.contains("\"replay.live_evolution_hierarchy_weight_mutations\""));
        assert!(replay.contains("\"replay.live_evolution_online_reward_feedbacks\""));
        assert!(replay.contains("\"replay.live_evolution_online_reward_reinforcements\""));
        assert!(replay.contains("\"replay.live_evolution_online_reward_penalties\""));
        assert!(replay.contains("\"replay.live_evolution_online_reward_strength\""));
        assert!(replay.contains("\"replay.live_evolution_online_reward_penalty_strength\""));
        assert!(replay.contains("\"replay.live_evolution_stored_memory_updates\""));
        assert!(replay.contains("\"replay.live_evolution_critical_reflection_issues\""));
        assert!(replay.contains("\"replay.live_evolution_revision_actions\""));
        assert!(replay.contains("\"state.evolution_replay_runs\""));

        let self_improve = model_service_endpoint_info_json(21, "self-improve");
        assert!(self_improve.contains("\"endpoint\":\"/v1/self-improve\""));
        assert!(self_improve.contains(
            "\"supported_fields\":[\"limit\",\"gate\",\"state_gate\",\"business_gate\",\"business_cycle_gate\",\"model_service_gate\",\"trace_gate\",\"require_deep_self_evolution\",\"tenant_id\",\"workspace_id\",\"session_id\"]"
        ));
        assert!(self_improve.contains("\"trace_gate\""));
        assert_trace_gate_runtime_closed_loop_contract_fields(&self_improve);
        assert!(self_improve.contains("\"self_improve.replay_planned\""));
        assert!(self_improve.contains("\"self_improve.replay_applied\""));
        assert!(self_improve.contains("\"self_improve.trace_gate_passed\""));
        assert!(self_improve.contains("\"self_improve.model_service_gate\""));
        assert!(self_improve.contains("\"self_improve.require_deep_self_evolution\""));
        assert!(self_improve.contains("\"self_improve.deep_self_evolution_passed\""));
        assert!(self_improve.contains("\"self_improve.depth_status\""));
        assert!(self_improve.contains("\"self_improve.reflection_issue_experiences\""));
        assert!(self_improve.contains("\"self_improve.revision_action_experiences\""));
        assert!(self_improve.contains("\"self_improve.live_memory_feedback_applied\""));
        assert!(self_improve.contains("\"self_improve.depth_failures\""));
        assert!(self_improve.contains("\"self_improve.self_evolution_admission_checked\""));
        assert!(self_improve.contains("\"self_improve.self_evolution_admission_blocked_reasons\""));
        assert!(self_improve.contains("\"self_improve.self_evolution_admission_trace_blocked\""));
        assert!(self_improve.contains("\"self_evolution_admission.validation_passed\""));
        assert!(self_improve.contains("\"self_evolution_admission.validation.compiler.items\""));
        assert!(self_improve.contains("\"self_evolution_admission.validation.tests.passed\""));
        assert!(self_improve.contains("\"self_evolution_admission.validation.benchmarks.failed\""));
        assert!(
            self_improve
                .contains("\"self_evolution_admission.validation.experiments.validation_passed\"")
        );
        assert!(self_improve.contains("\"self_evolution_admission.git_write_allowed\""));

        let inspect = model_service_endpoint_info_json(23, "inspect");
        assert!(inspect.contains("\"endpoint\":\"/v1/inspect\""));
        assert!(inspect.contains(
            "\"supported_fields\":[\"gate\",\"state_gate\",\"business_gate\",\"business_cycle_gate\",\"model_service_gate\",\"trace_gate\",\"tenant_id\",\"workspace_id\",\"session_id\"]"
        ));

        let state = model_service_endpoint_info_json(22, "state");
        assert!(state.contains("\"endpoint\":\"/v1/state\""));
        assert!(state.contains("\"method\":\"GET\""));
        assert!(state.contains("\"supported_fields\":[]"));
        assert!(state.contains("\"state.genome_profiles.active_genome_id\""));
        assert!(state.contains("\"state.genome_profiles.journal_record_count\""));
        assert!(!state.contains("\"state.top_experiences.runtime_model\""));
        assert!(!state.contains("\"state.evolution_external_feedbacks\""));

        let inspect = model_service_endpoint_info_json(22, "inspect");
        assert!(inspect.contains("\"endpoint\":\"/v1/inspect\""));
        assert!(inspect.contains("\"method\":\"POST\""));
        assert_trace_gate_runtime_closed_loop_contract_fields(&inspect);
        assert!(inspect.contains("\"state.runtime_adapter_experiences\""));
        assert!(inspect.contains("\"state.runtime_kv_import_experiences\""));
        assert!(inspect.contains("\"state.runtime_kv_budget_pressure_max\""));
        assert!(inspect.contains("\"state.memory_vector_dimensions\""));
        assert!(inspect.contains("\"state.runtime_kv_vector_dimensions.dimensions\""));
        assert!(inspect.contains("\"state.top_memories.key\""));
        assert!(inspect.contains("\"state.top_runtime_kv_memories.last_score\""));
        assert!(inspect.contains("\"state.top_experiences.runtime_primary_lane\""));
        assert!(inspect.contains("\"state.top_experiences.runtime_hidden_size\""));
        assert!(inspect.contains("\"state.top_experiences.runtime_kv_influence\""));
        assert!(inspect.contains("\"state.top_experiences.route_attention_fraction\""));
        assert!(inspect.contains("\"state.top_experiences.runtime_hot_kv_precision_bits\""));
        assert!(inspect.contains("\"state.top_experiences.runtime_kv_budget_pressure\""));
        assert!(inspect.contains("\"state.top_experiences.runtime_kv_segment_yield\""));
        assert!(inspect.contains("\"state.top_experiences.live_memory_feedback_updates\""));
        assert!(inspect.contains("\"state.top_experiences.live_memory_feedback_reinforced\""));
        assert!(inspect.contains("\"state.top_experiences.live_memory_feedback_detail\""));
        assert!(inspect.contains("\"state.top_experiences.runtime_error_message_chars\""));
        assert!(inspect.contains("\"state.business_contract_missing_signals\""));
        assert!(inspect.contains("\"state.business_contract_canonical_fallbacks\""));
        assert!(inspect.contains("\"state.top_experiences.rust_check_diagnostic_chars\""));
        assert!(inspect.contains("\"state.top_experiences.business_contract_missing_signals\""));
        assert!(
            inspect.contains("\"state.top_experiences.business_contract_canonical_fallbacks\"")
        );
        assert!(inspect.contains("\"state.top_experiences.pool_dispatch_forwarded\""));
        assert!(inspect.contains("\"state.top_experiences.pool_dispatch_clamped\""));
        assert!(inspect.contains("\"state.reflection_issue_experiences\""));
        assert!(inspect.contains("\"state.live_memory_feedback_strength_delta\""));
        assert!(inspect.contains("\"state.profile_observations_coding\""));
        assert!(inspect.contains("\"state.profile_threshold_general\""));
        assert!(inspect.contains("\"state.profile_threshold_long_document\""));
        assert!(inspect.contains("\"state.hierarchy_global\""));
        assert!(inspect.contains("\"state.hierarchy_convolution\""));
        assert!(inspect.contains("\"state.profile_hierarchy_global_general\""));
        assert!(inspect.contains("\"state.profile_hierarchy_observations_coding\""));
        assert!(inspect.contains("\"state.profile_hierarchy_convolution_coding\""));
        assert!(inspect.contains("\"state.profile_hierarchy_global_long_document\""));
        assert!(inspect.contains("\"state.tier_warm_ram\""));
        assert!(inspect.contains("\"state.memory_retention_stale_after\""));
        assert!(inspect.contains("\"state.memory_compaction_max_merges\""));
        assert!(inspect.contains("\"state.experience_hygiene_findings\""));
        assert!(inspect.contains("\"state.experience_repair_projected_quarantine_candidates\""));
        assert!(inspect.contains("\"state.experience_index_quality_score\""));
        assert!(inspect.contains("\"state.experience_index_noisy_records\""));
        assert!(inspect.contains("\"state.experience_index_samples.compacted\""));
        assert!(inspect.contains("\"state.experience_index_retrieval_ready\""));
        assert!(inspect.contains("\"state.evolution_recursive_runtime_calls\""));
        assert!(inspect.contains("\"state_gate\""));
    }

    fn assert_trace_gate_runtime_closed_loop_contract_fields(json: &str) {
        for field in [
            "trace_gate.runtime_closed_loop_counters",
            "trace_gate.runtime_closed_loop_counters.compute_budget_saved_tokens",
            "trace_gate.runtime_closed_loop_counters.compute_budget_avoided_tokens",
            "trace_gate.runtime_closed_loop_counters.compute_budget_write_allowed",
            "trace_gate.runtime_closed_loop_counters.compute_budget_applied",
            "trace_gate.runtime_closed_loop_counters.memory_admission_ledger_authorized",
            "trace_gate.runtime_closed_loop_counters.memory_admission_ledger_applied",
            "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_write_allowed",
            "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_durable_write_allowed",
            "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_applied",
            "trace_gate.runtime_closed_loop_counters.self_evolving_memory_store_applied_to_disk",
            "trace_gate.runtime_closed_loop_counters.memory_residency_write_allowed",
            "trace_gate.runtime_closed_loop_counters.memory_residency_durable_write_allowed",
            "trace_gate.runtime_closed_loop_counters.memory_residency_applied",
            "trace_gate.runtime_closed_loop_counters.auto_replay_live_memory_feedback_applied",
            "trace_gate.runtime_closed_loop_counters.auto_replay_business_contract_passed",
            "trace_gate.runtime_closed_loop_counters.auto_replay_live_evolution_memory_updates",
            "trace_gate.runtime_closed_loop_counters.auto_replay_recursive_runtime_calls",
            "trace_gate.runtime_closed_loop_counters.auto_replay_runtime_kv_budget_pressure_items",
            "trace_gate.runtime_closed_loop_counters.kv_fusion_saved_tokens",
            "trace_gate.runtime_closed_loop_counters.self_evolution_rollback_replay_apply_ready",
            "trace_gate.runtime_closed_loop_counters.self_evolution_promotion_preflight_ready",
            "trace_gate.runtime_closed_loop_counters.self_evolution_operator_approval_held",
            "trace_gate.runtime_closed_loop_counters.reasoning_genome_mutation_applied",
        ] {
            assert!(json.contains(&format!("\"{field}\"")), "{json}");
        }
    }

    #[test]
    fn replay_endpoint_contract_declares_emitted_json_fields() {
        let emitted_fields = emitted_json_fields(
            include_str!("../../response/replay/replay_json.rs"),
            "replay.",
        );

        for field in emitted_fields {
            assert!(
                MODEL_SERVICE_REPLAY_RESPONSE_FIELDS.contains(&field.as_str()),
                "missing replay endpoint response field: {field}"
            );
        }
    }

    #[test]
    fn self_improve_endpoint_contract_declares_summary_json_fields() {
        let source = include_str!("../../response/replay/self_improve_json.rs");
        let emitted_fields = emitted_json_fields(
            function_source(source, "self_improve_summary_json"),
            "self_improve.",
        );
        let missing = emitted_fields
            .into_iter()
            .filter(|field| !MODEL_SERVICE_SELF_IMPROVE_RESPONSE_FIELDS.contains(&field.as_str()))
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "missing self-improve endpoint response fields: {missing:?}"
        );
    }

    #[test]
    fn state_endpoint_contract_declares_emitted_top_level_json_fields() {
        let source = include_str!("../../response/state.rs");
        let mut emitted_fields = emitted_json_fields(
            function_source(source, "model_service_state_response_json"),
            "",
        );
        emitted_fields.extend(
            [
                "model_service_state_json",
                "runtime_kv_state_fields_json",
                "memory_vector_dimension_fields_json",
                "top_memory_state_fields_json",
                "top_experience_state_fields_json",
                "reflection_feedback_state_fields_json",
                "profile_tier_state_fields_json",
                "memory_policy_state_fields_json",
                "adaptive_loop_state_fields_json",
                "evolution_ledger_detail_state_fields_json",
            ]
            .into_iter()
            .flat_map(|function| emitted_json_fields(function_source(source, function), "state.")),
        );

        emitted_fields.sort();
        emitted_fields.dedup();

        let missing = emitted_fields
            .into_iter()
            .filter(|field| !MODEL_SERVICE_INSPECT_RESPONSE_FIELDS.contains(&field.as_str()))
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "missing state endpoint response fields: {missing:?}"
        );
    }

    #[test]
    fn openai_endpoint_contract_declares_emitted_norion_metadata_fields() {
        let source = include_str!("../../response/generate.rs");
        let mut emitted_fields = [
            "request_id",
            "endpoint",
            "model",
            "profile",
            "cancelled",
            "timeout",
            "retryable",
            "runtime_error_note",
            "elapsed_ms",
            "output_mode",
            "quality",
            "experience_id",
            "memory_stored",
            "persistent_writes",
            "memory_write_allowed",
            "genome_write_allowed",
            "self_evolution_write_allowed",
        ]
        .into_iter()
        .map(|field| format!("norion.{field}"))
        .collect::<Vec<_>>();
        emitted_fields.extend(
            [
                "model_service_task_metadata_json",
                "model_service_route_budget_metadata_json",
                "openai_norion_runtime_metadata_json",
                "model_service_runtime_kv_metadata_json",
            ]
            .into_iter()
            .flat_map(|function| emitted_json_fields(function_source(source, function), "norion.")),
        );

        emitted_fields.sort();
        emitted_fields.dedup();

        let missing = emitted_fields
            .into_iter()
            .filter(|field| !OPENAI_RESPONSE_FIELDS.contains(&field.as_str()))
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "missing OpenAI norion response fields: {missing:?}"
        );
    }

    #[test]
    fn generate_endpoint_contract_declares_emitted_top_level_json_fields() {
        let source = include_str!("../../response/generate.rs");
        let mut emitted_fields =
            emitted_json_fields(function_source(source, "model_service_response_json"), "");
        emitted_fields.extend(
            [
                "model_service_task_metadata_json",
                "model_service_route_budget_metadata_json",
                "model_service_runtime_kv_metadata_json",
            ]
            .into_iter()
            .flat_map(|function| emitted_json_fields(function_source(source, function), "")),
        );

        emitted_fields.sort();
        emitted_fields.dedup();

        let response_fields = endpoint_response_fields("generate");
        let missing = emitted_fields
            .into_iter()
            .filter(|field| !response_fields.contains(&field.as_str()))
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "missing generate endpoint response fields: {missing:?}"
        );
    }

    #[test]
    fn business_cycle_endpoint_contract_declares_emitted_top_level_json_fields() {
        let source = include_str!("../../response/business_cycle.rs");
        let mut emitted_fields = emitted_json_fields(
            function_source(source, "model_service_business_cycle_response_json"),
            "",
        );
        emitted_fields.extend(emitted_json_fields(
            function_source(source, "append_eval_section"),
            "",
        ));

        emitted_fields.sort();
        emitted_fields.dedup();

        let response_fields = endpoint_response_fields("business-cycle");
        let missing = emitted_fields
            .into_iter()
            .filter(|field| !response_fields.contains(&field.as_str()))
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "missing business-cycle endpoint response fields: {missing:?}"
        );
    }

    fn function_source<'a>(source: &'a str, name: &str) -> &'a str {
        let start = source
            .find(&format!("fn {name}"))
            .or_else(|| source.find(&format!("pub(super) fn {name}")))
            .or_else(|| source.find(&format!("pub(crate) fn {name}")))
            .unwrap_or_else(|| panic!("missing function source: {name}"));
        let tail = &source[start + 1..];
        let end = ["\nfn ", "\r\npub(", "\npub(", "\n#[cfg("]
            .into_iter()
            .filter_map(|pattern| tail.find(pattern))
            .min()
            .map(|offset| start + 1 + offset)
            .unwrap_or(source.len());
        &source[start..end]
    }

    fn emitted_json_fields(source: &str, prefix: &str) -> Vec<String> {
        source
            .split("\\\"")
            .skip(1)
            .step_by(2)
            .filter(|field| !field.is_empty())
            .filter(|field| {
                field
                    .chars()
                    .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
            })
            .map(|field| format!("{prefix}{field}"))
            .collect()
    }

    #[test]
    fn endpoint_info_json_reports_chat_route_contract() {
        let json = model_service_endpoint_info_json(2, "chat");

        assert!(json.contains("\"endpoint\":\"/v1/chat\""));
        assert!(json.contains("\"supported_fields\":[\"messages\",\"profile\",\"case\",\"output\",\"max_tokens\",\"max_completion_tokens\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_chat_stream_route() {
        let json = model_service_endpoint_info_json(3, "chat-stream");

        assert!(json.contains("\"endpoint\":\"/v1/chat-stream\""));
        assert!(json.contains("\"messages\""));
        assert!(json.contains("\"manual-chat-stream\""));
        assert!(json.contains("\"supported_fields\":[\"messages\",\"profile\",\"case\",\"output\",\"max_tokens\",\"max_completion_tokens\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"event:final.task_mode\""));
        assert!(json.contains("\"event:final.compute_budget_summary\""));
        assert!(json.contains("\"event:final.compute_budget_saved_tokens\""));
        assert!(json.contains("\"event:final.compute_budget_applied\""));
        assert!(json.contains("\"event:final.memory_write_allowed\""));
    }

    #[test]
    fn endpoint_info_json_reports_openai_chat_completions_contract() {
        let json = model_service_endpoint_info_json(11, "chat-completions");

        assert!(json.contains("\"endpoint\":\"/v1/chat/completions\""));
        assert!(json.contains("\"model\":\"rust-norion-local\""));
        assert!(json.contains("\"stream\":true"));
        assert!(json.contains("\"supported_fields\":[\"model\",\"messages\",\"max_tokens\",\"max_completion_tokens\",\"n\",\"stream\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"norion.runtime_model\""));
        assert!(json.contains("\"norion.runtime_adapter\""));
        assert!(json.contains("\"norion.runtime_device\""));
        assert!(json.contains("\"norion.runtime_primary_lane\""));
        assert!(json.contains("\"norion.runtime_fallback_lane\""));
        assert!(json.contains("\"norion.runtime_memory_mode\""));
        assert!(json.contains("\"norion.runtime_forward_energy\""));
        assert!(json.contains("\"norion.runtime_hot_kv_precision_bits\""));
        assert!(json.contains("\"norion.runtime_cold_kv_precision_bits\""));
        assert!(json.contains("\"norion.runtime_uncertainty_signal\""));
        assert!(json.contains("\"norion.runtime_device_execution_source\""));
        assert!(json.contains("\"norion.language_mode\""));
        assert!(json.contains("\"norion.coding_language\""));
        assert!(json.contains("\"norion.task_mode\""));
        assert!(json.contains("\"norion.compute_budget_summary\""));
        assert!(json.contains("\"norion.compute_budget_saved_tokens\""));
        assert!(json.contains("\"norion.elapsed_ms\""));
        for field in [
            "error.message",
            "error.type",
            "error.param",
            "error.code",
            "norion.endpoint",
            "norion.model",
            "norion.cancelled",
            "norion.timeout",
            "norion.retryable",
            "norion.runtime_error_note",
            "norion.used_memory_count",
            "norion.stored_runtime_kv_memory_ids",
            "norion.route_threshold",
            "norion.route_attention_tokens",
            "norion.route_fast_tokens",
            "norion.route_attention_fraction",
            "norion.runtime_kv_influence",
            "norion.runtime_imported_kv_blocks",
            "norion.runtime_weak_kv_imports_skipped",
            "norion.runtime_budget_limited_kv_imports_skipped",
            "norion.runtime_kv_budget_pressure",
            "norion.runtime_exported_kv_blocks",
            "norion.runtime_kv_segments_included",
            "norion.runtime_kv_segments_skipped",
            "norion.runtime_kv_segments_rejected",
            "norion.runtime_kv_segment_yield",
            "norion.runtime_closed_loop_counters.noiron_orchestration_live_feedback_closed",
            "norion.runtime_closed_loop_counters.control_expression_ready",
            "norion.memory_write_allowed",
            "norion.genome_write_allowed",
            "norion.self_evolution_write_allowed",
        ] {
            assert!(json.contains(&format!("\"{field}\"")), "{json}");
        }
        assert!(json.contains("\"stream_response_fields\""));
        assert!(json.contains("\"data:chunk\""));
        assert!(json.contains("\"object:chat.completion.chunk\""));
        assert!(json.contains("\"norion.model\""));
        assert!(json.contains("\"norion.compute_budget\",\"norion.compute_budget_summary\",\"norion.compute_budget_saved_tokens\",\"norion.compute_budget_avoided_tokens\",\"norion.compute_budget_kv_lookups_skipped\",\"norion.compute_budget_fanout_reduction\",\"norion.compute_budget_read_only\",\"norion.compute_budget_write_allowed\",\"norion.compute_budget_applied\",\"norion.stream_state\""));
        assert!(json.contains("\"norion.stream_state\""));
        assert!(json.contains("\"norion.streamed_tokens\""));
        assert!(json.contains("\"norion.runtime_model\",\"norion.runtime_adapter\",\"norion.runtime_device\",\"norion.runtime_primary_lane\",\"norion.runtime_fallback_lane\",\"norion.runtime_memory_mode\",\"norion.runtime_forward_energy\",\"norion.runtime_hot_kv_precision_bits\",\"norion.runtime_cold_kv_precision_bits\",\"norion.runtime_token_count\",\"norion.runtime_entropy_count\",\"norion.runtime_logprob_count\",\"norion.runtime_uncertainty_token_count\",\"norion.runtime_uncertainty_signal\",\"norion.runtime_average_entropy\",\"norion.runtime_average_neg_logprob\",\"norion.runtime_uncertainty_perplexity\",\"norion.runtime_architecture_signal\",\"norion.runtime_kv_precision_signal\",\"norion.runtime_device_execution_source\""));
        assert!(json.contains("\"norion.used_memory_count\",\"norion.stored_runtime_kv_memory_ids\",\"norion.route_threshold\",\"norion.route_attention_tokens\",\"norion.route_fast_tokens\",\"norion.route_attention_fraction\""));
        assert!(json.contains("\"norion.retryable\""));
        assert!(json.contains("\"norion.runtime_error_note\""));
        assert!(json.contains("\"unsupported_fields\":[\"tools\",\"tool_choice\",\"response_format\",\"logprobs\",\"temperature\",\"top_p\",\"presence_penalty\",\"frequency_penalty\",\"stop\",\"seed\",\"logit_bias\",\"stream_options\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_openai_completions_contract() {
        let json = model_service_endpoint_info_json(12, "completions");

        assert!(json.contains("\"endpoint\":\"/v1/completions\""));
        assert!(json.contains("\"model\":\"rust-norion-local\""));
        assert!(json.contains("\"prompt\":\"用中文"));
        assert!(json.contains("\"supported_fields\":[\"model\",\"prompt\",\"max_tokens\",\"n\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"norion.runtime_model\""));
        assert!(json.contains("\"norion.runtime_uncertainty_signal\""));
        assert!(json.contains("\"norion.runtime_device_execution_source\""));
        assert!(json.contains("\"norion.language_mode\""));
        assert!(json.contains("\"norion.coding_language\""));
        assert!(json.contains("\"norion.task_mode\""));
        assert!(json.contains("\"norion.compute_budget_summary\""));
        assert!(json.contains("\"norion.compute_budget_saved_tokens\""));
        for field in [
            "error.message",
            "error.type",
            "error.param",
            "error.code",
            "norion.endpoint",
            "norion.model",
            "norion.cancelled",
            "norion.timeout",
            "norion.retryable",
            "norion.runtime_error_note",
            "norion.used_memory_count",
            "norion.stored_runtime_kv_memory_ids",
            "norion.route_threshold",
            "norion.route_attention_tokens",
            "norion.route_fast_tokens",
            "norion.route_attention_fraction",
            "norion.runtime_kv_influence",
            "norion.runtime_imported_kv_blocks",
            "norion.runtime_weak_kv_imports_skipped",
            "norion.runtime_budget_limited_kv_imports_skipped",
            "norion.runtime_kv_budget_pressure",
            "norion.runtime_exported_kv_blocks",
            "norion.runtime_kv_segments_included",
            "norion.runtime_kv_segments_skipped",
            "norion.runtime_kv_segments_rejected",
            "norion.runtime_kv_segment_yield",
            "norion.runtime_closed_loop_counters.noiron_orchestration_live_feedback_closed",
            "norion.runtime_closed_loop_counters.control_expression_ready",
            "norion.memory_write_allowed",
            "norion.genome_write_allowed",
            "norion.self_evolution_write_allowed",
        ] {
            assert!(json.contains(&format!("\"{field}\"")), "{json}");
        }
        assert!(!json.contains("\"stream_response_fields\""));
        assert!(json.contains("\"unsupported_fields\":[\"stream\",\"logprobs\",\"suffix\",\"temperature\",\"top_p\",\"presence_penalty\",\"frequency_penalty\",\"stop\",\"seed\",\"logit_bias\",\"stream_options\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_generate_contract_fields() {
        let json = model_service_endpoint_info_json(13, "generate");

        assert!(json.contains("\"endpoint\":\"/v1/generate\""));
        assert!(json.contains("\"supported_fields\":[\"prompt\",\"profile\",\"case\",\"output\",\"max_tokens\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        for field in [
            "elapsed_ms",
            "output_mode",
            "quality",
            "process_reward",
            "action",
            "memory_stored",
            "stored_memory_id",
            "used_memory_count",
            "route_threshold",
            "route_attention_tokens",
            "route_fast_tokens",
            "route_attention_fraction",
            "used_memory_ids",
            "stored_gist_memory_ids",
            "stored_runtime_kv_memory_ids",
            "feedback_memory_ids",
            "runtime_entropy_count",
            "runtime_logprob_count",
            "runtime_uncertainty_token_count",
            "runtime_average_entropy",
            "runtime_average_neg_logprob",
            "runtime_uncertainty_perplexity",
            "runtime_architecture_signal",
            "runtime_kv_precision_signal",
            "runtime_device_execution_source",
            "runtime_kv_influence",
            "runtime_imported_kv_blocks",
            "runtime_weak_kv_imports_skipped",
            "runtime_budget_limited_kv_imports_skipped",
            "runtime_kv_budget_pressure",
            "runtime_exported_kv_blocks",
            "runtime_kv_segments_included",
            "runtime_kv_segments_skipped",
            "runtime_kv_segments_rejected",
            "runtime_kv_segment_yield",
        ] {
            assert!(json.contains(&format!("\"{field}\"")), "{json}");
        }
        assert!(json.contains("\"unsupported_fields\":[\"messages\",\"stream\",\"tools\",\"tool_choice\",\"response_format\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_chat_error_contract_fields() {
        let json = model_service_endpoint_info_json(17, "chat");

        assert!(json.contains("\"endpoint\":\"/v1/chat\""));
        for field in [
            "endpoint",
            "error_type",
            "cancelled",
            "timeout",
            "retryable",
            "runtime_error_note",
            "used_memory_count",
            "route_threshold",
            "route_attention_tokens",
            "route_fast_tokens",
            "route_attention_fraction",
            "persistent_writes",
            "memory_write_allowed",
            "genome_write_allowed",
            "self_evolution_write_allowed",
        ] {
            assert!(json.contains(&format!("\"{field}\"")), "{json}");
        }
    }

    #[test]
    fn endpoint_info_json_reports_stream_contract_fields() {
        let json = model_service_endpoint_info_json(14, "generate-stream");

        assert!(json.contains("\"endpoint\":\"/v1/generate-stream\""));
        assert!(json.contains("\"supported_fields\":[\"prompt\",\"profile\",\"case\",\"output\",\"max_tokens\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"event:final.task_mode\""));
        assert!(json.contains("\"event:final.stream_state\""));
        assert!(json.contains("\"event:final.retryable\""));
        assert!(json.contains("\"event:final.runtime_error_note\""));
        assert!(json.contains("\"event:final.compute_budget_summary\""));
        assert!(json.contains("\"event:final.compute_budget_saved_tokens\""));
        assert!(json.contains("\"event:final.compute_budget_avoided_tokens\""));
        assert!(json.contains("\"event:final.compute_budget_kv_lookups_skipped\""));
        assert!(json.contains("\"event:final.compute_budget_fanout_reduction\""));
        assert!(json.contains("\"event:final.compute_budget_read_only\""));
        assert!(json.contains("\"event:final.compute_budget_write_allowed\""));
        assert!(json.contains("\"event:final.compute_budget_applied\""));
        assert!(json.contains("\"event:final.route_threshold\""));
        assert!(json.contains("\"event:final.route_attention_tokens\""));
        assert!(json.contains("\"event:final.route_fast_tokens\""));
        assert!(json.contains("\"event:final.route_attention_fraction\""));
        for field in [
            "event:final.elapsed_ms",
            "event:final.output_mode",
            "event:final.quality",
            "event:final.process_reward",
            "event:final.action",
            "event:final.memory_stored",
            "event:final.stored_memory_id",
            "event:final.used_memory_ids",
            "event:final.stored_gist_memory_ids",
            "event:final.stored_runtime_kv_memory_ids",
            "event:final.feedback_memory_ids",
            "event:final.experience_id",
            "event:final.runtime_model",
            "event:final.runtime_adapter",
            "event:final.runtime_device",
            "event:final.runtime_primary_lane",
            "event:final.runtime_fallback_lane",
            "event:final.runtime_memory_mode",
            "event:final.runtime_forward_energy",
            "event:final.runtime_hot_kv_precision_bits",
            "event:final.runtime_cold_kv_precision_bits",
            "event:final.runtime_entropy_count",
            "event:final.runtime_logprob_count",
            "event:final.runtime_uncertainty_token_count",
            "event:final.runtime_average_entropy",
            "event:final.runtime_average_neg_logprob",
            "event:final.runtime_uncertainty_perplexity",
            "event:final.runtime_architecture_signal",
            "event:final.runtime_kv_precision_signal",
            "event:final.runtime_device_execution_source",
            "event:final.runtime_kv_influence",
            "event:final.runtime_imported_kv_blocks",
            "event:final.runtime_weak_kv_imports_skipped",
            "event:final.runtime_budget_limited_kv_imports_skipped",
            "event:final.runtime_kv_budget_pressure",
            "event:final.runtime_exported_kv_blocks",
            "event:final.runtime_kv_segments_included",
            "event:final.runtime_kv_segments_skipped",
            "event:final.runtime_kv_segments_rejected",
            "event:final.runtime_kv_segment_yield",
            "event:final.traceable",
        ] {
            assert!(json.contains(&format!("\"{field}\"")), "{json}");
        }
        assert!(json.contains("\"event:final.memory_write_allowed\""));
        assert!(json.contains(
            "\"unsupported_fields\":[\"messages\",\"tools\",\"tool_choice\",\"response_format\"]"
        ));
    }

    #[test]
    fn endpoint_info_json_reports_experience_hygiene_quarantine_route() {
        let json = model_service_endpoint_info_json(4, "experience-hygiene-quarantine");

        assert!(json.contains("\"endpoint\":\"/v1/experience-hygiene/quarantine\""));
        assert!(json.contains("\"apply\":false"));
        assert!(json.contains("\"limit\":20"));
        assert!(json.contains(
            "\"supported_fields\":[\"apply\",\"limit\",\"backup_path\",\"quarantine_path\"]"
        ));
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"experience_file\",\"applied\",\"backup_file\",\"quarantine_file\",\"plan\",\"total_records\",\"retained_records\",\"quarantine_candidates\",\"candidate_ids\",\"listed_findings\",\"experience_id\",\"severity\",\"reason\",\"markers\",\"prompt_preview\",\"lesson_preview\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"prompt\",\"profile\",\"model\",\"messages\",\"stream\",\"max_tokens\",\"tools\",\"tool_choice\",\"response_format\",\"logprobs\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_experience_retrieval_route() {
        let json = model_service_endpoint_info_json(5, "experience-retrieval");

        assert!(json.contains("\"endpoint\":\"/v1/experience-retrieval\""));
        assert!(json.contains("\"prompt\""));
        assert!(json.contains("\"profile\":\"coding\""));
        assert!(
            json.contains(
                "\"supported_fields\":[\"prompt\",\"profile\",\"limit\",\"index_context\",\"tenant_id\",\"workspace_id\",\"session_id\"]"
            )
        );
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"retrieval\",\"prompt\",\"profile\",\"retrieval_elapsed_ms\",\"index_context_used\",\"index_context_chars\",\"total_records\",\"requested_limit\",\"matches\",\"match_count\",\"skipped_cross_task_pollution\",\"development_evidence_surface_blocked_candidates\",\"retrieval_noise_penalized_candidates\",\"retrieval_noise_filtered_candidates\",\"suppressed_prompt_index_candidates\",\"max_retrieval_noise_penalty\",\"max_score\",\"experience_id\",\"score\",\"quality\",\"process_reward\",\"reward_action\",\"used_memory_count\",\"stored_runtime_kv_memory_ids\",\"route_threshold\",\"route_attention_tokens\",\"route_fast_tokens\",\"route_attention_fraction\",\"prompt_preview\",\"lesson_preview\",\"usable_hint_preview\",\"gist_hints\",\"reflection_issue_codes\",\"revision_actions\",\"runtime_model\",\"runtime_adapter\",\"runtime_device\",\"runtime_primary_lane\",\"runtime_fallback_lane\",\"runtime_memory_mode\",\"runtime_device_execution_source\",\"runtime_forward_energy\",\"runtime_kv_influence\",\"runtime_uncertainty_perplexity\",\"recursive_runtime_calls\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"model\",\"messages\",\"stream\",\"max_tokens\",\"tools\",\"tool_choice\",\"response_format\",\"logprobs\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_experience_repair_route() {
        let json = model_service_endpoint_info_json(6, "experience-repair");

        assert!(json.contains("\"endpoint\":\"/v1/experience-repair\""));
        assert!(json.contains("\"apply\":false"));
        assert!(json.contains("\"limit\":20"));
        assert!(json.contains("\"supported_fields\":[\"apply\",\"limit\",\"backup_path\"]"));
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"experience_file\",\"applied\",\"backup_file\",\"plan\",\"total_records\",\"legacy_metadata_lessons\",\"repairable_legacy_metadata_lessons\",\"index_noisy_records\",\"index_duplicate_outputs\",\"repairable_index_records\",\"remaining_legacy_metadata_lessons_after_repair\",\"remaining_watch_after_repair\",\"remaining_quarantine_candidates_after_repair\",\"skipped_quarantine_candidates\",\"skipped_missing_clean_gist\",\"projected_hygiene_after_repair\",\"legacy_metadata_without_clean_gist\",\"index_quality_score\",\"index_retrieval_ready\",\"index_risk_level\",\"listed_repairs\",\"listed_skipped_quarantine_candidates\",\"listed_skipped_missing_clean_gist\",\"experience_id\",\"action\",\"source\",\"old_lesson_preview\",\"proposed_lesson_preview\",\"source_gist_preview\",\"reason\",\"prompt_preview\",\"gist_count\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"prompt\",\"profile\",\"model\",\"messages\",\"stream\",\"max_tokens\",\"tools\",\"tool_choice\",\"response_format\",\"logprobs\",\"quarantine_path\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_model_pool_route_plan() {
        let json = model_service_endpoint_info_json(9, "model-pool-route-plan");

        assert!(json.contains("\"endpoint\":\"/v1/model-pool/route-plan\""));
        assert!(json.contains("\"task_kind\":\"review\""));
        assert!(json.contains("\"completed_roles\":[\"quality\",\"router\"]"));
        assert!(json.contains("\"supported_fields\":[\"task_kind\",\"task\",\"max_tokens\",\"max\",\"prompt\",\"content\",\"completed_roles\",\"completed_stage_roles\"]"));
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"schema_version\",\"contract_version\",\"task_kind\",\"read_only\",\"launches_process\",\"sends_prompt\",\"route_allowed\",\"reason\",\"route_block_reason\",\"agent_model_route_source\",\"agent_model_route_source.route_allowed\",\"agent_model_route_source.proof_ready\",\"agent_model_route_source.proof_block_reason\",\"agent_model_route_source.selected_role\",\"agent_model_route_source.model_registry_id\",\"agent_model_route_source.model_profile_id\",\"agent_model_route_source.inference_backend_id\",\"agent_model_route_source.model_pool_id\",\"role_candidates\",\"routing_weights\",\"service_backpressure\",\"dependency_precheck\",\"quality_context_tokens\",\"quality_context_required_tokens\",\"quality_context_sufficient\",\"quality_block_reason\",\"selected_role\",\"selected_base_url\",\"selected_port\",\"selected_default_max_tokens\",\"selected_context_window\",\"selected_context_required_tokens\",\"selected_context_buffer_tokens\",\"selected_context_buffer_policy\",\"selected_context_sufficient\",\"selected_context_block_reason\",\"configured_max_tokens\",\"effective_max_tokens\",\"max_tokens_clamped\",\"max_tokens_clamp_reason\",\"compute_budget_summary\",\"compute_budget_configured_max_tokens\",\"compute_budget_effective_max_tokens\",\"compute_budget_saved_tokens\",\"compute_budget_avoided_tokens\",\"compute_budget_max_tokens_clamped\",\"runtime_closed_loop_counters\",\"runtime_closed_loop_counters.compute_budget_saved_tokens\",\"runtime_closed_loop_counters.compute_budget_avoided_tokens\",\"runtime_closed_loop_counters.compute_budget_max_tokens_clamped\",\"runtime_closed_loop_counters.model_pool_budget_applied\",\"pool_dispatch\",\"route_metrics\",\"route_metrics.success_rate_milli\",\"route_metrics.failure_rate_milli\",\"worker_metrics\",\"worker_metrics.success_rate_milli\",\"worker_metrics.failure_rate_milli\",\"candidate_workers\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"model\",\"messages\",\"stream\",\"tools\",\"tool_choice\",\"response_format\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_model_pool_call() {
        let json = model_service_endpoint_info_json(16, "model-pool-call");

        assert!(json.contains("\"endpoint\":\"/v1/model-pool/call\""));
        assert!(json.contains("\"task_kind\":\"summary\""));
        assert!(json.contains("\"prompt\":\"summarize this runtime trace\""));
        assert!(json.contains("\"supported_fields\":[\"task_kind\",\"task\",\"prompt\",\"content\",\"max_tokens\",\"max\",\"completed_roles\",\"completed_stage_roles\"]"));
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"schema_version\",\"contract_version\",\"task_kind\",\"read_only\",\"launches_process\",\"sends_prompt\",\"route_allowed\",\"reason\",\"route_block_reason\",\"role_candidates\",\"dependency_precheck\",\"quality_context_tokens\",\"quality_context_required_tokens\",\"quality_context_sufficient\",\"quality_block_reason\",\"selected_role\",\"selected_base_url\",\"selected_port\",\"selected_default_max_tokens\",\"configured_max_tokens\",\"effective_max_tokens\",\"max_tokens_clamped\",\"max_tokens_clamp_reason\",\"pool_dispatch\",\"route_metrics\",\"route_metrics.success_rate_milli\",\"route_metrics.failure_rate_milli\",\"worker_metrics\",\"worker_metrics.success_rate_milli\",\"worker_metrics.failure_rate_milli\",\"candidate_workers\",\"elapsed_ms\",\"answer_chars\",\"answer_bytes\",\"answer_approx_tokens\",\"answer\",\"endpoint\",\"call_state\",\"cancelled\",\"timeout\",\"partial_result\",\"partial_finalized\",\"queue_time_ms\",\"compute_budget_summary\",\"compute_budget_configured_max_tokens\",\"compute_budget_effective_max_tokens\",\"compute_budget_saved_tokens\",\"compute_budget_avoided_tokens\",\"compute_budget_max_tokens_clamped\",\"runtime_closed_loop_counters\",\"runtime_closed_loop_counters.compute_budget_saved_tokens\",\"runtime_closed_loop_counters.compute_budget_avoided_tokens\",\"runtime_closed_loop_counters.compute_budget_max_tokens_clamped\",\"runtime_closed_loop_counters.model_pool_budget_applied\",\"error\",\"retryable\",\"dispatch_attempted\",\"persistent_writes\",\"memory_write_allowed\",\"genome_write_allowed\",\"self_evolution_write_allowed\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"model\",\"messages\",\"stream\",\"tools\",\"tool_choice\",\"response_format\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_request_cancel_route() {
        let json = model_service_endpoint_info_json(10, "requests-cancel");

        assert!(json.contains("\"endpoint\":\"/v1/requests/cancel\""));
        assert!(json.contains("\"request_id\":42"));
        assert!(json.contains("operator_runtime_splice"));
        assert!(json.contains("\"supported_fields\":[\"request_id\",\"reason\",\"retag_label\"]"));
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"target_request_id\",\"target_active\",\"target_endpoint\",\"repair_factor_released\",\"repair_factor\",\"retag_applied\",\"retag_label\",\"reason\",\"cooperative_only\",\"persistent_writes\",\"next_step\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"prompt\",\"messages\",\"stream\",\"tools\",\"tool_choice\",\"response_format\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_experience_cleanup_audit_route() {
        let json = model_service_endpoint_info_json(8, "experience-cleanup-audit");

        assert!(json.contains("\"endpoint\":\"/v1/experience-cleanup-audit\""));
        assert!(json.contains("\"limit\":20"));
        assert!(json.contains("\"supported_fields\":[\"limit\"]"));
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"experience_file\",\"checked\",\"writes_experience_state\",\"sample_limit\",\"error\",\"report\",\"index_report\",\"quarantine_plan\",\"repair_plan\",\"next_step\",\"total_records\",\"findings\",\"watch\",\"quarantine_candidates\",\"legacy_metadata_lessons\",\"legacy_metadata_without_clean_gist\",\"clean\",\"listed_findings\",\"compacted_records\",\"overlong_records\",\"overlong_without_clean_gist\",\"max_record_chars\",\"noisy_records\",\"duplicate_outputs\",\"max_noise_penalty\",\"quality_score\",\"retrieval_ready\",\"risk_level\",\"recommended_action\",\"retained_records\",\"candidate_ids\",\"experience_id\",\"severity\",\"reason\",\"markers\",\"prompt_preview\",\"lesson_preview\",\"repairable_legacy_metadata_lessons\",\"repairable_index_records\",\"remaining_legacy_metadata_lessons_after_repair\",\"remaining_watch_after_repair\",\"remaining_quarantine_candidates_after_repair\",\"skipped_quarantine_candidates\",\"skipped_missing_clean_gist\",\"projected_hygiene_after_repair\",\"listed_repairs\",\"listed_skipped_quarantine_candidates\",\"listed_skipped_missing_clean_gist\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"apply\",\"backup_path\",\"quarantine_path\",\"prompt\",\"profile\",\"model\",\"messages\",\"stream\",\"max_tokens\",\"tools\",\"tool_choice\",\"response_format\",\"logprobs\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
    }

    #[test]
    fn endpoint_info_json_keeps_generate_fallback() {
        let json = model_service_endpoint_info_json(1, "unknown");

        assert!(json.contains("\"endpoint\":\"/v1/generate\""));
        assert!(json.contains("\"manual-generate\""));
    }

    #[test]
    fn endpoint_contracts_expose_dna_closed_loop_state() {
        let generate = model_service_endpoint_info_json(2, "generate");
        let openai = model_service_endpoint_info_json(3, "chat-completions");
        let state = model_service_endpoint_info_json(4, "state");

        assert!(generate.contains("\"dna_closed_loop.generation_after\""));
        assert!(generate.contains("\"dna_closed_loop.writer_gate_decision\""));
        assert!(generate.contains("\"dna_closed_loop.rollback_applied\""));
        assert!(openai.contains("\"norion.dna_closed_loop.mutation_applied\""));
        assert!(state.contains("\"state.genome_profiles.active_genome_id\""));
        assert!(state.contains("\"state.genome_profiles.journal_record_count\""));
    }

    #[test]
    fn model_capabilities_json_reports_openai_models_contract() {
        let args = Args::parse(vec![]);
        let json = model_service_model_capabilities_json(15, &args);

        assert!(json.contains("\"object\":\"list\""));
        assert!(json.contains("\"id\":\"rust-norion-local\""));
        assert!(json.contains("\"display_name\":\"北极星\""));
        assert!(json.contains("\"runtime_mode\":\"built-in\""));
        assert!(json.contains("\"/v1/models\""));
        assert!(json.contains("\"/v1/chat/completions\""));
        assert!(json.contains("\"streaming\":true"));
        assert!(json.contains("\"cancellation\":true"));
        assert!(json.contains("\"max_tokens\":true"));
        assert!(json.contains(
            "\"supported_request_fields\":[\"model\",\"messages\",\"prompt\",\"stream\",\"max_tokens\",\"max_completion_tokens\",\"n\""
        ));
        assert!(json.contains("\"multiple_choices\""));
        assert!(json.contains("\"sampling_controls\""));
        assert!(json.contains("\"stop_sequences\""));
        assert!(json.contains("\"stream_usage_chunks\""));
        assert!(json.contains("\"/v1/model-pool/route-plan\""));
        assert!(json.contains("\"/v1/model-pool/call\""));
        assert!(json.contains("\"/v1/business-cycle\""));
        assert!(json.contains("\"/v1/business-cycle-stream\""));
        assert!(json.contains("\"/v1/state\""));
        assert!(json.contains("\"/v1/inspect\""));
        assert!(json.contains("\"/v1/feedback\""));
        assert!(json.contains("\"/v1/rust-check\""));
        assert!(json.contains("\"/v1/replay\""));
        assert!(json.contains("\"/v1/self-improve\""));
        assert!(json.contains("\"/v1/experience-retrieval\""));
        assert!(json.contains("\"/v1/experience-hygiene/quarantine\""));
        assert!(json.contains("\"/v1/experience-cleanup-audit\""));
        assert!(json.contains("\"/v1/experience-repair\""));
        assert!(json.contains("\"state_inspection\":true"));
        assert!(json.contains("\"feedback\":true"));
        assert!(json.contains("\"rust_check\":true"));
        assert!(json.contains("\"experience_replay\":true"));
        assert!(json.contains("\"experience_retrieval\":true"));
        assert!(json.contains("\"experience_hygiene_quarantine\":true"));
        assert!(json.contains("\"experience_cleanup_audit\":true"));
        assert!(json.contains("\"experience_repair\":true"));
        assert!(json.contains("\"hierarchical_routing\":true"));
        assert!(json.contains("\"/v1/diagnostics\""));
        assert!(json.contains("\"diagnostics_endpoint\":\"/v1/diagnostics\""));
        assert!(json.contains("\"health_response_fields\":["));
        assert!(json.contains("\"diagnostics_response_fields\":["));
        for field in HEALTH_DIAGNOSTICS_RESPONSE_FIELDS {
            assert!(json.contains(&service_json_string(field)), "{json}");
        }
        assert!(json.contains("\"weight_retraining_required\":false"));
    }
}
