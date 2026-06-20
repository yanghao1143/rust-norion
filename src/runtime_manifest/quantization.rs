use crate::kv_quant::QuantizationBits;
use crate::runtime::RuntimeMetadata;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeQuantizationPolicy {
    pub hot_kv: QuantizationBits,
    pub cold_kv: QuantizationBits,
    pub weights: Option<QuantizationBits>,
}

impl Default for RuntimeQuantizationPolicy {
    fn default() -> Self {
        Self {
            hot_kv: QuantizationBits::Eight,
            cold_kv: QuantizationBits::Four,
            weights: None,
        }
    }
}

impl RuntimeQuantizationPolicy {
    pub fn from_metadata(metadata: &RuntimeMetadata) -> Self {
        let hot_kv = QuantizationBits::from_width(metadata.hot_kv_precision_bits)
            .unwrap_or(QuantizationBits::Eight);
        let cold_kv = QuantizationBits::from_width(metadata.cold_kv_precision_bits)
            .filter(|bits| bits.width() <= hot_kv.width())
            .unwrap_or_else(|| {
                if metadata.cold_kv_precision_bits > hot_kv.width() {
                    hot_kv
                } else {
                    QuantizationBits::Four
                }
            });
        Self {
            hot_kv,
            cold_kv,
            weights: None,
        }
    }
}
