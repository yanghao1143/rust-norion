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
    let response_fields = endpoint_response_fields(endpoint);
    format!(
        "{{\"ok\":true,\"request_id\":{},\"endpoint\":\"{}\",\"method\":\"POST\",\"content_type\":\"application/json\",\"example\":{},\"supported_fields\":{},\"response_fields\":{},\"unsupported_fields\":{}}}",
        request_id,
        spec.path,
        spec.example,
        str_array_json(spec.supported_fields),
        str_array_json(response_fields),
        str_array_json(spec.unsupported_fields)
    )
}

fn model_service_model_capabilities_json(request_id: usize, args: &Args) -> String {
    format!(
        "{{\"object\":\"list\",\"data\":[{{\"id\":\"rust-norion-local\",\"object\":\"model\",\"created\":0,\"owned_by\":\"rust-norion\",\"root\":\"rust-norion-local\",\"parent\":null,\"norion\":{{\"runtime_mode\":\"{}\",\"supported_endpoints\":[\"/v1/chat/completions\",\"/v1/completions\",\"/v1/generate\",\"/v1/chat\",\"/v1/generate-stream\",\"/v1/chat-stream\",\"/v1/business-cycle\",\"/v1/business-cycle-stream\",\"/v1/experience-retrieval\",\"/v1/experience-hygiene/quarantine\",\"/v1/experience-cleanup-audit\",\"/v1/experience-repair\",\"/v1/model-pool/route-plan\",\"/v1/model-pool/call\",\"/v1/requests/cancel\",\"/v1/diagnostics\",\"/health\"],\"supported_request_fields\":[\"model\",\"messages\",\"prompt\",\"stream\",\"max_tokens\",\"tenant_id\",\"workspace_id\",\"session_id\"],\"unsupported_features\":[\"tools\",\"tool_choice\",\"response_format\",\"logprobs\"],\"capabilities\":{{\"chat\":true,\"completions\":true,\"streaming\":true,\"cancellation\":true,\"max_tokens\":true,\"diagnostics\":true,\"hierarchical_routing\":true,\"experience_retrieval\":true,\"experience_hygiene_quarantine\":true,\"experience_cleanup_audit\":true,\"experience_repair\":true,\"persistent_kv_memory\":true,\"self_improvement\":true,\"weight_retraining_required\":false}}}}}}],\"norion\":{{\"request_id\":{},\"default_model\":\"rust-norion-local\",\"diagnostics_endpoint\":\"/v1/diagnostics\",\"contracts_endpoint\":\"GET /v1/{{endpoint}}\"}}}}",
        model_service_runtime_mode(args),
        request_id
    )
}

fn model_service_runtime_mode(args: &Args) -> &'static str {
    if args.gemma_runtime_server.is_some() {
        "gemma-http"
    } else if args.gemma_12b_runtime {
        "gemma-command"
    } else {
        "built-in"
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
                    "stream",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &["tools", "tool_choice", "response_format"],
            },
            "completions" => Self {
                path: "/v1/completions",
                example: "{\"model\":\"rust-norion-local\",\"prompt\":\"用中文给一个 rust-norion 业务联调建议。\",\"max_tokens\":256}",
                supported_fields: &[
                    "model",
                    "prompt",
                    "max_tokens",
                    "tenant_id",
                    "workspace_id",
                    "session_id",
                ],
                unsupported_fields: &["stream", "logprobs", "suffix"],
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
                example: "{\"prompt\":\"帮我用rust输出一段for循环代码\",\"profile\":\"coding\",\"limit\":5}",
                supported_fields: &["prompt", "profile", "limit", "index_context"],
                unsupported_fields: &[
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

fn endpoint_response_fields(endpoint: &str) -> &'static [&'static str] {
    match endpoint {
        "chat-completions" => &[
            "id", "object", "created", "model", "choices", "usage", "norion", "error",
        ],
        "completions" => &[
            "id", "object", "created", "model", "choices", "usage", "norion", "error",
        ],
        "chat-stream" | "generate-stream" => &[
            "event:status",
            "event:delta",
            "event:final",
            "event:done",
            "event:error",
        ],
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
            "answer",
            "raw_answer",
            "enhanced_answer",
            "runtime_token_count",
            "runtime_uncertainty_signal",
            "traceable",
            "error",
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
            "eval",
            "error",
        ],
        "experience-retrieval" => &[
            "ok",
            "request_id",
            "retrieval",
            "prompt",
            "profile",
            "index_context_used",
            "index_context_chars",
            "total_records",
            "requested_limit",
            "matches",
            "match_count",
            "skipped_cross_task_pollution",
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
            "pool_dispatch",
            "route_metrics",
            "worker_metrics",
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
            "worker_metrics",
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
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"pool_dispatch\",\"pool_stage_dispatch\",\"business_cycle\",\"generate\",\"feedback\",\"rust_check\",\"self_improve\",\"replay\",\"state\",\"state_gate\",\"trace_gate\",\"eval\",\"error\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"model\",\"messages\",\"stream\",\"tools\",\"tool_choice\",\"response_format\",\"logprobs\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_chat_stream_route() {
        let json = model_service_endpoint_info_json(3, "chat-stream");

        assert!(json.contains("\"endpoint\":\"/v1/chat-stream\""));
        assert!(json.contains("\"messages\""));
        assert!(json.contains("\"manual-chat-stream\""));
    }

    #[test]
    fn endpoint_info_json_reports_openai_chat_completions_contract() {
        let json = model_service_endpoint_info_json(11, "chat-completions");

        assert!(json.contains("\"endpoint\":\"/v1/chat/completions\""));
        assert!(json.contains("\"model\":\"rust-norion-local\""));
        assert!(json.contains("\"stream\":true"));
        assert!(json.contains("\"supported_fields\":[\"model\",\"messages\",\"max_tokens\",\"stream\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"response_fields\":[\"id\",\"object\",\"created\",\"model\",\"choices\",\"usage\",\"norion\",\"error\"]"));
        assert!(
            json.contains("\"unsupported_fields\":[\"tools\",\"tool_choice\",\"response_format\"]")
        );
    }

    #[test]
    fn endpoint_info_json_reports_openai_completions_contract() {
        let json = model_service_endpoint_info_json(12, "completions");

        assert!(json.contains("\"endpoint\":\"/v1/completions\""));
        assert!(json.contains("\"model\":\"rust-norion-local\""));
        assert!(json.contains("\"prompt\":\"用中文"));
        assert!(json.contains("\"supported_fields\":[\"model\",\"prompt\",\"max_tokens\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"response_fields\":[\"id\",\"object\",\"created\",\"model\",\"choices\",\"usage\",\"norion\",\"error\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"stream\",\"logprobs\",\"suffix\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_generate_contract_fields() {
        let json = model_service_endpoint_info_json(13, "generate");

        assert!(json.contains("\"endpoint\":\"/v1/generate\""));
        assert!(json.contains("\"supported_fields\":[\"prompt\",\"profile\",\"case\",\"output\",\"max_tokens\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"profile\",\"answer\",\"raw_answer\",\"enhanced_answer\",\"runtime_token_count\",\"runtime_uncertainty_signal\",\"traceable\",\"error\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"messages\",\"stream\",\"tools\",\"tool_choice\",\"response_format\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_stream_contract_fields() {
        let json = model_service_endpoint_info_json(14, "generate-stream");

        assert!(json.contains("\"endpoint\":\"/v1/generate-stream\""));
        assert!(json.contains("\"supported_fields\":[\"prompt\",\"profile\",\"case\",\"output\",\"max_tokens\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
        assert!(json.contains("\"response_fields\":[\"event:status\",\"event:delta\",\"event:final\",\"event:done\",\"event:error\"]"));
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
                "\"supported_fields\":[\"prompt\",\"profile\",\"limit\",\"index_context\"]"
            )
        );
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"retrieval\",\"prompt\",\"profile\",\"index_context_used\",\"index_context_chars\",\"total_records\",\"requested_limit\",\"matches\",\"match_count\",\"skipped_cross_task_pollution\",\"retrieval_noise_penalized_candidates\",\"retrieval_noise_filtered_candidates\",\"suppressed_prompt_index_candidates\",\"max_retrieval_noise_penalty\",\"max_score\",\"experience_id\",\"score\",\"quality\",\"process_reward\",\"reward_action\",\"prompt_preview\",\"lesson_preview\",\"usable_hint_preview\",\"gist_hints\",\"reflection_issue_codes\",\"revision_actions\",\"runtime_model\",\"runtime_adapter\",\"runtime_device\",\"runtime_primary_lane\",\"runtime_fallback_lane\",\"runtime_memory_mode\",\"runtime_device_execution_source\",\"runtime_forward_energy\",\"runtime_kv_influence\",\"runtime_uncertainty_perplexity\",\"recursive_runtime_calls\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"model\",\"messages\",\"stream\",\"max_tokens\",\"tools\",\"tool_choice\",\"response_format\",\"logprobs\",\"tenant_id\",\"workspace_id\",\"session_id\"]"));
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
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"schema_version\",\"contract_version\",\"task_kind\",\"read_only\",\"launches_process\",\"sends_prompt\",\"route_allowed\",\"reason\",\"route_block_reason\",\"role_candidates\",\"routing_weights\",\"service_backpressure\",\"dependency_precheck\",\"quality_context_tokens\",\"quality_context_required_tokens\",\"quality_context_sufficient\",\"quality_block_reason\",\"selected_role\",\"selected_base_url\",\"selected_port\",\"selected_default_max_tokens\",\"selected_context_window\",\"selected_context_required_tokens\",\"selected_context_buffer_tokens\",\"selected_context_buffer_policy\",\"selected_context_sufficient\",\"selected_context_block_reason\",\"configured_max_tokens\",\"effective_max_tokens\",\"max_tokens_clamped\",\"max_tokens_clamp_reason\",\"pool_dispatch\",\"route_metrics\",\"worker_metrics\",\"candidate_workers\"]"));
        assert!(json.contains("\"unsupported_fields\":[\"model\",\"messages\",\"stream\",\"tools\",\"tool_choice\",\"response_format\"]"));
    }

    #[test]
    fn endpoint_info_json_reports_model_pool_call() {
        let json = model_service_endpoint_info_json(16, "model-pool-call");

        assert!(json.contains("\"endpoint\":\"/v1/model-pool/call\""));
        assert!(json.contains("\"task_kind\":\"summary\""));
        assert!(json.contains("\"prompt\":\"summarize this runtime trace\""));
        assert!(json.contains("\"supported_fields\":[\"task_kind\",\"task\",\"prompt\",\"content\",\"max_tokens\",\"max\",\"completed_roles\",\"completed_stage_roles\"]"));
        assert!(json.contains("\"response_fields\":[\"ok\",\"request_id\",\"schema_version\",\"contract_version\",\"task_kind\",\"read_only\",\"launches_process\",\"sends_prompt\",\"route_allowed\",\"reason\",\"route_block_reason\",\"role_candidates\",\"dependency_precheck\",\"quality_context_tokens\",\"quality_context_required_tokens\",\"quality_context_sufficient\",\"quality_block_reason\",\"selected_role\",\"selected_base_url\",\"selected_port\",\"selected_default_max_tokens\",\"configured_max_tokens\",\"effective_max_tokens\",\"max_tokens_clamped\",\"max_tokens_clamp_reason\",\"pool_dispatch\",\"route_metrics\",\"worker_metrics\",\"candidate_workers\",\"elapsed_ms\",\"answer_chars\",\"answer_bytes\",\"answer_approx_tokens\",\"answer\",\"endpoint\",\"call_state\",\"cancelled\",\"timeout\",\"partial_result\",\"partial_finalized\",\"queue_time_ms\",\"compute_budget_summary\",\"error\",\"retryable\",\"dispatch_attempted\",\"persistent_writes\",\"memory_write_allowed\",\"genome_write_allowed\",\"self_evolution_write_allowed\"]"));
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
    fn model_capabilities_json_reports_openai_models_contract() {
        let args = Args::parse(vec![]);
        let json = model_service_model_capabilities_json(15, &args);

        assert!(json.contains("\"object\":\"list\""));
        assert!(json.contains("\"id\":\"rust-norion-local\""));
        assert!(json.contains("\"runtime_mode\":\"built-in\""));
        assert!(json.contains("\"/v1/chat/completions\""));
        assert!(json.contains("\"streaming\":true"));
        assert!(json.contains("\"cancellation\":true"));
        assert!(json.contains("\"max_tokens\":true"));
        assert!(json.contains("\"/v1/model-pool/route-plan\""));
        assert!(json.contains("\"/v1/model-pool/call\""));
        assert!(json.contains("\"/v1/business-cycle\""));
        assert!(json.contains("\"/v1/business-cycle-stream\""));
        assert!(json.contains("\"/v1/experience-retrieval\""));
        assert!(json.contains("\"/v1/experience-hygiene/quarantine\""));
        assert!(json.contains("\"/v1/experience-cleanup-audit\""));
        assert!(json.contains("\"/v1/experience-repair\""));
        assert!(json.contains("\"experience_retrieval\":true"));
        assert!(json.contains("\"experience_hygiene_quarantine\":true"));
        assert!(json.contains("\"experience_cleanup_audit\":true"));
        assert!(json.contains("\"experience_repair\":true"));
        assert!(json.contains("\"hierarchical_routing\":true"));
        assert!(json.contains("\"/v1/diagnostics\""));
        assert!(json.contains("\"diagnostics_endpoint\":\"/v1/diagnostics\""));
        assert!(json.contains("\"weight_retraining_required\":false"));
    }
}
