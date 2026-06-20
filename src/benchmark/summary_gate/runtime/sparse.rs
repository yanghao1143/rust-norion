use super::super::super::BenchmarkGate;
use super::super::super::summary::BenchmarkSummary;
use super::super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(min_sparse_skipped_cases) = gate.min_sparse_skipped_cases {
        let sparse_skipped_cases = summary.sparse_skipped_cases();
        if sparse_skipped_cases < min_sparse_skipped_cases {
            failures.push(format!(
                "sparse_skipped_cases {} below minimum {}",
                sparse_skipped_cases, min_sparse_skipped_cases
            ));
        }
    }

    if let Some(min_sparse_skipped_tokens) = gate.min_sparse_skipped_tokens {
        let sparse_skipped_tokens = summary.total_sparse_skipped_tokens();
        if sparse_skipped_tokens < min_sparse_skipped_tokens {
            failures.push(format!(
                "sparse_skipped_tokens {} below minimum {}",
                sparse_skipped_tokens, min_sparse_skipped_tokens
            ));
        }
    }
}
