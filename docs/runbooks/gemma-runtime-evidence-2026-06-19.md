# Gemma Runtime Evidence 2026-06-19

本文记录 2026-06-19 运行态证据的只读读法。它不是启动手册；本窗口不 SSH、不启动或停止模型、不发送 prompt、不写模型权重，只整理已经落盘的证据和总窗口同步的实时状态。

## 当前结论

总窗口已完成的实时验证：

- 远程 Mac 模型池 `6/6` healthy，全部 Metal。
- quality worker 使用 `Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf`。
- model cache `5/5` ok。
- daemon 已重启为 PID `235920`。
- round `275` 已完成，stdout 有 `report_refresh:start` 和 `report_refresh:done`。
- daemon status 显示 report rounds `275`、`ledger_lag=0`、`stale=false`、`gate_failures=0`、`remote_runtime_acceleration_ok=true`。

本窗口只读复核的本地 evidence：

- `target\remote-gemma-chain\status-with-model-cache.json`：2026-06-19 21:39:04 +08:00，链路 ready，model API/backend/Web Lab ready，worker `6/6` healthy，remote runtime probed，`acceleration_ok=true`。
- `target\remote-gemma-chain\model-cache-status.json`：2026-06-18 18:41:27 +08:00，`all_ok=true`，5/5 model ok，0 copy needed，0 remote error。
- `target\evolution\daemon\report.json`：2026-06-19 21:44:43 +08:00，rounds `275`，success `265`，failures `10`，success rate `96.364%`，latest round `275` success，validation `134/134`，self-improve `274/274`，test-gate latest verdict `pass`。
- `target\evolution\daemon\evolution-loop.pid`：2026-06-19 21:39:02 +08:00，PID `235920`。
- `target\evolution\daemon\evolution-loop.out.log`：2026-06-19 21:45:17 +08:00，contains round `275` `report_refresh:done ... rounds=275 ... failures=0` and later round `276` activity.

因此：远程链路处于可用运行态，daemon 有新 PID 和新 report refresh，report/ledger 不再是 6 月 16 日的 stale mismatch 状态。本文仍不授权本窗口执行 launch、SSH 或 prompt。

## 只读命令边界

本窗口可使用的本地文件读取命令：

```powershell
Get-Content -Raw target\remote-gemma-chain\status-with-model-cache.json | ConvertFrom-Json
Get-Content -Raw target\remote-gemma-chain\model-cache-status.json | ConvertFrom-Json
Get-Content -Raw target\evolution\daemon\report.json | ConvertFrom-Json
Get-Content -Raw target\evolution\daemon\evolution-loop.pid
Get-Content target\evolution\daemon\evolution-loop.out.log -Tail 80
```

这些命令只读本地文件：

- `read_only=true`
- `starts_process=false`
- `sends_prompt=false`
- `starts_or_stops_model=false`
- `ssh=false`
- `writes_model_weights=false`

下列命令或状态入口也可作为只读诊断入口展示，但本窗口默认不运行会触碰远程或 daemon 的入口，除非用户明确授权：

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd pool-status -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd status-bundle -JsonStatus
.\tools\smartsteam-forge\evolution-daemon.cmd -JsonStatus -WorkDir target\evolution\daemon
.\tools\smartsteam-forge\evolution-daemon.cmd -Watch -Count 1 -IntervalSecs 1 -WorkDir target\evolution\daemon
```

这些入口必须只用于观察，并且必须满足：

- `read_only=true`
- `starts_process=false`
- `sends_prompt=false`
- 不执行 `Start`
- 不执行 SSH
- 不写模型权重

`StartCheck` 只能作为 dry-run preflight 记录；`Start` 是启动入口，本窗口默认不要运行。

## 证据来源

## Artifact 清单与解析边界

下表是常驻诊断的 artifact 边界。每个 artifact 只能证明自己的字段，不得互相代替。

| Artifact | 证明什么 | 不能替代什么 | stale/Metal/Mac sleep 读法 |
| --- | --- | --- | --- |
| `target\evolution\daemon\evolution-loop.pid` | daemon 启动时写入的本地 PID，例如 `235920` | 不能单独证明进程仍在运行，不能证明 Mac 在线，不能证明 report fresh | 只作为 daemon identity 线索；必须结合 daemon status、stdout/report mtime 或进程检查 |
| `target\evolution\daemon\report.json` | 最近 report refresh 汇总：rounds、latest success、validation、self-improve、report gate、continuation gate | 不能证明当前进程仍活着，不能证明远程端口此刻仍连通，不能替代 stdout 的 refresh 事件 | `rounds`/`last.round` 要与 daemon status/ledger 对齐；`report_gate.passed` 和 `continuation_gate_report_v1.allow_unattended_continuation` 说明 report gate，可与 `strict_report_gate` 分开展示 |
| `target\evolution\daemon\evolution-loop.out.log` | daemon 实际运行事件流：round start/done、`report_refresh:start/done`、remote chain gate、helper stage dispatch | 不能替代结构化 report，也不能单独证明当前端口仍 healthy | 看到新 round 或 `report_refresh:done` 可说明近期活性；若日志长时间停滞，应以 daemon status 复核 |
| daemon status | 当前性最强的 daemon 观察：PID、running、rounds、ledger_lag、stale、gate_failures | 本窗口未运行该命令时不能把本地文件 mtime 当作 status；daemon status 也不能替代 model cache hash 证明 | `stale=false`、`ledger_lag=0`、`gate_failures=0` 是 report 不 stale 的核心外部证据 |
| `target\remote-gemma-chain\status-with-model-cache.json` | 链路快照：backend/Web Lab/model API、worker health、remote runtime、Metal acceleration | 不能替代 daemon report freshness，不能证明 daemon 正在运行，不能授权 prompt 或 launch | `remote_runtime.acceleration_ok=true`、`cpu_or_no_gpu_count=0`、worker `runtime_accelerator=metal` 是 Metal 证据；`touches_remote=true` 表示写入快照的命令触碰过远程，本窗口只是读取 |
| `target\remote-gemma-chain\model-cache-status.json` | 模型缓存和远程 SHA/ok 状态：5/5 ok、0 remote error、0 copy needed | 不能证明 worker 进程仍健康，不能证明 Metal 在用，不能证明 daemon/report fresh | cache ok 是必要但不充分条件；模型文件一致不等于端口在线 |
| `target\evolution\daemon\evolution-ledger.jsonl` | 逐轮历史事实：round、success、runtime model/tokens、validation、错误 | 不能替代 report gate 汇总，也不能单独证明 report fresh | daemon status 的 `ledger_lag=0` 或 report/ledger round 对齐才说明 report 没落后 |

解析规则：

1. `pid` 只说明“曾写入的 daemon identity”；常驻判断必须用 daemon status 或新 stdout/report 证据确认活性。
2. `report.json` 只说明最后一次 report refresh 的结构化结论；如果 status 显示 `stale=true` 或 `ledger_lag>0`，report 必须视为过期。
3. `stdout` 可以证明 `report_refresh:start/done` 真实发生，但长期运行状态仍要看后续 status 或新日志增长。
4. `status-with-model-cache.json` 可以证明 remote runtime/worker 快照，不能证明 daemon report 不 stale。
5. `model-cache-status.json` 可以证明模型 cache ok，不能证明端口、Metal、daemon 或 prompt gate。
6. `daemon status` 的 stale/ledger fields 不能替代资源余量窗口；Mac sleep prevention 仍需要独立证据。

### Chain Snapshot

`target\remote-gemma-chain\status-with-model-cache.json` 的关键字段：

- `read_only=true`
- `starts_process=false`
- `sends_prompt=false`
- `touches_remote=true`
- `remote_probe_skipped=false` 或远程 runtime 已被 probe 的状态由写入该文件的主窗口承担；本窗口只读该文件。
- `readiness.ready=true`
- `readiness.model_api=true`
- `readiness.backend=true`
- `readiness.web_lab=true`
- `remote.host=192.168.10.11`
- `remote.user=xinghuan`
- `remote.root=/Users/xinghuan/smartsteam-model-box`
- `remote.model_port=8686`

Worker 状态：

| role | port | status | context | max tokens | model | accelerator |
| --- | --- | --- | --- | --- | --- | --- |
| quality | 8686 | healthy | 65536 | 4096 | `Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf` | metal |
| summary | 8687 | healthy | 8192 | 768 | `gemma-3-270m-it-qat-Q4_0.gguf` | metal |
| review | 8688 | healthy | 4096 | 1536 | `gemma-4-E4B-it-Q4_K_M.gguf` | metal |
| router | 8689 | healthy | 4096 | 512 | `functiongemma-270m-it-Q4_K_M.gguf` | metal |
| test-gate | 8688 | healthy | 4096 | 1536 | `gemma-4-E4B-it-Q4_K_M.gguf` | metal |
| index | 8690 | healthy | 8192 | 512 | `gemma-4-E2B-it-Q4_K_M.gguf` | metal |

### Model Cache

`target\remote-gemma-chain\model-cache-status.json` 的关键字段：

- `read_only=true`
- `starts_process=false`
- `sends_prompt=false`
- `all_ok=true`
- model count `5`
- ok count `5`
- copy needed `0`
- remote errors `0`

这证明 cache 文件在该快照写入时全部匹配；它不授权新增模型拷贝或 worker 启动。

### Daemon Report

`target\evolution\daemon\report.json` 的关键字段：

- `rounds=275`
- `success=265`
- `failures=10`
- `success_rate=96.364`
- latest round `275`
- latest success `true`
- latest runtime model `Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf`
- latest runtime tokens `60`
- validation `134/134`
- self-improve `274/274`
- `report_gate.passed=true`
- `continuation_gate_report_v1.allow_unattended_continuation=true`
- `continuation_gate_report_v1.latest_round=275`
- `continuation_gate_report_v1.latest_success=true`

注意：`strict_report_gate.passed=false`，原因是历史 runtime response failures `1` above maximum `0`。这不等于 round 275 失败；它表示严格历史门仍有历史失败债务，常驻判断必须区分 `report_gate/continuation_gate` 与 `strict_report_gate`。

### Daemon PID And Stdout

`target\evolution\daemon\evolution-loop.pid` 记录 PID `235920`。`target\evolution\daemon\evolution-loop.out.log` 中的关键行：

```text
[round 275] stage report_refresh:start
[round 275] stage report_refresh:done path=D:\rust-norion\target\evolution\daemon\report.json rounds=275 gate=report_continuation_gate failures=0
[round 275] ok runtime_tokens=60 elapsed_ms=234520
remote_chain_gate: passed ready:true ... workers:6/6 ... model_cache_ok:5/5 ... remote_runtime_acceleration_ok:true
```

日志尾部还出现 round `276` 的 `pool_lease`、`status rust-norion business cycle stream connected` 和多个 helper role stage dispatch，这说明 daemon 在 report refresh 后仍有后续活动。是否仍在运行应以 daemon status 或进程检查为准；本窗口未启动或停止该进程。

## 如何判断 Mac 没睡

本窗口不 SSH，因此不能独立断言当前 Mac 在线；只能把证据分层：

- 总窗口实时状态：远程 Mac 在线，daemon PID `235920`，端口和 status 已验证。
- 本地落盘证据：`status-with-model-cache.json` 最新快照写入于 2026-06-19 21:39:04 +08:00，`remote_runtime.probed=true`，worker runtime 有 PID、GPU layers 和 Metal 信息。
- daemon 活性证据：`evolution-loop.out.log` 写入于 2026-06-19 21:45:17 +08:00，round 275 后已有 round 276 活动。

判断规则：

1. 若主窗口 status 仍显示 `stale=false`、`ledger_lag=0` 且 daemon PID 有效，可认为 Mac 没睡且 daemon 在推进。
2. 若本地 stdout/report 长时间不更新，或 daemon status 变为 stale/ledger_lag 增长，应按需要重新只读验证，不要直接 prompt 或重启。
3. 本窗口没有实时 SSH 证据时，不把本地文件 mtime 单独当作“Mac 当前在线”的最终证明。

缺失的 sleep-prevention evidence：

- 没有本窗口采集的 `pmset -g assertions`、caffeinate/launchd assertion、或远程电源策略证据。
- 没有跨睡眠/唤醒事件的 daemon 自动续跑样本。
- 没有端口掉线后自动恢复或告警闭环样本。
- 没有多小时连续 `stale=false`、`ledger_lag=0`、worker `6/6`、Metal ok 的时间窗 artifact。

因此当前只能说“总窗口和本地 artifact 支持 Mac 近期在线且 daemon 近期推进”，不能说“睡眠防护已长期验证”。

## 如何判断 Metal 在用

可用证据：

- `status-with-model-cache.json` 的每个 worker 都有 `runtime_accelerator=metal`。
- `remote_runtime.acceleration_ok=true`。
- `remote_runtime.cpu_or_no_gpu_count=0`。
- `remote_runtime.workers[]` 中每个 worker `gpu_layers=999`，`cpu_or_no_gpu=false`。
- daemon report 的 `remote_chain.remote_runtime.acceleration_ok=true`。
- daemon report 的 `model_pool.capacity.metal_worker_count=6`、`cpu_worker_count=0`、`quality_runtime_accelerated=true`。

判断规则：

1. 若 `remote_runtime.acceleration_ok=true` 且 `cpu_or_no_gpu_count=0`，说明当前快照里的 worker 都不是 CPU/no-GPU 路径。
2. 若任一 worker `runtime_accelerator` 非 `metal`、`gpu_layers` 为 0、或 `cpu_or_no_gpu=true`，必须阻断 model-pool 扩容和 unattended 常驻复核。
3. quality worker 必须保持 Metal 加速，因为它是主推理 worker；helper worker 也应保持 Metal，避免挤占主开发机资源。

## 如何判断 Report 不 Stale

可用证据：

- 总窗口 daemon status：report rounds `275`、`ledger_lag=0`、`stale=false`、`gate_failures=0`。
- stdout：round `275` 已 `report_refresh:done ... rounds=275 ... failures=0`。
- `report.json`：`rounds=275`，`last.round=275`，`last.success=true`。
- `continuation_gate_report_v1.allow_unattended_continuation=true`。
- `report_gate.passed=true`。

判断规则：

1. daemon status 是当前性最强的证据；`stale=false` 和 `ledger_lag=0` 优先于旧的 `target\remote-gemma-unattended\*` 历史快照。
2. `report.json.rounds` 必须与最新 ledger/status 对齐；若 ledger_lag 增长或 latest ledger 新于 report，必须重新 refresh report。
3. `strict_report_gate.passed=false` 需要展示为历史严格门债务，不应误解为 round 275 stale；但若常驻策略要求 strict gate 也通过，则还需要处理历史 runtime response failure 债务。

## 长期证据采集计划

以下 checklist 是可执行计划，不是本窗口自动执行队列。默认不 SSH、不启动/停止模型、不发送 prompt；凡是 `requires_authorization=true` 的项都必须由总窗口或用户明确授权后执行。

| id | 验收目标 | 候选命令/证据 | SSH | sends_prompt | starts_process | requires_authorization | 通过标准 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `mac_sleep_prevention` | 证明远程 Mac 有防睡眠/保活策略 | `ssh smartsteam-mac pmset -g assertions`、`ssh smartsteam-mac pmset -g custom`、launchd/caffeinate 状态截图或文本 artifact | yes | no | no | yes | 有 active assertion 或等价策略；采样期间无 sleep/wake 导致 daemon/端口中断 |
| `daemon_status_window` | 多小时 daemon report 不 stale | 周期性只读 daemon status：rounds、ledger_lag、stale、gate_failures、PID；保存到 `target\remote-gemma-residency-window\sample-*` | no if local status; yes if remote status | no | no | yes if command touches daemon/remote | 至少 3 个样本，跨度 >= 2 小时，`stale=false`、`ledger_lag=0`、`gate_failures=0`、PID 一致或有明确重启事件 |
| `metal_worker_stability` | worker 长时间保持 6/6 healthy 且全部 Metal | `status-with-model-cache.json` 多样本，或 `gemma-chain.cmd pool-status -JsonStatus` 输出 artifact | no for local file; command may touch remote depending implementation | no | no | yes for live probe | 每个样本 worker `6/6` healthy，`metal_worker_count=6`，`cpu_worker_count=0`，`remote_runtime_acceleration_ok=true` |
| `model_cache_stability` | cache 5/5 持续 ok | `model-cache-status.json` 多样本，记录 all_ok、ok_count、remote_error_count、copy_needed | no for local file; yes if refreshing remote cache check | no | no | yes if refreshing cache probe | `all_ok=true`、5/5 ok、0 remote error、0 copy needed，且 SHA/模型角色不漂移 |
| `disconnect_recovery` | 断线/端口异常后能恢复或被告警 | 人工触发或自然发生的端口断开事件、daemon status、stdout、恢复记录；不得在本窗口触发 | maybe | no | maybe if recovery start is tested | yes | 有明确 incident timeline：detect -> block prompt/launch -> recover/alert -> report fresh；不能靠单次正常状态替代 |
| `web_lab_reachability` | Web Lab 与 backend 可达且不发 prompt | `curl http://127.0.0.1:8789/` 或浏览器只读打开首页；backend health endpoint 若存在 | no local; yes if remote host direct | no | no | yes if live check | HTTP 200/可打开首页；没有 prompt request；backend `7979` 与 Web Lab `8789` 状态一致 |
| `ledger_report_alignment` | report 与 ledger 长期对齐 | daemon status `ledger_lag=0`，`report.json.rounds` 与最新 ledger round 对齐，stdout 有 `report_refresh:done` | no for local files | no | no | no for file read; yes for live status | 每个样本 report rounds 等于 latest ledger round，`stale=false`，无 report refresh failure |
| `resource_headroom_window` | 远程内存/Metal 资源余量稳定 | approved owner flow 采集 `remote-resource-status.json` / `resource-headroom.json` | yes | no | no | yes | 至少 3 个样本，跨度 >= 2 小时，内存余量高于阈值，Metal/GPU available，无 swap/压力告警 |

推荐 artifact 目录：

```text
target\remote-gemma-residency-window\sample-YYYYMMDD-HHMMSS\
  daemon-status.json
  status-with-model-cache.json
  model-cache-status.json
  report.json
  ledger-tail.json
  stdout-tail.txt
  resource-headroom.json
  web-lab-health.txt
```

每个 sample 应记录：

- `collected_at_utc`
- `collector_window_id`
- `read_only=true`
- `sends_prompt=false`
- `starts_process=false`
- `touches_remote=true/false`
- `requires_authorization=true/false`
- 原始命令文本
- 命令退出码

长期验收建议：

1. 先由总窗口授权一个只读 owner flow，明确是否允许 SSH、daemon status、HTTP health check。
2. 采集至少 2 小时、至少 3 个 sample；若目标是“长期稳定”，建议扩展到 overnight window。
3. 每个 sample 必须同时包含 daemon freshness、worker/Metal、model cache、report/ledger alignment、resource headroom。
4. 任一 sample 出现 `stale=true`、`ledger_lag>0`、worker 少于 `6/6`、`remote_runtime_acceleration_ok=false`、Web Lab 不可达、或资源压力不足，应标记 window failed。
5. window failed 时只允许生成诊断报告，不自动重启、prompt、扩容或 SSH 修复，除非总窗口另行授权。

## 仍需保留的缺口

- 本窗口没有执行 SSH，因此没有本窗口独立的 Mac 在线/端口实时检查。
- 本窗口没有执行 daemon status 命令，只整理总窗口 status 与本地落盘文件。
- 没有新增连续资源窗口 artifact，例如 Metal/内存余量多样本窗口。
- 没有证明长期睡眠恢复策略，例如 Mac 睡眠后自动唤醒、daemon 自动续跑、端口掉线自动恢复。
- 没有授权 Web Lab/Forge/CLI/evolution-loop 发 prompt 或启动/停止服务；这些仍需要用户明确授权和对应 gate。
