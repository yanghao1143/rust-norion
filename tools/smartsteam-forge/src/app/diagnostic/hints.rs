pub(super) fn diagnostic_hints(
    health: &Result<String, String>,
    readiness: &Result<String, String>,
    safe_device: &Result<String, String>,
) -> Vec<&'static str> {
    if health.is_ok() && readiness.is_ok() {
        return ready_backend_hints(health, safe_device);
    }

    let mut hints = Vec::new();
    let failures = failure_text(health, readiness, safe_device);

    push_connection_hints(&failures, &mut hints);
    push_resolution_hints(&failures, &mut hints);
    push_gemma_runtime_hints(&failures, &mut hints);
    push_experience_hygiene_hints(&failures, &mut hints);
    push_busy_hints(&failures, &mut hints);
    push_safe_device_hints(&failures, &mut hints);

    if hints.is_empty() {
        hints.push("先看上面的 health/readiness 失败原因；发送真实 prompt 前，重新运行 /doctor 或 --doctor。");
    }

    hints
}

fn ready_backend_hints(
    health: &Result<String, String>,
    safe_device: &Result<String, String>,
) -> Vec<&'static str> {
    if safe_device.is_err() {
        return vec![
            "后端可达，但 safe-device 未通过：Gemma 12B 当前不是 GPU-first，长 prompt 前先确认 GPU-backed --gemma-runtime-server。",
            "只读诊断：curl.exe -s http://127.0.0.1:7878/health；nvidia-smi；cargo run -- --backend 127.0.0.1:7878 --doctor。",
            "只有 tiny CPU fallback 测试才临时关闭 safe-device guard。",
        ];
    }
    let summary = health
        .as_ref()
        .map(|value| value.to_lowercase())
        .unwrap_or_default();
    if summary.contains("gemma_12b_device") || summary.contains("cpu/disk-first") {
        return vec![
            "后端可达，但 /health 显示 Gemma 12B 是 CPU/disk-first；这通常说明 GPU 没被当前 runtime 使用。",
            "只读诊断：curl.exe -s http://127.0.0.1:7878/health；nvidia-smi；cargo run -- --backend 127.0.0.1:7878 --preflight --require-safe-device。",
        ];
    }
    vec!["后端已 ready。先发短 prompt，再用 /sessions 和 /summary 复盘。"]
}

fn push_connection_hints(failures: &str, hints: &mut Vec<&'static str>) {
    if contains_any(
        failures,
        &[
            "connect backend failed",
            "connection refused",
            "connection timed out",
            "actively refused",
            "no connection could be made",
        ],
    ) {
        hints.push("后端连不上：先用只读命令确认 7878 是否有 /health，而不是直接发送 prompt。");
        hints.push("只读诊断：curl.exe -s http://127.0.0.1:7878/health；.\\tools\\smartsteam-forge\\status-forge.cmd；cargo run -- --backend 127.0.0.1:7878 --connect-timeout-ms 500 --read-timeout-ms 500 --doctor。自定义端口时，把 127.0.0.1:7878 换成 target/backend 里的 backend。说明：--read-timeout-ms 是单次 read 轮询/heartbeat 间隔；真实 Gemma 流式总等待窗口用 --timeout-secs。");
        hints.push("如果确实需要启动一个轻量后端，可另开窗口运行：cargo run -- --serve --serve-bind 127.0.0.1:7878。");
    }
}

fn push_resolution_hints(failures: &str, hints: &mut Vec<&'static str>) {
    if failures.contains("resolve backend failed") || failures.contains("did not resolve") {
        hints.push("检查 --backend host:port；http:// 前缀可以写，也可以省略。");
    }
}

fn push_gemma_runtime_hints(failures: &str, hints: &mut Vec<&'static str>) {
    if failures.contains("gemma runtime is not reachable") {
        hints.push("Gemma HTTP runtime 不可达：检查后端 /health 的 gemma_runtime_server 和 gemma_runtime_reachable。");
        hints.push("如果你要连真实 Gemma 12B，启动 rust-norion 时确认 --gemma-runtime-server 指向正在监听的 Gemma HTTP runtime。");
        hints.push("只读诊断：curl.exe -s http://127.0.0.1:7878/health；cargo run -- --backend 127.0.0.1:7878 --preflight --require-safe-device。");
    }
}

fn push_experience_hygiene_hints(failures: &str, hints: &mut Vec<&'static str>) {
    if failures.contains("experience_hygiene") || failures.contains("experience hygiene") {
        hints.push(
            "聊天前先检查经验库卫生：curl.exe -s http://127.0.0.1:7878/v1/experience-hygiene",
        );
        if failures.contains("quarantine_candidates") {
            hints.push("先 dry-run quarantine，不写 .ndkv：curl.exe -s -X POST http://127.0.0.1:7878/v1/experience-hygiene/quarantine -H \"Content-Type: application/json\" -d \"{\\\"limit\\\":20}\"");
        }
        if failures.contains("experience_index")
            || failures.contains("retrieval_ready=false")
            || failures.contains("risk_level=blocked")
        {
            hints.push("如果阻塞来自 experience_index/retrieval_ready=false，先看 cleanup audit：cargo run -- --experience-cleanup-audit --experience-cleanup-audit-limit 20");
        }
    }
    if failures.contains("experience_repair")
        || failures.contains("repairable_legacy_metadata_lessons")
        || failures.contains("repairable_index_records")
    {
        hints.push("先 dry-run experience repair，不写 .ndkv：cargo run -- --experience-repair --experience-repair-limit 20");
        if failures.contains("repairable_index_records") {
            hints.push("如果阻塞来自 repairable_index_records，也先看 cleanup audit：cargo run -- --experience-cleanup-audit --experience-cleanup-audit-limit 20");
        }
    }
}

fn push_busy_hints(failures: &str, hints: &mut Vec<&'static str>) {
    if failures.contains("engine is busy") || failures.contains("busy=true") {
        hints.push("后端正在处理已有推理；/health 的 active_requests 会显示 request_id、endpoint、elapsed_ms 和 prompt_preview。");
        hints.push("只读诊断：curl.exe -s http://127.0.0.1:7878/health；cargo run -- --backend 127.0.0.1:7878 --preflight；TUI 内用 /status 或 /ready。");
    }
}

fn push_safe_device_hints(failures: &str, hints: &mut Vec<&'static str>) {
    if failures.contains("safe-device failed")
        || failures.contains("safe-device 未通过")
        || failures.contains("backend safe-device failed")
        || failures.contains("gemma_12b_device")
        || failures.contains("cpu/disk-first")
        || failures.contains("not gpu-first")
    {
        hints.push("GPU 没用或不是 GPU-first：看 /health 的 device_primary_lane、device_memory_mode、device_accelerators，再用 nvidia-smi 核对显存和利用率。");
        hints.push("真实 12B 长 prompt 前保持 --require-safe-device；只有 tiny CPU fallback 测试才关闭它。");
    }
}

fn failure_text(
    health: &Result<String, String>,
    readiness: &Result<String, String>,
    safe_device: &Result<String, String>,
) -> String {
    [
        health.as_ref().err(),
        readiness.as_ref().err(),
        safe_device.as_ref().err(),
    ]
    .into_iter()
    .flatten()
    .map(|error| error.to_lowercase())
    .collect::<Vec<_>>()
    .join("\n")
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}
