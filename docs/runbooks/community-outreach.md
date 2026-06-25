# Community Outreach Runbook

This runbook keeps rust-norion outreach useful instead of noisy. The goal is to
find contributors through relevant Rust/AI communities, not to bulk-post the
same recruitment message everywhere.

## Rules

- Do not post identical messages to many repositories or forums.
- Do not open issues in unrelated projects just to advertise rust-norion.
- Use project-specific categories, weekly/news-tip forms, resource-list PRs, or
  community discussions where self-submission is allowed.
- Tailor each submission to the audience and record the result in
  `docs/outreach/community-registry.yml`.
- Prefer monthly update batches unless there is a real release, article, or
  contributor-facing milestone.
- Keep claims modest: rust-norion is a research-engineering prototype, not a
  production LLM inference kernel.

## Current High-Value Channels

| Channel | Status | Next action |
| --- | --- | --- |
| RustCn weekly | Submitted | Track https://github.com/rustcn-org/rust-weekly/issues/11 |
| users.rust-lang.org | Draft ready | Post manually after account/category check |
| RustCC forum | Draft ready | Post manually after GitHub OAuth login |
| CSDN | Draft ready | Publish long Chinese contributor article |
| awesome-rust-llm | Submitted | Track https://github.com/jondot/awesome-rust-llm/issues/17 |
| Rust AI Libraries | Submitted | Track https://github.com/ever-works/awesome-rust-ai-libraries/issues/3 |
| Awesome Rust Machine Learning | Submitted | Track https://github.com/vaaaaanquish/Awesome-Rust-MachineLearning/issues/30 |
| Awesome Rust AI Governance | Submitted | Track https://github.com/mindfulcto-labs/awesome-rust-ai-governance/issues/1 |
| Awesome AI Agent Governance | Submitted | Track https://github.com/agentrust-io/awesome-ai-governance/issues/28 |
| Awesome AI Agents Security | Submitted | Track https://github.com/ProjectRecon/awesome-ai-agents-security/pull/61 |
| Awesome Agentic AI Security | Submitted | Track https://github.com/natnew/Awesome-Agentic-AI-Security/issues/12 |
| InftyAI Awesome LLMOps | Submitted | Track https://github.com/InftyAI/Awesome-LLMOps/issues/469 |
| Slava Awesome AI Agents | Submitted | Track https://github.com/slavakurilyak/awesome-ai-agents/issues/319 |
| Jim Awesome AI Agents | Submitted | Track https://github.com/jim-schwoebel/awesome_ai_agents/issues/359 |
| Jenqyang Awesome AI Agents | Submitted | Track https://github.com/Jenqyang/Awesome-AI-Agents/pull/333 |
| Awesome AI Agents 2026 | Submitted | Track https://github.com/caramaschiHG/awesome-ai-agents-2026/pull/376 |
| ARUN Awesome AI Agents 2026 | Submitted | Track https://github.com/ARUNAGIRINATHAN-K/awesome-ai-agents-2026/pull/123 |
| Alternbits Awesome AI Agents | Submitted | Track https://github.com/alternbits/awesome-ai-agents/pull/57 |
| ChatTeach Awesome AI Agents | Submitted | Track https://github.com/ChatTeach/Awesome-AI-Agents/pull/25 |
| Zijian Awesome AI Agents 2026 | Deferred | Avoid current self-promotional blast guard; revisit after traction |
| E2B Awesome AI Agents | Deferred | Main list points frameworks/tools to Awesome AI SDKs; PR already filed there |
| AgenticHealthAI Healthcare Agents | Deferred | Healthcare-specific; submit only with healthcare evidence |
| DirectorySurf AI Agent Directories | Deferred | Directory-of-directories, not individual project listing |
| Junhua Awesome LLM Agents | Submitted | Track https://github.com/junhua/awesome-llm-agents/issues/11 |
| Awesome Agentic Patterns | Deferred | Only submit a neutral reusable pattern, not a project ad |
| Awesome CLAWS | Deferred | OpenClaw-inspired projects only; do not submit rust-norion as-is |
| Awesome AI Apps | Deferred | Submit only if rust-norion has a complete app/demo example |
| MB-MAL Awesome AI Agents Frameworks | Deferred | No clear contribution source; avoid direct generated README edits |
| Awesome Rust AI Tools | Submitted | Track https://github.com/KatanoShingo/awesome-rust-ai-tools/issues/1 |
| Awesome Rust AI | Submitted | Track https://github.com/dhilipsiva/awesome-rust-ai/issues/1 |
| Awesome LLM Agents | Submitted | Track https://github.com/kaushikb11/awesome-llm-agents/issues/266 |
| Best of ML Rust | Submitted | Track https://github.com/e-tornike/best-of-ml-rust/issues/143 |
| Awesome Agentic AI ZH | Submitted | Track https://github.com/WenyuChiou/awesome-agentic-ai-zh/issues/44 |
| E2B Awesome AI SDKs | Submitted | Track https://github.com/e2b-dev/awesome-ai-sdks/pull/257 |
| TensorChord Awesome LLMOps | Submitted | Track https://github.com/tensorchord/Awesome-LLMOps/pull/603 |
| Brandon Himpfen Awesome LLMOps | Submitted | Track https://github.com/brandonhimpfen/awesome-llmops/pull/29 |
| Kenneth Awesome LLMOps | Submitted | Track https://github.com/KennethanCeyer/awesome-llmops/pull/21 |
| Gitee Rust Boom | Candidate | Verify contribution flow after Gitee login |
| Gitee Awesome LLM | Candidate | Verify tool/deployment category fit before posting |
| This Week in Rust | Wait | Submit only after a notable release/article |

The full registry is in `docs/outreach/community-registry.yml`.

## Monthly Update Workflow

1. Collect concrete changes since the last outreach cycle:
   - merged PRs
   - new release notes
   - new docs/runbooks
   - benchmark or trace evidence
   - open contributor tasks
2. Pick at most 3-5 relevant communities from the registry.
3. Validate the registry:

   ```powershell
   ./tools/outreach/validate-community-registry.ps1
   ```

4. Generate a local draft:

   ```powershell
   ./tools/outreach/generate-community-update.ps1 -SinceDays 30
   ```

5. Generate the next outreach queue:

   ```powershell
   ./tools/outreach/generate-community-queue.ps1
   ```

6. Discover additional candidate communities if needed:

   ```powershell
   ./tools/outreach/discover-community-candidates.ps1 -LimitPerQuery 20 -MinStars 50
   ```

  Add `-IncludeGitee` only when Gitee API access is working; otherwise inspect
  Gitee manually in a browser.

  Treat Gitee mirrors as discovery leads, not posting targets. Prefer the
  upstream GitHub project when the Gitee page is only a mirror, and post to
  Gitee only after login reveals an active issue/PR/comment path.

7. Fill or trim the matching template in `docs/outreach/templates/`.
8. Manually submit each update according to that community's rules.
9. Add the proof URL and date to the registry.
10. Do not repeat outreach to the same community until there is a real update.

## What Counts As A Real Update

- A release, merged milestone, or public demo.
- A new contributor-facing issue batch.
- A substantial runbook or architecture article.
- A new runtime adapter boundary, memory gate, benchmark, or trace schema.
- A project status update with validation evidence.

## What Does Not Count

- "Please star the project."
- Reposting the same contributor request.
- Cosmetic README edits without a contributor-facing reason.
- Unverified claims about model performance.

## Suggested Categories

- Rust AI / LLM tools
- Rust machine learning workflow
- Agent infrastructure
- Local-first AI systems
- Runtime adapter and inference gateway
- Memory / retrieval / KV control
- Open-source contributor onboarding

## Submission Checklist

- [ ] The community is listed in the registry or was verified manually.
- [ ] The post explains why rust-norion is relevant to that audience.
- [ ] The post includes a concrete update or contribution lane.
- [ ] The post does not overclaim maturity.
- [ ] The post includes validation evidence when technical claims are made.
- [ ] The proof URL is recorded after submission.

## Automation Boundary

The scheduled workflow `.github/workflows/community-outreach-reminder.yml`
opens a monthly GitHub issue and embeds a generated draft plus candidate
discovery output. It deliberately does not publish to external communities.
External submission still requires manual review, login context, category
selection, and community-specific wording.

The validation workflow `.github/workflows/community-outreach-validation.yml`
runs on outreach asset changes. It validates the registry, parses the
PowerShell tooling, generates a sample update, and runs candidate discovery so
broken templates or scripts are caught before a monthly outreach cycle.
