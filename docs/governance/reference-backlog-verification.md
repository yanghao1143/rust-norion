# Reference Backlog Verification

R95 verifies the external reference backlog behind issues #63, #64, and #65.
The result is preview-only governance evidence: it does not vendor external
source, enable durable memory writes, mutate the genome, or grant commercial
permission.

The executable companion is `src/reference_backlog.rs`, which records the same
source/license decisions in `reference_backlog_v1` and exposes a deterministic
`rust-norion-reference-backlog-v1` report.

## Scope

- #63: DNA-inspired genome architecture references for `DnaGeneChain`,
  `GeneSegment`, splicing, mutation detection, repair, lineage, aging,
  rejuvenation, and quarantine.
- #64: chunked-context, segmented KV, and fragment-repair references for
  `GeneSegment`, disk-backed KV, semantic retrieval, quarantine, and repair
  candidates.
- #65: Rust-native inference and gateway references for local model service,
  chunked KV hooks, streaming, cancellation, device choice, quantization, and
  CPU/stub fallback.

Biological DNA language is used only as a software-control metaphor. Norion is
not making biological, medical, rejuvenation, or human-DNA intervention claims.

## Decision Legend

| Decision | Meaning |
| --- | --- |
| `code_reference_with_attribution` | License is compatible enough to read as a reference, but any source-level reuse still needs attribution, dependency review, and a scoped Norion-owned port plan. |
| `safe_algorithmic_reference` | Verified paper or algorithmic idea may inform a software spec; no source-copy permission is inferred. |
| `concept_only` | Use only generic concepts; do not copy source, tests, prompts, schemas, docs, assets, UI layout, or tool implementation. |
| `blocked_until_license_review` | Do not use source-level material until an explicit license review clears it. |
| `unverified` | No canonical source was verified; keep it out of implementation issues except as a generic pattern to re-check. |

No record permits unreviewed source copy.

## Verified Backlog

| Reference | Area | Source / checked ref | License | Decision | Norion target |
| --- | --- | --- | --- | --- | --- |
| Evo | Genome | `evo-design/evo` `main@6856bba48bd0b212fb10919bdafc34795338e154`, pushed 2026-03-20 | Apache-2.0 | `code_reference_with_attribution` | `DnaGeneChain`, `DnaSplicer`, `MutDetector` |
| Evo2 | Genome | `ArcInstitute/evo2` `main@53f195997257c56c00e5ef8d33a54f5baad143a6`, pushed 2026-06-19 | Apache-2.0 | `code_reference_with_attribution` | `ReasoningGenome`, `GeneSegment`, `DnaGeneChain` |
| SpliceTransformer | Genome | `ShenLab-Genomics/SpliceTransformer` `main@b67a51dabf27e2980331cec197e4396513c0b34c`, pushed 2024-11-22 | Apache-2.0 | `code_reference_with_attribution` | `DnaSplicer`, `MutDetector`, Gene Scissors journal |
| SpliceBERT | Genome | `chenkenbio/SpliceBERT` `main@dc1d8781f6f167c70421c3f8b809772637031d98`, pushed 2024-05-20 | BSD-3-Clause | `code_reference_with_attribution` | splice classification fixtures |
| AlphaGenome | Genome | `google-deepmind/alphagenome` `main@d5c9fffa8a5151c9fbd537bf9d508701ff07f125` and `google-deepmind/alphagenome_research` `main@232fc695d1eab27bac9e94bcd4b50499139ba4e1`, checked 2026-06-22 | Apache-2.0 | `code_reference_with_attribution` | variant taxonomy, relabel validation, malignant-gene drills |
| GeneFormer | Genome | no canonical `Transformer-Based Gene Compression` paper or repository verified, checked 2026-06-22 | `NOASSERTION` | `unverified` | keep as a segmented-gene-blocking idea until an exact DOI/arXiv/repo is recorded |
| TrinityDNA | Genome | no canonical paper or repository verified, checked 2026-06-22 | `NOASSERTION` | `unverified` | keep as a hierarchy/inheritance idea until an exact DOI/arXiv/repo is recorded |
| CEPE | Chunk/KV repair | `princeton-nlp/CEPE` `main@53ca69b757b84872a234a4272e217ed453516616`, pushed 2024-06-13 | MIT | `code_reference_with_attribution` | chunked KV layout and recursive scheduling |
| StreamingLLM | Chunk/KV repair | `mit-han-lab/streaming-llm` `main@2e5042606d69933d88fbf909bd77907456b9b4dd`, pushed 2024-07-11 | MIT | `code_reference_with_attribution` | resident/rolling segment policy and cache budgeting |
| RAPID | Chunk/KV repair | `real-absolute-AI/RAPID` `main@22d41f4113fe862bc80b35f770218360182a1be3`, pushed 2025-03-02 | `NOASSERTION` | `blocked_until_license_review` | repair-plan inspiration only after license review |
| Omni-DNA / SEQPACK | Chunk/KV repair | `Zehui127/Omni-DNA` `main@fbc0a4ef3d7094b6d1bfcd027ae413d9f0eb9cdc`, pushed 2025-02-20 | `NOASSERTION` | `blocked_until_license_review` | paper-level compression ideas; repository source blocked |
| ChunkedRAG | Chunk/KV repair | no canonical repository verified, checked 2026-06-22 | `NOASSERTION` | `unverified` | generic schema-validated chunking pattern only |
| candle | Rust inference | `huggingface/candle` `main@29a15c2bb802b56e05c5c63a6da331473a94d98b`, pushed 2026-06-20 | Apache-2.0 | `code_reference_with_attribution` | local runtime, production kernel, runtime manifest |
| mistral.rs | Rust inference | `EricLBuehler/mistral.rs` `master@3ee69a72bb1b80d4ae14263905babe7cd7a831ea`, pushed 2026-06-22 | MIT | `code_reference_with_attribution` | `MistralRsHttpRuntime`, streaming, chunked KV hooks |
| axum-style LLM gateway | Rust inference | no canonical repository selected, checked 2026-06-22 | `NOASSERTION` | `concept_only` | generic service boundary and OpenAI-compatible adapter shape |
| fortunto2/rust-code | External agent baseline | `fortunto2/rust-code` `master@e8245c0bf2fc81d9feb060314e087231e7694d14`, pushed 2026-05-16 | MIT | `code_reference_with_attribution` | agent/service/CLI architecture comparison |
| Kuberwastaken/claurst | External agent baseline | `Kuberwastaken/claurst` `main@5030334858e227232cd55766bbb84dc956dee79c`, pushed 2026-06-17 | GPL-3.0 | `concept_only` | architecture inspiration only; no copying |

## Clean-Room Rules

- Compatible repositories may be read for architecture comparison, but source
  cannot be copied, translated, mechanically ported, or pasted into Norion
  without an explicit attribution and scoped port plan.
- GPL-3.0 sources remain concept-only unless the project explicitly accepts GPL
  obligations. R95 does not accept those obligations.
- Repositories with no detected SPDX license are blocked from source-level use.
- Paper-only references may inform behavior specs and deterministic fixtures,
  but do not grant source reuse rights.
- All implementation follow-ups must use Norion-owned type names, tests,
  prompts, schemas, docs, and tool contracts.

## Chunk Repair Fixture Catalog

#64 requires deterministic fixture coverage before chunk/KV repair ideas become
implementation work. The catalog in `src/reference_backlog.rs` tracks these
preview-only fixture classes:

| Fixture | Deterministic evidence target | Repair state |
| --- | --- | --- |
| malformed chunk | `DnaGeneSchemaError` parser/gate evidence | quarantine then repair candidate |
| missing field | `SemanticIndex` missing-field parser/gate evidence | reject or backfill candidate |
| stale chunk | `MutationFixtureKind::StaleLabel` | relabel or decay candidate |
| duplicate chunk | `semantic_index_suppresses_duplicates_and_respects_token_budget` | deduplicate without delete |
| oversized chunk | token-budget retention gate | evict or summarize candidate |
| poisoned payload | `MutationFixtureKind::MaliciousInstruction` and malignant-gene drills | digest-only quarantine |

These fixtures remain preview-only: `write_allowed=false` and `applied=false`.

## Follow-Up Routing

- #63 can feed genome implementation issues #12, #13, #14, #15, #50, #51, and
  #53 only through Norion-owned acceptance criteria.
- #64 can feed memory/runtime issues #23, #39, #44, #57, and #58 only after the
  fixture catalog is wired into focused tests or trace evidence.
- #65 can feed #5, #19, #38, #52, #55, and #58 through clean-room adapter
  spikes. `candle`, `mistral.rs`, and `rust-code` require attribution and a port
  plan; `claurst` remains concept-only.

## R95 Outcome

R95 is complete as a reference and license gate when:

- this note exists and links #63, #64, and #65 to source/license decisions;
- `cargo test --package rust-norion reference_backlog` passes;
- the roadmap and default pursuit queue advance to R96 clean-room audit;
- no durable memory, genome, experiment-ledger, or external source import is
  enabled by this research gate.
