use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizationBits {
    Four,
    Eight,
}

impl QuantizationBits {
    pub fn width(self) -> u8 {
        match self {
            Self::Four => 4,
            Self::Eight => 8,
        }
    }

    fn levels(self) -> u16 {
        match self {
            Self::Four => 15,
            Self::Eight => 255,
        }
    }

    fn from_width(width: u8) -> Option<Self> {
        match width {
            4 => Some(Self::Four),
            8 => Some(Self::Eight),
            _ => None,
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizationError {
    InvalidBits,
    InvalidFormat,
    InvalidHex,
    InvalidLength,
}

impl fmt::Display for QuantizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBits => formatter.write_str("invalid quantization bit width"),
            Self::InvalidFormat => formatter.write_str("invalid quantized vector format"),
            Self::InvalidHex => formatter.write_str("invalid quantized vector hex payload"),
            Self::InvalidLength => formatter.write_str("invalid quantized vector length"),
        }
    }
}

impl std::error::Error for QuantizationError {}

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

fn decode_f32(encoded: &str) -> Result<f32, QuantizationError> {
    let bits = u32::from_str_radix(encoded, 16).map_err(|_| QuantizationError::InvalidHex)?;
    Ok(f32::from_bits(bits))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }

    out
}

fn decode_hex(encoded: &str) -> Result<Vec<u8>, QuantizationError> {
    if encoded.len() % 2 != 0 {
        return Err(QuantizationError::InvalidHex);
    }

    encoded
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let high = hex_value(pair[0])?;
            let low = hex_value(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_value(byte: u8) -> Result<u8, QuantizationError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(QuantizationError::InvalidHex),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_bit_roundtrip_keeps_small_error() {
        let vector = vec![-1.0, -0.25, 0.0, 0.25, 0.9, 1.0];
        let quantized = QuantizedVector::quantize(&vector, QuantizationBits::Eight);
        let restored = quantized.dequantize();

        assert_eq!(restored.len(), vector.len());
        for (left, right) in vector.iter().zip(restored) {
            assert!((left - right).abs() <= 0.01);
        }
    }

    #[test]
    fn four_bit_packs_two_values_per_byte() {
        let vector = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let quantized = QuantizedVector::quantize(&vector, QuantizationBits::Four);

        assert_eq!(quantized.packed_len(), 3);
        assert!(quantized.compression_ratio() < 0.2);
        assert_eq!(quantized.dequantize().len(), vector.len());
    }

    #[test]
    fn encoding_roundtrip_preserves_quantized_payload() {
        let vector = vec![0.3, 0.7, -0.2, 1.4];
        let quantized = QuantizedVector::quantize(&vector, QuantizationBits::Four);
        let encoded = quantized.encode();
        let decoded = QuantizedVector::decode(&encoded).unwrap();

        assert_eq!(decoded, quantized);
        assert_eq!(decoded.bits(), QuantizationBits::Four);
    }

    #[test]
    fn invalid_bit_width_is_rejected() {
        assert_eq!(
            QuantizedVector::decode("q3:1:00000000:00000000:00").unwrap_err(),
            QuantizationError::InvalidBits
        );
    }
}
