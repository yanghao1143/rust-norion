use super::*;

#[test]
fn gemma_business_smoke_preflight_rejects_remote_tokened_runtime() {
    let args = Args::parse(vec!["--gemma-business-smoke".to_owned()]);
    let failures = gemma_business_smoke_preflight_failures(&args);

    assert!(args.gemma_business_smoke);
    assert!(args.gemma_12b_runtime);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("--token-source none")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("existing local snapshot")),
        "{failures:?}"
    );
}

#[test]
fn gemma_smoke_check_only_flag_builds_read_only_safety_report() {
    let args = Args::parse(vec![
        "--gemma-model-service-smoke".to_owned(),
        "--gemma-smoke-check-only".to_owned(),
        "--gemma-runtime-server".to_owned(),
        "http://127.0.0.1:9".to_owned(),
    ]);
    let failures = gemma_business_smoke_preflight_failures(&args);
    let report =
        crate::gemma_business::preflight::gemma_business_smoke_check_only_report(&args, &failures);
    let text = report.lines().join("\n");

    assert!(args.gemma_model_service_smoke);
    assert!(args.gemma_smoke_check_only);
    assert!(!report.passed());
    assert!(text.contains("starts_model=false writes_ndkv=false"));
    assert!(text.contains("state_dir:"));
    assert!(text.contains("serve_bind: 127.0.0.1:7878"));
    assert!(text.contains("backend_health: mode=gemma-http reachable=false"));
    assert!(text.contains("ram_vram:"));
    assert!(text.contains("experience_safety:"));
}

#[test]
fn gemma_business_smoke_preflight_rejects_incomplete_local_snapshot() {
    let snapshot = target_asset_dir("gemma-incomplete-snapshot");
    fs::create_dir_all(&snapshot).unwrap();
    let args = Args::parse(vec![
        "--gemma-business-smoke".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        snapshot.display().to_string(),
    ]);
    let failures = gemma_business_smoke_preflight_failures(&args);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("missing config.json")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("missing tokenizer asset")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("missing weight asset")),
        "{failures:?}"
    );

    fs::remove_dir_all(snapshot).unwrap();
}

#[test]
fn gemma_business_smoke_preflight_accepts_minimal_local_snapshot_assets() {
    let snapshot = target_asset_dir("gemma-complete-snapshot");
    write_minimal_gemma_snapshot(&snapshot);
    let args = Args::parse(vec![
        "--gemma-business-smoke".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        snapshot.display().to_string(),
    ]);
    let failures = gemma_business_smoke_preflight_failures(&args);

    assert!(failures.is_empty(), "{failures:?}");

    fs::remove_dir_all(snapshot).unwrap();
}

#[test]
fn gemma_business_smoke_preflight_accepts_alternate_tokenizer_asset() {
    let snapshot = target_asset_dir("gemma-tokenizer-model-snapshot");
    write_gemma_snapshot_with_assets(
        &snapshot,
        &[
            "config.json",
            "tokenizer.model",
            "model-00001-of-00001.safetensors",
        ],
    );
    let args = Args::parse(vec![
        "--gemma-business-smoke".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        snapshot.display().to_string(),
    ]);
    let failures = gemma_business_smoke_preflight_failures(&args);

    assert!(failures.is_empty(), "{failures:?}");

    fs::remove_dir_all(snapshot).unwrap();
}

#[test]
fn gemma_business_smoke_preflight_accepts_weight_index_asset() {
    let snapshot = target_asset_dir("gemma-weight-index-snapshot");
    write_gemma_snapshot_with_assets(
        &snapshot,
        &[
            "config.json",
            "tokenizer.json",
            "model.safetensors.index.json",
        ],
    );
    let args = Args::parse(vec![
        "--gemma-business-smoke".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        snapshot.display().to_string(),
    ]);
    let failures = gemma_business_smoke_preflight_failures(&args);

    assert!(failures.is_empty(), "{failures:?}");

    fs::remove_dir_all(snapshot).unwrap();
}

#[test]
fn gemma_business_smoke_preflight_rejects_snapshot_file_path() {
    let snapshot = target_asset_dir("gemma-snapshot-file-path");
    fs::create_dir_all(snapshot.parent().unwrap()).unwrap();
    File::create(&snapshot).unwrap();
    let args = Args::parse(vec![
        "--gemma-business-smoke".to_owned(),
        "--gemma-local-snapshot".to_owned(),
        snapshot.display().to_string(),
    ]);
    let failures = gemma_business_smoke_preflight_failures(&args);

    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("local snapshot must be a directory")),
        "{failures:?}"
    );

    fs::remove_file(snapshot).unwrap();
}

fn write_gemma_snapshot_with_assets(snapshot_dir: &Path, assets: &[&str]) {
    fs::create_dir_all(snapshot_dir).unwrap();
    for asset in assets {
        File::create(snapshot_dir.join(asset)).unwrap();
    }
}
