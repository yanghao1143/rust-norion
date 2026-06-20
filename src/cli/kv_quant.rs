use rust_norion::{KvQuantBenchmarkGateReport, KvQuantBenchmarkSummary};

pub(crate) fn print_kv_quant_gate_report(
    summary: &KvQuantBenchmarkSummary,
    report: &KvQuantBenchmarkGateReport,
) {
    println!("Noiron KV quantization benchmark");
    println!("{}", summary.summary_line());
    println!("{}", report.summary_line());
    println!("case,bits,len,max_abs_error,mean_abs_error,compression_ratio,elapsed_us");

    for result in summary.results() {
        println!(
            "{},q{},{},{:.6},{:.6},{:.3},{}",
            result.name,
            result.bits.width(),
            result.len,
            result.max_abs_error,
            result.mean_abs_error,
            result.compression_ratio,
            result.elapsed_us
        );
    }

    for failure in &report.failures {
        println!("kv_quant_gate_failure: {failure}");
    }
}
