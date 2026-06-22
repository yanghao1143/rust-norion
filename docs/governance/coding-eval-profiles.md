# Coding Evaluation Profiles

Issue #75 adds the first license-safe coding evaluation baseline for the
Noiron self-evolution loop. The corpus is intentionally small, deterministic,
and local: it exists to gate control-plane changes before larger benchmark
fixtures, not to import proprietary prompts or public benchmark payloads.

## Profiles

`norion-eval::default_coding_eval_corpus()` installs five fixture families:

- English instruction: evaluates recoverable Rust error-handling guidance.
- Chinese instruction: evaluates Chinese Rust ownership and borrowing guidance.
- Rust code generation: evaluates a small `parse_port`-style Rust function.
- Rust repair: evaluates lifetime-aware repair guidance using borrowed data
  where possible.
- Multilingual coding explanation: evaluates English and Chinese explanation
  of safe Rust service error handling.

Each fixture carries expected markers, forbidden markers, provenance flags,
overfit guard markers, and optional Rust validation plans. Rust code-generation
and repair fixtures must include both `cargo check` and `cargo test` manifest
plans before the corpus validator passes.

## Scoring Gates

`CodingEvalScoringProfile` keeps thresholds per profile:

- marker coverage floor
- compile-check requirement
- unit-test requirement
- benchmark-regression cap
- memory-hit floor
- token budget
- latency budget
- redaction requirement

Failures are categorized as missing markers, forbidden markers, missing or
failed compiler/test evidence, benchmark regression, memory miss, budget
overrun, redaction violation, overfit risk, or unsafe fixture provenance. The
suite report aggregates pass rate, average score, profile coverage, failure
counts, redaction failures, and overfit suspects so later CLI/benchmark
surfaces can attach the same evidence packet.

## Evidence Packets

Evidence packets are digest-only. They include fixture id, profile, prompt
digest, output digest, validation command lines, score inputs, failure
categories, and provenance digest. Raw prompts and raw model output are not
serialized into `record_line()` or `summary_line()`.

This makes the packets suitable for GitHub issue updates, experiment ledgers,
and future benchmark attachments without leaking private prompts, secrets,
hidden reasoning, copied third-party text, or proprietary benchmark material.

## Adding Fixtures Safely

New fixtures must follow these rules:

- Use synthetic local prompts written for this repository.
- Do not copy HumanEval, MBPP, LeetCode, private chat logs, proprietary
  benchmark text, or unreviewed external source.
- Keep `provenance` equal to `synthetic-local-noncommercial-fixtures-v1` unless
  a separate license review adds a new approved provenance value.
- Set `license_safe = true` and `private_source = false` only after checking
  the fixture text and expected markers.
- Add explicit `VerificationPlan` entries for Rust fixtures that claim compile
  or test evidence.
- Add forbidden markers for dangerous shortcuts, leaked private context, or
  invalid task-specific behavior.
- Add overfit guard markers when a fixture could be memorized through id,
  benchmark name, or hard-coded answer patterns.
- Extend tests for corpus validation, pass/fail thresholds, redaction, and
  improvement comparison whenever adding a new profile family.

## Queue Role

#75 is a completed baseline once the corpus, scorer, evidence packets, docs,
and focused tests pass. The default Noiron pursuit queue then advances through
#76 memory consolidation, #78 deployment guardrails, and R94 writer gate
consolidation before R95 reference fact/license verification. Later goals remain
isolated until the active goal reaches its success gate, clean stop condition,
rollback state, or approval hold.
