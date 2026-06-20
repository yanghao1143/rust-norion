use super::super::json::json_usize_field;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModelServiceExperienceCleanupAuditRequest {
    pub(crate) limit: Option<usize>,
}

pub(crate) fn parse_experience_cleanup_audit_request(
    body: &str,
) -> ModelServiceExperienceCleanupAuditRequest {
    ModelServiceExperienceCleanupAuditRequest {
        limit: json_usize_field(body, "limit").filter(|limit| *limit > 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_audit_request_defaults_to_args_limit() {
        let request = parse_experience_cleanup_audit_request("{}");

        assert_eq!(request.limit, None);
    }

    #[test]
    fn cleanup_audit_request_parses_limit() {
        let request = parse_experience_cleanup_audit_request("{\"limit\":7}");

        assert_eq!(request.limit, Some(7));
    }
}
