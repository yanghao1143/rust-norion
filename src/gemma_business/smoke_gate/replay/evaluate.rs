mod failures;

use failures::gemma_business_smoke_replay_failures;
use rust_norion::ExperienceReplayReport;

pub(super) fn evaluate_gemma_business_smoke_replay_report(
    replay_report: &ExperienceReplayReport,
) -> bool {
    let failures = gemma_business_smoke_replay_failures(replay_report);
    for failure in &failures {
        println!("gemma_business_smoke_replay_failure: {failure}");
    }
    failures.is_empty()
}
