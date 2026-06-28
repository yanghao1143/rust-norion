use std::fs;
use std::net::TcpStream;

use rust_norion::ExperienceStore;

use super::super::super::json::write_http_json;
use super::super::super::request::ModelServiceExperienceHygieneQuarantineRequest;
use super::super::super::response::{
    model_service_experience_hygiene_quarantine_response_json,
    model_service_experience_hygiene_response_json, ModelServiceExperienceHygieneQuarantineView,
    ModelServiceExperienceHygieneView,
};
use super::write_runtime_state_block_if_dirty;
use crate::path_utils::{ensure_parent_dir, timestamped_sidecar_path};
use crate::Args;

pub(super) fn handle_experience_hygiene(
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
) -> std::io::Result<()> {
    if !args.experience_path.exists() {
        let body =
            model_service_experience_hygiene_response_json(ModelServiceExperienceHygieneView {
                request_id,
                experience_path: &args.experience_path,
                report: None,
                index_report: None,
                quarantine_plan: None,
                error: Some("experience_file_missing"),
            });
        return write_http_json(stream, 200, "OK", &body);
    }

    let store = ExperienceStore::load_from_disk_kv_read_only(&args.experience_path)?;
    let report = store.hygiene_report(args.experience_hygiene_limit);
    let index_report = store.index_report(args.experience_hygiene_limit);
    let quarantine_plan = store.hygiene_quarantine_plan(args.experience_hygiene_limit);
    let body = model_service_experience_hygiene_response_json(ModelServiceExperienceHygieneView {
        request_id,
        experience_path: &args.experience_path,
        report: Some(&report),
        index_report: Some(&index_report),
        quarantine_plan: Some(&quarantine_plan),
        error: None,
    });
    write_http_json(stream, 200, "OK", &body)
}

pub(super) fn handle_experience_hygiene_quarantine(
    args: &Args,
    stream: &mut TcpStream,
    request_id: usize,
    request: ModelServiceExperienceHygieneQuarantineRequest,
) -> std::io::Result<()> {
    if request.apply
        && write_runtime_state_block_if_dirty(
            args,
            stream,
            request_id,
            "experience-hygiene-quarantine",
        )?
    {
        return Ok(());
    }
    let limit = request
        .limit
        .unwrap_or(args.experience_hygiene_limit)
        .max(1);
    let store = if request.apply {
        ExperienceStore::load_from_disk_kv(&args.experience_path)?
    } else {
        ExperienceStore::load_from_disk_kv_read_only(&args.experience_path)?
    };
    let (retained_store, quarantined_store, plan) = store.split_hygiene_quarantine(limit);
    let mut backup_path = None;
    let mut quarantine_path = None;
    let mut applied = false;

    if request.apply && !plan.is_empty() {
        let target_backup_path = request
            .backup_path
            .or_else(|| args.experience_hygiene_backup_path.clone())
            .unwrap_or_else(|| timestamped_sidecar_path(&args.experience_path, "backup"));
        let target_quarantine_path = request
            .quarantine_path
            .or_else(|| args.experience_hygiene_quarantine_path.clone())
            .unwrap_or_else(|| timestamped_sidecar_path(&args.experience_path, "quarantine"));

        ensure_parent_dir(&target_backup_path)?;
        ensure_parent_dir(&target_quarantine_path)?;
        fs::copy(&args.experience_path, &target_backup_path)?;
        quarantined_store.save_to_disk_kv(&target_quarantine_path)?;
        retained_store.save_to_disk_kv(&args.experience_path)?;

        backup_path = Some(target_backup_path);
        quarantine_path = Some(target_quarantine_path);
        applied = true;
    }

    let body = model_service_experience_hygiene_quarantine_response_json(
        ModelServiceExperienceHygieneQuarantineView {
            request_id,
            experience_path: &args.experience_path,
            applied,
            backup_path: backup_path.as_ref(),
            quarantine_path: quarantine_path.as_ref(),
            plan: &plan,
        },
    );
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
    fn dirty_runtime_state_blocks_quarantine_apply() {
        let args = Args::parse(vec![
            "--memory".to_owned(),
            "legacy-memory.ndkv".to_owned(),
            "--experience".to_owned(),
            "legacy-experience.ndkv".to_owned(),
            "--adaptive".to_owned(),
            "legacy-adaptive.ndkv".to_owned(),
        ]);
        let (mut client, mut server) = tcp_pair();

        handle_experience_hygiene_quarantine(
            &args,
            &mut server,
            41,
            ModelServiceExperienceHygieneQuarantineRequest {
                apply: true,
                limit: Some(1),
                backup_path: None,
                quarantine_path: None,
            },
        )
        .unwrap();
        drop(server);

        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.contains("HTTP/1.1 409 Conflict"));
        assert!(response.contains("\"endpoint\":\"experience-hygiene-quarantine\""));
        assert!(response.contains("\"blocked_reason\":\"runtime_state_bucket\""));
        assert!(response.contains("\"persistent_writes\":false"));
    }
}
