use std::path::PathBuf;

pub fn infer_hf_cache_from_local_snapshot(model_id: &str) -> Option<PathBuf> {
    let path = PathBuf::from(model_id);
    path.ancestors()
        .find(|ancestor| {
            ancestor
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.eq_ignore_ascii_case("hub"))
                .unwrap_or(false)
        })
        .and_then(|hub| hub.parent())
        .map(PathBuf::from)
}
