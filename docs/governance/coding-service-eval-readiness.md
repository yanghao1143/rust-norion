# Coding Service Eval Readiness

R97 begins the English/Chinese/Rust coding service and evaluation work for
issues #19 and #29. This baseline bridges the completed #75 coding evaluation
corpus into deterministic service request plans without requiring a live model,
network access, or model-cache warmup in CI.

The executable companion is `src/coding_service_eval.rs`, which exposes
`coding_service_eval_v1` and trace schema
`rust-norion-coding-service-eval-readiness-v1`.

## What This Baseline Proves

- Every #75 coding profile produces a `norion-service::ChatRequest`.
- English, Chinese, Rust, and mixed English/Chinese coding requests are all
  represented.
- Requests preserve task intent through model role, routing preference,
  endpoint selection, streaming mode, max-token budget, and output mode.
- The readiness gate covers OpenAI-style chat request serialization, streaming,
  cancellation probes, max-token handling, diagnostics, health, model
  capability reporting, offline/mock backend mode, evaluation evidence, and
  Rust validation.
- Evidence packets are digest-only and suitable for PR/issue comments.
- The gate is read-only: it does not call a live model, mutate memory, mutate
  genome state, write experiment ledgers, or apply self-evolution changes.

## Request Mapping

| Profile | Language lane | Service route intent | Max tokens | Extra gate |
| --- | --- | --- | --- | --- |
| English instruction | English | assistant / balanced / auto endpoint | 900 | streaming, diagnostics, health |
| Chinese instruction | Chinese | assistant / balanced / auto endpoint | 900 | streaming, diagnostics, health |
| Rust code generation | Rust | tester / prefer quality / summary tester | 1600 | Rust validation and model capabilities |
| Rust repair | Rust | tester / prefer quality / summary tester | 1800 | Rust validation and cancellation probe |
| Multilingual coding explanation | Mixed English/Chinese | reviewer / prefer quality / auto endpoint | 1200 | cancellation probe |

The mapping is intentionally deterministic so #19 service contract work and #29
evaluation harness work can share the same fixtures.

## Evidence Packet

Each `CodingServiceEvalRequestPlan` emits:

- schema version;
- fixture id and profile;
- language lane;
- prompt digest;
- model role and routing preference;
- endpoint kind and endpoint label;
- max-token budget;
- streaming/cancel/diagnostics/health/model-capability/offline/Rust-validation
  flags;
- sorted capability labels;
- digest-only provenance.

Raw fixture prompts and raw model outputs are not serialized into request
evidence. The underlying #75 `CodingEvalSuiteReport` also keeps prompt/output
evidence digest-only.

## CI Role

`default_coding_service_eval_readiness_report()` can run in CI with the mock
sample observations from the #75 corpus. It checks corpus validity, request-plan
coverage, capability coverage, suite pass rate, profile coverage, redaction, and
read-only flags.

The next R97 slices can now wire this readiness report into:

- stricter local service request/response contracts for #19;
- mock backend and streaming/cancellation tests for #19;
- offline evaluation runner and evidence serialization for #29;
- future benchmark gates without enabling automatic memory or genome mutation.
