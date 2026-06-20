use super::QuantizationError;

pub(crate) fn decode_f32(encoded: &str) -> Result<f32, QuantizationError> {
    let bits = u32::from_str_radix(encoded, 16).map_err(|_| QuantizationError::InvalidHex)?;
    Ok(f32::from_bits(bits))
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }

    out
}

pub(crate) fn decode_hex(encoded: &str) -> Result<Vec<u8>, QuantizationError> {
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
