use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::args::{Config, PoolLeaseBusyPolicy};
use crate::json::{json_string, json_string_field, json_u64_field};
use crate::pool_dispatch::PoolDispatchDecision;
use crate::pool_stage::PoolStageDispatchPlan;

#[derive(Debug)]
pub(crate) struct PoolLeaseGuard {
    path: PathBuf,
    token: String,
    summary: String,
}

#[derive(Debug)]
pub(crate) enum PoolLeaseAcquire {
    Disabled,
    Acquired(PoolLeaseGuard),
    Skipped { reason: String },
}

impl PoolLeaseGuard {
    pub(crate) fn summary(&self) -> &str {
        &self.summary
    }
}

impl Drop for PoolLeaseGuard {
    fn drop(&mut self) {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return;
        };
        if json_string_field(&text, "lease_token").as_deref() == Some(self.token.as_str()) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub(crate) fn acquire(
    config: &Config,
    decision: &PoolDispatchDecision,
    round: usize,
    case_name: &str,
    acquired_unix: u64,
) -> Result<PoolLeaseAcquire, String> {
    let Some(dir) = config.pool_lease_dir.as_deref() else {
        return Ok(PoolLeaseAcquire::Disabled);
    };
    if !config.require_pool_route {
        return Err("--pool-lease-dir requires --require-pool-route".to_owned());
    }
    fs::create_dir_all(dir)
        .map_err(|error| format!("create pool lease dir {} failed: {error}", dir.display()))?;
    let target = LeaseTarget::from_decision(decision);
    let path = lease_path(dir, &target);
    acquire_path_with_wait(&path, &target, round, case_name, acquired_unix, config)
}

pub(crate) fn acquire_stage(
    config: &Config,
    plan: &PoolStageDispatchPlan,
    round: usize,
    case_name: &str,
    acquired_unix: u64,
) -> Result<PoolLeaseAcquire, String> {
    let Some(dir) = config.pool_lease_dir.as_deref() else {
        return Ok(PoolLeaseAcquire::Disabled);
    };
    fs::create_dir_all(dir)
        .map_err(|error| format!("create pool lease dir {} failed: {error}", dir.display()))?;
    let target = LeaseTarget::from_stage_plan(plan);
    let path = lease_path(dir, &target);
    acquire_path_with_wait(&path, &target, round, case_name, acquired_unix, config)
}

fn acquire_path_with_wait(
    path: &Path,
    target: &LeaseTarget,
    round: usize,
    case_name: &str,
    acquired_unix: u64,
    config: &Config,
) -> Result<PoolLeaseAcquire, String> {
    let deadline_unix = acquired_unix.saturating_add(config.pool_lease_wait_secs);
    let mut now = acquired_unix;
    loop {
        match try_acquire_path(
            path,
            target,
            round,
            case_name,
            now,
            config.pool_lease_ttl_secs,
        ) {
            Ok(lease) => return Ok(PoolLeaseAcquire::Acquired(lease)),
            Err(LeaseAcquireFailure::Busy {
                path: _,
                expires_unix,
            }) => match busy_action(config, target, expires_unix, now, deadline_unix) {
                BusyAction::Fail => {
                    return Err(LeaseAcquireFailure::Busy {
                        path: path.to_path_buf(),
                        expires_unix,
                    }
                    .message(target));
                }
                BusyAction::Skip(reason) => return Ok(PoolLeaseAcquire::Skipped { reason }),
                BusyAction::Wait(sleep_secs) => {
                    println!(
                        "pool_lease: busy role={} endpoint={} expires_unix={} waiting {}s (deadline_unix={})",
                        target.selected_role,
                        target.selected_base_url.as_deref().unwrap_or("none"),
                        expires_unix,
                        sleep_secs,
                        deadline_unix
                    );
                    thread::sleep(Duration::from_secs(sleep_secs));
                    now = unix_seconds();
                }
            },
            Err(error) => return Err(error.message(target)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BusyAction {
    Fail,
    Wait(u64),
    Skip(String),
}

fn busy_action(
    config: &Config,
    target: &LeaseTarget,
    expires_unix: u64,
    now: u64,
    deadline_unix: u64,
) -> BusyAction {
    if config.pool_lease_busy_policy == PoolLeaseBusyPolicy::SkipLowPriority
        && target.can_accept_low_priority_task
    {
        return BusyAction::Skip(format!(
            "pool lease busy for low-priority-capable role={} endpoint={} expires_unix={}",
            target.selected_role,
            target.selected_base_url.as_deref().unwrap_or("none"),
            expires_unix
        ));
    }
    if config.pool_lease_busy_policy != PoolLeaseBusyPolicy::Fail
        && config.pool_lease_wait_secs > 0
        && now < deadline_unix
    {
        let remaining = deadline_unix.saturating_sub(now);
        return BusyAction::Wait(config.pool_lease_poll_secs.min(remaining).max(1));
    }
    BusyAction::Fail
}

fn try_acquire_path(
    path: &Path,
    target: &LeaseTarget,
    round: usize,
    case_name: &str,
    acquired_unix: u64,
    ttl_secs: u64,
) -> Result<PoolLeaseGuard, LeaseAcquireFailure> {
    try_acquire_path_with_owner_check(
        path,
        target,
        round,
        case_name,
        acquired_unix,
        ttl_secs,
        process_is_running,
    )
}

fn try_acquire_path_with_owner_check(
    path: &Path,
    target: &LeaseTarget,
    round: usize,
    case_name: &str,
    acquired_unix: u64,
    ttl_secs: u64,
    owner_is_running: impl Fn(u32) -> bool,
) -> Result<PoolLeaseGuard, LeaseAcquireFailure> {
    let token = lease_token(target, round, acquired_unix);
    let expires_unix = acquired_unix.saturating_add(ttl_secs);
    let body = lease_json(
        target,
        round,
        case_name,
        acquired_unix,
        expires_unix,
        &token,
    );
    match create_new_lease(path, &body) {
        Ok(()) => Ok(PoolLeaseGuard {
            path: path.to_path_buf(),
            token,
            summary: lease_summary(target, path, expires_unix),
        }),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let existing = fs::read_to_string(path).map_err(|read_error| {
                LeaseAcquireFailure::Fatal(format!(
                    "pool lease busy and existing lease {} could not be read: {read_error}",
                    path.display()
                ))
            })?;
            let expires = json_u64_field(&existing, "expires_unix").unwrap_or(u64::MAX);
            let cleanup_reason;
            if expires > acquired_unix {
                let owner_pid = json_u64_field(&existing, "owner_pid");
                if !lease_owner_is_stale(owner_pid, &owner_is_running) {
                    return Err(LeaseAcquireFailure::Busy {
                        path: path.to_path_buf(),
                        expires_unix: expires,
                    });
                }
                cleanup_reason = "stale-owner";
            } else {
                cleanup_reason = "expired";
            }
            fs::remove_file(path).map_err(|remove_error| {
                LeaseAcquireFailure::Fatal(format!(
                    "remove {cleanup_reason} pool lease {} failed: {remove_error}",
                    path.display(),
                ))
            })?;
            create_new_lease(path, &body).map_err(|retry_error| {
                LeaseAcquireFailure::Fatal(format!(
                    "acquire pool lease {} after {cleanup_reason} cleanup failed: {retry_error}",
                    path.display(),
                ))
            })?;
            Ok(PoolLeaseGuard {
                path: path.to_path_buf(),
                token,
                summary: lease_summary(target, path, expires_unix),
            })
        }
        Err(error) => Err(LeaseAcquireFailure::Fatal(format!(
            "acquire pool lease {} failed: {error}",
            path.display()
        ))),
    }
}

fn lease_owner_is_stale(owner_pid: Option<u64>, owner_is_running: impl Fn(u32) -> bool) -> bool {
    let Some(owner_pid) = owner_pid else {
        return false;
    };
    if owner_pid == u64::from(process::id()) || owner_pid > u64::from(u32::MAX) {
        return false;
    }
    !owner_is_running(owner_pid as u32)
}

fn process_is_running(pid: u32) -> bool {
    if pid == process::id() {
        return true;
    }

    #[cfg(windows)]
    {
        let output = process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .output();
        let Ok(output) = output else {
            return true;
        };
        let text = String::from_utf8_lossy(&output.stdout);
        let expected_pid = format!("\"{pid}\"");
        return text.lines().any(|line| {
            line.starts_with('"') && line.split(',').nth(1) == Some(expected_pid.as_str())
        });
    }

    #[cfg(unix)]
    {
        return process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|status| status.success())
            .unwrap_or(true);
    }

    #[cfg(not(any(windows, unix)))]
    {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LeaseAcquireFailure {
    Busy { path: PathBuf, expires_unix: u64 },
    Fatal(String),
}

impl LeaseAcquireFailure {
    fn message(self, target: &LeaseTarget) -> String {
        match self {
            Self::Busy { path, expires_unix } => format!(
                "pool lease busy: {} selected_role={} endpoint={} expires_unix={}",
                path.display(),
                target.selected_role,
                target.selected_base_url.as_deref().unwrap_or("none"),
                expires_unix
            ),
            Self::Fatal(message) => message,
        }
    }
}

fn create_new_lease(path: &Path, body: &str) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(body.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LeaseTarget {
    selected_role: String,
    selected_port: Option<u64>,
    selected_base_url: Option<String>,
    context_window: Option<u64>,
    default_max_tokens: Option<u64>,
    can_accept_low_priority_task: bool,
}

impl LeaseTarget {
    fn from_decision(decision: &PoolDispatchDecision) -> Self {
        Self {
            selected_role: decision.selected_role.clone(),
            selected_port: decision.selected_port,
            selected_base_url: decision.selected_base_url.clone(),
            context_window: decision.context_window,
            default_max_tokens: decision.default_max_tokens,
            can_accept_low_priority_task: decision.can_accept_low_priority_task,
        }
    }

    fn from_stage_plan(plan: &PoolStageDispatchPlan) -> Self {
        Self {
            selected_role: plan.selected_role.clone(),
            selected_port: plan.selected_port,
            selected_base_url: plan.selected_base_url.clone(),
            context_window: plan.context_window,
            default_max_tokens: plan.default_max_tokens,
            can_accept_low_priority_task: plan.can_accept_low_priority_task,
        }
    }
}

fn lease_path(dir: &Path, target: &LeaseTarget) -> PathBuf {
    let mut name = sanitize_path_part(&target.selected_role);
    if let Some(port) = target.selected_port {
        name.push('-');
        name.push_str(&port.to_string());
    }
    name.push_str(".lease.json");
    dir.join(name)
}

fn lease_json(
    target: &LeaseTarget,
    round: usize,
    case_name: &str,
    acquired_unix: u64,
    expires_unix: u64,
    token: &str,
) -> String {
    format!(
        "{{\"lease_token\":{},\"owner_pid\":{},\"round\":{},\"case\":{},\"selected_role\":{},\"selected_port\":{},\"selected_base_url\":{},\"context_window\":{},\"default_max_tokens\":{},\"acquired_unix\":{},\"expires_unix\":{}}}",
        json_string(token),
        process::id(),
        round,
        json_string(case_name),
        json_string(&target.selected_role),
        option_u64_json(target.selected_port),
        option_str_json(target.selected_base_url.as_deref()),
        option_u64_json(target.context_window),
        option_u64_json(target.default_max_tokens),
        acquired_unix,
        expires_unix
    )
}

fn lease_summary(target: &LeaseTarget, path: &Path, expires_unix: u64) -> String {
    format!(
        "role={} port={} endpoint={} expires_unix={} path={}",
        target.selected_role,
        target
            .selected_port
            .map(|port| port.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        target.selected_base_url.as_deref().unwrap_or("none"),
        expires_unix,
        path.display()
    )
}

fn lease_token(target: &LeaseTarget, round: usize, acquired_unix: u64) -> String {
    format!(
        "{}-{}-{}-{}",
        process::id(),
        acquired_unix,
        round,
        sanitize_path_part(&target.selected_role)
    )
}

fn sanitize_path_part(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        sanitized.push_str("worker");
    }
    sanitized
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

fn option_str_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("smartsteam-pool-lease-{name}-{}", process::id()))
    }

    fn decision() -> PoolDispatchDecision {
        PoolDispatchDecision {
            selected_role: "summary".to_owned(),
            selected_port: Some(8687),
            selected_base_url: Some("http://127.0.0.1:8687".to_owned()),
            context_window: Some(8192),
            default_max_tokens: Some(768),
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
            can_accept_low_priority_task: true,
            evidence: "route_allowed:true".to_owned(),
        }
    }

    fn stage_plan() -> PoolStageDispatchPlan {
        PoolStageDispatchPlan {
            task_kind: "summary".to_owned(),
            selected_role: "summary".to_owned(),
            selected_port: Some(8687),
            selected_base_url: Some("http://127.0.0.1:8687".to_owned()),
            context_window: Some(8192),
            default_max_tokens: Some(768),
            runtime_backend: None,
            runtime_device: None,
            runtime_accelerator: None,
            gpu_layers: None,
            configured_max_tokens: 4096,
            effective_max_tokens: 768,
            max_tokens_clamped: true,
            can_accept_low_priority_task: true,
        }
    }

    fn unwrap_acquired(value: PoolLeaseAcquire) -> PoolLeaseGuard {
        match value {
            PoolLeaseAcquire::Acquired(lease) => lease,
            other => panic!("expected acquired lease, got {other:?}"),
        }
    }

    #[test]
    fn no_lease_dir_disables_lease() {
        let lease = acquire(&Config::default(), &decision(), 1, "case", 10).unwrap();

        assert!(matches!(lease, PoolLeaseAcquire::Disabled));
    }

    #[test]
    fn lease_dir_requires_pool_route_gate() {
        let dir = unique_temp_dir("requires-route");
        let _ = fs::remove_dir_all(&dir);
        let error = acquire(
            &Config {
                pool_lease_dir: Some(dir.clone()),
                ..Config::default()
            },
            &decision(),
            1,
            "case",
            10,
        )
        .unwrap_err();

        assert!(error.contains("--pool-lease-dir requires --require-pool-route"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn lease_is_created_and_released_by_guard() {
        let dir = unique_temp_dir("create-release");
        let _ = fs::remove_dir_all(&dir);
        let config = Config {
            require_pool_route: true,
            pool_lease_dir: Some(dir.clone()),
            pool_lease_ttl_secs: 30,
            ..Config::default()
        };
        let lease = unwrap_acquired(acquire(&config, &decision(), 7, "case-a", 100).unwrap());
        let path = lease.path.clone();

        assert!(path.exists());
        assert!(lease.summary().contains("role=summary"));
        drop(lease);
        assert!(!path.exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn active_lease_blocks_second_owner() {
        let dir = unique_temp_dir("busy");
        let _ = fs::remove_dir_all(&dir);
        let config = Config {
            require_pool_route: true,
            pool_lease_dir: Some(dir.clone()),
            pool_lease_ttl_secs: 30,
            ..Config::default()
        };
        let lease = unwrap_acquired(acquire(&config, &decision(), 7, "case-a", 100).unwrap());

        let error = acquire(&config, &decision(), 8, "case-b", 101).unwrap_err();

        assert!(error.contains("pool lease busy"));
        assert!(error.contains("expires_unix=130"));
        drop(lease);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stale_live_lease_from_dead_owner_is_replaced_before_ttl() {
        let dir = unique_temp_dir("stale-owner");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let target = LeaseTarget::from_decision(&decision());
        let path = lease_path(&dir, &target);
        fs::write(
            &path,
            "{\"lease_token\":\"old\",\"owner_pid\":99999999,\"selected_role\":\"summary\",\"expires_unix\":130}\n",
        )
        .unwrap();

        let lease =
            try_acquire_path_with_owner_check(&path, &target, 8, "case-b", 100, 30, |_owner_pid| {
                false
            })
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();

        assert!(text.contains("\"round\":8"));
        assert!(text.contains(&format!("\"owner_pid\":{}", process::id())));
        drop(lease);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn live_owner_lease_still_blocks_before_ttl() {
        let dir = unique_temp_dir("live-owner");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let target = LeaseTarget::from_decision(&decision());
        let path = lease_path(&dir, &target);
        fs::write(
            &path,
            "{\"lease_token\":\"old\",\"owner_pid\":4242,\"selected_role\":\"summary\",\"expires_unix\":130}\n",
        )
        .unwrap();

        let error =
            try_acquire_path_with_owner_check(&path, &target, 8, "case-b", 100, 30, |_owner_pid| {
                true
            })
            .unwrap_err();

        assert_eq!(
            error,
            LeaseAcquireFailure::Busy {
                path: path.clone(),
                expires_unix: 130,
            }
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn lease_without_owner_pid_still_blocks_until_expiry() {
        let dir = unique_temp_dir("missing-owner");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let target = LeaseTarget::from_decision(&decision());
        let path = lease_path(&dir, &target);
        fs::write(
            &path,
            "{\"lease_token\":\"old\",\"selected_role\":\"summary\",\"expires_unix\":130}\n",
        )
        .unwrap();

        let error =
            try_acquire_path_with_owner_check(&path, &target, 8, "case-b", 100, 30, |_owner_pid| {
                false
            })
            .unwrap_err();

        assert_eq!(
            error,
            LeaseAcquireFailure::Busy {
                path: path.clone(),
                expires_unix: 130,
            }
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn expired_lease_is_replaced() {
        let dir = unique_temp_dir("expired");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let target = LeaseTarget::from_decision(&decision());
        let path = lease_path(&dir, &target);
        fs::write(
            &path,
            "{\"lease_token\":\"old\",\"selected_role\":\"summary\",\"expires_unix\":99}\n",
        )
        .unwrap();
        let config = Config {
            require_pool_route: true,
            pool_lease_dir: Some(dir.clone()),
            pool_lease_ttl_secs: 30,
            ..Config::default()
        };

        let lease = unwrap_acquired(acquire(&config, &decision(), 8, "case-b", 100).unwrap());
        let text = fs::read_to_string(&path).unwrap();

        assert!(text.contains("\"round\":8"));
        drop(lease);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn guard_does_not_remove_replaced_foreign_lease() {
        let dir = unique_temp_dir("foreign");
        let _ = fs::remove_dir_all(&dir);
        let config = Config {
            require_pool_route: true,
            pool_lease_dir: Some(dir.clone()),
            pool_lease_ttl_secs: 30,
            ..Config::default()
        };
        let lease = unwrap_acquired(acquire(&config, &decision(), 7, "case-a", 100).unwrap());
        let path = lease.path.clone();
        fs::write(
            &path,
            "{\"lease_token\":\"foreign\",\"selected_role\":\"summary\",\"expires_unix\":999}\n",
        )
        .unwrap();

        drop(lease);

        assert!(path.exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn skip_low_priority_policy_skips_busy_low_priority_worker() {
        let dir = unique_temp_dir("skip-low-priority");
        let _ = fs::remove_dir_all(&dir);
        let config = Config {
            require_pool_route: true,
            pool_lease_dir: Some(dir.clone()),
            pool_lease_ttl_secs: 30,
            pool_lease_busy_policy: PoolLeaseBusyPolicy::SkipLowPriority,
            ..Config::default()
        };
        let lease = unwrap_acquired(acquire(&config, &decision(), 7, "case-a", 100).unwrap());

        let skipped = acquire(&config, &decision(), 8, "case-b", 101).unwrap();

        let PoolLeaseAcquire::Skipped { reason } = skipped else {
            panic!("expected skipped busy lease");
        };
        assert!(reason.contains("low-priority-capable"));
        drop(lease);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stage_plan_uses_same_worker_lease_path() {
        let dir = unique_temp_dir("stage-shared");
        let _ = fs::remove_dir_all(&dir);
        let config = Config {
            require_pool_route: true,
            pool_lease_dir: Some(dir.clone()),
            pool_lease_ttl_secs: 30,
            ..Config::default()
        };
        let main_lease = unwrap_acquired(acquire(&config, &decision(), 7, "case-a", 100).unwrap());

        let error = acquire_stage(&config, &stage_plan(), 7, "case-a", 101).unwrap_err();

        assert!(error.contains("pool lease busy"));
        assert!(error.contains("summary-8687.lease.json"));
        drop(main_lease);
        let stage_lease =
            unwrap_acquired(acquire_stage(&config, &stage_plan(), 8, "case-b", 102).unwrap());
        assert!(stage_lease.summary().contains("role=summary"));
        drop(stage_lease);
        let _ = fs::remove_dir_all(dir);
    }
}
