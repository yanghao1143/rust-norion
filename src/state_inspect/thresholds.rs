pub(super) fn require_min_usize(
    failures: &mut Vec<String>,
    name: &str,
    actual: usize,
    required: Option<usize>,
) {
    if let Some(required) = required
        && actual < required
    {
        failures.push(format!("{name} {actual} below required {required}"));
    }
}

pub(super) fn require_max_usize(
    failures: &mut Vec<String>,
    name: &str,
    actual: usize,
    maximum: Option<usize>,
) {
    if let Some(maximum) = maximum
        && actual > maximum
    {
        failures.push(format!("{name} {actual} above maximum {maximum}"));
    }
}

pub(super) fn require_min_u64(
    failures: &mut Vec<String>,
    name: &str,
    actual: u64,
    required: Option<u64>,
) {
    if let Some(required) = required
        && actual < required
    {
        failures.push(format!("{name} {actual} below required {required}"));
    }
}

pub(super) fn require_max_u64(
    failures: &mut Vec<String>,
    name: &str,
    actual: u64,
    maximum: Option<u64>,
) {
    if let Some(maximum) = maximum
        && actual > maximum
    {
        failures.push(format!("{name} {actual} above maximum {maximum}"));
    }
}

pub(super) fn require_min_f32(
    failures: &mut Vec<String>,
    name: &str,
    actual: f32,
    required: Option<f32>,
) {
    if let Some(required) = required
        && actual < required
    {
        failures.push(format!("{name} {actual:.6} below required {required:.6}"));
    }
}

pub(super) fn require_max_f32(
    failures: &mut Vec<String>,
    name: &str,
    actual: f32,
    maximum: Option<f32>,
) {
    if let Some(maximum) = maximum
        && actual > maximum
    {
        failures.push(format!("{name} {actual:.6} above maximum {maximum:.6}"));
    }
}
