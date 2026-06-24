# Project Context

This document keeps the longer project positioning that used to make the root
README hard to scan. The short README should stay focused on onboarding,
current maturity, quick validation, architecture boundaries, and contribution
entry points.

## Positioning

`rust-norion` is a DNA-inspired Rust inference control layer and
self-evolution control engine. It is designed around a self-owned Transformer
runtime, but the repository currently provides the control plane prototype,
runtime boundaries, deterministic reference kernels, local state gates, and
backend adapters rather than a trained production inference kernel.

The project aims to make inference behavior more adaptive without retraining
model weights on every interaction. It does this by evolving the control
surface around inference: routing, memory selection, hierarchy weights,
reflection, process rewards, experience replay, hardware-aware execution plans,
and auditable writer gates.

## Non-Goals

- Do not claim KV memory or reasoning genes are a replacement for model-weight
  training.
- Do not treat the current repository as a complete production LLM runtime.
- Do not bind the core design to Gemma, Llama, Qwen, closed model services, or
  one vendor-specific runtime.
- Do not copy incompatible external code, generated private artifacts, model
  weights, local memory databases, credentials, or unreviewed durable evolution
  state into the repository.

## Control-Plane Model

The stable split is:

- model runtime: tokenizer, embeddings, weights, native context window,
  forward kernels, optional KV import/export
- control plane: adaptive routing, recursive scheduling, memory tiering,
  sparse context filtering, reflection, process rewards, experience replay,
  writer gates, persisted adaptive state
- runtime boundary: `ModelRuntime`, `InferenceBackend`, `RuntimeBackend`,
  `RuntimeManifest`, and `ProductionTransformerRuntime`

The current prototype can validate the boundary end to end with deterministic
local/reference runtimes. A real self-developed production forward kernel should
be connected behind the same manifest, device, and KV ABI gates.

## DNA / Reasoning Genome Direction

The DNA-inspired part of the project is a control-plane metaphor and data model,
not a claim of biological equivalence. Reasoning genes encode reusable routing,
retrieval, reflection, language, safety, and tool-use behavior. Gene Scissors
proposals are expected to pass preview, validation, transaction journal,
rollback, and writer-gate checks before any durable mutation is accepted.

More detail lives in:

- [Reasoning Genome Chain](architecture/reasoning-genome-chain.md)
- [Reasoning Genome Schema](architecture/reasoning-genome-schema.md)
- [Gene purpose relabel validator](governance/gene-purpose-relabel-validator.md)
- [Gene Scissors transaction journal](governance/gene-scissors-transaction-journal.md)

## Local Algorithm Stack

The intended algorithm stack is model-weight independent:

- adaptive routing across projection, local-window attention, global attention,
  and convolution-style fusion paths
- reinforced KV memory with decay, clustering, tiering, and compaction policy
- recursive long-context scheduling for inputs beyond the native model window
- task-aware hierarchy weights for coding, writing, general reasoning, and long
  document profiles
- reflection diagnostics, revision actions, process rewards, and drift gates
- hardware-aware execution plans for CPU-only, integrated/discrete GPU,
  unified-memory, mobile, embedded, browser-WASM, microcontroller, NPU, edge,
  server, and multi-GPU profiles

## Roadmap Location

The detailed roadmap is intentionally outside the README:

- [ROADMAP.md](../ROADMAP.md)
- [Focused Development Strategy](architecture/focused-development-strategy.md)
- [Open Source and Community Plan](governance/open-source-community.md)

Near-term engineering priorities remain:

- keep the control loop measurable, testable, and replaceable
- prefer model-side tokenizer/embedding signals when a self-developed runtime
  exposes them
- preserve deterministic reference kernels and local gates until a real
  production kernel is attached
- expand runtime conformance, KV quantization, memory hygiene, and all-device
  benchmark gates
- keep durable self-evolution writes behind explicit validation and maintainer
  approval

## Research Draft

The research-facing draft is:

- [Bio-Inspired Inference Control for Local Large Language Models: A DNA
  Gene-Chain Architecture in Rust](research/bio-inspired-inference-control-report.tex)

Submission metadata and the TeX upload checklist are tracked in
[docs/research/README.md](research/README.md). The draft should remain aligned
with the repository's prototype boundary and GPL-3.0 license statement.
