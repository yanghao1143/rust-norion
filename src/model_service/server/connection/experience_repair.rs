use std::fs;
use std::net::TcpStream;

use rust_norion::ExperienceStore;

use super::super::super::json::write_http_json;
use super::super::super::request::ModelServiceExperienceRepairRequest;
use super::super::super::response::{
    ModelServiceExperienceRepairView, model_service_experience_repair_response_json,
};
use crate::Args;
use crate::path_utils::{ensure_parent_dir, timestamped_sidecar_path};

pub(super) fn handle_experience_repair(
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceExperienceRepairRequest,
) -> std::io::Result<()> {
    let limit = request.limit.unwrap_or(args.experience_repair_limit).max(1);
    let store = ExperienceStore::load_from_disk_kv(&args.experience_path)?;
    let (repaired_store, plan) = store.repaired_legacy_metadata_store(limit);
    let mut backup_path = None;
    let mut applied = false;

    if request.apply && !plan.is_empty() {
        let target_backup_path = request
            .backup_path
            .or_else(|| args.experience_repair_backup_path.clone())
            .unwrap_or_else(|| timestamped_sidecar_path(&args.experience_path, "repair-backup"));

        ensure_parent_dir(&target_backup_path)?;
        fs::copy(&args.experience_path, &target_backup_path)?;
        repaired_store.save_to_disk_kv(&args.experience_path)?;

        backup_path = Some(target_backup_path);
        applied = true;
    }

    let body = model_service_experience_repair_response_json(ModelServiceExperienceRepairView {
        request_id,
        experience_path: &args.experience_path,
        applied,
        backup_path: backup_path.as_ref(),
        plan: &plan,
    });
    write_http_json(stream, 200, "OK", &body)
}
