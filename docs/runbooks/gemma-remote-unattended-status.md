# Gemma Remote Unattended Status

本文记录截至 2026-06-16 当前已落盘的远程 Gemma 模型池与 unattended evolution 状态。它是只读诊断说明，不是启动手册；除用户明确要求外，不启动模型、不 SSH、不发送 prompt、不停止任何现有进程。

2026-06-19 最新运行态证据见 `docs\runbooks\gemma-runtime-evidence-2026-06-19.md`。本文件中 2026-06-15/16 的 stale/report-ledger mismatch 结论保留为历史背景，不代表最新 daemon/report 状态。

下游 Web Lab/Forge/CLI/evolution-loop 的机器消费规则见 `docs\architecture\integration-gemma-readiness.md`；本文只记录 runbook 视角的证据和安全命令。

## 证据边界

以下文件是历史快照或上一轮运行产物。读取它们不会探测远程、不会触发模型、不会发 prompt。

| 文件 | 快照时间 | 证据类型 | 当前结论 |
| --- | --- | --- | --- |
| `target\remote-gemma-chain\model-cache-status.json` | 2026-06-16 10:23:00 +08:00 | 模型缓存/远程副本校验快照 | `all_ok=true`，5/5 model ok，0 remote error，0 copy needed；默认 30 分钟 freshness 下已 stale |
| `target\remote-gemma-chain\status-with-model-cache.json` | 2026-06-16 11:09:54 +08:00 | 链路与模型池状态快照 | `readiness.ready=true`，model/backend/Web Lab 均 true，6/6 worker healthy |
| `target\remote-gemma-unattended\evolution-report.json` | 2026-06-15 19:14:11 +08:00 | unattended evolution 汇总 report | 4 rounds，4 success，0 failures，report gate passed |
| `target\remote-gemma-unattended\evolution-ledger.jsonl` | 2026-06-16 10:23:20 +08:00 | 逐轮 ledger 历史记录 | 5 lines，latest round 5 failed；与 report 的 4/4 success 不一致 |

注意：这些文件证明“写入快照时”的状态，不等价于当前远程进程仍然存在。需要常驻确认时，先重新生成只读状态快照，再考虑任何启动或 prompt 入口。

## 实时端口与本地快照分层

2026-06-15 本轮总控同步的外部实时状态是：本机到苹果机 `smartsteam-mac` 已完成 SSH 免密，远程 Darwin 机器在线，远程端口 `8686/8687/8688/8689/8690` 有 `llama-server` 监听。这个信息来自主窗口同步；本窗口未执行 SSH、未探测远程端口、未启动或停止任何模型、未发送推理 prompt。

本窗口当前可直接复核的是本地落盘快照：

- `target\remote-gemma-chain\status-with-model-cache.json`
- `target\remote-gemma-chain\model-cache-status.json`
- `target\remote-gemma-unattended\evolution-report.json`
- `target\remote-gemma-unattended\evolution-ledger.jsonl`

因此读法必须分层：

| 层级 | 当前证据 | 可说明 | 不能说明 |
| --- | --- | --- | --- |
| 外部实时端口同步 | 主窗口报告 `smartsteam-mac` 在线，`8686-8690` 有 `llama-server` 监听 | 远程机器和 worker 端口在主窗口检查时存在 | 本窗口已授权 SSH、prompt、launch 或常驻 |
| 本地链路快照 | `status-with-model-cache.json` | 快照写入时 model API/backend/Web Lab ready，workers `6/6` healthy | 当前端口仍健康或远程资源仍充足 |
| 本地模型缓存快照 | `model-cache-status.json` | 快照写入时 5 个模型角色 cache ok，0 copy needed，0 remote error | 当前模型文件/远程挂载仍可用 |
| 本地 unattended 快照 | `evolution-report.json`、`evolution-ledger.jsonl` | report 仍是历史 `4/4` 成功；ledger 最新 round 5 失败 | 当前 daemon 正在/未在运行，或可以继续发 prompt |

### 端口和服务关系

本地快照里的链路关系是：

| 服务/角色 | 端口 | 证据来源 | 说明 |
| --- | --- | --- | --- |
| quality/model API | `8686` | 快照 + 主窗口端口同步 | 主要模型 API/quality worker |
| summary worker | `8687` | 快照 + 主窗口端口同步 | 摘要角色 worker |
| review/test-gate worker | `8688` | 快照 + 主窗口端口同步 | review 与 test-gate 复用该 worker/模型 |
| router worker | `8689` | 快照 + 主窗口端口同步 | 路由角色 worker |
| index worker | `8690` | 快照 + 主窗口端口同步 | 索引角色 worker |
| backend | `7979` | `status-with-model-cache.json` | Web/CLI 后端入口，连接模型 API/worker 池 |
| Web Lab | `8789` | `status-with-model-cache.json` | 浏览器实验入口，经 backend/模型链路发起交互 |

`status-with-model-cache.json` 记录 `read_only=true`、`starts_process=false`、`sends_prompt=false`、`touches_remote=false` 和 `remote_probe_skipped=true`。它是本地状态快照，不是远程探测。即使主窗口同步了 `8686-8690` 的实时监听状态，Web Lab/Forge/CLI/evolution-loop 仍必须按 readiness contract 读取 `authorization.*=false` 和 `consumer_projection[].current_allowed=false`，不能直接发 prompt 或启动常驻。

如果后续用户明确要求本窗口 SSH，只允许使用只读诊断命令确认远程状态，例如检查 Darwin、端口监听和进程摘要；不得启动/停止模型、不得发送推理 prompt、不得写模型权重。默认仍不 SSH。

## 历史证据不授权当前常驻

当前落盘证据可以说明上一组快照里：

- 模型缓存是 `5/5`：本地/远程模型文件在当时校验一致。
- 模型池是 `6/6`：当时 `quality,summary,review,router,test-gate,index` worker 都记录为 healthy。
- unattended evolution report 是 `4/4`：当时 4 轮全部成功，report gate passed。
- unattended ledger 更新到了 round `5` 且 `success=false`：runtime 返回 0 tokens、runtime model 缺失，触发 runtime response gate failed；这会阻断把旧 report 当作当前 readiness。

这些结论都只是历史证据。它们不能证明现在的远程 Mac 仍有相同进程、端口、Metal/内存余量或 daemon 状态，也不能授权常驻、launch、SSH 或 prompt。

机器读法：

- `summary.evidence_fresh_all=false`：至少一份核心 evidence 已超过 freshness 窗口或缺失。
- `residency_decision.classification=blocked_stale_evidence`：下一步只能补只读证据。
- `authorization.can_authorize_daemon=false`
- `authorization.can_authorize_launch=false`
- `authorization.can_authorize_prompt=false`
- `authorization.can_authorize_ssh=false`

只有重新生成当前状态、确认 daemon/report gate、确认没有重复常驻、重查 prompt/launch gates，并补齐连续端口健康与远程资源余量窗口后，别的窗口才可以进入下一层 gate。本文、`read-remote-unattended-snapshot.ps1` 和 `read-remote-evidence-freshness.ps1` 永远不直接放行。

## 只读重查观察

2026-06-15 21:06 +08:00 左右，本窗口运行了一组本地只读检查。未 SSH、未启动或停止模型、未发送 prompt。

运行入口：

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd pool-status -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd status-bundle -JsonStatus
.\tools\smartsteam-forge\evolution-daemon.cmd -JsonStatus -WorkDir target\remote-gemma-unattended
```

观察到的只读证据：

- `chain-status -JsonStatus`：`classification=prompt_ready`，quality worker `127.0.0.1:8686`、backend `127.0.0.1:7979`、Web Lab `127.0.0.1:8789` 均 reachable/health OK；该命令本身 read-only、does not send prompt。
- `pool-status -JsonStatus`：`launch_allowed=true`，worker `6/6` healthy；但 runtime metadata 是 unknown，`capacity.expansion_allowed=false`，recommendation 为 `verify_worker_runtime_metadata_before_expansion`。
- `status-bundle -JsonStatus`：顶层 `read_only=true`、`sends_prompt=false`、`launches_process=false`，但它报告 prompt/pool gate 当前 allowed；这只表示 gate 输出，不等价于本窗口授权发送 prompt 或 launch。
- `evolution-daemon.cmd -JsonStatus`：`read_only=true`、`starts_process=false`、`sends_prompt=false`；daemon `running=false`，`report_gate_preflight.continuation_state=no_report`，`unattended_start_plan.can_start=true`，但 start command 仍是 launch 入口，默认不要运行。

这些重查结果只补充“当前本地只读入口可读”的证据。它们仍不能替代：

- 连续端口健康窗口。
- 远程资源/Metal/内存余量窗口。
- 操作前即时 gate。
- 用户明确授权的 SSH、launch 或 prompt 操作。

因此 `read-remote-unattended-snapshot.ps1` 继续保持 `authorization.*=false` 和 consumer projection 全 blocked，这是有意的 fail-closed 行为。

## 当前模型池快照

`status-with-model-cache.json` 记录：

- `read_only=true`
- `starts_process=false`
- `sends_prompt=false`
- `touches_remote=false`
- `remote_probe_skipped=true`
- model API `127.0.0.1:8686`、backend `127.0.0.1:7979`、Web Lab `127.0.0.1:8789` 在快照中均 ready
- required roles `summary,router,review,index,test-gate` 全部 ready，missing required roles 为空
- capacity recommendation 为 `hold_or_add_optional_test_gate_if_memory_pressure_green`

Worker 快照：

| role | port | status | context | max tokens | accelerator |
| --- | --- | --- | --- | --- | --- |
| quality | 8686 | healthy | 65536 | 4096 | metal |
| summary | 8687 | healthy | 8192 | 768 | metal |
| review | 8688 | healthy | 4096 | 1024 | metal |
| router | 8689 | healthy | 4096 | 512 | metal |
| test-gate | 8688 | healthy | 4096 | 768 | metal |
| index | 8690 | healthy | 8192 | 512 | metal |

模型缓存快照显示 `quality,summary,review,router,index` 五个模型本地和远程 SHA-256 一致；`test-gate` 复用 `review` worker/模型。

## Unattended Evolution 快照

`evolution-report.json` 记录：

- `rounds=4`
- `success=4`
- `failures=0`
- `success_rate=100.0`
- runtime tokens total `2877`
- validation `3/3`
- self-improve `4/4`
- recent failures `0`
- test-gate latest verdict `pass`
- latest validation command: `cargo test -q --manifest-path tools/evolution-loop/Cargo.toml`
- remote chain ready，pool workers `6/6`

`evolution-ledger.jsonl` 最新记录：

- latest round `5`
- case `smartsteam-evolution-loop-0005`
- `success=false`
- error: runtime response gate failed，因为 runtime tokens 为 `0` 且 runtime model 缺失
- runtime model `null`
- runtime tokens `0`
- elapsed `1577 ms`
- validation 未检查，self-improve passed

## 只读复查命令

这些命令只读落盘文件或调用已存在的只读诊断入口。它们不启动模型、不发 prompt。

优先用汇总脚本生成稳定摘要：

```powershell
.\tools\gemma-chain\scripts\read-remote-unattended-snapshot.ps1
.\tools\gemma-chain\scripts\read-remote-unattended-snapshot.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-unattended-snapshot.ps1 -FreshMinutes 10 -Json
.\tools\gemma-chain\scripts\read-remote-evidence-freshness.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-readiness-contract.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-residency-gap-report.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-evidence-package-plan.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-evidence-package-status.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-owner-flow-handoff.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-surface-preflight.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-surface-preflight.ps1 -ConsumerId web_lab_prompt -Json
.\tools\gemma-chain\scripts\read-remote-consumer-preflight.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-consumer-preflight.ps1 -ConsumerId web_lab_prompt -Json
.\tools\gemma-chain\scripts\read-remote-link-boundary.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-dashboard-status.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-action-matrix.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-evolution-loop-guard.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-model-pool-guard.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-contract-manifest.ps1 -Json
```

该脚本的契约是 `read_only=true`、`starts_process=false`、`sends_prompt=false`、`touches_remote=false`、`writes_files=false`。它只读取下面四个 evidence 文件，并提醒这些文件是历史快照，不能直接授权 daemon、launch、SSH 或 prompt 动作。

JSON 输出会额外给每个 evidence 文件标出 `age_seconds` 和 `fresh`，默认 freshness 窗口是 30 分钟，可用 `-FreshMinutes` 调整。`authorization.can_authorize_daemon`、`can_authorize_launch`、`can_authorize_prompt`、`can_authorize_ssh` 永远是 `false`；脚本只做摘要，不能变成放行器。`residency_gaps[]` 是常驻前仍需补证据的机器可读清单。

`safe_next_read_only_commands[]` 给出下一步可收集证据的候选命令。它是交接清单，不是自动执行队列；执行前仍要确认当前窗口是否允许触碰对应入口。列表内命令必须保持 `read_only=true`、`starts_process=false`、`sends_prompt=false`、`touches_remote=false`。

`residency_evidence_checklist[]` 把每个 `residency_gaps[]` 项映射到需要补齐的证据、建议 proof source 和可选 `safe_command_id`。所有 checklist 项默认 `blocks_authorization=true`；只有独立的当前证据证明通过后，别的窗口才可以考虑进入下一层 gate。本文档和 snapshot 脚本本身永远不放行常驻、launch、SSH 或 prompt。

`read-remote-evidence-freshness.ps1 -Json` 是专门解释 `fresh_snapshot` 缺口的轻量 reader。它只读取 snapshot summary 和 evidence package status，不触远程、不启动模型、不发送 prompt、不写文件。输出会列出每个 evidence 文件的 freshness，并显式标记：

- `report_ledger_round_mismatch=true`：latest ledger round 5 新于 report 的 4 rounds。
- `latest_ledger_failed=true`：latest ledger round 5 失败。
- `requires_unattended_report_refresh=true`：需要刷新或调和 unattended report/ledger 后，才能进入外部 residency gate 复核。

它没有放行能力；`authorization.can_authorize_daemon=false`、`can_authorize_launch=false`、`can_authorize_prompt=false`、`can_authorize_ssh=false`。

`read-remote-readiness-contract.ps1 -Json` 会额外输出 `missing_evidence_actions[]`，把缺失证据映射到 checklist、`safe_command_id` 和只读命令；`pending_external_gate_actions[]` 则描述证据补齐后仍必须通过的外部 gate。两个字段都用于展示/交接，不是自动执行队列；即使 action 里列出 `forge_daemon_start_check`，它也只是 dry-run preflight，不等于允许 `Start`。

同一输出里的 `generated_at_utc` 只表示本地 contract 生成时间；底层快照 freshness 仍以 `source_status.snapshot.evidence[]`、`summary.max_evidence_age_seconds` 和 `summary.evidence_fresh_all` 为准。

`read-remote-residency-gap-report.ps1 -Json` 是给总控/Web Lab/Forge 展示用的投影报告。它复用 readiness contract 和 snapshot reader，合并输出 `decision`、`snapshot_claims`、`freshness`、`checklist`、`consumers` 与 `safety`。其中 `snapshot_claims.historical_only=true`，`decision.authorized=false`，`authorization.*=false`；它不能授权 daemon、launch、SSH 或 prompt，只是把当前缺口讲清楚。

`read-remote-evidence-package-plan.ps1 -Json` 是给后续获授权窗口准备采样包的只读规格输出。它不采集 evidence、不写 artifact、不 SSH，只把这三类证据写成机器可读计划：

- fresh snapshot package：需要更新/证明的链路、模型缓存、unattended report 和 ledger evidence。
- observation window package：`target\remote-gemma-observation-window\sample-*` 中每个 sample 应包含 `chain-status.json`、`pool-status.json`、`status-bundle.json`、`forge-daemon-status.json`，默认至少 3 个 sample、跨度 10 分钟。
- resource window package：`target\remote-gemma-resource-window\sample-*` 中每个 sample 应包含 `remote-resource-status.json`、`resource-status.json` 或 `resource-headroom.json`，并声明 approved owner flow、内存余量和 Metal/GPU 可用性。

这个 plan 只说明“要收集什么”和“用什么 reader 验证”。真正采样会写 artifact，必须由明确授权的 owner flow 执行；本窗口默认不做。

`read-remote-evidence-package-status.ps1 -Json` 用来轮询证据包是否已经达标。它读取 readiness、observation/resource reader 和 evidence package plan，输出 `package_items[]`：

- `fresh_snapshot`
- `unattended_report_ledger_consistency`
- `continuous_port_worker_window`
- `remote_resource_headroom_window`
- `fail_closed_contracts`

当前若只有 `fail_closed_contracts.ready=true`，说明安全合同是好的，但真实 freshness、unattended report/ledger 一致性、连续窗口和资源窗口仍缺。`unattended_report_ledger_consistency.ready=false` 时，通常表示 latest ledger 新于 report、latest ledger failed，或 `requires_unattended_report_refresh=true`；此时旧 `4/4` report 不能代表当前 unattended readiness。即使未来 `summary.package_ready_for_external_gate=true`，它也只表示证据包可以交给外部 gate 复核，不授权 daemon、launch、SSH 或 prompt。

`read-remote-owner-flow-handoff.ps1 -Json` 用来给获授权接手窗口生成 owner-flow 交接清单。它读取 evidence package status/plan 和 gap report，输出 staged handoff items：

- `collect_fresh_snapshot_package`
- `collect_observation_window_package`
- `collect_resource_window_package`
- `external_residency_gate_review`

其中会写 artifact 或触碰远程的 item 都必须 `requires_explicit_user_authorization=true`。该 handoff 不是执行队列；它自身 `read_only=true`、`writes_files=false`、`touches_remote=false`、`authorization.*=false`。

`read-remote-consumer-preflight.ps1 -Json` 是 Web Lab/Forge/CLI/model-pool/SSH 入口的窄口检查。可选 `-ConsumerId`：

```text
web_lab_prompt
forge_cli_prompt
backend_cli_direct_prompt
evolution_loop_prompt_round
model_pool_launch
forge_daemon_residency
ssh_remote_probe
```

该脚本只读取 readiness contract 的 `consumer_projection[]`，解析 `safe_command_id` 到只读命令，并输出 `current_allowed=false`、`blocked_by[]` 和 `safe_command`。它不发 prompt、不启动、不 SSH，也不授权任何 consumer。

CLI/CI gate 可显式加 `-FailOnBlocked`。默认不带该开关时 blocked 仍 exit 0，方便 UI 读取 JSON；带 `-FailOnBlocked` 时，当前 blocked 返回 exit code `2`，未知 consumer id 返回 exit code `3`。

`read-remote-surface-preflight.ps1 -Json` 是 Web Lab/Forge/CLI 顶部状态条的首选轻量入口。它只读取一次 readiness contract，返回当前 `display`、`status`、`consumers[]`、`missing_evidence_actions[]` 和 `pending_external_gate_actions[]`；单个入口也可用 `-ConsumerId` 过滤。`display` 里有可直接渲染的 `severity`、`headline`、`detail`、`primary_missing_evidence`、`next_action_label` 和 `badge`。它比 quick contract 更适合频繁轮询，但仍不是授权器。

`surface preflight` 也支持 `-FailOnBlocked`，退出码语义同 consumer preflight。

`read-remote-contract-manifest.ps1 -Json` 是给 Web Lab/Forge/CLI 发现只读 reader、自检入口、推荐超时和退出码语义的 manifest。它只读取 surface preflight，不启动模型、不发 prompt、不 SSH、不写文件。manifest 当前列出 `blocked_exit_code=2`、`unknown_consumer_exit_code=3`，并要求所有 `readers[]` 和 `selftests[]` 均保持 read-only/fail-closed；它不是执行队列，也不能授权 daemon、launch、SSH 或 prompt。

`read-remote-link-boundary.ps1 -Json` 是给 Web Lab/Forge/CLI/总控展示链路拓扑和证据边界的只读摘要。它只读取本地 snapshot/readiness，不 SSH、不探测端口、不触远程。输出会把 worker 端口 `8686-8690`、backend `7979`、Web Lab `8789` 和外部同步的 `smartsteam-mac` 端口事实分层展示，并固定标记 `realtime_ports_verified_by_this_script=false`、`historical_snapshot_authorizes_current_residency=false`。

`read-remote-dashboard-status.ps1 -Json` 是 Web Lab/Forge/CLI 可以直接渲染的状态面板 reader。它聚合 surface preflight、link boundary 和 evidence package status，输出 `display`、`dashboard_cards[]`、`package_items[]`、`topology` 和 `consumers[]`。它只适合展示/轮询，不采集 evidence、不写 artifact、不 SSH、不发送 prompt；`action_lock` 卡片和 `authorization.*=false` 是核心输出。

`read-remote-action-matrix.ps1 -Json` 是 Web Lab 按钮、Forge/CLI 子命令、evolution-loop prompt round、model-pool launch、daemon residency 和 SSH probe 的统一禁用矩阵。它只读取 consumer preflight/dashboard，不执行任何入口动作。当前 7 个 `actions[]` 都必须 `current_allowed=false`、`ui_enabled=false`、`cli_may_execute=false`；每项的 verifier command 只是人工补证据提示，不是自动执行队列。

`read-remote-evolution-loop-guard.ps1 -Json` 是 self-evolution loop/Forge daemon 的窄口 guard。它只读取 action matrix、owner-flow handoff 和 evidence package status，不调用 evolution-loop、不启动 daemon、不发 prompt。当前必须输出 `may_send_prompt_round=false`、`may_start_or_resume_daemon=false`、`may_enter_resident_loop=false`。默认 blocked 仍 exit 0 方便 UI/CLI 读 JSON；带 `-FailOnBlocked` 时 blocked 返回 exit code `2`。

`read-remote-model-pool-guard.ps1 -Json` 是 model-pool worker launch/expand 的窄口 guard。它只读取 action matrix、link boundary 和 evidence package status，不启动 worker、不扩容、不 SSH。当前必须输出 `may_launch_worker=false`、`may_expand_pool=false`、`may_reuse_snapshot_as_current_capacity=false`；历史 worker `6/6` 和 cache `5/5` 只用于展示，不授权当前容量或扩容。

`residency_decision` 是给 UI/CLI/Forge 消费的总判定。若 `can_proceed_to_resident_loop=false` 或 `read_only_evidence_collection_only=true`，后续窗口只能继续采集只读证据，不能把历史快照当成当前常驻许可。

`consumer_projection[]` 是给 Web Lab、SmartSteam Forge、Backend CLI、evolution-loop、model-pool 和远程主机入口使用的统一 fail-closed 视图。Snapshot-only 状态下，`web_lab_prompt`、`forge_cli_prompt`、`backend_cli_direct_prompt`、`evolution_loop_prompt_round`、`model_pool_launch`、`forge_daemon_residency`、`ssh_remote_probe` 都必须 `current_allowed=false`，并指向对应的只读 `safe_command_id` 去补证据。

`consumer_contract` 固定 `consumer_projection[]` 的机器契约。当前版本是 `smartsteam.remote-gemma-unattended.consumer-projection.v1`，`fail_closed_default=true`，`allowed_requires_external_gates=true`。任何 Web/Forge/CLI/evolution-loop/model-pool 消费者遇到缺字段、未知版本或无法解析 `safe_command_id` 时，都必须按 blocked 处理。

若后续窗口已经把只读重查输出保存成本地 JSON 文件，可以用 `-LocalObservationDir` 让 snapshot 脚本吸收这些证据，但脚本本身不会执行命令：

```powershell
.\tools\gemma-chain\scripts\read-remote-unattended-snapshot.ps1 -LocalObservationDir target\remote-gemma-observations -Json
```

目录约定：

```text
chain-status.json
pool-status.json
status-bundle.json
forge-daemon-status.json
```

`local_observation` 只做字段摘录，例如 chain classification、pool worker count、status-bundle read-only 标记、daemon running 和 report gate continuation state。它仍不授权 prompt、launch、SSH 或 resident loop；`authorization.*` 和 `consumer_projection[].current_allowed` 继续 fail closed。

当 `local_observation` 存在且可解析时，相关 checklist 项会从 `not_rechecked_by_this_script` 变为 `observed_once_insufficient`，端口健康会变为 `single_sample_observed_window_missing`。这些状态只说明“有过一次只读观察”，不会移除 `residency_gaps[]`，也不会替代连续健康窗口或远程资源余量窗口。

`local_observation.summary` 还暴露 `complete_parse_ok`、`single_sample_only`、`window_sample_count` 和 `continuous_window_present`。当前 `-LocalObservationDir` 只表示一个本地 observation bundle；除非未来有独立 monitor artifact 证明连续窗口，否则 `continuous_window_present=false` 必须继续阻断常驻。

连续窗口的本地 artifact 可以用另一个只读 reader 验证：

```powershell
.\tools\gemma-chain\scripts\read-remote-observation-window.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-observation-window.ps1 -WindowDir target\remote-gemma-observation-window -MinSamples 3 -MinSpanMinutes 10 -Json
```

该 reader 只读取已经落盘的 sample 目录，不执行 `gemma-chain.cmd`、不调用 Forge、不断言远程当前状态。每个 sample 目录约定包含：

```text
chain-status.json
pool-status.json
status-bundle.json
forge-daemon-status.json
```

`summary.continuous_window_present=true` 只表示本地 artifact 满足样本数、时间跨度、解析和只读契约检查；它仍不是授权。`authorization.can_authorize_daemon`、`can_authorize_launch`、`can_authorize_prompt`、`can_authorize_ssh` 仍固定为 `false`，后续常驻还必须经过外部即时 gate 和用户明确授权。

连续窗口 reader 还会检查每个 sample 的健康字段：chain 必须是 `prompt_ready`/`ready` 或明确给出 model API、backend、Web Lab 均 healthy；pool 必须有至少 6 个 worker 且 healthy worker 数等于 worker 总数。若缺字段会输出 `status=health_unknown`，若有不健康样本会输出 `status=health_not_ready`，两者都不能进入常驻复核。

远程资源/Metal/内存余量窗口也只能读取已经落盘的本地 artifact：

```powershell
.\tools\gemma-chain\scripts\read-remote-resource-window.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-resource-window.ps1 -WindowDir target\remote-gemma-resource-window -MinSamples 3 -MinSpanMinutes 10 -MinAvailableMemoryGb 8 -Json
```

该 reader 不 SSH、不执行采集命令、不触碰远程，只接受本地 sample 目录中的 `remote-resource-status.json`、`resource-status.json` 或 `resource-headroom.json`。每个资源 sample 必须声明 `read_only=true`、`starts_process=false`、`sends_prompt=false`、`writes_model_weights=false` 和 `approved_owner_flow=true`，并提供可解析的 `memory_available_gb`/`memory_available_bytes` 与 `metal_available`/`gpu_available`。即使 `summary.resource_window_present=true`，它也只表示“资源证据足够进入外部 gate 复核”，不授权 daemon、launch、SSH 或 prompt。

需要看原始字段时再读源文件：

```powershell
Get-Content -Raw target\remote-gemma-chain\model-cache-status.json | ConvertFrom-Json
Get-Content -Raw target\remote-gemma-chain\status-with-model-cache.json | ConvertFrom-Json
Get-Content -Raw target\remote-gemma-unattended\evolution-report.json | ConvertFrom-Json
Get-Content target\remote-gemma-unattended\evolution-ledger.jsonl -Tail 1 | ConvertFrom-Json
```

Wrapper/contract 只读自检：

```powershell
.\tools\gemma-chain\scripts\test-read-remote-unattended-snapshot.ps1
.\tools\gemma-chain\scripts\test-read-remote-evidence-freshness.ps1
.\tools\gemma-chain\scripts\test-read-remote-observation-window.ps1
.\tools\gemma-chain\scripts\test-read-remote-resource-window.ps1
.\tools\gemma-chain\scripts\test-read-remote-readiness-contract.ps1
.\tools\gemma-chain\scripts\test-read-remote-residency-gap-report.ps1
.\tools\gemma-chain\scripts\test-read-remote-evidence-package-plan.ps1
.\tools\gemma-chain\scripts\test-read-remote-evidence-package-status.ps1
.\tools\gemma-chain\scripts\test-read-remote-owner-flow-handoff.ps1
.\tools\gemma-chain\scripts\test-read-remote-consumer-preflight.ps1
.\tools\gemma-chain\scripts\test-read-remote-surface-preflight.ps1
.\tools\gemma-chain\scripts\test-read-remote-link-boundary.ps1
.\tools\gemma-chain\scripts\test-read-remote-dashboard-status.ps1
.\tools\gemma-chain\scripts\test-read-remote-action-matrix.ps1
.\tools\gemma-chain\scripts\test-read-remote-evolution-loop-guard.ps1
.\tools\gemma-chain\scripts\test-read-remote-model-pool-guard.ps1
.\tools\gemma-chain\scripts\test-read-remote-contract-manifest.ps1
.\tools\gemma-chain\scripts\test-remote-consumer-contract.ps1
.\tools\gemma-chain\scripts\test-remote-readiness-quick-contract.ps1
.\tools\gemma-chain\scripts\test-remote-readiness-readonly-contract.ps1
.\tools\gemma-chain\gemma-chain.cmd selftest
.\tools\gemma-chain\gemma-chain.cmd chain-status -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd pool-status
.\tools\gemma-chain\gemma-chain.cmd status-bundle -JsonStatus
```

`test-read-remote-unattended-snapshot.ps1` 使用临时 fixture 验证汇总脚本的安全契约和核心字段形状；它不读取真实模型状态，不触远程，不发送 prompt，结束时删除自己的临时目录。

`test-read-remote-evidence-freshness.ps1` 验证 freshness reader 只读、fail-closed，覆盖 stale evidence 计数、report/ledger mismatch、latest ledger failed、`requires_unattended_report_refresh` 和 `fresh_snapshot` 缺口。它不 SSH、不启动模型、不发送 prompt、不写模型权重。

`test-read-remote-readiness-contract.ps1` 使用完整临时 fixture 验证 fresh snapshot、连续端口窗口和资源窗口都存在时，readiness contract 可以进入 `can_support_external_residency_review=true`，但仍只暴露 `pending_external_gates=residency_external_gate`，且 `authorization.*` 和 `consumer_projection[].current_allowed` 继续 fail closed。

`test-read-remote-residency-gap-report.ps1` 验证展示/交接用 report 的只读契约、历史快照标记、checklist 字段、safe command 解析、consumer fail-closed 和 `authorization.*=false`。

`test-read-remote-evidence-package-plan.ps1` 验证 evidence package plan 不采集、不写 artifact，并且 observation/resource 采样包规格完整、仍需 approved owner flow、`authorization.*=false`。

`test-read-remote-evidence-package-status.ps1` 验证 evidence package status 的 package item、artifact 位置、read-only verifier、fail-closed 授权语义，以及 ready 证据包必须同时具备 fresh snapshot、observation window 和 resource window。

`test-read-remote-owner-flow-handoff.ps1` 验证 owner-flow handoff 的 staged item、显式授权边界、只读 verifier、artifact 位置和 fail-closed 授权语义。

`test-read-remote-consumer-preflight.ps1` 验证全部 consumer、单个 Web Lab consumer 和未知 consumer id 的窄口行为，确保所有 consumer blocked、safe command 可解析、`-FailOnBlocked` 退出码正确且 `authorization.*=false`。

`test-read-remote-surface-preflight.ps1` 验证 UI/CLI 轻量状态投影、`display` 字段、单 consumer 过滤、未知 consumer、safe command 解析、`-FailOnBlocked` 退出码和 fail-closed 授权语义。

`test-read-remote-link-boundary.ps1` 验证链路拓扑摘要不会把本地快照或外部同步笔记误当作本脚本实时端口验证，覆盖 worker/backend/Web Lab 端口、consumer fail-closed、next verifier 只读性和 `authorization.*=false`。

`test-read-remote-dashboard-status.ps1` 验证 dashboard 状态卡、证据包、拓扑、consumer 和推荐只读入口都保持 fail-closed，尤其是 `action_lock`、远程端口 `observed_by_this_script=false` 和 `authorization.*=false`。

`test-read-remote-action-matrix.ps1` 验证 7 个动作入口全部 disabled/blocked，CLI 不可执行，Web/Forge/CLI/evolution-loop/model-pool/daemon/SSH 风险标记正确，并且 verifier 命令只作为只读提示存在。

`test-read-remote-evolution-loop-guard.ps1` 验证 evolution-loop prompt round、Forge daemon residency 和 resident loop 全部 blocked，guard 不启动、不 prompt、不触远程，并验证 `-FailOnBlocked` 返回 exit code `2`。

`test-read-remote-model-pool-guard.ps1` 验证 model-pool launch/expand 全部 blocked，历史 worker/cache 快照不能作为当前容量授权，guard 不启动、不 prompt、不触远程，并验证 `-FailOnBlocked` 返回 exit code `2`。

`test-read-remote-contract-manifest.ps1` 验证 manifest 的 reader/selftest 清单、Web/Forge/CLI 可用退出码约定、当前 display 投影和 `authorization.*=false`，并拒绝任何会启动、prompt、SSH 或写文件的 manifest 条目。

`test-remote-readiness-quick-contract.ps1` 是 Web Lab/Forge/CLI 日常轮询用的轻量合同自检。它不跑 fixture，自身只读取 readiness、gap report、evidence package status、owner-flow handoff、consumer preflight、surface preflight、link boundary、dashboard status、action matrix、evolution-loop guard、model-pool guard 和 contract manifest，验证这些当前 reader 输出仍然只读、fail-closed、missing evidence 对齐、consumer/action blocked 状态、dashboard action lock、loop/daemon guard、model-pool guard、端口边界标记、manifest 退出码和 owner-flow item 不越权。当前约 118 秒内完成，运行时建议给 180 秒 timeout。

`test-remote-readiness-readonly-contract.ps1` 是给 Web Lab/Forge/CLI/evolution-loop 交接前使用的统一只读合同自检。它会运行三个 reader 的 fixture 自检，再读取默认 snapshot、observation window 和 resource window JSON，验证全部 fail-closed、consumer projection 全 blocked、safe command 列表不启动/不 prompt/不 SSH/不写文件。它仍不会授权任何 daemon、launch、SSH 或 prompt。

完整合同自检覆盖所有 fixture 和交接入口，当前会超过 120 秒边界；新增 link boundary/dashboard/action matrix/evolution-loop guard/model-pool guard 覆盖后实测约 418 秒，运行时建议给 540 秒 timeout。

`test-remote-consumer-contract.ps1` 专门验证 `consumer_projection[]` 与 `consumer_contract` 的下游契约：Web Lab、Forge CLI、Backend CLI、evolution-loop、model-pool、daemon residency 和 SSH probe 都必须存在；每项必须有 required fields、`current_allowed=false`、非空 `blocked_by`、可解析的 `safe_command_id`，并且 prompt/launch/ssh 入口必须正确标记自己的下游风险。

Forge daemon 只读状态入口：

```powershell
.\tools\smartsteam-forge\evolution-daemon.cmd -JsonStatus -WorkDir target\remote-gemma-unattended
.\tools\smartsteam-forge\evolution-daemon.cmd -Watch -Count 1 -IntervalSecs 1 -WorkDir target\remote-gemma-unattended
.\tools\smartsteam-forge\evolution-daemon.cmd -StartCheck -WorkDir target\remote-gemma-unattended -Backend 127.0.0.1:7979 -MaxTokens 64 -MaxTotalTokens 96 -MaxRuntimeSecs 0 -MaxFailures 1 -MaxNoFeedbackRounds 0 -TimeoutSecs 300
```

`StartCheck` 是 dry-run/preflight；`Start` 才是真启动入口，默认不要运行。

## 常驻运行还缺的证据

在把远程模型池交给长期 unattended 自进化前，还需要补齐这些证据：

- 重新生成一份当前时刻的 `status-with-model-cache.json`，确认快照不是过期状态。
- 确认 daemon status 的 `read_only=true`、`starts_process=false`、`sends_prompt=false`，并记录 `report_gate_continuation_state`。
- 记录当前是否已有 unattended/daemon 正在运行，避免重复常驻或并发发 prompt。
- 若要恢复 prompt-producing loop，先跑对应 gate，确认 `evolution_loop_prompt_round` 和 `model_pool_launch` 仍 allowed；只读快照本身不授权发送 prompt。
- 补一份持续运行窗口的资源证据：远程 Metal/GPU 状态、内存余量、worker 健康随时间稳定性、端口 8686-8690 的连续健康记录。

## 本窗口操作记录

本窗口只读取 target 快照、docs/tools 说明文件和本地只读诊断输出。未启动模型，未 SSH，未发送 prompt，未修改 `src`、`tools\smartsteam-forge`、`tools\evolution-loop` 或模型权重。

已新增/维护的本窗口 owned artifacts：

- `docs\runbooks\gemma-remote-unattended-status.md`
- `docs\architecture\integration-gemma-readiness.md`
- `tools\gemma-chain\scripts\read-remote-unattended-snapshot.ps1`
- `tools\gemma-chain\scripts\test-read-remote-unattended-snapshot.ps1`
- `tools\gemma-chain\scripts\read-remote-evidence-freshness.ps1`
- `tools\gemma-chain\scripts\test-read-remote-evidence-freshness.ps1`
- `tools\gemma-chain\scripts\read-remote-observation-window.ps1`
- `tools\gemma-chain\scripts\test-read-remote-observation-window.ps1`
- `tools\gemma-chain\scripts\read-remote-resource-window.ps1`
- `tools\gemma-chain\scripts\test-read-remote-resource-window.ps1`
- `tools\gemma-chain\scripts\test-remote-consumer-contract.ps1`
- `tools\gemma-chain\scripts\read-remote-readiness-contract.ps1`
- `tools\gemma-chain\scripts\test-read-remote-readiness-contract.ps1`
- `tools\gemma-chain\scripts\read-remote-residency-gap-report.ps1`
- `tools\gemma-chain\scripts\test-read-remote-residency-gap-report.ps1`
- `tools\gemma-chain\scripts\read-remote-evidence-package-plan.ps1`
- `tools\gemma-chain\scripts\test-read-remote-evidence-package-plan.ps1`
- `tools\gemma-chain\scripts\read-remote-evidence-package-status.ps1`
- `tools\gemma-chain\scripts\test-read-remote-evidence-package-status.ps1`
- `tools\gemma-chain\scripts\read-remote-owner-flow-handoff.ps1`
- `tools\gemma-chain\scripts\test-read-remote-owner-flow-handoff.ps1`
- `tools\gemma-chain\scripts\read-remote-consumer-preflight.ps1`
- `tools\gemma-chain\scripts\test-read-remote-consumer-preflight.ps1`
- `tools\gemma-chain\scripts\read-remote-surface-preflight.ps1`
- `tools\gemma-chain\scripts\test-read-remote-surface-preflight.ps1`
- `tools\gemma-chain\scripts\read-remote-link-boundary.ps1`
- `tools\gemma-chain\scripts\test-read-remote-link-boundary.ps1`
- `tools\gemma-chain\scripts\read-remote-dashboard-status.ps1`
- `tools\gemma-chain\scripts\test-read-remote-dashboard-status.ps1`
- `tools\gemma-chain\scripts\read-remote-action-matrix.ps1`
- `tools\gemma-chain\scripts\test-read-remote-action-matrix.ps1`
- `tools\gemma-chain\scripts\read-remote-evolution-loop-guard.ps1`
- `tools\gemma-chain\scripts\test-read-remote-evolution-loop-guard.ps1`
- `tools\gemma-chain\scripts\read-remote-model-pool-guard.ps1`
- `tools\gemma-chain\scripts\test-read-remote-model-pool-guard.ps1`
- `tools\gemma-chain\scripts\read-remote-contract-manifest.ps1`
- `tools\gemma-chain\scripts\test-read-remote-contract-manifest.ps1`
- `tools\gemma-chain\scripts\test-remote-readiness-quick-contract.ps1`
- `tools\gemma-chain\scripts\test-remote-readiness-readonly-contract.ps1`

最新只读验收已覆盖 `evidence_checklist[]` 每一项都具备 `id`、`gap_id`、`status`、`required_evidence`、`proof_source`、`safe_command_id`，并验证每个 `safe_command_id` 都能解析到 `safe_next_read_only_commands[]` 中的只读命令。自检还会拒绝 safe command 文本中出现 prompt/launch/SSH 入口，并确认 `StartCheck` 只作为 dry-run preflight 出现。
