# Parallel Workstreams

This file is the main-window coordination record for the six parallel workstreams.
It captures the current repository baseline, write boundaries, and integration gates
so each window can move independently without overwriting another module.

## Current Baseline

- Branch: `codex-runtime-device-abi`.
- Root crate: `rust-norion` is now the root package and workspace root.
- Existing implementation is concentrated under `src/**`.
- Modular crates are under `crates/**` and are wired into the root workspace.
- Stable loop baseline: `tools/evolution-loop` currently has 67 passing tests.
- Runtime loop baseline: Gemma 12B has completed at least fifteen self-evolution rounds
  against `target/evolution/runtime-model-gated-loop-20260613-175925.jsonl`.
- Runtime ports: Gemma on `127.0.0.1:8686`, backend on `127.0.0.1:7979`,
  Web Lab usually on `127.0.0.1:8789`.
- Current runtime gate: backend and Web Lab are reachable, but the quality
  worker on `127.0.0.1:8686` is down; all prompt-producing entrypoints and
  model-pool launch must remain blocked until `prompt-gate` is ready.
- Do not stop or restart the active model/backend while an evolution round is busy.
- Independent tools under `tools/evolution-loop`, `tools/rustgpt-lab`, and
  `tools/smartsteam-forge` keep their own workspace boundaries.

## Ownership

| Window | Owner | Write Scope | Existing Read-Only Sources |
| --- | --- | --- | --- |
| 1 | Kuhn | `crates/norion-core/**`, `docs/architecture/norion-core*.md` | `src/engine*`, `src/runtime*`, `src/transformer*`, `src/router*`, `src/hierarchy*`, `src/kv_cache*`, `src/kv_quant*`, `src/tiered_cache*`, `src/local_runtime*`, `src/production_runtime*`, `src/hardware*` |
| 2 | Kant | `crates/norion-memory/**`, `docs/architecture/norion-memory*.md` | `src/disk_kv*`, `src/experience*`, `src/gist_memory*`, `src/infini_memory*`, `src/kv_cache*`, `src/tiered_cache*`, `src/state_inspect*`, `src/adaptive_state*` |
| 3 | Franklin | `crates/norion-agent/**`, `docs/architecture/norion-agent*.md` | `src/agent_team*`, `src/reflection*`, `src/process_reward*`, `src/toolsmith*`, `src/recursive_scheduler*`, `src/drift*` |
| 4 | Euler | `crates/norion-service/**`, `crates/norion-cli/**`, `docs/architecture/norion-service*.md`, small scoped fixes in `tools/smartsteam-forge/**` | `src/model_service*`, `src/gemma_business*`, `src/cli*`, `src/token_stream*`, `src/runtime*`, `tools/smartsteam-forge/**` |
| 5 | Gibbs | `crates/norion-test/**`, `crates/norion-eval/**`, `docs/architecture/norion-eval*.md`, `docs/runbooks/evolution-loop*.md` | `tools/evolution-loop/**`, `src/benchmark*`, `src/rust_validation*`, `src/trace*`, `src/drift*`, `target/evolution/*.jsonl` |
| 6 | Carson | `tools/gemma-chain/**`, `docs/runbooks/gemma*.md`, `docs/architecture/integration*.md` | `tools/smartsteam-forge/**`, `tools/evolution-loop/**`, runtime health endpoints, daemon logs |

## Active Long-Running Threads

These are user-visible Codex threads, not one-shot helper agents. Each thread has
its own persistent goal and should keep moving through successive slices until the
main window asks it to stop or hand off.

| Window | Thread | Long-Running Focus | Current Status |
| --- | --- | --- | --- |
| 1 | `019ec081-bebd-7de3-9681-0f6b079c7aa6` | `norion-core` FHT-DKE, routing, attention, KV fusion, adapter contracts | active, runtime diagnostics, hardware plans, quantization, and core adapter-facing contracts verified in workspace |
| 2 | `019ec081-e127-78a3-8965-40ffda5fdfad` | `norion-memory` Agentic Memory, KVSwap, Context Rot, migration contracts | active, tiered placement, gist selection, replay planning, adapter projection, and KVSwap intent verified in workspace |
| 3 | `019ec082-0b09-7400-8bd4-34d664e1aad0` | `norion-agent` run ledger, budget policy, conflict gates, reflection workflow | active, cycle orchestration, resolved conflicts, and budget audit verified in workspace |
| 4 | `019ec082-2a87-7d32-8a74-a25186e83a1e` | `norion-service`/CLI and SmartSteam/Lab interaction safety | active, model routing, gated Enter, submit-prompt history, SSE parsing, and stream pressure states verified in workspace |
| 5 | `019ec082-4849-7a72-b42a-4696c9392209` | `norion-test`/`norion-eval` gate and rollback abstraction | active, model-pool eval gates and outage attribution verified in workspace |
| 6 | `019ec082-6bf8-7b41-8250-bbd76af9f8a6` | Gemma chain diagnosis, smoke safety, runbooks | active, chain-status, prompt-gate, and model-pool blocked readiness verified by read-only commands |

## Main Window Responsibilities

- Keep the active goal intact and coordinate results from all windows.
- Avoid editing child-owned code while child windows are running.
- Wire completed crates into the root workspace after their independent tests pass.
- Run formatting and tests in increasing scope:
  - `cargo test --manifest-path crates/<crate>/Cargo.toml`
  - `cargo test --manifest-path tools/evolution-loop/Cargo.toml`
  - root `cargo test` after workspace wiring
- Re-run strict evolution report gates before claiming the loop is healthy.
- Keep the six long-running threads aligned with this file and collect their
  completed slices before integrating cross-module changes.

## Integration Rules

- Child crates should compile independently before root workspace integration.
- Public APIs between modules should use traits and small data structs.
- Cross-crate dependencies should flow in this direction:
  `norion-core -> norion-memory -> norion-agent -> norion-service/cli -> norion-test/eval`.
- Runtime and Gemma scripts remain outside the core crates.
- Root-to-crate adapters must start in shadow/report-only mode; the current root
  `gemma_business::eval_adapter` only projects health/final JSON into
  `norion-eval` outage evidence and does not change runner behavior.
- Real `.ndkv` state files are read-only unless a dedicated migration/repair task is assigned.
- No window should rewrite history, reset the branch, or revert unknown changes.

## Apple Silicon Model Pool Strategy

The shared Mac/Gemma chain should optimize for throughput by role routing, not by
blindly starting many copies of the same 12B Q8 model.

Default stance:

- Keep one high-quality 12B worker for architecture decisions, final synthesis,
  hard code generation, and high-risk review.
- Add two to four smaller or lower-quantized workers for cheap parallel work:
  summary, review, test-gate, and index.
- Do not run multiple 12B Q8 workers by default. They compete for unified memory,
  GPU/Metal scheduling, KV cache, and disk bandwidth; this can make every request
  slower and less predictable.
- Route requests by role and expected value. Fast workers can produce candidates,
  summaries, and test ideas; the 12B worker should adjudicate or generate only
  when the smaller workers' output passes eval gates.

Suggested local model API ports:

| Port | Worker Role | Intended Model Class | Default Use |
| --- | --- | --- | --- |
| `8686` | `high_quality` | Gemma 12B Q8/Q6 | final answer, architecture, complex code |
| `8687` | `summary` | small fast model | log/session/ledger summary and prompt compression |
| `8688` | `review` | small/medium low-quant model | code review, risk scan, contradiction check |
| `8689` | `test-gate` | small fast model | test suggestions, command plans, failure triage |
| `8690` | `index` | small/medium model | repository maps, retrieval prefilters, and context hints |

Backpressure rules:

- If `high_quality` is busy, queue or route only low-risk summary/review tasks to
  small workers. Do not send another expensive 12B request just because a UI is idle.
- If small workers produce conflicting results, route the conflict summary to 12B
  instead of merging blindly.
- Eval must track each worker separately: latency, runtime tokens, success rate,
  validation pass rate, duplicate/noisy output, and whether it blocked the 12B lane.
- UI/CLI should expose role preference such as `prefer_fast`, `prefer_quality`,
  or an explicit worker role, while preserving `max_tokens` and history limits.

## Done Criteria For This Parallel Slice

- Each child window reports changed files and an independent test command.
- New crates are small, modular, and do not duplicate large root modules blindly.
- Main window integrates only crates that have passing local tests.
- Existing `tools/evolution-loop` tests remain green.
- The active Gemma chain is either prompt-ready for CLI/UI/evolution-loop testing
  or explicitly blocked by `gemma-chain prompt-gate` with no prompt submitted.

## Latest Main-Window Validation

Last verified by the main window on 2026-06-13:

- `cargo test --workspace` passed.
- Workspace crate counts included: `norion-agent` 23, `norion-cli` 15,
  `norion-core` 49, `norion-eval` 34, `norion-memory` 44,
  `norion-service` 35, `norion-test` 10, and root `rust-norion` 225.
- `cargo test gemma_business::eval_adapter` passed with 5 root adapter tests.
- `cargo test --manifest-path crates/norion-service/Cargo.toml` passed with 35 tests.
- `cargo test --manifest-path crates/norion-cli/Cargo.toml` passed with 15 tests.
- `cargo test --manifest-path crates/norion-test/Cargo.toml` passed with 10 tests.
- `cargo test --manifest-path crates/norion-eval/Cargo.toml` passed with 34 tests.
- `cargo test --manifest-path tools/evolution-loop/Cargo.toml` passed with 67 tests.
- `.\tools\gemma-chain\gemma-chain.cmd selftest` passed.
- `.\tools\gemma-chain\gemma-chain.cmd prompt-gate -JsonStatus` passed and
  blocked prompt entrypoints because the quality worker is unavailable.
- `.\tools\gemma-chain\gemma-chain.cmd chain-status -JsonStatus` passed with
  `classification=quality_worker_down` and `model-pool launch allowed=false`.
- `.\tools\gemma-chain\gemma-chain.cmd pool-plan -JsonPlan` printed the Apple
  Silicon model-pool plan without starting workers.
- Backend health was reachable on `127.0.0.1:7979` and `engine_busy=false`.
- Gemma runtime `127.0.0.1:8686` was not reachable, so `readiness_ok=false`.
  Manual smoke, Web Lab prompts, Forge prompts, and CLI prompts should stay
  blocked until the quality worker is reachable again.
