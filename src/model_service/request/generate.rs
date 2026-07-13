use rust_norion::{TaskProfile, TenantScope};

use super::super::json::{json_bool_field, json_string_field, json_usize_field};
use super::output::ModelServiceOutputMode;
use super::scope::require_tenant_scope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceRequest {
    pub(crate) prompt: String,
    pub(crate) profile: Option<TaskProfile>,
    pub(crate) case_name: Option<String>,
    pub(crate) output_mode: ModelServiceOutputMode,
    pub(crate) max_tokens: Option<usize>,
    pub(crate) tenant_scope: Option<TenantScope>,
    pub(crate) evolution_preview: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceOpenAiCompletionRequest {
    pub(crate) model: Option<String>,
    pub(crate) generate: ModelServiceRequest,
}

pub(super) fn parse_generate_request(body: &str) -> Result<ModelServiceRequest, String> {
    let prompt = json_string_field(body, "prompt")
        .filter(|prompt| !prompt.trim().is_empty())
        .ok_or_else(|| "JSON body must include a non-empty prompt string".to_owned())?;
    let profile = json_string_field(body, "profile")
        .map(|value| value.parse::<TaskProfile>())
        .transpose()
        .map_err(|error| error.to_string())?;
    let case_name = json_string_field(body, "case").filter(|case| !case.trim().is_empty());
    let output_mode = ModelServiceOutputMode::parse_from_body(body)?;
    let max_tokens = json_usize_field(body, "max_tokens")
        .or_else(|| json_usize_field(body, "max"))
        .map(|value| value.max(1));
    let tenant_scope = require_tenant_scope(body)?;
    let evolution_preview = json_bool_field(body, "norion_evolution_preview").unwrap_or(false);

    Ok(ModelServiceRequest {
        prompt,
        profile,
        case_name,
        output_mode,
        max_tokens,
        tenant_scope: Some(tenant_scope),
        evolution_preview,
    })
}

pub(super) fn parse_openai_completion_request(
    body: &str,
) -> Result<ModelServiceOpenAiCompletionRequest, String> {
    Ok(ModelServiceOpenAiCompletionRequest {
        model: json_string_field(body, "model").filter(|model| !model.trim().is_empty()),
        generate: parse_generate_request(body)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_request_parses_tenant_scope() {
        let request = parse_generate_request(
            "{\"prompt\":\"hi\",\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"session\"}",
        )
        .unwrap();

        assert_eq!(
            request.tenant_scope,
            Some(TenantScope::new("tenant-a", "workspace", "session"))
        );
        assert!(!request.evolution_preview);
    }

    #[test]
    fn generate_request_accepts_read_only_evolution_preview() {
        let request = parse_generate_request(
            "{\"prompt\":\"hi\",\"norion_evolution_preview\":true,\"tenant_id\":\"tenant-a\",\"workspace_id\":\"workspace\",\"session_id\":\"session\"}",
        )
        .unwrap();

        assert!(request.evolution_preview);
    }

    #[test]
    fn generate_request_rejects_missing_tenant_scope() {
        assert_eq!(
            parse_generate_request("{\"prompt\":\"hi\"}").unwrap_err(),
            "tenant scope requires tenant_id, workspace_id, and session_id"
        );
    }
}
