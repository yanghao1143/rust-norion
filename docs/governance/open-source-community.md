# Open Source and Community Plan

This plan keeps rust-norion focused on its real niche: a DNA-inspired inference
control-layer engine for long-context governance, no-weight self-evolution, and
auditable local model behavior. It is not a general-purpose high-throughput LLM
runtime, and community work should not pull the project into competing with
runtime-first projects on kernel count, model format coverage, or accelerator
specific tuning.

## Positioning

The project should describe itself in one sentence:

`rust-norion is a Rust inference control-layer engine inspired by DNA gene-chain
mechanisms, focused on long-context governance, self-evolving control state, and
safe local deployment around pluggable model runtimes.`

The practical distinction is:

| Project class | Primary job | rust-norion relationship |
| --- | --- | --- |
| candle / mistral.rs / rvLLM | Run model weights efficiently across hardware and model families. | Possible future backends, not the main battlefield. |
| AI gateways | Route requests, rate-limit, meter, and proxy models. | Adjacent infrastructure, but usually no genome/memory evolution layer. |
| rust-norion | Govern context, memory, routing, reflection, genome edits, writer gates, and self-goal evolution. | Core project identity. |

Before accepting new scope, ask:

- Does it strengthen the inference control-layer moat?
- Does it deepen gene-chain context governance, self-evolution, KV memory, or
  writer-gated safety?
- Can the model runtime remain pluggable?
- Can a single maintainer review and validate it without open-ended support
  burden?

If the answer is no, keep it out of the core repository or route it to an
external adapter/plugin.

## License Direction

Current policy is GPL-3.0. Everyone, including contributors, may use, deploy,
modify, and redistribute rust-norion commercially under the same GPL-3.0 terms.
Derivative works and redistributed modifications must remain open source under
GPL-3.0-compatible terms.

This avoids a contributor-only commercial carve-out, which would be hard to
reason about once contributors touch code written by other contributors. The
project stays open for commercial deployment and research while preserving the
copyleft requirement that improvements remain available to the community.

Future license changes should still be handled by an explicit ADR or
license-transition pull request before any repository-wide change.

## Documentation Layers

Keep docs layered so newcomers do not have to decode the whole roadmap first.

1. Five-minute path:
   - deterministic reference-runtime demo;
   - no model download required;
   - visible output for gene splicing, mutation repair, or self-goal preview;
   - exact command and expected summary output.
2. Design path:
   - reasoning genome chain;
   - express chain vs memory chain;
   - intron filtering;
   - mutation detection and repair;
   - self-goal proposal, admission, queue preview, writer preflight, apply plan;
   - model-runtime boundary and backend plug-in contract.
3. Contribution path:
   - issue-first workflow;
   - safety and clean-room rules;
   - focused validation commands;
   - good-first-issue examples;
   - maintainer response expectations.

## Demo Strategy

Prioritize demos that show the unique control-layer value without requiring a
large trained model:

- gene splicing demo: redundant context in, filtered gene segments out;
- mutation repair demo: malformed structured segment in, repaired or isolated
  segment out;
- self-goal demo: proposal report, admission report, queue preview, writer
  preflight, and apply plan, all digest-only and unapplied;
- backend boundary demo: deterministic local runtime behind the same trait a
  production backend would implement.

Each demo should print measurable counters such as compression ratio, isolated
mutation count, repair count, cache hit hint, rollback anchor count, and
writer-gate decision.

## Issue Taxonomy

Keep issues small enough for a single maintainer to review.

Recommended labels:

- `good first issue`: docs, examples, focused tests, clearer errors, small
  refactors with no policy change;
- `control-plane`: routing, hierarchy, reflection, scheduler, self-goal queue;
- `genome`: gene-chain, splicing, mutation repair, lineage, gene scissors;
- `memory`: disk-backed KV, tiering, KV-Fusion, consolidation;
- `runtime-boundary`: backend traits, reference kernel, conformance gates;
- `governance`: writer gates, rollback, privacy, licensing, branch protection;
- `benchmark-evidence`: reproducible measurements and regression checks.

Every implementation issue should state:

- target module;
- side effects and durable writes, if any;
- validation commands;
- rollback expectation;
- raw-data/privacy handling;
- external reference and license status.

## Community Growth

Do not optimize for broad traffic first. Start with users who care about Rust,
AI infrastructure, private deployment, long-context control, or bio-inspired
model governance.

Seed channels should focus on technical substance:

- Rust communities: architecture and safety tradeoffs;
- AI infrastructure groups: long-context governance and controlled local state;
- research circles: DNA-inspired control-plane design and experimental results.

Use GitHub Issues and Discussions as the primary community surface until there
are enough repeat contributors to justify a separate chat space.

## Contributor Path

Growth should be explicit and reversible:

1. Contributor: opens issues, docs fixes, tests, examples, or focused code PRs.
2. Module collaborator: repeatedly contributes to one module and can triage or
   review that module's issues.
3. Maintainer: trusted to approve scoped areas after sustained high-quality
   contributions.

Protected branches still require owner/maintainer approval. Commercial use is
allowed under GPL-3.0 terms, but public collaboration does not bypass review,
attribution, validation, or third-party license compatibility requirements.

## Single-Maintainer Guardrails

The project stays healthy by refusing avoidable support load:

- no broad model-format chase in the core repository;
- no production performance promises until the backend boundary and benchmark
  gates prove them;
- no durable memory/genome/experiment/goal-queue writes without writer gates,
  rollback anchors, privacy checks, and explicit approval;
- no copied GPL/AGPL/commercial source unless an explicit written license
  decision accepts the obligation;
- no feature that cannot be validated by focused tests, traces, or benchmark
  evidence.

The right community shape is small, serious, and technically dense. rust-norion
should win by making its gene-chain control layer real, measurable, and safe.
