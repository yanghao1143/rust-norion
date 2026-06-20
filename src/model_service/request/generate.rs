use rust_norion::TaskProfile;

use super::super::json::{json_string_field, json_usize_field};
use super::output::ModelServiceOutputMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceRequest {
    pub(crate) prompt: String,
    pub(crate) profile: Option<TaskProfile>,
    pub(crate) case_name: Option<String>,
    pub(crate) output_mode: ModelServiceOutputMode,
    pub(crate) max_tokens: Option<usize>,
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

    Ok(ModelServiceRequest {
        prompt,
        profile,
        case_name,
        output_mode,
        max_tokens,
    })
}
