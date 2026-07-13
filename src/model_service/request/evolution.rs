use rust_norion::TenantScope;

use super::super::json::json_string_field;
use super::scope::require_tenant_scope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModelServiceEvolutionAction {
    Apply,
    Rollback,
}

impl ModelServiceEvolutionAction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Apply => "apply",
            Self::Rollback => "rollback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceEvolutionRequest {
    pub(crate) action: ModelServiceEvolutionAction,
    pub(crate) token: String,
    pub(crate) tenant_scope: TenantScope,
}

pub(super) fn parse_evolution_request(body: &str) -> Result<ModelServiceEvolutionRequest, String> {
    let action = match json_string_field(body, "action")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "apply" => ModelServiceEvolutionAction::Apply,
        "rollback" => ModelServiceEvolutionAction::Rollback,
        _ => return Err("evolution action must be apply or rollback".to_owned()),
    };
    let token = json_string_field(body, "token")
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| "evolution request requires a non-empty token".to_owned())?;
    let tenant_scope = require_tenant_scope(body)?;
    Ok(ModelServiceEvolutionRequest {
        action,
        token,
        tenant_scope,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_apply_without_client_authority_flags() {
        let request = parse_evolution_request(
            "{\"action\":\"apply\",\"token\":\"candidate-token\",\"tenant_id\":\"local-console\",\"workspace_id\":\"rust-norion\",\"session_id\":\"session\"}",
        )
        .unwrap();

        assert_eq!(request.action, ModelServiceEvolutionAction::Apply);
        assert_eq!(request.token, "candidate-token");
        assert_eq!(request.tenant_scope.session_id, "session");
    }

    #[test]
    fn rejects_missing_action_or_token() {
        let error = parse_evolution_request(
            "{\"tenant_id\":\"local-console\",\"workspace_id\":\"rust-norion\",\"session_id\":\"session\"}",
        )
        .unwrap_err();
        assert_eq!(error, "evolution action must be apply or rollback");
    }
}
