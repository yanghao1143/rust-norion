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
        _ => None,
    }
}
