use super::QuantizationBits;

pub(crate) fn quantize_value(value: f32, min: f32, scale: f32, levels: u16) -> u8 {
    if scale == 0.0 {
        return 0;
    }

    ((value - min) / scale).round().clamp(0.0, levels as f32) as u8
}

pub(crate) fn pack_codes(codes: &[u8], bits: QuantizationBits) -> Vec<u8> {
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

pub(crate) fn unpack_codes(packed: &[u8], bits: QuantizationBits, len: usize) -> Vec<u8> {
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

pub(crate) fn unpacked_capacity(bits: QuantizationBits, packed_len: usize) -> usize {
    match bits {
        QuantizationBits::Eight => packed_len,
        QuantizationBits::Four => packed_len * 2,
    }
}
