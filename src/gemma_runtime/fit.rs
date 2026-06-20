use super::spec::GEMMA4_12B_DEFAULT_QUANT;

#[derive(Debug, Clone, PartialEq)]
pub struct GemmaRuntimeFitSummary {
    pub total_vram_mib: usize,
    pub bf16_weights_mib: usize,
    pub q4_weights_mib: usize,
    pub recommended_quantization: String,
    pub fits_bf16_weights: bool,
    pub fits_q4_weights: bool,
}

impl GemmaRuntimeFitSummary {
    pub fn for_vram(total_vram_mib: usize) -> Self {
        let bf16_weights_mib = 26_700;
        let q4_weights_mib = 6_700;
        Self {
            total_vram_mib,
            bf16_weights_mib,
            q4_weights_mib,
            recommended_quantization: GEMMA4_12B_DEFAULT_QUANT.to_owned(),
            fits_bf16_weights: total_vram_mib >= bf16_weights_mib,
            fits_q4_weights: total_vram_mib >= q4_weights_mib,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "vram_mib={} bf16_weights_mib={} q4_weights_mib={} recommended_quant={} fits_bf16_weights={} fits_q4_weights={}",
            self.total_vram_mib,
            self.bf16_weights_mib,
            self.q4_weights_mib,
            self.recommended_quantization,
            self.fits_bf16_weights,
            self.fits_q4_weights
        )
    }
}
