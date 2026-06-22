# Reasoning Genome Chain and Gene Scissors

The Reasoning Genome Chain is a DNA/NDA-style control layer for Noiron
reasoning. It does not retrain model weights directly. Instead, it records and
evolves the local strategies that decide how the control plane retrieves memory,
routes attention, allocates hierarchy weights, invokes tools, reflects on
drafts, and admits reusable experience.

推理基因链是面向 Noiron 推理的 DNA/NDA 风格控制层。它不直接重训模型权重，而是记录并进化控制层策略：如何检索记忆、路由注意力、分配层级权重、调用工具、反思草稿，以及准入可复用经验。

## Goal

- Make useful reasoning behavior persistent, composable, testable, and
  reversible.
- Let English, Chinese, Rust coding, long-context, and local-tool tasks keep
  separate strategy genomes instead of overwriting one global heuristic.
- Let the engine evolve by editing control genes under evidence gates rather
  than mutating weights after every interaction.
- Keep every durable genome edit local, auditable, non-commercial-research
  friendly, and approved before shared or public adoption.

## Concepts

- `ReasoningGene`: one bounded strategy atom. Examples include memory retrieval
  posture, KV-Fusion preference, route threshold bias, hierarchy balance,
  reflection checklist, language-mode preference, Rust-only tool policy,
  sub-agent budget posture, or safety constraint.
- `ReasoningGenome`: an ordered chain or small graph of genes selected for a
  task profile. A coding genome can prefer compiler evidence and Rust tests,
  while a Chinese writing genome can prefer bilingual reflection and gist
  memory.
- `GenomeExpression`: the runtime projection of a genome into an inference
  request. It can influence router thresholds, hierarchy weights, retrieval
  limits, Toolsmith plans, Agent Team dispatch hints, reflection checks, and
  replay priority.
- `ExpressionTrace`: the sanitized evidence of what the genome changed during a
  run: gene ids, active task profile, route budget deltas, memory policy
  deltas, reflection checks, validation gates, reward inputs, and rollback
  eligibility.
- `Fitness`: the score used to judge a gene or chain. It should combine process
  reward, reflection diagnostics, compiler/test/benchmark results, latency,
  wasted-compute reduction, memory usefulness, drift severity, and user or
  operator feedback.
- `GenomeLedger`: append-only local history for proposed, rejected, admitted,
  quarantined, and rolled-back genome edits.
- `GeneSegment`: an auditable segment derived from long-term experience,
  routing fragments, KV/gist memory, reflection heuristics, validation evidence,
  or rollback anchors. Segments are represented by ids, hashes, summaries,
  purpose tags, confidence, version, and last validation result, never by raw
  private prompt payloads or hidden reasoning.

## Gene Scissors

Gene Scissors is the controlled editor for the Reasoning Genome Chain. It is
allowed to propose edits, but it is not allowed to bypass validation or mutate
private runtime state directly.

The software analogy for "keeping the DNA young" is not biological immortality.
In rust-norion it means keeping reasoning strategies fresh: aged genes are
relabelled with their current purpose, contaminated genes are quarantined
before they can be reused, and replacements regenerate from stable anchors or
high-fitness sibling genes after validation.

Supported edit intents:

- `relabel`: refresh the gene label, purpose, and tags when a useful gene has
  aged, drifted in meaning, or lost enough metadata that the engine no longer
  knows what it is for.
- `cut`: remove or disable a low-fitness gene from a profile-specific chain.
- `splice`: insert a validated gene from an experiment, replay run, or clean
  operator-approved proposal.
- `quarantine`: isolate a gene linked to drift, private prompt leakage,
  unsafe memory admission, repeated test failure, or excessive compute waste.
- `repair`: replace a malformed gene reference with a known safe fallback.
- `crossover`: combine compatible genes from two high-fitness chains, then
  force the result through dry-run gates before admission.
- `rollback`: restore the previous stable genome when a new chain regresses
  quality, latency, safety, or memory hygiene.
- `regenerate`: rebuild a young replacement gene from a stable rollback anchor,
  high-fitness siblings, and clean replay evidence after a malignant gene has
  been quarantined.

Every durable edit must carry a `MutationPlan` with the changed gene ids,
source evidence ids, expected phenotype, validation commands, rollback target,
and admission state. Preview mode must be read-only.

## Aging, Malignancy, and Regeneration

An aged gene is not automatically bad. It may still be useful but stale: its
label can be wrong, its purpose can be too broad, or its fitness can have
fallen because the task profile changed. Aged genes should first produce a
read-only `relabel` plan that refreshes metadata and reminds the engine what
the gene is for.

A malignant gene is treated as contaminated strategy, not as something to
repair in place. Malignancy can be triggered by repeated drift, unsafe memory
admission, raw prompt leakage risk, high contradiction pressure, benchmark
regression, or repeated compiler/test failure. The safe sequence is:

1. detect the malformed behavior with bounded evidence
2. quarantine the gene so it cannot influence expression
3. cut or disable the contaminated strategy only after a rollback anchor exists
4. regenerate a replacement from stable anchors and validated high-fitness
   sibling genes
5. admit the regenerated gene only after trace, tests, benchmark, drift, and
   operator approval gates pass

The "youth pressure" metric tracks how strongly a genome needs refresh. It
should rise when quality, process reward, memory hygiene, reflection, or drift
evidence worsens. It should fall only when validated relabel, quarantine,
rollback, or regeneration evidence proves the chain is healthy again.

## Dual-Chain Storage

DNA double-strand inspiration maps to two software chains, not to biological
simulation:

- `express_chain`: active, trace-visible genes that can influence routing,
  hierarchy, memory retrieval, reflection, Toolsmith, Agent Team, or budget
  posture for the current task profile.
- `memory_chain`: latent, disk-backed evidence that explains where a gene came
  from: stable anchor id, source experience ids, KV/gist memory ids, fitness
  summaries, validation gates, rejection reasons, and rollback links.

The express chain is small and fast enough to project during inference. It is
the visible reasoning-control chain: task profile, selected route fragments,
threshold bias, KV admission posture, reflection heuristic, budget posture,
validation gate, and outcome digest. It explains which strategy atoms were
active without exposing hidden reasoning.

The memory chain is larger and append-only, so it can preserve provenance
without loading raw private prompts, raw `.ndkv` payloads, secrets, hidden
reasoning, or copied third-party internals. It stores source ids, sanitized
evidence summaries, version, purpose tags, confidence, last validation result,
fitness/drift summaries, rejection reasons, and rollback links. A future
admitted mutation must update both chains atomically: expression metadata for
runtime use, and memory-chain evidence for audit and rollback.

## Splicing and Variant Repair

`dna_splicer` treats long context and memory as `GeneSegment` records: token
range, source hash, profile, tenant/session scope, semantic gist, KV residency
hint, fitness score, drift score, privacy risk, version, confidence,
last-validation result, and validation status.
Segments can be classified as:

- `exon`: useful segment allowed into expression or KV prefill.
- `intron`: redundant or low-value segment kept only as cold evidence or
  omitted from expensive attention.
- `variant`: malformed, drifting, privacy-risk, or schema-invalid segment that
  must be isolated before reuse.

The first Rust model is read-only and is now wired into inference evidence.
`DnaSplicer::preview` classifies prompt chunks, retrieved memory, gist records,
and runtime-KV exports into splice segments. `MutDetector` reports insertion,
deletion, mislabel, truncation, format-drift, stale-label, drift, privacy,
KV-shape, schema, empty-range, and missing-source-hash variants as read-only
findings, and `MutFixer` converts those findings into preview `MutationPlan`
values. It can propose variable splicing with bounded overlap, intron
filtering, segment isolation, relabeling stale metadata, quarantining malignant
segments, regenerating from a stable anchor, or repairing invalid metadata, but
it cannot apply the change or authorize persisted writes. Trace schema and
benchmark gates emit splice segment, exon/intron/variant, finding, proposal,
and read-only status evidence so splicing cannot silently disappear from
control-plane runs.

## Relabel, Quarantine, and Regeneration

DNA aging maps to software freshness. When a memory or gene segment expires,
drifts, loses semantic precision, or keeps an obsolete purpose label, the first
action is a read-only relabel/rejuvenation plan. The plan must summarize the
evidence, current version, proposed purpose tags, confidence, last validation
result, and rollback anchor. Useful but old segments should be refreshed before
they are discarded.

Malignant mutation is handled by isolation, not blind repair in place. Harmful,
polluted, low-confidence, privacy-risk, or secret-risk segments are quarantined
with digest-only evidence, reason codes, validation requirements, and rollback
records. Repair, regeneration, and re-admission are separate later steps. They
can only use stable anchors, validated high-fitness siblings, sanitized
summaries, and explicit approval; they must not leak raw prompts, secrets,
copied source text, raw `.ndkv` payloads, or hidden reasoning.

## Reference Mapping

Public projects and papers are research references, not code sources. The
project should keep a clean-room boundary: copy no third-party implementation
details unless license review and attribution are explicit.

- Bio-inspired references:
  - Evo/Evo2: long DNA sequence modeling and multi-scale context inspire the
    express/memory-chain split and long-range gene expression.
  - SpliceBERT and SpliceTransformer: splice/variant-effect ideas inspire
    `dna_splicer`, `MutDetector`, and local edit impact scoring.
  - AlphaGenome: variant scoring inspires read-only mutation impact previews;
    model/API terms require non-commercial caution.
  - GeneFormer: hierarchy and perturbation ideas inspire networked gene
    fitness and task-context expression.
  - TrinityDNA: paper-level multi-scale genome modeling reference only; do not
    assume reusable code.
- Long-context and KV references:
  - CEPE, RAPID, SEQPACK, StreamingLLM, and ChunkedRAG are backlog references
    for parallel context chunks, attention sinks, sliding windows, segment-local
    KV, packing, and retrieval chunking.
  - These references need fact, license, and official-code-status verification
    before any implementation issue treats them as more than behavior
    inspiration.
- Rust runtime and gateway references:
  - Candle, mistral.rs, and Axum/OpenAI-compatible gateway projects
    inform runtime traits, pipeline stages, prompt/KV cache metadata,
    telemetry, and tenant isolation.
  - Migration should be by behavior specification and tests: no direct source
    copying, no third-party weight dependency in the core path.
- External agent codebases:
  - `fortunto2/rust-code` can only be used after MIT/license attribution review
    and a small Norion-owned port plan.
  - `Kuberwastaken/claurst` is GPL-3.0 concept reference only. Do not copy
    source, tests, prompts, assets, docs text, schemas, or command/tool
    implementations unless the project explicitly accepts GPL obligations.

## Safety Gates

- No raw private prompts, raw chat logs, model weights, or `.ndkv` payloads are
  copied into genome records. Store ids, summaries, counters, and bounded
  metrics only.
- Gene Scissors proposals start as read-only plans. `admission_write_authorized`
  remains false until validation and operator approval are explicit.
- Genome, memory, and experiment-ledger writes default to preview/read-only.
  Durable writes require a writer gate, validation evidence, rollback plan,
  privacy/license checks, and maintainer/operator approval.
- A proposal that touches routing, memory, reflection, Toolsmith, Agent Team, or
  adaptive state must pass trace/schema gates before it can be persisted.
- Rust-facing genome changes require `cargo fmt`, focused tests, and the
  relevant benchmark or inspection gate before admission.
- Fitness must include negative evidence. Repeated drift, stale-memory reuse,
  unsafe compaction, budget pressure, compiler failure, or benchmark regression
  must lower gene priority or trigger quarantine.
- Rollback is part of the edit, not a later wish. Each admitted mutation must
  name the stable ledger entry it can restore.
- Quarantine records preserve only digest, reason, evidence summary, and
  rollback metadata; they must not preserve raw private payloads.

## Integration

- `reflection` detects weak drafts, contradictions, repair actions, and memory
  admission hints that become fitness evidence.
- `process_reward` scores route, memory, hierarchy, latency, Toolsmith, Agent
  Team, and reflection behavior for gene fitness.
- `experience_replay` can propose reinforcement, penalty, cut, splice, or
  quarantine candidates from past runs.
- `adaptive_state` stores only admitted genome state and rollback pointers.
- `trace` emits expression and mutation-plan evidence with schema gates.
- `benchmark` and state inspection add floors for genome expression coverage,
  mutation proposal coverage, successful rollback evidence, and no unsafe
  write-through during preview.
- `agent_team` may produce read-only genome-edit proposals, but the main owner
  remains the only writer to code, memory, and adaptive state.

## Initial Milestones

1. Define `ReasoningGene`, `ReasoningGenome`, `GenomeExpression`,
   `MutationPlan`, and `GenomeLedger` as pure Rust data models.
2. Emit read-only expression traces for active task profiles without changing
   behavior.
3. Add benchmark and trace gates that prove expression evidence is present and
   sanitized.
4. Let process reward and reflection produce gene fitness summaries.
5. Add Gene Scissors proposal mode for cut, splice, quarantine, and rollback
   plans with `admission_write_authorized=false`.
6. Add gated admission after focused tests, benchmark floors, drift checks, and
   explicit operator approval.
7. Add task-specific genomes for English, Chinese, Rust coding, long-context,
   and local-tool workflows.
8. Add aging relabel, malignant quarantine/cut, regeneration, and youth-pressure
   gates for long-running genomes.
9. Add read-only `GeneSegment`, `DnaSplicer`, `MutDetector`, and `MutFixer`
   models for exon/intron/variant classification and mutation-plan previews.
10. Emit splice preview evidence from `InferenceOutcome`, trace schema, and
    benchmark summaries without granting write access.

## Non-Goals

- It is not biological simulation.
- It is not automatic weight retraining.
- It is not a path for bypassing review, licensing, non-commercial limits, or
  human approval.
- It is not allowed to store raw private prompts or copied third-party project
  internals as genes.
