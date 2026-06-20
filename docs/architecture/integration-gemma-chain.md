# Gemma Chain Integration Boundary

The Gemma 12B chain is an outer integration layer. It must stay outside core crates and root workspace metadata unless a separate owner explicitly changes that boundary.

## Components

```text
Gemma llama-server / tunnel
  Provides model inference over localhost port 8686.

rust-norion backend
  Owns request contracts, readiness gates, experience state, device safety, and business-cycle orchestration on port 7979.

Web Lab
  Owns browser UI and SSE proxy behavior on port 8789.

tools/gemma-chain
  Owns read-only diagnostics and tiny shared-chain smoke tests.
```

Future Apple Silicon pool:

```text
quality worker 127.0.0.1:8686
  One high-quality 12B worker for generation, architecture, and final synthesis.

small workers 127.0.0.1:8687-8690
  Optional summary, review, test-gate, and index roles for cheap parallel analysis.

pool router
  Future outer integration layer. It should route by role and backpressure, not by starting extra heavy models.
```

## Non-Goals

- Do not store credentials or tokens.
- Do not copy repository source to the remote model host.
- Do not modify model weights.
- Do not stop, restart, or replace shared model/backend processes from diagnostics.
- Do not couple the outer integration scripts into root `Cargo.toml` or `src/**`.
- Do not automatically start multiple 12B Q8 workers on one Apple Silicon host to bypass queueing.
- Do not add cloud Gemini credentials or fallback behavior to diagnostics.

## Health Contract

`tools/gemma-chain` treats backend `/health` as the primary integration source of truth because it projects both local backend state and model runtime metadata:

- model reachability
- model name
- context window
- train context window
- engine busy state
- active request summaries
- readiness and safe-device gates

The model service `/health` is used only to prove the tunnel or llama-server is alive. Model metadata endpoints may differ by llama-server build, so metadata failures are warnings unless generation also fails while idle.

For a model pool, health should eventually be tracked per worker:

- endpoint reachability
- model name and quantization class
- context window and train context window
- default or requested max token budget
- busy state and active request summary
- role label such as `quality`, `summary`, `review`, or `test-gate`

Until a pool router exists, `127.0.0.1:8686` remains the single source of truth for the quality Gemma chain.

## Read-Only Gate Contract

`tools/gemma-chain` exposes machine-readable gates for automation, but these gates must stay read-only. `chain-status`, `entrypoint-matrix`, `pool-status`, `status-bundle`, `handoff-report`, `contract-audit`, `wrapper-manifest`, `contract-fixture`, `prompt-gate`, `loop-status`, `recovery-plan`, `diagnose`, `selftest`, and `pool-plan` must not send prompts, start or restart processes, open SSH sessions, mutate remote machines, or print credentials.

Prompt-producing callers should gate through `chain-status` before they call Web Lab, Forge, direct backend CLI paths, evolution-loop prompt rounds, or model-pool launch:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked
```

Stable JSON fields from `chain-status -JsonStatus`:

| Field | Meaning |
| --- | --- |
| `schema_version` | Current schema integer, `1` |
| `contract_version` | Current contract string, `gemma-chain.v1` |
| `classification` | One of `backend_down`, `engine_busy`, `quality_worker_down`, `web_lab_down`, `prompt_ready`, or `prompt_blocked` |
| `next_step` | Human-readable next safe action |
| `require_action` | Requested action gate, such as `smoke`, `evolution_loop_prompt_round`, or `model_pool_launch` |
| `require_action_allowed` | Boolean decision for the requested action |
| `wait_ready` | Whether this invocation polled health until the requested action became allowed or timed out |
| `endpoints.quality_worker.tcp_reachable` | TCP reachability for `127.0.0.1:8686` |
| `endpoints.backend.health_ok` | Backend `/health` availability for `127.0.0.1:7979` |
| `endpoints.web_lab.health_ok` | Web Lab `/health` availability for `127.0.0.1:8789` |
| `prompt_gate.prompt_ready` | Backend-level prompt readiness |
| `prompt_gate.engine_busy` | Whether the backend is already serving an active model request |
| `prompt_gate.gemma_runtime_reachable` | Backend projection of quality worker reachability |
| `prompt_gate.readiness_ok` | Backend readiness projection |
| `prompt_gate.safe_device_ok` | Device safety projection |
| `prompt_gate.context_window` | Runtime context window if reported by health |
| `prompt_gate.min_context_tokens` | Caller-requested minimum context gate |
| `prompt_gate.context_ready` | Whether the runtime context satisfies the requested minimum |
| `allowed_actions[].id` | Stable action id |
| `allowed_actions[].allowed` | Boolean decision for that action |
| `allowed_actions[].reason` | Safe, non-secret explanation for the decision |
| `allowed_action_table[].category` | Normalized action class: `read_only`, `prompt`, or `launch` |
| `allowed_action_table[].status_command_sends_prompt` | Always `false`; status commands must not submit prompts |
| `allowed_action_table[].downstream_sends_prompt` | Whether the gated downstream action would send a prompt after approval |
| `allowed_action_table[].downstream_launches_process` | Whether the gated downstream action would launch a process after approval |
| `integration_entrypoints[].surface` | Wrapper-facing surface such as Web Lab, Forge CLI, evolution-loop, or model-pool |
| `integration_entrypoints[].gate_command` | Canonical read-only gate command for that surface |
| `integration_entrypoints[].current_allowed` | Current decision for the entrypoint |
| `integration_entrypoints[].blocked_by` | Safe block reason, or `none` |
| `machine_summary.read_only` | Always `true` for the status object |
| `machine_summary.any_prompt_allowed` | Fast boolean for whether any prompt entrypoint is currently allowed |
| `machine_summary.model_pool_launch_allowed` | Fast boolean for the Apple Silicon pool launch gate |
| `machine_summary.next_read_only_gate` | Safe command to refresh status without sending prompts |

`prompt-gate -JsonStatus` is the smaller backend-only contract. It reports `prompt_ready`, `block_reason`, context fields, and per-entrypoint decisions, but it does not classify Web Lab or direct `8686` TCP state by itself. `loop-status -JsonStatus` adds latest evolution ledger and daemon log timestamps plus a loop-specific classification; it must redact prompt lines and sensitive values. `recovery-plan -JsonStatus` packages the same gate decision with owner handoff fields and post-recovery validation commands.

Stable JSON fields from `prompt-gate -JsonStatus`:

| Field | Meaning |
| --- | --- |
| `schema_version` | Current schema integer, `1` |
| `contract_version` | Current contract string, `gemma-chain.v1` |
| `contract.schema_version` | Same schema integer repeated in the nested contract |
| `contract.contract_version` | Same contract string repeated in the nested contract |
| `contract.read_only` | Always `true` |
| `contract.sends_prompt` | Always `false` |
| `contract.launches_process` | Always `false` |
| `contract.scope` | `backend-health-only`; Web Lab TCP and direct model TCP are classified by `chain-status` |
| `entrypoint_decisions[].id` | Standard action id matching `chain-status -RequireAction` |
| `entrypoint_decisions[].allowed` | Boolean decision from backend health plus the invocation's `-MinContextTokens` |
| `entrypoint_decisions[].standard_gate_command` | Canonical `chain-status` command to run before the downstream action |
| `entrypoint_decisions[].standard_required_context_tokens` | `262144` for evolution-loop and model-pool gates, otherwise `0` |
| `entrypoint_decisions[].sends_prompt_after_gate` | Whether the downstream action sends a prompt after the gate passes |
| `entrypoint_decisions[].compatibility_alias` | Legacy alias, currently `web_lab_manual_prompt` for `web_lab_prompt` |

The legacy `entrypoints` object remains for compatibility. New wrappers should prefer `entrypoint_decisions[]` because its ids line up with Web Lab, Forge CLI, backend direct CLI, evolution-loop, and model-pool gates.

`pool-status -JsonStatus` is the read-only Apple Silicon pool contract. It merges the static pool plan, `model_pool_launch` gate, and TCP/health probes for ports `8686-8690`.

Stable JSON fields from `pool-status -JsonStatus`:

| Field | Meaning |
| --- | --- |
| `schema_version` | Current schema integer, `1` |
| `contract_version` | Current contract string, `gemma-chain.v1` |
| `summary` | Static pool recommendation |
| `blocked_policy` | Safety rule for blocking pool launch while quality is down |
| `launch_gate` | Command that automation should run before launch |
| `launch_allowed` | Boolean model-pool launch decision |
| `launch_block_reason` | Classification that blocked launch, or `none` |
| `min_context_tokens` | Effective context threshold, defaulting to the quality worker plan |
| `context_gate_source` | `quality default` unless `-MinContextTokens` was explicitly supplied |
| `chain_classification` | Current launch-gate classification |
| `chain_next_step` | Human-readable next safe action |
| `prompt_gate.prompt_ready` | Backend prompt readiness projected into the pool gate |
| `workers[].port` | Planned worker port |
| `workers[].role` | Planned worker role such as `quality`, `summary`, `review`, or `test-gate` |
| `workers[].launches_process` | Always `false` for diagnostics |
| `workers[].tcp_reachable` | TCP reachability for that worker port |
| `workers[].health_ok` | `/health` availability for that worker port |
| `workers[].runtime_backend` | Optional worker-reported backend such as `llama.cpp` |
| `workers[].runtime_device` | Optional worker-reported execution device such as `metal` or `cpu` |
| `workers[].runtime_accelerator` | Optional normalized accelerator hint, including `metal` when the worker reports Metal |
| `workers[].gpu_layers` | Optional worker-reported GPU/offloaded layer count |
| `capacity.policy` | Pool expansion policy, currently `one_quality_plus_small_helpers` |
| `capacity.expansion_allowed` | Conservative boolean for whether adding helper concurrency is currently advisable |
| `capacity.recommendation` | Machine-readable next action such as `restore_quality_gate_first`, `verify_worker_runtime_metadata_before_expansion`, or `add_summary_worker_first` |
| `capacity.healthy_helper_worker_count` | Healthy non-quality workers available for cheap parallel work |
| `capacity.metal_worker_count` | Healthy workers that self-report Metal execution |
| `capacity.cpu_worker_count` | Healthy workers that self-report CPU execution |
| `capacity.unknown_runtime_worker_count` | Healthy workers without runtime/device/GPU metadata |
| `capacity.zero_gpu_layer_worker_count` | Healthy workers reporting zero GPU/offloaded layers |
| `capacity.quality_runtime_accelerated` | `true`, `false`, or `null` for the quality worker acceleration evidence |

`pool-route-plan -JsonStatus` preserves the same runtime fields on
`candidate_workers[]` and `selected_worker`. These fields are diagnostic hints:
missing values do not prove CPU fallback, but `runtime_device=cpu`,
`runtime_accelerator=unknown`, or `gpu_layers=0` should be treated as a strong
reason to inspect the remote worker logs before adding concurrency.

The backend `/v1/model-pool/status` and Forge/Web Lab projections expose the
same `capacity` object. `expansion_allowed=false` is not a model-quality
failure; it is an operational instruction to restore the quality gate, fix a
known CPU fallback, or verify missing runtime metadata before adding more
workers.

`status-bundle -JsonStatus` is the owner-handoff contract for the main coordination window. It combines `chain-status`, `loop-status`, `pool-status`, recovery steps, and safe next commands into one read-only object. Stable top-level fields include `read_only`, `classification`, `require_action`, `require_action_allowed`, `prompt_ready`, `quality_worker_reachable`, `engine_busy`, `loop_classification`, `loop_action`, `model_pool_launch_allowed`, `model_pool_launch_block_reason`, `prompt_policy`, `chain`, `loop`, `pool`, `recovery`, `safe_next_commands`, and `safety_notes`.

`handoff-report -JsonStatus` is the concise owner-handoff projection. It is
derived from the same read-only bundle and audit, but keeps only current state,
operator recommendation, endpoint summary, blocked prompt actions, entrypoint
gates, safe next commands, and the Apple Silicon pool decision. Stable fields
include `schema_version`, `contract_version`, `read_only`, `sends_prompt`,
`launches_process`, `classification`, `current_state`,
`operator_recommendation`, `require_action`, `require_action_allowed`,
`prompt_ready`, `engine_busy`, `quality_worker_reachable`,
`model_pool_launch_allowed`, `blocked_prompt_actions`,
`apple_silicon_pool_summary`, and `safe_next_commands`.

`status-bundle -JsonStatus` also repeats `machine_summary`, `allowed_action_table`, and `safe_next_command_actions` at the top level for wrappers that should not parse nested human-oriented fields. In a blocked state, every `safe_next_command_actions[]` entry must be present in `prompt_policy.allowed_read_only_actions[]`; if a prompt-producing action appears there, the contract is broken. This protects smoke, Web Lab, Forge, direct backend CLI, evolution-loop, and model-pool launch from accidental execution while `any_prompt_allowed=false`.

Every machine-readable gate and plan object exposes `schema_version=1` and `contract_version=gemma-chain.v1` at top level; `pool-plan -JsonPlan` uses PascalCase `SchemaVersion` and `ContractVersion`. Wrappers must fail closed when these fields are missing or unknown, because a permissive parser could accidentally treat a changed gate shape as approval to send prompts or launch workers.

`status-bundle.wrapper_audit` is the fail-closed summary for automation. It reports whether the contract is supported, whether blocked-state safe next actions are a subset of allowed read-only actions, and which wrapper decision applies. `wrapper_decision=read_only_only` means Web Lab, Forge CLI, direct backend CLI, evolution-loop prompt rounds, and model-pool launch must remain blocked; read-only commands such as `diagnose`, `chain-status`, `entrypoint-matrix`, `loop-status`, and `pool-status` may continue. `wrapper_decision=action_gate_required` means the wrapper may proceed only to the exact action-specific `chain-status -RequireAction ... -JsonStatus -FailIfBlocked` gate; it is not permission to send a prompt by itself.

`chain-status -JsonStatus` and `status-bundle -JsonStatus` both expose `integration_entrypoints[]`, the wrapper-facing matrix for prompt and launch surfaces. Rows cover `smoke`, `web_lab_prompt`, `forge_cli_prompt`, `backend_cli_direct_prompt`, `evolution_loop_prompt_round`, and `model_pool_launch`. Each row includes the human surface, consuming tool, entrypoint kind, current decision, block reason, canonical gate command, downstream prompt/process behavior, and fail-closed wrapper decision. The matrix is informational unless the matching `gate_command` succeeds.

`entrypoint-matrix -JsonStatus` exposes the same rows as top-level `entrypoints[]` with a smaller payload. It is read-only, sets `sends_prompt=false` and `launches_process=false`, and may be used by UI/CLI wrappers that only need to render or enforce entrypoint state without reading loop, pool, or recovery details.

`contract-audit -JsonStatus` is a read-only verifier for wrapper integration.
It consumes the current `status-bundle` shape and checks that the contract can
be consumed fail-closed: supported version fields, read-only status flags,
safe-next subset, six entrypoint gates, `model_pool_launch` as a process gate
instead of a prompt gate, quality-worker down blocking pool launch, and the
post-recovery release sequence. Its `audit_passed` field proves internal
contract consistency only. Prompt or launch callers must still require the
matching `chain-status -RequireAction ... -JsonStatus -FailIfBlocked` gate
before acting.

`wrapper-manifest -JsonStatus` is the read-only field consumption manifest for
Web Lab, Forge CLI, direct backend CLI, evolution-loop, smoke, and model-pool
launch wrappers. Stable top-level fields include `schema_version`,
`contract_version`, `read_only`, `sends_prompt`, `launches_process`,
`classification`, `require_action`, `require_action_allowed`, `prompt_ready`,
`quality_worker_reachable`, `engine_busy`, `model_pool_launch_allowed`,
`wrapper_decision`, `fail_closed_required`,
`audit_passed`, `unknown_contract_policy`, `status_commands`,
`required_top_level_fields`, `consumption_rules`, `entrypoints`, and
`safe_next_command_actions`.

Each `wrapper-manifest.entrypoints[]` row includes `id`, `surface`,
`consumer`, `entrypoint_kind`, `current_allowed`, `blocked_by`,
`status_command`, `gate_command`, `gate_read_only`, `blocked_exit_code`,
`required_context_tokens`, `downstream_sends_prompt`,
`downstream_launches_process`, `consume_fields`, `proceed_only_if`, and
`fail_closed_if`. The manifest is metadata; it is never permission to skip the
row's `gate_command`.

`contract-fixture -JsonStatus` is the offline shape fixture for wrapper tests.
It must set `offline_fixture=true`, `touches_network=false`, `read_only=true`,
`sends_prompt=false`, and `launches_process=false`. It embeds sample
`chain_status`, `entrypoint_matrix`, `status_bundle`, `contract_audit`, and
`wrapper_manifest` objects for a `quality_worker_down` scenario. Consumers may
use it for parser tests and documentation examples, but never as live health
evidence.

The plain-text `status-bundle` output is only a human projection of this JSON contract. It may show `machine summary`, `prompt policy`, `entrypoint gates`, and `allowed action table`, but prompt-producing tools must still use the machine gates with `-JsonStatus -FailIfBlocked` before sending prompts or launching pool workers.

The embedded `prompt_policy` object is the quickest automation gate. It includes `prompt_ready`, `any_prompt_allowed`, `allowed_prompt_actions`, `blocked_prompt_actions`, `allowed_read_only_actions`, `entrypoint_gates`, `block_reason`, and `next_gate`. When the quality worker is down, `any_prompt_allowed` must be `false`, all prompt entrypoints must appear under `blocked_prompt_actions`, and read-only commands such as `diagnose`, `chain-status`, `entrypoint-matrix`, `loop-status`, `pool-status`, and `recovery-plan` may remain listed in `allowed_read_only_actions`.

`prompt_policy.entrypoint_gates[]` lists the exact read-only `chain-status -RequireAction ... -FailIfBlocked` command that must pass before a caller may use each prompt-producing entrypoint. Each gate object includes `id`, `action`, current `allowed`, current `reason`, `gate_command`, `gate_read_only`, `blocked_exit_code`, `required_context_tokens`, and `sends_prompt_after_gate`. The list covers `smoke`, `web_lab_prompt`, `forge_cli_prompt`, `backend_cli_direct_prompt`, `evolution_loop_prompt_round`, and `model_pool_launch`. The evolution-loop and model-pool gates set `required_context_tokens=262144`; smoke and manual prompt gates default to `0`. Listing a gate command does not permit the prompt; only `allowed=true` from the matching gate does.

The embedded `loop` object is the same safe summary shape as `loop-status -JsonStatus`: backend prompt gate, latest evolution ledger metadata, latest daemon log metadata, redacted last-record fields, `classification`, and `action`. It must not include raw prompts, raw secrets, or prompt-producing commands.

The embedded `recovery.handoff` object is the same safe endpoint and gate summary used by `recovery-plan -JsonStatus`. It carries classification, required action, endpoint reachability, prompt readiness, context readiness, and backend gate booleans so the main coordination window can hand recovery to the owning operator without copying raw logs or credentials.

The embedded `recovery.post_recovery_release_sequence[]` object is the
machine-readable release plan after an owner restores the quality worker. It is
not a current safe-next-command list while prompts are blocked. Stable row
fields include `order`, `id`, `phase`, `command`, `downstream_action`,
`read_only`, `sends_prompt`, `launches_process`, `requires_previous_success`,
`require_action`, `required_context_tokens`, and `allowed_only_when`.

Required release sequence invariants:

- read-only rows must have `sends_prompt=false` and `launches_process=false`.
- `smoke` must appear only after `smoke_gate`.
- `evolution_loop_prompt_gate` must require `262144` context tokens.
- `model_pool_launch_gate` must require `262144` context tokens.
- the final `model_pool_launch` row describes an external owner action with
  `launches_process=true`, `sends_prompt=false`, and `command=null`.
- diagnostics must never provide an executable worker-launch command or start a
  worker themselves.

When `classification=quality_worker_down`, every prompt-producing action must remain blocked, including `model_pool_launch`. Small workers on Apple Silicon may be planned with `pool-plan`, but they must not launch as a workaround for a missing quality worker. After the quality worker is restored, the model-pool release gate is:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked
```

## Shared-Chain Etiquette

When `engine_busy=true`, diagnostics must not submit prompts by default. A busy backend often means evolution-loop or Web Lab is already using the model. The correct integration behavior is to report the owner-visible active request summary and wait.

Tiny smoke requests are acceptable only when the backend is idle. Long context-window probes and max-token stress tests should be scheduled manually.

## Apple Silicon Pool Policy

The recommended pool shape is one 12B high-quality worker plus two to four small or lower-quantization workers. This improves development throughput by moving cheap tasks off the quality queue while preserving the 12B worker for decisions that matter.

Default roles:

| Port | Role | Contract |
| --- | --- | --- |
| `8686` | `quality` | single-flight 12B worker; queue high-quality work when busy |
| `8687` | `summary` | short outputs for logs, ledgers, and documents |
| `8688` | `review` | quick second opinion and risk scan |
| `8689` | `test-gate` | test-output triage and next validation hints |
| `8690` | `index` | lightweight repository map and retrieval-prefilter helper |

Backpressure rules:

- If the quality worker is busy, do not auto-spawn another 12B Q8. Queue, wait, or route only cheap analysis to small workers.
- If small workers are busy, low-priority review and summary tasks may be skipped or delayed.
- Deterministic local tests remain authoritative; small models can suggest next checks but should not pass or fail code by themselves.
- Cloud Gemini-style adapters are separate remote roles and must keep credentials outside this repository and outside diagnostic output.

`tools/gemma-chain` may generate a pool plan, but it must not start model workers, open SSH sessions, or mutate remote machines.
