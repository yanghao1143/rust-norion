# Pre-Reasoning Genome ISA

Status: issue #375 design note.

## Scope

| Item | Boundary |
| --- | --- |
| Goal | Define a small Genome ISA layer before model generation. |
| Default mode | read-only, digest-only, no-side-effect. |
| Environment | computer state is environment stimulus and facts, not direct control. |
| Output | `ReasoningFrame` consumed by scheduler, model reasoning, and gates. |
| Durable writes | denied. |
| Out of scope | shell, browser, network, file write, memory write, process launch, issue/PR creation, genome mutation, model-weight change. |
| Implementation | No implementation is required by issue #375. |

## Records

| Record | Definition | Emits | Forbidden |
| --- | --- | --- | --- |
| `PreReasoningGenomeIsa` | Finite instruction-set profile for projecting genome records before subjective reasoning. | opcode list, version, side-effect policy, frame schema | prompt chains, tool scripts, adapter behavior |
| `GenomeOpcode` | Bounded operation name with typed inputs and outputs. | frame fields or validation requirements | free-form hidden reasoning |
| `ExpressionVM` | Read-only interpreter for genome expression. | `ReasoningFrame`, digest evidence, rejection reasons | durable genome/memory/tool/process writes |
| `ReasoningFrame` | Phenotype boundary passed downstream before generation. | allowed observations, action vocabulary, suppression, budgets, evidence and validation gates | direct action execution |

## Boundary

| Layer | Owns | Input | Output | Side effect |
| --- | --- | --- | --- | --- |
| DNA expression | `PreReasoningGenomeIsa`, `GenomeOpcode`, `ExpressionVM` | `GeneSegment`, `GenomeExpression`, `TaskExpressionGene`, environment stimulus | `ReasoningFrame` | no-side-effect |
| Model reasoning | subjective generation and synthesis | `ReasoningFrame`, prompt, retrieved context | answer draft, reasoning plan summary | no durable write by default |
| Tool/action gates | shell/browser/network/process/file permission checks | requested action, `ReasoningFrame`, policy | allow, hold, reject, quarantine | gate only |
| Writer gates | memory/genome/filesystem/issue/PR write preflight | proposed write, evidence, rollback anchor | manual-review or apply denial | no bypass |
| Computer-use adapters | mouse, keyboard, browser, shell, desktop bridges | gate-approved action request | adapter result | downstream only |

## ReasoningFrame Fields

| Field | Meaning |
| --- | --- |
| `environment_signals` | Repo state, issue state, terminal state, runtime health, logs, screenshots, user constraints. |
| `allowed_observations` | Signal families that may be inspected next. |
| `action_vocab` | Observe, inspect, compare, summarize, propose, simulate, gate, verify, quarantine, rollback. |
| `suppressed_capabilities` | Durable writes, process launch, browser control, network calls, memory mutation, genome mutation. |
| `routing_bias` | Model, tool, or sub-agent routing posture. |
| `memory_policy` | Memory tiers allowed for read-only recall. |
| `risk_limits` | Budget, scope, privacy, provenance, approval limits. |
| `evidence_requirements` | Minimum facts before final answer or downstream gate request. |
| `validation_requirements` | Tests, trace fields, issue evidence, benchmark gates, or human approval required later. |
| `mutation_preview` | Read-only cut, splice, quarantine, or repair proposal. |

## Opcode Table

| Opcode | Semantics | Inputs | Outputs | Side effect |
| --- | --- | --- | --- | --- |
| `BIND_STIMULUS` | Bind current machine/user facts as stimulus. | observed facts, user constraints | `environment_signals` | none |
| `LOAD_GENE` | Select eligible gene records. | `GeneSegment`, `GenomeExpression` | selected gene ids | none |
| `MATCH_ENV` | Match genes to current task environment. | selected genes, signals | match scores | none |
| `EXPRESS_TRAIT` | Project a gene into frame posture. | gene match | routing, memory, validation hints | none |
| `SET_BUDGET` | Bound tokens, steps, calls, and lanes. | policy, task profile | `risk_limits` | none |
| `SELECT_MEMORY` | Choose read-only memory tiers. | memory scope, task profile | `memory_policy` | none |
| `PACK_CONTEXT` | Define context packing requirements. | signals, memory policy | context requirements | none |
| `FOCUS_SIGNAL` | Prefer important signal families. | match scores | observation focus | none |
| `MASK_SIGNAL` | Suppress unsafe or irrelevant signals. | risk rules | suppressed observations | none |
| `DECLARE_ACTION_VOCAB` | Bound primitive actions before generation. | task profile, gates | `action_vocab` | none |
| `SUPPRESS_CAPABILITY` | Disable capabilities before reasoning. | policy, risk limits | `suppressed_capabilities` | none |
| `REQUIRE_EVIDENCE` | Name facts required before final/gate. | task issue, policy | `evidence_requirements` | none |
| `DECLARE_GATE` | Bind downstream gate owners. | requested capability | gate requirements | none |
| `PREVIEW_MUTATION` | Emit read-only mutation candidates. | gene health, evidence | `mutation_preview` | none |
| `EMIT_FRAME` | Produce final `ReasoningFrame`. | accumulated fields | frame digest | none |

## Existing Surface Mapping

| Existing surface | Reuse |
| --- | --- |
| `GenomeExpression` | Already projects profile genes into runtime routing, retrieval, reflection, budget, and validation hints. |
| `TaskExpressionGene` | Already carries required capabilities, tool policy, memory scope, budget, validation gate, read-only, and write-allowed status. |
| `ThinkingScheduler` | Already orders pre-generation phases and includes `GenomeExpression` before route selection, answer synthesis, verification, and reflection. |
| `docs/architecture/reasoning-genome-chain.md` | Existing express-chain and memory-chain boundary. |
| #243 | Related control-expression gates for routing, context, safety, checkpoints, and memory; not a duplicate ISA. |
| #304 | Related mobile-gene movement guard for reusable control records; not a duplicate pre-reasoning frame. |

## Required Downstream Gates

| Capability | Gate |
| --- | --- |
| shell | tool/action gate plus operator approval if write/process risk exists |
| browser | tool/action gate plus session scope and approval |
| network | network gate plus provenance/privacy checks |
| file write | writer gate plus diff, tests, rollback anchor |
| memory write | memory admission gate plus digest/privacy checks |
| process launch | process gate plus explicit approval |
| issue/PR creation | repository gate plus version/deprecation/refs evidence |
| genome mutation | genome writer gate plus validation, rollback, and manual review |

Machine posture:

- `read_only=true`
- `write_allowed=false`
- `applied=false`
- `issue375_expression_vm_side_effect=read_only`
- `issue375_genome_isa_apply_allowed=false`

## Clean-Room Provenance

| Reference class | Use | Copy status |
| --- | --- | --- |
| Avida, Tierra | genome as instruction sequence and resource-bound expression analogy | no source copied |
| PushGP, Clojush, Propeller | finite opcode and typed primitive analogy | no source copied |
| DEAP, PonyGE2 | genotype-to-phenotype mapping analogy | no source copied |
| OSWorld, SWE-agent, mini-SWE-agent | computer environment and adapter boundary analogy | no source copied |
| ReAct, Reflexion, Voyager | behavior primitive and feedback analogy | no source copied |
| Evo 2, AlphaGenome | sequence/expression vocabulary analogy | no source copied |
| RISC-V, WebAssembly, LLVM | ISA design analogy | no source copied |

License posture: clean-room, rust-norion terms only, no external code, prompt, schema, dataset, model, or restricted-license material copied.

## Acceptance Checklist

- `PreReasoningGenomeIsa`, `GenomeOpcode`, `ExpressionVM`, and `ReasoningFrame` are defined.
- The pre-reasoning layer is read-only and no-side-effect.
- Computer state is treated as environment stimulus/facts, not direct control.
- Genome expression emits `ReasoningFrame` before subjective model reasoning.
- Tool/action gates, writer gates, and computer-use adapters remain downstream.
- Shell, browser, network, file write, memory write, process launch, issue/PR creation, and genome mutation require downstream gates.
- Existing `GenomeExpression`, `TaskExpressionGene`, and `ThinkingScheduler` concepts are reused.
- #243 and #304 remain related but non-duplicated.
- Clean-room provenance and license posture are explicit.
- No new dependency, framework, durable write path, prompt-chain storage, or model-weight mutation is introduced.
