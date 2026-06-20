use super::super::*;
use super::base::evaluate_base_counts;
use super::feedback::evaluate_feedback_and_contracts;
use super::live_evolution::evaluate_live_evolution;
use super::replay_evolution::evaluate_replay_evolution;
use super::runtime::evaluate_runtime_evidence;

impl StateInspectionReport {
    pub fn evaluate(&self, gate: &StateInspectionGate) -> StateInspectionGateReport {
        let mut failures = Vec::new();

        evaluate_base_counts(self, gate, &mut failures);
        evaluate_runtime_evidence(self, gate, &mut failures);
        evaluate_feedback_and_contracts(self, gate, &mut failures);
        evaluate_live_evolution(self, gate, &mut failures);
        evaluate_replay_evolution(self, gate, &mut failures);

        if gate.require_runtime_kv_dimensions && self.runtime_kv_vector_dimensions.is_empty() {
            failures.push("runtime_kv_vector_dimensions missing required buckets".to_owned());
        }

        StateInspectionGateReport {
            passed: failures.is_empty(),
            failures,
        }
    }
}
