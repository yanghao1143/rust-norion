pub(super) fn mean_abs(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().map(|value| value.abs()).sum::<f32>() / values.len() as f32
}

pub(super) fn scaled(values: &[f32], scale: f32) -> Vec<f32> {
    values.iter().map(|value| value * scale).collect()
}
