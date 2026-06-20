# Gemma GGUF Main Pool And MLX Experiment Slots

本文记录 GGUF 常驻主池与 MLX 实验槽的接入计划。它是文档-only 计划，不是执行手册；本窗口不 SSH、不下载模型、不启动或停止模型、不发送 prompt、不写模型权重。

## 当前已实践证据

总窗口实测证据：

- 远程主机是 Mac mini Apple M4 / 32GB unified memory。
- 磁盘约 `848GiB` 可用。
- `8686` 已运行 `Qwen3.6-14B-A3B-FableVibes-Q4_K_M.gguf`。
- llama.cpp health OK。
- `/v1/models` 显示约 `13.76B` params，`n_ctx=65536`。
- 模型池 `6/6` healthy，全部 Metal，`cpu_or_no_gpu=0`。
- daemon round `278` 已完成，`stale=false`、`ledger_lag=0`、`gate_failures=0`。

结论：当前有实证支持 `tvall43/Qwen3.6-14B-A3B-FableVibes-GGUF` 作为 llama.cpp/GGUF 常驻主池运行。该证据不能自动推广到 MLX/4bit 模型，因为 runtime、serve command、health endpoint、内存压力和模型格式都不同。

## 模型池边界

| 池 | 模型格式 | Runtime | 当前状态 | 允许用途 | 禁止混用 |
| --- | --- | --- | --- | --- | --- |
| GGUF 常驻主池 | GGUF | llama.cpp / llama-server | 已实践，6/6 healthy，全部 Metal | 常驻 daemon、Web Lab/Forge/CLI 的主推理链路 | 不要把 MLX 模型路径塞进 llama.cpp worker manifest |
| MLX 实验槽 | MLX 4bit | MLX runtime，例如 `mlx_lm.server` 或等价服务 | 未实践；远程当前未发现 `mlx_lm.server`；系统 `python3` 会触发 Xcode command line tools 缺失提示 | 单槽实验、隔离 preflight、资源评估 | 不要并入 GGUF 常驻池；不要影响 daemon 主链路 |

MLX 实验槽必须先证明 runtime 存在、可启动、可健康检查、可限制资源，并能在失败时不污染 GGUF 常驻池。未证明前，MLX 只能是候选实验方向，不是可调度 worker。

## 候选模型

候选模型的机器可读 planning manifest 位于：

```text
tools\gemma-chain\mlx-experiment-candidates.json
```

该 manifest 固定声明 `read_only=true`、`download_allowed=false`、`starts_process=false`、`sends_prompt=false`、`writes_model_weights=false`，并把 `pool_policy` 标为 `experimental_not_gguf_pool`。它只是候选清单和 preflight contract，不是下载、启动、SSH 或 prompt 授权。

| 候选 | 格式 | 建议顺序 | 原因 | 默认结论 |
| --- | --- | --- | --- | --- |
| `shuhulx/Qwopus3.5-4B-Coder-Fable5-v1-MLX-4bit` | MLX 4bit | 1 | 体量较小，适合作为 MLX runtime preflight 与资源压力探针 | 先单实验槽 |
| `usermma/Qwable-9B-Claude-Fable-5-mlx-4Bit` | MLX 4bit | 2 | 9B 更接近长期助手/实验上限，需要在 4B 通过后再试 | 只在 4B 通过后评估 |
| `usermma/Qwable-v1-mlx-4Bit` | MLX 4bit | 3 | 需先确认具体参数量、内存占用和服务形态 | 暂不进入常驻池 |
| `tvall43/Qwen3.6-14B-A3B-FableVibes-GGUF` | GGUF | 已运行 | 当前主模型，llama.cpp health 和 `/v1/models` 已验证 | 保持 GGUF 主池 |

## 接入验收矩阵

下表是计划和授权边界，不是自动执行队列。

| step | 目标 | 候选命令/证据 | SSH | downloads | starts_process | sends_prompt | 需要授权 | 通过标准 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `gguf_baseline_record` | 记录当前 GGUF 主池基线 | 读取 `status-with-model-cache.json`、`model-cache-status.json`、daemon status/report、`/v1/models` 实测摘要 | no for local artifacts; yes if refreshing live remote | no | no | no | no for local read; yes for live check | 14B GGUF 在 `8686` healthy，model pool `6/6`，Metal `6/6`，daemon `stale=false ledger_lag=0` |
| `mlx_runtime_inventory` | 确认远程是否具备 MLX runtime | `which mlx_lm.server`、`python3 -m mlx_lm --help`、venv/conda inventory | yes | no | no | no | yes | runtime 存在且命令不会触发 Xcode CLT 缺失；若不存在则停在安装计划 |
| `mlx_python_toolchain_preflight` | 确认 Python/MLX 工具链不破坏主机 | `python3 --version`、venv Python、`pip show mlx-lm`，只读检查 | yes | no | no | no | yes | 不触发系统安装弹窗；有隔离 venv/conda 路径；不写系统 Python |
| `mlx_4b_download_plan` | 准备 4B 模型下载方案 | HF repo、目标路径、预计磁盘、hash/manifest 计划 | yes if remote path check | yes if executed | no | no | yes | 下载路径与 GGUF cache 分离；磁盘余量足够；不覆盖 GGUF 模型权重 |
| `mlx_4b_single_slot_startcheck` | 只做 4B 单实验槽启动前检查 | 端口规划、内存预算、launch command dry-run、resource headroom | yes | no | no for dry-run | no | yes | 确认只占一个实验端口；不会改变 GGUF worker manifest；不会启动 daemon |
| `mlx_4b_live_smoke` | 4B MLX 服务最小 live smoke | 启动 MLX 服务、health/model endpoint、一次低 token smoke | yes | no if already downloaded | yes | yes | yes | 仅实验槽发送小 prompt；GGUF daemon 暂停或隔离；完成后记录资源和延迟 |
| `mlx_4b_teardown_or_residency_decision` | 决定 4B 是否保留实验槽 | 停止实验服务或保留为非 daemon worker，记录 PID/port | yes | no | may stop process | no | yes | 若保留，必须有监控和资源上限；若失败，清晰 teardown |
| `mlx_9b_preflight` | 4B 通过后评估 9B | 同 4B，但更严格内存/Swap/Metal headroom | yes | maybe | no for preflight | no | yes | 32GB unified memory 下仍有 headroom；不得挤压 GGUF 主池进入 swap |
| `mlx_9b_live_smoke` | 9B 单槽 smoke | 同 4B live smoke | yes | no if already downloaded | yes | yes | yes | 只允许单槽；不得与 14B GGUF 多模型长期并发，除非资源窗口证明安全 |
| `pool_integration_review` | 决定是否暴露给 Web Lab/Forge/CLI | 更新独立实验池 manifest 或文档；不改 GGUF 主池 | no for doc; yes for live status | no | no | no | yes | GGUF 常驻池与 MLX 实验池分开展示；下游不能把 MLX 当 llama.cpp worker |

## 内存与资源策略

Mac mini 32GB unified memory 不适合长期无边界地并发多个大模型。建议：

1. 保持 14B GGUF 主池为常驻路径，先不要动现有 `8686-8690` worker。
2. MLX 只开一个实验槽，先 4B，再 9B。
3. 4B 通过前，不下载/启动 9B。
4. 任一 MLX live smoke 前，先记录 GGUF daemon/report 处于 stable 状态。
5. live smoke 时记录内存、swap、Metal/GPU、daemon stale/ledger_lag、worker 6/6 是否受影响。
6. 若出现 swap 压力、worker 降级 CPU/no-GPU、daemon stale、ledger lag、Web Lab 不可达，立即标记实验失败，等待授权 teardown。

## Runtime Preflight 要求

MLX 实验槽 preflight contract 位于：

```text
tools\gemma-chain\mlx-experiment-preflight-contract.json
```

该 contract 是本地 planning/checklist artifact，固定声明 `executes_commands=false`、`ssh_allowed=false`、`download_allowed=false`、`starts_process=false`、`sends_prompt=false`、`pool_policy=experimental_not_gguf_pool`。它列出候选命令和验收条件，但不授权本窗口执行这些命令。

MLX runtime preflight 必须证明：

- `mlx_lm.server` 或等价服务可用。
- Python 环境隔离，不依赖会弹出 Xcode command line tools 安装提示的系统 `python3`。
- 服务端口与 GGUF pool 端口分离，不占用 `8686-8690`。
- 选择的实验端口当前未被占用。
- GGUF daemon/report baseline 在实验前仍为 `stale=false`、`ledger_lag=0`、`gate_failures=0`。
- 32GB unified memory 的 headroom 为 green，不会把 GGUF 池压入 swap。
- health/model endpoint 可读。
- 可设置 max tokens、context、host/port、日志路径。
- 可以明确停止实验进程。
- 不修改 GGUF worker manifest、不覆盖模型 cache、不写 daemon 配置。

如果任一项缺失，MLX 方向停在“runtime/toolchain 未就绪”，不得进入 live smoke。

manifest preflight contract 还要求每个候选具备：

- `model_id`
- `format=mlx`
- `suggested_role`
- `priority`
- `expected_memory_class`
- `requires_runtime=mlx_lm.server`
- `download_allowed=false`
- `starts_process=false`
- `sends_prompt=false`
- `pool_policy=experimental_not_gguf_pool`
- `authorization_required_for_download=true`
- `authorization_required_for_live_smoke=true`

消费者如果遇到 manifest 缺字段、未知 `contract_version`、`format` 不是 `mlx`、或任一候选允许 download/start/prompt，都必须按 blocked 处理。

只读解析检查：

```powershell
Get-Content -Raw tools\gemma-chain\mlx-experiment-candidates.json | ConvertFrom-Json
Get-Content -Raw tools\gemma-chain\mlx-experiment-preflight-contract.json | ConvertFrom-Json
```

## 授权后执行与回滚 Checklist

MLX 4B/9B 实验的阶段化执行/回滚 checklist 位于：

```text
tools\gemma-chain\mlx-experiment-execution-checklist.json
```

该 checklist 仍是 documentation-only contract，固定 `executes_commands=false`。它把授权后的真实实验拆成这些阶段：

1. `preflight_pass`
2. `download_cache_hash`
3. `single_slot_start`
4. `smoke_prompt`
5. `resource_sample`
6. `teardown_or_keep_decision`
7. `gguf_pool_health_recheck`

每个阶段都声明 `requires_authorization`、是否 SSH、是否 downloads、是否 starts_process、是否 sends_prompt、通过条件和失败回滚动作。未来真正执行时，第一条授权命令必须是只读 runtime inventory，而不是下载、启动或 prompt，例如：

```powershell
ssh smartsteam-mac 'command -v mlx_lm.server; python3 --version'
```

这条命令本身仍需要主窗口授权；本窗口默认不执行。

## 执行证据 Artifact 模板

授权后真正执行 MLX 4B/9B 实验时，证据 artifact 模板位于：

```text
tools\gemma-chain\mlx-experiment-artifact-template.json
```

模板默认根目录：

```text
target\remote-gemma-mlx-experiments\<run-id>\
```

每个 run 应至少生成这些文件：

```text
preflight.json
download.json
start.json
smoke.json
resource-sample.json
teardown.json
gguf-health-recheck.json
summary.json
```

这些 artifact 的职责不能互相替代：

- `preflight.json` 证明执行前 runtime/toolchain/端口/GGUF baseline/headroom 检查，不证明模型已下载。
- `download.json` 证明 isolated MLX cache、hash、size 和磁盘余量，不证明服务已启动。
- `start.json` 证明单实验槽进程和端口，不证明 smoke 成功。
- `smoke.json` 证明一次 bounded prompt 成功，不证明资源长期稳定。
- `resource-sample.json` 证明 smoke 后资源状态，不证明 GGUF production pool 最终健康。
- `teardown.json` 证明停止或保留决策，不证明 GGUF pool 未受影响。
- `gguf-health-recheck.json` 专门证明 GGUF production pool 在实验后仍 healthy、Metal、report fresh。

任何 run 若缺少 `gguf-health-recheck.json`，都不能被视为“未污染 GGUF 生产池”。

## GGUF 与 MLX 不混淆规则

- GGUF 常驻池继续由 llama.cpp 管理。
- MLX 模型不得写入 GGUF worker manifest 的 model path。
- MLX 端口不得复用 `8686-8690`。
- MLX smoke prompt 不得通过 unattended daemon 自动触发。
- Web Lab/Forge/CLI 展示时必须标注 `runtime=mlx` 与 `experimental=true`。
- readiness gate 必须分开：GGUF 主池 healthy 不代表 MLX ready；MLX ready 也不授权替换 GGUF 主池。

## 仍需证据

- MLX runtime 是否可用：当前主窗口证据显示 `mlx_lm.server` 不存在。
- Python 工具链是否可用且隔离：当前主窗口证据显示系统 `python3` 会触发 Xcode command line tools 缺失提示。
- 4B 模型下载大小、hash、目标路径和磁盘预算。
- 4B 单槽 live smoke 的资源窗口。
- 9B 是否可在 32GB unified memory 下不压入 swap。
- MLX 实验槽与 GGUF daemon 并发时的 stale/ledger_lag/worker stability。
