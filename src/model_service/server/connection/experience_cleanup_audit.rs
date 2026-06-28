use std::net::TcpStream;

use rust_norion::ExperienceStore;

use super::super::super::json::write_http_json;
use super::super::super::request::ModelServiceExperienceCleanupAuditRequest;
use super::super::super::response::{
    model_service_experience_cleanup_audit_response_json, ModelServiceExperienceCleanupAuditView,
};
use crate::Args;

const MAX_INLINE_EXPERIENCE_CLEANUP_AUDIT_BYTES: u64 = 1_000_000;

pub(super) fn handle_experience_cleanup_audit(
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceExperienceCleanupAuditRequest,
) -> std::io::Result<()> {
    let sample_limit = request
        .limit
        .unwrap_or(args.experience_cleanup_audit_limit)
        .max(1);
    if !args.experience_path.exists() {
        let body = model_service_experience_cleanup_audit_response_json(
            ModelServiceExperienceCleanupAuditView {
                request_id,
                experience_path: &args.experience_path,
                sample_limit,
                hygiene: None,
                quarantine: None,
                repair: None,
                index: None,
                error: Some("experience_file_missing"),
            },
        );
        return write_http_json(stream, 200, "OK", &body);
    }

    if let Ok(metadata) = std::fs::metadata(&args.experience_path) {
        let size_bytes = metadata.len();
        if size_bytes > MAX_INLINE_EXPERIENCE_CLEANUP_AUDIT_BYTES {
            let error = format!(
                "experience_hygiene_deferred_large_file: size_bytes={} max_inline_bytes={}; run CLI --experience-cleanup-audit for offline full audit",
                size_bytes, MAX_INLINE_EXPERIENCE_CLEANUP_AUDIT_BYTES
            );
            let body = model_service_experience_cleanup_audit_response_json(
                ModelServiceExperienceCleanupAuditView {
                    request_id,
                    experience_path: &args.experience_path,
                    sample_limit,
                    hygiene: None,
                    quarantine: None,
                    repair: None,
                    index: None,
                    error: Some(error.as_str()),
                },
            );
            return write_http_json(stream, 200, "OK", &body);
        }
    }

    let store = ExperienceStore::load_from_disk_kv_read_only(&args.experience_path)?;
    let hygiene = store.hygiene_report(sample_limit);
    let quarantine = store.hygiene_quarantine_plan(sample_limit);
    let repair = store.legacy_metadata_repair_plan(sample_limit);
    let index = store.index_report(sample_limit);
    let body = model_service_experience_cleanup_audit_response_json(
        ModelServiceExperienceCleanupAuditView {
            request_id,
            experience_path: &args.experience_path,
            sample_limit,
            hygiene: Some(&hygiene),
            quarantine: Some(&quarantine),
            repair: Some(&repair),
            index: Some(&index),
            error: None,
        },
    );
    write_http_json(stream, 200, "OK", &body)
}
