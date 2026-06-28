use std::fs;
use std::path::PathBuf;

use crate::Args;
use crate::model_service::json::{
    json_bool_field, json_string_field, json_u64_field, json_usize_field,
};

const MODEL_POOL_MANIFEST_ENV: [&str; 2] = [
    "SMARTSTEAM_MODEL_POOL_MANIFEST",
    "NORION_MODEL_POOL_MANIFEST",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkerSpec {
    pub(super) role: String,
    pub(super) port: u16,
    pub(super) base_url: String,
    pub(super) enabled_by_default: bool,
    pub(super) model_class: String,
    pub(super) suggested_quant: String,
    pub(super) default_context_tokens: usize,
    pub(super) default_max_tokens: usize,
    pub(super) low_priority: bool,
    pub(super) runtime_backend: Option<String>,
    pub(super) runtime_device: Option<String>,
    pub(super) runtime_accelerator: Option<String>,
    pub(super) gpu_layers: Option<usize>,
}

pub(super) fn worker_specs(args: &Args) -> std::io::Result<Vec<WorkerSpec>> {
    if let Some(path) = model_pool_manifest_path(args) {
        return load_worker_specs_from_manifest(path);
    }

    Ok(default_worker_specs(args))
}

fn model_pool_manifest_path(args: &Args) -> Option<PathBuf> {
    args.model_pool_manifest_path.clone().or_else(|| {
        args.serve.then(|| {
            MODEL_POOL_MANIFEST_ENV
                .iter()
                .find_map(|name| std::env::var(name).ok())
                .map(PathBuf::from)
        })?
    })
}

fn load_worker_specs_from_manifest(path: PathBuf) -> std::io::Result<Vec<WorkerSpec>> {
    let body = fs::read_to_string(&path)?;
    parse_worker_specs_manifest(&body).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("model pool manifest {} invalid: {error}", path.display()),
        )
    })
}

fn default_worker_specs(args: &Args) -> Vec<WorkerSpec> {
    let quality_base_url = args
        .gemma_runtime_server
        .as_deref()
        .map(normalize_base_url)
        .unwrap_or_else(|| "http://127.0.0.1:8686".to_owned());
    let quality_port = parse_port(&quality_base_url).unwrap_or(8686);
    vec![
        WorkerSpec {
            role: "quality".to_owned(),
            port: quality_port,
            base_url: quality_base_url,
            enabled_by_default: true,
            model_class: "Gemma 12B Q8 or best available local quality model".to_owned(),
            suggested_quant: "Q8 or best available quality quant".to_owned(),
            default_context_tokens: 262_144,
            default_max_tokens: 262_144,
            low_priority: false,
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
        },
        WorkerSpec {
            role: "summary".to_owned(),
            port: 8687,
            base_url: "http://127.0.0.1:8687".to_owned(),
            enabled_by_default: true,
            model_class: "small Gemma or low-quant local model".to_owned(),
            suggested_quant: "Q4 or Q5".to_owned(),
            default_context_tokens: 8192,
            default_max_tokens: 768,
            low_priority: true,
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
        },
        WorkerSpec {
            role: "review".to_owned(),
            port: 8688,
            base_url: "http://127.0.0.1:8688".to_owned(),
            enabled_by_default: true,
            model_class: "small Gemma or low-quant local model".to_owned(),
            suggested_quant: "Q4 or Q5".to_owned(),
            default_context_tokens: 8192,
            default_max_tokens: 1536,
            low_priority: true,
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
        },
        WorkerSpec {
            role: "test-gate".to_owned(),
            port: 8688,
            base_url: "http://127.0.0.1:8688".to_owned(),
            enabled_by_default: true,
            model_class: "small Gemma or low-quant local model".to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: 4096,
            default_max_tokens: 1536,
            low_priority: true,
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
        },
        WorkerSpec {
            role: "index".to_owned(),
            port: 8690,
            base_url: "http://127.0.0.1:8690".to_owned(),
            enabled_by_default: true,
            model_class: "small Gemma, embedding-capable helper, or low-quant local index model"
                .to_owned(),
            suggested_quant: "Q4".to_owned(),
            default_context_tokens: 4096,
            default_max_tokens: 512,
            low_priority: true,
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
        },
    ]
}

fn parse_worker_specs_manifest(body: &str) -> Result<Vec<WorkerSpec>, String> {
    let workers_body =
        json_array_field(body, "workers").ok_or_else(|| "requires workers array".to_owned())?;
    let workers = json_object_items(workers_body)
        .iter()
        .map(|worker| parse_worker_spec(worker))
        .collect::<Result<Vec<_>, _>>()?;
    if workers.is_empty() {
        return Err("requires at least one worker".to_owned());
    }
    if !workers.iter().any(|worker| worker.role == "quality") {
        return Err("requires a quality worker".to_owned());
    }
    Ok(workers)
}

fn parse_worker_spec(body: &str) -> Result<WorkerSpec, String> {
    let role = json_string_field(body, "role")
        .map(|role| role.trim().to_ascii_lowercase())
        .filter(|role| is_valid_role(role))
        .ok_or_else(|| "worker requires role [a-z0-9_-]+".to_owned())?;
    validate_worker_lifecycle(body, &role)?;
    let base_url = json_string_field(body, "base_url")
        .or_else(|| json_string_field(body, "url"))
        .map(|base_url| normalize_base_url(&base_url))
        .filter(|base_url| !base_url.trim().is_empty())
        .ok_or_else(|| format!("worker {role} requires base_url"))?;
    let port = json_u64_field(body, "port")
        .and_then(|port| u16::try_from(port).ok())
        .or_else(|| parse_port(&base_url))
        .ok_or_else(|| format!("worker {role} requires port or port-bearing base_url"))?;
    let low_priority = json_bool_field(body, "low_priority").unwrap_or(role != "quality");
    Ok(WorkerSpec {
        role,
        port,
        base_url,
        enabled_by_default: json_bool_field(body, "enabled_by_default")
            .or_else(|| json_bool_field(body, "enabled"))
            .unwrap_or(true),
        model_class: json_string_field(body, "model_class")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "configured model worker".to_owned()),
        suggested_quant: json_string_field(body, "suggested_quant")
            .or_else(|| json_string_field(body, "quant"))
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "configured quant".to_owned()),
        default_context_tokens: json_usize_field(body, "default_context_tokens")
            .or_else(|| json_usize_field(body, "context_window"))
            .unwrap_or(8192)
            .max(1),
        default_max_tokens: json_usize_field(body, "default_max_tokens")
            .or_else(|| json_usize_field(body, "max_tokens"))
            .unwrap_or(512)
            .max(1),
        low_priority,
        runtime_backend: optional_non_empty_string(body, &["runtime_backend", "backend", "engine"]),
        runtime_device: optional_non_empty_string(
            body,
            &[
                "runtime_device",
                "device",
                "device_profile",
                "execution_device",
            ],
        ),
        runtime_accelerator: optional_non_empty_string(
            body,
            &["runtime_accelerator", "accelerator", "device_accelerator"],
        ),
        gpu_layers: json_usize_field(body, "gpu_layers")
            .or_else(|| json_usize_field(body, "n_gpu_layers"))
            .or_else(|| json_usize_field(body, "offloaded_gpu_layers")),
    })
}

fn validate_worker_lifecycle(body: &str, role: &str) -> Result<(), String> {
    let state = worker_lifecycle_state(body)?;
    if state == "active" {
        return Ok(());
    }

    let mut missing = Vec::new();
    for field in [
        "reason_code",
        "source_digest",
        "parent_lineage",
        "rollback_anchor",
        "affected_scope",
        "readmission_gate",
    ] {
        if optional_non_empty_string(body, &[field]).is_none() {
            missing.push(field);
        }
    }
    match json_bool_field(body, "operator_approval_required") {
        Some(true) => {}
        Some(false) => missing.push("operator_approval_required=true"),
        None => missing.push("operator_approval_required"),
    }

    if !missing.is_empty() {
        return Err(format!(
            "worker {role} lifecycle {} missing evidence: {}",
            state,
            missing.join(",")
        ));
    }

    Err(format!(
        "worker {role} lifecycle {} blocks normal model-pool startup: reason_code={} source_digest={} parent_lineage={} rollback_anchor={} affected_scope={} readmission_gate={} operator_approval_required=true",
        state,
        optional_non_empty_string(body, &["reason_code"]).unwrap_or_default(),
        optional_non_empty_string(body, &["source_digest"]).unwrap_or_default(),
        optional_non_empty_string(body, &["parent_lineage"]).unwrap_or_default(),
        optional_non_empty_string(body, &["rollback_anchor"]).unwrap_or_default(),
        optional_non_empty_string(body, &["affected_scope"]).unwrap_or_default(),
        optional_non_empty_string(body, &["readmission_gate"]).unwrap_or_default()
    ))
}

fn worker_lifecycle_state(body: &str) -> Result<String, String> {
    let Some(state) = optional_non_empty_string(
        body,
        &[
            "lifecycle_state",
            "lifecycle",
            "runtime_lifecycle_state",
            "state",
        ],
    ) else {
        return Ok("active".to_owned());
    };

    match state.trim().to_ascii_lowercase().as_str() {
        "active" | "suspect" | "quarantined" | "retired_blocked" | "tombstone_preview"
        | "recycle_candidate" | "repaired_candidate" | "rejected_final" => {
            Ok(state.trim().to_ascii_lowercase())
        }
        _ => Err(format!("worker lifecycle_state {state} is unsupported")),
    }
}

fn optional_non_empty_string(body: &str, fields: &[&str]) -> Option<String> {
    fields
        .iter()
        .filter_map(|field| json_string_field(body, field))
        .find(|value| !value.trim().is_empty())
}

fn is_valid_role(role: &str) -> bool {
    !role.is_empty()
        && role.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '-'
                || character == '_'
        })
}

pub(super) fn normalize_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_owned()
    } else {
        format!("http://{trimmed}")
    }
}

pub(super) fn parse_port(base_url: &str) -> Option<u16> {
    let normalized = normalize_base_url(base_url);
    let without_scheme = normalized
        .strip_prefix("http://")
        .or_else(|| normalized.strip_prefix("https://"))
        .unwrap_or(&normalized);
    let authority = without_scheme
        .split_once('/')
        .map(|(authority, _)| authority)
        .unwrap_or(without_scheme);
    authority.rsplit_once(':')?.1.parse::<u16>().ok()
}

fn json_array_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let needle = format!("\"{field}\"");
    let after_field = body.get(body.find(&needle)? + needle.len()..)?;
    let after_colon = after_field.get(after_field.find(':')? + 1..)?;
    let trimmed = after_colon.trim_start();
    let close = find_matching_json_close(trimmed, '[', ']')?;
    trimmed.get(1..close)
}

fn json_object_items(input: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth = depth.saturating_add(1);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0
                    && let Some(start_index) = start.take()
                    && let Some(item) = input.get(start_index..=index)
                {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    items
}

fn find_matching_json_close(input: &str, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            value if value == open => depth = depth.saturating_add(1),
            value if value == close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Args;

    #[test]
    fn default_quality_worker_uses_configured_runtime_server() {
        let args = Args::parse(vec![
            "--gemma-runtime-server".to_owned(),
            "http://127.0.0.1:9999".to_owned(),
        ]);
        let specs = worker_specs(&args).unwrap();

        assert_eq!(specs[0].role, "quality");
        assert_eq!(specs[0].base_url, "http://127.0.0.1:9999");
        assert_eq!(specs[0].port, 9999);
    }

    #[test]
    fn default_quality_worker_uses_stable_pool_port() {
        let args = Args::parse(vec![]);
        let specs = worker_specs(&args).unwrap();

        assert_eq!(specs[0].base_url, "http://127.0.0.1:8686");
        assert_eq!(specs[0].port, 8686);
    }

    #[test]
    fn default_pool_exposes_index_helper_on_8690() {
        let args = Args::parse(vec![]);
        let specs = worker_specs(&args).unwrap();
        let index = specs
            .iter()
            .find(|worker| worker.role == "index")
            .expect("default model pool should include index helper");

        assert_eq!(index.port, 8690);
        assert_eq!(index.base_url, "http://127.0.0.1:8690");
        assert!(index.enabled_by_default);
        assert!(index.low_priority);
        assert_eq!(index.default_context_tokens, 4096);
        assert_eq!(index.default_max_tokens, 512);
    }

    #[test]
    fn default_pool_uses_four_helpers_without_router_worker() {
        let args = Args::parse(vec![]);
        let specs = worker_specs(&args).unwrap();
        let test_gate = specs
            .iter()
            .find(|worker| worker.role == "test-gate")
            .expect("default model pool should include test-gate helper");

        assert!(specs.iter().all(|worker| worker.role != "router"));
        assert_eq!(test_gate.port, 8688);
        assert_eq!(test_gate.base_url, "http://127.0.0.1:8688");
        assert_eq!(test_gate.default_max_tokens, 1536);
    }

    #[test]
    fn parses_manifest_workers_with_per_role_endpoint() {
        let specs = parse_worker_specs_manifest(
            r#"{
                "workers": [
                    {"role":"quality","base_url":"192.168.10.11:8686","model_class":"Gemma 12B","suggested_quant":"Q8","default_context_tokens":262144,"default_max_tokens":262144,"low_priority":false,"runtime_backend":"llama.cpp","runtime_device":"metal","runtime_accelerator":"metal","gpu_layers":999},
                    {"role":"review","base_url":"http://192.168.10.11:8688","context_window":8192,"max_tokens":1024,"backend":"llama.cpp","device":"metal","accelerator":"metal","n_gpu_layers":80}
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].base_url, "http://192.168.10.11:8686");
        assert_eq!(specs[0].port, 8686);
        assert!(!specs[0].low_priority);
        assert_eq!(specs[0].runtime_backend.as_deref(), Some("llama.cpp"));
        assert_eq!(specs[0].runtime_device.as_deref(), Some("metal"));
        assert_eq!(specs[0].runtime_accelerator.as_deref(), Some("metal"));
        assert_eq!(specs[0].gpu_layers, Some(999));
        assert_eq!(specs[1].role, "review");
        assert_eq!(specs[1].default_context_tokens, 8192);
        assert_eq!(specs[1].default_max_tokens, 1024);
        assert!(specs[1].low_priority);
        assert_eq!(specs[1].runtime_backend.as_deref(), Some("llama.cpp"));
        assert_eq!(specs[1].runtime_device.as_deref(), Some("metal"));
        assert_eq!(specs[1].runtime_accelerator.as_deref(), Some("metal"));
        assert_eq!(specs[1].gpu_layers, Some(80));
    }

    #[test]
    fn parses_versioned_gemma_chain_manifest() {
        let specs = parse_worker_specs_manifest(
            r#"{
                "schema_version": 1,
                "contract_version": "gemma-chain.v1",
                "read_only": true,
                "sends_prompt": false,
                "launches_process": false,
                "manifest_kind": "rust-norion.model-pool",
                "workers": [
                    {"role":"quality","port":8686,"base_url":"http://127.0.0.1:8686","enabled_by_default":true,"model_class":"Gemma 12B Q8","suggested_quant":"Q8","default_context_tokens":262144,"default_max_tokens":262144,"low_priority":false},
                    {"role":"review","port":8688,"base_url":"http://127.0.0.1:8688","enabled_by_default":true,"model_class":"small Gemma","suggested_quant":"Q4 or Q5","default_context_tokens":8192,"default_max_tokens":1024,"low_priority":true}
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].role, "quality");
        assert_eq!(specs[0].default_context_tokens, 262_144);
        assert_eq!(specs[0].default_max_tokens, 262_144);
        assert!(!specs[0].low_priority);
        assert_eq!(specs[1].role, "review");
        assert_eq!(specs[1].default_max_tokens, 1024);
        assert!(specs[1].low_priority);
    }

    #[test]
    fn manifest_accepts_active_worker_lifecycle_marker() {
        let specs = parse_worker_specs_manifest(
            r#"{
                "workers": [
                    {"role":"quality","base_url":"127.0.0.1:8686","lifecycle_state":"active"}
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].role, "quality");
    }

    #[test]
    fn manifest_rejects_retired_worker_before_normal_startup() {
        let error = parse_worker_specs_manifest(
            r#"{
                "workers": [
                    {
                        "role":"quality",
                        "base_url":"127.0.0.1:8686",
                        "lifecycle_state":"retired_blocked",
                        "reason_code":"retired_model_cell",
                        "source_digest":"sha256:retired-quality",
                        "parent_lineage":"lineage:model-pool:quality:v1",
                        "rollback_anchor":"rollback:model-pool:quality",
                        "affected_scope":"model_pool_worker:quality",
                        "readmission_gate":"hold_until_verifier_and_operator_approval",
                        "operator_approval_required":true
                    }
                ]
            }"#,
        )
        .unwrap_err();

        assert!(error.contains("worker quality lifecycle retired_blocked"));
        assert!(error.contains("blocks normal model-pool startup"));
        assert!(error.contains("reason_code=retired_model_cell"));
        assert!(error.contains("source_digest=sha256:retired-quality"));
        assert!(error.contains("rollback_anchor=rollback:model-pool:quality"));
    }

    #[test]
    fn manifest_rejects_repair_candidate_without_required_lifecycle_evidence() {
        let error = parse_worker_specs_manifest(
            r#"{
                "workers": [
                    {
                        "role":"quality",
                        "base_url":"127.0.0.1:8686",
                        "lifecycle_state":"repaired_candidate",
                        "reason_code":"repair_candidate"
                    }
                ]
            }"#,
        )
        .unwrap_err();

        assert!(error.contains("worker quality lifecycle repaired_candidate"));
        assert!(error.contains("missing evidence"));
        assert!(error.contains("source_digest"));
        assert!(error.contains("rollback_anchor"));
        assert!(error.contains("operator_approval_required"));
    }

    #[test]
    fn manifest_rejects_quarantined_polluted_worker() {
        let error = parse_worker_specs_manifest(
            r#"{
                "workers": [
                    {
                        "role":"quality",
                        "base_url":"127.0.0.1:8686",
                        "lifecycle":"quarantined",
                        "reason_code":"polluted_runtime_source",
                        "source_digest":"sha256:polluted-source",
                        "parent_lineage":"lineage:model-pool:quality:v1",
                        "rollback_anchor":"rollback:model-pool:quality",
                        "affected_scope":"model_pool_worker:quality",
                        "readmission_gate":"hold_until_verifier_and_operator_approval",
                        "operator_approval_required":true
                    }
                ]
            }"#,
        )
        .unwrap_err();

        assert!(error.contains("worker quality lifecycle quarantined"));
        assert!(error.contains("reason_code=polluted_runtime_source"));
        assert!(error.contains("blocks normal model-pool startup"));
    }

    #[test]
    fn manifest_requires_quality_worker() {
        let error = parse_worker_specs_manifest(
            r#"{"workers":[{"role":"review","base_url":"127.0.0.1:8688"}]}"#,
        )
        .unwrap_err();

        assert!(error.contains("quality"));
    }
}
