use super::super::BenchmarkGate;
use super::super::summary::BenchmarkSummary;
use super::GateFailures;

pub(super) fn evaluate(
    summary: &BenchmarkSummary,
    gate: &BenchmarkGate,
    failures: &mut GateFailures,
) {
    if let Some(max_reasoning_genome_failures) = gate.max_reasoning_genome_failures {
        let observed = summary.total_reasoning_genome_failures();
        if observed > max_reasoning_genome_failures {
            failures.push(format!(
                "reasoning_genome_failures {} above maximum {}: {}",
                observed,
                max_reasoning_genome_failures,
                summary.genome_evidence.failures.join("; ")
            ));
        }
    }

    if let Some(min_reasoning_genome_expression_cases) = gate.min_reasoning_genome_expression_cases
    {
        let observed = summary.reasoning_genome_expression_cases();
        if observed < min_reasoning_genome_expression_cases {
            failures.push(format!(
                "reasoning_genome_expression_cases {} below minimum {}",
                observed, min_reasoning_genome_expression_cases
            ));
        }
    }

    if let Some(min_reasoning_genome_expression_device_profiles) =
        gate.min_reasoning_genome_expression_device_profiles
    {
        let observed = summary.reasoning_genome_expression_device_profiles();
        if observed < min_reasoning_genome_expression_device_profiles {
            failures.push(format!(
                "reasoning_genome_expression_device_profiles {} below minimum {}",
                observed, min_reasoning_genome_expression_device_profiles
            ));
        }
    }

    if let Some(min_reasoning_genome_splice_cases) = gate.min_reasoning_genome_splice_cases {
        let observed = summary.reasoning_genome_splice_cases();
        if observed < min_reasoning_genome_splice_cases {
            failures.push(format!(
                "reasoning_genome_splice_cases {} below minimum {}",
                observed, min_reasoning_genome_splice_cases
            ));
        }
    }

    if let Some(min_reasoning_genome_splice_device_profiles) =
        gate.min_reasoning_genome_splice_device_profiles
    {
        let observed = summary.reasoning_genome_splice_device_profiles();
        if observed < min_reasoning_genome_splice_device_profiles {
            failures.push(format!(
                "reasoning_genome_splice_device_profiles {} below minimum {}",
                observed, min_reasoning_genome_splice_device_profiles
            ));
        }
    }

    if let Some(min_gene_scissors_proposal_cases) = gate.min_gene_scissors_proposal_cases {
        let observed = summary.gene_scissors_proposal_cases();
        if observed < min_gene_scissors_proposal_cases {
            failures.push(format!(
                "gene_scissors_proposal_cases {} below minimum {}",
                observed, min_gene_scissors_proposal_cases
            ));
        }
    }

    if let Some(min_gene_scissors_proposal_device_profiles) =
        gate.min_gene_scissors_proposal_device_profiles
    {
        let observed = summary.gene_scissors_proposal_device_profiles();
        if observed < min_gene_scissors_proposal_device_profiles {
            failures.push(format!(
                "gene_scissors_proposal_device_profiles {} below minimum {}",
                observed, min_gene_scissors_proposal_device_profiles
            ));
        }
    }

    if let Some(min_reasoning_genome_repair_payloads) = gate.min_reasoning_genome_repair_payloads {
        let observed = summary.total_reasoning_genome_repair_payloads();
        if observed < min_reasoning_genome_repair_payloads {
            failures.push(format!(
                "reasoning_genome_repair_payloads {} below minimum {}",
                observed, min_reasoning_genome_repair_payloads
            ));
        }
    }

    if let Some(min_reasoning_genome_regeneration_payloads) =
        gate.min_reasoning_genome_regeneration_payloads
    {
        let observed = summary.total_reasoning_genome_regeneration_payloads();
        if observed < min_reasoning_genome_regeneration_payloads {
            failures.push(format!(
                "reasoning_genome_regeneration_payloads {} below minimum {}",
                observed, min_reasoning_genome_regeneration_payloads
            ));
        }
    }
}
