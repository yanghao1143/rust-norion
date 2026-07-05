use rust_norion::TenantScope;

use super::super::json::{json_f32_field, json_string_field, json_u64_field};
use super::scope::require_tenant_scope;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelServiceRustCheckRequest {
    pub(crate) code: String,
    pub(crate) edition: String,
    pub(crate) case_name: Option<String>,
    pub(crate) amount: Option<f32>,
    pub(crate) experience_id: Option<u64>,
    pub(crate) memory_id: Option<u64>,
    pub(crate) tenant_scope: Option<TenantScope>,
}

pub(super) fn parse_rust_check_request(body: &str) -> Result<ModelServiceRustCheckRequest, String> {
    let code = json_string_field(body, "code")
        .filter(|code| !code.trim().is_empty())
        .ok_or_else(|| "rust_check requires a non-empty code string".to_owned())?;
    let edition = json_string_field(body, "edition")
        .filter(|edition| !edition.trim().is_empty())
        .unwrap_or_else(|| "2021".to_owned());
    let case_name = json_string_field(body, "case").filter(|case| !case.trim().is_empty());
    let amount = json_f32_field(body, "amount").map(|amount| amount.clamp(0.0, 1.0));
    let experience_id = json_u64_field(body, "experience_id");
    let memory_id = json_u64_field(body, "memory_id");
    let tenant_scope = if experience_id.is_some() || memory_id.is_some() {
        Some(require_tenant_scope(body)?)
    } else {
        None
    };

    Ok(ModelServiceRustCheckRequest {
        code,
        edition,
        case_name,
        amount,
        experience_id,
        memory_id,
        tenant_scope,
    })
}
