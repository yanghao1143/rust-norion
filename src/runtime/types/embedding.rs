#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeEmbedding {
    pub dimensions: usize,
    pub values: Vec<f32>,
}

impl RuntimeEmbedding {
    pub fn new(values: Vec<f32>) -> Self {
        Self {
            dimensions: values.len(),
            values,
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }
}
