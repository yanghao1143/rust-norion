use super::{
    bits::QuantizationBits,
    error::QuantizationError,
    hex::{decode_f32, decode_hex, encode_hex},
    packing::{pack_codes, quantize_value, unpack_codes, unpacked_capacity},
};

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
            range / bits.levels() as f32
        };
        let codes = sanitized
            .iter()
            .map(|value| quantize_value(*value, min, scale, bits.levels()))
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
