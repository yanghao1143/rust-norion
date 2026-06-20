# Gemma Readiness Integration Contract

本文定义 Web Lab、SmartSteam Forge、Backend CLI、evolution-loop、model-pool 和远程主机探测入口如何消费 Gemma 远程链路的只读 readiness 证据。它是 fail-closed 集成合同，不是启动手册；默认不 SSH、不启动/停止模型、不发送 prompt、不写模型权重。

## 输入入口

下游集成只允许读取这些本地只读入口：

```powershell
.\tools\gemma-chain\scripts\read-remote-unattended-snapshot.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-evidence-freshness.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-observation-window.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-resource-window.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-readiness-contract.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-residency-gap-report.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-evidence-package-plan.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-evidence-package-status.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-owner-flow-handoff.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-surface-preflight.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-consumer-preflight.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-link-boundary.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-dashboard-status.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-action-matrix.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-evolution-loop-guard.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-model-pool-guard.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-contract-manifest.ps1 -Json
.\tools\gemma-chain\scripts\test-remote-consumer-contract.ps1 -Json
.\tools\gemma-chain\scripts\test-remote-readiness-quick-contract.ps1 -Json
.\tools\gemma-chain\scripts\test-remote-readiness-readonly-contract.ps1 -Json
```

这些入口的共同契约必须保持：

- `read_only=true`
- `starts_process=false`
- `sends_prompt=false`
- `touches_remote=false`
- `writes_files=false`
- resource reader 还必须有 `writes_model_weights=false`

任何消费者如果遇到脚本无法执行、JSON 无法解析、字段缺失、未知 contract version、或上述安全字段不满足，都必须把结果当作 blocked。

## Snapshot Contract

`read-remote-unattended-snapshot.ps1 -Json` 输出历史快照摘要。它可以证明落盘 evidence 在写入时的状态，例如 cache `5/5`、workers `6/6`、unattended `4/4`，但不能证明当前模型服务仍在常驻。

下游只能把这些字段当作证据，不得当作授权：

- `summary.evidence_fresh_all`
- `residency_decision.classification`
- `residency_decision.can_proceed_to_resident_loop`
- `residency_decision.read_only_evidence_collection_only`
- `authorization.*`
- `consumer_projection[]`
- `consumer_contract`
- `safe_next_read_only_commands[]`
- `evidence_checklist[]`

当前 snapshot reader 永远输出：

- `authorization.can_authorize_daemon=false`
- `authorization.can_authorize_launch=false`
- `authorization.can_authorize_prompt=false`
- `authorization.can_authorize_ssh=false`

如果未来有别的 gate 允许动作，也不能由 snapshot reader 本身直接授权；必须由外部即时 gate 和用户明确授权决定。

## Evidence Freshness

`read-remote-evidence-freshness.ps1 -Json` 是解释 `fresh_snapshot` 缺口的轻量 reader。它只读取 snapshot summary 和 evidence package status，不触远程、不启动模型、不发送 prompt。

该 reader 会输出：

- 每个 snapshot 文件的 `fresh`、`age_seconds`、`last_write_time_utc` 和 parse 状态。
- `fresh_file_count`、`stale_file_count`、`evidence_fresh_all`。
- unattended report 与 latest ledger 的轮次/成功状态对比，例如 latest ledger 比 report 更新或 latest ledger failed 时，必须展示 `requires_unattended_report_refresh=true`。

它的用途是帮助 Web/Forge/CLI/dashboard 解释“为什么历史 cache `5/5`、worker `6/6`、unattended `4/4` 仍然不能授权”。即使所有文件 fresh，该 reader 也不授权 daemon、launch、prompt 或 SSH。

为控制合同成本，quick/full readiness selftest 不把 freshness reader 加进主轮询链；脚本变更或交接时运行 `test-read-remote-evidence-freshness.ps1` 和 manifest 自检即可。

## Consumer Projection

`consumer_projection[]` 是所有下游入口的统一 fail-closed 视图。当前消费者列表固定为：

| id | surface | kind | 下游风险 |
| --- | --- | --- | --- |
| `web_lab_prompt` | Web Lab | `prompt` | 发送 prompt |
| `forge_cli_prompt` | SmartSteam Forge CLI | `prompt` | 发送 prompt |
| `backend_cli_direct_prompt` | Backend CLI | `prompt` | 发送 prompt |
| `evolution_loop_prompt_round` | Evolution Loop | `prompt` | 发送 prompt |
| `model_pool_launch` | Model Pool | `launch` | 启动/扩容模型 worker |
| `forge_daemon_residency` | SmartSteam Forge Daemon | `launch` | 启动常驻 daemon |
| `ssh_remote_probe` | Remote Host | `ssh` | 触碰远程主机 |

Web Lab/Forge/CLI 等具体入口可以读取窄口 preflight：

```powershell
.\tools\gemma-chain\scripts\read-remote-consumer-preflight.ps1 -ConsumerId web_lab_prompt -Json
.\tools\gemma-chain\scripts\read-remote-consumer-preflight.ps1 -ConsumerId forge_cli_prompt -Json
.\tools\gemma-chain\scripts\read-remote-consumer-preflight.ps1 -ConsumerId backend_cli_direct_prompt -Json
```

不传 `-ConsumerId` 时返回全部 consumer。该入口只投影 `consumer_projection[]`，并解析每项 `safe_command_id`；`current_allowed=false` 时 UI/CLI 只能展示 blocked reason 和对应只读补证据命令。未知 consumer id 必须按 blocked 处理。

`consumer preflight` 也支持 `-FailOnBlocked`。默认不带该开关时 blocked 仍 exit 0，方便 UI 读取 JSON；带 `-FailOnBlocked` 时，当前 blocked 返回 exit code `2`，未知 consumer id 返回 exit code `3`。这只是本地 gate 信号，不授权任何动作。

每个 consumer 必须具备 `consumer_contract.required_fields[]` 中的全部字段。集成方必须检查：

- `current_allowed` 必须显式为 `true` 才能继续进入自己的下一层 gate；缺失、非布尔、或 `false` 都是 blocked。
- Snapshot-only 输出中 `current_allowed=false` 是预期结果，不是错误。
- `blocked_by[]` 必须非空，供 UI/CLI 展示缺口。
- `safe_command_id` 必须能在 `safe_next_read_only_commands[]` 找到。
- `entrypoint_kind=prompt` 必须有 `downstream_sends_prompt=true`。
- `entrypoint_kind=launch` 必须有 `downstream_launches_process=true`。
- `entrypoint_kind=ssh` 必须有 `downstream_touches_remote=true`。

`test-remote-consumer-contract.ps1` 是该 contract 的只读验收入口。集成前应先确认：

```powershell
.\tools\gemma-chain\scripts\test-remote-consumer-contract.ps1 -Json
```

验收摘要中必须满足：

- `consumer_count=7`
- `consumer_allowed_count=0`，除非未来外部 gate 明确定义新 contract
- `invalid_safe_command_count=0`
- `authorization.*=false`

## Observation Window

`read-remote-observation-window.ps1 -Json` 只读取已经落盘的本地 observation sample，不执行诊断命令。它用于证明端口/worker 健康窗口是否存在。

每个 sample 目录应包含：

```text
chain-status.json
pool-status.json
status-bundle.json
forge-daemon-status.json
```

reader 会检查：

- sample 数量和时间跨度。
- 每个 sample 的解析状态。
- 每个 sample 的只读契约。
- chain 是否 ready/prompt_ready，或 model API、backend、Web Lab 是否健康。
- pool 是否至少 6 个 worker 且全部 healthy。

若输出：

- `summary.status=missing_window`：没有本地连续窗口 artifact。
- `summary.status=health_unknown`：sample 缺关键健康字段。
- `summary.status=health_not_ready`：至少一个 sample 不健康。
- `summary.continuous_window_present=true`：只表示端口/worker 窗口证据足够进入外部 gate 复核，不授权 prompt、launch、SSH 或 daemon。

## Resource Window

`read-remote-resource-window.ps1 -Json` 只读取已经落盘的资源余量 sample，不 SSH、不采集远程状态。

每个 sample 可使用这些文件名之一：

```text
remote-resource-status.json
resource-status.json
resource-headroom.json
```

每个 sample 必须声明：

- `read_only=true`
- `starts_process=false`
- `sends_prompt=false`
- `writes_model_weights=false`
- `approved_owner_flow=true`

并提供 memory/Metal 证据：

- `memory_available_gb` 或 `memory_available_bytes`
- `metal_available` 或 `gpu_available`

若 `summary.resource_window_present=true`，也只表示资源余量证据足够进入外部 gate 复核；它不授权 SSH、launch、daemon 或 prompt。

## Readiness Selftest

UI/CLI 的首选轻量读取入口是：

```powershell
.\tools\gemma-chain\scripts\read-remote-surface-preflight.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-surface-preflight.ps1 -ConsumerId web_lab_prompt -Json
```

surface preflight 只读取一次 readiness contract，适合 Web Lab/Forge/CLI 顶部状态条或频繁轮询。它返回 `display`、`status`、`consumers[]`、`missing_evidence_actions[]`、`pending_external_gate_actions[]` 和少量 `quick_commands[]`，并固定 `authorization.*=false`。`display` 是 UI/CLI 可直接渲染的状态投影，包含 `severity`、`headline`、`detail`、`primary_missing_evidence`、`next_action_label` 和 `badge`。

CLI/CI gate 可以显式加 `-FailOnBlocked`：

```powershell
.\tools\gemma-chain\scripts\read-remote-surface-preflight.ps1 -ConsumerId web_lab_prompt -FailOnBlocked
```

默认不带该开关时 blocked 仍 exit 0；带 `-FailOnBlocked` 时，当前 blocked 返回 exit code `2`，未知 consumer id 返回 exit code `3`。

完整 readiness 聚合入口是：

```powershell
.\tools\gemma-chain\scripts\read-remote-readiness-contract.ps1 -Json
```

它不运行 fixture 自检，只聚合 snapshot、observation window、resource window 和 consumer contract 的只读 JSON，输出稳定的 `generated_at_utc`、`summary`、`source_status`、`authorization`、`consumer_projection[]`、`evidence_checklist[]`、`missing_evidence_actions[]` 和 `safe_next_read_only_commands[]`。下游重复轮询应优先使用这个 reader；CI、交接或改脚本后再运行 selftest。

`generated_at_utc` 是本次本地读取的时间，不代表模型服务当前可用；`source_status.snapshot.evidence[]` 和 `summary.max_evidence_age_seconds` 才描述底层历史 evidence 的年龄。UI/CLI 必须同时展示这两层时间，避免把刚生成的 readiness contract 误读为 fresh 模型状态。

`missing_evidence_actions[]` 把缺失证据映射回具体 checklist 项、`safe_command_id` 和只读命令。UI/CLI 可以展示这些命令作为下一步人工证据采集参考，但不得自动执行其中任何命令；即使命令标记为 safe，也仍需要当前窗口/用户授权边界允许。

`pending_external_gate_actions[]` 描述证据包补齐后仍必须通过的外部 gate。`residency_external_gate` 会列出 daemon status、watch-once、dry-run StartCheck、chain status、pool status 和 status-bundle 等候选读法，用于交接，不是执行队列。

总控、Web Lab 或 Forge 如果只需要展示“当前为什么不能常驻”和“下一步还缺什么”，可以读投影报告：

```powershell
.\tools\gemma-chain\scripts\read-remote-residency-gap-report.ps1 -Json
```

该报告复用 readiness contract 与 snapshot reader，输出 `decision`、`snapshot_claims`、`freshness`、`checklist`、`consumers` 和 `safety`。`snapshot_claims.historical_only=true` 必须展示给人看，避免把 cache `5/5`、workers `6/6`、unattended `4/4` 误读成当前已授权常驻。`decision.authorized=false` 和 `authorization.*=false` 是固定 fail-closed 语义。

如果需要准备一组可交给外部 gate 的证据包规格，读取：

```powershell
.\tools\gemma-chain\scripts\read-remote-evidence-package-plan.ps1 -Json
```

该 plan 只描述 artifact 规格，不采集、不写文件、不 SSH。它固定声明 `plan.operator_boundary.this_script_collects_evidence=false`、`this_script_writes_artifacts=false`，并把 observation window 的 4 个 sample 文件、resource window 的 approved-owner-flow 字段、最少 sample 数和时间跨度写成机器可读结构。

若需要轮询“证据包是否已经达标到可交外部 gate”，读取：

```powershell
.\tools\gemma-chain\scripts\read-remote-evidence-package-status.ps1 -Json
```

该 status 会把 fresh snapshot、unattended report/ledger consistency、continuous observation window、resource headroom window 和 fail-closed contract 汇总为 `package_items[]`。当 latest ledger 新于 report、latest ledger failed、或 `requires_unattended_report_refresh=true` 时，`unattended_report_ledger_consistency.ready=false` 必须阻断证据包进入外部 gate。`summary.package_ready_for_external_gate=true` 只表示证据包可进入外部 gate 复核；它仍不授权 daemon、launch、SSH 或 prompt。

若需要把下一步交给获授权 owner-flow，读取：

```powershell
.\tools\gemma-chain\scripts\read-remote-owner-flow-handoff.ps1 -Json
```

该 handoff 把 `collect_fresh_snapshot_package`、`collect_observation_window_package`、`collect_resource_window_package` 和 `external_residency_gate_review` 写成 staged items。所有会写 artifact 或触碰远程的 item 都必须 `requires_explicit_user_authorization=true`；`read_only_verifiers[]` 仍只允许本地只读验证命令。

若 Web Lab/Forge/CLI 需要发现当前推荐 reader、自检入口、超时和退出码约定，读取：

```powershell
.\tools\gemma-chain\scripts\read-remote-contract-manifest.ps1 -Json
```

该 manifest 只读取 surface preflight 输出并列出只读合同入口。它固定声明 `blocked_exit_code=2`、`unknown_consumer_exit_code=3`，并要求所有 `readers[]` 和 `selftests[]` 都是 `read_only=true`、`starts_process=false`、`sends_prompt=false`、`touches_remote=false`、`writes_files=false`。manifest 不能授权 daemon、launch、SSH 或 prompt。

若需要展示远程链路拓扑、端口/worker 快照、Web Lab/backend 关系和证据边界，读取：

```powershell
.\tools\gemma-chain\scripts\read-remote-link-boundary.ps1 -Json
```

该 reader 只读取本地 snapshot 和 readiness contract。它会列出 `8686-8690` worker 端口、backend `7979`、Web Lab `8789`，并把外部同步的 `smartsteam-mac` 端口事实标记为 `observed_by_this_script=false`。UI/CLI 必须展示这个边界：本地历史快照可以说明 cache/worker/unattended 在快照写入时的状态，不能证明当前远程端口仍健康，也不能授权 prompt、launch、SSH 或 daemon。

若 Web Lab/Forge/CLI 需要一条可直接渲染的 dashboard/status panel 数据源，读取：

```powershell
.\tools\gemma-chain\scripts\read-remote-dashboard-status.ps1 -Json
```

该 dashboard 聚合 surface preflight、link boundary 和 evidence package status，输出 `display`、`dashboard_cards[]`、`package_items[]`、`topology`、`consumers[]` 和 `recommended_read_only_entrypoints[]`。它适合 UI/CLI 轮询展示，不采集 evidence、不写 artifact、不 SSH、不发送 prompt；`action_lock` 卡片和 `authorization.*=false` 必须优先展示，避免用户把状态面板误当作执行许可。

若需要给 Web Lab 按钮、Forge/CLI 子命令、evolution-loop prompt round、model-pool launch 或 daemon/SSH 入口生成统一的禁用矩阵，读取：

```powershell
.\tools\gemma-chain\scripts\read-remote-action-matrix.ps1 -Json
```

该 action matrix 聚合 consumer preflight 与 dashboard status，输出 7 个固定 `actions[]`。每项包含 `ui_enabled=false`、`cli_may_execute=false`、下游风险标记、blocked reason、tooltip 和只读 verifier 命令。verifier 命令只是人工补证据提示，不得自动执行；任何 UI/CLI 遇到 action matrix 缺字段、未知版本或 `authorization.*` 非 false，都必须按 blocked 处理。

若 evolution-loop 或 Forge daemon 只需要判断“能否发 prompt round / 能否进入常驻 daemon”，读取窄口 guard：

```powershell
.\tools\gemma-chain\scripts\read-remote-evolution-loop-guard.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-evolution-loop-guard.ps1 -FailOnBlocked -Json
```

该 guard 只读取 action matrix、owner-flow handoff 和 evidence package status，输出 `may_send_prompt_round=false`、`may_start_or_resume_daemon=false`、`may_enter_resident_loop=false`、`guarded_actions[]` 和 `guard_exit_code`。默认 blocked 仍 exit 0 方便 JSON 读取；带 `-FailOnBlocked` 时当前 blocked 返回 exit code `2`。它不启动 daemon、不调用 evolution-loop、不发 prompt。

若 model-pool 或 Forge/CLI 只需要判断“能否 launch/expand worker，能否把历史 `6/6` worker 快照当成当前容量”，读取：

```powershell
.\tools\gemma-chain\scripts\read-remote-model-pool-guard.ps1 -Json
.\tools\gemma-chain\scripts\read-remote-model-pool-guard.ps1 -FailOnBlocked -Json
```

该 guard 只读取 action matrix、link boundary 和 evidence package status，输出 `may_launch_worker=false`、`may_expand_pool=false`、`may_reuse_snapshot_as_current_capacity=false`、worker/cache snapshot 和 `guard_exit_code`。它不启动 worker、不扩容、不 SSH；历史 worker `6/6` 只能展示，不能授权当前 model-pool 动作。

集成方的默认 preflight 应优先使用统一入口：

```powershell
.\tools\gemma-chain\scripts\test-remote-readiness-quick-contract.ps1 -Json
```

quick contract 不运行 fixture 自检，只读取当前 readiness、gap report、evidence package status、owner-flow handoff、consumer preflight、surface preflight、link boundary、dashboard status、action matrix、evolution-loop guard、model-pool guard 和 contract manifest，验证它们都只读、fail-closed，并检查 missing evidence、package readiness、consumer/action blocked 状态、dashboard action lock、loop/daemon guard、model-pool guard、端口边界标记、manifest 退出码和 owner-flow item 对齐。它适合 Web Lab/Forge/CLI 的日常轮询或启动前轻量 preflight。

脚本改动、交接或 CI 深度验收再运行完整入口：

```powershell
.\tools\gemma-chain\scripts\test-remote-readiness-readonly-contract.ps1 -Json
```

该脚本会：

- 运行 snapshot、observation window、resource window、consumer contract 的只读自检。
- 运行 readiness contract 的完整 fixture 自检，覆盖 fresh evidence、连续端口窗口和资源窗口同时存在时的语义：可以支持外部 gate 复核，但仍不授权动作。
- 运行 residency gap report 自检，覆盖展示/交接用投影仍然 fail-closed。
- 运行 evidence package plan 自检，覆盖 observation/resource 采样包规格仍然只读、fail-closed。
- 运行 evidence package status 自检，覆盖证据包验收摘要仍然只读、fail-closed。
- 运行 owner-flow handoff 自检，覆盖交接项不会变成执行队列，写 artifact/触远程项必须需要显式授权。
- 运行 surface preflight 自检，覆盖 UI/CLI 轻量状态投影仍然 fail-closed。
- 运行 consumer preflight 自检，覆盖 Web Lab/Forge/CLI/model-pool/SSH consumer 窄口仍然 fail-closed。
- 运行 link boundary 自检，覆盖 worker/backend/Web Lab 拓扑、外部端口同步边界和 snapshot-only 授权语义。
- 运行 dashboard status 自检，覆盖状态卡、证据包、拓扑、推荐 reader 和 action lock 都保持 fail-closed。
- 运行 action matrix 自检，覆盖 Web/Forge/CLI/evolution-loop/model-pool/daemon/SSH 动作全部 disabled，verifier 只是只读提示。
- 运行 evolution-loop guard 自检，覆盖 prompt round、daemon residency 和 resident loop 都保持 blocked，`-FailOnBlocked` 退出码为 2。
- 运行 model-pool guard 自检，覆盖 worker launch/expand blocked，历史 worker 快照不能当成当前容量授权。
- 运行 contract manifest 自检，覆盖 reader/selftest 发现列表、退出码约定和 fail-closed 授权语义。
- 读取默认 reader JSON。
- 验证全部 reader fail-closed。
- 验证 `consumer_projection[]` 全 blocked。
- 验证 `evidence_checklist[]` 每项都有 `required_evidence`、`proof_source`、`safe_command_id` 和 `status`，且每个 `safe_command_id` 都能解析到只读 safe command。
- 验证 `safe_next_read_only_commands[]` 不启动、不 prompt、不 SSH、不写文件。
- 汇总 `missing_evidence[]`。

只有当 `summary.can_support_external_residency_review=true` 时，下游才可以把当前证据交给外部即时 gate 复核。即便如此，仍不能跳过用户授权、daemon duplicate-runner 检查、prompt/launch gate 和资源 gate。

## Current 2026-06-19 Runtime State

最新运行态证据见 `docs\runbooks\gemma-runtime-evidence-2026-06-19.md`。总窗口已验证远程 Mac 模型池 `6/6` healthy、全部 Metal，quality worker 为 `Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf`，model cache `5/5` ok，daemon PID `235920`，round `275` 已完成并刷新 report，daemon status 为 rounds `275`、`ledger_lag=0`、`stale=false`、`gate_failures=0`、`remote_runtime_acceleration_ok=true`。

下游展示或诊断时应按该 runbook 的 Artifact 清单解析：PID、report、stdout、daemon status、`status-with-model-cache.json`、`model-cache-status.json` 和 ledger 各自证明不同事实，不能互相替代。特别是 cache `5/5` 不能证明 Metal 正在用，worker `6/6` 不能证明 report 不 stale，PID 不能证明 daemon 仍存活，report `rounds` 不能证明 Mac 没睡。

MLX/4bit 候选模型的接入计划见 `docs\runbooks\gemma-mlx-experiment-slots.md`。这些模型需要独立 MLX runtime preflight，不属于当前 llama.cpp/GGUF readiness contract；Web Lab/Forge/CLI 不得把 MLX 实验槽当作 `8686-8690` GGUF worker 池的一部分。

本窗口只读复核的本地 evidence 包括：

- `target\remote-gemma-chain\status-with-model-cache.json`：链路 ready，backend/Web Lab/model API ready，worker `6/6` healthy，remote runtime `acceleration_ok=true`。
- `target\remote-gemma-chain\model-cache-status.json`：`all_ok=true`，5/5 model ok，0 remote error。
- `target\evolution\daemon\report.json`：rounds `275`，latest round `275` success，validation `134/134`，self-improve `274/274`，`report_gate.passed=true`，`continuation_gate_report_v1.allow_unattended_continuation=true`。
- `target\evolution\daemon\evolution-loop.pid`：PID `235920`。
- `target\evolution\daemon\evolution-loop.out.log`：round `275` 有 `report_refresh:start/done`，并记录 `remote_runtime_acceleration_ok:true`。

这些证据说明 6 月 19 日运行态已经不同于 6 月 16 日 stale/report-ledger mismatch 状态。下游仍必须保留 fail-closed 边界：本文档和 reader 只能证明证据状态，不能授权本窗口执行 prompt、launch、SSH 或 daemon start/stop。

## Historical 2026-06-15/16 Blocked State

当前只读合同自检显示：

- `snapshot_classification=blocked_stale_evidence`
- `evidence_fresh_all=false`
- `observation_window_status=missing_window`
- `resource_window_status=missing_resource_window`
- `consumer_allowed_count=0`
- `consumer_contract_validated=true`
- `unsafe_safe_command_count=0`
- `can_support_external_residency_review=false`
- `missing_evidence=fresh_snapshot,continuous_port_worker_window,remote_resource_headroom_window`
- `pending_external_gates=residency_external_gate`

因此 Web Lab、Forge、Backend CLI、evolution-loop、model-pool 和 SSH probe 当前都必须保持 blocked。

主窗口另行同步的实时状态是：`smartsteam-mac` 在线，远程 `8686/8687/8688/8689/8690` 有 `llama-server` 监听。该实时端口事实可以作为外部证据展示，但不能覆盖本合同的 fail-closed 读法；本合同仍要求 fresh snapshot、continuous observation window、resource window 和外部 residency gate。Web Lab 通过 `8789` 暴露实验入口，backend 通过 `7979` 连接模型链路，`8686-8690` 是远程 worker/model API 端口。

## Integration Rule

下游集成必须采用这个顺序：

1. 运行 `test-remote-readiness-readonly-contract.ps1 -Json`。
2. 若脚本失败、JSON 缺字段、或 `authorization.*` 任一不为 `false`，按 blocked 处理并提示人工复查合同。
3. 若 `consumer_contract_validated=false` 或 `consumer_allowed_count>0`，按 blocked 处理，除非已迁移到明确的新 contract version。
4. 若 `can_support_external_residency_review=false`，只展示 `missing_evidence[]`、`missing_evidence_actions[]`、`pending_external_gates[]` 和对应 `safe_command_id`，不得执行 prompt、launch、SSH 或 daemon。
5. 若 `can_support_external_residency_review=true`，仍只允许进入 `pending_external_gates[]` 所描述的外部即时 gate；不得把本合同输出直接当作执行许可。

这样 Web Lab/Forge/CLI/evolution-loop 可以共享同一份只读证据模型，同时避免历史快照被误用为当前常驻授权。
