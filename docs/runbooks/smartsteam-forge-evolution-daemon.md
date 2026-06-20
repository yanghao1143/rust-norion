# SmartSteam Forge Evolution Daemon Status Runbook

本文只记录当前已有证据覆盖的 SmartSteam Forge 自进化 daemon/status 能力。以下命令用于本地检查和安全预检，不启动模型、不触碰远程、不发送 prompt。

## 安全边界

- `Status`、`JsonStatus`、`Candidates`、`CandidateList`、`CandidateGate`、`CandidateApplyCheck`、`Watch -Count 1` 是检查入口；selftest 断言这些路径 `starts_process=false`、`sends_prompt=false`。
- `StartCheck` 等价于启动前 dry-run：会做 candidate backlog gate 和 report gate preflight，但 `check_only=true`，不会启动 daemon，也不会发送 prompt。
- `Start` 是真实启动入口，不属于本文操作范围。

## 一次性验收

```powershell
cargo test -q --manifest-path tools\smartsteam-forge\Cargo.toml
```

可选 wrapper 自检：

```powershell
.\tools\smartsteam-forge\test-evolution-daemon.cmd
```

该自检覆盖 `StartCheck`、`StopCheck`、`JsonStatus`、candidate backlog、report gate 阻断、`Watch -Count 1`。脚本位置：

```text
tools\smartsteam-forge\scripts\test-evolution-daemon.ps1
```

## 状态检查

读取富 JSON 状态：

```powershell
.\tools\smartsteam-forge\evolution-daemon.cmd -JsonStatus -WorkDir target\remote-gemma-unattended
```

验收输出应包含：

- `"schema":"smartsteam.forge.evolution_status.v1"`
- `"read_only":true`
- `"starts_process":false`
- `"sends_prompt":false`
- `"report_gate_status":`
- `"report_gate_preflight":`
- `"candidate_backlog":`
- `"daemon_start_gate":`
- `"unattended_start_plan":`

`JsonStatus` 由 `evolution_status_render.rs` 组装，当前已暴露 `report_gate_status`、`report_gate_preflight`、`candidate_backlog`、`daemon_start_gate` 和 `unattended_start_plan`。

## Report Gate

安全预检命令：

```powershell
.\tools\smartsteam-forge\evolution-daemon.cmd -StartCheck -WorkDir target\remote-gemma-unattended -Backend 127.0.0.1:7979 -MaxTokens 64 -MaxTotalTokens 96 -MaxRuntimeSecs 0 -MaxFailures 1 -MaxNoFeedbackRounds 0 -TimeoutSecs 300
```

验收输出应包含：

- `candidate_preflight read_only=true starts_process=false sends_prompt=false`
- `report_gate_preflight read_only=true starts_process=false sends_prompt=false`
- `check_only=true`
- `starts_process=false`
- `sends_prompt=false`

`report_gate_status` 读取上一轮 report，并汇总这些字段：`report_gate_passed`、`ledger_gate_allow_next_round`、`ledger_gate_blocked`、`model_pool_alignment_ok`、route dependency failures、missing status roles、`test_gate_verdict`、`test_gate_validation_command_safety`、`can_continue_unattended` 和 `repair_hint`。

`report_gate_preflight` 的 continuation 状态：

- `no_report`：没有上一轮 report，允许首次 unattended start 的 dry-run 通过。
- `ready`：上一轮 report gate、ledger gate、model pool alignment 均满足继续条件。
- `blocked`：上一轮 report 已存在但不满足继续条件，`StartCheck` 会在启动前失败。
- `unreadable`：report 存在但不可读，`StartCheck` 会在启动前失败。

本地证据：

```text
target\remote-gemma-unattended\evolution-report.json
```

该 report 当前记录 `report_gate.passed=true`、`rounds=4`、`success=4`、`failures=0`、`test_gate.latest_verdict=pass`，并带有远程模型池和 helper stage evidence。

## Candidate Backlog Gate

查看 backlog 生命周期 gate：

```powershell
.\tools\smartsteam-forge\evolution-daemon.cmd -CandidateGate -WorkDir target\remote-gemma-unattended
```

查看待处理候选：

```powershell
.\tools\smartsteam-forge\evolution-daemon.cmd -CandidateList -WorkDir target\remote-gemma-unattended -CandidateStatus accepted -CandidatesLimit 5
```

验收输出应包含：

- `read_only=true starts_process=false sends_prompt=false`
- `candidate_lifecycle ready=...`
- `accepted_pending=...`
- `implemented_validated=...`
- `implemented_unvalidated=...`
- `implemented_failed=...`

`StartCheck` 会先跑 candidate preflight。若 backlog 中还有 accepted pending，或 implemented 但未验证/验证失败的候选，启动前会被阻断，block reason 为 `candidate_backlog_not_ready`。

## Watch

读取一次 daemon/status 快照：

```powershell
.\tools\smartsteam-forge\evolution-daemon.cmd -Watch -Count 1 -IntervalSecs 1 -WorkDir target\remote-gemma-unattended
```

验收输出应包含：

- `evolution_watch iteration=1`
- `read_only=true starts_process=false sends_prompt=false`
- `unattended_start_plan can_start=...`
- `report_gate_continuation_state=...`
- `next_step=...`

## 远程模型池已验证状态

本文不重新探测远程，只读取已落盘证据：

```powershell
Get-Content -Raw target\remote-gemma-chain\status-with-model-cache.json | ConvertFrom-Json
```

证据文件：

```text
target\remote-gemma-chain\status-with-model-cache.json
target\remote-gemma-chain\model-cache-status.json
target\remote-gemma-unattended\evolution-report.json
```

当前 `status-with-model-cache.json` 记录：

- `contract_version=smartsteam.remote-gemma-chain.status.v1`
- `read_only=true`
- `starts_process=false`
- `sends_prompt=false`
- `touches_remote=false`
- `remote_probe_skipped=true`
- `readiness.ready=true`
- `readiness.required_roles_ready=true`
- `readiness.model_cache_all_ok=true`
- `model_cache.ok_count/model_count=5/5`
- `model_pool.healthy_worker_count/worker_count=6/6`
- required roles：`summary,router,review,index,test-gate`

## 代码证据

关键实现片段在：

```text
tools\smartsteam-forge\src\app\evolution_status_render.rs
```

已读实现点：

- `render_enriched_evolution_status_json` 输出 `JsonStatus` 顶层 envelope。
- `read_report_gate_status` 读取 report path 并解析 report gate、ledger gate、model pool alignment、test gate。
- `render_report_gate_continuation_preflight` 和 `report_gate_preflight_json` 输出 report gate 预检状态。
- `unattended_start_plan_json` 和 `unattended_start_plan_lines` 汇总 candidate gate、daemon running、report gate 和 stale pid 的启动计划。
