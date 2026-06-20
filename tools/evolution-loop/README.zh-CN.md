# SmartSteam Evolution Loop

`tools/evolution-loop` 是外围自进化节拍器。它不训练 Gemma 权重，也不把逻辑塞进
`rust-norion` 主服务；它会常驻调用现有 `/v1/business-cycle-stream`，让模型在
业务循环里持续生成、反馈、回放、自检、保存状态，并把每轮结果写入 JSONL ledger。

## 先启动模型链路

推荐把 Gemma 12B 跑在远端 Mac，本机只跑 tunnel、`rust-norion` 后端、Web Lab：

```powershell
cd D:\rust-norion
.\tools\smartsteam-forge\start-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789 -ContextTokens 8192 -DefaultMaxTokens 4096
```

如果当前目标是验证 Gemma 4 12B 的完整原生窗口，启动链路时显式拉到模型上限：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789 -RestartRemote -ContextTokens 262144 -DefaultMaxTokens 262144
```

Web Lab 手动测试入口：

```text
http://127.0.0.1:8789/
```

## 跑有限轮自进化

```powershell
cd D:\rust-norion
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -MaxTokens 4096 -SelfImproveLimit 1
```

先看最终 cargo 命令、不连接后端也不发 prompt，可以加 `-CheckOnly`：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -CheckOnly -Backend 127.0.0.1:7979 -Rounds 5 -MaxTokens 4096 -SelfImproveLimit 1
```

如果要求夜跑只在真实 runtime 已经打开完整上下文窗口时进行，加上 runtime context gate：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -MaxTokens 4096 -SelfImproveLimit 1 -MinRuntimeContext 262144
```

默认会写：

```text
D:\rust-norion\target\evolution\evolution-ledger.jsonl
```

再次启动时会读取这个 ledger 里的最大 `round`，自动从下一轮继续，避免每次都生成
`smartsteam-evolution-loop-0001`。需要区分不同实验，可以改 case 前缀：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -CasePrefix nightly-evo
```

默认还会把已有 ledger 汇总成几行短上下文，附加到下一轮 prompt 里。这样常驻运行不是
盲目重复调用模型，而是会参考上一轮成功率、feedback、self-improve、Rust check 和最近
失败原因再行动。如果要做固定 prompt 对照实验，可以关掉：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -NoReportContext
```

如果要让下一轮同时看到当前模型池状态，先保存只读 pool status artifact，再把它传给
`evolution-loop`。这不会启动模型，也不会发送 prompt；它只让循环知道 `8686-8690`
哪些 worker 可达，以及 `model_pool_launch` 是否通过：

```powershell
.\tools\gemma-chain\gemma-chain.cmd pool-status -JsonStatus > target\evolution\pool-status.json
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -PoolStatusJson target\evolution\pool-status.json
```

如果已经有 `rust-norion` 后端在 `127.0.0.1:7979`，可以让 loop 在每轮发 prompt
之前自动刷新模型池 artifact。它只调用后端的 `/v1/model-pool/manifest`、
`/v1/model-pool/status` 和 `/v1/model-pool/route-plan`，并要求返回
`read_only=true`、`launches_process=false`、
`sends_prompt=false`；契约不满足时会直接停止，不会进入业务 prompt：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RefreshPoolArtifacts
```

默认会写：

```text
target\evolution\pool-manifest.json
target\evolution\pool-status.json
target\evolution\pool-route-review.json
```

要换任务路由类型，比如让本轮使用 test-gate 角色规划：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RefreshPoolArtifacts -PoolRouteTaskKind test-gate
```

如果要同时观察多角色流水线是否具备条件，可以让 loop 在每轮开始前刷新多个 stage
route artifact。主请求仍由 `-PoolRouteTaskKind` 决定；额外 stage route 只作为只读证据
写入 `allocation_evidence`，不会额外发送 prompt：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RefreshPoolArtifacts -PoolRouteTaskKind review -PoolStageRouteTaskKinds summary,review,test-gate
```

这会在 `target\evolution` 下生成或更新 `pool-route-summary.json`、
`pool-route-review.json`、`pool-route-test-gate.json`。后续报告和 ledger 能看到每个
stage 是否有 ready worker，为真正的 `summary -> review -> test-gate -> quality`
流水线提供证据。

如果这些 stage route 必须全部 ready，再加 `-PoolStageRouteGate`。它仍然不会额外发
prompt，只会在业务 prompt 前读取这些 route artifact；任意 stage `route_allowed=false`、
没有 selected role、没有 ready candidate，都会直接失败：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RefreshPoolArtifacts -PoolRouteTaskKind review -PoolStageRouteTaskKinds summary,review,test-gate -PoolStageRouteGate
```

如果还要让下一轮看到某类任务的只读路由建议，例如 review 任务应该候选
`review -> quality` 但当前是否被质量 worker gate 阻断，可以额外保存
`pool-route-plan` artifact：

```powershell
.\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind review -JsonStatus > target\evolution\pool-route-review.json
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -PoolStatusJson target\evolution\pool-status.json -PoolRouteJson target\evolution\pool-route-review.json
```

`PoolRouteJson` 只提供证据，不启动模型、不发送 prompt，也不会改变 report gate。
报告和下一轮上下文会显示 `task_kind`、`route_allowed`、`selected_role`、
`role_candidates`、`quality_context_tokens`、`quality_context_required_tokens`、
`quality_context_sufficient`、候选 worker 健康数量，以及 selected worker 自报的
`selected_runtime_backend`、`selected_runtime_device`、`selected_runtime_accelerator`
和 `selected_gpu_layers`。这些字段用于复盘是否疑似 CPU fallback；为空时代表当前
worker API 没有报告，不代表本机已经证明没有 GPU/Metal。
真正跑业务轮次时，`evolution-loop` 还会把 `pool_status` / `pool_route` 的短摘要写入
每条 JSONL ledger 的 `allocation_evidence` 数组，便于后续复盘某轮是在什么模型池
可用性和路由门禁背景下执行的。

如果这轮必须由模型池路由明确放行，再加 `-RequirePoolRoute`。这会在发送任何
business-cycle prompt 之前读取 `PoolRouteJson`；当 `route_allowed=false`、
`quality_context_sufficient=false`、`selected_role` 缺失或没有 ready candidate 时直接停止：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -PoolRouteJson target\evolution\pool-route-review.json -RequirePoolRoute
```

配合自动刷新时，可以省掉手动生成 JSON 的步骤：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RefreshPoolArtifacts -RequirePoolRoute
```

不使用 `-RefreshPoolArtifacts` 时，`-RequirePoolRoute` 不会自己刷新 route artifact；
需要先用 `gemma-chain pool-route-plan -TaskKind ... -JsonStatus` 生成当前只读路由证据。

远程苹果机 / 多模型链路也可以作为只读证据接进自进化循环。先生成一次链路状态，
这个命令不会 SSH、不会启动进程、不会发送 prompt，只把本机端口和模型池容量状态写成
JSON：

```powershell
.\tools\smartsteam-forge\status-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789 -JsonStatus > target\evolution\remote-chain-status.json
```

然后把这份 artifact 交给 `evolution-loop`。开启 `-RemoteChainGate` 后，如果
`readiness.ready` 不是 `true`，循环会在发送 business-cycle prompt 前 fail-closed，
不会继续消耗本地或远程模型：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RemoteChainStatusJson target\evolution\remote-chain-status.json -RemoteChainGate
```

也可以让启动器先刷新这份只读状态，再进入 gate；这仍然不会 SSH、不会启动模型、
不会发送 prompt：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RefreshRemoteChainStatus -RemoteChainGate
```

默认会写到 `target\evolution\remote-chain-status.json`，并从 `-Backend` 推断 backend
端口，Web Lab 默认按 `8789` 检查，模型 API 默认按本机隧道 `8686` 检查。端口不同时
显式覆盖：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RefreshRemoteChainStatus -RemoteChainLabPort 8789 -RemoteChainLocalModelPort 8686 -RemoteChainGate
```

需要强制要求辅助小模型时，传 `-RemoteChainRequiredPoolWorkerRoles`。状态 JSON 会记录
`required_roles_ready` 和 `missing_required_roles`；只要缺 `summary/review/test-gate`
中的任意一个，`-RemoteChainGate` 就会在发 prompt 前失败：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RefreshRemoteChainStatus -RemoteChainRequiredPoolWorkerRoles summary,review,test-gate -RemoteChainGate
```

日常联调可以直接用组合开关。它会设置：
`-RefreshRemoteChainStatus`、`-RemoteChainGate`、`-RefreshPoolArtifacts`、
`-RequirePoolRoute`、`-PoolCapacityGate`、`-PoolAlignmentGate`、
`-PoolBudgetFairnessGate`、`-RequirePoolBudgetPolicy`、`-ExecutePoolStageCalls`，并默认写入
`target\evolution\model-pool-budget-fairness.json` 和使用
`target\evolution\pool-leases`，同时刷新 `summary,review,test-gate` stage route，
并开启 `-PoolStageRouteGate`。主 business-cycle 成功后会真实调用 helper stage，
事后报告默认要求 `summary,review,test-gate` 的真实
helper 反馈、最近一轮也有这些 helper 反馈，以及 `quality` 预算保留 + 低优先级 helper clamp
证据，方便多窗口运行时不争同一个 worker：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RemoteModelPoolGate
```

启动这种夜跑前，先跑本地 helper 模型 guard 自测。它只执行 `CheckOnly`，不会 SSH、
不会启动远端模型、不会发送 prompt；通过后才说明 helper 不会默认复用 quality 的
12B 模型，也不会把明显 12B+ 的模型当成小 worker：

```powershell
.\tools\smartsteam-forge\test-remote-model-pool-guards.cmd
```

`evolution-loop` 自己也有启动器自测，专门验证 `-CheckOnly`、默认 pool
manifest/status/route 路径、`-PoolAlignmentGate` 和 `-RemoteModelPoolGate` 的参数接线。
它不连接后端、不启动 cargo、不发送 prompt：

```powershell
.\tools\evolution-loop\test-evolution-loop-launcher.cmd
```

这条命令仍然是“先验收再发 prompt”：如果 `8686` 模型 API、后端、Web Lab、
quality worker、`summary/review/test-gate`、capacity 或 selected route 没就绪，
会在发送 business-cycle prompt 前失败。

不加 `-RemoteChainGate` 时，`-RemoteChainStatusJson` 仍会进入 report JSON 和每轮
`allocation_evidence`，适合先观察苹果机上的 quality / summary / review worker 是否
真的就绪、是否有足够上下文，再决定是否让它们参与夜跑。

如果要让模型池扩容建议直接拦住夜跑，再加 capacity gate。它会在每轮发送
business-cycle prompt 前读取 `-PoolStatusJson`；如果 `capacity.expansion_allowed`
缺失或为 `false`，本轮会直接停止，不会继续给任何 worker 发 prompt：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RefreshPoolArtifacts -PoolCapacityGate -PoolStatusJson target\evolution\pool-status.json
```

这个 gate 是 fail-closed 的。看到 `restore_quality_gate_first`、
`fix_runtime_acceleration_before_adding_workers` 或
`verify_worker_runtime_metadata_before_expansion` 时，先修 quality/Metal/GPU 证据，
再继续加 summary/review/index worker。
报告模式里如果同时传 `-ReportGate -PoolCapacityGate`，同一份 capacity 判定也会纳入
report gate；只传 `-Report -PoolCapacityGate` 则仍是只读汇总，不会发送 prompt。

路由放行后，`evolution-loop` 会把 selected worker 的调度意图写入
`/v1/business-cycle-stream` 请求体里的 `pool_dispatch` 对象，并把本轮
`max_tokens` 按 selected worker 的 `default_max_tokens` 夹紧。也就是说，
summary/review/test-gate 这类辅助 worker 不会默认吃掉 12B 质量 worker 的长输出预算。
rust-norion 后端会解析 `pool_dispatch`，优先应用其中的 `effective_max_tokens`，
并在 SSE `meta` 与 final JSON 里回显调度证据。请求入口仍是 rust-norion backend
的 business-cycle endpoint；当后端 runtime 支持 endpoint override（例如
`MistralRsHttpRuntime`）时，后端会把本轮生成临时转发到
`pool_dispatch.selected_base_url`，final JSON 显示
`pool_dispatch.worker_forwarded=true` 和
`dispatch_mode=runtime_endpoint_override`。如果后端 runtime 不支持动态 endpoint，
它会保持本地后端执行，只应用 token 预算，并显示
`worker_forwarded=false` / `dispatch_mode=backend_budget_only`。
后端会在生成前校验 `pool_dispatch.selected_base_url`；如果 runtime 拒绝这个地址，
本轮请求会直接失败，不会把 prompt 继续发给任何模型 worker。

当同时启用了 `-PoolBudgetFairnessJson`，`model_worker_v1` artifact 会把每次真实
worker 调用的 `default_max_tokens`、`configured_max_tokens`、`effective_max_tokens`、
`max_tokens_clamped` 和 `can_accept_low_priority_task` 一起落盘。也就是说，事后报告能
区分“quality 12B 保留 262144 大预算”与“summary/review/test-gate 小 helper 被合理夹紧到
自己的默认预算”。如果 helper 降低了 token 预算却没有 `max_tokens_clamped=true` 证据，
或者 helper 的 `effective_max_tokens` 超过 worker 默认值，或者 quality role 被错误夹紧，
budget fairness gate 会失败。报告模式里再加 `-RequirePoolBudgetPolicy` 时，即使
`budget_fairness_blocked=false`，也会强制要求 artifact 同时证明 quality 预算未被夹紧、
`configured_max_tokens=effective_max_tokens`，并且至少有一个低优先级 helper 被正确 clamp。

当同时启用了 `-PoolStageRouteTaskKinds summary,review,test-gate` 时，loop 还会把
每个 ready stage 的只读调度计划写入请求体 `pool_stage_dispatch[]`。这个数组会
包含 `task_kind`、`selected_role`、`selected_base_url`、runtime 设备信息以及
`effective_max_tokens`。如果没有启用 `-ExecutePoolStageCalls`，后端只把它作为多角色流水线计划记录到 SSE `meta`、
final JSON 和经验 note；它不会额外发送 summary/review/test-gate prompt，也不会
自动启动 worker。这样可以先验证多模型角色契约和索引证据，再逐步提升到真正的
多阶段执行。

要让 ready stage 真的通过后端 `/v1/model-pool/call` 执行，加
`-ExecutePoolStageCalls`。`-RemoteModelPoolGate` 和 daemon 夜跑会默认带上这个开关。
这个开关会在主 business-cycle 成功后，把本轮 prompt、主回答预览和 final JSON
预览发给每个 ready helper stage；它会真实发送额外 prompt、消耗 Apple helper worker 资源：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 3 -RefreshPoolArtifacts -PoolRouteTaskKind quality -PoolStageRouteTaskKinds summary,review,test-gate -PoolStageRouteGate -ExecutePoolStageCalls -PoolBudgetFairnessJson target\evolution\model-pool-budget-fairness.json
```

每个成功的 helper call 会记录 `pool_stage_call:executed`，并把
`elapsed_ms`、`answer_chars`、`answer_bytes`、`answer_approx_tokens` 写进
`model_worker_v1` artifact。这样可以证明 summary/review/test-gate 是否真的产出
了有界反馈，而不是只在路由计划里“看起来能跑”。如果任意 stage call 返回 HTTP
错误或 `ok=false`，本轮会标记为失败，防止夜跑把坏的多模型链路当成成功闭环。
helper prompt 会按角色给出不同的短结构：`summary` 写 `memory_update`，
`review` 写 `risk/change_request/verification`，`test-gate` 写
`verdict/validation_command/failure_kind`，`index` 写
`clean_gist/tags/dependency_link/source_origin/validation_timestamp/retention`。
每轮 ledger 也会额外写出 `helper_stage_feedback_by_role`，方便后续报告、索引和
下一轮 prompt 按角色消费，而不是只从一长串 `meta` 里猜。运行中的 loop 会在每轮
发送 prompt 前重新读取 ledger 和 pool artifacts，所以同一个进程里上一轮刚产生的
helper 反馈可以进入下一轮 prompt；`-NoReportContext` 会关闭这条动态注入链路。
如果同时配置了 `-PoolLeaseDir`，每个 helper stage 在发起 `/v1/model-pool/call`
前也会按 `role-port` 获取同一套本地 lease。多个窗口抢同一个 summary/review worker
时，`-PoolLeaseBusyPolicy skip-low-priority` 会跳过该 helper stage，而不是继续挤占
Apple worker；`fail` 或等待超时则会让本轮失败。

要在事后报告里强制证明这些 helper 真的返回过反馈，而不是只存在于 stage route 计划里，
加 `-RequireHelperStageRoles`。它只认 ledger 中的 `pool_stage_call_answer`，不会把
`pool_stage_call_skipped` 或 planned route 当成成功。如果还要防止旧 ledger 里的历史反馈
掩盖最近一轮没有 helper 参与，再加 `-RequireLatestHelperStageRoles`；它只检查最新一轮：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -ReportGate -PoolBudgetFairnessJson target\evolution\model-pool-budget-fairness.json -RequirePoolBudgetPolicy -RequireHelperStageRoles summary,review,test-gate -RequireLatestHelperStageRoles summary,review,test-gate
```

如果 `test-gate` helper 负责做最后验收，可以再加 `-RequireTestGatePass`。它会读取最近
一条 `test-gate` 反馈里的 `verdict: pass` 或 `verdict=pass`；如果是 `warn`、`fail`
或没有明确 verdict，report gate 会失败。这仍然只是读取 ledger，不会自动执行模型建议的
`validation_command`：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -ReportGate -RequireHelperStageRoles summary,review,test-gate -RequireTestGatePass
```

报告 JSON 会额外输出 `test_gate.latest_verdict`、
`test_gate.latest_validation_command`、`test_gate.latest_validation_command_safety`
和 `test_gate.latest_failure_kind`。prompt context 也会带上 `latest_test_gate=...`，
让下一轮模型知道应该优先补哪条验证命令；真正执行命令仍需要显式使用
`-ValidationCommand`。安全分类目前很保守：只把单条 `cargo test` / `cargo check` /
`cargo clippy`，以及带 `--check` 的 `cargo fmt` 标记为 `safe`；命令串联、管道、
重定向、`cargo run`、`--fix` 等都会标为 `unsafe`。

需要让报告门禁同时要求 test-gate 建议的是安全验证命令时：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -ReportGate -RequireHelperStageRoles summary,review,test-gate -RequireTestGatePass -RequireSafeTestGateValidationCommand
```

当你确认要让上一轮 `test-gate` 的安全命令参与下一轮运行时，再显式加
`-UseTestGateValidationCommand`。它只会采用分类为 `safe` 的
`latest_validation_command`；如果没有命令或命令是 `unsafe`，运行会在发送 prompt 前失败。
手写的 `-ValidationCommand` 优先级更高。运行完成后，ledger 和 report JSON 会记录
`validation_command_source`、`validation_command_safety` 和 `validation_command_preview`，
同时记录 `validation_phase`、`validation_status_code`、`validation_elapsed_ms`、
`validation_stdout_tail` 和 `validation_stderr_tail`。下一轮 prompt context 也会看到
`last_validation_command=...` 和 `last_validation_result=...`，便于模型知道验证证据来自
手写命令还是上一轮 test-gate，以及验证到底失败在什么输出上：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 3 -UseTestGateValidationCommand -ValidationPhase pre -ValidationTimeoutSecs 300
```

需要把 report gate 提升到“最近一轮必须采用上一轮 `test-gate` 的安全命令，并且已经真实执行通过”时，
再加 `-RequireTestGateValidationRun`。它只读 ledger/report 证据，不会自己执行命令；它要求最近一轮同时满足
`validation_command_source=test-gate`、`validation_command_safety=safe`、
`validation_checked=true`、`validation_passed=true` 和 `validation_status_code=0`：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -ReportGate -PoolBudgetFairnessJson target\evolution\model-pool-budget-fairness.json -RequirePoolBudgetPolicy -RequireHelperStageRoles summary,review,test-gate -RequireLatestHelperStageRoles summary,review,test-gate -RequireTestGatePass -RequireSafeTestGateValidationCommand -RequireTestGateValidationRun
```

如果验证命令是人工/守护进程固定配置的 `-ValidationCommand`，不要用
`-RequireTestGateValidationRun`；改用 `-RequireConfiguredValidationRun`。它要求最近一轮满足
`validation_command_source=configured`、`validation_checked=true`、
`validation_passed=true` 和 `validation_status_code=0`，但不要求命令来源伪装成
`test-gate`：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -ReportGate -RequireConfiguredValidationRun
```

如果同时配置了 `-PoolBudgetFairnessJson`，这些 stage 计划也会追加到
`model_worker_v1` artifact，字段 `execution_state` 为 `planned`。planned 事件会
保留 role、端口、endpoint、runtime 设备和 token 预算，方便审计本轮应该由哪些
helper 参与；预算公平统计只计算 `execution_state=executed` 的真实 worker 事件，
所以计划不会被误算成 0 token 调用失败。

如果要证明模型池真的在辅助开发，而不是只是多开进程抢 12B 资源，可以让 loop 把每轮
selected worker 的运行事件追加到 `model_worker_v1` artifact：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RefreshPoolArtifacts -RequirePoolRoute -PoolBudgetFairnessJson target\evolution\model-pool-budget-fairness.json
```

每条事件会记录 `role`、`worker_port`、`task_kind`、`success`、`feedback_applied`、
`runtime_tokens`、`latency_ms` 和 `blocked_primary_12b`。这个文件不发送 prompt，
只在业务轮次完成后落盘，后续 `-Report` 会把它汇总成
`model_pool_budget_fairness_report_v1`。

如果要让上一轮预算公平性直接拦住下一轮运行，再加运行前 gate。它会在每轮发送
business-cycle prompt 前重新读取 `-PoolBudgetFairnessJson`；如果汇总结果里
`budget_fairness_blocked=true`，本轮会直接停止，不再把 prompt 发给任何 worker：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -RefreshPoolArtifacts -RequirePoolRoute -PoolBudgetFairnessJson target\evolution\model-pool-budget-fairness.json -PoolBudgetFairnessGate
```

不传 `-PoolBudgetFairnessGate` 时，这个 artifact 仍然只作为落盘证据和 report context；
适合先观察几轮再决定是否把模型池扩容门禁收紧。

多个窗口同时跑 evolution-loop 时，可以再加本地 worker lease，防止两个循环同时占用
同一个 selected worker：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -PoolRouteJson target\evolution\pool-route-review.json -RequirePoolRoute -PoolLeaseDir target\evolution\pool-leases -PoolLeaseTtlSecs 1800
```

lease 文件按 `role-port` 写入 `PoolLeaseDir`。未过期 lease 存在时，本轮会在发 prompt
前停止；过期 lease 会被回收；正常轮次结束后会自动释放。lease 只包住实际
business-cycle 模型调用，不占用本地 validation/report 时间。

如果希望窗口在 worker 忙时排队一小段时间，而不是立刻退出，可以设置等待和轮询间隔：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -PoolRouteJson target\evolution\pool-route-review.json -RequirePoolRoute -PoolLeaseDir target\evolution\pool-leases -PoolLeaseTtlSecs 1800 -PoolLeaseWaitSecs 120 -PoolLeasePollSecs 5
```

默认 `PoolLeaseWaitSecs=0`，也就是忙时立即失败；显式等待才会轮询 lease 文件。

辅助型小模型任务还可以选择忙时跳过，而不是把本轮记成失败：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -PoolRouteJson target\evolution\pool-route-summary.json -RequirePoolRoute -PoolLeaseDir target\evolution\pool-leases -PoolLeaseBusyPolicy skip-low-priority
```

`PoolLeaseBusyPolicy` 可选 `fail`、`wait`、`skip-low-priority`。默认 `wait` 搭配
`PoolLeaseWaitSecs=0` 等价于忙时立即失败；`skip-low-priority` 只会在 route artifact
标记 selected worker 可接受低优先级任务时跳过，本轮不会发送 prompt，也不会写成功轮次。
连续跳过默认最多 3 次，避免多个窗口都在礼让时无限空转；需要更长等待可调大
`-MaxPoolLeaseSkips`，传 `0` 表示关闭这个跳过上限：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -PoolRouteJson target\evolution\pool-route-summary.json -RequirePoolRoute -PoolLeaseDir target\evolution\pool-leases -PoolLeaseBusyPolicy skip-low-priority -MaxPoolLeaseSkips 10
```

每轮成功需要收到完整 SSE 终止事件：正常结束必须有 `done`，后端错误必须有 `error`。
如果连接在 `done/error` 前关闭，或者 EOF 前还残留半截 SSE frame，本轮会记为
`stream truncated` 失败，不会把半截输出当成功写进 ledger。收到完整 `done` 后，还必须
有 `final` 事件，并且 final JSON 里 `ok=true`、业务 gate 通过、feedback/self-improve
有效。失败会写 ledger；连续失败达到 `-MaxFailures` 后停止。
默认不开严格 `state_gate/business_cycle_gate/trace_gate`，因为隔离状态刚启动时可能还没满足
严格 state gate，trace gate 也需要后端配置 schema 路径。需要严格验收时加
`-BusinessGate`；需要强制 trace gate 时再加 `-TraceGate`。

## 常驻运行

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Forever -IntervalSecs 30 -MaxFailures 3 -MaxTokens 4096 -MaxTotalTokens 20000 -MaxRuntimeSecs 3600
```

停止用 `Ctrl+C`。常驻前建议先用 3 到 5 轮确认链路稳定，再放开更长时间。
默认连续 3 轮没有 feedback 更新会停止，避免模型或后端异常时空转；如果确实要关闭这个
护栏，传 `-MaxNoFeedbackRounds 0`。`-MaxTotalTokens 0` 和 `-MaxRuntimeSecs 0`
表示不启用对应预算。

如果希望放到后台跑，用 daemon wrapper。它默认写到
`target\evolution\daemon\`，带 PID 文件、stdout/stderr 日志、重复启动保护，并且默认
预算受限：`MaxTokens=4096`、`MaxTotalTokens=512`、`MaxRuntimeSecs=900`。它会启用
remote-chain gate、pool artifact refresh、stage route gate、pool alignment gate、
state consistency gate 和 experience audit gate；主模型成功后会执行
`summary/router/review/index/test-gate` helper stage，并在预算结束后的 post-run
report gate 中强制要求最新一轮 helper 反馈字段完整、`test-gate` verdict pass、
`test-gate.validation_command` 是安全 cargo 命令、固定 configured cargo validation
已经真实执行通过，以及 model-pool 预算公平证据：

```powershell
.\tools\evolution-loop\daemon-evolution-loop.cmd -Start
```

默认每轮先执行固定 cargo validation，并在 post-run report gate 中要求它通过；当前默认命令是
`cargo test -q --manifest-path tools/evolution-loop/Cargo.toml --target-dir target\evolution-loop-daemon-check`。
如果需要改用上一轮 `test-gate` 给出的安全命令作为真实 validation gate，显式加
`-EnableTestGateValidationRun`；如果是临时排障、确认不要执行本地 cargo 验证，可以加
`-DisableConfiguredValidationRun`。

查看后台状态：

```powershell
.\tools\evolution-loop\daemon-evolution-loop.cmd -Status
```

输出 JSON：

```powershell
.\tools\evolution-loop\daemon-evolution-loop.cmd -JsonStatus
```

如果外部监控只关心 daemon 活动是否健康，可以加 `-FailOnUnhealthy`。它仍然只读，
不会启动进程或发送 prompt；当 `activity.ok=false`，或底层 status readiness 不通过时，
会先打印状态再以非零退出：

```powershell
.\tools\evolution-loop\daemon-evolution-loop.cmd -Status -FailOnUnhealthy
.\tools\evolution-loop\daemon-evolution-loop.cmd -JsonStatus -FailOnUnhealthy
```

如果外部监控还要求当前 daemon 必须已经启用真实 validation 执行，可以再加
`-RequireValidationExecution`。它只检查 `launch_validation.validation_execution_enforced`，
不会启动、停止或重启 daemon；配合 `-FailOnUnhealthy` 时，老启动命令没有
`-EnableConfiguredValidationRun` / `-EnableTestGateValidationRun` 会直接非零退出：

```powershell
.\tools\evolution-loop\daemon-evolution-loop.cmd -JsonStatus -RequireValidationExecution -FailOnUnhealthy
```

如果要回答“当前常驻 daemon 是否真的在跑无人值守自进化闭环”，用严格 profile：

```powershell
.\tools\evolution-loop\daemon-evolution-loop.cmd -JsonStatus -StrictUnattendedEvolution -FailOnUnhealthy
```

这个入口会委托到 `status-evolution-loop.cmd -StrictUnattendedEvolution`，并保留 daemon
wrapper 自己的 `daemon` 摘要。它要求 daemon 活动健康、启动命令启用了 validation
执行、最近一轮有 configured validation/self-improve/helper/test-gate 证据；不会启动、
停止或重启 daemon，也不会给模型发送 prompt。

人工快速查看可以直接用更短的包装命令：

```powershell
.\tools\evolution-loop\strict-status-evolution-loop.cmd
```

它等价于 `status-evolution-loop.cmd -JsonStatus -StrictUnattendedEvolution -FailOnNotReady`，
并默认加 `-SkipProcess` 避免 Windows 进程枚举权限噪声；仍然只读当前 daemon ledger /
logs / health artifacts。

如果要把同一份严格状态留成 artifact，给 UI、监控或其他窗口读取：

```powershell
.\tools\evolution-loop\snapshot-strict-status-evolution-loop.cmd
```

默认写到 `target\evolution\strict-status.json`。也可以把第一个参数作为输出路径，其余参数会
继续透传给底层 strict status，例如：

```powershell
.\tools\evolution-loop\snapshot-strict-status-evolution-loop.cmd target\evolution\strict-status-latest.json -SkipRemoteChain
```

验证已有快照是否仍然新鲜、严格且 ready：

```powershell
.\tools\evolution-loop\verify-strict-status-snapshot.cmd -JsonStatus -FailOnNotReady
```

默认读取 `target\evolution\strict-status.json`，要求文件能解析、`strict_unattended_evolution=true`、
`ledger_source=daemon`、底层 strict status ready，且快照年龄不超过 900 秒。可用
`-SnapshotJson path\to\status.json` 和 `-MaxSnapshotAgeSeconds 60` 调整输入和新鲜度窗口。
JSON 输出里的 `summary` 是给 UI/监控/其他窗口稳定读取的 compact surface，包含
`latest_round`、`active_round`、`daemon_state`、`latest_case`、`self_improve_passed`、
`validation_passed`、`helper_stage_roles`、`helper_stage_contract_complete`、
`test_gate_passed`、`test_gate_validation_command_safety`、`remote_chain_ready` 和
`backend_model` 等关键字段。

如果只想给前端/监控暴露一个小 JSON，而不是完整 strict snapshot：

```powershell
.\tools\evolution-loop\publish-strict-status-summary.cmd -JsonStatus -FailOnNotReady
```

默认读取 `target\evolution\strict-status.json`，写出
`target\evolution\strict-status-summary.json`。该文件只包含 readiness、snapshot 年龄和
`summary` compact surface，适合 UI 高频读取。

生产/调试时更推荐一条命令同时刷新完整快照和小摘要：

```powershell
.\tools\evolution-loop\refresh-strict-status-artifacts.cmd -JsonStatus -FailOnNotReady
```

它会写 `target\evolution\strict-status.json` 和
`target\evolution\strict-status-summary.json`，再输出本次刷新结果；仍然只读 daemon
ledger/logs/health artifacts，不启动进程、不发送 prompt。

消费小摘要前也可以单独验证 summary artifact：

```powershell
.\tools\evolution-loop\verify-strict-status-summary.cmd -JsonStatus -FailOnNotReady
```

它默认读取 `target\evolution\strict-status-summary.json`，要求 summary 新鲜、ready、
self-improve/validation/test-gate 都通过、五个 helper stage 角色齐全、test-gate
validation command safety 为 `safe`。

`-Status` 会优先打印 `operator_summary`，这是一行给人和脚本都容易读的短摘要。例如：

```text
operator_summary=state=active ok=True active_round=62 ledger_round=61 lag=1 stage=generate:start stdout_age=16s ledger_age=189s next_step=wait for current round to finish or inspect log_preview
```

其中 `active_round` 来自 stdout 日志里的当前轮次，`ledger_round` 来自 JSONL ledger
最后一条，`lag=1` 通常表示当前轮正在运行、还没落盘；`lag=0` 表示日志轮次和 ledger
已经对齐。`activity.state` 会把常见状态归类：

给 CLI、主窗口或 Forge adapter 消费时，优先读取 JSON 中的
`daemon.daemon_round_transition_status`，不要从 `operator_summary` 或 stdout 文本里猜。
这是 report-only surface，固定 `read_only=true`、`starts_process=false`、
`sends_prompt=false`；它把 daemon 状态压成稳定字段：

可复用的下游消费 fixture 在
`tools\evolution-loop\fixtures\daemon-round-transition-status-v1.consumer.example.json`；
它覆盖 `normal_in_progress` 和 `round_done_waiting_ledger_commit`，并声明 CLI、Forge、
main-window adapter 不需要读取日志 prose 或 `operator_summary`。

下一轮展示决策的 report-only fixture 在
`tools\evolution-loop\fixtures\next-round-decision-evidence-v1.report.example.json`。
它只消费 live status bundle 里的
`live_status_bundle.daemon.daemon_round_transition_status` 和
`live_status_bundle.report_gate`，输出给 operator/UI 的
`safe-to-wait`、`safe-to-continue-after-current-round` 或
`blocked-operator-attention`。该 surface 固定 `read_only=true`、
`side_effects=false`、`starts_process=false`、`sends_prompt=false`，
不会改变 daemon loop、prompt、report gate stop semantics、runtime calls
或 model pool 行为。

`tools\evolution-loop\fixtures\next-round-decision-report-v1.report.example.json`
覆盖同一组事实的 report adapter 形状。status JSON 会同时发布
`next_round_decision_report_v1` 和
`live_status_bundle.next_round_decision_report_v1`，两者都固定
`schema=next_round_decision_report_v1`，并从现有 `next_round_decision`
证据复制 `display_state`、安全继续布尔值、operator-attention 布尔值、
`reason_code` 和 `evidence`。这是 additive JSON surface，不替换
`next_round_decision`。

`tools\evolution-loop\fixtures\next-round-downstream-status-consumers-v1.report.example.json`
覆盖下游消费者的规范化 report 形状。status JSON 会同时发布
`next_round_downstream_status_consumers_v1` 和
`live_status_bundle.next_round_downstream_status_consumers_v1`，两者都固定
`schema=next_round_downstream_status_consumers_v1`，其 consumer 决策仍只从
`next_round_decision_report_v1` 派生 `service_cli_display_status`、
`forge_operator_display_status`、`agent_assignment_acceptance` 和
`memory_self_improve_admission_visibility`。同一个 projection 还会把现有
`daemon.daemon_round_transition_status` 中的 `active_round`、
`ledger_latest_round` 和 `latest_done_round` 作为 additive `round_id_evidence`
镜像到下游 consumer surface；这些 round id 只用于显示、对账和 adapter 判断，
不参与安全继续决策。该 projection 只表达 consumer facts：不启动 daemon、
不发送 prompt、不调 service/CLI/Forge/agent/memory，不写 `.ndkv`。

当前 strict status JSON 也会直接暴露同一份 additive surface：
`live_status_bundle` 是只读输入 bundle，`next_round_decision` 是 report-only
决策结果，`next_round_decision_report_v1` 是给 service/CLI/Forge adapter
消费的同源 report 形状，`next_round_downstream_status_consumers_v1` 是给
service/CLI display、Forge operator display、agent assignment acceptance 和
memory self-improve admission visibility 的下游 projection。
`next_round_decision.safe_to_wait_current_round_active=true` 表示当前轮
仍在安全运行，operator/UI 应等待本轮结束；`safe_to_continue_after_current_round=true`
表示已看到 done marker、正在等待 ledger commit，当前轮之后可继续；如果
`operator_attention_blocked=true`，说明缺少安全继续证据或 report gate 已失败。
当 `-StrictUnattendedEvolution` 未提供独立 report JSON，但最新 ledger round 已满足
strict unattended 的 configured validation、self-improve、helper-stage contract 和
test-gate pass 要求时，status 会把该只读 ledger evidence 作为
`live_status_bundle.report_gate.source=strict_unattended_ledger_latest` 发布；这只解除
active daemon 正常运行时的误报 operator-attention display，不会改变 daemon loop、
prompt、runtime、model pool 或 report gate stop semantics。
这些字段只用于展示和 adapter 判断，不会启动进程、发送 prompt、写 ledger 或绕过
report gate stop semantics。compact strict summary 会把
`next_round_decision_report_v1`、`next_round_decision_display_state`、
`next_round_downstream_status_consumers_v1`、
`safe_to_wait_current_round_active`、
`safe_to_continue_after_current_round` 和 `operator_attention_blocked` 一起发布。

| JSON path | 用途 |
| --- | --- |
| `daemon.daemon_round_transition_status.schema` | 固定为 `daemon_round_transition_status_v1` |
| `daemon.daemon_round_transition_status.transition_kind` | 机器可读归类：`normal_in_progress`、`round_done_waiting_ledger_commit`、`stale_no_activity`，或其他原始 activity state |
| `daemon.daemon_round_transition_status.activity_reason` | 机器可读原因码，例如 `round_in_progress_stdout_recent`、`stdout_done_marker_seen_waiting_for_ledger_commit`、`round_in_progress_stdout_stale` |
| `daemon.daemon_round_transition_status.active_round` | stdout 中最新 `[round N]` |
| `daemon.daemon_round_transition_status.ledger_latest_round` | daemon ledger 最新 round |
| `daemon.daemon_round_transition_status.ledger_lag_rounds` | `active_round - ledger_latest_round`，缺证据时为 null |
| `daemon.daemon_round_transition_status.latest_round_state` | 原始 round 解析状态：`in_progress`、`round_done_waiting_ledger_commit`、`completed` 或 `unknown` |
| `daemon.daemon_round_transition_status.round_in_progress` | `round_done_waiting_ledger_commit` 必须为 `false`，避免把 done marker 当作普通运行中 |
| `daemon.daemon_round_transition_status.latest_done_round` | stdout 最近 `[round N] done [DONE]` 的 N |
| `daemon.daemon_round_transition_status.stdout_age_seconds` / `ledger_age_seconds` | 判定 stale/no-activity 的文件新鲜度证据 |
| `next_round_downstream_status_consumers_v1.next_round_downstream.round_id_evidence.source_path` | round id 证据来源，当前为 `daemon.daemon_round_transition_status` |
| `next_round_downstream_status_consumers_v1.next_round_downstream.active_round` / `ledger_latest_round` / `latest_done_round` | 从 daemon transition facts 镜像的下游显示 round id；缺少 daemon transition 时保持 null |

同一个 status 还会输出 `launch_validation`。它从 daemon stderr 中最近的启动命令解析
当前进程是否真的启用了验证执行：`mode=none` 表示只要求 test-gate 产出安全命令但未执行；
`mode=configured` 表示启用了固定 `-ValidationCommand` 和 `-RequireConfiguredValidationRun`；
`mode=test-gate` 表示启用了 `-UseTestGateValidationCommand` 和
`-RequireTestGateValidationRun`。这个字段是只读证据，不会启动或停止 daemon。

```text
active              当前轮正在运行，stdout 最近仍在更新
round_done_waiting_ledger_commit stdout 已看到 `[round N] done [DONE]`，但 ledger 最新轮仍小于 N；这是落盘提交前的短暂过渡状态，不再当作普通 in_progress
post_round_activity 最新轮已落盘，正在跑后置 gate/report 或等待下一轮间隔
idle_completed      最新轮已完成且 ledger 已跟上，等待下一轮或预算结束
stale_in_progress   显示有轮次进行中，但 stdout 已超过 300 秒未更新
stale_post_round_activity 后置 gate/report 超过阈值未更新
ledger_lag_after_completion 最新轮显示完成，但 ledger 仍没跟上
not_running         daemon 进程不在
stale_pid           PID 文件还在但进程不存在
```

同时 JSON 里会暴露 `stdout_freshness`、`ledger_freshness` 和 `stderr_freshness`，
包括文件是否存在、最后写入时间和 `age_seconds`。排查“是不是卡住”时先看
`operator_summary`；如果 `state=active`，通常是 12B 主模型还在推理；如果
`state=post_round_activity`，说明最新轮已经写入 ledger，但后置 gate/report 或下一轮间隔
还在推进；如果 `state=stale_in_progress` 或 `state=stale_post_round_activity`，再看
`log_preview`、后端 health 和远程模型 worker。

停止后台进程：

```powershell
.\tools\evolution-loop\daemon-evolution-loop.cmd -Stop
```

`-Stop` 会先枚举 PID 文件指向进程的子孙进程，先停子孙再停父进程，避免只杀掉
外层 PowerShell、留下 `evolution-loop.exe` 继续运行并锁住 `target\debug\evolution-loop.exe`。
如果 PID 文件已经过期，`-Stop` 还会按 daemon ledger 路径查找残留的本地
`cargo.exe` / `evolution-loop.exe` 命令并清理；`-Stop -CheckOnly` 会先打印 orphan
数量和 PID 列表。
要先预览将被停止的进程树而不实际停止：

```powershell
.\tools\evolution-loop\daemon-evolution-loop.cmd -Stop -CheckOnly
```

要先确认它会启动什么命令但不实际启动，用 `-CheckOnly`：

```powershell
.\tools\evolution-loop\daemon-evolution-loop.cmd -Start -CheckOnly -MaxTokens 64 -MaxTotalTokens 512 -MaxRuntimeSecs 900
```

即使当前 daemon 已经在跑，`-Start -CheckOnly` 也会继续打印下一次受控启动的命令预览，
并额外输出 `existing_running=True` / `existing_pid=...`；它仍然不会启动第二个进程。
这适合在真正重启前确认命令里已经包含 `-ValidationCommand` 和
`-RequireConfiguredValidationRun`。

`daemon-evolution-loop` 本身不会绕过预算。到达 token/runtime/failure/no-feedback
上限后，底层 loop 会退出，并自动生成带 gate 的 `report.json`；status 中
`daemon.running=false` 且 ledger/report ready，表示本次后台自进化已经按预算完成。
如果 PID 文件还在但进程已退出，status 会显示 `stale_pid_file=true`，并从日志里暴露
`last_stop_reason`、`stdout_tail` 和 `stderr_tail`，便于区分正常预算停止和异常退出。

如果要让夜跑持续推进，而不是每次 `MaxRuntimeSecs` 到点后人工重启，可以在外层跑
supervisor。它不改变 daemon 的预算，也不绕过 report gate / validation gate /
remote-chain gate；它只是周期性读取 strict status，只有当 daemon 不在或 PID 过期时，
才用同一套 strict daemon 启动参数重新拉起下一段预算：

```powershell
.\tools\evolution-loop\supervise-unattended-evolution.cmd -PollSecs 60 -MaxTotalTokens 2048 -MaxRuntimeSecs 3600
```

先看将要执行的只读预览：

```powershell
.\tools\evolution-loop\supervise-unattended-evolution.cmd -CheckOnly -Once -MaxTotalTokens 2048 -MaxRuntimeSecs 3600
```

`-CheckOnly` 会打印 strict status 命令和 daemon start 命令，但不会启动进程、不会发送
prompt，也不会触碰远程模型；真实 supervisor 是前台循环，适合放在一个专门终端里看护
daemon。

supervisor 自己也有只读状态和安全停止入口：

```powershell
.\tools\evolution-loop\supervise-unattended-evolution.cmd -Status
.\tools\evolution-loop\supervise-unattended-evolution.cmd -Stop -CheckOnly
.\tools\evolution-loop\supervise-unattended-evolution.cmd -Stop
```

`-Status` 只读 `target\evolution\daemon\supervisor.pid`、`supervisor.out.log`
和 `supervisor.err.log`，输出 `supervisor_running`、`supervisor_pid`、
`supervisor_stale_pid_file` 和最后一行 stdout/stderr；不会启动 daemon，也不会给模型发
prompt。`-Stop -CheckOnly` 只打印将停止的 PID；真实 `-Stop` 只停止 supervisor，
不直接停止底层 daemon。

长期无人值守建议开启本地 state consistency gate。它不连接 Gemma，只在每轮前检查
ledger 的 round 是否重复、倒退、缺少有效 round 或中间跳号；失败会在推理前停止，避免
把脏账本继续喂给下一轮：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Forever -IntervalSecs 30 -MaxFailures 3 -MaxTokens 512 -MaxTotalTokens 20000 -MaxRuntimeSecs 3600 -StateConsistencyGate
```

查看常驻/最近运行状态用只读 status 命令。它只读 ledger、可选 report JSON、本机
`/health` 和已有 remote-chain status artifact；不会启动进程，也不会给模型发送 prompt：

```powershell
.\tools\evolution-loop\status-evolution-loop.cmd -Ledger target\evolution\evolution-ledger.jsonl -StrictLedgerHygiene
```

如果需要给前端、脚本或 CI 消费，输出机器可读 JSON：

```powershell
.\tools\evolution-loop\status-evolution-loop.cmd -Ledger target\evolution\evolution-ledger.jsonl -ReportJson target\evolution\report.json -StrictLedgerHygiene -JsonStatus
```

status 会显示 `read_only=true`、`starts_process=false`、`sends_prompt=false`，并汇总
后台进程是否存在、最新 round 是否成功、feedback 总量、ledger hygiene、后端 health
和远程模型池 readiness。`process.running=false` 只代表当前没有常驻 loop 进程；如果
最近 ledger/report 是 ready，说明链路可继续用受预算的 `-Forever` 命令启动。
默认还会只读 `target\evolution\daemon\` 下的 PID、stdout 和 daemon ledger，打印
`daemon: ... summary=state=... active_round=... ledger_round=... lag=...`。如果当前
只想检查某个实验 ledger，不想读取后台 daemon 状态，加 `-SkipDaemon`；如果后台工作目录
不是默认值，用 `-DaemonWorkDir path\to\daemon-dir`。
如果你关心的是“当前常驻 daemon 是否在可靠自进化”，加 `-UseDaemonLedger`，它会把
顶层 ledger 摘要切到 `DaemonWorkDir\evolution-ledger.jsonl`，避免旧实验 ledger 的脏记录
污染当前 daemon readiness。
监控或 CI 希望 daemon 不健康时直接让 status fail，可以加 `-RequireDaemonHealthy`；
当 daemon 未运行、PID 文件过期、或 `activity.state=stale_in_progress` /
`stale_post_round_activity` 时，
`readiness.failures` 会包含 `daemon_not_healthy`。
默认 `stale_in_progress` 使用 300 秒 stdout 无更新作为阈值；如果某些轮次已知会更慢，
可以用 `-MaxDaemonInProgressStdoutAgeSeconds 900` 放宽。反过来，如果你想监控
“daemon 还在、最近一轮已完成、但迟迟没有进入下一轮”的假空转，可以设置
`-MaxDaemonIdleLedgerAgeSeconds 900`；默认值是 0，表示不对 idle ledger 年龄做额外
阻断。触发时 `activity.state=stale_idle_completed`，并同样通过
`daemon_not_healthy` 让 `-FailOnNotReady` 非零退出。
如果还要证明当前 daemon 启动命令已经启用真实 validation 执行，再加
`-RequireDaemonValidationExecution`。它会只读 daemon stderr 里的启动命令，解析
`launch_validation.mode` 和 `launch_validation.validation_execution_enforced`；
如果旧 daemon 只带 `-RequireTestGatePass`、但没有
`-EnableConfiguredValidationRun` / `-EnableTestGateValidationRun` 对应的执行 gate，
`readiness.failures` 会包含 `daemon_validation_execution_missing`。这个检查不会启动、
停止或重启 daemon，只用于把“模型在跑”和“验证闭环在跑”区分开。
如果还要证明最近已完成 round 的 ledger 里已经落下真实 configured validation 证据，
再加 `-RequireLatestConfiguredValidationRun`。它要求最近一条记录满足
`validation_checked=true`、`validation_passed=true`、
`validation_command_source=configured` 和 `validation_status_code=0`；失败时
`readiness.failures` 会包含 `latest_configured_validation_missing`。
如果还要证明最近已完成 round 确实进入自进化闭环，再加
`-RequireLatestSelfImprove`。它要求最近一条记录 `success=true`、
`feedback_applied>0` 且 `self_improve_passed=true`；失败时
`readiness.failures` 会包含 `latest_self_improve_missing`。这个检查只读 ledger，
不启动模型、不发送 prompt，用来区分“daemon 活着”和“最近一轮真的有反馈/自改进证据”。
如果还要证明小模型池的 helper stages 也参与了最近一轮，再加
`-RequireLatestHelperStageRoles summary,router,review,index,test-gate`。它会读取最近
ledger 里的 `helper_stage_feedback_by_role`，要求指定角色都有非空反馈；失败时
`readiness.failures` 会包含 `latest_helper_stage_roles_missing`。JSON status 还会暴露
`ledger.latest.helper_stage_roles`、`helper_stage_role_count`、
`helper_stage_feedback_total` 和 `helper_stage_contract_roles`。
如果还要证明这些 helper 回复满足结构化 contract，再加
`-RequireLatestHelperStageContracts`。它会检查每个角色的 `expected_markers` 是否都在
`matched_markers` 中；失败时 `readiness.failures` 会包含
`latest_helper_stage_contract_incomplete`，并在 JSON status 里暴露
`helper_stage_contract_incomplete_roles`。
如果还要证明最近的 `test-gate` helper 真正放行，再加 `-RequireLatestTestGatePass`；
它要求 `test_gate_verdict=pass`，失败时报告 `latest_test_gate_not_pass`。如果还要
证明 `test-gate.validation_command` 是保守安全的 cargo 验证命令，再加
`-RequireLatestSafeTestGateValidationCommand`；失败时报告
`latest_test_gate_validation_command_not_safe`。这两个检查都只读 ledger，不执行命令。
如果外部监控需要用进程退出码判断 readiness，再加 `-FailOnNotReady`。默认 status
即使 `readiness.ready=false` 也会退出 0，方便人工查看；开启 `-FailOnNotReady`
后会先完整打印 text/JSON 状态，再在 not-ready 时以非零退出：

```powershell
.\tools\evolution-loop\status-evolution-loop.cmd -RequireDaemonHealthy -FailOnNotReady
.\tools\evolution-loop\status-evolution-loop.cmd -JsonStatus -RequireDaemonHealthy -FailOnNotReady
.\tools\evolution-loop\status-evolution-loop.cmd -JsonStatus -StrictUnattendedEvolution -FailOnNotReady
.\tools\evolution-loop\status-evolution-loop.cmd -JsonStatus -UseDaemonLedger -RequireDaemonHealthy -MaxDaemonInProgressStdoutAgeSeconds 900 -MaxDaemonIdleLedgerAgeSeconds 900 -FailOnNotReady
```

`-StrictUnattendedEvolution` 是当前无人值守自进化的一键验收 profile。它会隐式使用
daemon ledger，并要求 daemon 健康、daemon 启动命令启用了 validation 执行、最近一轮
有 configured validation 证据、有 self-improve 证据、有
`summary,router,review,index,test-gate` 五类 helper 反馈、有完整 helper contract、且
`test-gate` verdict 为 `pass`、`test-gate.validation_command` 是保守安全的 cargo 验证命令。
它还会在未显式指定 freshness 参数时，把 in-progress stdout 和 idle ledger 的阈值都设为
900 秒，适配本地 12B/远程 Metal 推理的较长单轮耗时。JSON status 会暴露
`strict_unattended_evolution=true` 以及实际采用的 daemon freshness 阈值。

严格 gate 验收：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 3 -BusinessGate
```

## 经验库和索引质量 gate

长期自进化前可以开启只读 cleanup audit gate。它会在每轮发给 Gemma 之前调用
`/v1/experience-cleanup-audit`，检查经验库隔离候选、旧 metadata 可修复项、索引噪声
和最大噪声惩罚；不写经验库，不做 repair/quarantine：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 3 -ExperienceAuditGate
```

默认阈值是最严格的：`MaxIndexNoisyRecords=0`、`MaxIndexNoisePenalty=0.0`、
`MaxQuarantineCandidates=0`、`MaxRepairableLegacyRecords=0`、
`MaxLegacyMetadataWithoutCleanGist=0`。如果是在旧 ledger 或旧经验库上做迁移验证，可以
先放宽阈值，等 cleanup/repair 完成后再收紧：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 3 -ExperienceAuditGate -MaxIndexNoisyRecords 2 -MaxIndexNoisePenalty 0.2
```

## 只读报告

查看当前 ledger 摘要，不连接后端、不触发 Gemma：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Report
```

报告会显示总轮数、成功率、流式截断/缺 final 失败次数、token/耗时均值、feedback
总量、validation 通过数、self-improve 通过数、state/trace gate 通过数、最近一轮和
最近失败原因。

需要留下机器可读 artifact：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Report -ReportJson target\evolution\report.json
```

如果同时传入 `-PoolStatusJson target\evolution\pool-status.json`，报告 JSON 会额外包含
`model_pool` 摘要，便于夜跑复盘时区分“模型池未启动/quality worker 不可达”和真正的
业务循环质量问题。摘要会保留 worker 角色状态，例如 `quality:unreachable`、
`summary:healthy`、`review:tcp_only`，并在 prompt context 中暴露 `available_roles`
和 `blocked_roles`，让后续自进化轮次知道哪些模型岗位可以并行使用。
如果 status artifact 带有 `capacity`，报告和 prompt context 也会保留
`expansion_allowed`、`recommendation`、helper worker 数和 runtime 计数；夜跑看到
`restore_quality_gate_first` 或 `verify_worker_runtime_metadata_before_expansion`
时，应先修链路或补齐 Metal/GPU 证据，而不是继续增加 worker。
如果同时传入 `-PoolManifestJson target\evolution\pool-manifest.json`，报告和 prompt
context 会保留 manifest 的 `capacity_policy` 和 `advice`。`-RefreshPoolArtifacts`
刷新 `/v1/model-pool/manifest` 时会要求 manifest 带有 `advice.decision_source=
model-pool-advice-core`、`safe_to_enable_pool_workers`、`next_step`、`reason` 和
`extra_quality_12b_detected`，以及 `worker_shape.quality/helpers_visible/helper_target`；
缺少这些字段会在发送 prompt 前失败。配合
`-PoolAlignmentGate` 时，如果 manifest advice 明确阻断扩容、检测到额外 12B，
或 `worker_shape` 与实际 workers 不一致，
alignment gate 也会失败，防止自进化闭环继续挤占苹果机资源。
如果同时传入 `-PoolRouteJson target\evolution\pool-route-review.json`，报告 JSON 会额外
包含 `model_pool_route`，用于复盘某类任务是否已经有可用 worker，或只是被
`model_pool_launch` gate 正确阻断。它会保留 `selected_worker` 和
`candidate_workers[]` 的端口、`base_url`、ready 状态、context window、
默认 max tokens，方便后续 worker lease / queue / backpressure router 直接消费。
如果同时传入 `-PoolBudgetFairnessJson target\evolution\model-pool-budget-fairness.json`，
报告 JSON 会额外包含 `model_pool_budget_fairness_report_v1`，用 worker 事件判断
summary/review/test-gate 是否都有成功反馈、是否某个角色吃掉过多 token，以及辅助任务
是否阻塞了主 12B 路径。每条 `model_worker_v1` 事件和 role 汇总也会保留
`runtime_backend`、`runtime_device`、`runtime_accelerator`、`gpu_layers`，以及
`default_max_tokens` / `configured_max_tokens` / `effective_max_tokens` /
`max_tokens_clamped` 预算证据，方便复盘某轮是不是疑似 CPU fallback 或 helper 预算
是否被正确夹紧。配合 `-ReportGate` 时，这个 artifact 不再只是上下文：
如果 `budget_fairness_blocked=true`，report gate 会失败，防止夜跑继续扩容一个会挤占
quality 12B 的模型池。要把“一个 quality 12B 保留完整预算、小 helper 被限额”变成硬性验收，
再加 `-RequirePoolBudgetPolicy`；缺少 `model_worker_v1` artifact、缺少 quality 证据、
quality 被 clamp，或没有低优先级 helper clamp 证据都会失败。
如果要在运行期、每轮 prompt 前就阻断，请用上一节的 `-PoolBudgetFairnessGate`；
`-ReportGate` 是不连接后端的事后验收。

给 Web Lab、daemon 或 Forge 一类 adapter 消费时，优先读 report JSON 的纯数据
bundle，不要解析 helper prose：

| JSON path | 用途 |
| --- | --- |
| `adapter_closure_bundle_report_v1.schema` | 固定为 `adapter_closure_bundle_report_v1`，用于识别 adapter closure bundle surface |
| `adapter_closure_bundle_report_v1.consumer_surface` | 固定为 `adapter_closure_unattended_continuation`，表示给 adapter closure / unattended continuation 消费 |
| `adapter_closure_bundle_report_v1.source_report_keys[]` | 指向同一份 report JSON 中仍保持独立 contract 的 `ledger_gate_report_v1`、`strict_report_gate`、`continuation_gate_report_v1`、`validation_command_coverage_report_v1` 和 `report_gate` |
| `adapter_closure_bundle_report_v1.consumer_decision.*` | adapter 可直接读取的 closure/continuation 决策：report gate 是否通过、latest round 是否成功、是否允许 unattended continuation |
| `adapter_closure_bundle_report_v1.closure_evidence.*` | 不依赖 helper 文本的轮次、validation、self-improve、state/trace gate、runtime/stream 失败计数 |
| `adapter_closure_bundle_report_v1.validation_command_coverage.*` | coverage strict 请求与 tooling/report evidence 计数，用来对应 `validation_command_coverage_report_v1` |
| `adapter_closure_bundle_report_v1.adapter_surfaces.*` | helper/test-gate 只暴露 role 名、verdict 和 validation command safety，不暴露 helper prose |

R21 之后，如果主窗口或 UI 需要判断某个 Codex worker window 是否已经被污染、暂停，或是否
必须由 clean-room replacement 接管，可以把只读 worker-window status fixture 传给 report：

```powershell
cargo run --manifest-path .\tools\evolution-loop\Cargo.toml -- --report --ledger target\evolution\evolution-ledger.jsonl --worker-window-status-json docs\runbooks\smartsteam-worker-window-status-r21.example.json --report-json target\evolution\report.json
```

report JSON 会新增 `worker_window_replacement_report_v1`，这是 report-only surface，
不会 start/stop daemon、不会触碰远端模型池、不会发送 prompt 或 stream，也不会自动创建
clean-room replacement。推荐消费者读取这些字段：

| JSON path | 用途 |
| --- | --- |
| `worker_window_replacement_report_v1.schema` | 固定为 `worker_window_replacement_report_v1` |
| `worker_window_replacement_report_v1.status_loaded` | 是否加载了外部 worker-window status JSON |
| `worker_window_replacement_report_v1.source_status.windows[]` | R21 clean-room window contract 的只读行；包含 `paused`、`polluted`、`clean-room-replacement`、`clean_room_replacement_required` 和 `assignment_allowed` |
| `worker_window_replacement_report_v1.evidence_map.*` | 机器可读计数：paused、polluted、replacement、required replacement、blocked original |
| `worker_window_replacement_report_v1.side_effects.*` | 必须保持全 false，用来证明 report 消费不会启动进程、变更 worker 状态、触碰远端或发送 prompt |

R23/R24 之后，evolution-loop report 也可以消费 memory startup admission status
和 agent clean-room replacement plan fixture，把它们闭环成同一个 report-only
section。这个路径仍然只读外部 JSON，不会 expand memory admission、不会写 `.ndkv`，
不会创建 replacement thread、不会发送 message/prompt，也不会改变 daemon loop 或 report
gate：

```powershell
cargo run --manifest-path .\tools\evolution-loop\Cargo.toml -- --report --ledger target\evolution\evolution-ledger.jsonl --memory-startup-admission-json docs\runbooks\smartsteam-evolution-loop-memory-admission-r23.example.json --agent-clean-room-replacement-plan-json docs\runbooks\smartsteam-evolution-loop-agent-replacement-plan-r23.example.json --report-json target\evolution\report.json
```

report JSON 会新增 `clean_room_handoff_report_v1`：

| JSON path | 用途 |
| --- | --- |
| `clean_room_handoff_report_v1.schema` | 固定为 `clean_room_handoff_report_v1` |
| `clean_room_handoff_report_v1.memory_startup_admission.loaded` | 是否加载了外部 memory startup admission/status fixture |
| `clean_room_handoff_report_v1.memory_startup_admission.evidence_map.*` | 机器可读 admission/index/context-rot 计数，以及 `store_mutation_count`、`ndkv_write_allowed`、`admission_expanded_by_non_contract_evidence` |
| `clean_room_handoff_report_v1.agent_clean_room_replacement_plan.loaded` | 是否加载了外部 agent clean-room replacement plan fixture |
| `clean_room_handoff_report_v1.agent_clean_room_replacement_plan.evidence_map.*` | 机器可读 replacement plan 布尔位和 replacement prompt 的 task/evidence/reason 计数；prompt 只携带 ids/codes |
| `clean_room_handoff_report_v1.side_effects.*` | 必须保持全 false，证明 report 消费不会启动/停止 daemon、不会触碰远端、不会发送 prompt/message、不会改 worker 状态、不会扩展 memory admission 或写 `.ndkv` |

R25 之后，如果主窗口需要把 R24 已完成、R25 clean-room replacement 已打开、旧污染窗口
不可继续分配，以及 SSH/runtime/daemon/remote model pool 仍归主窗口所有这几个状态闭成
同一个 report-only 证据，可以传入 clean-room batch status fixture：

```powershell
cargo run --manifest-path .\tools\evolution-loop\Cargo.toml -- --report --ledger target\evolution\evolution-ledger.jsonl --clean-room-batch-status-json docs\runbooks\smartsteam-evolution-loop-clean-room-batch-status-r25.example.json --report-json target\evolution\report.json
```

report JSON 会新增 `clean_room_batch_status_report_v1`：

| JSON path | 用途 |
| --- | --- |
| `clean_room_batch_status_report_v1.schema` | 固定为 `clean_room_batch_status_report_v1` |
| `clean_room_batch_status_report_v1.status_loaded` | 是否加载了外部 clean-room batch status fixture |
| `clean_room_batch_status_report_v1.evidence_map.r24_completed` | R24 clean-room batch 是否已完成 |
| `clean_room_batch_status_report_v1.evidence_map.r25_clean_room_replacements_open` | R25 clean-room replacement 是否已打开 |
| `clean_room_batch_status_report_v1.evidence_map.old_polluted_windows_blocked` | 旧污染/暂停/陈旧窗口是否禁止继续分配 |
| `clean_room_batch_status_report_v1.evidence_map.main_window_runtime_owner` | SSH、daemon、runtime start/stop 和 remote model pool 是否仍由主窗口拥有 |
| `clean_room_batch_status_report_v1.evidence_map.worker_runtime_ownership_allowed` | 必须为 `false`，表示 worker 不拥有 runtime/SSH/start-stop |
| `clean_room_batch_status_report_v1.side_effects.*` | 必须保持全 false，证明 report 消费不会创建线程、发送消息、读旧线程、启动/停止 daemon/Forge/Web Lab、触碰远端或发送 prompt/stream |

R26 之后，report JSON 还会从 ledger round 和 `final_preview` 中投影
`self_improve_proposal_artifact_v1`。它是 report-only 的候选 artifact，用来把
self-improve 建议变成机器可读但不自动执行的行动项；缺少显式 final proposal 时，会退回
到最新 helper contract 里的 `review.change_request` 和 `test-gate.validation_command`
生成候选。这个 section 不改变 daemon loop、prompt、report gate stop semantics、remote
model pool，也不会改代码：

| JSON path | 用途 |
| --- | --- |
| `self_improve_proposal_artifact_v1.schema` | 固定为 `self_improve_proposal_artifact_v1` |
| `self_improve_proposal_artifact_v1.candidate_only` | 固定为 `true`，表示只是候选，不是已执行变更 |
| `self_improve_proposal_artifact_v1.proposals[].proposal_id` | proposal 的稳定 id；显式 final JSON 缺失时由 round/action 派生 |
| `self_improve_proposal_artifact_v1.proposals[].source_round` | 产生该候选的 ledger round |
| `self_improve_proposal_artifact_v1.proposals[].evidence_id` | 指向 final JSON 或 helper contract 字段的证据 id |
| `self_improve_proposal_artifact_v1.proposals[].suggested_action` | 建议行动，通常来自 explicit proposal 或 `review.change_request` |
| `self_improve_proposal_artifact_v1.proposals[].validation.*` | 建议验证命令、来源和保守安全分类；report 只记录，不执行 |
| `self_improve_proposal_artifact_v1.proposals[].admission.status` | 固定为 `candidate_report_only`；`auto_apply=false` |
| `self_improve_proposal_artifact_v1.side_effects.*` | 必须保持全 false，证明该 artifact 不写文件、不改 ledger/memory、不启动/停止 runtime、不触碰远端、不发送 prompt/stream |

R28 之后，report JSON 还会从最新 ledger round 的 helper contract 字段投影
`helper_stage_repair_status_report_v1`。它只描述不完整 helper role 的修复候选，
不会重新调用 helper stage、不会发送 prompt、不会修改 daemon 行为，也不会自动应用代码：

| JSON path | 用途 |
| --- | --- |
| `helper_stage_repair_status_report_v1.schema` | 固定为 `helper_stage_repair_status_report_v1` |
| `helper_stage_repair_status_report_v1.report_only` | 固定为 `true`，表示只读报告 surface |
| `helper_stage_repair_status_report_v1.repair_required` | 最新 helper contract 是否有缺失字段或占位字段 |
| `helper_stage_repair_status_report_v1.proposals[].role` | 需要修复的 helper role，例如 `review` 或 `test-gate` |
| `helper_stage_repair_status_report_v1.proposals[].target_role` | Same target helper role as `role`; added for repair consumers that key on explicit target-role fields |
| `helper_stage_repair_status_report_v1.proposals[].missing_role` | `true` when a `-RequireLatestHelperStageRoles` role is absent from latest helper evidence; still report-only |
| `helper_stage_repair_status_report_v1.proposals[].evidence_id` | Stable latest-round evidence pointer; missing required/latest roles use `required_latest_helper_stage_roles.<role>.missing` |
| `helper_stage_repair_status_report_v1.proposals[].missing_fields` | 按 role contract 规则缺失的必需字段；`test-gate` 在 `pass` 时不要求 `failure_kind` |
| `helper_stage_repair_status_report_v1.proposals[].placeholder_fields` | 已出现但仍是模板/占位内容的字段 |
| `helper_stage_repair_status_report_v1.proposals[].suggested_action` | 下一次 helper 输出应补齐的具体字段说明 |
| `helper_stage_repair_status_report_v1.proposals[].admission.status` | 固定为 `repair_proposal_report_only`；`auto_apply=false` |
| `helper_stage_repair_status_report_v1.side_effects.*` | 必须保持全 false，证明该 surface 不写文件、不改 ledger/memory、不启动/停止 runtime、不触碰远端、不发送 prompt/stream |

需要让夜跑或 CI 用退出码判断是否达标时，用 report gate：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -ReportGate -MinReportRounds 3 -MinSuccessRate 60 -MinFeedbackTotal 1
```

`-ReportGate` 不连接后端、不触发 Gemma。默认要求 ledger 非空、最近一轮成功、
`feedback_applied` 总量至少为 1，并且流式截断和缺 `final` 事件次数都为 0；
如果同时传入 `-RemoteChainStatusJson -RemoteChainGate`，还要求远程模型链路
`readiness.ready=true`；如果同时传入 `-PoolBudgetFairnessJson`，还要求模型池预算公平性没有阻断扩容。
如果同时传入 `-RequirePoolBudgetPolicy`，还要求这份 artifact 明确证明 quality 预算被保留，
并且 helper 低优先级调用有 clamp 证据。
如果同时传入 `-RequireHelperStageRoles summary,review,test-gate`，还要求 ledger 中已经
存在这些 helper role 的真实 `pool_stage_call_answer` 反馈；只读 route plan 和 skipped
stage 不会通过这个验收。
如果同时传入 `-RequireLatestHelperStageRoles summary,review,test-gate`，还要求最新一轮
也有这些 helper role 的真实反馈，避免历史成功记录掩盖当前模型池断流或跳过 helper。
如果同时传入 `-RequireTestGatePass`，还要求最近的 `test-gate` helper verdict 明确为
`pass`，让模型池里的测试门禁可以阻断下一轮无人值守演进。
如果同时传入 `-RequireSafeTestGateValidationCommand`，还要求最近的 `test-gate`
建议命令安全分类为 `safe`，防止把不可信 shell 片段推进到后续验证链路。
如果同时传入 `-RequireTestGateValidationRun`，还要求最近一轮确实使用来源为
`test-gate` 的安全验证命令，并且本地 validation gate 已经执行成功。
如果同时传入 `-RequireConfiguredValidationRun`，还要求最近一轮确实使用来源为
`configured` 的验证命令，并且本地 validation gate 已经执行成功；这适合守护进程固定
cargo 验证命令，不会和 test-gate 来源门禁混在一起。
`-AllowLastFailure` 可以取消“最近一轮必须成功”。如果是在迁移旧 ledger、已确认历史
失败不会污染当前判断，可以显式放宽：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -ReportGate -MaxStreamTruncations 1 -MaxMissingFinal 1
```

报告还会输出 `ledger_hygiene`，包括唯一 round 数、重复 round、非递增 round、缺少
有效 round 的记录数，以及 round 序列跳号数。已有历史 ledger 可以继续报告；夜跑或 CI
要求账本更干净时，开启严格卫生 gate：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -ReportGate -MinReportRounds 3 -MinSuccessRate 60 -MinFeedbackTotal 1 -StrictLedgerHygiene
```

如果本轮目标包含 Rust 代码验证，可以把 Rust 编译反馈也纳入验收：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -ReportGate -MinReportRounds 3 -MinRustChecks 1 -MinRustFeedbackTotal 1
```

## 自定义任务池

把每轮 prompt 写到文本文件，一行一个任务，空行和 `#` 开头的行会跳过：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -PromptFile .\target\evolution\prompts.txt -Rounds 20
```

也可以只跑一个固定 prompt：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Prompt "检查 SmartSteam Forge 当前联调链路，给出一个可验证改进。" -Rounds 3
```

## Rust 编译反馈

如果本轮任务要验证 Rust 代码，可以把代码片段附到 business-cycle 请求里。后端会调用
Rust check，并把编译通过/失败转成反馈和 replay 信号：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 3 -RustCheckFile .\target\evolution\candidate.rs -RustCheckEdition 2024
```

也可以传 inline code：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -RustCheckCode "pub fn ok() -> bool { true }" -RustCheckCase evolution-rust-check
```

ledger/report 会记录 `rust_check_checked`、`rust_check_passed` 和
`rust_check_feedback_applied`，用于判断编译反馈有没有进入自改进闭环。

## 本地验证 gate

`Rust 编译反馈` 会把代码片段交给后端业务循环，用于让模型吸收编译反馈；`本地验证 gate`
则直接在本机运行命令，用退出码约束无人值守循环。命令失败或超时会停止本轮，避免在
当前仓库已经不通过测试时继续消耗 Gemma 推理：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 3 -StateConsistencyGate -ValidationCommand "cargo test --manifest-path .\tools\evolution-loop\Cargo.toml" -ValidationTimeoutSecs 300
```

默认在每轮推理前运行。需要同时在推理后再跑一次，可以用 `-ValidationPhase both`：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 3 -ValidationCommand "cargo test --manifest-path .\tools\evolution-loop\Cargo.toml" -ValidationPhase both
```

ledger/report 会记录 `validation_checked` 和 `validation_passed`，用于证明某轮是否真的被
本地命令 gate 约束过。

## 什么时候算达到自进化

短期可达成的是工程闭环自进化：

- 模型持续完成业务循环，而不是一次性聊天。
- 每轮生成都会进入 feedback、self-improve、save_state、gate。
- `noiron` 的 memory、experience、adaptive state 和 evolution ledger 持续增长。
- 失败轮次会被记录，连续失败会自动停止，ledger 会结构化记录 token、feedback、
  self-improve、state/trace gate 结果，避免硬件无意义空转。
- 后续轮次能从前面的经验、反馈和回放里获益。

还没有做的是权重级自训练。那需要单独的数据集、评测集、微调/蒸馏任务、回滚策略和
硬件预算，不能直接在在线业务循环里静默改 Gemma 权重。

## 输出预算和上下文窗口

`-MaxTokens` 是每次请求最多生成多少 token，不等于模型上下文窗口。远端启动脚本里的
`-ContextTokens` 才是 llama-server 的上下文窗口。日常建议：

```powershell
-ContextTokens 8192 -DefaultMaxTokens 4096
```

Gemma 原生窗口可以很大，但直接把上下文窗口或输出预算拉到 262144 会明显增加内存、
KV cache 和断流风险。需要压测时再显式运行：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789 -RestartRemote -ContextTokens 262144 -DefaultMaxTokens 8192
```

`evolution-loop` 的 health gate 会读取主服务 `/health` 里的
`gemma_runtime_context_window`。如果使用 `-MinRuntimeContext 262144`，当远端
llama-server 实际仍是 `n_ctx=4096` 或 health 没有返回 runtime 元信息时，循环会在
发起业务轮次前停止，避免把长上下文实验误跑成短窗口实验。

远端模型或 SSH tunnel 刚恢复时，`/health` 的 metadata 探针可能短暂返回 timeout。
`evolution-loop` 会对这类 metadata 抖动做 6 次短重试；如果最终仍拿不到
`gemma_runtime_context_window`，才会停止。真实的低上下文、safe-device 失败、
quarantine/repair 候选和超出索引噪声阈值仍然会直接阻断。

当前远端 Gemma 12B 完整窗口链路的保守 smoke 命令：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 1 -MaxTokens 256 -SelfImproveLimit 1 -MinRuntimeContext 262144 -ExperienceAuditGate -MaxIndexNoisyRecords 1 -MaxIndexNoisePenalty 0.2
```

跑完后用 report gate 验 ledger：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -ReportGate -MinReportRounds 1 -MinFeedbackTotal 1 -StrictLedgerHygiene
```

## 验证

```powershell
cd D:\rust-norion
cargo test --manifest-path .\tools\evolution-loop\Cargo.toml
.\tools\evolution-loop\test-evolution-loop-launcher.cmd
```
