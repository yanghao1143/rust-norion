# Coding Service Eval Readiness

R97 begins the English/Chinese/Rust coding service and evaluation work for
issues #19 and #29. This baseline bridges the completed #75 coding evaluation
corpus into deterministic service request plans and an offline mock runner
without requiring a live model, network access, or model-cache warmup in CI.

The executable companion is `src/coding_service_eval.rs`, which exposes
`coding_service_eval_v1` and trace schema
`rust-norion-coding-service-eval-readiness-v1`. The runner companion exposes
`coding_service_eval_runner_v1`.

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
- The offline runner executes each plan through `norion-service::ChatSession`,
  emits deterministic streaming/status/metadata/final/done chunks, probes
  cancellation where required, and converts the mock output into
  `CodingEvalObservation` values for scoring.
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

## Offline Runner

`default_coding_service_eval_runner_report()` runs the same #75-derived request
plans through a deterministic offline service path. It checks:

- stream lifecycle coverage: start, delta, status, metadata, final, and done;
- cancellation probes for Rust repair and multilingual explanation lanes;
- diagnostics, health, and model-capability visibility for every plan;
- max-token budget compliance;
- Rust validation readiness for the Rust generation and repair lanes;
- digest-only run evidence and scored `CodingEvalSuiteReport` output.

The runner is intentionally a CI-safe mock. It proves the local service/eval
contract before a real model endpoint, HTTP transport, or benchmark runner is
attached.

## CI Role

`default_coding_service_eval_readiness_report()` and
`default_coding_service_eval_runner_report()` can run in CI with the mock sample
observations from the #75 corpus. Together they check corpus validity,
request-plan coverage, capability coverage, stream/cancel execution coverage,
suite pass rate, profile coverage, redaction, and read-only flags.

The next R97 slices can now wire these reports into:

- stricter local service request/response contracts for #19;
- endpoint or CLI execution surfaces for #19;
- benchmark gate feed and runner artifact serialization for #29;
- future benchmark gates without enabling automatic memory or genome mutation.
