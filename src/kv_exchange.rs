use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeKvBlock {
    pub layer: usize,
    pub head: usize,
    pub token_start: usize,
    pub token_end: usize,
    pub key: Vec<f32>,
    pub value: Vec<f32>,
}

impl RuntimeKvBlock {
    pub fn new(
        layer: usize,
        head: usize,
        token_start: usize,
        token_end: usize,
        key: Vec<f32>,
        value: Vec<f32>,
    ) -> Self {
        Self {
            layer,
            head,
            token_start,
            token_end,
            key,
            value,
        }
    }

    pub fn vector(&self) -> Vec<f32> {
        self.key
            .iter()
            .chain(self.value.iter())
            .copied()
            .collect::<Vec<_>>()
    }

    pub fn is_empty(&self) -> bool {
        self.key.is_empty() && self.value.is_empty()
    }

    pub fn validate_shape(
        &self,
        max_layers: usize,
        max_heads: usize,
        dimensions: Option<usize>,
    ) -> Result<(), RuntimeKvBlockValidationError> {
        if self.token_start >= self.token_end {
            return Err(RuntimeKvBlockValidationError::EmptyTokenRange);
        }
        if self.layer >= max_layers.max(1) {
            return Err(RuntimeKvBlockValidationError::LayerOutOfRange {
                layer: self.layer,
                max_layers: max_layers.max(1),
            });
        }
        if self.head >= max_heads.max(1) {
            return Err(RuntimeKvBlockValidationError::HeadOutOfRange {
                head: self.head,
                max_heads: max_heads.max(1),
            });
        }
        if self.key.is_empty() {
            return Err(RuntimeKvBlockValidationError::EmptyKey);
        }
        if self.value.is_empty() {
            return Err(RuntimeKvBlockValidationError::EmptyValue);
        }
        if self.key.iter().any(|value| !value.is_finite()) {
            return Err(RuntimeKvBlockValidationError::NonFiniteKey);
        }
        if self.value.iter().any(|value| !value.is_finite()) {
            return Err(RuntimeKvBlockValidationError::NonFiniteValue);
        }
        if let Some(expected) = dimensions.filter(|value| *value > 0) {
            if self.key.len() != expected {
                return Err(RuntimeKvBlockValidationError::KeyDimensions {
                    actual: self.key.len(),
                    expected,
                });
            }
            if self.value.len() != expected {
                return Err(RuntimeKvBlockValidationError::ValueDimensions {
                    actual: self.value.len(),
                    expected,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeKvBlockValidationError {
    EmptyTokenRange,
    LayerOutOfRange { layer: usize, max_layers: usize },
    HeadOutOfRange { head: usize, max_heads: usize },
    EmptyKey,
    EmptyValue,
    NonFiniteKey,
    NonFiniteValue,
    KeyDimensions { actual: usize, expected: usize },
    ValueDimensions { actual: usize, expected: usize },
}

impl fmt::Display for RuntimeKvBlockValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTokenRange => write!(f, "token range is empty or reversed"),
            Self::LayerOutOfRange { layer, max_layers } => {
                write!(
                    f,
                    "layer {layer} is outside runtime layer count {max_layers}"
                )
            }
            Self::HeadOutOfRange { head, max_heads } => {
                write!(
                    f,
                    "head {head} is outside runtime KV head count {max_heads}"
                )
            }
            Self::EmptyKey => write!(f, "key vector is empty"),
            Self::EmptyValue => write!(f, "value vector is empty"),
            Self::NonFiniteKey => write!(f, "key vector contains non-finite values"),
            Self::NonFiniteValue => write!(f, "value vector contains non-finite values"),
            Self::KeyDimensions { actual, expected } => {
                write!(
                    f,
                    "key vector dimensions {actual} do not match expected {expected}"
                )
            }
            Self::ValueDimensions { actual, expected } => write!(
                f,
                "value vector dimensions {actual} do not match expected {expected}"
            ),
        }
    }
}
