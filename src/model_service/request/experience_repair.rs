use std::path::PathBuf;

use super::super::json::{json_bool_field, json_string_field, json_usize_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceExperienceRepairRequest {
    pub(crate) apply: bool,
    pub(crate) limit: Option<usize>,
    pub(crate) backup_path: Option<PathBuf>,
}

pub(crate) fn parse_experience_repair_request(body: &str) -> ModelServiceExperienceRepairRequest {
    ModelServiceExperienceRepairRequest {
        apply: json_bool_field(body, "apply").unwrap_or(false),
        limit: json_usize_field(body, "limit").filter(|limit| *limit > 0),
        backup_path: json_string_field(body, "backup_path").map(PathBuf::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repair_request_defaults_to_dry_run() {
        let request = parse_experience_repair_request("{}");

        assert!(!request.apply);
        assert_eq!(request.limit, None);
        assert_eq!(request.backup_path, None);
    }

    #[test]
    fn repair_request_parses_apply_limit_and_backup() {
        let request = parse_experience_repair_request(
            "{\"apply\":true,\"limit\":7,\"backup_path\":\"repair-backup.ndkv\"}",
        );

        assert!(request.apply);
        assert_eq!(request.limit, Some(7));
        assert_eq!(
            request.backup_path,
            Some(PathBuf::from("repair-backup.ndkv"))
        );
    }
}
