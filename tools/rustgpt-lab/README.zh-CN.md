# rustgpt-lab 外围流式测试工具

这个目录是独立的测试工具，不属于 `rust-norion` 主程序。它的作用是提供一个类似 RustGPT 的 Web 对话体验，用来测试本机 `rust-norion`/Gemma 12B 后端效果。

## 为什么单独放在外围

用户要求前后分离，所以这里不修改根 `Cargo.toml`，不把 UI、代理服务或 RustGPT 风格页面写进 `src/`。耦合方式只有 HTTP：

浏览器前端 -> `rustgpt-lab` 流式代理 -> `http://127.0.0.1:7878/v1/chat`

也可以在 UI 中切换到 `/v1/business-cycle`，用同一个外围代理测试生成、反馈、Rust 检查、自改进和 gate 报告的业务联调链路。

这样主程序仍然是模型服务，`tools/rustgpt-lab` 只是临时测试外壳。

## 参考的 RustGPT 信息

上游项目：[bitswired/rustgpt](https://github.com/bitswired/rustgpt)

中文概述：

RustGPT 是一个用 Rust 和 HTMX 构建的 ChatGPT 风格 Web 应用。它使用 Axum 作为 Web 框架，SQLite/SQLx 做持久化，Tera 渲染 HTML 模板，并使用 Server-Sent Events 提供实时流式交互。它的目标是展示 Rust 在 Web 聊天产品里的后端、模板、数据库和实时响应能力。

技术栈翻译：

- Axum：Rust Web 框架，用来处理 HTTP 路由和服务端逻辑。
- HTMX：通过 HTML 属性发起交互，减少前端 JavaScript 框架依赖。
- SQLite/SQLx：轻量数据库和类型安全 SQL 工具。
- Tera：类似 Jinja2 的 HTML 模板引擎。
- SSE：服务端事件流，用来把回答增量推到页面上。

许可证注意：

RustGPT 使用 AGPL-3.0。为了避免把 AGPL 源码混入当前项目，本目录没有复制 RustGPT 源码，只参考它的产品形态和公开 README/Cargo 元信息，重新实现一个很小的测试代理和页面。

## 两条安全启动路径

### A. built-in 后端流式联调，不启动 Gemma

这条路径用于先验证 Web Lab 前后端分离、SSE 流式代理、`/health` 状态栏和 `/v1/*-stream` 路由。它不启动 Gemma 12B，也不需要 `mistralrs`。默认把状态放到隔离目录 `target\manual-web-lab-service\built-in-lab-state`，不会写项目根目录的 `noiron-experience.ndkv`。

推荐一条命令启动 built-in `rust-norion` 后端和 Web Lab UI：

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\start-built-in-lab.cmd
```

启动前只做静态检查，不构建、不启动服务、不打开浏览器、不写 `.ndkv`：

```powershell
.\tools\rustgpt-lab\start-built-in-lab.cmd -CheckOnly
```

常用参数：

```powershell
.\tools\rustgpt-lab\start-built-in-lab.cmd -NoOpen -SkipBuild
.\tools\rustgpt-lab\start-built-in-lab.cmd -StateDir target\manual-web-lab-service\built-in-lab-state
.\tools\rustgpt-lab\start-built-in-lab.cmd -BackendPort 7878 -WebPort 8787
.\tools\rustgpt-lab\start-built-in-lab.cmd -LabBackendTimeoutSeconds 1800
```

脚本会提示这是 built-in 安全路径，只启动 `rust-norion` built-in 后端和 `rustgpt-lab` Web UI；如果发现目标端口已经连着非 built-in 后端，会停止并要求换端口或先停掉旧进程。打开：

```text
http://127.0.0.1:8787/
```

这条路径仍会在你发送测试 prompt 时写入隔离的 `target\manual-web-lab-service\built-in-lab-state\*.ndkv`，但不会启动 Gemma，也不会改项目根目录经验库。只看状态可以访问：

```powershell
Invoke-RestMethod http://127.0.0.1:7878/health
Invoke-RestMethod http://127.0.0.1:8787/api/backend-health
```

也可以用只读状态脚本检查默认 `7878/8787` 上的 built-in 后端和 Web Lab：

```powershell
.\tools\rustgpt-lab\status-built-in-lab.cmd
```

Web Lab 还会只读轮询主后端的模型池状态：

```powershell
Invoke-RestMethod http://127.0.0.1:8787/api/model-pool-status
Invoke-RestMethod http://127.0.0.1:8787/api/model-pool-advice
```

这个接口只代理 `GET /v1/model-pool/status`，不会发送 prompt，也不会启动模型。苹果机或其它远程机器挂多个 worker 后，页面顶部的“模型池”行会显示 `healthy` 数量、调度是否阻断、`route/selected/blocked/in_flight` 和每个 worker 的忙碌/延迟，用它判断多模型是不是实际分担了任务。
`/api/model-pool-advice` 会把同一份只读状态整理成 `safe_to_enable_pool_workers`、`next_step`、`reason` 和中文 `advice`，用于判断先修 `quality 12B`、修 Metal/GPU，还是先加 `summary/review/index` 小模型。
它也会返回并在页面/REPL 中显示 `expected_helper_roles`、`missing_helper_roles` 和 `recommended_launch_order`，直接指出一主多小拓扑里还缺 summary/review/index/test-gate 哪些 helper，以及推荐的 quality -> summary -> review -> index -> test-gate 启动顺序。

### 只启动 Web Lab UI，连接已有后端

如果 `rust-norion` 后端已经在本机监听，你只想打开浏览器测试 UI/代理，不想启动 Gemma、不想启动 `mistralrs`、也不想再起一个新的 `rust-norion`，用这条命令：

```powershell
cd D:\rust-norion
cargo run --manifest-path tools\rustgpt-lab\Cargo.toml -- `
  --backend 127.0.0.1:7878 `
  --bind 127.0.0.1:8787
```

这条命令只启动 `8787` 上的 Web Lab UI/代理；`7878` 的 `rust-norion` 后端必须已经存在，`8686` 的 Gemma/mistralrs runtime 不会被启动或直接调用。

然后打开：

```text
http://127.0.0.1:8787/
```

端口速查：

- `127.0.0.1:7878` 是 `rust-norion` 主模型服务后端，Web Lab 会把 `/api/chat-stream` 转发到这里；如果看到 7878 拒绝连接，说明 UI/代理可能开了，但主后端没在这个端口监听。
- `127.0.0.1:8686` 是可选的 Gemma/mistralrs runtime，由 `rust-norion` 在后面调用；Web Lab 不应该直接把 prompt 发到 8686。
- `127.0.0.1:8787` 是 `rustgpt-lab` 浏览器 UI 和本地流式代理。

遇到拒绝连接时，先用只读状态脚本查清是谁没起来：

```powershell
.\tools\rustgpt-lab\status-built-in-lab.cmd
.\tools\rustgpt-lab\status-gemma-lab.cmd
```

已有后端时，可以直接复制这组“不启动模型”的检查/连接链路：

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\status-gemma-lab.cmd
cargo run --manifest-path tools\rustgpt-lab\Cargo.toml -- `
  --backend 127.0.0.1:7878 `
  --bind 127.0.0.1:8787
```

然后在浏览器打开 `http://127.0.0.1:8787/`，或者用 CLI 接到这个已经运行的 Web Lab：

```powershell
.\tools\rustgpt-lab\chat-gemma-lab.cmd -Lab http://127.0.0.1:8787 -Prompt "你好" -ShowMeta
.\tools\rustgpt-lab\repl-gemma-lab.cmd -SkipStart -BackendPort 7878
```

`chat-gemma-lab.cmd` 连接的是 `8787` 上的 Web Lab 代理；`repl-gemma-lab.cmd -SkipStart` 直接连接已有的 `7878` rust-norion 后端。两条命令都不会启动 Gemma 或 `mistralrs`。`8686` 只是 `rust-norion` 后面可选调用的 Gemma runtime，不是 Web Lab/CLI 应该直接发送 prompt 的端口。

停止 built-in Web Lab 前建议先 dry-run；默认只会停止 `/health` 确认安全的 `rust-norion runtime_mode=built-in` 和指向该后端的 `rustgpt-lab`：

```powershell
.\tools\rustgpt-lab\stop-built-in-lab.cmd -DryRun
.\tools\rustgpt-lab\stop-built-in-lab.cmd
```

`-BackendPort` 和 `-WebPort` 可改端口。`-ForceAll` 会按进程名停止所有 `rust-norion`/`rustgpt-lab`，可能误停其它本地测试进程，只有确认这些进程都可丢弃时再用。

### B. 真实 Gemma：先 CheckOnly，再启动

真实 Gemma 12B 联调前先运行只读 CheckOnly。它只检查配置、端口、StateDir、RAM/VRAM、backend health 和经验库安全提示；不会启动 Gemma，不会启动 Web Lab，也不会写 `.ndkv`。

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\start-gemma-lab.cmd `
  -CheckOnly `
  -StateDir target\manual-gemma-service\lab-state `
  -Snapshot "D:\hf-cache\hub\models--google--gemma-4-12B-it\snapshots\5926caa4ec0cac5cbfadaf4077420520de1d5205"
```

如果 CheckOnly 提示项目经验库 dirty，先使用隔离状态目录，或者在获得显式授权后再清理经验库。不要直接对项目根目录 `noiron-experience.ndkv` 做真实 Gemma 测试。

如果只想跑更底层的 model-service smoke gate，可以用 `cargo run -- --gemma-model-service-smoke --gemma-smoke-check-only`。

CheckOnly 通过后，再启动真实 Gemma/Web Lab 链路：

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\start-gemma-lab.cmd `
  -StateDir target\manual-gemma-service\lab-state
```

`-StateDir` 会把 `memory.ndkv`、`experience.ndkv`、`adaptive.ndkv` 放到隔离目录。只有在你明确确认并授权使用项目经验库时，才使用 `-UseProjectState`。

## 启动和停止

Gemma 12B 测试需要 `mistralrs serve` 常驻。这个常驻的是你机器上的后台进程，不依赖 Codex 对话窗口在线；脚本启动成功后，你可以直接用浏览器测试。常驻会占用约十几 GB 内存/显存；这是测试模式，不建议空闲时一直开着。

推荐用脚本一键启动完整测试链路：

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\start-gemma-lab.cmd
```

启动脚本会先做资源预检，确认可用内存和 NVIDIA 显存大致够跑 12B。默认阈值是 18GB 可用系统内存和 13GB 可用显存；如果你明确要强制启动，可以：

```powershell
.\tools\rustgpt-lab\start-gemma-lab.cmd -Force
```

长回答或首次加载 12B 时，推荐显式拉长 Web Lab 和主服务 runtime 超时：

```powershell
.\tools\rustgpt-lab\start-gemma-lab.cmd `
  -RuntimeTimeoutMs 1800000 `
  -LabBackendTimeoutSeconds 1800
```

它会启动：

- `mistralrs serve`：Gemma 12B 常驻 runtime，默认 `127.0.0.1:8686`；
- `rust-norion`：主模型服务，默认 `127.0.0.1:7878`；
- `rustgpt-lab`：外围 Web 测试工具，默认 `127.0.0.1:8787`。

脚本返回后进程仍会继续运行。输出里会打印每个进程的 PID 和日志路径，日志默认放在：

```text
D:\rust-norion\target\manual-gemma-service
```

打开浏览器：

```text
http://127.0.0.1:8787
```

查看当前测试链路状态，不启动任何进程：

```powershell
.\tools\rustgpt-lab\status-gemma-lab.cmd
```

在 CLI 里直接发一条测试对话，不打开浏览器：

```powershell
.\tools\rustgpt-lab\chat-gemma-lab.cmd -Prompt "用中文说明 rust-norion 现在怎么和 Gemma 联调。"
```

这个脚本不会启动服务，只会连接已经常驻的 `rustgpt-lab`。如果 Gemma 或后端没起来，它会先报清楚原因，而不是让你等一个必然失败的请求。想看 SSE 状态、meta、raw/enhanced 辅助事件：

```powershell
.\tools\rustgpt-lab\chat-gemma-lab.cmd -Prompt "你好" -Output raw -Profile coding -ShowMeta
```

想看 Noiron 增强后的回答流时，把 `-Output` 改成 `enhanced`；`-Profile` 会随请求 payload 一起转发给后端。

在 CLI 里测试完整业务联调流，包括生成、反馈、自改进、Rust 检查和 final 汇总：

```powershell
.\tools\rustgpt-lab\chat-gemma-lab.cmd `
  -Endpoint business-cycle `
  -Prompt "用中文给一个 rust-norion 业务联调建议。" `
  -FeedbackAmount 0.75 `
  -NoSelfImprove `
  -RustCheckCode "pub fn ok() -> bool { true }" `
  -ShowMeta
```

CLI 会直接显示 `stage`、`meta`、生成 delta，以及最后的 `business_cycle passed=...` 汇总；只有加 `-ShowMeta` 时才打印完整 `final` JSON。`-Output`、`-Profile`、`-FeedbackAmount`、`-NoSelfImprove` 和 `-RustCheckCode` 会发到同一组 business-cycle payload 字段，离线 safety 套件也会验证这些字段。

`chat-gemma-lab.cmd -TimeoutSeconds 1800` 会拉长 PowerShell SSE 客户端的整次流式等待窗口。跑慢 12B 长输出时，这个值应不小于 Web Lab 后端窗口，也就是 `--backend-timeout-secs` 或 `-LabBackendTimeoutSeconds` 配置的值。脚本会把 SSE `done` 之前的 EOF 和半截 SSE frame 视为截断流，并在收到 SSE `error` 后以非零状态退出，不会安静接受半截或被拒绝的回答。

运行 `.\tools\rustgpt-lab\test-gemma-lab-safety.cmd` 可以离线验证 PowerShell SSE 客户端，以及 start/status/stop 包装器的 help、CheckOnly 和 DryRun 安全路径；测试只使用本地假 Web Lab 或随机空端口，不需要启动 Gemma。假流式用例覆盖 heartbeat、comment-only keep-alive、CR-only frame 分隔、多行 data 字段、空 event 字段、无冒号 SSE 字段、字段值空格、business-cycle CLI endpoint/output/profile/Rust check/self-improve/feedback payload 字段、带或不带尾随 `done` 的 error、HTTP stream 建立失败、`done` 前 EOF、已有 `final` 但缺少 `done` 的 EOF、半截 frame 截断、body 空闲超时和响应头前超时；同时会检查 `web/app.js` 语法、提取 Web UI SSE parser，并验证同一组核心 frame 解析边界和半截 frame 保留逻辑。Node.js 离线检查还会用最小 DOM/fetch harness 加载真实 Web UI 脚本，覆盖 Enter 发送、输入法组合/重复/修饰键 Enter 不误发送、Shift+Enter 换行、发送按钮和输入框状态切换、上下文消息窗口 2..256 clamp、失败/取消后恢复草稿、heartbeat/status 进度行可见性、流式输出 auto-scroll、business-cycle Rust check payload、清空上下文后下一次请求不携带旧 history、流式过程中清空上下文后完成回包不回写旧 history、低上下文窗口下失败请求不裁掉已完成 history、busy/readiness/safe-device/经验库预检阻断时不清空草稿且不发 `/api/chat-stream`、用户取消恢复、HTTP stream 建立失败恢复，以及被取消/拒绝/截断流不写入浏览器 conversation；这些 Web 检查要求 `PATH` 上有 Node.js。PowerShell CLI 检查也会证明 `chat-gemma-lab.cmd` 在 Web Lab 不可达、backend busy、readiness、safe-device、经验库 hygiene/index 或 Gemma runtime 预检失败时，会在 `/api/chat-stream` 之前退出；Gemma runtime 不可达时会先提示只读 status 和 `start-gemma-lab.cmd -CheckOnly`，不会直接要求启动。还会证明 `repl-gemma-lab.cmd -SkipStart` 在后端端口未监听时保持 attach-only，先退出并提示 `7878` 后端、`8787` Web Lab 和 `8686` runtime 的区别，不进入 Gemma/start/REPL 路径；同一套 safety 也会锁住 `status-built-in-lab.cmd -Help` 的 built-in 端口图，确认 built-in status 路径不会查询 `8686`。旧的 `test-chat-gemma-lab-client.cmd` 入口保留为兼容别名；可先运行 `.\tools\rustgpt-lab\test-gemma-lab-safety.cmd -Help` 查看离线覆盖清单而不执行测试。

同一套 safety 也会锁住 built-in start/stop help 的 `7878`/`8787`/`8686` 端口图，确认 built-in 路径不会使用或停止 `8686`；也会锁住真实 Gemma start/status/stop help 的同一端口图，确认 `8686` 是 `rust-norion` 后面的可选 runtime，不是直接发送 prompt 的目标。

也可以直接运行 Rust 写的交互式 REPL，不经过 PowerShell 包装：

```powershell
cd D:\rust-norion\tools\rustgpt-lab
cargo run -- --help
cargo run -- --repl --backend 127.0.0.1:7878
```

`--help` 只打印参数说明并退出，不会启动 Web Lab、后端、Gemma，也不会发送 prompt。
即使和 `--repl` 等启动参数一起出现，help 也优先生效，适合作为第一条只读命令。

如果想一条命令启动/确认完整 Gemma lab 链路并进入 CLI：

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\repl-gemma-lab.cmd
```

如果服务已经启动，只想挂到现有后端上对话：

```powershell
.\tools\rustgpt-lab\repl-gemma-lab.cmd -SkipStart
```

如果要等待更慢的 12B 长回答：

```powershell
cargo run -- --repl --backend 127.0.0.1:7878 --backend-timeout-secs 1800
```

如果要在 CLI 里做更长多轮上下文测试，可以启动时调整短上下文窗口；默认 64 条，
范围 2..256：

```powershell
cargo run -- --repl --backend 127.0.0.1:7878 --context-messages 128
```

REPL 里普通输入会直接发送，`/help` 查看命令。常用命令：

```text
/mode chat
/mode business-cycle
/output raw
/profile coding
/max 262144
/context-window 128
/rust pub fn ok() -> bool { true }
/status
/pool-advice
/clear
/quit
```

停止测试链路并释放 Gemma 12B 内存/显存：

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\stop-gemma-lab.cmd -DryRun
.\tools\rustgpt-lab\stop-gemma-lab.cmd
```

如果只想停前端和主服务，保留 Gemma 12B 常驻 runtime：

```powershell
.\tools\rustgpt-lab\stop-gemma-lab.cmd -KeepMistral
```

默认停止路径只会停止已确认的本地测试栈进程：后端必须在 `/health` 中报告
`rust-norion`，Web Lab 必须指向这个后端，Gemma runtime 端口必须由
`mistralrs` 占用。只有确认所有同名本地进程都可丢弃时，才使用 `-ForceAll`。

也可以手动检查 rust-norion 后端：

```powershell
Invoke-RestMethod http://127.0.0.1:7878/health
```

推荐测试节奏：

1. `start-gemma-lab.cmd` 开启常驻测试；
2. 浏览器在 `8787` 测对话；
3. `status-gemma-lab.cmd` 看端口、进程、health 和 GPU 摘要；
4. `stop-gemma-lab.cmd` 释放资源。

## 当前流式能力

当前版本提供“前端流式显示”：

- 浏览器提交消息后，代理立刻返回 SSE 状态事件。
- 代理优先连接 `rust-norion` 的 `/v1/chat-stream`、`/v1/generate-stream` 或
  `/v1/business-cycle-stream`，把后端 delta 边收到边透传给浏览器。
- 如果只剩同步后端路由可用，代理才会把最终 `answer` 分片作为兼容 fallback
  推给浏览器。
- 页面会把 `status` 和 `heartbeat` 合并到同一条进度行里更新，慢首 token 时不会刷屏；`stage` 和 `meta` 仍逐条显示，方便看 business-cycle 审计轨迹。
- 离线 safety 套件会加载真实浏览器脚本验证输入体验：普通 Enter 请求提交，输入法组合/重复/修饰键 Enter 不误发送，Shift+Enter 留在 textarea 内换行，发送按钮和输入框在流式请求中禁用并在完成后恢复，heartbeat/status 持续显示在进度行，streamed delta 渲染时 auto-scroll 会跟到底部；同一个离线 harness 也会验证 business-cycle 模式会发送 `endpoint=business-cycle` 和 Rust check code，清空上下文会让下一次请求只带新 user prompt，流式过程中清空上下文后，即使随后收到 `final`/`done`，也不会把该轮或旧 history 写回临时 conversation，低上下文窗口下失败请求不会裁掉已完成 history；backend busy、readiness、safe-device 和经验库 hygiene/index 的提交预检阻断会保留草稿、不发 `/api/chat-stream`、不改浏览器 conversation。用户取消、HTTP stream 建立失败、SSE `error`、`done` 前 EOF 和已有 `final` 但缺少 `done` 的流都会把 assistant 标成中断、恢复输入和恢复草稿，并且不会把这轮半截 user/assistant 写入浏览器 conversation 上下文。
- 浏览器里的临时 `conversation` 只来自完整 Web SSE 回合；lab state 目录里的 `.jsonl` 是 trace 产物，不会作为浏览器对话上下文来源。
- 浏览器只有收到 SSE `done` 且没有 `error`、取消或残留不完整 SSE frame 时，才会把 assistant 回答写入临时 `conversation` 上下文；如果连接 EOF 前缺少 `done`，页面会显示 `stream truncated`，并丢弃这次 partial assistant，避免半截回答污染后续多轮上下文。
- 浏览器 `/v1/chat` 测试默认保留 64 条临时对话消息，页面控件会 clamp 到 2..256 条；每次发送会把这个值当作请求消息上限：最多 `limit - 1` 条历史对话消息加上当前 user prompt。这里的 128/256 是短会话消息槽位，不是 128/256 token 模型上限。页面默认 `max_tokens=262144`，这是较大的请求生成预算参数，必要时可以手动调小来减少等待。
- Rust REPL 也默认携带最近 64 条对话上下文消息，可用启动参数 `--context-messages 128`、`--context-window 128`、`--max-context-messages 128` 或运行时命令 `/context-window 128` 调整到最多 256 条；这和 `max_tokens` 是两件事，一个控制短会话历史消息数，一个控制输出预算，不要把 128 条上下文消息误读成 128 token。
- `rustgpt-lab` 代理默认给后端整次流式请求 900 秒总窗口；流式读取会用短轮询保持 SSE 心跳，即使真实 Gemma 首 token 很慢，浏览器也能持续看到 `heartbeat`。可以通过 `--backend-timeout-secs`（别名 `--timeout-secs`）、`start-gemma-lab.cmd -LabBackendTimeoutSeconds` 或 `start-built-in-lab.cmd -LabBackendTimeoutSeconds` 调整到 1800 秒等更长窗口。这个值是整次流式请求的总窗口，不是每次 socket read 的单次超时；headers、body delta 和流式错误体共享同一个截止时间，短轮询只负责维持浏览器心跳。
- 页面顶部会定时轮询 `/api/backend-health`，显示后端是否忙碌、活跃推理数、已处理请求数、Gemma runtime 实际模型和 `n_ctx/n_ctx_train`、`experience_hygiene.experience_file`、quarantine/repairable 经验债、索引风险、readiness/safe-device 和最近一次推理。
- 页面顶部也会定时轮询 `/api/model-pool-status`，只读显示模型池 worker 健康数、调度阻断原因、全局 route/selected/blocked/in_flight 统计，以及每个 worker 的 ready/busy/avg latency。这个状态专门用来观察苹果机或远程模型池是否真正并行分担了开发任务。
- 页面、REPL `/pool-advice` 和 `status-*-lab.cmd` 也会读取 `/api/model-pool-advice`，用同一套只读判断给出“不要多开 12B、优先一主多小”的下一步建议。
- 页面可以直接连接 built-in 安全后端做前后端分离/SSE 测试；真实 Gemma HTTP runtime 配置了但不可达，或 readiness/safe-device/经验库门禁失败时，发送按钮会显示 `预检失败` 并禁用。经验库门禁包括 `quarantine_candidates>0`、`repairable_legacy_metadata_lessons>0`、`repairable_index_records>0`、索引 `retrieval_ready=false`、索引 `risk_level=blocked` 或 `experience_hygiene.clean=false`。
- `rustgpt-lab` 服务端也会在每次 `POST /api/chat-stream` 转发前重新读取后端 `/health`。如果后端正在推理、Gemma runtime 不可达、readiness 失败、safe-device 失败或经验库/索引门禁失败，代理会返回 `status: checking backend prompt gate before forwarding request`，随后发送 SSE `error` 和 `done`，不会把 prompt 转发给主后端。
- 如果看到 `预检失败`，先运行 `.\tools\rustgpt-lab\status-gemma-lab.cmd`、`.\tools\rustgpt-lab\status-built-in-lab.cmd` 或 REPL `/status` 看具体是 Gemma 未启动、后端 busy、safe-device 失败，还是经验库 hygiene/index 阻塞；再用 `/hygiene dry-run [limit]`、`/repair dry-run [limit]`、`/audit [limit]` 做只读检查。
- REPL `/status` 会打印同一套 prompt gate 结果，以及 `experience_hygiene` 和 `experience_index` 字段；只用终端联调时也能看见为什么 prompt 被阻断。

`/v1/chat` 和 `/v1/generate` 测试现在会优先走主服务的 `/v1/chat-stream`、`/v1/generate-stream`。主程序的 Gemma HTTP runtime 会请求 mistralrs/OpenAI `stream:true`，把 SSE delta 一边转成 `RuntimeResponse.tokens`，一边通过 service-level SSE 透传给 `rustgpt-lab`。推理结束后主服务再发送 `final` 事件，里面是完整 JSON，包括 Noiron 反思、记忆写入、runtime token 统计等字段。

`/v1/business-cycle` 现在会优先走 `/v1/business-cycle-stream`：生成阶段会透传 Gemma delta，反馈、自改进、Rust 检查、保存状态和 gate 检查会发送 `stage`/`meta` 事件，最后用 `final` 事件返回完整业务联调 JSON。

## 输出模式

前端支持：

- `raw`：请求 Gemma/runtime 原始回答，用来观察模型本身效果。
- `enhanced`：请求 Noiron 增强后的回答，用来观察业务记忆、反思和自进化链路效果。

对应传给后端的 JSON 字段是：

```json
{
  "output": "raw"
}
```

## 业务联调模式

接口选择 `/v1/business-cycle` 时，页面会额外显示：

- `反馈`：传给后端的 `feedback_amount`；
- `自改进`：控制 `self_improve`；
- 可选 Rust 代码检查输入框：传给后端的 `rust_check_code`。

代理会优先把请求发到 `http://127.0.0.1:7878/v1/business-cycle-stream`。页面会实时显示生成 delta、`stage`、`meta`，最后根据 `final` 事件里的完整业务联调 JSON 更新回答。如果 stream 路由不可用，代理会回退到 `http://127.0.0.1:7878/v1/business-cycle`，并把 `business_cycle_passed`、`feedback_applied`、`rust_check_passed`、`self_improve_passed` 作为 SSE `meta` 事件显示出来。

## 目录边界

这个目录允许快速实验和删除。不要把这里的 UI 状态、HTML、代理逻辑提升进 `src/`，除非未来明确决定把 Web 产品化，并重新做许可证和架构评审。

## 代码结构

`src/` 已按测试耦合层职责拆开：

- `app.rs`：HTTP 路由和请求生命周期；
- `backend.rs`：调用 `rust-norion:7878` 的代理逻辑；
- `backend/stream.rs`：后端 SSE 解析、终止事件处理和 heartbeat 短轮询；
- `backend/io.rs`：后端 TCP 连接、读取和写入 timeout 设置；
- `repl.rs`：终端 REPL 循环和命令处理；
- `request.rs`：前端请求解析；
- `sse.rs`：SSE header 和事件输出；
- `http.rs`：最小 HTTP 读写工具；
- `chunk.rs`：回答分片；
- `json.rs`：当前无依赖 JSON 字段提取和转义；
- `config.rs`：命令行参数、上下文窗口默认值和 timeout 默认值；
- `status.rs`：长时间 Gemma 推理等待文案和错误提示。

这样后续要替换成 Axum/HTMX/Tera，或加入会话历史和消息持久化时，可以在外围测试目录里演进，不影响主模型服务。
