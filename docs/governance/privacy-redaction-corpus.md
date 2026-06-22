# Privacy Redaction Corpus

Issue #47 adds a deterministic, synthetic redaction corpus for memory, genome,
trace, benchmark, and GitHub evidence. The goal is to keep exported evidence
useful for review while making raw private or executable payloads impossible to
publish by accident.

## Policy

Policy version: `privacy_redaction_policy_v1`

- May store: stable digests, reason codes, lane names, counts, validation
  status, rollback anchors, and already-digest-only source hashes.
- Must hash: prompts, answers, private chats, tenant identifiers, credentials,
  secrets, hidden reasoning markers, and unreviewed source snippets.
- Must drop: executable payload text, raw secrets, private key material, copied
  third-party source, raw tenant ids, and hidden chain-of-thought.
- Future gates: memory admission, reasoning genome, trace schema, benchmark
  evidence, and GitHub evidence comments must use the shared detector or corpus
  before durable export.

## Synthetic Fixtures

The corpus lives in `src/privacy_redaction.rs` and covers:

- secrets and API keys
- private chats
- credentials
- private prompt payloads
- raw answer payloads
- malicious executable instructions
- tenant identifiers
- hidden reasoning markers
- unreviewed external source payloads

Every fixture is synthetic, digest-only in output, and safe to publish. Fixture
results expose `redaction-digest:*`, reason codes, lane, action, policy version,
and corpus version. They never export the raw fixture payload.

## Gate Contract

Any future writer or evidence exporter should fail closed when:

- output contains a private or executable marker
- output contains a raw fixture payload
- output lacks a stable digest
- output lacks reason codes
- output cannot identify its lane and policy version
- memory/genome/evolution writes are not still preview/read-only unless writer
  gates, validation, rollback, redaction, and operator approval all pass

The shared detector is intentionally broader than the previous local checks. It
now catches raw prompt/answer markers, credentials, tenant ids, private chats,
hidden reasoning markers, executable shell payloads, and unreviewed source
payloads across the existing memory, genome, trace, and benchmark validators.
