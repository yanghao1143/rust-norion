use super::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn rust_snippet_validator_passes_metadata_only_for_valid_code() {
    let work_dir = target_test_dir("rust-snippet-valid");
    let validator = RustSnippetValidator::new(&work_dir);
    let report = validator
        .check(
            &RustSnippetCheck::new("pub fn ownership_hint(input: String) -> usize { input.len() }")
                .with_edition("2021")
                .with_case_name("valid ownership"),
        )
        .unwrap();

    assert!(report.passed, "{report:?}");
    assert!(report.metadata_path.is_file());
    assert!(!report.source_path.with_extension("exe").exists());
    assert_eq!(report.feedback_label(), "rustc_passed");
    fs::remove_dir_all(work_dir).unwrap();
}

#[test]
fn rust_snippet_validator_reports_failed_compile_without_executing() {
    let work_dir = target_test_dir("rust-snippet-invalid");
    let validator = RustSnippetValidator::new(&work_dir);
    let report = validator
        .check(&RustSnippetCheck::new(
            "pub fn broken() -> u32 { missing_symbol }",
        ))
        .unwrap();

    assert!(!report.passed);
    assert!(report.status_code.is_some());
    assert!(report.stderr.contains("missing_symbol"));
    assert_eq!(report.feedback_label(), "rustc_failed");
    fs::remove_dir_all(work_dir).unwrap();
}

#[test]
fn rust_snippet_validator_rejects_invalid_edition_before_spawning() {
    let validator = RustSnippetValidator::new(target_test_dir("rust-snippet-edition"));
    let error = validator
        .check(&RustSnippetCheck::new("pub fn ok() {}").with_edition("2015"))
        .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(error.to_string().contains("unsupported Rust edition"));
}

fn target_test_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    Path::new("target").join(format!("{name}-{unique}"))
}
