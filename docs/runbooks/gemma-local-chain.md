# Gemma Local Chain Runbook

This runbook covers the current outer integration chain for Gemma 12B:

```text
remote Mac llama-server or tunnel
  -> local model API http://127.0.0.1:8686
  -> rust-norion backend http://127.0.0.1:7979
  -> Web Lab http://127.0.0.1:8789
  -> CLI/UI/evolution-loop tests
```

It intentionally avoids secrets. Do not paste historical tokens, SSH passwords, private keys, model download credentials, or remote shell history into this repo.

## One Command

From `D:\rust-norion`, run the read-mostly diagnostic:

```powershell
.\tools\gemma-chain\gemma-chain.cmd diagnose
```

Run the script-only self check after editing diagnostics. It does not connect to the model, backend, or Web Lab:

```powershell
.\tools\gemma-chain\gemma-chain.cmd selftest
```

`selftest` validates local redaction, SSE continuity, chain classifications, and the allowed action table with in-memory fixtures only.

Print the Apple Silicon multi-model pool plan without starting any model:

```powershell
.\tools\gemma-chain\gemma-chain.cmd pool-plan
```

Print the rust-norion `--model-pool-manifest` JSON generated from that plan:

```powershell
.\tools\gemma-chain\gemma-chain.cmd pool-manifest
```

Start the remote Gemma chain, generate that manifest, wire it into the local
backend, and open SmartSteam Forge in one wrapper command:

```powershell
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd
```

`-CheckOnly` is local-only: it generates and validates the pool manifest,
prints the intended ports and target host, then exits before SSH, process
launch, or any prompt.

Check whether any prompt entrypoint is allowed:

```powershell
.\tools\gemma-chain\gemma-chain.cmd prompt-gate
```

Classify the whole chain and print the allowed action table:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status
```

Print only the prompt/launch entrypoint matrix for Web Lab, Forge, CLI, evolution-loop, smoke, and model-pool wrappers:

```powershell
.\tools\gemma-chain\gemma-chain.cmd entrypoint-matrix
.\tools\gemma-chain\gemma-chain.cmd entrypoint-matrix -JsonStatus -FailIfBlocked
```

Print a safe recovery handoff bundle:

```powershell
.\tools\gemma-chain\gemma-chain.cmd recovery-plan
```

Print a concise main-window handoff summary:

```powershell
.\tools\gemma-chain\gemma-chain.cmd handoff-report
.\tools\gemma-chain\gemma-chain.cmd handoff-report -JsonStatus
```

`handoff-report` summarizes the current classification, endpoint reachability,
blocked prompt actions, safe next read-only commands, Apple Silicon pool
decision, and the exact model-pool gate. It is read-only and does not replace
the action-specific `chain-status -RequireAction ... -FailIfBlocked` gate.

Scan the Gemma chain docs and tools for accidental credential-shaped text:

```powershell
.\tools\gemma-chain\gemma-chain.cmd secret-scan
.\tools\gemma-chain\gemma-chain.cmd secret-scan -JsonStatus -FailIfBlocked
```

`secret-scan` is read-only. It scans only `docs/runbooks/gemma*.md`,
`docs/architecture/integration*.md`, and `tools/gemma-chain/**` for high-risk
secret shapes such as private keys, bearer tokens, `sk-...` keys, and
assignment-like token, password, or API-key fields. It reports path,
line, rule id, and a safe preview; do not paste raw matched text into handoff.

Machine-readable recovery output includes both `post_recovery_validation`
for compatibility and `post_recovery_release_sequence[]` for wrappers that
need an ordered release plan. The structured rows mark whether each step is
read-only, whether the downstream step sends a prompt, whether it launches a
process, the required `chain-status -RequireAction ...` gate, and any context
threshold. While `8686` is down this sequence is only a plan; it is not a safe
next-command list.

Audit the wrapper-facing contract without sending prompts:

```powershell
.\tools\gemma-chain\gemma-chain.cmd contract-audit
.\tools\gemma-chain\gemma-chain.cmd contract-audit -JsonStatus
```

`contract-audit` consumes the same read-only `status-bundle` state and checks
that version fields, safe-next commands, entrypoint gates, model-pool launch
shape, and post-recovery release ordering are still fail-closed. It is a
contract verifier, not permission to send a prompt or launch a worker.

Print the field-level manifest for Web Lab, Forge, CLI, evolution-loop, and
model-pool wrappers:

```powershell
.\tools\gemma-chain\gemma-chain.cmd wrapper-manifest
.\tools\gemma-chain\gemma-chain.cmd wrapper-manifest -JsonStatus
```

`wrapper-manifest` tells each wrapper which fields to consume, which
`gate_command` must pass immediately before the downstream action, and whether
that action sends a prompt or launches a process. It is read-only status data;
it must not be treated as a prompt or launch approval.

Print an offline contract sample without probing any localhost endpoint:

```powershell
.\tools\gemma-chain\gemma-chain.cmd contract-fixture -JsonStatus
```

`contract-fixture` is a pure in-memory `quality_worker_down` fixture. It embeds
sample `chain_status`, `entrypoint_matrix`, `status_bundle`, `contract_audit`,
and `wrapper_manifest` objects so wrappers can develop against the JSON shape
without contacting `8686`, `7979`, or `8789`. It is not a health check and must
not be used as approval to send prompts.

Machine-readable prompt gate:

```powershell
.\tools\gemma-chain\gemma-chain.cmd prompt-gate -JsonStatus
```

`prompt-gate -JsonStatus` is backend-only and read-only. It does not check Web Lab TCP state and does not send a prompt. Use `entrypoint_decisions[]` for stable action ids that match `chain-status -RequireAction`: `smoke`, `web_lab_prompt`, `forge_cli_prompt`, `backend_cli_direct_prompt`, `evolution_loop_prompt_round`, and `model_pool_launch`. The older `entrypoints.web_lab_manual_prompt` field is kept only as a compatibility alias for `web_lab_prompt`.

Machine-readable status outputs use `schema_version=1` and `contract_version=gemma-chain.v1`. Wrappers should check those fields before trusting gate decisions, then fail closed if a future contract version is unknown.

Check the latest evolution-loop ledger and daemon logs without sending prompts:

```powershell
.\tools\gemma-chain\gemma-chain.cmd loop-status
```

Machine-readable loop gate:

```powershell
.\tools\gemma-chain\gemma-chain.cmd loop-status -JsonStatus
```

When the backend is idle and prompt-ready, run the smallest end-to-end SSE smoke:

```powershell
.\tools\gemma-chain\gemma-chain.cmd smoke
```

If `smoke` reports `engine_busy=true`, wait for the current evolution round or user request to finish. Do not stop or restart the model just to run the smoke.

## Operator Flow

Use this order when sharing the chain with evolution-loop:

1. Run `diagnose`.
2. If `engine_busy=true`, stop there. Record the active request id, endpoint, elapsed time, and safe preview status. Do not send Web Lab, Forge, CLI, or smoke prompts.
3. If `gemma_runtime_reachable` or `readiness_ok` is not true, stop there. Record the failed gate and do not send prompts.
4. If `engine_busy=false`, `gemma_runtime_reachable=true`, `readiness_ok=true`, and `safe_device_ok=true`, run `smoke`.
5. After smoke passes, open Web Lab at `http://127.0.0.1:8789/`.
6. If Forge CLI testing is needed, keep the prompt tiny and require backend health:

```powershell
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --backend 127.0.0.1:7979 --mode chat --prompt "Reply only with OK." --require-health --timeout-secs 60
```

Return to `diagnose` between manual tests if evolution-loop may have resumed.

## Port Responsibilities

| Port | Owner | Role | Safe check |
| --- | --- | --- | --- |
| `127.0.0.1:8686` | Gemma model service or SSH tunnel | llama-server compatible model API. The remote Mac owns model loading and weights. | `GET /health`, optionally `GET /v1/models` |
| `127.0.0.1:7979` | rust-norion backend | Adds request routing, experience state, device/readiness gates, business-cycle streams, and metadata projection. | `GET /health` |
| `127.0.0.1:8789` | Web Lab | Browser UI and SSE proxy to the backend. | `GET /health`, `GET /api/backend-health` |

The remote Mac should only receive model-serving commands and model files already present there. Do not copy rust-norion source to the remote machine as part of this chain.

For a future Apple Silicon pool, reserve adjacent model ports rather than replacing the stable `8686` quality worker:

| Port | Suggested role | Default budget | Use |
| --- | --- | --- | --- |
| `8686` | `quality` 12B worker | context `262144`, max tokens `262144` | generation, architecture, final synthesis |
| `8687` | `summary` small worker | context `8192`, max tokens `768` | logs, ledger summaries, runbook condensation |
| `8688` | `review` small worker | context `8192`, max tokens `1024` | patch review, risk scan, second opinion |
| `8689` | `test-gate` small worker | context `4096`, max tokens `512` | test output triage and next-command hints |
| `8690` | `index` small worker | context `4096`, max tokens `512` | repository maps, retrieval prefilters, and context hints |

Do not make multiple 12B Q8 workers the default on one Apple Silicon host. They compete for unified memory, GPU residency, and KV cache, often reducing total throughput. Prefer one high-quality 12B worker plus two to four smaller or lower-quantization workers.

## Current Healthy Shape

A healthy backend `/health` normally includes:

- `runtime_mode="gemma-http"`
- `gemma_runtime_reachable=true`
- `gemma_runtime_model` similar to `gemma-4-12b-it-Q8_0.gguf`
- `gemma_runtime_context_window=262144`
- `gemma_runtime_train_context_window=262144`
- `readiness_ok=true`
- `safe_device_ok=true`
- `engine_busy=false` when no inference is active

If `engine_busy=true`, the chain may still be healthy. Treat it as occupied, not broken. The health payload usually lists `active_requests` with endpoint, request id, elapsed time, and prompt preview. Wait unless the owning operator explicitly asks you to intervene.

Prompt tests require all of these to be true: `engine_busy=false`, `gemma_runtime_reachable=true`, `readiness_ok=true`, and `safe_device_ok=true`. An idle backend with an unreachable model is not prompt-ready.

Web Lab enforces the same prompt gate server-side before forwarding every
`POST /api/chat-stream` request. The browser send button is only the UX layer;
direct POSTs still trigger a fresh backend `/health` check. If the backend is
busy, the Gemma runtime is unreachable, readiness fails, or the safe-device
gate fails, Web Lab returns SSE `status`, `error`, then `done` without calling
`/v1/chat-stream`, `/v1/generate-stream`, or `/v1/business-cycle-stream`.

## 8686 Unreachable Prompt Block

If `127.0.0.1:8686` is not reachable, treat the quality worker as unavailable even when the backend is online and `engine_busy=false`.

Blocked entrypoints while `gemma_runtime_reachable=false` or `readiness_ok=false`:

- `tools\gemma-chain smoke`
- Web Lab manual chat prompt
- Forge CLI prompt
- direct backend chat or business-cycle prompt
- new evolution-loop prompt rounds
- model-pool startup or pool routing tests

Allowed read-only commands:

```powershell
.\tools\gemma-chain\gemma-chain.cmd selftest
.\tools\gemma-chain\gemma-chain.cmd diagnose
.\tools\gemma-chain\gemma-chain.cmd chain-status
.\tools\gemma-chain\gemma-chain.cmd entrypoint-matrix
.\tools\gemma-chain\gemma-chain.cmd recovery-plan
.\tools\gemma-chain\gemma-chain.cmd status-bundle
.\tools\gemma-chain\gemma-chain.cmd prompt-gate
.\tools\gemma-chain\gemma-chain.cmd prompt-gate -JsonStatus -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd loop-status
.\tools\gemma-chain\gemma-chain.cmd loop-status -JsonStatus -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd pool-plan
.\tools\gemma-chain\gemma-chain.cmd pool-status
.\tools\evolution-loop\start-evolution-loop.cmd -Report
.\tools\evolution-loop\start-evolution-loop.cmd -ReportGate
```

Do not use Web Lab or Forge as a recovery probe while 8686 is unreachable. Those paths submit prompts. Use `diagnose` and `prompt-gate` until the quality worker is reachable and backend readiness is true.

## Quality Worker Recovery Checklist

Before an operator restores or hands off the quality worker, collect only safe, non-secret facts:

1. Confirm backend and Web Lab are still reachable:

```powershell
.\tools\gemma-chain\gemma-chain.cmd diagnose
```

2. Confirm prompt entrypoints are blocked while quality is down:

```powershell
.\tools\gemma-chain\gemma-chain.cmd prompt-gate
```

3. Print the safe handoff bundle for the model-chain owner:

```powershell
.\tools\gemma-chain\gemma-chain.cmd recovery-plan
```

It includes status fields and recovery steps, but no raw prompts, credentials, SSH commands, or private keys.

4. Check whether evolution-loop is only reporting, already stopped, or blocked by quality-worker health:

```powershell
.\tools\gemma-chain\gemma-chain.cmd loop-status
```

Do not kill or restart it from diagnostics.

5. Ask the model-chain owner to restore the tunnel or model worker if needed. Do not SSH, paste credentials, or restart shared services from this runbook.

6. After the owner reports recovery, rerun `diagnose`, `prompt-gate`, and `chain-status -RequireAction smoke -JsonStatus -FailIfBlocked`. Only run `smoke` after that gate passes.

## Post-Recovery Release Sequence

After the model-chain owner reports that the `8686` quality worker or tunnel is restored, release prompt-producing entrypoints in this order. Stop at the first failed gate.

1. Confirm the whole chain is classified:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status
```

Required before moving on:

- `classification=prompt_ready`, or `classification=web_lab_down` only if you are intentionally testing backend-only paths.
- `quality_worker tcp=True health=True`.
- `backend tcp=True health=True`.
- `prompt_ready=True`.

2. Confirm backend gate details:

```powershell
.\tools\gemma-chain\gemma-chain.cmd prompt-gate
```

Required before any prompt:

- `engine_busy=False`
- `gemma_runtime_reachable=True`
- `readiness_ok=True`
- `safe_device_ok=True`

3. Run read-mostly diagnosis to capture context and metadata:

```powershell
.\tools\gemma-chain\gemma-chain.cmd diagnose
```

Metadata timeouts are warnings only if `prompt-gate` is already prompt-ready and generation smoke succeeds. If metadata and generation both fail, return to the recovery owner.

4. Run the tiny Web Lab SSE smoke only when the explicit smoke gate passes:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -JsonStatus -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd smoke
```

5. After smoke passes, allow one manual Web Lab test:

```text
http://127.0.0.1:8789/
```

Keep the prompt tiny, and rerun the dedicated gate immediately before sending
it:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction web_lab_prompt -JsonStatus -FailIfBlocked
```

If Web Lab is down but backend-only actions are allowed, skip UI testing and
repair Web Lab separately.

6. Allow one Forge CLI prompt only after `chain-status` says `forge cli prompt allowed=True`:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction forge_cli_prompt -JsonStatus -FailIfBlocked
cd D:\rust-norion\tools\smartsteam-forge
cargo run -- --backend 127.0.0.1:7979 --mode chat --prompt "Reply only with OK." --require-health --require-safe-device --timeout-secs 60
```

7. Before resuming evolution-loop prompt rounds, check loop state:

```powershell
.\tools\gemma-chain\gemma-chain.cmd loop-status
```

Resume only if the loop is not already active and `chain-status` allows `evolution-loop prompt round`.

8. Only after the quality worker is stable should model-pool launch be considered:

```powershell
.\tools\gemma-chain\gemma-chain.cmd pool-plan
.\tools\smartsteam-forge\start-remote-gemma-forge.cmd -CheckOnly -EnablePoolWorkers -RemoteSmallModel /Users/xinghuan/smartsteam-model-box/models/gemma-small-Q4.gguf
```

Do not enable small workers before the `8686` quality worker is prompt-ready.
`tools/gemma-chain` remains read-only; the optional process launch lives in the
SmartSteam Forge remote wrapper and requires an explicit remote small-model
path.

Abort and return to read-only diagnostics if any of these appear:

- `engine_busy=True` while another owner is active.
- `gemma_runtime_reachable=False`.
- `readiness_ok=False`.
- `safe_device_ok=False`.
- stream missing `event: done` or ending with a partial SSE frame.
- `classification=quality_worker_down`, `backend_down`, or `prompt_blocked`.

## Context Window And max_tokens

The current Gemma service reports a 262144 token context window through the backend health projection. That is a capacity signal, not permission to send huge test prompts during shared development.

For integration checks:

- Prefer tiny prompts such as `Reply only with OK.`
- Prefer short max token budgets when the caller supports them.
- Do not run long context-window probes while evolution-loop is active.
- Use health metadata to confirm `n_ctx` before scheduling any long-context test.

If metadata is unavailable but `/health` is OK, record the metadata failure and retry later. Do not restart the model for a transient metadata timeout.

In a model pool, use smaller defaults for non-quality roles. Summary, review, and test-gate workers should use explicit short max token budgets so they do not starve the 12B worker.

For the quality Gemma path, stable HTTP retry must preserve the caller's
`max_tokens` budget. It may lower sampling temperature to recover from
unstable sampling errors, but it must not silently clamp long Web Lab or Forge
requests down to a tiny 64-token retry. If a response is unexpectedly short,
check the selected model-pool role and `effective_max_tokens` first: helper
roles intentionally use smaller budgets, while the quality worker keeps the
requested long budget.

## engine_busy Handling

Default behavior for `tools\gemma-chain`:

- `health`: only reads health endpoints.
- `diagnose`: reads endpoints and classifies common failures.
- `smoke`: checks backend health first; if `engine_busy=true`, it exits without sending a prompt.
- `selftest`: validates local redaction and SSE continuity fixtures only.
- `pool-plan`: prints an Apple Silicon model-pool plan only.
- `pool-manifest`: prints the rust-norion `--model-pool-manifest` JSON derived from the plan and never starts workers.
- `pool-status`: reads `8686-8690` model-pool worker ports, merges the static pool plan with the launch gate, and never starts workers.
- `status-bundle`: prints one read-only owner handoff bundle with chain status, loop status, pool status, recovery steps, and safe next commands.
- `handoff-report`: prints the concise main-window handoff summary derived from the same read-only status bundle.
- `contract-audit`: verifies the wrapper-facing machine contract and fail-closed invariants using only read-only status data.
- `wrapper-manifest`: prints the wrapper-facing field consumption manifest and exact gate commands for each entrypoint.
- `contract-fixture`: prints an offline `quality_worker_down` JSON fixture without endpoint probes.
- `secret-scan`: scans the Gemma chain docs/tools for credential-shaped text without printing raw secrets.
- `chain-status`: reads model/backend/Web Lab health, classifies the chain, and prints the allowed action table.
- `recovery-plan`: reads chain status and prints safe owner handoff fields plus post-recovery validation commands.
- `prompt-gate`: reads backend health and reports whether smoke, Web Lab, Forge, CLI, evolution-loop, and pool startup may send prompts.
- `loop-status`: reads backend health plus latest `target\evolution` ledger and daemon log tails; redacts prompt lines and classifies common evolution-loop gate failures.

`status-bundle -JsonStatus` embeds the same safe loop summary under `loop`, with top-level `loop_classification` and `loop_action` for automation. It also embeds the safe recovery endpoint summary under `recovery.handoff`, matching `recovery-plan -JsonStatus` without raw logs or credentials. Use `prompt_policy.any_prompt_allowed` as the fast machine gate: when it is `false`, use only `prompt_policy.allowed_read_only_actions` and do not run any item in `prompt_policy.blocked_prompt_actions`. Use `prompt_policy.entrypoint_gates[]` for the exact `chain-status -RequireAction ... -FailIfBlocked` command that must pass before smoke, Web Lab, Forge, direct backend CLI, evolution-loop, or model-pool launch is allowed; each row also carries the current `allowed` boolean, block `reason`, `gate_read_only=true`, `blocked_exit_code=2`, and any `required_context_tokens`. In a blocked state, `safe_next_commands` should include read-only `loop-status -JsonStatus` but should not include `smoke` or any prompt-producing command.

`recovery.post_recovery_release_sequence[]` is the ordered plan for after the
quality worker owner reports `8686` restored. Its early rows are read-only
status checks. `smoke` appears only after `smoke_gate`, evolution-loop release
requires `evolution_loop_prompt_round` with `-MinContextTokens 262144`, and
model-pool release requires `model_pool_launch` with `-MinContextTokens
262144`. The final `model_pool_launch` row describes an owner action outside
diagnostics: `launches_process=true`, `sends_prompt=false`, and `command=null`.
`tools/gemma-chain` must never start model workers itself.

`prompt-gate -JsonStatus` has a smaller `entrypoint_decisions[]` array for wrappers that only need backend health. Each row includes the standard action `id`, display `action`, current `allowed` boolean, safe `reason`, `prompt_gate_read_only=true`, `blocked_exit_code=2`, `standard_gate_command`, `standard_required_context_tokens`, and `sends_prompt_after_gate`. Evolution-loop and model-pool rows point to standard gates with `-MinContextTokens 262144`. The model-pool row has `sends_prompt_after_gate=false` because it is a launch gate, not a prompt itself.

`chain-status -JsonStatus` and `status-bundle -JsonStatus` also include a compact `machine_summary` and normalized `allowed_action_table`. The summary is always a status object: `read_only=true`, `sends_prompt=false`, and `launches_process=false`. Use it when automation only needs the current classification, `require_action_allowed`, `any_prompt_allowed`, endpoint booleans, and `model_pool_launch_allowed`. Use `allowed_action_table[]` when automation needs per-action details; each row keeps the stable `id`, display `action`, current `allowed` boolean, safe `reason`, `category` (`read_only`, `prompt`, or `launch`), and whether the downstream action would send a prompt or launch a process after its read-only gate passes.

The versioned contract applies to `prompt-gate -JsonStatus`, `chain-status -JsonStatus`, `pool-status -JsonStatus`, `pool-route-plan -JsonStatus`, `status-bundle -JsonStatus`, `pool-plan -JsonPlan`, and `pool-manifest -JsonStatus`. The `machine_summary` and nested `contract` objects repeat the same version fields so downstream code can validate either the top-level object or the compact summary it consumes.

When prompts are blocked, `status-bundle.safe_next_command_actions[]` must stay a subset of `prompt_policy.allowed_read_only_actions[]`. That means a wrapper may run the listed status commands, but it must not run `smoke`, Web Lab chat, Forge chat, direct backend prompts, evolution-loop prompt rounds, or model-pool launch until the matching `entrypoint_gates[]` row reports `allowed=true`.

For wrappers, `status-bundle -JsonStatus` also exposes `wrapper_audit` at top level and under `prompt_policy.wrapper_audit`. Use it as a fail-closed summary:

- `contract_supported=false`: stop and refresh the wrapper; do not send prompts.
- `safe_next_read_only_subset_required=true` plus `safe_next_actions_subset_of_allowed_read_only=false`: stop; the blocked-state bundle is internally unsafe.
- `fail_closed_required=true` with `wrapper_decision=read_only_only`: run only the listed read-only status commands.
- `wrapper_decision=action_gate_required`: a prompt or launch may still require its exact `chain-status -RequireAction ... -JsonStatus -FailIfBlocked` gate before the downstream action.

`entrypoint-matrix -JsonStatus` is the smallest wrapper-facing matrix command for Web Lab, Forge CLI, direct backend CLI, evolution-loop, smoke, and model-pool launch. `chain-status -JsonStatus` embeds the same rows under `integration_entrypoints[]`; `status-bundle -JsonStatus` repeats them at top level and under `prompt_policy.integration_entrypoints[]` while adding loop, pool, recovery, and wrapper-audit context. Each row includes `id`, `surface`, `consumer`, `entrypoint_kind`, `current_allowed`, `blocked_by`, `gate_command`, `required_context_tokens`, `downstream_sends_prompt`, `downstream_launches_process`, `wrapper_decision`, and `safe_when_blocked`. A wrapper may display these rows directly, but it still must run the row's `gate_command` before any prompt or launch.

`contract-audit -JsonStatus` is a compact verifier for wrapper integration.
It checks the contract version, read-only flags, safe-next subset rule, six
entrypoint gates, `model_pool_launch` as a non-prompt process gate, the rule
that quality-worker down blocks pool launch, and the post-recovery release
sequence. `audit_passed=true` means the contract is internally consistent; it
does not override `require_action_allowed=false` or `prompt_ready=false`.

`wrapper-manifest -JsonStatus` is the companion field-level manifest. It
repeats the current classification and audit summary, then lists six
`entrypoints[]` rows. Each row includes `status_command`, `gate_command`,
`consume_fields`, `proceed_only_if`, and `fail_closed_if`. Wrappers should
read it as integration metadata and still run the exact `gate_command`
immediately before any Web Lab, Forge, backend direct, evolution-loop, smoke,
or model-pool downstream action.

`contract-fixture -JsonStatus` is the offline companion for wrapper tests. It
sets `offline_fixture=true`, `touches_network=false`, `read_only=true`,
`sends_prompt=false`, and `launches_process=false`, then embeds sample objects
with `classification=quality_worker_down`. Use it for parser fixtures and
schema examples only; use live `chain-status`, `entrypoint-matrix`,
`status-bundle`, or `wrapper-manifest` before interacting with the chain.

For human handoff, plain `status-bundle` prints the same policy in sections named `machine summary`, `prompt policy`, `entrypoint gates`, and `allowed action table`. If the operator does not need raw JSON, this is the preferred one-screen report:

```powershell
.\tools\gemma-chain\gemma-chain.cmd status-bundle
```

For a shorter main-window update, use:

```powershell
.\tools\gemma-chain\gemma-chain.cmd handoff-report
```

It keeps only the current state, recommendation, endpoint summary, blocked
prompt actions, safe next commands, and Apple Silicon pool decision.

In the current `8686` down state, expected human lines include `any_prompt_allowed=False`, `model_pool_launch_allowed=False`, `quality_worker_reachable=False`, `blocked_prompt_actions=smoke,...,model-pool launch`, and a `model_pool_launch category=launch allowed=False` action-table row.

`chain-status`, `pool-status`, `status-bundle`, `recovery-plan`, `prompt-gate`, and `loop-status` support:

- `-JsonStatus`: print a single JSON object for automation.
- `-FailIfBlocked`: exit `2` when prompts should remain blocked.
- `-RequireAction`: for `chain-status` and `recovery-plan`, require a specific prompt-producing action before returning success with `-FailIfBlocked`.
- `-WaitReady`: for `chain-status` and `recovery-plan`, poll read-only health checks until the required action becomes allowed or `-WaitTimeoutSec` expires.
- `-MinContextTokens`: require the backend health context window before allowing prompt-producing actions.

Use `-FailIfBlocked` only in scripts that expect a nonzero gate result. It is still read-only and does not submit a prompt. For `pool-status`, `-FailIfBlocked` means "model-pool launch remains blocked"; it still only reads health and TCP state.

Action-specific gates:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction forge_cli_prompt -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction evolution_loop_prompt_round -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd pool-status -JsonStatus -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd status-bundle -JsonStatus -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd contract-audit -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd wrapper-manifest -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd contract-fixture -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd handoff-report -JsonStatus
.\tools\gemma-chain\gemma-chain.cmd secret-scan -JsonStatus -FailIfBlocked
```

Supported action ids are `any_prompt`, `smoke`, `web_lab_prompt`, `forge_cli_prompt`, `backend_cli_direct_prompt`, `evolution_loop_prompt_round`, and `model_pool_launch`.

Read-only wait gate after a recovery owner says the quality worker is coming back:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction smoke -WaitReady -WaitTimeoutSec 120 -PollIntervalSec 5 -JsonStatus -FailIfBlocked
```

This polls only health endpoints and exits `2` if the action is still blocked at timeout. It does not submit a prompt.

For full-window evolution-loop or model-pool release, require the restored 12B context:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction evolution_loop_prompt_round -MinContextTokens 262144 -JsonStatus -FailIfBlocked
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction model_pool_launch -MinContextTokens 262144 -JsonStatus -FailIfBlocked
```

`chain-status` supports the same switches and uses these classifications:

| Classification | Meaning | Prompt policy |
| --- | --- | --- |
| `backend_down` | `127.0.0.1:7979` TCP or `/health` is unavailable | Block all prompts; coordinate with backend owner before starting another backend |
| `engine_busy` | Backend is online but currently serving an active request | Block all new prompts; wait for active request or evolution round |
| `quality_worker_down` | Backend is online but `8686` or readiness is not OK | Block all prompts; restore quality worker/tunnel first |
| `web_lab_down` | Model and backend are ready but `8789` is unavailable | Block Web Lab UI and Web Lab smoke; backend-only CLI/loop/pool decisions may still be allowed by the action table |
| `prompt_ready` | Backend, quality worker, readiness, safe-device gate, and Web Lab are OK | Run tiny smoke before manual or loop prompts |
| `prompt_blocked` | Catch-all for a failed gate not covered above | Keep prompts blocked and inspect `diagnose` |

The allowed action table always permits read-only commands such as `selftest`, `diagnose`, `chain-status`, `entrypoint-matrix`, `recovery-plan`, `prompt-gate`, `loop-status`, `pool-plan`, and `pool-manifest`. It blocks every prompt action for `backend_down`, `engine_busy`, `quality_worker_down`, and `prompt_blocked`. For `web_lab_down`, it keeps Web Lab and Web Lab smoke blocked while allowing backend-only actions such as Forge CLI, direct backend CLI, evolution-loop prompt rounds, and model-pool launch when the quality worker gate is otherwise ready.

Use this rule in manual testing too:

```powershell
.\tools\gemma-chain\gemma-chain.cmd health
```

If the backend is busy, or if the model runtime/readiness gate is not OK, wait and diagnose before using CLI or UI smoke tests.

## Evolution Loop Status

Use `loop-status` when deciding whether the model chain is safe for the self-evolution loop:

```powershell
.\tools\gemma-chain\gemma-chain.cmd loop-status
```

It is read-only. It does not run `cargo`, does not invoke evolution-loop, and does not call any prompt endpoint. It reports:

- current backend prompt gate.
- latest `target\evolution\*.jsonl` ledger file.
- latest daemon `*.out.log` and `*.err.log` timestamps.
- safe tails with prompt lines redacted.
- classification such as `quality_worker_gate_blocked`.

If the classification is `quality_worker_gate_blocked`, restore the `8686` quality worker or tunnel first. Then rerun:

```powershell
.\tools\gemma-chain\gemma-chain.cmd diagnose
.\tools\gemma-chain\gemma-chain.cmd prompt-gate
```

Only after `prompt_ready=True` should an operator decide whether to run `smoke` or resume prompt-producing loop work.

For unattended wrappers, put a read-only JSON gate before any command that could send a prompt:

```powershell
.\tools\gemma-chain\gemma-chain.cmd chain-status -RequireAction evolution_loop_prompt_round -JsonStatus -FailIfBlocked
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
```

The JSON includes `classification`, `require_action`, `require_action_allowed`, `prompt_gate.prompt_ready`, and per-entrypoint decisions. A `classification` of `quality_worker_down` means the quality worker/tunnel must be restored before smoke or evolution-loop prompt rounds. Use `loop-status -JsonStatus` next when you also need latest ledger/log timestamps.

In a pool-aware future backend, `engine_busy` should be tracked per worker and rolled up into backpressure:

- high-quality tasks wait for the `quality` worker rather than spawning another 12B Q8.
- summary/review/test-gate tasks can route to small idle workers.
- if all small workers are busy, low-priority analysis can be skipped, delayed, or retried.

## Metadata Timeout

During active inference, the model service or tunnel can respond slowly to metadata requests. Symptoms include backend health containing `gemma_runtime_metadata_error`, or logs with socket timeout errors while later retries succeed.

Recommended response:

1. Confirm `127.0.0.1:8686/health` is still OK.
2. Confirm backend `/health` still reports `gemma_runtime_reachable=true`.
3. Retry metadata after the active request ends.
4. Escalate only if metadata and generation both fail while the backend is idle.

`diagnose` treats unsupported `/metadata` responses as warnings when `/v1/models` and backend `/health` still expose context metadata. Timeout-like failures are recorded and retried later; diagnostics do not restart services.

## Stream Continuity And Truncation

Expected SSE stream shape:

```text
event: status
event: delta
event: delta
event: done
data: [DONE]
```

`event: error` is also terminal if followed by `done` from the proxy layer. A stream that ends after only `delta` events, or ends with a partial SSE frame, should be treated as truncated. Record:

- Whether backend `/health` was busy.
- Whether Web Lab returned `event: error`.
- Whether `event: done` appeared.
- The endpoint used: backend stream or Web Lab stream.

If Web Lab blocks a prompt before forwarding, the SSE tail should include the
status line `checking backend prompt gate before forwarding request`, then a
human-readable `event: error`, then `event: done`. Treat that as a clean gate
block, not as stream truncation.

Web Lab may still display the partial assistant text for diagnosis, but it must
not commit that assistant text into chat history/context unless the stream
reaches `done` without `error` or returns a valid `final` event.

Run:

```powershell
.\tools\gemma-chain\gemma-chain.cmd diagnose
```

Then retry a tiny smoke only after the backend is idle:

```powershell
.\tools\gemma-chain\gemma-chain.cmd smoke
```

The `selftest` action validates local redaction, SSE continuity, chain classification, and allowed-action fixtures without touching the model:

```powershell
.\tools\gemma-chain\gemma-chain.cmd selftest
```

## Secret Hygiene

The Gemma chain tools are diagnostics, not credential capture:

- Do not paste tokens, passwords, private keys, SSH commands with secrets, or model download credentials into prompts.
- Diagnostic JSON output redacts common credential field names and secret-like strings.
- Active request prompt previews are printed only through a safe preview path.
- Gemini-style cloud adapters, if added later, must keep credentials outside this repo and outside diagnostic output.

Before reporting or handing off the Gemma chain artifacts, run:

```powershell
.\tools\gemma-chain\gemma-chain.cmd secret-scan -JsonStatus -FailIfBlocked
```

If it exits `2`, fix or redact the matching source line before sharing the
handoff. The scan reports only safe previews; it is not a reason to copy a raw
secret into chat or logs.

## CLI And UI Testing

UI:

```text
http://127.0.0.1:8789/
```

CLI-style Web Lab SSE smoke:

```powershell
.\tools\gemma-chain\gemma-chain.cmd smoke
```

Pure health check:

```powershell
.\tools\gemma-chain\gemma-chain.cmd health
```

If evolution-loop is running a round, use `health` or `diagnose` only. Wait until `engine_busy=false` before sending prompts from CLI or UI.

## Troubleshooting Checklist

1. `8686` unreachable: check the local tunnel or remote llama-server owner. Do not restart shared services without coordination.
2. `7979` unreachable: backend is down or on a different port. Check the launch owner before starting another backend.
3. `8789` unreachable: Web Lab is down or on a different port. Health and CLI smoke can still use `7979`.
4. `engine_busy=true`: current inference is active. Wait.
5. Metadata timeout: retry after active inference; treat as warning if generation still works.
6. Stream truncation: capture the SSE tail and backend health, then retry a tiny smoke when idle.
7. Context/max token mismatch: trust backend health projection first; avoid large tests until the model service metadata is stable.
