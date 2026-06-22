# Malignant Gene Recovery Drills

Issue #50 turns the "malignant gene" idea into software fault isolation. It is
not a biological claim. In rust-norion, a malignant gene or segment is a
metadata/control-plane unit that has become unsafe to express because of drift,
privacy risk, contradiction, false memory, unsafe routing pressure, or
destructive mutation intent.

## Drill Flow

The recovery drill is deterministic and read-only:

1. Detect polluted `GeneSegment` evidence with `MutDetector`.
2. Quarantine the target segment before expression or runtime-KV import.
3. Create a reversible cut candidate with a tombstone id.
4. Create a young regeneration candidate from the stable rollback anchor.
5. Replay validation decides whether the candidate stays held or rejected.
6. Durable apply remains blocked until validation, rollback evidence, redaction,
   and operator approval all pass.

## Fixture Coverage

`MalignantGeneRecoveryDrillCorpus` covers:

- malicious instruction segments
- false memory
- bad routing thresholds
- contradictory rules
- stale labels
- irreversible delete attempts

Every fixture is synthetic. Evidence stores only digest-only summaries, stable
rollback anchors, classification, confidence, validation status, redaction
status, tombstone/cut/regeneration presence, and hold reasons.

## Invariants

- Malignant segments are quarantined and never promoted by default.
- Regeneration sources must include the stable anchor and must not copy the
  polluted target as a source.
- Cut candidates are reversible tombstone previews, not durable deletes.
- Failed replay validation creates auditable hold/reject reasons.
- Evidence packets must remain redacted and preview-only.
- Neighboring healthy segments must stay retained.
