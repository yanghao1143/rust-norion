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
