# Memory KV Ledger Writer Gates

This document defines the first append-only disk-backed KV writer gate for
rust-norion memory admission. The gate is a software-control layer: it does not
train model weights and it does not grant automatic memory mutation.

## Gate Inputs

`MemoryAdmissionPreview` produces candidates and review packets from reflection,
process reward, drift, runtime KV, gist memory, and tool reliability evidence.
Each candidate carries:

- approval state through its review packet
- `source_hash`
- prompt digest only, not raw prompt text
- privacy classification and `privacy_checked`
- rollback anchor
- validation evidence

## Ledger Records

`MemoryKvLedgerWritePlan` turns admission candidates into append-only
`MemoryKvLedgerRecord` rows. The plan records every candidate, including
preview-only, held, rejected, duplicate, decayed, merged, and rollback cases, so
operators can audit why a memory did or did not become durable.

Default policy keeps research runs preview-only:

- `durable_writes_enabled = false`
- `operator_approved = false`
- `write_allowed = false`
- `applied = false`

## Write Authorization

A record can be authorized only when all gates pass:

- candidate decision is ready
- durable writes are enabled by policy
- operator approval is present
- source hash is present
- privacy gate passed
- rollback anchor is present
- validation evidence is present
- record is not duplicate, decayed, merged, rejected, held, quarantined, or in
  rollback mode

Authorized records may then be appended to an isolated `DiskKvStore` using
`append_authorized_records`. Tests write only temporary `.ndkv` fixtures.

## Trace And Benchmark Evidence

Trace and benchmark summaries count ledger records, authorized writes, applied
writes, preview-only records, held records, rejected records, duplicate records,
decayed records, merged records, and rollback records. Normal inference keeps
authorized and applied counts at zero until a caller explicitly enables the
writer policy and passes operator approval.
