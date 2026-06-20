use super::BenchmarkFlagParse;
use crate::cli::args::values::parse_usize;

pub(crate) fn parse(
    parser: &mut BenchmarkFlagParse<'_>,
    raw: &[String],
    index: usize,
) -> Option<usize> {
    match raw.get(index)?.as_str() {
        "--benchmark-min-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_device_profiles = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-min-recursive-device-profiles" if index + 1 < raw.len() => {
            *parser.benchmark_min_recursive_device_profiles = Some(parse_usize(&raw[index + 1], 0));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-drift-blocks" if index + 1 < raw.len() => {
            *parser.benchmark_max_drift_blocks = Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-max-drift-rollbacks" if index + 1 < raw.len() => {
            *parser.benchmark_max_drift_rollbacks = Some(parse_usize(&raw[index + 1], usize::MAX));
            *parser.benchmark_gate_enabled = true;
            Some(2)
        }
        "--benchmark-roundtrip" | "--roundtrip-gate" => {
            *parser.benchmark_roundtrip = true;
            Some(1)
        }
        _ => None,
    }
}
