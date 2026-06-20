# Apple Silicon Gemma Model Pool Runbook

This runbook answers the operational question: can an Apple computer run multiple Gemma or Gemini-style workers to speed up development?

Short answer: yes, but do not default to multiple copies of the same 12B Q8 model on one Apple Silicon host. A better default is one high-quality 12B worker plus lightweight helper workers for summary, review, test-gate, and index tasks.

## Recommended Shape

```text
quality 12B worker
  port 8686
  owns hard generation, architecture, synthesis, difficult debugging

small summary worker
  port 8687
  owns log summaries, ledger summaries, document condensation

small review worker
  port 8688
  owns patch review, risk lists, second opinions

small test-gate worker
  port 8689
  owns test-output triage and next validation hints

small index worker
  port 8690
  owns repository map summaries, symbol/file index hints, retrieval prefiltering
```

Keep `127.0.0.1:8686` as the stable quality worker so the existing backend, Web Lab, and evolution-loop integration keep working while pool routing is designed.

The default pool shape is five endpoints: one quality 12B endpoint plus four lightweight helpers. Any spare, experimental, remote, or Gemini-style adapter must be explicitly opt-in and must not reuse the helper slots as extra 12B instances.

Do not start pool experiments while the quality worker is unreachable. The pool is an extension of a stable quality chain, not a replacement for a broken `8686` worker.

Current gate rule: when `chain-status` classifies the chain as `quality_worker_down`, `model-pool launch` must remain blocked. Fix `8686` first; do not start small workers to mask the missing quality worker.

Machine gate rule for the current incident:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd status-bundle -JsonStatus -FailIfBlocked
```

While `127.0.0.1:8686` is TCP/HTTP unreachable, both commands should report `machine_summary.model_pool_launch_allowed=false`, `prompt_policy.any_prompt_allowed=false`, and an `allowed_action_table` row where `id=model_pool_launch`, `category=launch`, and `allowed=false`. `-FailIfBlocked` exits `2`; that is the expected safe result, not a script failure. Only read-only status commands may continue.

## Why Not Multiple 12B Q8 By Default

Apple Silicon has strong local inference characteristics, but model workers share unified memory, GPU bandwidth, CPU scheduling, thermal headroom, and KV cache. Multiple 12B Q8 instances can make every request slower, increase swap pressure, and destabilize long streams.

Use one 12B worker for quality. Add small workers when they can remove cheap analysis from the quality queue.

## Port Plan

| Port | Role | Model class | Context default | max_tokens default | Primary use |
| --- | --- | --- | --- | --- | --- |
| `8686` | `quality` | Gemma 12B Q8 or best local quality model | `262144` when intentionally enabled | `262144` model-window default | code generation, architecture, final synthesis |
| `8687` | `summary` | small Gemma or low-quant local model | `8192` | `768` | logs, runbooks, ledger summaries |
| `8688` | `review` | small Gemma or low-quant local model | `8192` | `1024` | patch review, risk scan, second opinion |
| `8689` | `test-gate` | small Gemma or low-quant local model | `4096` | `512` | test failure triage and next-command hints |
| `8690` | `index` | small Gemma, embedding-capable helper, or low-quant local index model | `4096` | `512` | repository map summaries, symbol/file index hints, retrieval prefiltering |

For daily operation, the `quality` worker preserves the requested budget up to the model window. Helper workers still keep small defaults because summary, review, test-gate, and index tasks are supposed to remove cheap work from the 12B queue instead of producing model-window-length answers.

## Routing Policy

Use the `quality` 12B worker for:

- code changes that affect behavior.
- architectural decisions.
- final synthesis after small workers produce summaries.
- hard debugging where context quality matters.

Use small workers for:

- summarizing daemon logs or ledgers.
- reviewing a patch for obvious risks.
- classifying test output.
- building compact repository maps or retrieval prefilters.
- producing candidate next validation commands.

Small workers are filters and accelerators. They should not be the final authority for correctness.

Route candidates by task kind:

| TaskKind | Candidate order |
| --- | --- |
| `summary` | `summary` |
| `review` | `review` |
| `test-gate` | `test-gate`, `review` |
| `index` | `index`, `summary` |
| `quality` | `quality` |
| `spare` | legacy alias only; prefer `index` for `8690` |
| `auto` | `summary`, `review`, `test-gate`, `index` |

This is a routing preference, not a launch instruction. The route is still
blocked whenever `model_pool_launch` is blocked.

Low-priority routes intentionally do not fall back to `quality` by default.
If `review` is down, a review route should block instead of silently stealing
the 12B worker. Use `TaskKind=quality` only when the request really needs the
quality worker.

## Forge Pool Calls

`pool-status` and `pool-route` are read-only diagnostics. They never send a
prompt. After the optional small workers are running, SmartSteam Forge can send
a single auxiliary prompt to the route-plan selected worker with `pool-call`:

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --backend 127.0.0.1:7979 --pool-call review --prompt "review this patch for obvious risks"
```

Inside the TUI:

```text
/pool-call review review this patch for obvious risks
/pool call summary summarize the last test log
```

This command prefers the local `rust-norion` backend mediation endpoint:
`POST /v1/model-pool/call`. The backend applies the same quality-worker gate,
chooses the selected worker, and sends the prompt only after the route is
allowed. Older backends that do not expose `/v1/model-pool/call` fall back to
the previous compatibility path: read `/v1/model-pool/route-plan`, then call
the selected worker's `/v1/chat/completions` endpoint. The result is displayed
as an auxiliary system message and does not replace the main quality-worker
chat path.

Successful mediated calls include per-call execution evidence in the JSON
response: `elapsed_ms`, `answer_chars`, `answer_bytes`, and
`answer_approx_tokens`. Use these fields with worker metrics before adding
more Apple Silicon workers. A helper is useful only when it produces bounded
summary, review, test-gate, or index output without increasing quality-worker
latency or unified-memory pressure.

## Evolution Loop Pool Gates

Use SmartSteam Forge `--pool-smoke` for read-only diagnostics. Use
`evolution-loop` only after the read-only smoke is clean, because a successful
gate run proceeds to the requested prompt.

The pool alignment gate checks the full worker shape before a real round:

- the manifest-planned roles are visible in pool status.
- no unplanned worker role is visible.
- primary and helper route artifacts are allowed.
- one Apple host does not expand beyond `max_quality_12b_workers=1`.
- experience index quality stays at or above `0.92`, with
  `retrieval_ready=true`.

Example guarded one-round smoke:

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
  --prompt "Summarize current SmartSteam model-pool status and suggest the next validation command."
```

If a helper is missing or a route is blocked, the command fails before the
prompt with `missing_manifest_helper_roles`, `missing_status_helper_roles`,
`missing_status_roles`, `unplanned_status_roles`, or `route_blocked_or_failed`
in the error text. `missing_manifest_helper_roles` means the manifest has not
planned a helper role required by policy; `missing_status_helper_roles` means
the role is required but not visible in current worker status. Keep the stage
list explicit; it is the contract that lets `summary`, `review`, `index`, and
`test-gate` work as independent helpers instead of silently falling back to the
12B quality worker.

## rust-norion Model Pool Manifest

rust-norion can read a service-side model-pool manifest instead of the built-in
localhost defaults. This is the intended shape for an Apple Silicon host or SSH
tunnel where each role has its own endpoint.

Example:

```json
{
  "workers": [
    {
      "role": "quality",
      "base_url": "http://127.0.0.1:8686",
      "model_class": "Gemma 12B Q8 or best local quality model",
      "suggested_quant": "Q8",
      "default_context_tokens": 262144,
      "default_max_tokens": 262144,
      "low_priority": false
    },
    {
      "role": "summary",
      "base_url": "http://127.0.0.1:8687",
      "model_class": "small Gemma or low-quant local model",
      "suggested_quant": "Q4 or Q5",
      "default_context_tokens": 8192,
      "default_max_tokens": 768
    },
    {
      "role": "review",
      "base_url": "http://127.0.0.1:8688",
      "model_class": "small Gemma or low-quant local model",
      "suggested_quant": "Q4 or Q5",
      "default_context_tokens": 8192,
      "default_max_tokens": 1024
    },
    {
      "role": "test-gate",
      "base_url": "http://127.0.0.1:8689",
      "model_class": "small Gemma or low-quant local model",
      "suggested_quant": "Q4",
      "default_context_tokens": 4096,
      "default_max_tokens": 512
    },
    {
      "role": "index",
      "base_url": "http://127.0.0.1:8690",
      "model_class": "small Gemma, embedding-capable helper, or low-quant local index model",
      "suggested_quant": "Q4",
      "default_context_tokens": 4096,
      "default_max_tokens": 512
    }
  ]
}
```

Start the service with the manifest:

```powershell
cargo run -- --serve --serve-bind 127.0.0.1:7878 --model-pool-manifest D:\rust-norion\docs\runbooks\apple-model-pool.example.json
```

Generate the same manifest from the current `gemma-chain` pool plan without
starting workers or sending prompts:

```powershell
cd D:\rust-norion
New-Item -ItemType Directory -Force .\target\gemma-chain | Out-Null
.\tools\gemma-chain\gemma-chain.cmd pool-manifest > .\target\gemma-chain\apple-model-pool.generated.json
cargo run -- --serve --serve-bind 127.0.0.1:7979 --model-pool-manifest .\target\gemma-chain\apple-model-pool.generated.json
```

For day-to-day SmartSteam Forge testing, use the wrapper that performs the
manifest generation, remote chain startup, backend wiring, Web Lab startup,
and Forge launch:

```powershell
cd D:\rust-norion
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd
```

Run `-CheckOnly` first when validating a workstation: it is local-only,
generates and validates the manifest, and exits before SSH, process launch, or
prompt submission.

The default remote wrapper starts only the `8686` quality worker. The generated
manifest still advertises the pool roles, but helper workers are launched only
when the operator provides a remote small GGUF explicitly:

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf
```

That starts `summary=8687`, `review=8688`, `test-gate=8689`, and `index=8690`
helpers and their local SSH tunnels. Do not use any helper slot for a second
12B instance.

When the Apple host has role-specific helper models, keep the same port plan
and override only the model path for each role. Unspecified roles fall back to
`-RemoteSmallModel`, and every helper still runs behind the local model-pool
manifest:

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly -NoForge -EnablePoolWorkers `
  -RemoteSummaryModel /Users/xinghuan/smartsteam-model-box/models/gemma-summary-Q4.gguf `
  -RemoteReviewModel /Users/xinghuan/smartsteam-model-box/models/gemma-review-Q4.gguf `
  -RemoteTestGateModel /Users/xinghuan/smartsteam-model-box/models/gemma-test-gate-Q4.gguf `
  -RemoteIndexModel /Users/xinghuan/smartsteam-model-box/models/gemma-index-Q4.gguf
```

The wrapper also accepts `-RemoteSummaryLlamaServer`,
`-RemoteReviewLlamaServer`, `-RemoteTestGateLlamaServer`, and
`-RemoteIndexLlamaServer` when a role needs a different serving binary. The
per-role context and budget knobs are `-SummaryContextTokens`,
`-ReviewContextTokens`, `-TestGateContextTokens`, `-IndexContextTokens`,
`-SummaryDefaultMaxTokens`, `-ReviewDefaultMaxTokens`,
`-TestGateDefaultMaxTokens`, and `-IndexDefaultMaxTokens`. For generated
manifests, these values are written into `workers[].default_context_tokens`
and `workers[].default_max_tokens` before the local rust-norion backend reads
the manifest.

Stop the remote chain with:

```powershell
.\tools\smartsteam-forge\stop-remote-gemma-chain.cmd -DryRun
.\tools\smartsteam-forge\stop-remote-gemma-chain.cmd
```

The same path can be supplied with `SMARTSTEAM_MODEL_POOL_MANIFEST` or
`NORION_MODEL_POOL_MANIFEST` when running in service mode.

## Backpressure

Current single-worker chain behavior stays simple:

```text
engine_busy=true -> record status only, do not send prompts
engine_busy=false + quality reachable + readiness OK -> tiny smoke or manual prompt is allowed
quality unreachable or readiness failed -> prompt blocked even if engine_busy=false
```

Future pool behavior should be per role:

```text
quality busy + quality task -> queue or wait
quality busy + summary task -> try summary worker
small worker busy + low-priority task -> skip, retry, or delay
all workers busy -> no prompt, report backpressure
```

Do not start another 12B Q8 instance automatically to bypass `engine_busy`. That usually moves the bottleneck into unified memory and GPU contention.

## Startup Prerequisite

Before starting any model-pool worker or pool router, the quality chain must pass:

```powershell
cd D:\rust-norion
.\tools\gemma-chain\gemma-chain.cmd chain-status
.\tools\gemma-chain\gemma-chain.cmd diagnose
.\tools\gemma-chain\gemma-chain.cmd prompt-gate
```

Required fields:

- `gemma_runtime_reachable=true`
- `readiness_ok=true`
- `safe_device_ok=true`
- `engine_busy=false` before optional smoke
- `prompt_ready=True` from `prompt-gate`
- `model-pool launch allowed=True` from the `chain-status` allowed action table

If `127.0.0.1:8686` is tcp/http unreachable, do not use the pool as a workaround. Record the state and hand recovery to the model-chain owner.

Safe owner handoff command:

```powershell
.\tools\gemma-chain\gemma-chain.cmd recovery-plan
```

Use this instead of copying raw logs or shell history.

Machine-readable gate:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked
```

Read-only wait gate after the model-chain owner starts restoring the quality worker:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -WaitReady -WaitTimeoutSec 120 -PollIntervalSec 5 -JsonStatus -FailIfBlocked
```

This polls health only. It does not start workers and does not send prompts.

Expected blocked shape while quality is down:

```text
classification=quality_worker_down
prompt_gate.prompt_ready=false
prompt_gate.context_ready=false if n_ctx is below the requested window
allowed_actions[model-pool launch].allowed=false
```

Only after this gate becomes prompt-ready should the operator enable the recommended shape: one 12B quality worker on `8686` plus lightweight `summary`, `review`, `test-gate`, and `index` helpers on `8687-8690`.

The same rule is exposed in the recovery contract:

```powershell
.\tools\gemma-chain\gemma-chain.cmd recovery-plan -JsonStatus
```

Read `post_recovery_release_sequence[]` when a wrapper needs a machine-readable
release checklist. The `model_pool_launch_gate` row must pass before any pool
worker is started. The following `model_pool_launch` row is intentionally not
an executable command; it has `command=null`, `launches_process=true`, and
`sends_prompt=false` to show that the model-chain owner, not diagnostics, owns
the actual launch. While `8686` is unreachable, this row remains blocked by
the gate and must not be used as a workaround.

Web Lab availability is useful for UI testing but is not the quality-worker prerequisite for the pool. If `chain-status` reports `web_lab_down`, Web Lab UI and Web Lab smoke remain blocked, while backend-only pool planning may proceed only if `model-pool launch allowed=True`.

## Pool Release Sequence

Use this sequence after `8686` recovers. Do not skip directly from recovery to multiple workers.

1. Prove the quality worker is back:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status
.\tools\gemma-chain\gemma-chain.cmd prompt-gate
.\tools\gemma-chain\gemma-chain.cmd diagnose
```

2. Run one tiny smoke through the existing chain:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -JsonStatus -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd smoke
```

3. Run one backend-only or Forge check only if the allowed action table permits it.

4. Keep one 12B `quality` worker on `8686`. Do not launch a second 12B Q8 worker for throughput.

5. Add small workers one role at a time:

```text
8687 summary
8688 review
8689 test-gate
8690 index
```

6. After each worker is added by the model-chain owner, rerun:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status
.\tools\gemma-chain\gemma-chain.cmd pool-plan
```

7. If the 12B quality worker becomes unreachable again, immediately block pool launch and stop using small workers as a workaround.

The pool is for parallel low-cost summaries, reviews, test triage, and repository indexing around a healthy quality worker. It is not a high-availability substitute for the quality worker.

## Gemini-Style Adapters

Gemini is normally a remote/cloud model family rather than a local Apple Silicon worker. If a cloud adapter is added later, treat it as a separate explicit opt-in role, such as `remote-quality`, with strict secret boundaries:

- no API keys in this repo.
- no credentials in diagnostics.
- no prompt content with secrets.
- no automatic fallback from local Gemma to cloud Gemini unless the operator explicitly enables it.

## Read-Only Planning Command

Print the current suggested pool plan:

```powershell
cd D:\rust-norion
.\tools\gemma-chain\gemma-chain.cmd pool-plan
```

Print the same plan as machine-readable JSON:

```powershell
.\tools\gemma-chain\gemma-chain.cmd pool-plan -JsonPlan
```

Print the rust-norion service manifest shape directly:

```powershell
.\tools\gemma-chain\gemma-chain.cmd pool-manifest
.\tools\gemma-chain\gemma-chain.cmd pool-manifest -JsonStatus
```

These commands do not start models, open SSH connections, or send prompts.
Each worker row includes `SuggestedQuant`, `EnabledByDefault`,
`LaunchesProcess=false`, and `RequiresQualityGate=true` so future pool routers
can consume the plan without mistaking it for a launch command.
`pool-plan -JsonPlan` uses the public PowerShell plan shape with PascalCase
fields. Runtime status commands such as `pool-status -JsonStatus` and
`pool-route-plan -JsonStatus` use the snake_case machine contract consumed by
Rust tools.
All three machine-readable pool commands expose the same version contract:
`schema_version=1` or `SchemaVersion=1`, and
`contract_version=gemma-chain.v1` or `ContractVersion=gemma-chain.v1`.
Pool wrappers should fail closed if these fields are missing or unknown.

Check the current worker ports without starting models or sending prompts:

```powershell
.\tools\gemma-chain\gemma-chain.cmd pool-status
.\tools\gemma-chain\gemma-chain.cmd pool-status -JsonStatus
```

`pool-status` merges the static plan with TCP/health probes for `8686-8690` and
the existing `model_pool_launch` gate. It remains useful while the quality
worker is down because it shows which worker ports are actually reachable, while
still reporting that launch is blocked until the quality gate passes.
Each worker row also exposes normalized readiness fields:

- `status`: `healthy`, `tcp_only`, or `unreachable`.
- `role_ready`: true only when the worker health check passes.
- `role_block_reason`: `none`, `health_failed`, or `tcp_unreachable`.

By default, `pool-status` applies the planned quality-worker context gate
(`262144`) even if the operator does not pass `-MinContextTokens`. Pass
`-MinContextTokens` only when intentionally testing a different pool release
threshold.

Plan where a task would go without starting workers or sending prompts:

```powershell
.\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind summary
.\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind review -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind test-gate -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind index -JsonStatus
```

`pool-route-plan` is read-only. It reports `launches_process=false` and
`sends_prompt=false`, returns candidate roles for the requested `TaskKind`, and
sets `route_allowed=false` whenever `model_pool_launch` is blocked. This lets
the development loop reason about role routing while still respecting the
quality-worker gate.

When `evolution-loop` runs with `-RequirePoolRoute`, it passes the selected
worker as a `pool_dispatch` object to rust-norion's
`/v1/business-cycle-stream` endpoint. rust-norion consumes
`pool_dispatch.effective_max_tokens` as the generation budget. If the active
backend runtime supports endpoint override, such as `MistralRsHttpRuntime`, the
generation request is forwarded to `pool_dispatch.selected_base_url` for that
round and final JSON reports `pool_dispatch.worker_forwarded=true` with
`dispatch_mode=runtime_endpoint_override`. If the active backend does not
support dynamic endpoints, rust-norion keeps the request on the configured
backend, still applies the selected worker token budget, and reports
`worker_forwarded=false` with `dispatch_mode=backend_budget_only`.
The selected endpoint is validated before generation; if the runtime rejects
`pool_dispatch.selected_base_url`, rust-norion fails the request without
sending a prompt to any model worker.

For multi-role development loops, `evolution-loop` can also attach a
`pool_stage_dispatch` array to `/v1/business-cycle-stream`. Each item describes
a planned helper stage such as `summary`, `review`, or `test-gate`, including
the selected role, endpoint, runtime metadata, and effective token budget.
rust-norion currently treats this array as an observed plan only: it emits
business-cycle meta, includes the array in final JSON, and annotates the
generated experience, but it does not send extra prompts or start additional
workers. This keeps Apple Silicon resource usage stable while the routing
contract is promoted from read-only evidence to real multi-stage execution.

The JSON plan also includes:

- `BlockedPolicy`: model-pool launch stays blocked while `8686` is unreachable or readiness is false.
- `QualityGateCommand`: the machine gate that must pass before pool launch.
- `Validation`: read-only checks only.
- `PromptValidationAfterGate`: commands that may send a prompt only after the matching `chain-status -RequireAction` gate succeeds.

## Verification Commands

Use this read-only verification sequence after changing the tool or before reporting current pool state:

```powershell
.\tools\gemma-chain\gemma-chain.cmd selftest
.\tools\gemma-chain\gemma-chain.cmd prompt-gate -MinContextTokens 262144 -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd diagnose
.\tools\gemma-chain\gemma-chain.cmd pool-plan -JsonPlan
.\tools\gemma-chain\gemma-chain.cmd pool-manifest -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd pool-status -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind review -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd pool-route-plan -TaskKind index -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd loop-status -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd status-bundle -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd contract-audit -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd wrapper-manifest -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd contract-fixture -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd handoff-report -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd secret-scan -JsonStatus -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked
```

This includes in-memory checks for `backend_down`, `engine_busy`, `quality_worker_down`, `web_lab_down`, `prompt_ready`, undersized context, pool-plan gate fields, loop status embedding in `status-bundle`, and the machine-readable `chain-status` / `pool-status` JSON contracts. If the last command exits `2`, model-pool launch remains blocked by design.

For main-window handoff, prefer `status-bundle -JsonStatus`: it includes `recovery.handoff` with safe endpoint reachability and prompt gate fields, plus `loop_classification`, `prompt_policy`, pool launch state, and read-only next commands. If `prompt_policy.any_prompt_allowed=false`, model-pool launch remains blocked and only `prompt_policy.allowed_read_only_actions` may be used. `prompt_policy.entrypoint_gates[]` includes the exact `model_pool_launch` gate command plus its current `allowed`, `reason`, `gate_read_only`, `blocked_exit_code`, and `required_context_tokens`; it is still read-only and must pass before any worker launch is considered. Do not paste raw daemon logs, credentials, SSH commands, or tokens into the handoff.

For a short human summary, use `handoff-report -JsonStatus` or plain
`handoff-report`. It repeats the Apple Silicon decision as either "plan only;
model-pool launch blocked" or "pool launch gate may be checked immediately
before launch." It never starts workers.

Pool launch wrappers should also check `status-bundle.wrapper_audit`. If `wrapper_decision=read_only_only` or `fail_closed_required=true`, keep pool launch blocked and run only read-only status commands. If `wrapper_decision=action_gate_required`, the wrapper still must run the exact `model_pool_launch` gate before starting any worker:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked
```

Before wiring a pool launcher to these fields, run the contract verifier:

```powershell
.\tools\gemma-chain\gemma-chain.cmd contract-audit -JsonStatus
```

`audit_passed=true` means the fail-closed contract is internally consistent:
the launch row is a process gate, not a prompt gate; the release sequence
requires the `model_pool_launch_gate`; and quality-worker down still implies
`model_pool_launch_allowed=false`. It does not mean launch is allowed. The
wrapper must still respect `model_pool_launch_allowed` and rerun the exact
`chain-status -RequireAction model_pool_launch -MinContextTokens 262144`
command before starting workers.

Pool wrappers can also read the field-level manifest:

```powershell
.\tools\gemma-chain\gemma-chain.cmd wrapper-manifest -JsonStatus
```

Use the `entrypoints[]` row where `id=model_pool_launch`. It should identify
the entrypoint as `entrypoint_kind=launch`, `downstream_launches_process=true`,
`downstream_sends_prompt=false`, and a `gate_command` with
`-MinContextTokens 262144`. If that row is missing, has an unknown contract
version, or reports `current_allowed=false`, the launcher must fail closed.

For parser tests that must not touch the running chain, use the offline fixture:

```powershell
.\tools\gemma-chain\gemma-chain.cmd contract-fixture -JsonStatus
```

It always models `quality_worker_down`, so `model_pool_launch_allowed=false`.
Use it to test JSON parsing and fail-closed behavior, not to decide current
runtime health.

Before sharing a pool handoff or adding a cloud/Gemini-style adapter, run:

```powershell
.\tools\gemma-chain\gemma-chain.cmd secret-scan -JsonStatus -FailIfBlocked
```

Pool and cloud-adapter notes must not contain API keys, bearer tokens,
password assignments, cookies, or private keys. If the scan fails, redact the
source line before reporting.

For a single-row pool decision, use the lightweight matrix or read the embedded matrix from chain/status bundle:

```powershell
.\tools\gemma-chain\gemma-chain.cmd entrypoint-matrix -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked
```

Read `entrypoints[]` from `entrypoint-matrix`, or `integration_entrypoints[]` from `chain-status -JsonStatus` / `status-bundle -JsonStatus`, where `id=model_pool_launch`. In the current `8686` down state that row should report `entrypoint_kind=launch`, `current_allowed=false`, `downstream_launches_process=true`, `downstream_sends_prompt=false`, `safe_when_blocked=true`, and `blocked_by=quality_worker_down`.

For the user's Apple Silicon throughput question, report this concise recommendation: yes, run multiple workers only after the quality chain is healthy. Do not make several `12B Q8` copies the default. Use one `12B` high-quality worker on `8686`, then add lightweight helpers on `8687-8690` for summary, review, test-gate, and index work. If `machine_summary.model_pool_launch_allowed=false`, the answer is "plan only; launch blocked until 8686 is reachable and readiness passes."

When worker events exist, prove that the pool is helping rather than just
burning memory by passing a worker artifact to `tools/evolution-loop` with
`--pool-budget-fairness-json`. The additive
`model_pool_budget_fairness_report_v1` should show `allow_pool_expansion=true`
only when summary/index, review/reviewer, and test-gate/tester workers all
produce successful feedback, no role consumes more than 60% of runtime tokens,
and no helper blocks the primary 12B path.

Recommended main-window summary while the current incident persists:

```text
Apple Silicon can help by running a pool, but only after the quality chain is healthy.
Current state: 8686 quality worker unreachable, so model-pool-launch is blocked.
Default recovery shape: 8686 one 12B quality worker, then 8687 summary, 8688 review, 8689 test-gate, 8690 index.
Do not start multiple 12B Q8 copies by default; they compete for unified memory, GPU bandwidth, and KV cache.
Validation: selftest, prompt-gate, diagnose, pool-plan, chain-status/entrypoint-matrix/status-bundle gates.
```

Only after the quality worker recovers and the smoke gate succeeds should an operator run the tiny smoke:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -JsonStatus -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd smoke
```

If `diagnose` reports `engine_busy=true` or `chain-status` reports `quality_worker_down`, stop at read-only status and let the active owner recover the chain.
