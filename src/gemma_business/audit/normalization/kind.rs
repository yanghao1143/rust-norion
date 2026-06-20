#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GemmaModelServiceBusinessNormalizationKind {
    RawDirect,
    Sanitized,
    CanonicalFallback,
}

impl GemmaModelServiceBusinessNormalizationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RawDirect => "raw_direct",
            Self::Sanitized => "sanitized",
            Self::CanonicalFallback => "canonical_fallback",
        }
    }

    pub fn response_normalized(self) -> bool {
        matches!(self, Self::Sanitized | Self::CanonicalFallback)
    }

    pub fn canonical_fallback(self) -> bool {
        matches!(self, Self::CanonicalFallback)
    }
}
