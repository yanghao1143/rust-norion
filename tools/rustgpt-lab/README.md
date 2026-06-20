# rustgpt-lab

`rustgpt-lab` is an isolated frontend and streaming proxy for manually testing
the local rust-norion model service. It is inspired by RustGPT's product shape
but does not copy RustGPT source code.

## Two Safe Run Paths

### A. Built-in backend streaming test, no Gemma

Use this path to verify the split Web Lab frontend/proxy, SSE behavior,
`/health`, and rust-norion stream routes before loading Gemma 12B. It does not
start `mistralrs` or Gemma. By default it keeps state under
`target\manual-web-lab-service\built-in-lab-state`, away from the project-root
`noiron-*.ndkv` files.

Recommended one-command start:

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\start-built-in-lab.cmd
```

Read-only startup check:

```powershell
.\tools\rustgpt-lab\start-built-in-lab.cmd -CheckOnly
```

Open:

```text
http://127.0.0.1:8787/
```

This path writes only the isolated
`target\manual-web-lab-service\built-in-lab-state\*.ndkv` files after you send
prompts. It does not start Gemma and does not modify the project-root
experience store. For read-only status checks:

```powershell
.\tools\rustgpt-lab\status-built-in-lab.cmd
```

Stop the built-in Web Lab safely:

```powershell
.\tools\rustgpt-lab\stop-built-in-lab.cmd -DryRun
.\tools\rustgpt-lab\stop-built-in-lab.cmd
```

### Attach only the Web Lab UI to an existing backend

Use this when `rust-norion` is already listening and you only want the browser
test UI/proxy. This starts only `rustgpt-lab`; it does not start Gemma,
`mistralrs`, or a new `rust-norion` process:

```powershell
cd D:\rust-norion
cargo run --manifest-path tools\rustgpt-lab\Cargo.toml -- `
  --backend 127.0.0.1:7878 `
  --bind 127.0.0.1:8787
```

Then open:

```text
http://127.0.0.1:8787/
```

Port map for debugging connection errors:

- `127.0.0.1:7878` is the `rust-norion` model-service backend that Web Lab
  forwards to. If this refuses connections, Web Lab is up but the backend is not.
- `127.0.0.1:8686` is the optional Gemma/mistralrs runtime behind `rust-norion`.
  Do not send prompts directly there from Web Lab.
- `127.0.0.1:8787` is the `rustgpt-lab` browser UI and local proxy.

Read-only checks when a port refuses connections:

```powershell
.\tools\rustgpt-lab\status-built-in-lab.cmd
.\tools\rustgpt-lab\status-gemma-lab.cmd
```

Copy/paste no-start chain for an already-running backend:

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\status-gemma-lab.cmd
cargo run --manifest-path tools\rustgpt-lab\Cargo.toml -- `
  --backend 127.0.0.1:7878 `
  --bind 127.0.0.1:8787
```

Then use the browser at `http://127.0.0.1:8787/`, or attach the CLI to that
already-running Web Lab:

```powershell
.\tools\rustgpt-lab\chat-gemma-lab.cmd -Lab http://127.0.0.1:8787 -Prompt "hello" -ShowMeta
.\tools\rustgpt-lab\repl-gemma-lab.cmd -SkipStart -BackendPort 7878
```

The `chat-gemma-lab.cmd` command connects to the Web Lab proxy on `8787`; the
REPL `-SkipStart` path connects directly to the existing `rust-norion` backend
on `7878`. Neither command starts Gemma or `mistralrs`. Port `8686` is only the
optional Gemma runtime that `rust-norion` may call behind the backend.

### B. Real Gemma: CheckOnly first, then start

Before a real Gemma 12B run, use the read-only CheckOnly gate. It reports
configuration, ports, StateDir, RAM/VRAM, backend health, and experience-store
safety. It does not start Gemma, start Web Lab, or write `.ndkv`.

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\start-gemma-lab.cmd `
  -CheckOnly `
  -StateDir target\manual-gemma-service\lab-state `
  -Snapshot "D:\hf-cache\hub\models--google--gemma-4-12B-it\snapshots\5926caa4ec0cac5cbfadaf4077420520de1d5205"
```

If CheckOnly reports a dirty project experience store, use isolated state first,
or clean/apply only after explicit authorization. Do not point real Gemma tests
at the project-root `noiron-experience.ndkv` by accident.

For the lower-level model-service smoke gate, use `cargo run -- --gemma-model-service-smoke --gemma-smoke-check-only`.

After CheckOnly passes, start the real Gemma/Web Lab stack:

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\start-gemma-lab.cmd `
  -StateDir target\manual-gemma-service\lab-state
```

`-StateDir` keeps `memory.ndkv`, `experience.ndkv`, and `adaptive.ndkv` in an
isolated directory. Use `-UseProjectState` only after explicitly deciding to use
the project experience store.

## Run

Gemma 12B testing requires a resident `mistralrs serve` process. The resident
process runs on your machine and does not depend on this Codex chat staying
open. After the script succeeds, you can test from the browser. That process
uses several GB of RAM/VRAM, so treat it as test mode and stop it when idle.

Recommended one-command start:

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\start-gemma-lab.cmd
```

The start script runs a resource preflight before loading Gemma 12B. The default
thresholds are 18 GB free system RAM and 13 GB free NVIDIA VRAM. To override the
preflight intentionally:

```powershell
.\tools\rustgpt-lab\start-gemma-lab.cmd -Force
```

This starts:

- `mistralrs serve` on `127.0.0.1:8686`;
- `rust-norion` on `127.0.0.1:7878`;
- `rustgpt-lab` on `127.0.0.1:8787`.

The processes keep running after the script exits. The script prints each PID
and writes logs under:

```text
D:\rust-norion\target\manual-gemma-service
```

Stop the full test stack and release Gemma 12B RAM/VRAM:

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\stop-gemma-lab.cmd -DryRun
.\tools\rustgpt-lab\stop-gemma-lab.cmd
```

Stop only the lab/backend and keep the resident Gemma runtime:

```powershell
.\tools\rustgpt-lab\stop-gemma-lab.cmd -KeepMistral
```

The default stop path only stops confirmed local test-stack processes. The
backend must answer `/health` as `rust-norion`, the Web Lab must point at that
backend, and the Gemma runtime port must be owned by `mistralrs`. Use
`-ForceAll` only when every matching local process is disposable.

Open:

```text
http://127.0.0.1:8787/
```

Check the current stack without starting anything:

```powershell
.\tools\rustgpt-lab\status-gemma-lab.cmd
```

Send one CLI chat request without opening the browser:

```powershell
.\tools\rustgpt-lab\chat-gemma-lab.cmd -Prompt "Explain in Chinese how rust-norion is wired to Gemma."
```

This script does not start services. It connects to the resident `rustgpt-lab`
proxy and fails early if the backend or Gemma runtime is not ready. To print
status/meta/raw/enhanced SSE events as well:

```powershell
.\tools\rustgpt-lab\chat-gemma-lab.cmd -Prompt "hello" -Output raw -Profile coding -ShowMeta
```

Use `-Output enhanced` when you want the Noiron-enhanced answer stream instead
of the raw runtime answer; `-Profile` is forwarded with the request payload.

Run the full business-cycle stream from the CLI:

```powershell
.\tools\rustgpt-lab\chat-gemma-lab.cmd `
  -Endpoint business-cycle `
  -Prompt "Explain in Chinese one rust-norion business integration check." `
  -FeedbackAmount 0.75 `
  -NoSelfImprove `
  -RustCheckCode "pub fn ok() -> bool { true }" `
  -ShowMeta
```

The CLI prints `stage`, `meta`, generation deltas, and a final
`business_cycle passed=...` summary. The full final JSON is printed only with
`-ShowMeta`. Use `-Output`, `-Profile`, `-FeedbackAmount`, `-NoSelfImprove`,
and `-RustCheckCode` to exercise the same business-cycle payload fields covered
by the offline safety suite.

`chat-gemma-lab.cmd -TimeoutSeconds 1800` raises the PowerShell SSE client's
total stream wait window. For slow 12B runs, keep this at least as large as the
Web Lab backend window configured with `--backend-timeout-secs` or
`-LabBackendTimeoutSeconds`. The script treats EOF before SSE `done` and
incomplete SSE frames as truncated streams, and exits nonzero after SSE
`error`, instead of silently accepting a partial or rejected answer.

Run `.\tools\rustgpt-lab\test-gemma-lab-safety.cmd` to validate the
PowerShell SSE client plus start/status/stop wrapper help, CheckOnly, and
DryRun safety against local fake or random-port endpoints without starting
Gemma. The fake stream cases cover heartbeats, comment-only keep-alive frames,
CR-only frame separators, multiline data fields, empty event fields, no-colon
SSE fields, field value spacing, errors with or without a trailing `done`, HTTP
stream setup failures, EOF before `done` including after `final`,
incomplete-frame truncation, CLI business-cycle endpoint/output/profile/Rust
check/self-improve/feedback payload fields, idle body timeouts, and pre-header
timeouts. It
also runs a `web/app.js` syntax check, extracts the
Web UI SSE parser, and checks the same core frame parsing edges plus
incomplete-frame retention. The Node.js checks also load the real Web UI script
with an offline DOM/fetch harness to cover Enter-to-send, IME/repeat/modified
Enter safety, Shift+Enter newline, send-button state transitions,
input disable/recover state, draft restore after rejected sends,
context-window 2..256 clamping, heartbeat/status progress visibility,
auto-scroll during streamed output, clear-context reset including mid-stream
clears, and business-cycle Rust check payloads. The same Web harness covers
user cancel, HTTP stream setup failures,
busy/readiness/safe-device/experience preflight gate blocks, and SSE
`error`/truncated streams, verifying that the composer recovers, rejected
low-window sends do not trim completed history, and rejected partial turns do
not enter browser conversation context; those Web checks require Node.js on
`PATH`. The PowerShell client checks also prove `chat-gemma-lab.cmd` exits
before `/api/chat-stream` when Web Lab is unreachable, or when backend busy,
readiness, safe-device, experience hygiene/index, or Gemma runtime preflight
fails; the Gemma-runtime case points to read-only status and
`start-gemma-lab.cmd -CheckOnly` before any real start. They also prove
`repl-gemma-lab.cmd -SkipStart` stays attach-only when
the backend port is missing: it exits before any Gemma/start/REPL path and
prints the `7878` backend, `8787` Web Lab, and `8686` runtime distinction.
The same safety run locks the built-in start/status/stop help text to the
`7878`/`8787`/`8686` port map, including that `8686` is not used, queried, or
targeted by the built-in paths. It also locks the real Gemma start/status/stop
help text to the same port map, including that `8686` is the optional runtime
behind `rust-norion`, not a direct prompt target. The older
`test-chat-gemma-lab-client.cmd` entrypoint remains as a compatibility alias.
Use `.\tools\rustgpt-lab\test-gemma-lab-safety.cmd -Help` to print the
offline coverage checklist without running the tests.

You can also run the Rust-native interactive REPL directly:

```powershell
cd D:\rust-norion\tools\rustgpt-lab
cargo run -- --help
cargo run -- --repl --backend 127.0.0.1:7878
```

The `--help` path prints CLI options and exits without starting the Web Lab,
backend, Gemma, or any prompt stream. It wins even when combined with startup
flags such as `--repl`, so it is safe as a first read-only command.

Or start/validate the full Gemma lab stack and enter the REPL with one command:

```powershell
cd D:\rust-norion
.\tools\rustgpt-lab\repl-gemma-lab.cmd
```

If the stack is already running and you only want to attach the CLI:

```powershell
.\tools\rustgpt-lab\repl-gemma-lab.cmd -SkipStart
```

For longer multi-turn CLI testing, set the short context window at startup.
The default is 64 messages and the range is 2..256:

```powershell
cargo run -- --repl --backend 127.0.0.1:7878 --context-messages 128
```

Plain input sends a prompt. Use `/help` for commands such as:

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

## Interfaces

- `GET /` serves the test UI.
- `GET /health` returns proxy status.
- `GET /api/model-pool-status` proxies rust-norion model-pool status read-only.
- `GET /api/model-pool-advice` returns read-only Apple model-pool expansion advice.
- `POST /api/chat-stream` accepts:

```json
{
  "prompt": "你好，用中文回答。",
  "profile": "coding",
  "output": "raw",
  "endpoint": "chat"
}
```

The response is `text/event-stream`. The proxy emits status and heartbeat events
immediately, then prefers `http://127.0.0.1:7878/v1/chat-stream`,
`/v1/generate-stream`, or `/v1/business-cycle-stream` so backend deltas reach
the browser as rust-norion receives them. If only the synchronous backend route
is available, the proxy can still split the final `answer` into small delta
events as a compatibility fallback.

In the Web UI, `status` and `heartbeat` update one progress row so slow
first-token waits stay visible without flooding the transcript. `stage` and
`meta` events remain separate rows because they are part of the business-cycle
audit trail.

The offline safety suite exercises the browser script's input path as well:
plain Enter requests submit, IME/repeat/modified Enter does not submit,
Shift+Enter stays in the textarea for a newline, the send button and composer
inputs disable while the stream is in flight and recover afterward,
heartbeat/status events remain visible in the progress row, and auto-scroll
lands at the bottom while streamed deltas render. It also checks that clearing
context removes completed browser conversation from the next request, and that
business-cycle mode sends `endpoint=business-cycle` with the Rust check code.
The same offline harness also checks submit preflight blocks for backend busy,
readiness, safe-device, and experience hygiene/index gates: the draft prompt is
kept, `/api/chat-stream` is not called, and browser conversation is unchanged.
It also checks
that user cancel, SSE `error`, EOF-before-`done`, and final-without-`done`
streams restore the draft prompt and mark the assistant bubble interrupted,
that HTTP stream setup failures recover the composer, and that rejected partial
turns do not commit user/assistant messages to browser conversation context or
trim previously completed browser history.

The browser's temporary `conversation` state is driven only by complete Web SSE
turns. JSONL files under the lab state directories are trace artifacts, not a
source of browser chat history.

If you clear context while a stream is still running, the browser keeps the
visible partial/complete answer but does not write that turn or the old request
history back into the temporary conversation after `done`.

For `/v1/chat`, the browser keeps 64 temporary conversation messages by default,
with the page control clamped to 2..256. Each send uses that value as a request
message limit: up to `limit - 1` previous conversation messages plus the new
user prompt. A value such as 128 in the context controls means 128 short chat
message slots, not 128 model tokens. The page also sends
`max_tokens=262144` by default as a large generation-budget request parameter;
lower it when you want faster short checks.

The Rust REPL uses the same short-context policy: 64 messages by default,
configurable with `--context-messages 128` at startup or `/context-window 128`
while running, capped at 256. This controls chat history, while `max_tokens`
controls the generation budget; a 128-message context window is not a 128-token
model limit. Startup aliases `--context-window` and
`--max-context-messages` use the same 2..256 clamp.

The proxy gives each backend streaming request a 900-second total window by
default and uses short read polling to keep SSE heartbeats flowing while Gemma is
slow to produce the first token. Increase the total window with
`--backend-timeout-secs` (alias `--timeout-secs`) or the start scripts' lab
timeout flags for especially long 12B runs. This is a total streaming window,
not a per-read socket timeout: headers, body deltas, and streamed error bodies
all share the same deadline while short read polls keep browser heartbeats
alive.

The UI also polls `GET /api/backend-health`, which proxies rust-norion
`/health` so the header can show busy state, active requests, processed request
count, `experience_hygiene.experience_file`, quarantine/repairable experience
debt, index risk, readiness/safe-device status, and the latest inference summary.
Built-in safe backends can be used for split frontend/SSE testing. The send
button is disabled with a preflight failure when a configured Gemma HTTP runtime
is unreachable, readiness/safe-device fails, `quarantine_candidates` is nonzero,
`repairable_legacy_metadata_lessons` or `repairable_index_records` is nonzero,
the experience index is not retrieval-ready or reports `risk_level=blocked`, or
`experience_hygiene.clean=false`. Use `status-gemma-lab.cmd`,
`status-built-in-lab.cmd`, REPL `/status`, `/hygiene dry-run`, `/repair dry-run`,
and `/audit` to inspect the debt before sending real prompts.
The REPL `/status` command prints the same prompt-gate summary plus
`experience_hygiene` and `experience_index` fields, so terminal-only checks can
see why a prompt is blocked.

The UI, REPL `/pool-advice`, and status scripts also read
`GET /api/model-pool-advice`. That endpoint never launches workers or sends
prompts; it turns model-pool status into `safe_to_enable_pool_workers`,
`next_step`, `reason`, and a short recommendation such as fixing quality 12B,
fixing Metal/GPU fallback, or adding the summary/review/index helper first.
It also surfaces `expected_helper_roles`, `missing_helper_roles`, and
`recommended_launch_order` so the UI and REPL can show the one-quality plus
small-helper target and the quality -> summary -> review -> index -> test-gate
launch order.

Set `"endpoint": "business-cycle"` to call `/v1/business-cycle` instead. The
lab forwards `feedback_amount`, `self_improve`, and optional `rust_check_code`,
then surfaces business-cycle pass/fail fields as SSE `meta` events.

## Streaming

For `/v1/chat` and `/v1/generate`, the lab now prefers rust-norion's
`/v1/chat-stream` and `/v1/generate-stream` routes. The main Gemma HTTP runtime
requests mistralrs/OpenAI `stream:true`, parses SSE deltas into
`RuntimeResponse.tokens`, and forwards those deltas through service-level SSE to
the browser. After inference completes, rust-norion sends a `final` event with
the complete JSON response, including Noiron reflection, memory writes, runtime
token counts, and related telemetry.

`/v1/business-cycle` now prefers `/v1/business-cycle-stream`: the generation
phase forwards Gemma deltas, feedback/self-improvement/Rust-check/state-save/gate
work emits `stage` and `meta` events, and the final event carries the complete
business-cycle JSON.

## Source Layout

The lab is split by responsibility so test-only coupling does not grow into the
main service:

- `app.rs`: routes and request lifecycle;
- `backend.rs`: proxy calls to `rust-norion:7878`;
- `backend/stream.rs`: backend SSE parsing, terminal-event handling, and heartbeat polling;
- `backend/io.rs`: backend TCP connect/read/write timeout setup;
- `repl.rs`: terminal REPL loop and command handling;
- `request.rs`: UI request parsing;
- `sse.rs`: SSE headers and events;
- `http.rs`: minimal HTTP helpers;
- `chunk.rs`: answer chunking;
- `json.rs`: dependency-free JSON helpers;
- `config.rs`: CLI flags, context-window defaults, and timeout defaults;
- `status.rs`: long Gemma inference status and backend error hints.
