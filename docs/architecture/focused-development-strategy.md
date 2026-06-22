# Focused Development Strategy

rust-norion should stay focused on one core identity: a Rust inference
control-layer engine with DNA-inspired gene-chain governance and bounded
self-evolution. It is not trying to become a general-purpose high-throughput
model runtime.

## Core Position

The durable moat is the control plane:

- reasoning genome chain: express chain, memory chain, gene scissors, relabel,
  mutation isolation, and rollback;
- long-context governance: splicing, intron filtering, segment repair,
  recursive scheduling, and KV residency;
- adaptive control state: routing thresholds, hierarchy weights, KV-Fusion,
  process reward, and reflection;
- writer-gated safety: digest-only evidence, preview plans, rollback anchors,
  branch protection, and explicit apply gates;
- backend boundary: `InferenceBackend`, `ModelRuntime`, and production-kernel
  conformance keep the control layer independent from the runtime.

Runtime kernels are supporting infrastructure. The local deterministic runtime
should remain useful for ABI tests and governance gates, while production
performance can later come from a pluggable backend.

## Priority Order

### 1. Foundations

- Freeze the key abstractions only after focused validation: `GeneSegment`,
  `DnaGeneChain`, splicing, mutation detection, `InferenceBackend`, and
  `ModelRuntime`.
- Keep Hot/Warm/Cold KV, append-only ledgers, KV-Fusion, and tier migration
  stable before adding more autonomous behavior.
- Keep the backend boundary stable enough that candle, mistral.rs, or a
  self-owned production kernel could attach without rewriting the control
  plane.

### 2. Distinctive Control Features

- Complete the splicing and mutation-repair pipeline from input segmentation to
  isolated repair, transcription, inference, and reviewed write-back.
- Keep self-evolution minimal and measurable first: execute, score, propose
  mutation, validate, apply only through explicit gates, and roll back on failed
  evidence.
- Build benchmarks around the unique value: context compression, KV reuse,
  mutation isolation, repair success, writer-gate pass/fail rates, and
  self-evolution rollback safety.

### 3. Peripheral Features

Defer broad features until the core is stable:

- large model-format matrices;
- many hardware backends;
- generic gateway features;
- broad visual operations dashboards;
- plugin ecosystems that are not needed by the gene-chain control loop.

## Acceptance Filter

Before merging new scope, answer yes to at least one:

- It strengthens gene-chain inference control.
- It improves durable KV memory, routing, hierarchy, reflection, or
  self-evolution evidence.
- It makes the backend boundary safer or more measurable.
- It reduces maintainer load through tests, docs, or automation.

If none apply, keep it out of the core or make it an external adapter.

## Single-Maintainer Guardrails

- Prefer small issues with explicit validation commands.
- Keep durable writes preview-only until a writer/apply gate and rollback plan
  are proven.
- Do not copy GPL/AGPL/commercial source without an explicit written license
  decision.
- Avoid abstractions that exist only to resemble biology; every DNA metaphor
  must map to measurable engineering behavior.
- Do not accept performance claims without a benchmark gate or trace evidence.
