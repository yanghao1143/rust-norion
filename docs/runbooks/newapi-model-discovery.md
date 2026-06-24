# NewAPI Model Discovery Runbook

This runbook records the repeatable NewAPI discovery flow for the
evolution-loop/model-pool candidate matrix. It does not require changes to the
main README.

## Inputs

Set credentials only through environment variables:

```powershell
$env:NORION_NEWAPI_BASE_URL = "https://newapi.example.test"
# Set NORION_NEWAPI_API_KEY in the current shell or secret manager before running.
```

Do not put API keys in files, command history examples, logs, commits, or
artifacts. The discovery script writes only the environment variable names,
model ids, tiering decisions, probe status, and failure reasons.

## Local policy self-test

Run this before touching a live NewAPI endpoint:

```powershell
.\tools\evolution-loop\test-newapi-model-discovery.cmd -RepoRoot D:\rust-norion
```

The self-test uses an embedded fixture, does not touch the network, does not
send prompts, and verifies that GPT-5/GPT-5-or-newer ids are excluded.

## Discovery only

```powershell
.\tools\evolution-loop\discover-newapi-models.cmd `
  -OutputJson target\evolution\newapi-model-discovery.json `
  -MatrixMarkdown target\evolution\newapi-model-matrix.md
```

This calls `/v1/models`, excludes forbidden GPT-5-or-newer ids before probing,
and writes:

- `target\evolution\newapi-model-discovery.json`
- `target\evolution\newapi-model-matrix.md`

## Discovery with lightweight probes

```powershell
.\tools\evolution-loop\discover-newapi-models.cmd `
  -Probe `
  -MaxProbeModels 12 `
  -ProbeTimeoutSecs 45 `
  -ProbeMaxTokens 24
```

Probe mode calls `/v1/chat/completions` with a tiny health prompt for the first
allowed models. Models that fail the probe are moved to `failed_models` with
`probe_failed` and the provider error text. The API key is used only in the
Authorization header and is never written to artifacts.

## Tier meanings

`fast_router_reviewer` is for small, mini, flash, haiku, router, review, or
low-parameter models suited to routing, summaries, and first-pass review.

`heavy_reasoning` is for reasoning, R-series, o1/o3/o4, large, and high-parameter
models suited to expensive synthesis or hard judgment.

`coding` is for coder/code/devstral/codestral-style models and coding-specialized
variants.

`fallback` is for allowed general models that do not match the specialized
patterns above.

The policy layer always wins over tiering: GPT-5, GPT-5.3, GPT-5.4, GPT-6, or
other GPT major-version 5-or-newer ids are excluded even if they would otherwise
look useful.

## Current Manual Evidence

As of 2026-06-24, the main thread observed these NewAPI model facts:

- `qwen/qwen3.5-397b-a17b` exists in `/v1/models` and short chat-completion is
  usable. An exact-ok probe returned anomalous tool text in about 2904 ms, but
  an evolution-loop three-line contract prompt succeeded in about 15879 ms with
  `risk/change_request/verification` output. A Rust coding probe also succeeded
  in about 6274 ms with clean `add_one` code. Treat it as heavy reasoning and
  coding capable, not a default fast router.
- `qwen/qwen3-next-80b-a3b-instruct` completed the same contract prompt in about
  3417 ms and was stable in the observed run. Treat it as a strong heavy
  reasoning candidate.
- `moonshotai/kimi-k2.6` took about 40717 ms in this run and repeated
  `Destruct`; downgrade it and mark it unstable until a later probe provides
  cleaner evidence.
