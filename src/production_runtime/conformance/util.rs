use super::super::util::{normalize, stable_hash};

pub(super) fn deterministic_vector(seed: &str, dims: usize) -> Vec<f32> {
    let dims = dims.max(1);
    let mut vector = (0..dims)
        .map(|index| {
            let hash = stable_hash(&format!("{seed}:{index}"));
            ((hash % 997) as f32 / 997.0) - 0.5
        })
        .collect::<Vec<_>>();
    normalize(&mut vector);
    vector
}

pub(super) fn option_f32_display(value: Option<f32>) -> String {
    value
        .filter(|value| value.is_finite())
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "none".to_owned())
}
