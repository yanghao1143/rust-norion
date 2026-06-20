#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemmaRuntimeQuantizationMode {
    Quant,
    Isq,
}

impl GemmaRuntimeQuantizationMode {
    pub fn flag(self) -> &'static str {
        match self {
            Self::Quant => "--quant",
            Self::Isq => "--isq",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quant => "quant",
            Self::Isq => "isq",
        }
    }
}
