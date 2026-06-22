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
- whether the repository license will remain non-commercial or transition to a
  permissive core license;
- which benchmark numbers, if any, are ready to publish;
- whether the report should be submitted as architecture, systems, or AI
  infrastructure work;
- final title, abstract, and references.

Local compile command when a TeX toolchain is installed:

```bash
pdflatex bio-inspired-inference-control-report.tex
```

The draft is intentionally self-contained and does not require a separate
BibTeX file.
