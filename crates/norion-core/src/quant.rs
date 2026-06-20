use std::fmt;

use crate::kv::{KvBlock, KvNamespace};
use crate::manifest::QuantizationBits;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizationError {
    InvalidBits,
    InvalidFormat,
    InvalidLength,
}

impl fmt::Display for QuantizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBits => formatter.write_str("invalid quantization bits"),
            Self::InvalidFormat => formatter.write_str("invalid quantized vector format"),
            Self::InvalidLength => formatter.write_str("invalid quantized vector length"),
        }
    }
}

impl std::error::Error for QuantizationError {}

#[derive(Debug, Clone, PartialEq)]
pub struct QuantizedVector {
    bits: QuantizationBits,
    len: usize,
    min: f32,
    scale: f32,
    packed: Vec<u8>,
}

impl QuantizedVector {
    pub fn quantize(vector: &[f32], bits: QuantizationBits) -> Self {
        if vector.is_empty() {
            return Self {
                bits,
                len: 0,
                min: 0.0,
                scale: 0.0,
                packed: Vec::new(),
            };
        }

        let sanitized = vector
            .iter()
            .map(|value| if value.is_finite() { *value } else { 0.0 })
            .collect::<Vec<_>>();
        let min = sanitized
            .iter()
            .copied()
            .fold(f32::INFINITY, |left, right| left.min(right));
        let max = sanitized
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, |left, right| left.max(right));
        let range = max - min;
        let scale = if range.abs() <= f32::EPSILON {
            0.0
        } else {
            range / quantization_levels(bits) as f32
        };
        let codes = sanitized
            .iter()
            .map(|value| quantize_value(*value, min, scale, quantization_levels(bits)))
            .collect::<Vec<_>>();

        Self {
            bits,
            len: sanitized.len(),
            min,
            scale,
            packed: pack_codes(&codes, bits),
        }
    }

    pub fn decode(encoded: &str) -> Result<Self, QuantizationError> {
        let mut fields = encoded.split(':');
        let Some(prefix) = fields.next() else {
            return Err(QuantizationError::InvalidFormat);
        };
        let Some(width) = prefix.strip_prefix('q') else {
            return Err(QuantizationError::InvalidFormat);
        };
        let width = width
            .parse::<u8>()
            .map_err(|_| QuantizationError::InvalidBits)?;
        let bits = QuantizationBits::from_width(width).ok_or(QuantizationError::InvalidBits)?;
        let len = fields
            .next()
            .ok_or(QuantizationError::InvalidFormat)?
            .parse::<usize>()
            .map_err(|_| QuantizationError::InvalidLength)?;
        let min = decode_f32(fields.next().ok_or(QuantizationError::InvalidFormat)?)?;
        let scale = decode_f32(fields.next().ok_or(QuantizationError::InvalidFormat)?)?;
        let packed = decode_hex(fields.next().ok_or(QuantizationError::InvalidFormat)?)?;

        if fields.next().is_some() {
            return Err(QuantizationError::InvalidFormat);
        }
        if unpacked_capacity(bits, packed.len()) < len {
            return Err(QuantizationError::InvalidLength);
        }

        Ok(Self {
            bits,
            len,
            min,
            scale,
            packed,
        })
    }

    pub fn encode(&self) -> String {
        format!(
            "q{}:{}:{:08x}:{:08x}:{}",
            self.bits.width(),
            self.len,
            self.min.to_bits(),
            self.scale.to_bits(),
            encode_hex(&self.packed)
        )
    }

    pub fn dequantize(&self) -> Vec<f32> {
        unpack_codes(&self.packed, self.bits, self.len)
            .into_iter()
            .map(|code| self.min + self.scale * code as f32)
            .collect()
    }

    pub fn bits(&self) -> QuantizationBits {
        self.bits
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn packed_len(&self) -> usize {
        self.packed.len()
    }

    pub fn compression_ratio(&self) -> f32 {
        if self.len == 0 {
            return 1.0;
        }
        self.packed.len() as f32 / (self.len * std::mem::size_of::<f32>()) as f32
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuantizedKvBlock {
    pub id: u64,
    pub namespace: KvNamespace,
    pub layer: usize,
    pub head: usize,
    pub token_start: usize,
    pub token_end: usize,
    pub key: QuantizedVector,
    pub value: QuantizedVector,
    pub score: f32,
    pub reinforcement: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuantizedKvPayloadSummary {
    pub namespace_label: String,
    pub is_runtime_namespace: bool,
    pub bits: QuantizationBits,
    pub key_value_len: usize,
    pub packed_payload_len: usize,
    pub compression_ratio: f32,
    pub key_len: usize,
    pub value_len: usize,
    pub key_packed_len: usize,
    pub value_packed_len: usize,
}

impl QuantizedKvPayloadSummary {
    pub fn is_empty(&self) -> bool {
        self.key_value_len == 0
    }

    pub fn key_value_lengths_match(&self) -> bool {
        self.key_len.saturating_add(self.value_len) == self.key_value_len
    }

    pub fn packed_lengths_match(&self) -> bool {
        self.key_packed_len.saturating_add(self.value_packed_len) == self.packed_payload_len
    }

    pub fn payload_shape_balanced(&self) -> bool {
        self.key_value_lengths_match() && self.packed_lengths_match()
    }

    pub fn has_key_value_payload(&self) -> bool {
        self.key_len > 0 && self.value_len > 0
    }

    pub fn key_value_lengths_are_symmetric(&self) -> bool {
        self.key_len == self.value_len
    }

    pub fn packed_lengths_are_symmetric(&self) -> bool {
        self.key_packed_len == self.value_packed_len
    }

    pub fn is_compressed(&self) -> bool {
        !self.is_empty() && self.compression_ratio < 1.0
    }

    pub fn uses_expected_namespace_bits(
        &self,
        hot_bits: QuantizationBits,
        cold_bits: QuantizationBits,
    ) -> bool {
        if self.is_runtime_namespace {
            self.bits == hot_bits
        } else {
            self.bits == cold_bits
        }
    }

    pub fn uses_hot_runtime_bits(&self, hot_bits: QuantizationBits) -> bool {
        self.is_runtime_namespace && self.bits == hot_bits
    }

    pub fn uses_cold_non_runtime_bits(&self, cold_bits: QuantizationBits) -> bool {
        !self.is_runtime_namespace && self.bits == cold_bits
    }

    pub fn runtime_namespace_signal_component_count(&self) -> usize {
        usize::from(self.is_runtime_namespace)
    }

    pub fn payload_presence_signal_component_count(&self) -> usize {
        usize::from(self.has_key_value_payload())
    }

    pub fn compressed_payload_signal_component_count(&self) -> usize {
        usize::from(self.is_compressed())
    }

    pub fn quantized_payload_signal_component_count(&self) -> usize {
        self.runtime_namespace_signal_component_count()
            .saturating_add(self.payload_presence_signal_component_count())
            .saturating_add(self.compressed_payload_signal_component_count())
    }

    pub fn has_quantized_payload_signals(&self) -> bool {
        self.quantized_payload_signal_component_count() > 0
    }

    pub fn compression_ratio_shape_is_valid(&self) -> bool {
        self.compression_ratio.is_finite() && self.compression_ratio >= 0.0
    }

    pub fn empty_payload_shape_is_valid(&self) -> bool {
        !self.is_empty()
            || (self.key_len == 0
                && self.value_len == 0
                && self.packed_payload_len == 0
                && self.key_packed_len == 0
                && self.value_packed_len == 0
                && (self.compression_ratio - 1.0).abs() <= f32::EPSILON)
    }

    pub fn key_value_length_drift_component_count(&self) -> usize {
        usize::from(!self.key_value_lengths_match())
    }

    pub fn packed_length_drift_component_count(&self) -> usize {
        usize::from(!self.packed_lengths_match())
    }

    pub fn key_value_symmetry_drift_component_count(&self) -> usize {
        usize::from(!self.key_value_lengths_are_symmetric())
    }

    pub fn packed_symmetry_drift_component_count(&self) -> usize {
        usize::from(!self.packed_lengths_are_symmetric())
    }

    pub fn compression_ratio_shape_drift_component_count(&self) -> usize {
        usize::from(!self.compression_ratio_shape_is_valid())
    }

    pub fn empty_payload_shape_drift_component_count(&self) -> usize {
        usize::from(!self.empty_payload_shape_is_valid())
    }

    pub fn namespace_bit_drift_component_count(
        &self,
        hot_bits: QuantizationBits,
        cold_bits: QuantizationBits,
    ) -> usize {
        usize::from(!self.uses_expected_namespace_bits(hot_bits, cold_bits))
    }

    pub fn quantized_payload_problem_component_count(
        &self,
        hot_bits: QuantizationBits,
        cold_bits: QuantizationBits,
    ) -> usize {
        self.key_value_length_drift_component_count()
            .saturating_add(self.packed_length_drift_component_count())
            .saturating_add(self.key_value_symmetry_drift_component_count())
            .saturating_add(self.packed_symmetry_drift_component_count())
            .saturating_add(self.compression_ratio_shape_drift_component_count())
            .saturating_add(self.empty_payload_shape_drift_component_count())
            .saturating_add(self.namespace_bit_drift_component_count(hot_bits, cold_bits))
    }

    pub fn has_quantized_payload_problem_components(
        &self,
        hot_bits: QuantizationBits,
        cold_bits: QuantizationBits,
    ) -> bool {
        self.quantized_payload_problem_component_count(hot_bits, cold_bits) > 0
    }

    pub fn quantized_payload_accounting_is_consistent(
        &self,
        hot_bits: QuantizationBits,
        cold_bits: QuantizationBits,
    ) -> bool {
        let expected_problem_count = usize::from(!self.key_value_lengths_match())
            .saturating_add(usize::from(!self.packed_lengths_match()))
            .saturating_add(usize::from(!self.key_value_lengths_are_symmetric()))
            .saturating_add(usize::from(!self.packed_lengths_are_symmetric()))
            .saturating_add(usize::from(!self.compression_ratio_shape_is_valid()))
            .saturating_add(usize::from(!self.empty_payload_shape_is_valid()))
            .saturating_add(usize::from(
                !self.uses_expected_namespace_bits(hot_bits, cold_bits),
            ));

        self.quantized_payload_problem_component_count(hot_bits, cold_bits)
            == expected_problem_count
            && self.has_quantized_payload_problem_components(hot_bits, cold_bits)
                == (expected_problem_count > 0)
    }

    pub fn quantized_payload_commit_signal_component_count(&self) -> usize {
        self.quantized_payload_signal_component_count()
    }

    pub fn has_quantized_payload_commit_signals(&self) -> bool {
        self.quantized_payload_commit_signal_component_count() > 0
    }

    pub fn quantized_payload_commit_blocker_component_count(
        &self,
        hot_bits: QuantizationBits,
        cold_bits: QuantizationBits,
    ) -> usize {
        self.quantized_payload_problem_component_count(hot_bits, cold_bits)
    }

    pub fn has_quantized_payload_commit_blockers(
        &self,
        hot_bits: QuantizationBits,
        cold_bits: QuantizationBits,
    ) -> bool {
        self.quantized_payload_commit_blocker_component_count(hot_bits, cold_bits) > 0
    }

    pub fn quantized_payload_commit_accounting_is_consistent(
        &self,
        hot_bits: QuantizationBits,
        cold_bits: QuantizationBits,
    ) -> bool {
        self.quantized_payload_accounting_is_consistent(hot_bits, cold_bits)
            && self.quantized_payload_commit_signal_component_count()
                == self.quantized_payload_signal_component_count()
            && self.has_quantized_payload_commit_signals()
                == (self.quantized_payload_commit_signal_component_count() > 0)
            && self.quantized_payload_commit_blocker_component_count(hot_bits, cold_bits)
                == self.quantized_payload_problem_component_count(hot_bits, cold_bits)
            && self.has_quantized_payload_commit_blockers(hot_bits, cold_bits)
                == (self.quantized_payload_commit_blocker_component_count(hot_bits, cold_bits) > 0)
    }

    pub fn quantized_payload_commit_is_clean(
        &self,
        hot_bits: QuantizationBits,
        cold_bits: QuantizationBits,
    ) -> bool {
        !self.has_quantized_payload_commit_blockers(hot_bits, cold_bits)
            && self.quantized_payload_commit_accounting_is_consistent(hot_bits, cold_bits)
    }

    pub fn can_commit_quantized_payload(
        &self,
        hot_bits: QuantizationBits,
        cold_bits: QuantizationBits,
    ) -> bool {
        self.quantized_payload_commit_is_clean(hot_bits, cold_bits)
    }
}

impl QuantizedKvBlock {
    pub fn from_block(block: &KvBlock, bits: QuantizationBits) -> Self {
        Self {
            id: block.id,
            namespace: block.namespace.clone(),
            layer: block.layer,
            head: block.head,
            token_start: block.token_start,
            token_end: block.token_end,
            key: QuantizedVector::quantize(&block.key, bits),
            value: QuantizedVector::quantize(&block.value, bits),
            score: block.score,
            reinforcement: block.reinforcement,
        }
    }

    pub fn dequantize(&self) -> KvBlock {
        KvBlock {
            id: self.id,
            namespace: self.namespace.clone(),
            layer: self.layer,
            head: self.head,
            token_start: self.token_start,
            token_end: self.token_end,
            key: self.key.dequantize(),
            value: self.value.dequantize(),
            score: self.score,
            reinforcement: self.reinforcement,
        }
    }

    pub fn bits(&self) -> QuantizationBits {
        self.key.bits()
    }

    pub fn vector_value_len(&self) -> usize {
        self.key.len().saturating_add(self.value.len())
    }

    pub fn packed_payload_len(&self) -> usize {
        self.key
            .packed_len()
            .saturating_add(self.value.packed_len())
    }

    pub fn compression_ratio(&self) -> f32 {
        let value_len = self.vector_value_len();
        if value_len == 0 {
            1.0
        } else {
            self.packed_payload_len() as f32 / (value_len * std::mem::size_of::<f32>()) as f32
        }
    }

    pub fn payload_summary(&self) -> QuantizedKvPayloadSummary {
        QuantizedKvPayloadSummary {
            namespace_label: self.namespace.label().to_owned(),
            is_runtime_namespace: self.namespace.is_runtime_exchange(),
            bits: self.bits(),
            key_value_len: self.vector_value_len(),
            packed_payload_len: self.packed_payload_len(),
            compression_ratio: self.compression_ratio(),
            key_len: self.key.len(),
            value_len: self.value.len(),
            key_packed_len: self.key.packed_len(),
            value_packed_len: self.value.packed_len(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KvQuantizationPlan {
    pub hot_bits: QuantizationBits,
    pub cold_bits: QuantizationBits,
}

impl KvQuantizationPlan {
    pub fn new(hot_bits: QuantizationBits, cold_bits: QuantizationBits) -> Self {
        Self {
            hot_bits,
            cold_bits: if cold_bits.width() > hot_bits.width() {
                hot_bits
            } else {
                cold_bits
            },
        }
    }

    pub fn from_widths(hot_bits: u8, cold_bits: u8) -> Self {
        let hot_bits = QuantizationBits::from_width(hot_bits).unwrap_or(QuantizationBits::Eight);
        let cold_bits = QuantizationBits::from_width(cold_bits).unwrap_or(QuantizationBits::Four);
        Self::new(hot_bits, cold_bits)
    }

    pub fn bits_for_namespace(&self, namespace: &KvNamespace) -> QuantizationBits {
        if namespace.is_runtime_exchange() {
            self.hot_bits
        } else {
            self.cold_bits
        }
    }

    pub fn quantize_block(&self, block: &KvBlock) -> QuantizedKvBlock {
        QuantizedKvBlock::from_block(block, self.bits_for_namespace(&block.namespace))
    }
}

impl Default for KvQuantizationPlan {
    fn default() -> Self {
        Self::new(QuantizationBits::Eight, QuantizationBits::Four)
    }
}

fn quantization_levels(bits: QuantizationBits) -> u16 {
    match bits {
        QuantizationBits::Four => 15,
        QuantizationBits::Eight => 255,
    }
}

fn quantize_value(value: f32, min: f32, scale: f32, levels: u16) -> u8 {
    if scale == 0.0 {
        return 0;
    }

    ((value - min) / scale).round().clamp(0.0, levels as f32) as u8
}

fn pack_codes(codes: &[u8], bits: QuantizationBits) -> Vec<u8> {
    match bits {
        QuantizationBits::Eight => codes.to_vec(),
        QuantizationBits::Four => {
            let mut packed = Vec::with_capacity(codes.len().div_ceil(2));
            for chunk in codes.chunks(2) {
                let low = chunk[0] & 0x0f;
                let high = chunk.get(1).copied().unwrap_or(0) & 0x0f;
                packed.push(low | (high << 4));
            }
            packed
        }
    }
}

fn unpack_codes(packed: &[u8], bits: QuantizationBits, len: usize) -> Vec<u8> {
    match bits {
        QuantizationBits::Eight => packed.iter().copied().take(len).collect(),
        QuantizationBits::Four => {
            let mut codes = Vec::with_capacity(len);
            for byte in packed {
                if codes.len() < len {
                    codes.push(byte & 0x0f);
                }
                if codes.len() < len {
                    codes.push((byte >> 4) & 0x0f);
                }
            }
            codes
        }
    }
}

fn unpacked_capacity(bits: QuantizationBits, packed_len: usize) -> usize {
    match bits {
        QuantizationBits::Eight => packed_len,
        QuantizationBits::Four => packed_len * 2,
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(hex_char(byte >> 4));
        encoded.push(hex_char(byte & 0x0f));
    }
    encoded
}

fn decode_hex(encoded: &str) -> Result<Vec<u8>, QuantizationError> {
    if !encoded.len().is_multiple_of(2) {
        return Err(QuantizationError::InvalidFormat);
    }

    encoded
        .as_bytes()
        .chunks(2)
        .map(|chunk| {
            let high = hex_value(chunk[0])?;
            let low = hex_value(chunk[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_char(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + value - 10) as char,
        _ => '0',
    }
}

fn hex_value(byte: u8) -> Result<u8, QuantizationError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(QuantizationError::InvalidFormat),
    }
}

fn decode_f32(encoded: &str) -> Result<f32, QuantizationError> {
    let bits = u32::from_str_radix(encoded, 16).map_err(|_| QuantizationError::InvalidFormat)?;
    let value = f32::from_bits(bits);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(QuantizationError::InvalidFormat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_bit_quantization_packs_two_values_per_byte() {
        let vector =
            QuantizedVector::quantize(&[0.0, 0.5, 1.0, f32::NAN, 0.25], QuantizationBits::Four);

        assert_eq!(vector.bits(), QuantizationBits::Four);
        assert_eq!(vector.len(), 5);
        assert_eq!(vector.packed_len(), 3);
        assert!(vector.compression_ratio() < 0.20);
        assert_eq!(vector.dequantize().len(), 5);
    }

    #[test]
    fn encoding_roundtrip_preserves_quantized_payload() {
        let quantized = QuantizedVector::quantize(&[-1.0, 0.0, 1.0, 2.0], QuantizationBits::Eight);

        let encoded = quantized.encode();
        let decoded = QuantizedVector::decode(&encoded).expect("encoded vector should decode");

        assert_eq!(decoded, quantized);
        assert_eq!(decoded.bits(), QuantizationBits::Eight);
    }

    #[test]
    fn invalid_encoding_is_rejected() {
        assert_eq!(
            QuantizedVector::decode("q3:1:00000000:00000000:00").unwrap_err(),
            QuantizationError::InvalidBits
        );
        assert_eq!(
            QuantizedVector::decode("q4:3:00000000:00000000:0").unwrap_err(),
            QuantizationError::InvalidFormat
        );
    }

    #[test]
    fn quantization_plan_uses_hot_bits_only_for_runtime_kv() {
        let plan = KvQuantizationPlan::from_widths(8, 4);
        let runtime = KvBlock::new(
            1,
            KvNamespace::Runtime,
            0,
            0,
            0..4,
            vec![0.0, 0.5, 1.0, 1.5],
            vec![1.5, 1.0, 0.5, 0.0],
        );
        let semantic = KvBlock::new(
            2,
            KvNamespace::Semantic,
            0,
            0,
            0..4,
            vec![0.0, 0.5, 1.0, 1.5],
            vec![1.5, 1.0, 0.5, 0.0],
        );

        let runtime_quantized = plan.quantize_block(&runtime);
        let semantic_quantized = plan.quantize_block(&semantic);

        assert_eq!(runtime_quantized.bits(), QuantizationBits::Eight);
        assert_eq!(semantic_quantized.bits(), QuantizationBits::Four);
        assert_eq!(runtime_quantized.vector_value_len(), 8);
        assert_eq!(runtime_quantized.packed_payload_len(), 8);
        assert_eq!(semantic_quantized.vector_value_len(), 8);
        assert_eq!(semantic_quantized.packed_payload_len(), 4);
        assert!(semantic_quantized.compression_ratio() < runtime_quantized.compression_ratio());
        let runtime_summary = runtime_quantized.payload_summary();
        let semantic_summary = semantic_quantized.payload_summary();

        assert_eq!(runtime_summary.namespace_label, "runtime");
        assert!(runtime_summary.is_runtime_namespace);
        assert_eq!(runtime_summary.bits, QuantizationBits::Eight);
        assert_eq!(runtime_summary.key_value_len, 8);
        assert_eq!(runtime_summary.packed_payload_len, 8);
        assert_eq!(runtime_summary.key_len, 4);
        assert_eq!(runtime_summary.value_len, 4);
        assert_eq!(runtime_summary.key_packed_len, 4);
        assert_eq!(runtime_summary.value_packed_len, 4);
        assert!(!runtime_summary.is_empty());
        assert!(runtime_summary.key_value_lengths_match());
        assert!(runtime_summary.packed_lengths_match());
        assert!(runtime_summary.payload_shape_balanced());
        assert!(runtime_summary.has_key_value_payload());
        assert!(runtime_summary.key_value_lengths_are_symmetric());
        assert!(runtime_summary.packed_lengths_are_symmetric());
        assert!(runtime_summary.is_compressed());
        assert!(runtime_summary.uses_hot_runtime_bits(QuantizationBits::Eight));
        assert!(
            runtime_summary
                .uses_expected_namespace_bits(QuantizationBits::Eight, QuantizationBits::Four)
        );
        assert_eq!(
            runtime_summary.runtime_namespace_signal_component_count(),
            1
        );
        assert_eq!(runtime_summary.payload_presence_signal_component_count(), 1);
        assert_eq!(
            runtime_summary.compressed_payload_signal_component_count(),
            1
        );
        assert_eq!(
            runtime_summary.quantized_payload_signal_component_count(),
            3
        );
        assert!(runtime_summary.has_quantized_payload_signals());
        assert!(runtime_summary.compression_ratio_shape_is_valid());
        assert!(runtime_summary.empty_payload_shape_is_valid());
        assert_eq!(runtime_summary.key_value_length_drift_component_count(), 0);
        assert_eq!(runtime_summary.packed_length_drift_component_count(), 0);
        assert_eq!(
            runtime_summary.key_value_symmetry_drift_component_count(),
            0
        );
        assert_eq!(runtime_summary.packed_symmetry_drift_component_count(), 0);
        assert_eq!(
            runtime_summary.compression_ratio_shape_drift_component_count(),
            0
        );
        assert_eq!(
            runtime_summary.empty_payload_shape_drift_component_count(),
            0
        );
        assert_eq!(
            runtime_summary.namespace_bit_drift_component_count(
                QuantizationBits::Eight,
                QuantizationBits::Four
            ),
            0
        );
        assert_eq!(
            runtime_summary.quantized_payload_problem_component_count(
                QuantizationBits::Eight,
                QuantizationBits::Four,
            ),
            0
        );
        assert!(!runtime_summary.has_quantized_payload_problem_components(
            QuantizationBits::Eight,
            QuantizationBits::Four,
        ));
        assert!(runtime_summary.quantized_payload_accounting_is_consistent(
            QuantizationBits::Eight,
            QuantizationBits::Four,
        ));
        assert_eq!(
            runtime_summary.quantized_payload_commit_signal_component_count(),
            3
        );
        assert!(runtime_summary.has_quantized_payload_commit_signals());
        assert_eq!(
            runtime_summary.quantized_payload_commit_blocker_component_count(
                QuantizationBits::Eight,
                QuantizationBits::Four,
            ),
            0
        );
        assert!(!runtime_summary.has_quantized_payload_commit_blockers(
            QuantizationBits::Eight,
            QuantizationBits::Four,
        ));
        assert!(
            runtime_summary.quantized_payload_commit_accounting_is_consistent(
                QuantizationBits::Eight,
                QuantizationBits::Four,
            )
        );
        assert!(
            runtime_summary
                .quantized_payload_commit_is_clean(QuantizationBits::Eight, QuantizationBits::Four)
        );
        assert!(
            runtime_summary
                .can_commit_quantized_payload(QuantizationBits::Eight, QuantizationBits::Four)
        );

        assert_eq!(semantic_summary.namespace_label, "semantic");
        assert!(!semantic_summary.is_runtime_namespace);
        assert_eq!(semantic_summary.bits, QuantizationBits::Four);
        assert_eq!(semantic_summary.packed_payload_len, 4);
        assert!(semantic_summary.compression_ratio < runtime_summary.compression_ratio);
        assert!(semantic_summary.uses_cold_non_runtime_bits(QuantizationBits::Four));
        assert!(semantic_summary.payload_shape_balanced());
        assert!(semantic_summary.key_value_lengths_are_symmetric());
        assert!(semantic_summary.packed_lengths_are_symmetric());
        assert!(semantic_summary.is_compressed());
        assert!(
            semantic_summary
                .uses_expected_namespace_bits(QuantizationBits::Eight, QuantizationBits::Four)
        );
        assert_eq!(
            semantic_summary.runtime_namespace_signal_component_count(),
            0
        );
        assert_eq!(
            semantic_summary.payload_presence_signal_component_count(),
            1
        );
        assert_eq!(
            semantic_summary.compressed_payload_signal_component_count(),
            1
        );
        assert_eq!(
            semantic_summary.quantized_payload_signal_component_count(),
            2
        );
        assert!(semantic_summary.has_quantized_payload_signals());
        assert_eq!(
            semantic_summary.quantized_payload_problem_component_count(
                QuantizationBits::Eight,
                QuantizationBits::Four,
            ),
            0
        );
        assert!(!semantic_summary.has_quantized_payload_problem_components(
            QuantizationBits::Eight,
            QuantizationBits::Four,
        ));
        assert_eq!(
            semantic_summary.quantized_payload_commit_signal_component_count(),
            2
        );
        assert!(semantic_summary.has_quantized_payload_commit_signals());
        assert_eq!(
            semantic_summary.quantized_payload_commit_blocker_component_count(
                QuantizationBits::Eight,
                QuantizationBits::Four,
            ),
            0
        );
        assert!(!semantic_summary.has_quantized_payload_commit_blockers(
            QuantizationBits::Eight,
            QuantizationBits::Four,
        ));
        assert!(
            semantic_summary.quantized_payload_commit_accounting_is_consistent(
                QuantizationBits::Eight,
                QuantizationBits::Four,
            )
        );
        assert!(
            semantic_summary
                .quantized_payload_commit_is_clean(QuantizationBits::Eight, QuantizationBits::Four)
        );
        assert!(
            semantic_summary
                .can_commit_quantized_payload(QuantizationBits::Eight, QuantizationBits::Four)
        );
        assert_eq!(runtime_quantized.dequantize().token_len(), 4);
        assert_eq!(
            semantic_quantized.dequantize().namespace,
            KvNamespace::Semantic
        );
    }

    #[test]
    fn quantized_kv_payload_summary_marks_empty_payloads() {
        let block = KvBlock::new(9, KvNamespace::Runtime, 0, 0, 0..1, Vec::new(), Vec::new());
        let quantized = QuantizedKvBlock::from_block(&block, QuantizationBits::Four);

        let summary = quantized.payload_summary();

        assert!(summary.is_empty());
        assert_eq!(summary.key_value_len, 0);
        assert_eq!(summary.packed_payload_len, 0);
        assert_eq!(summary.compression_ratio, 1.0);
        assert!(summary.key_value_lengths_match());
        assert!(summary.packed_lengths_match());
        assert!(summary.payload_shape_balanced());
        assert!(!summary.has_key_value_payload());
        assert!(summary.key_value_lengths_are_symmetric());
        assert!(summary.packed_lengths_are_symmetric());
        assert!(!summary.is_compressed());
        assert_eq!(summary.runtime_namespace_signal_component_count(), 1);
        assert_eq!(summary.payload_presence_signal_component_count(), 0);
        assert_eq!(summary.compressed_payload_signal_component_count(), 0);
        assert_eq!(summary.quantized_payload_signal_component_count(), 1);
        assert!(summary.has_quantized_payload_signals());
        assert!(summary.compression_ratio_shape_is_valid());
        assert!(summary.empty_payload_shape_is_valid());
        assert_eq!(
            summary.quantized_payload_problem_component_count(
                QuantizationBits::Four,
                QuantizationBits::Four,
            ),
            0
        );
        assert!(!summary.has_quantized_payload_problem_components(
            QuantizationBits::Four,
            QuantizationBits::Four,
        ));
        assert!(summary.quantized_payload_accounting_is_consistent(
            QuantizationBits::Four,
            QuantizationBits::Four,
        ));
        assert_eq!(summary.quantized_payload_commit_signal_component_count(), 1);
        assert!(summary.has_quantized_payload_commit_signals());
        assert_eq!(
            summary.quantized_payload_commit_blocker_component_count(
                QuantizationBits::Four,
                QuantizationBits::Four,
            ),
            0
        );
        assert!(
            !summary.has_quantized_payload_commit_blockers(
                QuantizationBits::Four,
                QuantizationBits::Four,
            )
        );
        assert!(summary.quantized_payload_commit_accounting_is_consistent(
            QuantizationBits::Four,
            QuantizationBits::Four,
        ));
        assert!(
            summary
                .quantized_payload_commit_is_clean(QuantizationBits::Four, QuantizationBits::Four)
        );
        assert!(
            summary.can_commit_quantized_payload(QuantizationBits::Four, QuantizationBits::Four)
        );
    }

    #[test]
    fn quantized_kv_payload_summary_reports_shape_drift() {
        let summary = QuantizedKvPayloadSummary {
            namespace_label: "runtime".to_owned(),
            is_runtime_namespace: true,
            bits: QuantizationBits::Four,
            key_value_len: 4,
            packed_payload_len: 4,
            compression_ratio: f32::NAN,
            key_len: 1,
            value_len: 2,
            key_packed_len: 1,
            value_packed_len: 2,
        };

        assert!(!summary.is_empty());
        assert!(summary.has_key_value_payload());
        assert!(!summary.key_value_lengths_match());
        assert!(!summary.packed_lengths_match());
        assert!(!summary.key_value_lengths_are_symmetric());
        assert!(!summary.packed_lengths_are_symmetric());
        assert!(!summary.is_compressed());
        assert!(
            !summary.uses_expected_namespace_bits(QuantizationBits::Eight, QuantizationBits::Four)
        );
        assert_eq!(summary.runtime_namespace_signal_component_count(), 1);
        assert_eq!(summary.payload_presence_signal_component_count(), 1);
        assert_eq!(summary.compressed_payload_signal_component_count(), 0);
        assert_eq!(summary.quantized_payload_signal_component_count(), 2);
        assert!(summary.has_quantized_payload_signals());
        assert!(!summary.compression_ratio_shape_is_valid());
        assert!(summary.empty_payload_shape_is_valid());
        assert_eq!(summary.key_value_length_drift_component_count(), 1);
        assert_eq!(summary.packed_length_drift_component_count(), 1);
        assert_eq!(summary.key_value_symmetry_drift_component_count(), 1);
        assert_eq!(summary.packed_symmetry_drift_component_count(), 1);
        assert_eq!(summary.compression_ratio_shape_drift_component_count(), 1);
        assert_eq!(summary.empty_payload_shape_drift_component_count(), 0);
        assert_eq!(
            summary.namespace_bit_drift_component_count(
                QuantizationBits::Eight,
                QuantizationBits::Four,
            ),
            1
        );
        assert_eq!(
            summary.quantized_payload_problem_component_count(
                QuantizationBits::Eight,
                QuantizationBits::Four,
            ),
            6
        );
        assert!(summary.has_quantized_payload_problem_components(
            QuantizationBits::Eight,
            QuantizationBits::Four,
        ));
        assert!(summary.quantized_payload_accounting_is_consistent(
            QuantizationBits::Eight,
            QuantizationBits::Four,
        ));
        assert_eq!(summary.quantized_payload_commit_signal_component_count(), 2);
        assert!(summary.has_quantized_payload_commit_signals());
        assert_eq!(
            summary.quantized_payload_commit_blocker_component_count(
                QuantizationBits::Eight,
                QuantizationBits::Four,
            ),
            6
        );
        assert!(summary.has_quantized_payload_commit_blockers(
            QuantizationBits::Eight,
            QuantizationBits::Four,
        ));
        assert!(summary.quantized_payload_commit_accounting_is_consistent(
            QuantizationBits::Eight,
            QuantizationBits::Four,
        ));
        assert!(
            !summary
                .quantized_payload_commit_is_clean(QuantizationBits::Eight, QuantizationBits::Four)
        );
        assert!(
            !summary.can_commit_quantized_payload(QuantizationBits::Eight, QuantizationBits::Four)
        );
    }
}
