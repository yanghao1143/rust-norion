use super::BenchmarkFlagParse;
use crate::cli::args::values::parse_usize;

pub(crate) fn parse(
    parser: &mut BenchmarkFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--benchmark-min-reasoning-genome-expression-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_reasoning_genome_expression_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-reasoning-genome-expression-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_reasoning_genome_expression_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-reasoning-genome-splice-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_reasoning_genome_splice_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-reasoning-genome-splice-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_reasoning_genome_splice_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-gene-scissors-proposal-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_gene_scissors_proposal_cases =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-gene-scissors-proposal-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_gene_scissors_proposal_device_profiles =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            *parser.benchmark_all_devices = true;
            Some(2)
        }
        "--benchmark-min-reasoning-genome-repair-payloads" if index + 1 < raw.len() => {
            *parser.benchmark_min_reasoning_genome_repair_payloads =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-reasoning-genome-regeneration-payloads" if index + 1 < raw.len() => {
            *parser.benchmark_min_reasoning_genome_regeneration_payloads =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-mutation-repair-fixtures" if index + 1 < raw.len() => {
            *parser.benchmark_min_mutation_repair_fixtures = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-mutation-repair-fixture-kinds" if index + 1 < raw.len() => {
            *parser.benchmark_min_mutation_repair_fixture_kinds =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-mutation-repair-candidates" if index + 1 < raw.len() => {
            *parser.benchmark_min_mutation_repair_candidates =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-mutation-repair-review-packets" if index + 1 < raw.len() => {
            *parser.benchmark_min_mutation_repair_review_packets =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-malignant-gene-recovery-drills" if index + 1 < raw.len() => {
            *parser.benchmark_min_malignant_gene_recovery_drills =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-malignant-gene-quarantines" if index + 1 < raw.len() => {
            *parser.benchmark_min_malignant_gene_quarantines =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-malignant-gene-cut-candidates" if index + 1 < raw.len() => {
            *parser.benchmark_min_malignant_gene_cut_candidates =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-malignant-gene-regeneration-candidates" if index + 1 < raw.len() => {
            *parser.benchmark_min_malignant_gene_regeneration_candidates =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-dna-evolution-reports" if index + 1 < raw.len() => {
            *parser.benchmark_min_dna_evolution_reports = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-dna-evolution-candidates" if index + 1 < raw.len() => {
            *parser.benchmark_min_dna_evolution_candidates = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-dna-evolution-candidate-previews" if index + 1 < raw.len() => {
            *parser.benchmark_min_dna_evolution_candidate_previews =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-dna-evolution-candidate-ledger-records" if index + 1 < raw.len() => {
            *parser.benchmark_min_dna_evolution_candidate_ledger_records =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-dna-evolution-candidate-ledger-preview-only" if index + 1 < raw.len() => {
            *parser.benchmark_min_dna_evolution_candidate_ledger_preview_only =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-dna-evolution-activation-eligible" if index + 1 < raw.len() => {
            *parser.benchmark_max_dna_evolution_activation_eligible =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-dna-evolution-transaction-replays" if index + 1 < raw.len() => {
            *parser.benchmark_min_dna_evolution_transaction_replays =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-dna-evolution-replay-passed" if index + 1 < raw.len() => {
            *parser.benchmark_min_dna_evolution_replay_passed =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-dna-evolution-validation-passed" if index + 1 < raw.len() => {
            *parser.benchmark_min_dna_evolution_validation_passed =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-dna-evolution-writer-gate-reports" if index + 1 < raw.len() => {
            *parser.benchmark_min_dna_evolution_writer_gate_reports =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-dna-evolution-writer-gate-holds" if index + 1 < raw.len() => {
            *parser.benchmark_min_dna_evolution_writer_gate_holds =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-dna-evolution-writer-gate-explicit-apply-required"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_min_dna_evolution_writer_gate_explicit_apply_required =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-dna-evolution-writer-gate-ready" if index + 1 < raw.len() => {
            *parser.benchmark_max_dna_evolution_writer_gate_ready =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-dna-evolution-writer-gate-durable-write-allowed"
            if index + 1 < raw.len() =>
        {
            *parser.benchmark_max_dna_evolution_writer_gate_durable_write_allowed =
                Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        _ => None,
    }
}
