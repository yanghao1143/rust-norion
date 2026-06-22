# Research Reports

This directory stores draft research-facing reports for rust-norion.

## Bio-Inspired Inference Control Report

Draft:
[`bio-inspired-inference-control-report.tex`](bio-inspired-inference-control-report.tex)

Working title:

> Bio-Inspired Inference Control for Local Large Language Models: A DNA
> Gene-Chain Architecture in Rust

Purpose:

- explain rust-norion as a DNA-inspired inference control-layer engine;
- attach the public project code URL;
- describe the current prototype honestly as control-plane work, not a
  production high-throughput LLM runtime;
- create a base manuscript for arXiv-style technical exposure.

Before external submission, confirm:

- final author list, affiliations, and contact email;
- whether the paper reuse statement matches the repository's GPL-3.0 license;
- which benchmark numbers, if any, are ready to publish;
- whether the report should be submitted as architecture, systems, or AI
  infrastructure work;
- final title, abstract, and references.

Current arXiv submission metadata:

- account: `asd8841315`
- author name: Hao Yang YangHao
- email: `2499510083@qq.com`
- affiliation: Independent Researcher
- URL: <https://claude.chiddns.com>
- default category: `cs.AI`
- group: `cs`
- country: People's Republic of China
- career status: Staff

Submission checklist:

- Upload TeX source, not only a PDF produced from TeX.
- Prepare a clean upload folder containing only the files intended for the
  archival arXiv submission.
- Remove private notes, unused drafts, local paths, secrets, and unwanted TeX
  comments before upload; announced arXiv content is archival.
- Double-check missing files, missing references, figures, and generated
  artifacts before final submit.
- Create a stable repository tag or commit snapshot and cite it in the report
  before external submission.
- Keep the repository license statement and the paper's reuse statement in
  sync.

Local compile command when a TeX toolchain is installed:

```bash
pdflatex bio-inspired-inference-control-report.tex
```

The draft is intentionally self-contained and does not require a separate
BibTeX file.
