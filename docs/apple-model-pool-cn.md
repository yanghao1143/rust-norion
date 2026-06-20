# Apple 远端模型池联调说明

SmartSteam/Steam Forge 的远端 Apple 机器应当作为“模型盒子”，不要作为 Rust
源码开发机。远端只放模型文件、推理二进制和启动脚本；`rust-norion` 后端、Web
Lab、Forge CLI/TUI、evolution-loop 和项目状态目录仍留在 Windows 本机。

## 推荐拓扑

推荐先跑一个常驻 12B quality worker，再按资源情况增加小模型 worker：

```text
Windows 开发机
  Forge / Web Lab / rust-norion backend
  127.0.0.1:7979 -> local backend
  127.0.0.1:8789 -> Web Lab
  SSH tunnel:
    127.0.0.1:8686 -> Apple quality 12B
    127.0.0.1:8687 -> Apple summary small/Q4
    127.0.0.1:8688 -> Apple review small/Q4
    127.0.0.1:8689 -> Apple test-gate small/Q4
    127.0.0.1:8690 -> Apple index small/Q4
```

12B 不建议盲目多开。多个 12B 实例会重复占用统一内存和 KV cache，还会抢
Metal/GPU 内存带宽；通常会让首 token 和整体交互更慢。更有效的做法是让 12B 只
处理 `quality/chat/business-cycle`，把摘要、审查、索引、测试门禁交给小模型或低量化
worker。

## 多模型是否有用

有用，但有效果的前提是“按职责分工”，不是“把同一个 12B 复制几份”。同一台 Apple
Silicon 的多个本地模型共享统一内存、Metal/GPU、CPU 和磁盘 I/O。一个 12B Q8 worker
已经会吃掉大量内存和 KV cache；同时跑两个或更多 12B，通常只会让每个请求排队更久、
首 token 更慢、Memory Pressure 更高。

推荐的开发加速方式：

| Worker | 建议模型 | 作用 | 是否能跳过 12B |
| --- | --- | --- | --- |
| `quality` | 12B/Q8 或主质量模型 | 主对话、架构决策、最终合成、业务循环 | 不能 |
| `summary` | 小模型/Q4 | 日志、ledger、长输出摘要 | 可以独立跑 |
| `review` | 小模型/Q4 | 快速风险扫描、补充审查意见 | 只能辅助 |
| `test-gate` | 小模型/Q4 | 解释测试失败、建议下一条验证命令 | 不能替代真实测试 |
| `index` | 小模型/Q4 | 仓库地图、检索预过滤、上下文裁剪 | 可以独立跑 |

这样多模型才会对开发产生实际效果：小 worker 把便宜但频繁的任务提前完成，12B
只处理需要质量和长上下文的环节。最终是否推进目标，仍由 Rust 编译、测试、模型池
质量门禁、`business-cycle` 和人工确认共同决定。

## 并发建议

先按这个顺序扩容：

1. 只跑 `quality`，确认 `quality_context_sufficient=true`、`runtime_device=metal`
   或 `runtime_accelerator=metal`。
2. 加一个 `summary` worker，用短输出验证小模型不会拖慢 12B。
3. 再加 `review` 或 `index`，观察 Activity Monitor 的 GPU History、CPU、Memory
   Pressure 和 `/pool-status`。
4. 只有内存压力长期绿色、首 token 没变差，再考虑 `test-gate`。

不要把 `quality` 做成多副本来掩盖慢的问题。如果 12B 慢，优先检查 Metal/GPU 是否
启用、`gpu_layers` 是否大于 0、上下文窗口是否过大、当前是否有长请求占着
`engine_busy`，以及 prompt 是否被经验库或索引塞得太长。

## 启动入口

本机先用只读检查确认 manifest、端口和 tunnel 计划：

```powershell
cd D:\rust-norion
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly
```

启动常驻链路：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd
```

如果 Apple 上已经放好了小模型，再开启多 worker：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf
```

只启动链路、不进入 Forge TUI：

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -NoForge
```

## 必看门禁

启动后先查只读状态，不要急着发 prompt：

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --backend 127.0.0.1:7979 --model-pool-status
cargo run -- --backend 127.0.0.1:7979 --model-pool-route quality
```

12B quality 链路合格时应看到：

```text
quality_ready=true
quality_context_tokens=262144
quality_context_required_tokens=262144
quality_context_sufficient=true
launch_allowed=true
```

如果远端 worker 的 `/v1/models` 或代理 metadata 暴露运行时信息，`/pool-status` 和
`/pool-route` 还会显示：

```text
runtime_backend=llama.cpp
runtime_device=metal
runtime_accelerator=metal
gpu_layers=99
```

这些字段是 worker 自报的诊断线索，不是本机强制检测。字段为空时不代表一定没用
Metal，只代表当前 worker API 没有报告；字段显示 `cpu`、`runtime_accelerator=unknown`
或 `gpu_layers=0` 时，要优先怀疑 CPU fallback。

如果 `quality_context_sufficient=false`，说明远端 12B 虽然可能健康，但实际上下文
窗口不够。此时 `/pool route`、`/pool call` 和 evolution-loop 的
`-RequirePoolRoute` 都应阻止继续发送真实 prompt。

`min_context_tokens` 不是判断 12B 的字段。它是全池 ready worker 的最小窗口，可能被
summary/index 小模型拉低。判断 12B 质量链路只看 `quality_context_*`。

后端、Forge 和 Web Lab 还会显示 `capacity` 摘要，用来判断是否应该加 worker：

```text
capacity policy=one_quality_plus_small_helpers
capacity expansion_allowed=false
capacity recommendation=verify_worker_runtime_metadata_before_expansion
capacity healthy_helper_worker_count=1
capacity metal_worker_count=2
capacity cpu_worker_count=0
capacity unknown_runtime_worker_count=1
capacity zero_gpu_layer_worker_count=0
```

`expansion_allowed=false` 不是模型回答质量失败，而是运维保护：先恢复 quality gate、
修掉明确的 CPU fallback，或补齐 worker runtime metadata，再继续增加并发。推荐值
`restore_quality_gate_first` 表示先别启动小 worker；`add_summary_worker_first` 表示
可以先加摘要 worker；`add_review_or_index_worker_after_short_smoke` 表示先跑短 smoke，
再加 review 或 index。

## Apple 侧资源诊断

确认是否真的走 Metal/GPU：

- 推理服务启动日志应显示 Metal/Accelerate/MPS 之类后端，而不是纯 CPU fallback。
- Activity Monitor 里看 GPU History、Memory Pressure 和目标推理进程的内存。
- 如果 GPU 基本不动、CPU 长时间满载、首 token 很慢，通常是推理二进制没有启用 Metal
  或模型/量化格式触发了 CPU fallback。
- 如果 Memory Pressure 变黄/红，减少 worker 数量或降低小模型量化，不要继续多开
  12B。

## 远端目录约束

远端建议只保留：

```text
/Users/xinghuan/smartsteam-model-box/bin/...
/Users/xinghuan/smartsteam-model-box/models/...
/Users/xinghuan/smartsteam-model-box/logs/...
```

不要把 `D:\rust-norion` Rust 源码、状态库、经验库或 Hugging Face token 放到 Apple
机器。模型文件从内网传过去，避免远端重复拉取大模型拖慢网络。

## 联调策略

先让 quality 单实例稳定，再开小 worker。每次增加 worker 后，先跑只读 status 和
route，再跑极短 smoke prompt。只有当 `quality_context_sufficient=true` 且
`pool_dispatch` 显示的 worker、context、max token 都符合预期，才让 Web Lab、
Forge TUI 或 evolution-loop 长时间联调。

只读检查模型池整体形状：

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --backend 127.0.0.1:7979 --pool-smoke
```

让 evolution-loop 参与联调时，先启用模型池对齐门禁。它会刷新
manifest/status/route artifacts，并在发送真实 prompt 前检查：

- manifest 计划的 `quality/summary/review/index/test-gate` 是否都出现在 status。
- status 是否出现未规划 worker。
- primary route 和 helper stage routes 是否允许。
- `quality` 12B 是否没有超过 `max_quality_12b_workers=1`。
- 经验库索引是否达到 `quality_score>=0.92`，并且 `retrieval_ready=true`。

示例：

```powershell
cd D:\rust-norion
cargo run --manifest-path tools\evolution-loop\Cargo.toml -- `
  --backend 127.0.0.1:7979 `
  --refresh-pool-artifacts `
  --pool-capacity-gate `
  --pool-alignment-gate `
  --experience-audit-gate `
  --min-index-quality-score 0.92 `
  --pool-stage-route-task-kinds summary,review,index,test-gate `
  --pool-stage-route-gate `
  --rounds 1 `
  --prompt "用当前模型池总结最近一次联调状态，并给出下一条最小验证命令"
```

如果 helper 断了或 `index` 没接进来，命令会在 prompt 前失败，并打印
`missing_manifest_helper_roles`、`missing_status_helper_roles`、
`missing_status_roles`、`unplanned_status_roles` 或
`route_blocked_or_failed`。其中 `missing_manifest_helper_roles` 说明
manifest 还没有规划该角色，`missing_status_helper_roles` 说明角色已在
策略中要求，但实际 worker 状态里还没上线。
