use rust_norion::TenantScope;

use super::super::json::json_string_field;

pub(super) fn parse_tenant_scope(body: &str) -> Option<TenantScope> {
    let tenant_id = scope_field(body, "tenant_id");
    let workspace_id = scope_field(body, "workspace_id");
    let session_id = scope_field(body, "session_id");
    if tenant_id.is_none() && workspace_id.is_none() && session_id.is_none() {
        return None;
    }
    Some(TenantScope::new(
        tenant_id.as_deref().unwrap_or("local"),
        workspace_id.as_deref().unwrap_or("default"),
        session_id.as_deref().unwrap_or("interactive"),
    ))
}

fn scope_field(body: &str, field: &str) -> Option<String> {
    json_string_field(body, field).filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_scope_defaults_missing_scope_parts() {
        let scope = parse_tenant_scope("{\"tenant_id\":\"tenant-a\"}").unwrap();

        assert_eq!(
            scope,
            TenantScope::new("tenant-a", "default", "interactive")
        );
    }

    #[test]
    fn tenant_scope_is_absent_without_scope_fields() {
        assert_eq!(parse_tenant_scope("{\"prompt\":\"hi\"}"), None);
    }
}
