# SmartSteam CLI/UI status paths

本 runbook 用来区分三条交互入口的启动路径和只读状态检查路径：
`norion-cli`、`rustgpt-lab` Web Lab、SmartSteam Forge TUI。

默认原则：

- 只读状态检查不能启动 Gemma、不能提交 prompt、不能写经验库。
- 启动 Web Lab 或 Forge 时连接已有 `rust-norion` 后端，除非明确选择
  built-in 或 Gemma lab 启动脚本。
- `engine_busy`、safe-device、readiness、经验库 hygiene/index gate 是发送
  prompt 前的门禁；状态检查要显示这些字段，但不能绕过它们。
- 未显式选择 worker 时保持 `endpoint=auto pinned=false`，由后端 scheduler
  选择模型；只有 `/endpoint`、`/worker` 或 CLI `--endpoint/--worker` 才固定
  worker。

门禁展示优先级：`backend_online=false`、safe-device 失败、experience
hygiene/index 失败属于 repair gate，应优先显示 `repair_gate failed: ...`；它们
不能被 `engine_busy`、worker busy、queued 或 backpressure 改写成
`wait_for_current_stream` / `retry_later`。UI 可以继续展示 worker pool 和
route_pool 状态作为诊断信息，但发送按钮必须以 repair gate 为最终禁用原因，并保留
prompt 草稿。

不会启动模型的快速清单：

| 入口 | 只读/不启动模型 | 允许真实发送 prompt 的路径 |
| --- | --- | --- |
| `norion-cli` | 直接 `cargo run --manifest-path crates\norion-cli\Cargo.toml`、`cli_smoke`、嵌入 host 的 `/status` / `/workers` | 仅宿主收到 `InputAction::StartStream` 后显式接到后端 |
| evolution daemon/report | 已有 daemon 的 PID/round/report/ledger 状态、report refresh 结果 | 不发送 prompt；不从这里启动/停止模型或 daemon |
| remote model pool | 后端或运维侧模型池健康汇总、worker healthy/Metal/quality 标签 | 不发送 prompt；只作为 Web Lab/Forge/CLI gate 诊断输入 |
| Web Lab | `status-*-lab.cmd`、`start-gemma-lab.cmd -CheckOnly`、`/api/backend-health`、`/api/model-pool-status`、`/api/model-pool-advice` | 聊天输入发送按钮或等价 `/api/chat-stream` 请求 |
| Forge TUI | `/status`、`/workers`、`/context-window`、`/max-tokens`、`/hygiene dry-run`、`/repair dry-run`、`/audit` | 对话输入/发送动作 |

当前 R4 运行窗口如果已经由总窗口证明 `rust-norion` 后端和 Web Lab 在替代端口
运行，例如 backend `127.0.0.1:7979`、Web Lab `127.0.0.1:8789`，本窗口仍只做
只读确认，不接管进程生命周期。可读的证据包括后端 `/health`、Web Lab
`/health` 或 `/api/backend-health`、模型池 status/advice，以及 CLI/Forge 本地
`/status`、`/workers`、`/context-window` 投影；不可做的动作包括启动/停止远程
worker、SSH、重启 daemon、调用 `/api/chat-stream` 试探活性，或把 `6/6 healthy`
直接解释成 prompt 授权。真正发送前仍要让对应入口消费自己的
`engine_busy`、safe-device、experience hygiene/index、route_pool 和上下文/token
门禁。

如果一次操作既会改变 session history、写入 user turn、打开 SSE/stream，或消耗
`max_tokens` 生成预算，就不是只读状态检查；必须先通过对应入口的 gate/advice。

自动化脚本和 UI host 的快速判别规则：

- 命令名或参数含 `status`、`health`、`CheckOnly`、`DryRun`、`dry-run` 或
  `-Help` 时，预期只能读取状态或打印覆盖清单；若它会打开 chat/stream、记录
  user turn、启动 Gemma 或写经验库，就是 contract 破坏。
- `start-built-in-lab.cmd` 只启动 built-in 假后端/UI 链路，不启动 Gemma；它可用于
  浏览器和 SSE 代理交互检查，但不能作为真实模型健康证据。
- `start-gemma-lab.cmd -CheckOnly` 是启动前只读检查；去掉 `-CheckOnly` 后才可能进入
  真实 Gemma 链路，必须由总窗口明确允许。
- `InputAction::Status`、`InputAction::RoutingChanged`、
  `InputAction::SessionConfigChanged` 和 workers/status snapshot 是本地 UI 状态动作；
  `InputAction::StartStream`、Web Lab `/api/chat-stream` 和 Forge 对话发送动作才是
  prompt 发送边界。

只读探测只走本地状态命令或 HTTP `GET` health/status API；不要用 chat/stream
请求试探端口是否可用：

| 目标 | 只读探测 | 不属于只读探测 |
| --- | --- | --- |
| CLI shell | `/status`、`/workers`、`/model`、`/endpoint`、`/max-tokens` | 宿主把 `InputAction::StartStream` 接到后端 |
| evolution daemon/report | 读取已运行 daemon 的 PID、round、report_refresh、ledger_lag、stale 状态 | 启动/停止 daemon、触发 evolution loop、发送 prompt |
| remote model pool | 读取 worker healthy 数量、quality/fast/summary endpoint、Metal/runtime 标签 | SSH、启动/停止远端 runtime、用 prompt 试探 worker |
| 后端 `127.0.0.1:7878` | `GET /health`、模型池/status/advice API | 后端 chat/generate stream API |
| runtime `127.0.0.1:8686` | runtime health/metadata，由后端或运维脚本读取 | Web Lab/Forge/CLI 直接向 runtime 发送 prompt |
| Web Lab proxy `127.0.0.1:8787/8789` | `/health`、`/api/backend-health`、`/api/model-pool-*` | `/api/chat-stream` 或浏览器发送按钮 |
| Forge TUI | `/status`、`/workers`、`/context-window`、dry-run/audit 命令 | 对话输入/发送动作 |

## daemon 和远程模型池

evolution daemon/report 状态是系统运行态证据，不是聊天入口。读取已有 PID、
round、`report_refresh`、`ledger_lag`、`stale` 或 report 文件属于只读检查；不要为了
确认 UI 是否能对话而启动/停止 daemon、触发新 evolution round，或让 daemon 发送
prompt。

远程 model pool 状态也是 gate 诊断输入，不是 prompt 通道。`6/6 healthy`、
quality endpoint、Metal/runtime 标签只能说明后端可用于调度；Web Lab、Forge 或
CLI host 仍必须读取自己的 backend/model-pool health、`engine_busy`、
safe-device/readiness 和 experience hygiene/index gate，再由明确的聊天发送动作进入
stream。远程池状态异常时，优先显示 wait/retry/repair advice，并继续允许只读状态命令。

service/CLI 现在也有一个更粗粒度的 SmartSteam status host snapshot，用于把
daemon/supervisor/remote model pool/readiness/busy/active round 聚合给 UI、Web Lab
或 Forge。这个 DTO 只能读取已有状态：它标记 `read_only=true`，并显式锁住
`starts_daemon=false`、`stops_daemon=false`、`touches_remote=false`、
`downloads_model=false`、`warms_model_cache=false`、`sends_prompt=false`、
`starts_stream=false`、`replays_prompt=false`、`mutates_busy=false`、
`mutates_readiness=false` 和 `mutates_active_round=false`。CLI host wrapper 只额外附带
`history_messages` / `partial_chars`，用于证明状态读取没有记录 user turn、没有回放
prompt，也没有改变 partial stream。

### model-pool readiness snapshot bundle

Web Lab、Forge 和 CLI 消费 model-pool readiness 时，应把它看成只读 bundle，而不是
隐式发送入口。当前 service/CLI host 可稳定投影的字段包括 quality/helper lane 的
worker identity、role/preference、route worker picker、`engine_busy`、
safe-device/readiness gate、`worker_count` / `pool_status` / `route_pool_status`
以及 busy/backpressure/repair advice。`model_cache` 目前不是 service/CLI host DTO 的
独立字段；如果 Web Lab 或 Forge 从外部 backend/provider status 读取 model cache 状态，
只能把它作为 coarse diagnostic label 展示，不能用它启动模型、预热 cache、改写
`readiness_ok`、清除 `engine_busy`、重放 prompt，或跳过 safe-device / route gate。

因此 consumer-facing readiness bundle 的判定顺序固定为：

| 维度 | 当前只读来源 | 使用规则 |
| --- | --- | --- |
| quality/helper ports | `workers` / `route_workers` 的 endpoint、role、preference、picker 字段 | 展示候选 lane 和 helper advice，不直接向端口发送 prompt |
| readiness / safe-device | backend health、frontend gate、repair/preflight gate | readiness 或 safe-device 失败优先于 worker ready 文案 |
| `engine_busy` | frontend gate / host status snapshot | 只阻断发送，不阻断继续读取 status |
| worker count / capacity | `pool_status`、`route_pool_status`、worker rows | 诊断容量；发送仍以 prompt gate 和 route gate 为准 |
| `model_cache` | 外部 backend/provider status 的 coarse cache label | 只展示，不触发下载、预热、启动 runtime 或修改 readiness/busy |

### 8686-8690 runtime worker DTO 口径

当后端或运维脚本把 `127.0.0.1:8686` 到 `127.0.0.1:8690` 投影成
service/CLI host DTO 时，这些端口只能作为 worker identity/status 数据被 UI、Web Lab
和 Forge 消费，不能被状态面板直接当成 prompt 入口。`endpoint_label` 可以显示
`127.0.0.1:8686` 这类地址；Web Lab/Forge 仍只能通过自己的发送按钮或 chat API
进入真实 stream，不能因为某个端口 row 为 ready 就直接调用 runtime。

端口池 DTO 的字段语义固定如下：

| 字段 | 含义 | UI/Forge/Web Lab 使用规则 |
| --- | --- | --- |
| `worker_status_label` / `worker_status_state_label` | worker 自身健康：`available`、`busy`、`backpressure` | 用来展示端口健康和等待原因，不等价于发送授权 |
| `worker_status_is_available` | 该 worker 当前没有 busy/queue saturation | 可以渲染 ready 标识；仍要结合 route gate 和 `send_allowed` |
| `worker_status_is_pressure` / `worker_status_blocks_prompt_submit` | busy、queued、backpressure 这类压力态 | 禁用当前 row 的直接选择/发送，保留输入草稿 |
| `route_match` / `selectable` / `picker_action_label` | 当前 role/preference/endpoint lane 是否能用该 worker | `unavailable` 表示对当前 route 不可用，可能是 role/preference mismatch，不必推断端口宕机 |
| `decision_action_label` / `decision_state_label` / `decision_reason` | 当前 row 的解释：send、wait、retry、repair | 只能解释 gate，不得触发 `StartStream`、重放输入或模型调用 |
| `selection_wire_*` | 如果用户之后显式 pin 该 worker，请求 wire 会携带哪些 route 字段 | `selection_wire_sends_model_endpoint=true` 只是 picker 预览数据，不是 status 读取会发送 endpoint |
| `pool_status` / `route_pool_status` | 全池与当前 route lane 聚合容量 | 诊断用；发送按钮仍以 `prompt_submit_control`、`send_allowed`、`route_send_block_*` 为准 |

示例端口池语义：

| 端口 row | DTO 表达 | 宿主动作 |
| --- | --- | --- |
| `127.0.0.1:8686` | `worker_status_label=available`、`route_match=true`、`selectable=true`、`picker_action_label=select` | 可显示 ready/selectable，但 status 读取不发送 prompt |
| `127.0.0.1:8687` | `worker_status_label=busy`、`picker_action_label=wait`、`decision_action_label=wait_for_current_stream` | 显示 busy/active request，禁用该 row 的发送 |
| `127.0.0.1:8688` | worker 自身可 `available`，但 `route_match=false`、`picker_action_label=unavailable` | 显示当前 route 不可用，不把它误判为端口故障或可发送 |
| `127.0.0.1:8689` | `worker_status_label=backpressure`、`decision_action_label=retry_later` | 显示 queue/backpressure，保留草稿 |
| `127.0.0.1:8690` | `available` 且 route selectable | 可作为 ready worker 展示；仍不能从 status 面板直接调用 runtime |

当前一手测试证据是 `crates/norion-service/src/gate.rs` 的
`workers_host_snapshot_marks_8686_8690_readiness_without_side_effects`、
`workers_host_snapshot_json_field_names_are_stable_for_8686_8690` 和
`crates/norion-cli/src/status.rs` 的
`workers_host_snapshot_projects_8686_8690_readiness_for_web_lab_and_forge`、
`workers_host_snapshot_json_field_names_are_stable_for_8686_8690`。这些测试都只构造
内存 DTO，不访问端口、不启动 daemon/model、不走 chat-stream；field-name snapshot
测试用穷尽 DTO 解构绑定锁住 JSON-facing 字段集合，防止 Web Lab/Forge 后续消费时字段漂移，并断言
`read_only=true`、`launches_process=false`、`sends_prompt=false`、`starts_stream=false`
和无 request preview / stream chunk / input action side effect。

## norion-cli

`norion-cli` 是协议、输入和输出边界的 no-backend shell。直接运行它不会启动
Gemma，也不会连接后端发 prompt；它只打印启动快照、路由、上下文历史上限、
生成 token 预算和本地命令提示。

启动/协议检查：

```powershell
cd D:\rust-norion
cargo run --manifest-path crates\norion-cli\Cargo.toml
cargo run --manifest-path crates\norion-cli\Cargo.toml -- --role reviewer --prefer fast
cargo run --manifest-path crates\norion-cli\Cargo.toml -- --role reviewer --prefer fast --worker mlx-reviewer-8b --max-tokens 8192
```

只读测试路径：

```powershell
cargo test -q --manifest-path crates\norion-cli\Cargo.toml --test cli_smoke
```

在嵌入式终端或 TUI host 里，状态只读命令是 `/status` 和 `/workers`；`/state`
等价 `/status`，`/worker-status` 和 `/endpoints` 等价 `/workers`。它们应读取
`CliStatusSnapshot`、`InputReadinessSnapshot`、`InputControlSnapshot` 或
`OutputUpdate.status_snapshot`，展示 `send_allowed`、`send_block_state_label`、
`send_block_reason`、route labels 和 `wire_*` 字段；如果宿主走 viewport/output
envelope，优先消费结构化 `status_snapshot` / `route_workers` / `route_pool_*`，
不要解析 `appended` 文本，更不要通过发送 prompt 来探测 busy/backpressure。
这些别名在 model-pool gate 下也必须保持同一契约：`engine_busy`、`backend is offline`
或 route lane `backpressure` 时，`/status` 与 `/state` 仍返回同一组结构化 status
字段，`/workers`、`/worker-status` 与 `/endpoints` 仍返回同一组 worker/route
worker rows；它们都只能落成本地 `InputAction::Status` / host snapshot，不得因为别名
不同而启动 stream、写入 history，或退回到解析文本行的兼容分支。
已验证的 output/view 契约还锁住了 busy、backpressure 和 repair gate 下的
`/status`、`/workers` host snapshot：它们保持 `source=local_status`，不携带
`request_preview`、`stream_chunk` 或 `input_action_snapshot`，不会污染
`history_messages` / `partial_chars`，同时仍保留 worker role/preference、health
和 route worker picker 的结构化状态。
如果 Web Lab 或 Forge 需要一个不依赖终端文本的只读 DTO，CLI 层已经提供
`CliStatusSnapshot::workers_host_snapshot()`。它把当前 route、send/route block reason、
pool/route pool 摘要、worker role/preference/health、picker action、decision display
和 pinned selection wire 字段投影到 `CliWorkersHostSnapshot` /
`CliWorkerHostSnapshot`，并显式标记 `read_only=true`、`launches_process=false`、
`sends_prompt=false`、`starts_stream=false`、不携带 request preview / stream chunk /
input action snapshot。
Service 层的 `ModelPoolRouteSnapshot::workers_host_snapshot()` 是同一类只读 host
DTO：它和 CLI DTO 都只投影 route、worker、picker、selection wire 与阻断原因字段，
不会发送 prompt、不会进入 `StartStream`，也不会携带 request preview、可回放 history
payload、stream chunk 或 input action。CLI DTO 里的 `history_messages` /
`partial_chars` 只是宿主状态计数，用来证明 `/workers` 没有污染会话历史或 partial
输出；service DTO 则通过 `mutates_history=false` 显式锁住同一边界。

SmartSteam status snapshot 还单独暴露协作 worker window 的 clean-room 状态，用来把
“daemon/supervisor/remote pool 正在运行”和“某个工作窗口已暂停或污染，需要替换”
分开显示。`worker_windows[]` 行只描述 Codex/协作窗口，不是模型 runtime worker：
`status_label=running|paused|polluted`、`paused`、`polluted`、
`archived`、`completed_evidence_only`、`clean_room_replacement`、
`assignment_allowed`、`original_window_blocks_assignment`、
`clean_room_replacement_required`、`future_work_requires_fresh_clean_room`、
`replacement_window_id` 和 `reason` 用于 UI/CLI/Forge 渲染 clean-room replacement 提示。
已完成 worker 只能作为完成证据，不可继续分配；`archived=true`、`polluted=true`、
`completed_evidence_only=true` 或 `original_window_blocks_assignment=true` 的旧窗口必须显示
为 not assignable。后续业务只能分配到新的 clean-room window，或分配到显式标记
`clean_room_replacement=true` 且 `assignment_allowed=true` 的新窗口；聚合字段 `worker_window_status`、
`worker_windows_total`、`worker_windows_paused`、`worker_windows_polluted` 和
`worker_windows_clean_room_replacements_required` 用于顶部摘要。这个状态面仍是只读：
`starts_clean_room_replacement=false`、`mutates_worker_window_status=false`，并继续保持
`starts_daemon=false`、`stops_daemon=false`、`touches_remote=false`、`sends_prompt=false`、
`starts_stream=false`、`replays_prompt=false`、`mutates_busy=false`、
`mutates_readiness=false` 和 `mutates_active_round=false`。看到
`clean_room_replacement_required=true` 只能提示主窗口或调度层重新分配 clean-room
窗口，不能由状态读取路径创建线程、归档旧窗口、重放 prompt、启动 daemon 或触碰远端。
Forge 的 `worker_window_status` / `worker_window_replacement_report` parser 会保留同一组
字段，并提供 `tools/smartsteam-forge/fixtures/r30-clean-room-status.example.json` 作为
R30 clean-room status fixture；该 fixture 只作为状态消费样例，不代表可写入或可启动流程。
R31 起，service/CLI/Forge 状态面也可以携带
`daemon_round_transition_status`：当 stdout 已观察到 `round N done [DONE]`，但
ledger/report 最新提交仍停在上一轮时，UI 应显示
`status=round-done-ledger-commit-pending`、`done_round` / `latest_done_round`、
`round_in_progress=false`、`ledger_round`、`ledger_commit_pending=true` 和
`ledger_lag_rounds`。普通活跃轮次可显示 top-level `latest_done_round` 与
`round_in_progress=true`，例如 active round 已进入下一轮但 ledger/report 最新 done 轮仍停在
上一轮。这些都只是“等待 ledger commit/report
catch-up”的读态提示，不能触发 daemon start/stop、远端操作、prompt、stream、active round
mutation 或 `.ndkv` 写入；对应字段必须保持 `starts_daemon=false`、`stops_daemon=false`、
`touches_remote=false`、`sends_prompt=false`、`starts_stream=false`、
`replays_prompt=false`、`mutates_active_round=false` 和 `writes_ndkv=false`。
service/CLI status 同步投影 `context_hygiene_status`：
`completed_window_evidence_non_actionable=true` 只说明已完成窗口可作为 evidence 展示，
后续工作仍需要 fresh clean-room window，不能把 completed window 重新分配或读取旧窗口
payload；`reads_old_window_payload=false` 必须保持为只读 hygiene 证据。
后端离线、`engine_busy` 或 backpressure 只阻断真实 prompt；`/status`、`/state`、
`/workers`、`/worker-status`、`/endpoints`、`/model`、`/endpoint` 和 `/max-tokens`
仍按本地命令处理。若这些本地命令本身无效，
UI 应显示 `input_error` / `fix_command`，不要把它们折叠成 `repair_gate` 或
`backend is offline`。

SmartSteam status snapshot 也可以携带 R22 memory startup admission 的只读投影。
service `SmartSteamStatusSource::with_memory_startup_admission()` 只接收
`MemoryStartupAdmissionEvidence` 的 typed evidence，并投影为
`memory_startup_admission_status` / `memory_startup_admission_summary`。这些字段显示
startup admission、index quality、index operation/refresh、context rot blocker、
helper prose 和 non-contract line 计数；它们不会重新解析 helper prose、旧窗口文本、
`write_mode=live_write` 字符串或 `.ndkv` 建议文本。UI/CLI 只能把
`helper_prose_line_count`、`non_contract_line_count`、
`admission_expanded_by_non_contract_evidence=false`、`ndkv_write_allowed=false` 和
`live_store_mutation_requested=false` 当作状态显示，不能据此扩张 admission、写 live
store、创建真实 `.ndkv` 文件或重放 prompt。

R25 service/CLI status 还可以把 clean-room handoff/admission 作为同一份 typed
snapshot 暴露给 UI、Web Lab 或 Forge。service 侧只接收
`SmartSteamCleanRoomHandoffStatusSource` 和已投影的
`MemoryStartupAdmissionEvidence`，输出 `clean_room_handoff_status` /
`clean_room_handoff_summary`；CLI host 只复制这些结构化字段。该状态面固定表达：
`memory_admission_safe=true`、`agent_replacement_plan_required=true`、
`replacement_prompt_ready=true`、`original_window_follow_up_blocked=true`、
`reads_old_window_payload=false`、`live_write_allowed=false`、
`live_store_mutation_allowed=false`、`ndkv_write_allowed=false` 和
`runtime_side_effects_allowed=false`。消费方不能解析 helper prose、旧窗口 payload 或
handoff 文案来生成这些字段，也不能因为看到 replacement plan ready 就创建线程、
发送消息、归档旧窗口、启动/停止 daemon、触碰远端、发送 prompt、开启 stream、写 live
store 或写 `.ndkv`；它只能作为 clean-room handoff/admission 状态证据展示。

R26 service/CLI status 进一步暴露 self-improve proposal lifecycle 的只读 typed
snapshot。service 侧只接收 `SmartSteamSelfImproveProposalStatusSource` 行，输出
`self_improve_proposal_status` / `self_improve_proposal_summary`；CLI host 只复制这些
结构化字段。每条 proposal 明确给出 `lifecycle_label`：
`candidate`、`validated`、`admitted`、`quarantined`、`promoted` 或
`repair-required`，并携带 `source_round`、`evidence_ids`、typed
`validation_status`、typed `memory_admission_status` 和 side-effect flags。消费方不能从
helper prose、summary 文案或旧窗口 payload 反推 lifecycle，也不能 replay prompt、调用
模型、启动 stream、写 memory、写 `.ndkv`、修改 live store、promote runtime 或 quarantine
runtime；这些 status 字段只能用于展示 proposal 处在候选、验证、admission、隔离、提升或
需要修复的哪一步。

R28 service/CLI status 还单独暴露 helper-stage repair-required 的只读 typed
snapshot。service 侧只接收 `SmartSteamHelperStageRepairStatusSource`，输出
`helper_stage_repair_status` / `helper_stage_repair_summary`；CLI host 只复制这些
结构化字段。该状态面明确给出 `stage_label`、`state_label=complete|repair-required`、
`helper_stage_contract_complete`、`helper_stage_repair_required`、`source_round`、
`evidence_ids` 和 `reason_codes`，并固定 `read_only=true`、`report_only=true`、
`pure_data_only=true`。R29 起同一状态面还会显式展示完全缺失的 helper role 修复项：
`missing_helper_role_repair_required`、
`missing_helper_role_repair_proposal_count`、`missing_helper_roles` 和 typed
`missing_helper_role_repair_proposals`，避免把缺失 helper role 混入普通 incomplete
field proposal。消费方不能从 helper prose、summary 文案或旧窗口 payload 推断
helper stage 是否完成，也不能 replay prompt、调用模型、启动 stream、写 memory、写
`.ndkv`、修改 live store、创建 replacement window 或修改 worker-window status；这些
status 字段只能用于展示 helper-stage contract 是否需要 repair-first 处理。

区分全池和当前 route lane：`pool_*` 字段说明后端整体 worker 池，`route_pool_*`
字段说明当前 role/preference/endpoint 会命中的 worker 子集。全池可以同时显示
`available=true` 和 `capacity_state=queued`，但如果 `route_pool_capacity_state` 是
`busy` 或 `backpressure`，当前 prompt 仍必须禁用 Enter/send，保留输入草稿，并显示
`wait_for_current_stream` 或 `retry_later`。UI 不要只看全池还有可用 worker 就允许
auto-route prompt；应以 `prompt_submit_control`、`send_block_reason` 和
`route_send_block_*` 为发送按钮的最终状态来源。

如果宿主已经直接拿到了 service/CLI 的结构化对象，也要沿用同一套“layered
originals”优先级，而不是把所有来源摊平成一个文本状态：

- 发送按钮/Enter：先看 `prompt_submit_control`，再看 `route_send_block_*` /
  `send_block_reason`。
- 当前 route 的主等待/阻断文案：优先用 route 级 `send_block_chunk` 或
  `route_send_block_chunk`。
- 当前 route 的 worker picker：优先用 `route_workers[*]` 的
  `picker_action_label`、`decision_display_snapshot()` 和
  `worker_status_display_snapshot()`。
- 全池健康概览：只在 route 级信息不足时再回退到 `workers[*]`、`pool_*`、
  `route_pool_*` 和紧凑文本摘要。

## Web Lab

Web Lab 是浏览器和流式代理路径。它可以连接已有后端，也可以通过 lab 脚本启动
built-in 或真实 Gemma 测试链路。

连接已有后端，不启动 Gemma：

```powershell
cd D:\rust-norion
cargo run --manifest-path tools\rustgpt-lab\Cargo.toml -- --backend 127.0.0.1:7878 --bind 127.0.0.1:8787
```

built-in 安全启动路径，不启动 Gemma：

```powershell
.\tools\rustgpt-lab\start-built-in-lab.cmd
```

真实 Gemma 启动前必须先做只读 CheckOnly：

```powershell
.\tools\rustgpt-lab\start-gemma-lab.cmd -CheckOnly -StateDir target\manual-gemma-service\lab-state
```

只读状态检查：

```powershell
.\tools\rustgpt-lab\status-built-in-lab.cmd
.\tools\rustgpt-lab\status-gemma-lab.cmd
Invoke-RestMethod http://127.0.0.1:8787/api/backend-health
Invoke-RestMethod http://127.0.0.1:8787/api/model-pool-status
Invoke-RestMethod http://127.0.0.1:8787/api/model-pool-advice
```

离线安全证据入口：

```powershell
.\tools\rustgpt-lab\test-gemma-lab-safety.cmd -Help
.\tools\rustgpt-lab\test-gemma-lab-safety.cmd
```

`-Help` 只打印覆盖清单；完整 safety 脚本使用假 Web Lab、随机空端口和
CheckOnly/DryRun 路径验证 CLI/Web 断流、取消、草稿恢复、preflight gate 和
payload 字段，并验证 `repl-gemma-lab.cmd -SkipStart` 在后端缺失时保持
attach-only、不进入 Gemma/start/REPL 路径；全程不启动 Gemma、不 SSH、不发送
真实推理请求。它也锁住 Gemma 和 built-in wrapper help 里的 `7878` 后端、
`8787` Web Lab 和 `8686` runtime 端口图，避免把 runtime 误当成 prompt 入口。

端口口径：

- `127.0.0.1:7878` 是 `rust-norion` 主模型服务后端；Web Lab 的
  `--backend` 指向这里。7878 拒绝连接时，先查后端是否启动，不要直接重启
  Gemma。
- `127.0.0.1:8686` 是可选的 Gemma/mistralrs runtime，由 `rust-norion`
  在后端调用；Web Lab 不直接向 8686 发送 prompt。
- `127.0.0.1:8787` 是 `rustgpt-lab` 浏览器 UI 和本地 SSE 代理；Web Lab
  的 `--bind` 控制这个监听地址。
- `127.0.0.1:8789` 是部分 Gemma chain/runbook 使用的 Web Lab/SSE 代理替代
  端口。它和 8787 同类，都是浏览器 UI/代理入口，不是 Gemma runtime，也不是
  `rust-norion` 后端。先读 `/health` 或 `/api/backend-health` 确认它实际指向的
  后端端口，再决定是否允许发送 prompt。

端口只读检查边界：

| 地址 | 角色 | 只读检查 | 真实 prompt 边界 |
| --- | --- | --- | --- |
| `127.0.0.1:7878` | `rust-norion` 后端 | `GET /health`、模型池/status API | 后端 chat/generate stream API，由 UI/CLI host 显式调用 |
| `127.0.0.1:8686` | Gemma/mistralrs runtime | 运行时 health/metadata，只由后端或运维脚本探测 | 不从 Web Lab/Forge/CLI 直接发送 prompt |
| `127.0.0.1:8787` | Web Lab UI/SSE 代理 | `/health`、`/api/backend-health`、`/api/model-pool-*` | `/api/chat-stream` 或浏览器发送按钮 |
| `127.0.0.1:8789` | Web Lab UI/SSE 代理替代端口 | 同 8787，先确认 `/health` 指向哪个后端 | 同 8787；不要把端口在线当作 prompt 授权 |

这些状态接口只能读取 `/health`、模型池状态和建议。Web Lab 发送按钮应在
`engine_busy=true`、Gemma runtime 不可达、safe-device/readiness 失败或
experience hygiene/index gate 失败时保持禁用，并保留草稿和浏览器临时上下文。
真正发送 prompt 只应来自聊天输入框的发送按钮或等价的 chat API；`status-*-lab.cmd`、
`/api/backend-health`、`/api/model-pool-status` 和 `/api/model-pool-advice`
都不能用来探测性发送 prompt。

Web Lab 的前端判断顺序也应保持结构化优先：若模型池状态接口已返回 route lane
字段，就先消费 `route_send_block_*`、`route_pool_*` 和 route worker 诊断，再决定
发送按钮、提示文案和 worker 行高亮；不要只看全池 `available`、单条 advice 文本，或
把 `/api/model-pool-status` 拼接成“看起来可发”的弱信号。若未来 Web Lab 直接接入
CLI host snapshot / viewport envelope，也应优先消费 `OutputUpdate.status_snapshot`
而不是解析终端文本。

当前 Web Lab API 到 CLI snapshot 概念的映射应按下面理解：

| Web Lab / backend 字段 | 更接近的 CLI/host 概念 | 宿主使用规则 |
| --- | --- | --- |
| `/api/backend-health.engine_busy`、`active_requests[]` | `send_block_state=busy`、`send_block_reason` | 这是发送按钮的最高优先级 busy 信号；一旦为真，不要再被 pool `available` 或 advice 文本覆盖 |
| `/api/backend-health.readiness_ok`、`safe_device_ok`、`experience_hygiene*` | repair/preflight gate、`prompt_submit_control` | 这些属于真实发送 gate；应直接禁用发送，而不是等模型池 advice 再反推 |
| `/api/model-pool-status.launch_allowed=false`、`launch_block_reason`、`reason` | 最接近 `route_send_block_reason` / `send_block_reason` | 这是 pool/route 调度阻断的主信号；如果后端尚未直接暴露 `route_send_block_*`，前端应优先读这里 |
| `/api/model-pool-status.route_metrics`、`capacity` | `route_pool_*` / pool capacity 概览 | 只能说明 route lane 的聚合容量与阻断统计，不等价于完整的 `route_workers` picker rows |
| `/api/model-pool-status.workers[]`、`worker_count`、`healthy_worker_count` | `workers` 全池健康行 | 这是 coarse pool 视图；当前不能替代 CLI 的 `route_workers` 决策行，除非后端未来补出 route-scoped worker 列表 |
| `/api/model-pool-advice.advice`、`kind`、helper/capacity 建议 | 辅助文案，不是发送源真相 | advice 只用于解释“为什么现在建议这样做”，不能单独决定发送按钮是否可点 |

换句话说，Web Lab 当前是“`backend-health` 决定真实 gate，`model-pool-status` 决定
pool/route 容量诊断，`model-pool-advice` 决定解释文案”。只要这三个来源之间有冲突，
前端必须按这个优先级裁决，而不是把它们合成为单一文本状态后再反推能否发送。
当前 tools 源码证据是：`tools/rustgpt-lab/src/backend/parse.rs` 读取
`engine_busy`、`active_requests[]`、`readiness_ok`、`safe_device_ok`、
`readiness_failures`、`safe_device_failures` 和 `experience_hygiene`；
`tools/rustgpt-lab/src/backend/gate.rs` 用这些字段生成发送前阻断原因；
`tools/rustgpt-lab/src/model_pool_advice.rs` 从 `workers[]`、`capacity` 和 helper
role 字段生成只读 advice，并输出 `read_only=true`、`launches_process=false`、
`sends_prompt=false`。当前 Web Lab 还没有直接消费 CLI 已验证的
`route_workers[*].picker_action_label`、`decision_display_snapshot()`、
`worker_status_display_snapshot()` 或 `selection_wire_*` 字段；这些是后续接入
CLI/service host snapshot 时应补齐的缺口，不能把现有 coarse `workers[]` 当成完整
worker picker 契约。

## SmartSteam Forge TUI

Forge TUI 是 SmartSteam 操作员路径，适合看 provider/runtime 状态、诊断、
hygiene/repair dry-run、worker 列表、上下文窗口和取消控制。它可以连接已有后端，
但不应该为了查看状态而启动或停止 Gemma。

连接已有后端：

```powershell
cd D:\rust-norion
cargo run --manifest-path tools\smartsteam-forge\Cargo.toml -- --provider runtime --backend 127.0.0.1:7878
cargo run --manifest-path tools\smartsteam-forge\Cargo.toml -- --provider runtime --backend 127.0.0.1:7878 --max-tokens default
```

只读状态检查在 TUI 内完成：

```text
/status
/workers
/context-window
/max-tokens
/hygiene dry-run
/repair dry-run
/audit
```

这些命令用于观察 provider、route、worker pool、上下文窗口和经验治理状态。
遇到 busy、queued 或 backpressure 时，Forge 应显示 wait/retry advice，保留输入
草稿，并让只读诊断命令继续可用。Ctrl+X 只取消当前流，不应当停止后端或 Gemma
runtime。
真正发送 prompt 只应来自 Forge 的对话输入/发送动作；`/status`、`/workers`、
`/context-window`、`/max-tokens`、`/hygiene dry-run`、`/repair dry-run` 和
`/audit` 是只读或 dry-run 路径。

Forge 的状态渲染顺序也应固定：发送按钮/Enter 先读 `prompt_submit_control`，再读
`route_send_block_*`、`send_block_reason` 和 `request_preview`；worker 面板优先展示
`route_workers` 的 picker/decision 行，再回退到全池 `workers`。`/state`、`/status`
必须等价，`/workers`、`/worker-status`、`/endpoints` 也必须等价；不要因为入口名字
不同而切到另一套解析逻辑或降低 gate 严格度。

当前 Forge provider/runtime 信号到 CLI snapshot 概念的映射建议按下面理解：

| Forge 当前来源 | 更接近的 CLI/host 概念 | 宿主使用规则 |
| --- | --- | --- |
| `/ready`、`/preflight`、`/doctor` 的 `health/readiness/safe-device` 结果 | `prompt_submit_control`、repair/preflight gate | 这些是发送 gate 的第一优先级；只要 readiness 或 safe-device 失败，就不要再被 worker healthy 数量或 provider ready 文案覆盖 |
| `/doctor` 或 `/health` 暴露的 `busy=true`、`active_requests`、`prompt_preview` | `send_block_state=busy`、`send_block_reason` | 这是 active stream/busy 的主信号；用于禁用 Enter/send，并展示“当前谁在占用引擎” |
| Forge model-pool/provider 路由结果里的 `route_allowed`、`reason`、`dependency_precheck`、`routing_weights` | `route_send_block_*`、route gate 解释 | 这些字段比普通 advice 更接近 route lane 真相；如果它们说当前 role/task 不能派发，就不要再被全池 ready 文案覆盖 |
| Forge model-pool/provider 路由结果里的 `selected_role`、`role_candidates`、`route_metrics`、`candidate_workers` | `route_workers` / `route_pool_*` 的近似替身 | 这是当前 task/route 的 worker 视角；应优先用于 worker 面板与 route 诊断，而不是退回到全池 healthy 汇总 |
| `/status`、`/workers`、`/context-window` 的本地只读命令 | `status_snapshot`、`workers`、`request_preview` 等宿主状态投影 | 这是 TUI 内最接近 CLI host contract 的入口；如果这些命令和 `/doctor` 文案冲突，先信本地结构化状态，再把 `/doctor` 当解释信息 |
| provider/runtime helper 建议、hints、advice 文本 | 辅助文案，不是发送源真相 | 只用于解释后续动作和排障路径，不能单独决定发送按钮是否启用 |

换句话说，Forge 当前应按“本地只读命令/route 结果决定状态，`/doctor` 和 hints 解释原因”
来消费数据，而不是把 provider/runtime 文本提示当成唯一真相。尤其是 busy、safe-device、
experience hygiene 和 route dependency 这几类阻断，必须先落实到发送 gate，再决定是否
展示 worker 扩容或恢复建议。
当前 Forge 源码证据是：`tools/smartsteam-forge/src/provider/model_pool.rs` 对
model-pool status/route 响应强制检查 `read_only=true`、`launches_process=false`、
`sends_prompt=false`，并从 worker row 读取 `role`、`status`、`ready` /
`role_ready`、`base_url`、`role_block_reason`、runtime/device 和 queue/capacity
相关字段；route 侧读取 `route_allowed`、`reason`、`selected_role`、
`selected_base_url`、`resource_precheck` 和 `dependency_precheck`。这些字段可映射到
CLI/service 的 `workers`、`route_pool_*`、`route_send_block_reason` 和
`send_block_reason`，但当前还不是完整的 `route_workers` picker rows，也没有
`request_preview` / `stream_chunk` / `input_action_snapshot` 这种 CLI output envelope
字段。接入时应继续保持只读：这些 status/route 检查只能解释 gate 和 worker 状态，不得
触发 `StartStream` 或发送 prompt。

## 流式发送、取消和恢复

只有实际需要验证 Gemma 12B 对话时才发送 prompt。发送前先看只读状态：

- Web Lab：顶部 backend/model-pool 状态必须显示后端可用，且没有 `engine_busy`、
  safe-device/readiness 或 experience hygiene/index 阻断。
- Forge TUI：先用 `/status`、`/workers`、`/context-window` 和必要的
  `/hygiene dry-run` / `/repair dry-run` 确认 gate；不要用试探 prompt 代替状态检查。
- CLI/TUI host：读 `InputControlSnapshot.prompt_submit_control`、
  `request_preview`、`route_send_block_chunk` 和 `send_block_reason`。blocked 状态下
  `request_preview` 仍可展示下一次会发送的 route/history/token 边界，但不能记录 user
  turn 或启动 stream。

取消当前流时只取消正在进行的回答：

- Forge Ctrl+X、CLI Ctrl+X 或 Web 取消按钮应映射到 `ChatSession::cancel_stream()`。
- 已收到的 partial assistant 文本可以继续显示为 interrupted partial，但不能写入
  assistant history，也不能污染下一轮短上下文。
- 取消后下一次被 gate 接受的 prompt 会重新 `StartStream`，清空旧 partial 和
  `last_error`；下一轮请求只携带已记录的 user/assistant history 和新 user prompt。
- 如果后端晚到 `done`、`delta`、`final` 或 `error`，UI/CLI 应按 terminal state
  忽略它们，不能把旧 partial 改写成 completed answer。

## 上下文与 token 口径

三类入口的状态文案要保持同一套含义：

- `history_messages` 是会话里已经记录的 user/assistant 历史条数。
- `context_messages` 是下一次请求会携带的历史上下文条数；单轮请求可以是 0。
- `messages` 是下一次请求的总消息数，通常等于 `context_messages + 1` 个新 user
  prompt。
- `history_limit` 是本地短上下文保留条数，不是生成 token 数。
- `max_tokens` 是本次请求的生成 token 预算；`backend-default` 表示 CLI/UI 不下发
  `max_tokens` 字段，由后端使用自己的默认值。
- `wire_sends_max_tokens=false` 时，UI 可以显示 `backend-default`，但不要把它渲染成
  一个具体数字；`wire_sends_max_tokens=true` 时才显示实际会随请求发送的预算。
- Web Lab 浏览器里的“生成预算 max_tokens”控件是生成预算控件；当前默认会随请求显式
  发送 `max_tokens=262144`。这和 `norion-cli` 的 `backend-default` 不是同一种状态，
  后者表示请求体里没有 `max_tokens`。

Web Lab 顶部的“上下文 N/M 条短会话消息”对应短会话 history/context
消息窗口，Forge `/context-window` 和 CLI `history_limit` 也按消息条数解释。
后端健康里的 `n_ctx` / `gemma_runtime_context_window` 是模型运行时上下文容量；
请求里的 `max_tokens` 是生成预算。三者可以同时显示，但不要互相替换或把
`backend-default` 推断成某个固定数值。

## 证据索引

下面这些测试是当前 CLI/UI 只读状态 contract 的一手证据，适合在后续集成或审计时直接引用：

| 主题 | 当前一手证据 |
| --- | --- |
| model-pool gate 下 `/status`、`/workers` 与别名保持本地只读，不启动 stream、不污染 history | `crates/norion-cli/src/input.rs`: `model_pool_status_and_workers_stay_read_only_under_engine_busy_and_health_preflight`、`model_pool_status_commands_stay_read_only_when_route_is_backpressured` |
| CLI host control snapshot 在 gate pressure 下继续携带结构化 route/pool/worker 状态 | `crates/norion-cli/src/input.rs`: `model_pool_status_commands_preserve_structured_host_snapshot_under_gate_pressure` |
| CLI workers host DTO 可把 `/workers` 的 read-only route/worker/picker/selection-wire 字段投影给 Web Lab/Forge，且标记不启动 stream、不发送 prompt、不携带 request preview/stream/input-action | `crates/norion-cli/src/status.rs`: `workers_host_snapshot_projects_read_only_dto_for_web_and_forge` |
| service workers host DTO 可从 `ModelPoolRouteSnapshot` 投影 read-only route/worker/picker/selection-wire 字段；safe-device repair gate 下保留 ready/busy/backpressure worker 状态、repair reason 和 pinned selection wire，且标记不启动进程、不发送 prompt、不启动 stream、不携带 request preview/history/stream/input-action side effect；allowed、engine busy、repair gate、route backpressure 下未来 Web Lab/Forge 消费该 DTO 仍只读 | `crates/norion-service/src/gate.rs`: `workers_host_snapshot_projects_service_dto_under_repair_gate_without_side_effects`、`workers_host_snapshot_keeps_web_lab_forge_boundary_read_only` |
| 8686-8690 runtime worker row 可清楚区分 ready、busy、route-unavailable 和 backpressure，且 Web Lab/Forge/CLI 消费该 DTO 仍只读、不访问端口、不启动 stream、不重放输入；JSON-facing field-name snapshot 锁住 host/worker 字段集合 | `crates/norion-service/src/gate.rs`: `workers_host_snapshot_marks_8686_8690_readiness_without_side_effects`、`workers_host_snapshot_json_field_names_are_stable_for_8686_8690`；`crates/norion-cli/src/status.rs`: `workers_host_snapshot_projects_8686_8690_readiness_for_web_lab_and_forge`、`workers_host_snapshot_json_field_names_are_stable_for_8686_8690` |
| 8686-8690 status consumer field bundle 同时覆盖 ready、busy、route-unavailable、backpressure、read-only、no prompt、no stream 字段 | `crates/norion-service/src/gate.rs`: `status_consumer_field_bundle_covers_8686_8690_worker_states_and_read_only_flags`；`crates/norion-cli/src/status.rs`: `status_consumer_field_bundle_covers_8686_8690_worker_states_and_read_only_flags` |
| UI/Forge/Web Lab 连续读取 8686-8690 status/worker DTO 时只能观察现有 busy/readiness，不能触发 stream、重放输入、启动模型或改变 session/pool 状态 | `crates/norion-service/src/gate.rs`: `status_consumer_reads_8686_8690_without_changing_busy_or_readiness`；`crates/norion-cli/src/status.rs`: `status_consumer_reads_8686_8690_without_replaying_input_or_changing_session` |
| model-pool readiness snapshot bundle 的 consumer-facing 口径：quality/helper ports、readiness/safe-device、`engine_busy`、worker count/capacity 和外部 `model_cache` label 都只能作为只读诊断；`model_cache` 不是当前 workers host DTO 字段，不能触发模型下载、cache 预热、stream、prompt replay 或改写 busy/readiness | 本 runbook `model-pool readiness snapshot bundle`；`crates/norion-service/src/gate.rs`: `workers_host_snapshot_keeps_web_lab_forge_boundary_read_only`、`status_consumer_field_bundle_covers_8686_8690_worker_states_and_read_only_flags`、`workers_host_snapshot_keeps_external_model_cache_diagnostics_out_of_service_dto`；`crates/norion-cli/src/status.rs`: `workers_host_snapshot_projects_read_only_dto_for_web_and_forge`、`status_consumer_field_bundle_covers_8686_8690_worker_states_and_read_only_flags`、`workers_host_snapshot_keeps_external_model_cache_diagnostics_out_of_cli_dto` |
| SmartSteam daemon/supervisor/model-pool status host snapshot 只读消费契约：UI/CLI 可以读取 daemon running/PID、supervisor check-only、active/ledger round、readiness、engine busy、remote chain、external model cache label、pool/route pool 和 route block reason；重复读取不能启动/停止 daemon、触碰远端、下载/预热模型、发送 prompt、开启 stream、重放输入或修改 busy/readiness/active round | `crates/norion-service/src/gate.rs`: `smartsteam_status_snapshot_reads_daemon_supervisor_pool_without_side_effects`、`smartsteam_status_snapshot_field_bundle_is_read_only_for_ui_consumers`; `crates/norion-cli/src/status.rs`: `smartsteam_status_host_snapshot_reads_daemon_supervisor_pool_without_replay`、`smartsteam_status_host_snapshot_field_bundle_keeps_cli_boundary_read_only` |
| SmartSteam worker-window clean-room replacement 状态：UI/CLI/Forge 可以同时显示 daemon/supervisor/remote pool 运行中，以及某个协作 worker window `paused`、`polluted`、`archived` 或 `completed_evidence_only`、需要 fresh clean-room replacement；旧窗口 `assignment_allowed=false` / `original_window_blocks_assignment=true`，新的 clean-room replacement 才能 `assignment_allowed=true`；读取该状态不能创建新窗口、归档旧窗口、启动/停止 daemon、触碰远端、发送 prompt、开启 stream、重放输入或修改 busy/readiness/active round | `crates/norion-service/src/gate.rs`: `smartsteam_status_snapshot_marks_polluted_windows_for_clean_room_without_side_effects`; `crates/norion-cli/src/status.rs`: `smartsteam_status_host_snapshot_exposes_clean_room_window_replacement_without_replay`; `tools/smartsteam-forge/src/app/evolution_worker_window_status.rs`: `worker_window_status_surfaces_replacement_without_side_effects`、`worker_window_replacement_report_projects_evolution_loop_report_contract`; fixture: `tools/smartsteam-forge/fixtures/r30-clean-room-status.example.json` |
| SmartSteam daemon round-done ledger-lag 状态：UI/CLI/Forge 可以显示 stdout 已完成 `done_round` 但 ledger/report commit 仍 pending 的过渡态；读取该状态不能启动/停止 daemon、触碰远端、发送 prompt、开启 stream、重放输入、修改 active round 或写 `.ndkv` | `crates/norion-service/src/gate.rs`: `smartsteam_status_snapshot_surfaces_round_done_ledger_pending_without_side_effects`; `crates/norion-cli/src/status.rs`: `smartsteam_status_host_snapshot_surfaces_round_done_ledger_pending_without_replay`; `tools/smartsteam-forge/src/app/evolution_worker_window_status.rs`: `daemon_round_transition_status_surfaces_ledger_pending_without_side_effects`; fixture: `tools/smartsteam-forge/fixtures/r30-clean-room-status.example.json` |
| SmartSteam live daemon round progress 字段：service status 从 active/ledger/done 状态投影 `latest_done_round` 与 `round_in_progress`；CLI host adapter 原样复制 service 的 transition/context hygiene DTO。active round 大于 latest done round 时是只读 in-progress 诊断；stdout done marker 已出现但 ledger 未 commit 时是 `round_in_progress=false` 的 pending commit 诊断 | `crates/norion-service/src/gate.rs`: `smartsteam_status_snapshot_reads_daemon_supervisor_pool_without_side_effects`、`smartsteam_status_snapshot_surfaces_round_done_ledger_pending_without_side_effects`; `crates/norion-cli/src/status.rs`: `smartsteam_status_host_snapshot_surfaces_round_done_ledger_pending_without_replay` |
| SmartSteam next-round decision status 字段：service/CLI 可以可选显示 `next_round_decision_report_v1` 等价 live facts，包括 `safe_to_wait_current_round_active`、`safe_to_continue_after_current_round`、`operator_attention_blocked`、round/evidence/reason 信息；也可从当前 evolution-loop status/report-shaped `decision_status`、`display_state` / `live_status_display_state`、`current_round_active`、`readiness_can_schedule_next_round`、`operator_attention_required`、`failure_reasons` 等字段投影；缺席时保持 `None` 兼容旧消费者，带 process/prompt/dispatch/memory/`.ndkv` side-effect 许可的 report 不会被显示成 safe status；读取该状态不能启动/停止 daemon、触碰远端、发送 prompt、开启 stream、创建线程、修改 worker-window/active round 或写 `.ndkv` | `crates/norion-service/src/gate.rs`: `smartsteam_status_snapshot_preserves_absent_next_round_decision_compatibility`、`smartsteam_status_snapshot_surfaces_next_round_decision_without_side_effects`、`smartsteam_status_snapshot_surfaces_operator_attention_blocked_next_round_decision`、`smartsteam_status_snapshot_maps_current_next_round_decision_report_fields`、`smartsteam_status_snapshot_ignores_next_round_decision_report_side_effect_markers`; `crates/norion-cli/src/status.rs`: `smartsteam_status_host_snapshot_preserves_absent_next_round_decision_compatibility`、`smartsteam_status_host_snapshot_surfaces_next_round_decision_without_replay`、`smartsteam_status_host_snapshot_consumes_report_shaped_next_round_decision_without_replay` |
| SmartSteam next-round downstream consumer status 字段：service/CLI 可以从 `next_round_downstream_status_consumers_v1` 的 root 或 `live_status_bundle` nested 形状消费 display-only facts，保留可选 `round_id_evidence` 的 `source_schema`、`active_round`、`ledger_latest_round`、`latest_done_round` 和 daemon transition 证据；缺席时保持旧兼容，带 prompt/process/dispatch/memory/`.ndkv` side-effect 许可的 downstream block 不会被提升为 safe consumer status | `crates/norion-service/src/gate.rs`: `smartsteam_status_snapshot_maps_captured_current_status_json_next_round_decision_fixture`、`captured_current_status_json_downstream_consumers_accept_root_and_nested_round_evidence`、`post_r44_safe_to_wait_status_replay_accepts_root_and_live_bundle_downstream_status`、`next_round_decision_report_drops_downstream_side_effect_markers_only`; `crates/norion-cli/src/status.rs`: `smartsteam_status_host_snapshot_surfaces_captured_current_status_json_next_round_decision`、`captured_current_status_json_downstream_consumers_accept_root_and_nested_round_evidence`、`smartsteam_status_host_replays_post_r44_safe_to_wait_root_and_live_bundle_downstream_status` |
| SmartSteam memory startup admission 状态：UI/CLI 可以显示 R22 `MemoryStartupAdmissionEvidence` 的 startup admission、index quality/index operation/refresh、context rot blocker、helper prose 和 non-contract line evidence；读取该状态不能把 helper prose 或旧窗口 payload 扩张为 live write/admission、不能允许 `.ndkv` 写入、不能发送 prompt 或开启 stream | `crates/norion-service/src/gate.rs`: `smartsteam_status_snapshot_surfaces_memory_startup_admission_without_side_effects`; `crates/norion-cli/src/status.rs`: `smartsteam_status_host_snapshot_surfaces_memory_admission_without_replay` |
| SmartSteam clean-room handoff/admission 状态：UI/CLI 可以同时消费 memory admission safe、agent replacement plan required/available/prompt-ready、旧窗口 follow-up blocked、no old-window payload read、no live write/no store mutation/no `.ndkv` write/no runtime side effects；读取该状态不能解析 helper prose 或旧窗口 payload，不能创建线程/发送消息/启动 stream/触碰 daemon 或远端 | `crates/norion-service/src/gate.rs`: `smartsteam_status_snapshot_surfaces_clean_room_handoff_without_side_effects`; `crates/norion-cli/src/status.rs`: `smartsteam_status_host_snapshot_surfaces_clean_room_handoff_without_replay` |
| SmartSteam self-improve proposal lifecycle 状态：UI/CLI 可以显示 proposal 的 `candidate`、`validated`、`admitted`、`quarantined`、`promoted`、`repair-required` 生命周期、source round、evidence ids、validation status、memory admission status 和 side-effect flags；读取该状态不能解析 helper prose、replay prompt、调用模型、启动 stream、写 memory、写 `.ndkv`、修改 live store 或执行 runtime promote/quarantine | `crates/norion-service/src/gate.rs`: `smartsteam_status_snapshot_surfaces_self_improve_proposals_without_side_effects`; `crates/norion-cli/src/status.rs`: `smartsteam_status_host_snapshot_surfaces_self_improve_proposals_without_replay` |
| SmartSteam helper-stage repair-required 状态：UI/CLI 可以显示 typed `stage_label`、`state_label=complete|repair-required`、contract complete、repair required、source round、evidence ids、reason codes、以及缺失 helper role 的 repair-required proposal count/roles/proposals；读取该状态不能解析 helper prose、replay prompt、调用模型、启动 stream、写 memory、写 `.ndkv`、修改 live store、创建 replacement window 或修改 worker-window status | `crates/norion-service/src/gate.rs`: `smartsteam_status_snapshot_surfaces_helper_stage_repair_required_without_side_effects`; `crates/norion-cli/src/status.rs`: `smartsteam_status_host_snapshot_surfaces_helper_stage_repair_without_replay` |
| service / CLI workers host DTO 字段 parity：两边都只投影 host 可消费的 route、worker、picker、selection wire 与 block reason；边界字段锁定 read-only、不发送 prompt、不 StartStream、不携带 request preview、可回放 history、stream chunk 或 input action | `crates/norion-service/src/gate.rs`: `ModelPoolWorkersHostSnapshot` / `ModelWorkerHostSnapshot` / `workers_host_snapshot_projects_service_dto_under_repair_gate_without_side_effects`；`crates/norion-cli/src/status.rs`: `CliWorkersHostSnapshot` / `CliWorkerHostSnapshot` / `workers_host_snapshot_projects_read_only_dto_for_web_and_forge` |
| CLI output/view 层在 busy、backpressure、repair gate 下保持 `/status`、`/workers` 为 read-only host snapshot，不携带 request preview、action snapshot 或 stream chunk，并保留 worker role/health/picker 状态 | `crates/norion-cli/src/output.rs`: `status_and_workers_host_snapshots_keep_local_envelope_under_gates`、`workers_snapshot_projects_web_forge_fields_under_repair_gate_without_stream_side_effects` |
| CLI output/view 层在 route backpressure 下继续携带 `status_snapshot`、`route_workers`、`route_pool_*` | `crates/norion-cli/src/output.rs`: `route_backpressure_host_outputs_preserve_worker_rows_and_route_lane_snapshot` |
| service frontend gate 的原始展示源稳定，不会把 offline/repair/busy/backpressure 混成同一种文本 | `crates/norion-service/src/gate.rs`: `frontend_gate_decision_display_snapshot_is_stable_for_hosts` |
| service 单 worker 健康展示源稳定，busy/backpressure 的 worker chunk 文案与 pressure 标志固定 | `crates/norion-service/src/gate.rs`: `worker_status_display_snapshot_is_stable_for_busy_and_backpressure_hosts` |
| service worker picker 在 safe-device repair gate 下仍暴露 role/preference、worker health、decision display chunk 和 pinned selection wire 字段，但最终 action 保持 `repair_gate` | `crates/norion-service/src/gate.rs`: `route_workers_keep_frontend_repair_gate_over_worker_availability` |
| service route snapshot 内的 worker picker rows 与 `route_workers(intent)` helper 保持完全一致 | `crates/norion-service/src/gate.rs`: `route_snapshot_keeps_same_worker_rows_as_route_workers_helper` |
| request/send preview 只暴露 route/history/token/start 边界，不泄露 prompt 文本，并保留 `backend-default` / `history_limit` 语义 | `crates/norion-cli/src/input.rs`: `control_snapshot_request_preview_keeps_context_messages_distinct_from_backend_default_tokens`; `crates/norion-cli/src/output.rs`: `request_preview_snapshot_exposes_send_boundary_without_prompt_text`、`request_preview_can_carry_history_policy_without_changing_terminal_line`、`started_turn_preview_snapshot_exposes_start_boundary`、`started_turn_preview_keeps_prompt_text_out_of_local_send_line` |
| 取消当前流后，宿主只能看到 interrupted partial / retry-ready 状态，不能把 partial 或 cancel reason 污染下一轮上下文 | `crates/norion-cli/src/input.rs`: `status_command_after_cancel_reports_interrupted_partial_without_request`、`control_snapshot_reenables_start_after_cancel_without_promoting_partial_context`; `crates/norion-service/src/session.rs`: `cancel_stream_interrupts_active_stream_and_keeps_partial_without_history`、`cancel_stream_recovery_respects_history_limit_without_partial_context` |
| backend `status`/`heartbeat` 与 timeout/late frames 只更新流内诊断，不得清掉 pressure gate 或污染后续重试上下文 | `crates/norion-service/src/stream.rs`: `status_and_heartbeat_frames_do_not_clear_pressure_gate`、`status_and_heartbeat_frames_do_not_pollute_stream_context`、`read_timeout_interrupt_does_not_pollute_retry_context` |
| 终止态/`outcome` 必须区分 completed、interrupted、failed 与 pressure close，并保留结构化 terminal state | `crates/norion-cli/src/output.rs`: `stream_outcome_output_carries_structured_terminal_and_pressure_state`、`outcome_status_distinguishes_completed_interrupted_and_failed`、`outcome_status_preserves_pressure_close_reason_after_incomplete_stream`; `crates/norion-service/src/session.rs`: `timeout_interrupted_snapshot_keeps_retry_gate_open_without_partial_context` |

## 快速判别

| 入口 | 用途 | 启动会发生什么 | 只读状态路径 | 真实发送 prompt |
| --- | --- | --- | --- | --- |
| `norion-cli` | 协议/input/output shell | 打印本地启动快照，不连后端 | `cli_smoke`、嵌入 host 的 `/status`、`/workers` | 仅宿主把 `InputAction::StartStream` 接到后端时发送 |
| Web Lab | 浏览器 SSE 联调 | 连接已有后端，或由 lab 脚本启动 built-in/Gemma 链路 | `status-*-lab.cmd`、`/api/backend-health`、`/api/model-pool-*` | 聊天输入发送按钮或等价 chat API |
| Forge TUI | SmartSteam 操作员界面 | 连接已有 runtime provider/backend | `/status`、`/workers`、`/hygiene dry-run`、`/repair dry-run` | 对话输入/发送动作 |

如果目标只是确认系统状态，先选只读路径；只有需要实际对话或流式验证时，才发送
prompt。
