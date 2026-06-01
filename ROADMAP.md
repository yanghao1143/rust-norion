# rust-norion Roadmap

## Optimized North Star / 优化后的总目标

Build a Rust-native, sovereignty-first FHT-DKE + Noiron local inference engine
for self-trained Transformer-family models. `rust-norion` should act as a
model-agnostic control plane around a self-owned model runtime, so future
versions of the internal model can be swapped in without rewriting routing,
memory, reflection, or scheduling logic.

构建 Rust 原生、自主可控优先的 FHT-DKE + Noiron 本地推理引擎，默认服务于自主训练的
Transformer 系列模型。`rust-norion` 的定位是模型无关的推理控制平面，让后续任何版本的自研模型都能复用同一套路由、记忆、反思和调度能力。

The target is local, offline, lightweight, ultra-long-context inference through
independently implemented public algorithms, not through external model weights,
closed services, or vendor-specific runtimes.

核心目标是通过自主实现公开算法，达成本地离线、轻量化、超长上下文推理，而不是依赖外部模型权重、闭源服务或厂商绑定运行时。

The north star is now explicitly scoped around five core requirements:

总目标明确收敛到五个核心诉求：

1. Self-developed model first / 自研模型优先
   The default production backend is an internally trained Transformer-family
   model. Third-party weights can be adapter examples only, never the core
   dependency.

2. Anti lock-in / 规避卡脖子
   The engine must stay useful if external model providers, weight licenses,
   cloud APIs, or vendor runtimes become unavailable.

3. Extreme local, lightweight, ultra-long context / 极致本地化、轻量化、超长上下文
   The control plane should make limited local hardware practical through disk
   KV, mixed-precision cache, sparse context, recursive scheduling, and
   global/local memory separation.

4. Frontier ideas as owned Rust implementations / 前沿技术本地落地
   Public research ideas are treated as algorithmic references. The project
   owns its Rust implementation of memory, routing, quantization, reflection,
   scheduling, and runtime boundaries.

5. Universal device adaptation / 全设备适配
   The control plane must run across CPU-only PCs, integrated-GPU laptops,
   discrete-GPU desktops, unified-memory machines, phones, tablets,
   wearable/XR/TV targets, embedded boards, browser-WASM,
   microcontroller-class tiny targets, NPU/AI accelerator devices, multi-GPU
   workstations, edge/robot/vehicle gateways, and servers through explicit
   hardware profiles and portable execution plans instead of vendor lock-in.

## Sovereignty Contract / 自主可控约束

- No default dependency on Gemma, Llama, Qwen, or other third-party model
  weights.
- No closed quantization, attention, memory, or scheduling component in the
  core path.
- Tokenization, embedding, and model forward execution should come from the
  self-developed Transformer runtime through explicit Rust traits.
- Public papers and open algorithm ideas may guide the design, but the codebase
  should own its implementation details.
- Runtime memory, experience, and adaptive state remain local, inspectable, and
  replaceable.
- Semantic retrieval, sparse filtering, and gist generation reuse the
  self-developed model's own tokenizer and embedding surface instead of
  introducing a hidden third-party encoder dependency.

- 默认不依赖 Gemma、Llama、Qwen 等第三方模型权重。
- 核心路径不依赖闭源量化、注意力、记忆或调度组件。
- Tokenizer、Embedding 和模型前向计算由自研 Transformer 运行时通过 Rust trait 显式接入。
- 可借鉴公开论文和开放算法思想，但实现细节由本仓库自主掌控。
- 运行时记忆、经验库和自适应状态全部本地化、可检查、可替换。
- 语义检索、稀疏筛选和 gist 生成复用自研模型自身的 tokenizer 与 embedding 接口，不引入隐藏的第三方编码器依赖。

## Architecture Target / 架构目标

1. Application layer / 应用层
   CLI, local API, and future desktop or service entry points.

2. `rust-norion` control plane / 控制平面
   Multi-factor router, hierarchy controller, recursive scheduler,
   Hot/Warm/Cold cache scheduler, sparse KV filter, reflection loop, RLVR-style
   process rewards, drift guard, Rust-only Toolsmith planning, read-only Agent
   Team coordination, experience replay, and persisted adaptive state.

3. Memory system / 记忆系统
   Infini-attention-style global/local KV split, hierarchical gist memory,
   reinforced KV fusion, 4/8-bit mixed-precision KV quantization, disk-backed
   append-only storage, and promotion/demotion policies.

4. Hardware abstraction / 硬件抽象
   Device profiles convert heterogeneous CPU, GPU, unified-memory, mobile,
   wearable/XR/TV, browser-WASM, embedded/tiny, NPU, multi-GPU, edge/robot/
   vehicle, and server pressure into latency budgets, KV budgets, routing
   constraints, and hierarchy weights. Profiles also map into capability tiers
   from tiny devices through distributed accelerators. The CLI should use
   best-effort local probing when no explicit profile is provided, while
   preserving manual overrides. Each plan should also emit a device execution
   profile: primary compute lane, portable fallback lane, memory mode, candidate
   runtime adapters, KV precision policy, prefetch budget, disk-spill policy,
   and recursive parallelism budget. Every explicit profile should also carry a
   coverage descriptor with common aliases so new device names map into stable
   policy classes instead of fragmenting the runtime into one-off vendor paths.
   Unrecognized manual profiles must degrade to the portable CPU profile, so
   new or niche devices are supported first through a safe generic execution
   path and then promoted to richer profiles when calibrated.

5. Runtime boundary / 运行时边界
   `InferenceBackend` and `ModelRuntime` traits remain model-agnostic. The
   default production target is a self-developed Transformer runtime.

6. Self-developed model stack / 自研模型栈
   The model runtime owns weights, tokenizer, embedding, and forward kernels.
   The control plane decides how context, memory, routing, and reflection are
   applied around that model.

## Priority Tracks / 优先级方向

1. Multi-factor routing / 多因子路由
   Entropy, task profile, context length, cache hit rate, latency budget, and
   hardware pressure choose among projection, local-window attention, global
   attention, and convolutional fusion. Device compute headroom lets larger
   accelerator profiles spend more attention on borderline tokens while tiny or
   overloaded devices stay closer to fast paths. Router thresholds should
   evolve per task profile so coding, writing, general reasoning, and
   long-document runs do not overwrite each other's compute strategy.

   Hierarchy weights should follow the same profile-specific rule: learned
   global/local/convolution balances for coding, writing, general reasoning,
   and long-document workloads persist independently instead of one task
   overwriting another.

2. Self-owned Transformer boundary / 自研 Transformer 边界
   Strengthen the runtime trait so the self-developed model exposes tokenizer,
   embeddings, native context window, KV import/export, and forward execution
   without tying the control plane to any external weight format.

3. Mixed-precision KV compression / 混合精度 KV 压缩
   Implement local 8-bit hot KV and 4-bit cold KV quantization in Rust, with
   model-specific accuracy benchmarks before aggressive compression is enabled.

4. Infini-style global/local memory / 全局 + 局部 KV 分离
   Separate permanent global memory from the active local window. Keep the
   active window small while persisting high-value global memory to disk.

5. Sparse context filtering / 稀疏上下文筛选
   Add a SpeContext-style filter before KV loading so redundant or low-value
   memories do not enter expensive attention paths.

6. Hierarchical gist memory / 层级摘要记忆
   Use reflection to produce document, section, and paragraph-level gist
   records. Store high-value summaries permanently and keep low-level detail in
   short-lived tiers.

7. Recursive scheduling / 递归调度
   Add `recursive_scheduler.rs` for inputs beyond the native model window:
   chunk, infer, merge, store, and recursively refine cross-chunk answers.

8. RLVR-style control rewards / 可验证奖励控制
   Score routing choices, KV reads, hierarchy weights, latency, structured
   reflection issues, contradictions, revision actions, and final quality.
   Update control state without modifying model weights.

9. Experience replay / 经验回放
   Extend `experience.rs` from passive retrieval to replayable records:
   prompt, route plan, KV usage, output quality, reward, and follow-up action.

10. Hardware-aware compute allocation / 硬件感知算力分配
    Use local CPU/GPU/RAM/disk pressure and explicit device profiles for
    CPU-only, integrated GPU, discrete GPU, unified-memory, mobile, embedded,
    browser-WASM, microcontroller, NPU/AI accelerator, multi-GPU, edge, and
    server targets to decide when to lower compute, shrink windows, evict
    memory, tighten retention/compaction governance, or spend extra attention
    on hard tasks.

11. Universal execution planning / 全设备执行计划
    Map every supported device profile to a primary compute lane, fallback lane,
    memory mode, runtime adapter hints, KV precision, prefetch count, disk-spill
    policy, and recursive parallel chunk budget. These hints must remain optional
    so the self-developed runtime can choose CUDA, ROCm, Metal, Vulkan, WGPU,
    DirectML, CoreML, NNAPI, QNN, OpenVINO, CANN, MLU, RKNN, WebGPU, portable
    Rust, or future adapters without making any one runtime mandatory.
    Unknown explicit device names must map to the portable CPU fallback instead
    of aborting, keeping all future devices reachable through a conservative
    baseline plan.

12. Device compatibility gate / 全设备兼容门禁
    The CLI should provide a `--device-gate` check that validates every explicit
    hardware profile keeps nonzero KV budgets, bounded prefetch, valid precision
    policy, adapter hints, bounded memory governance policy, alias roundtrips,
    and a CPU/portable Rust escape hatch. This makes device adaptation a
    regression gate rather than a documentation claim.

13. Runtime manifest device gate / 运行时 Manifest 设备门禁
    The CLI `--runtime-manifest-gate` should validate the production manifest
    against the current target device plan, not just local assets and
    Transformer shape. It must emit the stable `runtime_device_contract`, pick
    only adapters supported by both manifest and device execution plan, and fail
    when KV prefetch or hot/cold precision requirements exceed manifest bounds.

14. Rust-only Toolsmith and read-only Agent Team / Rust-only 工具匠与只读 Agent Team
    When the control loop needs new local helper capabilities, it can plan
    small Rust-only tool blueprints with explicit build and validation gates.
    Agent-style decomposition stays read-only: sub-agents write structured
    messages, risks, gates, and evolution hints into a blackboard, while the
    main thread remains the only writer to code, memory, and adaptive state.

## Target Module Fusion / 目标模块融合

The following algorithmic ideas are merged into the project goal as owned local
modules, not external product dependencies:

以下算法思想已合并进项目目标，并以本地自研模块落地，而不是作为外部产品依赖：

- Infini-style memory control:
  `kv_cache.rs`, `infini_memory.rs`, `tiered_cache.rs`, and `disk_kv.rs` split
  global permanent memory from the active local window, then persist and filter
  high-value KV records.
- Hierarchical gist memory:
  `reflection.rs` and `experience.rs` should produce document, section, and
  paragraph-level summaries after long runs, then write only high-value gist
  records into durable memory.
- Recursive language-model scheduling:
  `recursive_scheduler.rs` should chunk prompts beyond the native model window,
  run per-chunk inference through the same backend boundary, merge results, and
  store cross-chunk experience.
- SpeContext-style sparse KV filtering:
  memory loading should reject redundant, stale, or low-value KV records before
  they enter expensive attention paths.
- Mixed-precision KV compression:
  hot local KV uses safer precision, cold disk KV can use more aggressive 4-bit
  storage, and both paths require benchmark gates before production defaults.
  Reinforced KV-Fusion also includes batch compaction so older near-duplicate
  memories merge into stronger entries instead of expanding the local state
  forever.
- Test-time scaling and RLVR-style rewards:
  reflection should score not only the final answer but also routing choices,
  KV reads, hierarchy weights, latency, structured issue severity, revision
  actions, contradictions, and memory admission.
- Experience replay:
  the experience store should become replayable training data for the control
  plane state while leaving model weights untouched. Records should preserve
  route budgets, used KV memory ids, gist memory ids, runtime-KV memory ids,
  structured reflection issues, and revision actions so replay can reinforce or
  penalize the actual memory and reasoning path used by an answer.
- Drift guard:
  contradiction, low-confidence, or high-perplexity drafts should gate durable
  memory writes, block unsafe runtime KV admission, penalize contaminated
  memory reuse, and roll back adaptive state when the drift is severe.
- Trace schema gate:
  benchmark and inference JSONL traces should have an executable field-presence
  gate for the control-plane schema so route, memory, drift, reward, hardware,
  hardware KV budgets, recursion, runtime-KV, retention, and compaction
  diagnostics cannot silently disappear.
- State inspection:
  persisted memory, experience, global/profile router thresholds, hierarchy
  weights, tier counts, effective memory policies, and persisted memory vector
  dimension buckets should be inspectable from the CLI without running a new
  inference.
- Rust-native Transformer reconstruction:
  transformer planning should evolve into explicit templates and ABI contracts
  for self-developed model runtimes, including native window, embedding access,
  KV exchange, and structured request/response wiring. A built-in local runtime
  prototype and a manifest-backed reference production kernel should exercise
  that ABI before trained production kernels are available.
- Universal hardware profiles:
  hardware allocation should stay device-agnostic while supporting explicit
  policy profiles for PC, laptop, workstation, server, mobile, embedded,
  browser-WASM, microcontroller-class tiny targets, NPU/AI accelerator, edge,
  vehicle, robotics, and heterogeneous multi-GPU deployments. Common aliases
  such as unknown, generic, x86_64, arm64, LoongArch64, laptop, Steam Deck,
  DirectML, RTX, MacBook, iPhone, HarmonyOS, wearable, Snapdragon, Hailo,
  microcontroller, Jetson, automotive, NAS, datacenter, EPYC, and HPC should
  resolve into stable profiles instead of adding vendor-specific code paths.
- Universal execution plans:
  every hardware profile should produce portable runtime adapter hints and
  fallback policies so the same control plane can run on CPU-only machines,
  GPUs, unified-memory systems, phones, embedded boards, browser-WASM sandboxes,
  microcontroller-class targets, NPUs, edge devices, and multi-accelerator
  servers without assuming one vendor stack.
- Device compatibility gate:
  the repository should fail fast when any supported device profile loses valid
  alias coverage, execution lanes, KV budgets, adapter hints, disk-spill policy,
  memory governance policy, portable fallback coverage, or the stable
  `runtime_device_contract` ABI consumed by external self-developed runtimes.
- Runtime manifest device gate:
  production manifests should be checked against the current target device plan,
  including the emitted `runtime_device_contract`, adapter intersection,
  KV-import prefetch budget, and hot/cold KV precision bounds.
- Rust-only Toolsmith:
  `toolsmith.rs` should propose local helper-tool blueprints only as Rust
  source with explicit build/validation steps, reject non-Rust tool requests,
  and carry bounded summaries into runtime requests, traces, and reward notes.
- Read-only Agent Team:
  `agent_team.rs` should decompose complex local work into read-only lanes with
  single-writer isolation, conflict summaries, collision-free gates, and
  bounded evolution signals. Sub-agents can inform the control plane but cannot
  directly mutate code, memory, or adaptive state.

## Research-Inspired Algorithms / 公开算法启发

These are algorithmic references, not product dependencies:

- Infini-attention-style global memory plus local window:
  <https://arxiv.org/abs/2404.07143>
- Titans-style test-time memory update:
  <https://arxiv.org/abs/2501.00663>
- ReadAgent-style gist memory:
  <https://arxiv.org/abs/2402.09727>
- Standard uniform 4/8-bit KV quantization:
  implement locally instead of depending on vendor-specific compression stacks.
- Recursive long-context inference:
  implement as control-plane scheduling so the self-developed model can keep a
  stable native window.
- RLVR and test-time scaling:
  optimize routing, memory retention, and compute allocation without frequent
  weight retraining.

以上方向只作为公开算法启发，不作为外部权重、闭源组件或厂商运行时依赖。

## Version Plan / 版本计划

- v0.1: control layer prototype, heuristic backend, disk-backed memory
- v0.2: multi-factor router, self-developed runtime contract, explicit
  sovereignty constraints
- v0.3: 4/8-bit KV quantization, retention policy, automatic tier migration
  (initial local q4 disk-KV persistence, memory retention, and persisted tier
  migration tracing are in place; engine memory now defaults to append-only
  DiskKvStore persistence with legacy TSV migration; retention and batch
  KV-Fusion compaction policies are configurable through engine setters and CLI
  flags with conservative clamping and persist through adaptive state; batch
  KV-Fusion compaction can merge near-duplicate persistent memories while
  protecting ids used by the current inference; a KV quantization benchmark
  gate now checks deterministic q4/q8 accuracy, compression ratio, and elapsed
  time before compression policy changes are accepted)
- v0.4: Infini-style global/local KV split and sparse context filtering
  (initial control-plane memory planner with token-budget filtering is in
  place; the runtime KV import path now consumes the Infini local/global plan
  directly, imports local-window candidates before global-memory candidates,
  and treats sparse-skipped memories as a hard exclusion rather than falling
  back to all active memory)
- v0.5: hierarchical gist memory and recursive long-context scheduler
  (initial native-window-aware recursive schedule planning, chunk overlap, merge
  rounds, runtime prompt propagation, CLI reporting, document/section/paragraph
  gist generation, gist persistence, and KV admission are in place)
- v0.6: RLVR-style process rewards, experience replay, hardware-aware compute
  allocation
  (initial process reward scoring for route, memory, hierarchy, reflection,
  structured reflection issue severity, latency, and memory admission is in
  place and persisted with experience;
  reward-ranked experience replay can now update router, hierarchy, and KV
  memory state before inference, including a default low-pressure automatic
  replay pass over prior experience; device-agnostic hardware pressure planning now
  adapts latency budgets, KV token budgets, and hierarchy weights for CPU-only,
  integrated GPU, discrete GPU, unified-memory, mobile, embedded, browser-WASM,
  microcontroller, NPU/AI accelerator, multi-GPU, edge, and server devices,
  with capability tiers and common device aliases covered by tests;
  best-effort auto probing now maps OS,
  architecture, CPU parallelism, common GPU/NPU environment hints, edge device
  hints, WASM/tiny targets, and discrete GPU adapter names into a conservative
  device profile; each profile now emits execution-lane, memory mode,
  adapter-hint, KV-precision, prefetch, disk-spill, and recursive parallelism
  policies; runtime KV import now honors the device prefetch budget; recursive
  schedules are now grouped into execution waves using the device
  max-parallel-chunk budget; every explicit device profile now carries a
  coverage descriptor with broad aliases and a generic CPU fallback class; the
  CLI can print the full built-in device matrix and run a `--device-gate`
  compatibility check across every explicit device profile, including alias
  roundtrips; device/pressure-aware memory governance now adjusts retention and
  KV-Fusion compaction defaults so tiny, browser-WASM, mobile, and overloaded
  devices keep smaller durable memory state while accelerated and distributed
  profiles can retain and scan more context; `--list-devices` now prints the
  same local/global KV token budgets and latency budget used by the engine so
  device matrix review covers actual runtime boundaries, not only adapter hints;
  router thresholds now persist separately for general, coding, writing, and
  long-document profiles;
  hierarchy weights now persist separately for general, coding, writing, and
  long-document profiles;
  route scoring now consumes hardware pressure and device compute headroom from
  the hardware plan;
  runtime token entropy/logprob now feeds the main generation metrics used by
  drift checks, router/hierarchy adaptation, process rewards, and experience
  replay instead of remaining limited to token-window monitoring;
  experience records now persist route budgets plus used/stored/gist/runtime-KV
  memory ids, structured reflection issues, and revision actions for replay;
  recursive runtime call costs now feed replay metrics so expensive successful
  long-context paths are weaker control-plane exemplars than efficient ones,
  and replay summaries/traces expose recursive call pressure for regression
  checks; benchmark gates can now require auto-replay recursive coverage and
  enforce replay recursive call pressure floors and ceilings; replay reports,
  traces, CLI output, and benchmark gates now expose router updates, hierarchy
  updates, memory reinforcements, and memory penalties so automatic replay must
  prove real control-plane mutation, not just execution; replay planning now
  keeps at least one recursive runtime sample when the limit allows so
  long-context cost learning is not starved by short high-reward samples;
  replay pressure now parses
  recursive chunks, waves, parallelism, and runtime calls from reward notes so
  long-document costs are scored against the recursive schedule instead of
  being diluted by route token counts;
  reflection now performs one low-risk repair pass for non-critical weak drafts
  and blocks stale runtime-KV admission when the final answer came from a
  repaired draft;
  the CLI can inspect persisted local state without running inference, including
  effective memory policies and persisted memory vector dimensions;
  drift guard now gates memory writes, runtime KV
  admission, used-memory penalties, and adaptive-state rollback)
- v0.7: Rust-native Transformer templates, KV import/export ABI, benchmark
  harness for self-developed model runtimes
  (explicit general, coding-local, creative-writing-global, and
  long-context-convolution Transformer templates are now exposed through the
  refactor plan, CLI output, traces, and runtime request ABI;
  initial runtime metadata, explicit Transformer architecture shape, tokenizer,
  embedding, and KV import/export trait hooks are in place; `RuntimeBackend` now exposes model-side embeddings to the
  control plane for memory lookup and writes when a self-developed runtime
  provides them, while preserving the portable Rust fallback; KV-Fusion
  similarity now penalizes mismatched vector dimensions so memories from
  different runtime embedding spaces do not over-fuse; `RuntimeBackend` now
  injects runtime metadata, architecture, and device execution contracts into each request, command runtime prompts expose
  the ABI boundary, and backend native context windows feed recursive scheduling; runtime metadata now carries
  Transformer layer/head/window shape, KV import/export limits, and hot/cold KV
  precision into both text and JSON request surfaces; CLI command runtimes can
  pass model id, tokenizer, native window, embedding dimensions, explicit
  architecture flags, device execution contracts, and KV exchange flags; active Noiron memory can now be imported into runtime KV and accepted
  exported runtime KV can be written back into reinforced memory; JSONL trace
  records now capture route, hierarchy, KV, recursion, hardware execution, the
  stable runtime device contract, hardware KV budgets, structured reflection
  diagnostics, drift, reward, effective memory policies, and memory counters per inference, with a CLI trace schema gate for required
  control-plane fields; a
  built-in benchmark suite now writes one
  trace record per coding, long-context, general-reflection, and writing case;
  benchmark regression gates can enforce minimum quality, minimum reward, total
  latency ceilings, minimum recursive-case coverage, recursive chunk ceilings,
  and maximum drift block/rollback counts; a persistent roundtrip gate now verifies memory, experience, and
  runtime KV reuse after full-state reload; a deterministic Rust-native local
  runtime prototype now implements
  tokenizer, embedding, deterministic global/local/convolution forward layers,
  imported-KV influence, generation, token trace, and KV import/export through
  the same `ModelRuntime` ABI; command runtimes can now use a structured
  JSON wire format carrying Noiron route, hierarchy, recursive, hardware
  execution, memory, and experience context and returning answer/token/trace
  metadata; `RuntimeManifest` now captures self-developed model metadata,
  Transformer shape, local asset paths, KV import/export limits, quantization
  policy, supported devices, and adapter hints so future runtime versions can
  be validated before production kernels are loaded; production manifest
  validation now requires weights and tokenizer assets to exist as local files
  while keeping prototype/demo validation non-blocking; the CLI now exposes a
  `--runtime-manifest-gate` with explicit layer/head/window architecture plus
  weights/tokenizer/config asset flags so this contract can fail before a
  production runtime is used; the gate now also validates the current target
  device execution contract, adapter intersection, KV prefetch budget, and
  hot/cold KV precision bounds; the same explicit architecture flags configure
  the local Rust runtime prototype and command-runtime request ABI; command
  runtimes now receive `{runtime_device_contract}` and structured JSON requests
  carry `hardware.runtime_device_contract` so external self-developed backends
  can choose portable or accelerated adapters from the same hardware plan
  without reconstructing the contract from expanded fields; runtime responses can now carry structured
  forward diagnostics, and trace JSONL records model id,
  selected adapter, executed layers, hidden size, local window, forward energy,
  KV influence, runtime token uncertainty, and runtime KV import/export counts;
  runtime requests, inference outcomes, traces, and benchmark summaries now
  filter adapter observations against the current device execution plan, and
  runtime responses are checked against the requested model id, architecture
  envelope, and device adapter hints before exported runtime KV can be admitted; a
  `ProductionTransformerRuntime` boundary now binds production manifests to
  existing local assets, the current device contract, adapter intersection,
  bootstrap tokenizer/embedding access, bounded KV import, and bounded KV
  export; generation fails explicitly until a kernel is attached, and
  `ProductionForwardKernel` now provides the stable trait slot for a real
  self-developed Transformer forward kernel to return answer text, token
  uncertainty, trace, diagnostics, and exported KV blocks; exported KV from
  production kernels is now ABI-validated for manifest layer/head bounds,
  request/recursive token ranges, non-empty matching key/value dimensions, and
  finite float values before it can reach `RuntimeBackend`; imported KV from
  the Noiron control plane is now validated before a production runtime accepts
  it, and `RuntimeBackend` maps imported KV memory blocks onto manifest-bounded
  layer/head ids instead of assuming unbounded heads; Rust-only Toolsmith
  planning and read-only Agent Team coordination now flow through engine
  outcomes, runtime requests, JSON traces, and process reward notes so local
  capability growth remains traceable and gated; `ReferenceProductionForwardKernel`
  now provides a deterministic Rust kernel that runs behind the same
  manifest/device/KV boundary for CI and integration validation without
  pretending to be a trained production model; the CLI can now
  select that boundary with `--production-runtime` for normal inference and
  benchmark runs, or attach the reference kernel with
  `--production-reference-kernel`; `ModelRuntimeForwardKernel` can now wrap a
  Rust `ModelRuntime`, and `--production-local-kernel` uses it to run the
  local Transformer runtime through the production manifest/device/KV boundary;
  `--production-kernel-conformance-gate` now runs a short manifest-backed
  forward pass with deterministic KV import and fails unless the attached
  kernel returns token uncertainty, reasoning trace, forward energy, KV
  influence, and exported KV when enabled; benchmark runs now seed
  deterministic sparse-memory fixtures and can gate Infini/SpeContext coverage
  with minimum sparse-skipped case/token counts, so sparse filtering regressions
  are caught from a clean state; `--runtime-manifest-all-devices-gate` can now
  validate a self-developed runtime manifest against every built-in device
  execution profile, so CPU, mobile, embedded, browser-WASM, microcontroller,
  NPU, multi-GPU, edge, and server support cannot silently diverge from the
  production manifest; benchmark summaries now record the effective device
  profile for each case, `--benchmark-all-devices` runs the default suite
  across every explicit hardware profile, and `--benchmark-min-device-profiles`
  can fail CI when CPU, integrated GPU, discrete GPU, UMA, mobile, embedded,
  browser-WASM, microcontroller, NPU, multi-GPU, edge, and server control-loop
  execution coverage is incomplete; `--benchmark-min-recursive-device-profiles`
  can additionally require each explicit device profile to trigger real
  recursive long-context scheduling, so all-device support covers ultra-long
  context paths instead of only short control-loop cases; production runtime
  all-device benchmark sweeps now rebuild the manifest-backed runtime per
  explicit device profile, so the reference/local production kernel path is
  checked with the matching runtime device contract, adapter hint, KV
  prefetch/precision boundary, and recursive parallelism budget for each
  target instead of reusing one runtime contract across the matrix; the
  Rust-native `LocalTransformerRuntime` prototype now has the same
  production-local all-device recursive benchmark coverage as the deterministic
  reference kernel, so tokenizer/embedding/Transformer-plan/KV exchange
  behavior is gated through the production ABI rather than only through local
  unit tests; benchmark summaries and gates now count runtime adapter contract
  cases and violations, so production all-device sweeps must prove every
  selected adapter is inside that device's allowed adapter hints; benchmark
  summaries and gates also count runtime KV import cases and imported block
  totals, so production sweeps must prove persisted Noiron memory is actually
  fed back into the runtime rather than only exported after generation;
  benchmark summaries and gates now also count runtime token cases and
  uncertainty-bearing token totals, so production sweeps can prove token-level
  entropy/logprob feedback remains present across every device)
- v1.0: production-grade local Agent Harness and test-time scaling inference
  engine for self-owned Transformer models

## Definition of Done / 验收标准

- The default build can run without external model weights or closed services.
- Every control decision can be traced: route, memory, hierarchy, reflection,
  drift, reward, device-derived hardware KV budgets, effective memory policies,
  Toolsmith planning, Agent Team coordination, and adaptive-state update.
- Runtime token uncertainty is part of the control feedback loop: token
  entropy/logprob can raise generation perplexity and influence drift, reward,
  routing, hierarchy, and experience updates; trace JSONL now emits the
  aggregate runtime token counts, average entropy, average negative logprob, and
  derived uncertainty perplexity, and the schema gate requires that block.
  Benchmark gates can also require runtime uncertainty case coverage and
  uncertainty-bearing token totals before production runtime sweeps pass.
- Runtime forward diagnostics are observable: local and command runtimes can
  report model id, selected adapter, executed layer count, hidden size, local
  window, forward energy, KV influence, and runtime KV exchange counters, and
  the trace schema gate requires the diagnostics block.
- Trace JSONL files have a CLI schema gate that fails when required
  control-plane fields disappear.
- KV compression has an accuracy, compression-ratio, and latency benchmark gate
  before it becomes default.
- Long-context claims are tied to reproducible benchmarks, including gates that
  require at least one truly recursive scheduling case and gates that require
  recursive coverage across every explicit device profile instead of marketing
  language.
- Self-evolution is bounded by drift controls: confidence gates, decay,
  rollback, configurable retention, protected-id memory compaction, and
  inspectable local state.
- The CLI can inspect memory count, experience count, adaptive global/profile
  router state, hierarchy weights, tier counts, effective memory policies,
  memory-vector dimension buckets, and top memories/lessons from persisted
  local files without invoking a model runtime.
- Adaptive state persistence covers router thresholds, hierarchy weights, tier
  placement, and memory governance policies, while legacy adaptive files
  without policy keys still load with conservative defaults.
- The control plane remains compatible with future self-developed model
  versions through stable Rust traits.
- A built-in local runtime prototype proves tokenizer, model-side embedding,
  deterministic Transformer layer execution, imported-KV influence, generation,
  KV exchange, manifest-based runtime configuration, and control-plane memory
  integration end to end before production Transformer kernels are connected.
- A manifest-backed reference production kernel proves the production boundary
  can execute end to end through `RuntimeBackend`, including token uncertainty,
  runtime diagnostics, imported-KV influence, exported KV validation, trace, and
  process-reward feedback, while remaining explicitly replaceable by a trained
  self-developed kernel. Benchmark gates can require a minimum number of runtime
  forward-signal cases, imported runtime KV cases, imported runtime KV blocks,
  and exported runtime KV blocks so the reference production ABI remains a
  regression target, not just a demo path.
- Production runtime manifests have a hard local-file gate for required weights
  and tokenizer assets before a self-developed model runtime is accepted, and
  the CLI exposes that gate directly for local/CI checks with explicit
  Transformer layer/head/window shape validation.
- The same production runtime manifest can be checked against every built-in
  device profile with `--runtime-manifest-all-devices-gate`, including adapter
  intersections, KV prefetch bounds, KV precision bounds, and portable fallback
  coverage.
- The production runtime adapter boundary is explicit: it can only be
  constructed after manifest and device gates pass, it exposes selected adapter
  and runtime device contract state, and it refuses generation until a real
  self-developed forward kernel is wired behind it.
- Real production kernels have a stable Rust attachment point:
  `ProductionForwardKernel` receives the manifest, device contract, asset
  summary, imported KV blocks, and runtime request, and returns the same
  response surfaces consumed by `RuntimeBackend`.
- Rust-native model runtimes can be lifted into that production attachment
  point with `ModelRuntimeForwardKernel`, which forwards manifest metadata,
  architecture, imported KV, token uncertainty, trace, diagnostics, and
  exported KV through the same ABI used by real kernels.
- Real production kernels must pass a dedicated conformance gate before being
  treated as integrated: `--production-kernel-conformance-gate` requires a
  connected kernel, non-empty answer, token uncertainty, reasoning trace,
  positive forward energy, finite KV influence, and exported KV when KV export
  is enabled.
- Production kernel exported KV cannot bypass the runtime boundary: invalid
  layer/head ids, token ranges, empty vectors, mismatched key/value dimensions,
  oversized vectors, or non-finite values must fail before any long-term memory
  admission path can see the blocks.
- Production runtime imported KV cannot bypass the same boundary: invalid
  control-plane KV imports must clear the pending import set and fail before a
  production kernel receives them.
- Runtime-generated KV import blocks are bounded by the target Transformer
  architecture layer/head shape, so active memory import cannot create
  out-of-manifest heads on self-developed runtimes.
- Runtime KV import now honors the Infini/SpeContext sparse plan before model
  execution: local-window memory is imported first, global memory is second,
  and skipped memory never enters the expensive backend attention path.
- Benchmark summaries and gates expose runtime KV import coverage through
  `--benchmark-min-runtime-kv-import-cases` and
  `--benchmark-min-runtime-kv-imported`, so local/CI production sweeps can fail
  when persisted control-plane memory stops reaching the runtime.
- Benchmark summaries and gates expose runtime token uncertainty coverage
  through `--benchmark-min-runtime-uncertainty-cases` and
  `--benchmark-min-runtime-uncertainty-tokens`, so local/CI production sweeps
  can fail when a runtime stops returning token entropy/logprob signals.
- Toolsmith and Agent Team control surfaces stay local and constrained:
  Toolsmith accepts only Rust-source helper blueprints, and Agent Team lanes are
  read-only with single-writer isolation and trace/reward visibility.
- The CLI can execute the production runtime boundary through
  `--production-runtime`, so production manifest/device failures and the
  kernel-not-connected state are observable in the same control loop used by
  local prototypes and command runtimes.
- The CLI can attach the deterministic reference production kernel through
  `--production-reference-kernel` for local/CI validation of the same production
  ABI without requiring external weights or cloud services.
- The CLI can attach the local Rust-native Transformer runtime through
  `--production-local-kernel`, so the self-developed runtime prototype exercises
  the same production boundary and conformance gate before a trained kernel is
  available.
- The CLI can run `--production-kernel-conformance-gate` so a reference or real
  self-developed kernel must prove the full response/KV diagnostic surface
  before benchmark or integration claims rely on it.
- Mixed runtime versions or fallback/runtime embedding spaces do not silently
  over-fuse incompatible KV memories.
- Hardware adaptation is profile-driven and test-covered across constrained
  devices and high-capacity accelerator targets, including execution-plan
  fallbacks and alias coverage for each device class.
- Memory governance is also hardware-aware: every supported device profile
  produces bounded retention/compaction policy defaults, and explicit CLI flags
  remain authoritative overrides.
- The device compatibility gate passes across all explicit profiles and fails
  if a profile loses valid alias mappings, budgets, adapter hints, memory
  governance bounds, a portable fallback, or required `runtime_device_contract`
  fields.
- Default CLI execution performs conservative local device probing, and manual
  device/load flags remain authoritative.
- Benchmark execution can sweep every explicit device profile with
  `--benchmark-all-devices`, and the benchmark gate can require all-device
  control-loop coverage through `--benchmark-min-device-profiles`.
- Benchmark execution can require every explicit device profile to exercise
  recursive long-context scheduling through
  `--benchmark-min-recursive-device-profiles`.
- Production runtime benchmark execution rebuilds the manifest-backed runtime
  per explicit device profile during `--benchmark-all-devices`, so reference and
  local production kernels can be gated against each device's own runtime
  device contract and long-context recursion path.
- Benchmark execution can require runtime adapter contract coverage through
  `--benchmark-min-runtime-adapter-contract-cases` and fail on selected-adapter
  contract drift through `--benchmark-max-runtime-adapter-contract-violations`.
- The Rust-native local Transformer runtime prototype must pass the same
  production-local all-device recursive benchmark gate as the deterministic
  reference kernel before it is treated as a valid self-owned runtime boundary.
- Benchmark gates can fail CI or local checks when quality, reward, latency,
  recursive scheduling coverage, recursive scheduling budgets, runtime
  forward diagnostics, runtime token uncertainty, runtime KV import/export,
  runtime adapter contract coverage,
  auto-replay router/hierarchy/memory update coverage, auto-replay recursive pressure coverage/bounds,
  Infini/SpeContext sparse filtering coverage, all-device execution coverage,
  drift block/rollback counts, or persistent state reuse regress.
