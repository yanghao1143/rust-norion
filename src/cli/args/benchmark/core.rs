use super::BenchmarkFlagParse;
use crate::cli::args::values::{parse_f32, parse_u128, parse_usize};
use std::path::PathBuf;

pub(crate) fn parse(
    parser: &mut BenchmarkFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--benchmark" if index + 1 < raw.len() => {
            *parser.benchmark_path = Some(PathBuf::from(&raw[index + 1]));
            Some(2)
        }
        "--benchmark-gate" => {
            *parser.benchmark_gate_enabled = true;
            Some(1)
        }
        "--benchmark-all-devices" => {
            *parser.benchmark_all_devices = true;
            Some(1)
        }
        "--benchmark-min-quality" if index + 1 < raw.len() => {
            *parser.benchmark_min_quality = Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-reward" if index + 1 < raw.len() => {
            *parser.benchmark_min_reward = Some(parse_f32(&raw[index + 1], 0.0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-total-ms" if index + 1 < raw.len() => {
            *parser.benchmark_max_total_ms = Some(parse_u128(&raw[index + 1], u128::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-recursive-chunks" if index + 1 < raw.len() => {
            *parser.benchmark_max_recursive_chunks = Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-recursive-cases" if index + 1 < raw.len() => {
            *parser.benchmark_min_recursive_cases = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-recursive-runtime-calls" if index + 1 < raw.len() => {
            *parser.benchmark_min_recursive_runtime_calls = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        _ => None,
    }
}
