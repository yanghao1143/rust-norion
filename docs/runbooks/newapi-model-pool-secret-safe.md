# NewAPI / OpenAI-compatible Model Pool Secret-safe Runbook

This runbook defines the minimum safe configuration for using a NewAPI or
OpenAI-compatible model pool with Norion. It is intentionally secret-free:
commit only variable names, example placeholders, and policy. Never commit a
real API key, provider token, account ID, invoice URL, or bearer token.

## Runtime Environment

Use process environment variables, a local untracked `.env`, or a deployment
secret manager. Do not put secrets in tracked TOML, JSON, Markdown, shell
history, CI logs, screenshots, or issue comments.

Required variables:

| Variable | Required | Meaning |
| --- | --- | --- |
| `NORION_NEWAPI_BASE_URL` | yes | HTTPS base URL of the NewAPI/OpenAI-compatible gateway, without a trailing `/v1` unless the gateway explicitly requires it. |
| `NORION_NEWAPI_API_KEY` | yes | Bearer credential for the gateway. Store only in local env or a secret manager. |
| `NORION_NEWAPI_ALLOWED_MODELS` | yes | Comma-separated allowlist of model IDs that Norion may route to. |

Use the companion example file:

```powershell
Copy-Item docs\runbooks\newapi-model-pool.env.example .env
notepad .env
```

The copied `.env` is ignored by git. Replace the placeholder key locally only.
Keep `docs/runbooks/newapi-model-pool.env.example` tracked and placeholder-only.

## Example Values

```env
NORION_NEWAPI_BASE_URL=https://newapi.example.invalid
# Set NORION_NEWAPI_API_KEY only in local untracked env or a secret manager.
NORION_NEWAPI_ALLOWED_MODELS=qwen/qwen3-next-80b-a3b-instruct,qwen/qwen3.5-397b-a17b
```

`NORION_NEWAPI_ALLOWED_MODELS` is an allowlist, not a routing hint. If a model
is not listed exactly, the router must treat it as unavailable even when the
provider exposes it in `/v1/models`.

Current recommended starting allowlist:

| Model ID | Role | Notes |
| --- | --- | --- |
| `qwen/qwen3-next-80b-a3b-instruct` | fast/helper | Candidate for helper, summary, review, and other lower-latency routes. |
| `qwen/qwen3.5-397b-a17b` | coding/heavy worker | Recommended candidate for harder coding, architecture, and synthesis routes where latency is acceptable. A main-thread coding probe passed a Rust small-function prompt in about 6274 ms with clean code output: `pub fn add_one(x: i32) -> i32 { x + 1 }`. |

`moonshotai/kimi-k2.6` is not a default model at this time because current
operator evidence shows duplicate-output anomalies. Keep it out of
`NORION_NEWAPI_ALLOWED_MODELS` unless a later verification run clears that
specific issue.

Prefer stable provider model IDs over display names. Do not use wildcard values
such as `*`, `all`, `latest`, or `default`; they make later provider changes
silently expand production access.

## GPT-5 Family Disable Rule

GPT-5-family models are disabled for this pool unless a future security and
cost review explicitly changes this runbook.

The deny rule applies before the allowlist. Operators must reject model IDs
whose normalized lowercase ID is exactly `gpt-5` or starts with one of these
prefixes:

- `gpt-5-`
- `openai/gpt-5`
- `openai:gpt-5`
- `chatgpt-5`

This means `NORION_NEWAPI_ALLOWED_MODELS=gpt-5` is still invalid. The model
must remain blocked even if the upstream NewAPI gateway advertises it, aliases
it, or makes it the provider default.

Do not route by provider default model when using NewAPI. Always send an
explicit allowed model ID for each request.

## Local Verification

Before launching any writer, evolution loop, or unattended agent, verify:

1. `NORION_NEWAPI_BASE_URL` points to the intended gateway and uses `https://`
   outside local test networks.
2. `NORION_NEWAPI_API_KEY` is present in the process environment but absent
   from `git diff`, `git status --short`, terminal logs, and screenshots.
3. `NORION_NEWAPI_ALLOWED_MODELS` contains only explicit, reviewed model IDs.
4. No listed model matches the GPT-5-family deny rule above.
5. A read-only `/v1/models` probe succeeds without printing the API key.

PowerShell spot checks:

```powershell
git status --short
git diff -- . ':!README.md' ':!LICENSE'
$pattern = "NORION_NEWAPI_API_" + "KEY="
git grep -n $pattern -- ':!docs/runbooks/newapi-model-pool.env.example'
```

The last command should return no tracked secret assignment. It is acceptable
for docs and examples to mention the variable name without a real value.

## Rotating a Leaked Key

Treat a key as leaked if it appears in a commit, chat transcript, issue, CI log,
terminal capture, crash dump, screenshot, pasted `.env`, or remote sync mirror.

Rotation sequence:

1. Revoke or disable the exposed key in NewAPI immediately.
2. Create a replacement key with the minimum scopes and quotas needed for the
   model pool.
3. Update the deployment secret store or local `.env` with the replacement.
4. Restart only the processes that need the key; do not print the value during
   restart.
5. Search tracked files, logs, and CI artifacts for the old key fingerprint.
6. If the key reached git history, assume GitHub/Gitee mirrors and forks have
   copied it. Rotate first, then rewrite history only as cleanup.
7. Add an incident note that records the key owner, revocation time, affected
   environments, and follow-up guardrail. Do not record the full key.

Never "fix" a leaked committed key by only deleting it in a later commit. The
old commit remains enough for compromise.

## GitHub and Gitee Sync Hygiene

GitHub/Gitee synchronization increases blast radius because a secret can be
replicated into mirrors, pull request refs, CI caches, and fork networks.

Before pushing to either remote:

- Keep real `.env` files untracked. This repo ignores `.env` and `.env.*` while
  allowing checked-in `*.env.example` files.
- Review staged changes with `git diff --cached`.
- Do not paste real keys into pull request descriptions, issue templates,
  screenshots, or release notes.
- Configure CI secrets separately per platform; do not sync secret values
  through repository files.
- Mask `NORION_NEWAPI_API_KEY` in GitHub Actions, Gitee Go, and any external
  runner logs.
- Avoid mirroring private CI artifacts that may contain process environments.
- When testing sync automation, use sentinel values such as
  `replace-with-secret-from-vault`, never real or production-like tokens.

If a push is rejected by secret scanning, stop and rotate the exposed key before
attempting another push. Do not bypass scanning to "unblock" documentation work.

## Ownership

Changes to the model allowlist, gateway URL, or GPT-5-family deny rule should be
reviewed by both an operator and a code owner for the model router. A docs-only
change may update examples and rotation procedure, but it must not weaken the
deny rule or introduce real credentials.
