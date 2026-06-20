use std::path::PathBuf;

use rust_norion::{RustSnippetCheck, RustSnippetCheckReport, RustSnippetValidator};

use super::request::ModelServiceRustCheckRequest;

pub(crate) fn model_service_rust_check_report(
    request: &ModelServiceRustCheckRequest,
    default_case_name: &str,
) -> std::io::Result<RustSnippetCheckReport> {
    let validator =
        RustSnippetValidator::new(PathBuf::from("target").join("model-service-rust-check"));
    let check = RustSnippetCheck::new(request.code.clone())
        .with_edition(request.edition.clone())
        .with_case_name(
            request
                .case_name
                .clone()
                .unwrap_or_else(|| default_case_name.to_owned()),
        );
    validator.check(&check)
}
