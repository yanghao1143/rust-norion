# rust-norion

`rust-norion` is a Rust prototype for a local Noiron-style self-evolving
inference control layer for self-developed Transformer runtimes.

`rust-norion` 是一个用 Rust 编写的本地 Noiron 风格自进化推理控制层原型，默认面向自主训练的
Transformer 运行时。

## Project Goal / 项目目标

The goal is to build a practical, sovereignty-first local inference control
engine that can make a self-developed model backend behave more adaptively over
time without retraining model weights on every interaction.

本项目目标是构建一个实用、自主可控优先的本地推理控制引擎，让自研模型后端在不频繁重训权重的前提下，能够随着使用逐步调整推理策略、记忆选择和计算分配。

The optimized target combines five non-negotiable requirements:

优化后的目标由五个硬约束共同定义：

- self-developed model stack: the default backend is a self-trained
  Transformer-family model, not external weights
- anti lock-in: no closed model service, vendor-only runtime, or opaque
  quantization path in the core engine
- extreme local deployment: offline-first, lightweight, disk-backed memory,
  and ultra-long-context control for consumer or edge hardware
- universal device adaptation: laptops, desktops, workstations, servers,
  phones, tablets, wearable/XR/TV targets, embedded boards, browser-WASM,
  microcontroller-class tiny targets, edge/robot/vehicle devices, NPU/AI
  accelerator devices, and heterogeneous multi-GPU machines should all map into
  explicit hardware profiles that tune latency, KV budgets, routing pressure,
  and hierarchy weights, then into execution plans with portable fallbacks
- frontier algorithms as owned implementations: use public papers as
  inspiration, but implement attention, memory, quantization, routing,
  reflection, and scheduling locally in Rust

- 自研模型栈：默认后端是自主训练的 Transformer 系列模型，而不是外部权重
- 规避卡脖子：核心引擎不绑定闭源模型服务、厂商专用运行时或不透明量化路径
- 极致本地化部署：离线优先、轻量化、磁盘记忆、面向消费级/边缘硬件的超长上下文控制
- 全设备适配：笔记本、台式机、工作站、服务器、手机、平板、可穿戴/XR/TV、嵌入式板卡、浏览器 WASM、微控制器级 tiny 目标、边缘/机器人/车载设备、NPU/AI 加速器设备以及异构多 GPU 机器，都应映射到明确的硬件 profile，用于调整延迟、KV budget、路由压力、层级权重，并生成带可移植降级路径的执行计划
- 前沿算法自主实现：公开论文只作为思想来源，注意力、记忆、量化、路由、反思和调度都在 Rust 中本地实现

The project focuses on the control loop around inference:

项目重点不是从零实现完整大模型，而是实现推理外层闭环：

- multi-factor adaptive routing: decide when a token should use projection,
  local-window attention, global attention, or convolutional fusion based on
  entropy, task profile, context length, cache hit rate, latency pressure,
  hardware pressure, and device compute headroom; each task profile keeps its
  own adaptive threshold
- reinforced KV memory: store useful context, fuse similar memories, weaken bad
  memories, and persist local state
- task-aware hierarchy: shift global, local, and convolution-style compute
  weights for coding, writing, general reasoning, or long-document tasks, with
  profile-specific learned weights persisted across runs
- Rust-native Transformer refactor planning: express global, local-window, and
  convolutional-fusion layer plans as explicit Rust data structures
- reflection loop: score drafts, detect weak outputs, revise confidence, and
  decide what should become reusable memory
- backend abstraction: keep the control layer independent from the actual model
  runtime

- 多因子自适应路由：基于熵、任务类型、上下文长度、缓存命中率、延迟压力、硬件压力和设备算力余量，判断 token 应该走投影、局部窗口注意力、全局注意力还是卷积融合，并为不同任务 profile 分别维护自适应阈值
- 强化式 KV 记忆：保存有用上下文，融合相似记忆，削弱错误记忆，并持久化到本地
- 任务感知层级调度：针对代码、写作、通用推理、长文档任务调整全局/局部/卷积式计算权重，并按任务 profile 持久化学习后的权重
- Rust 原生 Transformer 重构规划：用明确的 Rust 数据结构表达全局注意力、局部窗口注意力和卷积融合层计划
- 反思闭环：评估草稿质量，发现薄弱输出，修正置信度，并决定是否写入可复用记忆
- 后端抽象：让控制层与真实模型运行时解耦

## Self-Owned Stack / 自主双栈

`rust-norion` is designed as an Agent Harness and test-time scaling control
plane around a self-owned Transformer runtime:

`rust-norion` 的架构定位是围绕自研 Transformer 运行时的 Agent Harness 与
Test-time Scaling 控制平面：

- model runtime: owns tokenizer, embeddings, weights, native context window,
  forward kernels, and optional KV import/export
- control plane: owns recursive scheduling, adaptive routing, memory tiering,
  sparse context filtering, reflection, RLVR-style process rewards, experience
  replay, and persisted adaptive state
- stable boundary: `ModelRuntime` and `InferenceBackend` keep model iteration
  independent from routing, memory, and reflection iteration

- 模型运行时：负责 tokenizer、embedding、权重、原生上下文窗口、前向计算内核，以及可选的 KV 导入/导出
- 控制平面：负责递归调度、自适应路由、记忆分层、稀疏上下文筛选、反思、RLVR 风格过程奖励、经验回放和持久化自适应状态
- 稳定边界：通过 `ModelRuntime` 和 `InferenceBackend` 让模型迭代与路由、记忆、反思迭代解耦

## Sovereignty Scope / 自主可控范围

The default target is a self-trained Transformer-family model. The core project
does not depend on Gemma, Llama, Qwen, closed model services, or vendor-specific
runtime features. Public papers and open algorithm ideas can guide the design,
but quantization, attention routing, memory scheduling, reflection, and adaptive
state should be implemented as local Rust components.

默认目标是自主训练的 Transformer 系列模型。核心项目不依赖 Gemma、Llama、Qwen、闭源模型服务或厂商绑定运行时能力。可以借鉴公开论文和开放算法思想，但量化、注意力路由、记忆调度、反思闭环和自适应状态都应作为本地 Rust 组件自主实现。

All semantic filtering, gist generation, and memory scoring should prefer the
self-developed model's own tokenizer and embeddings. The project should not add
a second third-party encoder just to make memory retrieval work.

语义筛选、gist 生成和记忆评分优先复用自研模型自身的 tokenizer 与 embedding。项目不应为了记忆检索再引入第二套第三方编码器。

## Local Algorithm Stack / 本地算法栈

The target algorithm stack is model-weight independent:

目标算法栈与具体模型权重解耦：

- ultra-long context: Infini-style global/local KV separation, recursive
  long-context scheduling, hierarchical gist memory, and SpeContext-style
  sparse KV filtering
- lightweight KV system: self-owned 4/8-bit uniform KV quantization,
  reinforced KV-Fusion, time decay, semantic clustering, and Hot/Warm/Cold
  storage
- self-evolution loop: test-time scaling, RLVR-style rewards for control
  decisions, reflection scoring, drift gates, adaptive rollback, and experience
  replay
- Rust Transformer refactor: explicit layer templates for local-window,
  global-memory, and convolutional-fusion compute paths

- 超长上下文：Infini 风格全局/局部 KV 分离、递归长上下文调度、层级 gist 记忆、SpeContext 风格稀疏 KV 筛选
- 轻量 KV 系统：自研 4/8-bit uniform KV 量化、强化式 KV-Fusion、时间衰减、语义聚类和 Hot/Warm/Cold 分层存储
- 自进化闭环：Test-time Scaling、针对控制决策的 RLVR 风格奖励、反思评分、漂移门控、自适应回滚和经验回放
- Rust Transformer 重构：用显式层模板表达局部窗口、全局记忆、卷积融合等计算路径

## Current Status / 当前状态

This repository currently contains a working control-plane prototype. It does
not yet include the self-developed Transformer runtime or production inference
kernels.

当前仓库已经包含一个可运行的控制层原型，但还没有接入自研 Transformer 运行时或生产级推理内核。

Implemented modules:

已实现模块：

- `src/router.rs`: multi-factor adaptive router with task-profile-specific attention thresholds and hardware-aware compute pressure
- `src/adaptive_state.rs`: persisted router, hierarchy, and tier-plan control state
- `src/benchmark.rs`: built-in benchmark cases, regression gates, KV quantization accuracy/latency gate, and persistent roundtrip reuse gate
- `src/disk_kv.rs`: append-only disk-backed KV store
- `src/drift.rs`: drift guard for memory-write gates, runtime-KV admission, used-memory penalties, and adaptive-state rollback
- `src/infini_memory.rs`: Infini-style global/local memory planner with sparse token-budget filtering
- `src/kv_cache.rs`: reinforced KV fusion cache with disk persistence, retention policy, and batch semantic compaction for near-duplicate memories
- `src/kv_exchange.rs`: shared runtime KV block type for import/export between Noiron and model runtimes
- `src/kv_quant.rs`: self-owned 4/8-bit uniform KV vector quantization
- `src/local_runtime.rs`: Rust-native self-developed Transformer-style runtime prototype implementing tokenizer, embedding, generation, and KV import/export
- `src/recursive_scheduler.rs`: native-window-aware recursive long-context scheduler with hardware-bounded execution waves
- `src/tiered_cache.rs`: Hot/Warm/Cold memory tier scheduler with migration traces
- `src/token_stream.rs`: generated-token window monitor for router feedback
- `src/trace.rs`: JSONL trace writer for routing, hierarchy, KV, recursion, hardware, reflection diagnostics, drift, reward, and memory counters
- `src/experience.rs`: structured reflection experience store with route budget, KV usage traces, persisted reflection issues, and revision actions
- `src/experience_replay.rs`: reward-ranked experience replay planner that can reinforce or penalize used, stored, gist, and runtime-KV memories using reward and reflection-diagnostic signals
- `src/gist_memory.rs`: hierarchical document/section/paragraph gist memory generator
- `src/hardware.rs`: device-agnostic hardware pressure, best-effort auto probing, device coverage descriptors and aliases, compute allocation, execution-plan selection, and a device compatibility gate for CPU-only, integrated GPU, discrete GPU, unified-memory, mobile, embedded, NPU/AI accelerator, multi-GPU, edge, and server profiles
- `src/process_reward.rs`: RLVR-style process reward scoring for control decisions, including structured reflection issue penalties
- `src/transformer.rs`: Rust-native Transformer layer refactor planner
- `src/hierarchy.rs`: task-profile hierarchy controller with profile-specific learned weights
- `src/reflection.rs`: draft reflection, structured issue/severity diagnostics, revision actions, and memory admission logic
- `src/runtime.rs`: model runtime adapter contract for real LLM backends, including metadata, tokenizer, embedding, KV import/export ABI hooks, and structured JSON command-runtime request/response wiring
- `src/state_inspect.rs`: local state inspection report for memory, experience, reflection diagnostics, adaptive router, hierarchy, and tier counts
- `src/engine.rs`: closed-loop Noiron engine and `InferenceBackend` trait
- `src/main.rs`: CLI demo using `HeuristicBackend`

## Non-Goals / 非目标

This prototype does not claim that KV memory is equivalent to model-weight
training, and it does not claim to be a complete LLM runtime.

本原型不声称 KV 记忆等同于模型权重训练，也不声称自己已经是完整的大模型运行时。

The near-term engineering target is to make the control loop measurable,
testable, and replaceable before connecting a real model backend.

近期工程目标是先让控制闭环可测、可运行、可替换，再接入真实模型后端。

## Run / 运行

```powershell
cargo run -- --profile coding "Build a Rust Noiron inference engine"
```

Trigger recursive long-context scheduling with a small demo native window:

```powershell
cargo run -- --profile long --native-window 8 --chunk-tokens 6 --chunk-overlap 2 --merge-fan-in 2 "one two three four five six seven eight nine ten eleven twelve"
```

Replay high/low reward experience before the next inference:

```powershell
cargo run -- --replay 4 --profile coding "Improve Rust Noiron routing from prior experience"
```

Inspect persisted local state without running inference:

```powershell
cargo run -- --inspect-state --inspect-limit 5
```

查看本地持久化状态，但不执行推理：

```powershell
cargo run -- --inspect-state --inspect-limit 5
```

Write one structured JSONL trace record for benchmark comparison:

```powershell
cargo run -- --trace target/noiron-trace.jsonl --profile coding "Trace Rust Noiron routing and memory decisions"
```

Run the built-in benchmark suite and append one JSONL trace record per case:

```powershell
cargo run -- --benchmark target/noiron-benchmark.jsonl
```

Run the same suite as a regression gate:

```powershell
cargo run -- --benchmark target/noiron-benchmark.jsonl --benchmark-gate --benchmark-min-quality 0.6 --benchmark-min-reward 0.5 --benchmark-max-drift-blocks 0 --benchmark-max-drift-rollbacks 0
```

Run the KV quantization gate for reproducible 4/8-bit compression accuracy,
payload ratio, and latency checks:

```powershell
cargo run -- --kv-quant-gate
```

运行 KV 量化门禁，检查 4/8-bit 压缩误差、payload 压缩比和耗时：

```powershell
cargo run -- --kv-quant-gate
```

Run a persistence roundtrip gate that writes state, reloads it, and verifies the
second local-runtime pass uses persisted memory, persisted experience, and
imported runtime KV:

```powershell
cargo run -- --benchmark-roundtrip --memory target/roundtrip-memory.ndkv --experience target/roundtrip-experience.ndkv --adaptive target/roundtrip-adaptive.ndkv --profile coding "Verify persistent Noiron memory reuse"
```

运行持久化 roundtrip 门禁：第一轮写入状态，第二轮重新加载，并验证 local runtime
确实使用了持久化 memory、experience 和导入的 runtime KV：

```powershell
cargo run -- --benchmark-roundtrip --memory target/roundtrip-memory.ndkv --experience target/roundtrip-experience.ndkv --adaptive target/roundtrip-adaptive.ndkv --profile coding "Verify persistent Noiron memory reuse"
```

Benchmark summaries include compacted memory counts and drift
watch/block/rollback counts, so memory-growth or safety regressions in the
self-evolution loop can fail the gate even when average quality still looks
acceptable.

Benchmark 汇总会包含 memory compaction 计数以及 drift watch/block/rollback 计数，因此即使平均质量看起来仍然合格，记忆膨胀或自进化安全门控退化也可以触发失败。

Apply universal device-profile hardware pressure hints:

```powershell
cargo run -- --device cpu --cpu-load 85 --ram-load 70 --disk-load 40 --profile long "Summarize a long local document"
```

Print the built-in device execution matrix:

```powershell
cargo run -- --list-devices
```

The matrix includes each profile's scope, common aliases, primary compute lane,
portable fallback lane, memory mode, runtime adapter hints, KV precision,
prefetch count, disk-spill policy, and recursive parallelism budget.

打印内置全设备执行矩阵：

```powershell
cargo run -- --list-devices
```

矩阵会列出每个 profile 的覆盖范围、常见别名、主计算通道、可移植降级通道、内存模式、
runtime adapter hint、KV 精度、预取数量、磁盘溢出策略和递归并行预算。

Run the device compatibility gate. It fails with exit code `2` if any supported
device profile loses its alias coverage, execution plan, KV budget, adapter
hints, or portable fallback path:

```powershell
cargo run -- --device-gate
```

运行全设备兼容门禁。如果任一设备 profile 缺失别名覆盖、执行计划、KV budget、
adapter hint 或可移植降级路径，命令会以退出码 `2` 失败：

```powershell
cargo run -- --device-gate
```

If `--device auto` is used, or no device is provided, the CLI performs a
best-effort local probe using OS, architecture, CPU parallelism, and common
GPU/NPU environment variables. Manual flags such as `--device`, `--cpu-load`,
`--gpu-load`, `--ram-load`, and `--disk-load` always override probe defaults.
If a manual device name or `NOIRON_DEVICE_PROFILE` value is not recognized yet,
the control plane deliberately falls back to the portable `cpu` profile instead
of failing or binding to an unknown vendor path.

使用 `--device auto` 或不指定设备时，CLI 会根据 OS、CPU 架构、CPU 并行度以及常见
GPU/NPU 环境变量做保守本地探测。`--device`、`--cpu-load`、`--gpu-load`、
`--ram-load`、`--disk-load` 等手动参数始终优先。如果手动设备名或
`NOIRON_DEVICE_PROFILE` 暂未被识别，控制平面会明确降级到可移植 `cpu`
profile，而不是失败或绑定到未知厂商路径。

Examples of accepted device profiles include `cpu`, `integrated`, `discrete`,
`uma`, `mobile`, `embedded`, `npu`, `multi-gpu`, `edge`, and `server`.
Common aliases such as `unknown`, `generic`, `x86_64`, `arm64`, `loongarch64`,
`laptop`, `steamdeck`, `directml`, `rtx`, `macbook`, `iphone`, `harmonyos`,
`wearable`, `snapdragon`, `hailo`, `ascend`, `rknn`, `microcontroller`, `wasm`,
`riscv`, `jetson`, `automotive`, `nas`, `datacenter`, `epyc`, and `hpc` map
into those profiles. Internally, profiles are also grouped into `tiny`,
`constrained`, `balanced`, `accelerated`, and `distributed` capability tiers so
the same control loop can scale down or up without binding to one vendor
device.

可用设备 profile 包括 `cpu`、`integrated`、`discrete`、`uma`、`mobile`、
`embedded`、`npu`、`multi-gpu`、`edge` 和 `server`。常见别名如 `unknown`、
`generic`、`x86_64`、`arm64`、`loongarch64`、`laptop`、`steamdeck`、
`directml`、`rtx`、`macbook`、`iphone`、`harmonyos`、`wearable`、
`snapdragon`、`hailo`、`ascend`、`rknn`、`microcontroller`、`wasm`、`riscv`、
`jetson`、`automotive`、`nas`、`datacenter`、`epyc` 和 `hpc` 会映射到这些
profile。内部还会按 `tiny`、`constrained`、`balanced`、`accelerated`、
`distributed` 能力档位调整策略，保证同一控制闭环可以在不同设备上降级或扩展。

Every hardware plan also emits a `DeviceExecutionPlan`: primary compute lane,
fallback lane, memory mode, candidate runtime adapters, hot/cold KV precision,
prefetch count, disk-spill policy, and recursive parallel chunk budget. These
are policy hints, not hard dependencies; a real self-developed runtime can pick
the first supported adapter and still fall back to portable Rust/CPU paths.

每个硬件计划还会生成 `DeviceExecutionPlan`：主计算通道、降级通道、内存模式、候选
runtime adapter、冷热 KV 精度、预取数量、磁盘溢出策略和递归 chunk 并行预算。这些是策略提示，不是硬依赖；真实自研 runtime 可以选择第一个已支持 adapter，同时始终保留 Rust/CPU 可移植降级路径。

Probe override environment variables include `NOIRON_DEVICE_PROFILE`,
`NOIRON_CPU_LOAD`, `NOIRON_GPU_LOAD`, `NOIRON_RAM_LOAD`, `NOIRON_DISK_LOAD`,
GPU hints such as `CUDA_VISIBLE_DEVICES`, `NVIDIA_VISIBLE_DEVICES`,
`HIP_VISIBLE_DEVICES`, `DIRECTML_VISIBLE_DEVICES`, and `WGPU_ADAPTER_NAME`,
edge hints such as `JETSON_MODEL_NAME`, and NPU hints such as `NOIRON_NPU` or
`NPU_VISIBLE_DEVICES`.

可用于覆盖探测结果的环境变量包括 `NOIRON_DEVICE_PROFILE`、`NOIRON_CPU_LOAD`、
`NOIRON_GPU_LOAD`、`NOIRON_RAM_LOAD`、`NOIRON_DISK_LOAD`，GPU 提示如
`CUDA_VISIBLE_DEVICES`、`NVIDIA_VISIBLE_DEVICES`、`HIP_VISIBLE_DEVICES`、
`DIRECTML_VISIBLE_DEVICES`、`WGPU_ADAPTER_NAME`，边缘设备提示如
`JETSON_MODEL_NAME`，以及 `NOIRON_NPU`、`NPU_VISIBLE_DEVICES` 等 NPU 提示。

Run through a local command runtime:

```powershell
cargo run -- --runtime-command ./self-transformer-cli --runtime-model-id noiron-dev-transformer --runtime-tokenizer noiron-bpe --runtime-native-window 32768 --runtime-embedding-dims 4096 --runtime-kv-exchange --runtime-arg "-p" --runtime-arg "{prompt}" --runtime-prompt-mode args "Build a Rust Noiron inference engine"
```

If `--runtime-prompt-mode stdin` is used, the formatted Noiron runtime request is
written to the child process stdin.

Use the structured JSON runtime ABI when the self-developed runtime can parse a
machine-readable request and return token/trace metadata:

```powershell
cargo run -- --runtime-command ./self-transformer-cli --runtime-wire-format json --runtime-prompt-mode stdin --runtime-model-id noiron-dev-transformer --runtime-native-window 32768 --runtime-embedding-dims 4096 "Build a Rust Noiron inference engine"
```

In JSON mode, stdin receives `rust-norion-runtime-request-v1` and stdout must
return `rust-norion-runtime-response-v1` with an `answer`, optional `tokens`,
and optional `trace` entries.

当自研 runtime 支持机器可读协议时，可以使用结构化 JSON ABI。JSON 模式下 stdin 会收到
`rust-norion-runtime-request-v1`，stdout 需要返回
`rust-norion-runtime-response-v1`，其中包含 `answer`，并可选包含 `tokens` 与
`trace`，方便 Noiron 控制层继续做 token 监控与反思。

Run through the built-in Rust-native local runtime prototype:

```powershell
cargo run -- --local-runtime --runtime-native-window 32768 --runtime-embedding-dims 128 --profile coding "Build a Rust Noiron runtime with KV exchange"
```

The local runtime is still a deterministic prototype, not a production LLM
kernel. Its purpose is to exercise the same self-developed runtime ABI that a
real Transformer runtime will implement: tokenizer, embedding, native context
window, KV import/export, token trace, and generation.

内置 local runtime 仍然是确定性原型，不是生产级大模型内核。它的作用是打通真实自研
Transformer runtime 将要实现的同一套 ABI：tokenizer、embedding、原生上下文窗口、KV
导入/导出、token trace 和生成接口。

By default, the demo writes local memory to `noiron-memory.ndkv`, structured
reflection experience to `noiron-experience.ndkv`, and adaptive router/hierarchy
state to `noiron-adaptive.ndkv`. These files are ignored by Git because they are
local runtime state.

demo 默认会把本地记忆写入 `noiron-memory.ndkv`，并把结构化反思经验写入
`noiron-experience.ndkv`，同时把自适应路由和层级权重状态写入
`noiron-adaptive.ndkv`。这些文件属于本地运行状态，已被 Git 忽略。

The memory file uses the append-only `DiskKvStore` format with quantized KV
vectors. Legacy `noiron-memory.tsv` files can still be loaded and are preserved
as `.legacy.tsv` backups when migrated to disk KV.

记忆文件使用追加式 `DiskKvStore` 格式，并保存量化后的 KV 向量。旧版
`noiron-memory.tsv` 仍可读取；迁移到磁盘 KV 时会保留为 `.legacy.tsv` 备份。

`NoironEngine::save_full_state` and `NoironEngine::load_full_state` round-trip
memory, experience, and adaptive state together, so a later run can retrieve
prior lessons and import persisted KV into the runtime.

`NoironEngine::save_full_state` 与 `NoironEngine::load_full_state` 会把记忆、经验和自适应状态一起往返保存，因此后续运行可以检索旧经验，并把持久化 KV 导入 runtime。

After each inference, the engine now applies retention and then batch
KV-Fusion compaction. Current-run memory ids are protected, while older
near-duplicate records can merge into the stronger entry to keep long-lived
local memory useful without growing indefinitely.

每次推理后，引擎会先执行 retention，再执行批量 KV-Fusion compaction。本轮正在使用或刚写入的
memory id 会被保护，旧的近重复记忆会合并到更强的条目里，避免长期本地记忆无限膨胀。

## Test / 测试

```powershell
cargo test
```

## Architecture / 架构

```mermaid
flowchart LR
    Prompt[Prompt] --> Embed[Local Embedding]
    Embed --> Memory[KV Fusion Cache]
    Memory --> Infini[Infini Memory Planner]
    Memory --> DiskKV[Append-Only Disk KV]
    Memory --> Tiers[Hot/Warm/Cold Tier Planner]
    Prompt --> Router[Adaptive Router]
    Prompt --> Hierarchy[Hierarchy Controller]
    Prompt --> Recursive[Recursive Scheduler]
    Prompt --> Hardware[Hardware Allocator]
    Hardware --> DeviceExec[Device Execution Plan]
    Prompt --> Experience[Experience Store]
    Hierarchy --> Transformer[Transformer Refactor Plan]
    Memory --> Backend[InferenceBackend]
    Infini --> Backend
    Recursive --> Backend
    Hardware --> Router
    Hardware --> Infini
    Hardware --> Hierarchy
    DeviceExec --> Backend
    DiskKV --> Memory
    Tiers --> Backend
    Experience --> Backend
    Transformer --> Backend
    Router --> Backend
    Hierarchy --> Backend
    Backend --> Draft[Draft Answer]
    Draft --> Stream[Token Stream Monitor]
    Draft --> Reflect[Reflection Loop]
    Reflect --> Drift[Drift Guard]
    Reflect --> Gist[Hierarchical Gist Memory]
    Reflect --> Reward[Process Reward]
    Stream --> Router
    Reflect --> Experience[Experience Store]
    Drift --> Experience
    Drift --> Memory
    Drift --> Adaptive
    Gist --> Experience
    Gist --> Memory
    Reward --> Experience
    Experience --> Replay[Experience Replay]
    Replay --> Router
    Replay --> Hierarchy
    Replay --> Memory
    Experience --> DiskKV
    Reflect --> Router
    Reflect --> Hierarchy
    Router --> Adaptive[Adaptive State]
    Hierarchy --> Adaptive
    Adaptive --> DiskKV
```

## Backend Integration / 后端接入

To connect a real model, implement `ModelRuntime` and wrap it in
`RuntimeBackend`, or implement `InferenceBackend` directly for a custom
self-developed runtime surface.

要接入真实模型，可以实现 `ModelRuntime` 并用 `RuntimeBackend` 包装，也可以为更定制的自研运行时控制面直接实现 `InferenceBackend`，替换当前 demo 使用的 `HeuristicBackend`。

`ModelRuntime` now exposes the self-developed runtime boundary explicitly:
metadata, tokenizer access, embedding access, optional KV import/export, and
generation. Unsupported capabilities have safe defaults so a command-line
runtime can still start with only `generate`.

`ModelRuntime` 现在显式暴露自研运行时边界：模型元数据、tokenizer、embedding、可选 KV 导入/导出以及生成接口。不支持的能力有安全默认值，因此命令行后端仍然可以只从 `generate` 起步。

`RuntimeBackend` reports the runtime's native context window back to the engine,
so recursive long-context scheduling can use the actual self-developed model
window instead of a hardcoded control-plane default.

`RuntimeBackend` 会把运行时的原生上下文窗口反馈给引擎，因此递归长上下文调度可以使用真实自研模型窗口，而不是固定的控制层默认值。

The CLI exposes this metadata through `--runtime-model-id`,
`--runtime-tokenizer`, `--runtime-native-window`, `--runtime-embedding-dims`,
`--runtime-kv-import`, `--runtime-kv-export`, and `--runtime-kv-exchange`.

CLI 通过 `--runtime-model-id`、`--runtime-tokenizer`、`--runtime-native-window`、`--runtime-embedding-dims`、`--runtime-kv-import`、`--runtime-kv-export` 和 `--runtime-kv-exchange` 暴露这些元数据。

When KV exchange is enabled, `RuntimeBackend` imports active non-cold memory
vectors as runtime KV blocks before generation. After generation, exported KV
blocks are attached to the draft; the engine stores them back into reinforced
memory only when reflection admits the answer as useful.

启用 KV 交换后，`RuntimeBackend` 会在生成前把活跃且非冷层的记忆向量导入为 runtime KV block。生成后，runtime 导出的 KV block 会随草稿返回；只有当反思模块认为答案有价值时，引擎才会把这些 KV 写回强化记忆。

Runtime KV import is capped by `DeviceExecutionPlan.kv_prefetch_blocks`, so
constrained devices can keep the runtime KV working set small while larger
accelerator profiles can prefetch more memory.

Runtime KV 导入数量会受 `DeviceExecutionPlan.kv_prefetch_blocks` 限制，因此低配设备可以保持较小 KV 工作集，高容量加速器设备则可以预取更多记忆。

Recursive long-context schedules are also grouped into execution waves using
`DeviceExecutionPlan.max_parallel_chunks`: constrained devices run fewer chunks
at once, while accelerated and distributed profiles can expose wider parallel
waves to the runtime.

递归长上下文调度也会根据 `DeviceExecutionPlan.max_parallel_chunks` 分组成执行 wave：
低配设备减少同时执行的 chunk，高容量加速器和分布式 profile 可以向 runtime 暴露更宽的并行 wave。

Expected integration loop:

预期接入流程：

1. embed prompt and retrieve local memory
2. read runtime metadata such as model id, tokenizer, native context window,
   embedding dimensions, and KV exchange support
3. compute route budget and hierarchy weights
4. plan single-pass or recursive chunk/merge scheduling for the native model window
5. adapt latency, KV budgets, and hierarchy weights to CPU-only, integrated GPU,
   discrete GPU, unified-memory, mobile, embedded, NPU/AI accelerator,
   multi-GPU, edge, or server devices, then select a portable execution plan
6. optionally replay high/low reward experience into router, hierarchy, and KV state
7. retrieve relevant reflection lessons from the experience store
8. import active KV memory into the runtime, call the real model backend, and
   collect exported runtime KV
9. reflect on the draft
10. generate document, section, and paragraph-level gist records
11. run drift gates before durable memory or runtime KV admission
12. score route, memory, hierarchy, latency, and admission with process rewards
13. reinforce or penalize memory, including accepted exported runtime KV
14. update or roll back routing threshold, hierarchy weights, and experience records

1. 对 prompt 做嵌入并检索本地记忆
2. 读取模型 id、tokenizer、原生上下文窗口、embedding 维度和 KV 交换能力等 runtime metadata
3. 计算路由预算和层级权重
4. 针对自研模型原生窗口规划单次推理或递归 chunk/merge 调度
5. 根据 CPU-only、集显、独显、统一内存、移动端、嵌入式、NPU/AI 加速器、多 GPU、边缘设备或服务器压力调整延迟、KV budget 和层级权重，并选择带降级路径的执行计划
6. 可选地把高/低 reward 经验回放到 router、hierarchy 和 KV 状态
7. 从经验库检索相关反思 lesson
8. 把活跃 KV 记忆导入 runtime，调用真实模型后端，并收集 runtime 导出的 KV
9. 对草稿答案做反思评估
10. 生成 document、section、paragraph 三级 gist 记忆
11. 在持久记忆或 runtime KV 准入前执行漂移门控
12. 对路由、记忆、层级、延迟和记忆准入做过程奖励评分
13. 强化或惩罚记忆，包括通过反思准入的 runtime 导出 KV
14. 更新或回滚路由阈值、层级权重和经验记录

## Roadmap / 路线图

The optimized roadmap is tracked in [`ROADMAP.md`](ROADMAP.md).

优化后的路线图维护在 [`ROADMAP.md`](ROADMAP.md)。

- replace heuristic embedding with model-side embeddings or compact vector
  encoders
- implement a self-developed Transformer runtime adapter
- expand mixed-precision 4/8-bit KV quantization benchmarks and policies
- add Infini-style global/local KV separation and sparse context filtering
- add recursive scheduling for inputs beyond the native model window
- add benchmark cases for long-context routing and memory reuse
- add configurable memory retention policies
- expand real-device probing and execution-plan calibration beyond explicit profiles and aliases
- expand the built-in benchmark suite into regression gates

- 用模型侧 embedding 或轻量向量编码器替换当前启发式 embedding
- 实现自研 Transformer 运行时适配器
- 扩展 4/8-bit 混合精度 KV 量化 benchmark 和策略
- 增加 Infini 风格全局/局部 KV 分离和稀疏上下文筛选
- 增加超过模型原生窗口输入的递归调度
- 增加长上下文路由和记忆复用 benchmark
- 增加可配置的记忆保留策略
- 扩展全设备硬件 profile、执行计划和真实设备探测适配
- 把内置 benchmark 套件扩展成回归门禁
