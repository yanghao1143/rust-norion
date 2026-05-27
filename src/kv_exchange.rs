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
}
