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

## Gene Scissors

Gene Scissors is the controlled editor for the Reasoning Genome Chain. It is
allowed to propose edits, but it is not allowed to bypass validation or mutate
private runtime state directly.

Supported edit intents:

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

Every durable edit must carry a `MutationPlan` with the changed gene ids,
source evidence ids, expected phenotype, validation commands, rollback target,
and admission state. Preview mode must be read-only.

## Safety Gates

- No raw private prompts, raw chat logs, model weights, or `.ndkv` payloads are
  copied into genome records. Store ids, summaries, counters, and bounded
  metrics only.
- Gene Scissors proposals start as read-only plans. `admission_write_authorized`
  remains false until validation and operator approval are explicit.
- A proposal that touches routing, memory, reflection, Toolsmith, Agent Team, or
  adaptive state must pass trace/schema gates before it can be persisted.
- Rust-facing genome changes require `cargo fmt`, focused tests, and the
  relevant benchmark or inspection gate before admission.
- Fitness must include negative evidence. Repeated drift, stale-memory reuse,
  unsafe compaction, budget pressure, compiler failure, or benchmark regression
  must lower gene priority or trigger quarantine.
- Rollback is part of the edit, not a later wish. Each admitted mutation must
  name the stable ledger entry it can restore.

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

## Non-Goals

- It is not biological simulation.
- It is not automatic weight retraining.
- It is not a path for bypassing review, licensing, non-commercial limits, or
  human approval.
- It is not allowed to store raw private prompts or copied third-party project
  internals as genes.
