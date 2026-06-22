# Cross-Window Experience Exchange

Issue #70 adds a payload-free exchange protocol for four parallel development
windows. Each window reports compact evidence packets to the main coordinator;
the coordinator merges only non-conflicting packets and quarantines stale,
duplicate, polluted, over-budget, or ownership-conflicting packets.

The exchange is a report surface, not an authority surface. It cannot write
memory, mutate the Reasoning Genome, merge code, bypass CI, bypass maintainer
approval, or approve rollback-sensitive self-evolution changes.

## Packet Shape

`CrossWindowExperiencePacket` is the unit exchanged between windows. A packet
contains:

- `packet_id`, `packet_digest`, and `provenance_digest`
- `lane_id` and `source_window_id`
- `TenantScope`
- owner `AgentRole`
- freshness epoch
- redacted summary
- files touched
- tests run
- decisions made
- blockers and risks
- next handoff and next recommended issue
- evidence ids represented as digests
- token and step budget counters
- raw/private payload flags and redaction count

Packet text fields are sanitized when they enter the packet. Raw prompts, raw
answers, hidden reasoning, secrets, transcript dumps, and private payloads must
not be copied into packets. If a packet carries raw/private payload flags, a raw
payload marker, or redactions, the aggregator quarantines it.

## Four-Window Handoff

A four-window run should use stable lanes before work begins. A typical split
is:

- `runtime`: model service, session state, adapter, streaming, cancellation
- `memory`: disk KV, vector cache, KV-Fusion, admission and retention
- `genome`: Reasoning Genome, splicing, Gene Scissors, relabel/regeneration
- `validation`: tests, benchmarks, trace gates, issue/PR evidence

Each window owns its lane and should avoid touching files owned by another
lane. The main coordinator constructs `CrossWindowExchangeContext` with the
current epoch, expected tenant scope, and `AgentHandoffContext` for branch,
head, known issues, known PRs, and dirty files. Windows submit packets with
only redacted summaries and command evidence.

The coordinator then calls:

```rust
let report = CrossWindowExchangeAggregator::new().aggregate(&context, &packets);
```

The returned `CrossWindowExchangeReport` contains merged summaries, files,
tests, decisions, blockers, risks, evidence digests, packet reviews, a budget
report, and an embedded `AgentHandoffAggregationReport`.

## Conflict Handling

The aggregator accepts only packets that are fresh, unique, in-scope,
non-polluted, within budget, and non-conflicting with already accepted lane/file
owners. It emits `CrossWindowPacketReview` rows with deterministic conflict
classes:

- `duplicate_packet`
- `file_overlap`
- `lane_owner_collision`
- `stale_packet`
- `polluted_payload`
- `budget_exceeded`

Duplicate packets are counted separately and are not merged twice. Conflicting
or stale packets are quarantined and their blockers/risks stay visible in the
report, but their work summary, tests, files, and evidence digests do not enter
the accepted merge set.

## Budget Report

`CrossWindowBudgetReport` summarizes accepted work:

- number of windows and lanes
- accepted, duplicate, and quarantined packet counts
- token budget, spent tokens, and remaining tokens
- step budget, spent steps, and remaining steps
- work done
- tests run
- unresolved blockers
- next recommended issue

Budget exhaustion quarantines the packet because a depleted window cannot spend
from another lane's allowance. The report remains useful as evidence for the
main coordinator to schedule repair or continuation work.

## Admission Boundary

All cross-window exchange output is preview-only, read-only, and report-only:

- `can_promote_memory = false`
- `can_bypass_approval = false`
- durable writes remain disabled
- accepted packets may inform the agent-team coordinator only when there are no
  duplicate or quarantined packets and the embedded handoff sanitizer reports no
  quarantined handoffs

This keeps parallel development fast while preserving the core rust-norion
rules: no raw private payloads, no hidden context copying, no cross-tenant
pollution, no uncontrolled self-evolution, and no merge without maintainer
approval and validation evidence.
