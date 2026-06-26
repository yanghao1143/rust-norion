# Submission Package: Reasoning Genome Chain Technical Report v0.1

This file collects the metadata needed to publish the rust-norion technical
report on GitHub/Zenodo, OSF Preprints, and TechRxiv.

## Important Account Boundary

The report can be prepared locally, but final submission requires the repository
owner or author to log in and confirm platform terms:

- Zenodo requires GitHub authorization and enabling the repository integration.
- OSF Preprints requires an OSF account and author metadata.
- TechRxiv requires an account, author metadata, and moderation screening.

Do not paste API tokens, platform passwords, or private author identifiers into
chat. Fill those fields directly in the target platform UI.

## Files

- Main paper Markdown:
  `docs/papers/reasoning-genome-chain-technical-report-v0.1.md`
- Chinese summary:
  `docs/papers/reasoning-genome-chain-technical-report-v0.1.zh.md`
- PDF output, if generated:
  `docs/papers/reasoning-genome-chain-technical-report-v0.1.pdf`
- HTML output, if generated:
  `docs/papers/reasoning-genome-chain-technical-report-v0.1.html`
- Citation metadata:
  `CITATION.cff`
- Zenodo metadata:
  `.zenodo.json`

## Title

Reasoning Genome Chain: A DNA-Inspired Control Layer for Auditable Self-Evolving AI Inference

## Short Title

Reasoning Genome Chain

## Authors

Draft author line:

Yang Hao (杨浩)

Before submission, add affiliation, ORCID id if available, and contact email in
the target platform UI.

## Abstract

Large language model applications increasingly depend on control logic outside
the model weights: memory retrieval, routing, tool use, reflection, evaluation,
rollback, and operator approval. In many prototypes this control logic is
implicit, scattered across prompts, scripts, logs, and ad hoc agent state. This
technical report introduces the Reasoning Genome Chain, a DNA-inspired
software-control abstraction implemented in the open-source rust-norion
prototype. The approach does not claim biological simulation and does not
retrain model weights. Instead, it represents reusable reasoning behavior as
bounded, typed, auditable strategy records called reasoning genes. A task
profile selects an express chain that can influence runtime routing, retrieval,
reflection, budget posture, tool dispatch, and validation gates, while a
separate memory chain preserves provenance, fitness evidence, rejection
reasons, rollback anchors, and privacy-safe digests. Gene Scissors provides a
guarded mutation pipeline for relabel, cut, splice, quarantine, repair,
crossover, rollback, and regenerate operations. Durable mutation is denied by
default and must pass trace, test, benchmark, drift, privacy, license, rollback,
and operator-approval gates before admission. The contribution is a concrete
engineering frame for building auditable self-evolving inference control layers
in Rust, with explicit separation between runtime expression, append-only
evidence, and write authorization.

## Keywords

Reasoning Genome Chain; Rust; AI inference control layer; self-evolving
systems; agent memory; rollback; evidence gates; auditable AI; Gene Scissors;
runtime governance

## Categories

Suggested OSF/TechRxiv categories:

- Computer Science
- Artificial Intelligence
- Software Engineering
- Machine Learning Systems
- Human-centered / auditable AI governance, if available

## Repository Links

- GitHub: https://github.com/yanghao1143/rust-norion
- Gitee: https://gitee.com/babalibaba/rust-norion
- Contributor Zone:
  https://github.com/yanghao1143/rust-norion/blob/main/docs/contributor-zone.md
- Architecture note:
  https://github.com/yanghao1143/rust-norion/blob/main/docs/architecture/reasoning-genome-chain.md

## License

Repository license: GPL-3.0, via `LICENSE`.

For preprint platforms, select the license that matches your intended paper
reuse policy. If unsure, use a conservative all-rights-reserved or platform
default setting for the manuscript, while keeping source code under the
repository license. Do not change source-code license without maintainer
approval.

## GitHub + Zenodo Route

Official GitHub documentation states that Zenodo can archive a repository and
issue a DOI for each GitHub release after the repository owner authorizes Zenodo
and toggles the repository integration on. The practical path is:

1. Merge the report files to the default branch.
2. Log in to Zenodo with GitHub.
3. Enable `yanghao1143/rust-norion` in the Zenodo GitHub integration.
4. Create a GitHub release, for example `paper-rgc-v0.1`.
5. Wait for Zenodo to ingest the release.
6. Edit the Zenodo metadata if needed and record the DOI back in the repository.

Do not create a DOI before author metadata and release contents are ready.

## OSF Preprints Route

OSF Preprints supports uploading a preprint file, metadata, and supplemental
materials. OSF documentation states that preprints receive a DOI and persistent
URL. Recommended upload package:

- PDF manuscript;
- source Markdown;
- repository link;
- optional supplemental archive with diagrams, validation notes, and release
  metadata.

Suggested OSF description:

This preprint describes the Reasoning Genome Chain, a DNA-inspired software
control-layer abstraction implemented in the open-source rust-norion prototype.
It frames self-evolving AI inference as a preview-first, evidence-gated Rust
engineering problem rather than a direct model-weight mutation problem.

## TechRxiv Route

TechRxiv describes itself as an open, moderated preprint server for unpublished
research in engineering, computer science, and related technology. Recommended
upload package:

- PDF manuscript;
- author metadata;
- keywords and category;
- repository link;
- statement that the manuscript is an unpublished technical report and not
  peer reviewed.

Suggested TechRxiv description:

This technical report introduces a Rust implementation frame for auditable
self-evolving inference control. The proposed Reasoning Genome Chain separates
runtime strategy expression from append-only provenance and forces mutation
through trace, test, benchmark, rollback, privacy, and operator-approval gates.

## Suggested GitHub Release Notes

Title:

Reasoning Genome Chain Technical Report v0.1

Body:

This release publishes the first citable technical report for rust-norion:
**Reasoning Genome Chain: A DNA-Inspired Control Layer for Auditable
Self-Evolving AI Inference**.

The report introduces:

- reasoning genes as bounded strategy records;
- express_chain / memory_chain dual-chain architecture;
- Gene Scissors mutation intents;
- preview-first safety and rollback gates;
- current prototype surfaces and validation strategy;
- an open research roadmap for auditable Rust AI control layers.

Primary files:

- `docs/papers/reasoning-genome-chain-technical-report-v0.1.md`
- `docs/papers/reasoning-genome-chain-technical-report-v0.1.pdf`
- `docs/papers/reasoning-genome-chain-technical-report-v0.1.zh.md`
- `CITATION.cff`
- `.zenodo.json`

## Post-Publication Checklist

- Add DOI to `CITATION.cff`.
- Add DOI badge or citation block to the README after maintainer approval.
- Update `docs/runbooks/community-outreach.md` with the DOI and preprint URL.
- Submit the paper-backed artifact to paper-only AI memory, agent evolution,
  LLM systems, and AI engineering lists that were previously deferred.
- Prepare a follow-up benchmark report with ablation and reproduction scripts.
