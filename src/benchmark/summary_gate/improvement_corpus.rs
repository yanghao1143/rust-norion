use super::super::BenchmarkGate;
use super::super::summary::BenchmarkSummary;
use super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    let evidence = &summary.improvement_corpus_evidence;

    if let Some(minimum) = gate.min_improvement_corpus_reports {
        if evidence.reports < minimum {
            failures.push(format!(
                "improvement_corpus_reports {} below minimum {}",
                evidence.reports, minimum
            ));
        }
    }
    if let Some(minimum) = gate.min_improvement_corpus_episodes {
        if evidence.episodes < minimum {
            failures.push(format!(
                "improvement_corpus_episodes {} below minimum {}",
                evidence.episodes, minimum
            ));
        }
    }
    if let Some(minimum) = gate.min_improvement_corpus_active_adaptation {
        if evidence.active_adaptation < minimum {
            failures.push(format!(
                "improvement_corpus_active_adaptation {} below minimum {}",
                evidence.active_adaptation, minimum
            ));
        }
    }
    if let Some(minimum) = gate.min_improvement_corpus_compiler_passed {
        if evidence.compiler_passed < minimum {
            failures.push(format!(
                "improvement_corpus_compiler_passed {} below minimum {}",
                evidence.compiler_passed, minimum
            ));
        }
    }
    if let Some(minimum) = gate.min_improvement_corpus_test_passed {
        if evidence.test_passed < minimum {
            failures.push(format!(
                "improvement_corpus_test_passed {} below minimum {}",
                evidence.test_passed, minimum
            ));
        }
    }
    if let Some(minimum) = gate.min_improvement_corpus_benchmark_passed {
        if evidence.benchmark_passed < minimum {
            failures.push(format!(
                "improvement_corpus_benchmark_passed {} below minimum {}",
                evidence.benchmark_passed, minimum
            ));
        }
    }
    if let Some(minimum) = gate.min_improvement_corpus_rollback_replayed {
        if evidence.rollback_replayed < minimum {
            failures.push(format!(
                "improvement_corpus_rollback_replayed {} below minimum {}",
                evidence.rollback_replayed, minimum
            ));
        }
    }
    if let Some(maximum) = gate.max_improvement_corpus_secret_leaks {
        if evidence.secret_leaks > maximum {
            failures.push(format!(
                "improvement_corpus_secret_leaks {} above maximum {}",
                evidence.secret_leaks, maximum
            ));
        }
    }
    if let Some(maximum) = gate.max_improvement_corpus_dataset_export_enabled {
        if evidence.dataset_export_enabled > maximum {
            failures.push(format!(
                "improvement_corpus_dataset_export_enabled {} above maximum {}",
                evidence.dataset_export_enabled, maximum
            ));
        }
    }
}
