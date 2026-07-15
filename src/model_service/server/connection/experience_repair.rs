use std::fs;
use std::net::TcpStream;

use rust_norion::NoironEngine;

use super::super::super::json::write_http_json;
use super::super::super::request::ModelServiceExperienceRepairRequest;
use super::super::super::response::{
    ModelServiceExperienceRepairView, model_service_experience_repair_response_json,
};
use super::persist_engine_or_restore;
use crate::Args;
use crate::path_utils::{ensure_parent_dir, timestamped_sidecar_path};

pub(super) fn handle_experience_repair(
    engine: &mut NoironEngine,
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceExperienceRepairRequest,
) -> std::io::Result<()> {
    let limit = request.limit.unwrap_or(args.experience_repair_limit).max(1);
    let (repaired_store, plan) = engine.experience.repaired_legacy_metadata_store(limit);
    let mut backup_path = None;
    let mut applied = false;

    if request.apply && !plan.is_empty() {
        let target_backup_path = request
            .backup_path
            .or_else(|| args.experience_repair_backup_path.clone())
            .unwrap_or_else(|| timestamped_sidecar_path(&args.experience_path, "repair-backup"));

        ensure_parent_dir(&target_backup_path)?;
        let (_, experience_read_path, _) = NoironEngine::full_state_read_paths(
            &args.memory_path,
            &args.experience_path,
            &args.adaptive_path,
        )?;
        fs::copy(experience_read_path, &target_backup_path)?;
        let engine_before = engine.clone();
        engine.experience = repaired_store;
        persist_engine_or_restore(engine, args, engine_before)?;

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
