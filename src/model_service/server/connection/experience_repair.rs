use std::fs;
use std::net::TcpStream;

use rust_norion::ExperienceStore;

use super::super::super::json::write_http_json;
use super::super::super::request::ModelServiceExperienceRepairRequest;
use super::super::super::response::{
    model_service_experience_repair_response_json, ModelServiceExperienceRepairView,
};
use super::write_runtime_state_block_if_dirty;
use crate::path_utils::{ensure_parent_dir, timestamped_sidecar_path};
use crate::Args;

pub(super) fn handle_experience_repair(
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceExperienceRepairRequest,
) -> std::io::Result<()> {
    if request.apply
        && write_runtime_state_block_if_dirty(args, stream, request_id, "experience-repair")?
    {
        return Ok(());
    }
    let limit = request.limit.unwrap_or(args.experience_repair_limit).max(1);
    let store = if request.apply {
        ExperienceStore::load_from_disk_kv(&args.experience_path)?
    } else {
        ExperienceStore::load_from_disk_kv_read_only(&args.experience_path)?
    };
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

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};

    use super::*;

    fn tcp_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        (client, server)
    }

    #[test]
    fn dirty_runtime_state_blocks_repair_apply() {
        let args = Args::parse(vec![
            "--memory".to_owned(),
            "legacy-memory.ndkv".to_owned(),
            "--experience".to_owned(),
            "legacy-experience.ndkv".to_owned(),
            "--adaptive".to_owned(),
            "legacy-adaptive.ndkv".to_owned(),
        ]);
        let (mut client, mut server) = tcp_pair();

        handle_experience_repair(
            &args,
            &mut server,
            42,
            ModelServiceExperienceRepairRequest {
                apply: true,
                limit: Some(1),
                backup_path: None,
            },
        )
        .unwrap();
        drop(server);

        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.contains("HTTP/1.1 409 Conflict"));
        assert!(response.contains("\"endpoint\":\"experience-repair\""));
        assert!(response.contains("\"blocked_reason\":\"runtime_state_bucket\""));
        assert!(response.contains("\"persistent_writes\":false"));
    }
}
