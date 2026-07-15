use std::path::Path;

mod gate;
mod minimums;
mod report;

use gate::require_state_gate_report;
use report::require_state_report_minimums;
use rust_norion::{NoironEngine, StateInspectionReport};

pub(super) fn require_state_artifacts(
    memory_path: &Path,
    experience_path: &Path,
    adaptive_path: &Path,
    report_body: &str,
    expected_case_count: u64,
    failures: &mut Vec<String>,
) {
    match NoironEngine::full_state_files_exist(memory_path, experience_path, adaptive_path) {
        Ok(true) => {}
        Ok(false) => {
            push_state_artifact_load_failure(
                memory_path,
                experience_path,
                adaptive_path,
                "full-state files are missing",
                failures,
            );
            return;
        }
        Err(error) => {
            push_state_artifact_load_failure(
                memory_path,
                experience_path,
                adaptive_path,
                &error.to_string(),
                failures,
            );
            return;
        }
    }
    match NoironEngine::load_full_state(memory_path, experience_path, adaptive_path) {
        Ok(engine) => {
            let inspection = StateInspectionReport::from_engine(&engine, 1);
            require_state_gate_report(&inspection, expected_case_count, failures);
            require_state_report_minimums(&inspection, report_body, expected_case_count, failures);
        }
        Err(error) => push_state_artifact_load_failure(
            memory_path,
            experience_path,
            adaptive_path,
            &error.to_string(),
            failures,
        ),
    }
}

fn push_state_artifact_load_failure(
    memory_path: &Path,
    experience_path: &Path,
    adaptive_path: &Path,
    error: &str,
    failures: &mut Vec<String>,
) {
    failures.push(format!(
        "state artifacts could not be loaded memory={} experience={} adaptive={}: {error}",
        memory_path.display(),
        experience_path.display(),
        adaptive_path.display()
    ));
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::push_state_artifact_load_failure;

    #[test]
    fn push_state_artifact_load_failure_formats_all_state_paths() {
        let mut failures = Vec::new();

        push_state_artifact_load_failure(
            Path::new("memory.json"),
            Path::new("experience.json"),
            Path::new("adaptive.json"),
            "bad state",
            &mut failures,
        );

        assert_eq!(
            failures,
            [
                "state artifacts could not be loaded memory=memory.json experience=experience.json adaptive=adaptive.json: bad state"
            ]
        );
    }
}
