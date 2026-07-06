# Meta-Cognitive Evolution Loop

Status: issue #377 apply-closeout design note.

## Scope

| Item | Boundary |
| --- | --- |
| Goal | Convert current-run evidence into bounded self-evolution previews. |
| Default mode | read-only, digest-only, preview-only. |
| Durable writes | denied. |
| Apply owner | `UnifiedWriterGate` preflight plus explicit future apply workflow. |
| Out of scope | memory writes, genome writes, filesystem writes, Git writes, process launch, model-weight changes, hidden reasoning storage. |
| Non-goal | no consciousness claim; `self` means auditable control-plane self-model. |

## Loop

```text
SelfObservation
-> ProblemFinding
-> HypothesisCandidate
-> ExperimentPlan
-> EvidenceBundle
-> ExperimentDecision
-> MutationCandidateEmitter
-> UnifiedWriterGate preflight
-> manual_review
```

## Engine Contracts

| Engine | Emits | Required evidence | Forbidden |
| --- | --- | --- | --- |
| Self-observation and problem awareness | `SelfObservation`, `ProblemFinding` | source digest, current-truth digest, kind, severity, confidence, affected surface, next step | raw payload, repair authorization, write/apply |
| Hypothesis candidate | `HypothesisCandidate` | testable claim, target surface, expected metric/direction, required gates, rollback anchor | patch text, prompt text, genome splice text, write/apply |
| Experiment design and validation | `ExperimentPlan`, `EvidenceBundle`, `ExperimentDecision` | minimal level path, bounded budget, stop-on-fail, pass/fail counts, command label | runner authority, automatic experiment platform, apply |
| Consolidation and candidate emitter | `MutationCandidateEmitter` preview lanes | evidence digest, rollback anchor, requested write scope, writer-gate preflight owner | direct durable write, writer-gate bypass, ready-for-explicit-apply |

## Data Definitions

| Record | Minimal fields | Current machine evidence |
| --- | --- | --- |
| `SelfObservation` | schema, signal source, source digest, observation window, current-truth digest | `issue377_self_observation_schema=self_observation_v1`; `issue377_self_observation_digest_only=true`; `issue377_self_observation_write_allowed=false`; `issue377_self_observation_applied=false` |
| `ProblemFinding` | id, kind, severity, confidence, evidence digest, source digest, affected surface, next step | `issue377_problem_finding_present=true`; `issue377_problem_finding_raw_payload_present=false` |
| `HypothesisCandidate` | id, kind, status, target surface, metric, direction, gates, rollback anchor | `issue377_hypothesis_candidate_operator_approval_required=true`; `issue377_hypothesis_candidate_write_allowed=false`; `issue377_hypothesis_candidate_applied=false` |
| `ExperimentPlan` | id, mode, level path, gates, budget, stop-on-fail, rollback anchor | `issue377_experiment_plan_level_path=L0_schema_safety|L1_focused_validation|L3_benchmark`; `issue377_experiment_plan_write_allowed=false`; `issue377_experiment_plan_applied=false` |
| `EvidenceBundle` | id, schema, metric, direction, pass/fail counts, command label | `issue377_evidence_bundle_schema=evidence_bundle_v1`; `issue377_evidence_bundle_refs_digest_only=true` |
| `ExperimentDecision` | decision, schema, reason, evidence-bundle id, target, manual approval flag | `issue377_experiment_decision=promote_for_approval`; `issue377_experiment_decision_manual_approval_required=true`; `issue377_experiment_decision_apply_allowed=false` |
| `MutationCandidateEmitter` | emitter id, candidate id, evidence digest, rollback anchor, write scope, kind | `issue377_mutation_candidate_preview_only=true`; `issue377_mutation_candidate_apply_allowed=false`; `issue377_mutation_candidate_manual_review_required=true` |

## Predicament Trigger

| Field | Rule |
| --- | --- |
| `issue377_predicament_progress_delta` | stuck requires `0`. |
| `issue377_predicament_repeat_count` | stuck requires `>= 2`. |
| `issue377_predicament_evidence_gap_count` | `> 0` forces `hold_for_evidence`. |
| `issue377_predicament_action_novelty` | stuck requires `0`. |
| `issue377_predicament_stuck` | derived as `progress_delta == 0 && repeat_count >= 2 && action_novelty == 0`. |
| `issue377_best_next_state` | `problem_finding_preview` only when stuck and no evidence gap. |

## Admission Rule

| Order | Gate | Failure action |
| --- | --- | --- |
| 1 | user intent preservation | hold, reject, quarantine, or manual_review |
| 2 | safety | hold, reject, quarantine, or manual_review |
| 3 | digest-only evidence | hold, reject, quarantine, or manual_review |
| 4 | rollback anchor | hold, reject, quarantine, or manual_review |
| 5 | quality delta | tie-breaker only |
| 6 | cost delta | tie-breaker only |
| 7 | latency delta | tie-breaker only unless an issue makes it an SLO |

Risk blockers override performance:

- `issue377_negative_evidence_count > 0`
- `issue377_privacy_risk != low`
- `issue377_license_risk != low`
- `issue377_unsupported_capability_requested=true`
- `issue377_unsafe_side_effect_allowed=true`

## Validation Levels

| Level | Current #377 status |
| --- | --- |
| L0 schema/safety | required. |
| L1 focused validation | required. |
| L2 replay | skipped for current minimal existing evidence path. |
| L3 benchmark | required. |
| L4 integration/readiness | skipped for current minimal existing evidence path. |
| L5 promotion window | skipped for current minimal existing evidence path. |
| L6 human apply | outside the engine. |

Machine fields:

- `issue377_validation_skipped_levels=L2_replay|L4_integration_readiness|L5_promotion_window`
- `issue377_validation_skipped_reason=minimal_existing_evidence_path`
- `issue377_human_apply_level=L6_human_apply`
- `issue377_human_apply_inside_engine=false`
- `issue377_validation_level_apply_allowed=false`

## Candidate Emitter Boundary

| Lane | Candidate kind |
| --- | --- |
| `reasoning_genome_preview` | `mutation_plan_preview` |
| `memory_admission_preview` | `memory_admission_preview` |
| `routing_policy_preview` | `routing_shadow_proposal` |
| `tool_policy_preview` | `tool_policy_candidate` |
| `evolution_goal_preview` | `evolution_goal_preview` |

Machine fields:

- `issue377_candidate_emitter_durable_preflight_owner=unified_writer_gate`
- `issue377_candidate_emitter_writer_gate_bypass_allowed=false`
- `issue377_candidate_emitter_direct_durable_write_allowed=false`
- `issue377_candidate_emitter_ready_for_explicit_apply=false`

## Manual Approval Binding

| Binding | Required |
| --- | --- |
| candidate id | `issue377_manual_approval_candidate_id` equals mutation candidate id. |
| evidence digest | `issue377_manual_approval_evidence_digest` equals mutation evidence digest. |
| rollback anchor | `issue377_manual_approval_rollback_anchor` equals mutation rollback anchor. |
| write scope | `issue377_manual_approval_requested_write_scope` equals requested write scope. |
| approval ref | `issue377_manual_approval_ref` is digest-only. |
| expiration | `issue377_manual_approval_expiration` is present. |
| apply | `issue377_manual_approval_apply_allowed=false`; `issue377_manual_approval_applied=false`. |

## Related Issue Boundary

| Issue | Owner scope |
| --- | --- |
| #6 | experiment gates |
| #7 | memory admission pipeline |
| #74 | thinking scheduler |
| #79 | evolution goal queue |
| #375 | pre-reasoning Genome ISA |
| #377 | meta-cognitive evolution loop |

Machine fields:

- `issue377_related_issue_refs=#6|#7|#74|#79|#375`
- `issue377_related_issue_scope_map=#6:experiment_gates|#7:memory_admission_pipeline|#74:thinking_scheduler|#79:evolution_goal_queue|#375:pre_reasoning_genome_isa`
- `issue377_related_issue_owner_scope=meta_cognitive_evolution_loop`
- `issue377_related_issue_non_duplicate_count=5`
- `issue377_related_issue_all_non_duplicate=true`
- `issue377_related_issue_apply_allowed=false`

## Clean-Room Posture

| Reference class | Use |
| --- | --- |
| Reflexion, Self-Refine, ReAct, Voyager | behavior pattern references only. |
| Darwin Godel Machine, AlphaEvolve, ADAS, POET | evaluator/archive/search references only. |
| SWE-agent, OpenHands | downstream sandbox validation references only. |
| OpenTelemetry, Prometheus, Merlion | telemetry and anomaly-signal references only. |

Machine fields:

- `issue377_clean_room_reference_mode=rust_norion_terms_only`
- `issue377_external_code_copied=false`
- `issue377_external_prompt_or_schema_copied=false`
- `issue377_restricted_license_material_copied=false`
- `issue377_license_provenance_posture=project_owned_digest_only`
- `issue377_clean_room_apply_allowed=false`

## Implementation Anchors

| Surface | Anchor |
| --- | --- |
| Predicament signal | `src/self_goal_proposal.rs` |
| Issue #30/#377 packet row | `src/benchmark/roundtrip.rs` |
| CLI validator and negative tests | `crates/norion-cli/src/evidence_packet.rs` |
| Fresh checkout gate | `tools/ci/issue30-fresh-checkout-smoke.sh` |
| Version/deprecation ledger | `VERSION_LEDGER.md` |

## Acceptance Checklist

- `SelfObservation`, `ProblemFinding`, `HypothesisCandidate`, `ExperimentPlan`, `EvidenceBundle`, `ExperimentDecision`, and `MutationCandidateEmitter` are represented.
- Problem findings are evidence-backed, digest-only, confidence-scored, and source-bound.
- Predicament detection uses progress delta, repeat count, evidence gap count, and action novelty.
- Best-next-state admission is lexicographic and fail-closed.
- Negative evidence, privacy risk, license risk, unsupported capability, unsafe side effects, and rollback failure outrank performance.
- Candidate output is preview-only and manual-review-only.
- Unified writer gate remains the durable-write preflight owner.
- Manual approval binds candidate id, evidence digest, rollback anchor, requested write scope, approval ref, and expiration.
- #6, #7, #74, #79, and #375 stay related but non-duplicated.
- Clean-room evidence stays project-owned and digest-only.
