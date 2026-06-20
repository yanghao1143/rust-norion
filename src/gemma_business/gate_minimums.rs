pub(in crate::gemma_business) fn require_min_usize(value: &mut Option<usize>, minimum: usize) {
    *value = Some(value.unwrap_or(0).max(minimum));
}

pub(in crate::gemma_business) fn require_min_u64(value: &mut Option<u64>, minimum: u64) {
    *value = Some(value.unwrap_or(0).max(minimum));
}

pub(in crate::gemma_business) fn require_min_f32(value: &mut Option<f32>, minimum: f32) {
    *value = Some(value.unwrap_or(0.0).max(minimum));
}

#[cfg(test)]
mod tests {
    use super::{require_min_f32, require_min_u64, require_min_usize};

    #[test]
    fn minimum_helpers_fill_missing_values() {
        let mut usize_value = None;
        let mut u64_value = None;
        let mut f32_value = None;

        require_min_usize(&mut usize_value, 3);
        require_min_u64(&mut u64_value, 5);
        require_min_f32(&mut f32_value, 0.75);

        assert_eq!(usize_value, Some(3));
        assert_eq!(u64_value, Some(5));
        assert_eq!(f32_value, Some(0.75));
    }

    #[test]
    fn minimum_helpers_keep_stronger_existing_values() {
        let mut usize_value = Some(7);
        let mut u64_value = Some(11);
        let mut f32_value = Some(0.9);

        require_min_usize(&mut usize_value, 3);
        require_min_u64(&mut u64_value, 5);
        require_min_f32(&mut f32_value, 0.75);

        assert_eq!(usize_value, Some(7));
        assert_eq!(u64_value, Some(11));
        assert_eq!(f32_value, Some(0.9));
    }
}
