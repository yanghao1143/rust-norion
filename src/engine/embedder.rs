use super::text::{char_weight, hash_char};

#[derive(Debug, Clone)]
pub(super) struct TextEmbedder {
    dimensions: usize,
}

impl Default for TextEmbedder {
    fn default() -> Self {
        Self { dimensions: 64 }
    }
}

impl TextEmbedder {
    pub(super) fn embed(&self, text: &str) -> Vec<f32> {
        let mut vector = vec![0.0; self.dimensions];

        for ch in text.chars().filter(|ch| !ch.is_whitespace()) {
            let index = hash_char(ch) % self.dimensions;
            vector[index] += char_weight(ch);
        }

        let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in &mut vector {
                *value /= norm;
            }
        }

        vector
    }
}
