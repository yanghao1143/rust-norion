use std::path::Path;

use rust_norion::{MemoryUpdateReport, RustSnippetCheckReport};

use super::super::super::json::{
    option_i32_service_json, option_str_service_json, option_u64_service_json, service_json_string,
    service_memory_update_array, service_u64_array,
};
use super::super::super::types::ModelServiceBusinessCycleReport;
use super::super::update_stats::{
    memory_update_applied_count, memory_update_missing_count, memory_update_removed_count,
    memory_update_strength_delta,
};

pub(super) fn option_business_cycle_rust_check_json(
    report: &ModelServiceBusinessCycleReport,
) -> String {
    let Some(check_report) = report.rust_check_report.as_ref() else {
        return "null".to_owned();
    };
    let feedback_request = report
        .rust_check_feedback_request
        .as_ref()
        .expect("rust check feedback request should exist with report");
    let rust_request = report
        .rust_check_request
        .as_ref()
        .expect("rust check request should exist with report");
    rust_check_json(RustCheckJsonInput {
        check_report,
        case_name: rust_request.case_name.as_deref(),
        feedback_action: feedback_request.action.as_str(),
        feedback_amount: feedback_request.amount,
        experience_id: feedback_request.experience_id,
        memory_ids: &report.rust_check_memory_ids,
        updates: &report.rust_check_updates,
    })
}

fn rust_check_json(input: RustCheckJsonInput<'_>) -> String {
    let check_report = input.check_report;
    format!(
        "{{\"passed\":{},\"label\":\"{}\",\"edition\":\"{}\",\"case\":{},\"status_code\":{},\"diagnostic_chars\":{},\"stdout\":{},\"stderr\":{},\"source_path\":{},\"metadata_path\":{},\"feedback_action\":\"{}\",\"feedback_amount\":{:.6},\"experience_id\":{},\"memory_ids\":{},\"applied\":{},\"missing\":{},\"removed\":{},\"strength_delta\":{:.6},\"updates\":{}}}",
        check_report.passed,
        check_report.feedback_label(),
        check_report.edition,
        option_str_service_json(input.case_name),
        option_i32_service_json(check_report.status_code),
        check_report.diagnostic_chars(),
        service_json_string(&check_report.stdout),
        service_json_string(&check_report.stderr),
        path_json(&check_report.source_path),
        path_json(&check_report.metadata_path),
        input.feedback_action,
        input.feedback_amount,
        option_u64_service_json(input.experience_id),
        service_u64_array(input.memory_ids),
        memory_update_applied_count(input.updates),
        memory_update_missing_count(input.updates),
        memory_update_removed_count(input.updates),
        memory_update_strength_delta(input.updates),
        service_memory_update_array(input.updates)
    )
}

struct RustCheckJsonInput<'a> {
    check_report: &'a RustSnippetCheckReport,
    case_name: Option<&'a str>,
    feedback_action: &'a str,
    feedback_amount: f32,
    experience_id: Option<u64>,
    memory_ids: &'a [u64],
    updates: &'a [MemoryUpdateReport],
}

fn path_json(path: &Path) -> String {
    service_json_string(&path.display().to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rust_norion::MemoryUpdateAction;

    use super::*;

    #[test]
    fn rust_check_json_renders_compiler_feedback_and_memory_updates() {
        let report = RustSnippetCheckReport {
            passed: false,
            edition: "2024".to_owned(),
            status_code: Some(1),
            stdout: "compiler stdout".to_owned(),
            stderr: "missing semicolon".to_owned(),
            source_path: PathBuf::from("target/model-service-rust-check/case/lib.rs"),
            metadata_path: PathBuf::from("target/model-service-rust-check/case/check.rmeta"),
        };
        let updates = [
            MemoryUpdateReport::applied(7, MemoryUpdateAction::Penalize, 0.35, 0.8, 0.45, false),
            MemoryUpdateReport::missing(8, MemoryUpdateAction::Penalize, 0.35),
        ];

        let json = rust_check_json(RustCheckJsonInput {
            check_report: &report,
            case_name: Some("case-a"),
            feedback_action: "penalize",
            feedback_amount: 0.35,
            experience_id: Some(42),
            memory_ids: &[7, 8],
            updates: &updates,
        });

        assert!(json.contains("\"passed\":false"));
        assert!(json.contains("\"label\":\"rustc_failed\""));
        assert!(json.contains("\"edition\":\"2024\""));
        assert!(json.contains("\"case\":\"case-a\""));
        assert!(json.contains("\"status_code\":1"));
        assert!(json.contains("\"diagnostic_chars\":32"));
        assert!(json.contains("\"feedback_action\":\"penalize\""));
        assert!(json.contains("\"experience_id\":42"));
        assert!(json.contains("\"memory_ids\":[7,8]"));
        assert!(json.contains("\"applied\":1"));
        assert!(json.contains("\"missing\":1"));
        assert!(json.contains("\"strength_delta\":0.350000"));
    }

    #[test]
    fn rust_check_json_renders_optional_case_and_status_as_null() {
        let report = RustSnippetCheckReport {
            passed: true,
            edition: "2021".to_owned(),
            status_code: None,
            stdout: String::new(),
            stderr: String::new(),
            source_path: PathBuf::from("lib.rs"),
            metadata_path: PathBuf::from("check.rmeta"),
        };

        let json = rust_check_json(RustCheckJsonInput {
            check_report: &report,
            case_name: None,
            feedback_action: "reinforce",
            feedback_amount: 0.45,
            experience_id: None,
            memory_ids: &[],
            updates: &[],
        });

        assert!(json.contains("\"passed\":true"));
        assert!(json.contains("\"label\":\"rustc_passed\""));
        assert!(json.contains("\"case\":null"));
        assert!(json.contains("\"status_code\":null"));
        assert!(json.contains("\"experience_id\":null"));
        assert!(json.contains("\"memory_ids\":[]"));
        assert!(json.contains("\"applied\":0"));
    }
}
