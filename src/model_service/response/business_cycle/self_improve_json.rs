use super::super::super::types::ModelServiceBusinessCycleReport;

pub(super) fn business_cycle_self_improve_json(
    report: &ModelServiceBusinessCycleReport,
    passed: bool,
) -> String {
    self_improve_json(SelfImproveJsonInput {
        enabled: report.self_improve_enabled,
        limit: report.self_improve_limit,
        passed,
    })
}

fn self_improve_json(input: SelfImproveJsonInput) -> String {
    format!(
        "{{\"enabled\":{},\"limit\":{},\"passed\":{}}}",
        input.enabled, input.limit, input.passed
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelfImproveJsonInput {
    enabled: bool,
    limit: usize,
    passed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_improve_json_renders_enabled_replay_gate() {
        let json = self_improve_json(SelfImproveJsonInput {
            enabled: true,
            limit: 4,
            passed: true,
        });

        assert_eq!(json, "{\"enabled\":true,\"limit\":4,\"passed\":true}");
    }

    #[test]
    fn self_improve_json_renders_disabled_replay_gate() {
        let json = self_improve_json(SelfImproveJsonInput {
            enabled: false,
            limit: 0,
            passed: true,
        });

        assert_eq!(json, "{\"enabled\":false,\"limit\":0,\"passed\":true}");
    }
}
