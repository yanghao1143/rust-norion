# SmartSteam Forge Architecture

SmartSteam Forge is the long-lived Rust TUI product layer for local Gemma and
rust-norion development. It should feel closer to Claude Code or Gemini CLI than
to the temporary browser lab, while keeping the implementation clean-room and
license-safe.

## Current Repository Boundary

`tools/rustgpt-lab` already exists and should stay as the short-lived Web/REPL
test harness. It is useful for validating `rust-norion:7878` and Gemma 12B over
HTTP/SSE, but it is intentionally small and zero-dependency:

- `app.rs`: tiny HTTP server and request lifecycle.
- `backend.rs`: direct TCP proxy to `rust-norion`.
- `request.rs`: chat/generate/business-cycle request parsing.
- `sse.rs`: SSE response helpers.
- `repl.rs`: basic line REPL with `/mode`, `/status`, `/output`, `/rust`, and
  limited in-memory chat history.

SmartSteam Forge should be separate from `tools/rustgpt-lab` and should not
move UI, session, or tool orchestration into root `src/`.

## External Base

Use `fortunto2/rust-code` as the architecture base because it is Rust-native,
MIT-licensed, and aligned with a local coding-agent workflow. The parts worth
borrowing as concepts are:

- Ratatui/Crossterm terminal shell with a persistent event loop.
- Session history as a first-class subsystem instead of ad hoc vectors.
- Tool execution boundaries for shell, file search, file read, and patch/diff.
- Provider abstraction so the UI can talk to local Gemma, rust-norion, or future
  OpenAI-compatible endpoints through one stream interface.
- Agent/MCP hooks as optional extension points, not as the core chat path.

Do not vendor the upstream project wholesale into this repo. Recreate the
minimal slices we need and keep module names local to SmartSteam Forge.

## Clean-Room UX References

`Kuberwastaken/claurst` is GPL-3.0, so its source code must not be copied,
translated, or mechanically ported. Treat it only as a product/UX reference.
Allowed clean-room ideas:

- Slash command palette for session, model, provider, memory, and tool actions.
- Diff and permission review surfaces before file writes or shell execution.
- Multi-provider switching and runtime status indicators.
- Memory controls for project notes, session summaries, and pinned context.
- ACP/MCP-style agent hooks for future multi-agent collaboration.
- Claude/Gemini-like streaming transcript with visible tool calls and progress.

Forbidden:

- Copying source files, function bodies, command parsers, UI layouts, or tests.
- Translating GPL code line-by-line into this project.
- Reusing GPL assets or embedded prompts.

## MVP Module Tree

Start with a new standalone Rust crate under this directory:

```text
tools/smartsteam-forge/
  Cargo.toml
  src/
    main.rs
    app.rs
    config.rs
    ui/
      mod.rs
      layout.rs
      theme.rs
      input.rs
      transcript.rs
      status_bar.rs
      command_palette.rs
      permission_modal.rs
      diff_view.rs
    event/
      mod.rs
      loop.rs
      keymap.rs
      tick.rs
    session/
      mod.rs
      model.rs
      history.rs
      store.rs
      summary.rs
    provider/
      mod.rs
      stream.rs
      rust_norion.rs
      openai_compat.rs
      health.rs
    tools/
      mod.rs
      registry.rs
      shell.rs
      files.rs
      search.rs
      patch.rs
      permissions.rs
    memory/
      mod.rs
      project_notes.rs
      pinned.rs
      recall.rs
    agent/
      mod.rs
      task.rs
      events.rs
      mcp.rs
    state/
      mod.rs
      paths.rs
      jsonl.rs
    diagnostics/
      mod.rs
      logs.rs
      metrics.rs
```

The first implementation should wire only:

1. `ui` transcript, input box, status bar, and auto-scroll.
2. `provider::rust_norion` to `http://127.0.0.1:7878/v1/chat-stream`.
3. `session::history` with recent context and JSONL transcript persistence.
4. `/status`, `/new`, `/clear`, `/mode`, `/output`, `/help`, and `/quit`.
5. A read-only tool registry stub so future shell/file tools have a place to
   live without being bolted into the chat loop.

## Runtime Flow

```text
keyboard input
  -> event loop
  -> command router or prompt submit
  -> session context builder
  -> provider stream request
  -> stream events
  -> transcript model
  -> Ratatui render
  -> session store
```

All provider events should normalize into one internal enum:

```text
AssistantEvent::Status(text)
AssistantEvent::Stage(name)
AssistantEvent::Delta(text)
AssistantEvent::ToolCall(call)
AssistantEvent::ToolResult(result)
AssistantEvent::Final(summary)
AssistantEvent::Error(message)
AssistantEvent::Done
```

This keeps the UI independent from rust-norion's current SSE field names and
leaves room for future OpenAI-compatible or MCP-backed providers.

## Provider Contract

Provider modules should expose a small stream-first trait:

```text
trait ChatProvider {
    fn health(&self) -> ProviderHealth;
    fn stream_chat(&self, request: ChatRequest, sink: &mut dyn EventSink) -> Result<()>;
}
```

`rust_norion` should initially support:

- `/health`
- `/v1/chat-stream`
- `/v1/generate-stream`
- `/v1/business-cycle-stream`

The UI must surface `runtime_mode`, `gemma_runtime_reachable`,
`active_engine_requests`, `elapsed_ms`, and `runtime_token_count` when present.

## Session And Memory

Use disk-backed session state from day one:

- `state/sessions/<session_id>/transcript.jsonl`
- `state/sessions/<session_id>/summary.md`
- `state/project_notes.md`
- `state/pinned_context.json`

MVP context policy:

- Keep the last N turns in full text.
- Keep one rolling summary for older turns.
- Keep pinned project notes separate from transient chat.
- Never silently add huge files to prompt context; show them as selected context.

This directly addresses the indexing/performance concern: long conversations and
large files must become explicit summaries, notes, or selected context entries,
not one ever-growing prompt blob.

Current memory hygiene contract:

- `state/project_notes.md` is the only user-maintained pinned project-notes
  file for Forge.
- Model-pool `index` worker answers are stored as `model_pool_index:` blocks
  inside project notes, bounded by `model_pool_index_end:` when possible.
- The latest complete delimited `model_pool_index` block is the active index
  context for normal prompts and `/retrieve`; older complete blocks and legacy
  unterminated tails are history only.
- A legacy unterminated `model_pool_index` tail may be used only as fallback
  active context when no complete delimited block exists.
- `/context` must report `model_pool_index_active=latest_delimited`,
  `latest_legacy_undelimited`, or `none`, and `/index-notes` must mark the
  active block with `active=true`.
- `/index-notes clear` must remove every parsed model-pool index span,
  including legacy unterminated tails, while preserving handwritten notes that
  sit outside those spans. After clearing, the in-memory session context must be
  resynced so stale index text cannot be sent on the next prompt.

## Tool Permission Model

Tools should be registered before they are usable. Each tool declares:

- name
- risk level
- input schema
- dry-run support
- permission requirement
- output summary policy

MVP permissions:

- `read`: allowed by default inside the workspace.
- `search`: allowed by default inside the workspace.
- `shell`: ask before running.
- `patch`: ask and show diff before applying.
- `network`: disabled unless explicitly enabled in config.

This gives SmartSteam Forge a Claude-like permission rhythm without entangling
the first TUI milestone with full autonomous editing.

## UI Requirements

The TUI must include:

- streaming transcript with stable scrollback and auto-follow when at bottom;
- multiline input with Enter to submit and Shift+Enter or Alt+Enter for newline;
- status bar for backend, model, runtime, busy state, and token/time stats;
- command palette with fuzzy-ish prefix matching;
- compact event rows for stage/meta/heartbeat without flooding the answer;
- clear error row when Gemma is offline or the backend is busy;
- graceful cancel key for long-running local 12B inference.

Avoid putting all UI state in `main.rs` or `app.rs`. The transcript model should
own messages, selection, scroll state, and stream lifecycle markers.

## Implementation Order

1. Scaffold crate and module tree with no business-code changes.
2. Add Ratatui/Crossterm shell with static transcript fixtures.
3. Add `provider::rust_norion` health and SSE stream client.
4. Add session JSONL persistence and `/new`, `/sessions`, `/resume`.
5. Add status bar and command palette.
6. Add permission/diff placeholders, then tool registry.
7. Add business-cycle mode once chat/generate streaming is stable.

Each step should run `cargo check` inside `tools/smartsteam-forge` and at least
one provider/unit test that does not require Gemma 12B to be running.

## Risks

- Local Gemma 12B can freeze the desktop if a UI loop blocks on network reads.
  Provider streaming must run outside the render path.
- Provider streaming uses short per-read polling for heartbeat/status events;
  the total one-shot or stream wait window is `request_timeout` /
  `--timeout-secs`. Do not document `--read-timeout-ms` as the Gemma response
  budget; it is only the socket read poll interval.
- `claurst` is GPL-3.0. Keep all implementation clean-room.
- `rustgpt-lab` already has direct TCP and no dependencies; do not mutate it into
  the TUI product.
- The root repo is under heavy parallel edits. SmartSteam Forge should remain a
  standalone tool until its API contract is stable.
- Session context can grow quickly. Persist history, summarize old turns, and
  bound the prompt context by default.

## Source Links

- `fortunto2/rust-code`: https://github.com/fortunto2/rust-code
- `Kuberwastaken/claurst`: https://github.com/Kuberwastaken/claurst
- Existing lab: `tools/rustgpt-lab`
