# Compiler-Guided Rust Coding Repair Loop

The Rust coding repair loop turns compiler and test feedback into reusable
self-training evidence without auto-committing, auto-pushing, or promoting a
behavior change by itself.

`RustCodingRepairHarness` is the first read-only harness for this loop. It can
record five validation lanes:

- formatting: `cargo fmt --check` or an equivalent formatter result
- compiler: `cargo check`, `rustc --emit=metadata`, or a small snippet fixture
- lint: `cargo clippy` or a local lint-equivalent gate
- tests: unit, integration, or focused fixture tests
- benchmarks: smoke benchmarks or regression budget checks

The harness can run a small Rust snippet through `RustSnippetValidator` for the
compiler lane. Other command outcomes are supplied as structured evidence by
the caller, which keeps the core crate deterministic and avoids spawning broad
workspace commands from library code.

## Evidence Packet

Each command outcome becomes a `RustCodingCommandEvidence` record with:

- command kind
- passed, failed, timed-out, or skipped outcome
- optional status code
- duration budget observation
- redacted diagnostic preview
- diagnostic digest
- redacted evidence id

Raw prompts, raw answers, private diagnostics, secrets, and unrelated source
payloads are not stored in reports. Payloads are digested; public previews are
bounded and redacted.

## Decisions

`RustCodingRepairReport` classifies a repair attempt as:

- `validated_candidate`: formatting, compiler, lint, tests, benchmarks,
  rollback anchor, and rollback replay are all present and clean
- `held_for_evidence`: required evidence is missing or skipped
- `retained_for_learning`: a command failed or timed out
- `privacy_blocked`: prompt or response payload contains private material

Failure classes are retained for benchmark regression analysis: formatting,
compiler, lint, tests, benchmarks, timeout, missing evidence, and privacy
blocked.

## Candidate Flow

Only `validated_candidate` repairs produce candidate summaries. The harness
creates both:

- memory candidate: reusable compiler/test/benchmark repair experience
- gene candidate: reusable Rust-coding strategy or prompt-policy gene

Those candidates are immediately evaluated by `NoWeightRetrainGate`. They stay
preview-only and write-disabled. Operator approval, rollback anchors,
compiler/test/benchmark validation, benchmark delta, and regression budget are
still required before any downstream gate can admit them.

Failed repairs are still useful. They are retained in `ImprovementCorpus` as
non-promoted learning examples with failure-class tags, so future benchmarks
can detect recurring failure modes without turning a bad repair into active
memory.

## Safety Rules

- No automatic git commit.
- No automatic git push.
- No durable memory write.
- No gene-chain mutation.
- No adapter training.
- No model-weight mutation.

This loop creates evidence. Separate approval, benchmark, rollback, memory, and
genome gates decide whether any future action may be applied.
