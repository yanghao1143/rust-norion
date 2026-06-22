# Runtime Session State API

`SessionStateRecord` is the runtime-facing session API for resumable local
chats, Rust coding tasks, and Telegram-style multi-tenant conversations. It is
a sanitized state record, not a transcript store.

## Record Shape

Each session state record includes:

- `TenantScope`: tenant id, workspace id, and session id
- `SessionRuntimeProfile`: task profile, model id, tokenizer, context window,
  max tokens, streaming flag, and cancellation anchor digest
- memory-chain anchors
- gene-chain anchors
- retrieval evidence anchors
- routing evidence anchors
- turn digests
- source trace ids
- state digest

Turn records use `SessionTurnDigest`. Raw user and assistant payloads are
digested; durable state stores a digest and bounded metadata only. The default
record flags are:

- `raw_messages_stored = false`
- `read_only = true`
- `report_only = true`
- `preview_only = true`
- `write_allowed = false`

## Persistence

`SessionStateStore` persists records through `DiskKvStore` with
`TenantResourceLane::SessionState`. Durable writes are disabled by default and
require all of:

- `durable_writes_enabled = true`
- `operator_approved = true`
- tenant isolation gate allows the actor scope to write the target scope
- record validation passes

Default use is preview-only. The store never auto-commits, auto-pushes, mutates
memory, mutates gene chains, trains adapters, or changes model weights.

## Replay Preview

`SessionReplayPlanner::preview` reconstructs replay inputs from digests and
anchors:

- retrieval inputs come from memory-chain and retrieval evidence anchors
- routing inputs come from routing evidence plus runtime profile metadata
- gene inputs come from gene-chain anchors

The preview is read-only and emits no raw payloads. Cross-tenant or
cross-session replay is rejected before any inputs are returned.

## Corruption Handling

Session state parsing fails closed. A malformed, schema-mismatched, or digest
mismatched record returns `SessionStateReadReport` with:

- `record = None`
- `corrupt = true`
- redacted error label
- error digest
- `raw_payload_exposed = false`

The error digest lets operators correlate corrupt state with storage evidence
without printing the corrupt payload.

## Local Model Service Use

The #19 local model service should treat this API as the conversation-state
boundary:

1. Build or load a `SessionStateRecord` for the tenant/workspace/session.
2. Use `SessionReplayPlanner` to preview memory, gene, and routing inputs.
3. Build `RuntimeRequest` from the previewed anchors and the current prompt.
4. After inference, write only sanitized session metadata if explicit writer
   policy and operator approval allow it.
5. Keep raw transcript storage in a separate, explicitly governed component if
   a product ever needs it.
