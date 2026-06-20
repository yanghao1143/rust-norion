use std::path::PathBuf;

use super::super::json::{json_bool_field, json_string_field, json_usize_field};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelServiceExperienceHygieneQuarantineRequest {
    pub(crate) apply: bool,
    pub(crate) limit: Option<usize>,
    pub(crate) backup_path: Option<PathBuf>,
    pub(crate) quarantine_path: Option<PathBuf>,
}

pub(crate) fn parse_experience_hygiene_quarantine_request(
    body: &str,
) -> ModelServiceExperienceHygieneQuarantineRequest {
    ModelServiceExperienceHygieneQuarantineRequest {
        apply: json_bool_field(body, "apply").unwrap_or(false),
        limit: json_usize_field(body, "limit").filter(|limit| *limit > 0),
        backup_path: json_string_field(body, "backup_path").map(PathBuf::from),
        quarantine_path: json_string_field(body, "quarantine_path").map(PathBuf::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarantine_request_defaults_to_dry_run() {
        let request = parse_experience_hygiene_quarantine_request("{}");

        assert!(!request.apply);
        assert_eq!(request.limit, None);
        assert_eq!(request.backup_path, None);
        assert_eq!(request.quarantine_path, None);
    }

    #[test]
    fn quarantine_request_parses_explicit_apply_and_paths() {
        let request = parse_experience_hygiene_quarantine_request(
            "{\"apply\":true,\"limit\":7,\"backup_path\":\"backup.ndkv\",\"quarantine_path\":\"quarantine.ndkv\"}",
        );

        assert!(request.apply);
        assert_eq!(request.limit, Some(7));
        assert_eq!(request.backup_path, Some(PathBuf::from("backup.ndkv")));
        assert_eq!(
            request.quarantine_path,
            Some(PathBuf::from("quarantine.ndkv"))
        );
    }
}
