# External Agent Baselines

Clean-room audit date: 2026-06-20.

This note records external architecture signals for Norion without importing
external source code. It is a product and architecture baseline only: no files
from the audited projects are vendored, no generated prompts are reused, and
GPL-licensed material remains inspiration for independently specified work.

## Sources Checked

| Project | License posture | Baseline checked | Source links |
| --- | --- | --- | --- |
| `fortunto2/rust-code` | GitHub reports MIT | `master` at `e8245c0bf2fc81d9feb060314e087231e7694d14`; repo pushed 2026-05-16 | [repo](https://github.com/fortunto2/rust-code), [README](https://github.com/fortunto2/rust-code/blob/master/README.md), [Cargo workspace](https://github.com/fortunto2/rust-code/blob/master/Cargo.toml) |
| `Kuberwastaken/claurst` | GitHub reports GPL-3.0; `src-rust` workspace also declares GPL-3.0 | `main` at `5030334858e227232cd55766bbb84dc956dee79c`; repo pushed 2026-06-17 | [repo](https://github.com/Kuberwastaken/claurst), [Rust workspace](https://github.com/Kuberwastaken/claurst/blob/main/src-rust/Cargo.toml), [spec index](https://github.com/Kuberwastaken/claurst/blob/main/spec/INDEX.md), [tools docs](https://github.com/Kuberwastaken/claurst/blob/main/docs/tools.md), [configuration docs](https://github.com/Kuberwastaken/claurst/blob/main/docs/configuration.md) |

## Reuse Rules

- `rust-code`: MIT-compatible ideas may be reused, but any code import still
  needs attribution, dependency review, and a tiny scoped port plan. This audit
  recommends ideas only.
- `claurst`: treat as GPL-only architecture inspiration. Do not copy source,
  prompts, schemas, UI layouts, docs text, or command/tool implementations into
  Norion unless the project explicitly accepts GPL-3.0 obligations.
- For both projects, keep future work behind Norion-owned requirements,
  Norion-owned data types, and fresh implementations.

## MIT-Compatible Ideas From `rust-code`

`rust-code` is closest to Norion as a Rust terminal agent workspace. The most
useful reusable ideas are structural rather than code-level:

| Idea | Why it matters for Norion | Candidate landing zone |
| --- | --- | --- |
| Workspace split between CLI, core agent, tools, TUI, ML/search helpers, and code-intelligence server | Reinforces the current Norion direction of keeping core contracts separate from service, CLI, and UI surfaces | Existing `crates/norion-*`; future UI or code-intelligence crate |
| Typed tool registry with separate tool backends | Lets the agent loop reason over tools as contracts while adapters own side effects | `crates/norion-agent`, `crates/norion-service`, `crates/norion-cli` |
| TUI ergonomics for fuzzy file search, symbol search, task panels, session resume, and background task visibility | Useful as product baseline for operator-facing Norion controls, especially read-only status and debugging panes | `crates/norion-cli`; optional future TUI crate |
| Project guidance directory plus `AGENTS.md`/context loading | Matches Norion clean-room handoff and role guidance needs without requiring runtime memory writes | `crates/norion-agent`, `crates/norion-memory` read-only projection |
| Provider setup and doctor commands | A small preflight command can reduce failed model/runtime setup before agent work starts | `crates/norion-cli`, `crates/norion-service` readiness gates |
| MCP and OpenAPI-as-tool discovery | Good fit for adapter-based tool admission, provided every discovered tool has explicit permission and evidence gates | `crates/norion-service`, `crates/norion-agent` ports |
| Loop detection, autonomous loop caps, and self-evaluation reports | Aligns with Norion eval gates, repair-first scheduling, and evolution-loop safety | `crates/norion-agent`, `crates/norion-eval` |
| Session JSONL and local state separation | Supports resumable work while preserving a clean boundary between transcript, tasks, and memory | `crates/norion-memory`, `crates/norion-service` |

Near-term use should be limited to design comparison, not code porting. The best
first follow-up is a Norion-owned "agent tool contract matrix" that compares
existing `norion-agent`/`norion-service` ports against the operator-facing tool
families above.

## GPL-Only Inspiration From `claurst`

`claurst` should not be a source of implementation for Norion. It is still a
useful architecture reference because its Rust workspace is organized around
the same broad agent concerns: core state, provider API, tool execution,
query/turn orchestration, terminal UI, commands, MCP, plugins, bridge/remote
session support, and editor-facing integration.

Architecture ideas worth independently specifying:

| Inspiration | Norion-safe interpretation | Candidate landing zone |
| --- | --- | --- |
| Dedicated crates for core, API/providers, tools, query orchestration, TUI, commands, MCP, plugins, bridge, CLI, and ACP-like editor integration | Keep each boundary as a Norion-owned trait/data contract with no GPL names or implementation details | Workspace roadmap and crate dependency rules |
| Permission-aware tool assembly | Extend Norion side-effect gates so read, write, shell, web, MCP, monitor, and worktree operations all expose uniform admission evidence | `crates/norion-service/src/gate.rs`, `crates/norion-agent` |
| Configuration hierarchy covering project settings, provider settings, tool access, command extensions, hooks, and named agents | Define a Norion config contract that is explicit about precedence, redaction, and read-only preview mode | `crates/norion-cli`, `crates/norion-service` |
| Hook/plugin surfaces with declared capabilities | Useful product shape for extension governance, but Norion should specify a minimal capability manifest from scratch | Future plugin/extension crate or service adapter |
| Remote/bridge architecture | Good reminder to keep remote sessions, worktrees, and IDE/editor bridges out of core inference and memory crates | Future bridge crate; service adapter only |
| ACP/editor integration | Treat as a separate external protocol boundary rather than embedding editor concerns in core agent logic | Future `norion-bridge` or `norion-ide` crate |
| Tool docs organized by families | Good documentation pattern for Norion's tool admission and side-effect model | `docs/architecture/norion-service.md` or new tool contract doc |

The claurst-derived work must start from Norion requirements such as "operator
can preview tool admission without executing", "plugin capabilities are
deny-by-default", or "remote bridge cannot mutate memory directly". Those
requirements are generic and implementable without referencing GPL code.

## Norion Direction

The two baselines point in the same direction:

- Keep the workspace modular. Norion already has the right major crates; the
  next improvement is sharper public contracts between agent, service, CLI, and
  eval.
- Make side-effect boundaries first-class. Tool discovery, MCP, OpenAPI,
  background tasks, hooks, and bridge sessions all need admission summaries that
  can be observed before anything mutates files, processes, remotes, or memory.
- Treat local state as multiple stores. Sessions, task queues, project guidance,
  clean memory, and runtime KV should stay separately auditable.
- Prefer read-only projections before integration. New external-tool or
  extension ideas should begin as manifest/readiness rows before they become
  executable adapters.

## Suggested Next Patch

Create a Norion-owned tool and extension contract matrix:

- rows for file read, file edit, shell, PowerShell, search, web, MCP, OpenAPI,
  task, agent/team, monitor/background, config, hook, plugin, bridge, and
  worktree operations;
- columns for owning crate, permission inputs, side effects, read-only preview,
  compact evidence row, and repair-first behavior;
- explicit "GPL-clean implementation required" note for any row inspired by
  `claurst`.

That matrix can absorb the useful `rust-code` product ideas while keeping
`claurst` safely at the architecture-inspiration level.
