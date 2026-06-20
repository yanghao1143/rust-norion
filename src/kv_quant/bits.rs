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

    pub(crate) fn levels(self) -> u16 {
        match self {
            Self::Four => 15,
            Self::Eight => 255,
        }
    }

    pub fn from_width(width: u8) -> Option<Self> {
        match width {
            4 => Some(Self::Four),
            8 => Some(Self::Eight),
            _ => None,
        }
    }
}
