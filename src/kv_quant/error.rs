use std::fmt;

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
