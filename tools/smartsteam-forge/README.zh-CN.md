# SmartSteam Forge

SmartSteam Forge 是独立于 `rust-norion` 主服务和 `rustgpt-lab` Web Lab 的
Rust TUI 流式对话前端。它默认连接本地 `rust-norion`：

远端 Gemma 12B + 本地后端 + Web Lab + Forge CLI/UI 的推荐一键入口是
`start-remote-gemma-forge.cmd`。推荐拓扑是 `1 个 12B quality worker + 多个小模型
helper workers`；不要为了并行把多个 12B worker 同时开在 Apple Silicon 上，它们会
争抢统一内存和 GPU。先跑只读预检，再真实启动：

```powershell
cd D:\rust-norion
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd
```

如果远端 Mac 已经放好了小/低量化 helper GGUF，再显式开启 helper workers：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf
```

当前 32GB Mac 推荐直接用 SmartSteam 预设：`summary/index` 共享 E2B，
`review/test-gate` 共享 E4B，避免为了逻辑角色重复常驻同一个小模型：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly -UseMac32GBModelPool -NoForge
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -UseMac32GBModelPool -NoForge
```

模型文件只放在本机 D 盘缓存和远端 Mac 的 `smartsteam-model-box\models`，远端不放
Rust 源码。要用国内 Hugging Face 镜像下载缺失文件并通过内网 SSH 同步到 Mac，
先把下载 URL 放进一个 JSON manifest，再执行同步脚本；已有文件会按字节数跳过：

```powershell
$env:HF_ENDPOINT = "https://hf-mirror.com"
.\tools\smartsteam-forge\sync-remote-gemma-model-cache.cmd -JsonStatus
.\tools\smartsteam-forge\sync-remote-gemma-model-cache.cmd -DownloadMissing -DownloadManifest D:\models\smartsteam-model-urls.json
```

`-CheckOnly` 会打印 `recommended_topology=one_12b_quality_plus_small_helpers`、
`avoid_multiple_12b_workers=true`、`check_only_command=...` 和
`start_command=...`；它不 SSH、不启动进程、不发送 prompt。只想启动链路但不进入
Forge TUI 时加 `-NoForge`。

安全的一键联调命令是 `start-forge-stack.cmd`。它只启动 `rust-norion`
built-in/heuristic 后端、可选 Web lab，然后进入 SmartSteam Forge；不会启动
Gemma 12B，也不会启动 `mistralrs`：

```powershell
cd D:\rust-norion
.\tools\smartsteam-forge\start-forge-stack.cmd -CheckOnly
.\tools\smartsteam-forge\start-forge-stack.cmd
```

`-CheckOnly` 只检查目录、`cargo`、端口和已存在后端 health，不启动任何进程、
不打开 TUI、不发送 prompt。它适合先确认 `127.0.0.1:7878` 当前是否空闲，或是否
已经有可用 `rust-norion` 后端。安全 built-in 栈默认使用隔离状态目录
`target\manual-forge-service\forge-state`，新启动的后端会把
memory/experience/adaptive/trace 写到这里，不会写仓库根目录的
`noiron-experience.ndkv`。

如果你明确要让安全 built-in 栈使用仓库根目录项目状态，才加
`-UseProjectState`；如果只想换一个隔离目录，用 `-StateDir <path>`。两者只能
二选一。

如果你已经在 `tools\smartsteam-forge` 目录里，也可以直接：

```powershell
.\start-forge-stack.cmd
```

如果只想启动安全后端和 Web lab，不进入 Forge TUI：

```powershell
.\tools\smartsteam-forge\start-forge-stack.cmd -NoForge
```

如果只想启动后端和 Forge TUI，不打开 Web lab：

```powershell
.\tools\smartsteam-forge\start-forge-stack.cmd -NoLab
```

如果要做一次可重复的外围联调验收，不启动 Gemma、不打开 TUI、不发送真实 prompt：

```powershell
.\tools\smartsteam-forge\smoke-forge-stack.cmd
```

这个 smoke 默认使用 `127.0.0.1:7891` 临时端口启动 built-in 后端，依次检查
Forge `/health`、`--doctor`、`--preflight` 和 UI preflight，结束后只清理这次
启动的后端进程。smoke 会把 built-in 后端的 memory/experience/adaptive/trace
状态隔离到 `target\manual-forge-service\smoke-state-*`，不会读取或写入仓库
根目录的 `noiron-experience.ndkv`。

如果后端已经启动，只想进入 CLI UI：

```powershell
.\tools\smartsteam-forge\start-forge-ui.cmd
```

`start-forge-ui.cmd` 默认要求后端是 Gemma HTTP 模式；如果你连的是
`start-forge-stack.cmd` 启动的 built-in 后端，用：

```powershell
.\tools\smartsteam-forge\start-forge-ui.cmd -AllowBuiltIn
```

只检查现在是否能进入 CLI UI，不打开 TUI、不发 prompt：

```powershell
.\tools\smartsteam-forge\start-forge-ui.cmd -CheckOnly
```

如果 Gemma 正在推理，等后端空闲后再进入 CLI UI：

```powershell
.\tools\smartsteam-forge\start-forge-ui.cmd -WaitReady
```

查看当前 Gemma runtime、`rust-norion` 后端和 Web Lab 状态：

```powershell
.\tools\smartsteam-forge\status-forge.cmd
```

`status-forge.cmd` 是只读命令，会显示端口监听、端口 owner、相关进程、
`/health` 里的 runtime、Gemma 可达性、`engine_busy`、`active_requests`、
`experience_hygiene.experience_file`、readiness/safe-device、最近一次推理、
`/v1/model-pool/status` 的 worker/route 指标和 GPU 摘要。它不会发送 prompt，
也不会启动或停止任何进程。如果只想跳过 model-pool worker 探测，加
`-NoModelPool`。

只读诊断“后端忙了 / GPU 没用 / 能不能安全发 prompt”：

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --backend 127.0.0.1:7878 --connect-timeout-ms 500 --read-timeout-ms 500 --health
cargo run -- --backend 127.0.0.1:7878 --connect-timeout-ms 500 --read-timeout-ms 500 --doctor
cargo run -- --backend 127.0.0.1:7878 --connect-timeout-ms 500 --read-timeout-ms 500 --preflight --require-safe-device
curl.exe -s http://127.0.0.1:7878/health
nvidia-smi
```

这些命令只读取 `/health` 或本机 GPU 状态，不发送 prompt、不触发 Gemma 推理。
这里的 `--read-timeout-ms 500` 只适合短探测：它是单次 socket read 的轮询/heartbeat
间隔，不是整次请求总超时；真实 Gemma 流式推理请用 `--timeout-secs` 调整总等待窗口。
如果 `/health` 里 `engine_busy=true`，看 `active_requests` 的 `request_id`、
`endpoint`、`elapsed_ms` 和 `prompt_preview`；等它空闲后再 `/ready`。如果
`device_primary_lane` 是 `cpu-vector` 或 `disk-backed-streaming`，说明当前后端
不是 GPU-first；真实 12B 长 prompt 前保持 `--require-safe-device`。

停止测试链路。安全 built-in 联调用更窄的 built-in 停止脚本：

```powershell
.\tools\rustgpt-lab\stop-built-in-lab.cmd -DryRun
.\tools\rustgpt-lab\stop-built-in-lab.cmd
```

真实 Gemma 联调结束后，先 dry-run 查看将释放哪些进程，再不带参数释放
12B 内存/显存；如果只想保留 `mistralrs` 常驻，加 `-KeepMistral`：

```powershell
.\tools\smartsteam-forge\stop-forge.cmd -DryRun
.\tools\smartsteam-forge\stop-forge.cmd -KeepMistral
.\tools\smartsteam-forge\stop-forge.cmd
```

`stop-forge.cmd` 默认只停止已确认的本地测试栈进程：`rust-norion` 必须在
`/health` 中报告 `runtime_mode=gemma-http` 或 `built-in`，Web Lab 必须指向
当前后端，`MistralPort` 必须由 `mistralrs` 占用；`-DryRun` 只列出目标不停止。
只有你明确要清理所有同名 `rust-norion`、`rustgpt-lab`、`mistralrs` 进程时才
使用 `-ForceAll`。

`cargo run -- --backend 127.0.0.1:7878` 只启动 Forge 客户端，不会启动
`rust-norion`。如果 7878 没有后端监听，它会直接报连接被拒绝。先启动安全后端，
或确认已有真实后端后再运行：

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --backend 127.0.0.1:7878
```

需要真实 Gemma 12B 联调时，先做 heavy path 检查，再显式启动 Gemma 全栈：

```powershell
cd D:\rust-norion
.\tools\smartsteam-forge\start-gemma-forge.cmd -CheckOnly
.\tools\smartsteam-forge\start-gemma-forge.cmd
```

`-CheckOnly` 只做 snapshot、端口、RAM/VRAM 和已存在后端 `/health` 检查；
不会启动 `mistralrs`、Gemma、`rust-norion`、Web lab 或 Forge TUI，也不会创建
`StateDir`、不会写任何 `.ndkv`。

`start-gemma-forge.cmd` 会启动 Gemma 12B runtime、`rust-norion` 后端、Web lab，
并直接进入 SmartSteam Forge。显存/RAM 不足时不要用 `-Force`，除非你明确接受
CPU/disk fallback 风险。

真实 Gemma 联调默认使用隔离状态目录
`target\manual-gemma-service\forge-state`。`rust-norion` 的
memory/experience/adaptive/trace 都会写到这里，不会读取或写入仓库根目录的
`noiron-experience.ndkv`。这样可以先把模型、后端、Web lab 和 Forge UI 跑通，
不被当前项目经验库的清理门禁卡住。

如果想换一个隔离状态目录：

```powershell
.\tools\smartsteam-forge\start-gemma-forge.cmd -StateDir target\manual-gemma-service\my-test-state
```

如果你明确要用仓库根目录的真实项目经验库，才加 `-UseProjectState`：

```powershell
.\tools\smartsteam-forge\start-gemma-forge.cmd -UseProjectState
```

`-StateDir` 和 `-UseProjectState` 只能二选一；前者显式指定隔离目录，后者显式
回到仓库根目录的项目状态文件。

当前根目录经验库如果还有 quarantine/repair/index 噪声，Forge readiness guard
会拦截正常 prompt。先完成 audit、dry-run、备份后的 apply 和 strict inspect gate，
再用 `-UseProjectState` 做真实业务经验联调。

如果 `127.0.0.1:7878` 已经有一个 Gemma 后端，脚本会读取 `/health` 里的
`experience_hygiene.experience_file`。只要它不是当前 `StateDir` 下的
`experience.ndkv`，默认会重启后端到隔离状态；只有加 `-KeepExistingBackend`
才会保留已有后端。

## Remote Gemma model box

如果不想让 Gemma 12B 吃本机开发资源，可以把 Mac 当成远端模型盒子。远端只需要
预先放好 `llama-server` 可执行文件和 GGUF 模型；本仓库的 Rust 源码、构建产物、
`rust-norion` 后端、Web Lab 和 Forge CLI 都留在本机。脚本使用 SSH key 登录，
`BatchMode=yes`，不会保存密码，也不会把 Hugging Face token 或 Rust 源码复制到
远端。

默认远端文件位置是：

```text
/Users/xinghuan/smartsteam-model-box/bin/llama-b9616/llama-server
/Users/xinghuan/smartsteam-model-box/models/gemma-4-12b-it-Q8_0.gguf
```

32GB Mac 当前推荐的常驻模型池是一条命令：

```powershell
cd D:\rust-norion
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly -UseMac32GBModelPool -NoForge
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -UseMac32GBModelPool -NoForge
```

`-UseMac32GBModelPool` 会自动启用 worker 池，并使用下面的共享拓扑：

```text
8686: quality -> Gemma 4 12B Q8, Metal, 65536 ctx / 4096 max_tokens
8687: summary -> Gemma 3 270M Q4, Metal, 8192 ctx
8689: router -> FunctionGemma 270M Q4_K_M, Metal, 4096 ctx
8690: index -> Gemma 4 E2B Q4_K_M, Metal, 8192 ctx
8688: review/test-gate -> Gemma 4 E4B Q4_K_M, Metal, 4096 ctx
```

这个 preset 要求 helper workers 也带 Metal/GPU runtime metadata；如果 manifest 或远端进程显示
`runtime_device=cpu`、`--device none` 或 `gpu_layers=0`，预检和 Forge runtime advice 会阻止继续扩池，
需要用 `-RestartRemote` 重启到 Metal 配置。
质量模型默认使用 65536 上下文，这是当前 32GB Mac + 12B Q8 + helper pool 的稳定档；
需要测试 262144 上下文时，显式传 `-ContextTokens 262144 -DefaultMaxTokens 262144`，并先观察内存压力。

这个 preset 默认查找：

```text
/Users/xinghuan/smartsteam-model-box/models/gemma-4-E2B-it-Q4_K_M.gguf
/Users/xinghuan/smartsteam-model-box/models/gemma-4-E4B-it-Q4_K_M.gguf
```

启动后 `status` 里如果看到 `capacity_expansion_allowed=false`，意思是“当前链路可用，
但不要继续加 worker”；只要 `readiness.ready=true` 且
`required_roles_ready=true`，就可以继续 Web Lab / Forge / evolution-loop 联调。

最短的端到端验证是一条 smoke 命令。它会启动或复用远端模型盒子、建立 SSH tunnel、
启动本地 `rust-norion` 后端和 Web Lab，先只读检查
`/v1/model-pool/status`：必须是 `read_only=true`、`sends_prompt=false`、
`launches_process=false`，且 quality worker 的 `base_url/port` 要和当前
`LocalModelPort` tunnel 对齐并 ready。通过后再分别通过 Web Lab SSE 和 Forge CLI
one-shot 发送极短真实 prompt：`Reply only with OK.`：

```powershell
cd D:\rust-norion
.\tools\smartsteam-forge\smoke-remote-gemma-chain.cmd
```

这个 smoke 默认使用 `LocalModelPort=8686`、`BackendPort=7979`、`LabPort=8789`，
适合和默认本地 `7878/8787` 栈并行。它不是只读检查：真实 prompt 会写入隔离状态
目录 `target\remote-gemma-chain\state`；smoke 通过后链路保持常驻，方便继续手动
测试。每个真实 prompt 发送前都会先执行只读
`gemma-chain chain-status -RequireAction web_lab_prompt|forge_cli_prompt -JsonStatus -FailIfBlocked`
并校验 `schema_version=1`、`contract_version=gemma-chain.v1`；如果后端 busy、
设备不安全或经验库 gate 未通过，会在发送 prompt 前失败。只想验 Web Lab SSE、
不跑 Forge CLI one-shot 时：

```powershell
.\tools\smartsteam-forge\smoke-remote-gemma-chain.cmd -NoForgeCli
```

默认 smoke 只要求 quality worker ready。如果远端已经放好了小模型，可以显式验
多 worker 模型池；这会把选中的 `summary/review/test-gate/index` 也作为
`/v1/model-pool/status` 必须 ready 的条件：

```powershell
.\tools\smartsteam-forge\smoke-remote-gemma-chain.cmd -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf
.\tools\smartsteam-forge\smoke-remote-gemma-chain.cmd -EnablePoolWorkers -PoolWorkerRoles review,index -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf
```

如果只想启动常驻链路，不自动发送 smoke prompt，用 start 命令。`start` 的默认
后端和 Web Lab 端口仍是 `7878/8787`；如果本机已有默认栈，建议显式改到
`7979/8789`：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789
```

如果要把 Apple Silicon 多 worker 计划直接接到后端模型池，并启动 Forge TUI，
推荐用单命令入口：

```powershell
cd D:\rust-norion
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd
```

它默认使用 `BackendPort=7979`、`LabPort=8789`，自动生成
`target\gemma-chain\apple-model-pool.generated.json`，再把它作为
`--model-pool-manifest` 传给本地 `rust-norion` 后端。`-CheckOnly` 只做本地
manifest/参数预检，不 SSH、不启动进程、不发送 prompt；它会明确打印
`model_pool_safe_to_enable_pool_workers`、`model_pool_next_step`、
`model_pool_worker_shape`、`model_pool_manifest_quality` 和启用的小 worker
`role=http://127.0.0.1:port` 映射，如果 manifest 里的 role、port 或 `base_url` 和当前 `LocalModelPort`、
`8687-8690` 小 worker 约定不一致，会在启动前失败。默认只启动远端
`8686` 的 12B quality worker；manifest 会列出 `8687-8690` 的模型池角色，
但小 worker 不会自动启动。只想启动链路、不进入 Forge TUI 时：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -NoForge
```

如果远端 Mac 已经放好了更小或低量化的 GGUF，可以显式开启小 worker。它会在
远端启动 `summary/review/test-gate/index` 四个 worker，并建立本机
`8687-8690` SSH tunnel：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf
```

如果不想四个逻辑角色各占一个物理进程，用 32GB Mac preset；它会生成共享端口的
model-pool manifest：`summary/index` 指向 `8690`，`review/test-gate` 指向
`8688`。这是当前 SmartSteam 远程小模型池的默认推荐：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -UseMac32GBModelPool -NoForge
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -Status -JsonStatus -UseMac32GBModelPool
```

`-EnablePoolWorkers` 默认会拒绝把 helper 指到 quality 的同一个 12B 模型路径，
也会拒绝明显包含 `12B/13B/27B/...` 的 helper 模型名。正常开发链路应该是
`1 个 12B quality + 多个小/低量化 helper`；如果确实要做压力测试，才显式加
`-AllowLargePoolWorkerModels`。这个保护只在启动/预检脚本层生效，不会 SSH、不发
prompt；`-CheckOnly` 可以先验证参数是否会被放行。

这条本地自测会自动验证“12B helper 默认被拦、小 helper 放行、显式压力测试开关
放行”。它只跑 `-CheckOnly`，不会 SSH、不会启动进程、不会发送 prompt：

```powershell
.\tools\smartsteam-forge\test-remote-model-pool-guards.cmd
```

如果每个 helper 角色要用不同 GGUF，可以只给对应角色设置模型路径；没有设置
per-role 路径的角色会回退到 `-RemoteSmallModel`。`-CheckOnly` 会先本地生成并
校验 manifest，不 SSH、不启动进程、不发送 prompt：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly -NoForge -EnablePoolWorkers `
  -RemoteSummaryModel /Users/xinghuan/smartsteam-model-box/models/gemma-summary-Q4.gguf `
  -RemoteReviewModel /Users/xinghuan/smartsteam-model-box/models/gemma-review-Q4.gguf `
  -RemoteTestGateModel /Users/xinghuan/smartsteam-model-box/models/gemma-test-gate-Q4.gguf `
  -RemoteIndexModel /Users/xinghuan/smartsteam-model-box/models/gemma-index-Q4.gguf
```

默认仍共用 `-RemoteLlamaServer`。如果某个角色需要不同的 server 二进制，可以用
`-RemoteSummaryLlamaServer`、`-RemoteReviewLlamaServer`、
`-RemoteTestGateLlamaServer` 或 `-RemoteIndexLlamaServer` 单独覆盖。
`-SummaryContextTokens`、`-ReviewContextTokens`、`-TestGateContextTokens`、
`-IndexContextTokens` 会同步写入自动生成的 model-pool manifest；helper 的默认
输出预算也可通过 `-SummaryDefaultMaxTokens`、`-ReviewDefaultMaxTokens`、
`-TestGateDefaultMaxTokens`、`-IndexDefaultMaxTokens` 调整。

结束测试时先 dry-run，再停止本地 tunnel、后端、Web Lab 和远端 worker：

```powershell
.\tools\smartsteam-forge\stop-remote-gemma-chain.cmd -DryRun
.\tools\smartsteam-forge\stop-remote-gemma-chain.cmd
```

也可以手动拆开执行，便于调试 manifest：

```powershell
cd D:\rust-norion
New-Item -ItemType Directory -Force .\target\gemma-chain | Out-Null
.\tools\gemma-chain\gemma-chain.cmd pool-manifest > .\target\gemma-chain\apple-model-pool.generated.json
.\tools\smartsteam-forge\start-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789 -ContextTokens 262144 -DefaultMaxTokens 262144 -ModelPoolManifest .\target\gemma-chain\apple-model-pool.generated.json -LaunchForge
```

`pool-manifest` 只生成 rust-norion 可读取的 `--model-pool-manifest` JSON；
`start-remote-gemma-chain.cmd` 仍然只把 manifest 传给本地后端，不会把 Rust 源码
复制到远端 Mac。

端口关系如下：

- `RemoteModelPort=8686`：远端 Mac 上 `llama-server` 监听的模型端口。
- `LocalModelPort=8686`：本机 SSH tunnel 暴露的模型 API，转发到远端
  `RemoteModelPort`。
- `BackendPort=7878`、`LabPort=8787`：`start-remote-gemma-chain.cmd` 的默认
  本机 `rust-norion` 和 Web Lab 端口。
- `BackendPort=7979`、`LabPort=8789`：推荐的并行联调端口，避免撞到默认
  `7878/8787`。

如果本机 `8686` 也被其他模型服务占用，换一个本地 tunnel 端口；后端会自动连到
你传入的 `-LocalModelPort`，状态和 smoke 命令也要传同一组端口：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-chain.cmd -LocalModelPort 8696 -BackendPort 7979 -LabPort 8789
.\tools\smartsteam-forge\status-remote-gemma-chain.cmd -LocalModelPort 8696 -BackendPort 7979 -LabPort 8789
.\tools\smartsteam-forge\smoke-remote-gemma-chain.cmd -LocalModelPort 8696 -BackendPort 7979 -LabPort 8789 -SkipBuild
```

查看状态不会发送 prompt，也不会启动或停止进程；如果本地后端支持
`/v1/model-pool/status`，它会同时显示模型池 worker/route 指标，方便判断
哪个 worker 在忙、哪个 worker 不可达：

```powershell
.\tools\smartsteam-forge\status-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789
.\tools\smartsteam-forge\status-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789 -JsonStatus
.\tools\smartsteam-forge\status-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789 -JsonStatus -FailOnNotReady
.\tools\smartsteam-forge\status-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789 -JsonStatus -ProbeRemoteRuntime
.\tools\smartsteam-forge\status-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789 -Watch -WatchIntervalSeconds 5
.\tools\smartsteam-forge\status-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789 -Watch -WatchIntervalSeconds 2 -WatchCount 3
.\tools\smartsteam-forge\status-gemma-forge.cmd
.\tools\smartsteam-forge\test-remote-gemma-forge-status.cmd
```

`-Watch` 只是重复执行同一个只读状态检查，按 `Ctrl+C` 停止；`-WatchCount` 适合
smoke 或短时间观察重启过程。它会 SSH 查询远端 pid/API 状态，但不会启动、停止或
向模型发送 prompt。
默认 `-JsonStatus` 保持本地只读并输出 `remote_probe_skipped=true`；显式加
`-ProbeRemoteRuntime` 后才会 SSH 到 Mac 读取 `lsof/ps`，并在
`remote_runtime.workers[]` 里写入 `gpu_layers/device/kv_offload/cpu_or_no_gpu`，
方便 report gate 或自动化脚本读到真实 CPU/GPU 放置。
监控或 CI 只需要机器判定时，加 `-FailOnNotReady`：脚本会先写出 JSON 和可选
`-OutputJson` 文件，再在 `readiness.ready=false` 时以非零退出；这个模式仍然不 SSH、
不启动进程、不发送 prompt，除非同时显式加 `-ProbeRemoteRuntime`。
远端状态还会按实际 `llama-server` 进程和监听端口打印
`launch flags: gpu_layers=... device=... kv_offload=...`；这是真实启动参数，
可以用来判断某个 worker 是否跑在 CPU，和本地 backend manifest metadata 分开看。
如果看到 `launch warning: cpu_or_no_gpu=true backend_metadata_may_differ=true`，
以远端真实进程为准；backend 里的 `runtime=metal:* cpu:*` 是 manifest/backend 视角，
可能还没有反映手动重启过的远端 worker 参数。
`status-gemma-forge.cmd` 是本地只读别名，默认检查 `8686/7979/8789`，适合确认
远程 Gemma 隧道、本地后端和 Web Lab 是否和推荐联调端口一致。
`test-remote-gemma-forge-status.cmd` 是离线自测，只验证 Forge 状态包装脚本能输出
只读 JSON、不会触远端、不会启动进程、不会发送 prompt。
如果页面显示 `Gemma 未启动 http://127.0.0.1:8686`，优先运行：

```powershell
.\tools\smartsteam-forge\status-gemma-forge.cmd -NoGpu
```

状态里的 `Model runtime diagnosis` 会同时显示本机 `8686` 模型 API 是否 healthy、
本地 `ssh-tunnel.pid` 是否 stale/mismatched、以及 `remote_ssh_probe` 是否能连到
`192.168.10.11:22`。如果 `remote_ssh_probe tcp=False`，通常是 Mac 睡眠、离线、
不在同一内网或 SSH 未开启；先恢复 Mac/SSH。SSH 可达后，如果本机后端和 Web Lab
已经在 `7979/8789`，优先只修复本机 tunnel，不重启远端模型：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-chain.cmd -BackendPort 7979 -LabPort 8789 -LocalModelPort 8686 -SkipBuild -NoBackend -NoLab
```

这条命令会复用已运行的远端 `llama-server`，清理 stale/mismatched 的本机 tunnel
pid 并重新建立 `127.0.0.1:8686 -> Mac:8686` 转发；它不发送 prompt，也不会启动
新的本地后端或 Web Lab。只有你明确要停止并重启远端 `llama-server` 时，才额外加
`-RestartRemote`。

如果你对远端链路使用的
`target\remote-gemma-chain\state\experience.ndkv` 做了 quarantine/repair apply，
必须让本地 `rust-norion` 后端重读磁盘状态；否则旧的内存副本可能在下一轮保存状态时
把刚清掉的脏记录写回来。用 backend-only reload，不停远端 Mac 模型、不停 tunnel、
不停 Web Lab：

```powershell
.\tools\smartsteam-forge\reload-remote-gemma-backend.cmd -CheckOnly
.\tools\smartsteam-forge\reload-remote-gemma-backend.cmd -SkipBuild
```

它会校验 `target\remote-gemma-chain\rust-norion.pid` 指向的确实是
`remote gemma via ssh tunnel` 后端，只停止并重启这个本地 `rust-norion.exe`。
对应自测只跑 `-CheckOnly`，不会停止或启动任何进程：

```powershell
.\tools\smartsteam-forge\test-remote-gemma-backend-reload.cmd
```

状态输出还会给出 `remote_chain_readiness` 和 `remote_chain_next_step`。前者把
本机 tunnel、后端、Web Lab、quality worker、模型池 launch gate 和 capacity gate
压成一行；后者在 `quality_worker_down`、`capacity.expansion_allowed=false` 或端口未开
时给出下一条建议命令。
`-JsonStatus` 输出纯 JSON，字段包含 `readiness`、`model_pool.capacity` 和
`next_step`，用于脚本/自动化读取；这个模式只做本地 HTTP/端口检查，不 SSH、不启动
进程、不发送 prompt。

如果要让远端 Gemma 链路直接进入 SmartSteam 自进化闭环，用一条命令启动或复用
远端模型盒子、本地 `rust-norion` 后端，然后跑 `tools/evolution-loop`。默认是一轮、
启用 `summary/review/test-gate/index` 小 helper，并在结束后用 report gate 确认
最新一轮确实拿到了可反哺下一轮的 helper 反馈：

```powershell
cd D:\rust-norion
.\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd -CheckOnly
.\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd
```

默认 helper 使用 Mac32GB 预设：`summary` 用 Gemma 3 270M，`router` 用
FunctionGemma 270M，`review/test-gate` 用 E4B，`index` 用 E2B。旧的单小模型 smoke
仍可用 `-RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf`
覆盖。先用 `-CheckOnly` 看完整命令和预检结果；如果只想先让 12B quality 单独跑一轮：
真实运行会用 `status-remote-gemma-chain.cmd -JsonStatus -ProbeRemoteRuntime` 刷新
`status-with-model-cache.json`，所以 evolution-loop 的 `remote_chain` 上下文和
report JSON 会包含 `remote_runtime_probed`、`remote_runtime_workers` 和
`remote_runtime_cpu_or_no_gpu`。

```powershell
.\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd -NoPoolWorkers
```

如果要多跑几轮：

```powershell
.\tools\smartsteam-forge\run-remote-gemma-evolution-loop.cmd -Rounds 3
```

如果要让它进入“带预算的无人值守”模式，用更短的一键入口。它固定转到
`-Forever`，但默认带 `MaxRuntimeSecs=3600`、`MaxTotalTokens=20000`、
`MaxNoFeedbackRounds=3`、`MaxFailures=3`，结束后仍会跑 report gate：

```powershell
.\tools\smartsteam-forge\run-remote-gemma-unattended.cmd -CheckOnly
.\tools\smartsteam-forge\run-remote-gemma-unattended.cmd
```

查看无人值守闭环是否正在跑、上次为什么停、report gate 是否允许继续，用同一个入口：

```powershell
.\tools\smartsteam-forge\run-remote-gemma-unattended.cmd -Status
.\tools\smartsteam-forge\run-remote-gemma-unattended.cmd -JsonStatus
```

`-Status` 和 `-JsonStatus` 都是只读：只代理 `evolution-daemon` 状态，不启动进程、
不发送 prompt。输出会明确显示 `daemon running`、`last_stop_reason`、ledger 轮数、
`report_gate passed`、`unattended_start_plan can_start` 和建议的 StartCheck/Start 命令。
如果 daemon 正在跑但 ledger 还落后一轮，先看 `daemon_round_transition_status`：
`latest_round_state=round_in_progress` 且 `round_in_progress=true` 表示只是当前轮仍在
生成/验证中，后端 busy 通常来自 active quality worker；
`latest_round_state=round_done_waiting_ledger_commit` 且 `ledger_commit_pending=true`
表示 stdout 已看到
`[round N] done [DONE]`，正在等 ledger/report 提交追上。两种状态都保持
`read_only=true`、`starts_process=false`、`sends_prompt=false`、`writes_ndkv=false`，
只用于显示和测试断言，不会启动/停止 daemon、触碰远端或 replay prompt。
`-JsonStatus` 和 `-JsonStartCheck` 都会输出 `daemon_start_gate`、
`report_gate_start_gate` 和 `readiness_start_gate` 三个结构化 start gate，且三者都会
带 `read_only=true`、`starts_process=false`、`sends_prompt=false`。前两个分别
对应候选补丁生命周期和上一轮 report continuation gate；`readiness_start_gate` 会把
`status_ready`、`start_ready`、`blocks_start`、`failures` 和
`start_blocking_failures` 单独暴露出来，脚本可以直接判断是后端/远程链路/ledger
hygiene 阻断启动，还是仅仅 first-run ledger 不足。旧的多行 `candidate_preflight`、
`report_gate_preflight` 字符串仍保留给人工阅读。
`-JsonStartCheck` 还会同步带出 `candidate_backlog` 和 `report_gate_status`，所以自动化脚本
可以一次读取候选补丁明细、上一轮 report 失败原因和三类 start gate，不必先额外跑
`-JsonStatus`。
Forge 会在输出前校验 StartCheck JSON：`preview_source` 必须是 `rust_pure_preview`，
`command_output` 必须包含 `check_only=true`、`starts_process=false` 和
`sends_prompt=false`，否则直接失败，避免旧脚本预览或不安全预览混入自动化链路。
`-JsonStatus` 的 enriched JSON 也会在输出前校验：顶层
`smartsteam.forge.evolution_status.v1`、嵌入的原始 `evolution_status`、以及
`daemon_log_tail`、`daemon_round_transition_status`、
`report_gate_status/preflight/start_gate`、`daemon_start_gate`、
`readiness_start_gate`、`worker_window_replacement_report`、
`clean_room_handoff_report`、`self_improve_proposal_panel`、
`helper_stage_repair_panel`、`unified_status` 和
`unattended_start_plan` 都必须
保持只读安全标记。`unified_status` 是给 Web
Lab/Forge 同屏消费的只读聚合视图：它只投影 daemon/supervisor、model-pool、
worker-window replacement report、typed `memory_startup_admission_status` 和
`clean_room_handoff_report_v1`、`self_improve_proposal_artifact_v1`、
`helper_stage_repair_status_report_v1`，
并显示 `worker_replacement_required`、`memory_admission_safe`、`no_live_write`
、`no_ndkv_write`、`clean_room_handoff_loaded`、`clean_room_handoff_safe`、
`self_improve_proposal_loaded`、`self_improve_proposal_safe`、
`helper_stage_repair_loaded`、`helper_stage_repair_safe` 和
`helper_stage_repair_required`。R26 proposal panel
只投影 candidate/validated/admitted/quarantined/promoted/repair-required 的计数、
候选 id 和 reason code，不复制 helper prose、旧窗口 payload 或 prompt body；
它不会启动/停止 daemon、触碰远端、下载/预热模型、发送 prompt、开启 stream、
创建 clean-room replacement，或把 helper prose/旧窗口 payload 扩张成
admission/live write/真实 `.ndkv` 写入。R28/R29 helper-stage repair panel 只投影
latest round、repair-required、proposal/incomplete role 计数、缺失 helper role 的
repair-required proposal count/roles、role、proposal id、missing/placeholder fields 和
validation safety；它不复制 helper prose、旧窗口
payload 或 prompt body，也不会启动 Forge/Web Lab、启动/停止 daemon、调用模型、
发送 prompt、开启 stream、replay prompt、修改 ledger/memory 或写 `.ndkv`。
`worker_window_status` / `worker_window_replacement_report` 中的
`polluted=true`、`archived=true` 或 `completed_evidence_only=true` 表示原窗口只剩
复盘证据，`assignment_allowed=false`、`future_work_requires_fresh_clean_room=true`
时不要继续给它派活；后续实现必须使用新的 clean-room 窗口。这个判断同样是只读
投影，不会自动创建 replacement window，也不会修改 worker-window 状态。

临时缩短验收可以覆盖预算：

```powershell
.\tools\smartsteam-forge\run-remote-gemma-unattended.cmd -MaxRuntimeSecs 600 -MaxTotalTokens 8000
```

默认 report gate 会要求最新 helper 反馈至少按角色契约输出可用字段，例如 `summary`
用 `memory_update/next_context/duplicate_guard`，`review` 用
`risk/change_request/verification`，`index` 用
`clean_gist/tags/dependency_link/source_origin/validation_timestamp/retention`，
`test-gate` 用 `verdict/validation_command/failure_kind`。临时排障时可加
`-NoUsefulHelperFeedbackGate` 只检查最新 helper 角色是否存在。夜跑或正式验收时可加
`-RequireCompleteHelperFeedbackGate`，强制每个最新 helper 补齐必需字段；其中
`test-gate` 在 `verdict: pass` 时允许不写 `failure_kind`，`warn/fail` 时必须写清。
`evolution-report.json`
会同步写出 `helper_stage_contract_by_role`，下一轮 prompt context 也会带上同名摘要，
其中 `fields` 会按角色拆出 `risk`、`change_request`、`clean_gist` 等稳定字段，
方便模型直接复用 helper 的风险、验证和索引建议。

这条闭环命令也有一个只跑 `-CheckOnly -NoStartChain` 的轻量自测，验证默认 helper
参数、`-NoPoolWorkers` 分支和安全标记；它不 SSH、不启动进程、不发送 prompt：

```powershell
.\tools\smartsteam-forge\test-remote-gemma-evolution-loop.cmd
.\tools\smartsteam-forge\test-remote-gemma-unattended-status.cmd
```

后端启动后，也可以直接从 Forge 查询模型池只读契约。它只调用
`rust-norion` 的 `/v1/model-pool/status` 和 `/v1/model-pool/route-plan`，
不会启动 worker、不会 SSH、不会发送 prompt：

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --backend 127.0.0.1:7979 --pool-status
cargo run -- --backend 127.0.0.1:7979 --pool-watch 5
cargo run -- --backend 127.0.0.1:7979 --pool-watch 2 --pool-watch-count 3
cargo run -- --backend 127.0.0.1:7979 --pool-route review
```

`--pool-watch` 只重复读取 `/v1/model-pool/status`，不会调用 route/call，不会发送
prompt。默认每 5 秒刷新一次，按 `Ctrl+C` 停止；`--pool-watch-count` 适合 smoke
或脚本里做有限轮询。

进入 TUI 后对应命令是：

```text
/pool-status
/pool-watch 5
/pool-watch 2 3
/pool-watch off
/pool-route review
```

TUI 里的 `/pool-watch` 是非阻塞轮询：命令只启停 watch，实际状态读取由 UI tick
后台发起，不会在命令处理里 sleep 卡住输入。

`/pool-route` 只是 route plan，用来查看 `summary/review/test-gate/index/quality`
这类任务会候选哪个 worker，以及为什么被 `quality_worker_down` 或其他 gate
阻断；它不会把当前输入派发给模型。
如果 worker 的 metadata 有报告，status/route 输出中的每个 `worker` 和选中的
`pool_dispatch` 会显示 `runtime_backend`、`runtime_device`、`runtime_accelerator`
和 `gpu_layers`。这些是远端 worker 自报的 Metal/GPU 线索；字段为空或 unknown 时，
需要继续看远端启动日志、Activity Monitor GPU History 和 Memory Pressure。
`/pool-status` 还会显示 `capacity` 摘要：`expansion_allowed=false` 时先按
`recommendation` 恢复 quality gate、修复 CPU fallback 或补齐 runtime metadata，
再继续增加小 worker。

如果已经显式启用了小 worker，可以用 `pool-call` 把一次辅助任务交给本地后端
统一调度。新版会优先调用 `rust-norion` 的 `POST /v1/model-pool/call`：后端先
检查 quality worker gate，再选择 worker，只有允许时才发送 prompt。旧后端没有
这个 endpoint 时，Forge 才会 fallback 到以前的兼容路径：先问
`/v1/model-pool/route-plan`，再直连 selected worker 的 `/v1/chat/completions`：

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --backend 127.0.0.1:7979 --pool-call review --prompt "review this patch for obvious risks"
cargo run -- --backend 127.0.0.1:7979 --pool-call index --prompt "refresh repository map for model_service and experience retrieval"
```

TUI 内对应命令：

```text
/pool-call review review this patch for obvious risks
/pool call summary summarize the last test log
/pool-call index refresh repository map for model_service and experience retrieval
```

`pool-call` 是显式发 prompt 的辅助调用；不要把它当成只读诊断。`pool-status` 和
`pool-route` 才是只读。主聊天仍走原来的 quality 链路，避免小模型结果自动污染
主会话上下文。worker 需要兼容 OpenAI-style `/v1/chat/completions`；manifest
里的 `base_url` 可以是 `http://127.0.0.1:8688` 或
`http://127.0.0.1:8688/v1`。
`/pool-smoke` 的 `section=alignment` 会直接显示
`extra_quality_12b_detected=true|false`；如果为 true，先停止额外的 quality/12B
worker，回到一个 12B quality worker 加 summary/review/index/test-gate 小 helper
的拓扑，再继续联调。alignment 也会列出 `missing_manifest_helper_roles` 和
`missing_status_helper_roles`；只有 manifest/status 都看见 summary、review、index、
test-gate，没有 `unexpected_manifest_roles` / `unexpected_status_roles`，且
`helper_worker_count_aligned=true`、`missing_route_smoke_tasks=none`、
`unexpected_route_smoke_tasks=none`、`route_smoke_count_aligned=true`、四类 route
smoke 都允许时，才算 helper 池真正对齐。报告头部的 `smoke_alignment_ok=true|false`
是给 CLI/TUI 快速查看或脚本 grep 的同一结论。`/pool-status` 和
`/pool-smoke section=status` 也会在后端契约提供时显示 `expected_helper_roles` /
`missing_helper_roles`，用于直接确认 root 后端看到的 helper 角色缺口。

CLI `--pool-call index ...` 和 TUI `/pool-call index ...` 的回答都会被写入
`state/project_notes.md` 里的 `model_pool_index` 区块。用 `/index-notes`
查看当前索引区块；输出里的 `active=latest_delimited` 和
`index_note_N ... active=true` 表示当前会被 Forge 请求上下文和 `/retrieve`
使用的最新完整索引，旧的完整区块或尾部 legacy 区块只作为历史显示。用
`/index-notes clear` 会清理所有模型池索引区块，包括旧完整区块和 legacy 未闭合块，
但不会清空手写项目笔记。后续
`/retrieve 5 <prompt>` 和 Forge 请求上下文会读取这些 project notes；如果
retrieve 输出里出现 `index_context_used=true`、
`index_context_chars=N` 或 `index_context_query=used chars=N`，说明索引 worker
的仓库地图已经作为结构化 `index_context` 进入后端检索预过滤上下文，并可供
后续 Forge 上下文注入使用。

常驻链路启动后，Web Lab 打开：

```text
http://127.0.0.1:8789/
```

CLI UI 连接同一个本地后端：

```powershell
.\tools\smartsteam-forge\start-forge-ui.cmd -Backend 127.0.0.1:7979 -WaitReady
```

也可以不用进入 TUI，直接跑一次 Forge CLI one-shot：

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --backend 127.0.0.1:7979 --mode chat --require-health --require-safe-device --timeout-secs 120 --prompt "Reply only with OK."
```

远端 Gemma 链路稳定后，可以启动外围自进化节拍器。它独立放在
`tools\evolution-loop`，不会把常驻循环塞进 Forge 或 `rust-norion` 主服务：

```powershell
cd D:\rust-norion
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 5 -MaxTokens 4096 -SelfImproveLimit 1
```

常驻运行用：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Forever -IntervalSecs 30 -MaxFailures 3 -MaxTokens 4096 -MaxTotalTokens 20000 -MaxRuntimeSecs 3600
```

它会调用 `/v1/business-cycle-stream`，把每轮的 stage、meta、final gate、错误和 token
统计写入 `target\evolution\evolution-ledger.jsonl`。这就是当前可落地的“自我进化”：
持续业务循环、反馈、回放、保存状态和 gate 验证；不是在线静默修改 Gemma 权重。
再次启动时会从已有 ledger 的最大 round 继续；不同实验可以加 `-CasePrefix nightly-evo`
区分 case 名。

已经启动常驻 daemon 后，先刷新严格状态 artifact，再用 Forge 只读查看当前轮次、
Mac 模型池、验证门禁和 self-improve 是否通过。这个入口只读，不启动脚本、不发送
prompt：

```powershell
.\tools\evolution-loop\refresh-strict-status-artifacts.cmd -JsonStatus -FailOnNotReady
.\tools\smartsteam-forge\target\debug\smartsteam-forge.exe --evolution-strict-summary
```

在 Forge TUI 里也可以直接输入 `/strict-status` 查看同一个摘要；需要指定其他
summary JSON 时用 `/strict-status <summary-json-path>`。

只想复盘而不触发模型时用：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Report
```

需要给夜跑或下一轮模型分析留下机器可读 artifact：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Report -ReportJson target\evolution\report.json
```

夜跑验收用退出码判断：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -ReportGate -MinReportRounds 3 -MinSuccessRate 60 -MinFeedbackTotal 1
```

Rust 代码任务可以把编译检查一起并入业务循环：

```powershell
.\tools\evolution-loop\start-evolution-loop.cmd -Backend 127.0.0.1:7979 -Rounds 3 -RustCheckFile .\target\evolution\candidate.rs -RustCheckEdition 2024
```

`evolution-loop` 默认不开严格 state/business/trace gate，避免隔离状态冷启动时把正常
反馈回放误判失败；需要严格验收时加 `-BusinessGate`。只有后端启动时配置了 trace
schema gate 路径，才再加 `-TraceGate`。

注意：`max_tokens` 是输出预算，不等于模型上下文窗口。远端脚本里的
`-ContextTokens` 控制 llama-server 上下文窗口，`-DefaultMaxTokens` 控制后端默认输出预算。
默认远端 quality 链路按模型窗口启动：`-ContextTokens 262144 -DefaultMaxTokens 262144`，
并把 runtime/Web Lab 等待窗口对齐到长推理。低资源或快速冒烟时可以显式降级：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -ContextTokens 8192 -DefaultMaxTokens 4096 -RuntimeTimeoutMs 600000
```

直接拉到 262144 会带来更高的 KV cache 内存压力，启动和首 token 都可能更慢；这是为了完整
Gemma 12B 联调和长上下文进化链路准备的默认形态。

停止链路前可以先 dry-run；不加 `-KeepRemoteModel` 会同时停止远端
`llama-server`，加 `-KeepRemoteModel` 则只停本机 tunnel、后端和 Web Lab：

```powershell
.\tools\smartsteam-forge\stop-remote-gemma-chain.cmd -DryRun
.\tools\smartsteam-forge\stop-remote-gemma-chain.cmd -KeepRemoteModel
.\tools\smartsteam-forge\stop-remote-gemma-chain.cmd
```

远端重启模型时可给 start 加 `-RestartRemote`。脚本会确认
`http://127.0.0.1:<LocalModelPort>/v1/models` 可用后再继续启动本地后端，避免
本机误连到空端口或其他模型服务。

如果只是想测试终端交互，不启动 Gemma 或 `rust-norion`：

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --mock
```

如果只想在 CLI 里发一条消息并退出，用 one-shot 模式：

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --mock --prompt "hello"
cargo run -- --mock --context-messages 128 --max-tokens 8192 --prompt "hello"
cargo run -- --mock --smoke
cargo run -- --mock --mode business-cycle --smoke
cargo run -- --sessions
cargo run -- --sessions failed --session-limit 20
cargo run -- --summary 1
```

真实后端已经启动时，可以直接测本地 `rust-norion` SSE 链路：

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --backend 127.0.0.1:7878 --connect-timeout-ms 500 --read-timeout-ms 500 --health
cargo run -- --backend 127.0.0.1:7878 --connect-timeout-ms 500 --read-timeout-ms 500 --hygiene
cargo run -- --backend 127.0.0.1:7878 --connect-timeout-ms 500 --read-timeout-ms 500 --hygiene-quarantine --hygiene-limit 20
cargo run -- --backend 127.0.0.1:7878 --connect-timeout-ms 500 --read-timeout-ms 500 --repair --repair-limit 20
cargo run -- --backend 127.0.0.1:7878 --connect-timeout-ms 500 --read-timeout-ms 500 --audit --audit-limit 20
cargo run -- --backend 127.0.0.1:7878 --connect-timeout-ms 500 --read-timeout-ms 500 --preflight
cargo run -- --backend 127.0.0.1:7878 --connect-timeout-ms 500 --read-timeout-ms 500 --preflight --require-safe-device
cargo run -- --backend 127.0.0.1:7878 --connect-timeout-ms 500 --read-timeout-ms 500 --doctor
cargo run -- --backend 127.0.0.1:7878 --mode chat --max-tokens 4096 --require-health --require-safe-device --connect-timeout-ms 500 --read-timeout-ms 500 --timeout-secs 60 --prompt "用一句话介绍 SmartSteam Forge"
cargo run -- --backend 127.0.0.1:7878 --mode business-cycle --max-tokens 4096 --require-health --require-safe-device --connect-timeout-ms 500 --read-timeout-ms 500 --timeout-secs 60 --prompt "检查当前业务联调链路"
```

## 当前能力

- Ratatui/Crossterm 终端界面。
- 顶部状态栏、消息区、底部输入框。
- Enter 发送，Ctrl+C 或 Esc 退出。
- Alt+Enter 或 Shift+Enter 在输入框内换行，输入框会随多行内容增高。
- 自动滚动到最新输出。
- PageUp/PageDown 回看长回答；Home 到顶部，End 回到底部并恢复自动跟随。
- `--prompt`/`--once` 可以在 CLI 里发一条消息并退出；`--smoke` 会发内置 smoke prompt，适合快速确认 mock 或真实 SSE 链路是否可用。
- `--mode chat|generate|business-cycle` 可以配合 one-shot 或 TUI 启动时选择接口。
- `--max-tokens <count>`、`--max-output-tokens <count>` 会把请求级输出预算作为
  `max_tokens` 发给 `rust-norion`；它不是上下文窗口。默认显式发送 262144，和
  Gemma 12B quality 链路的模型窗口对齐；如果要用后端默认，传 `--max-tokens default`。
- `--context-messages <count>`、`--context-window <count>`、`--max-context-messages <count>` 可以在 CLI 启动时设置短上下文窗口；默认 64，最小会按 2 处理。这个窗口限制最近 user/assistant 历史，避免长会话无限膨胀；项目 notes 和 resume 摘要仍按各自字符上限单独注入。
- `--mock --mode business-cycle --smoke` 会模拟 stage、final payload 和多行 gate report，适合先验证 CLI/TUI 的流式展示、自动换行和 gate report 解析，不需要启动真实 Gemma。
- `start-gemma-forge.cmd -CheckOnly` 是真实启动前的安全检查；它只读取本机状态，不启动任何进程，并会显示真实启动将使用的隔离状态目录。显存低于默认门槛时，优先关闭占 GPU 的程序；只有明确接受 tight VRAM/CPU fallback 风险时才使用 `-Force`。
- `--health`/`--check` 只读取后端 `/health`，不会触发 Gemma 推理；`--timeout-secs` 可以给真实 one-shot/stream 调用设置总等待时间，避免长时间卡住终端。
- `--hygiene` 会读取后端 `/v1/experience-hygiene`，显示经验库总数、污染候选、candidate ids、样本预览、长经验索引统计和索引噪声样本；不会触发 Gemma 推理。
- `--hygiene-quarantine` 会调用后端 quarantine dry-run，默认 `apply=false`，只显示会移动哪些经验记录，不会重写 `.ndkv`；`--hygiene-limit <count>` 可以调整样本数量。
- `--repair`/`--repair-dry-run` 会调用后端 `/v1/experience-repair` dry-run，默认 `apply=false`，只显示可从 clean gist 修复的旧格式经验和修复后预计剩余噪声；`--repair-limit <count>` 可以调整样本数量。
- `--audit`/`--cleanup-audit` 会聚合 hygiene 报告、quarantine dry-run 和 repair dry-run，默认 `writes_experience_state=false`，不会触发 Gemma 推理，也不会写 `.ndkv`；`--audit-limit <count>` 可以调整样本数量。
- `--preflight`/`--ready` 会执行 readiness 预检并退出，不会进入 TUI，也不会触发 Gemma 推理；配合 `--require-safe-device` 时会额外拦截 Gemma 12B CPU/disk-first。
- `--doctor`/`--diagnose` 会输出 backend 目标、health、readiness、safe-device 和下一步建议；如果后端 `/health` 暴露了 device plan，还会显示 `device/lane/memory` 和 12B CPU-first warnings。它只做健康探测，不会触发 Gemma 推理。
- 如果后端 `/health` 报告 `experience_hygiene.clean=false`、存在 `experience_hygiene.quarantine_candidates`，存在 `experience_hygiene.repair.repairable_legacy_metadata_lessons` / `repairable_index_records`，或经验索引 `retrieval_ready=false` / `risk_level=blocked`，`--preflight`、`--require-health` 和 TUI `/ready` 会失败并阻止发送 prompt，避免脏经验继续污染对话。
- `--sessions [all|passed|failed]` 会在 CLI 里列出 transcript 索引，不会连接后端、不会启动 TUI、也不会创建新的 transcript；`--session-limit <count>` 可以调整输出数量。
- `--summary [index|id]` 会在 CLI 里为 transcript 写出确定性的 Markdown 摘要，同样不会连接后端。
- 新版后端 `/health` 会暴露结构化 preflight 字段：`readiness_ok`、`readiness_failures`、`safe_device_ok`、`safe_device_failures` 和 `experience_hygiene`。Forge 会优先使用这些字段判断是否拦截 prompt；旧后端没有这些字段时仍会使用兼容的 summary/warnings 解析。
- `--connect-timeout-ms` 和 `--read-timeout-ms` 可以缩短后端连接/readiness 探测等待时间，适合本机联调时快速发现 `7878` 没启动或网络异常。`--read-timeout-ms` 控制单次 read 的轮询/heartbeat 间隔，不是整次流式请求总超时；Gemma 首 token 慢或长回答时，应调大 `--timeout-secs`。
- `--require-health` 会在 one-shot prompt 发送前或 TUI 启动前先检查 readiness；TUI 用这个参数启动后还会开启发送前 readiness guard。如果 `rust-norion` 没启动、engine 正忙，或 `runtime_mode=gemma-http` 但 Gemma runtime 不可达，会先失败，不会进入推理等待。
- `--require-health` 也会拦截脏经验库和 blocked 经验索引。先用 `curl.exe -s http://127.0.0.1:7878/v1/experience-hygiene` 查看候选；如果有 quarantine candidates，先 dry-run `curl.exe -s -X POST http://127.0.0.1:7878/v1/experience-hygiene/quarantine -H "Content-Type: application/json" -d "{\"limit\":20}"`；如果有 repairable legacy metadata 或 repairable index records，再 dry-run `cargo run -- --repair --repair-limit 20`；如果阻塞来自 `retrieval_ready=false` 或 `risk_level=blocked`，先跑只读 `cargo run -- --audit --audit-limit 20` 看 hygiene、quarantine 和 repair 投影。
- `--require-safe-device` 会额外拦截 Gemma 12B CPU/disk-first warnings；它适合真实 12B 联调，避免误把长 prompt 发到 CPU fallback。需要刻意测 CPU fallback 时，可以不用该参数，或在 TUI 内 `/safe-device off`。
- `/help`、`/clear`、`/status`、`/quit`。
- `/cancel` 会取消当前 TUI 正在接收的 provider stream；如果 12B 推理仍在后端清理中，可以再用 `/ready` 看是否空闲。
- `/new` 会开启新的 JSONL transcript，并清空当前短上下文。
- `/status` 会读取 `rust-norion /health`，显示 runtime、Gemma 可达性、busy 状态、最近一次推理耗时和 token 信息。
- `/strict-status [summary-json-path]` 会读取 evolution-loop 的严格状态摘要，显示 daemon
  round、readiness、helper roles、test-gate、远端模型池和 backend model；只读、不启动进程、不发送 prompt。
- `/ready` 会执行和 `--require-health` 相同的 readiness 检查，适合在 TUI 里发送真实 prompt 之前确认后端和 Gemma runtime 是否已经准备好。
- `/hygiene` 会在 TUI 内显示经验库卫生报告和索引噪声样本，不发送 prompt。
- `/hygiene dry-run [limit]` 会在 TUI 内显示隔离 dry-run 计划；Forge 不提供 apply 命令，避免误写主经验库。
- `/repair dry-run [limit]` 会在 TUI 内显示旧格式经验修复 dry-run 计划；Forge 不提供 apply 命令，避免误写主经验库。
- `/audit [limit]` 会在 TUI 内聚合显示 hygiene、quarantine dry-run 和 repair dry-run；只读、不发送 prompt、不触发 Gemma。
- `/retrieve [limit] <prompt>` 会在 TUI 内预览该 prompt 会召回哪些经验，显示 index context 使用情况、噪声过滤计数、match id 以及后端返回的 runtime model/device/KV 诊断字段；它不发送 prompt、不触发 Gemma。
- `/doctor` 或 `/diagnose` 会在 TUI 内输出同样的诊断报告和下一步建议，不会发送 prompt。
- `/doctor` 如果看到经验库污染，会提示 `/v1/experience-hygiene`、quarantine dry-run、legacy metadata repair 和 index repair dry-run 命令；真正应用隔离或修复仍需要你显式调用带 `apply=true` 的后端接口或 CLI。
- `/guard on|off` 会切换 TUI 发送前 readiness guard；guard 开启后，如果 readiness 失败，普通 prompt 会被拦截并保留在输入框里，方便后端恢复后重试。
- `/safe-device on|off` 会切换 TUI 发送前 safe-device guard；开启后，如果后端 health 报告 Gemma 12B 是 CPU/disk-first，普通 prompt 会被拦截并保留在输入框里。
- `/mode chat|generate|business-cycle` 可以切换 `rust-norion` 的流式接口。
- `/output raw|enhanced`、`/profile coding|general|writing|long` 可以切换输出和提示 profile。
- `/feedback 0.0..1.0`、`/self on|off` 可以调整 business-cycle 测试参数。
- `/max-tokens <count>` 或 `/max-output-tokens <count>` 会在 TUI 内调整请求输出预算；
  `/max-tokens default`、`off`、`none` 会恢复后端默认。
- 模型池命令 `/pool route ...`、`/pool call ...` 也会带上当前输出预算；返回里的
  `configured_max_tokens`、`effective_max_tokens` 和 `max_tokens_clamped` 可以用来判断
  是不是某个低优先级 worker 把输出预算截短了。
- `/pool status` 会显示 `launch_block_reason`、`chain_classification`、`quality_context_tokens`、
  `quality_context_required_tokens`、`quality_context_sufficient` 和 `quality_default_max_tokens`。
  `min_context_tokens` 是全池 ready worker 的最小窗口，可能被 summary/index 小模型拉低；
  判断 12B quality 链路是否足够大时看 `quality_context_sufficient=true`，并确认
  `quality_context_tokens >= quality_context_required_tokens`。
  `/pool route ...` 也会带出同一组 `quality_context_*` 字段；如果它显示
  `quality_context_sufficient=false`，Forge 不会继续把 `/pool call ...` 发送到 worker。
  `/pool route ...`、`/pool call ...` 的 `pool_dispatch` 会显示实际选中的 worker、`context_window`、
  默认/有效输出预算和 clamp 原因。
- `/rust-check inline <code>` 或 `/rust-check file <path>` 可以给 business-cycle 请求附带 Rust 编译检查代码。
- `/rust-check edition <2021|2024>`、`/rust-check case <name|off>`、`/rust-check off` 可以调整或清空 Rust check 设置。
- `/show` 会显示当前 mode/output/profile/feedback/self-improve、rust-check 设置和短上下文长度。
- `/context` 或 `/ctx` 会显示当前上下文预览，不发送 prompt；输出包含 `context_budget`，例如下一次 chat 消息数、会携带的短历史条数、被丢弃的短历史条数、`max_context_messages`、pinned context 数量/字符数、resume 摘要、项目 notes 的字符预算，以及 `model_pool_index_active=latest_delimited|latest_legacy_undelimited|none`。
- `/context-window <count>`、`/context-messages <count>` 或 `/ctx-window <count>` 会在 TUI 内调整短上下文窗口，并立即回显新的 `context_budget`。
- `/sessions` 会列出最近 transcript，包含第一条 user、最后一条 assistant、最近 final payload 摘要、最近 gate outcome、gate report 数量、行数和路径。
- `/sessions` 也会显示最近 transcript 的 preflight 数量和最近 preflight 摘要，方便定位是否是后端未启动、Gemma runtime 不可达或 safe-device 拦截。
- `/sessions failed|passed|all` 可以按最近 business-cycle gate outcome 过滤 transcript。
- `/resume [index|id]` 会按 `/sessions` 的序号或 session id/id 前缀恢复 transcript 的最近短上下文，并把该 transcript 的摘要作为有上限的 system context 注入下一次 chat。
- `/summary [index|id]` 会为 transcript 生成确定性的 Markdown 摘要。
- `/notes` 会显示项目固定笔记；`/notes add <text>` 追加一条笔记，`/notes set <text>` 替换全部笔记，`/notes clear` 清空笔记。笔记写入 `state/project_notes.md`，作为有上限的 pinned context 注入请求，不会和短历史或 transcript 混在一起。
- `/index-notes` 会只显示模型池 `index` worker 写入的索引区块，并用 `active=true` 标出当前会进入请求上下文和 `/retrieve` 的最新完整索引；`/index-notes clear` 会清理所有索引区块，包括 legacy 未闭合块，不会清空手写项目笔记。
- `/clear` 会同时清空界面消息和当前短上下文，避免下一轮测试带着旧对话。
- 默认通过 `/v1/chat-stream` 接 `rust-norion` SSE 流。
- 如果后端连接在 `done` 事件前关闭，Forge 会把它标记为截断错误；不会把半截回答写入短上下文或 transcript 的 assistant 消息。
- final payload 会显示为可读摘要，例如 runtime model、token count、uncertainty、rust-check/self-improve/state/trace gate 状态；如果 final answer 和流式草稿不同，会用 final answer 替换最后一条助手消息。
- business-cycle final payload 会额外显示多行 gate report，便于快速判断 overall/generate/feedback/rust-check/self-improve/state/trace 是否通过。
- 真实 provider 会把 prompt、answer、final payload、gate report 和错误写入 `state/sessions/session_*.jsonl`。
- provider/session/UI 分层，避免把前端、HTTP、会话记忆堆到一个文件里。

## 会话记录

默认 transcript 路径：

```text
D:\rust-norion\tools\smartsteam-forge\state\sessions\session_*.jsonl
```

每行是一条 JSONL 事件，便于后续做摘要、索引、回放和业务联调复盘。
`/show` 会显示当前 transcript 路径；`/new` 会轮换到新的 transcript 文件。
`/sessions` 会扫描当前 `state/sessions` 目录，默认输出最近 50 个 transcript 的轻量索引；如果 transcript 有 business-cycle gate report，会直接显示 `gate=PASS` 或 `gate=FAIL`。`/sessions failed` 和 `/sessions passed` 会先按最近修改时间排序，再过滤 gate outcome，再按 `--session-limit <count>` 或 TUI 命令里的可选 limit 截断。
`/resume` 默认恢复最近一个 transcript；`/resume 2` 恢复 `/sessions` 输出里的第 2 个；`/resume session_...` 可按 id 或 id 前缀恢复。
恢复时只加载最近的短上下文窗口，不会把整份 transcript 塞回 prompt；默认窗口为 64 条消息，可用 CLI `--context-messages <count>` 或 TUI `/context-window <count>` 调整。摘要上下文最多约 4000 字符，`/clear` 会同时清空短上下文和摘要上下文。
`/summary` 默认摘要最近一个 transcript；`/summary 2` 或 `/summary session_...` 可摘要指定 transcript。
CLI 里也可以不用进入 TUI 直接运行 `cargo run -- --sessions failed` 或 `cargo run -- --summary 1`，适合快速复盘 business-cycle gate 是否通过。
摘要文件会写到同目录下的 `session_*.summary.md`，包含消息计数、health/preflight/doctor 计数、final payload 计数、错误计数、最近 user、assistant、preflight 和 final payload 片段。
如果 transcript 中有 business-cycle gate report，摘要也会记录 gate report 计数和最近一次 gate report，`/resume` 注入的摘要上下文也会携带这段复盘信号。

## 验证

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo test
cargo fmt --check
```

`--mock` 模式不需要模型；真实模式需要先启动 Gemma runtime 和
`rust-norion` 后端服务。
