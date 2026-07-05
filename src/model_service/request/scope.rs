use rust_norion::TenantScope;

use super::super::json::json_string_field;

pub(super) fn parse_tenant_scope(body: &str) -> Result<Option<TenantScope>, String> {
    let tenant_id = scope_field(body, "tenant_id");
    let workspace_id = scope_field(body, "workspace_id");
    let session_id = scope_field(body, "session_id");
    if tenant_id.is_none() && workspace_id.is_none() && session_id.is_none() {
        return Ok(None);
    }
    let (Some(tenant_id), Some(workspace_id), Some(session_id)) =
        (tenant_id, workspace_id, session_id)
    else {
        return Err("tenant scope requires tenant_id, workspace_id, and session_id".to_owned());
    };
    Ok(Some(TenantScope::new(tenant_id, workspace_id, session_id)))
}

fn scope_field(body: &str, field: &str) -> Option<String> {
    json_string_field(body, field).filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_scope_rejects_missing_scope_parts() {
        assert_eq!(
            parse_tenant_scope("{\"tenant_id\":\"tenant-a\"}").unwrap_err(),
            "tenant scope requires tenant_id, workspace_id, and session_id"
        );
    }

    #[test]
    fn tenant_scope_is_absent_without_scope_fields() {
        assert_eq!(parse_tenant_scope("{\"prompt\":\"hi\"}").unwrap(), None);
    }
}
