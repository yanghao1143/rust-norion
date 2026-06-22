use std::path::PathBuf;

pub fn infer_hf_cache_from_local_snapshot(model_id: &str) -> Option<PathBuf> {
    let separator = if model_id.contains('\\') { "\\" } else { "/" };
    let normalized = model_id.replace('\\', "/");
    let segments = normalized.split('/').collect::<Vec<_>>();
    let hub_index = segments
        .iter()
        .position(|segment| segment.eq_ignore_ascii_case("hub"))?;

    let root_segments = &segments[..hub_index];
    if root_segments.is_empty() {
        return None;
    }

    let root = root_segments.join(separator);
    if root.is_empty() && normalized.starts_with('/') {
        Some(PathBuf::from("/"))
    } else if root.is_empty() {
        None
    } else {
        Some(PathBuf::from(root))
    }
}
